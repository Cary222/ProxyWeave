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

/// Evaluate the Manifest contract before trusting any HTTP status.
pub fn evaluate_health_status(
    health: &crate::manifest::HealthCheck,
    status_code: u16,
    identity: Option<&str>,
) -> ProbeStatus {
    if identity != Some(health.identity_value.as_str()) {
        return ProbeStatus::Failed {
            error: "Health check identity mismatch or missing header".into(),
        };
    }
    if status_code == health.expected_status {
        ProbeStatus::Ready
    } else if health.busy_statuses.contains(&status_code) {
        ProbeStatus::Busy {
            reason: format!("Manifest-declared unavailable HTTP status {status_code}"),
        }
    } else {
        ProbeStatus::Failed {
            error: format!("Unexpected HTTP status code: {status_code}"),
        }
    }
}
/// Performs an asynchronous HTTP GET probe against a target URL.
pub async fn probe_http(
    url: &str,
    health: &crate::manifest::HealthCheck,
    timeout: Duration,
) -> ProbeReport {
    let timeout = timeout.min(Duration::from_millis(health.timeout_ms));
    let start = Instant::now();
    let client = match reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(timeout)
        .build()
    {
        Ok(c) => c,
        Err(_) => {
            return ProbeReport {
                status: ProbeStatus::Failed {
                    error: "Failed to build HTTP client".into(),
                },
                status_code: None,
                latency_ms: 0,
                detail: "HTTP client build failed".into(),
            };
        }
    };

    match client.get(url).send().await {
        Ok(resp) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            let status_code = resp.status().as_u16();

            let identity = resp
                .headers()
                .get(&health.identity_header)
                .and_then(|v| v.to_str().ok());
            let status = evaluate_health_status(health, status_code, identity);
            // Never echo a peer-controlled header into status/log messages.
            let detail = format!("HTTP {status_code} in {latency_ms}ms");
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
                    detail: "Connection failed or not yet listening".into(),
                }
            } else {
                ProbeReport {
                    status: ProbeStatus::Failed {
                        error: "HTTP request failed".into(),
                    },
                    status_code: None,
                    latency_ms,
                    detail: "HTTP request failed".into(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::HealthCheck;

    #[test]
    fn identity_precedes_ready_and_busy_status_contract() {
        let health = HealthCheck {
            path: "/healthz".into(),
            expected_status: 200,
            identity_header: "X-Identity".into(),
            identity_value: "fixture".into(),
            timeout_ms: 100,
            busy_statuses: vec![503],
        };
        assert_eq!(
            evaluate_health_status(&health, 200, Some("fixture")),
            ProbeStatus::Ready
        );
        assert!(matches!(
            evaluate_health_status(&health, 503, Some("fixture")),
            ProbeStatus::Busy { .. }
        ));
        for identity in [
            None,
            Some("different-service"),
            Some("FIXTURE"),
            Some("fixture "),
        ] {
            for status in [200, 503] {
                assert!(matches!(
                    evaluate_health_status(&health, status, identity),
                    ProbeStatus::Failed { .. }
                ));
            }
        }
        assert!(matches!(
            evaluate_health_status(&health, 500, Some("fixture")),
            ProbeStatus::Failed { .. }
        ));
        let no_busy = HealthCheck {
            busy_statuses: vec![],
            ..health
        };
        assert!(matches!(
            evaluate_health_status(&no_busy, 503, Some("fixture")),
            ProbeStatus::Failed { .. }
        ));
    }
}
