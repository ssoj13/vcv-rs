//! # CUDA Toolkit Detection
//!
//! Finds an installed CUDA Toolkit and reports the facts a build needs: where it is, which
//! version it is, and its declared MSVC eligibility bounds. These bounds do not prove
//! compatibility with a particular compiler or with GCC.
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
//! - **Windows** — discovers toolkits and target-tagged library directories.
//! - **Linux** — discovers standard roots and target-tagged libraries; legacy `lib64` requires
//!   native ELF architecture evidence from a CUDA dynamic library.
//! - **macOS** — NVIDIA shipped no CUDA Toolkit after 10.2 (2020), so detection returns nothing by
//!   design rather than searching paths that cannot exist.

use std::fmt;
use std::path::{Path, PathBuf};

use crate::Arch;
use crate::discovery::{self, RootRequest};

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

    /// Re-encode the macro, returning `None` for a minor above 99 or arithmetic overflow.
    pub const fn to_macro(self) -> Option<u32> {
        if self.minor > 99 {
            return None;
        }
        let major = match self.major.checked_mul(1000) {
            Some(value) => value,
            None => return None,
        };
        let minor = match self.minor.checked_mul(10) {
            Some(value) => value,
            None => return None,
        };
        major.checked_add(minor)
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
    /// Host-tool directories to put on `PATH`.
    ///
    /// Runtime-only directories are selected independently by [`Self::runtime_dir`].
    pub fn bin_dirs(&self) -> Vec<PathBuf> {
        [self.root.join("bin")]
            .into_iter()
            .filter(|p| p.is_dir())
            .collect()
    }

    /// The header directory.
    pub fn include_dir(&self) -> PathBuf {
        self.root.join("include")
    }

    /// A library directory with CUDA link files for the requested target architecture.
    ///
    /// Target-tagged directories establish architecture; empty directories do not count.
    /// Linux legacy `lib64` is eligible only for the native target and requires a matching ELF
    /// CUDA dynamic library. No target falls back to another architecture's directory.
    pub fn lib_dir(&self, target: Arch) -> Option<PathBuf> {
        #[cfg(windows)]
        {
            let tag = match target {
                Arch::X64 => "x64",
                Arch::X86 => "Win32",
                Arch::Arm64 => "arm64",
            };
            let dir = self.root.join("lib").join(tag);
            has_cuda_link_file(&dir, target).then_some(dir)
        }
        #[cfg(target_os = "linux")]
        {
            let tag = match target {
                Arch::X64 => "x86_64-linux",
                Arch::Arm64 => "aarch64-linux",
                Arch::X86 => return None,
            };
            let dir = self.root.join("targets").join(tag).join("lib");
            if has_cuda_link_file(&dir, target) {
                return Some(dir);
            }
            if Some(target) != Arch::native() {
                return None;
            }
            let legacy = self.root.join("lib64");
            has_native_cuda_library(&legacy, target).then_some(legacy)
        }
        #[cfg(not(any(windows, target_os = "linux")))]
        {
            let _ = target;
            None
        }
    }

    /// A directory containing a native-format CUDA runtime library for the requested target.
    ///
    /// Linux accepts matching tagged target directories, then native-target-only legacy `lib64`.
    /// Windows checks `bin` independently of host tools; `bin/x64` is eligible only for x64.
    /// Runtime sonames such as `libcudart.so.13` qualify here without establishing link metadata.
    pub fn runtime_dir(&self, target: Arch) -> Option<PathBuf> {
        #[cfg(windows)]
        {
            let bin = self.root.join("bin");
            [bin.clone(), bin.join("x64")].into_iter().find(|dir| {
                (target == Arch::X64 || dir == &bin) && has_cuda_runtime_file(dir, target)
            })
        }
        #[cfg(target_os = "linux")]
        {
            let tag = match target {
                Arch::X64 => "x86_64-linux",
                Arch::Arm64 => "aarch64-linux",
                Arch::X86 => return None,
            };
            let dir = self.root.join("targets").join(tag).join("lib");
            if has_cuda_runtime_file(&dir, target) {
                return Some(dir);
            }
            if Some(target) != Arch::native() {
                return None;
            }
            let legacy = self.root.join("lib64");
            has_cuda_runtime_file(&legacy, target).then_some(legacy)
        }
        #[cfg(not(any(windows, target_os = "linux")))]
        {
            let _ = target;
            None
        }
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
/// Matching it is the point: if vcv-rs preferred a different variable, `vcv-rs | iex` could select one
/// toolkit while the crate compiled against another, and the mismatch would surface as a link
/// error far from its cause.
const ROOT_VARS: [&str; 4] = [
    "CUDA_PATH",
    "CUDA_HOME",
    "CUDA_ROOT",
    "CUDA_TOOLKIT_ROOT_DIR",
];

/// Environment variables accepted for an explicit CUDA root, highest priority first.
pub fn root_vars() -> &'static [&'static str] {
    &ROOT_VARS
}

/// The first nonempty root requested by the environment, even if the path is invalid.
pub fn requested_root() -> Option<RootRequest> {
    discovery::first_root(root_vars())
}

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
        let mut tokens = line.split_whitespace();
        if tokens.next()? != "#define" || tokens.next()? != "CUDA_VERSION" {
            return None;
        }
        tokens.next()?.parse::<u32>().ok()
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

/// Every discovered toolkit, newest first. Inventory does not select the active toolkit.
///
/// Order of discovery, and each step is deliberate:
/// 1. the first nonempty explicit root, if it probes successfully;
/// 2. the platform's standard install location;
/// 3. `nvcc` on `PATH`, which catches installs in none of the above.
///
/// Duplicates are removed by root path, so a toolkit named by both `CUDA_PATH` and the directory
/// scan appears once.
pub fn list_toolkits() -> Vec<CudaInfo> {
    inventory_with(requested_root().as_ref(), searched_roots())
}

fn searched_roots() -> Vec<PathBuf> {
    let mut roots = standard_roots();
    if let Some(root) = nvcc_on_path() {
        roots.push(root);
    }
    roots
}

fn inventory_with(request: Option<&RootRequest>, roots: Vec<PathBuf>) -> Vec<CudaInfo> {
    let mut found: Vec<CudaInfo> = Vec::new();
    for root in request.into_iter().map(|r| r.path.clone()).chain(roots) {
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

/// The explicitly requested toolkit, or the newest discovered toolkit when no root is requested.
/// An invalid explicit root returns `None`; it never falls back to another alias or installation.
/// This reports host toolkit metadata; target library eligibility is checked by [`CudaInfo::lib_dir`].
pub fn detect_cuda() -> Option<CudaInfo> {
    detect_with(requested_root().as_ref(), searched_roots)
}

fn detect_with(
    request: Option<&RootRequest>,
    roots: impl FnOnce() -> Vec<PathBuf>,
) -> Option<CudaInfo> {
    match request {
        Some(request) => probe(&request.path),
        None => inventory_with(None, roots()).into_iter().next(),
    }
}

/// Standard install locations for the host platform.
fn standard_roots() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        // The installer places every toolkit under one parent, one directory per version.
        let base = std::env::var_os("ProgramFiles")
            .unwrap_or_else(|| std::ffi::OsString::from(r"C:\Program Files"));
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
    nvcc_on_path_with(&path)
}

fn nvcc_on_path_with(path: &std::ffi::OsStr) -> Option<PathBuf> {
    std::env::split_paths(path)
        .map(|dir| dir.join(NVCC))
        .filter_map(|file| nvcc_root_from_file(&file))
        .find(|root| probe(root).is_some())
}

fn nvcc_root_from_file(file: &Path) -> Option<PathBuf> {
    if !file.is_file() {
        return None;
    }
    let resolved = file.canonicalize().ok()?;
    resolved.parent()?.parent().map(Path::to_path_buf)
}

#[cfg(any(windows, target_os = "linux"))]
fn is_cuda_family(name: &str) -> bool {
    let name = name.strip_prefix("lib").unwrap_or(name);
    [
        "cudart",
        "cuda",
        "cublas",
        "cufft",
        "curand",
        "cusolver",
        "cusparse",
        "nvrtc",
        "nvJitLink",
        "cupti",
        "cufile",
        "npp",
        "nvjpeg",
        "nvfatbin",
        "nvblas",
        "nvptxcompiler",
    ]
    .iter()
    .any(|prefix| {
        name.get(..prefix.len()).is_some_and(|start| {
            if cfg!(windows) {
                start.eq_ignore_ascii_case(prefix)
            } else {
                start == *prefix
            }
        })
    })
}

#[cfg(any(windows, target_os = "linux"))]
fn is_cuda_library(name: &str, dynamic_only: bool) -> bool {
    #[cfg(windows)]
    let linkable = !dynamic_only
        && name
            .rsplit_once('.')
            .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("lib"));
    #[cfg(target_os = "linux")]
    let linkable = name.ends_with(".so") || (!dynamic_only && name.ends_with(".a"));
    is_cuda_family(name) && linkable
}

#[cfg(any(windows, target_os = "linux"))]
fn is_cuda_runtime_file(name: &str) -> bool {
    #[cfg(windows)]
    {
        let name = name.to_ascii_lowercase();
        is_cuda_family(&name) && name.ends_with(".dll")
    }
    #[cfg(target_os = "linux")]
    {
        is_cuda_family(name) && (name.ends_with(".so") || name.contains(".so."))
    }
}

#[cfg(any(windows, target_os = "linux"))]
fn has_cuda_runtime_file(dir: &Path, target: Arch) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| {
        let path = entry.path();
        path.is_file()
            && entry.file_name().to_str().is_some_and(is_cuda_runtime_file)
            && discovery::binary_arch(&path) == Some(target)
    })
}

#[cfg(any(windows, target_os = "linux"))]
fn has_cuda_link_file(dir: &Path, target: Arch) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| {
        let path = entry.path();
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return false;
        };
        if !path.is_file() || !is_cuda_library(name, false) {
            return false;
        }
        #[cfg(target_os = "linux")]
        if name.ends_with(".so") {
            return discovery::binary_arch(&path) == Some(target);
        }
        let format = if cfg!(windows) {
            object::BinaryFormat::Coff
        } else {
            object::BinaryFormat::Elf
        };
        discovery::archive_arch(&path, format) == Some(target)
    })
}

#[cfg(target_os = "linux")]
fn has_native_cuda_library(dir: &Path, target: Arch) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| {
        let path = entry.path();
        path.is_file()
            && entry
                .file_name()
                .to_str()
                .is_some_and(|name| is_cuda_library(name, true))
            && discovery::binary_arch(&path) == Some(target)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    fn fixture(label: &str) -> PathBuf {
        let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "vcv_rs_cuda_{label}_{}_{time}_{id}",
            std::process::id()
        ));
        std::fs::create_dir(&root).expect("create unique fixture");
        root
    }

    #[test]
    fn version_macro_round_trips() {
        // The encoding is the only reason a `13020` in a header means 13.2; assert both ways so a
        // change to either half cannot pass unnoticed.
        let v = CudaVersion::from_macro(13020);
        assert_eq!((v.major, v.minor), (13, 2));
        assert_eq!(v.to_macro(), Some(13020));
        for value in [0, 12060, 13020, 4_294_967_290] {
            assert_eq!(CudaVersion::from_macro(value).to_macro(), Some(value));
        }
        assert_eq!(
            CudaVersion {
                major: u32::MAX,
                minor: 0
            }
            .to_macro(),
            None
        );
        assert_eq!(
            CudaVersion {
                major: 1,
                minor: 100
            }
            .to_macro(),
            None
        );
        assert_eq!(
            CudaVersion {
                major: 0,
                minor: u32::MAX
            }
            .to_macro(),
            None
        );
        assert_eq!(
            CudaVersion {
                major: 4_294_967,
                minor: 30
            }
            .to_macro(),
            None
        );
        assert_eq!(
            CudaVersion {
                major: 4_294_968,
                minor: 0
            }
            .to_macro(),
            None
        );
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
        let dir = fixture("guard");
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
        std::fs::remove_dir_all(&dir).expect("remove own fixture");
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
        let dir = fixture("partial");
        std::fs::create_dir_all(dir.join("include")).expect("temp dir");
        std::fs::write(
            dir.join("include").join("cuda.h"),
            "#define CUDA_VERSION 13020\n",
        )
        .expect("write header");

        assert!(probe(&dir).is_none());
        std::fs::remove_dir_all(&dir).expect("remove own fixture");
    }

    #[test]
    fn any_major_version_is_read_the_same_way() {
        // The point of this test is that NOTHING in this module is version-shaped: 12.x, 13.x,
        // 14.x and a version that does not exist yet all go through one path. A mutation that
        // special-cases a known major -- or filters install directories by a `v12`/`v13` name
        // pattern instead of probing every candidate -- reddens it.
        let base = fixture("versions");

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
        std::fs::remove_dir_all(&base).expect("remove own fixture");
    }

    #[test]
    fn the_header_outranks_the_directory_name() {
        // A toolkit directory can be renamed, and a patch install can carry a version its parent
        // directory never mentioned. Reading the name instead of the header would report 12.6
        // here, and every downstream decision keyed on the version would then be wrong.
        let base = fixture("naming");
        let root = base.join("v12.6");
        fake_toolkit(&root, 14000, None);

        let info = probe(&root).expect("a well-formed toolkit");
        assert_eq!(info.version.to_string(), "14.0");
        // No host_config.h was written: an absent guard is reported as "unknown", never guessed.
        assert_eq!(info.msvc, None);
        std::fs::remove_dir_all(&base).expect("remove own fixture");
    }

    #[test]
    fn version_macro_name_must_match_exactly() {
        let base = fixture("macro_token");
        let header = base.join("cuda.h");
        std::fs::write(&header, "#define CUDA_VERSION123 13020\n").expect("write misleading macro");
        assert_eq!(read_version(&header), None);
        std::fs::write(
            &header,
            "#define CUDA_VERSION123 13020\n#define CUDA_VERSION 12060\n",
        )
        .expect("write exact macro");
        assert_eq!(read_version(&header), Some(CudaVersion::from_macro(12060)));
        std::fs::remove_dir_all(&base).expect("remove own fixture");
    }

    #[test]
    fn explicit_older_root_outranks_newer_inventory() {
        let base = fixture("selection");
        let old = base.join("old");
        let new = base.join("new");
        fake_toolkit(&old, 12060, None);
        fake_toolkit(&new, 13020, None);
        let request = RootRequest {
            variable: "CUDA_PATH",
            path: old.clone(),
        };
        let inventory = inventory_with(Some(&request), vec![new.clone(), old.clone()]);
        assert_eq!(inventory.len(), 2);
        assert_eq!(inventory[0].root, new);
        assert_eq!(inventory[1].root, old);
        assert_eq!(
            detect_with(Some(&request), || panic!("explicit root must skip scan"))
                .expect("explicit toolkit")
                .root,
            old
        );
        assert_eq!(
            detect_with(None, || vec![old.clone(), new.clone()])
                .expect("newest toolkit")
                .root,
            new
        );
        std::fs::remove_dir_all(&base).expect("remove own fixture");
    }

    #[test]
    fn invalid_explicit_root_never_uses_valid_search_or_lower_alias() {
        let base = fixture("invalid_request");
        let valid = base.join("valid");
        fake_toolkit(&valid, 13020, None);
        let request = discovery::first_root_with(root_vars(), |var| match var {
            "CUDA_PATH" => Some(base.join("missing").into_os_string()),
            "CUDA_HOME" => Some(valid.clone().into_os_string()),
            _ => None,
        })
        .expect("authoritative request");
        assert_eq!(request.variable, "CUDA_PATH");
        assert!(detect_with(Some(&request), || vec![valid.clone()]).is_none());
        assert_eq!(
            inventory_with(Some(&request), vec![valid.clone()])[0].root,
            valid
        );
        std::fs::remove_dir_all(&base).expect("remove own fixture");
    }

    #[test]
    fn path_scan_skips_stale_nvcc_but_explicit_root_remains_authoritative() {
        let base = fixture("path_scan");
        let stale = base.join("stale");
        let valid = base.join("valid");
        std::fs::create_dir_all(stale.join("bin")).expect("stale tools");
        std::fs::write(stale.join("bin").join(NVCC), []).expect("stale nvcc");
        fake_toolkit(&valid, 13020, None);
        let path =
            std::env::join_paths([stale.join("bin"), valid.join("bin")]).expect("injected PATH");
        assert_eq!(
            nvcc_on_path_with(&path),
            Some(valid.canonicalize().expect("canonical toolkit"))
        );
        let request = RootRequest {
            variable: "CUDA_PATH",
            path: stale,
        };
        assert!(detect_with(Some(&request), || panic!("explicit root must skip PATH")).is_none());
        std::fs::remove_dir_all(&base).expect("remove own fixture");
    }

    #[test]
    #[cfg(any(windows, target_os = "linux"))]
    fn toolkit_families_require_native_link_and_runtime_evidence() {
        use object::BinaryFormat::{Coff, Elf};
        let base = fixture("toolkit_families");
        let link = base.join("link");
        let runtime = base.join("runtime");
        std::fs::create_dir_all(&link).expect("link directory");
        std::fs::create_dir_all(&runtime).expect("runtime directory");
        let format = if cfg!(windows) { Coff } else { Elf };
        let foreign_format = if cfg!(windows) { Elf } else { Coff };
        for stem in [
            "nppc",
            "nvjpeg",
            "nvfatbin",
            "nvblas",
            "nvptxcompiler_static",
        ] {
            let name = if cfg!(windows) {
                format!("{stem}.lib")
            } else {
                format!("lib{stem}.a")
            };
            let file = link.join(&name);
            std::fs::write(&file, discovery::test_archive(format, Arch::X64))
                .expect("native archive");
            assert!(has_cuda_link_file(&link, Arch::X64), "{name}");
            assert!(
                !has_cuda_link_file(&link, Arch::Arm64),
                "{name}: foreign target"
            );
            std::fs::write(&file, discovery::test_archive(foreign_format, Arch::X64))
                .expect("foreign archive format");
            assert!(
                !has_cuda_link_file(&link, Arch::X64),
                "{name}: foreign format"
            );
            std::fs::write(&file, b"not an archive").expect("malformed archive");
            assert!(
                !has_cuda_link_file(&link, Arch::X64),
                "{name}: malformed archive"
            );
            std::fs::remove_file(&file).expect("remove isolated archive");
        }
        #[cfg(windows)]
        let runtime_names = [
            "nppc64_13.dll",
            "nvjpeg64_13.dll",
            "nvfatbin_130_0.dll",
            "nvblas64_13.dll",
        ];
        #[cfg(target_os = "linux")]
        let runtime_names = [
            "libnppc.so.13",
            "libnvjpeg.so.13",
            "libnvfatbin.so.13",
            "libnvblas.so.13",
        ];
        for name in runtime_names {
            let file = runtime.join(name);
            #[cfg(windows)]
            std::fs::write(&file, pe(Arch::X64)).expect("native runtime");
            #[cfg(target_os = "linux")]
            std::fs::write(&file, elf(Arch::X64)).expect("native runtime");
            assert!(has_cuda_runtime_file(&runtime, Arch::X64), "{name}");
            assert!(
                !has_cuda_runtime_file(&runtime, Arch::Arm64),
                "{name}: foreign target"
            );
            std::fs::write(&file, discovery::test_archive(format, Arch::X64))
                .expect("archive renamed as runtime");
            assert!(
                !has_cuda_runtime_file(&runtime, Arch::X64),
                "{name}: non-runtime format"
            );
            std::fs::remove_file(&file).expect("remove isolated runtime");
        }
        for stem in ["unrelated", "nvcomp", "cudnn", "nvsdk_ngx"] {
            let name = if cfg!(windows) {
                format!("{stem}.lib")
            } else {
                format!("lib{stem}.a")
            };
            let unrelated = link.join(name);
            std::fs::write(&unrelated, discovery::test_archive(format, Arch::X64))
                .expect("unrelated SDK archive");
            assert!(
                !has_cuda_link_file(&link, Arch::X64),
                "{stem} is not a CUDA Toolkit family"
            );
            std::fs::remove_file(unrelated).expect("remove unrelated SDK archive");
        }
        #[cfg(target_os = "linux")]
        assert!(
            !is_cuda_library("libNVJPEG.a", false),
            "Linux library names are case sensitive"
        );
        std::fs::remove_dir_all(&base).expect("remove own fixture");
    }

    #[test]
    fn root_alias_priority_skips_only_empty_values() {
        assert_eq!(
            root_vars(),
            &[
                "CUDA_PATH",
                "CUDA_HOME",
                "CUDA_ROOT",
                "CUDA_TOOLKIT_ROOT_DIR",
            ]
        );
        let request = discovery::first_root_with(root_vars(), |var| match var {
            "CUDA_PATH" => Some(std::ffi::OsString::new()),
            "CUDA_HOME" => Some(std::ffi::OsString::from(" ")),
            _ => panic!("a nonempty alias must stop lookup"),
        })
        .expect("whitespace remains a native nonempty path");
        assert_eq!(request.variable, "CUDA_HOME");
        assert_eq!(request.path, PathBuf::from(" "));
    }

    #[test]
    fn bin_directories_follow_the_host_platform() {
        let base = fixture("bin_dirs");
        fake_toolkit(&base, 13020, None);
        std::fs::create_dir_all(base.join("bin/x64")).expect("optional runtime bin");
        let info = probe(&base).expect("toolkit");
        assert_eq!(info.bin_dirs(), vec![base.join("bin")]);
        assert_eq!(
            info.runtime_dir(Arch::X64),
            None,
            "an empty runtime directory is not evidence"
        );
        std::fs::remove_dir_all(&base).expect("remove own fixture");
    }

    #[test]
    #[cfg(windows)]
    fn windows_libraries_require_matching_target_and_link_files() {
        let base = fixture("windows_libs");
        fake_toolkit(&base, 13020, None);
        let info = probe(&base).expect("toolkit");
        let x64 = base.join("lib/x64");
        std::fs::create_dir_all(&x64).expect("x64 lib");
        assert_eq!(info.lib_dir(Arch::X64), None);
        assert!(
            !is_cuda_library("cudart.LIB", true),
            "import libraries are not runtime files"
        );
        assert!(
            !is_cuda_library("unrelated.LIB", false),
            "the CUDA family remains required"
        );
        for name in ["CUDART.LIB", "CuBlAs.LiB", "cudart.lib"] {
            let library = x64.join(name);
            std::fs::write(
                &library,
                discovery::test_archive(object::BinaryFormat::Coff, Arch::X64),
            )
            .expect("x64 import library");
            assert_eq!(info.lib_dir(Arch::X64), Some(x64.clone()), "{name}");
            std::fs::remove_file(&library).expect("remove own import library");
            assert_eq!(info.lib_dir(Arch::X64), None);
        }
        assert_eq!(info.lib_dir(Arch::X86), None);
        assert_eq!(info.lib_dir(Arch::Arm64), None);
        for (target, tag) in [(Arch::X86, "Win32"), (Arch::Arm64, "arm64")] {
            let dir = base.join("lib").join(tag);
            std::fs::create_dir_all(&dir).expect("target lib");
            assert_eq!(info.lib_dir(target), None);
            std::fs::write(
                dir.join("cudart.lib"),
                discovery::test_archive(object::BinaryFormat::Coff, target),
            )
            .expect("target import library");
            assert_eq!(info.lib_dir(target), Some(dir));
        }
        std::fs::remove_dir_all(&base).expect("remove own fixture");
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn linux_tagged_libraries_do_not_fall_back_across_targets() {
        let base = fixture("linux_libs");
        fake_toolkit(&base, 13020, None);
        let info = probe(&base).expect("toolkit");
        let x64 = base.join("targets/x86_64-linux/lib");
        std::fs::create_dir_all(&x64).expect("x64 lib");
        assert_eq!(info.lib_dir(Arch::X64), None);
        std::fs::write(x64.join("libcudart.so.13"), elf(Arch::X64)).expect("runtime only");
        assert_eq!(info.lib_dir(Arch::X64), None);
        std::fs::write(x64.join("cudart.lib"), []).expect("foreign import library");
        assert_eq!(info.lib_dir(Arch::X64), None);
        let dynamic = x64.join("libcudart.so");
        std::fs::write(&dynamic, b"MZ foreign format").expect("foreign format");
        assert_eq!(info.lib_dir(Arch::X64), None);
        std::fs::write(&dynamic, elf(Arch::Arm64)).expect("wrong architecture");
        assert_eq!(info.lib_dir(Arch::X64), None);
        std::fs::write(&dynamic, elf(Arch::X64)).expect("x64 link library");
        assert_eq!(info.lib_dir(Arch::X64), Some(x64));
        assert_eq!(info.lib_dir(Arch::Arm64), None);
        assert_eq!(info.lib_dir(Arch::X86), None);
        let arm = base.join("targets/aarch64-linux/lib");
        std::fs::create_dir_all(&arm).expect("arm lib");
        assert_eq!(info.lib_dir(Arch::Arm64), None);
        std::fs::write(
            arm.join("libcudart_static.a"),
            discovery::test_archive(object::BinaryFormat::Elf, Arch::Arm64),
        )
        .expect("arm static library");
        assert_eq!(info.lib_dir(Arch::Arm64), Some(arm));
        assert_eq!(info.lib_dir(Arch::X86), None);
        std::fs::remove_dir_all(&base).expect("remove own fixture");
    }

    #[cfg(target_os = "linux")]
    fn elf(arch: Arch) -> [u8; 20] {
        let (class, machine): (u8, u16) = match arch {
            Arch::X86 => (1, 3),
            Arch::X64 => (2, 62),
            Arch::Arm64 => (2, 183),
        };
        let mut bytes = [0; 20];
        bytes[..4].copy_from_slice(b"\x7fELF");
        bytes[4] = class;
        bytes[5] = 1;
        bytes[6] = 1;
        bytes[18..20].copy_from_slice(&machine.to_le_bytes());
        bytes
    }

    #[test]
    #[cfg(all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    ))]
    fn linux_legacy_lib64_requires_native_elf_link_evidence() {
        let base = fixture("legacy_lib64");
        fake_toolkit(&base, 13020, None);
        let info = probe(&base).expect("toolkit");
        let native = Arch::native().expect("supported test host");
        let foreign = if native == Arch::X64 {
            Arch::Arm64
        } else {
            Arch::X64
        };
        let lib = base.join("lib64");
        std::fs::create_dir_all(&lib).expect("legacy lib");
        assert_eq!(info.lib_dir(native), None);
        let file = lib.join("libcudart.so");
        std::fs::write(&file, elf(foreign)).expect("foreign ELF");
        assert_eq!(info.lib_dir(native), None);
        std::fs::write(&file, elf(native)).expect("native ELF");
        assert_eq!(info.lib_dir(native), Some(lib.clone()));
        assert_eq!(info.lib_dir(foreign), None);
        std::fs::remove_file(&file).expect("remove native link file");
        std::fs::write(lib.join("libcudart.so.13"), elf(native)).expect("runtime only");
        assert_eq!(info.lib_dir(native), None);
        let mut pe = [0; 70];
        pe[..2].copy_from_slice(b"MZ");
        pe[0x3c..0x40].copy_from_slice(&64u32.to_le_bytes());
        pe[64..68].copy_from_slice(b"PE\0\0");
        pe[68..70].copy_from_slice(&0x8664u16.to_le_bytes());
        std::fs::write(&file, pe).expect("foreign format");
        assert_eq!(info.lib_dir(native), None);
        std::fs::remove_dir_all(&base).expect("remove own fixture");
    }

    #[cfg(any(windows, target_os = "linux"))]
    fn pe(arch: Arch) -> [u8; 70] {
        let machine: u16 = match arch {
            Arch::X86 => 0x014c,
            Arch::X64 => 0x8664,
            Arch::Arm64 => 0xaa64,
        };
        let mut bytes = [0; 70];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&64u32.to_le_bytes());
        bytes[64..68].copy_from_slice(b"PE\0\0");
        bytes[68..70].copy_from_slice(&machine.to_le_bytes());
        bytes
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn linux_tagged_runtime_evidence_is_separate_from_link_evidence() {
        let base = fixture("tagged_runtime");
        fake_toolkit(&base, 13020, None);
        let info = probe(&base).expect("toolkit");
        let dir = base.join("targets/x86_64-linux/lib");
        std::fs::create_dir_all(&dir).expect("target lib");
        let archive = dir.join("libcudart_static.a");
        std::fs::write(
            &archive,
            discovery::test_archive(object::BinaryFormat::Elf, Arch::X64),
        )
        .expect("static archive");
        assert_eq!(info.lib_dir(Arch::X64), Some(dir.clone()));
        assert_eq!(info.runtime_dir(Arch::X64), None);
        std::fs::remove_file(&archive).expect("remove own archive");
        let runtime = dir.join("libcudart.so.13");
        std::fs::write(&runtime, elf(Arch::X64)).expect("native runtime");
        assert_eq!(info.runtime_dir(Arch::X64), Some(dir.clone()));
        assert_eq!(info.lib_dir(Arch::X64), None);
        assert_eq!(info.runtime_dir(Arch::Arm64), None);
        assert_eq!(info.runtime_dir(Arch::X86), None);
        std::fs::write(&runtime, elf(Arch::Arm64)).expect("wrong target runtime");
        assert_eq!(info.runtime_dir(Arch::X64), None);
        std::fs::write(&runtime, pe(Arch::X64)).expect("foreign runtime format");
        assert_eq!(info.runtime_dir(Arch::X64), None);
        let arm = base.join("targets/aarch64-linux/lib");
        std::fs::create_dir_all(&arm).expect("arm target lib");
        std::fs::write(arm.join("libcudart.so.13"), elf(Arch::Arm64)).expect("arm runtime");
        assert_eq!(info.runtime_dir(Arch::Arm64), Some(arm));
        assert_eq!(info.runtime_dir(Arch::X64), None);
        std::fs::remove_dir_all(&base).expect("remove own fixture");
    }

    #[test]
    #[cfg(all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    ))]
    fn linux_legacy_runtime_requires_native_target_and_binary() {
        let base = fixture("legacy_runtime");
        fake_toolkit(&base, 13020, None);
        let info = probe(&base).expect("toolkit");
        let native = Arch::native().expect("supported test host");
        let foreign = if native == Arch::X64 {
            Arch::Arm64
        } else {
            Arch::X64
        };
        let dir = base.join("lib64");
        std::fs::create_dir_all(&dir).expect("legacy lib");
        std::fs::write(
            dir.join("libcudart_static.a"),
            discovery::test_archive(object::BinaryFormat::Elf, native),
        )
        .expect("static archive");
        assert_eq!(info.runtime_dir(native), None);
        let runtime = dir.join("libcudart.so.13");
        std::fs::write(&runtime, elf(native)).expect("native soname");
        assert_eq!(info.runtime_dir(native), Some(dir));
        assert_eq!(info.lib_dir(native), None);
        assert_eq!(info.runtime_dir(foreign), None);
        std::fs::write(&runtime, elf(foreign)).expect("foreign architecture");
        assert_eq!(info.runtime_dir(native), None);
        assert_eq!(info.runtime_dir(foreign), None);
        std::fs::write(&runtime, pe(native)).expect("foreign format");
        assert_eq!(info.runtime_dir(native), None);
        std::fs::remove_dir_all(&base).expect("remove own fixture");
    }

    #[test]
    #[cfg(windows)]
    fn windows_runtime_requires_matching_pe_and_eligible_bin_candidate() {
        let base = fixture("windows_runtime");
        fake_toolkit(&base, 13020, None);
        let info = probe(&base).expect("toolkit");
        let libraries = base.join("lib/x64");
        std::fs::create_dir_all(&libraries).expect("import library dir");
        std::fs::write(
            libraries.join("cudart.lib"),
            discovery::test_archive(object::BinaryFormat::Coff, Arch::X64),
        )
        .expect("import library");
        assert_eq!(info.lib_dir(Arch::X64), Some(libraries));
        assert_eq!(info.runtime_dir(Arch::X64), None);
        let x64 = base.join("bin/x64");
        std::fs::create_dir_all(&x64).expect("runtime candidate");
        let runtime = x64.join("CUDART64_130.DLL");
        std::fs::write(&runtime, pe(Arch::X64)).expect("x64 runtime");
        assert_eq!(info.runtime_dir(Arch::X64), Some(x64));
        assert_eq!(
            info.bin_dirs(),
            vec![base.join("bin")],
            "verified runtime stays separate from tools"
        );
        assert_eq!(info.runtime_dir(Arch::Arm64), None);
        assert_eq!(info.runtime_dir(Arch::X86), None);
        for foreign in [Arch::Arm64, Arch::X86] {
            std::fs::write(&runtime, pe(foreign)).expect("foreign target in x64 dir");
            assert_eq!(info.runtime_dir(Arch::X64), None);
            assert_eq!(info.runtime_dir(foreign), None);
        }
        let mut elf = [0; 20];
        elf[..4].copy_from_slice(b"\x7fELF");
        elf[4] = 2;
        elf[5] = 1;
        elf[6] = 1;
        elf[18..20].copy_from_slice(&62u16.to_le_bytes());
        std::fs::write(&runtime, elf).expect("foreign ELF format");
        assert_eq!(info.runtime_dir(Arch::X64), None);
        let bin = base.join("bin");
        let host_runtime = bin.join("CuDaRt.DlL");
        for target in [Arch::Arm64, Arch::X86, Arch::X64] {
            std::fs::write(&host_runtime, pe(target)).expect("matching bin runtime");
            assert_eq!(info.runtime_dir(target), Some(bin.clone()));
        }
        std::fs::remove_dir_all(&base).expect("remove own fixture");
    }

    #[test]
    #[cfg(unix)]
    fn nvcc_file_symlink_resolves_before_deriving_the_root() {
        let base = fixture("nvcc_symlink");
        let toolkit = base.join("toolkit");
        fake_toolkit(&toolkit, 13020, None);
        let aliases = base.join("aliases");
        std::fs::create_dir(&aliases).expect("alias dir");
        let link = aliases.join(NVCC);
        std::os::unix::fs::symlink(toolkit.join("bin").join(NVCC), &link).expect("nvcc symlink");
        assert_eq!(
            nvcc_root_from_file(&link),
            Some(toolkit.canonicalize().expect("real root"))
        );
        assert!(nvcc_root_from_file(&aliases.join("missing")).is_none());
        std::fs::remove_dir_all(&base).expect("remove own fixture");
    }
}
