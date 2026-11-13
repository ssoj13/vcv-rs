//! # OptiX SDK Detection
//!
//! Finds an installed NVIDIA OptiX SDK and reports what a build needs: its root, the SDK version,
//! the function-table ABI the headers were written against, and whether the device-side
//! implementation header is present.
//!
//! ## Why this lives in vcv-rs
//! The OptiX SDK is headers only — the runtime ships inside the display driver — so a build that
//! binds OptiX needs exactly one thing from the machine: the right `include/` directory. Every
//! consumer that rediscovers it on its own (`OPTIX_ROOT` here, `OptiX_INSTALL_DIR` in NVIDIA's
//! CMake, a hard-coded ProgramData path elsewhere) can pick a different SDK; vcv-rs already owns
//! "assemble the build environment", including the CUDA Toolkit, so OptiX belongs beside it.
//!
//! ## Nothing here is hard-coded per release
//! - the **version** comes from `include/optix.h`'s `OPTIX_VERSION` (see [`OptixVersion`]);
//! - the **ABI** comes from `include/optix_function_table.h`'s `OPTIX_ABI_VERSION`, the number the
//!   host passes to `optixQueryFunctionTable` — a lookup table from SDK version to ABI would be
//!   wrong the first time NVIDIA bumps one without the other;
//! - **device support** is the presence of `include/internal/optix_device_impl.h`, the header a
//!   Rust device layer is generated from.
//!
//! ## Platform support
//! - **Windows** — the installer's default `%ProgramData%\NVIDIA Corporation\OptiX SDK <ver>` and
//!   the root variables below.
//! - **Linux** — the SDK is a self-extracting archive with no standard location, so only the root
//!   variables and `/opt/*optix*` directories are searched.
//! - **macOS** — OptiX does not exist there; detection returns nothing by design.

use std::fmt;
use std::path::{Path, PathBuf};

/// An OptiX SDK version, as `optix.h` reports it.
///
/// `#define OPTIX_VERSION 90100` encodes `major * 10000 + minor * 100 + micro`. Not parsed from the
/// directory name, which is a convention (`OptiX SDK 9.1.0`) that a clone of `NVIDIA/optix-dev` or a
/// renamed folder does not follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct OptixVersion {
    pub major: u32,
    pub minor: u32,
    pub micro: u32,
}

impl OptixVersion {
    /// Decode the `OPTIX_VERSION` macro value.
    pub const fn from_macro(v: u32) -> Self {
        Self {
            major: v / 10000,
            minor: (v % 10000) / 100,
            micro: v % 100,
        }
    }

    /// Re-encode to the macro form, so a round-trip can be asserted.
    pub const fn to_macro(self) -> u32 {
        self.major * 10000 + self.minor * 100 + self.micro
    }
}

impl fmt::Display for OptixVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.micro)
    }
}

/// A located OptiX SDK.
#[derive(Debug, Clone)]
pub struct OptixInfo {
    /// SDK root — the directory holding `include/`.
    pub root: PathBuf,
    pub version: OptixVersion,
    /// `OPTIX_ABI_VERSION`: the function-table ABI these headers describe.
    pub abi: u32,
    /// `include/internal/optix_device_impl.h` exists (needed to generate device bindings).
    pub device_impl: bool,
}

impl OptixInfo {
    /// The header directory.
    pub fn include_dir(&self) -> PathBuf {
        self.root.join("include")
    }
}

/// Environment variables that name an SDK root, in the order they are honoured: `OPTIX_ROOT` is
/// what nv-rs and most Rust build scripts read, `OptiX_INSTALL_DIR` is what NVIDIA's own CMake
/// (`FindOptiX.cmake`) reads, `OPTIX_PATH` is a common third spelling.
const ROOT_VARS: [&str; 3] = ["OPTIX_ROOT", "OptiX_INSTALL_DIR", "OPTIX_PATH"];

/// Is this directory a usable SDK, and if so what is in it?
///
/// Requires `include/optix.h` with `OPTIX_VERSION` AND `include/optix_function_table.h` with
/// `OPTIX_ABI_VERSION`: without the ABI a host cannot query the driver's function table, so a root
/// missing it is a partial copy, not an SDK.
pub fn probe(root: &Path) -> Option<OptixInfo> {
    let include = root.join("include");
    let version = read_define(&include.join("optix.h"), "OPTIX_VERSION").map(OptixVersion::from_macro)?;
    let abi = read_define(&include.join("optix_function_table.h"), "OPTIX_ABI_VERSION")?;
    let device_impl = include.join("internal").join("optix_device_impl.h").is_file();
    Some(OptixInfo {
        root: root.to_path_buf(),
        version,
        abi,
        device_impl,
    })
}

/// Read `#define <name> <integer>` from a header.
fn read_define(header: &Path, name: &str) -> Option<u32> {
    let text = std::fs::read_to_string(header).ok()?;
    text.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("#define")?.trim_start();
        let rest = rest.strip_prefix(name)?;
        // `OPTIX_VERSION` must not match `OPTIX_VERSION_MAJOR`: the next char has to be a space.
        if !rest.starts_with(char::is_whitespace) {
            return None;
        }
        rest.split_whitespace().next()?.parse::<u32>().ok()
    })
}

/// Every SDK this machine offers, newest first.
///
/// Explicitly named roots come first (an operator's choice outranks a search), then the platform's
/// standard locations. Duplicates are removed by root path.
pub fn list_sdks() -> Vec<OptixInfo> {
    let mut roots: Vec<PathBuf> = Vec::new();
    for var in ROOT_VARS {
        if let Ok(v) = std::env::var(var)
            && !v.trim().is_empty()
        {
            roots.push(PathBuf::from(v));
        }
    }
    roots.extend(standard_roots());

    let mut found: Vec<OptixInfo> = Vec::new();
    for root in roots {
        if found.iter().any(|o| o.root == root) {
            continue;
        }
        if let Some(info) = probe(&root) {
            found.push(info);
        }
    }
    found.sort_by_key(|o| std::cmp::Reverse(o.version));
    found
}

/// The newest usable SDK, or `None` when no OptiX SDK is installed. An exact SDK is still
/// selectable with [`probe`].
pub fn detect_optix() -> Option<OptixInfo> {
    list_sdks().into_iter().next()
}

/// Standard install locations for the host platform.
fn standard_roots() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        // The installer places each SDK in its own "OptiX SDK <version>" folder under ProgramData.
        let base = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".to_string());
        subdirs_matching(&PathBuf::from(base).join("NVIDIA Corporation"), |n| n.starts_with("OptiX SDK"))
    }
    #[cfg(target_os = "linux")]
    {
        // No installer-defined location: the archive extracts wherever it is run.
        subdirs_matching(Path::new("/opt"), |n| n.to_ascii_lowercase().contains("optix"))
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        // OptiX does not ship for this platform.
        Vec::new()
    }
}

#[cfg(any(windows, target_os = "linux"))]
fn subdirs_matching(dir: &Path, keep: impl Fn(&str) -> bool) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir() && p.file_name().and_then(|n| n.to_str()).is_some_and(&keep))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a throwaway SDK tree with the two required headers and, optionally, the device header.
    fn fake_sdk(root: &Path, version: u32, abi: Option<u32>, device: bool) {
        let include = root.join("include");
        std::fs::create_dir_all(include.join("internal")).expect("include/internal");
        std::fs::write(
            include.join("optix.h"),
            format!("#define OPTIX_VERSION_MAJOR 1\n#define OPTIX_VERSION {version}\n"),
        )
        .expect("optix.h");
        if let Some(abi) = abi {
            std::fs::write(
                include.join("optix_function_table.h"),
                format!("#define OPTIX_ABI_VERSION {abi}\n"),
            )
            .expect("optix_function_table.h");
        }
        if device {
            std::fs::write(include.join("internal").join("optix_device_impl.h"), b"").expect("device impl");
        }
    }

    #[test]
    fn version_macro_round_trips() {
        let v = OptixVersion::from_macro(90100);
        assert_eq!((v.major, v.minor, v.micro), (9, 1, 0));
        assert_eq!(v.to_macro(), 90100);
        assert_eq!(OptixVersion::from_macro(80103).to_string(), "8.1.3");
    }

    #[test]
    fn a_define_prefix_does_not_match_a_longer_name() {
        // Mutation that reddens this: drop the whitespace check in `read_define`, and
        // `OPTIX_VERSION_MAJOR 1` (written first in the fake header) would be read as the version.
        let root = std::env::temp_dir().join("vcv_rs_optix_prefix_test");
        std::fs::remove_dir_all(&root).ok();
        fake_sdk(&root, 90100, Some(118), true);
        let info = probe(&root).expect("a well-formed SDK");
        assert_eq!(info.version.to_string(), "9.1.0");
        assert_eq!(info.abi, 118);
        assert!(info.device_impl);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_root_without_the_function_table_header_is_not_an_sdk() {
        // Without OPTIX_ABI_VERSION a host cannot query the driver's table: accepting it would
        // produce bindings that compile and fail at the first optixQueryFunctionTable.
        let root = std::env::temp_dir().join("vcv_rs_optix_partial_test");
        std::fs::remove_dir_all(&root).ok();
        fake_sdk(&root, 90100, None, false);
        assert!(probe(&root).is_none());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn the_header_outranks_the_directory_name_and_device_impl_is_reported() {
        let root = std::env::temp_dir().join("vcv_rs_optix_naming_test").join("OptiX SDK 7.7.0");
        std::fs::remove_dir_all(root.parent().expect("parent")).ok();
        fake_sdk(&root, 90000, Some(105), false);
        let info = probe(&root).expect("a well-formed SDK");
        assert_eq!(info.version.to_string(), "9.0.0");
        assert!(!info.device_impl, "absent device header must be reported, never assumed");
        std::fs::remove_dir_all(root.parent().expect("parent")).ok();
    }
}
