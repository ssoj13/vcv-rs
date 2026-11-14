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

use crate::discovery::{RootRequest, first_root};

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

    /// Re-encode the macro, rejecting minor/micro above 99 and arithmetic overflow.
    pub const fn to_macro(self) -> Option<u32> {
        if self.minor > 99 || self.micro > 99 {
            return None;
        }
        let major = match self.major.checked_mul(10000) {
            Some(value) => value,
            None => return None,
        };
        let minor = match self.minor.checked_mul(100) {
            Some(value) => value,
            None => return None,
        };
        let version = match major.checked_add(minor) {
            Some(value) => value,
            None => return None,
        };
        version.checked_add(self.micro)
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

/// SDK root variables, in authoritative precedence order.
pub fn root_vars() -> &'static [&'static str] {
    &ROOT_VARS
}

/// The first nonempty configured root, without validating its contents.
/// `None` means no root variable supplied a path.
pub fn requested_root() -> Option<RootRequest> {
    first_root(root_vars())
}

/// Is this directory a usable SDK, and if so what is in it?
///
/// Requires `include/optix.h` with `OPTIX_VERSION` AND `include/optix_function_table.h` with
/// `OPTIX_ABI_VERSION`: without the ABI a host cannot query the driver's function table, so a root
/// missing it is a partial copy, not an SDK.
pub fn probe(root: &Path) -> Option<OptixInfo> {
    let include = root.join("include");
    let version =
        read_define(&include.join("optix.h"), "OPTIX_VERSION").map(OptixVersion::from_macro)?;
    let abi = read_define(&include.join("optix_function_table.h"), "OPTIX_ABI_VERSION")?;
    let device_impl = include
        .join("internal")
        .join("optix_device_impl.h")
        .is_file();
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
        let mut tokens = line.split_whitespace();
        if tokens.next()? != "#define" || tokens.next()? != name {
            return None;
        }
        tokens.next()?.parse::<u32>().ok()
    })
}

/// Every usable SDK from the first configured root and standard locations, newest first.
///
/// Duplicates are removed by root path. Inventory ordering does not override an explicit
/// root during selection; see [`detect_optix`].
pub fn list_sdks() -> Vec<OptixInfo> {
    inventory_with(requested_root().as_ref(), standard_roots())
}

fn inventory_with(
    request: Option<&RootRequest>,
    roots: impl IntoIterator<Item = PathBuf>,
) -> Vec<OptixInfo> {
    let roots = request.map(|r| r.path.clone()).into_iter().chain(roots);
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

/// Select the first configured root, or the newest standard SDK when no root is configured.
///
/// `None` means the configured root is unusable or no standard SDK was found. An invalid
/// explicit root never falls back to another installation.
pub fn detect_optix() -> Option<OptixInfo> {
    detect_with(requested_root().as_ref(), standard_roots())
}

fn detect_with(
    request: Option<&RootRequest>,
    roots: impl IntoIterator<Item = PathBuf>,
) -> Option<OptixInfo> {
    match request {
        Some(request) => probe(&request.path),
        None => inventory_with(None, roots).into_iter().next(),
    }
}

/// Standard install locations for the host platform.
fn standard_roots() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        // The installer places each SDK in its own "OptiX SDK <version>" folder under ProgramData.
        let base = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".to_string());
        subdirs_matching(&PathBuf::from(base).join("NVIDIA Corporation"), |n| {
            n.starts_with("OptiX SDK")
        })
    }
    #[cfg(target_os = "linux")]
    {
        // No installer-defined location: the archive extracts wherever it is run.
        subdirs_matching(Path::new("/opt"), |n| {
            n.to_ascii_lowercase().contains("optix")
        })
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
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("fixture creation time")
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "vcv_rs_optix_{}_{}_{}",
                std::process::id(),
                stamp,
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            // A successful exclusive create establishes ownership; never adopt or pre-delete a root.
            std::fs::create_dir(&root).expect("create owned OptiX fixture");
            Self(root)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            if let Err(error) = std::fs::remove_dir_all(&self.0) {
                if std::thread::panicking() {
                    eprintln!(
                        "could not remove OptiX fixture {}: {error}",
                        self.0.display()
                    );
                } else {
                    panic!(
                        "could not remove OptiX fixture {}: {error}",
                        self.0.display()
                    );
                }
            }
        }
    }

    #[test]
    fn fixture_roots_are_unique_and_cleanup_is_owned() {
        let first = Fixture::new();
        let second = Fixture::new();
        assert_ne!(first.0, second.0);
        let first_root = first.0.clone();
        let second_root = second.0.clone();
        let marker = second_root.join("owner");
        std::fs::write(&marker, b"second fixture").expect("write second fixture marker");
        drop(first);
        assert!(!first_root.exists(), "the dropped fixture must be removed");
        assert_eq!(
            std::fs::read(&marker).expect("other fixture remains"),
            b"second fixture"
        );
        drop(second);
        assert!(
            !second_root.exists(),
            "each fixture removes only its own directory"
        );
    }

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
            std::fs::write(include.join("internal").join("optix_device_impl.h"), b"")
                .expect("device impl");
        }
    }

    #[test]
    fn explicit_older_root_wins_over_newer_inventory() {
        let fixture = Fixture::new();
        let base = &fixture.0;
        let older = base.join("older");
        let newer = base.join("newer");
        fake_sdk(&older, 80000, Some(87), false);
        fake_sdk(&newer, 90100, Some(118), true);
        let request = RootRequest {
            variable: "OPTIX_ROOT",
            path: older.clone(),
        };
        let inventory = inventory_with(Some(&request), [newer.clone(), older.clone()]);
        assert_eq!(inventory.len(), 2);
        assert_eq!(inventory[0].root, newer);
        assert_eq!(
            detect_with(Some(&request), [newer.clone()])
                .expect("explicit SDK")
                .root,
            older
        );
        assert_eq!(
            detect_with(None, [older, newer.clone()])
                .expect("newest SDK")
                .root,
            newer
        );
    }

    #[test]
    fn invalid_explicit_root_rejects_selection_but_inventory_keeps_alternatives() {
        let fixture = Fixture::new();
        let base = &fixture.0;
        let alternative = base.join("alternative");
        fake_sdk(&alternative, 90100, Some(118), true);
        let request = RootRequest {
            variable: "OPTIX_ROOT",
            path: base.join("missing"),
        };
        assert!(detect_with(Some(&request), [alternative.clone()]).is_none());
        let inventory = inventory_with(Some(&request), [alternative.clone()]);
        assert_eq!(inventory.len(), 1);
        assert_eq!(inventory[0].root, alternative);
    }

    #[test]
    fn version_macro_round_trips() {
        let v = OptixVersion::from_macro(90100);
        assert_eq!((v.major, v.minor, v.micro), (9, 1, 0));
        assert_eq!(v.to_macro(), Some(90100));
        for value in [0, 80103, 90100, u32::MAX] {
            assert_eq!(OptixVersion::from_macro(value).to_macro(), Some(value));
        }
        assert_eq!(
            OptixVersion {
                major: u32::MAX,
                minor: 0,
                micro: 0
            }
            .to_macro(),
            None
        );
        assert_eq!(
            OptixVersion {
                major: 0,
                minor: 100,
                micro: 0
            }
            .to_macro(),
            None
        );
        assert_eq!(
            OptixVersion {
                major: 0,
                minor: 0,
                micro: 100
            }
            .to_macro(),
            None
        );
        assert_eq!(
            OptixVersion {
                major: 0,
                minor: u32::MAX,
                micro: 0
            }
            .to_macro(),
            None
        );
        assert_eq!(
            OptixVersion {
                major: 0,
                minor: 0,
                micro: u32::MAX
            }
            .to_macro(),
            None
        );
        assert_eq!(
            OptixVersion {
                major: 429_496,
                minor: 72,
                micro: 96
            }
            .to_macro(),
            None
        );
        assert_eq!(
            OptixVersion {
                major: 429_496,
                minor: 73,
                micro: 0
            }
            .to_macro(),
            None
        );
        assert_eq!(
            OptixVersion {
                major: 429_497,
                minor: 0,
                micro: 0
            }
            .to_macro(),
            None
        );
        assert_eq!(OptixVersion::from_macro(80103).to_string(), "8.1.3");
    }

    #[test]
    fn a_define_prefix_does_not_match_a_longer_name() {
        // Both the directive and macro name must be complete whitespace-separated tokens.
        let fixture = Fixture::new();
        let root = fixture.0.join("sdk");
        fake_sdk(&root, 90100, Some(118), true);
        let info = probe(&root).expect("a well-formed SDK");
        assert_eq!(info.version.to_string(), "9.1.0");
        assert_eq!(info.abi, 118);
        assert!(info.device_impl);
        let header = root.join("include/optix.h");
        std::fs::write(
            &header,
            "#defineOPTIX_VERSION 1\n#define OPTIX_VERSION_EXTRA 2\n",
        )
        .expect("write malformed tokens");
        assert_eq!(read_define(&header, "OPTIX_VERSION"), None);
        for directive in [
            "#define OPTIX_VERSION 90100\n",
            "#define\tOPTIX_VERSION\t90100\n",
        ] {
            std::fs::write(&header, directive).expect("write separated tokens");
            assert_eq!(read_define(&header, "OPTIX_VERSION"), Some(90100));
        }
    }

    #[test]
    fn a_root_without_the_function_table_header_is_not_an_sdk() {
        // Without OPTIX_ABI_VERSION a host cannot query the driver's table: accepting it would
        // produce bindings that compile and fail at the first optixQueryFunctionTable.
        let fixture = Fixture::new();
        let root = fixture.0.join("sdk");
        fake_sdk(&root, 90100, None, false);
        assert!(probe(&root).is_none());
    }

    #[test]
    fn the_header_outranks_the_directory_name_and_device_impl_is_reported() {
        let fixture = Fixture::new();
        let root = fixture.0.join("OptiX SDK 7.7.0");
        fake_sdk(&root, 90000, Some(105), false);
        let info = probe(&root).expect("a well-formed SDK");
        assert_eq!(info.version.to_string(), "9.0.0");
        assert!(
            !info.device_impl,
            "absent device header must be reported, never assumed"
        );
    }
}
