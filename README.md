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

The crate ships as a **library** (consumed by other projects via git-ref) plus an
optional, Windows-only **`vcv` CLI** behind the `cli` feature. On non-Windows
targets the library compiles to a stub so workspace builds stay green.

## Consume as a library (git-ref)

This is a standalone canonical crate. Depend on it directly from Git:

```toml
[dependencies]
vcv-rs = { git = "ssh://git@github.com/ssoj13/vcv-rs.git", branch = "main" }
```

The import name is `vcv_rs`. Windows-only detection lives in the `detect`, `env`,
and `format` modules:

```rust
use vcv_rs::{detect, env, format, Arch};

// Detect VS (None = newest available); pass Some(year) for 2017/2019/2022.
if let Some(vs) = detect::detect_vs(None) {
    let sdk = detect::detect_sdk();
    let ucrt = detect::detect_ucrt();

    // Assemble PATH/INCLUDE/LIB/LIBPATH for host x64 -> target x64.
    let e = env::build_env(&vs, sdk.as_ref(), ucrt.as_ref(), Arch::X64, Arch::X64);

    // Emit for a shell: fmt_ps / fmt_cmd / fmt_sh / fmt_json.
    print!("{}", format::fmt_ps(&e));
}
```

`Arch` (`X64` / `X86` / `Arm64`) is available on all targets; `detect`, `env`,
`format`, and `registry` are gated to `#[cfg(windows)]`.

## CLI (`vcv`)

The CLI is Windows-only and requires the `cli` feature:

```powershell
cargo build --release --features cli
# binary: target\release\vcv.exe
cargo install --path . --features cli   # installs `vcv`
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
-v, --vs        VS version year: 2017, 2019, 2022
-q, --quiet     Suppress info messages
    --no-validate  Skip cl.exe validation
-h, --help      Print help
```

All paths are **prepended**, not replaced: existing `PATH`, `INCLUDE`, etc. stay
intact and VS tools simply gain priority. Variables set: `PATH`, `INCLUDE`, `LIB`,
`LIBPATH`, `VCToolsInstallDir`, `WindowsSdkDir`, `UCRTVersion`.

## Build

The repo ships a `bootstrap.py` wrapper (release by default, Python 3 stdlib only):

```sh
python bootstrap.py b   # build (cargo build --workspace --release)
python bootstrap.py t   # test  (cargo test --workspace)
python bootstrap.py c   # check (cargo fmt --check + clippy -D warnings)
```

`bootstrap.py b` builds the library; the `vcv` binary additionally needs
`--features cli` (see above), since it is gated behind `required-features`.

## License

MIT
