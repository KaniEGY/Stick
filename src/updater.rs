//! Verified release discovery and download for Stick's Windows updater.
//! A newer version is not installed until the running program closes.
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

const MAX_EXE_SIZE: u64 = 350 * 1024 * 1024;
const RELEASE_EXE_NAME: &str = "Stick.exe";
const RELEASE_ZIP_NAME: &str = "Stick-update.zip";
const RELEASE_URL_PREFIX: &str = "https://github.com/KaniEGY/Stick/releases/download/";

/** The version compiled INTO Stick.exe (not a potentially stale sidecar file).
 * Official Windows Actions builds set STICK_BUILD_VERSION to the full release
 * tag version, including github.run_number. Local cargo builds use Cargo.toml.
 */
pub fn embedded_build_version() -> &'static str {
    option_env!("STICK_BUILD_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseInfo {
    pub version: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

fn version_parts(s: &str) -> Result<[u64; 4], String> {
    let value = s.trim().trim_start_matches('v');
    let fields: Vec<&str> = value.split('.').collect();
    if !(2..=4).contains(&fields.len()) {
        return Err(format!("Invalid Stick version: {value}"));
    }
    let mut parts = [0; 4];
    for (i, field) in fields.iter().enumerate() {
        if field.is_empty() || !field.chars().all(|c| c.is_ascii_digit()) {
            return Err(format!("Invalid numeric version: {value}"));
        }
        parts[i] = field.parse::<u64>().map_err(|e| e.to_string())?;
    }
    Ok(parts)
}

pub fn is_newer(latest: &str, installed: &str) -> Result<bool, String> {
    Ok(version_parts(latest)?.cmp(&version_parts(installed)?) == Ordering::Greater)
}

pub fn parse_latest_release(json: &Value, current: &str) -> Result<Option<ReleaseInfo>, String> {
    let tag = json.get("tag_name").and_then(Value::as_str)
        .ok_or("GitHub did not return a release tag.")?;
    let version = tag.trim_start_matches('v');
    if !is_newer(version, current)? {
        return Ok(None);
    }
    let assets = json.get("assets").and_then(Value::as_array)
        .ok_or("The newer release has no downloadable assets.")?;
    let asset = assets.iter()
        .find(|a| a.get("name").and_then(Value::as_str) == Some(RELEASE_ZIP_NAME))
        .or_else(|| assets.iter()
            .find(|a| a.get("name").and_then(Value::as_str) == Some(RELEASE_EXE_NAME)))
        .ok_or("The newer release has no Stick update package.")?;
    let url = asset.get("browser_download_url").and_then(Value::as_str)
        .ok_or("The release download URL is missing.")?;
    let is_official_package = url.starts_with(RELEASE_URL_PREFIX)
        && (url.ends_with("/Stick-update.zip") || url.ends_with("/Stick.exe"));
    if !is_official_package {
        return Err("The release download is not an official Stick GitHub asset.".into());
    }
    let digest = asset.get("digest").and_then(Value::as_str)
        .and_then(|v| v.strip_prefix("sha256:"))
        .ok_or("The release has no SHA-256 digest; update canceled for safety.")?;
    if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("GitHub returned an invalid SHA-256 digest.".into());
    }
    let size = asset.get("size").and_then(Value::as_u64)
        .ok_or("GitHub did not provide the download size.")?;
    if !(1024..=MAX_EXE_SIZE).contains(&size) {
        return Err("The release executable size is invalid or too large.".into());
    }
    Ok(Some(ReleaseInfo { version: version.to_string(), url: url.into(), sha256: digest.to_ascii_lowercase(), size }))
}

const GITHUB_LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/KaniEGY/Stick/releases/latest";
const GITHUB_RELEASE_LIST_API: &str =
    "https://api.github.com/repos/KaniEGY/Stick/releases?per_page=20";

/// Retry transient GitHub gateway/network failures using fresh HTTPS
/// connections. A 502/503/504 from the dedicated /latest route should not
/// prevent the updater from checking the separate Releases collection.
fn fetch_github_api_json(url: &str, label: &str) -> Result<Value, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(12))
        .timeout_read(Duration::from_secs(20))
        .build();
    let mut last_error = String::new();

    for attempt in 1..=2 {
        match agent.get(url)
            .set("User-Agent", "Stick-Studio-Updater/4.1")
            .set("Accept", "application/vnd.github+json")
            .set("Cache-Control", "no-cache")
            .set("Connection", "close")
            .call()
        {
            Ok(response) => {
                return response.into_json()
                    .map_err(|e| format!("{label} returned unreadable JSON: {e}"));
            }
            Err(ureq::Error::Status(code, _response)) => {
                last_error = format!("HTTP {code}");
            }
            Err(ureq::Error::Transport(error)) => {
                last_error = error.to_string();
            }
        }

        if attempt < 2 {
            std::thread::sleep(Duration::from_millis(650 * attempt as u64));
        }
    }

    Err(format!("{label} is temporarily unavailable after 2 attempts ({last_error})."))
}

/// Alternate to /releases/latest. Pick the newest published stable release
/// by publication time, then apply the exact same official asset/hash checks.
fn parse_latest_release_list(json: &Value, installed: &str) -> Result<Option<ReleaseInfo>, String> {
    let releases = json.as_array()
        .ok_or("GitHub returned an invalid Releases list.")?;
    let latest = releases.iter()
        .filter(|release| {
            !release.get("draft").and_then(Value::as_bool).unwrap_or(false)
                && !release.get("prerelease").and_then(Value::as_bool).unwrap_or(false)
        })
        .max_by(|a, b| {
            a.get("published_at").and_then(Value::as_str)
                .cmp(&b.get("published_at").and_then(Value::as_str))
        })
        .ok_or("GitHub did not return a published stable release.")?;
    parse_latest_release(latest, installed)
}

pub fn check_online(installed: &str) -> Result<Option<ReleaseInfo>, String> {
    match fetch_github_api_json(GITHUB_LATEST_RELEASE_API, "GitHub latest-release check") {
        Ok(json) => parse_latest_release(&json, installed),
        Err(primary_error) => {
            // This alternate endpoint often remains available when the
            // dedicated latest-release route returns a gateway timeout.
            match fetch_github_api_json(GITHUB_RELEASE_LIST_API, "GitHub Releases fallback") {
                Ok(json) => parse_latest_release_list(&json, installed).map_err(|fallback_error| {
                    format!("Latest-release check failed ({primary_error}); fallback validation failed ({fallback_error}).")
                }),
                Err(fallback_error) => Err(format!(
                    "Couldn't check for Stick updates. GitHub's update service is temporarily unavailable. {primary_error} {fallback_error} Please try again in a moment."
                )),
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ArchivedRelease {
    pub info: ReleaseInfo,
    /// ISO-8601 publication time from GitHub's official Releases API.
    pub published_at: String,
}

/// The GitHub Releases API is not always ordered by *publication date*.
/// Only offer verified official older versions, not beta tags or draft builds.
pub fn parse_rollback_history(json: &Value, installed: &str) -> Result<Vec<ArchivedRelease>, String> {
    let releases = json.as_array().ok_or("GitHub returned an invalid releases list.")?;
    let current = version_parts(installed)?;
    let mut candidates = Vec::<ArchivedRelease>::new();
    for release in releases {
        if release.get("draft").and_then(Value::as_bool).unwrap_or(false)
            || release.get("prerelease").and_then(Value::as_bool).unwrap_or(false) { continue; }
        let Some(tag) = release.get("tag_name").and_then(Value::as_str) else { continue; };
        let Ok(version) = version_parts(tag) else { continue; };
        if version.cmp(&current) != Ordering::Less { continue; }
        let Ok(Some(info)) = parse_latest_release(release, "0.0.0") else { continue; };
        let Some(published_at) = release.get("published_at").and_then(Value::as_str) else { continue; };
        if published_at.len() < 10 { continue; }
        candidates.push(ArchivedRelease { info, published_at: published_at.to_owned() });
    }
    candidates.sort_by(|a, b| b.published_at.cmp(&a.published_at));
    candidates.dedup_by(|a, b| a.info.version == b.info.version);
    // First entry is the immediate previous release; ten *more* appear below it.
    candidates.truncate(11);
    Ok(candidates)
}

pub fn fetch_rollback_history(installed: &str) -> Result<Vec<ArchivedRelease>, String> {
    let json = fetch_github_api_json(
        "https://api.github.com/repos/KaniEGY/Stick/releases?per_page=100",
        "GitHub release history",
    )?;
    parse_rollback_history(&json, installed)
}

fn verify_pe_file(bytes: &[u8]) -> bool {
    if bytes.len() < 0x40 || &bytes[0..2] != b"MZ" { return false; }
    let offset = u32::from_le_bytes([bytes[0x3c], bytes[0x3d], bytes[0x3e], bytes[0x3f]]) as usize;
    bytes.get(offset..offset.saturating_add(4)) == Some(&b"PE\0\0"[..])
}

fn download_with_ureq_once<F: FnMut(u64, u64)>(
    info:&ReleaseInfo,
    destination:&Path,
    mut on_progress:F,
)->Result<(),String>{
    use std::io::Write;

    let response=ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(25))
        .timeout_read(Duration::from_secs(90))
        .build()
        .get(&info.url)
        // Force a fresh HTTP connection for every updater attempt. This avoids
        // reusing a TLS connection that a CDN/proxy has already half-closed.
        .set("Connection","close")
        .set("User-Agent","Stick-Updater")
        .call().map_err(|e|format!("HTTPS request failed: {e}"))?;

    let mut reader=response.into_reader();
    let mut file=std::fs::File::create(destination)
        .map_err(|e|format!("Can't write to the Stick install folder: {e}"))?;
    let mut total=0u64;
    let mut chunk=[0u8;64*1024];
    on_progress(0,info.size);

    loop {
        let n=reader.read(&mut chunk)
            .map_err(|e|format!("HTTPS stream failed: {e}"))?;
        if n==0 {break}
        total+=n as u64;
        if total>info.size || total>MAX_EXE_SIZE {
            return Err("Update package is larger than the published release.".into());
        }
        file.write_all(&chunk[..n])
            .map_err(|e|format!("Couldn't write the downloaded update: {e}"))?;
        on_progress(total,info.size);
    }
    file.sync_all().map_err(|e|format!("Couldn't finish writing the update: {e}"))?;
    if total!=info.size {
        return Err(format!(
            "Downloaded {} bytes but GitHub published {} bytes.",
            total,info.size
        ));
    }
    Ok(())
}

fn download_with_windows_curl(info:&ReleaseInfo,destination:&Path)->Result<(),String>{
    // Windows 10/11 ship curl.exe using the Windows TLS stack (Schannel).
    // It is deliberately a fallback for TLS/CDN failures in the Rust client,
    // not a replacement for our SHA-256 and PE verification below.
    let output=std::process::Command::new("curl.exe")
        .arg("--location")
        .arg("--fail")
        .arg("--silent")
        .arg("--show-error")
        .arg("--retry").arg("3")
        .arg("--retry-delay").arg("1")
        .arg("--connect-timeout").arg("25")
        .arg("--max-time").arg("240")
        .arg("--proto").arg("=https")
        .arg("--proto-redir").arg("=https")
        .arg("--user-agent").arg("Stick-Updater")
        .arg("--output").arg(destination)
        .arg(&info.url)
        .output()
        .map_err(|e|format!("Windows curl could not start: {e}"))?;

    if !output.status.success() {
        let stderr=String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(if stderr.is_empty() {
            format!("Windows curl failed with exit code {:?}",output.status.code())
        } else {
            format!("Windows curl failed: {stderr}")
        });
    }
    Ok(())
}

fn verify_downloaded_package(info:&ReleaseInfo,path:&Path)->Result<(),String>{
    let metadata=std::fs::metadata(path)
        .map_err(|e|format!("Couldn't inspect the downloaded update: {e}"))?;
    if metadata.len()!=info.size {
        return Err(format!(
            "The downloaded update is incomplete ({} / {} bytes).",
            metadata.len(),info.size
        ));
    }
    if metadata.len()>MAX_EXE_SIZE {
        return Err("The downloaded update package is too large.".into());
    }

    let mut file=std::fs::File::open(path)
        .map_err(|e|format!("Couldn't verify the downloaded update: {e}"))?;
    let mut hasher=Sha256::new();
    let mut chunk=[0u8;128*1024];
    loop {
        let n=file.read(&mut chunk)
            .map_err(|e|format!("Couldn't verify the downloaded update: {e}"))?;
        if n==0 {break}
        hasher.update(&chunk[..n]);
    }
    let digest=format!("{:x}",hasher.finalize());
    if digest!=info.sha256 {
        return Err("SHA-256 mismatch. The update was canceled for safety.".into());
    }
    Ok(())
}

fn download_with_transport_fallback<F: FnMut(u64, u64)>(
    info:&ReleaseInfo,
    destination:&Path,
    mut on_progress:F,
)->Result<(),String>{
    let mut rust_errors=Vec::new();

    for attempt in 1..=3 {
        let _=std::fs::remove_file(destination);
        match download_with_ureq_once(info,destination,|done,total|on_progress(done,total)) {
            Ok(())=>{
                match verify_downloaded_package(info,destination) {
                    Ok(())=>return Ok(()),
                    Err(e)=>rust_errors.push(format!("attempt {attempt} verification: {e}")),
                }
            }
            Err(e)=>rust_errors.push(format!("attempt {attempt}: {e}")),
        }
        std::thread::sleep(Duration::from_millis(350*attempt as u64));
    }

    // A TLS decryption/record failure can be specific to rustls/ureq, an
    // antivirus HTTPS interceptor or a CDN edge. Retry through Windows
    // Schannel so users do not have to download the release manually.
    let _=std::fs::remove_file(destination);
    on_progress(0,info.size);
    match download_with_windows_curl(info,destination) {
        Ok(())=>{
            verify_downloaded_package(info,destination)?;
            on_progress(info.size,info.size);
            Ok(())
        }
        Err(curl_error)=>{
            let _=std::fs::remove_file(destination);
            Err(format!(
                "All secure download methods failed. Rust HTTPS: {}. Windows TLS fallback: {}",
                rust_errors.join(" | "),curl_error
            ))
        }
    }
}

pub fn download_verified_with_progress<F: FnMut(u64, u64)>(
    info: &ReleaseInfo, current_exe: &Path, mut on_progress: F,
) -> Result<PathBuf, String> {
    let official = info.url.starts_with(RELEASE_URL_PREFIX)
        && (info.url.ends_with("/Stick-update.zip") || info.url.ends_with("/Stick.exe"));
    if !official {
        return Err("Only official Stick GitHub release downloads are allowed.".into());
    }
    if !(1024..=MAX_EXE_SIZE).contains(&info.size) {
        return Err("The update file size is invalid.".into());
    }

    let parent = current_exe.parent().ok_or("Couldn't locate the program folder.")?;
    let staged = parent.join("Stick_new.exe");
    let package_tmp = parent.join("Stick_update.download");
    let extracted_tmp = parent.join("Stick_new.download");
    let _ = std::fs::remove_file(&package_tmp);
    let _ = std::fs::remove_file(&extracted_tmp);

    // Download through the Rust HTTPS client first. If a CDN/proxy/AV TLS
    // interceptor breaks that stream (for example "cannot decrypt peer's
    // message"), retry with fresh connections and finally Windows Schannel.
    let download_result=download_with_transport_fallback(
        info,
        &package_tmp,
        |done,total|on_progress(done,total),
    );

    if let Err(error)=download_result {
        let _=std::fs::remove_file(&package_tmp);
        return Err(error);
    }

    let prepare_result = if info.url.ends_with("/Stick-update.zip") {
        (|| -> Result<(), String> {
            let package = std::fs::File::open(&package_tmp)
                .map_err(|e| format!("Couldn't open the downloaded update package: {e}"))?;
            let mut archive = zip::ZipArchive::new(package)
                .map_err(|e| format!("The update ZIP is invalid: {e}"))?;
            let mut entry = archive.by_name(RELEASE_EXE_NAME)
                .map_err(|_| "The update ZIP does not contain Stick.exe.".to_string())?;
            if entry.is_dir() || !(1024..=MAX_EXE_SIZE).contains(&entry.size()) {
                return Err("The Stick.exe inside the update ZIP has an invalid size.".into());
            }
            let mut output = std::fs::File::create(&extracted_tmp)
                .map_err(|e| format!("Couldn't prepare the updated executable: {e}"))?;
            std::io::copy(&mut entry, &mut output)
                .map_err(|e| format!("Couldn't extract Stick.exe: {e}"))?;
            output.sync_all().map_err(|e| format!("Couldn't finish extracting Stick.exe: {e}"))?;

            let mut exe = std::fs::File::open(&extracted_tmp)
                .map_err(|e| format!("Couldn't verify extracted Stick.exe: {e}"))?;
            let mut first_bytes = vec![0u8; 4096];
            let n = exe.read(&mut first_bytes).map_err(|e| format!("Couldn't verify Stick.exe: {e}"))?;
            first_bytes.truncate(n);
            if !verify_pe_file(&first_bytes) {
                return Err("The extracted update is not a valid Windows executable.".into());
            }
            Ok(())
        })()
    } else {
        (|| -> Result<(), String> {
            let mut exe = std::fs::File::open(&package_tmp)
                .map_err(|e| format!("Couldn't verify the downloaded executable: {e}"))?;
            let mut first_bytes = vec![0u8; 4096];
            let n = exe.read(&mut first_bytes).map_err(|e| format!("Couldn't verify Stick.exe: {e}"))?;
            first_bytes.truncate(n);
            if !verify_pe_file(&first_bytes) {
                return Err("The downloaded update is not a valid Windows executable.".into());
            }
            std::fs::rename(&package_tmp, &extracted_tmp)
                .map_err(|e| format!("Couldn't stage the downloaded update: {e}"))?;
            Ok(())
        })()
    };

    if let Err(error) = prepare_result {
        let _ = std::fs::remove_file(&package_tmp);
        let _ = std::fs::remove_file(&extracted_tmp);
        return Err(error);
    }
    let _ = std::fs::remove_file(&package_tmp);

    // Remove an older staged update only after the new executable passes all checks.
    if staged.exists() {
        std::fs::remove_file(&staged)
            .map_err(|e| format!("Can't remove a previous update package: {e}"))?;
    }
    std::fs::rename(&extracted_tmp, &staged)
        .map_err(|e| format!("Couldn't prepare the update for restart: {e}"))?;
    Ok(staged)
}

/// Spawn a separate updater that waits for this executable to exit, swaps it,
/// rolls back if replacement fails, writes version.txt, and starts Stick again.
/// The arguments are separate process args, not interpolated into PowerShell.
pub fn spawn_replacer(current_exe: &Path, staged: &Path, new_version: &str) -> Result<(), String> {
    let _ = version_parts(new_version)?;
    if !staged.is_file() {
        return Err("The downloaded update file is missing.".into());
    }
    let script_path = std::env::temp_dir().join(format!("StickUpdater-{}.ps1", std::process::id()));
    let script = r#"
param(
    [Parameter(Mandatory=$true)][string]$CurrentExe,
    [Parameter(Mandatory=$true)][string]$StagedExe,
    [Parameter(Mandatory=$true)][string]$Version,
    [Parameter(Mandatory=$true)][int]$ParentPid
)
$ErrorActionPreference = 'Stop'
$dir = Split-Path -Parent $CurrentExe
$backup = Join-Path $dir 'Stick_previous.exe'
$versionPath = Join-Path $dir 'version.txt'
$logPath = Join-Path $dir 'Stick_update_error.log'
$previousVersion = if (Test-Path -LiteralPath $versionPath) { [System.IO.File]::ReadAllText($versionPath) } else { '' }
try {
    for ($attempt = 0; $attempt -lt 100; $attempt++) {
        if (-not (Get-Process -Id $ParentPid -ErrorAction SilentlyContinue)) { break }
        Start-Sleep -Milliseconds 250
    }
    if (Get-Process -Id $ParentPid -ErrorAction SilentlyContinue) { throw 'The previous Stick process did not close.' }
    if (-not (Test-Path -LiteralPath $StagedExe)) { throw 'The staged update is missing.' }
    if (Test-Path -LiteralPath $backup) { Remove-Item -LiteralPath $backup -Force }
    Move-Item -LiteralPath $CurrentExe -Destination $backup -Force
    try {
        Move-Item -LiteralPath $StagedExe -Destination $CurrentExe -Force
        [System.IO.File]::WriteAllText($versionPath, $Version)
        Start-Process -FilePath $CurrentExe -WorkingDirectory $dir
    } catch {
        if (Test-Path -LiteralPath $CurrentExe) { Remove-Item -LiteralPath $CurrentExe -Force }
        Move-Item -LiteralPath $backup -Destination $CurrentExe -Force
        [System.IO.File]::WriteAllText($versionPath, $previousVersion)
        Start-Process -FilePath $CurrentExe -WorkingDirectory $dir
        throw
    }
} catch {
    [System.IO.File]::WriteAllText($logPath, $_.Exception.Message)
}
"#;
    std::fs::write(&script_path, script)
        .map_err(|e| format!("Couldn't prepare the restart helper: {e}"))?;
    std::process::Command::new("powershell.exe")
        .arg("-NoProfile").arg("-NonInteractive").arg("-WindowStyle").arg("Hidden")
        .arg("-ExecutionPolicy").arg("Bypass")
        .arg("-File").arg(&script_path)
        .arg("-CurrentExe").arg(current_exe)
        .arg("-StagedExe").arg(staged)
        .arg("-Version").arg(new_version)
        .arg("-ParentPid").arg(std::process::id().to_string())
        .spawn()
        .map_err(|e| format!("Couldn't start the update restarter: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture_with_asset(tag: &str, digest: &str, name: &str, size: u64) -> Value {
        serde_json::json!({
            "tag_name":tag,
            "assets":[{
                "name":name,
                "browser_download_url":format!("https://github.com/KaniEGY/Stick/releases/download/{tag}/{name}"),
                "size":size,
                "digest":format!("sha256:{digest}")
            }]
        })
    }
    fn fixture(tag: &str, digest: &str) -> Value {
        fixture_with_asset(tag,digest,RELEASE_EXE_NAME,25_000_000)
    }
    #[test] fn bundled_build_version_is_always_comparable() {
        let compiled = embedded_build_version();
        assert!(version_parts(compiled).is_ok(), "Invalid compiled Stick version: {compiled}");
        if let Some(from_ci) = option_env!("STICK_BUILD_VERSION") {
            assert_eq!(compiled, from_ci);
        }
    }
    #[test] fn version_comparison_is_numeric_not_lexicographic() {
        assert!(is_newer("4.1.0.100","4.1.0.99").unwrap());
        assert!(is_newer("v4.1.0.72","4.1.0").unwrap());
        assert!(!is_newer("4.1.0.72","4.1.0.72").unwrap());
        assert!(!is_newer("2.2.71","4.1.0").unwrap());
    }
    #[test] fn compressed_update_is_preferred_over_raw_exe() {
        let digest = "d".repeat(64);
        let mut r = fixture_with_asset("v4.1.0.73",&digest,RELEASE_EXE_NAME,31_000_000);
        r["assets"].as_array_mut().unwrap().push(serde_json::json!({
            "name":RELEASE_ZIP_NAME,
            "browser_download_url":"https://github.com/KaniEGY/Stick/releases/download/v4.1.0.73/Stick-update.zip",
            "size":12_000_000u64,
            "digest":format!("sha256:{}", "e".repeat(64))
        }));
        let parsed=parse_latest_release(&r,"4.1.0.72").unwrap().unwrap();
        assert!(parsed.url.ends_with("/Stick-update.zip"));
        assert_eq!(parsed.size,12_000_000);
    }

    #[test] fn legacy_raw_exe_release_still_updates() {
        let r=fixture("v4.1.0.73",&"a".repeat(64));
        let parsed=parse_latest_release(&r,"4.1.0.72").unwrap().unwrap();
        assert!(parsed.url.ends_with("/Stick.exe"));
    }

    #[test] fn matching_release_is_not_downloaded_again() {
        let r=fixture("v4.1.0.72", &"a".repeat(64));
        assert!(parse_latest_release(&r,"4.1.0.72").unwrap().is_none());
    }
    #[test] fn valid_newer_release_has_checksum() {
        let r=fixture("v4.1.0.73", &"b".repeat(64));
        let parsed=parse_latest_release(&r,"4.1.0.72").unwrap().unwrap();
        assert_eq!(parsed.version,"4.1.0.73");
        assert_eq!(parsed.sha256,"b".repeat(64));
    }
    #[test] fn missing_checksum_is_a_real_error_not_no_update() {
        let mut r=fixture("v4.1.0.73",&"c".repeat(64));
        r["assets"][0]["digest"] = Value::Null;
        assert!(parse_latest_release(&r,"4.1.0.72").is_err());
    }
    #[test] fn latest_release_fallback_chooses_newest_published_stable_release() {
        let mut older = fixture("v4.1.0.80", &"a".repeat(64));
        older["published_at"] = Value::String("2026-11-20T09:00:00Z".into());
        let mut newer = fixture("v4.1.0.95", &"b".repeat(64));
        newer["published_at"] = Value::String("2026-11-21T09:00:00Z".into());
        let mut prerelease = fixture("v9.0.0", &"c".repeat(64));
        prerelease["published_at"] = Value::String("2026-11-22T09:00:00Z".into());
        prerelease["prerelease"] = Value::Bool(true);
        let mut draft = fixture("v10.0.0", &"d".repeat(64));
        draft["published_at"] = Value::String("2026-11-23T09:00:00Z".into());
        draft["draft"] = Value::Bool(true);

        let list = serde_json::json!([older, draft, prerelease, newer]);
        let release = parse_latest_release_list(&list, "4.1.0.90").unwrap().unwrap();
        assert_eq!(release.version, "4.1.0.95");
        assert_eq!(release.sha256, "b".repeat(64));
    }

    #[test] fn latest_release_fallback_returns_none_when_current_is_latest_stable() {
        let mut current = fixture("v4.1.0.95", &"b".repeat(64));
        current["published_at"] = Value::String("2026-11-21T09:00:00Z".into());
        let list = serde_json::json!([current]);
        assert!(parse_latest_release_list(&list, "4.1.0.95").unwrap().is_none());
    }

    #[test] fn rollback_history_is_by_publication_and_excludes_current_or_newer() {
        let mut latest = fixture("v4.1.0.140", &"e".repeat(64));
        latest["published_at"] = Value::String("2026-11-17T13:00:00Z".into());
        let mut older = fixture("v4.1.0.129", &"e".repeat(64));
        older["published_at"] = Value::String("2026-11-17T11:00:00Z".into());
        let mut previous = fixture("v4.1.0.128", &"e".repeat(64));
        previous["published_at"] = Value::String("2026-11-17T10:00:00Z".into());
        let mut old = fixture("v4.1.0.90", &"e".repeat(64));
        old["published_at"] = Value::String("2026-11-16T10:00:00Z".into());
        let items = serde_json::json!([old, latest, older, previous]);
        let result = parse_rollback_history(&items, "4.1.0.130").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].info.version, "4.1.0.129");
        assert_eq!(result[1].info.version, "4.1.0.128");
        assert_eq!(result[2].info.version, "4.1.0.90");
    }
    #[test] fn rollback_history_keeps_previous_plus_ten_older() {
        let items: Vec<Value> = (0..20).map(|i| {
            let build = 220 - i;
            let mut release = fixture(&format!("v4.1.0.{build}"), &"f".repeat(64));
            release["published_at"] =
                Value::String(format!("2026-11-17T12:{:02}:00Z", 59 - i));
            release
        }).collect();
        let result = parse_rollback_history(&Value::Array(items), "4.1.0.250").unwrap();
        assert_eq!(result.len(), 11);
        assert_eq!(result[0].info.version, "4.1.0.220");
        assert_eq!(result[10].info.version, "4.1.0.210");
    }
    #[test] fn rollback_history_skips_unsigned_and_prerelease_assets() {
        let mut unsigned = fixture("v4.1.0.127", &"a".repeat(64));
        unsigned["published_at"] = Value::String("2026-11-17T09:00:00Z".into());
        unsigned["assets"][0]["digest"] = Value::Null;
        let mut beta = fixture("v4.1.0.126", &"a".repeat(64));
        beta["published_at"] = Value::String("2026-11-17T08:00:00Z".into());
        beta["prerelease"] = Value::Bool(true);
        assert!(parse_rollback_history(&serde_json::json!([unsigned, beta]),"4.1.0.130").unwrap().is_empty());
    }
    #[test] fn download_verifier_rejects_wrong_hash_and_wrong_size() {
        let unique=format!("stick-updater-test-{}-{}.bin",std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
        let path=std::env::temp_dir().join(unique);
        std::fs::write(&path,b"hello update").unwrap();

        let mut good_hasher=Sha256::new();
        good_hasher.update(b"hello update");
        let good_hash=format!("{:x}",good_hasher.finalize());
        let good=ReleaseInfo{
            version:"4.1.0.999".into(),
            url:"https://github.com/KaniEGY/Stick/releases/download/v4.1.0.999/Stick-update.zip".into(),
            sha256:good_hash,
            size:12,
        };
        assert!(verify_downloaded_package(&good,&path).is_ok());

        let mut bad_size=good.clone();
        bad_size.size=13;
        assert!(verify_downloaded_package(&bad_size,&path).unwrap_err().contains("incomplete"));

        let mut bad_hash=good;
        bad_hash.sha256="0".repeat(64);
        assert!(verify_downloaded_package(&bad_hash,&path).unwrap_err().contains("SHA-256"));
        let _=std::fs::remove_file(path);
    }

    #[test] fn reject_html_disguised_as_executable() {
        assert!(!verify_pe_file(b"<html>hello</html>"));
        let mut exe=vec![0u8; 256];
        exe[0..2].copy_from_slice(b"MZ");
        exe[0x3c..0x40].copy_from_slice(&(128u32).to_le_bytes());
        exe[128..132].copy_from_slice(b"PE\0\0");
        assert!(verify_pe_file(&exe));
    }
}
