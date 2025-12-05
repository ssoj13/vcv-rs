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

/// Build VsInfo from vswhere entry
fn build_vs_info(vs: VsWhereEntry) -> Option<VsInfo> {
    let install = PathBuf::from(&vs.installation_path);
    let vc = install.join("VC");
    let aux = vc.join("Auxiliary").join("Build");

    // Get tools version (try v143 first, then default)
    let tools_ver = read_txt(&aux.join("Microsoft.VCToolsVersion.v143.default.txt"))
        .or_else(|| read_txt(&aux.join("Microsoft.VCToolsVersion.default.txt")))?;

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

/// Detect VS installation via vswhere
/// If vs_year is Some, filter by year (2019, 2022, etc.)
pub fn detect_vs(vs_year: Option<u16>) -> Option<VsInfo> {
    let vswhere = PathBuf::from(r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe");
    if !vswhere.exists() {
        return None;
    }

    let output = Command::new(&vswhere)
        .args(["-all", "-format", "json", "-utf8"])
        .output()
        .ok()?;

    let entries: Vec<VsWhereEntry> = serde_json::from_slice(&output.stdout).ok()?;
    
    // Filter by year if specified
    let filtered: Vec<_> = if let Some(year) = vs_year {
        let major = match year {
            2017 => "15.",
            2019 => "16.",
            2022 => "17.",
            _ => return None,
        };
        entries.into_iter()
            .filter(|e| e.installation_version.starts_with(major))
            .collect()
    } else {
        entries
    };

    // Sort by version descending (latest first)
    let mut sorted = filtered;
    sorted.sort_by(|a, b| b.installation_version.cmp(&a.installation_version));

    // Try to build VsInfo from first valid entry
    sorted.into_iter().find_map(build_vs_info)
}

/// List all installed VS versions (for error messages)
pub fn list_vs_versions() -> Vec<(u16, String)> {
    let vswhere = PathBuf::from(r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe");
    if !vswhere.exists() {
        return vec![];
    }

    let output = match Command::new(&vswhere)
        .args(["-all", "-format", "json", "-utf8"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return vec![],
    };

    let entries: Vec<VsWhereEntry> = match serde_json::from_slice(&output.stdout) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    entries.into_iter()
        .filter_map(|e| {
            let year = if e.installation_version.starts_with("17.") {
                2022
            } else if e.installation_version.starts_with("16.") {
                2019
            } else if e.installation_version.starts_with("15.") {
                2017
            } else {
                return None;
            };
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

    versions.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
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

    versions.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
    let version = versions.first()?.file_name().to_string_lossy().to_string();

    Some(SdkInfo { path: root, version })
}
