//! Local package contract, registration, and runtime management.
//!
//! Enforces:
//! - Manifest validation and schema checking (ADR-002)
//! - Win32 Job Object process containment (`KILL_ON_JOB_CLOSE`) (ADR-003)
//! - Sensitive config atomic projection with Windows Protected DACL
//! - Port conflict detection with explicit error distinction
//! - Health check probing with Busy (503) state recognition
//! - External service zero-kill protection

pub use registry::activation;
pub mod detector;
pub mod downloader;
pub mod job;
pub mod management;
pub mod manifest;
pub mod marketplace;
pub mod port;
pub mod probe;
pub mod projection;
pub mod registry;
pub mod runtime;
pub mod supervisor;
pub mod updater;

pub use detector::{detect_project, stage_and_import_project, DetectionReport};
pub use downloader::{download_and_install_release, DownloadProgress};
pub use job::JobObject;
pub use manifest::{
    Capabilities, Configuration, HealthCheck, Lifecycle, Management, ManagementPresentation,
    Manifest, Metadata, Network, Ownership, Runtime,
};
pub use marketplace::{
    get_builtin_catalog, get_marketplace_catalog, search_marketplace, MarketplaceCatalogResponse,
    MarketplacePluginItem,
};
pub use port::{check_port_available, PortConflictError};
pub use probe::{probe_http, ProbeReport, ProbeStatus};
pub use projection::{project_managed_configuration, ProjectedFiles, ProjectionContext};
pub use registry::{manifest_fingerprint, ImportResult, PluginRecord, Registry};
pub use runtime::{
    ActivePlugin, ManagedInstance, RequestLease, RuntimeError, RuntimeManager, RuntimeStatus,
};
pub use supervisor::{ManagedHandle, PluginError, PluginSupervisor};
pub use updater::{
    check_plugin_update, check_plugin_update_at, evaluate_release, DigestEvidence, Immutability,
    InstalledSource, PackageContract, PluginSource, SourceProvenance, UpdateCheck, UpdateStatus,
};
