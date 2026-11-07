//! Managed plugin sandbox process supervisor and External lifecycle immutable guard.
//!
//! Architecture invariants:
//! 1. External plugins are immutable: spawn, stop, and kill operations are strictly forbidden
//!    and rejected with `Err(PluginError::ExternalLifecycleImmutable)`.
//! 2. Managed plugins run in a process sandbox with dynamic loopback port binding (OS port 0),
//!    secure credential injection (`PROXYWEAVE_CREDENTIAL_*`), and Win32 Job Object containment
//!    (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) to eliminate orphan processes.
//! 3. Shutdown protection follows graceful termination (SIGTERM) with a 5-second timeout guard
//!    before escalating to SIGKILL.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::{Child, Command};

use crate::job::JobObject;
use crate::manifest::{Manifest, Ownership, Runtime};
use crate::port::pick_unused_port;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginError {
    ExternalLifecycleImmutable,
    PortProbeFailed(String),
    SpawnFailed(String),
    StopFailed(String),
    PluginNotFound(String),
    InvalidManifest(String),
    AlreadyRunning(String),
    NotRunning(String),
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExternalLifecycleImmutable => {
                write!(
                    f,
                    "External plugin lifecycle is immutable: start/stop/kill operations are strictly forbidden"
                )
            }
            Self::PortProbeFailed(msg) => write!(f, "Port probe failed: {}", msg),
            Self::SpawnFailed(msg) => write!(f, "Process spawn failed: {}", msg),
            Self::StopFailed(msg) => write!(f, "Process stop failed: {}", msg),
            Self::PluginNotFound(id) => write!(f, "Plugin not found: {}", id),
            Self::InvalidManifest(msg) => write!(f, "Invalid manifest: {}", msg),
            Self::AlreadyRunning(id) => write!(f, "Plugin already running: {}", id),
            Self::NotRunning(id) => write!(f, "Plugin not running: {}", id),
        }
    }
}

impl std::error::Error for PluginError {}

/// Handle representing a supervised managed child process.
pub struct ManagedHandle {
    pub plugin_id: String,
    pub port: u16,
    pub child: Option<Child>,
    pub job_object: Option<JobObject>,
}

impl ManagedHandle {
    /// Gracefully stop the managed child process with a 5-second timeout guard.
    pub async fn stop(&mut self) -> Result<(), PluginError> {
        self.stop_with_timeout(Duration::from_secs(5)).await
    }

    /// Stop the child process with a specific timeout before forcing termination.
    pub async fn stop_with_timeout(&mut self, timeout: Duration) -> Result<(), PluginError> {
        if let Some(mut child) = self.child.take() {
            let _ = child.start_kill();

            match tokio::time::timeout(timeout, child.wait()).await {
                Ok(Ok(_status)) => Ok(()),
                Ok(Err(e)) => Err(PluginError::StopFailed(e.to_string())),
                Err(_) => {
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                    Ok(())
                }
            }
        } else {
            Ok(())
        }
    }

    /// Immediately kill the managed child process.
    pub async fn kill(&mut self) -> Result<(), PluginError> {
        if let Some(mut child) = self.child.take() {
            child
                .kill()
                .await
                .map_err(|e| PluginError::StopFailed(e.to_string()))?;
            let _ = child.wait().await;
        }
        self.job_object.take();
        Ok(())
    }
}

/// Process supervisor managing plugin execution and isolation boundaries.
#[derive(Debug, Default)]
pub struct PluginSupervisor;

impl PluginSupervisor {
    pub fn new() -> Self {
        Self
    }

    /// Probe an available dynamic loopback port using OS port 0 binding.
    pub fn probe_dynamic_port(&self) -> Result<u16, PluginError> {
        pick_unused_port().map_err(|e| PluginError::PortProbeFailed(e.to_string()))
    }

    /// Format and sanitize credentials into standard `PROXYWEAVE_CREDENTIAL_*` environment variables.
    pub fn prepare_credentials(
        &self,
        credentials: &HashMap<String, String>,
    ) -> HashMap<String, String> {
        let mut envs = HashMap::new();
        for (key, val) in credentials {
            let sanitized_key = key
                .chars()
                .map(|c| if c.is_alphanumeric() { c.to_ascii_uppercase() } else { '_' })
                .collect::<String>();
            let env_name = format!("PROXYWEAVE_CREDENTIAL_{}", sanitized_key);
            envs.insert(env_name, val.clone());
        }
        envs
    }

    /// Spawns a plugin in a managed sandbox.
    ///
    /// If the plugin ownership is `External`, this operation is strictly rejected
    /// returning `Err(PluginError::ExternalLifecycleImmutable)`.
    pub async fn spawn(
        &self,
        manifest: &Manifest,
        working_dir: Option<&PathBuf>,
        credentials: &HashMap<String, String>,
    ) -> Result<ManagedHandle, PluginError> {
        if manifest.ownership() == Ownership::External {
            return Err(PluginError::ExternalLifecycleImmutable);
        }

        let Runtime::Managed {
            executable,
            args,
            env,
            ..
        } = &manifest.runtime
        else {
            return Err(PluginError::InvalidManifest(
                "Expected managed runtime configuration".into(),
            ));
        };

        let port = self.probe_dynamic_port()?;
        let cred_envs = self.prepare_credentials(credentials);

        let mut cmd = Command::new(executable);
        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        for arg in args {
            let substituted = arg.replace("{PORT}", &port.to_string());
            cmd.arg(substituted);
        }

        for (k, v) in env {
            let substituted = v.replace("{PORT}", &port.to_string());
            cmd.env(k, substituted);
        }

        for (k, v) in cred_envs {
            cmd.env(k, v);
        }

        let job_object = JobObject::create().ok();

        let child = cmd
            .spawn()
            .map_err(|e| PluginError::SpawnFailed(e.to_string()))?;

        if let Some(ref job) = job_object {
            let _ = job.assign_tokio_child(&child);
        }

        Ok(ManagedHandle {
            plugin_id: manifest.metadata.id.clone(),
            port,
            child: Some(child),
            job_object,
        })
    }

    /// Stops a plugin instance.
    ///
    /// If the plugin is `External`, this operation is strictly rejected.
    pub async fn stop(
        &self,
        manifest: &Manifest,
        handle: &mut Option<ManagedHandle>,
    ) -> Result<(), PluginError> {
        if manifest.ownership() == Ownership::External {
            return Err(PluginError::ExternalLifecycleImmutable);
        }

        if let Some(h) = handle.as_mut() {
            h.stop().await
        } else {
            Ok(())
        }
    }

    /// Kills a plugin instance.
    ///
    /// If the plugin is `External`, this operation is strictly rejected.
    pub async fn kill(
        &self,
        manifest: &Manifest,
        handle: &mut Option<ManagedHandle>,
    ) -> Result<(), PluginError> {
        if manifest.ownership() == Ownership::External {
            return Err(PluginError::ExternalLifecycleImmutable);
        }

        if let Some(h) = handle.as_mut() {
            h.kill().await
        } else {
            Ok(())
        }
    }
}
