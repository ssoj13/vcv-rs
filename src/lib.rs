//! Toolchain and SDK discovery with environment generation on Windows and Linux.
//!
//! SDK discovery, environment assembly, and shell formatters are available on both
//! systems. Visual Studio and Windows registry discovery are Windows-only.

use std::fmt;

/// Target or host architecture for the toolchain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum Arch {
    X64,
    X86,
    Arm64,
}

impl Arch {
    /// Map a Rust target architecture name to the supported toolchain architecture.
    pub fn from_target_arch(arch: &str) -> Option<Self> {
        match arch {
            "x86_64" => Some(Self::X64),
            "x86" => Some(Self::X86),
            "aarch64" => Some(Self::Arm64),
            _ => None,
        }
    }

    /// Architecture of the platform for which this library was compiled.
    pub fn native() -> Option<Self> {
        Self::from_target_arch(std::env::consts::ARCH)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Arch::X64 => "x64",
            Arch::X86 => "x86",
            Arch::Arm64 => "arm64",
        }
    }
}

impl fmt::Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// CUDA Toolkit discovery. Compiled on every platform: the search locations differ per OS but the
/// facts a caller wants (root, version, accepted host compilers) do not, and a Windows-only module
/// would have to be re-invented the first time this runs on Linux.
#[cfg(feature = "cuda")]
pub mod cuda;

/// OptiX SDK discovery: root, version, function-table ABI, device-header presence.
#[cfg(feature = "optix")]
pub mod optix;

/// cuDNN, Video Codec, Optical Flow, NVAPI, NGX, nvCOMP, and NCCL SDK discovery.
#[cfg(feature = "nvsdk")]
pub mod nvsdk;

/// Shared explicit-root and native binary metadata contracts.
pub mod discovery;
pub mod env;
pub mod format;

#[cfg(windows)]
pub mod detect;
#[cfg(windows)]
pub mod registry;

#[cfg(feature = "cuda")]
pub use cuda::{CudaInfo, CudaVersion, MsvcRange, detect_cuda};
#[cfg(feature = "optix")]
pub use optix::{OptixInfo, OptixVersion, detect_optix};

#[cfg(windows)]
pub use detect::{
    Prerelease, SdkInfo, VsEdition, VsFilter, VsInfo, detect_vs_filtered, detect_vs_range, list_vs,
};
pub use env::{Env, EnvError, VcpkgInfo, add_vcpkg, detect_vcpkg, probe_vcpkg};
pub use format::{FormatError, fmt_cmd, fmt_json, fmt_ps, fmt_sh};

#[cfg(test)]
mod tests {
    use super::Arch;

    #[test]
    fn rust_target_architectures_are_explicit() {
        assert_eq!(Arch::from_target_arch("x86_64"), Some(Arch::X64));
        assert_eq!(Arch::from_target_arch("x86"), Some(Arch::X86));
        assert_eq!(Arch::from_target_arch("aarch64"), Some(Arch::Arm64));
        for unsupported in ["", "x64", "arm64", "riscv64", "nvptx64"] {
            assert_eq!(Arch::from_target_arch(unsupported), None);
        }
    }

    #[test]
    fn native_architecture_uses_the_rust_target_name() {
        assert_eq!(
            Arch::native(),
            Arch::from_target_arch(std::env::consts::ARCH)
        );
    }
}
