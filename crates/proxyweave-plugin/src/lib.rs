//! Local package contract and registration glue. Parsing/hash/temp/atomic-write mechanisms
//! are direct dependencies; see ADR-002. This crate never starts or terminates processes.
mod manifest;
mod registry;

pub use manifest::{
    Capabilities, Configuration, HealthCheck, Lifecycle, Manifest, Metadata, Network, Ownership,
    Runtime,
};
pub use registry::{ImportResult, PluginRecord, Registry};
