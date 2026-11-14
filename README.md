# vcv-rs

Toolchain and NVIDIA SDK discovery with environment generation for Windows and Linux.
On Windows, Visual Studio/MSVC detection uses `vswhere.exe` and the registry. On Linux,
the CLI assembles a native SDK environment without consulting Visual Studio.

The crate provides the `vcv_rs` library and the `vcv-rs` binary
(`vcv-rs.exe` on Windows). Default features are `cli`, `cuda`, `optix` and `nvsdk`.
Windows/Linux implementation source is present; independent review, feature checks and native
execution gates for this change are still pending. See the [work queue](TODO.md) and
[domain index](docs/mastermind/MASTERMIND_INDEX.md). ARM64 fixtures do not establish native
ARM64 hardware validation.

## Consume as a library (git-ref)

This is the canonical discovery crate. Depend on it directly:

```toml
[dependencies]
vcv-rs = { git = "ssh://git@github.com/ssoj13/vcv-rs.git", branch = "main", default-features = false, features = ["cuda", "optix", "nvsdk"] }
```

`Arch`, `discovery`, `env` and `format` are common library APIs. `detect` and `registry`
are Windows-only. `Arch::from_target_arch` maps Rust's `x86_64`, `x86` and `aarch64`
names to `X64`, `X86` and `Arm64`; `Arch::native()` returns the compiled target's
architecture, or `None` when unsupported.

This example creates an SDK environment for the native architecture. `Env::new` does not
add a Windows compiler; use `build_env` below when MSVC is required.

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

The Windows builder retains separate host and target architecture arguments:

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

`add_cuda`, `add_nvsdk` and `add_vcpkg` return `Result<(), EnvError>`.
`add_optix` adds headers and root aliases without a fallible return.
Paths and scalar values remain `PathBuf`/`OsString` until output validation; non-Unicode
values are rejected by a formatter instead of being converted lossily. Library callers
can inspect `requested_root()` alongside a missing detection result to diagnose an invalid
explicit root.

## CLI (`vcv-rs`)

```sh
cargo build --release
cargo install --path .
# target/release/vcv-rs, or target\release\vcv-rs.exe on Windows
```

### Usage

```powershell
# Render first, then apply only after a successful CLI exit.
$script = vcv-rs -f ps -q
if ($LASTEXITCODE -eq 0) { ($script -join "`n") | Invoke-Expression }

# JSON metadata for tools
vcv-rs -f json -q | ConvertFrom-Json
```

```cmd
:: Generate and immediately apply a snapshot of this process's inherited lists.
vcv-rs -f cmd > vcenv.bat && call vcenv.bat
```

```sh
# Linux / Bash / MSYS2: keep command substitution and evaluation quoted.
script=$(vcv-rs -f sh -q) && eval "$script"
```

Informational output goes to stderr; stdout contains the fully rendered environment or
inventory. Invalid selection or formatting fails before an executable script is emitted.

### Options

| Option | Behavior |
|---|---|
| `-a, --arch` / `-s, --host` | Target/host: `x64`, `x86`, `arm64`; default is native |
| `-f, --format` | `auto`, `ps`/`powershell`, `cmd`, `sh`/`bash`, `json` |
| `-v, --vs` | Exact Windows VS year: 2017, 2019, 2022, 2026 |
| `--vs-min` / `--vs-max` | Windows VS year bounds; conflict with `--vs` |
| `-e, --edition` | Windows: `community`, `professional`, `enterprise`, `buildtools` |
| `--prerelease` | Windows: `exclude` (default), `allow`, `only` |
| `-c, --cuda` / `-o, --optix` | `auto` (default), `on` (require), `off` (ignore); feature-gated |
| `--no-nvsdk` | Omit separately shipped NVIDIA SDKs and ignore their root requests |
| `-l, --list` | Inventory discovered installations and mark the selected roots |
| `-q, --quiet` | Suppress informational stderr messages |
| `--no-validate` | Windows: skip checking the selected host/target `cl.exe` path |
| `-h, --help` | Show help and examples |

Linux accepts native host and target setup. Explicit Windows-only flags, a non-native
host/target or CMD output return errors; vcv-rs does not configure Linux cross-compilers.

### Selecting a toolchain

Windows selection supports year, edition and release channel filters:

```sh
vcv-rs --list
vcv-rs --vs 2022 --edition buildtools
vcv-rs --vs-min 2022
vcv-rs --vs-max 2022
vcv-rs --prerelease only
```

Preview installations are excluded by default. With no explicit year bounds, the parsed CUDA
`_MSC_VER` guard supplies VS-year search hints. Explicit year bounds take precedence.
These hints and the existence of `cl.exe` do not prove that the actual compiler version is
accepted by CUDA; Linux SDK discovery likewise does not establish GCC compatibility.

The first nonempty configured SDK root alias is authoritative, even if invalid. Selection
never replaces it with a newer searched installation or a lower-priority alias. For CUDA and
OptiX, `auto` and `on` reject an invalid explicit root; `off` ignores that SDK and its roots.
Separately shipped SDK requests are ignored only with `--no-nvsdk`. Inventories may include
alternatives, but their selected marker comes from the actual selection.

## Environment output

`Env` separates executable, runtime, include, linker and Windows assembly-reference paths.
All `fmt_cmd`, `fmt_ps`, `fmt_sh` and `fmt_json` calls return
`Result<String, FormatError>`.

| Component | Windows | Linux |
|---|---|---|
| Executable paths | `PATH` | `PATH` |
| Shared-library runtime paths | `PATH` | `LD_LIBRARY_PATH` |
| Header paths | `INCLUDE` | `CPATH` |
| Linker-library paths | `LIB` | `LIBRARY_PATH` |
| Assembly references | `LIBPATH` | Unsupported when nonempty |

PowerShell and POSIX output prepend new lists to the values present when the script is
applied, without adding a separator for an unset or empty inherited value. On Windows,
POSIX output converts drive paths in `PATH` to MSYS spelling; `INCLUDE`, `LIB` and
`LIBPATH` retain native Windows spelling and separators. PowerShell on Linux uses Linux
names and separators.

CMD output captures inherited lists from the generator process at render time. It writes
literal assignments; a saved script replaces later list changes with that captured snapshot.
New entries precede inherited entries, with exact-string duplicates removed. Unset/empty
inherited lists add no separator. An empty component already present in a nonempty inherited
list is retained at most once: `A;;B;` contributes `A;;B`.

Generated empty path entries, native list separators inside a component, control characters,
invalid keys, reserved-list key collisions and non-Unicode values return errors. CMD additionally
requires ASCII and rejects `%`, `!`, double quotes and assignments over 8,191 characters.
A legitimate Windows key such as `ProgramFiles(x86)` is supported by PowerShell/CMD but cannot
be exported as a POSIX identifier. JSON contains native path arrays and literal scalar strings;
it does not merge an inherited environment.

## CUDA

CUDA discovery reads `CUDA_VERSION` from `include/cuda.h` and the MSVC guard from
`include/crt/host_config.h`. A toolkit probe requires the CUDA header and `bin/nvcc`
(`nvcc.exe` on Windows). Explicit aliases, in order:

`CUDA_PATH`, `CUDA_HOME`, `CUDA_ROOT`, `CUDA_TOOLKIT_ROOT_DIR`.

Search locations include Windows's CUDA install parent and Linux's `/usr/local/cuda*` and
`/opt/cuda`, plus `nvcc` on `PATH`. An `nvcc` symlink is resolved before deriving its root.
`add_cuda` sets every root alias and `CUDA_PATH_V<major>_<minor>` to the selected root.

`CudaInfo::lib_dir(target)` and `runtime_dir(target)` describe separate capabilities.
Windows uses target-tagged libraries and matching PE runtime evidence. Linux uses
`targets/x86_64-linux/lib` or `targets/aarch64-linux/lib`; legacy `lib64` is native-target
only. A static archive can establish tagged link metadata without establishing a runtime.
A versioned shared soname can establish runtime metadata without an unversioned link artifact.
A missing target link directory makes `add_cuda` fail.

`CudaVersion::to_macro()` and `OptixVersion::to_macro()` return `Option<u32>` for checked
version encoding. Discovering a future version does not establish compiler or runtime support.

## OptiX

OptiX discovery reports the header version, function-table ABI and device-header presence.
Root aliases are `OPTIX_ROOT`, `OptiX_INSTALL_DIR`, then `OPTIX_PATH`. Standard searches
include `%ProgramData%\\NVIDIA Corporation\\OptiX SDK *` on Windows and `/opt/*optix*` on Linux.
`add_optix` adds headers and aliases; the display driver supplies the runtime.

## Separately shipped NVIDIA SDKs

`nvsdk::Kind` covers cuDNN, Video Codec, Optical Flow, NVAPI, NGX, nvCOMP and NCCL.
Use `nvsdk::detect(kind, target)` or `probe(kind, root, target)`; inventory functions also
take an explicit target. `SdkInfo` contains `target`, `include_dir`, optional
`runtime_dir` and optional `lib_dir`.

Root aliases come from `Kind::root_vars()`. Search parents include installer locations,
Linux `/usr` and `/usr/local`, and the native path list `NV_SDK_DIRS`. Windows nvCOMP
CUDA-major layouts and Linux cuDNN/multiarch layouts are recognized. NVAPI is Windows-only;
NCCL is Linux-only. cuFile discovery comes through the CUDA Toolkit.

Header-only and runtime-only results remain distinct. For example, `libnccl.so.2` without
an unversioned shared library or archive yields runtime metadata, not a link directory.
Passive ELF/PE architecture checks do not load the library or prove its ABI, dependencies,
GPU support or execution behavior.

## vcpkg

Set `VCPKG_ROOT` to a tree with `installed/<triplet>/{include,lib}`. A nonempty
`VCPKG_DEFAULT_TRIPLET` must name a supported installed triplet; it never silently falls back.
Without it, discovery tries the native-platform candidates in order:

| Platform | Supported candidates |
|---|---|
| Windows | `x64`/`x86`/`arm64` + `-windows`, `-windows-static-md`, `-windows-static` |
| Linux | `x64`/`arm64` + `-linux`, `-linux-dynamic` |

Linux x86 and custom triplets are unsupported. Shared-library runtime metadata requires
matching native binary evidence; a static-only directory does not qualify. `add_vcpkg`
propagates both the selected root and `VCPKG_DEFAULT_TRIPLET`.
Library callers use `detect_vcpkg`, `probe_vcpkg(root, target)` and fallible `add_vcpkg`.

## Migrating existing callers

- Construct `Env::new(target)` instead of `Env::default()`.
- Call `add_cuda(&mut environment, &cuda)?`; its target comes from `Env`, so the old third architecture argument is removed.
- Propagate `add_nvsdk`, `add_vcpkg` and all formatter errors. Formatters return a string inside `Result`; render it before printing.
- Pass a target to NVIDIA SDK discovery and use `SdkInfo.runtime_dir` in place of `bin_dir`; keep `lib_dir` separate.
- Handle the `Option<u32>` returned by CUDA/OptiX `to_macro()`.

## Build

The library can omit all optional features with `--no-default-features`. `cli` alone
enables clap and the binary; `cuda`, `optix` and `nvsdk` enable their discovery APIs.
Their full Windows/Linux feature matrix remains a verification gate.

The repository includes a Python 3 standard-library wrapper:

```sh
python bootstrap.py b   # release build
python bootstrap.py t   # tests
python bootstrap.py c   # formatting/lint checks
```

For the current delivery wave, follow the [work queue](TODO.md): finish source and commit/push
to main before compiler or test commands. Examples above document interfaces; they are not
receipts that the pending gates have passed.

## License

MIT
