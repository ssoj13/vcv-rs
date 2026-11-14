//! Passive discovery of separately shipped NVIDIA SDK headers and native libraries.

use std::path::{Path, PathBuf};

use crate::Arch;

/// Maximum depth below an explicit SDK search parent.
pub const SEARCH_DEPTH: usize = 3;

/// The SDKs this module knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Cudnn,
    VideoCodec,
    OpticalFlow,
    Nvapi,
    Ngx,
    Nvcomp,
    Nccl,
}

impl Kind {
    /// Every SDK, in display order.
    pub const ALL: [Kind; 7] = [
        Self::Cudnn,
        Self::VideoCodec,
        Self::OpticalFlow,
        Self::Nvapi,
        Self::Ngx,
        Self::Nvcomp,
        Self::Nccl,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Cudnn => "cuDNN",
            Self::VideoCodec => "Video Codec SDK",
            Self::OpticalFlow => "Optical Flow SDK",
            Self::Nvapi => "NVAPI",
            Self::Ngx => "NGX (DLSS) SDK",
            Self::Nvcomp => "nvCOMP",
            Self::Nccl => "NCCL",
        }
    }

    /// Root aliases, in precedence order.
    pub fn root_vars(self) -> &'static [&'static str] {
        match self {
            Self::Cudnn => &["CUDNN_ROOT", "CUDNN_PATH", "CUDNN_HOME"],
            Self::VideoCodec => &["VIDEO_CODEC_SDK_ROOT", "NVIDIA_VIDEO_CODEC_SDK"],
            Self::OpticalFlow => &["OPTICAL_FLOW_SDK_ROOT"],
            Self::Nvapi => &["NVAPI_ROOT", "NVAPI_PATH"],
            Self::Ngx => &["NGX_SDK_ROOT", "DLSS_ROOT"],
            Self::Nvcomp => &["NVCOMP_ROOT", "NVCOMP_PATH"],
            Self::Nccl => &["NCCL_ROOT", "NCCL_PATH"],
        }
    }

    /// First nonempty configured alias, including an invalid requested path.
    pub fn requested_root(self) -> Option<crate::discovery::RootRequest> {
        crate::discovery::first_root(self.root_vars())
    }

    /// Whether this SDK's headers are supported on the compiled host platform.
    pub fn is_supported(self) -> bool {
        match self {
            Self::Nvapi => cfg!(target_os = "windows"),
            Self::Nccl => cfg!(target_os = "linux"),
            _ => cfg!(any(target_os = "windows", target_os = "linux")),
        }
    }

    pub fn marker(self) -> &'static str {
        match self {
            Self::Cudnn => "cudnn_version.h",
            Self::VideoCodec => "nvEncodeAPI.h",
            Self::OpticalFlow => "nvOpticalFlowCommon.h",
            Self::Nvapi => "nvapi_interface.h",
            Self::Ngx => "nvsdk_ngx.h",
            Self::Nvcomp => "nvcomp.h",
            Self::Nccl => "nccl.h",
        }
    }

    pub fn version_define(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::Cudnn => &[
                ("cudnn_version.h", "CUDNN_MAJOR"),
                ("cudnn_version.h", "CUDNN_MINOR"),
                ("cudnn_version.h", "CUDNN_PATCHLEVEL"),
            ],
            Self::VideoCodec => &[
                ("nvEncodeAPI.h", "NVENCAPI_MAJOR_VERSION"),
                ("nvEncodeAPI.h", "NVENCAPI_MINOR_VERSION"),
            ],
            Self::OpticalFlow => &[
                ("nvOpticalFlowCommon.h", "NV_OF_API_MAJOR_VERSION"),
                ("nvOpticalFlowCommon.h", "NV_OF_API_MINOR_VERSION"),
            ],
            Self::Nvapi => &[("nvapi_lite_common.h", "NVAPI_SDK_VERSION")],
            Self::Ngx => &[("nvsdk_ngx_defs.h", "NVSDK_NGX_VERSION_API_MACRO")],
            Self::Nvcomp => &[
                ("nvcomp/version.h", "NVCOMP_VER_MAJOR"),
                ("nvcomp/version.h", "NVCOMP_VER_MINOR"),
                ("nvcomp/version.h", "NVCOMP_VER_PATCH"),
            ],
            Self::Nccl => &[
                ("nccl.h", "NCCL_MAJOR"),
                ("nccl.h", "NCCL_MINOR"),
                ("nccl.h", "NCCL_PATCH"),
            ],
        }
    }

    fn include_subdirs(self) -> &'static [&'static str] {
        match self {
            Self::Cudnn | Self::Ngx | Self::Nvcomp | Self::Nccl => &["include"],
            Self::VideoCodec => &["Interface", "include"],
            Self::OpticalFlow => &["NvOFInterface", "include"],
            Self::Nvapi => &[""],
        }
    }
}

/// An SDK's headers and any verified native components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkInfo {
    pub kind: Kind,
    pub target: Arch,
    pub root: PathBuf,
    pub include_dir: PathBuf,
    pub runtime_dir: Option<PathBuf>,
    pub lib_dir: Option<PathBuf>,
    pub version: String,
}

/// Probe a root for headers and native components for the requested architecture.
pub fn probe(kind: Kind, root: &Path, target: Arch) -> Option<SdkInfo> {
    probe_with(kind, root, target, selected_cuda(target))
}

fn selected_cuda(_target: Arch) -> Option<(u32, u32)> {
    #[cfg(feature = "cuda")]
    {
        let cuda = crate::cuda::detect_cuda()?;
        Some((cuda.version.major, cuda.version.minor))
    }
    #[cfg(not(feature = "cuda"))]
    {
        None
    }
}

fn probe_with(kind: Kind, root: &Path, target: Arch, cuda: Option<(u32, u32)>) -> Option<SdkInfo> {
    if !kind.is_supported() {
        return None;
    }
    let include_dir = include_dir(kind, root, target, cuda)?;
    let version = read_version(kind, &include_dir)?;
    let (runtime_dir, lib_dir) = runtime_dirs(
        kind,
        root,
        &include_dir,
        target,
        cuda,
        cfg!(target_os = "windows"),
    );
    Some(SdkInfo {
        kind,
        target,
        root: root.to_path_buf(),
        include_dir,
        runtime_dir,
        lib_dir,
        version,
    })
}

fn include_dir(kind: Kind, root: &Path, target: Arch, cuda: Option<(u32, u32)>) -> Option<PathBuf> {
    for sub in kind.include_subdirs() {
        let dir = root.join(sub);
        if cfg!(target_os = "windows")
            && kind == Kind::Cudnn
            && let Some(versioned) = pick_cuda_versioned(&dir, kind.marker(), cuda)
        {
            return Some(versioned);
        }
        if dir.join(kind.marker()).is_file() {
            return Some(dir);
        }
        if cfg!(target_os = "linux") {
            for tag in native_tags(target, false) {
                let nested = dir.join(tag);
                if nested.join(kind.marker()).is_file() {
                    return Some(nested);
                }
            }
        }
    }
    None
}

fn pick_cuda_versioned(dir: &Path, marker: &str, cuda: Option<(u32, u32)>) -> Option<PathBuf> {
    let mut found: Vec<(Vec<u32>, PathBuf)> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.join(marker).is_file())
        .filter_map(|p| Some((version_key(p.file_name()?.to_str()?)?, p)))
        .filter(|(v, _)| v.len() == 2)
        .collect();
    found.sort();
    if let Some((major, minor)) = cuda
        && let Some((_, path)) = found.iter().find(|(v, _)| v.as_slice() == [major, minor])
    {
        return Some(path.clone());
    }
    found.pop().map(|(_, path)| path)
}

fn native_tags(target: Arch, windows: bool) -> &'static [&'static str] {
    match (windows, target) {
        (true, Arch::X64) => &["x64", "amd64", "Windows_x86_64"],
        (true, Arch::X86) => &["Win32", "x86", "Windows_x86"],
        (true, Arch::Arm64) => &["arm64", "aarch64", "Windows_aarch64", "Windows_arm64"],
        (false, Arch::X64) => &["x86_64-linux-gnu", "x86_64-linux", "Linux_x86_64"],
        (false, Arch::X86) => &["i386-linux-gnu", "i686-linux-gnu", "Linux_x86"],
        (false, Arch::Arm64) => &["aarch64-linux-gnu", "aarch64-linux", "Linux_aarch64"],
    }
}

fn relevant_library(kind: Kind, name: &str, runtime: bool) -> bool {
    let prefixes: &[&str] = match kind {
        Kind::Cudnn => &["cudnn", "libcudnn"],
        Kind::Nvcomp => &["nvcomp", "libnvcomp"],
        Kind::Nccl => &["nccl", "libnccl"],
        Kind::Ngx if runtime => &["nvngx_", "libnvidia-ngx"],
        Kind::Ngx => &["nvsdk_ngx", "libnvsdk_ngx"],
        Kind::Nvapi => &["nvapi"],
        Kind::VideoCodec => &["nvencodeapi", "nvcuvid", "libnvcuvid", "libnvidia-encode"],
        Kind::OpticalFlow => &["nvof", "libnvidia-opticalflow"],
    };
    prefixes.iter().any(|prefix| name.starts_with(*prefix))
}

fn has_library(dir: &Path, kind: Kind, target: Arch, windows: bool, runtime: bool) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| {
        let path = entry.path();
        if !path.is_file() {
            return false;
        }
        let Some(name) = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(str::to_ascii_lowercase)
        else {
            return false;
        };
        let dynamic = if windows {
            name.ends_with(".dll")
        } else {
            name.ends_with(".so") || name.contains(".so.")
        };
        let link = if windows {
            name.ends_with(".lib")
        } else {
            name.ends_with(".so") || name.ends_with(".a")
        };
        if !relevant_library(kind, &name, runtime) || !(if runtime { dynamic } else { link }) {
            return false;
        }
        let arch = if dynamic {
            crate::discovery::binary_arch(&path)
        } else {
            let format = if windows {
                object::BinaryFormat::Coff
            } else {
                object::BinaryFormat::Elf
            };
            crate::discovery::archive_arch(&path, format)
        };
        arch == Some(target)
    })
}

fn runtime_dirs(
    kind: Kind,
    root: &Path,
    include_dir: &Path,
    target: Arch,
    cuda: Option<(u32, u32)>,
    windows: bool,
) -> (Option<PathBuf>, Option<PathBuf>) {
    let mut runtime_bases = Vec::new();
    let mut link_bases = Vec::new();
    if kind == Kind::VideoCodec {
        // The SDK's Linux stubs are for linking only, never runtime loader search paths.
        match (windows, target) {
            (true, Arch::X64) => link_bases.push(root.join("Lib").join("win").join("x64")),
            (false, Arch::X64) => {
                link_bases.push(root.join("Lib").join("linux").join("stubs").join("x86_64"))
            }
            (false, Arch::Arm64) => {
                link_bases.push(root.join("Lib").join("linux").join("stubs").join("aarch64"))
            }
            _ => {}
        }
    }
    if kind == Kind::Ngx && target == Arch::X64 {
        // NVIDIA/DLSS packages feature runtimes separately from the static NGX loader.
        let package = root.join("lib").join(if windows {
            "Windows_x86_64"
        } else {
            "Linux_x86_64"
        });
        runtime_bases.extend([package.join("rel"), package.join("dev")]);
        if windows {
            link_bases.push(package.join("x64"));
        }
    }
    if windows {
        if kind == Kind::Cudnn
            && let Some(version) = include_dir
                .file_name()
                .filter(|_| include_dir.parent().is_some_and(|p| p.ends_with("include")))
            && version
                .to_str()
                .and_then(version_key)
                .is_some_and(|v| v.len() == 2)
        {
            runtime_bases.push(root.join("bin").join(version));
            link_bases.push(root.join("lib").join(version));
        }
        if kind == Kind::Nvcomp
            && let Some((major, _)) = cuda
        {
            runtime_bases.push(root.join("bin").join(major.to_string()));
            link_bases.push(root.join("lib").join(major.to_string()));
        }
        runtime_bases.push(root.join("bin"));
        link_bases.extend([root.join("lib"), root.join("Lib"), root.to_path_buf()]);
    } else {
        for base in ["lib", "lib64", "Lib"] {
            runtime_bases.push(root.join(base));
            link_bases.push(root.join(base));
        }
        for tag in native_tags(target, false) {
            runtime_bases.push(root.join("targets").join(tag).join("lib"));
            link_bases.push(root.join("targets").join(tag).join("lib"));
        }
    }
    let choose = |bases: &[PathBuf], runtime| {
        for base in bases {
            for tag in native_tags(target, windows) {
                let dir = base.join(tag);
                if has_library(&dir, kind, target, windows, runtime) {
                    return Some(dir);
                }
            }
            if has_library(base, kind, target, windows, runtime) {
                return Some(base.clone());
            }
        }
        None
    };
    let runtime = choose(&runtime_bases, true);
    let link = choose(&link_bases, false);
    (runtime, link)
}

fn read_version(kind: Kind, include_dir: &Path) -> Option<String> {
    let parts: Vec<String> = kind
        .version_define()
        .iter()
        .map(|(header, define)| read_define(&include_dir.join(header), define))
        .collect::<Option<Vec<_>>>()?;
    Some(parts.join("."))
}

fn read_define(header: &Path, name: &str) -> Option<String> {
    let text = std::fs::read_to_string(header).ok()?;
    text.lines().find_map(|line| {
        let mut tokens = line.split_whitespace();
        if tokens.next()? != "#define" || tokens.next()? != name {
            return None;
        }
        let token = tokens.next()?;
        numeric_part(token).map(|_| token.to_string())
    })
}
/// Standard installer parents and explicitly configured SDK search parents.
pub fn search_parents() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if cfg!(target_os = "windows") {
        if let Some(pf) = std::env::var_os("ProgramFiles").filter(|s| !s.is_empty()) {
            out.push(PathBuf::from(pf).join("NVIDIA"));
        }
        if let Some(pd) = std::env::var_os("ProgramData").filter(|s| !s.is_empty()) {
            out.push(PathBuf::from(pd).join("NVIDIA Corporation"));
        }
    }
    if cfg!(target_os = "linux") {
        out.extend([PathBuf::from("/usr"), PathBuf::from("/usr/local")]);
    }
    if let Some(dirs) = std::env::var_os("NV_SDK_DIRS") {
        out.extend(std::env::split_paths(&dirs).filter(|p| !p.as_os_str().is_empty()));
    }
    out
}

/// Inventory, with a valid configured root first and searched alternatives afterward.
pub fn list(kind: Kind, target: Arch) -> Vec<SdkInfo> {
    list_with(kind, &search_parents(), target)
}

/// Inventory over supplied parents, still honoring the first configured root.
pub fn list_with(kind: Kind, parents: &[PathBuf], target: Arch) -> Vec<SdkInfo> {
    inventory(
        kind,
        kind.requested_root().as_ref(),
        parents,
        target,
        selected_cuda(target),
    )
}

fn inventory(
    kind: Kind,
    requested: Option<&crate::discovery::RootRequest>,
    parents: &[PathBuf],
    target: Arch,
    cuda: Option<(u32, u32)>,
) -> Vec<SdkInfo> {
    let mut named: Vec<_> = requested
        .and_then(|request| probe_with(kind, &request.path, target, cuda))
        .into_iter()
        .collect();
    let mut searched = Vec::new();
    if !kind.is_supported() {
        return named;
    }
    for parent in parents {
        // Distribution prefixes are direct roots; never recursively scan system trees.
        let depth = if parent == Path::new("/usr") || parent == Path::new("/usr/local") {
            0
        } else {
            SEARCH_DEPTH
        };
        walk(parent, depth, &mut |dir| {
            if let Some(info) = probe_with(kind, dir, target, cuda) {
                if !named
                    .iter()
                    .chain(searched.iter())
                    .any(|s| s.root == info.root)
                {
                    searched.push(info);
                }
                return true;
            }
            false
        });
    }
    searched.sort_by(|a, b| {
        matches_installed_cuda(b, cuda)
            .cmp(&matches_installed_cuda(a, cuda))
            .then_with(|| version_key(&b.version).cmp(&version_key(&a.version)))
            .then_with(|| a.root.cmp(&b.root))
    });
    named.extend(searched);
    named
}

/// Select the requested root, or the best searched SDK when no root was requested.
pub fn detect(kind: Kind, target: Arch) -> Option<SdkInfo> {
    detect_with(
        kind,
        kind.requested_root().as_ref(),
        &search_parents(),
        target,
        selected_cuda(target),
    )
}

fn detect_with(
    kind: Kind,
    requested: Option<&crate::discovery::RootRequest>,
    parents: &[PathBuf],
    target: Arch,
    cuda: Option<(u32, u32)>,
) -> Option<SdkInfo> {
    match requested {
        Some(request) => probe_with(kind, &request.path, target, cuda),
        None => inventory(kind, None, parents, target, cuda)
            .into_iter()
            .next(),
    }
}

fn matches_installed_cuda(sdk: &SdkInfo, cuda: Option<(u32, u32)>) -> bool {
    if sdk.kind != Kind::Cudnn {
        return true;
    }
    let Some((major, minor)) = cuda else {
        return false;
    };
    if sdk
        .include_dir
        .parent()
        .is_some_and(|p| p.ends_with("include"))
        && let Some(folder) = sdk.include_dir.file_name().and_then(|n| n.to_str())
        && let Some(parts) = version_key(folder)
    {
        return parts.as_slice() == [major, minor];
    }
    // Flat or multiarch headers do not declare a CUDA release restriction.
    true
}

fn numeric_part(value: &str) -> Option<u32> {
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        (!hex.is_empty())
            .then(|| u32::from_str_radix(hex, 16).ok())
            .flatten()
    } else {
        (!value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()))
            .then(|| value.parse().ok())
            .flatten()
    }
}

fn version_key(value: &str) -> Option<Vec<u32>> {
    value.split('.').map(numeric_part).collect()
}

/// Visit directories to a bounded depth, without following symlink directories.
fn walk(dir: &Path, depth: usize, visit: &mut dyn FnMut(&Path) -> bool) {
    if visit(dir) || depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut subdirs: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|ty| ty.is_dir()))
        .map(|e| e.path())
        .collect();
    subdirs.sort();
    for sub in subdirs {
        if sub
            .file_name()
            .is_some_and(|n| n == ".git" || n == "target" || n == "node_modules")
        {
            continue;
        }
        walk(&sub, depth - 1, visit);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "vcv_nvsdk_{}_{}_{}",
                std::process::id(),
                stamp,
                id
            ));
            std::fs::create_dir(&root).expect("unique fixture directory");
            Self(root)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            if let Err(error) = std::fs::remove_dir_all(&self.0) {
                if std::thread::panicking() {
                    eprintln!("failed to remove own fixture {}: {error}", self.0.display());
                } else {
                    panic!("failed to remove own fixture {}: {error}", self.0.display());
                }
            }
        }
    }

    fn write(path: &Path, text: &str) {
        write_bytes(path, text.as_bytes());
    }

    fn write_bytes(path: &Path, bytes: &[u8]) {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("fixture dirs");
        std::fs::write(path, bytes).expect("fixture write");
    }

    fn elf(path: &Path, target: Arch) {
        let mut bytes = [0; 64];
        bytes[..4].copy_from_slice(b"\x7fELF");
        bytes[4] = if target == Arch::X86 { 1 } else { 2 };
        bytes[5] = 1;
        bytes[6] = 1;
        let machine: u16 = match target {
            Arch::X64 => 62,
            Arch::X86 => 3,
            Arch::Arm64 => 183,
        };
        bytes[18..20].copy_from_slice(&machine.to_le_bytes());
        write_bytes(path, &bytes);
    }

    fn pe(path: &Path, target: Arch) {
        let mut bytes = [0; 70];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&64u32.to_le_bytes());
        bytes[64..68].copy_from_slice(b"PE\0\0");
        let machine: u16 = match target {
            Arch::X64 => 0x8664,
            Arch::X86 => 0x014c,
            Arch::Arm64 => 0xaa64,
        };
        bytes[68..70].copy_from_slice(&machine.to_le_bytes());
        write_bytes(path, &bytes);
    }

    fn cudnn(root: &Path, cuda: &str, minor: u32) {
        let include = if cfg!(target_os = "windows") {
            root.join("include").join(cuda)
        } else {
            root.join("include")
        };
        write(
            &include.join("cudnn_version.h"),
            &format!(
                "#define CUDNN_MAJOR 9\n#define CUDNN_MINOR {minor}\n#define CUDNN_PATCHLEVEL 0\n"
            ),
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn cudnn_versioned_layout_is_found_under_a_parent() {
        let base = Fixture::new();
        let root = base.0.join("CUDNN").join("v9.23");
        cudnn(&root, "13.3", 23);
        cudnn(&root, "13.5", 28);
        let found = inventory(
            Kind::Cudnn,
            None,
            std::slice::from_ref(&base.0),
            Arch::X64,
            Some((13, 3)),
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].version, "9.23.0");
        assert_eq!(found[0].include_dir, root.join("include").join("13.3"));
        assert_eq!(
            found[0].runtime_dir, None,
            "empty or absent native dirs are not runtime evidence"
        );
    }

    #[test]
    fn header_only_sdks_report_their_header_directory_and_version() {
        let base = Fixture::new();
        let video = base.0.join("video");
        write(
            &video.join("Interface").join("nvEncodeAPI.h"),
            "#define NVENCAPI_MAJOR_VERSION 13\n#define NVENCAPI_MINOR_VERSION 1\n",
        );
        let ngx = base.0.join("Streamline").join("external").join("ngx-sdk");
        write(&ngx.join("include").join("nvsdk_ngx.h"), "");
        write(
            &ngx.join("include").join("nvsdk_ngx_defs.h"),
            "#define NVSDK_NGX_VERSION_API_MACRO 0x0000015\n",
        );
        let parents = std::slice::from_ref(&base.0);
        let video = inventory(Kind::VideoCodec, None, parents, Arch::Arm64, None).remove(0);
        assert_eq!(video.version, "13.1");
        assert_eq!(video.target, Arch::Arm64);
        assert!(video.runtime_dir.is_none() && video.lib_dir.is_none());
        let ngx = inventory(Kind::Ngx, None, parents, Arch::X64, None).remove(0);
        assert_eq!(ngx.version, "0x0000015");
        assert!(ngx.root.ends_with("ngx-sdk"));
        assert!(ngx.runtime_dir.is_none() && ngx.lib_dir.is_none());
    }

    #[test]
    fn a_define_prefix_does_not_match_a_longer_name() {
        let base = Fixture::new();
        write(
            &base.0.join("x.h"),
            "#defineNVAPI_SDK_VERSION 1\n#define NVAPI_SDK_VERSION_EXTRA 2\n",
        );
        assert_eq!(read_define(&base.0.join("x.h"), "NVAPI_SDK_VERSION"), None);
        for directive in [
            "#define NVAPI_SDK_VERSION 7\n",
            "#define\tNVAPI_SDK_VERSION\t7\n",
        ] {
            write(&base.0.join("x.h"), directive);
            assert_eq!(
                read_define(&base.0.join("x.h"), "NVAPI_SDK_VERSION").as_deref(),
                Some("7")
            );
        }
        write(
            &base.0.join("x.h"),
            "#define NVSDK_NGX_VERSION_API 0x0000015\n",
        );
        assert_eq!(
            read_define(&base.0.join("x.h"), "NVSDK_NGX_VERSION_API").as_deref(),
            Some("0x0000015")
        );
        write(&base.0.join("x.h"), "#define NVAPI_SDK_VERSION bad\n");
        assert_eq!(read_define(&base.0.join("x.h"), "NVAPI_SDK_VERSION"), None);
        assert_eq!(version_key("9.bad.1"), None);
        assert_eq!(version_key("9..1"), None);
    }

    #[test]
    fn invalid_explicit_root_never_selects_an_alternative() {
        let base = Fixture::new();
        let alternate = base.0.join("valid");
        cudnn(&alternate, "13.3", 23);
        let request = crate::discovery::first_root_with(Kind::Cudnn.root_vars(), |name| {
            Some(
                if name == "CUDNN_ROOT" {
                    base.0.join("missing")
                } else {
                    alternate.clone()
                }
                .into_os_string(),
            )
        })
        .expect("first alias");
        let parents = std::slice::from_ref(&base.0);
        assert_eq!(request.variable, "CUDNN_ROOT");
        assert!(
            detect_with(
                Kind::Cudnn,
                Some(&request),
                parents,
                Arch::X64,
                Some((13, 3))
            )
            .is_none()
        );
        assert_eq!(
            inventory(
                Kind::Cudnn,
                Some(&request),
                parents,
                Arch::X64,
                Some((13, 3))
            )
            .len(),
            1
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn named_roots_and_cuda_compatibility_have_separate_precedence() {
        let base = Fixture::new();
        let compatible = base.0.join("compatible");
        let newer = base.0.join("newer");
        cudnn(&compatible, "13.3", 23);
        cudnn(&newer, "13.5", 28);
        let parents = std::slice::from_ref(&base.0);
        assert_eq!(
            detect_with(Kind::Cudnn, None, parents, Arch::X64, Some((13, 3)))
                .expect("selected")
                .root,
            compatible
        );
        let request = crate::discovery::RootRequest {
            variable: "CUDNN_ROOT",
            path: newer.clone(),
        };
        assert_eq!(
            detect_with(
                Kind::Cudnn,
                Some(&request),
                parents,
                Arch::X64,
                Some((13, 3))
            )
            .expect("named")
            .root,
            newer
        );
        assert_eq!(
            inventory(
                Kind::Cudnn,
                Some(&request),
                parents,
                Arch::X64,
                Some((13, 3))
            )[0]
            .root,
            newer
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn nvcomp_major_layout_uses_selected_cuda_and_verified_architecture() {
        let base = Fixture::new();
        let root = &base.0;
        write(&root.join("include/nvcomp.h"), "");
        write(
            &root.join("include/nvcomp/version.h"),
            "#define NVCOMP_VER_MAJOR 6\n#define NVCOMP_VER_MINOR 0\n#define NVCOMP_VER_PATCH 0\n",
        );
        let headers =
            probe_with(Kind::Nvcomp, root, Arch::X64, None).expect("headers without CUDA major");
        assert_eq!(headers.version, "6.0.0");
        assert!(headers.runtime_dir.is_none() && headers.lib_dir.is_none());
        pe(&root.join("bin/12/nvcomp64_6.dll"), Arch::X64);
        pe(&root.join("bin/13/nvcomp64_6.dll"), Arch::X64);
        write_bytes(
            &root.join("lib/12/nvcomp.lib"),
            &crate::discovery::test_archive(object::BinaryFormat::Coff, Arch::X64),
        );
        write_bytes(
            &root.join("lib/13/nvcomp.lib"),
            &crate::discovery::test_archive(object::BinaryFormat::Coff, Arch::X64),
        );
        let dirs = runtime_dirs(
            Kind::Nvcomp,
            root,
            &root.join("include"),
            Arch::X64,
            Some((12, 9)),
            true,
        );
        assert_eq!(dirs, (Some(root.join("bin/12")), Some(root.join("lib/12"))));
        assert_eq!(
            runtime_dirs(
                Kind::Nvcomp,
                root,
                &root.join("include"),
                Arch::X64,
                None,
                true
            ),
            (None, None)
        );
        assert_eq!(
            runtime_dirs(
                Kind::Nvcomp,
                root,
                &root.join("include"),
                Arch::Arm64,
                Some((12, 9)),
                true
            ),
            (None, None)
        );
    }

    #[test]
    fn flat_libraries_require_physical_target_headers() {
        let base = Fixture::new();
        let root = &base.0;
        write(&root.join("bin/nvcomp64_6.dll"), "unknown");
        write(
            &root.join("lib/nvcomp_static.lib"),
            "unknown static archive",
        );
        assert_eq!(
            runtime_dirs(Kind::Nvcomp, root, root, Arch::X64, None, true),
            (None, None)
        );
        #[cfg(target_os = "windows")]
        {
            elf(&root.join("bin/nvcomp64_6.dll"), Arch::X64);
            assert_eq!(
                runtime_dirs(Kind::Nvcomp, root, root, Arch::X64, None, true),
                (None, None)
            );
            pe(&root.join("bin/nvcomp64_6.dll"), Arch::Arm64);
            assert_eq!(
                runtime_dirs(Kind::Nvcomp, root, root, Arch::X64, None, true),
                (None, None)
            );
            pe(&root.join("bin/nvcomp64_6.dll"), Arch::X64);
            assert_eq!(
                runtime_dirs(Kind::Nvcomp, root, root, Arch::X64, None, true),
                (Some(root.join("bin")), None)
            );
        }
        #[cfg(target_os = "linux")]
        {
            pe(&root.join("lib/libnvcomp.so.6"), Arch::X64);
            assert_eq!(
                runtime_dirs(Kind::Nvcomp, root, root, Arch::X64, None, false),
                (None, None)
            );
            elf(&root.join("lib/libnvcomp.so.6"), Arch::Arm64);
            assert_eq!(
                runtime_dirs(Kind::Nvcomp, root, root, Arch::X64, None, false),
                (None, None)
            );
            elf(&root.join("lib/libnvcomp.so.6"), Arch::X64);
            assert_eq!(
                runtime_dirs(Kind::Nvcomp, root, root, Arch::X64, None, false),
                (Some(root.join("lib")), None)
            );
            elf(&root.join("lib/libnvcomp.so"), Arch::X64);
            assert_eq!(
                runtime_dirs(Kind::Nvcomp, root, root, Arch::X64, None, false),
                (Some(root.join("lib")), Some(root.join("lib")))
            );
        }
    }

    #[test]
    fn tagged_dynamic_files_still_require_native_binary_headers() {
        let base = Fixture::new();
        let root = &base.0;
        #[cfg(target_os = "windows")]
        {
            let library = root.join("bin/x64/nvcomp64_6.dll");
            elf(&library, Arch::X64);
            assert_eq!(
                runtime_dirs(Kind::Nvcomp, root, root, Arch::X64, None, true),
                (None, None)
            );
            pe(&library, Arch::Arm64);
            assert_eq!(
                runtime_dirs(Kind::Nvcomp, root, root, Arch::X64, None, true),
                (None, None)
            );
            pe(&library, Arch::X64);
            assert_eq!(
                runtime_dirs(Kind::Nvcomp, root, root, Arch::X64, None, true).0,
                Some(root.join("bin/x64"))
            );
        }
        #[cfg(target_os = "linux")]
        {
            let library = root.join("lib/x86_64-linux-gnu/libnvcomp.so.6");
            pe(&library, Arch::X64);
            assert_eq!(
                runtime_dirs(Kind::Nvcomp, root, root, Arch::X64, None, false),
                (None, None)
            );
            elf(&library, Arch::Arm64);
            assert_eq!(
                runtime_dirs(Kind::Nvcomp, root, root, Arch::X64, None, false),
                (None, None)
            );
            elf(&library, Arch::X64);
            assert_eq!(
                runtime_dirs(Kind::Nvcomp, root, root, Arch::X64, None, false).0,
                Some(root.join("lib/x86_64-linux-gnu"))
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_cudnn_uses_flat_or_multiarch_headers() {
        let base = Fixture::new();
        let root = &base.0;
        write(
            &root.join("include/13.3/cudnn_version.h"),
            "#define CUDNN_MAJOR 9\n#define CUDNN_MINOR 28\n#define CUDNN_PATCHLEVEL 0\n",
        );
        assert!(probe_with(Kind::Cudnn, root, Arch::X64, Some((13, 3))).is_none());
        write(
            &root.join("include/x86_64-linux-gnu/cudnn_version.h"),
            "#define CUDNN_MAJOR 9\n#define CUDNN_MINOR 23\n#define CUDNN_PATCHLEVEL 0\n",
        );
        let info =
            probe_with(Kind::Cudnn, root, Arch::X64, Some((13, 3))).expect("multiarch headers");
        assert_eq!(info.include_dir, root.join("include/x86_64-linux-gnu"));
        assert_eq!(info.version, "9.23.0");
        assert!(probe_with(Kind::Cudnn, root, Arch::Arm64, None).is_none());
        cudnn(root, "13.3", 24);
        assert_eq!(
            probe_with(Kind::Cudnn, root, Arch::Arm64, None)
                .expect("flat headers")
                .version,
            "9.24.0"
        );
    }

    #[test]
    fn tagged_sdk_archives_require_matching_format_and_architecture() {
        use object::BinaryFormat::{Coff, Elf};
        let base = Fixture::new();
        let root = &base.0;
        for (windows, tag, name, format, foreign_format) in [
            (true, "Windows_x86_64", "nvsdk_ngx_d.lib", Coff, Elf),
            (false, "Linux_x86_64", "libnvsdk_ngx.a", Elf, Coff),
        ] {
            let dir = root.join("lib").join(tag);
            let file = dir.join(name);
            write_bytes(
                &file,
                &crate::discovery::test_archive(foreign_format, Arch::X64),
            );
            assert_eq!(
                runtime_dirs(Kind::Ngx, root, root, Arch::X64, None, windows),
                (None, None)
            );
            write_bytes(&file, &crate::discovery::test_archive(format, Arch::Arm64));
            assert_eq!(
                runtime_dirs(Kind::Ngx, root, root, Arch::X64, None, windows),
                (None, None)
            );
            if windows {
                elf(&file, Arch::Arm64);
            } else {
                pe(&file, Arch::X64);
            }
            assert_eq!(
                runtime_dirs(Kind::Ngx, root, root, Arch::X64, None, windows),
                (None, None)
            );
            for malformed in [
                b"".as_slice(),
                b"!<arch>\n".as_slice(),
                b"not an archive".as_slice(),
            ] {
                write_bytes(&file, malformed);
                assert_eq!(
                    runtime_dirs(Kind::Ngx, root, root, Arch::X64, None, windows),
                    (None, None)
                );
            }
            write_bytes(&file, &crate::discovery::test_archive(format, Arch::X64));
            assert_eq!(
                runtime_dirs(Kind::Ngx, root, root, Arch::X64, None, windows),
                (None, Some(dir))
            );
            std::fs::remove_file(file).expect("remove isolated archive");
        }
    }

    #[test]
    #[cfg(windows)]
    fn nvcomp_import_proof_is_independent_of_runtime_proof() {
        use object::BinaryFormat::Coff;
        let base = Fixture::new();
        let root = &base.0;
        let link = root.join("lib/13");
        let runtime = root.join("bin/13");
        let import = link.join("nvcomp.lib");
        let dll = runtime.join("nvcomp64_6.dll");
        write_bytes(&import, &crate::discovery::test_archive(Coff, Arch::X64));
        assert_eq!(
            runtime_dirs(Kind::Nvcomp, root, root, Arch::X64, Some((13, 3)), true),
            (None, Some(link.clone()))
        );
        pe(&dll, Arch::X64);
        for corrupt in [b"".as_slice(), b"not an import library".as_slice()] {
            write_bytes(&import, corrupt);
            assert_eq!(
                runtime_dirs(Kind::Nvcomp, root, root, Arch::X64, Some((13, 3)), true),
                (Some(runtime.clone()), None)
            );
        }
        pe(&import, Arch::Arm64);
        assert_eq!(
            runtime_dirs(Kind::Nvcomp, root, root, Arch::X64, Some((13, 3)), true),
            (Some(runtime.clone()), None)
        );
        write_bytes(&import, &crate::discovery::test_archive(Coff, Arch::Arm64));
        assert_eq!(
            runtime_dirs(Kind::Nvcomp, root, root, Arch::X64, Some((13, 3)), true),
            (Some(runtime.clone()), None)
        );
        assert_eq!(
            runtime_dirs(Kind::Nvcomp, root, root, Arch::Arm64, Some((13, 3)), true),
            (None, Some(link.clone()))
        );
        write_bytes(&import, &crate::discovery::test_archive(Coff, Arch::X64));
        assert_eq!(
            runtime_dirs(Kind::Nvcomp, root, root, Arch::X64, Some((13, 3)), true),
            (Some(runtime), Some(link.clone()))
        );
        std::fs::remove_file(dll).expect("remove runtime");
        assert_eq!(
            runtime_dirs(Kind::Nvcomp, root, root, Arch::X64, Some((13, 3)), true),
            (None, Some(link))
        );
    }

    #[test]
    fn tagged_sdk_folders_respect_platform_and_target() {
        let base = Fixture::new();
        let root = &base.0;
        write_bytes(
            &root.join("lib/Windows_x86_64/nvsdk_ngx.lib"),
            &crate::discovery::test_archive(object::BinaryFormat::Coff, Arch::X64),
        );
        write_bytes(
            &root.join("lib/Linux_x86_64/libnvsdk_ngx.a"),
            &crate::discovery::test_archive(object::BinaryFormat::Elf, Arch::X64),
        );
        write_bytes(
            &root.join("lib/arm64/nvsdk_ngx.lib"),
            &crate::discovery::test_archive(object::BinaryFormat::Coff, Arch::Arm64),
        );
        assert_eq!(
            runtime_dirs(Kind::Ngx, root, root, Arch::X64, None, false).1,
            Some(root.join("lib/Linux_x86_64"))
        );
        assert_eq!(
            runtime_dirs(Kind::Ngx, root, root, Arch::Arm64, None, false),
            (None, None)
        );
        assert_eq!(
            runtime_dirs(Kind::Ngx, root, root, Arch::Arm64, None, true).1,
            Some(root.join("lib/arm64"))
        );
        #[cfg(target_os = "windows")]
        pe(&root.join("bin/13.3/x64/cudnn64_9.dll"), Arch::X64);
        write_bytes(
            &root.join("lib/13.3/x64/cudnn.lib"),
            &crate::discovery::test_archive(object::BinaryFormat::Coff, Arch::X64),
        );
        let include = root.join("include/13.3");
        assert_eq!(
            runtime_dirs(
                Kind::Cudnn,
                root,
                &include,
                Arch::Arm64,
                Some((13, 3)),
                true
            ),
            (None, None)
        );
        #[cfg(target_os = "windows")]
        assert_eq!(
            runtime_dirs(Kind::Cudnn, root, &include, Arch::X64, Some((13, 3)), true).0,
            Some(root.join("bin/13.3/x64"))
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn ngx_windows_package_paths_preserve_target_and_release_precedence() {
        let base = Fixture::new();
        let root = &base.0;
        let package = root.join("lib/Windows_x86_64");
        let flat = package.join("nvsdk_ngx_d.lib");
        write_bytes(
            &flat,
            &crate::discovery::test_archive(object::BinaryFormat::Coff, Arch::X64),
        );
        assert_eq!(
            runtime_dirs(Kind::Ngx, root, root, Arch::X64, None, true).1,
            Some(package.clone())
        );
        let nested = package.join("x64");
        write_bytes(
            &nested.join("nvsdk_ngx_d.lib"),
            &crate::discovery::test_archive(object::BinaryFormat::Coff, Arch::X64),
        );
        assert_eq!(
            runtime_dirs(Kind::Ngx, root, root, Arch::X64, None, true).1,
            Some(nested)
        );
        let release = package.join("rel");
        let development = package.join("dev");
        let release_file = release.join("nvngx_dlss.dll");
        let development_file = development.join("nvngx_dlssd.dll");
        write(&release_file, "not a native library");
        pe(&development_file, Arch::X64);
        assert_eq!(
            runtime_dirs(Kind::Ngx, root, root, Arch::X64, None, true).0,
            Some(development.clone())
        );
        pe(&release_file, Arch::X64);
        assert_eq!(
            runtime_dirs(Kind::Ngx, root, root, Arch::X64, None, true).0,
            Some(release)
        );
        pe(&release_file, Arch::Arm64);
        assert_eq!(
            runtime_dirs(Kind::Ngx, root, root, Arch::X64, None, true).0,
            Some(development)
        );
        pe(&development_file, Arch::Arm64);
        assert_eq!(
            runtime_dirs(Kind::Ngx, root, root, Arch::X64, None, true).0,
            None
        );
        elf(&release_file, Arch::X64);
        assert_eq!(
            runtime_dirs(Kind::Ngx, root, root, Arch::X64, None, true).0,
            None
        );
        for foreign in [Arch::X86, Arch::Arm64] {
            assert_eq!(
                runtime_dirs(Kind::Ngx, root, root, foreign, None, true),
                (None, None)
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn ngx_linux_package_paths_preserve_target_and_release_precedence() {
        let base = Fixture::new();
        let root = &base.0;
        let package = root.join("lib/Linux_x86_64");
        write_bytes(
            &package.join("libnvsdk_ngx.a"),
            &crate::discovery::test_archive(object::BinaryFormat::Elf, Arch::X64),
        );
        assert_eq!(
            runtime_dirs(Kind::Ngx, root, root, Arch::X64, None, false).1,
            Some(package.clone())
        );
        let release = package.join("rel");
        let development = package.join("dev");
        let release_file = release.join("libnvidia-ngx-dlss.so.310.9.16");
        let development_file = development.join("libnvidia-ngx-dlssd.so.310.9.16");
        write(&release_file, "not a native library");
        elf(&development_file, Arch::X64);
        assert_eq!(
            runtime_dirs(Kind::Ngx, root, root, Arch::X64, None, false).0,
            Some(development.clone())
        );
        elf(&release_file, Arch::X64);
        assert_eq!(
            runtime_dirs(Kind::Ngx, root, root, Arch::X64, None, false).0,
            Some(release)
        );
        elf(&release_file, Arch::Arm64);
        assert_eq!(
            runtime_dirs(Kind::Ngx, root, root, Arch::X64, None, false).0,
            Some(development)
        );
        elf(&development_file, Arch::Arm64);
        assert_eq!(
            runtime_dirs(Kind::Ngx, root, root, Arch::X64, None, false).0,
            None
        );
        pe(&release_file, Arch::X64);
        assert_eq!(
            runtime_dirs(Kind::Ngx, root, root, Arch::X64, None, false).0,
            None
        );
        for foreign in [Arch::X86, Arch::Arm64] {
            assert_eq!(
                runtime_dirs(Kind::Ngx, root, root, foreign, None, false),
                (None, None)
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn nccl_versioned_soname_is_sufficient_native_library_evidence() {
        let base = Fixture::new();
        let root = &base.0;
        write(
            &root.join("include/nccl.h"),
            "#define NCCL_MAJOR 2\n#define NCCL_MINOR 31\n#define NCCL_PATCH 2\n",
        );
        elf(&root.join("lib/libnccl.so.2"), Arch::X64);
        assert_eq!(
            read_version(Kind::Nccl, &root.join("include")).as_deref(),
            Some("2.31.2")
        );
        assert_eq!(
            runtime_dirs(
                Kind::Nccl,
                root,
                &root.join("include"),
                Arch::X64,
                None,
                false
            ),
            (Some(root.join("lib")), None)
        );
        assert_eq!(
            runtime_dirs(
                Kind::Nccl,
                root,
                &root.join("include"),
                Arch::Arm64,
                None,
                false
            ),
            (None, None)
        );
        let runtime_only = probe_with(Kind::Nccl, root, Arch::X64, None).expect("NCCL SDK");
        assert_eq!(runtime_only.runtime_dir, Some(root.join("lib")));
        assert_eq!(runtime_only.lib_dir, None);
        elf(&root.join("lib/libnccl.so"), Arch::X64);
        assert_eq!(
            runtime_dirs(
                Kind::Nccl,
                root,
                &root.join("include"),
                Arch::X64,
                None,
                false
            ),
            (Some(root.join("lib")), Some(root.join("lib")))
        );
        assert_eq!(
            probe_with(Kind::Nccl, root, Arch::X64, None)
                .expect("linkable NCCL SDK")
                .lib_dir,
            Some(root.join("lib"))
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn video_codec_windows_imports_are_link_only_and_target_specific() {
        let base = Fixture::new();
        let root = &base.0;
        let dir = root.join("Lib/win/x64");
        for name in ["nvcuvid.lib", "nvencodeapi.lib"] {
            let library = dir.join(name);
            write_bytes(
                &library,
                &crate::discovery::test_archive(object::BinaryFormat::Coff, Arch::X64),
            );
            assert_eq!(
                runtime_dirs(Kind::VideoCodec, root, root, Arch::X64, None, true),
                (None, Some(dir.clone()))
            );
            for foreign in [Arch::X86, Arch::Arm64] {
                assert_eq!(
                    runtime_dirs(Kind::VideoCodec, root, root, foreign, None, true),
                    (None, None)
                );
            }
            std::fs::remove_file(library).expect("remove own import library");
        }
        pe(&dir.join("nvcuvid.dll"), Arch::X64);
        assert_eq!(
            runtime_dirs(Kind::VideoCodec, root, root, Arch::X64, None, true),
            (None, None),
            "link-only paths never supply runtime DLLs"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn video_codec_linux_stubs_are_link_only_and_require_native_target() {
        let base = Fixture::new();
        let root = &base.0;
        for (target, tag, foreign) in [
            (Arch::X64, "x86_64", Arch::Arm64),
            (Arch::Arm64, "aarch64", Arch::X64),
        ] {
            let dir = root.join("Lib/linux/stubs").join(tag);
            for name in ["libnvcuvid.so", "libnvidia-encode.so"] {
                let library = dir.join(name);
                pe(&library, target);
                assert_eq!(
                    runtime_dirs(Kind::VideoCodec, root, root, target, None, false),
                    (None, None),
                    "foreign binary format"
                );
                elf(&library, foreign);
                assert_eq!(
                    runtime_dirs(Kind::VideoCodec, root, root, target, None, false),
                    (None, None),
                    "foreign architecture"
                );
                elf(&library, target);
                assert_eq!(
                    runtime_dirs(Kind::VideoCodec, root, root, target, None, false),
                    (None, Some(dir.clone())),
                    "native stubs are link metadata only"
                );
                std::fs::remove_file(library).expect("remove own stub");
            }
        }
        assert_eq!(
            runtime_dirs(Kind::VideoCodec, root, root, Arch::X86, None, false),
            (None, None)
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn nvapi_is_unsupported_on_linux_even_with_headers() {
        let base = Fixture::new();
        write(&base.0.join("nvapi_interface.h"), "");
        write(
            &base.0.join("nvapi_lite_common.h"),
            "#define NVAPI_SDK_VERSION 62115\n",
        );
        assert!(!Kind::Nvapi.is_supported());
        assert!(probe_with(Kind::Nvapi, &base.0, Arch::X64, None).is_none());
    }
}
