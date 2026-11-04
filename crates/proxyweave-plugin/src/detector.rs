use crate::manifest::{validate_relative_path, Manifest, Runtime, MAX_MANIFEST_BYTES};
use crate::registry::{
    copy_verified_mapped, hash_file, read_bounded, reject_existing_links, safe_file,
    verify_package, ImportResult, Registry, MAX_TOTAL_PACKAGE_BYTES,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const ADAPTERS: &[u8] = include_bytes!("../../../manifests/local-project-adapters.yaml");
const ASSETS: &[(&str, &[u8])] = &[(
    "wb2api",
    include_bytes!("../../../manifests/wb2api.manifest.yaml"),
)];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetectionReport {
    pub id: String,
    pub name: String,
    pub version: String,
    pub project_type: String,
    pub executable_relative_path: String,
    pub executable_sha256: String,
    pub manifest: Manifest,
    pub notes: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Adapter {
    manifest_asset: String,
    executable_sources: Vec<String>,
}

fn check_directory(dir: &Path) -> Result<(), String> {
    reject_existing_links(dir)?;
    if !dir.is_dir() {
        return Err("project directory missing or not a directory".into());
    }
    Ok(())
}

// Check even absent leaves/ancestors before probing; never follow links to select a layout.
fn declared_exists(dir: &Path, relative: &str) -> Result<bool, String> {
    validate_relative_path(relative)?;
    let path = dir.join(relative);
    reject_existing_links(&path)?;
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err("cannot inspect declared source file".into()),
    }
}

fn explicit_manifest(dir: &Path) -> Result<Option<Manifest>, String> {
    if !declared_exists(dir, "manifest.yaml")? {
        return Ok(None);
    }
    Manifest::parse(&read_bounded(
        &safe_file(dir, "manifest.yaml")?,
        MAX_MANIFEST_BYTES,
    )?)
    .map(Some)
}

fn report(manifest: Manifest, project_type: &str, source_executable: String) -> DetectionReport {
    let sha256 = match &manifest.runtime {
        Runtime::Managed { sha256, .. } => sha256.clone(),
        Runtime::External { .. } => String::new(),
    };
    DetectionReport {
        id: manifest.metadata.id.clone(),
        name: manifest.metadata.name.clone(),
        version: manifest.metadata.version.clone(),
        project_type: project_type.into(),
        executable_relative_path: source_executable,
        executable_sha256: sha256,
        manifest,
        notes: vec![
            "Only declared files are read; SHA256 is content integrity, not publisher identity or a sandbox".into(),
        ],
    }
}

fn canonical_executable(manifest: &Manifest) -> String {
    match &manifest.runtime {
        Runtime::Managed { executable, .. } => executable.clone(),
        Runtime::External { .. } => String::new(),
    }
}

pub fn detect_project(dir: impl AsRef<Path>) -> Result<DetectionReport, String> {
    detect_with_assets(dir.as_ref(), ADAPTERS, ASSETS)
}

// Injectable only within the crate (including the offline test harness), never a host API.
pub(crate) fn detect_with_assets(
    dir: &Path,
    adapters: &[u8],
    assets: &[(&str, &[u8])],
) -> Result<DetectionReport, String> {
    check_directory(dir)?;
    if let Some(manifest) = explicit_manifest(dir)? {
        verify_package(dir, &manifest)?;
        let executable = canonical_executable(&manifest);
        return Ok(report(manifest, "manifest", executable));
    }
    let adapters: Vec<Adapter> = serde_yaml_ng::from_slice(adapters)
        .map_err(|_| "invalid embedded local adapter catalog")?;
    let mut selected = None;
    for adapter in adapters {
        let bytes = assets
            .iter()
            .find(|(key, _)| *key == adapter.manifest_asset)
            .ok_or("unknown embedded manifest asset")?
            .1;
        let manifest = Manifest::parse(bytes)?;
        let Runtime::Managed {
            sha256, resources, ..
        } = &manifest.runtime
        else {
            return Err("directory adapter requires a Managed manifest".into());
        };
        let mut source = None;
        for relative in adapter.executable_sources {
            if declared_exists(dir, &relative)? && source.replace(relative).is_some() {
                return Err("ambiguous declared executable sources".into());
            }
        }
        if let Some(source) = source {
            if selected.is_some() {
                return Err("ambiguous local project adapters".into());
            }
            let mut total = hash_file(&safe_file(dir, &source)?, sha256)?;
            for (path, hash) in resources {
                total = total
                    .checked_add(hash_file(&safe_file(dir, path)?, hash)?)
                    .ok_or("package size overflow")?;
                if total > MAX_TOTAL_PACKAGE_BYTES {
                    return Err("package exceeds total size limit".into());
                }
            }
            selected = Some(report(manifest, "local_adapter", source));
        }
    }
    selected.ok_or_else(|| {
        "unsupported local project; an explicit manifest or audited artifact is required".into()
    })
}

pub fn stage_and_import_project(
    registry: &Registry,
    dir: impl AsRef<Path>,
    manifest_override: Option<Manifest>,
) -> Result<ImportResult, String> {
    stage_with_assets(registry, dir.as_ref(), manifest_override, ADAPTERS, ASSETS)
}

pub(crate) fn stage_with_assets(
    registry: &Registry,
    dir: &Path,
    manifest_override: Option<Manifest>,
    adapters: &[u8],
    assets: &[(&str, &[u8])],
) -> Result<ImportResult, String> {
    check_directory(dir)?;
    // An explicit manifest is authoritative, including errors; no fallback to a catalog.
    let (manifest, source_executable) = if let Some(manifest) = explicit_manifest(dir)? {
        let executable = canonical_executable(&manifest);
        (manifest, executable)
    } else if let Some(manifest) = manifest_override {
        manifest.validate()?;
        let executable = canonical_executable(&manifest);
        (manifest, executable)
    } else {
        let detected = detect_with_assets(dir, adapters, assets)?;
        (detected.manifest, detected.executable_relative_path)
    };
    let Runtime::Managed {
        executable,
        sha256,
        resources,
        ..
    } = &manifest.runtime
    else {
        return Err("External services cannot be staged as local Managed packages".into());
    };
    let staging = tempfile::Builder::new()
        .prefix(".stage-pkg-")
        .tempdir()
        .map_err(|_| "cannot create temporary package directory")?;
    let mut total =
        copy_verified_mapped(dir, staging.path(), &source_executable, executable, sha256)?;
    for (path, hash) in resources {
        total = total
            .checked_add(copy_verified_mapped(dir, staging.path(), path, path, hash)?)
            .ok_or("package size overflow")?;
        if total > MAX_TOTAL_PACKAGE_BYTES {
            return Err("package exceeds total size limit".into());
        }
    }
    let yaml = serde_yaml_ng::to_string(&manifest).map_err(|_| "cannot serialize manifest")?;
    let manifest_path = staging.path().join("manifest.yaml");
    fs::write(&manifest_path, yaml).map_err(|_| "cannot write staged manifest")?;
    registry.import(manifest_path)
}
