# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2026-10-25

### Added

- CUDA Toolkit detection for any major version, folded into `PATH`, `INCLUDE`, `LIB`, `CUDA_PATH`, `CUDA_HOME` and `CUDA_PATH_V<major>_<minor>`.
- Visual Studio selection by year, edition and prerelease channel (`--vs-min`, `--vs-max`, `--edition`, `--prerelease`), including VS 2026.
- vcpkg: when `VCPKG_ROOT` is set, `installed/<triplet>` libs, headers and binaries are added to the environment.
- `ProgramFiles(x86)`, `ProgramFiles` and `ProgramW6432` are restored when unset and the directory exists, so tools such as vcpkg can find `vswhere.exe`.

### Changed

- The CLI binary is `vcv-rs`, the same name as the crate.
- `VisualStudioVersion` comes from the selected install instead of a fixed `17.0`.
- `winreg` 0.56 (`windows-sys` 0.61).

### Fixed

- A shell that dropped `ProgramFiles(x86)` no longer looks, to vcpkg, like a machine with no Visual Studio.

## [0.1.0] - 2025-12-05

### Added

- Initial release: fast Visual Studio environment setup for Windows.

[0.1.1]: https://github.com/ssoj13/vcv-rs/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/ssoj13/vcv-rs/releases/tag/v0.1.0
