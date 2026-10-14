# Verification Report: Flight Backend Seam (mv5j99yk-i6x9x5)

## 1. Executive Summary
This document records the verification results for the desktop Flight transport backend seam under UI Goal `mv5j99yk-i6x9x5`. The backend seam connects the desktop Tauri host to the local unified gateway loopback listener (`proxyweave_gateway::Gateway`), enabling genuine model discovery, non-streaming chat, and Server-Sent Events (SSE) streaming with live reasoning/text deltas and cancellation.

In accordance with `.pi/inspection/flight-feature-reviewer/dispatch-backend.md`:
- Fake-result fallback (DEF013 containment) is treated strictly as containment and is not counted as feature completion.
- A mature external SSE parser (`eventsource-stream` 0.2.3) was audited and integrated into `apps/desktop/src-tauri`.
- The 4 known unsafe existing tests touching fixed shared Keyring entries were strictly excluded from execution.
- 18 comprehensive owned unit and integration tests were executed and passed (18/18).
- The 5 original test suites of `eventsource-stream` were executed and passed in an isolated test harness.
- This deliverable covers the backend seam only. No claim is made that the entire UI or Managed goal is complete.

## 2. File Ownership Audit

### 2.1 Authorized Write Scope
- `apps/desktop/src-tauri/src/flight.rs` (new module + owned unit/integration tests)
- `apps/desktop/src-tauri/src/host.rs` (DesktopState integration + 4 Tauri commands)
- `apps/desktop/src-tauri/src/lib.rs` (module export, command registration, window Destroyed cancellation)
- `apps/desktop/src-tauri/Cargo.toml` (`eventsource-stream = "0.2.3"` addition)
- `Cargo.lock` (lockfile update for `eventsource-stream`)
- `docs/architecture/ADR-007-flight-transport.md` (ADR record)
- `docs/verification/FLIGHT-BACKEND.md` (this verification document)
- `.pi/development/flight-backend/**` (baseline, freeze bytes, test receipts)

### 2.2 Actual Modifications Relative to Dirty Baseline
A strict SHA-256 byte-level audit against the pre-task baseline (`.pi/development/flight-backend/baseline/bytes/`) confirms that only authorized files were touched:
- `apps/desktop/src-tauri/Cargo.toml`: Modified (added `eventsource-stream = "0.2.3"`).
- `Cargo.lock`: Modified (resolved `eventsource-stream 0.2.3`).
- `apps/desktop/src-tauri/src/host.rs`: Modified (added `Arc<FlightManager>` to `DesktopState`, exposed commands).
- `apps/desktop/src-tauri/src/lib.rs`: Modified (registered `flight` module, invoke handlers, window destroy hook).
- `apps/desktop/src-tauri/src/flight.rs`: Created.
- `docs/architecture/ADR-007-flight-transport.md`: Created.
- `docs/verification/FLIGHT-BACKEND.md`: Created.
- `.pi/development/flight-backend/**`: Created.

Zero modifications were made to frontend files (`apps/desktop/src/**`), `apps/desktop/src/api.ts`, `docs/DEFECTS_LEDGER.md`, `.pi/inspection/**`, or any other crates/documentation.

## 3. Mature SSE Parser Audit (`eventsource-stream` 0.2.3)

### 3.1 Provenance and License
- Crate: `eventsource-stream`
- Version: `0.2.3`
- Registry Path: `D:/build-tools/cargo/registry/src/rsproxy.cn-e3de039b2554c837/eventsource-stream-0.2.3`
- License: `MIT OR Apache-2.0` (compatible with Quotio/ProxyWeave license policy)
- Dependencies: `futures-core`, `pin-project-lite`, `nom 7.1.3`
- Architecture:
  - `Utf8Stream`: Maintains an internal buffer of incomplete UTF-8 byte sequences across chunk boundaries using `std::str::from_utf8`. Splits cleanly without decoding corruption.
  - `nom`-based line/event parser: Correctly handles arbitrary combinations of `\r\n`, `\n`, `\r`, event comments (`:comment`), `event:`, `data:`, `id:`, and `retry:`.

### 3.2 Upstream Original Test Audit
- Standalone execution of `cargo test` in the original registry package fails on modern Rust due to an unaligned packed struct reference (`E0793`) in its outdated dev-dependency `ntapi 0.3.6` (pulled by legacy `reqwest 0.11.11`).
- An isolated test harness was constructed at `.pi/development/flight-backend/harness/` copying the exact source of `eventsource-stream 0.2.3` and updating the dev-dependency `ntapi` to `0.4.1`.
- Original test suites verified:
  - `valid_data_fields`: Passed.
  - `spec_examples`: Passed.
  - `valid_streams`: Passed.
  - `invalid_streams`: Passed.
  - `populate_fields`: Passed.
- Test log recorded at `.pi/development/flight-backend/original-crate-audit/original-tests-after-ntapi-update.txt`.

## 4. Keyring Safety Audit
The following 4 existing integration tests were identified as accessing fixed shared Keyring credentials on the host operating system:
1. `desktop_host.rs`: `test_desktop_state_master_key_and_config_persistence`
2. `desktop_host.rs`: `test_host_events_bounded_fifo`
3. `desktop_host.rs`: `test_desktop_gateway_lifecycle_and_backend_attachment`
4. `runtime_lifecycle.rs`: `test_generic_managed_projection_and_spawn_without_hardcoding`

**Safety Guarantee**: None of these 4 tests were executed during this task. Test execution was strictly targeted to `cargo test -p quotio-desktop --lib flight` and the isolated parser harness. Host keyring credentials and user profiles remained completely untouched.

## 5. Owned Test Suite Execution

Command executed:
```bash
cargo test -p quotio-desktop --lib flight -- --nocapture
```

Results: **18 passed, 0 failed, 0 ignored**

| Test Name | Verification Focus | Result |
| :--- | :--- | :--- |
| `test_validate_chat_input` | Rejects empty model, empty messages, whitespace-only messages | PASS |
| `test_truncate_error_message` | Enforces 256-char cap on error messages | PASS |
| `test_scrub_sensitive_strings` | Scrubs loopback URLs (`127.0.0.1`, `localhost`), bearer tokens, paths | PASS |
| `test_list_models_core` | Discovers models from `/v1/models` through running gateway | PASS |
| `test_list_models_scrubs_401` | Masks 401 Unauthorized errors and prevents header/key leakage | PASS |
| `test_chat_nonstream_core` | Returns complete non-streaming responses with monotonic elapsed time | PASS |
| `test_chat_nonstream_preheader_cancel` | Proves pre-header cancellation aborts request before headers arrive | PASS |
| `test_chat_stream_core_happy_path` | Handles 3-byte UTF-8 split chunks, CRLF/LF, comments, reasoning, usage, and `[DONE]` | PASS |
| `test_chat_stream_eof_without_done_fails` | Rejects truncated SSE streams ending without `[DONE]`; never synthesizes success | PASS |
| `test_chat_stream_midstream_cancel` | Mid-stream cancellation terminates stream and drops connection immediately | PASS |
| `test_chat_stream_429_error` | Reports 429 rate limit errors honestly with scrubbed error message | PASS |
| `test_flight_manager_session_tracking` | Enforces 1 active session per window, pre-empts old session, cleans up on destroy | PASS |
| `test_real_loopback_gateway_integration_states` | Verifies running gateway checks and backend mount checks | PASS |
| `test_chat_stream_multiline_data` | Verifies multiline `data:` blocks in SSE frames combine with newlines | PASS |
| `test_chat_stream_cr_only_line_endings` | Verifies legacy Mac CR-only line endings parse correctly | PASS |
| `test_chat_stream_oversized_frame_rejected` | Rejects frames exceeding 512KB to protect process memory | PASS |
| `test_chat_stream_accumulated_payload_cap` | Rejects streams accumulating over 2MB total content | PASS |
| `test_chat_nonstream_oversized_body_rejected` | Rejects non-streaming responses exceeding 2MB | PASS |

## 6. Frontend IPC Contract (DTO Specifications)

The following contract is ready for implementation by the frontend integration stage:

### 6.1 Tauri Commands

#### 1. `flight_list_models`
- **Arguments**: None
- **Return Type**: `Vec<FlightModelInfo>`
  ```typescript
  interface FlightModelInfo {
    id: string;
    owned_by: string;
    description?: string;
  }
  ```
- **Error**: `String` (sanitized, no sensitive endpoints or tokens)

#### 2. `flight_chat_nonstream`
- **Arguments**:
  - `input`: `FlightChatInput`
    ```typescript
    interface FlightChatInput {
      model: string;
      messages: Array<{
        role: "system" | "user" | "assistant";
        content: string;
      }>;
      temperature?: number; // 0.0 to 2.0, default 0.7
    }
    ```
- **Return Type**: `FlightNonstreamResponse`
  ```typescript
  interface FlightNonstreamResponse {
    request_id: string;
    model: string;
    role: string;
    content: string;
    reasoning_content?: string;
    elapsed_ms: number;
    usage?: {
      prompt_tokens: number;
      completion_tokens: number;
      total_tokens: number;
    };
  }
  ```

#### 3. `flight_chat_stream`
- **Arguments**:
  - `input`: `FlightChatInput` (as defined above)
  - `on_event`: `Channel<FlightStreamEvent>` (Tauri official IPC Channel)
- **Return Type**: `String` (assigned `request_id`)
- **Channel Event Schema (`FlightStreamEvent`)**:
  ```typescript
  type FlightStreamEvent =
    | {
        event: "delta";
        data: {
          request_id: string;
          text?: string;
          reasoning?: string;
          elapsed_ms: number;
          first_content_ms?: number;
        };
      }
    | {
        event: "usage";
        data: {
          prompt_tokens: number;
          completion_tokens: number;
          total_tokens: number;
        };
      }
    | {
        event: "terminal";
        data:
          | { state: "complete" }
          | { state: "cancelled" }
          | { state: "failed"; code: string; message: string };
      };
  ```

#### 4. `flight_cancel_chat`
- **Arguments**:
  - `request_id`: `string`
- **Return Type**: `boolean` (`true` if active session was cancelled; `false` if not found or already completed)

## 7. Remaining Gaps & Unverified Items
1. **Frontend UI Seam**: `apps/desktop/src/components/FlightPanel.tsx` currently displays static/mock elements; it has not yet been connected to the backend Tauri commands.
2. **End-to-End Real Model Verification**: Verification of a live, installed upstream backend (e.g., `wb2api`) generating actual inference tokens in a native desktop window remains open for Main inspection.
3. **Whole Goal Completion**: This task completes only the backend seam of UI Goal `mv5j99yk-i6x9x5`. Neither the UI Goal nor the Managed Goal is claimed as fully completed.
