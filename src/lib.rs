//! Library for detecting Visual Studio / MSVC environment (Windows).
//!
//! On non-Windows targets the crate compiles as a stub so workspace builds succeed;
//! use [detect::detect_vs](detect::detect_vs) and [env::build_env](env::build_env) only on Windows.

use std::fmt;

/// Target or host architecture for the toolchain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(windows, derive(clap::ValueEnum))]
pub enum Arch {
    X64,
    X86,
    Arm64,
}

impl Arch {
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

#[cfg(windows)]
pub mod detect;
#[cfg(windows)]
pub mod env;
#[cfg(windows)]
pub mod format;
#[cfg(windows)]
pub mod registry;

#[cfg(feature = "cuda")]
pub use cuda::{CudaInfo, CudaVersion, MsvcRange, detect_cuda};

#[cfg(windows)]
pub use detect::{
    Prerelease, SdkInfo, VsEdition, VsFilter, VsInfo, detect_vs_filtered, detect_vs_range, list_vs,
};
#[cfg(windows)]
pub use env::Env;
#[cfg(windows)]
pub use format::{fmt_cmd, fmt_json, fmt_ps, fmt_sh};
