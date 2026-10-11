use crate::manifest::{
    validate_id, validate_relative_path, validate_settings, MAX_MANIFEST_BYTES,
    MAX_PACKAGE_FILE_BYTES,
};
use crate::{Manifest, Runtime};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

const MAX_RECORD_BYTES: u64 = 512 * 1024;
const MAX_TOTAL_PACKAGE_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PluginRecord {
    pub manifest: Manifest,
    pub fingerprint: String,
    pub settings: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportResult {
    pub record: PluginRecord,
    pub already_registered: bool,
}

/// Single-instance desktop owns one registry. Clones share its mutation lock. Separate
/// processes still cannot overwrite imports: publication is a non-replacing directory rename.
#[derive(Clone)]
pub struct Registry {
    root: PathBuf,
    mutation: Arc<Mutex<()>>,
}

impl Registry {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, String> {
        let root = root.as_ref();
        reject_existing_links(root)?;
        fs::create_dir_all(root).map_err(|_| "cannot create registry directory")?;
        let root = root
            .canonicalize()
            .map_err(|_| "cannot resolve registry directory")?;
        for name in ["packages", "data"] {
            let p = root.join(name);
            reject_existing_links(&p)?;
            fs::create_dir_all(&p).map_err(|_| "cannot create registry subdirectory")?;
        }
        Ok(Self {
            root,
            mutation: Arc::new(Mutex::new(())),
        })
    }

    /// Source must be a manifest, not an arbitrary project directory. Only explicitly
    /// declared files are copied; no credentials, config, logs or source discovery occurs.
    pub fn import(&self, source: impl AsRef<Path>) -> Result<ImportResult, String> {
        let _lock = self
            .mutation
            .lock()
            .map_err(|_| "registry mutation lock poisoned")?;
        let source = source.as_ref();
        reject_existing_links(source)?;
        let source = source
            .canonicalize()
            .map_err(|_| "manifest file not found")?;
        let manifest = Manifest::parse(&read_bounded(&source, MAX_MANIFEST_BYTES)?)?;
        let fingerprint = manifest_fingerprint(&manifest)?;
        let package_root = source.parent().ok_or("manifest has no parent directory")?;
        let package_dir = self.package_dir(&manifest.metadata.id)?;
        // Validate source even for an idempotent import: a corrupt source package is not success.
        verify_package(package_root, &manifest)?;
        if package_dir.exists() {
            let record = self.load(&manifest.metadata.id)?;
            if record.fingerprint != fingerprint {
                return Err("plugin id already registered with different package contents".into());
            }
            return Ok(ImportResult {
                record,
                already_registered: true,
            });
        }
        let staged = tempfile::Builder::new()
            .prefix(".import-")
            .tempdir_in(self.root.join("packages"))
            .map_err(|_| "cannot create package staging directory")?;
        if let Runtime::Managed {
            executable,
            sha256,
            resources,
            ..
        } = &manifest.runtime
        {
            let mut total = copy_verified(package_root, staged.path(), executable, sha256)?;
            for (path, hash) in resources {
                total = total
                    .checked_add(copy_verified(package_root, staged.path(), path, hash)?)
                    .ok_or("package size overflow")?;
                if total > MAX_TOTAL_PACKAGE_BYTES {
                    return Err("package exceeds total size limit".into());
                }
            }
        }
        let record = PluginRecord {
            settings: manifest.configuration.settings.clone(),
            manifest,
            fingerprint,
        };
        let bytes =
            serde_json::to_vec_pretty(&record).map_err(|_| "cannot serialize plugin record")?;
        quotio_platform::atomic_write(&staged.path().join("record.json"), &bytes, false)
            .map_err(|_| "cannot persist plugin record")?;
        reject_existing_links(&self.root.join("packages"))?;
        fs::rename(staged.path(), &package_dir)
            .map_err(|_| "cannot publish package; existing registration was not overwritten")?;
        Ok(ImportResult {
            record,
            already_registered: false,
        })
    }

    pub fn list(&self) -> Result<Vec<PluginRecord>, String> {
        let mut records = Vec::new();
        reject_existing_links(&self.root.join("packages"))?;
        for entry in fs::read_dir(self.root.join("packages"))
            .map_err(|_| "cannot enumerate registered plugins")?
        {
            let entry = entry.map_err(|_| "cannot enumerate registry entry")?;
            let id = entry
                .file_name()
                .into_string()
                .map_err(|_| "invalid registry directory name")?;
            if id.starts_with(".import-") {
                continue;
            } // crashed staging is never a published record.
            records.push(self.load(&id)?);
        }
        records.sort_by(|a, b| a.manifest.metadata.id.cmp(&b.manifest.metadata.id));
        Ok(records)
    }

    pub fn load(&self, id: &str) -> Result<PluginRecord, String> {
        let dir = self.package_dir(id)?;
        let raw = read_bounded(&dir.join("record.json"), MAX_RECORD_BYTES)?;
        let record: PluginRecord =
            serde_json::from_slice(&raw).map_err(|_| "registered plugin record is corrupt")?;
        record.manifest.validate()?;
        if record.manifest.metadata.id != id
            || manifest_fingerprint(&record.manifest)? != record.fingerprint
        {
            return Err("registered plugin identity or manifest fingerprint mismatch".into());
        }
        validate_settings(&record.settings, &record.manifest.configuration.settings)?;
        verify_package(&dir, &record.manifest)?;
        Ok(record)
    }

    /// Only explicitly declared non-sensitive scalar settings enter the registration record.
    /// Credentials are deliberately absent; Feature 2 owns secure projection/Keyring APIs.
    pub fn save_settings(
        &self,
        id: &str,
        settings: BTreeMap<String, Value>,
    ) -> Result<PluginRecord, String> {
        let _lock = self
            .mutation
            .lock()
            .map_err(|_| "registry mutation lock poisoned")?;
        let mut record = self.load(id)?;
        validate_settings(&settings, &record.manifest.configuration.settings)?;
        record.settings = settings;
        let bytes =
            serde_json::to_vec_pretty(&record).map_err(|_| "cannot serialize plugin settings")?;
        quotio_platform::atomic_write(&self.package_dir(id)?.join("record.json"), &bytes, false)
            .map_err(|_| "cannot persist settings; prior registration remains authoritative")?;
        Ok(record)
    }

    pub fn package_dir(&self, id: &str) -> Result<PathBuf, String> {
        validate_id(id)?;
        let path = self.root.join("packages").join(id);
        reject_existing_links(&path)?;
        Ok(path)
    }

    pub fn data_dir(&self, id: &str) -> Result<PathBuf, String> {
        validate_id(id)?;
        let path = self.root.join("data").join(id);
        reject_existing_links(&path)?;
        Ok(path)
    }
}

fn manifest_fingerprint(manifest: &Manifest) -> Result<String, String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(manifest).map_err(|_| "cannot fingerprint manifest")?)
    ))
}

fn verify_package(root: &Path, manifest: &Manifest) -> Result<(), String> {
    if let Runtime::Managed {
        executable,
        sha256,
        resources,
        ..
    } = &manifest.runtime
    {
        let mut total = hash_file(&safe_file(root, executable)?, sha256)?;
        for (path, hash) in resources {
            total = total
                .checked_add(hash_file(&safe_file(root, path)?, hash)?)
                .ok_or("package size overflow")?;
            if total > MAX_TOTAL_PACKAGE_BYTES {
                return Err("package exceeds total size limit".into());
            }
        }
    }
    Ok(())
}

fn safe_file(root: &Path, relative: &str) -> Result<PathBuf, String> {
    validate_relative_path(relative)?;
    let root = root.canonicalize().map_err(|_| "package root missing")?;
    let path = root.join(relative);
    reject_existing_links(&path)?;
    let canonical = path
        .canonicalize()
        .map_err(|_| "declared package file missing")?;
    if !canonical.starts_with(&root) || !canonical.is_file() {
        return Err("declared file escapes package or is not a file".into());
    }
    Ok(canonical)
}

fn hash_file(path: &Path, expected: &str) -> Result<u64, String> {
    let mut f = fs::File::open(path).map_err(|_| "cannot read declared package file")?;
    let mut hash = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let count = f.read(&mut buf).map_err(|_| "package read failed")?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > MAX_PACKAGE_FILE_BYTES {
            return Err("package file exceeds 256 MiB limit".into());
        }
        hash.update(&buf[..count]);
    }
    if format!("{:x}", hash.finalize()) != expected {
        return Err("package SHA256 mismatch".into());
    }
    Ok(total)
}

fn copy_verified(
    source_root: &Path,
    target_root: &Path,
    relative: &str,
    expected: &str,
) -> Result<u64, String> {
    let source = safe_file(source_root, relative)?;
    let target = target_root.join(relative);
    fs::create_dir_all(target.parent().ok_or("missing package parent")?)
        .map_err(|_| "cannot create staged package directory")?;
    let mut input = fs::File::open(source).map_err(|_| "cannot open package source")?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)
        .map_err(|_| "staged package collision")?;
    let mut total = 0u64;
    let mut hash = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let count = input
            .read(&mut buf)
            .map_err(|_| "package copy read failed")?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > MAX_PACKAGE_FILE_BYTES {
            return Err("package file exceeds 256 MiB limit".into());
        }
        hash.update(&buf[..count]);
        output
            .write_all(&buf[..count])
            .map_err(|_| "package copy write failed")?;
    }
    output
        .sync_all()
        .map_err(|_| "cannot sync staged package")?;
    if format!("{:x}", hash.finalize()) != expected {
        return Err("package changed during copy or SHA256 mismatch".into());
    }
    Ok(total)
}

fn read_bounded(path: &Path, max: u64) -> Result<Vec<u8>, String> {
    reject_existing_links(path)?;
    if !path.is_file() {
        return Err("expected regular file".into());
    }
    let mut data = Vec::new();
    fs::File::open(path)
        .map_err(|_| "cannot read file")?
        .take(max + 1)
        .read_to_end(&mut data)
        .map_err(|_| "file read failed")?;
    if data.len() as u64 > max {
        return Err("file exceeds size limit".into());
    }
    Ok(data)
}

fn reject_existing_links(path: &Path) -> Result<(), String> {
    // Reject all existing ancestors, not only the final leaf (Windows junction/ADS boundary).
    for p in path.ancestors() {
        match fs::symlink_metadata(p) {
            Ok(m) => {
                if m.file_type().is_symlink() {
                    return Err("links are not allowed in package or registry paths".into());
                }
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if m.file_attributes() & 0x400 != 0 {
                        return Err("Windows reparse points are not allowed".into());
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("cannot inspect path ownership boundary".into()),
        }
    }
    Ok(())
}
