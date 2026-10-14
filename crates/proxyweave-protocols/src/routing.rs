// Copyright 2026 tiylabs / TiyGate Authors.
// SPDX-License-Identifier: Apache-2.0
//
// Extracted from tiygate (commit: 6db40e54663455fb99d675bd8985e4dc50f68a14) under Apache-2.0 License.

use serde::{Deserialize, Serialize};

/// Error classification for upstream and gateway errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorClass {
    // --- Upstream error classification ---
    /// Transient error (5xx, timeout, transport) — may retry/transfer.
    #[default]
    Transient,
    /// Rate limited (429) — transfer to different target, apply cooling.
    RateLimited,
    /// Authentication error (401/403) — don't retry same account, try different.
    Auth,
    /// Bad request (400/422) — fail immediately, don't transfer.
    BadRequest,
    /// Capability mismatch (lossy conversion or unsupported feature).
    LossyOrCapability,

    // --- Gateway-originated error classification ---
    /// No route found for the requested virtual model (404).
    ModelNotFound,
    /// Gateway deadline exceeded / timeout (504).
    DeadlineExceeded,
    /// All upstream targets exhausted without success (502).
    UpstreamExhausted,
    /// Inbound API key missing when `require_api_key` is on (401).
    AuthMissing,
    /// Inbound API key did not match any active key (401).
    AuthInvalid,
    /// Inbound API key matched a disabled / revoked key (403).
    AuthDisabled,
    /// Inbound API key is not allowed to access the requested virtual model (403).
    ModelAccessDenied,
    /// Gateway is overloaded / queue full (503).
    Overloaded,
}

impl ErrorClass {
    /// Map this class to the canonical HTTP status code.
    pub fn http_status(self) -> u16 {
        match self {
            Self::Transient => 502,
            Self::RateLimited => 429,
            Self::Auth => 401,
            Self::BadRequest => 400,
            Self::LossyOrCapability => 400,
            Self::ModelNotFound => 404,
            Self::DeadlineExceeded => 504,
            Self::UpstreamExhausted => 502,
            Self::AuthMissing => 401,
            Self::AuthInvalid => 401,
            Self::AuthDisabled => 403,
            Self::ModelAccessDenied => 403,
            Self::Overloaded => 503,
        }
    }
}

pub fn classify_upstream_error(
    upstream_status: Option<u16>,
    upstream_code: Option<&str>,
) -> ErrorClass {
    if let Some(status) = upstream_status {
        match status {
            429 => return ErrorClass::RateLimited,
            401 | 403 => return ErrorClass::Auth,
            400 | 422 => return ErrorClass::BadRequest,
            404 => return ErrorClass::ModelNotFound,
            503 => return ErrorClass::Overloaded,
            504 => return ErrorClass::DeadlineExceeded,
            _ if status >= 500 => {}
            _ => {}
        }
    }

    if let Some(code) = upstream_code {
        let lower = code.to_lowercase();
        if lower.contains("rate_limit")
            || lower.contains("quota")
            || lower == "429"
            || lower == "resource_exhausted"
        {
            return ErrorClass::RateLimited;
        }
        if lower.contains("auth")
            || lower == "401"
            || lower == "403"
            || lower.contains("invalid_api_key")
            || lower.contains("permission")
            || lower.contains("unauthenticated")
        {
            return ErrorClass::Auth;
        }
        if lower.contains("bad_request")
            || lower == "400"
            || lower.contains("invalid_argument")
            || lower.contains("invalid_request")
        {
            return ErrorClass::BadRequest;
        }
        if lower.contains("overloaded")
            || lower.contains("service_unavailable")
            || lower == "529"
            || lower == "503"
            || lower == "unavailable"
        {
            return ErrorClass::Overloaded;
        }
        if lower.contains("timeout") || lower.contains("deadline") {
            return ErrorClass::DeadlineExceeded;
        }
        if lower.contains("not_found") || lower.contains("model_not_found") || lower == "404" {
            return ErrorClass::ModelNotFound;
        }
    }

    ErrorClass::Transient
}
