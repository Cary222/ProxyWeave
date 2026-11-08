//! Port allocation and conflict detection with explicit evidence collection.
//!
//! Rule per ADR-003 and Plan §6.3:
//! A readiness timeout or probe timeout must NEVER be inferred as a port conflict.
//! A port conflict is only declared when explicit evidence exists (such as `AddrInUse`
//! / WSAEADDRINUSE 10048 during bind, or explicit child process exit logs).
//!
//! A bind can also fail for a reason that is not "someone holds this port": Windows WinNAT
//! and Hyper-V reserve whole port segments and answer `WSAEACCES` (10013) for every port
//! inside them. Such a refusal is never read as a conflict. It is classified and handed to
//! the caller verbatim, so the allocator can step over the segment instead of treating a
//! reserved port as a busy one.

use std::io;
use std::net::{SocketAddr, TcpListener};

/// How a bind probe failed, in the terms the allocator needs to decide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeFailureKind {
    /// The port is held by another socket (`WSAEADDRINUSE` / `EADDRINUSE`): proven conflict.
    AddressInUse,
    /// The OS refuses to bind anywhere in this segment (`WSAEACCES` / `EACCES`): a reserved
    /// segment, not a conflict, and not a candidate that was actually screened.
    PermissionDenied,
    /// Anything else; kept verbatim, never read as a conflict.
    Other,
}

impl std::fmt::Display for ProbeFailureKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::AddressInUse => "address_in_use",
            Self::PermissionDenied => "permission_denied",
            Self::Other => "other",
        };
        f.write_str(name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortConflictError {
    AddressInUse {
        port: u16,
        detail: String,
    },
    IoError {
        port: u16,
        /// The `io::ErrorKind` the OS reported, kept so a reserved segment can be told
        /// apart from an unrelated syscall failure.
        kind: io::ErrorKind,
        /// Raw OS error code when the OS supplied one (`10013` = `WSAEACCES` on Windows).
        raw_os_error: Option<i32>,
        message: String,
    },
}

impl PortConflictError {
    /// Port the probe was run against.
    pub fn port(&self) -> u16 {
        match self {
            Self::AddressInUse { port, .. } | Self::IoError { port, .. } => *port,
        }
    }

    /// Three-way classification; a non-`AddressInUse` verdict is never a conflict.
    pub fn failure_kind(&self) -> ProbeFailureKind {
        match self {
            Self::AddressInUse { .. } => ProbeFailureKind::AddressInUse,
            Self::IoError { kind, .. } => match kind {
                io::ErrorKind::AddrInUse => ProbeFailureKind::AddressInUse,
                io::ErrorKind::PermissionDenied => ProbeFailureKind::PermissionDenied,
                _ => ProbeFailureKind::Other,
            },
        }
    }

    /// True when the OS excluded the whole segment, as Windows reserved port ranges do.
    pub fn is_reserved_segment(&self) -> bool {
        self.failure_kind() == ProbeFailureKind::PermissionDenied
    }

    /// Raw OS error code, when the OS supplied one.
    pub fn raw_os_error(&self) -> Option<i32> {
        match self {
            Self::AddressInUse { .. } => None,
            Self::IoError { raw_os_error, .. } => *raw_os_error,
        }
    }

    /// `io::ErrorKind` for an `IoError` verdict.
    pub fn io_kind(&self) -> Option<io::ErrorKind> {
        match self {
            Self::AddressInUse { .. } => None,
            Self::IoError { kind, .. } => Some(*kind),
        }
    }
}

impl std::fmt::Display for PortConflictError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AddressInUse { port, detail } => {
                write!(f, "Port {} is already in use: {}", port, detail)
            }
            Self::IoError {
                port,
                kind,
                raw_os_error,
                message,
            } => match raw_os_error {
                Some(code) => write!(
                    f,
                    "Failed to check port {}: {} (kind {:?}, os error {})",
                    port, message, kind, code
                ),
                None => write!(
                    f,
                    "Failed to check port {}: {} (kind {:?})",
                    port, message, kind
                ),
            },
        }
    }
}

impl std::error::Error for PortConflictError {}

/// Checks whether a loopback port is available for binding on 127.0.0.1.
/// Returns `Ok(())` if available.
/// Returns `Err(PortConflictError::AddressInUse)` if explicitly already in use.
pub fn check_port_available(port: u16) -> Result<(), PortConflictError> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    match TcpListener::bind(addr) {
        Ok(listener) => {
            drop(listener);
            Ok(())
        }
        Err(err) if err.kind() == io::ErrorKind::AddrInUse => {
            Err(PortConflictError::AddressInUse {
                port,
                detail: format!(
                    "TCP bind on 127.0.0.1:{} failed with AddrInUse ({})",
                    port, err
                ),
            })
        }
        // Everything else keeps the OS verdict intact: kind and raw code travel with the
        // error, so `WSAEACCES` (reserved segment) is never mistaken for a busy port.
        Err(err) => Err(PortConflictError::IoError {
            port,
            kind: err.kind(),
            raw_os_error: err.raw_os_error(),
            message: format!("TCP bind error on 127.0.0.1:{}: {}", port, err),
        }),
    }
}

/// Finds an available ephemeral loopback port by binding to port 0.
pub fn pick_unused_port() -> io::Result<u16> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let listener = TcpListener::bind(addr)?;
    let local_addr = listener.local_addr()?;
    drop(listener);
    Ok(local_addr.port())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pick_unused_port_returns_valid_port() {
        let port = pick_unused_port().expect("pick unused port");
        assert!(port > 0);
        // It should be available immediately after drop
        assert!(check_port_available(port).is_ok());
    }

    #[test]
    fn test_port_conflict_detection_when_bound() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let listener = TcpListener::bind(addr).expect("bind listener");
        let port = listener.local_addr().unwrap().port();

        // While listener is held, check_port_available must return AddressInUse
        let err = check_port_available(port).expect_err("must detect conflict");
        match err {
            PortConflictError::AddressInUse { port: p, detail } => {
                assert_eq!(p, port);
                assert!(detail.contains("AddrInUse"));
            }
            other => panic!("expected AddressInUse, got: {:?}", other),
        }

        // After dropping listener, port becomes available
        drop(listener);
        assert!(check_port_available(port).is_ok());
    }

    #[test]
    fn test_io_error_kinds_are_classified_with_their_raw_code() {
        let reserved = PortConflictError::IoError {
            port: 20_000,
            kind: io::ErrorKind::PermissionDenied,
            raw_os_error: Some(10013),
            message: "TCP bind error on 127.0.0.1:20000: WSAEACCES".to_string(),
        };
        assert_eq!(reserved.failure_kind(), ProbeFailureKind::PermissionDenied);
        assert!(reserved.is_reserved_segment());
        assert_eq!(reserved.raw_os_error(), Some(10013));
        assert_eq!(reserved.port(), 20_000);
        assert!(reserved.to_string().contains("WSAEACCES"));
        assert!(reserved.to_string().contains("10013"));

        let unrelated = PortConflictError::IoError {
            port: 20_001,
            kind: io::ErrorKind::ConnectionRefused,
            raw_os_error: None,
            message: "TCP bind error on 127.0.0.1:20001: other".to_string(),
        };
        assert_eq!(unrelated.failure_kind(), ProbeFailureKind::Other);
        assert!(!unrelated.is_reserved_segment());
        assert_eq!(unrelated.raw_os_error(), None);

        let busy = PortConflictError::AddressInUse {
            port: 20_002,
            detail: "in use".to_string(),
        };
        assert_eq!(busy.failure_kind(), ProbeFailureKind::AddressInUse);
        assert!(!busy.is_reserved_segment());
        assert_eq!(busy.io_kind(), None);
    }
}
