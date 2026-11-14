# vcv-rs

`vcv-rs` is a small, stubborn piece of plumbing for Windows and Linux build environments. It locates Visual Studio, CUDA, OptiX, `nvsdk` libraries, and `vcpkg`, checks that the target binaries match what you asked for, and prints a clean, properly escaped environment—or fails early if your explicit paths don't add up.

`vcv-rs` comes as both a library (`vcv_rs`) and a CLI (`vcv-rs` / `vcv-rs.exe`). Default features are `cli`, `cuda`, `optix`, and `nvsdk`.

---

## 1. Using it as a library (`vcv_rs`)

Pull it straight from `main`:

```toml
[dependencies]
vcv-rs = { git = "ssh://git@github.com/ssoj13/vcv-rs.git", branch = "main", default-features = false, features = ["cuda", "optix", "nvsdk"] }
```

`Arch`, `discovery`, `env`, and `format` work on both Windows and Linux. `detect` and `registry` are Windows-only. `Arch::from_target_arch` maps Rust's `x86_64`, `x86`, and `aarch64` to `X64`, `X86`, and `Arm64`; `Arch::native()` gives you the architecture you compiled for, or `None` if it isn't supported.

Setting up CUDA and OptiX without MSVC is straightforward:

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

When you need MSVC on Windows, pass both host and target architectures:

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

A few practical notes:
- `add_cuda`, `add_nvsdk`, and `add_vcpkg` return `Result<(), EnvError>`. `add_optix` only wires headers and root variables, so it doesn't fail.
- Paths and values stay raw `PathBuf`/`OsString` until formatting. Anything non-Unicode is rejected cleanly rather than mangled.
- If detection returns `None`, check `requested_root()` to see whether an explicit environment variable pointed somewhere invalid.

---

## 2. Using the CLI (`vcv-rs`)

```sh
cargo build --release
cargo install --path .
```

Status messages go to `stderr`; the actual script or JSON goes to `stdout`, and only after everything checks out.

```powershell
# PowerShell: render first, check exit code, then apply.
$script = vcv-rs -f ps -q
if ($LASTEXITCODE -eq 0) { ($script -join "`n") | Invoke-Expression }

# JSON for scripts and tooling
vcv-rs -f json -q | ConvertFrom-Json
```

```cmd
:: CMD: snapshots the current process's inherited lists at render time.
vcv-rs -f cmd > vcenv.bat && call vcenv.bat
```

```sh
# Linux / Bash / MSYS2: keep the quotes around $script.
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
| `--no-nvsdk` | Skip separately shipped NVIDIA SDKs and ignore their root overrides |
| `-l, --list` | Show everything found on the machine and mark what was picked |
| `-q, --quiet` | Keep `stderr` quiet |
| `--no-validate` | Windows: skip checking the selected `cl.exe` |
| `-h, --help` | Print help and examples |

On Linux, `vcv-rs` sets up native SDKs without Visual Studio. If you ask it for Windows-only flags, cross-target builds, or CMD output, it stops and returns an error.

### How toolchains get picked

```sh
vcv-rs --list
vcv-rs --vs 2022 --edition buildtools
vcv-rs --vs-min 2022
vcv-rs --prerelease only
```

- **Explicit roots come first.** The first nonempty root variable you set is the one `vcv-rs` uses—even if it's broken. It will never silently swap your path for a newer install in `Program Files`. In `auto` or `on` mode, a bad explicit root is an error; `off` ignores that SDK completely.
- **Hints aren't guarantees.** Without `--vs*` bounds, CUDA's `_MSC_VER` guard gives a VS-year hint. Finding `cl.exe` doesn't guarantee NVCC will like its exact minor version, just like finding CUDA on Linux doesn't prove your GCC version matches.

---

## 3. Environment Output & Limits

`Env` keeps executables, runtime libraries, headers, link libraries, and Windows assembly references separate. All four formatters (`fmt_cmd`, `fmt_ps`, `fmt_sh`, `fmt_json`) return `Result<String, FormatError>`.

| Component | Windows | Linux |
|---|---|---|
| Executable paths | `PATH` | `PATH` |
| Shared-library runtime paths | `PATH` | `LD_LIBRARY_PATH` |
| Header paths | `INCLUDE` | `CPATH` |
| Linker-library paths | `LIB` | `LIBRARY_PATH` |
| Assembly references | `LIBPATH` | Error if nonempty |

- **PowerShell & POSIX (`fmt_ps`, `fmt_sh`)**: Prepend new paths to whatever is already in your shell when you apply the script. On Windows, `fmt_ps` enforces the .NET `SetEnvironmentVariable` 32,766 UTF-16 code-unit limit and checks everything up front so a late error won't leave your shell half-configured. On Windows, `fmt_sh` converts `PATH` to MSYS style while keeping `INCLUDE`/`LIB`/`LIBPATH` native.
- **CMD (`fmt_cmd`)**: Captures inherited lists when the script is generated, drops exact duplicates, keeps at most one existing empty entry, requires ASCII, and rejects `%`, `!`, `"`, and lines over 8,191 characters.
- **JSON (`fmt_json`)**: Emits raw path arrays and scalar strings without touching inherited variables.

---

## 4. What SDKs it knows

- **CUDA**: Reads `CUDA_VERSION` (`include/cuda.h`) and MSVC guards (`include/crt/host_config.h`), requires `bin/nvcc[.exe]`, and resolves `nvcc` symlinks before finding the root. Checks `CUDA_PATH`, `CUDA_HOME`, `CUDA_ROOT`, `CUDA_TOOLKIT_ROOT_DIR`. Keeps `lib_dir(target)` (link) separate from `runtime_dir(target)` (runtime); legacy `lib64` is native-target only.
- **OptiX**: Reads `OPTIX_VERSION`, `OPTIX_ABI_VERSION`, and checks for device headers. Checks `OPTIX_ROOT`, `OptiX_INSTALL_DIR`, `OPTIX_PATH`. `add_optix` wires headers only—the display driver supplies the runtime.
- **NVIDIA SDKs (`nvsdk::Kind`)**: cuDNN, Video Codec, Optical Flow, NVAPI (Windows-only), NGX, nvCOMP, and NCCL (Linux-only). Looks in `Kind::root_vars()`, standard install folders, and `NV_SDK_DIRS`. Uses `object` to inspect PE/ELF headers passively without loading DLLs or `.so` files; versioned sonames like `libnccl.so.2` count as runtime, not link libraries.
- **vcpkg**: Uses `VCPKG_ROOT/installed/<triplet>/{include,lib}`. If `VCPKG_DEFAULT_TRIPLET` is set, it must name a supported triplet (`x64`/`x86`/`arm64`-`windows*` or `x64`/`arm64`-`linux*`); no silent fallback.

---

## 5. Building & Testing

You can compile with `--no-default-features` (just `Env` and formatters, no `clap`), or pick any subset of `cli`, `cuda`, `optix`, and `nvsdk`.

```sh
python bootstrap.py b   # release build
python bootstrap.py t   # tests
python bootstrap.py c   # fmt + clippy
```

Current test receipts and remaining optional Linux/ARM64 checks are in [PLAN.md](PLAN.md); version history is in [CHANGELOG.md](CHANGELOG.md).

## License

MIT
