//! Dynamic loopback port ledger for Managed plugins (spec v10, Plan.md §6.3).
//!
//! Responsibilities:
//! - Hand out a loopback port per Managed plugin instance, proving it free by an actual
//!   bind probe (`check_port_available`) before it is returned.
//! - Never hand the same port to two live instances: the ledger is process-wide, every
//!   reservation is tracked by port, and reservations are reference counted, so two leases
//!   of the same port for the same plugin both have to be dropped before it is reclaimed.
//! - Reclaim deterministically: a [`PortLease`] returns its port to the ledger on drop
//!   (stop, restart, crash reconciliation, failed spawn), so nothing leaks across restarts.
//!
//! Rule kept from ADR-003: a conflict is only ever *proven* by `AddrInUse` evidence from a
//! bind probe. A slow start or a probe timeout is never read as a port conflict.
//!
//! ## Alignment with Plan.md §6.3
//!
//! - Range: Plan.md §6.3 A fixes `[20000, 60000]`; [`PortRange::default`] is exactly that.
//! - Initial screening: Plan.md §6.3 A asks for a randomised-backoff screening pass. The
//!   first candidate of every allocation is drawn from a per-ledger xorshift PRNG and the
//!   scan then walks forward, so concurrent allocations do not all converge on the lowest
//!   free port (the thundering-herd pattern portpicker-rs avoids).
//! - `portpicker-rs` itself is not taken as a dependency: its `pick_unused_port` cannot
//!   express (a) a per-process exclusion/lease ledger or (b) the typed `AddrInUse` evidence
//!   ADR-003 requires for the retry decision, and the project already has that probe as
//!   tested code in `crate::port::check_port_available` (Direct Reuse First, level 2:
//!   reuse the existing module rather than a new dependency). Deviation recorded in
//!   `docs/architecture/ADR-008-dynamic-port-allocation-and-conflict-recovery.md`.
//! - Plan.md §6.3 B (proven conflict → blacklist → re-screen → re-launch with the new port,
//!   at most 3 retries, then `Failed` + structured diagnostic) is implemented in
//!   `runtime::RuntimeManager::start_managed`, which uses [`PortAllocator::allocate_excluding`]
//!   to honour the blacklist. This module owns allocation only.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::port::{check_port_available, PortConflictError};

/// First port of the default dynamic range (Plan.md §6.3 A).
pub const DEFAULT_PORT_RANGE_START: u16 = 20_000;
/// Last port of the default dynamic range (Plan.md §6.3 A).
pub const DEFAULT_PORT_RANGE_END: u16 = 60_000;
/// Default probe budget for one allocation.
pub const DEFAULT_MAX_ATTEMPTS: u16 = 64;

/// Inclusive loopback port range the allocator hands out from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PortRange {
    pub start: u16,
    pub end: u16,
}

impl PortRange {
    /// Builds a validated range. Port 0 is rejected: the kernel would pick a port we could
    /// not record in the route table.
    pub fn new(start: u16, end: u16) -> Result<Self, PortAllocatorError> {
        if start == 0 || end < start {
            return Err(PortAllocatorError::InvalidRange { start, end });
        }
        Ok(Self { start, end })
    }

    /// The range Plan.md §6.3 A mandates for dynamic plugin ports.
    pub fn plan_default() -> Self {
        Self {
            start: DEFAULT_PORT_RANGE_START,
            end: DEFAULT_PORT_RANGE_END,
        }
    }

    /// Number of ports in the range.
    pub fn len(&self) -> u32 {
        u32::from(self.end) - u32::from(self.start) + 1
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn contains(&self, port: u16) -> bool {
        port >= self.start && port <= self.end
    }
}

impl Default for PortRange {
    fn default() -> Self {
        Self::plan_default()
    }
}

/// Why an allocation or reservation failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortAllocatorError {
    /// Every probe in the budget failed with provable `AddrInUse` evidence.
    AddressInUse { port: u16, attempts: u16 },
    /// The range was fully scanned without finding a free port (or the budget ran out
    /// with no proven conflict, e.g. repeated OS errors).
    RangeExhausted { range: PortRange, attempts: u16 },
    /// The requested range is not usable.
    InvalidRange { start: u16, end: u16 },
    /// The port is already leased to another plugin in this process.
    AlreadyReserved { port: u16, plugin_id: String, by: String },
    /// The plugin already holds a different port; release it first.
    PluginAlreadyActive { plugin_id: String, port: u16 },
    /// An empty plugin id cannot own a port.
    EmptyPluginId,
}

impl std::fmt::Display for PortAllocatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AddressInUse { port, attempts } => write!(
                f,
                "No free port after {attempts} probes; last proven conflict on port {port} (AddrInUse)"
            ),
            Self::RangeExhausted { range, attempts } => write!(
                f,
                "Port range {}-{} exhausted after {attempts} probes",
                range.start, range.end
            ),
            Self::InvalidRange { start, end } => {
                write!(f, "Invalid port range {start}-{end}")
            }
            Self::AlreadyReserved { port, plugin_id, by } => write!(
                f,
                "Port {port} is already leased to plugin '{by}' (requested by '{plugin_id}')"
            ),
            Self::PluginAlreadyActive { plugin_id, port } => write!(
                f,
                "Plugin '{plugin_id}' already holds port {port}; release it before allocating again"
            ),
            Self::EmptyPluginId => write!(f, "Plugin id must not be empty"),
        }
    }
}

impl std::error::Error for PortAllocatorError {}

impl From<PortConflictError> for PortAllocatorError {
    fn from(err: PortConflictError) -> Self {
        match err {
            PortConflictError::AddressInUse { port, .. } => Self::AddressInUse {
                port,
                attempts: 1,
            },
            PortConflictError::IoError { port, .. } => Self::RangeExhausted {
                range: PortRange { start: port, end: port },
                attempts: 1,
            },
        }
    }
}

/// A live reservation, as reported by [`PortAllocator::stats`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PortReservation {
    pub plugin_id: String,
    pub port: u16,
    /// Outstanding leases on this port. One plugin may legitimately hold several (an
    /// idempotent `allocate_specific`); the port is reclaimed when this reaches zero.
    pub refs: u32,
}

/// Point-in-time view of the ledger, for the Workbench route table and tests.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PortAllocatorStats {
    pub range: PortRange,
    pub reserved: usize,
    pub available: u32,
    pub cursor: u16,
    pub reservations: Vec<PortReservation>,
}

#[derive(Debug, Clone)]
struct Reservation {
    plugin_id: String,
    refs: u32,
}

#[derive(Debug)]
struct Ledger {
    /// port -> reservation
    ports: HashMap<u16, Reservation>,
    /// Next candidate offset; makes allocation walk forward instead of re-probing the same
    /// lowest port when several processes start together.
    cursor: u16,
    /// PRNG state for the randomised initial candidate (Plan.md §6.3 A).
    rng: u64,
}

impl Ledger {
    fn owner_of(&self, port: u16) -> Option<&str> {
        self.ports.get(&port).map(|r| r.plugin_id.as_str())
    }

    fn port_of(&self, plugin_id: &str) -> Option<u16> {
        self.ports
            .iter()
            .find(|(_, reservation)| reservation.plugin_id == plugin_id)
            .map(|(port, _)| *port)
    }

    /// Adds one lease. Returns the number of leases now held on that port.
    fn add_ref(&mut self, port: u16, plugin_id: &str) -> u32 {
        let entry = self
            .ports
            .entry(port)
            .or_insert_with(|| Reservation {
                plugin_id: plugin_id.to_string(),
                refs: 0,
            });
        entry.refs = entry.refs.saturating_add(1);
        entry.refs
    }

    /// Drops one lease. Returns `true` when the port was reclaimed.
    fn drop_ref(&mut self, port: u16) -> bool {
        match self.ports.get_mut(&port) {
            Some(entry) if entry.refs > 1 => {
                entry.refs -= 1;
                false
            }
            Some(_) => {
                self.ports.remove(&port);
                true
            }
            None => false,
        }
    }

    fn next_random(&mut self) -> u64 {
        // xorshift64*: deterministic per ledger, no dependency, good enough to decorrelate
        // the first candidate of concurrent allocations.
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

fn seed() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15);
    // Never seed with 0: xorshift stays at 0 forever.
    nanos ^ (u64::from(std::process::id()) << 32) ^ 0xD1B5_4A32_D192_ED03
}

/// Process-wide dynamic port ledger. Cloning shares the same ledger.
#[derive(Clone)]
pub struct PortAllocator {
    inner: Arc<Mutex<Ledger>>,
    range: PortRange,
}

impl Default for PortAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for PortAllocator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let stats = self.stats();
        f.debug_struct("PortAllocator")
            .field("range", &stats.range)
            .field("reserved", &stats.reserved)
            .finish()
    }
}

impl PortAllocator {
    /// Ledger over the Plan.md §6.3 range.
    pub fn new() -> Self {
        Self::with_range(PortRange::plan_default())
    }

    /// Ledger over an explicit range (tests, or a deployment pinning a window).
    pub fn with_range(range: PortRange) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Ledger {
                ports: HashMap::new(),
                cursor: range.start,
                rng: seed(),
            })),
            range,
        }
    }

    pub fn range(&self) -> PortRange {
        self.range
    }

    /// Allocates a proven-free port, probing up to [`DEFAULT_MAX_ATTEMPTS`] candidates.
    pub fn allocate(&self, plugin_id: &str) -> Result<PortLease, PortAllocatorError> {
        self.allocate_with_attempts(plugin_id, DEFAULT_MAX_ATTEMPTS)
    }

    /// Allocates with an explicit probe budget. The budget is capped by the range size.
    pub fn allocate_with_attempts(
        &self,
        plugin_id: &str,
        max_attempts: u16,
    ) -> Result<PortLease, PortAllocatorError> {
        self.allocate_excluding_with_attempts(plugin_id, max_attempts, &[])
    }

    /// Allocates a proven-free port while skipping ports that have already produced
    /// `AddrInUse` evidence during this start attempt (Plan.md §6.3 B blacklist).
    pub fn allocate_excluding(
        &self,
        plugin_id: &str,
        excluded: &[u16],
    ) -> Result<PortLease, PortAllocatorError> {
        self.allocate_excluding_with_attempts(plugin_id, DEFAULT_MAX_ATTEMPTS, excluded)
    }

    /// [`Self::allocate_excluding`] with an explicit probe budget.
    pub fn allocate_excluding_with_attempts(
        &self,
        plugin_id: &str,
        max_attempts: u16,
        excluded: &[u16],
    ) -> Result<PortLease, PortAllocatorError> {
        if plugin_id.trim().is_empty() {
            return Err(PortAllocatorError::EmptyPluginId);
        }
        let mut ledger = self.lock();
        if let Some(port) = ledger.port_of(plugin_id) {
            return Err(PortAllocatorError::PluginAlreadyActive {
                plugin_id: plugin_id.to_string(),
                port,
            });
        }

        let span = self.range.len();
        let budget = u32::from(max_attempts.max(1)).min(span);
        let start = u32::from(self.range.start);
        // Randomised screening pass, then a forward walk from there (Plan.md §6.3 A).
        let mut offset = (ledger.next_random() % u64::from(span)) as u32;
        let mut last_conflict: Option<u16> = None;
        let mut attempts: u16 = 0;

        for _ in 0..budget {
            let candidate = (start + offset) as u16;
            offset = (offset + 1) % span;
            attempts = attempts.saturating_add(1);

            if excluded.contains(&candidate) || ledger.ports.contains_key(&candidate) {
                continue;
            }
            match check_port_available(candidate) {
                Ok(()) => {
                    ledger.add_ref(candidate, plugin_id);
                    ledger.cursor = next_port(&self.range, candidate);
                    return Ok(PortLease::new(self.clone(), plugin_id.to_string(), candidate));
                }
                // Proven conflict: try the next candidate.
                Err(PortConflictError::AddressInUse { .. }) => {
                    last_conflict = Some(candidate);
                }
                // Not conflict evidence; skip this candidate and keep scanning.
                Err(PortConflictError::IoError { .. }) => {}
            }
        }

        match last_conflict {
            Some(port) => Err(PortAllocatorError::AddressInUse { port, attempts }),
            None => Err(PortAllocatorError::RangeExhausted {
                range: self.range,
                attempts,
            }),
        }
    }

    /// Reserves one exact port (an explicit Manifest or user-chosen port).
    ///
    /// Idempotent for the same plugin: re-entering with the port it already holds adds a
    /// lease on the *same* reservation instead of a second ledger entry, and returns a new
    /// [`PortLease`]. Both leases must be dropped before the port is reclaimed, so one
    /// instance dropping its lease can never free a port another instance is still using.
    pub fn allocate_specific(
        &self,
        plugin_id: &str,
        port: u16,
    ) -> Result<PortLease, PortAllocatorError> {
        if plugin_id.trim().is_empty() {
            return Err(PortAllocatorError::EmptyPluginId);
        }
        if port == 0 {
            return Err(PortAllocatorError::InvalidRange {
                start: port,
                end: port,
            });
        }
        let mut ledger = self.lock();
        if let Some(by) = ledger.owner_of(port).map(str::to_string) {
            if by == plugin_id {
                ledger.add_ref(port, plugin_id);
                return Ok(PortLease::new(self.clone(), plugin_id.to_string(), port));
            }
            return Err(PortAllocatorError::AlreadyReserved {
                port,
                plugin_id: plugin_id.to_string(),
                by,
            });
        }
        if let Some(held) = ledger.port_of(plugin_id) {
            return Err(PortAllocatorError::PluginAlreadyActive {
                plugin_id: plugin_id.to_string(),
                port: held,
            });
        }
        check_port_available(port)?;
        ledger.add_ref(port, plugin_id);
        Ok(PortLease::new(self.clone(), plugin_id.to_string(), port))
    }

    /// Hard reclaim: removes the port from the ledger regardless of outstanding leases.
    /// Returns `true` when the port was reserved. Deliberate shutdown paths use this; an
    /// ordinary instance drop goes through the reference count instead.
    pub fn release(&self, port: u16) -> bool {
        self.lock().ports.remove(&port).is_some()
    }

    /// Drops one lease. Returns `true` when the port was reclaimed.
    fn release_ref(&self, port: u16) -> bool {
        self.lock().drop_ref(port)
    }

    /// Owner of a reserved port, if any.
    pub fn owner_of(&self, port: u16) -> Option<String> {
        self.lock().owner_of(port).map(str::to_string)
    }

    /// Port currently held by a plugin, if any.
    pub fn port_of(&self, plugin_id: &str) -> Option<u16> {
        self.lock().port_of(plugin_id)
    }

    /// Outstanding leases on a port (0 when unreserved).
    pub fn refs(&self, port: u16) -> u32 {
        self.lock().ports.get(&port).map(|r| r.refs).unwrap_or(0)
    }

    pub fn reserved_count(&self) -> usize {
        self.lock().ports.len()
    }

    /// Snapshot for the route table / Workbench (sorted by port, stable for diffing).
    pub fn stats(&self) -> PortAllocatorStats {
        let ledger = self.lock();
        let mut reservations: Vec<PortReservation> = ledger
            .ports
            .iter()
            .map(|(port, reservation)| PortReservation {
                plugin_id: reservation.plugin_id.clone(),
                port: *port,
                refs: reservation.refs,
            })
            .collect();
        reservations.sort_by_key(|r| r.port);
        PortAllocatorStats {
            range: self.range,
            reserved: reservations.len(),
            available: self.range.len().saturating_sub(reservations.len() as u32),
            cursor: ledger.cursor,
            reservations,
        }
    }

    /// Recovers the lock if a holder panicked; a poisoned ledger is still structurally sound.
    fn lock(&self) -> std::sync::MutexGuard<'_, Ledger> {
        match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

fn next_port(range: &PortRange, port: u16) -> u16 {
    if port >= range.end {
        range.start
    } else {
        port + 1
    }
}

/// RAII reservation. Dropping it drops one lease; the port returns to the ledger when the
/// last lease on it is gone.
pub struct PortLease {
    allocator: PortAllocator,
    plugin_id: String,
    port: u16,
    released: bool,
}

impl PortLease {
    fn new(allocator: PortAllocator, plugin_id: String, port: u16) -> Self {
        Self {
            allocator,
            plugin_id,
            port,
            released: false,
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    pub fn is_released(&self) -> bool {
        self.released
    }

    /// Releases this lease explicitly; the port is available immediately unless another
    /// lease still holds it.
    pub fn release(mut self) {
        self.release_inner();
    }

    fn release_inner(&mut self) {
        if !self.released {
            self.allocator.release_ref(self.port);
            self.released = true;
        }
    }
}

impl Drop for PortLease {
    fn drop(&mut self) {
        self.release_inner();
    }
}

impl std::fmt::Debug for PortLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PortLease")
            .field("plugin_id", &self.plugin_id)
            .field("port", &self.port)
            .field("released", &self.released)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    fn small_allocator() -> PortAllocator {
        PortAllocator::with_range(PortRange::new(52000, 52004).expect("valid range"))
    }

    #[test]
    fn test_range_validation_rejects_zero_and_reversed() {
        assert!(PortRange::new(0, 10).is_err());
        assert!(PortRange::new(60000, 59000).is_err());
        assert_eq!(PortRange::new(50000, 50009).unwrap().len(), 10);
    }

    #[test]
    fn test_default_range_matches_plan_6_3() {
        let range = PortRange::default();
        assert_eq!(range.start, 20000);
        assert_eq!(range.end, 60000);
        assert_eq!(PortAllocator::new().range(), range);
    }

    #[test]
    fn test_allocate_returns_proven_free_port_inside_range() {
        let allocator = small_allocator();
        let lease = allocator.allocate("plugin-a").expect("allocate");
        assert!(allocator.range().contains(lease.port()));
        assert_eq!(allocator.owner_of(lease.port()).as_deref(), Some("plugin-a"));
        assert_eq!(allocator.refs(lease.port()), 1);
        assert_eq!(check_port_available(lease.port()), Ok(()));
    }

    #[test]
    fn test_two_plugins_never_share_a_port() {
        let allocator = small_allocator();
        let a = allocator.allocate("plugin-a").expect("a");
        let b = allocator.allocate("plugin-b").expect("b");
        assert_ne!(a.port(), b.port());
        assert_eq!(allocator.reserved_count(), 2);
    }

    #[test]
    fn test_drop_releases_the_port() {
        let allocator = small_allocator();
        let port = {
            let lease = allocator.allocate("plugin-a").expect("allocate");
            lease.port()
        };
        assert_eq!(allocator.owner_of(port), None);
        // The same port is reservable again immediately.
        let again = allocator
            .allocate_specific("plugin-b", port)
            .expect("re-reserve released port");
        assert_eq!(again.port(), port);
    }

    #[test]
    fn test_explicit_release_is_idempotent_with_drop() {
        let allocator = small_allocator();
        let lease = allocator.allocate("plugin-a").expect("allocate");
        let port = lease.port();
        lease.release();
        assert_eq!(allocator.reserved_count(), 0);
        assert_eq!(allocator.owner_of(port), None);
    }

    #[test]
    fn test_specific_port_rejects_another_plugins_reservation() {
        let allocator = small_allocator();
        let _a = allocator.allocate_specific("plugin-a", 52001).expect("a");
        let err = allocator
            .allocate_specific("plugin-b", 52001)
            .expect_err("must be refused");
        match err {
            PortAllocatorError::AlreadyReserved { port, by, .. } => {
                assert_eq!(port, 52001);
                assert_eq!(by, "plugin-a");
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn test_specific_port_is_idempotent_for_the_same_plugin() {
        let allocator = small_allocator();
        let first = allocator.allocate_specific("plugin-a", 52002).expect("first");
        let second = allocator
            .allocate_specific("plugin-a", 52002)
            .expect("same plugin, same port");
        assert_eq!(first.port(), second.port());
        assert_eq!(allocator.reserved_count(), 1);
        assert_eq!(allocator.refs(52002), 2);
    }

    #[test]
    fn test_dropping_one_of_two_leases_keeps_the_port_reserved() {
        let allocator = small_allocator();
        let first = allocator.allocate_specific("plugin-a", 52003).expect("first");
        let second = allocator
            .allocate_specific("plugin-a", 52003)
            .expect("second lease");

        // A live instance still holds the port: dropping the other lease must not free it.
        drop(first);
        assert_eq!(allocator.owner_of(52003).as_deref(), Some("plugin-a"));
        assert_eq!(allocator.refs(52003), 1);
        assert_eq!(allocator.reserved_count(), 1);

        // While the second instance is alive no other plugin may take the port.
        let err = allocator
            .allocate_specific("plugin-b", 52003)
            .expect_err("port is still leased");
        assert!(matches!(err, PortAllocatorError::AlreadyReserved { .. }));

        drop(second);
        assert_eq!(allocator.owner_of(52003), None);
        assert_eq!(allocator.reserved_count(), 0);
    }

    #[test]
    fn test_specific_port_reports_addr_in_use_evidence() {
        let allocator = small_allocator();
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind ephemeral");
        let busy = listener.local_addr().expect("addr").port();
        let err = allocator
            .allocate_specific("plugin-a", busy)
            .expect_err("busy port must be refused");
        assert!(matches!(err, PortAllocatorError::AddressInUse { port, .. } if port == busy));
        drop(listener);
    }

    #[test]
    fn test_range_exhaustion_is_reported() {
        let allocator = PortAllocator::with_range(PortRange::new(52100, 52101).expect("range"));
        let _a = allocator.allocate("plugin-a").expect("a");
        let _b = allocator.allocate("plugin-b").expect("b");
        let err = allocator.allocate("plugin-c").expect_err("range is full");
        assert!(matches!(
            err,
            PortAllocatorError::RangeExhausted { .. }
        ));
    }

    #[test]
    fn test_excluded_ports_are_never_handed_out() {
        let allocator = PortAllocator::with_range(PortRange::new(52200, 52202).expect("range"));
        // Blacklist two of the three ports: the allocator must land on the third.
        let lease = allocator
            .allocate_excluding("plugin-a", &[52200, 52201])
            .expect("allocate outside the blacklist");
        assert_eq!(lease.port(), 52202);
    }

    #[test]
    fn test_blank_plugin_id_is_refused() {
        let allocator = small_allocator();
        assert_eq!(
            allocator.allocate("   ").unwrap_err(),
            PortAllocatorError::EmptyPluginId
        );
    }

    #[test]
    fn test_stats_lists_reservations_sorted_by_port() {
        let allocator = small_allocator();
        let _a = allocator.allocate_specific("plugin-a", 52004).expect("a");
        let _b = allocator.allocate_specific("plugin-b", 52000).expect("b");
        let stats = allocator.stats();
        assert_eq!(stats.reserved, 2);
        assert_eq!(stats.available, 3);
        assert_eq!(stats.reservations[0].port, 52000);
        assert_eq!(stats.reservations[1].port, 52004);
    }

    #[test]
    fn test_clone_shares_one_ledger() {
        let allocator = small_allocator();
        let handle = allocator.clone();
        let lease = allocator.allocate("plugin-a").expect("allocate");
        assert_eq!(handle.owner_of(lease.port()).as_deref(), Some("plugin-a"));
        assert_eq!(handle.reserved_count(), 1);
    }

    #[test]
    fn test_twenty_parallel_allocations_stay_unique() {
        let allocator = PortAllocator::with_range(PortRange::new(53000, 53099).expect("range"));
        let handles: Vec<_> = (0..20)
            .map(|i| {
                let allocator = allocator.clone();
                std::thread::spawn(move || allocator.allocate(&format!("plugin-{i}")))
            })
            .collect();
        let leases: Vec<_> = handles
            .into_iter()
            .map(|h| h.join().expect("join").expect("allocate"))
            .collect();
        let mut ports: Vec<u16> = leases.iter().map(|l| l.port()).collect();
        ports.sort_unstable();
        ports.dedup();
        assert_eq!(ports.len(), 20);
        assert_eq!(allocator.reserved_count(), 20);
    }
}
