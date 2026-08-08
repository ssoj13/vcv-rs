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
use vcv_rs::detect::{Prerelease, VsEdition, VsFilter};
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

VS selection:
  vcv -l                               # List everything detected, mark what would be picked
  vcv -v 2022 | iex                    # Exactly VS 2022
  vcv -v 2026 | iex                    # Exactly VS 2026
  vcv --vs-max 2022 | iex              # Newest up to 2022
  vcv --vs-min 2022 | iex              # 2022 or newer
  vcv -e enterprise | iex              # Require the Enterprise edition
  vcv -e buildtools | iex              # Standalone C++ Build Tools (CI machines)
  vcv --prerelease allow | iex         # Let Preview installs compete
  vcv --prerelease only | iex          # Preview channel on purpose

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
    #[arg(short = 'v', long = "vs", conflicts_with_all = ["vs_min", "vs_max"])]
    vs_year: Option<u16>,

    /// Oldest acceptable VS year
    #[arg(long = "vs-min")]
    vs_min: Option<u16>,

    /// Newest acceptable VS year
    #[arg(long = "vs-max")]
    vs_max: Option<u16>,

    /// Require a specific edition (default: any)
    #[arg(short = 'e', long = "edition", value_enum)]
    edition: Option<VsEdition>,

    /// Preview / Insiders channel handling
    #[arg(long = "prerelease", value_enum, default_value = "exclude")]
    prerelease: Prerelease,

    /// List every detected toolchain and exit
    #[arg(short = 'l', long = "list")]
    list: bool,

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

/// One line describing an installation, in the order a human scans for it.
fn describe_vs(vs: &vcv_rs::VsInfo) -> String {
    let year = vs
        .year
        .map_or_else(|| "????".to_string(), |y| y.to_string());
    let channel = if vs.prerelease { "  [prerelease]" } else { "" };
    format!(
        "{year}  {:<16} {:<13} VC {:<12} {}{channel}",
        vs.version,
        vs.edition.as_str(),
        vs.tools_ver,
        vs.install.display()
    )
}

/// The active filter in words, for the "nothing matched" message.
///
/// Reported as the constraint that failed rather than as "not found": with four independent axes,
/// "Visual Studio not found" on a machine that has three of them is a message that sends the
/// operator looking in the wrong place.
fn describe(filter: &VsFilter) -> String {
    let years = match (filter.min_year, filter.max_year) {
        (Some(a), Some(b)) if a == b => format!("year {a}"),
        (Some(a), Some(b)) => format!("years {a}..{b}"),
        (Some(a), None) => format!("year >= {a}"),
        (None, Some(b)) => format!("year <= {b}"),
        (None, None) => "any year".to_string(),
    };
    let edition = filter
        .edition
        .map_or_else(|| "any edition".to_string(), |e| e.as_str().to_string());
    let channel = match filter.prerelease {
        Prerelease::Exclude => "released only",
        Prerelease::Allow => "released or prerelease",
        Prerelease::Only => "prerelease only",
    };
    format!("{years}, {edition}, {channel}")
}

/// Everything detected, with the entry the current flags would select marked.
///
/// This exists so "which compiler will I actually get" is answerable without running a build and
/// reading a compiler banner — the selection is shown next to the alternatives it beat.
fn print_list(filter: &VsFilter) {
    let installs = detect::list_vs();
    let chosen = detect::detect_vs_filtered(filter).map(|vs| vs.install);

    println!("Filter: {}", describe(filter));
    println!("\nVisual Studio ({} installed):", installs.len());
    if installs.is_empty() {
        println!("  (none)");
    }
    for vs in &installs {
        let mark = if chosen.as_ref() == Some(&vs.install) {
            "->"
        } else {
            "  "
        };
        println!("{mark} {}", describe_vs(vs));
    }

    if let Some(sdk) = detect::detect_sdk() {
        println!("\nWindows SDK: {}  {}", sdk.version, sdk.path.display());
    }
    if let Some(ucrt) = detect::detect_ucrt() {
        println!("UCRT:        {}  {}", ucrt.version, ucrt.path.display());
    }

    #[cfg(feature = "cuda")]
    {
        let toolkits = cuda::list_toolkits();
        println!("\nCUDA ({} installed):", toolkits.len());
        if toolkits.is_empty() {
            println!("  (none)");
        }
        for (i, c) in toolkits.iter().enumerate() {
            let mark = if i == 0 { "->" } else { "  " };
            let msvc = c.msvc.map_or_else(
                || "no declared _MSC_VER range".to_string(),
                |r| format!("_MSC_VER {}..{}", r.min, r.max_exclusive),
            );
            println!("{mark} {:<6} {:<28} {}", c.version, msvc, c.root.display());
        }
    }
}

fn main() {
    let args = Args::parse();

    // Validate every year the operator can supply, on all three flags. 2026 is accepted because
    // detection maps it (VS major 18); rejecting it here while `detect` resolves it would make the
    // newest compiler unselectable.
    for year in [args.vs_year, args.vs_min, args.vs_max]
        .into_iter()
        .flatten()
    {
        if !matches!(year, 2017 | 2019 | 2022 | 2026) {
            eprintln!(
                "Error: Invalid VS year {}. Use 2017, 2019, 2022, or 2026",
                year
            );
            std::process::exit(1);
        }
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

    // Year bounds. The operator's flags win outright; the CUDA-derived range applies only when no
    // bound was given at all, so `-v`/`--vs-min`/`--vs-max` are never silently narrowed by a
    // toolkit. A pinned year outside the toolkit's range is warned about below, not overridden.
    // `mut` is used only by the CUDA narrowing below, which compiles out with the feature.
    #[cfg_attr(not(feature = "cuda"), allow(unused_mut))]
    let (mut min_year, mut max_year) = (args.vs_year.or(args.vs_min), args.vs_year.or(args.vs_max));
    #[cfg(feature = "cuda")]
    if min_year.is_none()
        && max_year.is_none()
        && let Some(range) = cuda.as_ref().and_then(|c| c.msvc)
    {
        (min_year, max_year) = (range.min_vs_year(), range.max_vs_year());
    }

    let filter = VsFilter {
        min_year,
        max_year,
        edition: args.edition,
        prerelease: args.prerelease,
    };

    if args.list {
        print_list(&filter);
        return;
    }

    // Detect VS
    let vs = match detect::detect_vs_filtered(&filter) {
        Some(vs) => vs,
        None => {
            eprintln!(
                "Error: no Visual Studio installation matches {}",
                describe(&filter)
            );
            let all = detect::list_vs();
            if all.is_empty() {
                eprintln!("No Visual Studio C++ toolchain is installed.");
            } else {
                eprintln!("Installed:");
                for vs in all {
                    eprintln!("  {}", describe_vs(&vs));
                }
            }
            std::process::exit(1);
        }
    };

    let sdk = detect::detect_sdk();
    let ucrt = detect::detect_ucrt();

    // Print info to stderr
    if !args.quiet {
        eprintln!(
            "# VS {} {} | VC {}",
            vs.version,
            vs.edition.as_str(),
            vs.tools_ver
        );
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
