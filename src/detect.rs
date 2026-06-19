//! # Detection Module
//!
//! Detects Visual Studio, Windows SDK, and Universal CRT installations.
//!
//! ## Purpose
//! Locates VS toolchain and SDK paths without running slow batch scripts.
//! Uses vswhere.exe for VS detection and Windows registry for SDK/UCRT.
//!
//! ## Key Functions
//! - `detect_vs(vs_year)` - Find VS installation, optionally filter by year (2017/2019/2022)
//! - `detect_sdk()` - Find Windows 10/11 SDK via registry
//! - `detect_ucrt()` - Find Universal CRT via registry
//! - `list_vs_versions()` - List all installed VS versions (for error messages)
//!
//! ## Dependencies
//! - `registry` module for Windows registry access
//! - `serde_json` for parsing vswhere.exe JSON output

use crate::registry::reg_find;
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;

/// Visual Studio installation info
#[derive(Debug)]
pub struct VsInfo {
    pub install: PathBuf,
    pub version: String,
    pub vc: PathBuf,
    pub tools_ver: String,
    pub tools: PathBuf,
}

/// SDK/UCRT info
#[derive(Debug)]
pub struct SdkInfo {
    pub path: PathBuf,
    pub version: String,
}

#[derive(Deserialize)]
struct VsWhereEntry {
    #[serde(rename = "installationPath")]
    installation_path: String,
    #[serde(rename = "installationVersion", default)]
    installation_version: String,
}

/// Read single-line text file
fn read_txt(path: &PathBuf) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
}

/// Map vswhere installationVersion major to release year.
/// Unknown majors return None (treated as "too new" by range filters).
fn vs_major_to_year(version: &str) -> Option<u16> {
    let major = version.split('.').next()?;
    match major {
        "15" => Some(2017),
        "16" => Some(2019),
        "17" => Some(2022),
        "18" => Some(2026),
        _ => None,
    }
}

/// Build VsInfo from vswhere entry
fn build_vs_info(vs: VsWhereEntry) -> Option<VsInfo> {
    let install = PathBuf::from(&vs.installation_path);
    let vc = install.join("VC");
    let aux = vc.join("Auxiliary").join("Build");

    // Prefer the true system default; v143 file lags behind after VS updates
    let tools_ver = read_txt(&aux.join("Microsoft.VCToolsVersion.default.txt"))
        .or_else(|| read_txt(&aux.join("Microsoft.VCToolsVersion.v143.default.txt")))?;

    let tools = vc.join("Tools").join("MSVC").join(&tools_ver);
    if !tools.exists() {
        return None;
    }

    Some(VsInfo {
        install,
        version: vs.installation_version,
        vc,
        tools_ver,
        tools,
    })
}

/// Collect and sort all vswhere entries descending by version string.
fn all_vs_entries() -> Vec<VsWhereEntry> {
    let vswhere = PathBuf::from(
        r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe",
    );
    if !vswhere.exists() {
        return vec![];
    }
    let Ok(output) = Command::new(&vswhere)
        .args(["-all", "-format", "json", "-utf8"])
        .output()
    else {
        return vec![];
    };
    let mut entries: Vec<VsWhereEntry> =
        serde_json::from_slice(&output.stdout).unwrap_or_default();
    entries.sort_by(|a, b| b.installation_version.cmp(&a.installation_version));
    entries
}

/// Find the best (latest) VS installation matching an optional year range.
///
/// - `min_year`: require release year >= this value  (`None` = no lower bound)
/// - `max_year`: require release year <= this value  (`None` = no upper bound)
///
/// Entries with an unrecognised major version are skipped when either bound is set.
///
/// # Examples
/// ```ignore
/// detect_vs_range(None, None)        // latest available
/// detect_vs_range(None, Some(2022))  // latest that CUDA 13 supports
/// detect_vs_range(Some(2022), None)  // 2022 or newer
/// detect_vs_range(Some(2022), Some(2022)) // exactly 2022
/// ```
pub fn detect_vs_range(min_year: Option<u16>, max_year: Option<u16>) -> Option<VsInfo> {
    all_vs_entries()
        .into_iter()
        .filter(|e| {
            if min_year.is_none() && max_year.is_none() {
                return true;
            }
            let Some(year) = vs_major_to_year(&e.installation_version) else {
                return false; // unknown version excluded when bounds are set
            };
            min_year.is_none_or(|min| year >= min) && max_year.is_none_or(|max| year <= max)
        })
        .find_map(build_vs_info)
}

/// Find the best VS installation, optionally restricted to an exact release year.
pub fn detect_vs(vs_year: Option<u16>) -> Option<VsInfo> {
    detect_vs_range(vs_year, vs_year)
}

/// List all installed VS versions (for error messages).
pub fn list_vs_versions() -> Vec<(u16, String)> {
    all_vs_entries()
        .into_iter()
        .filter_map(|e| {
            let year = vs_major_to_year(&e.installation_version)?;
            Some((year, e.installation_version))
        })
        .collect()
}

/// Find Windows 10/11 SDK
pub fn detect_sdk() -> Option<SdkInfo> {
    let sdk_path = reg_find(r"Microsoft\Microsoft SDKs\Windows\v10.0", "InstallationFolder")?;
    let root = PathBuf::from(sdk_path);
    let inc = root.join("include");
    if !inc.exists() {
        return None;
    }

    // Find latest 10.x with winsdkver.h
    let mut versions: Vec<_> = std::fs::read_dir(&inc)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            name.starts_with("10.") && e.path().join("um").join("winsdkver.h").exists()
        })
        .collect();

    versions.sort_by_key(|b| std::cmp::Reverse(b.file_name()));
    let version = versions.first()?.file_name().to_string_lossy().to_string();

    Some(SdkInfo { path: root, version })
}

/// Find Universal CRT
pub fn detect_ucrt() -> Option<SdkInfo> {
    let ucrt_path = reg_find(r"Microsoft\Windows Kits\Installed Roots", "KitsRoot10")?;
    let root = PathBuf::from(ucrt_path);
    let lib = root.join("Lib");
    if !lib.exists() {
        return None;
    }

    // Find latest 10.x with ucrt.lib
    let mut versions: Vec<_> = std::fs::read_dir(&lib)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            name.starts_with("10.")
                && e.path().join("ucrt").join("x64").join("ucrt.lib").exists()
        })
        .collect();

    versions.sort_by_key(|b| std::cmp::Reverse(b.file_name()));
    let version = versions.first()?.file_name().to_string_lossy().to_string();

    Some(SdkInfo { path: root, version })
}
