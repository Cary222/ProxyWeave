//! GitHub Release asset downloader, SHA256 integrity verifier, and secure archive unpacker.
//!
//! Portions derived from quotio-core/src/proxy_download.rs
//! Copyright (c) 2025 Trong Nguyen (MIT License)
//! Adapted for ProxyWeave plugin system release downloader and secure unpacker.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::manifest::{validate_id, validate_relative_path, Manifest, MAX_MANIFEST_BYTES};
use crate::registry::{read_bounded, safe_file, verify_package, ImportResult, Registry};

/// Network metadata: 2 MiB; checksum text: 1 MiB. Archive and expanded content
/// each share Registry's 512 MiB ceiling; at most 4096 archive entries.
pub const MAX_RELEASE_JSON_BYTES: u64 = 2 * 1024 * 1024;
pub const MAX_CHECKSUM_BYTES: u64 = 1024 * 1024;
pub const MAX_ARCHIVE_BYTES: u64 = crate::registry::MAX_TOTAL_PACKAGE_BYTES;
pub const MAX_ARCHIVE_ENTRIES: usize = 4096;

pub async fn read_response_bounded(
    mut response: reqwest::Response,
    limit: u64,
) -> Result<Vec<u8>, String> {
    if !response.status().is_success() {
        return Err(format!("HTTP 失败: {}", response.status()));
    }
    if response.content_length().is_some_and(|n| n > limit) {
        return Err("response exceeds size limit".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| "response read failed")? {
        if chunk.len() as u64 > limit.saturating_sub(bytes.len() as u64) {
            return Err("response exceeds size limit".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

/// Download and installation progress payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DownloadProgress {
    pub plugin_id: String,
    pub stage: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub percent: f64,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub tag_name: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseAsset {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub digest: Option<String>,
    pub name: String,
    pub browser_download_url: String,
    #[serde(default)]
    pub size: u64,
}

/// Parse owner and repo from a GitHub URL or "owner/repo" shorthand.
pub fn parse_github_owner_repo(repo_url: &str) -> Result<(String, String), String> {
    let raw = repo_url.trim();
    let path = if let Some(rest) = raw.strip_prefix("git@github.com:") {
        rest.to_string()
    } else if raw.contains("://") {
        let url = url::Url::parse(raw).map_err(|_| "invalid GitHub repository")?;
        if url.scheme() != "https"
            || url.host_str() != Some("github.com")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.port().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err("invalid GitHub repository".into());
        }
        url.path().trim_start_matches('/').to_string()
    } else {
        raw.to_string()
    };
    let path = path.strip_suffix(".git").unwrap_or(&path);
    let parts: Vec<_> = path.split('/').collect();
    if parts.len() != 2
        || !crate::updater::valid_repository_part(parts[0], true)
        || !crate::updater::valid_repository_part(parts[1], false)
    {
        return Err("invalid GitHub repository".into());
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// Build an HTTP client with optional proxy configuration and timeout bounds.
pub fn build_client(proxy_url: Option<&str>) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(300))
        .user_agent("proxyweave-desktop");

    let chosen = proxy_url
        .map(str::trim)
        .filter(|url| !url.is_empty())
        .map(ToOwned::to_owned)
        .or_else(proxy_from_env);

    if let Some(url) = chosen {
        if let Ok(proxy) = reqwest::Proxy::all(&url) {
            builder = builder.proxy(proxy);
        }
    }

    builder
        .build()
        .map_err(|_| "HTTP client construction failed".to_string())
}

/// Fallback to proxy environment variables.
pub fn proxy_from_env() -> Option<String> {
    for key in [
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
        "ALL_PROXY",
        "all_proxy",
    ] {
        if let Ok(value) = std::env::var(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

/// Fetch GitHub release metadata by tag or latest.
pub async fn fetch_release_metadata(
    client: &reqwest::Client,
    owner: &str,
    repo: &str,
    tag: Option<&str>,
) -> Result<Release, String> {
    if !crate::updater::valid_repository_part(owner, true)
        || !crate::updater::valid_repository_part(repo, false)
    {
        return Err("invalid GitHub repository".into());
    }
    let mut url = url::Url::parse(&format!(
        "https://api.github.com/repos/{owner}/{repo}/releases/"
    ))
    .map_err(|_| "invalid release endpoint")?;
    match tag {
        Some(t) if crate::updater::valid_tag(t) => {
            url.path_segments_mut()
                .map_err(|_| "invalid release endpoint")?
                .pop_if_empty()
                .push("tags")
                .push(t);
        }
        Some(_) => return Err("invalid release tag".into()),
        None => {
            url.path_segments_mut()
                .map_err(|_| "invalid release endpoint")?
                .pop_if_empty()
                .push("latest");
        }
    }
    let resp = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| "release request failed")?;
    let bytes = read_response_bounded(resp, MAX_RELEASE_JSON_BYTES).await?;
    serde_json::from_slice(&bytes).map_err(|_| "invalid release JSON".into())
}

/// Match a release asset name to a specific OS and architecture target.
pub fn asset_matches_platform_target(name: &str, target_os: &str, target_arch: &str) -> bool {
    let lower = name.to_lowercase();
    if lower.contains("checksum")
        || lower.ends_with(".sha256")
        || lower.ends_with(".sig")
        || lower.ends_with(".txt")
        || lower.ends_with(".md")
    {
        return false;
    }

    let os_ok = match target_os {
        "windows" => lower.contains("windows") || lower.contains("win"),
        "macos" => lower.contains("darwin") || lower.contains("macos") || lower.contains("apple"),
        "linux" => lower.contains("linux"),
        _ => false,
    };

    let arch_ok = match target_arch {
        "x86_64" => lower.contains("amd64") || lower.contains("x86_64") || lower.contains("x64"),
        "aarch64" => lower.contains("arm64") || lower.contains("aarch64"),
        _ => true,
    };

    let ext_ok = if target_os == "windows" {
        lower.ends_with(".zip")
            || lower.ends_with(".exe")
            || lower.ends_with(".tar.gz")
            || lower.ends_with(".tgz")
    } else {
        lower.ends_with(".zip") || lower.ends_with(".tar.gz") || lower.ends_with(".tgz")
    };

    os_ok && arch_ok && ext_ok
}

/// Match a release asset name to the current runtime platform.
pub fn asset_matches_platform(name: &str) -> bool {
    asset_matches_platform_target(name, std::env::consts::OS, std::env::consts::ARCH)
}

/// Find the best matching release asset for the current platform.
pub fn find_matching_asset(assets: &[ReleaseAsset]) -> Option<&ReleaseAsset> {
    assets
        .iter()
        .find(|asset| asset_matches_platform(&asset.name))
}

/// Parse checksum file content, supporting both per-asset single hash files and multi-asset checksums lists.
pub fn parse_checksums_content(
    content: &str,
    asset_name: &str,
    is_per_asset: bool,
) -> Option<String> {
    let want_base = asset_name.rsplit(['/', '\\']).next().unwrap_or(asset_name);

    if is_per_asset {
        return content
            .split_whitespace()
            .next()
            .filter(|val| crate::updater::valid_digest(val))
            .map(|val| val.to_lowercase());
    }

    for line in content.lines() {
        let mut parts = line.split_whitespace();
        let hash = match parts.next() {
            Some(v) if !v.is_empty() => v,
            _ => continue,
        };
        let file = match parts.last() {
            Some(v) => v.trim_start_matches('*'),
            None => continue,
        };
        let file_base = file.rsplit(['/', '\\']).next().unwrap_or(file);
        if file_base == want_base && crate::updater::valid_digest(hash) {
            return Some(hash.to_lowercase());
        }
    }

    None
}

/// Attempt to find and download the expected SHA256 from release checksum assets.
pub async fn find_expected_sha256(
    release: &Release,
    client: &reqwest::Client,
    asset_name: &str,
) -> Option<String> {
    expected_digest(release, client, asset_name)
        .await
        .ok()
        .map(|(hash, _)| hash)
}

pub async fn expected_digest(
    release: &Release,
    client: &reqwest::Client,
    asset_name: &str,
) -> Result<(String, crate::updater::DigestEvidence), String> {
    use crate::updater::{valid_digest, DigestEvidence};
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or("asset missing")?;
    if let Some(raw) = asset.digest.as_deref() {
        let hash = raw
            .strip_prefix("sha256:")
            .filter(|h| valid_digest(h))
            .ok_or("invalid or unsupported asset digest")?;
        return Ok((
            hash.to_ascii_lowercase(),
            DigestEvidence::ReleaseAssetDigest,
        ));
    }
    let per_asset = format!("{asset_name}.sha256");
    let checksums: Vec<_> = release
        .assets
        .iter()
        .filter(|a| a.name.eq_ignore_ascii_case(&per_asset))
        .collect();
    let checksums: Vec<_> = if checksums.is_empty() {
        release
            .assets
            .iter()
            .filter(|a| a.name.to_ascii_lowercase().contains("checksum"))
            .collect()
    } else {
        checksums
    };
    if checksums.len() != 1 {
        return Err("missing or ambiguous expected SHA256".into());
    }
    let checksum = checksums[0];
    validate_asset_filename(&checksum.name)?;
    if checksum.id == 0 || checksum.id == asset.id {
        return Err("invalid checksum asset identity".into());
    }
    let response = client
        .get(&checksum.browser_download_url)
        .send()
        .await
        .map_err(|_| "checksum request failed")?;
    let bytes = read_response_bounded(response, MAX_CHECKSUM_BYTES).await?;
    let text = std::str::from_utf8(&bytes).map_err(|_| "invalid checksum text")?;
    let hash = parse_checksums_content(
        text,
        asset_name,
        checksum.name.eq_ignore_ascii_case(&per_asset),
    )
    .filter(|h| valid_digest(h))
    .ok_or("missing or invalid expected SHA256")?;
    Ok((
        hash,
        DigestEvidence::ChecksumAsset {
            asset_id: checksum.id,
            asset_name: checksum.name.clone(),
        },
    ))
}

pub fn validate_release_asset_url(
    raw: &str,
    owner: &str,
    repo: &str,
    tag: &str,
    name: &str,
) -> Result<(), String> {
    let url = url::Url::parse(raw).map_err(|_| "invalid release asset URL")?;
    let segments: Vec<_> = url
        .path_segments()
        .ok_or("invalid release asset URL")?
        .collect();
    // Construct encoded segments with url rather than interpolating tag/filename paths.
    let mut expected =
        url::Url::parse("https://github.com/").map_err(|_| "invalid release asset URL")?;
    expected
        .path_segments_mut()
        .map_err(|_| "invalid release asset URL")?
        .clear()
        .extend([owner, repo, "releases", "download", tag, name]);
    if url.scheme() != "https"
        || url.host_str() != Some("github.com")
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || segments.len() != 6
        || url.path() != expected.path()
    {
        return Err("invalid release asset URL".into());
    }
    Ok(())
}
/// Compute SHA256 hexadecimal string of a file.
pub fn calculate_file_sha256(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|e| format!("无法读取文件以计算哈希: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| format!("读取文件内容失败: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Set executable permission (0o755) on Unix platforms.
pub fn set_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(path) {
            let mut perms = meta.permissions();
            perms.set_mode(perms.mode() | 0o755);
            let _ = fs::set_permissions(path, perms);
        }
    }
    #[cfg(not(unix))]
    let _ = path;
}

/// Securely extract a `.zip` archive with strict ZipSlip protection.
pub fn extract_zip_secure(archive_path: &Path, target_dir: &Path) -> Result<(), String> {
    let file = fs::File::open(archive_path).map_err(|e| format!("打开 ZIP 文件失败: {e}"))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("读取 ZIP 归档失败: {e}"))?;
    if zip.len() > MAX_ARCHIVE_ENTRIES {
        return Err(format!("ZIP 归档条目数超过上限 ({MAX_ARCHIVE_ENTRIES})"));
    }

    let mut total_extracted: u64 = 0;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| format!("读取 ZIP 条目失败: {e}"))?;
        let entry_name = entry.name().to_string();

        // ZipSlip check: reject parent directory or absolute prefix
        if entry_name.contains("..") || entry_name.starts_with('/') || entry_name.starts_with('\\')
        {
            return Err(format!("检测到恶意路径遍历 (ZipSlip): {}", entry_name));
        }

        let enclosed = match entry.enclosed_name() {
            Some(path) => path.to_path_buf(),
            None => return Err(format!("检测到非法相对路径 (ZipSlip): {}", entry_name)),
        };

        for component in enclosed.components() {
            if matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            ) {
                return Err(format!("检测到恶意路径组件 (ZipSlip): {}", entry_name));
            }
        }

        let out_path = target_dir.join(&enclosed);

        if entry.is_dir() {
            fs::create_dir_all(&out_path).map_err(|e| format!("创建解压目录失败: {e}"))?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("创建父目录失败: {e}"))?;
            }
            let mut outfile =
                fs::File::create(&out_path).map_err(|e| format!("创建解压文件失败: {e}"))?;
            let remaining = MAX_ARCHIVE_BYTES.saturating_sub(total_extracted);
            let mut bounded = std::io::Read::take(&mut entry, remaining + 1);
            let copied = std::io::copy(&mut bounded, &mut outfile)
                .map_err(|e| format!("解压文件写入失败: {e}"))?;
            total_extracted = total_extracted
                .checked_add(copied)
                .ok_or_else(|| "解压总大小溢出".to_string())?;
            if total_extracted > MAX_ARCHIVE_BYTES {
                return Err(format!("解压总大小超过上限 ({MAX_ARCHIVE_BYTES} 字节)"));
            }

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = entry.unix_mode() {
                    let _ = fs::set_permissions(&out_path, fs::Permissions::from_mode(mode));
                } else {
                    set_executable(&out_path);
                }
            }
        }
    }

    Ok(())
}

/// Securely extract a `.tar.gz` archive with strict ZipSlip protection.
pub fn extract_tar_gz_secure(archive_path: &Path, target_dir: &Path) -> Result<(), String> {
    let file = fs::File::open(archive_path).map_err(|e| format!("打开 tar.gz 文件失败: {e}"))?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut tar = tar::Archive::new(decoder);

    let entries = tar
        .entries()
        .map_err(|e| format!("读取 tar 归档条目失败: {e}"))?;
    let mut entry_count: usize = 0;
    let mut total_extracted: u64 = 0;
    for entry in entries {
        entry_count += 1;
        if entry_count > MAX_ARCHIVE_ENTRIES {
            return Err(format!("tar 归档条目数超过上限 ({MAX_ARCHIVE_ENTRIES})"));
        }
        let mut entry = entry.map_err(|e| format!("读取 tar 条目失败: {e}"))?;
        let path = entry
            .path()
            .map_err(|e| format!("读取条目路径失败: {e}"))?
            .to_path_buf();
        let path_str = path.to_string_lossy().to_string();

        if path_str.contains("..") || path_str.starts_with('/') || path_str.starts_with('\\') {
            return Err(format!("检测到恶意路径遍历 (ZipSlip): {}", path_str));
        }

        for component in path.components() {
            if matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            ) {
                return Err(format!("检测到恶意路径组件 (ZipSlip): {}", path_str));
            }
        }

        let out_path = target_dir.join(&path);

        if entry.header().entry_type() == tar::EntryType::Directory {
            fs::create_dir_all(&out_path).map_err(|e| format!("创建解压目录失败: {e}"))?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("创建父目录失败: {e}"))?;
            }
            let mut outfile =
                fs::File::create(&out_path).map_err(|e| format!("创建解压文件失败: {e}"))?;
            let remaining = MAX_ARCHIVE_BYTES.saturating_sub(total_extracted);
            let mut bounded = std::io::Read::take(&mut entry, remaining + 1);
            let copied = std::io::copy(&mut bounded, &mut outfile)
                .map_err(|e| format!("解压文件写入失败: {e}"))?;
            total_extracted = total_extracted
                .checked_add(copied)
                .ok_or_else(|| "解压总大小溢出".to_string())?;
            if total_extracted > MAX_ARCHIVE_BYTES {
                return Err(format!("解压总大小超过上限 ({MAX_ARCHIVE_BYTES} 字节)"));
            }

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(mode) = entry.header().mode() {
                    let _ = fs::set_permissions(&out_path, fs::Permissions::from_mode(mode));
                } else {
                    set_executable(&out_path);
                }
            }
        }
    }

    Ok(())
}

/// Accept a portable single filename, never a path or a Windows device alias.
pub fn validate_asset_filename(name: &str) -> Result<(), String> {
    validate_relative_path(name).map_err(|e| format!("无效资产文件名: {e}"))?;
    if name.contains('/') {
        return Err("无效资产文件名: 必须是单个文件名".into());
    }
    // Windows also resolves device stems with spaces before the extension and
    // recognizes superscript 1/2/3 in COM/LPT aliases. Do not silently rename.
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches(' ');
    let upper = stem.to_ascii_uppercase();
    let numbered_device = upper
        .strip_prefix("COM")
        .or_else(|| upper.strip_prefix("LPT"))
        .is_some_and(|n| {
            matches!(
                n,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        });
    if matches!(
        upper.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || numbered_device
    {
        return Err("无效资产文件名: Windows 保留设备名".into());
    }
    Ok(())
}

/// Securely extract an archive based on its filename extension.
pub fn extract_archive_secure(
    archive_path: &Path,
    target_dir: &Path,
    asset_name: &str,
) -> Result<(), String> {
    validate_asset_filename(asset_name)?;
    let lower = asset_name.to_lowercase();
    if lower.ends_with(".zip") {
        extract_zip_secure(archive_path, target_dir)
    } else if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        extract_tar_gz_secure(archive_path, target_dir)
    } else if lower.ends_with(".exe") {
        let meta =
            fs::metadata(archive_path).map_err(|e| format!("读取可执行文件元数据失败: {e}"))?;
        if meta.len() > MAX_ARCHIVE_BYTES {
            return Err(format!("可执行文件大小超过上限 ({MAX_ARCHIVE_BYTES} 字节)"));
        }
        let dest = target_dir.join(asset_name);
        fs::copy(archive_path, &dest).map_err(|e| format!("复制可执行文件失败: {e}"))?;
        set_executable(&dest);
        Ok(())
    } else {
        Err(format!(
            "暂不支持自动解包该资产格式（{}）。仅支持 .zip / .tar.gz / .exe。",
            asset_name
        ))
    }
}

/// Validate an explicit package contract before any registry publication.
/// The release version is not a substitute for package metadata.
pub fn ensure_staging_manifest(
    staging_dir: &Path,
    plugin_id: &str,
    _version: &str,
) -> Result<PathBuf, String> {
    validate_id(plugin_id)?;
    let root_manifest = staging_dir.join("manifest.yaml");
    let package_root = match fs::symlink_metadata(&root_manifest) {
        Ok(_) => staging_dir.to_path_buf(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Only a single wrapper directory is supported. Root contract errors
            // must never be hidden by selecting another contract or inventing one.
            let mut subdirs = Vec::new();
            for entry in
                fs::read_dir(staging_dir).map_err(|e| format!("读取 staging 目录失败: {e}"))?
            {
                let entry = entry.map_err(|e| format!("读取 staging 条目失败: {e}"))?;
                if entry
                    .file_type()
                    .map_err(|e| format!("读取 staging 文件类型失败: {e}"))?
                    .is_dir()
                {
                    subdirs.push(entry.path());
                }
            }
            if subdirs.len() != 1 {
                return Err("不支持无显式 manifest.yaml 的发布包".into());
            }
            subdirs.remove(0)
        }
        Err(e) => return Err(format!("读取 manifest.yaml 元数据失败: {e}")),
    };
    let manifest_path = safe_file(&package_root, "manifest.yaml")
        .map_err(|e| format!("无效或缺失 manifest.yaml: {e}"))?;
    let bytes = read_bounded(&manifest_path, MAX_MANIFEST_BYTES)?;
    let manifest = Manifest::parse(&bytes)?;
    if manifest.metadata.id != plugin_id {
        return Err(format!(
            "所选插件 ID ({plugin_id}) 与 manifest.yaml ID ({}) 不匹配",
            manifest.metadata.id
        ));
    }
    verify_package(&package_root, &manifest)?;
    Ok(manifest_path)
}

/// Downloaded, verified archive retained until the caller publishes or drops it.
pub struct DownloadedPackage {
    _directory: tempfile::TempDir,
    pub manifest_path: PathBuf,
    pub source: crate::updater::InstalledSource,
}

pub async fn download_and_install_release<F>(
    registry: &Registry,
    repo_url: &str,
    tag: Option<&str>,
    plugin_id: &str,
    expected_sha256: Option<&str>,
    proxy_url: Option<&str>,
    on_progress: F,
) -> Result<ImportResult, String>
where
    F: FnMut(DownloadProgress) + Send,
{
    let package = download_and_stage_release_package(
        repo_url,
        tag,
        plugin_id,
        expected_sha256,
        proxy_url,
        on_progress,
    )
    .await?;
    registry.import_with_source(&package.manifest_path, Some(package.source))
}

/// Full download, checksum verification, secure unpacking, and atomic registry registration workflow.
pub async fn download_and_stage_release_package<F>(
    repo_url: &str,
    tag: Option<&str>,
    plugin_id: &str,
    expected_sha256: Option<&str>,
    proxy_url: Option<&str>,
    mut on_progress: F,
) -> Result<DownloadedPackage, String>
where
    F: FnMut(DownloadProgress) + Send,
{
    validate_id(plugin_id)?;
    let (owner, repo) = parse_github_owner_repo(repo_url)?;

    on_progress(DownloadProgress {
        plugin_id: plugin_id.to_string(),
        stage: "fetching_release".to_string(),
        downloaded_bytes: 0,
        total_bytes: 0,
        percent: 0.0,
        message: format!("正在获取 GitHub Release 元数据 ({owner}/{repo})..."),
    });

    let client = build_client(proxy_url)?;
    let release = fetch_release_metadata(&client, &owner, &repo, tag).await?;
    let tag_name = release
        .tag_name
        .as_deref()
        .ok_or_else(|| "release missing tag".to_string())?;
    if !crate::updater::valid_tag(tag_name) {
        return Err("invalid release tag".into());
    }
    if release.id == 0 || release.draft || release.prerelease {
        return Err("release is not a stable release".into());
    }

    let asset = find_matching_asset(&release.assets).ok_or_else(|| {
        format!(
            "在 GitHub Release ({}) 中未找到匹配当前操作系统 ({}) 与架构 ({}) 的可用二进制资产",
            tag_name,
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    })?;

    validate_asset_filename(&asset.name)?;
    if asset.id == 0 {
        return Err("invalid release asset identity".into());
    }
    if asset.size > MAX_ARCHIVE_BYTES {
        return Err("release asset size exceeds maximum limit (512 MiB)".into());
    }
    validate_release_asset_url(
        &asset.browser_download_url,
        &owner,
        &repo,
        tag_name,
        &asset.name,
    )?;

    // Determine expected SHA256 checksum (fail-closed)
    let discovered = expected_digest(&release, &client, &asset.name).await.ok();
    let (effective_expected_sha, digest_evidence) = match (expected_sha256, discovered) {
        (Some(caller_sha), Some((disc_sha, disc_ev))) => {
            if !crate::updater::valid_digest(caller_sha) {
                return Err("invalid expected SHA256 format".into());
            }
            if !disc_sha.eq_ignore_ascii_case(caller_sha) {
                return Err("caller-supplied SHA256 does not match release checksum".into());
            }
            (caller_sha.to_lowercase(), disc_ev)
        }
        (Some(caller_sha), None) => {
            if !crate::updater::valid_digest(caller_sha) {
                return Err("invalid expected SHA256 format".into());
            }
            (
                caller_sha.to_lowercase(),
                crate::updater::DigestEvidence::UserProvidedSha256,
            )
        }
        (None, Some((disc_sha, disc_ev))) => (disc_sha, disc_ev),
        (None, None) => {
            return Err(
                "missing required SHA256 digest: release provides no asset digest or checksum file"
                    .into(),
            );
        }
    };

    // Stage 1: Download
    let temp_dl_dir = tempfile::Builder::new()
        .prefix(".pw-download-")
        .tempdir()
        .map_err(|e| format!("创建临时下载目录失败: {e}"))?;
    let downloaded_archive = temp_dl_dir.path().join(&asset.name);

    let mut resp = client
        .get(&asset.browser_download_url)
        .header("User-Agent", "proxyweave-desktop")
        .send()
        .await
        .map_err(|e| format!("请求下载资产失败: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("下载资产 HTTP 失败: {}", resp.status()));
    }
    if resp
        .content_length()
        .is_some_and(|len| len > MAX_ARCHIVE_BYTES)
    {
        return Err("下载资产超出最大归档大小限制 (512 MiB)".into());
    }

    let total_bytes = resp
        .content_length()
        .filter(|&len| len > 0)
        .unwrap_or(asset.size);

    on_progress(DownloadProgress {
        plugin_id: plugin_id.to_string(),
        stage: "downloading".to_string(),
        downloaded_bytes: 0,
        total_bytes,
        percent: 0.0,
        message: format!("正在流式下载资产: {}...", asset.name),
    });

    let mut file =
        fs::File::create(&downloaded_archive).map_err(|e| format!("创建临时归档文件失败: {e}"))?;
    let mut hasher = Sha256::new();
    let mut downloaded_bytes: u64 = 0;

    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| format!("读取下载数据流中断: {e}"))?
    {
        downloaded_bytes = downloaded_bytes
            .checked_add(chunk.len() as u64)
            .ok_or_else(|| "下载字节数溢出".to_string())?;
        if downloaded_bytes > MAX_ARCHIVE_BYTES {
            let _ = fs::remove_file(&downloaded_archive);
            return Err("下载资产超出最大归档大小限制 (512 MiB)".into());
        }
        file.write_all(&chunk)
            .map_err(|e| format!("写入临时归档文件失败: {e}"))?;
        hasher.update(&chunk);

        let percent = if total_bytes > 0 {
            ((downloaded_bytes as f64 / total_bytes as f64) * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };

        on_progress(DownloadProgress {
            plugin_id: plugin_id.to_string(),
            stage: "downloading".to_string(),
            downloaded_bytes,
            total_bytes,
            percent,
            message: format!(
                "正在下载: {:.1} MB / {:.1} MB ({:.0}%)",
                downloaded_bytes as f64 / 1_048_576.0,
                total_bytes as f64 / 1_048_576.0,
                percent
            ),
        });
    }

    drop(file);

    // Stage 2: SHA256 Verification
    on_progress(DownloadProgress {
        plugin_id: plugin_id.to_string(),
        stage: "verifying_checksum".to_string(),
        downloaded_bytes,
        total_bytes,
        percent: 100.0,
        message: "正在比对 SHA256 校验和...".to_string(),
    });

    let actual_sha = format!("{:x}", hasher.finalize());
    if !effective_expected_sha.eq_ignore_ascii_case(&actual_sha) {
        let _ = fs::remove_file(&downloaded_archive);
        let err_msg = format!(
            "SHA256 校验和不匹配 (期望 {}, 实际 {}), 已终止并删除临时文件以防执行被篡改的二进制",
            effective_expected_sha, actual_sha
        );
        on_progress(DownloadProgress {
            plugin_id: plugin_id.to_string(),
            stage: "error".to_string(),
            downloaded_bytes,
            total_bytes,
            percent: 0.0,
            message: err_msg.clone(),
        });
        return Err(err_msg);
    }

    // Stage 3: Secure Extraction & Unpacking
    on_progress(DownloadProgress {
        plugin_id: plugin_id.to_string(),
        stage: "extracting".to_string(),
        downloaded_bytes,
        total_bytes,
        percent: 100.0,
        message: "正在安全解包与验证归档内容...".to_string(),
    });

    let staging_dir = tempfile::Builder::new()
        .prefix(".pw-stage-release-")
        .tempdir()
        .map_err(|e| format!("创建临时 staging 目录失败: {e}"))?;

    extract_archive_secure(&downloaded_archive, staging_dir.path(), &asset.name)?;
    let _ = fs::remove_file(&downloaded_archive);

    // Stage 4: Manifest and Atomic Registration
    on_progress(DownloadProgress {
        plugin_id: plugin_id.to_string(),
        stage: "registering".to_string(),
        downloaded_bytes,
        total_bytes,
        percent: 100.0,
        message: "正在原子注册与发布插件...".to_string(),
    });

    let manifest_path = ensure_staging_manifest(staging_dir.path(), plugin_id, tag_name)?;
    let manifest_bytes =
        crate::registry::read_bounded(&manifest_path, crate::manifest::MAX_MANIFEST_BYTES)?;
    let manifest = Manifest::parse(&manifest_bytes)?;
    let manifest_fingerprint = crate::registry::manifest_fingerprint(&manifest)?;

    let source = crate::updater::InstalledSource {
        owner,
        repo,
        release_id: release.id,
        tag_name: tag_name.to_string(),
        asset_id: asset.id,
        asset_name: asset.name.clone(),
        target_os: std::env::consts::OS.to_string(),
        target_arch: std::env::consts::ARCH.to_string(),
        archive_sha256: actual_sha,
        expected_sha256: effective_expected_sha,
        digest_evidence,
        package_contract: crate::updater::PackageContract::ManifestV1,
        manifest_fingerprint,
        immutability: crate::updater::Immutability::Unknown,
    };

    on_progress(DownloadProgress {
        plugin_id: plugin_id.to_string(),
        stage: "completed".to_string(),
        downloaded_bytes,
        total_bytes,
        percent: 100.0,
        message: format!("发布包 {plugin_id} ({tag_name}) 解包与校验完成"),
    });

    Ok(DownloadedPackage {
        _directory: staging_dir,
        manifest_path,
        source,
    })
}
