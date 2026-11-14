# vcv-rs

> *"A human being should be able to change a diaper, plan an invasion, butcher a hog, conn a ship, design a building, write a sonnet, balance accounts, build a wall, set a bone, comfort the dying, take orders, give orders, cooperate, act alone, solve equations, analyze a new problem, pitch manure, program a computer, cook a tasty meal, fight efficiently, die gallantly. Specialization is for insects."*
> — Robert A. Heinlein, *Time Enough for Love*

An engineer who lets a thousand-line batch file guess where his compiler lives deserves whatever linker error he gets. `vcv-rs` does not guess. It inspects the machine—Windows registry and `vswhere.exe` when MSVC is needed, native filesystem layouts on Linux—verifies the actual binaries on disk, and hands you a clean, quoted, validated environment. If an explicit root you gave it is wrong, it tells you so to your face instead of silently falling back to whatever garbage happened to be lying around on `PATH`.

`vcv-rs` ships both the `vcv_rs` library crate and the `vcv-rs` (`vcv-rs.exe`) CLI. Default features are `cli`, `cuda`, `optix`, and `nvsdk`.

---

## 1. Library Usage (`vcv_rs`)

Depend on it straight from `main`:

```toml
[dependencies]
vcv-rs = { git = "ssh://git@github.com/ssoj13/vcv-rs.git", branch = "main", default-features = false, features = ["cuda", "optix", "nvsdk"] }
```

`Arch`, `discovery`, `env`, and `format` are cross-platform. `detect` and `registry` are Windows-only. `Arch::from_target_arch` maps Rust's `x86_64`, `x86`, and `aarch64` to `X64`, `X86`, and `Arm64`; `Arch::native()` gives you the compiled target's architecture, or `None` if your hardware isn't on the roster.

Building an SDK environment without MSVC:

```rust
use vcv_rs::{Arch, Env, env, fmt_ps};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let target = Arch::native()
        .ok_or_else(|| std::io::Error::other("unsupported native architecture"))?;
    let mut environment = Env::new(target);
    if let Some(cuda) = vcv_rs::cuda::detect_cuda() {
        env::add_cuda(&mut environment, &cuda)?;
    }
    if let Some(optix) = vcv_rs::optix::detect_optix() {
        env::add_optix(&mut environment, &optix);
    }
    let script = fmt_ps(&environment)?;
    print!("{script}");
    Ok(())
}
```

When you actually need a Windows compiler, supply explicit host and target architectures:

```rust
#[cfg(windows)]
fn msvc_environment(
    host: vcv_rs::Arch,
    target: vcv_rs::Arch,
) -> Result<vcv_rs::Env, Box<dyn std::error::Error>> {
    let vs = vcv_rs::detect::detect_vs(None)
        .ok_or_else(|| std::io::Error::other("Visual Studio C++ tools not found"))?;
    let sdk = vcv_rs::detect::detect_sdk();
    let ucrt = vcv_rs::detect::detect_ucrt();
    Ok(vcv_rs::env::build_env(&vs, sdk.as_ref(), ucrt.as_ref(), host, target))
}
```

**Design rules worth knowing:**
- `add_cuda`, `add_nvsdk`, and `add_vcpkg` return `Result<(), EnvError>`. `add_optix` wires headers and root aliases infallibly.
- Paths and scalar values stay raw `PathBuf`/`OsString` until formatting. Non-Unicode junk is rejected cleanly rather than mangled with replacement characters.
- Call `requested_root()` alongside a missing detection result to diagnose an invalid explicit environment override.

---

## 2. The CLI (`vcv-rs`)

```sh
cargo build --release
cargo install --path .
```

### Applying an Environment

Informational chatter goes to `stderr`; `stdout` gets the finished script or JSON—and **only** after every validation check passes.

```powershell
# PowerShell: render first, verify exit code, then apply.
$script = vcv-rs -f ps -q
if ($LASTEXITCODE -eq 0) { ($script -join "`n") | Invoke-Expression }

# JSON for scripts and tooling
vcv-rs -f json -q | ConvertFrom-Json
```

```cmd
:: CMD: captures this process's inherited lists at render time.
vcv-rs -f cmd > vcenv.bat && call vcenv.bat
```

```sh
# Linux / Bash / MSYS2: keep your quotes intact.
script=$(vcv-rs -f sh -q) && eval "$script"
```

### Options

| Option | Behavior |
|---|---|
| `-a, --arch` / `-s, --host` | Target / host architecture: `x64`, `x86`, `arm64` (default: native) |
| `-f, --format` | `auto`, `ps`/`powershell`, `cmd`, `sh`/`bash`, `json` |
| `-v, --vs` | Exact Windows VS year: `2017`, `2019`, `2022`, `2026` |
| `--vs-min` / `--vs-max` | Windows VS year bounds (conflicts with `--vs`) |
| `-e, --edition` | Windows: `community`, `professional`, `enterprise`, `buildtools` |
| `--prerelease` | Windows: `exclude` (default), `allow`, `only` |
| `-c, --cuda` / `-o, --optix` | `auto` (default), `on` (require), `off` (ignore); feature-gated |
| `--no-nvsdk` | Omit separately shipped NVIDIA SDKs and ignore their root overrides |
| `-l, --list` | Inventory discovered installations and mark selected roots |
| `-q, --quiet` | Suppress informational `stderr` output |
| `--no-validate` | Windows: skip checking the selected host/target `cl.exe` |
| `-h, --help` | Print help and examples |

Linux runs native host/target setups without Visual Studio. Ask Linux for Windows-only flags, cross-target builds, or CMD output and `vcv-rs` will refuse politely and exit non-zero.

### Toolchain Selection Rules

```sh
vcv-rs --list
vcv-rs --vs 2022 --edition buildtools
vcv-rs --vs-min 2022
vcv-rs --prerelease only
```

- **Explicit roots are law.** The first nonempty configured SDK root alias is authoritative, even if broken. `vcv-rs` will never quietly swap it for a newer install it found in `Program Files`. For CUDA and OptiX, `auto` and `on` reject an invalid explicit root; `off` ignores the SDK altogether.
- **Hints are not guarantees.** Without explicit `--vs*` bounds, CUDA's `_MSC_VER` guard supplies VS-year search hints. Finding `cl.exe` does not prove NVCC will like its minor version, any more than finding CUDA on Linux proves GCC compatibility.

---

## 3. Environment Output & Safety Limits

`Env` separates executable, runtime, header, linker, and Windows assembly-reference paths. Every formatter (`fmt_cmd`, `fmt_ps`, `fmt_sh`, `fmt_json`) returns `Result<String, FormatError>`.

| Component | Windows | Linux |
|---|---|---|
| Executable paths | `PATH` | `PATH` |
| Shared-library runtime paths | `PATH` | `LD_LIBRARY_PATH` |
| Header paths | `INCLUDE` | `CPATH` |
| Linker-library paths | `LIB` | `LIBRARY_PATH` |
| Assembly references | `LIBPATH` | Error if nonempty |

- **PowerShell & POSIX (`fmt_ps`, `fmt_sh`)**: Prepend new entries to the inherited list present at *apply time*. On Windows, `fmt_ps` enforces the .NET `SetEnvironmentVariable` 32,766 UTF-16 code-unit limit on names, scalars, and generated lists—and runs an all-or-nothing preflight before mutating your shell. `fmt_sh` on Windows translates `PATH` drive letters to MSYS spelling while keeping `INCLUDE`/`LIB`/`LIBPATH` native.
- **CMD (`fmt_cmd`)**: Snapshots inherited lists at *render time*, removes exact duplicates, keeps at most one preexisting empty component, requires ASCII, and rejects `%`, `!`, `"`, and assignments over 8,191 characters.
- **JSON (`fmt_json`)**: Emits native path arrays and literal scalar strings without merging inherited state.

---

## 4. Supported SDKs

- **CUDA**: Reads `CUDA_VERSION` (`include/cuda.h`) and MSVC guards (`include/crt/host_config.h`), requires `bin/nvcc[.exe]`, and resolves `nvcc` symlinks before deriving the root. Root aliases: `CUDA_PATH`, `CUDA_HOME`, `CUDA_ROOT`, `CUDA_TOOLKIT_ROOT_DIR`. Separates `lib_dir(target)` (link) from `runtime_dir(target)` (runtime). Legacy `lib64` is native-target only.
- **OptiX**: Reports header `OPTIX_VERSION`, `OPTIX_ABI_VERSION`, and device-header presence. Root aliases: `OPTIX_ROOT`, `OptiX_INSTALL_DIR`, `OPTIX_PATH`. `add_optix` wires headers; the display driver provides the runtime.
- **NVIDIA SDKs (`nvsdk::Kind`)**: cuDNN, Video Codec, Optical Flow, NVAPI (Windows-only), NGX, nvCOMP, and NCCL (Linux-only). Searches root aliases (`Kind::root_vars()`), standard installer trees, and `NV_SDK_DIRS`. Passive PE/ELF checks via `object` verify architecture without loading libraries; versioned sonames (e.g. `libnccl.so.2`) register runtime without pretending to be linkable `.so`/`.a` artifacts.
- **vcpkg**: Uses `VCPKG_ROOT/installed/<triplet>/{include,lib}`. Explicit `VCPKG_DEFAULT_TRIPLET` must name a supported triplet (`x64`/`x86`/`arm64`-`windows*` or `x64`/`arm64`-`linux*`); it never falls back silently.

---

## 5. Build & Verification

The crate builds with `--no-default-features` (core `Env` + formatters, no `clap`), or any combination of `cli`, `cuda`, `optix`, and `nvsdk`.

```sh
python bootstrap.py b   # release build
python bootstrap.py t   # test suite
python bootstrap.py c   # fmt + clippy checks
```

See [PLAN.md](PLAN.md) for current platform verification receipts and [CHANGELOG.md](CHANGELOG.md) for release history.

## License

MIT
