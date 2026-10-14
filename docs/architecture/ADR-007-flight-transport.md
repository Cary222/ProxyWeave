# ADR-007: Flight Desktop Transport and SSE Streaming

- Status: Accepted
- Date: 2026-03-29
- Deciders: Core Engineering, Developer Agent
- Context: UI Goal Flight Feature Backend Seam (mv5j99yk-i6x9x5)

## 1. Context and Problem Statement
Active goal `mv5j99yk-i6x9x5` requires visual alignment and real host interactions for the desktop Flight panel. Prior blanket completion reports were retracted because containment DEF013 (disabling fake fallback data) was not an implementation of real Flight Chat or SSE streaming. The frontend requires a secure, high-integrity backend seam to list models discovered by the active backend, initiate non-streaming and streaming completions, track live reasoning and text deltas, and reliably cancel in-flight requests.

The solution must satisfy stringent desktop security and stability constraints:
1. The frontend must never communicate with upstream providers directly or learn gateway master keys, upstream bearer tokens, loopback internal URLs, or upstream command-line flags.
2. The transport must reuse the existing unified gateway (`proxyweave_gateway::Gateway`) loopback listener, requiring an active mounted backend plugin and an internal master authorization bearer.
3. Cancellation must cleanly drop in-flight HTTP sockets and runtime request leases promptly (within monotonic deadlines shorter than default gateway headers timeouts), both before response headers arrive and during mid-stream chunks.
4. Server-Sent Events (SSE) must parse arbitrarily fragmented byte streams, CRLF/LF line endings, comments, multiline data, and reasoning deltas using a mature external parser rather than a homemade framing state machine.
5. Incomplete streams ending in premature transport EOF without a valid protocol terminal marker (such as OpenAI-compatible `data: [DONE]`) must fail truthfully and never synthesize a successful finish.
6. The desktop state must enforce single-session concurrency per window to prevent unbounded task creation and memory leaks.

## 2. Decision Drivers
- **Security & Privacy**: Zero credential leakage across the Tauri IPC bridge. No URLs, command lines, master keys, or raw provider error payloads returned to the frontend or written to diagnostic logs.
- **Protocol Truthfulness**: Accurate distinction between non-streaming responses, live streaming deltas, user-initiated cancellations, upstream errors, and unexpected EOF disconnects.
- **Process Isolation & Lifecycle Cleanup**: Guaranteed cancellation and lease release when windows are closed, destroyed, or unmounted, or when the app exits.
- **Dependency Pragmatism**: Reuse of the mature `eventsource-stream` (0.2.3) crate for SSE stream decoding rather than handcrafting an ad-hoc parser.

## 3. Considered Alternatives
1. **Frontend Direct Fetch via Tauri HTTP Plugin**:
   - Rejected. Exposes the internal gateway master key to the webview, bypasses desktop request concurrency controls, and risks leaking loopback authorization tokens across webview boundaries.
2. **Ad-hoc Custom SSE Parser in `flight.rs`**:
   - Rejected. Self-made parsers frequently mismanage multi-byte UTF-8 boundaries split across arbitrary chunk buffers, CRLF vs LF line endings, multiline `data:` payloads, and SSE comments.
3. **Mature External SSE Parser (`eventsource-stream` 0.2.3)**:
   - Accepted. `eventsource-stream` is a lightweight, widely adopted, `MIT OR Apache-2.0` licensed crate that cleanly wraps any `Stream<Item = Result<Bytes, reqwest::Error>>` with a robust UTF-8 decoder (`Utf8Stream`) and `nom`-based event parser.

## 4. Architecture & Implementation

### 4.1 Modular Structure
- `apps/desktop/src-tauri/src/flight.rs`:
  - `FlightManager`: Thread-safe session tracker storing active cancellation handles (`tokio_util::sync::CancellationToken`) keyed by `(window_label, request_id)`. Enforces single-active-session per window label, automatic pre-emption/cancellation of previous requests on new submission, and window-level cleanup on window destruction.
  - Core asynchronous orchestration functions:
    - `list_models_core(gateway, client)`
    - `chat_nonstream_core(gateway, client, input, cancel_token)`
    - `chat_stream_core(gateway, client, input, on_event, cancel_token)`
  - Safe payload limits: 2MB maximum payload size for nonstream responses and accumulated streams; 512KB maximum frame size; 64KB maximum error message size.
- `apps/desktop/src-tauri/src/host.rs`:
  - `DesktopState`: Integrates `Arc<FlightManager>`.
  - Exposes Tauri commands `flight_list_models`, `flight_chat_stream`, `flight_chat_nonstream`, and `flight_cancel_chat`. Verifies that the gateway is running and that a backend plugin is currently mounted before issuing requests.
- `apps/desktop/src-tauri/src/lib.rs`:
  - Registers the 4 flight commands in Tauri invoke handlers.
  - Registers `tauri::WindowEvent::Destroyed` listener to trigger `FlightManager::cancel_all_for_window` when any desktop window closes.

### 4.2 Tauri IPC Channel Protocol
Streaming events are delivered to the frontend via official Tauri `Channel<FlightStreamEvent>`:
- `Delta`: Emitted incrementally with monotonic metrics:
  - `request_id`: UUID/request tracking ID.
  - `text`: Incremental content text delta (if present).
  - `reasoning`: Incremental reasoning content delta (e.g., DeepSeek/OpenAI thinking deltas).
  - `elapsed_ms`: Monotonic milliseconds since request dispatch.
  - `first_content_ms`: Monotonic milliseconds until the first content/reasoning delta arrived.
- `Usage`: Emitted when upstream returns a final token usage report (`prompt_tokens`, `completion_tokens`, `total_tokens`). No chars-as-tokens heuristics are fabricated.
- `Terminal`: Concludes the stream with one of three mutually exclusive terminal states:
  - `Complete`: Successful stream terminated by genuine protocol finish evidence (`[DONE]`).
  - `Cancelled`: Request cleanly stopped via cancellation token before or mid-stream.
  - `Failed`: Honest error code and scrubbed message on network failure, 4xx/5xx HTTP errors, or unexpected EOF without finish marker.

### 4.3 Redaction and Error Scrubbing
All error strings returned over IPC or logged internally pass through `scrub_sensitive_strings`:
- Replaces loopback URLs (`http://127.0.0.1:...` or `http://localhost:...`) with `[LOOPBACK_URL]`.
- Replaces master auth headers and bearer tokens with `[AUTH_REDACTED]`.
- Replaces process paths, binary names, and internal command-line arguments with `[PATH_REDACTED]`.
- Truncates oversized error messages to 256 characters.

## 5. Consequences & Verification
- **Positive**:
  - The frontend receives genuine model completions and reasoning deltas through a typed, responsive channel.
  - Cancellation immediately aborts the underlying reqwest stream, releasing the loopback gateway connection and runtime lease.
  - Zero sensitive keys or loopback endpoints are disclosed to the webview.
- **Verification**:
  - 18 comprehensive tests in `quotio-desktop --lib flight` verify input validation, nonstream and stream execution, UTF-8 boundary chunking across 3-byte characters, CRLF/LF and comment parsing, reasoning extraction, token usage extraction, pre-header and mid-stream cancellation, 401/429/503 handling, loopback URL scrubbing, EOF without `[DONE]` rejection, and concurrent window session tracking.
  - Standalone verification of `eventsource-stream` 0.2.3 original tests confirmed 5/5 passing test suites.
