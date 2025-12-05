//! # Output Formatters Module
//!
//! Converts assembled environment into shell-specific output formats.
//!
//! ## Purpose
//! Generates ready-to-execute shell commands for different environments:
//! - PowerShell: `$env:VAR = "value"`
//! - CMD: `set "VAR=value"`
//! - Bash/MSYS2: `export VAR="value"` (with path conversion)
//! - JSON: structured output for tooling
//!
//! ## Key Functions
//! - `fmt_ps()` - PowerShell format
//! - `fmt_cmd()` - CMD.exe format
//! - `fmt_sh()` - Bash/MSYS2 format (converts C:\ to /c/)
//! - `fmt_json()` - JSON format for programmatic use
//!
//! ## Dependencies
//! - `env::Env` struct with assembled paths
//! - `serde_json` for JSON serialization

use crate::env::Env;
use std::path::Path;

/// Format for cmd.exe
pub fn fmt_cmd(env: &Env) -> String {
    let mut lines = Vec::new();

    if !env.path.is_empty() {
        let paths: Vec<_> = env.path.iter().map(|p| p.display().to_string()).collect();
        lines.push(format!("set \"PATH={};%PATH%\"", paths.join(";")));
    }
    if !env.include.is_empty() {
        let paths: Vec<_> = env.include.iter().map(|p| p.display().to_string()).collect();
        lines.push(format!("set \"INCLUDE={};%INCLUDE%\"", paths.join(";")));
    }
    if !env.lib.is_empty() {
        let paths: Vec<_> = env.lib.iter().map(|p| p.display().to_string()).collect();
        lines.push(format!("set \"LIB={};%LIB%\"", paths.join(";")));
    }
    if !env.libpath.is_empty() {
        let paths: Vec<_> = env.libpath.iter().map(|p| p.display().to_string()).collect();
        lines.push(format!("set \"LIBPATH={};%LIBPATH%\"", paths.join(";")));
    }

    for (k, v) in &env.vars {
        lines.push(format!("set \"{}={}\"", k, v));
    }

    lines.join("\n")
}

/// Format for PowerShell
pub fn fmt_ps(env: &Env) -> String {
    let mut lines = Vec::new();

    if !env.path.is_empty() {
        let paths: Vec<_> = env.path.iter().map(|p| p.display().to_string()).collect();
        lines.push(format!("$env:PATH = \"{};$env:PATH\"", paths.join(";")));
    }
    if !env.include.is_empty() {
        let paths: Vec<_> = env.include.iter().map(|p| p.display().to_string()).collect();
        lines.push(format!("$env:INCLUDE = \"{};$env:INCLUDE\"", paths.join(";")));
    }
    if !env.lib.is_empty() {
        let paths: Vec<_> = env.lib.iter().map(|p| p.display().to_string()).collect();
        lines.push(format!("$env:LIB = \"{};$env:LIB\"", paths.join(";")));
    }
    if !env.libpath.is_empty() {
        let paths: Vec<_> = env.libpath.iter().map(|p| p.display().to_string()).collect();
        lines.push(format!("$env:LIBPATH = \"{};$env:LIBPATH\"", paths.join(";")));
    }

    for (k, v) in &env.vars {
        lines.push(format!("$env:{} = \"{}\"", k, v));
    }

    lines.join("\n")
}

/// Convert Windows path to MSYS2/bash path
fn win_to_unix(p: &Path) -> String {
    let s = p.display().to_string();
    if s.len() >= 2 && s.chars().nth(1) == Some(':') {
        let drive = s.chars().next().unwrap().to_lowercase();
        format!("/{}{}", drive, s[2..].replace('\\', "/"))
    } else {
        s.replace('\\', "/")
    }
}

/// Format for bash/MSYS2
pub fn fmt_sh(env: &Env) -> String {
    let mut lines = Vec::new();

    if !env.path.is_empty() {
        let paths: Vec<_> = env.path.iter().map(|p| win_to_unix(p)).collect();
        lines.push(format!("export PATH=\"{}:$PATH\"", paths.join(":")));
    }
    if !env.include.is_empty() {
        let paths: Vec<_> = env.include.iter().map(|p| p.display().to_string()).collect();
        lines.push(format!("export INCLUDE=\"{};$INCLUDE\"", paths.join(";")));
    }
    if !env.lib.is_empty() {
        let paths: Vec<_> = env.lib.iter().map(|p| p.display().to_string()).collect();
        lines.push(format!("export LIB=\"{};$LIB\"", paths.join(";")));
    }
    if !env.libpath.is_empty() {
        let paths: Vec<_> = env.libpath.iter().map(|p| p.display().to_string()).collect();
        lines.push(format!("export LIBPATH=\"{};$LIBPATH\"", paths.join(";")));
    }

    for (k, v) in &env.vars {
        lines.push(format!("export {}=\"{}\"", k, v));
    }

    lines.join("\n")
}

/// Format as JSON
pub fn fmt_json(env: &Env) -> String {
    let mut map = serde_json::Map::new();

    let path_arr: Vec<_> = env.path.iter().map(|p| serde_json::Value::String(p.display().to_string())).collect();
    map.insert("PATH".into(), serde_json::Value::Array(path_arr));

    let inc_arr: Vec<_> = env.include.iter().map(|p| serde_json::Value::String(p.display().to_string())).collect();
    map.insert("INCLUDE".into(), serde_json::Value::Array(inc_arr));

    let lib_arr: Vec<_> = env.lib.iter().map(|p| serde_json::Value::String(p.display().to_string())).collect();
    map.insert("LIB".into(), serde_json::Value::Array(lib_arr));

    let libpath_arr: Vec<_> = env.libpath.iter().map(|p| serde_json::Value::String(p.display().to_string())).collect();
    map.insert("LIBPATH".into(), serde_json::Value::Array(libpath_arr));

    for (k, v) in &env.vars {
        map.insert(k.clone(), serde_json::Value::String(v.clone()));
    }

    serde_json::to_string_pretty(&serde_json::Value::Object(map)).unwrap()
}
