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
//!
//! ## Dependencies
//! - `detect` module for VsInfo/SdkInfo structs
//! - `std::collections::BTreeMap` for stable key ordering

use crate::detect::{SdkInfo, VsInfo};
use crate::Arch;
use std::collections::BTreeMap;
use std::path::PathBuf;

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
pub fn build_env(vs: &VsInfo, sdk: Option<&SdkInfo>, ucrt: Option<&SdkInfo>, host: Arch, target: Arch) -> Env {
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
    Env::add_if_exists(&mut env.include, &[
        tp.join("include"),
        tp.join("ATLMFC").join("include"),
    ]);
    Env::add_if_exists(&mut env.lib, &[
        tp.join("lib").join(tgt),
        tp.join("ATLMFC").join("lib").join(tgt),
    ]);
    Env::add_if_exists(&mut env.libpath, &[
        tp.join("lib").join(tgt),
        tp.join("ATLMFC").join("lib").join(tgt),
    ]);

    // Windows SDK
    if let Some(sdk) = sdk {
        let sp = &sdk.path;
        let sv = &sdk.version;
        let host_str = host.as_str();

        Env::add_if_exists(&mut env.path, &[sp.join("bin").join(sv).join(host_str)]);
        // Note: ucrt include is added from UCRT section, not here
        Env::add_if_exists(&mut env.include, &[
            sp.join("include").join(sv).join("um"),
            sp.join("include").join(sv).join("shared"),
            sp.join("include").join(sv).join("winrt"),
            sp.join("include").join(sv).join("cppwinrt"),
        ]);
        Env::add_if_exists(&mut env.lib, &[sp.join("lib").join(sv).join("um").join(tgt)]);
        Env::add_if_exists(&mut env.libpath, &[
            sp.join("UnionMetadata").join(sv),
            sp.join("References").join(sv),
        ]);
    }

    // UCRT
    if let Some(ucrt) = ucrt {
        let up = &ucrt.path;
        let uv = &ucrt.version;

        Env::add_if_exists(&mut env.include, &[up.join("include").join(uv).join("ucrt")]);
        Env::add_if_exists(&mut env.lib, &[up.join("lib").join(uv).join("ucrt").join(tgt)]);
    }

    // Standard variables
    env.vars.insert("VSINSTALLDIR".into(), format!("{}\\", vs.install.display()));
    env.vars.insert("VCINSTALLDIR".into(), format!("{}\\", vs.vc.display()));
    env.vars.insert("VCToolsInstallDir".into(), format!("{}\\", tp.display()));
    env.vars.insert("VCToolsVersion".into(), vs.tools_ver.clone());
    env.vars.insert("VisualStudioVersion".into(), "17.0".into());
    env.vars.insert("Platform".into(), tgt.into());

    if let Some(sdk) = sdk {
        env.vars.insert("WindowsSdkDir".into(), format!("{}\\", sdk.path.display()));
        env.vars.insert("WindowsSDKVersion".into(), format!("{}\\", sdk.version));
    }

    if let Some(ucrt) = ucrt {
        env.vars.insert("UniversalCRTSdkDir".into(), format!("{}\\", ucrt.path.display()));
        env.vars.insert("UCRTVersion".into(), ucrt.version.clone());
    }

    env
}
