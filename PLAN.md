kind:inline
text:# vcv-rs — Consolidated Plan & Architecture

Updated: 2026-11-14 | Baseline commit: `31c5428` (`main`)

## 1. Current Status

**Windows SDK discovery, environment assembly, formatters, and CLI are complete and verified on Windows (`rustc 1.98.0`).**

| Gate / Target | Status | Details |
|---|---|---|
| `cargo +1.98.0 fmt --all -- --check` | PASS | Verified on `main` (`31c5428`, 2026-11-13) |
| `cargo +1.98.0 clippy --all-targets -- -D warnings` | PASS | 0 warnings |
| `cargo +1.98.0 test` | PASS | `117` passed (`85` lib + `15` cli + `6` integration + `11` roundtrip), `0` failed |
| Native Windows shells | PASS | `powershell_core_shell_roundtrip`, `git_bash_shell_roundtrip`, `powershell_preflight_prevents_partial_mutation_when_inherited_list_overflows` |
| Seeded mutant (`VCV_TEST_QUOTE_MUTANT=1`) | PASS | Caught (`exit 101`) when set; green when unset |
| Live `vcv-rs -l` (`NV_SDK_DIRS=C:\projects\projects.rust.cg\nv`) | PASS | Detects VS 2026, WinSDK 26100, CUDA 13.3, OptiX 9.1.0, cuDNN 9.23.0, Video Codec 13.1, Optical Flow 5.0, NVAPI 62115, NGX 0x15, nvCOMP 6.0.0, vcpkg `x64-windows` |
| Linux / WSL2 optional gates | PENDING | `sh_shell_roundtrip`, `VCV_TEST_COFF_ARCHIVE`, `VCV_TEST_ELF_ARCHIVE` (non-blocking for Windows/`nv-rs`) |
| Native ARM64 hardware | NOT RUN | Covered by layout fixtures only |

---

## 2. Architecture & Domain Map

| Domain | Scope | Key Contracts |
|---|---|---|
| **SDK Discovery** (`cuda.rs`, `optix.rs`, `nvsdk.rs`, `discovery.rs`) | Authoritative explicit-root probing, target-aware layout resolution, passive COFF/ELF/PE arch inspection (`object =0.40.0`) | First nonempty root alias wins even if invalid (no silent fallback). `CudaInfo::lib_dir(target)` and `runtime_dir(target)` are independent. NVAPI is Windows-only; NCCL is Linux-only. Version macros return `Option<u32>`. |
| **Environment & Formatting** (`env.rs`, `format.rs`, `tests/format_roundtrip.rs`) | `Env::new(target)`, raw `OsString` storage, `build_env`, `add_*`, `fmt_{cmd,ps,sh,json}` | Separate exec (`PATH`), runtime (`PATH`/`LD_LIBRARY_PATH`), include (`INCLUDE`/`CPATH`), link (`LIB`/`LIBRARY_PATH`), and Windows `LIBPATH`. `fmt_ps` enforces the 32,766 UTF-16 code-unit `.NET SetEnvironmentVariable` limit + apply-time preflight. `fmt_cmd` snapshots inherited lists, deduplicates, and rejects `>`, `%`, `!`, `"`, non-ASCII, and >8,191 chars. |
| **CLI & Features** (`cli.rs`, `cli/{args,platform,sdks}.rs`, `main.rs`, `lib.rs`, `detect.rs`) | `vcv-rs` binary, optional `clap` (`cli` feature), `Arch` mapping (`x64`/`x86`/`arm64`) | Windows requires VS/MSVC (with CUDA `_MSC_VER` year hints); Linux builds native SDK envs without VS and rejects Windows-only/cross-target/CMD flags. Full rendered output is validated before printing to stdout. |

---

## 3. Seam Contracts (`V01`–`V08`)

| ID | Producer → Consumer | Contract |
|---|---|---|
| `V01` | `cuda.rs` / `optix.rs` (`requested_root`, `detect_*`) → CLI / discovery | First nonempty configured root is authoritative (even when invalid); inventory sorting never overrides it. |
| `V02` | `cuda.rs` (`probe`, `lib_dir`, `runtime_dir`) + `discovery.rs` (`binary_arch`) → `env.rs::add_cuda` | Version, target, and native binary evidence agree; static-only archives never advertise a runtime dir. |
| `V03` | `nvsdk.rs::detect(kind, target)` / `optix.rs::detect_optix` → `env.rs::add_nvsdk` / `nv-rs` | Single owner selects headers, target, `runtime_dir`, and `lib_dir`; versioned runtime sonames (`.so.2`) do not imply link artifacts. |
| `V04` | Selected SDK / `VcpkgInfo` → `env.rs::add_*` | Preserves raw `OsString` values, target identity, chosen aliases, and `VCPKG_DEFAULT_TRIPLET`. |
| `V05` | `env.rs::Env` → `format.rs::fmt_*` | Native variable names/separators, literal quoting, typed rejection; CMD snapshots at render time vs. PS/POSIX merge at apply time. |
| `V06` | `lib.rs::Arch` + Cargo features → `main.rs` / `cli` | `cli`, `cuda`, `optix`, `nvsdk` are independently gateable (`--no-default-features` builds cleanly); unsupported explicit requests fail fast. |
| `V07` | Pushed `vcv-rs` `main` → `nv-rs` | Downstream `nv-rs` consumes `vcv-rs` with `default-features = false, features = ["cuda", "optix", "nvsdk"]`. |
| `V08` | Source + fixtures → Platform CI receipts | Windows verified (`31c5428`); Linux/WSL optional gates tracked in §4. |

---

## 4. Remaining Optional Follow-Ups

- [ ] Run Linux/WSL2 native `sh_shell_roundtrip` and real archive env gates (`VCV_TEST_COFF_ARCHIVE`, `VCV_TEST_ELF_ARCHIVE`) on a Linux runner.
- [ ] Run full 16-combination Cargo feature-powerset check on Linux CI if cross-platform matrix certification is needed.
