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
#[derive(Debug, Clone)]
pub struct VsInfo {
    pub install: PathBuf,
    pub version: String,
    pub vc: PathBuf,
    pub tools_ver: String,
    pub tools: PathBuf,
    /// Release year (2017/2019/2022/2026), or `None` for a major version not mapped yet.
    pub year: Option<u16>,
    pub edition: VsEdition,
    /// Raw vswhere `productId`, kept verbatim so an unrecognised product is still reportable.
    pub product_id: String,
    /// vswhere `displayName`, e.g. "Visual Studio Community 2026".
    pub display_name: String,
    /// vswhere `isPrerelease` — a Preview/Insiders channel install.
    pub prerelease: bool,
}

/// Which Visual Studio product an installation is.
///
/// Parsed from vswhere's `productId`. `Other` deliberately covers both the products that are not
/// a C++ toolchain (`TeamExplorer`, `TestAgent`, ...) and any id Microsoft has not shipped yet:
/// folding an unknown id into a known edition would quietly select the wrong install, which is the
/// one failure this classification exists to prevent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum VsEdition {
    Community,
    Professional,
    Enterprise,
    BuildTools,
    #[value(skip)]
    Other,
}

impl VsEdition {
    fn from_product_id(id: &str) -> Self {
        match id.rsplit('.').next().unwrap_or_default() {
            "Community" => Self::Community,
            "Professional" => Self::Professional,
            "Enterprise" => Self::Enterprise,
            "BuildTools" => Self::BuildTools,
            _ => Self::Other,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Community => "Community",
            Self::Professional => "Professional",
            Self::Enterprise => "Enterprise",
            Self::BuildTools => "BuildTools",
            Self::Other => "Other",
        }
    }
}

/// How to treat Preview / Insiders installations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum Prerelease {
    /// Not a candidate. The default, because an unqualified "give me a compiler" should not land
    /// on a preview channel just because it happens to carry the highest version number.
    #[default]
    Exclude,
    /// Competes with released installs on version alone.
    Allow,
    /// Only preview installs — for verifying against a channel on purpose.
    Only,
}

/// Everything that narrows the choice of a Visual Studio installation.
///
/// `Default` is the sensible default in full: no year bounds, any edition, released channels only.
/// Each field is an independent axis, so a caller sets exactly what it cares about and inherits
/// the rest — there is no combination that has to be spelled out to get ordinary behaviour.
#[derive(Debug, Clone, Default)]
pub struct VsFilter {
    /// Require release year >= this value.
    pub min_year: Option<u16>,
    /// Require release year <= this value.
    pub max_year: Option<u16>,
    /// Require this exact edition.
    pub edition: Option<VsEdition>,
    pub prerelease: Prerelease,
}

impl VsFilter {
    /// Bound to a single release year (the `-v 2022` shape).
    pub fn year(mut self, year: Option<u16>) -> Self {
        self.min_year = year;
        self.max_year = year;
        self
    }

    /// Does this installation satisfy every axis?
    pub fn matches(&self, vs: &VsInfo) -> bool {
        if self.edition.is_some_and(|want| vs.edition != want) {
            return false;
        }
        match self.prerelease {
            Prerelease::Exclude if vs.prerelease => return false,
            Prerelease::Only if !vs.prerelease => return false,
            _ => {}
        }
        if self.min_year.is_none() && self.max_year.is_none() {
            return true;
        }
        // An unmapped major cannot be placed on the year axis, so it cannot satisfy a year bound.
        let Some(year) = vs.year else {
            return false;
        };
        self.min_year.is_none_or(|min| year >= min) && self.max_year.is_none_or(|max| year <= max)
    }
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
    #[serde(rename = "productId", default)]
    product_id: String,
    #[serde(rename = "displayName", default)]
    display_name: String,
    #[serde(rename = "isPrerelease", default)]
    is_prerelease: bool,
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
        year: vs_major_to_year(&vs.installation_version),
        version: vs.installation_version,
        vc,
        tools_ver,
        tools,
        edition: VsEdition::from_product_id(&vs.product_id),
        product_id: vs.product_id,
        display_name: vs.display_name,
        prerelease: vs.is_prerelease,
    })
}

/// Sort key for an `installationVersion`, numeric per component.
///
/// Comparing the string directly puts 17.9 ABOVE 17.14, because '9' > '1' — i.e. two servicing
/// updates of the same VS release sort backwards, and the older one wins. Non-numeric components
/// sort as 0 rather than failing the whole comparison.
fn version_key(version: &str) -> Vec<u32> {
    version
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect()
}

/// Collect and sort all vswhere entries, newest first.
///
/// `-products *` is required, not cosmetic: without it vswhere omits **Build Tools** installs
/// entirely, so a build server carrying only the standalone C++ toolchain looks like a machine
/// with no compiler at all. `-prerelease` likewise — preview channels are filtered by
/// [`VsFilter`] afterwards, on a field, rather than by never asking about them.
fn all_vs_entries() -> Vec<VsWhereEntry> {
    let vswhere =
        PathBuf::from(r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe");
    if !vswhere.exists() {
        return vec![];
    }
    let Ok(output) = Command::new(&vswhere)
        .args([
            "-all",
            "-prerelease",
            "-products",
            "*",
            "-format",
            "json",
            "-utf8",
        ])
        .output()
    else {
        return vec![];
    };
    let mut entries: Vec<VsWhereEntry> = serde_json::from_slice(&output.stdout).unwrap_or_default();
    entries.sort_by_key(|e| std::cmp::Reverse(version_key(&e.installation_version)));
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
    detect_vs_filtered(&VsFilter {
        min_year,
        max_year,
        ..Default::default()
    })
}

/// Find the best VS installation, optionally restricted to an exact release year.
pub fn detect_vs(vs_year: Option<u16>) -> Option<VsInfo> {
    detect_vs_range(vs_year, vs_year)
}

/// The newest installation satisfying every axis of `filter`.
///
/// This is the one selection path; [`detect_vs`] and [`detect_vs_range`] are shorthands for it, so
/// a rule added to [`VsFilter::matches`] cannot apply on some routes and not others.
pub fn detect_vs_filtered(filter: &VsFilter) -> Option<VsInfo> {
    list_vs().into_iter().find(|vs| filter.matches(vs))
}

/// Every usable C++ installation, newest first.
///
/// "Usable" is not a claim about the product id: an entry survives only if it actually carries an
/// MSVC toolset directory ([`build_vs_info`] checks), so products that install no compiler drop
/// out here rather than being rejected by name.
pub fn list_vs() -> Vec<VsInfo> {
    all_vs_entries()
        .into_iter()
        .filter_map(build_vs_info)
        .collect()
}

/// List all installed VS versions (for error messages).
pub fn list_vs_versions() -> Vec<(u16, String)> {
    list_vs()
        .into_iter()
        .filter_map(|vs| Some((vs.year?, vs.version)))
        .collect()
}

/// Find Windows 10/11 SDK
pub fn detect_sdk() -> Option<SdkInfo> {
    let sdk_path = reg_find(
        r"Microsoft\Microsoft SDKs\Windows\v10.0",
        "InstallationFolder",
    )?;
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

    Some(SdkInfo {
        path: root,
        version,
    })
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
            name.starts_with("10.") && e.path().join("ucrt").join("x64").join("ucrt.lib").exists()
        })
        .collect();

    versions.sort_by_key(|b| std::cmp::Reverse(b.file_name()));
    let version = versions.first()?.file_name().to_string_lossy().to_string();

    Some(SdkInfo {
        path: root,
        version,
    })
}

#[cfg(test)]
mod vs_tests {
    use super::*;

    /// A `VsInfo` carrying only the fields selection reads; paths play no part in it.
    fn vs(version: &str, edition: VsEdition, prerelease: bool) -> VsInfo {
        VsInfo {
            install: PathBuf::new(),
            year: vs_major_to_year(version),
            version: version.to_string(),
            vc: PathBuf::new(),
            tools_ver: String::new(),
            tools: PathBuf::new(),
            edition,
            product_id: String::new(),
            display_name: String::new(),
            prerelease,
        }
    }

    #[test]
    fn servicing_updates_sort_numerically() {
        // Mutation that reddens this: compare the version STRINGS, as the code used to. '9' > '1'
        // puts 17.9 above 17.14, so the older servicing update of the same release wins and the
        // machine silently builds with a stale toolset.
        let mut versions = ["17.9.1", "17.14.37314.3", "18.6.11822.322", "17.10.0"];
        versions.sort_by_key(|v| std::cmp::Reverse(version_key(v)));
        assert_eq!(
            versions,
            ["18.6.11822.322", "17.14.37314.3", "17.10.0", "17.9.1"]
        );
    }

    #[test]
    fn edition_is_classified_and_unknown_stays_unknown() {
        let of = VsEdition::from_product_id;
        assert_eq!(
            of("Microsoft.VisualStudio.Product.Community"),
            VsEdition::Community
        );
        assert_eq!(
            of("Microsoft.VisualStudio.Product.Enterprise"),
            VsEdition::Enterprise
        );
        assert_eq!(
            of("Microsoft.VisualStudio.Product.BuildTools"),
            VsEdition::BuildTools
        );
        // A product id nobody has shipped yet must NOT be folded into a known edition, or an
        // `--edition community` would one day select something that is not Community.
        assert_eq!(
            of("Microsoft.VisualStudio.Product.Whatever"),
            VsEdition::Other
        );
        assert_eq!(of(""), VsEdition::Other);
    }

    #[test]
    fn every_filter_axis_is_independent() {
        let vs2022 = vs("17.14.0", VsEdition::Community, false);
        let vs2026 = vs("18.6.0", VsEdition::Enterprise, false);

        // Default: no bound on anything, so both qualify.
        let any = VsFilter::default();
        assert!(any.matches(&vs2022) && any.matches(&vs2026));

        // Year alone.
        let by_year = VsFilter::default().year(Some(2022));
        assert!(by_year.matches(&vs2022) && !by_year.matches(&vs2026));

        // Edition alone — note it selects the 2026 install, so a passing edition filter cannot be
        // an accident of the year axis agreeing with it.
        let by_edition = VsFilter {
            edition: Some(VsEdition::Enterprise),
            ..Default::default()
        };
        assert!(!by_edition.matches(&vs2022) && by_edition.matches(&vs2026));

        // Both together: an install must satisfy each axis, not either.
        let both = VsFilter {
            edition: Some(VsEdition::Enterprise),
            ..Default::default()
        }
        .year(Some(2022));
        assert!(!both.matches(&vs2022) && !both.matches(&vs2026));
    }

    #[test]
    fn preview_is_excluded_until_asked_for() {
        let preview = vs("18.7.0", VsEdition::Community, true);
        let released = vs("18.6.0", VsEdition::Community, false);

        // The default must reject the preview even though it carries the HIGHER version — that is
        // the whole point, and a mutation making Allow the default reddens here.
        let default = VsFilter::default();
        assert!(!default.matches(&preview) && default.matches(&released));

        let allow = VsFilter {
            prerelease: Prerelease::Allow,
            ..Default::default()
        };
        assert!(allow.matches(&preview) && allow.matches(&released));

        let only = VsFilter {
            prerelease: Prerelease::Only,
            ..Default::default()
        };
        assert!(only.matches(&preview) && !only.matches(&released));
    }

    #[test]
    fn an_unmapped_major_cannot_satisfy_a_year_bound() {
        // VS 19 does not exist yet; it has no year, so it must not slip through a bounded request.
        // Unbounded requests still see it, which is what keeps a future release usable by default.
        let future = vs("19.0.0", VsEdition::Community, false);
        assert_eq!(future.year, None);
        assert!(VsFilter::default().matches(&future));
        assert!(!VsFilter::default().year(Some(2026)).matches(&future));
    }
}
