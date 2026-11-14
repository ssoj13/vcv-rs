//! Command-line options and host-native shell selection.

use clap::{Parser, ValueEnum};
use vcv_rs::Arch;

/// An explicit configured root must be valid in both Auto and On modes.
#[cfg(any(feature = "cuda", feature = "optix"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(super) enum SdkMode {
    /// Add a detected SDK; reject invalid explicit configuration.
    Auto,
    /// Require an SDK.
    On,
    /// Ignore this SDK and its configured roots.
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(super) enum Format {
    Auto,
    Ps,
    Powershell,
    Cmd,
    Sh,
    Bash,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(super) enum Edition {
    Community,
    Professional,
    Enterprise,
    #[value(name = "buildtools", alias = "build-tools")]
    BuildTools,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(super) enum Channel {
    Exclude,
    Allow,
    Only,
}

const EXAMPLES: &str = r#"Windows PowerShell:
  vcv-rs -f ps | iex
  vcv-rs -v 2022 -e buildtools -f ps | iex
  vcv-rs -s x64 -a arm64 -f ps | iex

Windows CMD:
  vcv-rs -f cmd > vcenv.bat && vcenv.bat

Linux / Bash / MSYS2:
  eval "$(vcv-rs -f sh)"

Inventory and tools:
  vcv-rs --list
  vcv-rs -f json --quiet"#;

#[derive(Debug, Parser)]
#[command(name = "vcv-rs", about = "Toolchain and SDK environments for Windows and Linux", after_help = EXAMPLES)]
pub(super) struct Args {
    /// Target architecture (default: native; cross targets require Windows tools)
    #[arg(short = 'a', long = "arch", value_enum)]
    pub(super) arch: Option<Arch>,
    /// Host architecture (default: native)
    #[arg(short = 's', long = "host", value_enum)]
    pub(super) host: Option<Arch>,
    /// Output format (auto detects the current shell)
    #[arg(short = 'f', long = "format", value_enum, default_value = "auto")]
    pub(super) format: Format,
    /// Exact Windows Visual Studio year: 2017, 2019, 2022, 2026
    #[arg(short = 'v', long = "vs", conflicts_with_all = ["vs_min", "vs_max"])]
    pub(super) vs_year: Option<u16>,
    /// Oldest acceptable Visual Studio year (Windows)
    #[arg(long = "vs-min")]
    pub(super) vs_min: Option<u16>,
    /// Newest acceptable Visual Studio year (Windows)
    #[arg(long = "vs-max")]
    pub(super) vs_max: Option<u16>,
    /// Require a Visual Studio edition (Windows)
    #[arg(short = 'e', long = "edition", value_enum)]
    pub(super) edition: Option<Edition>,
    /// Visual Studio preview channel (Windows; default: exclude)
    #[arg(long = "prerelease", value_enum)]
    pub(super) prerelease: Option<Channel>,
    /// List discovered installations and mark the selected roots
    #[arg(short = 'l', long = "list")]
    pub(super) list: bool,
    /// CUDA Toolkit handling
    #[cfg(feature = "cuda")]
    #[arg(short = 'c', long = "cuda", value_enum, default_value = "auto")]
    pub(super) cuda: SdkMode,
    /// OptiX SDK handling
    #[cfg(feature = "optix")]
    #[arg(short = 'o', long = "optix", value_enum, default_value = "auto")]
    pub(super) optix: SdkMode,
    /// Omit separately shipped NVIDIA SDKs and their root requests
    #[cfg(feature = "nvsdk")]
    #[arg(long = "no-nvsdk")]
    pub(super) no_nvsdk: bool,
    /// Suppress informational stderr output
    #[arg(short = 'q', long = "quiet")]
    pub(super) quiet: bool,
    /// Skip validation of the selected host/target cl.exe path (Windows)
    #[arg(long = "no-validate")]
    pub(super) no_validate: bool,
}

pub(super) fn detect_shell() -> Format {
    detect_shell_with(cfg!(windows), |key| std::env::var_os(key).is_some())
}

fn detect_shell_with(windows: bool, mut is_set: impl FnMut(&str) -> bool) -> Format {
    if is_set("MSYSTEM") || is_set("ZSH_VERSION") || is_set("BASH_VERSION") {
        return Format::Sh;
    }
    if windows && (is_set("PROMPT") || is_set("CMDCMDLINE") || is_set("CmdCmdLine")) {
        return Format::Cmd;
    }
    if is_set("POWERSHELL_DISTRIBUTION_CHANNEL")
        || is_set("PSExecutionPolicyPreference")
        || (windows && is_set("PSModulePath"))
    {
        return Format::Ps;
    }
    if windows { Format::Ps } else { Format::Sh }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detect_from(windows: bool, values: &[(&str, &str)]) -> Format {
        detect_shell_with(windows, |key| {
            values.iter().any(|(candidate, _)| *candidate == key)
        })
    }

    #[test]
    fn detect_msystem_as_sh() {
        assert_eq!(detect_from(true, &[("MSYSTEM", "MINGW64")]), Format::Sh);
    }

    #[test]
    fn detect_zsh_as_sh() {
        assert_eq!(detect_from(false, &[("ZSH_VERSION", "5.9")]), Format::Sh);
    }

    #[test]
    fn detect_bash_as_sh() {
        assert_eq!(detect_from(false, &[("BASH_VERSION", "5.2")]), Format::Sh);
    }

    #[test]
    fn detect_cmd() {
        assert_eq!(detect_from(true, &[("PROMPT", "$P$G")]), Format::Cmd);
    }

    #[test]
    fn detect_cmd_over_pwsh_markers() {
        assert_eq!(
            detect_from(
                true,
                &[
                    ("PROMPT", "$P$G"),
                    ("POWERSHELL_DISTRIBUTION_CHANNEL", "PowerShell")
                ]
            ),
            Format::Cmd
        );
    }

    #[test]
    fn detect_psmodulepath_as_ps() {
        assert_eq!(
            detect_from(true, &[("PSModulePath", "modules")]),
            Format::Ps
        );
    }

    #[test]
    fn linux_ignores_inherited_windows_shell_markers() {
        assert_eq!(
            detect_from(
                false,
                &[("PROMPT", "$P$G"), ("PSModulePath", "windows-modules")]
            ),
            Format::Sh
        );
        assert_eq!(
            detect_from(false, &[("POWERSHELL_DISTRIBUTION_CHANNEL", "PowerShell")]),
            Format::Ps
        );
    }

    #[test]
    fn shell_defaults_are_native() {
        assert_eq!(detect_from(true, &[]), Format::Ps);
        assert_eq!(detect_from(false, &[]), Format::Sh);
    }
}
