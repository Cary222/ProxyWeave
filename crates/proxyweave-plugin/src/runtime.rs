//! Plugin runtime management with strict ownership boundaries,
//! Win32 Job Object process lifecycle binding, and External service zero-kill protection.
//!
//! Architecture Rules (ADR-003):
//! 1. External Providers are owned externally: ZERO kill signals, stop/restart calls are rejected.
//! 2. Managed Providers are owned by ProxyWeave: processes are assigned to a Win32 Job Object
//!    (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) ensuring child tree cleanup on host exit/crash.
//! 3. Port conflict must have explicit evidence (`AddrInUse`); probe timeout is never conflated.
//! 4. `wb2api` HTTP 503 with `X-Service: workbuddy2api` is evaluated as Busy/Unconfigured, not crash.
//! 5. Singleflight concurrency lock prevents multiple simultaneous starts of the same plugin.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};

use crate::job::JobObject;
use crate::manifest::{Ownership, Runtime};
use crate::port::{check_port_available, pick_unused_port, PortConflictError};
use crate::probe::{probe_http, ProbeStatus};
use crate::registry::PluginRecord;

use crate::projection::{project_managed_configuration, ProjectionContext};
use std::collections::BTreeMap;
#[derive(Debug, Clone)]
pub enum RuntimeError {
    ExternalServiceProtected(String),
    PortConflict(String),
    PluginNotFound(String),
    AlreadyRunning(String),
    ExecutableNotFound(String),
    ConfigProjectionFailed(String),
    SpawnFailed(String),
    ProbeTimeout(String),
    HealthCheckFailed(String),
    Io(String),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExternalServiceProtected(msg) => write!(f, "External service protected: {}", msg),
            Self::PortConflict(msg) => write!(f, "Port conflict: {}", msg),
            Self::PluginNotFound(msg) => write!(f, "Plugin not found: {}", msg),
            Self::AlreadyRunning(msg) => write!(f, "Plugin already running: {}", msg),
            Self::ExecutableNotFound(msg) => write!(f, "Executable not found: {}", msg),
            Self::ConfigProjectionFailed(msg) => write!(f, "Config projection failed: {}", msg),
            Self::SpawnFailed(msg) => write!(f, "Process spawn failed: {}", msg),
            Self::ProbeTimeout(msg) => write!(f, "Probe timeout: {}", msg),
            Self::HealthCheckFailed(msg) => write!(f, "Health check failed: {}", msg),
            Self::Io(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<PortConflictError> for RuntimeError {
    fn from(err: PortConflictError) -> Self {
        Self::PortConflict(err.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RuntimeStatus {
    Stopped,
    Starting,
    Ready {
        port: u16,
        pid: Option<u32>,
    },
    Busy {
        port: u16,
        pid: Option<u32>,
        reason: String,
    },
    Failed {
        error: String,
    },
}

/// Represents an active runtime for a Managed plugin.
pub struct ManagedInstance {
    pub plugin_id: String,
    pub port: u16,
    pub pid: u32,
    pub child: tokio::process::Child,
    pub job_object: JobObject,
    pub projected_config_path: Option<PathBuf>,
}

/// Represents state of an active plugin runtime (Managed or External).
pub enum ActivePlugin {
    Managed(Box<ManagedInstance>),
    External {
        plugin_id: String,
        port: u16,
        base_url: String,
    },
}

pub struct RuntimeEntry {
    pub status: RuntimeStatus,
    pub active: Option<ActivePlugin>,
}

#[derive(Clone, Default)]
pub struct RuntimeManager {
    runtimes: Arc<RwLock<HashMap<String, Arc<Mutex<RuntimeEntry>>>>>,
    start_locks: Arc<RwLock<HashMap<String, Arc<Mutex<()>>>>>,
}

impl RuntimeManager {
    pub fn new() -> Self {
        Self {
            runtimes: Arc::new(RwLock::new(HashMap::new())),
            start_locks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Gets or creates a singleflight startup lock for a specific plugin.
    async fn get_start_lock(&self, plugin_id: &str) -> Arc<Mutex<()>> {
        let mut locks = self.start_locks.write().await;
        locks
            .entry(plugin_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Reconcile Managed state with the OS; never treat a cached Ready PID as liveness.
    pub async fn get_status(&self, plugin_id: &str) -> RuntimeStatus {
        let entry_lock = self.runtimes.read().await.get(plugin_id).cloned();
        let Some(entry_lock) = entry_lock else {
            return RuntimeStatus::Stopped;
        };
        let mut entry = entry_lock.lock().await;
        if let Some(ActivePlugin::Managed(managed)) = entry.active.as_mut() {
            match managed.child.try_wait() {
                Ok(None) => {}
                Ok(Some(exit)) => {
                    entry.status = RuntimeStatus::Failed {
                        error: format!("Managed process exited: {exit}"),
                    };
                    // Closing the owned Job also retires any surviving descendants.
                    entry.active = None;
                }
                Err(error) => {
                    entry.status = RuntimeStatus::Failed {
                        error: format!("Cannot inspect Managed process: {error}"),
                    };
                    entry.active = None;
                }
            }
        } else if entry.status == RuntimeStatus::Starting {
            let lifecycle_lock = self.get_start_lock(plugin_id).await;
            if lifecycle_lock.try_lock().is_ok() {
                // An aborted start dropped its local child/Job and released this lock.
                entry.status = RuntimeStatus::Failed {
                    error: "Startup cancelled before publication".into(),
                };
            }
        }
        entry.status.clone()
    }
    /// Starts a plugin instance (Managed or External).
    /// Enforces Singleflight mutex: concurrent calls for the same plugin_id are serialized.
    pub async fn start_plugin(
        &self,
        record: &PluginRecord,
        package_dir: &Path,
        allocated_port: Option<u16>,
        data_dir: &Path,
        api_key: Option<&str>,
    ) -> Result<RuntimeStatus, RuntimeError> {
        let plugin_id = &record.manifest.metadata.id;
        let start_lock = self.get_start_lock(plugin_id).await;
        let _guard = start_lock.lock().await;

        // Check if already running / ready
        let current_status = self.get_status(plugin_id).await;
        if matches!(
            current_status,
            RuntimeStatus::Ready { .. } | RuntimeStatus::Busy { .. }
        ) {
            return Ok(current_status);
        }

        let entry_lock = {
            let mut runtimes = self.runtimes.write().await;
            runtimes
                .entry(plugin_id.clone())
                .or_insert_with(|| {
                    Arc::new(Mutex::new(RuntimeEntry {
                        status: RuntimeStatus::Stopped,
                        active: None,
                    }))
                })
                .clone()
        };
        {
            let mut entry = entry_lock.lock().await;
            // A failed Managed attempt must not survive a replacement spawn.
            if matches!(entry.active, Some(ActivePlugin::Managed(_))) {
                entry.active = None;
            }
            entry.status = RuntimeStatus::Starting;
        }
        let result = match record.manifest.ownership() {
            Ownership::External => self.start_external(record, allocated_port).await,
            Ownership::Managed => {
                let port = allocated_port.unwrap_or(8080);
                self.start_managed(record, package_dir, port, data_dir, api_key)
                    .await
            }
        };
        if let Err(error) = &result {
            entry_lock.lock().await.status = RuntimeStatus::Failed {
                error: error.to_string(),
            };
        }
        result
    }

    /// Starts an External plugin: validates URL and health probe, NO process spawned.
    async fn start_external(
        &self,
        record: &PluginRecord,
        allocated_port: Option<u16>,
    ) -> Result<RuntimeStatus, RuntimeError> {
        let plugin_id = &record.manifest.metadata.id;
        let endpoint = match &record.manifest.runtime {
            Runtime::External { endpoint } => endpoint.clone(),
            _ => return Err(RuntimeError::SpawnFailed("Plugin is not external".into())),
        };

        let port = allocated_port.unwrap_or_else(|| {
            url::Url::parse(&endpoint)
                .ok()
                .and_then(|u| u.port())
                .unwrap_or(8080)
        });

        let probe_url = format!(
            "{}{}",
            endpoint.trim_end_matches('/'),
            record.manifest.network.health_check.path
        );

        // Probe external service
        let report = probe_http(
            &probe_url,
            &record.manifest.network.health_check,
            Duration::from_secs(3),
        )
        .await;

        let status = match report.status {
            ProbeStatus::Ready => RuntimeStatus::Ready { port, pid: None },
            ProbeStatus::Busy { reason } => RuntimeStatus::Busy {
                port,
                pid: None,
                reason,
            },
            ProbeStatus::Starting => RuntimeStatus::Starting,
            ProbeStatus::Timeout => RuntimeStatus::Failed {
                error: format!("External service probe timed out: {}", report.detail),
            },
            ProbeStatus::Failed { error } => RuntimeStatus::Failed {
                error: format!("External service probe failed: {}", error),
            },
            ProbeStatus::Stopped => RuntimeStatus::Stopped,
        };

        let mut runtimes = self.runtimes.write().await;
        let entry_lock = runtimes
            .entry(plugin_id.clone())
            .or_insert_with(|| {
                Arc::new(Mutex::new(RuntimeEntry {
                    status: RuntimeStatus::Stopped,
                    active: None,
                }))
            })
            .clone();

        let mut entry = entry_lock.lock().await;
        entry.status = status.clone();
        entry.active = Some(ActivePlugin::External {
            plugin_id: plugin_id.clone(),
            port,
            base_url: endpoint,
        });

        Ok(status)
    }

    /// Starts a Managed plugin: pre-projects config with Windows Protected DACL,
    /// checks port availability, creates Win32 Job Object, spawns child, and probes health.
    async fn start_managed(
        &self,
        record: &PluginRecord,
        package_dir: &Path,
        port: u16,
        data_dir: &Path,
        api_key: Option<&str>,
    ) -> Result<RuntimeStatus, RuntimeError> {
        let plugin_id = &record.manifest.metadata.id;

        // Retry only proven bind conflicts, never infer conflict from a probe timeout.
        let mut port = port;
        let mut retries_left = record.manifest.lifecycle.max_port_retries;
        loop {
            match check_port_available(port) {
                Ok(()) => break,
                Err(PortConflictError::AddressInUse { .. }) if retries_left > 0 => {
                    retries_left -= 1;
                    port = pick_unused_port().map_err(|e| RuntimeError::Io(e.to_string()))?;
                }
                Err(error) => return Err(error.into()),
            }
        }
        // Locate executable from Managed runtime
        let (exe_rel, args, env_vars) = match &record.manifest.runtime {
            Runtime::Managed {
                executable,
                args,
                env,
                ..
            } => (executable.clone(), args.clone(), env.clone()),
            _ => return Err(RuntimeError::SpawnFailed("Plugin is not managed".into())),
        };

        let exe_path = package_dir.join(&exe_rel);
        if !exe_path.exists() {
            return Err(RuntimeError::ExecutableNotFound(format!(
                "Executable not found at {}",
                exe_path.display()
            )));
        }

        // Pre-project sensitive config if needed (e.g. for wb2api)
        let plugin_data_dir = data_dir.join("plugins").join(plugin_id);
        std::fs::create_dir_all(&plugin_data_dir)
            .map_err(|e| RuntimeError::ConfigProjectionFailed(e.to_string()))?;

        // Read all required secrets declared in manifest from OS Keyring
        let mut secrets = BTreeMap::new();
        for secret_name in &record.manifest.configuration.required_secrets {
            if let Ok(Some(secret_val)) =
                quotio_platform::get_plugin_credential(plugin_id, secret_name)
            {
                secrets.insert(secret_name.clone(), secret_val);
            }
        }

        // Generic projection context
        let projection_ctx =
            ProjectionContext::new(plugin_id, port, plugin_data_dir.clone(), api_key, secrets);

        // Generic configuration and credential files projection with Windows Protected DACL
        let projected = project_managed_configuration(&record.manifest, &projection_ctx)
            .map_err(RuntimeError::ConfigProjectionFailed)?;
        let projected_config_path = projected.config_path;

        let mut cmd = tokio::process::Command::new(&exe_path);
        cmd.current_dir(&plugin_data_dir);
        // Tokio owns abort/assignment-error cleanup before Job ownership is established.
        cmd.kill_on_drop(true);

        // Generic template rendering for process arguments
        for arg in &args {
            let rendered_arg = projection_ctx
                .render_str(arg)
                .map_err(RuntimeError::ConfigProjectionFailed)?;
            cmd.arg(rendered_arg);
        }

        // Generic template rendering for environment variables
        for (k, v) in &env_vars {
            let rendered_val = projection_ctx
                .render_str(v)
                .map_err(RuntimeError::ConfigProjectionFailed)?;
            cmd.env(k, rendered_val);
        }

        // Create Win32 Job Object before spawn
        let job = JobObject::create()
            .map_err(|e| RuntimeError::SpawnFailed(format!("Failed to create JobObject: {}", e)))?;

        // Spawn child
        let mut child = cmd
            .spawn()
            .map_err(|e| RuntimeError::SpawnFailed(format!("Failed to spawn child: {}", e)))?;
        let pid = child.id().unwrap_or(0);

        // Assign child to Job Object immediately
        job.assign_tokio_child(&child).map_err(|e| {
            RuntimeError::SpawnFailed(format!("Failed to assign child to JobObject: {}", e))
        })?;

        // Health probe loop
        let probe_path = &record.manifest.network.health_check.path;
        let probe_url = format!("http://127.0.0.1:{}{}", port, probe_path);

        let probe_start = std::time::Instant::now();
        let timeout = Duration::from_millis(record.manifest.lifecycle.startup_timeout_ms);

        let mut final_status = RuntimeStatus::Starting;
        while probe_start.elapsed() < timeout {
            if let Some(exit) = child
                .try_wait()
                .map_err(|e| RuntimeError::Io(e.to_string()))?
            {
                final_status = RuntimeStatus::Failed {
                    error: format!("Managed process exited during startup: {exit}"),
                };
                break;
            }
            let remaining = timeout.saturating_sub(probe_start.elapsed());
            let report = probe_http(
                &probe_url,
                &record.manifest.network.health_check,
                remaining.min(Duration::from_millis(500)),
            )
            .await;
            match report.status {
                ProbeStatus::Ready => {
                    final_status = RuntimeStatus::Ready {
                        port,
                        pid: Some(pid),
                    };
                    break;
                }
                ProbeStatus::Busy { reason } => {
                    final_status = RuntimeStatus::Busy {
                        port,
                        pid: Some(pid),
                        reason,
                    };
                    break;
                }
                ProbeStatus::Starting | ProbeStatus::Timeout => {
                    tokio::time::sleep(Duration::from_millis(150)).await;
                }
                ProbeStatus::Failed { .. } => {
                    // Could still be warming up
                    tokio::time::sleep(Duration::from_millis(150)).await;
                }
                ProbeStatus::Stopped => {
                    tokio::time::sleep(Duration::from_millis(150)).await;
                }
            }
        }

        if final_status == RuntimeStatus::Starting {
            final_status = RuntimeStatus::Failed {
                error: format!("Readiness probe timed out after {:?}", timeout),
            };
        }

        let instance = ManagedInstance {
            plugin_id: plugin_id.clone(),
            port,
            pid,
            child,
            job_object: job,
            projected_config_path,
        };

        let mut runtimes = self.runtimes.write().await;
        let entry_lock = runtimes
            .entry(plugin_id.clone())
            .or_insert_with(|| {
                Arc::new(Mutex::new(RuntimeEntry {
                    status: RuntimeStatus::Stopped,
                    active: None,
                }))
            })
            .clone();

        let mut entry = entry_lock.lock().await;
        entry.status = final_status.clone();
        entry.active = Some(ActivePlugin::Managed(Box::new(instance)));

        Ok(final_status)
    }

    /// Stops a plugin.
    /// CRITICAL IRON LAW: If the plugin is External, this explicitly rejects with
    /// `RuntimeError::ExternalServiceProtected` and sends ZERO kill signals!
    pub async fn stop_plugin(&self, plugin_id: &str) -> Result<(), RuntimeError> {
        // FIFO with start: stop settles after the in-flight attempt and cleans its owned child.
        let lifecycle_lock = self.get_start_lock(plugin_id).await;
        let _guard = lifecycle_lock.lock().await;
        let runtimes = self.runtimes.read().await;
        let entry_lock = runtimes
            .get(plugin_id)
            .ok_or_else(|| RuntimeError::PluginNotFound(format!("Plugin {} not found", plugin_id)))?
            .clone();
        drop(runtimes);

        let mut entry = entry_lock.lock().await;
        if let Some(active) = entry.active.take() {
            match active {
                ActivePlugin::External { ref plugin_id, .. } => {
                    let id = plugin_id.clone();
                    // Put it back
                    entry.active = Some(active);
                    Err(RuntimeError::ExternalServiceProtected(format!(
                        "External provider '{}' is owned outside ProxyWeave; refusing stop/kill signal",
                        id
                    )))
                }
                ActivePlugin::Managed(mut managed) => {
                    // Kill child and drop job object
                    let _ = managed.child.kill().await;
                    drop(managed.job_object);
                    if let Some(cfg) = managed.projected_config_path {
                        let _ = std::fs::remove_file(cfg);
                    }
                    entry.status = RuntimeStatus::Stopped;
                    Ok(())
                }
            }
        } else {
            entry.status = RuntimeStatus::Stopped;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_external_provider_refuses_stop_and_kill() {
        let manager = RuntimeManager::new();

        // Simulate an external entry
        {
            let mut runtimes = manager.runtimes.write().await;
            runtimes.insert(
                "ext_provider".to_string(),
                Arc::new(Mutex::new(RuntimeEntry {
                    status: RuntimeStatus::Ready {
                        port: 9999,
                        pid: None,
                    },
                    active: Some(ActivePlugin::External {
                        plugin_id: "ext_provider".to_string(),
                        port: 9999,
                        base_url: "http://127.0.0.1:9999".to_string(),
                    }),
                })),
            );
        }

        // Attempting to stop an external provider must fail with ExternalServiceProtected
        let err = manager
            .stop_plugin("ext_provider")
            .await
            .expect_err("external stop must fail");
        assert!(matches!(err, RuntimeError::ExternalServiceProtected(_)));
        println!("External stop rejection verified: {}", err);
    }
}
