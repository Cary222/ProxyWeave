//! Test fixture: the Managed stand-in the host-owned credential vectors spawn.
//!
//! The host regression must run the real runtime path (import -> spawn -> probe -> stop)
//! against a package whose declared executable is a few hundred KiB. The host crate's own
//! debug test binary is hundreds of MiB, so it can neither be staged inside the 256 MiB
//! per-file package cap nor be written or hard-linked while Windows keeps it open.
//!
//! Behaviour comes from the Manifest's rendered environment, the same way a real Managed
//! plugin receives its port:
//!
//! - `OWNED_KEY_PORT` (required): rendered from `{{PORT}}`.
//! - `OWNED_KEY_MODE` (required): `busy` answers the readiness probe with the declared busy
//!   status and the expected identity, so the runtime reports `Busy`; any other value
//!   answers `200` with a foreign identity, which the runtime reports as `Failed`.
//!
//! It holds no credential and never reads one; the host owns the secret.

use std::io::{Read, Write};
use std::net::TcpListener;

fn main() {
    let port: u16 = std::env::var("OWNED_KEY_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .expect("OWNED_KEY_PORT must be a port number");
    let mode = std::env::var("OWNED_KEY_MODE").unwrap_or_default();
    let listener = match TcpListener::bind(("127.0.0.1", port)) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("fixture: cannot bind 127.0.0.1:{port}: {error}");
            std::process::exit(1);
        }
    };
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else {
            continue;
        };
        let mut bytes = [0; 2048];
        let _ = stream.read(&mut bytes);
        let (status, identity) = if mode == "busy" {
            ("503 Service Unavailable", "fixture")
        } else {
            ("200 OK", "wrong-identity")
        };
        let _ = stream.write_all(
            format!(
                "HTTP/1.1 {status}\r\nX-Owned: {identity}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )
            .as_bytes(),
        );
    }
}
