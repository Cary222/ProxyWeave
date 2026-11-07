//! Dynamic directory watcher and hot discovery for ProxyWeave plugins (spec v8).
//!
//! Enforces:
//! - Zero-DB: Fully in-memory state tracking without database engines.
//! - External Physical Isolation: External plugins are strictly read-only; no start/stop/kill mutation.
//! - Graceful Degradation: Corrupted, malformed, or unauthorized files are safely rejected without panicking.
//! - Directory scan: Discovers manifest files (*.manifest.json, *.manifest.yaml, manifest.json, manifest.yaml).
//! - Dynamic update: If manifest file is modified, in-memory plugin record is refreshed.

use crate::manifest::{Manifest, Ownership, MAX_MANIFEST_BYTES};
use crate::registry::{manifest_fingerprint, PluginRecord};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WatcherReport {
    pub total_discovered: usize,
    pub managed_count: usize,
    pub external_count: usize,
    pub rejected_count: usize,
    pub errors: Vec<String>,
}

#[derive(Clone)]
pub struct DirectoryWatcher {
    watch_dirs: Arc<RwLock<Vec<PathBuf>>>,
    // In-memory registry: plugin_id -> PluginRecord (Zero-DB)
    discovered: Arc<RwLock<BTreeMap<String, PluginRecord>>>,
}

impl Default for DirectoryWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl DirectoryWatcher {
    pub fn new() -> Self {
        Self {
            watch_dirs: Arc::new(RwLock::new(Vec::new())),
            discovered: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    pub fn with_paths(paths: Vec<PathBuf>) -> Self {
        let watcher = Self::new();
        for p in paths {
            watcher.add_watch_dir(p);
        }
        watcher
    }

    pub fn add_watch_dir(&self, dir: PathBuf) {
        let mut dirs = self.watch_dirs.write().unwrap();
        if !dirs.iter().any(|d| d == &dir) {
            dirs.push(dir);
        }
    }

    pub fn watch_dirs(&self) -> Vec<PathBuf> {
        self.watch_dirs.read().unwrap().clone()
    }

    /// Recursively find potential manifest files in a directory up to max depth.
    fn collect_manifest_files(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
        if depth > 4 || !dir.is_dir() {
            return;
        }
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                Self::collect_manifest_files(&path, depth + 1, out);
            } else if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name == "manifest.json"
                        || name == "manifest.yaml"
                        || name == "manifest.yml"
                        || name.ends_with(".manifest.json")
                        || name.ends_with(".manifest.yaml")
                        || name.ends_with(".manifest.yml")
                    {
                        out.push(path);
                    }
                }
            }
        }
    }

    /// Scans all configured directories for manifest files, validates them,
    /// and safely populates the in-memory discovered table.
    /// Non-compliant or corrupted manifests are gracefully recorded in the report without crashing.
    pub fn scan_all(&self) -> WatcherReport {
        let dirs = self.watch_dirs();
        let mut manifest_paths = Vec::new();
        for dir in &dirs {
            Self::collect_manifest_files(dir, 0, &mut manifest_paths);
        }

        let mut managed_count = 0;
        let mut external_count = 0;
        let mut rejected_count = 0;
        let mut errors = Vec::new();

        let mut new_discovered = BTreeMap::new();

        for path in manifest_paths {
            let file_data = match fs::read(&path) {
                Ok(data) => data,
                Err(e) => {
                    rejected_count += 1;
                    errors.push(format!("{}: cannot read file: {}", path.display(), e));
                    continue;
                }
            };

            if file_data.is_empty() || file_data.len() as u64 > MAX_MANIFEST_BYTES {
                rejected_count += 1;
                errors.push(format!(
                    "{}: manifest size outside 1..256 KiB (actual {} bytes)",
                    path.display(),
                    file_data.len()
                ));
                continue;
            }

            let manifest = match Manifest::parse(&file_data) {
                Ok(m) => m,
                Err(e) => {
                    rejected_count += 1;
                    errors.push(format!("{}: invalid manifest schema: {}", path.display(), e));
                    continue;
                }
            };

            let fingerprint = match manifest_fingerprint(&manifest) {
                Ok(f) => f,
                Err(e) => {
                    rejected_count += 1;
                    errors.push(format!("{}: fingerprint failed: {}", path.display(), e));
                    continue;
                }
            };

            match manifest.ownership() {
                Ownership::Managed => managed_count += 1,
                Ownership::External => external_count += 1,
            }

            let id = manifest.metadata.id.clone();
            new_discovered.insert(
                id,
                PluginRecord {
                    manifest,
                    fingerprint,
                    settings: BTreeMap::new(),
                    source: None,
                },
            );
        }

        let total_discovered = new_discovered.len();

        // Update in-memory registry safely
        {
            let mut disc = self.discovered.write().unwrap();
            *disc = new_discovered;
        }

        WatcherReport {
            total_discovered,
            managed_count,
            external_count,
            rejected_count,
            errors,
        }
    }

    /// Retrieve all dynamically discovered plugins from memory.
    pub fn list_discovered(&self) -> Vec<PluginRecord> {
        let disc = self.discovered.read().unwrap();
        let mut list: Vec<PluginRecord> = disc.values().cloned().collect();
        list.sort_by(|a, b| a.manifest.metadata.id.cmp(&b.manifest.metadata.id));
        list
    }

    /// Retrieve a single discovered plugin by id from memory.
    pub fn get_plugin(&self, id: &str) -> Option<PluginRecord> {
        let disc = self.discovered.read().unwrap();
        disc.get(id).cloned()
    }

    /// Check if a discovered plugin has external ownership.
    pub fn is_external(&self, id: &str) -> bool {
        let disc = self.discovered.read().unwrap();
        disc.get(id)
            .map(|p| p.manifest.ownership() == Ownership::External)
            .unwrap_or(false)
    }
}
