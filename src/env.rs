//! # Environment Builder Module
//!
//! Assembles PATH, INCLUDE, LIB, LIBPATH from detected VS/SDK/UCRT paths.
//!
//! ## Purpose
//! Builds the complete environment needed for MSVC compilation:
//! - PATH: compiler binaries (cl.exe, link.exe) and SDK tools
//! - INCLUDE: headers (VC++, ATL/MFC, SDK, UCRT)
//! - LIB: static libraries for linking
//! - LIBPATH: assembly references (.NET metadata)
//!
//! ## Key Functions
//! - `build_env()` - Main function that assembles all paths based on host/target arch
//! - `add_cuda()` - Appends a CUDA Toolkit to an already-built environment (`cuda` feature)
//! - `add_vcpkg()` - Appends `VCPKG_ROOT` installed libs (openssl etc.) for MSVC link
//!
//! ## Dependencies
//! - `detect` module for VsInfo/SdkInfo structs
//! - `std::collections::BTreeMap` for stable key ordering

use crate::Arch;
#[cfg(feature = "cuda")]
use crate::cuda::CudaInfo;
use crate::detect::{SdkInfo, VsInfo};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Assembled environment
#[derive(Debug, Default)]
pub struct Env {
    pub path: Vec<PathBuf>,
    pub include: Vec<PathBuf>,
    pub lib: Vec<PathBuf>,
    pub libpath: Vec<PathBuf>,
    pub vars: BTreeMap<String, String>,
}

impl Env {
    fn add_if_exists(lst: &mut Vec<PathBuf>, paths: &[PathBuf]) {
        for p in paths {
            if p.exists() {
                lst.push(p.clone());
            }
        }
    }
}

/// Build complete environment
pub fn build_env(
    vs: &VsInfo,
    sdk: Option<&SdkInfo>,
    ucrt: Option<&SdkInfo>,
    host: Arch,
    target: Arch,
) -> Env {
    let mut env = Env::default();
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
    env.vars
        .insert("VSINSTALLDIR".into(), format!("{}\\", vs.install.display()));
    env.vars
        .insert("VCINSTALLDIR".into(), format!("{}\\", vs.vc.display()));
    env.vars
        .insert("VCToolsInstallDir".into(), format!("{}\\", tp.display()));
    env.vars
        .insert("VCToolsVersion".into(), vs.tools_ver.clone());
    env.vars.insert("VisualStudioVersion".into(), "17.0".into());
    env.vars.insert("Platform".into(), tgt.into());

    if let Some(sdk) = sdk {
        env.vars
            .insert("WindowsSdkDir".into(), format!("{}\\", sdk.path.display()));
        env.vars
            .insert("WindowsSDKVersion".into(), format!("{}\\", sdk.version));
    }

    if let Some(ucrt) = ucrt {
        env.vars.insert(
            "UniversalCRTSdkDir".into(),
            format!("{}\\", ucrt.path.display()),
        );
        env.vars.insert("UCRTVersion".into(), ucrt.version.clone());
    }

    env
}

/// Append a CUDA Toolkit to an environment built by [`build_env`].
///
/// Additive by construction — it only pushes onto the existing lists, so the MSVC environment is
/// exactly what it was and CUDA cannot shadow a compiler path. That is also why it is a separate
/// function rather than a parameter of `build_env`: a caller that does not want CUDA gets the old
/// behaviour byte for byte, without a flag threaded through the assembly.
///
/// **All three root variables are written to the same value on purpose.** `CUDA_PATH` is what
/// NVIDIA's installer sets, `CUDA_HOME` is what most build scripts read (`cudarc`'s among them),
/// and the versioned `CUDA_PATH_V13_2` is how tools select between side-by-side toolkits. Leaving
/// any of them pointing elsewhere is how a build compiles against one toolkit and links another.
#[cfg(feature = "cuda")]
pub fn add_cuda(env: &mut Env, cuda: &CudaInfo, target: Arch) {
    env.path.extend(cuda.bin_dirs());
    Env::add_if_exists(&mut env.include, &[cuda.include_dir()]);
    if let Some(lib) = cuda.lib_dir(target) {
        env.lib.push(lib);
    }

    let root = cuda.root.display().to_string();
    env.vars.insert("CUDA_PATH".into(), root.clone());
    env.vars.insert("CUDA_HOME".into(), root.clone());
    env.vars.insert(cuda.versioned_var(), root);
}

/// Installed vcpkg tree resolved from `VCPKG_ROOT`.
///
/// `triplet` is the first `installed/<triplet>/lib` that exists, so a machine with only
/// `x64-windows-static-md` still works. `VCPKG_DEFAULT_TRIPLET` wins when that directory exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VcpkgInfo {
    pub root: PathBuf,
    pub triplet: String,
    pub lib: PathBuf,
    pub include: PathBuf,
    pub bin: Option<PathBuf>,
}

/// `VCPKG_ROOT` when it names an existing directory.
pub fn detect_vcpkg() -> Option<PathBuf> {
    let root = PathBuf::from(std::env::var_os("VCPKG_ROOT")?);
    root.is_dir().then_some(root)
}

fn vcpkg_triplet_candidates(target: Arch) -> Vec<String> {
    let arch = target.as_str();
    let mut out = Vec::new();
    if let Ok(explicit) = std::env::var("VCPKG_DEFAULT_TRIPLET") {
        let t = explicit.trim();
        if !t.is_empty() {
            out.push(t.to_string());
        }
    }
    for suffix in ["windows", "windows-static-md", "windows-static"] {
        let t = format!("{arch}-{suffix}");
        if !out.iter().any(|e| e == &t) {
            out.push(t);
        }
    }
    out
}

/// Locate `installed/<triplet>/{lib,include}` under a vcpkg root.
pub fn probe_vcpkg(root: &Path, target: Arch) -> Option<VcpkgInfo> {
    for triplet in vcpkg_triplet_candidates(target) {
        let installed = root.join("installed").join(&triplet);
        let lib = installed.join("lib");
        if !lib.is_dir() {
            continue;
        }
        let include = installed.join("include");
        let bin = installed.join("bin");
        return Some(VcpkgInfo {
            root: root.to_path_buf(),
            triplet,
            lib,
            include,
            bin: bin.is_dir().then_some(bin),
        });
    }
    None
}

/// Append vcpkg installed libs to an environment built by [`build_env`].
///
/// Additive: MSVC/CUDA paths stay as they were. `LIB`/`LIBPATH` get `installed/<triplet>/lib`
/// so `link.exe` can open `libssl.lib` / `libcrypto.lib` without each crate repeating the probe.
/// `VCPKG_ROOT` is written so `vcpkg` crate build scripts see the same tree.
pub fn add_vcpkg(env: &mut Env, vcpkg: &VcpkgInfo) {
    Env::add_if_exists(&mut env.include, &[vcpkg.include.clone()]);
    env.lib.push(vcpkg.lib.clone());
    env.libpath.push(vcpkg.lib.clone());
    if let Some(bin) = &vcpkg.bin {
        env.path.push(bin.clone());
    }
    env.vars
        .insert("VCPKG_ROOT".into(), vcpkg.root.display().to_string());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_dir(p: &Path) {
        fs::create_dir_all(p).expect("mkdir");
    }

    #[test]
    fn probe_prefers_default_triplet_when_installed() {
        let root = std::env::temp_dir().join("vcv_rs_vcpkg_probe_test");
        let _ = fs::remove_dir_all(&root);
        write_dir(
            &root
                .join("installed")
                .join("x64-windows-static-md")
                .join("lib"),
        );
        write_dir(&root.join("installed").join("x64-windows").join("lib"));
        // Safety: test-local; restored below.
        let prev = std::env::var_os("VCPKG_DEFAULT_TRIPLET");
        unsafe { std::env::set_var("VCPKG_DEFAULT_TRIPLET", "x64-windows-static-md") };
        let info = probe_vcpkg(&root, Arch::X64).expect("probe");
        match prev {
            Some(v) => unsafe { std::env::set_var("VCPKG_DEFAULT_TRIPLET", v) },
            None => unsafe { std::env::remove_var("VCPKG_DEFAULT_TRIPLET") },
        }
        assert_eq!(info.triplet, "x64-windows-static-md");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn probe_falls_back_to_x64_windows() {
        let root = std::env::temp_dir().join("vcv_rs_vcpkg_fallback_test");
        let _ = fs::remove_dir_all(&root);
        write_dir(&root.join("installed").join("x64-windows").join("lib"));
        write_dir(&root.join("installed").join("x64-windows").join("include"));
        let prev = std::env::var_os("VCPKG_DEFAULT_TRIPLET");
        unsafe { std::env::remove_var("VCPKG_DEFAULT_TRIPLET") };
        let info = probe_vcpkg(&root, Arch::X64).expect("probe");
        match prev {
            Some(v) => unsafe { std::env::set_var("VCPKG_DEFAULT_TRIPLET", v) },
            None => unsafe { std::env::remove_var("VCPKG_DEFAULT_TRIPLET") },
        }
        assert_eq!(info.triplet, "x64-windows");
        assert!(info.bin.is_none());
        let _ = fs::remove_dir_all(&root);
    }
}
