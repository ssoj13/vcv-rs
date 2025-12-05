# vcv-rs

Fast Visual Studio environment setup. **~50x faster** than `vcvars64.bat`.

| | Time |
|---|---:|
| `vcv-rs` | ~20ms |
| `vcvars64.bat` | ~2000ms |

## Why?

Microsoft's `vcvars64.bat` is slow because it:
- Spawns PowerShell for telemetry
- Runs 15+ batch files sequentially
- Queries registry multiple times
- Searches directories with `dir` commands

This tool does the same job with:
- Single `vswhere.exe` call (~20ms)
- Direct registry lookups
- Zero telemetry
- Native compiled binary

## Installation

```powershell
# Build from source
cargo build --release
copy target\release\vcv-rs.exe C:\bin\
```

## Usage

### PowerShell

```powershell
# Apply environment (auto-detect shell)
vcv-rs | iex

# Quiet mode
vcv-rs -q | iex

# Add to $PROFILE
function vcvars { vcv-rs @args | iex }
```

### CMD

```cmd
vcv-rs -f cmd > vcenv.bat && vcenv.bat

:: Or inline
for /f "delims=" %i in ('vcv-rs -f cmd') do @%i
```

### Bash / MSYS2

```bash
eval $(vcv-rs -f sh)
```

### JSON (for tools)

```powershell
vcv-rs -f json -q
```

## Options

```
-a, --arch      Target architecture: x64 (default), x86, arm64
-s, --host      Host architecture: x64 (default), x86, arm64
-f, --format    Output format: auto (default), ps, cmd, sh, json
-v, --vs        VS version year: 2017, 2019, 2022
-q, --quiet     Suppress info messages
--no-validate   Skip cl.exe validation
-h, --help      Print help
```

## Examples

```powershell
# x64 native (default)
vcv-rs | iex

# x86 target
vcv-rs -a x86 | iex

# Cross-compile for ARM64
vcv-rs -a arm64 | iex

# Use specific VS version
vcv-rs -v 2019 | iex

# JSON for parsing
vcv-rs -f json | ConvertFrom-Json
```

## Output

**Note:** All paths are prepended (added to the beginning), not replaced. Your existing PATH, INCLUDE, etc. remain intact - VS tools just get priority.

Sets these environment variables:

| Variable | Description |
|----------|-------------|
| `PATH` | Compiler binaries, SDK tools |
| `INCLUDE` | Headers (VC++, SDK, UCRT) |
| `LIB` | Libraries for linking |
| `LIBPATH` | Assembly references |
| `VCToolsInstallDir` | VC++ toolset path |
| `WindowsSdkDir` | Windows SDK path |
| `UCRTVersion` | Universal CRT version |

## License

MIT
