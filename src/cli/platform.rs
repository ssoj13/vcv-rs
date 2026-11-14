//! Native environment setup; Visual Studio is consulted only on Windows.

use super::args::Args;
#[cfg(windows)]
use super::args::{Channel, Edition};
use vcv_rs::{Arch, Env};
#[cfg(windows)]
use vcv_rs::{VsFilter, VsInfo, detect, env};

pub(super) struct Base {
    pub(super) env: Env,
    pub(super) info: Vec<String>,
}

pub(super) fn architectures(args: &Args) -> Result<(Arch, Arch), String> {
    if !cfg!(any(target_os = "windows", target_os = "linux")) {
        return Err("environment setup supports Windows and Linux only".into());
    }
    let native = Arch::native()
        .ok_or_else(|| format!("unsupported native architecture {}", std::env::consts::ARCH))?;
    validate_architectures(args, native, cfg!(windows))
}

fn validate_architectures(
    args: &Args,
    native: Arch,
    windows: bool,
) -> Result<(Arch, Arch), String> {
    for year in [args.vs_year, args.vs_min, args.vs_max]
        .into_iter()
        .flatten()
    {
        if !matches!(year, 2017 | 2019 | 2022 | 2026) {
            return Err(format!(
                "invalid Visual Studio year {year}; use 2017, 2019, 2022, or 2026"
            ));
        }
    }
    if let (Some(min), Some(max)) = (args.vs_min, args.vs_max)
        && min > max
    {
        return Err("--vs-min cannot exceed --vs-max".into());
    }
    let host = args.host.unwrap_or(native);
    let target = args.arch.unwrap_or(native);
    if !windows {
        if args.vs_year.is_some()
            || args.vs_min.is_some()
            || args.vs_max.is_some()
            || args.edition.is_some()
            || args.prerelease.is_some()
            || args.no_validate
        {
            return Err(
                "Visual Studio selection and --no-validate are Windows-only options".into(),
            );
        }
        if host != native || target != native {
            return Err(format!(
                "Linux environment setup requires native host and target {native}; cross-compiler setup is unsupported"
            ));
        }
    }
    Ok((host, target))
}

#[cfg(windows)]
fn filter(args: &Args, cuda_bounds: Option<(Option<u16>, Option<u16>)>) -> VsFilter {
    let explicit = (args.vs_year.or(args.vs_min), args.vs_year.or(args.vs_max));
    let (min_year, max_year) = if explicit == (None, None) {
        cuda_bounds.unwrap_or(explicit)
    } else {
        explicit
    };
    let edition = args.edition.map(|edition| match edition {
        Edition::Community => vcv_rs::VsEdition::Community,
        Edition::Professional => vcv_rs::VsEdition::Professional,
        Edition::Enterprise => vcv_rs::VsEdition::Enterprise,
        Edition::BuildTools => vcv_rs::VsEdition::BuildTools,
    });
    let prerelease = match args.prerelease.unwrap_or(Channel::Exclude) {
        Channel::Exclude => vcv_rs::Prerelease::Exclude,
        Channel::Allow => vcv_rs::Prerelease::Allow,
        Channel::Only => vcv_rs::Prerelease::Only,
    };
    VsFilter {
        min_year,
        max_year,
        edition,
        prerelease,
    }
}

#[cfg(windows)]
fn describe_vs(vs: &VsInfo) -> String {
    let year = vs
        .year
        .map_or_else(|| "unknown year".into(), |year| year.to_string());
    let channel = if vs.prerelease { " [prerelease]" } else { "" };
    format!(
        "{year} {} {} | VC {} | {}{channel}",
        vs.version,
        vs.edition.as_str(),
        vs.tools_ver,
        vs.install.display()
    )
}

#[cfg(windows)]
fn describe(filter: &VsFilter) -> String {
    let years = match (filter.min_year, filter.max_year) {
        (Some(a), Some(b)) if a == b => format!("year {a}"),
        (Some(a), Some(b)) => format!("years {a}..{b}"),
        (Some(a), None) => format!("year >= {a}"),
        (None, Some(b)) => format!("year <= {b}"),
        (None, None) => "any year".into(),
    };
    let edition = filter
        .edition
        .map_or("any edition", |edition| edition.as_str());
    let channel = match filter.prerelease {
        vcv_rs::Prerelease::Exclude => "released only",
        vcv_rs::Prerelease::Allow => "released or prerelease",
        vcv_rs::Prerelease::Only => "prerelease only",
    };
    format!("{years}, {edition}, {channel}")
}

#[cfg(windows)]
pub(super) fn build(
    args: &Args,
    host: Arch,
    target: Arch,
    cuda_bounds: Option<(Option<u16>, Option<u16>)>,
) -> Result<Base, String> {
    let filter = filter(args, cuda_bounds);
    let vs = detect::detect_vs_filtered(&filter).ok_or_else(|| {
        format!(
            "no Visual Studio C++ installation matches {}",
            describe(&filter)
        )
    })?;
    let host_dir = match host {
        Arch::X64 => "Hostx64",
        Arch::X86 => "Hostx86",
        Arch::Arm64 => "Hostarm64",
    };
    let compiler = vs
        .tools
        .join("bin")
        .join(host_dir)
        .join(target.as_str())
        .join("cl.exe");
    if !args.no_validate && !compiler.is_file() {
        return Err(format!(
            "selected Visual Studio has no {host} host / {target} target compiler at {}; use --list to inspect installations",
            compiler.display()
        ));
    }
    let sdk = detect::detect_sdk();
    let ucrt = detect::detect_ucrt();
    let environment = env::build_env(&vs, sdk.as_ref(), ucrt.as_ref(), host, target);
    let mut info = vec![
        format!("VS {}", describe_vs(&vs)),
        format!("host {host}, target {target}"),
    ];
    if let Some(sdk) = sdk {
        info.push(format!(
            "Windows SDK {} | {}",
            sdk.version,
            sdk.path.display()
        ));
    }
    if let Some(ucrt) = ucrt {
        info.push(format!("UCRT {} | {}", ucrt.version, ucrt.path.display()));
    }
    Ok(Base {
        env: environment,
        info,
    })
}

#[cfg(not(windows))]
pub(super) fn build(
    _args: &Args,
    _host: Arch,
    target: Arch,
    _cuda_bounds: Option<(Option<u16>, Option<u16>)>,
) -> Result<Base, String> {
    Ok(Base {
        env: Env::new(target),
        info: vec![format!("Linux native target {target}")],
    })
}

#[cfg(windows)]
pub(super) fn inventory(args: &Args, cuda_bounds: Option<(Option<u16>, Option<u16>)>) -> String {
    let filter = filter(args, cuda_bounds);
    let installs = detect::list_vs();
    let chosen = detect::detect_vs_filtered(&filter).map(|vs| vs.install);
    let mut output = format!(
        "Visual Studio filter: {}\n\nVisual Studio ({} installed):\n",
        describe(&filter),
        installs.len()
    );
    if installs.is_empty() {
        output.push_str("  (none)\n");
    }
    for vs in installs {
        let mark = if chosen.as_ref() == Some(&vs.install) {
            "->"
        } else {
            "  "
        };
        output.push_str(&format!("{mark} {}\n", describe_vs(&vs)));
    }
    if let Some(sdk) = detect::detect_sdk() {
        output.push_str(&format!(
            "\nWindows SDK: {} | {}\n",
            sdk.version,
            sdk.path.display()
        ));
    }
    if let Some(ucrt) = detect::detect_ucrt() {
        output.push_str(&format!(
            "UCRT: {} | {}\n",
            ucrt.version,
            ucrt.path.display()
        ));
    }
    output
}

#[cfg(not(windows))]
pub(super) fn inventory(_args: &Args, _cuda_bounds: Option<(Option<u16>, Option<u16>)>) -> String {
    format!(
        "Linux native architecture: {}\nVisual Studio: unsupported on Linux\n",
        std::env::consts::ARCH
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn args(values: &[&str]) -> Args {
        Args::try_parse_from(std::iter::once("vcv-rs").chain(values.iter().copied()))
            .expect("fixture args")
    }

    #[test]
    fn native_architecture_is_the_default_on_both_platforms() {
        for windows in [false, true] {
            assert_eq!(
                validate_architectures(&args(&[]), Arch::Arm64, windows),
                Ok((Arch::Arm64, Arch::Arm64))
            );
        }
    }

    #[test]
    fn linux_rejects_explicit_windows_requests() {
        for request in [
            vec!["--vs", "2022"],
            vec!["--vs-min", "2019"],
            vec!["--vs-max", "2026"],
            vec!["--edition", "community"],
            vec!["--prerelease", "exclude"],
            vec!["--no-validate"],
        ] {
            assert!(validate_architectures(&args(&request), Arch::X64, false).is_err());
        }
    }

    #[test]
    fn unsupported_linux_cross_targets_are_rejected() {
        for request in [vec!["--arch", "x86"], vec!["--host", "arm64"]] {
            assert!(validate_architectures(&args(&request), Arch::X64, false).is_err());
        }
        assert_eq!(
            validate_architectures(&args(&["--arch", "x86"]), Arch::X64, true),
            Ok((Arch::X64, Arch::X86))
        );
    }

    #[test]
    fn all_year_inputs_and_inverted_ranges_are_checked() {
        for flag in ["--vs", "--vs-min", "--vs-max"] {
            assert!(validate_architectures(&args(&[flag, "2025"]), Arch::X64, true).is_err());
        }
        assert!(
            validate_architectures(
                &args(&["--vs-min", "2026", "--vs-max", "2022"]),
                Arch::X64,
                true
            )
            .is_err()
        );
    }

    #[cfg(windows)]
    #[test]
    fn explicit_year_bounds_outrank_cuda_search_hints() {
        let cuda = Some((Some(2019), Some(2022)));
        let explicit = filter(&args(&["--vs-min", "2026"]), cuda);
        assert_eq!((explicit.min_year, explicit.max_year), (Some(2026), None));
        let automatic = filter(&args(&[]), cuda);
        assert_eq!(
            (automatic.min_year, automatic.max_year),
            (Some(2019), Some(2022))
        );
    }
}
