//! Health check probe implementation with protocol-specific semantic evaluation.
//!
//! Handles general HTTP endpoints as well as plugin-specific status semantics
//! (such as wb2api's HTTP 503 + `X-Service: workbuddy2api` indicating an unconfigured/exhausted
//! account pool rather than a crash or conflict).

use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProbeStatus {
    /// Service is completely offline / unreachable
    Stopped,
    /// Service process is starting up (e.g. connection refused or warming up)
    Starting,
    /// Service is fully ready to serve traffic (HTTP 200)
    Ready,
    /// Service process is alive and responsive, but temporarily cannot serve traffic
    /// (e.g. wb2api account pool unconfigured or saturated; MUST NOT be killed or restarted)
    Busy { reason: String },
    /// Probe request timed out (never conflated with port conflict)
    Timeout,
    /// Service returned an unexpected error response
    Failed { error: String },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProbeReport {
    pub status: ProbeStatus,
    pub status_code: Option<u16>,
    pub latency_ms: u64,
    pub detail: String,
}

/// Evaluates HTTP response for wb2api `/healthz` per its Go source (`handler.go:181-203`):
/// - 200 OK -> Ready
/// - 503 Service Unavailable + `X-Service: workbuddy2api` -> Busy (Pool.ServableNow() is false; alive, not a crash)
/// - Other -> Failed
pub fn evaluate_wb2api_status(status_code: u16, x_service_header: Option<&str>) -> ProbeStatus {
    if status_code == 200 {
        ProbeStatus::Ready
    } else if status_code == 503 {
        let is_wb2api = x_service_header
            .map(|h| h.trim().eq_ignore_ascii_case("workbuddy2api"))
            .unwrap_or(false);
        if is_wb2api {
            ProbeStatus::Busy {
                reason: "workbuddy2api account pool empty or saturated (X-Service: workbuddy2api)"
                    .to_string(),
            }
        } else {
            ProbeStatus::Failed {
                error: "HTTP 503 returned without X-Service: workbuddy2api header".to_string(),
            }
        }
    } else {
        ProbeStatus::Failed {
            error: format!("Unexpected HTTP status code: {}", status_code),
        }
    }
}

/// Performs an asynchronous HTTP GET probe against a target URL.
pub async fn probe_http(url: &str, plugin_id: Option<&str>, timeout: Duration) -> ProbeReport {
    let start = Instant::now();
    let client = match reqwest::Client::builder().timeout(timeout).build() {
        Ok(c) => c,
        Err(e) => {
            return ProbeReport {
                status: ProbeStatus::Failed {
                    error: format!("Failed to build HTTP client: {}", e),
                },
                status_code: None,
                latency_ms: 0,
                detail: format!("Client build error: {}", e),
            };
        }
    };

    match client.get(url).send().await {
        Ok(resp) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            let status_code = resp.status().as_u16();

            let x_service = resp
                .headers()
                .get("x-service")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            let status =
                if plugin_id == Some("wb2api") || x_service.as_deref() == Some("workbuddy2api") {
                    evaluate_wb2api_status(status_code, x_service.as_deref())
                } else if status_code == 200 {
                    ProbeStatus::Ready
                } else {
                    ProbeStatus::Failed {
                        error: format!("HTTP {}", status_code),
                    }
                };

            let detail = format!(
                "HTTP {} in {}ms (x-service: {:?})",
                status_code, latency_ms, x_service
            );

            ProbeReport {
                status,
                status_code: Some(status_code),
                latency_ms,
                detail,
            }
        }
        Err(err) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            if err.is_timeout() {
                ProbeReport {
                    status: ProbeStatus::Timeout,
                    status_code: None,
                    latency_ms,
                    detail: format!("Probe timed out after {:?}", timeout),
                }
            } else if err.is_connect() {
                ProbeReport {
                    status: ProbeStatus::Starting,
                    status_code: None,
                    latency_ms,
                    detail: format!("Connection refused or not yet listening: {}", err),
                }
            } else {
                ProbeReport {
                    status: ProbeStatus::Failed {
                        error: format!("HTTP error: {}", err),
                    },
                    status_code: None,
                    latency_ms,
                    detail: format!("HTTP request error: {}", err),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wb2api_status_evaluation() {
        // 200 -> Ready
        assert_eq!(
            evaluate_wb2api_status(200, Some("workbuddy2api")),
            ProbeStatus::Ready
        );

        // 503 with X-Service: workbuddy2api -> Busy (NOT Failed, NOT crash, must not kill)
        let busy = evaluate_wb2api_status(503, Some("workbuddy2api"));
        match busy {
            ProbeStatus::Busy { reason } => {
                assert!(reason.contains("workbuddy2api"));
            }
            other => panic!("expected Busy, got {:?}", other),
        }

        // 503 without X-Service header -> Failed
        let failed_503 = evaluate_wb2api_status(503, None);
        assert!(matches!(failed_503, ProbeStatus::Failed { .. }));

        // 500 Internal Server Error -> Failed
        let failed_500 = evaluate_wb2api_status(500, Some("workbuddy2api"));
        assert!(matches!(failed_500, ProbeStatus::Failed { .. }));
    }
}
