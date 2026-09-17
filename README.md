# vcv-rs

Fast Visual Studio / MSVC environment detection for Windows. The library probes
`vswhere.exe` and the Windows registry directly to assemble the same `PATH`,
`INCLUDE`, `LIB`, `LIBPATH` (and related) variables that `vcvars64.bat` sets — in
a fraction of the time.

| | Time |
|---|---:|
| `vcv-rs` | ~20ms |
| `vcvars64.bat` | ~2000ms |

`vcvars64.bat` is slow because it spawns PowerShell for telemetry, runs 15+ batch
files sequentially, and re-queries the registry. This crate does the same job with
a single `vswhere.exe` call, direct registry lookups, and zero telemetry.

The crate ships as a **library** (consumed by other projects via git-ref) plus a
Windows-only **`vcv` CLI**. Both the `cli` and `cuda` features are on by default.
On non-Windows targets the library compiles to a stub (and `vcv` is a stub binary)
so workspace builds stay green.

## Consume as a library (git-ref)

This is a standalone canonical crate. Depend on it directly from Git:

```toml
[dependencies]
vcv-rs = { git = "ssh://git@github.com/ssoj13/vcv-rs.git", branch = "main" }
```

The import name is `vcv_rs`. Windows-only detection lives in the `detect`, `env`,
and `format` modules:

```rust
use vcv_rs::{detect, detect_cuda, env, format, Arch};

// Detect VS (None = newest available); pass Some(year) for 2017/2019/2022/2026.
if let Some(vs) = detect::detect_vs(None) {
    let sdk = detect::detect_sdk();
    let ucrt = detect::detect_ucrt();

    // Assemble PATH/INCLUDE/LIB/LIBPATH for host x64 -> target x64.
    let mut e = env::build_env(&vs, sdk.as_ref(), ucrt.as_ref(), Arch::X64, Arch::X64);
    if let Some(cuda) = detect_cuda() {
        env::add_cuda(&mut e, &cuda, Arch::X64);
    }

    // Emit for a shell: fmt_ps / fmt_cmd / fmt_sh / fmt_json.
    print!("{}", format::fmt_ps(&e));
}
```

`Arch` (`X64` / `X86` / `Arm64`) is available on all targets; `detect`, `env`,
`format`, and `registry` are gated to `#[cfg(windows)]`.

## CLI (`vcv`)

The CLI is Windows-only (a stub binary on other targets). Default features include `cli` and `cuda`:

```powershell
cargo build --release
# binary: target\release\vcv.exe
cargo install --path .   # installs `vcv`
```

### Usage

```powershell
# PowerShell: apply environment to the current session (auto-detect shell)
vcv | iex
vcv -q | iex                            # quiet (suppress info on stderr)

# Persist a helper in $PROFILE
function vcvars { vcv @args | iex }
```

```cmd
:: CMD
vcv -f cmd > vcenv.bat && vcenv.bat
for /f "delims=" %i in ('vcv -f cmd') do @%i
```

```bash
# Bash / MSYS2
eval $(vcv -f sh)
```

```powershell
# JSON for tools
vcv -f json -q | ConvertFrom-Json
```

### Options

```
-a, --arch      Target architecture: x64 (default), x86, arm64
-s, --host      Host architecture: x64 (default), x86, arm64
-f, --format    Output format: auto (default), ps, cmd, sh, json
-v, --vs        VS version year: 2017, 2019, 2022, 2026 (exact)
    --vs-min    Oldest acceptable VS year
    --vs-max    Newest acceptable VS year
-e, --edition   community | professional | enterprise | buildtools (default: any)
    --prerelease  exclude (default), allow, only
-c, --cuda      CUDA Toolkit: auto (default), on (require), off (ignore)
-l, --list      List every detected toolchain and exit
-q, --quiet     Suppress info messages
    --no-validate  Skip cl.exe validation
-h, --help      Print help
```

### Selecting a toolchain

Four independent axes, each with a default that needs no flag: **any year**, **any edition**,
**released channels only**, newest wins. Set only what you care about.

```sh
vcv -l                    # what is installed, and which one the current flags pick
vcv -v 2022               # exactly VS 2022
vcv -v 2026               # exactly VS 2026
vcv --vs-max 2022         # newest up to 2022
vcv --vs-min 2022         # 2022 or newer
vcv -e buildtools         # standalone C++ Build Tools (the usual CI install)
vcv --prerelease only     # a Preview channel, on purpose
```

`-v` is shorthand for setting both bounds and conflicts with `--vs-min`/`--vs-max`. Preview
installs are excluded by default: they usually carry the highest version number, and an
unqualified request should not land on a preview channel for that reason alone.

When nothing matches, the error names the constraint that failed and lists what IS installed --
"Visual Studio not found" on a machine with three of them points the operator at the wrong problem.

All paths are **prepended**, not replaced: existing `PATH`, `INCLUDE`, etc. stay
intact and VS tools simply gain priority. Variables set: `PATH`, `INCLUDE`, `LIB`,
`LIBPATH`, `VCToolsInstallDir`, `WindowsSdkDir`, `UCRTVersion`.

## CUDA

When a CUDA Toolkit is installed the `vcv` CLI adds it automatically: `bin` and `bin/x64` on
`PATH`, `include` on `INCLUDE`, `lib/x64` on `LIB`, and `CUDA_PATH` / `CUDA_HOME` /
`CUDA_PATH_V<major>_<minor>` all pointing at the same root. Turn it off with `-c off`,
or drop the code entirely with `--no-default-features` (the `cuda` feature).

No CUDA version is hard-coded, so 12.x, 13.x, 14.x and whatever follows work the same:

| Fact | Read from |
|---|---|
| toolkit version | `include/cuda.h` -> `#define CUDA_VERSION` |
| accepted host compilers | `include/crt/host_config.h` -> the `_MSC_VER` guard |
| candidate roots | `CUDA_PATH`/`CUDA_HOME`/`CUDA_ROOT`/`CUDA_TOOLKIT_ROOT_DIR`, the install directory, `nvcc` on `PATH` |

Because the toolkit declares its own `_MSC_VER` range, the CLI (with no year flags) picks a
Visual Studio that CUDA accepts instead of the newest one installed. That matters on a machine
carrying a VS too new for its toolkit: `nvcc` would otherwise stop at a `#error` in
`host_config.h`, which reads as a broken CUDA install rather than a compiler one release ahead.
`-v` / `--vs-min` / `--vs-max` win outright: they are never silently narrowed by the toolkit.
A pinned year outside the declared range is a warning, not an override.

`build_env` does not look at CUDA. Library callers should call `detect_cuda` + `add_cuda` the
same way the CLI does (see the git-ref example above).

The environment variables searched are exactly the set `cudarc`'s build script reads, so a
shell configured by `vcv` and a crate built in it can never select different toolkits.

Linux root discovery is implemented (`/usr/local/cuda*`, `/opt/cuda`) but its library layout
is still marked TODO; macOS has had no toolkit since CUDA 10.2 and is not searched.

## vcpkg

When `VCPKG_ROOT` points at an existing tree, `installed/<triplet>/lib` is prepended to `LIB`
and `LIBPATH`, `include` to `INCLUDE`, and `bin` to `PATH`. The triplet is `VCPKG_DEFAULT_TRIPLET`
if that directory exists, otherwise `x64-windows` / `x86-windows` / `arm64-windows` (then
`-static-md`, then `-static`). This is how MSVC `link.exe` finds `libssl.lib` from the vcpkg
`openssl` port without each consumer hard-coding a path.

Callers that consume the library (git-ref) should call `detect_vcpkg` + `probe_vcpkg` +
`add_vcpkg` the same way the CLI does — `build_env` alone does not look at vcpkg.

## Build

The repo ships a `bootstrap.py` wrapper (release by default, Python 3 stdlib only):

```sh
python bootstrap.py b   # build (cargo build --workspace --release)
python bootstrap.py t   # test  (cargo test --workspace)
python bootstrap.py c   # check (cargo fmt --check + clippy -D warnings)
```

`bootstrap.py b` builds the library and the `vcv` binary (both `cli` and `cuda` are default
features). Drop them with `--no-default-features` if a consumer wants the library only.

## License

MIT
