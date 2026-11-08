use axum::http::HeaderValue;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

/// Routing strategies inspired by Magpie (M16) routing design.
/// Crucial principle: Routing strategies only reorder (permute) backends,
/// they never filter out backends (fail-open).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RouteStrategy {
    /// Ranks backends by lowest recent latency and error-rate.
    #[default]
    Smart,
    /// Strict registration priority order (first is primary, then backup).
    Order,
    /// Round-robin across available backends.
    Rotate,
}

impl RouteStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Smart => "smart",
            Self::Order => "order",
            Self::Rotate => "rotate",
        }
    }

    pub fn from_str_name(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "smart" => Some(Self::Smart),
            "order" => Some(Self::Order),
            "rotate" => Some(Self::Rotate),
            _ => None,
        }
    }
}

/// An active backend endpoint registered to the gateway.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Backend {
    pub plugin_id: String,
    pub protocol: String,
    pub models_path: String,
    pub inference_path: String,
    pub credential: HeaderValue,
    pub instance_generation: Option<u64>,
}

impl Backend {
    pub fn new(
        plugin_id: String,
        protocol: String,
        models_path: String,
        inference_path: String,
        credential: HeaderValue,
    ) -> Self {
        Self {
            plugin_id,
            protocol,
            models_path,
            inference_path,
            credential,
            instance_generation: None,
        }
    }

    pub fn with_generation(mut self, generation: u64) -> Self {
        self.instance_generation = Some(generation);
        self
    }

    pub fn with_instance_generation(mut self, generation: Option<u64>) -> Self {
        self.instance_generation = generation;
        self
    }

    pub fn matches_generation(&self, actual: u64) -> bool {
        self.instance_generation.is_none_or(|g| g == actual)
    }
}

/// Runtime metrics for a backend instance used by Smart routing.
#[derive(Debug, Clone, Default)]
pub struct BackendMetrics {
    pub latencies_ms: VecDeque<u64>,
    pub success_count: u64,
    pub error_count: u64,
    pub last_status: Option<u16>,
}

impl BackendMetrics {
    pub fn record(&mut self, latency_ms: u64, status: u16) {
        if self.latencies_ms.len() >= 5 {
            self.latencies_ms.pop_front();
        }
        self.latencies_ms.push_back(latency_ms);
        if (200..400).contains(&status) {
            self.success_count = self.success_count.saturating_add(1);
        } else if status >= 500 {
            self.error_count = self.error_count.saturating_add(1);
        }
        self.last_status = Some(status);
    }

    pub fn average_latency_ms(&self) -> u64 {
        if self.latencies_ms.is_empty() {
            // Unvisited backends start with a baseline latency of 200ms
            200
        } else {
            let sum: u64 = self.latencies_ms.iter().sum();
            sum / (self.latencies_ms.len() as u64)
        }
    }

    /// Lower score is better. Latency + 300ms penalty per recent 5xx error.
    pub fn health_score(&self) -> u64 {
        let avg = self.average_latency_ms();
        let error_penalty = self.error_count.saturating_mul(300);
        avg.saturating_add(error_penalty)
    }
}

/// Session stickiness cache to preserve upstream Prompt Caching (Claude / DeepSeek).
/// When multi-turn requests come from the same session, they stick to the same backend
/// so that the server-side prompt cache stays warm.
#[derive(Debug)]
pub struct SessionStickiness {
    sessions: HashMap<String, (String, Instant)>,
    ttl: Duration,
}

impl SessionStickiness {
    pub fn new(ttl: Duration) -> Self {
        Self {
            sessions: HashMap::new(),
            ttl,
        }
    }

    pub fn get(&mut self, session_key: &str) -> Option<String> {
        self.cleanup();
        self.sessions.get(session_key).map(|(id, _)| id.clone())
    }

    pub fn bind(&mut self, session_key: String, plugin_id: String) {
        self.cleanup();
        self.sessions
            .insert(session_key, (plugin_id, Instant::now()));
    }

    fn cleanup(&mut self) {
        let now = Instant::now();
        let ttl = self.ttl;
        self.sessions
            .retain(|_, (_, ts)| now.duration_since(*ts) < ttl);
    }
}

/// Thread-safe multi-backend router pool.
pub struct RoutePool {
    strategy: RwLock<RouteStrategy>,
    backends: RwLock<Vec<Backend>>,
    metrics: RwLock<HashMap<String, BackendMetrics>>,
    stickiness: RwLock<SessionStickiness>,
    rotate_counter: AtomicUsize,
}

impl RoutePool {
    pub fn new() -> Self {
        Self {
            strategy: RwLock::new(RouteStrategy::Smart),
            backends: RwLock::new(Vec::new()),
            metrics: RwLock::new(HashMap::new()),
            stickiness: RwLock::new(SessionStickiness::new(Duration::from_secs(600))),
            rotate_counter: AtomicUsize::new(0),
        }
    }

    pub async fn set_strategy(&self, strategy: RouteStrategy) {
        *self.strategy.write().await = strategy;
    }

    pub async fn get_strategy(&self) -> RouteStrategy {
        *self.strategy.read().await
    }

    pub async fn set_single_backend(&self, backend: Backend) {
        let mut list = self.backends.write().await;
        list.clear();
        list.push(backend);
    }

    pub async fn add_backend(&self, backend: Backend) {
        let mut list = self.backends.write().await;
        if let Some(pos) = list.iter().position(|b| b.plugin_id == backend.plugin_id) {
            list[pos] = backend;
        } else {
            list.push(backend);
        }
    }

    /// Re-binds an existing route entry to the instance generation the runtime reports.
    /// Returns `true` when a matching entry was updated, `false` when the plugin is unknown.
    pub async fn update_generation(&self, plugin_id: &str, generation: Option<u64>) -> bool {
        let mut list = self.backends.write().await;
        match list
            .iter_mut()
            .find(|backend| backend.plugin_id == plugin_id)
        {
            Some(backend) => {
                backend.instance_generation = generation;
                true
            }
            None => false,
        }
    }

    pub async fn remove_backend(&self, plugin_id: &str) {
        let mut list = self.backends.write().await;
        list.retain(|b| b.plugin_id != plugin_id);
    }

    pub async fn clear_backends(&self) {
        let mut list = self.backends.write().await;
        list.clear();
    }

    pub async fn get_primary_backend(&self) -> Option<Backend> {
        let list = self.backends.read().await;
        list.first().cloned()
    }

    pub async fn list_backends(&self) -> Vec<Backend> {
        self.backends.read().await.clone()
    }

    /// Select an ordered list of candidate backends based on session stickiness and strategy.
    /// Returns: (candidates, applied_strategy, is_sticky_hit)
    /// Select an ordered list of candidate backends based on session stickiness and strategy.
    /// Returns: (candidates, applied_strategy, is_sticky_hit)
    pub async fn select_candidates(
        &self,
        session_key: Option<&str>,
    ) -> (Vec<Backend>, RouteStrategy, bool) {
        self.select_candidates_for_target(session_key, None).await
    }

    /// Select candidates optionally filtered by a specific target plugin_id.
    pub async fn select_candidates_for_target(
        &self,
        session_key: Option<&str>,
        target_plugin_id: Option<&str>,
    ) -> (Vec<Backend>, RouteStrategy, bool) {
        let backends = self.backends.read().await.clone();
        let backends: Vec<Backend> = if let Some(target) = target_plugin_id {
            backends
                .into_iter()
                .filter(|b| b.plugin_id.eq_ignore_ascii_case(target))
                .collect()
        } else {
            backends
        };
        if backends.is_empty() {
            let strategy = *self.strategy.read().await;
            return (Vec::new(), strategy, false);
        }
        if backends.len() == 1 {
            let strategy = *self.strategy.read().await;
            return (backends, strategy, false);
        }

        let strategy = *self.strategy.read().await;

        // 1. Check session stickiness
        let mut sticky_hit = None;
        if let Some(key) = session_key {
            let mut stick_guard = self.stickiness.write().await;
            if let Some(target_id) = stick_guard.get(key) {
                if let Some(pos) = backends.iter().position(|b| b.plugin_id == target_id) {
                    sticky_hit = Some(pos);
                }
            }
        }

        // 2. Fail-Open Sorting based on strategy
        let mut candidates = backends;
        match strategy {
            RouteStrategy::Smart => {
                let metrics_guard = self.metrics.read().await;
                candidates.sort_by_key(|b| {
                    metrics_guard
                        .get(&b.plugin_id)
                        .map(|m| m.health_score())
                        .unwrap_or(200)
                });
            }
            RouteStrategy::Order => {
                // Keep declared order
            }
            RouteStrategy::Rotate => {
                let count = candidates.len();
                let idx = self.rotate_counter.fetch_add(1, Ordering::Relaxed) % count;
                candidates.rotate_left(idx);
            }
        }

        // 3. If session stickiness hit, lift the sticky candidate to the top
        let is_sticky = if let Some(orig_pos) = sticky_hit {
            let sticky_id = &self.backends.read().await[orig_pos].plugin_id;
            if let Some(cand_pos) = candidates.iter().position(|b| &b.plugin_id == sticky_id) {
                let sticky_backend = candidates.remove(cand_pos);
                candidates.insert(0, sticky_backend);
                true
            } else {
                false
            }
        } else {
            false
        };

        (candidates, strategy, is_sticky)
    }
    pub async fn record_outcome(
        &self,
        plugin_id: &str,
        latency_ms: u64,
        status_code: u16,
        session_key: Option<&str>,
    ) {
        {
            let mut metrics_guard = self.metrics.write().await;
            let m = metrics_guard.entry(plugin_id.to_string()).or_default();
            m.record(latency_ms, status_code);
        }

        if (200..400).contains(&status_code) {
            if let Some(key) = session_key {
                let mut stick_guard = self.stickiness.write().await;
                stick_guard.bind(key.to_string(), plugin_id.to_string());
            }
        }
    }
}

impl Default for RoutePool {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract session identification key from headers or request attributes.
pub fn extract_session_key(
    headers: &axum::http::HeaderMap,
    client_ip: Option<&str>,
    model: Option<&str>,
) -> Option<String> {
    if let Some(s) = headers
        .get("x-session-id")
        .or_else(|| headers.get("x-request-session"))
        .or_else(|| headers.get("x-conversation-id"))
    {
        if let Ok(v) = s.to_str() {
            if !v.trim().is_empty() {
                return Some(v.trim().to_string());
            }
        }
    }

    if let (Some(ip), Some(m)) = (client_ip, model) {
        return Some(format!("{}:{}", ip, m));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_backend(id: &str) -> Backend {
        Backend {
            plugin_id: id.to_string(),
            protocol: "openai_chat".to_string(),
            models_path: "/v1/models".to_string(),
            inference_path: "/v1/chat/completions".to_string(),
            credential: HeaderValue::from_static("Bearer test"),
            instance_generation: None,
        }
    }

    #[tokio::test]
    async fn test_order_strategy_preserves_order() {
        let pool = RoutePool::new();
        pool.set_strategy(RouteStrategy::Order).await;
        pool.add_backend(dummy_backend("b1")).await;
        pool.add_backend(dummy_backend("b2")).await;
        pool.add_backend(dummy_backend("b3")).await;

        let (candidates, strat, sticky) = pool.select_candidates(None).await;
        assert_eq!(strat, RouteStrategy::Order);
        assert!(!sticky);
        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates[0].plugin_id, "b1");
        assert_eq!(candidates[1].plugin_id, "b2");
        assert_eq!(candidates[2].plugin_id, "b3");
    }

    #[tokio::test]
    async fn test_rotate_strategy_rotates_candidates() {
        let pool = RoutePool::new();
        pool.set_strategy(RouteStrategy::Rotate).await;
        pool.add_backend(dummy_backend("b1")).await;
        pool.add_backend(dummy_backend("b2")).await;

        let (c1, _, _) = pool.select_candidates(None).await;
        let (c2, _, _) = pool.select_candidates(None).await;
        assert_eq!(c1[0].plugin_id, "b1");
        assert_eq!(c2[0].plugin_id, "b2");
    }

    #[tokio::test]
    async fn test_smart_strategy_ranks_by_health_and_latency() {
        let pool = RoutePool::new();
        pool.set_strategy(RouteStrategy::Smart).await;
        pool.add_backend(dummy_backend("slow")).await;
        pool.add_backend(dummy_backend("fast")).await;

        pool.record_outcome("slow", 800, 200, None).await;
        pool.record_outcome("fast", 50, 200, None).await;

        let (candidates, strat, _) = pool.select_candidates(None).await;
        assert_eq!(strat, RouteStrategy::Smart);
        assert_eq!(candidates[0].plugin_id, "fast");
        assert_eq!(candidates[1].plugin_id, "slow");
    }

    #[tokio::test]
    async fn test_prompt_cache_session_stickiness() {
        let pool = RoutePool::new();
        pool.set_strategy(RouteStrategy::Rotate).await;
        pool.add_backend(dummy_backend("b1")).await;
        pool.add_backend(dummy_backend("b2")).await;

        let session_id = "user-conv-123";
        // Record successful call with b2
        pool.record_outcome("b2", 100, 200, Some(session_id)).await;

        // Even under Rotate strategy, session stickiness pins b2 to position 0
        let (candidates, _, sticky) = pool.select_candidates(Some(session_id)).await;
        assert!(sticky);
        assert_eq!(candidates[0].plugin_id, "b2");
    }

    #[tokio::test]
    async fn test_fail_open_principle_never_discards_backends() {
        let pool = RoutePool::new();
        pool.set_strategy(RouteStrategy::Smart).await;
        pool.add_backend(dummy_backend("bad_500")).await;
        pool.add_backend(dummy_backend("ok_200")).await;

        pool.record_outcome("bad_500", 500, 500, None).await;
        pool.record_outcome("ok_200", 200, 200, None).await;

        let (candidates, _, _) = pool.select_candidates(None).await;
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].plugin_id, "ok_200");
        assert_eq!(candidates[1].plugin_id, "bad_500");
    }

    #[tokio::test]
    async fn test_select_candidates_for_target_filtering() {
        let pool = RoutePool::new();
        pool.add_backend(dummy_backend("generic-adapter-a")).await;
        pool.add_backend(dummy_backend("external-cpa")).await;

        let (c_all, _, _) = pool.select_candidates(None).await;
        assert_eq!(c_all.len(), 2);

        let (c_wb, _, _) = pool
            .select_candidates_for_target(None, Some("generic-adapter-a"))
            .await;
        assert_eq!(c_wb.len(), 1);
        assert_eq!(c_wb[0].plugin_id, "generic-adapter-a");

        let (c_cpa, _, _) = pool
            .select_candidates_for_target(None, Some("external-cpa"))
            .await;
        assert_eq!(c_cpa.len(), 1);
        assert_eq!(c_cpa[0].plugin_id, "external-cpa");

        let (c_none, _, _) = pool
            .select_candidates_for_target(None, Some("nonexistent"))
            .await;
        assert!(c_none.is_empty());
    }

    #[tokio::test]
    async fn test_update_generation_rebinds_only_the_named_backend() {
        let pool = RoutePool::new();
        pool.add_backend(dummy_backend("b1").with_instance_generation(Some(1)))
            .await;
        pool.add_backend(dummy_backend("b2").with_instance_generation(Some(7)))
            .await;

        assert!(pool.update_generation("b1", Some(2)).await);
        assert!(!pool.update_generation("missing", Some(3)).await);

        let backends = pool.list_backends().await;
        assert_eq!(backends[0].instance_generation, Some(2));
        assert_eq!(backends[1].instance_generation, Some(7));
        assert!(backends[0].matches_generation(2));
        assert!(!backends[0].matches_generation(1));
    }

    #[tokio::test]
    async fn test_backend_with_instance_generation() {
        let b = dummy_backend("b1").with_instance_generation(Some(10));
        assert_eq!(b.instance_generation, Some(10));
        assert!(b.matches_generation(10));
        assert!(!b.matches_generation(9));

        let b_none = dummy_backend("b2");
        assert_eq!(b_none.instance_generation, None);
        assert!(b_none.matches_generation(999));
    }
}
