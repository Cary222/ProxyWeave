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
        publish_package(staged.path(), &package_dir)?;
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

// Windows retry loop extracted from upstream-rs 2.16.3
// src/utils/filesystem/safe_move.rs (rename_with_retry / is_retryable_windows_rename).
// Copyright (c) 2025 what386
// Permission is hereby granted, free of charge, to any person obtaining a copy of
// this software and associated documentation files (the "Software"), to deal in
// the Software without restriction, including without limitation the rights to
// use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
// Thin adaptation: validate boundaries before EVERY attempt; only publish a
// directory to an absent sibling. No copy/delete fallback, overwrite or ACL change.
// Twenty 50ms waits bound the retry delay to 1s (OS calls have their own latency).
#[cfg(windows)]
const WINDOWS_RENAME_RETRIES: usize = 20;
#[cfg(windows)]
const WINDOWS_RENAME_RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(50);

fn publish_package(from: &Path, to: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        publish_with_retry(from, to, || fs::rename(from, to), std::thread::sleep)
    }
    #[cfg(not(windows))]
    {
        validate_publication(from, to)?;
        fs::rename(from, to).map_err(|_| "cannot publish package".into())
    }
}

fn validate_publication(from: &Path, to: &Path) -> Result<(), String> {
    if from.parent().is_none() || from.parent() != to.parent() {
        return Err("package publication requires sibling directories".into());
    }
    reject_existing_links(from)?;
    reject_existing_links(to)?;
    if !fs::symlink_metadata(from)
        .map_err(|_| "cannot inspect staged package")?
        .is_dir()
    {
        return Err("staged package is not a directory".into());
    }
    match fs::symlink_metadata(to) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err("package publication target already exists".into()),
        Err(_) => Err("cannot inspect package publication target".into()),
    }
}

#[cfg(windows)]
fn publish_with_retry(
    from: &Path,
    to: &Path,
    mut rename: impl FnMut() -> std::io::Result<()>,
    mut pause: impl FnMut(std::time::Duration),
) -> Result<(), String> {
    for attempt in 0..=WINDOWS_RENAME_RETRIES {
        validate_publication(from, to)?;
        match rename() {
            Ok(()) => return Ok(()),
            Err(error)
                if attempt < WINDOWS_RENAME_RETRIES && is_retryable_windows_rename(&error) =>
            {
                pause(WINDOWS_RENAME_RETRY_DELAY);
            }
            Err(error) => {
                // Report only the bounded OS category, never source/destination paths.
                return Err(format!(
                    "cannot publish package (OS error {}); registration unchanged",
                    error.raw_os_error().unwrap_or(0)
                ));
            }
        }
    }
    unreachable!("bounded rename loop always returns")
}

#[cfg(windows)]
fn is_retryable_windows_rename(error: &std::io::Error) -> bool {
    use windows_sys::Win32::Foundation::{
        ERROR_ACCESS_DENIED, ERROR_LOCK_VIOLATION, ERROR_SHARING_VIOLATION,
    };
    matches!(error.raw_os_error(), Some(code)
        if code == ERROR_ACCESS_DENIED as i32
            || code == ERROR_SHARING_VIOLATION as i32
            || code == ERROR_LOCK_VIOLATION as i32)
}

#[cfg(all(test, windows))]
mod publication_tests {
    use super::*;
    use std::{io, time::Duration};

    fn staged() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let from = root.path().join("stage");
        let to = root.path().join("published");
        fs::create_dir(&from).unwrap();
        fs::write(from.join("payload"), b"original").unwrap();
        (root, from, to)
    }

    #[test]
    fn extracted_retry_codes_are_limited_to_windows_5_32_33() {
        for code in [5, 32, 33] {
            assert!(is_retryable_windows_rename(&io::Error::from_raw_os_error(
                code
            )));
        }
        for code in [2, 3, 17, 80, 112, 183] {
            assert!(!is_retryable_windows_rename(&io::Error::from_raw_os_error(
                code
            )));
        }
        assert!(!is_retryable_windows_rename(&io::Error::from(
            io::ErrorKind::PermissionDenied
        )));
    }

    #[test]
    fn transient_errors_retry_but_permanent_and_other_errors_are_bounded() {
        let (_root, from, to) = staged();
        let mut calls = 0;
        let mut waits = 0;
        publish_with_retry(
            &from,
            &to,
            || {
                calls += 1;
                if calls <= 3 {
                    Err(io::Error::from_raw_os_error([5, 32, 33][calls - 1]))
                } else {
                    fs::rename(&from, &to)
                }
            },
            |delay| {
                assert_eq!(delay, Duration::from_millis(50));
                waits += 1;
            },
        )
        .unwrap();
        assert_eq!((calls, waits), (4, 3));
        assert_eq!(fs::read(to.join("payload")).unwrap(), b"original");
        for code in [5, 32, 33, 2, 112] {
            let (_root, from, to) = staged();
            let mut calls = 0;
            let mut waits = 0;
            let error = publish_with_retry(
                &from,
                &to,
                || {
                    calls += 1;
                    Err(io::Error::from_raw_os_error(code))
                },
                |_| waits += 1,
            )
            .unwrap_err();
            let expected = if [5, 32, 33].contains(&code) {
                (21, 20)
            } else {
                (1, 0)
            };
            assert_eq!((calls, waits), expected);
            assert!(error.contains(&format!("OS error {code}")));
            assert!(!to.exists());
            assert_eq!(fs::read(from.join("payload")).unwrap(), b"original");
        }
    }

    #[test]
    fn appearing_target_stops_retry_without_overwrite() {
        let (_root, from, to) = staged();
        let mut calls = 0;
        let error = publish_with_retry(
            &from,
            &to,
            || {
                calls += 1;
                Err(io::Error::from_raw_os_error(5))
            },
            |_| {
                fs::create_dir(&to).unwrap();
                fs::write(to.join("sentinel"), b"competitor").unwrap();
            },
        )
        .unwrap_err();
        assert!(error.contains("already exists"));
        assert_eq!(calls, 1);
        assert_eq!(fs::read(to.join("sentinel")).unwrap(), b"competitor");
        assert!(from.join("payload").is_file());
    }

    #[test]
    fn sibling_and_changed_reparse_boundaries_fail_closed() {
        let (_root, from, to) = staged();
        assert!(publish_package(&from, &to.join("child"))
            .unwrap_err()
            .contains("sibling"));
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("sentinel"), b"untouched").unwrap();
        let mut calls = 0;
        let error = publish_with_retry(
            &from,
            &to,
            || {
                calls += 1;
                Err(io::Error::from_raw_os_error(5))
            },
            |_| {
                let status = std::process::Command::new("cmd")
                    .args(["/C", "mklink", "/J"])
                    .arg(&to)
                    .arg(outside.path())
                    .status()
                    .unwrap();
                assert!(status.success());
            },
        )
        .unwrap_err();
        assert!(error.contains("reparse") || error.contains("links"));
        assert_eq!(calls, 1);
        assert_eq!(
            fs::read(outside.path().join("sentinel")).unwrap(),
            b"untouched"
        );
        fs::remove_dir(&to).unwrap();
    }

    #[test]
    fn real_directory_handle_is_released_during_bounded_retry() {
        use std::os::windows::fs::OpenOptionsExt;
        let (_root, from, to) = staged();
        let held = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .custom_flags(0x02000000)
            .open(&from)
            .unwrap(); // FILE_FLAG_BACKUP_SEMANTICS
        let initial = fs::rename(&from, &to).unwrap_err();
        assert!(is_retryable_windows_rename(&initial), "{initial}");
        let release = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(150));
            drop(held);
        });
        publish_package(&from, &to).unwrap();
        release.join().unwrap();
        assert_eq!(fs::read(to.join("payload")).unwrap(), b"original");
    }

    #[test]
    fn permanent_directory_handle_fails_and_tempdir_cleanup_leaves_no_half_package() {
        use std::os::windows::fs::OpenOptionsExt;
        let (root, from, to) = staged();
        let held = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .custom_flags(0x02000000)
            .open(&from)
            .unwrap();
        let begin = std::time::Instant::now();
        assert!(publish_package(&from, &to).is_err());
        assert!(
            begin.elapsed() < Duration::from_secs(5),
            "bounded retry did not settle"
        );
        assert!(!to.exists());
        drop(held);
        fs::remove_dir_all(&from).unwrap();
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[test]
    fn permanent_owned_acl_denial_fails_without_publication_or_permission_bypass() {
        let (_root, from, to) = staged();
        let sid_output = std::process::Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-Command",
                "[System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value",
            ])
            .output()
            .unwrap();
        assert!(sid_output.status.success());
        let sid = String::from_utf8(sid_output.stdout)
            .unwrap()
            .trim()
            .to_owned();
        struct Denial(Vec<PathBuf>, String);
        impl Drop for Denial {
            fn drop(&mut self) {
                for path in &self.0 {
                    let status = std::process::Command::new("icacls.exe")
                        .arg(path)
                        .args(["/remove:d", &format!("*{}", self.1)])
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .status()
                        .unwrap();
                    assert!(status.success(), "owned ACL restoration failed");
                }
            }
        }
        let denial = Denial(
            vec![from.parent().unwrap().to_path_buf(), from.clone()],
            sid.clone(),
        );
        for (path, right) in [(&denial.0[0], "DC"), (&denial.0[1], "D")] {
            let status = std::process::Command::new("icacls.exe")
                .arg(path)
                .args(["/deny", &format!("*{sid}:({right})")])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .unwrap();
            assert!(status.success());
        }
        let initial = fs::rename(&from, &to).unwrap_err();
        assert_eq!(initial.raw_os_error(), Some(5));
        let begin = std::time::Instant::now();
        let result = publish_package(&from, &to);
        let elapsed = begin.elapsed();
        drop(denial); // restore only this test's owned paths, before assertions/cleanup
        assert!(result.unwrap_err().contains("OS error 5"));
        assert!(elapsed < Duration::from_secs(5));
        assert!(!to.exists());
        assert_eq!(fs::read(from.join("payload")).unwrap(), b"original");
    }

    #[test]
    fn ancestor_reparse_change_is_rechecked_before_next_attempt() {
        let root = tempfile::tempdir().unwrap();
        let parent = root.path().join("packages");
        fs::create_dir(&parent).unwrap();
        let from = parent.join("stage");
        let to = parent.join("published");
        fs::create_dir(&from).unwrap();
        let parked = root.path().join("parked");
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir(outside.path().join("stage")).unwrap();
        fs::write(outside.path().join("sentinel"), b"untouched").unwrap();
        let mut calls = 0;
        let result = publish_with_retry(
            &from,
            &to,
            || {
                calls += 1;
                Err(io::Error::from_raw_os_error(5))
            },
            |_| {
                fs::rename(&parent, &parked).unwrap();
                assert!(std::process::Command::new("cmd")
                    .args(["/C", "mklink", "/J"])
                    .arg(&parent)
                    .arg(outside.path())
                    .status()
                    .unwrap()
                    .success());
            },
        );
        fs::remove_dir(&parent).unwrap();
        let error = result.unwrap_err();
        assert!(
            error == "Windows reparse points are not allowed"
                || error == "links are not allowed in package or registry paths",
            "unexpected boundary failure: {error}"
        );
        assert_eq!(calls, 1);
        assert_eq!(
            fs::read(outside.path().join("sentinel")).unwrap(),
            b"untouched"
        );
        assert!(!outside.path().join("published").exists());
    }

    #[tokio::test]
    async fn aborting_blocking_waiter_does_not_leave_infinite_retry() {
        // spawn_blocking cannot be forcibly cancelled once running; prove the
        // worker still settles its finite retry and owned tempdir is reclaimed.
        let (root, from, to) = staged();
        let root_path = root.path().to_path_buf();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (done_tx, done_rx) = tokio::sync::oneshot::channel();
        let worker = tokio::task::spawn_blocking(move || {
            let _ = started_tx.send(());
            let result = publish_with_retry(
                &from,
                &to,
                || Err(io::Error::from_raw_os_error(5)),
                std::thread::sleep,
            );
            drop(root);
            let _ = done_tx.send(result.is_err());
        });
        started_rx.await.unwrap();
        worker.abort();
        assert!(tokio::time::timeout(Duration::from_secs(5), done_rx)
            .await
            .unwrap()
            .unwrap());
        assert!(!root_path.exists());
        let _ = worker.await;
    }
}
