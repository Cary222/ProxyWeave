#![cfg(windows)]
use proxyweave_gateway::{Gateway, GatewayHandle};
use proxyweave_plugin::{Manifest, PluginRecord, RuntimeError, RuntimeManager, RuntimeStatus};
use std::{
    io::{Read, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
const LOCAL_KEY: &str = "local-test-key-not-an-upstream-secret-123456";
const UPSTREAM_KEY: &str = "upstream-test-key-must-not-be-local-key";
const MODELS: &str =
    "{ \"data\": [ {\"id\":\"actual-fixture-model\",\"vendor_extension\":true} ] }\n";

// Real Owned child test harness, the same mature fixture technique used by F2.
#[test]
#[ignore = "invoked by RuntimeManager as a real Managed child"]
fn gateway_fixture_child() {
    let port = std::env::var("FIXTURE_PORT").unwrap();
    let root = Arc::new(PathBuf::from(std::env::var("FIXTURE_ROOT").unwrap()));
    std::fs::write(root.join("pid"), std::process::id().to_string()).unwrap();
    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).unwrap();
    for connection in listener.incoming() {
        let root = root.clone();
        std::thread::spawn(move || serve_fixture(connection.unwrap(), &root));
    }
}
fn serve_fixture(mut stream: std::net::TcpStream, root: &Path) {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut b = [0; 4096];
    let header_end = loop {
        match stream.read(&mut b) {
            Ok(0) | Err(_) => return,
            Ok(n) => bytes.extend_from_slice(&b[..n]),
        }
        if let Some(i) = bytes.windows(4).position(|s| s == b"\r\n\r\n") {
            break i + 4;
        }
        assert!(bytes.len() < 65536);
    };
    let headers = String::from_utf8_lossy(&bytes[..header_end]).to_string();
    let length = headers
        .lines()
        .find_map(|line| {
            line.split_once(':')
                .filter(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                .map(|(_, v)| v.trim().parse::<usize>().unwrap())
        })
        .unwrap_or(0);
    while bytes.len() < header_end + length {
        match stream.read(&mut b) {
            Ok(0) | Err(_) => return,
            Ok(n) => bytes.extend_from_slice(&b[..n]),
        }
    }
    let path = headers
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap();
    if path == "/healthz" {
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nX-Fixture: gateway-fixture\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok");
        return;
    }
    let mode = headers
        .lines()
        .find_map(|l| {
            l.split_once(':')
                .filter(|(k, _)| k.eq_ignore_ascii_case("x-test-mode"))
                .map(|(_, v)| v.trim())
        })
        .unwrap_or("normal");
    std::fs::write(root.join(format!("request-{mode}.bin")), &bytes).unwrap();
    if mode == "headerwait" {
        std::fs::write(root.join("entered-headerwait"), b"yes").unwrap();
        // Native upstream socket sees EOF, not a handler task-drop assertion.
        if matches!(stream.read(&mut b), Ok(0)) {
            std::fs::write(root.join("closed-headerwait"), b"yes").unwrap();
        }
        return;
    }
    if mode == "timeout" {
        std::thread::sleep(Duration::from_secs(2));
        return;
    }
    if mode == "endless" {
        let _ = stream.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n",
        );
        std::fs::write(root.join("entered-endless"), b"yes").unwrap();
        loop {
            if stream
                .write_all(b"data: {\"delta\":\"streaming\"}\n\n")
                .is_err()
            {
                std::fs::write(root.join("closed-endless"), b"yes").unwrap();
                return;
            }
            std::thread::sleep(Duration::from_millis(30));
        }
    }
    let (status, body, content_type) = if mode == "sse" {
        (
            200,
            "data: {\"delta\":\"中🌍\"}\r\n\r\ndata: [DONE]\r\n\r\n".to_string(),
            "text/event-stream; charset=utf-8",
        )
    } else if mode == "truncated" || mode == "eof" {
        (
            200,
            "data: {\"delta\":\"unfinished".to_string(),
            "text/event-stream",
        )
    } else if mode == "error" {
        (
            429,
            "{\"error\":{\"code\":\"upstream_rate_limit\"}}".to_string(),
            "application/problem+json",
        )
    } else if path.starts_with("/actual/models") {
        (200, MODELS.to_string(), "application/json")
    } else {
        (
            200,
            " {\"result\":\"nonstream\", \"keep\": [1,2]}\n".to_string(),
            "application/json",
        )
    };
    let length = if mode == "truncated" {
        body.len() + 100
    } else {
        body.len()
    };
    let _ = write!(stream, "HTTP/1.1 {status} Fixture\r\nContent-Type: {content_type}\r\nContent-Length: {length}\r\nConnection: close, x-drop\r\nX-Drop: must-not-forward\r\nX-End-To-End: preserved\r\n\r\n");
    if mode == "sse" {
        // Split UTF-8, CRLF and SSE event boundaries. The proxy must not decode/re-encode.
        for chunk in body.as_bytes().chunks(3) {
            if stream.write_all(chunk).is_err() {
                return;
            }
            std::thread::sleep(Duration::from_millis(3));
        }
    } else {
        let _ = stream.write_all(body.as_bytes());
    }
}

struct Fixture {
    root: tempfile::TempDir,
    runtime: RuntimeManager,
    record: PluginRecord,
    pid: u32,
    gateway: Gateway,
    handle: GatewayHandle,
}
impl Fixture {
    async fn new(header_timeout: Duration) -> Self {
        use sha2::{Digest, Sha256};
        let root = tempfile::tempdir().unwrap();
        let exe = std::env::current_exe().unwrap();
        std::fs::copy(&exe, root.path().join("fixture.exe")).unwrap();
        let hash = format!("{:x}", Sha256::digest(std::fs::read(exe).unwrap()));
        let yaml = format!(
            r#"schema_version: '1.0'
metadata: {{id: gateway-fixture, name: Real Managed gateway fixture, version: 1.0.0, license: MIT}}
runtime:
  ownership: managed
  executable: fixture.exe
  sha256: {hash}
  args: ['--exact', gateway_fixture_child, '--ignored', '--nocapture']
  env: {{FIXTURE_PORT: '{{PORT}}', FIXTURE_ROOT: '{{DATA_DIR}}'}}
network:
  health_check: {{path: /healthz, expected_status: 200, identity_header: X-Fixture, identity_value: gateway-fixture, timeout_ms: 300}}
lifecycle: {{startup_timeout_ms: 2000, idle_timeout_ms: null, max_port_retries: 3}}
capabilities: {{protocol: openai_chat, models_path: /actual/models, inference_path: /actual/chat, streaming: true}}
configuration: {{config_template: {{}}}}
"#
        );
        let record = PluginRecord {
            manifest: Manifest::parse(yaml.as_bytes()).unwrap(),
            fingerprint: "fixture".into(),
            settings: Default::default(),
        };
        let runtime = RuntimeManager::new();
        let status = runtime
            .start_plugin(
                &record,
                root.path(),
                Some(proxyweave_plugin::port::pick_unused_port().unwrap()),
                root.path(),
                None,
            )
            .await
            .unwrap();
        let RuntimeStatus::Ready { pid: Some(pid), .. } = status else {
            panic!("Not Ready {status:?}");
        };
        let gateway = Gateway::new(runtime.clone(), LOCAL_KEY, header_timeout).unwrap();
        gateway
            .set_backend(&record.manifest, UPSTREAM_KEY)
            .await
            .unwrap();
        let handle = gateway.listen(0).await.unwrap();
        Self {
            root,
            runtime,
            record,
            pid,
            gateway,
            handle,
        }
    }
    fn data(&self) -> PathBuf {
        self.root.path().join("plugins/gateway-fixture")
    }
    fn base(&self) -> String {
        format!("http://{}", self.handle.address())
    }
    fn client(&self) -> reqwest::Client {
        reqwest::Client::builder().no_proxy().build().unwrap()
    }
    async fn wait_count(&self, n: usize) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.runtime.active_request_count("gateway-fixture").await != n {
            assert!(Instant::now() < deadline, "active count did not become {n}");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
    async fn request(&self, mode: &str) -> reqwest::Response {
        self.client()
            .post(format!("{}/v1/chat/completions", self.base()))
            .bearer_auth(LOCAL_KEY)
            .header("x-test-mode", mode)
            .body("{ \"model\": \"actual-fixture-model\", \"vendor\": [true,1] }")
            .send()
            .await
            .unwrap()
    }
    async fn finish(self) {
        self.handle.shutdown().await.unwrap();
        self.runtime.stop_plugin("gateway-fixture").await.unwrap();
        wait_exit(self.pid).await;
    }
}
fn alive(pid: u32) -> bool {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE},
    };
    unsafe {
        let p = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if p.is_null() {
            return false;
        }
        let alive = WaitForSingleObject(p, 0) == 258;
        CloseHandle(p);
        alive
    }
}
async fn wait_exit(pid: u32) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while alive(pid) {
        assert!(Instant::now() < deadline, "Owned PID alive");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
async fn wait_file(p: &Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !p.exists() {
        assert!(
            Instant::now() < deadline,
            "Missing fixture event {}",
            p.display()
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
async fn raw_request(f: &Fixture, mode: &str) -> tokio::net::TcpStream {
    let mut stream = tokio::net::TcpStream::connect(f.handle.address())
        .await
        .unwrap();
    stream.write_all(format!("POST /v1/chat/completions HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {LOCAL_KEY}\r\nX-Test-Mode: {mode}\r\nContent-Length: 2\r\n\r\n{{}}").as_bytes()).await.unwrap();
    stream
}

#[tokio::test]
async fn auth_models_nonstream_headers_error_passthrough() {
    let f = Fixture::new(Duration::from_secs(10)).await;
    let client = f.client();
    for key in [None, Some("wrong-local-secret")] {
        let mut req = client.get(format!("{}/v1/models", f.base()));
        if let Some(key) = key {
            req = req.bearer_auth(key);
        }
        assert_eq!(req.send().await.unwrap().status(), 401);
    }
    assert!(!f.data().join("request-normal.bin").exists());
    let mut duplicate = tokio::net::TcpStream::connect(f.handle.address())
        .await
        .unwrap();
    duplicate.write_all(format!("GET /v1/models HTTP/1.1\r\nHost: local\r\nAuthorization: Bearer {LOCAL_KEY}\r\nAuthorization: Bearer {LOCAL_KEY}\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
    let mut duplicate_response = Vec::new();
    duplicate
        .read_to_end(&mut duplicate_response)
        .await
        .unwrap();
    assert!(String::from_utf8_lossy(&duplicate_response).starts_with("HTTP/1.1 401"));
    let res = client
        .get(format!("{}/v1/models?keep=%2F", f.base()))
        .bearer_auth(LOCAL_KEY)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.headers()["x-end-to-end"], "preserved");
    assert!(!res.headers().contains_key("x-drop"));
    assert_eq!(res.bytes().await.unwrap().as_ref(), MODELS.as_bytes());
    let model_request = std::fs::read(f.data().join("request-normal.bin")).unwrap();
    assert!(String::from_utf8_lossy(&model_request).starts_with("GET /actual/models?keep=%2F "));
    let body = "{\n \"model\":\"actual-fixture-model\", \"stream\":false, \"unknown_vendor\":{\"never_strip\":true}\n}";
    let res = client
        .post(format!("{}/v1/chat/completions", f.base()))
        .bearer_auth(LOCAL_KEY)
        .header("connection", "x-untrusted, authorization")
        .header("x-untrusted", "remove-me")
        .header("cookie", "private-cookie-marker")
        .header("proxy-authorization", "private-proxy-marker")
        .body(body)
        .send()
        .await
        .unwrap();
    assert_eq!(
        res.bytes().await.unwrap().as_ref(),
        b" {\"result\":\"nonstream\", \"keep\": [1,2]}\n"
    );
    let captured = std::fs::read(f.data().join("request-normal.bin")).unwrap();
    let text = String::from_utf8_lossy(&captured);
    assert!(text.contains(UPSTREAM_KEY));
    assert!(!text.contains(LOCAL_KEY));
    assert!(
        !text.contains("remove-me")
            && !text.contains("private-cookie-marker")
            && !text.contains("private-proxy-marker")
    );
    assert_eq!(
        captured
            .split_at(captured.windows(4).position(|b| b == b"\r\n\r\n").unwrap() + 4)
            .1,
        body.as_bytes()
    );
    let error = f.request("error").await;
    assert_eq!(error.status(), 429);
    assert_eq!(error.headers()["content-type"], "application/problem+json");
    assert_eq!(
        error.bytes().await.unwrap().as_ref(),
        b"{\"error\":{\"code\":\"upstream_rate_limit\"}}"
    );
    f.wait_count(0).await;
    f.finish().await;
}

#[tokio::test]
async fn streaming_bytes_truncation_and_plain_eof_never_synthesize_done() {
    use futures_util::StreamExt;
    let f = Fixture::new(Duration::from_secs(10)).await;
    let normal = f.request("sse").await;
    assert_eq!(
        normal.headers()["content-type"],
        "text/event-stream; charset=utf-8"
    );
    assert_eq!(
        normal.bytes().await.unwrap().as_ref(),
        "data: {\"delta\":\"中🌍\"}\r\n\r\ndata: [DONE]\r\n\r\n".as_bytes()
    );
    let mut stream = f.request("truncated").await.bytes_stream();
    let mut bytes = Vec::new();
    let mut failed = false;
    while let Some(frame) = stream.next().await {
        match frame {
            Ok(b) => bytes.extend_from_slice(&b),
            Err(_) => {
                failed = true;
                break;
            }
        }
    }
    assert!(failed, "transport truncation must stay a body error");
    assert!(!String::from_utf8_lossy(&bytes).contains("[DONE]"));
    let eof = f.request("eof").await.bytes().await.unwrap();
    assert_eq!(eof.as_ref(), b"data: {\"delta\":\"unfinished");
    assert!(!String::from_utf8_lossy(&eof).contains("[DONE]"));
    f.wait_count(0).await;
    f.finish().await;
}

#[tokio::test]
async fn native_disconnect_during_header_wait_cancels_upstream_and_releases_lease() {
    let f = Fixture::new(Duration::from_secs(20)).await;
    let stream = raw_request(&f, "headerwait").await;
    wait_file(&f.data().join("entered-headerwait")).await;
    f.wait_count(1).await;
    assert!(matches!(
        f.runtime.stop_plugin("gateway-fixture").await,
        Err(RuntimeError::ActiveRequests)
    ));
    assert!(alive(f.pid));
    drop(stream);
    wait_file(&f.data().join("closed-headerwait")).await;
    f.wait_count(0).await;
    f.finish().await;
}

#[tokio::test]
async fn native_disconnect_during_stream_cancels_upstream_and_releases_lease() {
    let f = Fixture::new(Duration::from_secs(20)).await;
    let mut stream = raw_request(&f, "endless").await;
    let mut bytes = [0; 4096];
    let n = stream.read(&mut bytes).await.unwrap();
    assert!(n > 0);
    wait_file(&f.data().join("entered-endless")).await;
    f.wait_count(1).await;
    assert!(matches!(
        f.runtime.stop_plugin("gateway-fixture").await,
        Err(RuntimeError::ActiveRequests)
    ));
    drop(stream);
    wait_file(&f.data().join("closed-endless")).await;
    f.wait_count(0).await;
    f.finish().await;
}

#[tokio::test]
async fn header_timeout_port_conflict_backend_unavailable_and_gateway_shutdown() {
    let f = Fixture::new(Duration::from_millis(200)).await;
    let occupied = TcpListener::bind("127.0.0.1:0").unwrap();
    assert_eq!(
        f.gateway
            .listen(occupied.local_addr().unwrap().port())
            .await
            .err()
            .unwrap()
            .kind(),
        std::io::ErrorKind::AddrInUse
    );
    assert_eq!(f.request("timeout").await.status(), 504);
    f.wait_count(0).await;
    let mut stream = raw_request(&f, "endless").await;
    let mut bytes = [0; 4096];
    assert!(stream.read(&mut bytes).await.unwrap() > 0);
    f.wait_count(1).await;
    let Fixture {
        handle,
        runtime,
        pid,
        root,
        ..
    } = f;
    tokio::time::timeout(Duration::from_secs(5), handle.shutdown())
        .await
        .unwrap()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while runtime.active_request_count("gateway-fixture").await != 0 {
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    runtime.stop_plugin("gateway-fixture").await.unwrap();
    wait_exit(pid).await;
    drop(root);
}

#[tokio::test]
async fn concurrent_leases_block_stop_and_release_only_after_last_drop() {
    let f = Fixture::new(Duration::from_secs(10)).await;
    let (a, b) = tokio::join!(
        f.runtime.acquire_request("gateway-fixture"),
        f.runtime.acquire_request("gateway-fixture")
    );
    let a = a.unwrap();
    let b = b.unwrap();
    f.wait_count(2).await;
    drop(a);
    f.wait_count(1).await;
    assert!(matches!(
        f.runtime.stop_plugin("gateway-fixture").await,
        Err(RuntimeError::ActiveRequests)
    ));
    drop(b);
    f.wait_count(0).await;
    f.runtime.stop_plugin("gateway-fixture").await.unwrap();
    wait_exit(f.pid).await;
    assert_eq!(f.request("normal").await.status(), 503);
    f.gateway.clear_backend().await;
    assert_eq!(f.request("normal").await.status(), 503);
    // Reuse after stop, dynamic new port is read from lease.
    let status = f
        .runtime
        .start_plugin(
            &f.record,
            f.root.path(),
            Some(proxyweave_plugin::port::pick_unused_port().unwrap()),
            f.root.path(),
            None,
        )
        .await
        .unwrap();
    let RuntimeStatus::Ready { pid: Some(pid), .. } = status else {
        panic!("{status:?}");
    };
    f.gateway
        .set_backend(&f.record.manifest, UPSTREAM_KEY)
        .await
        .unwrap();
    assert_eq!(f.request("normal").await.status(), 200);
    f.handle.shutdown().await.unwrap();
    f.runtime.stop_plugin("gateway-fixture").await.unwrap();
    wait_exit(pid).await;
}

#[tokio::test]
async fn external_gateway_cancel_never_controls_provider_lifecycle() {
    use axum::{routing::get, Router};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let provider = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new()
                .route(
                    "/healthz",
                    get(|| async { ([("X-Fixture", "external-fixture")], "ok") }),
                )
                .route("/actual/models", get(|| async { MODELS })),
        )
        .await
        .unwrap();
    });
    let manifest = Manifest::parse(format!(r#"schema_version: '1.0'
metadata: {{id: external-fixture, name: Externally owned fixture, version: 1.0.0, license: MIT}}
runtime: {{ownership: external, endpoint: 'http://{addr}'}}
network:
  health_check: {{path: /healthz, expected_status: 200, identity_header: X-Fixture, identity_value: external-fixture, timeout_ms: 1000}}
lifecycle: {{startup_timeout_ms: 1000, idle_timeout_ms: null, max_port_retries: 0}}
capabilities: {{protocol: openai_chat, models_path: /actual/models, inference_path: /actual/chat, streaming: true}}
"#).as_bytes()).unwrap();
    let runtime = RuntimeManager::new();
    let record = PluginRecord {
        manifest: manifest.clone(),
        fingerprint: "external".into(),
        settings: Default::default(),
    };
    let temp = tempfile::tempdir().unwrap();
    assert!(matches!(
        runtime
            .start_plugin(&record, temp.path(), None, temp.path(), None)
            .await
            .unwrap(),
        RuntimeStatus::Ready { pid: None, .. }
    ));
    let gateway = Gateway::new(runtime.clone(), LOCAL_KEY, Duration::from_secs(5)).unwrap();
    gateway.set_backend(&manifest, UPSTREAM_KEY).await.unwrap();
    let handle = gateway.listen(0).await.unwrap();
    let lease = runtime.acquire_request("external-fixture").await.unwrap();
    assert!(matches!(
        runtime.stop_plugin("external-fixture").await,
        Err(RuntimeError::ExternalServiceProtected(_))
    ));
    drop(lease);
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let res = client
        .get(format!("http://{}/v1/models", handle.address()))
        .bearer_auth(LOCAL_KEY)
        .send()
        .await
        .unwrap();
    assert_eq!(res.bytes().await.unwrap().as_ref(), MODELS.as_bytes());
    handle.shutdown().await.unwrap();
    assert!(matches!(
        runtime.stop_plugin("external-fixture").await,
        Err(RuntimeError::ExternalServiceProtected(_))
    ));
    assert!(!provider.is_finished());
    assert_eq!(
        client
            .get(format!("http://{addr}/healthz"))
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
    // Only test owner cleans this fixture; RuntimeManager/Gateway never touches it.
    provider.abort();
    let _ = provider.await;
}
