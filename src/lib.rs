//! Library for detecting Visual Studio / MSVC environment.
//!
//! Use [detect::detect_vs](detect::detect_vs) and [env::build_env](env::build_env) to get
//! PATH, INCLUDE, LIB, LIBPATH for the current or specified VS installation.

pub mod detect;
pub mod env;
pub mod format;
pub mod registry;

pub use detect::{SdkInfo, VsInfo};
pub use env::Env;
pub use format::{fmt_cmd, fmt_json, fmt_ps, fmt_sh};

use std::fmt;

/// Target or host architecture for the toolchain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
