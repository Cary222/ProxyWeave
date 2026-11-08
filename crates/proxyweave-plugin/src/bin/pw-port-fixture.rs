//! Test fixture: a Managed plugin stand-in that binds the dynamic port it is given and
//! answers the readiness probe.
//!
//! It exists so the spec v10 port tests can exercise the real runtime path (spawn → probe →
//! stop → restart) instead of a mock. Two behaviours are configurable through the
//! Manifest's rendered environment:
//!
//! - `FIXTURE_PORT` (required): the port to bind, rendered from `{{PORT}}`.
//! - `FIXTURE_CONFLICT_ATTEMPTS` (optional): how many of the first attempts must fail with
//!   real `AddrInUse` evidence. The fixture produces that evidence the only honest way —
//!   it holds the port and then asks the kernel for it a second time, printing the OS error.
//! - `FIXTURE_STATE` (required with `FIXTURE_CONFLICT_ATTEMPTS`): a file that counts attempts
//!   across processes, so "conflict once, then serve" is deterministic.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;

const IDENTITY_HEADER: &str = "X-Service";
const IDENTITY_VALUE: &str = "port-fixture";

fn main() {
    let port: u16 = std::env::var("FIXTURE_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .expect("FIXTURE_PORT must be a port number");

    let conflict_attempts: u32 = std::env::var("FIXTURE_CONFLICT_ATTEMPTS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);

    if conflict_attempts > 0 {
        let state = std::env::var("FIXTURE_STATE").expect("FIXTURE_STATE is required");
        let attempt = next_attempt(Path::new(&state));
        if attempt <= conflict_attempts {
            // Hold the port, then request it again: the kernel, not the test, produces the
            // evidence (EADDRINUSE / WSAEADDRINUSE).
            let held = match TcpListener::bind(("127.0.0.1", port)) {
                Ok(listener) => listener,
                Err(err) => {
                    eprintln!("fixture: cannot bind 127.0.0.1:{port}: {err}");
                    std::process::exit(1);
                }
            };
            match TcpListener::bind(("127.0.0.1", port)) {
                Ok(_) => {
                    eprintln!("fixture: second bind on 127.0.0.1:{port} unexpectedly succeeded");
                    std::process::exit(2);
                }
                Err(err) => {
                    eprintln!("fixture: cannot bind 127.0.0.1:{port}: {err}");
                    drop(held);
                    std::process::exit(1);
                }
            }
        }
    }

    let listener = match TcpListener::bind(("127.0.0.1", port)) {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("fixture: cannot bind 127.0.0.1:{port}: {err}");
            std::process::exit(1);
        }
    };
    println!("fixture: listening on 127.0.0.1:{port}");
    for stream in listener.incoming().flatten() {
        let _ = serve(stream);
    }
}

fn next_attempt(state: &Path) -> u32 {
    let current = std::fs::read_to_string(state)
        .ok()
        .and_then(|text| text.trim().parse::<u32>().ok())
        .unwrap_or(0);
    let next = current.saturating_add(1);
    let _ = std::fs::write(state, next.to_string());
    next
}

fn serve(mut stream: TcpStream) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    while reader.read_line(&mut line)? > 0 {
        if line == "\r\n" || line == "\n" {
            break;
        }
        line.clear();
    }

    let body = "{\"service\":\"port-fixture\"}";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{}: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        IDENTITY_HEADER,
        IDENTITY_VALUE,
        body
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    let _ = stream.shutdown(std::net::Shutdown::Both);
    Ok(())
}
