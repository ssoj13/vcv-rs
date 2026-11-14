//! # Environment Builder Module
//!
//! Assembles target-specific compiler, SDK, and vcpkg environments on Windows and Linux.
//!
//! ## Purpose
//! Keeps executable paths, shared-library runtime directories, headers, linker
//! libraries, and Windows assembly references separate. Formatters map these
//! components to the native environment variable names.
//!
//! ## Key Functions
//! - `build_env()` - Main function that assembles all paths based on host/target arch
//! - `add_cuda()` - Appends a CUDA Toolkit to an already-built environment (`cuda` feature)
//! - `add_optix()` - Appends an OptiX SDK's headers and root variables (`optix` feature)
//! - `add_nvsdk()` - Appends cuDNN / Video Codec / Optical Flow / NVAPI / NGX / nvCOMP (`nvsdk` feature)
//! - `add_vcpkg()` - Appends target-compatible installed headers, libraries, and runtime metadata
//!
//! ## Dependencies
//! - `detect` module for VsInfo/SdkInfo structs
//! - `std::collections::BTreeMap` for stable key ordering

use crate::Arch;
#[cfg(feature = "cuda")]
use crate::cuda::CudaInfo;
#[cfg(windows)]
use crate::detect::{SdkInfo, VsInfo};
#[cfg(feature = "nvsdk")]
use crate::nvsdk::SdkInfo as NvSdkInfo;
#[cfg(feature = "optix")]
use crate::optix::OptixInfo;
use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::{Path, PathBuf};

/// Assembled environment for one explicit target architecture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Env {
    pub target: Arch,
    pub path: Vec<PathBuf>,
    pub runtime: Vec<PathBuf>,
    pub include: Vec<PathBuf>,
    pub lib: Vec<PathBuf>,
    pub libpath: Vec<PathBuf>,
    pub vars: BTreeMap<String, OsString>,
}

impl Env {
    pub fn new(target: Arch) -> Self {
        Self {
            target,
            path: Vec::new(),
            runtime: Vec::new(),
            include: Vec::new(),
            lib: Vec::new(),
            libpath: Vec::new(),
            vars: BTreeMap::new(),
        }
    }

    fn add_if_exists(lst: &mut Vec<PathBuf>, paths: &[PathBuf]) {
        for p in paths {
            if p.is_dir() && !lst.contains(p) {
                lst.push(p.clone());
            }
        }
    }
}

/// An environment component cannot be applied to this target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvError {
    TargetMismatch { environment: Arch, component: Arch },
    MissingCudaLibrary { target: Arch },
    UnsupportedSdk { sdk: &'static str },
    UnsupportedVcpkgTriplet { triplet: String },
}

impl fmt::Display for EnvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TargetMismatch {
                environment,
                component,
            } => write!(
                f,
                "environment target {environment:?} differs from component target {component:?}"
            ),
            Self::MissingCudaLibrary { target } => {
                write!(f, "CUDA has no native library directory for {target:?}")
            }
            Self::UnsupportedSdk { sdk } => {
                write!(f, "{sdk} is unsupported on this operating system")
            }
            Self::UnsupportedVcpkgTriplet { triplet } => {
                write!(f, "unsupported vcpkg triplet {triplet}")
            }
        }
    }
}

impl std::error::Error for EnvError {}

#[cfg(windows)]
fn with_trailing_backslash(value: &OsStr) -> OsString {
    let mut value = value.to_os_string();
    value.push("\\");
    value
}

/// Build complete Windows MSVC environment.
#[cfg(windows)]
pub fn build_env(
    vs: &VsInfo,
    sdk: Option<&SdkInfo>,
    ucrt: Option<&SdkInfo>,
    host: Arch,
    target: Arch,
) -> Env {
    let mut env = Env::new(target);
    let tp = &vs.tools;

    let hd = match host {
        Arch::X64 => "Hostx64",
        Arch::X86 => "Hostx86",
        Arch::Arm64 => "Hostarm64",
    };
    let tgt = target.as_str();

    // VC++ binaries
    Env::add_if_exists(&mut env.path, &[tp.join("bin").join(hd).join(tgt)]);
    if host != target {
        let host_str = host.as_str();
        Env::add_if_exists(&mut env.path, &[tp.join("bin").join(hd).join(host_str)]);
    }

    // VC++ headers & libs
    Env::add_if_exists(
        &mut env.include,
        &[tp.join("include"), tp.join("ATLMFC").join("include")],
    );
    Env::add_if_exists(
        &mut env.lib,
        &[
            tp.join("lib").join(tgt),
            tp.join("ATLMFC").join("lib").join(tgt),
        ],
    );
    Env::add_if_exists(
        &mut env.libpath,
        &[
            tp.join("lib").join(tgt),
            tp.join("ATLMFC").join("lib").join(tgt),
        ],
    );

    // Windows SDK
    if let Some(sdk) = sdk {
        let sp = &sdk.path;
        let sv = &sdk.version;
        let host_str = host.as_str();

        Env::add_if_exists(&mut env.path, &[sp.join("bin").join(sv).join(host_str)]);
        // Note: ucrt include is added from UCRT section, not here
        Env::add_if_exists(
            &mut env.include,
            &[
                sp.join("include").join(sv).join("um"),
                sp.join("include").join(sv).join("shared"),
                sp.join("include").join(sv).join("winrt"),
                sp.join("include").join(sv).join("cppwinrt"),
            ],
        );
        Env::add_if_exists(
            &mut env.lib,
            &[sp.join("lib").join(sv).join("um").join(tgt)],
        );
        Env::add_if_exists(
            &mut env.libpath,
            &[
                sp.join("UnionMetadata").join(sv),
                sp.join("References").join(sv),
            ],
        );
    }

    // UCRT
    if let Some(ucrt) = ucrt {
        let up = &ucrt.path;
        let uv = &ucrt.version;

        Env::add_if_exists(
            &mut env.include,
            &[up.join("include").join(uv).join("ucrt")],
        );
        Env::add_if_exists(
            &mut env.lib,
            &[up.join("lib").join(uv).join("ucrt").join(tgt)],
        );
    }

    // Standard variables
    env.vars.insert(
        "VSINSTALLDIR".into(),
        with_trailing_backslash(vs.install.as_os_str()),
    );
    env.vars.insert(
        "VCINSTALLDIR".into(),
        with_trailing_backslash(vs.vc.as_os_str()),
    );
    env.vars.insert(
        "VCToolsInstallDir".into(),
        with_trailing_backslash(tp.as_os_str()),
    );
    env.vars
        .insert("VCToolsVersion".into(), vs.tools_ver.clone().into());
    env.vars
        .insert("VisualStudioVersion".into(), vs.vs_ver().into());
    env.vars.insert("Platform".into(), tgt.into());

    if let Some(sdk) = sdk {
        env.vars.insert(
            "WindowsSdkDir".into(),
            with_trailing_backslash(sdk.path.as_os_str()),
        );
        env.vars.insert(
            "WindowsSDKVersion".into(),
            with_trailing_backslash(OsStr::new(&sdk.version)),
        );
    }

    if let Some(ucrt) = ucrt {
        env.vars.insert(
            "UniversalCRTSdkDir".into(),
            with_trailing_backslash(ucrt.path.as_os_str()),
        );
        env.vars
            .insert("UCRTVersion".into(), ucrt.version.clone().into());
    }

    // Windows sets these at logon. Agent and service shells drop them. vcpkg finds
    // vswhere.exe only as `%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe`
    // and otherwise reports that no complete Visual Studio instance exists.
    // A value the process already has is left alone.
    for (key, path) in
        program_files_if_missing(|k| std::env::var_os(k).is_some(), |p| Path::new(p).is_dir())
    {
        env.vars.insert((*key).to_string(), OsString::from(path));
    }

    env
}

/// Standard program-files locations, restored only when the variable is unset and the directory exists.
#[cfg(any(windows, test))]
fn program_files_if_missing(
    is_set: impl Fn(&str) -> bool,
    dir_exists: impl Fn(&str) -> bool,
) -> Vec<(&'static str, &'static str)> {
    [
        ("ProgramFiles(x86)", "C:/Program Files (x86)"),
        ("ProgramFiles", "C:/Program Files"),
        ("ProgramW6432", "C:/Program Files"),
    ]
    .into_iter()
    .filter(|(key, path)| !is_set(key) && dir_exists(path))
    .collect()
}

/// Append CUDA tools, headers, native libraries, and all root aliases.
/// A missing target library fails before any environment fields change.
#[cfg(feature = "cuda")]
pub fn add_cuda(env: &mut Env, cuda: &CudaInfo) -> Result<(), EnvError> {
    let lib = cuda
        .lib_dir(env.target)
        .ok_or(EnvError::MissingCudaLibrary { target: env.target })?;
    Env::add_if_exists(&mut env.path, &cuda.bin_dirs());
    Env::add_if_exists(&mut env.include, &[cuda.include_dir()]);
    Env::add_if_exists(&mut env.lib, std::slice::from_ref(&lib));
    if let Some(runtime) = cuda.runtime_dir(env.target) {
        Env::add_if_exists(&mut env.runtime, std::slice::from_ref(&runtime));
    }
    for var in crate::cuda::root_vars() {
        env.vars
            .insert((*var).into(), cuda.root.as_os_str().to_os_string());
    }
    env.vars
        .insert(cuda.versioned_var(), cuda.root.as_os_str().to_os_string());
    Ok(())
}

/// Append OptiX headers and all root aliases.
#[cfg(feature = "optix")]
pub fn add_optix(env: &mut Env, optix: &OptixInfo) {
    Env::add_if_exists(&mut env.include, &[optix.include_dir()]);
    for var in crate::optix::root_vars() {
        env.vars
            .insert((*var).into(), optix.root.as_os_str().to_os_string());
    }
}

/// Append an SDK's headers, optional native libraries, runtime paths, and root aliases.
#[cfg(feature = "nvsdk")]
pub fn add_nvsdk(env: &mut Env, sdk: &NvSdkInfo) -> Result<(), EnvError> {
    if env.target != sdk.target {
        return Err(EnvError::TargetMismatch {
            environment: env.target,
            component: sdk.target,
        });
    }
    if !sdk.kind.is_supported() {
        return Err(EnvError::UnsupportedSdk {
            sdk: sdk.kind.name(),
        });
    }
    Env::add_if_exists(&mut env.include, std::slice::from_ref(&sdk.include_dir));
    if let Some(lib) = &sdk.lib_dir {
        Env::add_if_exists(&mut env.lib, std::slice::from_ref(lib));
    }
    if let Some(runtime) = &sdk.runtime_dir {
        Env::add_if_exists(&mut env.runtime, std::slice::from_ref(runtime));
    }
    for var in sdk.kind.root_vars() {
        env.vars
            .insert((*var).into(), sdk.root.as_os_str().to_os_string());
    }
    Ok(())
}

/// Installed vcpkg tree for one native-platform target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VcpkgInfo {
    pub target: Arch,
    pub root: PathBuf,
    pub triplet: String,
    pub lib: PathBuf,
    pub include: PathBuf,
    pub runtime_dir: Option<PathBuf>,
}

/// The first nonempty configured vcpkg root, including an invalid path.
pub fn requested_vcpkg_root() -> Option<crate::discovery::RootRequest> {
    crate::discovery::first_root(&["VCPKG_ROOT"])
}

/// The configured triplet without trimming, Unicode conversion, or validation.
pub fn requested_vcpkg_triplet() -> Option<OsString> {
    std::env::var_os("VCPKG_DEFAULT_TRIPLET").filter(|value| !value.is_empty())
}

/// `VCPKG_ROOT` when it names an existing directory.
pub fn detect_vcpkg() -> Option<PathBuf> {
    let root = requested_vcpkg_root()?.path;
    root.is_dir().then_some(root)
}

fn vcpkg_triplet_candidates(target: Arch, explicit: Option<&OsStr>, windows: bool) -> Vec<String> {
    let suffixes: &[&str] = if windows {
        &["windows", "windows-static-md", "windows-static"]
    } else if target != Arch::X86 {
        &["linux", "linux-dynamic"]
    } else {
        // No supported native x86 Linux triplets in this discovery contract.
        &[]
    };
    let supported: Vec<String> = suffixes
        .iter()
        .map(|suffix| format!("{}-{suffix}", target.as_str()))
        .collect();
    match explicit.filter(|value| !value.is_empty()) {
        Some(value) => supported
            .into_iter()
            .filter(|candidate| value == OsStr::new(candidate))
            .collect(),
        None => supported,
    }
}

/// Locate native-platform `installed/<triplet>/{lib,include}`.
/// An explicit nonempty triplet is authoritative, including when invalid or uninstalled.
pub fn probe_vcpkg(root: &Path, target: Arch) -> Option<VcpkgInfo> {
    if !cfg!(any(target_os = "windows", target_os = "linux")) {
        return None;
    }
    probe_vcpkg_with(
        root,
        target,
        requested_vcpkg_triplet().as_deref(),
        cfg!(windows),
        |path| path.is_dir(),
        has_vcpkg_runtime,
    )
}

fn probe_vcpkg_with(
    root: &Path,
    target: Arch,
    explicit: Option<&OsStr>,
    windows: bool,
    dir_exists: impl Fn(&Path) -> bool,
    runtime_exists: impl Fn(&Path, Arch, bool) -> bool,
) -> Option<VcpkgInfo> {
    for triplet in vcpkg_triplet_candidates(target, explicit, windows) {
        let installed = root.join("installed").join(&triplet);
        let lib = installed.join("lib");
        let include = installed.join("include");
        if !dir_exists(&lib) || !dir_exists(&include) {
            continue;
        }
        let runtime = if windows {
            installed.join("bin")
        } else {
            lib.clone()
        };
        return Some(VcpkgInfo {
            target,
            root: root.to_path_buf(),
            triplet,
            lib,
            include,
            runtime_dir: runtime_exists(&runtime, target, windows).then_some(runtime),
        });
    }
    None
}

fn vcpkg_runtime_candidate(
    path: &Path,
    target: Arch,
    windows: bool,
    arch: impl Fn(&Path) -> Option<Arch>,
) -> bool {
    let Some(name) = path.file_name().and_then(OsStr::to_str) else {
        return false;
    };
    let shared = if windows {
        path.extension()
            .and_then(OsStr::to_str)
            .is_some_and(|ext| ext.eq_ignore_ascii_case("dll"))
    } else {
        name.ends_with(".so")
            || name.split_once(".so.").is_some_and(|(_, version)| {
                !version.is_empty()
                    && version.split('.').all(|part| {
                        !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit())
                    })
            })
    };
    shared && arch(path) == Some(target)
}

fn has_vcpkg_runtime(dir: &Path, target: Arch, windows: bool) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| {
        let path = entry.path();
        path.is_file()
            && vcpkg_runtime_candidate(&path, target, windows, crate::discovery::binary_arch)
    })
}

/// Append native vcpkg headers, libraries, runtime directories, and root variable.
pub fn add_vcpkg(env: &mut Env, vcpkg: &VcpkgInfo) -> Result<(), EnvError> {
    if env.target != vcpkg.target {
        return Err(EnvError::TargetMismatch {
            environment: env.target,
            component: vcpkg.target,
        });
    }
    if !cfg!(any(target_os = "windows", target_os = "linux"))
        || !vcpkg_triplet_candidates(env.target, None, cfg!(windows)).contains(&vcpkg.triplet)
    {
        return Err(EnvError::UnsupportedVcpkgTriplet {
            triplet: vcpkg.triplet.clone(),
        });
    }
    Env::add_if_exists(&mut env.include, std::slice::from_ref(&vcpkg.include));
    Env::add_if_exists(&mut env.lib, std::slice::from_ref(&vcpkg.lib));
    #[cfg(windows)]
    Env::add_if_exists(&mut env.libpath, std::slice::from_ref(&vcpkg.lib));
    if let Some(runtime) = &vcpkg.runtime_dir {
        Env::add_if_exists(&mut env.runtime, std::slice::from_ref(runtime));
    }
    env.vars
        .insert("VCPKG_ROOT".into(), vcpkg.root.as_os_str().to_os_string());
    env.vars.insert(
        "VCPKG_DEFAULT_TRIPLET".into(),
        OsString::from(vcpkg.triplet.as_str()),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn write_dir(dirs: &mut BTreeSet<PathBuf>, p: &Path) {
        dirs.insert(p.to_path_buf());
    }

    fn installed(root: &Path, triplet: &str) -> BTreeSet<PathBuf> {
        let mut dirs = BTreeSet::new();
        for sub in ["lib", "include"] {
            write_dir(&mut dirs, &root.join("installed").join(triplet).join(sub));
        }
        dirs
    }

    #[test]
    fn probe_prefers_default_triplet_when_installed() {
        let root = Path::new("fixture");
        let mut dirs = installed(root, "x64-windows-static-md");
        dirs.extend(installed(root, "x64-windows"));
        let info = probe_vcpkg_with(
            root,
            Arch::X64,
            Some(OsStr::new("x64-windows-static-md")),
            true,
            |path| dirs.contains(path),
            |_, _, _| false,
        )
        .expect("probe");
        assert_eq!(info.triplet, "x64-windows-static-md");
        assert_eq!(info.target, Arch::X64);
    }

    #[test]
    fn probe_falls_back_to_x64_windows() {
        let root = Path::new("fixture");
        let dirs = installed(root, "x64-windows");
        let info = probe_vcpkg_with(
            root,
            Arch::X64,
            None,
            true,
            |path| dirs.contains(path),
            |_, _, _| false,
        )
        .expect("probe");
        assert_eq!(info.triplet, "x64-windows");
        assert!(info.runtime_dir.is_none());
    }

    #[test]
    fn program_files_only_when_unset_and_present() {
        assert!(program_files_if_missing(|_| true, |_| true).is_empty());
        let x86 = program_files_if_missing(|_| false, |p| p.contains("x86"));
        assert_eq!(x86, vec![("ProgramFiles(x86)", "C:/Program Files (x86)")]);
    }

    #[test]
    fn explicit_triplets_never_fall_back() {
        let root = Path::new("fixture");
        let dirs = installed(root, "x64-windows");
        for value in [
            "x64-windows-static",
            "arm64-windows",
            "x64-linux",
            "custom",
            "../x64-windows",
            " x64-windows ",
            " ",
        ] {
            assert!(
                probe_vcpkg_with(
                    root,
                    Arch::X64,
                    Some(OsStr::new(value)),
                    true,
                    |path| dirs.contains(path),
                    |_, _, _| false
                )
                .is_none(),
                "{value}"
            );
        }
        assert!(
            probe_vcpkg_with(
                root,
                Arch::X64,
                Some(OsStr::new("")),
                true,
                |path| dirs.contains(path),
                |_, _, _| false
            )
            .is_some()
        );
    }

    #[test]
    fn linux_triplets_are_target_and_platform_specific() {
        let root = Path::new("fixture");
        let mut dirs = installed(root, "arm64-linux-dynamic");
        dirs.extend(installed(root, "x64-windows"));
        let info = probe_vcpkg_with(
            root,
            Arch::Arm64,
            None,
            false,
            |path| dirs.contains(path),
            |_, _, _| false,
        )
        .expect("arm64 Linux");
        assert_eq!(info.triplet, "arm64-linux-dynamic");
        assert!(info.runtime_dir.is_none());
        assert!(
            probe_vcpkg_with(
                root,
                Arch::X64,
                None,
                false,
                |path| dirs.contains(path),
                |_, _, _| false
            )
            .is_none()
        );
        assert!(vcpkg_triplet_candidates(Arch::X86, None, false).is_empty());
    }

    #[test]
    fn probe_requires_headers_and_libraries() {
        let root = Path::new("fixture");
        let mut dirs = installed(root, "x64-linux");
        dirs.remove(&root.join("installed/x64-linux/include"));
        assert!(
            probe_vcpkg_with(
                root,
                Arch::X64,
                None,
                false,
                |path| dirs.contains(path),
                |_, _, _| false
            )
            .is_none()
        );
    }

    #[test]
    fn runtime_requires_native_shared_binary_evidence() {
        for (name, windows, expected) in [
            ("native.dll", true, true),
            ("native.DLL", true, true),
            ("libnative.so", false, true),
            ("libnative.so.1.2", false, true),
            ("libnative.a", false, false),
            ("native.lib", true, false),
            ("native.dll", false, false),
            ("libnative.so", true, false),
            ("libnative.so.bad", false, false),
        ] {
            let path = Path::new(name);
            assert_eq!(
                vcpkg_runtime_candidate(path, Arch::X64, windows, |_| Some(Arch::X64)),
                expected,
                "{name}"
            );
            assert!(!vcpkg_runtime_candidate(
                path,
                Arch::X64,
                windows,
                |_| Some(Arch::Arm64)
            ));
            assert!(!vcpkg_runtime_candidate(path, Arch::X64, windows, |_| None));
        }
        let root = Path::new("fixture");
        let dirs = installed(root, "arm64-linux-dynamic");
        let info = probe_vcpkg_with(
            root,
            Arch::Arm64,
            None,
            false,
            |path| dirs.contains(path),
            |path, target, windows| path.ends_with("lib") && target == Arch::Arm64 && !windows,
        )
        .expect("native shared library evidence");
        assert_eq!(info.runtime_dir.as_ref(), Some(&info.lib));
    }

    fn raw_root() -> PathBuf {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            PathBuf::from(OsString::from_vec(b"raw-\xff-root".to_vec()))
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStringExt;
            PathBuf::from(OsString::from_wide(&[b'r' as u16, 0xd800, b't' as u16]))
        }
        #[cfg(not(any(unix, windows)))]
        PathBuf::from("raw-root")
    }

    #[test]
    fn non_unicode_triplet_is_invalid() {
        let value = raw_root().into_os_string();
        assert!(vcpkg_triplet_candidates(Arch::X64, Some(&value), true).is_empty());
    }

    fn vcpkg_fixture(target: Arch) -> VcpkgInfo {
        VcpkgInfo {
            target,
            root: raw_root(),
            triplet: format!(
                "{}-{}",
                target.as_str(),
                if cfg!(windows) { "windows" } else { "linux" }
            ),
            include: PathBuf::from("."),
            lib: PathBuf::from("."),
            runtime_dir: Some(PathBuf::from(".")),
        }
    }

    #[test]
    fn vcpkg_mismatch_does_not_mutate() {
        let mut env = Env::new(Arch::X64);
        let before = env.clone();
        let mut sdk = vcpkg_fixture(Arch::Arm64);
        sdk.triplet = "invalid".into();
        assert!(matches!(
            add_vcpkg(&mut env, &sdk),
            Err(EnvError::TargetMismatch { .. })
        ));
        assert_eq!(env, before);
    }

    #[test]
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    fn vcpkg_add_deduplicates_and_preserves_raw_root() {
        let triplets: &[&str] = if cfg!(windows) {
            &["x64-windows", "x64-windows-static-md", "x64-windows-static"]
        } else {
            &["x64-linux", "x64-linux-dynamic"]
        };
        for triplet in triplets {
            let mut sdk = vcpkg_fixture(Arch::X64);
            sdk.triplet = (*triplet).into();
            let mut env = Env::new(Arch::X64);
            env.vars
                .insert("VCPKG_DEFAULT_TRIPLET".into(), "stale-triplet".into());
            add_vcpkg(&mut env, &sdk).expect("first add");
            add_vcpkg(&mut env, &sdk).expect("second add");
            assert_eq!(env.include, [PathBuf::from(".")]);
            assert_eq!(env.lib, [PathBuf::from(".")]);
            assert_eq!(env.runtime, [PathBuf::from(".")]);
            assert!(env.path.is_empty());
            assert_eq!(env.vars["VCPKG_ROOT"], sdk.root.as_os_str());
            assert_eq!(env.vars["VCPKG_DEFAULT_TRIPLET"], OsStr::new(*triplet));
            assert_eq!(env.libpath.len(), usize::from(cfg!(windows)));
        }
    }

    #[test]
    fn vcpkg_invalid_triplet_does_not_mutate() {
        let mut env = Env::new(Arch::X64);
        env.include.push(PathBuf::from("already-present"));
        env.vars
            .insert("VCPKG_DEFAULT_TRIPLET".into(), "prior-triplet".into());
        let before = env.clone();
        let mut sdk = vcpkg_fixture(Arch::X64);
        sdk.triplet = "invalid".into();
        assert!(matches!(
            add_vcpkg(&mut env, &sdk),
            Err(EnvError::UnsupportedVcpkgTriplet { .. })
        ));
        assert_eq!(env, before);
    }

    #[test]
    #[cfg(feature = "nvsdk")]
    fn nvsdk_mismatch_does_not_mutate() {
        let mut env = Env::new(Arch::X64);
        let before = env.clone();
        let sdk = NvSdkInfo {
            kind: crate::nvsdk::Kind::Cudnn,
            target: Arch::Arm64,
            root: raw_root(),
            include_dir: PathBuf::from("."),
            lib_dir: None,
            runtime_dir: None,
            version: "1".into(),
        };
        assert!(matches!(
            add_nvsdk(&mut env, &sdk),
            Err(EnvError::TargetMismatch { .. })
        ));
        assert_eq!(env, before);
    }

    #[test]
    #[cfg(all(feature = "nvsdk", any(target_os = "windows", target_os = "linux")))]
    fn nvsdk_header_only_and_runtime_only_stay_distinct() {
        let sdk = NvSdkInfo {
            kind: crate::nvsdk::Kind::Cudnn,
            target: Arch::X64,
            root: raw_root(),
            include_dir: PathBuf::from("."),
            lib_dir: None,
            runtime_dir: None,
            version: "1".into(),
        };
        let mut env = Env::new(Arch::X64);
        add_nvsdk(&mut env, &sdk).expect("header-only");
        assert!(env.lib.is_empty() && env.runtime.is_empty() && env.path.is_empty());
        let mut runtime_sdk = sdk.clone();
        runtime_sdk.runtime_dir = Some(PathBuf::from("."));
        add_nvsdk(&mut env, &runtime_sdk).expect("runtime-only");
        add_nvsdk(&mut env, &runtime_sdk).expect("duplicate");
        assert_eq!(env.include.len(), 1);
        assert_eq!(env.runtime.len(), 1);
        assert!(env.lib.is_empty() && env.path.is_empty());
        for alias in sdk.kind.root_vars() {
            assert_eq!(env.vars[*alias], sdk.root.as_os_str());
        }
    }

    #[test]
    #[cfg(feature = "optix")]
    fn optix_sets_every_alias_without_loss() {
        let sdk = OptixInfo {
            root: raw_root(),
            version: crate::optix::OptixVersion {
                major: 1,
                minor: 0,
                micro: 0,
            },
            abi: 1,
            device_impl: false,
        };
        let mut env = Env::new(Arch::X64);
        add_optix(&mut env, &sdk);
        for alias in crate::optix::root_vars() {
            assert_eq!(env.vars[*alias], sdk.root.as_os_str());
        }
    }

    #[test]
    #[cfg(all(feature = "cuda", any(target_os = "windows", target_os = "linux")))]
    fn cuda_runtime_requires_matching_shared_library() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
        struct Fixture(PathBuf);
        impl Drop for Fixture {
            fn drop(&mut self) {
                // Test cleanup must not replace an assertion failure during unwinding.
                if let Err(error) = std::fs::remove_dir_all(&self.0) {
                    eprintln!(
                        "could not remove CUDA fixture {}: {error}",
                        self.0.display()
                    );
                }
            }
        }

        let root = std::env::temp_dir().join(format!(
            "vcv-rs-env-cuda-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).expect("create unique fixture");
        let fixture = Fixture(root);
        let lib = if cfg!(windows) {
            fixture.0.join("lib").join("x64")
        } else {
            fixture.0.join("targets/x86_64-linux/lib")
        };
        let runtime = if cfg!(windows) {
            fixture.0.join("bin").join("x64")
        } else {
            lib.clone()
        };
        let tools = fixture.0.join("bin");
        std::fs::create_dir_all(&tools).expect("create executable directory");
        std::fs::create_dir_all(&lib).expect("create link directory");
        std::fs::create_dir_all(&runtime).expect("create runtime directory");
        std::fs::write(
            lib.join(if cfg!(windows) {
                "cudart.lib"
            } else {
                "libcudart_static.a"
            }),
            crate::discovery::test_archive(
                if cfg!(windows) {
                    object::BinaryFormat::Coff
                } else {
                    object::BinaryFormat::Elf
                },
                Arch::X64,
            ),
        )
        .expect("write native link archive");
        let sdk = CudaInfo {
            root: fixture.0.clone(),
            version: crate::cuda::CudaVersion {
                major: 13,
                minor: 0,
            },
            nvcc: fixture.0.join("bin/nvcc"),
            msvc: None,
        };
        let mut env = Env::new(Arch::X64);
        assert_eq!(sdk.lib_dir(Arch::X64), Some(lib.clone()));
        assert_eq!(sdk.runtime_dir(Arch::X64), None);
        add_cuda(&mut env, &sdk).expect("static-only toolkit");
        assert_eq!(env.lib, std::slice::from_ref(&lib));
        assert!(env.runtime.is_empty());
        assert_eq!(env.path, std::slice::from_ref(&tools));
        let output: serde_json::Value =
            serde_json::from_str(&crate::format::fmt_json(&env).expect("static toolkit output"))
                .expect("parse static toolkit output");
        assert_eq!(
            output["PATH"],
            serde_json::json!([tools.to_str().expect("Unicode fixture path")])
        );

        let runtime_file = runtime.join(if cfg!(windows) {
            "cudart64_13.dll"
        } else {
            "libcudart.so.13"
        });
        let mut header = if cfg!(windows) {
            let mut bytes = vec![0; 70];
            bytes[..2].copy_from_slice(b"MZ");
            bytes[0x3c..0x40].copy_from_slice(&64u32.to_le_bytes());
            bytes[64..68].copy_from_slice(b"PE\0\0");
            bytes[68..70].copy_from_slice(&0xaa64u16.to_le_bytes());
            bytes
        } else {
            let mut bytes = vec![0; 20];
            bytes[..4].copy_from_slice(b"\x7fELF");
            bytes[4..7].copy_from_slice(&[2, 1, 1]);
            bytes[18..20].copy_from_slice(&183u16.to_le_bytes());
            bytes
        };
        std::fs::write(&runtime_file, &header).expect("write wrong-architecture runtime metadata");
        assert_eq!(sdk.runtime_dir(Arch::X64), None);
        add_cuda(&mut env, &sdk).expect("ignore wrong-architecture runtime");
        assert!(env.runtime.is_empty());
        assert_eq!(env.path, std::slice::from_ref(&tools));
        let output: serde_json::Value =
            serde_json::from_str(&crate::format::fmt_json(&env).expect("foreign runtime output"))
                .expect("parse foreign runtime output");
        assert_eq!(
            output["PATH"],
            serde_json::json!([tools.to_str().expect("Unicode fixture path")])
        );

        if cfg!(windows) {
            header[68..70].copy_from_slice(&0x8664u16.to_le_bytes());
        } else {
            header[18..20].copy_from_slice(&62u16.to_le_bytes());
        }
        std::fs::write(&runtime_file, &header).expect("write matching runtime metadata");
        assert_eq!(sdk.runtime_dir(Arch::X64), Some(runtime.clone()));
        add_cuda(&mut env, &sdk).expect("matching runtime");
        add_cuda(&mut env, &sdk).expect("duplicate runtime");
        assert_eq!(env.runtime, std::slice::from_ref(&runtime));
        assert_eq!(env.lib, [lib]);
        assert_eq!(env.path, std::slice::from_ref(&tools));
        let output: serde_json::Value =
            serde_json::from_str(&crate::format::fmt_json(&env).expect("verified runtime output"))
                .expect("parse verified runtime output");
        #[cfg(windows)]
        assert_eq!(
            output["PATH"],
            serde_json::json!([
                tools.to_str().expect("Unicode tools path"),
                runtime.to_str().expect("Unicode runtime path"),
            ])
        );
        #[cfg(target_os = "linux")]
        {
            assert_eq!(
                output["PATH"],
                serde_json::json!([tools.to_str().expect("Unicode tools path")])
            );
            assert_eq!(
                output["LD_LIBRARY_PATH"],
                serde_json::json!([runtime.to_str().expect("Unicode runtime path")])
            );
        }
        for alias in crate::cuda::root_vars() {
            assert_eq!(env.vars[*alias], sdk.root.as_os_str());
        }
        assert_eq!(env.vars[&sdk.versioned_var()], sdk.root.as_os_str());
    }

    #[test]
    #[cfg(feature = "cuda")]
    fn cuda_missing_target_library_does_not_mutate() {
        let sdk = CudaInfo {
            root: PathBuf::from("vcv-rs-missing-cuda-fixture"),
            version: crate::cuda::CudaVersion { major: 1, minor: 0 },
            nvcc: PathBuf::from("vcv-rs-missing-cuda-fixture/bin/nvcc"),
            msvc: None,
        };
        let mut env = Env::new(Arch::Arm64);
        let before = env.clone();
        assert!(matches!(
            add_cuda(&mut env, &sdk),
            Err(EnvError::MissingCudaLibrary { .. })
        ));
        assert_eq!(env, before);
    }
}
