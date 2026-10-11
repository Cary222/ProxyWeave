//! Local package contract, registration, and runtime management.
//!
//! Enforces:
//! - Manifest validation and schema checking (ADR-002)
//! - Win32 Job Object process containment (`KILL_ON_JOB_CLOSE`) (ADR-003)
//! - Sensitive config atomic projection with Windows Protected DACL
//! - Port conflict detection with explicit error distinction
//! - Health check probing with Busy (503) state recognition
//! - External service zero-kill protection

pub mod job;
pub mod manifest;
pub mod port;
pub mod probe;
pub mod projection;
pub mod registry;
pub mod runtime;

pub use job::JobObject;
pub use manifest::{
    Capabilities, Configuration, HealthCheck, Lifecycle, Manifest, Metadata, Network, Ownership,
    Runtime,
};
pub use port::{check_port_available, PortConflictError};
pub use probe::{probe_http, ProbeReport, ProbeStatus};
pub use projection::{project_managed_configuration, ProjectedFiles, ProjectionContext};
pub use registry::{ImportResult, PluginRecord, Registry};
pub use runtime::{
    ActivePlugin, ManagedInstance, RequestLease, RuntimeError, RuntimeManager, RuntimeStatus,
};
