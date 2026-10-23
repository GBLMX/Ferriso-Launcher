//! Update checks for the native release channel.
//!
//! The Tauri updater cannot be reused here: it needs an `AppHandle`. This
//! module reads a small, native-only manifest and never installs anything on
//! its own. Linux package-managed builds are notify-only; Windows can open the
//! signed installer from the manifest.

use std::collections::HashMap;

use semver::Version;
use serde::Deserialize;

/// Version injected by the release workflow, falling back to Cargo's version
/// for local builds.
pub const VERSION: &str = match option_env!("FERRISO_VERSION") {
    Some(version) => version,
    None => env!("CARGO_PKG_VERSION"),
};

/// Public manifest for `ferriso-v*` GitHub releases. It is deliberately
/// separate from Modrinth's `updates.json`, which belongs to the Tauri app.
pub const DEFAULT_MANIFEST_URL: &str = "https://github.com/GBLMX/code/releases/latest/download/ferriso-updates.json";

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub version: String,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub pub_date: Option<String>,
    #[serde(default)]
    pub platforms: HashMap<String, Platform>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Platform {
    pub url: String,
    #[serde(default)]
    pub install_urls: Vec<String>,
    #[serde(default)]
    pub signature: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: Option<String>,
    pub published: Option<String>,
    pub download_url: String,
    pub install_url: String,
    pub signature: Option<String>,
}

pub fn platform_key() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    let Ok(candidate) = Version::parse(candidate.trim_start_matches('v'))
    else {
        return false;
    };
    let Ok(current) = Version::parse(current.trim_start_matches('v')) else {
        return false;
    };
    candidate > current
}

/// Fetch and parse the native manifest. Network errors are returned so the
/// caller can log them; the UI stays quiet rather than showing a startup error.
pub async fn check() -> Result<Option<UpdateInfo>, String> {
    let url = std::env::var("FERRISO_UPDATE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_MANIFEST_URL.to_string());

    let client = reqwest::Client::builder()
        .user_agent(format!("FerrisoLauncher/{VERSION}"))
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(describe)?;
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|error| format!("update check failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("update check failed: HTTP {}", response.status()));
    }
    let body = response
        .text()
        .await
        .map_err(|error| format!("update manifest read failed: {error}"))?;
    let manifest = serde_json::from_str::<Manifest>(&body)
        .map_err(|error| format!("update manifest is invalid: {error}"))?;
    Ok(update_from_manifest(manifest, VERSION, &platform_key()))
}

fn update_from_manifest(
    manifest: Manifest,
    current: &str,
    key: &str,
) -> Option<UpdateInfo> {
    if !is_newer(&manifest.version, current) {
        return None;
    }
    let platform = manifest.platforms.get(key)?;
    if platform.url.trim().is_empty() {
        return None;
    }
    let install_url = platform
        .install_urls
        .first()
        .cloned()
        .unwrap_or_else(|| platform.url.clone());
    Some(UpdateInfo {
        version: manifest.version,
        notes: manifest.notes,
        published: manifest.pub_date,
        download_url: platform.url.clone(),
        install_url,
        signature: platform.signature.clone(),
    })
}

fn describe(error: reqwest::Error) -> String {
    format!("update client failed: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"{
        "version": "0.2.0",
        "notes": "Native project pages",
        "pub_date": "2026-10-23T00:00:00Z",
        "platforms": {
            "linux-x86_64": {
                "url": "https://example.invalid/ferriso-linux.tar.gz",
                "install_urls": ["https://example.invalid/ferriso.deb"]
            },
            "windows-x86_64": {
                "url": "https://example.invalid/ferriso-setup.exe",
                "signature": "signed"
            }
        }
    }"#;

    #[test]
    fn newer_versions_are_detected() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("v1.0.0", "0.9.9"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("garbage", "0.1.0"));
    }

    #[test]
    fn platform_manifest_selects_download_and_install_urls() {
        let manifest: Manifest = serde_json::from_str(MANIFEST).unwrap();
        let update =
            update_from_manifest(manifest.clone(), "0.1.0", "linux-x86_64")
                .expect("update");
        assert_eq!(update.version, "0.2.0");
        assert!(update.download_url.ends_with(".tar.gz"));
        assert!(update.install_url.ends_with(".deb"));

        let update =
            update_from_manifest(manifest, "0.1.0", "windows-x86_64").unwrap();
        assert_eq!(update.install_url, update.download_url);
        assert_eq!(update.signature.as_deref(), Some("signed"));
    }

    #[test]
    fn equal_or_unknown_platform_is_not_an_update() {
        let manifest: Manifest = serde_json::from_str(MANIFEST).unwrap();
        assert!(
            update_from_manifest(manifest.clone(), "0.2.0", "linux-x86_64")
                .is_none()
        );
        assert!(
            update_from_manifest(manifest, "0.1.0", "plan9-x86_64").is_none()
        );
    }
}
