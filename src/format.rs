//! Validated native environment output for CMD, PowerShell, POSIX shells, and JSON.
//!
//! Executable and runtime paths share PATH on Windows. Linux keeps runtime
//! libraries in LD_LIBRARY_PATH. Formatting rejects values a shell cannot preserve.

use crate::env::Env;
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::Path;

/// An environment cannot be represented safely in the requested output.
#[derive(Debug)]
pub enum FormatError {
    Unsupported(&'static str),
    NonUnicode(String),
    InvalidKey(String),
    InvalidValue { key: String, reason: &'static str },
    Json(serde_json::Error),
    WindowsKeyComparison(std::io::Error),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(reason) => write!(f, "unsupported environment output: {reason}"),
            Self::NonUnicode(key) => write!(f, "{key} contains a non-Unicode value"),
            Self::InvalidKey(key) => write!(f, "invalid or conflicting environment key {key:?}"),
            Self::InvalidValue { key, reason } => {
                write!(f, "{key} cannot be represented: {reason}")
            }
            Self::Json(error) => write!(f, "cannot serialize environment: {error}"),
            Self::WindowsKeyComparison(error) => {
                write!(f, "cannot compare Windows environment keys: {error}")
            }
        }
    }
}

impl std::error::Error for FormatError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            Self::WindowsKeyComparison(error) => Some(error),
            _ => None,
        }
    }
}

fn invalid(key: &str, reason: &'static str) -> FormatError {
    FormatError::InvalidValue {
        key: key.into(),
        reason,
    }
}

fn unicode<'a>(value: &'a OsStr, key: &str) -> Result<&'a str, FormatError> {
    value
        .to_str()
        .ok_or_else(|| FormatError::NonUnicode(key.into()))
}

fn validate_text(value: &str, key: &str) -> Result<(), FormatError> {
    if value.chars().any(char::is_control) {
        return Err(invalid(key, "control characters are not allowed"));
    }
    Ok(())
}

fn validate_key(key: &str) -> Result<(), FormatError> {
    if key.is_empty() || key.contains('=') || key.chars().any(char::is_control) {
        return Err(FormatError::InvalidKey(key.into()));
    }
    Ok(())
}

fn dedup(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

fn path_values(
    paths: &[std::path::PathBuf],
    key: &str,
    separator: char,
) -> Result<Vec<String>, FormatError> {
    let mut values = Vec::new();
    for path in paths {
        let value = unicode(path.as_os_str(), key)?;
        validate_text(value, key)?;
        if value.is_empty() {
            return Err(invalid(key, "empty path component"));
        }
        if value.contains(separator) {
            return Err(invalid(key, "path contains the native list separator"));
        }
        values.push(value.into());
    }
    Ok(dedup(values))
}

fn same_key(left: &str, right: &str, windows: bool) -> Result<bool, FormatError> {
    if !windows {
        return Ok(left == right);
    }
    if left.is_ascii() && right.is_ascii() {
        return Ok(left.eq_ignore_ascii_case(right));
    }
    #[cfg(windows)]
    {
        windows_key_eq(left, right)
    }
    #[cfg(not(windows))]
    {
        Err(FormatError::Unsupported(
            "non-ASCII Windows key comparison requires Windows",
        ))
    }
}

#[cfg(windows)]
fn windows_key_eq(left: &str, right: &str) -> Result<bool, FormatError> {
    use windows_sys::Win32::Foundation::TRUE;
    use windows_sys::Win32::Globalization::{CSTR_EQUAL, CompareStringOrdinal};
    let left: Vec<u16> = left.encode_utf16().collect();
    let right: Vec<u16> = right.encode_utf16().collect();
    let left_len = i32::try_from(left.len()).map_err(|_| {
        invalid(
            "environment key",
            "UTF-16 key exceeds Windows comparison length",
        )
    })?;
    let right_len = i32::try_from(right.len()).map_err(|_| {
        invalid(
            "environment key",
            "UTF-16 key exceeds Windows comparison length",
        )
    })?;
    // SAFETY: Both UTF-16 buffers remain live for the call and each checked length
    // names exactly its buffer's initialized elements. Explicit counts need no NUL.
    let result =
        unsafe { CompareStringOrdinal(left.as_ptr(), left_len, right.as_ptr(), right_len, TRUE) };
    if result == 0 {
        return Err(FormatError::WindowsKeyComparison(
            std::io::Error::last_os_error(),
        ));
    }
    Ok(result == CSTR_EQUAL)
}

struct NativeEnv {
    lists: Vec<(&'static str, Vec<String>)>,
    vars: Vec<(String, String)>,
    separator: char,
}

fn native_env(env: &Env, windows: bool) -> Result<NativeEnv, FormatError> {
    if !windows && !env.libpath.is_empty() {
        return Err(FormatError::Unsupported("LIBPATH has no Linux equivalent"));
    }
    let separator = if windows { ';' } else { ':' };
    let mut path = path_values(&env.path, "PATH", separator)?;
    let runtime = path_values(
        &env.runtime,
        if windows { "PATH" } else { "LD_LIBRARY_PATH" },
        separator,
    )?;
    let lists = if windows {
        path.extend(runtime);
        vec![
            ("PATH", dedup(path)),
            ("INCLUDE", path_values(&env.include, "INCLUDE", separator)?),
            ("LIB", path_values(&env.lib, "LIB", separator)?),
            ("LIBPATH", path_values(&env.libpath, "LIBPATH", separator)?),
        ]
    } else {
        vec![
            ("PATH", path),
            ("CPATH", path_values(&env.include, "CPATH", separator)?),
            (
                "LIBRARY_PATH",
                path_values(&env.lib, "LIBRARY_PATH", separator)?,
            ),
            ("LD_LIBRARY_PATH", runtime),
        ]
    };
    let mut keys = Vec::new();
    let mut vars = Vec::new();
    for (key, value) in &env.vars {
        validate_key(key)?;
        for previous in lists
            .iter()
            .map(|(reserved, _)| *reserved)
            .chain(keys.iter().copied())
        {
            if same_key(previous, key, windows)? {
                return Err(FormatError::InvalidKey(key.clone()));
            }
        }
        keys.push(key.as_str());
        let value = unicode(value, key)?;
        validate_text(value, key)?;
        vars.push((key.clone(), value.into()));
    }
    Ok(NativeEnv {
        lists,
        vars,
        separator,
    })
}

fn native_platform() -> Result<bool, FormatError> {
    if cfg!(windows) {
        Ok(true)
    } else if cfg!(target_os = "linux") {
        Ok(false)
    } else {
        Err(FormatError::Unsupported(
            "only Windows and Linux are supported",
        ))
    }
}

/// Format literal CMD assignments using the generator process's inherited lists.
///
/// The snapshot is taken when rendering; later apply-time list values are replaced.
/// New entries precede inherited entries, with exact-string duplicates removed in
/// first-occurrence order. Unset or empty inherited lists add no separator. Empty
/// components already present in a nonempty inherited list retain their first
/// occurrence (for example, `A;;B;` contributes `A;;B`). Non-ASCII and CMD
/// expansion syntax are rejected.
pub fn fmt_cmd(env: &Env) -> Result<String, FormatError> {
    fmt_cmd_with(env, cfg!(windows), |key| std::env::var_os(key))
}

fn fmt_cmd_with(
    env: &Env,
    windows: bool,
    mut existing: impl FnMut(&str) -> Option<OsString>,
) -> Result<String, FormatError> {
    if !windows {
        return Err(FormatError::Unsupported("CMD requires Windows"));
    }
    let native = native_env(env, true)?;
    let mut lines = Vec::new();
    for (key, values) in native.lists {
        if values.is_empty() {
            continue;
        }
        let mut combined = values;
        if let Some(value) = existing(key) {
            let value = unicode(&value, key)?;
            if !value.is_empty() {
                combined.extend(value.split(';').map(str::to_owned));
            }
        }
        lines.push(cmd_assignment(key, &dedup(combined).join(";"))?);
    }
    for (key, value) in native.vars {
        lines.push(cmd_assignment(&key, &value)?);
    }
    Ok(lines.join("\r\n"))
}

fn cmd_assignment(key: &str, value: &str) -> Result<String, FormatError> {
    if value.is_empty() {
        return Err(invalid(
            key,
            "CMD deletes empty scalar variables instead of preserving them",
        ));
    }
    for text in [key, value] {
        if !text.is_ascii()
            || text
                .chars()
                .any(|ch| ch.is_control() || matches!(ch, '"' | '%' | '!'))
        {
            return Err(invalid(
                key,
                "CMD requires ASCII without controls, quotes, percent signs, or exclamation marks",
            ));
        }
    }
    let line = format!("set \"{key}={value}\"");
    if line.len() > 8191 {
        return Err(invalid(key, "CMD assignment exceeds 8191 characters"));
    }
    Ok(line)
}

fn ps_quote(value: &str) -> String {
    let mut quoted = String::from("'");
    for ch in value.chars() {
        quoted.push(ch);
        if matches!(ch, '\'' | '‘' | '’') {
            quoted.push(ch);
        }
    }
    quoted.push('\'');
    quoted
}

/// Maximum UTF-16 code units accepted by Windows PowerShell 5.1 / .NET Framework
/// `Environment.SetEnvironmentVariable` for both variable names and values.
const WIN_PS_MAX_UTF16: usize = 32_766;

fn check_win_ps_utf16(key: &str, value: &str, reason: &'static str) -> Result<(), FormatError> {
    if value.encode_utf16().count() > WIN_PS_MAX_UTF16 {
        return Err(invalid(key, reason));
    }
    Ok(())
}

/// Format Process-scope environment assignments for PowerShell.
/// Empty scalar values are rejected so output remains lossless on Windows PowerShell 5.1.
pub fn fmt_ps(env: &Env) -> Result<String, FormatError> {
    fmt_ps_for(env, native_platform()?)
}

fn fmt_ps_for(env: &Env, windows: bool) -> Result<String, FormatError> {
    let native = native_env(env, windows)?;
    let mut preflight = Vec::new();
    let mut setters = Vec::new();
    for (idx, (key, values)) in native.lists.into_iter().enumerate() {
        if values.is_empty() {
            continue;
        }
        let joined = values.join(&native.separator.to_string());
        if windows {
            check_win_ps_utf16(
                key,
                &joined,
                "Windows PowerShell 5.1 limits environment values to 32766 UTF-16 code units",
            )?;
        }
        let q_key = ps_quote(key);
        let prefix = ps_quote(&joined);
        let separator = ps_quote(&native.separator.to_string());
        let get = format!("[Environment]::GetEnvironmentVariable({q_key}, 'Process')");
        if windows {
            let var = format!("$vcv_{idx}");
            let err_msg = ps_quote(&format!(
                "{key} exceeds Windows PowerShell 32766 UTF-16 code unit limit"
            ));
            preflight.push(format!(
                "{var} = ({prefix} + $(if ([string]::IsNullOrEmpty({get})) {{ [string]::Empty }} else {{ {separator} + {get} }}))"
            ));
            preflight.push(format!(
                "if ({var}.Length -gt {WIN_PS_MAX_UTF16}) {{ throw [System.ArgumentException]::new({err_msg}) }}"
            ));
            setters.push(format!(
                "[Environment]::SetEnvironmentVariable({q_key}, {var}, 'Process')"
            ));
        } else {
            setters.push(format!(
                "[Environment]::SetEnvironmentVariable({q_key}, ({prefix} + $(if ([string]::IsNullOrEmpty({get})) {{ [string]::Empty }} else {{ {separator} + {get} }})), 'Process')"
            ));
        }
    }
    for (key, value) in native.vars {
        if value.is_empty() {
            return Err(invalid(
                &key,
                "Windows PowerShell 5.1 deletes empty scalar variables",
            ));
        }
        if windows {
            check_win_ps_utf16(
                &key,
                &key,
                "Windows PowerShell 5.1 limits environment names to 32766 UTF-16 code units",
            )?;
            check_win_ps_utf16(
                &key,
                &value,
                "Windows PowerShell 5.1 limits environment values to 32766 UTF-16 code units",
            )?;
        }
        setters.push(format!(
            "[Environment]::SetEnvironmentVariable({}, {}, 'Process')",
            ps_quote(&key),
            ps_quote(&value)
        ));
    }
    if preflight.is_empty() {
        Ok(setters.join("\n"))
    } else {
        let mut body = preflight;
        body.extend(setters);
        Ok(format!("& {{\n    {}\n}}", body.join("\n    ")))
    }
}

/// Convert an absolute Windows drive path to its MSYS/Git Bash spelling.
fn win_to_unix(path: &Path) -> Result<String, FormatError> {
    let value = unicode(path.as_os_str(), "PATH")?;
    let mut chars = value.chars();
    let first = chars.next();
    let second = chars.next();
    let result = match (first, second) {
        (Some(drive), Some(':')) if drive.is_ascii_alphabetic() => {
            let rest: String = chars.collect();
            if !rest.starts_with(['\\', '/']) {
                return Err(invalid("PATH", "drive-relative Windows path"));
            }
            format!("/{}{}", drive.to_ascii_lowercase(), rest.replace('\\', "/"))
        }
        _ => value.replace('\\', "/"),
    };
    if result.is_empty() || result.contains(':') {
        return Err(invalid("PATH", "empty or colon-containing shell path"));
    }
    Ok(result)
}

fn sh_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn sh_key(key: &str) -> bool {
    let mut chars = key.chars();
    matches!(chars.next(), Some(ch) if ch.is_ascii_alphabetic() || ch == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

/// Format POSIX exports, with drive conversion only for Windows PATH.
pub fn fmt_sh(env: &Env) -> Result<String, FormatError> {
    let windows = native_platform()?;
    let native = native_env(env, windows)?;
    let mut lines = Vec::new();
    for (key, mut values) in native.lists {
        if values.is_empty() {
            continue;
        }
        let separator = if windows && key == "PATH" {
            values = values
                .iter()
                .map(|value| win_to_unix(Path::new(value)))
                .collect::<Result<Vec<_>, _>>()?;
            values = dedup(values);
            ':'
        } else {
            native.separator
        };
        lines.push(format!(
            r#"export {key}={}"${{{key}:+{separator}}}${{{key}-}}""#,
            sh_quote(&values.join(&separator.to_string()))
        ));
    }
    for (key, value) in native.vars {
        if !sh_key(&key) {
            return Err(FormatError::InvalidKey(key));
        }
        lines.push(format!("export {key}={}", sh_quote(&value)));
    }
    Ok(lines.join("\n"))
}

/// Serialize native path arrays and raw scalar variables without collisions.
pub fn fmt_json(env: &Env) -> Result<String, FormatError> {
    let native = native_env(env, native_platform()?)?;
    let mut map = serde_json::Map::new();
    for (key, values) in native.lists {
        map.insert(
            key.into(),
            serde_json::Value::Array(values.into_iter().map(serde_json::Value::String).collect()),
        );
    }
    for (key, value) in native.vars {
        map.insert(key, serde_json::Value::String(value));
    }
    serde_json::to_string_pretty(&map).map_err(FormatError::Json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Arch;
    use std::path::PathBuf;

    fn cmd_env() -> Env {
        let mut env = Env::new(Arch::X64);
        env.path.push(PathBuf::from(r"C:\new"));
        env
    }

    #[test]
    #[cfg(any(windows, target_os = "linux"))]
    fn empty_scalars_are_rejected_by_cmd_and_powershell_but_preserved_by_json_and_sh() {
        let mut env = Env::new(Arch::X64);
        env.vars.insert("VCV_EMPTY".into(), OsString::new());
        assert!(matches!(fmt_cmd_with(&env, true, |_| None),
            Err(FormatError::InvalidValue { key, .. }) if key == "VCV_EMPTY"));
        assert!(matches!(fmt_ps(&env),
            Err(FormatError::InvalidValue { key, .. }) if key == "VCV_EMPTY"));
        let json: serde_json::Value =
            serde_json::from_str(&fmt_json(&env).expect("empty JSON scalar")).expect("parse JSON");
        assert_eq!(
            json.get("VCV_EMPTY"),
            Some(&serde_json::Value::String(String::new()))
        );
        assert_eq!(
            fmt_sh(&env).expect("empty POSIX scalar"),
            "export VCV_EMPTY=''"
        );
    }

    #[test]
    fn cmd_snapshot_handles_empty_and_deduplicates_existing() {
        let mut env = cmd_env();
        env.runtime
            .extend([PathBuf::from(r"C:\new"), PathBuf::from(r"C:\runtime")]);
        for existing in [None, Some(OsString::new())] {
            let output =
                fmt_cmd_with(&env, true, |_| existing.clone()).expect("empty inherited PATH");
            assert_eq!(output, r#"set "PATH=C:\new;C:\runtime""#);
        }
        let output = fmt_cmd_with(&env, true, |_| {
            Some(OsString::from(r"C:\old;C:\new;C:\old"))
        })
        .expect("ordinary inherited PATH");
        assert_eq!(output, r#"set "PATH=C:\new;C:\runtime;C:\old""#);
        assert!(!output.contains("%PATH%"));
        let output = fmt_cmd_with(&env, true, |_| Some(OsString::from("A;;B;")))
            .expect("inherited empty components");
        assert_eq!(output, r#"set "PATH=C:\new;C:\runtime;A;;B""#);
    }

    #[test]
    fn cmd_rejects_unsafe_inherited_snapshot() {
        let env = cmd_env();
        for value in [
            r"C:\percent%PATH%",
            r"C:\bang!",
            r#"C:\"quoted"#,
            "C:\\line\nnext",
            "C:\\café",
        ] {
            assert!(
                fmt_cmd_with(&env, true, |_| Some(OsString::from(value))).is_err(),
                "{value:?}"
            );
        }
        assert!(matches!(
            fmt_cmd_with(&env, false, |_| None),
            Err(FormatError::Unsupported(_))
        ));
    }

    #[test]
    fn cmd_quoted_metacharacters_and_program_files_key_are_literal() {
        let mut env = Env::new(Arch::X64);
        env.vars.insert(
            "ProgramFiles(x86)".into(),
            OsString::from(r"C:\a&b|c<d>e^f(g)"),
        );
        assert_eq!(
            fmt_cmd_with(&env, true, |_| None).expect("literal metacharacters"),
            r#"set "ProgramFiles(x86)=C:\a&b|c<d>e^f(g)""#
        );
    }

    #[test]
    fn cmd_line_length_includes_assignment_syntax() {
        let mut env = Env::new(Arch::X64);
        env.vars
            .insert("X".into(), OsString::from("a".repeat(8183)));
        assert_eq!(
            fmt_cmd_with(&env, true, |_| None)
                .expect("8191 characters")
                .len(),
            8191
        );
        env.vars
            .insert("X".into(), OsString::from("a".repeat(8184)));
        assert!(fmt_cmd_with(&env, true, |_| None).is_err());
    }

    #[test]
    fn drive_conversion_is_checked_and_unicode_safe() {
        for (input, expected) in [
            (r"C:\Tools\bin", "/c/Tools/bin"),
            ("D:/", "/d/"),
            (r"\\server\share\bin", "//server/share/bin"),
            (r"C:\café\工具", "/c/café/工具"),
            ("/already/native", "/already/native"),
        ] {
            assert_eq!(win_to_unix(Path::new(input)).expect("valid path"), expected);
        }
        for input in ["", "C:", "C:relative", "é:/foreign", "C:/colon:part"] {
            assert!(win_to_unix(Path::new(input)).is_err(), "{input}");
        }
    }

    #[test]
    fn powershell_doubles_ascii_and_smart_single_quotes() {
        assert_eq!(ps_quote(r#"a'b‘c’d`$()""#), r#"'a''b‘‘c’’d`$()"'"#);
    }

    #[test]
    fn native_mapping_separates_linux_runtime_and_rejects_libpath() {
        let mut env = Env::new(Arch::X64);
        env.path.push("/tools".into());
        env.runtime.push("/runtime".into());
        env.include.push("/headers".into());
        env.lib.push("/libraries".into());
        let native = native_env(&env, false).expect("Linux environment");
        assert_eq!(native.separator, ':');
        assert_eq!(
            native.lists,
            vec![
                ("PATH", vec!["/tools".to_string()]),
                ("CPATH", vec!["/headers".to_string()]),
                ("LIBRARY_PATH", vec!["/libraries".to_string()]),
                ("LD_LIBRARY_PATH", vec!["/runtime".to_string()]),
            ]
        );
        env.libpath.push("/metadata".into());
        assert!(matches!(
            native_env(&env, false),
            Err(FormatError::Unsupported(_))
        ));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn powershell_on_linux_uses_native_names_and_separator() {
        let mut env = Env::new(Arch::X64);
        env.include
            .extend([PathBuf::from("/one"), PathBuf::from("/two")]);
        env.runtime.push("/runtime".into());
        let output = fmt_ps(&env).expect("PowerShell for Linux");
        assert!(output.contains("'CPATH'"));
        assert!(output.contains("'/one:/two'"));
        assert!(output.contains("'LD_LIBRARY_PATH'"));
        assert!(!output.contains("'INCLUDE'"));
        assert!(output.contains("IsNullOrEmpty"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_unicode_key_aliases_are_rejected_without_merging_distinct_names() {
        assert!(
            !same_key("\u{131}", "I", true)
                .expect("Windows ordinal comparison does not fold dotless-i to ASCII I")
        );
        assert!(same_key("Ж", "ж", true).expect("Windows ordinal Cyrillic case fold"));
        assert!(
            !same_key("ß", "SS", true).expect("Windows ordinal comparison does not expand sharp-s")
        );
        let mut env = Env::new(Arch::X64);
        env.vars.insert("Ä".into(), "first".into());
        env.vars.insert("ä".into(), "second".into());
        assert!(matches!(fmt_json(&env), Err(FormatError::InvalidKey(_))));
        assert!(matches!(fmt_ps(&env), Err(FormatError::InvalidKey(_))));
        env.vars.remove("ä");
        env.vars.insert("Ö".into(), "distinct".into());
        let json: serde_json::Value =
            serde_json::from_str(&fmt_json(&env).expect("distinct Unicode names"))
                .expect("parse distinct names");
        assert_eq!(json["Ä"], "first");
        assert_eq!(json["Ö"], "distinct");
        let script = fmt_ps(&env).expect("PowerShell distinct names");
        assert!(script.contains("'Ä'") && script.contains("'Ö'"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_unicode_key_case_remains_distinct() {
        let mut env = Env::new(Arch::X64);
        env.vars.insert("Ä".into(), "first".into());
        env.vars.insert("ä".into(), "second".into());
        let json: serde_json::Value =
            serde_json::from_str(&fmt_json(&env).expect("Linux case-sensitive names"))
                .expect("parse distinct names");
        assert_eq!(json["Ä"], "first");
        assert_eq!(json["ä"], "second");
        assert!(fmt_ps(&env).is_ok());
        assert!(matches!(
            native_env(&env, true),
            Err(FormatError::Unsupported(_))
        ));
    }

    #[test]
    fn scalar_aliases_and_generated_keys_are_rejected_even_for_empty_lists() {
        for key in ["PATH", "Path", "include", "lib", "LIBPATH"] {
            let mut env = Env::new(Arch::X64);
            env.vars.insert(key.into(), "value".into());
            assert!(native_env(&env, true).is_err(), "{key}");
        }
        let mut env = Env::new(Arch::X64);
        env.vars.insert("Foo".into(), "first".into());
        env.vars.insert("FOO".into(), "second".into());
        assert!(native_env(&env, true).is_err());
        assert!(native_env(&env, false).is_ok());
    }

    #[test]
    fn keys_paths_and_control_values_have_explicit_errors() {
        for key in ["", "A=B", "A\0B", "A\nB"] {
            assert!(validate_key(key).is_err());
        }
        for key in ["_VALID_2", "PATH", "plain"] {
            assert!(sh_key(key));
        }
        for key in ["ProgramFiles(x86)", "A-B", "2BAD", ""] {
            assert!(!sh_key(key));
        }
        for value in ["", "/one:/two", "/line\nnext"] {
            let mut env = Env::new(Arch::X64);
            env.path.push(value.into());
            assert!(native_env(&env, false).is_err(), "{value:?}");
        }
        assert_eq!(sh_quote("a'b"), "'a'\"'\"'b'");
    }

    #[test]
    #[cfg(any(unix, windows))]
    fn non_unicode_paths_and_scalars_are_rejected_without_loss() {
        #[cfg(unix)]
        let raw = {
            use std::os::unix::ffi::OsStringExt;
            OsString::from_vec(vec![b'x', 0xff])
        };
        #[cfg(windows)]
        let raw = {
            use std::os::windows::ffi::OsStringExt;
            OsString::from_wide(&[b'x' as u16, 0xd800])
        };
        let mut env = Env::new(Arch::X64);
        env.path.push(PathBuf::from(raw.clone()));
        assert!(matches!(
            native_env(&env, cfg!(windows)),
            Err(FormatError::NonUnicode(_))
        ));
        env.path.clear();
        env.vars.insert("RAW".into(), raw);
        assert!(matches!(
            native_env(&env, cfg!(windows)),
            Err(FormatError::NonUnicode(_))
        ));
    }

    #[test]
    fn windows_powershell_enforces_utf16_name_value_and_list_limits() {
        let mut env = Env::new(Arch::X64);
        env.vars.insert(
            "VCV_MAX".into(),
            OsString::from("x".repeat(WIN_PS_MAX_UTF16)),
        );
        assert!(fmt_ps_for(&env, true).is_ok());

        env.vars.insert(
            "VCV_MAX".into(),
            OsString::from("x".repeat(WIN_PS_MAX_UTF16 + 1)),
        );
        assert!(matches!(
            fmt_ps_for(&env, true),
            Err(FormatError::InvalidValue { key, .. }) if key == "VCV_MAX"
        ));
        // Linux PowerShell Core has no 32766 UTF-16 SetEnvironmentVariable limit.
        assert!(fmt_ps_for(&env, false).is_ok());

        // Astral characters occupy 2 UTF-16 code units each: 16383 * 2 == 32766, +1 BMP overflows.
        env.vars.clear();
        let astral_exact = "🦀".repeat(WIN_PS_MAX_UTF16 / 2);
        env.vars
            .insert("VCV_ASTRAL".into(), OsString::from(astral_exact.clone()));
        assert!(fmt_ps_for(&env, true).is_ok());
        env.vars.insert(
            "VCV_ASTRAL".into(),
            OsString::from(format!("{astral_exact}a")),
        );
        assert!(fmt_ps_for(&env, true).is_err());

        // Key length boundary.
        env.vars.clear();
        env.vars
            .insert("K".repeat(WIN_PS_MAX_UTF16), OsString::from("ok"));
        assert!(fmt_ps_for(&env, true).is_ok());
        env.vars.clear();
        env.vars
            .insert("K".repeat(WIN_PS_MAX_UTF16 + 1), OsString::from("no"));
        assert!(fmt_ps_for(&env, true).is_err());

        // Joined list length boundary.
        env.vars.clear();
        env.path.push(PathBuf::from(format!(
            r"C:\{}",
            "p".repeat(WIN_PS_MAX_UTF16 - 3)
        )));
        assert!(fmt_ps_for(&env, true).is_ok());
        env.path.push(PathBuf::from(r"C:\extra"));
        assert!(matches!(
            fmt_ps_for(&env, true),
            Err(FormatError::InvalidValue { key, .. }) if key == "PATH"
        ));
    }
}
