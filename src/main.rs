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
//! - `detect` - VS/SDK/UCRT detection via vswhere and registry
//! - `env` - Environment variable assembly
//! - `format` - Output formatters (ps, cmd, sh, json)
//! - `registry` - Windows registry helpers
//!
//! ## Dependencies
//! - `clap` - CLI argument parsing
//! - `winreg` - Windows registry access
//! - `serde_json` - JSON parsing (vswhere output)

mod detect;
mod env;
mod format;
mod registry;

use clap::{Parser, ValueEnum};
use std::env as std_env;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
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

#[derive(Debug, Clone, Copy, ValueEnum)]
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
    // MSYS2/Git Bash
    if std_env::var("MSYSTEM").is_ok() {
        return Format::Sh;
    }
    // CMD always sets PROMPT variable
    if std_env::var("PROMPT").is_ok() {
        return Format::Cmd;
    }
    // Default to PowerShell on Windows
    Format::Ps
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
  vcv -v 2022 | iex                    # Use VS 2022 specifically"#;

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

    /// VS version year (2017, 2019, 2022)
    #[arg(short = 'v', long = "vs")]
    vs_year: Option<u16>,

    /// Suppress info messages
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,

    /// Skip cl.exe validation
    #[arg(long = "no-validate")]
    no_validate: bool,
}

fn main() {
    let args = Args::parse();

    // Validate VS year if specified
    if let Some(year) = args.vs_year {
        if !matches!(year, 2017 | 2019 | 2022) {
            eprintln!("Error: Invalid VS year {}. Use 2017, 2019, or 2022", year);
            std::process::exit(1);
        }
    }

    // Detect VS
    let vs = match detect::detect_vs(args.vs_year) {
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
    }

    // Build environment
    let env = env::build_env(&vs, sdk.as_ref(), ucrt.as_ref(), args.host, args.arch);

    // Validate cl.exe exists
    if !args.no_validate {
        let cl_exists = env.path.iter().any(|p| p.join("cl.exe").exists());
        if !cl_exists {
            eprintln!("Warning: cl.exe not found in PATH");
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
