//! Spec v10 acceptance suite: dynamic port allocation for Managed plugins, port reclamation,
//! and the gateway route table following the live runtime generation.
//!
//! Every test here is black-box: public API only, no mocks of the allocator or route pool.

use std::path::{Path, PathBuf};
use std::time::Duration;

use axum::http::HeaderValue;
use proxyweave_gateway::routing::Backend;
use proxyweave_gateway::Gateway;
use proxyweave_plugin::port_allocator::{
    PortAllocator, PortAllocatorError, PortRange, DEFAULT_PORT_RANGE_END, DEFAULT_PORT_RANGE_START,
};
use proxyweave_plugin::{RuntimeManager, RuntimeStatus};

const LOCAL_KEY: &str = "spec-v10-local-key-0123456789abcdef";

fn window(start: u16, end: u16) -> PortAllocator {
    PortAllocator::with_range(PortRange::new(start, end).expect("valid range"))
}

fn backend(plugin_id: &str, protocol: &str, generation: Option<u64>) -> Backend {
    Backend::new(
        plugin_id.to_string(),
        protocol.to_string(),
        "/v1/models".to_string(),
        "/v1/messages".to_string(),
        HeaderValue::from_static("Bearer backend-key"),
    )
    .with_instance_generation(generation)
}

fn gateway_with(runtime: RuntimeManager) -> Gateway {
    Gateway::new(runtime, LOCAL_KEY, Duration::from_secs(5)).expect("gateway")
}

// ---------------------------------------------------------------------------
// 1. Allocation: distinct ports, inside the documented range
// ---------------------------------------------------------------------------

#[test]
fn managed_plugins_receive_distinct_dynamic_ports() {
    let allocator = PortAllocator::new();
    let mut leases = Vec::new();
    for i in 0..8 {
        leases.push(
            allocator
                .allocate(&format!("plugin-{i}"))
                .expect("allocation succeeds"),
        );
    }

    let mut ports: Vec<u16> = leases.iter().map(|lease| lease.port()).collect();
    ports.sort_unstable();
    ports.dedup();

    assert_eq!(ports.len(), 8, "every Managed plugin gets its own port");
    for port in ports {
        assert!(
            (DEFAULT_PORT_RANGE_START..=DEFAULT_PORT_RANGE_END).contains(&port),
            "port {port} must come from the documented dynamic range"
        );
    }
}

#[test]
fn explicit_port_is_honoured_and_proven_free_before_use() {
    let allocator = window(54000, 54009);
    let lease = allocator
        .allocate_specific("pinned-plugin", 54003)
        .expect("explicit port");
    assert_eq!(lease.port(), 54003);
    assert_eq!(allocator.reserved_count(), 1);
}

// ---------------------------------------------------------------------------
// 2. Conflict prevention
// ---------------------------------------------------------------------------

#[test]
fn two_plugins_cannot_be_handed_the_same_port() {
    let allocator = window(54100, 54109);
    let _first = allocator
        .allocate_specific("plugin-a", 54101)
        .expect("first owner");

    let err = allocator
        .allocate_specific("plugin-b", 54101)
        .expect_err("second owner must be refused");
    match err {
        PortAllocatorError::AlreadyReserved { port, by, .. } => {
            assert_eq!(port, 54101);
            assert_eq!(by, "plugin-a");
        }
        other => panic!("expected AlreadyReserved, got {other}"),
    }
}

#[test]
fn a_port_held_by_another_process_is_never_handed_out() {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind ephemeral");
    let busy = listener.local_addr().expect("addr").port();
    let allocator = PortAllocator::new();

    let err = allocator
        .allocate_specific("plugin-a", busy)
        .expect_err("externally bound port must be refused");
    assert!(
        matches!(err, PortAllocatorError::AddressInUse { port, .. } if port == busy),
        "conflict must be reported from bind evidence, not guessed: {err}"
    );
    drop(listener);
}

#[test]
fn a_full_window_reports_exhaustion_instead_of_reusing_a_port() {
    let allocator = window(54200, 54201);
    let _a = allocator.allocate("plugin-a").expect("a");
    let _b = allocator.allocate("plugin-b").expect("b");

    let err = allocator.allocate("plugin-c").expect_err("window is full");
    assert!(
        matches!(err, PortAllocatorError::RangeExhausted { .. }),
        "expected RangeExhausted, got {err}"
    );
}

// ---------------------------------------------------------------------------
// 3. Reclamation: restart, stop and failed spawn all return the port
// ---------------------------------------------------------------------------

#[test]
fn a_stopped_instance_returns_its_port_for_the_next_start() {
    let allocator = window(54300, 54309);

    // First run of the plugin.
    let port = {
        let lease = allocator.allocate("plugin-a").expect("first start");
        lease.port()
    };
    assert_eq!(allocator.owner_of(port), None, "stop releases the port");
    assert_eq!(allocator.reserved_count(), 0);

    // Restart after the port was released: the same port is usable again.
    let restarted = allocator
        .allocate_specific("plugin-a", port)
        .expect("restart may re-use the reclaimed port");
    assert_eq!(restarted.port(), port);
}

#[test]
fn a_dropped_instance_record_reclaims_the_port() {
    struct FakeInstance {
        _lease: proxyweave_plugin::PortLease,
    }

    let allocator = window(54400, 54409);
    {
        let lease = allocator.allocate("crashing-plugin").expect("allocate");
        let _instance = FakeInstance { _lease: lease };
        assert_eq!(allocator.reserved_count(), 1);
    }
    // Simulates a panic/abort path: the ledger must not leak the reservation.
    assert_eq!(allocator.reserved_count(), 0);
    assert!(allocator.allocate("crashing-plugin").is_ok());
}

#[test]
fn reservations_are_visible_to_the_workbench_view() {
    let allocator = window(54500, 54509);
    let _a = allocator.allocate_specific("plugin-a", 54502).expect("a");
    let _b = allocator.allocate_specific("plugin-b", 54507).expect("b");

    let stats = allocator.stats();
    assert_eq!(stats.reserved, 2);
    assert_eq!(stats.reservations[0].port, 54502);
    assert_eq!(stats.reservations[0].plugin_id, "plugin-a");
    assert_eq!(stats.reservations[1].port, 54507);

    let json = serde_json::to_string(&stats).expect("stats serialize for the UI");
    assert!(json.contains("\"reserved\":2"));
}

#[test]
fn the_runtime_manager_shares_one_ledger_with_its_callers() {
    let runtime = RuntimeManager::new();
    let handle = runtime.port_allocator();
    let lease = handle.allocate("managed-plugin").expect("allocate");

    assert_eq!(
        runtime.port_allocator().owner_of(lease.port()).as_deref(),
        Some("managed-plugin")
    );
    assert_eq!(runtime.port_allocator().stats().reserved, 1);
}

// ---------------------------------------------------------------------------
// 4. Gateway: the route table follows the live instance generation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn route_snapshot_reports_a_stopped_instance_as_unbound() {
    let gateway = gateway_with(RuntimeManager::new());
    gateway
        .route_pool()
        .add_backend(backend("ghost-plugin", "anthropic", Some(4)))
        .await;

    let snapshot = gateway.route_snapshot().await;
    assert_eq!(snapshot.len(), 1);
    let entry = &snapshot[0];
    assert_eq!(entry.plugin_id, "ghost-plugin");
    assert_eq!(entry.port, None, "no live instance means no port");
    assert_eq!(entry.endpoint, None);
    assert!(!entry.ready);
    assert_eq!(entry.state, "stopped");
    assert_eq!(entry.route_generation, Some(4));
    assert_eq!(entry.runtime_generation, None);
}

#[tokio::test]
async fn refresh_route_never_blanks_a_generation_it_cannot_confirm() {
    let gateway = gateway_with(RuntimeManager::new());
    gateway
        .route_pool()
        .add_backend(backend("drifted-plugin", "openai", Some(1)))
        .await;

    // The runtime knows nothing about this plugin, so it cannot name a live generation. `None`
    // in a route entry means "any generation", so the binding must be left alone instead of
    // silently switching the stale-instance check off.
    let generation = gateway.refresh_route("drifted-plugin").await;
    assert_eq!(generation, None);

    let snapshot = gateway.route_snapshot().await;
    assert_eq!(snapshot[0].route_generation, Some(1));
}

#[tokio::test]
async fn admission_refreshes_the_route_once_and_retries_on_a_stale_generation() {
    let runtime = RuntimeManager::new();
    // The plugin restarted: the runtime is on generation 7 and port 23456.
    runtime
        .install_external_entry_for_test(
            "revived-plugin",
            23_456,
            "http://127.0.0.1:23456",
            7,
            RuntimeStatus::Ready {
                port: 23_456,
                pid: None,
            },
            true,
        )
        .await;
    let gateway = gateway_with(runtime);
    // The route table still holds the retired generation 1.
    let mut route = backend("revived-plugin", "openai", Some(1));
    gateway.route_pool().add_backend(route.clone()).await;

    let lease = gateway
        .acquire_lease_following_route(&mut route)
        .await
        .expect("admission follows the restarted instance");

    assert_eq!(
        lease.generation(),
        7,
        "the lease belongs to the live instance"
    );
    assert_eq!(route.instance_generation, Some(7), "the caller is re-bound");
    let snapshot = gateway.route_snapshot().await;
    assert_eq!(
        snapshot[0].route_generation,
        Some(7),
        "the route table kept up with the restart"
    );
}

#[tokio::test]
async fn admission_refuses_without_looping_when_no_instance_is_live() {
    let runtime = RuntimeManager::new();
    // The entry exists but nothing is serving: refresh cannot resolve a generation.
    runtime
        .install_external_entry_for_test(
            "gone-plugin",
            23_457,
            "http://127.0.0.1:23457",
            3,
            RuntimeStatus::Stopped,
            false,
        )
        .await;
    let gateway = gateway_with(runtime);
    let mut route = backend("gone-plugin", "openai", Some(1));
    gateway.route_pool().add_backend(route.clone()).await;

    let error = gateway
        .acquire_lease_following_route(&mut route)
        .await
        .expect_err("no live instance must be refused, not retried");
    let message = error.to_string();
    assert!(
        message.contains("no live instance"),
        "expected an explicit refusal, got {message}"
    );
    assert_eq!(
        route.instance_generation,
        Some(1),
        "a failed admission does not invent a generation"
    );
    let snapshot = gateway.route_snapshot().await;
    assert_eq!(snapshot[0].route_generation, Some(1));
}

#[tokio::test]
async fn a_saturated_instance_is_reported_but_never_ready() {
    let runtime = RuntimeManager::new();
    runtime
        .install_external_entry_for_test(
            "busy-plugin",
            23_458,
            "http://127.0.0.1:23458",
            2,
            RuntimeStatus::Busy {
                port: 23_458,
                pid: None,
                reason: "account pool unconfigured".to_string(),
            },
            true,
        )
        .await;
    let gateway = gateway_with(runtime);
    gateway
        .route_pool()
        .add_backend(backend("busy-plugin", "openai", Some(2)))
        .await;

    let snapshot = gateway.route_snapshot().await;
    assert_eq!(snapshot[0].port, Some(23_458), "the port is known");
    assert!(
        !snapshot[0].ready,
        "Plan.md §6.3 B: a saturated 503 is not Ready and never a conflict signal"
    );
    assert!(snapshot[0].state.starts_with("busy:"));
}

#[tokio::test]
async fn route_pool_rebinds_only_the_named_backend() {
    let gateway = gateway_with(RuntimeManager::new());
    let pool = gateway.route_pool();
    pool.add_backend(backend("plugin-a", "openai", Some(3)))
        .await;
    pool.add_backend(backend("plugin-b", "openai", Some(5)))
        .await;

    assert!(pool.update_generation("plugin-a", Some(9)).await);
    assert!(!pool.update_generation("unknown-plugin", Some(9)).await);

    let backends = pool.list_backends().await;
    let a = backends.iter().find(|b| b.plugin_id == "plugin-a").unwrap();
    let b = backends.iter().find(|b| b.plugin_id == "plugin-b").unwrap();
    assert_eq!(a.instance_generation, Some(9));
    assert_eq!(
        b.instance_generation,
        Some(5),
        "unrelated route is untouched"
    );
    assert!(a.matches_generation(9));
    assert!(
        !a.matches_generation(3),
        "the retired generation no longer matches"
    );
}

#[tokio::test]
async fn removing_a_backend_drops_its_route_entry() {
    let gateway = gateway_with(RuntimeManager::new());
    gateway
        .route_pool()
        .add_backend(backend("leaving-plugin", "openai", None))
        .await;
    assert_eq!(gateway.route_snapshot().await.len(), 1);

    gateway.remove_backend_target("leaving-plugin").await;
    assert!(gateway.route_snapshot().await.is_empty());
}

// ---------------------------------------------------------------------------
// 5. Red line: no adapter-specific hardcoding in the protocol/gateway layers
// ---------------------------------------------------------------------------

fn collect_rs_sources(root: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_sources(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// Strips a line's `//` comment while keeping `://` inside URLs intact.
fn code_of(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(pos) = rest.find("//") {
        let before = &rest[..pos];
        if before.ends_with(':') {
            out.push_str(before);
            out.push_str("//");
            rest = &rest[pos + 2..];
            continue;
        }
        out.push_str(before);
        return out;
    }
    out.push_str(rest);
    out
}

#[test]
fn adapter_and_host_layers_contain_no_plugin_specific_code() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let crates_dir = manifest_dir.parent().expect("crates dir").to_path_buf();

    // Adapter layer and host layer both: the red line is that no plugin name reaches code.
    let mut sources = Vec::new();
    for crate_name in ["proxyweave-protocols", "proxyweave-gateway"] {
        let root = crates_dir.join(crate_name).join("src");
        let mut found = Vec::new();
        collect_rs_sources(&root, &mut found);
        assert!(!found.is_empty(), "{crate_name} sources must be found");
        sources.extend(found);
    }

    // Comments and doc examples may name a plugin; what the red line forbids is plugin
    // specific *code*, so only the code part of each line is inspected.
    let forbidden = ["wb2api", "Wb2Api", "WB2API"];
    for path in sources {
        let text = std::fs::read_to_string(&path).expect("read source");
        for (index, line) in text.lines().enumerate() {
            let code = code_of(line);
            for needle in forbidden {
                assert!(
                    !code.contains(needle),
                    "plugin-specific literal '{needle}' in code at {}:{} (spec v10 forbids it)",
                    path.display(),
                    index + 1
                );
            }
        }
    }
}
