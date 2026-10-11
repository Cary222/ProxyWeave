//! Port allocation and conflict detection with explicit evidence collection.
//!
//! Rule per ADR-003 and Plan §6.3:
//! A readiness timeout or probe timeout must NEVER be inferred as a port conflict.
//! A port conflict is only declared when explicit evidence exists (such as `AddrInUse`
//! / WSAEADDRINUSE 10048 during bind, or explicit child process exit logs).

use std::io;
use std::net::{SocketAddr, TcpListener};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortConflictError {
    AddressInUse { port: u16, detail: String },
    IoError { port: u16, message: String },
}

impl std::fmt::Display for PortConflictError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AddressInUse { port, detail } => {
                write!(f, "Port {} is already in use: {}", port, detail)
            }
            Self::IoError { port, message } => {
                write!(f, "Failed to check port {}: {}", port, message)
            }
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
        Err(err) => Err(PortConflictError::IoError {
            port,
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
}
