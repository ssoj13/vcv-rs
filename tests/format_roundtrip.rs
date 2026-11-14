//! Executes generated scripts in native shells and checks their resulting environment.
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use vcv_rs::{Arch, Env, fmt_cmd, fmt_json, fmt_ps, fmt_sh};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);

impl Scratch {
    fn create(path: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir(&path)?;
        Ok(Self(path))
    }

    fn new() -> Self {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("fixture clock is after Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "vcv-format-{}-{stamp}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        Self::create(path).expect("atomically create owned shell fixture")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        // Cleanup must not mask an assertion failure while this test unwinds.
        if let Err(error) = std::fs::remove_dir_all(&self.0) {
            eprintln!(
                "could not remove shell fixture {}: {error}",
                self.0.display()
            );
        }
    }
}

#[test]
fn scratch_creation_preserves_an_existing_directory() {
    let scratch = Scratch::new();
    let sentinel = scratch.0.join("sentinel");
    std::fs::write(&sentinel, b"owned by the original fixture").expect("write ownership sentinel");
    let error = Scratch::create(scratch.0.clone())
        .err()
        .expect("existing directory must not be claimed");
    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(
        std::fs::read(&sentinel).expect("existing tree preserved"),
        b"owned by the original fixture"
    );
}

fn fixture() -> Env {
    let mut env = Env::new(Arch::X64);
    #[cfg(windows)]
    {
        env.path.push(PathBuf::from(r"C:\SDK tools"));
        env.runtime.push(PathBuf::from(r"C:\SDK runtime"));
        env.include.push(PathBuf::from(r"C:\SDK include"));
        env.lib.push(PathBuf::from(r"C:\SDK lib"));
        env.libpath.push(PathBuf::from(r"C:\SDK assemblies"));
    }
    #[cfg(not(windows))]
    {
        env.path.push(PathBuf::from("/sdk/tools"));
        env.runtime.push(PathBuf::from("/sdk/runtime"));
        env.include.push(PathBuf::from("/sdk/include"));
        env.lib.push(PathBuf::from("/sdk/lib"));
    }
    env.vars.insert(
        "VCV_LITERAL".into(),
        OsString::from("Unicode Ж λ ' ‘ ’ “ ” \" $HOME `x` & $(false)"),
    );
    env
}

fn ps_quote(value: &str) -> String {
    format!(
        "'{}'",
        value
            .replace('\'', "''")
            .replace('‘', "‘‘")
            .replace('’', "’’")
    )
}

fn sh_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn sh_inheritance(old: Option<&str>) -> String {
    [
        "PATH",
        "INCLUDE",
        "LIB",
        "LIBPATH",
        "CPATH",
        "LIBRARY_PATH",
        "LD_LIBRARY_PATH",
    ]
    .into_iter()
    .map(|key| match old {
        Some(value) => format!("export {key}={}\n", sh_quote(value)),
        None => format!("unset {key}\n"),
    })
    .collect()
}

fn read_dump(path: &Path) -> BTreeMap<String, String> {
    serde_json::from_slice(&std::fs::read(path).expect("read child environment"))
        .expect("decode child environment")
}

fn child_command(command: &mut Command, output: &Path, old: Option<&str>) {
    command.env("VCV_DUMP_PATH", output);
    for key in [
        "PATH",
        "INCLUDE",
        "LIB",
        "LIBPATH",
        "CPATH",
        "LIBRARY_PATH",
        "LD_LIBRARY_PATH",
    ] {
        match old {
            Some(value) => {
                command.env(key, value);
            }
            None => {
                command.env_remove(key);
            }
        }
    }
}

#[test]
#[ignore = "child process helper, invoked by native shell roundtrip tests"]
fn dump_environment() {
    let output = std::env::var_os("VCV_DUMP_PATH").expect("child output path");
    let values: BTreeMap<String, String> = std::env::vars()
        .map(|(key, value)| {
            #[cfg(windows)]
            let key = key.to_ascii_uppercase();
            (key, value)
        })
        .collect();
    std::fs::write(
        output,
        serde_json::to_vec(&values).expect("encode environment"),
    )
    .expect("write environment");
}

#[test]
fn json_has_native_paths_and_literal_values() {
    let env = fixture();
    let value: serde_json::Value =
        serde_json::from_str(&fmt_json(&env).expect("format json")).expect("parse json");
    assert_eq!(
        value["VCV_LITERAL"],
        env.vars["VCV_LITERAL"].to_str().expect("fixture unicode")
    );
    #[cfg(windows)]
    {
        assert_eq!(
            value["PATH"],
            serde_json::json!([r"C:\SDK tools", r"C:\SDK runtime"])
        );
        assert_eq!(value["LIB"], serde_json::json!([r"C:\SDK lib"]));
    }
    #[cfg(not(windows))]
    {
        assert_eq!(value["PATH"], serde_json::json!(["/sdk/tools"]));
        assert_eq!(
            value["LD_LIBRARY_PATH"],
            serde_json::json!(["/sdk/runtime"])
        );
        assert_eq!(value["CPATH"], serde_json::json!(["/sdk/include"]));
    }
}

#[test]
fn format_rejects_reserved_keys_and_empty_path_components() {
    let mut env = Env::new(Arch::X64);
    env.vars.insert("PATH".into(), "collision".into());
    assert!(fmt_json(&env).is_err());
    assert!(fmt_ps(&env).is_err());
    assert!(fmt_sh(&env).is_err());
    env.vars.clear();
    env.path.push(PathBuf::new());
    assert!(fmt_json(&env).is_err());
    assert!(fmt_ps(&env).is_err());
    assert!(fmt_sh(&env).is_err());
    #[cfg(windows)]
    assert!(fmt_cmd(&env).is_err());
}

#[test]
fn format_rejects_control_values_and_separator_paths() {
    let mut env = Env::new(Arch::X64);
    env.vars.insert("VCV_BAD".into(), "line\nnext".into());
    assert!(fmt_ps(&env).is_err());
    assert!(fmt_sh(&env).is_err());
    env.vars.clear();
    env.path.push(PathBuf::from(if cfg!(windows) {
        r"C:\bad;path"
    } else {
        "/bad:path"
    }));
    assert!(fmt_ps(&env).is_err());
    assert!(fmt_sh(&env).is_err());
    assert!(fmt_json(&env).is_err());
}

#[cfg(not(windows))]
#[test]
fn linux_rejects_cmd_assemblies_and_non_unicode() {
    use std::os::unix::ffi::OsStringExt;
    let mut env = Env::new(Arch::X64);
    assert!(fmt_cmd(&env).is_err());
    env.libpath.push("/assemblies".into());
    assert!(fmt_json(&env).is_err());
    assert!(fmt_ps(&env).is_err());
    assert!(fmt_sh(&env).is_err());
    env.libpath.clear();
    env.path.push(PathBuf::from(OsString::from_vec(vec![0xff])));
    assert!(fmt_json(&env).is_err());
    assert!(fmt_ps(&env).is_err());
    assert!(fmt_sh(&env).is_err());
    env.path.clear();
    env.vars
        .insert("VCV_BAD".into(), OsString::from_vec(vec![0xff]));
    assert!(fmt_json(&env).is_err());
    assert!(fmt_ps(&env).is_err());
    assert!(fmt_sh(&env).is_err());
}

#[cfg(windows)]
#[test]
fn windows_non_unicode_path_is_rejected() {
    use std::os::windows::ffi::OsStringExt;
    let mut env = Env::new(Arch::X64);
    env.path.push(PathBuf::from(OsString::from_wide(&[0xd800])));
    assert!(fmt_cmd(&env).is_err());
    assert!(fmt_json(&env).is_err());
    assert!(fmt_ps(&env).is_err());
    assert!(fmt_sh(&env).is_err());
}

#[cfg(windows)]
#[test]
fn windows_key_rules_and_cmd_unsafe_values() {
    let mut env = Env::new(Arch::X64);
    env.vars
        .insert("ProgramFiles(x86)".into(), r"C:\Program Files (x86)".into());
    assert!(fmt_ps(&env).is_ok());
    assert!(fmt_cmd(&env).is_ok());
    assert!(fmt_sh(&env).is_err());
    env.vars.clear();
    env.vars.insert("path".into(), "case collision".into());
    assert!(fmt_json(&env).is_err());
    env.vars.clear();
    for value in ["%PATH%", "!PATH!", "a\"b", "a\nb"] {
        env.vars.insert("VCV_BAD".into(), value.into());
        assert!(fmt_cmd(&env).is_err(), "CMD accepted {value:?}");
    }
}

#[cfg(not(windows))]
#[test]
fn sh_shell_roundtrip() {
    let mut env = fixture();
    env.vars.insert("VCV_EMPTY".into(), OsString::new());
    let exe = std::env::current_exe().expect("test executable");
    for old in [
        None,
        Some(""),
        Some("/old/one:/old/two"),
        Some("/old/'quote:$HOME:$(false):`tick`"),
    ] {
        let scratch = Scratch::new();
        let output = scratch.0.join("dump.json");
        let mut script = fmt_sh(&env).expect("format sh");
        // Mutate only the test's generated script. The same committed harness must fail
        // with VCV_TEST_QUOTE_MUTANT=1 and pass without it.
        if std::env::var_os("VCV_TEST_QUOTE_MUTANT").is_some() {
            script = script.replace("'\"'\"'", "'");
        }
        script = format!("{}{script}", sh_inheritance(old));
        script.push_str(&format!(
            "\n{} --ignored --exact dump_environment --nocapture\n",
            sh_quote(exe.to_str().expect("unicode executable"))
        ));
        let mut command = Command::new("/bin/sh");
        command.arg("-c").arg(script);
        child_command(&mut command, &output, old);
        let result = command.output().expect("run /bin/sh");
        assert!(
            result.status.success(),
            "shell failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let values = read_dump(&output);
        let suffix = old
            .filter(|value| !value.is_empty())
            .map(|value| format!(":{value}"))
            .unwrap_or_default();
        assert_eq!(values["PATH"], format!("/sdk/tools{suffix}"));
        assert_eq!(values["CPATH"], format!("/sdk/include{suffix}"));
        assert_eq!(values["LIBRARY_PATH"], format!("/sdk/lib{suffix}"));
        assert_eq!(values["LD_LIBRARY_PATH"], format!("/sdk/runtime{suffix}"));
        assert_eq!(values.get("VCV_EMPTY"), Some(&String::new()));
        assert_eq!(
            values["VCV_LITERAL"],
            env.vars["VCV_LITERAL"].to_str().expect("unicode literal")
        );
    }
}

#[cfg(windows)]
fn cmd_fixture() -> Env {
    let mut env = fixture();
    env.runtime.insert(0, PathBuf::from(r"C:\SDK tools"));
    env.vars.clear();
    env.vars.insert(
        "VCV_LITERAL".into(),
        "space & caret ^ pipe | angles < > parentheses ( ) apostrophe ' dollar $ backtick `".into(),
    );
    env.vars
        .insert("ProgramFiles(x86)".into(), r"C:\Program Files (x86)".into());
    env
}

#[cfg(windows)]
#[test]
#[ignore = "child process helper, invoked with an isolated generator environment"]
fn render_cmd_environment() {
    let output = std::env::var_os("VCV_RENDER_PATH").expect("renderer output path");
    let outcome = match fmt_cmd(&cmd_fixture()) {
        Ok(script) => serde_json::json!({ "script": script }),
        Err(error) => serde_json::json!({ "error": error.to_string() }),
    };
    std::fs::write(
        output,
        serde_json::to_vec(&outcome).expect("encode renderer outcome"),
    )
    .expect("write renderer outcome");
}

#[cfg(windows)]
fn render_cmd_snapshot(
    output: &Path,
    old: Option<&str>,
    unsafe_list: Option<(&str, &str)>,
) -> serde_json::Value {
    let mut command = Command::new(std::env::current_exe().expect("test executable"));
    command.args([
        "--ignored",
        "--exact",
        "render_cmd_environment",
        "--nocapture",
    ]);
    child_command(&mut command, output, old);
    command.env("VCV_RENDER_PATH", output);
    if let Some((key, value)) = unsafe_list {
        command.env(key, value);
    }
    let result = command.output().expect("run isolated CMD renderer");
    assert!(
        result.status.success(),
        "renderer failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let outcome: serde_json::Value =
        serde_json::from_slice(&std::fs::read(output).expect("read renderer outcome"))
            .expect("decode renderer outcome");
    assert_eq!(
        outcome
            .as_object()
            .expect("structured renderer outcome")
            .len(),
        1
    );
    outcome
}

#[cfg(windows)]
#[test]
fn cmd_shell_roundtrip() {
    let env = cmd_fixture();
    let exe = std::env::current_exe().expect("test executable");
    let system_root = std::env::var_os("SystemRoot").expect("Windows system root");
    let cmd = PathBuf::from(system_root).join(r"System32\cmd.exe");
    for (old, suffix) in [
        (None, ""),
        (Some(""), ""),
        (
            Some(r"C:\old one;D:\old two;C:\old one"),
            r";C:\old one;D:\old two",
        ),
        (Some("A;;B;"), ";A;;B"),
        (
            Some(r"C:\old & ^ | < > ( ) $ `;D:\two"),
            r";C:\old & ^ | < > ( ) $ `;D:\two",
        ),
    ] {
        let scratch = Scratch::new();
        let outcome = render_cmd_snapshot(&scratch.0.join("render.json"), old, None);
        assert!(
            outcome.get("error").is_none(),
            "CMD renderer rejected {old:?}: {outcome}"
        );
        let mut script = outcome["script"]
            .as_str()
            .expect("successful renderer script")
            .to_owned();
        let output = scratch.0.join("dump.json");
        let script_path = scratch.0.join("apply.cmd");
        script.push_str(&format!(
            "\r\n\"{}\" --ignored --exact dump_environment --nocapture\r\n",
            exe.to_str().expect("unicode executable")
        ));
        std::fs::write(&script_path, script).expect("write cmd script");
        let mut command = Command::new(&cmd);
        command.args(["/d", "/v:off", "/c"]).arg(&script_path);
        child_command(&mut command, &output, Some(r"C:\VCV_APPLY_ONLY"));
        let result = command.output().expect("run cmd");
        assert!(
            result.status.success(),
            "CMD failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let values = read_dump(&output);
        for (key, prefix) in [
            ("PATH", r"C:\SDK tools;C:\SDK runtime"),
            ("INCLUDE", r"C:\SDK include"),
            ("LIB", r"C:\SDK lib"),
            ("LIBPATH", r"C:\SDK assemblies"),
        ] {
            assert_eq!(
                values[key],
                format!("{prefix}{suffix}"),
                "{key}, generator {old:?}"
            );
            assert!(
                !values[key].contains("VCV_APPLY_ONLY"),
                "{key} used apply-time inheritance"
            );
        }
        assert_eq!(
            values["VCV_LITERAL"],
            env.vars["VCV_LITERAL"].to_str().expect("ASCII literal")
        );
        assert_eq!(values["PROGRAMFILES(X86)"], r"C:\Program Files (x86)");
    }
}

#[cfg(windows)]
#[test]
fn cmd_rejects_unsafe_generator_lists_without_script() {
    let too_long = "a".repeat(8192);
    for key in ["PATH", "INCLUDE", "LIB", "LIBPATH"] {
        for value in [
            "a\"b",
            "%PATH%",
            "!PATH!",
            "a\nb",
            "a\rb",
            "a\tb",
            "caf\u{e9}",
            too_long.as_str(),
        ] {
            let scratch = Scratch::new();
            let outcome =
                render_cmd_snapshot(&scratch.0.join("render.json"), None, Some((key, value)));
            assert!(
                outcome.get("script").is_none(),
                "{key} emitted a script for {value:?}"
            );
            assert!(
                !outcome["error"]
                    .as_str()
                    .expect("structured FormatError")
                    .is_empty()
            );
        }
    }
}

#[cfg(windows)]
#[test]
fn windows_dump_normalizes_environment_key_case() {
    let scratch = Scratch::new();
    let output = scratch.0.join("dump.json");
    let mut command = Command::new(std::env::current_exe().expect("test executable"));
    command.args(["--ignored", "--exact", "dump_environment", "--nocapture"]);
    child_command(&mut command, &output, None);
    command
        .env("Path", r"C:\mixed case")
        .env("VcV_MiXeD_CaSe", "literal");
    let result = command.output().expect("run native environment dump");
    assert!(
        result.status.success(),
        "dump failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let values = read_dump(&output);
    assert_eq!(values["PATH"], r"C:\mixed case");
    assert_eq!(values["VCV_MIXED_CASE"], "literal");
    assert!(!values.contains_key("Path"));
    assert!(!values.contains_key("VcV_MiXeD_CaSe"));
}

fn required_shell(name: &str) -> PathBuf {
    let search =
        std::env::var_os("PATH").expect("PATH is required to discover the requested test shell");
    let path = std::env::split_paths(&search)
        .map(|dir| dir.join(name))
        .find(|path| path.is_file())
        .unwrap_or_else(|| panic!("required test shell {name} was not found on PATH"));
    std::fs::canonicalize(path).expect("resolve absolute test shell before changing child PATH")
}

fn powershell_roundtrip(powershell: &Path) {
    let env = fixture();
    let exe = std::env::current_exe().expect("test executable");
    #[cfg(windows)]
    let (separator, inherited, expected) = (
        ';',
        [
            None,
            Some(""),
            Some(r"C:\old one;D:\old two"),
            Some(r"C:\old ' “ & $HOME `tick`;D:\two"),
        ],
        vec![
            ("PATH", r"C:\SDK tools;C:\SDK runtime"),
            ("INCLUDE", r"C:\SDK include"),
            ("LIB", r"C:\SDK lib"),
            ("LIBPATH", r"C:\SDK assemblies"),
        ],
    );
    #[cfg(not(windows))]
    let (separator, inherited, expected) = (
        ':',
        [
            None,
            Some(""),
            Some("/old/one:/old/two"),
            Some("/old/'quote:$HOME:$(false):`tick`"),
        ],
        vec![
            ("PATH", "/sdk/tools"),
            ("CPATH", "/sdk/include"),
            ("LIBRARY_PATH", "/sdk/lib"),
            ("LD_LIBRARY_PATH", "/sdk/runtime"),
        ],
    );
    for old in inherited {
        let scratch = Scratch::new();
        let output = scratch.0.join("dump.json");
        let mut script = fmt_ps(&env).expect("format PowerShell");
        if std::env::var_os("VCV_TEST_QUOTE_MUTANT").is_some() {
            script = script.replace("''", "'");
        }
        script.push_str(&format!(
            "\n& {} --ignored --exact dump_environment --nocapture\nexit $LASTEXITCODE\n",
            ps_quote(exe.to_str().expect("unicode executable"))
        ));
        let mut command = Command::new(powershell);
        command
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
            ])
            .arg(script);
        child_command(&mut command, &output, old);
        let result = command.output().expect("run PowerShell");
        assert!(
            result.status.success(),
            "PowerShell failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let values = read_dump(&output);
        let suffix = old
            .filter(|value| !value.is_empty())
            .map(|value| format!("{separator}{value}"))
            .unwrap_or_default();
        for (key, prefix) in &expected {
            assert_eq!(values[*key], format!("{prefix}{suffix}"), "{key}");
        }
        assert_eq!(
            values["VCV_LITERAL"],
            env.vars["VCV_LITERAL"].to_str().expect("unicode literal")
        );
    }
}

#[cfg(windows)]
#[test]
fn powershell_shell_roundtrip() {
    let system_root = std::env::var_os("SystemRoot").expect("Windows system root");
    let powershell =
        PathBuf::from(system_root).join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
    powershell_roundtrip(&powershell);
}

#[cfg(any(windows, target_os = "linux"))]
#[test]
#[ignore = "requires PowerShell Core (pwsh) on PATH; run this named gate explicitly with --ignored"]
fn powershell_core_shell_roundtrip() {
    let powershell = required_shell(if cfg!(windows) { "pwsh.exe" } else { "pwsh" });
    powershell_roundtrip(&powershell);
}

#[cfg(windows)]
#[test]
#[ignore = "requires Git for Windows on PATH; run this named gate explicitly with --ignored"]
fn git_bash_shell_roundtrip() {
    let git = required_shell("git.exe");
    let dir = git.parent().expect("Git executable parent");
    let shell = [
        dir.join("sh.exe"),
        dir.join("../bin/sh.exe"),
        dir.join("../../bin/sh.exe"),
    ]
    .into_iter()
    .find(|path| path.is_file())
    .expect("Git for Windows sh.exe is required");
    let shell = std::fs::canonicalize(shell).expect("absolute Git Bash executable");
    let mut env = fixture();
    env.vars.insert("VCV_EMPTY".into(), OsString::new());
    for old in [
        None,
        Some(""),
        Some("/old/one:/old/two"),
        Some("/old/'quote:$HOME:$(false):`tick`"),
    ] {
        let mut script = fmt_sh(&env).expect("format Git Bash");
        if std::env::var_os("VCV_TEST_QUOTE_MUTANT").is_some() {
            script = script.replace("'\"'\"'", "'");
        }
        script = format!("{}{script}", sh_inheritance(old));
        // Read shell variables with a builtin before MSYS converts them for a Windows child.
        script.push('\n');
        script.push_str(r#"printf '%s\0' "$PATH" "$INCLUDE" "$LIB" "$LIBPATH" "$VCV_LITERAL" "${VCV_EMPTY+x}" "$VCV_EMPTY""#);
        let mut command = Command::new(&shell);
        command.arg("-c").arg(script);
        child_command(&mut command, Path::new("unused-shell-side-dump"), old);
        let result = command.output().expect("run Git Bash");
        assert!(
            result.status.success(),
            "Git Bash failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let values: Vec<&str> = result
            .stdout
            .split(|byte| *byte == 0)
            .map(|value| std::str::from_utf8(value).expect("UTF-8 shell-side dump"))
            .collect();
        let suffix = old
            .filter(|value| !value.is_empty())
            .map(|value| format!(":{value}"))
            .unwrap_or_default();
        let native_suffix = old
            .filter(|value| !value.is_empty())
            .map(|value| format!(";{value}"))
            .unwrap_or_default();
        assert_eq!(
            values,
            vec![
                format!("/c/SDK tools:/c/SDK runtime{suffix}"),
                format!(r"C:\SDK include{native_suffix}"),
                format!(r"C:\SDK lib{native_suffix}"),
                format!(r"C:\SDK assemblies{native_suffix}"),
                env.vars["VCV_LITERAL"]
                    .to_str()
                    .expect("unicode literal")
                    .to_owned(),
                "x".to_owned(),
                String::new(),
                String::new(),
            ]
        );
    }
}
