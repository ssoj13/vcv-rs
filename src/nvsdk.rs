//! # NVIDIA SDK Detection (beyond the CUDA Toolkit and OptiX)
//!
//! Finds the separately shipped NVIDIA SDKs a build binds against — cuDNN, the Video Codec SDK,
//! the Optical Flow SDK, NVAPI, the NGX (DLSS) SDK and nvCOMP — and reports each one's root,
//! header directory, runtime-library directory and version.
//!
//! ## Why this lives in vcv-rs
//! Most of these SDKs are zips with no installer, so every consumer used to need its own
//! environment variable (`VIDEO_CODEC_SDK_ROOT`, `NVAPI_ROOT`, ...). One table here replaces that,
//! and the `vcv-rs` shell environment and a build script can never pick different copies.
//!
//! ## How a root is found, in order
//! 1. the SDK's own environment variables ([`Kind::root_vars`]) — an explicit choice wins;
//! 2. the installers' standard parents: `%ProgramFiles%\NVIDIA` and
//!    `%ProgramData%\NVIDIA Corporation`;
//! 3. every directory listed in `NV_SDK_DIRS` (path-list syntax), for SDKs unpacked anywhere —
//!    e.g. a folder of git clones and zips.
//!
//! Parents are searched at most [`SEARCH_DEPTH`] levels deep for a directory that *is* the SDK
//! (its marker header exists). Nothing is version-shaped: the version is read from the SDK's own
//! header ([`Kind::version_define`]).

use std::path::{Path, PathBuf};

/// How deep below a search parent an SDK root may sit (`<parent>/CUDNN/v9.23` is depth 2,
/// `<parent>/Streamline/external/ngx-sdk` is depth 3).
pub const SEARCH_DEPTH: usize = 3;

/// The SDKs this module knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// cuDNN. Windows installs put headers under `include/<cuda version>/`; the folder matching
    /// the installed CUDA Toolkit is chosen.
    Cudnn,
    /// Video Codec SDK (NVENC/NVDEC headers in `Interface/`).
    VideoCodec,
    /// Optical Flow SDK (headers in `NvOFInterface/`).
    OpticalFlow,
    /// NVAPI (headers at the root).
    Nvapi,
    /// NGX / DLSS SDK (headers in `include/`, static libraries in `lib/`).
    Ngx,
    /// nvCOMP.
    Nvcomp,
}

impl Kind {
    /// Every kind, in display order.
    pub const ALL: [Kind; 6] = [Kind::Cudnn, Kind::VideoCodec, Kind::OpticalFlow, Kind::Nvapi, Kind::Ngx, Kind::Nvcomp];

    /// Human name.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Cudnn => "cuDNN",
            Kind::VideoCodec => "Video Codec SDK",
            Kind::OpticalFlow => "Optical Flow SDK",
            Kind::Nvapi => "NVAPI",
            Kind::Ngx => "NGX (DLSS) SDK",
            Kind::Nvcomp => "nvCOMP",
        }
    }

    /// Environment variables naming a root, in the order they are honoured. The first is also the
    /// variable [`crate::env::add_nvsdk`] sets.
    pub fn root_vars(self) -> &'static [&'static str] {
        match self {
            Kind::Cudnn => &["CUDNN_ROOT", "CUDNN_PATH", "CUDNN_HOME"],
            Kind::VideoCodec => &["VIDEO_CODEC_SDK_ROOT", "NVIDIA_VIDEO_CODEC_SDK"],
            Kind::OpticalFlow => &["OPTICAL_FLOW_SDK_ROOT"],
            Kind::Nvapi => &["NVAPI_ROOT", "NVAPI_PATH"],
            Kind::Ngx => &["NGX_SDK_ROOT", "DLSS_ROOT"],
            Kind::Nvcomp => &["NVCOMP_ROOT", "NVCOMP_PATH"],
        }
    }

    /// The header (relative to the header directory) whose presence makes a directory this SDK.
    pub fn marker(self) -> &'static str {
        match self {
            Kind::Cudnn => "cudnn_version.h",
            Kind::VideoCodec => "nvEncodeAPI.h",
            Kind::OpticalFlow => "nvOpticalFlowCommon.h",
            Kind::Nvapi => "nvapi_interface.h",
            Kind::Ngx => "nvsdk_ngx.h",
            Kind::Nvcomp => "nvcomp.h",
        }
    }

    /// The `#define`s that make up the version, read from the marker's directory.
    pub fn version_define(self) -> &'static [(&'static str, &'static str)] {
        // (header, define)
        match self {
            Kind::Cudnn => &[("cudnn_version.h", "CUDNN_MAJOR"), ("cudnn_version.h", "CUDNN_MINOR"), ("cudnn_version.h", "CUDNN_PATCHLEVEL")],
            Kind::VideoCodec => &[("nvEncodeAPI.h", "NVENCAPI_MAJOR_VERSION"), ("nvEncodeAPI.h", "NVENCAPI_MINOR_VERSION")],
            Kind::OpticalFlow => &[("nvOpticalFlowCommon.h", "NV_OF_API_MAJOR_VERSION"), ("nvOpticalFlowCommon.h", "NV_OF_API_MINOR_VERSION")],
            Kind::Nvapi => &[("nvapi_lite_common.h", "NVAPI_SDK_VERSION")],
            Kind::Ngx => &[("nvsdk_ngx_defs.h", "NVSDK_NGX_VERSION_API_MACRO")],
            Kind::Nvcomp => &[("nvcomp/version.h", "NVCOMP_VER_MAJOR"), ("nvcomp/version.h", "NVCOMP_VER_MINOR"), ("nvcomp/version.h", "NVCOMP_VER_PATCH")],
        }
    }

    /// Header-directory candidates relative to a root, most specific first.
    fn include_subdirs(self) -> &'static [&'static str] {
        match self {
            Kind::Cudnn | Kind::Ngx | Kind::Nvcomp => &["include"],
            Kind::VideoCodec => &["Interface"],
            Kind::OpticalFlow => &["NvOFInterface"],
            Kind::Nvapi => &[""],
        }
    }
}

/// A located SDK.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkInfo {
    /// Which SDK.
    pub kind: Kind,
    /// The SDK root.
    pub root: PathBuf,
    /// The directory holding the marker header (what goes on the include path).
    pub include_dir: PathBuf,
    /// Runtime libraries (DLLs) shipped with the SDK, if it ships any (`None` for header-only
    /// SDKs whose runtime is the driver: Video Codec, Optical Flow, NVAPI).
    pub bin_dir: Option<PathBuf>,
    /// Link libraries shipped with the SDK (`lib/...`), if any.
    pub lib_dir: Option<PathBuf>,
    /// Version as the SDK's headers state it (`9.23.0`, `13.1`, `62115`).
    pub version: String,
}

/// Is `root` an installation of `kind`?
pub fn probe(kind: Kind, root: &Path) -> Option<SdkInfo> {
    let include_dir = include_dir(kind, root)?;
    let version = read_version(kind, &include_dir)?;
    let (bin_dir, lib_dir) = runtime_dirs(kind, root, &include_dir);
    Some(SdkInfo { kind, root: root.to_path_buf(), include_dir, bin_dir, lib_dir, version })
}

fn include_dir(kind: Kind, root: &Path) -> Option<PathBuf> {
    for sub in kind.include_subdirs() {
        let dir = if sub.is_empty() { root.to_path_buf() } else { root.join(sub) };
        if dir.join(kind.marker()).is_file() {
            return Some(dir);
        }
        // cuDNN on Windows: include/<cuda version>/cudnn_version.h, one folder per CUDA release.
        if kind == Kind::Cudnn
            && let Some(versioned) = pick_cuda_versioned(&dir, kind.marker())
        {
            return Some(versioned);
        }
    }
    None
}

/// Among `dir/<major.minor>/` folders holding `marker`, the one matching the installed CUDA
/// Toolkit, else the newest.
fn pick_cuda_versioned(dir: &Path, marker: &str) -> Option<PathBuf> {
    let mut found: Vec<(Vec<u32>, PathBuf)> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.join(marker).is_file())
        .filter_map(|p| {
            let v = p.file_name()?.to_str()?.split('.').map(|s| s.parse::<u32>().ok()).collect::<Option<Vec<_>>>()?;
            Some((v, p))
        })
        .collect();
    found.sort();
    #[cfg(feature = "cuda")]
    if let Some(cuda) = crate::cuda::detect_cuda()
        && let Some((_, p)) = found.iter().find(|(v, _)| v.as_slice() == [cuda.version.major, cuda.version.minor])
    {
        return Some(p.clone());
    }
    found.pop().map(|(_, p)| p)
}

fn runtime_dirs(kind: Kind, root: &Path, include_dir: &Path) -> (Option<PathBuf>, Option<PathBuf>) {
    let existing = |p: PathBuf| p.is_dir().then_some(p);
    match kind {
        Kind::Cudnn => {
            // Versioned layout mirrors include/<cuda>/: bin/<cuda>/x64, lib/<cuda>/x64.
            let cuda = include_dir.file_name().filter(|_| include_dir.parent().is_some_and(|p| p.ends_with("include")));
            match cuda {
                Some(v) => (
                    existing(root.join("bin").join(v).join("x64")).or_else(|| existing(root.join("bin").join(v))),
                    existing(root.join("lib").join(v).join("x64")).or_else(|| existing(root.join("lib").join(v))),
                ),
                None => (existing(root.join("bin")), existing(root.join("lib")).or_else(|| existing(root.join("lib").join("x64")))),
            }
        }
        Kind::Nvcomp => (existing(root.join("bin")), existing(root.join("lib"))),
        Kind::Ngx => (None, existing(root.join("lib").join("Windows_x86_64")).or_else(|| existing(root.join("lib")))),
        Kind::Nvapi => (None, existing(root.join("amd64"))),
        Kind::VideoCodec => (None, existing(root.join("Lib").join("x64"))),
        Kind::OpticalFlow => (None, None),
    }
}

fn read_version(kind: Kind, include_dir: &Path) -> Option<String> {
    let parts: Vec<String> = kind
        .version_define()
        .iter()
        .map(|(header, define)| read_define(&include_dir.join(header), define))
        .collect::<Option<Vec<_>>>()?;
    Some(parts.join("."))
}

/// `#define NAME <value>` -> value (first token, `0x15` kept as written).
fn read_define(header: &Path, name: &str) -> Option<String> {
    let text = std::fs::read_to_string(header).ok()?;
    text.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("#define")?.trim_start().strip_prefix(name)?;
        if !rest.starts_with(char::is_whitespace) {
            return None;
        }
        rest.split_whitespace().next().map(str::to_string)
    })
}

/// Search parents: the installers' standard locations plus `NV_SDK_DIRS`.
pub fn search_parents() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(pf) = std::env::var("ProgramFiles") {
        out.push(PathBuf::from(pf).join("NVIDIA"));
    }
    if let Ok(pd) = std::env::var("ProgramData") {
        out.push(PathBuf::from(pd).join("NVIDIA Corporation"));
    }
    if let Some(dirs) = std::env::var_os("NV_SDK_DIRS") {
        out.extend(std::env::split_paths(&dirs));
    }
    out
}

/// Every installation of `kind`, explicitly named ones first, then by version (newest first).
pub fn list(kind: Kind) -> Vec<SdkInfo> {
    list_with(kind, &search_parents())
}

/// [`list`] over explicit search parents (the root variables are still honoured first).
pub fn list_with(kind: Kind, parents: &[PathBuf]) -> Vec<SdkInfo> {
    let mut named: Vec<SdkInfo> = kind
        .root_vars()
        .iter()
        .filter_map(|v| std::env::var_os(v).filter(|s| !s.is_empty()))
        .filter_map(|root| probe(kind, Path::new(&root)))
        .collect();
    let mut searched: Vec<SdkInfo> = Vec::new();
    for parent in parents {
        walk(parent, SEARCH_DEPTH, &mut |dir| {
            if let Some(info) = probe(kind, dir)
                && !named.iter().chain(searched.iter()).any(|i| i.root == info.root)
            {
                searched.push(info);
                return true; // an SDK root is not searched further down
            }
            false
        });
    }
    // An install built for the CUDA Toolkit on this machine outranks a newer one built for
    // another toolkit (cuDNN v9.28 for CUDA 13.5 is unusable next to CUDA 13.3), then newest first.
    searched.sort_by_key(|s| (std::cmp::Reverse(matches_installed_cuda(s)), std::cmp::Reverse(version_key(&s.version))));
    named.dedup_by(|a, b| a.root == b.root);
    named.extend(searched);
    named
}

/// The preferred installation of `kind`.
pub fn detect(kind: Kind) -> Option<SdkInfo> {
    list(kind).into_iter().next()
}

/// For cuDNN's versioned layout: is the chosen `include/<cuda>` folder the installed toolkit's?
/// Always true for SDKs not tied to a CUDA version.
fn matches_installed_cuda(s: &SdkInfo) -> bool {
    if s.kind != Kind::Cudnn {
        return true;
    }
    #[cfg(feature = "cuda")]
    if let (Some(cuda), Some(folder)) = (crate::cuda::detect_cuda(), s.include_dir.file_name().and_then(|n| n.to_str())) {
        return folder == format!("{}.{}", cuda.version.major, cuda.version.minor);
    }
    false
}

fn version_key(v: &str) -> Vec<u64> {
    v.split('.').map(|p| u64::from_str_radix(p.trim_start_matches("0x"), if p.starts_with("0x") { 16 } else { 10 }).unwrap_or(0)).collect()
}

/// Visits `dir` and its subdirectories down to `depth`; `visit` returns true to stop descending.
fn walk(dir: &Path, depth: usize, visit: &mut dyn FnMut(&Path) -> bool) {
    if visit(dir) || depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut subdirs: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.is_dir()).collect();
    subdirs.sort();
    for sub in subdirs {
        // Never descend into VCS metadata or build output.
        if sub.file_name().is_some_and(|n| n == ".git" || n == "target" || n == "node_modules") {
            continue;
        }
        walk(&sub, depth - 1, visit);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
        std::fs::write(path, text).expect("write");
    }

    #[test]
    fn cudnn_versioned_layout_is_found_under_a_parent() {
        let base = std::env::temp_dir().join("vcv_rs_nvsdk_cudnn");
        std::fs::remove_dir_all(&base).ok();
        let root = base.join("CUDNN").join("v9.23");
        write(&root.join("include").join("13.3").join("cudnn_version.h"), "#define CUDNN_MAJOR 9\n#define CUDNN_MINOR 23\n#define CUDNN_PATCHLEVEL 0\n");
        std::fs::create_dir_all(root.join("bin").join("13.3").join("x64")).expect("bin");
        let found = list_with(Kind::Cudnn, std::slice::from_ref(&base));
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].version, "9.23.0");
        assert_eq!(found[0].include_dir, root.join("include").join("13.3"));
        assert_eq!(found[0].bin_dir, Some(root.join("bin").join("13.3").join("x64")));
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn header_only_sdks_report_their_header_directory_and_version() {
        let base = std::env::temp_dir().join("vcv_rs_nvsdk_misc");
        std::fs::remove_dir_all(&base).ok();
        write(&base.join("Video_Codec_SDK_13.1.15").join("Interface").join("nvEncodeAPI.h"), "#define NVENCAPI_MAJOR_VERSION 13\n#define NVENCAPI_MINOR_VERSION 1\n");
        write(&base.join("nvapi").join("nvapi_interface.h"), "");
        write(&base.join("nvapi").join("nvapi_lite_common.h"), "#define NVAPI_SDK_VERSION 62115\n");
        write(&base.join("Streamline").join("external").join("ngx-sdk").join("include").join("nvsdk_ngx.h"), "");
        write(
            &base.join("Streamline").join("external").join("ngx-sdk").join("include").join("nvsdk_ngx_defs.h"),
            "#define NVSDK_NGX_VERSION_API_MACRO 0x0000015  // 1.5.0\n",
        );
        let parents = [base.clone()];
        assert_eq!(list_with(Kind::VideoCodec, &parents)[0].version, "13.1");
        assert_eq!(list_with(Kind::Nvapi, &parents)[0].version, "62115");
        let ngx = &list_with(Kind::Ngx, &parents)[0];
        assert_eq!(ngx.version, "0x0000015");
        assert!(ngx.root.ends_with("ngx-sdk"), "depth-3 SDK found: {:?}", ngx.root);
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn a_define_prefix_does_not_match_a_longer_name() {
        let base = std::env::temp_dir().join("vcv_rs_nvsdk_define");
        std::fs::remove_dir_all(&base).ok();
        write(&base.join("x.h"), "#define NVAPI_SDK_VERSION_EXTRA 1\n#define NVAPI_SDK_VERSION 7\n");
        assert_eq!(read_define(&base.join("x.h"), "NVAPI_SDK_VERSION").as_deref(), Some("7"));
        std::fs::remove_dir_all(&base).ok();
    }
}
