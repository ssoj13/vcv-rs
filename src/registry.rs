//! # Windows Registry Helpers Module
//!
//! Low-level Windows registry access for SDK/UCRT path detection.
//!
//! ## Purpose
//! Reads installation paths from Windows registry without external dependencies.
//! Searches across HKLM/HKCU and Wow6432Node for compatibility with 32/64-bit.
//!
//! ## Key Functions
//! - `reg_val()` - Read single registry value from specific key
//! - `reg_find()` - Search value across HKLM/HKCU and Wow6432Node variants
//!
//! ## Registry Paths Used
//! - `Microsoft\Microsoft SDKs\Windows\v10.0` - Windows SDK location
//! - `Microsoft\Windows Kits\Installed Roots` - UCRT location
//!
//! ## Dependencies
//! - `winreg` crate for Windows registry API

use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
use winreg::RegKey;

/// Read single registry value
pub fn reg_val(root: &RegKey, path: &str, name: &str) -> Option<String> {
    root.open_subkey(path)
        .ok()
        .and_then(|k| k.get_value::<String, _>(name).ok())
}

/// Search registry value across HKLM/HKCU and Wow6432Node
pub fn reg_find(path: &str, name: &str) -> Option<String> {
    let roots = [
        RegKey::predef(HKEY_LOCAL_MACHINE),
        RegKey::predef(HKEY_CURRENT_USER),
    ];
    let prefixes = [r"SOFTWARE\Wow6432Node", r"SOFTWARE"];

    for root in &roots {
        for prefix in &prefixes {
            let full_path = format!(r"{}\{}", prefix, path);
            if let Some(val) = reg_val(root, &full_path, name) {
                return Some(val);
            }
        }
    }
    None
}
