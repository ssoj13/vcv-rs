#![cfg(all(windows, feature = "cli"))]

//! # vcv - Fast Visual Studio Environment Setup
//!
//! CLI tool for setting up Visual Studio build environment ~50x faster than vcvars64.bat.
//!
//! ## Purpose
//! Replaces slow vcvars64.bat by directly querying vswhere.exe and Windows registry
//! to build PATH, INCLUDE, LIB, LIBPATH environment variables.
//!
//! ## Usage
//! ```powershell
//! vcv | iex                 # PowerShell (auto-detect)
//! vcv -f cmd > env.bat      # CMD
//! ```
//!
//! ## Modules
//! - `cuda` - CUDA Toolkit detection (version and accepted host compilers)
//! - `detect` - VS/SDK/UCRT detection via vswhere and registry
//! - `env` - Environment variable assembly
//! - `format` - Output formatters (ps, cmd, sh, json)
//! - `registry` - Windows registry helpers
//!
//! ## Dependencies
//! - `clap` - CLI argument parsing
//! - `winreg` - Windows registry access
//! - `serde_json` - JSON parsing (vswhere output)

use clap::{Parser, ValueEnum};
use std::env as std_env;
#[cfg(feature = "cuda")]
use vcv_rs::cuda;
use vcv_rs::{Arch, detect, env, format};

/// How to treat an installed CUDA Toolkit.
#[cfg(feature = "cuda")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CudaMode {
    /// Add it when one is found, say nothing when there is none.
    Auto,
    /// Require one; exit non-zero if absent.
    On,
    /// Ignore CUDA entirely.
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Format {
    Auto,
    Ps,
    Powershell,
    Cmd,
    Sh,
    Bash,
    Json,
}

/// Detect current shell from environment
fn detect_shell() -> Format {
    detect_shell_with(|key| std_env::var(key).ok())
}

fn detect_shell_with<F>(mut get: F) -> Format
where
    F: FnMut(&str) -> Option<String>,
{
    // MSYS2/Git Bash on Windows
    if get("MSYSTEM").is_some() {
        return Format::Sh;
    }
    // zsh (macOS default, Linux)
    if get("ZSH_VERSION").is_some() {
        return Format::Sh;
    }
    // bash
    if get("BASH_VERSION").is_some() {
        return Format::Sh;
    }
    // CMD (PROMPT or CMDCMDLINE is set by cmd.exe sessions)
    let is_cmd =
        get("PROMPT").is_some() || get("CMDCMDLINE").is_some() || get("CmdCmdLine").is_some();
    if is_cmd {
        return Format::Cmd;
    }
    // PowerShell (PSModulePath and related markers)
    if get("PSModulePath").is_some()
        || get("POWERSHELL_DISTRIBUTION_CHANNEL").is_some()
        || get("PSExecutionPolicyPreference").is_some()
    {
        return Format::Ps;
    }
    // Default: sh on Unix, ps on Windows
    #[cfg(windows)]
    return Format::Ps;
    #[cfg(not(windows))]
    return Format::Sh;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn detect_from(env: &[(&str, &str)]) -> Format {
        let mut map = HashMap::new();
        for (k, v) in env {
            map.insert((*k).to_string(), (*v).to_string());
        }
        detect_shell_with(|key| map.get(key).cloned())
    }

    #[test]
    fn detect_msystem_as_sh() {
        assert_eq!(detect_from(&[("MSYSTEM", "MINGW64")]), Format::Sh);
    }

    #[test]
    fn detect_zsh_as_sh() {
        assert_eq!(detect_from(&[("ZSH_VERSION", "5.9")]), Format::Sh);
    }

    #[test]
    fn detect_bash_as_sh() {
        assert_eq!(detect_from(&[("BASH_VERSION", "5.2")]), Format::Sh);
    }

    #[test]
    fn detect_cmd() {
        assert_eq!(detect_from(&[("PROMPT", "$P$G")]), Format::Cmd);
    }

    #[test]
    fn detect_cmd_over_pwsh_markers() {
        assert_eq!(
            detect_from(&[
                ("POWERSHELL_DISTRIBUTION_CHANNEL", "PowerShell"),
                ("PROMPT", "$P$G"),
            ]),
            Format::Cmd
        );
    }

    #[test]
    fn detect_psmodulepath_as_ps() {
        assert_eq!(
            detect_from(&[(
                "PSModulePath",
                "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\Modules"
            )]),
            Format::Ps
        );
    }
}

const EXAMPLES: &str = r#"
PowerShell:
  vcv | iex                            # Auto-detect, apply to session
  vcv -f ps | iex                      # Explicit PowerShell format
  vcv -q | iex                         # Quiet mode (no info)
  vcv -a x86 | iex                     # x86 target

CMD:
  vcv -f cmd > vcenv.bat && vcenv.bat  # Create and run batch
  for /f "delims=" %i in ('vcv -f cmd') do @%i

Bash / MSYS2:
  eval $(vcv -f sh)                    # Apply to current session

JSON (for tools):
  vcv -f json -q                       # Machine-readable output

Cross-compile:
  vcv -a arm64 | iex                   # Build for ARM64
  vcv -s x64 -a x86 | iex              # Host x64, target x86

VS version:
  vcv -v 2019 | iex                    # Use VS 2019 specifically
  vcv -v 2022 | iex                    # Use VS 2022 specifically

CUDA:
  vcv | iex                            # Toolkit added automatically when installed
  vcv -c on | iex                      # Fail if no CUDA Toolkit is present
  vcv -c off | iex                     # Plain MSVC environment"#;

#[derive(Parser)]
#[command(
    name = "vcv",
    about = "Fast VS environment (~50x faster than vcvars64.bat)",
    after_help = EXAMPLES
)]
struct Args {
    /// Target architecture
    #[arg(short = 'a', long = "arch", value_enum, default_value = "x64")]
    arch: Arch,

    /// Host architecture
    #[arg(short = 's', long = "host", value_enum, default_value = "x64")]
    host: Arch,

    /// Output format (auto = detect shell)
    #[arg(short = 'f', long = "format", value_enum, default_value = "auto")]
    format: Format,

    /// VS version year (2017, 2019, 2022, 2026)
    #[arg(short = 'v', long = "vs")]
    vs_year: Option<u16>,

    /// CUDA Toolkit handling
    #[cfg(feature = "cuda")]
    #[arg(short = 'c', long = "cuda", value_enum, default_value = "auto")]
    cuda: CudaMode,

    /// Suppress info messages
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,

    /// Skip cl.exe validation
    #[arg(long = "no-validate")]
    no_validate: bool,
}

fn main() {
    let args = Args::parse();

    // Validate VS year if specified. 2026 is accepted because detection maps it (VS major 18);
    // rejecting it here while `detect` resolves it would make the newest compiler unselectable.
    if let Some(year) = args.vs_year
        && !matches!(year, 2017 | 2019 | 2022 | 2026)
    {
        eprintln!(
            "Error: Invalid VS year {}. Use 2017, 2019, 2022, or 2026",
            year
        );
        std::process::exit(1);
    }

    // CUDA is resolved BEFORE Visual Studio, because the toolkit constrains which compiler is
    // usable. Building with a VS that nvcc rejects fails inside `host_config.h` with a `#error`,
    // which reads as a broken CUDA install rather than a compiler one release too new.
    #[cfg(feature = "cuda")]
    let cuda = {
        let found = match args.cuda {
            CudaMode::Off => None,
            CudaMode::Auto | CudaMode::On => cuda::detect_cuda(),
        };
        if args.cuda == CudaMode::On && found.is_none() {
            eprintln!(
                "Error: CUDA Toolkit not found (checked CUDA_PATH/CUDA_HOME/CUDA_ROOT/\
                 CUDA_TOOLKIT_ROOT_DIR, the standard install directory, and nvcc on PATH)"
            );
            std::process::exit(1);
        }
        found
    };

    // An explicit -v is the operator's decision and is never overridden; it is only checked
    // against the toolkit afterwards, so the warning names the real problem.
    #[cfg(feature = "cuda")]
    let (min_year, max_year) = match (args.vs_year, cuda.as_ref().and_then(|c| c.msvc)) {
        (Some(y), _) => (Some(y), Some(y)),
        (None, Some(range)) => (range.min_vs_year(), range.max_vs_year()),
        (None, None) => (None, None),
    };
    #[cfg(not(feature = "cuda"))]
    let (min_year, max_year) = (args.vs_year, args.vs_year);

    // Detect VS
    let vs = match detect::detect_vs_range(min_year, max_year) {
        Some(vs) => vs,
        None => {
            if let Some(year) = args.vs_year {
                eprintln!("Error: Visual Studio {} not found", year);
                let versions = detect::list_vs_versions();
                if !versions.is_empty() {
                    eprintln!("Available versions:");
                    for (y, v) in versions {
                        eprintln!("  {} ({})", y, v);
                    }
                }
            } else {
                eprintln!("Error: Visual Studio not found");
            }
            std::process::exit(1);
        }
    };

    let sdk = detect::detect_sdk();
    let ucrt = detect::detect_ucrt();

    // Print info to stderr
    if !args.quiet {
        eprintln!("# VS {} | VC {}", vs.version, vs.tools_ver);
        if let Some(ref s) = sdk {
            eprintln!("# SDK {}", s.version);
        }
        #[cfg(feature = "cuda")]
        if let Some(ref c) = cuda {
            eprintln!("# CUDA {} | {}", c.version, c.root.display());
        }
    }

    // Build environment
    #[cfg_attr(not(feature = "cuda"), allow(unused_mut))]
    let mut env = env::build_env(&vs, sdk.as_ref(), ucrt.as_ref(), args.host, args.arch);
    #[cfg(feature = "cuda")]
    if let Some(ref c) = cuda {
        env::add_cuda(&mut env, c, args.arch);
    }

    // Validate cl.exe exists
    if !args.no_validate {
        let cl_exists = env.path.iter().any(|p| p.join("cl.exe").exists());
        if !cl_exists {
            eprintln!("Warning: cl.exe not found in PATH");
        }
        // A pinned -v can land outside what the toolkit accepts. Warn with the numbers rather
        // than silently emitting an environment whose first .cu file fails with a #error.
        #[cfg(feature = "cuda")]
        if let (Some(year), Some(c)) = (args.vs_year, cuda.as_ref())
            && let Some(range) = c.msvc
            && !range.max_vs_year().is_some_and(|max| year <= max)
        {
            eprintln!(
                "Warning: VS {} may be rejected by CUDA {} (accepts _MSC_VER {}..{})",
                year, c.version, range.min, range.max_exclusive
            );
        }
    }

    // Resolve format
    let format = match args.format {
        Format::Auto => detect_shell(),
        other => other,
    };

    let output = match format {
        Format::Cmd => format::fmt_cmd(&env),
        Format::Ps | Format::Powershell => format::fmt_ps(&env),
        Format::Sh | Format::Bash => format::fmt_sh(&env),
        Format::Json => format::fmt_json(&env),
        Format::Auto => unreachable!(),
    };

    println!("{}", output);
}
