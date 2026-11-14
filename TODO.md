# vcv-rs — work queue

Updated: 2026-11-13.

## Completed (2026-11-13)

- [x] D11: Target-aware Windows/Linux discovery (`cuda.rs`, `optix.rs`, `nvsdk.rs`, `discovery.rs`), passive COFF/ELF/PE native binary/archive checks (`object =0.40.0`), and `ID 226` formatted.
- [x] D12: Windows PowerShell 5.1 32,766 UTF-16 code-unit limit + apply-time preflight before `SetEnvironmentVariable` in `src/format.rs::fmt_ps`; `CompareStringOrdinal` key-alias checks; CMD/PS/POSIX/JSON roundtrips + seeded mutant verified on Windows.
- [x] D13: Common `Arch` mapping, optional CLI feature gating, and Windows `vcv-rs -l` smoke verified against installed SDKs (CUDA 13.3, OptiX 9.1.0, cuDNN 9.23.0, Video Codec 13.1, Optical Flow 5.0, NVAPI 62115, NGX, nvCOMP 6.0.0, vcpkg).
- [x] `cargo +1.98.0 clippy --all-targets -- -D warnings` clean; 117 tests + `pwsh` + `git-bash` roundtrips + quote mutant gate green.

## Optional Follow-Up

- [ ] Run Linux native `sh_shell_roundtrip` and real COFF/ELF archive env gate (`VCV_TEST_COFF_ARCHIVE` / `VCV_TEST_ELF_ARCHIVE`) on a Linux/WSL runner when convenient.
