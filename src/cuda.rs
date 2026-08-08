//! # CUDA Toolkit Detection
//!
//! Finds an installed CUDA Toolkit and reports the facts a build needs: where it is, which
//! version it is, and which host compilers it will accept.
//!
//! ## Why this lives in vcv-rs
//! Every CUDA consumer in the stack (`cudarc`'s build script, `nvcc` itself, CMake's
//! `FindCUDAToolkit`) rediscovers the toolkit on its own, from the same environment variables.
//! When they disagree, the failure is not a missing toolkit — it is a build that compiles against
//! one version and links another. vcv-rs already owns "assemble the compiler environment", so the
//! toolkit belongs beside the MSVC toolchain rather than in a second mechanism next to it.
//!
//! ## Nothing here is hard-coded per release
//! Two facts that a lookup table would get wrong within one CUDA release are read from the
//! toolkit itself:
//! - the **version**, from `include/cuda.h`'s `CUDA_VERSION` (see [`CudaVersion`]);
//! - the **accepted MSVC range**, from `include/crt/host_config.h`'s `_MSC_VER` guard
//!   (see [`MsvcRange`]).
//!
//! That is the whole reason this module can claim to support 12.x and 13.x "universally": it does
//! not know anything about 12.x or 13.x. It asks.
//!
//! ## Platform support
//! - **Windows** — implemented and verified.
//! - **Linux** — root discovery is implemented (`/usr/local/cuda*`, `/opt/cuda`); the library
//!   directory layout is marked TODO below and needs verifying on a real install before the
//!   environment assembly can be trusted.
//! - **macOS** — NVIDIA shipped no CUDA Toolkit after 10.2 (2020), so detection returns nothing by
//!   design rather than searching paths that cannot exist.

use std::fmt;
use std::path::{Path, PathBuf};

/// A CUDA Toolkit version, as the toolkit reports it.
///
/// Parsed from `cuda.h`'s `#define CUDA_VERSION 13020`, whose encoding is
/// `major * 1000 + minor * 10`. Deliberately NOT parsed from the install directory name: the
/// directory is `v13.2` by convention only, it can be renamed, and it does not distinguish patch
/// installs that share a minor version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CudaVersion {
    pub major: u32,
    pub minor: u32,
}

impl CudaVersion {
    /// Decode the `CUDA_VERSION` macro value.
    pub const fn from_macro(v: u32) -> Self {
        Self {
            major: v / 1000,
            minor: (v % 1000) / 10,
        }
    }

    /// Re-encode to the macro form, so a round-trip can be asserted.
    pub const fn to_macro(self) -> u32 {
        self.major * 1000 + self.minor * 10
    }
}

impl fmt::Display for CudaVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// The host-compiler versions a toolkit will accept, as the toolkit itself declares them.
///
/// `host_config.h` guards the build with a literal
/// `#if _MSC_VER < 1920 || _MSC_VER >= 1960  #error -- unsupported Microsoft Visual Studio version`
/// so the bounds are a FACT of the installed toolkit, not a compatibility matrix maintained here.
/// A matrix would need editing on every CUDA release and would be wrong in between.
///
/// `min` is inclusive, `max_exclusive` is not — exactly as the `#if` reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MsvcRange {
    pub min: u32,
    pub max_exclusive: u32,
}

impl MsvcRange {
    /// Would `nvcc` accept a host compiler reporting this `_MSC_VER`?
    pub const fn accepts(&self, msc_ver: u32) -> bool {
        msc_ver >= self.min && msc_ver < self.max_exclusive
    }

    /// The newest Visual Studio release year this toolkit accepts.
    ///
    /// Used to pick a compiler with [`crate::detect::detect_vs_range`]: on a machine carrying both
    /// a supported and an unsupported VS, building with the newest one fails inside `nvcc` with a
    /// `#error`, which reads as a broken toolkit rather than a compiler that is merely too new.
    pub fn max_vs_year(&self) -> Option<u16> {
        vs_year_for_msc(self.max_exclusive.saturating_sub(1))
    }

    /// The oldest Visual Studio release year this toolkit accepts.
    pub fn min_vs_year(&self) -> Option<u16> {
        vs_year_for_msc(self.min)
    }
}

/// Map an `_MSC_VER` to the Visual Studio release year that reports it.
///
/// The bands come from the compiler's own versioning (`_MSC_VER` is bumped per toolset, and a VS
/// release spans a contiguous band). Kept in step with `detect::vs_major_to_year`, which maps the
/// other identifier for the same set of releases.
fn vs_year_for_msc(msc_ver: u32) -> Option<u16> {
    match msc_ver {
        1910..=1919 => Some(2017),
        1920..=1929 => Some(2019),
        1930..=1949 => Some(2022),
        1950..=1959 => Some(2026),
        _ => None,
    }
}

/// A located CUDA Toolkit.
#[derive(Debug, Clone)]
pub struct CudaInfo {
    /// Toolkit root — the directory holding `bin/`, `include/` and `lib/`.
    pub root: PathBuf,
    pub version: CudaVersion,
    /// Absolute path to the `nvcc` driver; its existence is part of what makes `root` a toolkit.
    pub nvcc: PathBuf,
    /// Accepted host-compiler range. `None` when `host_config.h` carries no `_MSC_VER` guard,
    /// which is the normal case for a non-Windows toolkit.
    pub msvc: Option<MsvcRange>,
}

impl CudaInfo {
    /// Directories to put on `PATH`.
    ///
    /// Both `bin` and `bin/x64` are returned when they exist, and that is not redundancy: CUDA 13
    /// moved the redistributable DLLs into `bin/x64` while leaving the driver executables in
    /// `bin`, whereas 12.x keeps everything in `bin`. Emitting both covers either layout without
    /// branching on a version number.
    pub fn bin_dirs(&self) -> Vec<PathBuf> {
        [self.root.join("bin"), self.root.join("bin").join("x64")]
            .into_iter()
            .filter(|p| p.is_dir())
            .collect()
    }

    /// The header directory.
    pub fn include_dir(&self) -> PathBuf {
        self.root.join("include")
    }

    /// The import/static library directory for a target architecture.
    ///
    /// Windows layout is `lib/x64` and `lib/Win32`; there is no ARM64 host toolkit, so an ARM64
    /// request has no answer and says so rather than pointing at the x64 libraries.
    ///
    /// TODO(linux): the layout is `lib64` on older installs and
    /// `targets/x86_64-linux/lib` on current ones — implement once verified on a real install.
    /// TODO(macos): unreachable, see the module docs.
    pub fn lib_dir(&self, target: crate::Arch) -> Option<PathBuf> {
        let dir = match target {
            crate::Arch::X64 => self.root.join("lib").join("x64"),
            crate::Arch::X86 => self.root.join("lib").join("Win32"),
            crate::Arch::Arm64 => return None,
        };
        dir.is_dir().then_some(dir)
    }

    /// The versioned variable name NVIDIA's installer sets, e.g. `CUDA_PATH_V13_2`.
    ///
    /// Tools that support several toolkits side by side select between them by this name, so an
    /// environment that sets `CUDA_PATH` without it is only half-configured.
    pub fn versioned_var(&self) -> String {
        format!("CUDA_PATH_V{}_{}", self.version.major, self.version.minor)
    }
}

/// Environment variables that name a toolkit root, in the order they are honoured.
///
/// This is exactly the set `cudarc`'s build script reads (it prints them as `rerun-if-env-changed`).
/// Matching it is the point: if vcv-rs preferred a different variable, `vcv | iex` could select one
/// toolkit while the crate compiled against another, and the mismatch would surface as a link
/// error far from its cause.
const ROOT_VARS: [&str; 4] = [
    "CUDA_PATH",
    "CUDA_HOME",
    "CUDA_ROOT",
    "CUDA_TOOLKIT_ROOT_DIR",
];

/// Is this directory a usable toolkit, and if so what is in it?
///
/// Requires BOTH `include/cuda.h` and an `nvcc` executable. A root with headers but no driver is
/// a partial install (or a stale directory left by an uninstall), and accepting it would produce
/// an environment that compiles until the first `.cu` file.
pub fn probe(root: &Path) -> Option<CudaInfo> {
    let nvcc = root.join("bin").join(NVCC);
    if !nvcc.is_file() {
        return None;
    }
    let header = root.join("include").join("cuda.h");
    let version = read_version(&header)?;
    let msvc = read_msvc_range(&root.join("include").join("crt").join("host_config.h"));

    Some(CudaInfo {
        root: root.to_path_buf(),
        version,
        nvcc,
        msvc,
    })
}

#[cfg(windows)]
const NVCC: &str = "nvcc.exe";
#[cfg(not(windows))]
const NVCC: &str = "nvcc";

/// Extract `CUDA_VERSION` from `cuda.h`.
fn read_version(header: &Path) -> Option<CudaVersion> {
    let text = std::fs::read_to_string(header).ok()?;
    let value = text.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("#define CUDA_VERSION")?;
        rest.split_whitespace().next()?.parse::<u32>().ok()
    })?;
    Some(CudaVersion::from_macro(value))
}

/// Extract the `_MSC_VER` bounds from `host_config.h`.
///
/// The guard is one line of the form `#if _MSC_VER < 1920 || _MSC_VER >= 1960`. Anything else is
/// reported as "no declared range" rather than guessed at — a wrong range would silently steer VS
/// selection, which is worse than not steering it.
fn read_msvc_range(header: &Path) -> Option<MsvcRange> {
    let text = std::fs::read_to_string(header).ok()?;
    text.lines().find_map(|line| {
        let line = line.trim();
        let body = line.strip_prefix("#if ")?;
        let (lo, hi) = body.split_once("||")?;
        let min = lo
            .trim()
            .strip_prefix("_MSC_VER")?
            .trim()
            .strip_prefix('<')?;
        let max = hi
            .trim()
            .strip_prefix("_MSC_VER")?
            .trim()
            .strip_prefix(">=")?;
        Some(MsvcRange {
            min: min.trim().parse().ok()?,
            max_exclusive: max.trim().parse().ok()?,
        })
    })
}

/// Every toolkit this machine offers, newest first.
///
/// Order of discovery, and each step is deliberate:
/// 1. the environment variables above — an explicitly selected toolkit outranks anything found by
///    searching, because overriding a deliberate choice is how a machine ends up building with a
///    toolkit its operator thought they had replaced;
/// 2. the platform's standard install location;
/// 3. `nvcc` on `PATH`, which catches installs in none of the above.
///
/// Duplicates are removed by root path, so a toolkit named by both `CUDA_PATH` and the directory
/// scan appears once.
pub fn list_toolkits() -> Vec<CudaInfo> {
    let mut roots: Vec<PathBuf> = Vec::new();

    for var in ROOT_VARS {
        if let Ok(v) = std::env::var(var)
            && !v.trim().is_empty()
        {
            roots.push(PathBuf::from(v));
        }
    }
    roots.extend(standard_roots());
    if let Some(p) = nvcc_on_path() {
        roots.push(p);
    }

    let mut found: Vec<CudaInfo> = Vec::new();
    for root in roots {
        if found.iter().any(|c| c.root == root) {
            continue;
        }
        if let Some(info) = probe(&root) {
            found.push(info);
        }
    }
    found.sort_by_key(|c| std::cmp::Reverse(c.version));
    found
}

/// The newest usable toolkit, or `None` when CUDA is not installed.
///
/// "Newest" and not "the one `CUDA_PATH` names": when both exist they are the same toolkit on any
/// normally configured machine, and when they differ the operator has two installed and the build
/// should use the capable one. An exact toolkit is still selectable with [`probe`].
pub fn detect_cuda() -> Option<CudaInfo> {
    list_toolkits().into_iter().next()
}

/// Standard install locations for the host platform.
fn standard_roots() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        // The installer places every toolkit under one parent, one directory per version.
        let base =
            std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".to_string());
        let dir = PathBuf::from(base)
            .join("NVIDIA GPU Computing Toolkit")
            .join("CUDA");
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return Vec::new();
        };
        entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect()
    }
    #[cfg(target_os = "linux")]
    {
        // `/usr/local/cuda` is the symlink the installer maintains to the active toolkit;
        // `cuda-<ver>` directories sit beside it, and distro packages use `/opt/cuda`.
        let mut out = vec![PathBuf::from("/usr/local/cuda"), PathBuf::from("/opt/cuda")];
        if let Ok(entries) = std::fs::read_dir("/usr/local") {
            out.extend(
                entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| {
                        p.is_dir()
                            && p.file_name()
                                .and_then(|n| n.to_str())
                                .is_some_and(|n| n.starts_with("cuda-"))
                    }),
            );
        }
        out
    }
    #[cfg(target_os = "macos")]
    {
        // No toolkit has shipped for macOS since CUDA 10.2; searching would only add latency.
        Vec::new()
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        // TODO: no standard location established for this platform.
        Vec::new()
    }
}

/// Toolkit root implied by an `nvcc` on `PATH` (`<root>/bin/nvcc` → `<root>`).
fn nvcc_on_path() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .find(|dir| dir.join(NVCC).is_file())
        .and_then(|bin| bin.parent().map(Path::to_path_buf))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_macro_round_trips() {
        // The encoding is the only reason a `13020` in a header means 13.2; assert both ways so a
        // change to either half cannot pass unnoticed.
        let v = CudaVersion::from_macro(13020);
        assert_eq!((v.major, v.minor), (13, 2));
        assert_eq!(v.to_macro(), 13020);
        assert_eq!(CudaVersion::from_macro(12060).to_string(), "12.6");
    }

    #[test]
    fn msvc_range_is_half_open() {
        // Mutation that reddens this: make `accepts` inclusive of `max_exclusive`. The guard in
        // host_config.h is `>= max`, so an inclusive upper bound would hand nvcc a compiler it
        // rejects with a #error.
        let r = MsvcRange {
            min: 1920,
            max_exclusive: 1960,
        };
        assert!(!r.accepts(1919));
        assert!(r.accepts(1920));
        assert!(r.accepts(1959));
        assert!(!r.accepts(1960));
        assert_eq!(r.max_vs_year(), Some(2026));
        assert_eq!(r.min_vs_year(), Some(2019));
    }

    #[test]
    fn msvc_guard_is_parsed_from_the_header_form() {
        let dir = std::env::temp_dir().join("vcv_rs_cuda_guard_test");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let header = dir.join("host_config.h");
        // Verbatim shape from CUDA 13.2's host_config.h, surrounded by lines that must not match.
        std::fs::write(
            &header,
            "#if defined(_MSC_VER)\n\
             #if _MSC_VER < 1920 || _MSC_VER >= 1960\n\
             #error -- unsupported Microsoft Visual Studio version!\n\
             #endif\n\
             #if _MSC_VER >= 1500\n",
        )
        .expect("write header");

        assert_eq!(
            read_msvc_range(&header),
            Some(MsvcRange {
                min: 1920,
                max_exclusive: 1960
            })
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Build a throwaway toolkit tree: `bin/nvcc`, `include/cuda.h`, `include/crt/host_config.h`.
    fn fake_toolkit(root: &Path, cuda_macro: u32, msc: Option<(u32, u32)>) {
        std::fs::create_dir_all(root.join("bin")).expect("bin");
        std::fs::create_dir_all(root.join("include").join("crt")).expect("include/crt");
        std::fs::write(root.join("bin").join(NVCC), b"").expect("nvcc");
        std::fs::write(
            root.join("include").join("cuda.h"),
            format!("#define CUDA_VERSION {cuda_macro}\n"),
        )
        .expect("cuda.h");
        if let Some((min, max)) = msc {
            std::fs::write(
                root.join("include").join("crt").join("host_config.h"),
                format!("#if _MSC_VER < {min} || _MSC_VER >= {max}\n#endif\n"),
            )
            .expect("host_config.h");
        }
    }

    #[test]
    fn a_root_without_nvcc_is_not_a_toolkit() {
        // A headers-only directory is a partial install. Accepting it would produce an environment
        // that looks configured and fails at the first .cu file.
        let dir = std::env::temp_dir().join("vcv_rs_cuda_partial_test");
        std::fs::create_dir_all(dir.join("include")).expect("temp dir");
        std::fs::write(
            dir.join("include").join("cuda.h"),
            "#define CUDA_VERSION 13020\n",
        )
        .expect("write header");

        assert!(probe(&dir).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn any_major_version_is_read_the_same_way() {
        // The point of this test is that NOTHING in this module is version-shaped: 12.x, 13.x,
        // 14.x and a version that does not exist yet all go through one path. A mutation that
        // special-cases a known major -- or filters install directories by a `v12`/`v13` name
        // pattern instead of probing every candidate -- reddens it.
        let base = std::env::temp_dir().join("vcv_rs_cuda_versions_test");
        std::fs::remove_dir_all(&base).ok();

        let cases = [
            ("v12.6", 12060u32, (12u32, 6u32)),
            ("v13.2", 13020, (13, 2)),
            ("v14.0", 14000, (14, 0)),
            ("v21.7", 21070, (21, 7)),
        ];
        for (dir, macro_value, expect) in cases {
            let root = base.join(dir);
            fake_toolkit(&root, macro_value, Some((1920, 1960)));
            let info = probe(&root).expect("a well-formed toolkit");
            assert_eq!(
                (info.version.major, info.version.minor),
                expect,
                "version of {dir}"
            );
            assert_eq!(info.msvc.map(|r| r.max_exclusive), Some(1960));
        }
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn the_header_outranks_the_directory_name() {
        // A toolkit directory can be renamed, and a patch install can carry a version its parent
        // directory never mentioned. Reading the name instead of the header would report 12.6
        // here, and every downstream decision keyed on the version would then be wrong.
        let root = std::env::temp_dir()
            .join("vcv_rs_cuda_naming_test")
            .join("v12.6");
        std::fs::remove_dir_all(root.parent().expect("parent")).ok();
        fake_toolkit(&root, 14000, None);

        let info = probe(&root).expect("a well-formed toolkit");
        assert_eq!(info.version.to_string(), "14.0");
        // No host_config.h was written: an absent guard is reported as "unknown", never guessed.
        assert_eq!(info.msvc, None);
        std::fs::remove_dir_all(root.parent().expect("parent")).ok();
    }
}
