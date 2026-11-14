//! Tests native CLI output and rejection of invalid configuration before script emission.

#![cfg(feature = "cli")]

use std::path::PathBuf;
use std::process::{Command, Output};

fn command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_vcv-rs"));
    for key in ["VCPKG_ROOT", "VCPKG_DEFAULT_TRIPLET", "NV_SDK_DIRS"] {
        command.env_remove(key);
    }
    #[cfg(feature = "cuda")]
    for key in vcv_rs::cuda::root_vars() {
        command.env_remove(key);
    }
    #[cfg(feature = "optix")]
    for key in vcv_rs::optix::root_vars() {
        command.env_remove(key);
    }
    #[cfg(feature = "nvsdk")]
    for kind in vcv_rs::nvsdk::Kind::ALL {
        for key in kind.root_vars() {
            command.env_remove(key);
        }
    }
    command
}

fn without_sdks(command: &mut Command) {
    #[cfg(feature = "cuda")]
    command.args(["--cuda", "off"]);
    #[cfg(feature = "optix")]
    command.args(["--optix", "off"]);
    #[cfg(feature = "nvsdk")]
    command.arg("--no-nvsdk");
    command.arg("--quiet");
}

fn missing() -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("fixture clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("vcv-cli-missing-{}-{nonce}", std::process::id()));
    assert!(!path.exists());
    path
}

fn failure(output: Output, expected: &str) {
    assert!(!output.status.success(), "{output:?}");
    assert!(
        output.stdout.is_empty(),
        "failed command emitted stdout: {output:?}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(expected),
        "expected {expected:?} in {stderr:?}"
    );
}

#[test]
fn help_describes_both_platforms() {
    let output = command().arg("--help").output().expect("run CLI");
    assert!(output.status.success(), "{output:?}");
    let help = String::from_utf8(output.stdout).expect("help UTF-8");
    assert!(help.contains("Windows and Linux"));
    assert!(help.contains("--format"));
}

#[cfg(feature = "cuda")]
#[test]
fn automatic_cuda_rejects_an_invalid_explicit_root_before_any_script() {
    let output = command()
        .env(vcv_rs::cuda::root_vars()[0], missing())
        .args(["--cuda", "auto", "--format", "json"])
        .output()
        .expect("run CLI");
    failure(output, "invalid explicit root");
}

#[cfg(feature = "optix")]
#[test]
fn automatic_optix_rejects_an_invalid_explicit_root_before_any_script() {
    let mut command = command();
    #[cfg(feature = "cuda")]
    command.args(["--cuda", "off"]);
    let output = command
        .env(vcv_rs::optix::root_vars()[0], missing())
        .args(["--optix", "auto", "--format", "json"])
        .output()
        .expect("run CLI");
    failure(output, "invalid explicit root");
}

#[test]
fn disabled_sdks_ignore_invalid_explicit_roots() {
    let mut command = command();
    without_sdks(&mut command);
    #[cfg(feature = "cuda")]
    command.env(vcv_rs::cuda::root_vars()[0], missing());
    #[cfg(feature = "optix")]
    command.env(vcv_rs::optix::root_vars()[0], missing());
    #[cfg(feature = "nvsdk")]
    for kind in vcv_rs::nvsdk::Kind::ALL {
        command.env(kind.root_vars()[0], missing());
    }
    let output = command.arg("--list").output().expect("run CLI");
    assert!(output.status.success(), "{output:?}");
    assert!(!output.stdout.is_empty());
}

#[cfg(feature = "nvsdk")]
#[test]
fn explicitly_requested_sdk_reports_unsupported_platform() {
    let kind = if cfg!(windows) {
        vcv_rs::nvsdk::Kind::Nccl
    } else {
        vcv_rs::nvsdk::Kind::Nvapi
    };
    let mut command = command();
    #[cfg(feature = "cuda")]
    command.args(["--cuda", "off"]);
    #[cfg(feature = "optix")]
    command.args(["--optix", "off"]);
    let output = command
        .env(kind.root_vars()[0], missing())
        .arg("--list")
        .output()
        .expect("run CLI");
    failure(output, "unsupported on");
}

#[test]
fn vcpkg_configuration_errors_do_not_emit_scripts() {
    let mut invalid = command();
    without_sdks(&mut invalid);
    failure(
        invalid
            .env("VCPKG_ROOT", missing())
            .arg("--list")
            .output()
            .expect("run CLI"),
        "invalid explicit root",
    );
    let mut orphan = command();
    without_sdks(&mut orphan);
    failure(
        orphan
            .env("VCPKG_DEFAULT_TRIPLET", "x64-linux")
            .arg("--list")
            .output()
            .expect("run CLI"),
        "requires a configured VCPKG_ROOT",
    );
}

#[cfg(target_os = "linux")]
#[test]
fn linux_rejects_windows_options_cross_targets_and_cmd_before_output() {
    let other = if vcv_rs::Arch::native() == Some(vcv_rs::Arch::Arm64) {
        "x64"
    } else {
        "arm64"
    };
    for args in [
        vec!["--vs", "2022"],
        vec!["--prerelease", "exclude"],
        vec!["--no-validate"],
        vec!["--arch", other],
        vec!["--host", other],
        vec!["--format", "cmd"],
    ] {
        let mut command = command();
        without_sdks(&mut command);
        failure(command.args(args).output().expect("run CLI"), "Error:");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn linux_json_without_sdks_uses_native_variable_names() {
    let mut command = command();
    without_sdks(&mut command);
    let output = command
        .args(["--format", "json"])
        .output()
        .expect("run CLI");
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON output");
    for name in ["PATH", "CPATH", "LIBRARY_PATH", "LD_LIBRARY_PATH"] {
        assert!(json[name].is_array(), "{json}");
    }
    for name in ["INCLUDE", "LIB", "LIBPATH", "VSINSTALLDIR"] {
        assert!(json.get(name).is_none(), "{json}");
    }
}
