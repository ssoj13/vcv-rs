# D13 — CLI and feature integration

## Transfer status — 2026-11-13

Paused by owner for environment transfer. The [current handoff](../../../HANDOFF.md) supersedes the historical counts and statuses below. D11 has 225 statically reviewed IDs plus added unreviewed ID226; D12 has 125 IDs and an open Windows PowerShell length defect; D13's 110-ID independent review remains pending. No platform or runtime gate has run. Preserved JSON receipts and unexecuted scripts are in ../evidence/2026-11-13-transfer/.

## Historical source ledger


Updated: 2026-11-13. Status: SOURCE FROZEN — NOT VERIFIED.
See the [domain index](../MASTERMIND_INDEX.md), [seam matrix](../SEAM_MATRIX.md) and [work queue](../../../TODO.md).

## Scope and source checkpoint

Source boundary: `src/cli.rs`, `src/cli/{args,platform,sdks}.rs`, `src/main.rs`, `src/lib.rs`,
`Cargo.toml`, `Cargo.lock` and `tests/cli.rs`. The only authorized `src/detect.rs` changes
are three clap-related cfg attributes. Documentation is maintained separately.
No compiler, test, formatter, SDK execution or git result is established by this ledger.

| Unit | Inspected source | Coordinator-reported size | State |
|---|---|---:|---|
| U01 | lib/main/Cargo feature wiring and bounded detect.rs cfg attributes | 150 primary physical lines plus three cfg attributes | source frozen |
| U02 | cli/args.rs and cli/platform.rs | 407 physical lines | source frozen |
| U03 | cli.rs, cli/sdks.rs, tests/cli.rs and two runtime-display strings | 454 physical lines plus two strings | source frozen |

The coordinator reports 1,011 primary physical lines and 1,530 including adjacent detect.rs review.
These are partition/inventory reports, not a complete independent audit.

## Written behavior

- The common library exposes native/Rust-target architecture mapping, discovery, environments and formatters without requiring clap; `cli` activates optional clap.
- Windows retains VS filtering and host/target compiler-path validation. Linux builds a native environment and rejects explicit Windows/cross-target/CMD requests.
- One selected SDK descriptor is reused for environment assembly and selected inventory markers.
- Invalid configured roots fail in Auto/On; Off ignores the SDK. Unsupported explicit SDK requests remain errors.
- The CLI constructs the complete output string before stdout. Formatting/selection errors produce no executable script.
- CUDA/MSVC year bounds are search hints, not proof of actual compiler compatibility.

## Numbered manifest

The 110 rows below reproduce the coordinator's source-freeze manifest. Citations are checkpoint
anchors and must be refreshed after repairs. No row is an Inquisitor approval.

| # | Unit | Source | Symbol | Gate |
|---|---|---|---|---|
| 1 | U01 | Cargo.toml:15 | features/dependencies | source inventory; review pending |
| 2 | U01 | src/detect.rs:50 | VsEdition clap cfg attributes | source inventory; review pending |
| 3 | U01 | src/detect.rs:84 | Prerelease clap cfg attributes | source inventory; review pending |
| 4 | U01 | src/lib.rs:11 | Arch | source inventory; review pending |
| 5 | U01 | src/lib.rs:12 | Arch::X64 | source inventory; review pending |
| 6 | U01 | src/lib.rs:13 | Arch::X86 | source inventory; review pending |
| 7 | U01 | src/lib.rs:14 | Arch::Arm64 | source inventory; review pending |
| 8 | U01 | src/lib.rs:19 | Arch::from_target_arch | source inventory; review pending |
| 9 | U01 | src/lib.rs:29 | Arch::native | source inventory; review pending |
| 10 | U01 | src/lib.rs:33 | Arch::as_str | source inventory; review pending |
| 11 | U01 | src/lib.rs:43 | Arch::fmt | source inventory; review pending |
| 12 | U01 | src/lib.rs:89 | rust_target_architectures_are_explicit | source inventory; review pending |
| 13 | U01 | src/lib.rs:99 | native_architecture_uses_the_rust_target_name | source inventory; review pending |
| 14 | U01 | src/main.rs:5 | main | source inventory; review pending |
| 15 | U03 | src/cli.rs:12 | run | source inventory; review pending |
| 16 | U03 | src/cli.rs:30 | execute | source inventory; review pending |
| 17 | U02 | src/cli/args.rs:9 | SdkMode | source inventory; review pending |
| 18 | U02 | src/cli/args.rs:11 | SdkMode::Auto | source inventory; review pending |
| 19 | U02 | src/cli/args.rs:13 | SdkMode::On | source inventory; review pending |
| 20 | U02 | src/cli/args.rs:15 | SdkMode::Off | source inventory; review pending |
| 21 | U02 | src/cli/args.rs:19 | Format | source inventory; review pending |
| 22 | U02 | src/cli/args.rs:20 | Format::Auto | source inventory; review pending |
| 23 | U02 | src/cli/args.rs:21 | Format::Ps | source inventory; review pending |
| 24 | U02 | src/cli/args.rs:22 | Format::Powershell | source inventory; review pending |
| 25 | U02 | src/cli/args.rs:23 | Format::Cmd | source inventory; review pending |
| 26 | U02 | src/cli/args.rs:24 | Format::Sh | source inventory; review pending |
| 27 | U02 | src/cli/args.rs:25 | Format::Bash | source inventory; review pending |
| 28 | U02 | src/cli/args.rs:26 | Format::Json | source inventory; review pending |
| 29 | U02 | src/cli/args.rs:30 | Edition | source inventory; review pending |
| 30 | U02 | src/cli/args.rs:31 | Edition::Community | source inventory; review pending |
| 31 | U02 | src/cli/args.rs:32 | Edition::Professional | source inventory; review pending |
| 32 | U02 | src/cli/args.rs:33 | Edition::Enterprise | source inventory; review pending |
| 33 | U02 | src/cli/args.rs:35 | Edition::BuildTools | source inventory; review pending |
| 34 | U02 | src/cli/args.rs:39 | Channel | source inventory; review pending |
| 35 | U02 | src/cli/args.rs:40 | Channel::Exclude | source inventory; review pending |
| 36 | U02 | src/cli/args.rs:41 | Channel::Allow | source inventory; review pending |
| 37 | U02 | src/cli/args.rs:42 | Channel::Only | source inventory; review pending |
| 38 | U02 | src/cli/args.rs:45 | EXAMPLES | source inventory; review pending |
| 39 | U02 | src/cli/args.rs:62 | Args | source inventory; review pending |
| 40 | U02 | src/cli/args.rs:65 | Args.arch | source inventory; review pending |
| 41 | U02 | src/cli/args.rs:68 | Args.host | source inventory; review pending |
| 42 | U02 | src/cli/args.rs:71 | Args.format | source inventory; review pending |
| 43 | U02 | src/cli/args.rs:74 | Args.vs_year | source inventory; review pending |
| 44 | U02 | src/cli/args.rs:77 | Args.vs_min | source inventory; review pending |
| 45 | U02 | src/cli/args.rs:80 | Args.vs_max | source inventory; review pending |
| 46 | U02 | src/cli/args.rs:83 | Args.edition | source inventory; review pending |
| 47 | U02 | src/cli/args.rs:86 | Args.prerelease | source inventory; review pending |
| 48 | U02 | src/cli/args.rs:89 | Args.list | source inventory; review pending |
| 49 | U02 | src/cli/args.rs:93 | Args.cuda | source inventory; review pending |
| 50 | U02 | src/cli/args.rs:97 | Args.optix | source inventory; review pending |
| 51 | U02 | src/cli/args.rs:101 | Args.no_nvsdk | source inventory; review pending |
| 52 | U02 | src/cli/args.rs:104 | Args.quiet | source inventory; review pending |
| 53 | U02 | src/cli/args.rs:107 | Args.no_validate | source inventory; review pending |
| 54 | U02 | src/cli/args.rs:110 | detect_shell | source inventory; review pending |
| 55 | U02 | src/cli/args.rs:114 | detect_shell_with | source inventory; review pending |
| 56 | U02 | src/cli/args.rs:133 | tests::detect_from | source inventory; review pending |
| 57 | U02 | src/cli/args.rs:138 | tests::detect_msystem_as_sh | source inventory; review pending |
| 58 | U02 | src/cli/args.rs:143 | tests::detect_zsh_as_sh | source inventory; review pending |
| 59 | U02 | src/cli/args.rs:148 | tests::detect_bash_as_sh | source inventory; review pending |
| 60 | U02 | src/cli/args.rs:153 | tests::detect_cmd | source inventory; review pending |
| 61 | U02 | src/cli/args.rs:158 | tests::detect_cmd_over_pwsh_markers | source inventory; review pending |
| 62 | U02 | src/cli/args.rs:164 | tests::detect_psmodulepath_as_ps | source inventory; review pending |
| 63 | U02 | src/cli/args.rs:169 | tests::linux_ignores_inherited_windows_shell_markers | source inventory; review pending |
| 64 | U02 | src/cli/args.rs:176 | tests::shell_defaults_are_native | source inventory; review pending |
| 65 | U02 | src/cli/platform.rs:10 | Base | source inventory; review pending |
| 66 | U02 | src/cli/platform.rs:11 | Base.env | source inventory; review pending |
| 67 | U02 | src/cli/platform.rs:12 | Base.info | source inventory; review pending |
| 68 | U02 | src/cli/platform.rs:15 | architectures | source inventory; review pending |
| 69 | U02 | src/cli/platform.rs:24 | validate_architectures | source inventory; review pending |
| 70 | U02 | src/cli/platform.rs:51 | filter | source inventory; review pending |
| 71 | U02 | src/cli/platform.rs:73 | describe_vs | source inventory; review pending |
| 72 | U02 | src/cli/platform.rs:81 | describe | source inventory; review pending |
| 73 | U02 | src/cli/platform.rs:99 | build [Windows] | source inventory; review pending |
| 74 | U02 | src/cli/platform.rs:132 | build [non-Windows] | source inventory; review pending |
| 75 | U02 | src/cli/platform.rs:145 | inventory [Windows] | source inventory; review pending |
| 76 | U02 | src/cli/platform.rs:168 | inventory [non-Windows] | source inventory; review pending |
| 77 | U02 | src/cli/platform.rs:178 | tests::args | source inventory; review pending |
| 78 | U02 | src/cli/platform.rs:183 | tests::native_architecture_is_the_default_on_both_platforms | source inventory; review pending |
| 79 | U02 | src/cli/platform.rs:191 | tests::linux_rejects_explicit_windows_requests | source inventory; review pending |
| 80 | U02 | src/cli/platform.rs:201 | tests::unsupported_linux_cross_targets_are_rejected | source inventory; review pending |
| 81 | U02 | src/cli/platform.rs:210 | tests::all_year_inputs_and_inverted_ranges_are_checked | source inventory; review pending |
| 82 | U02 | src/cli/platform.rs:220 | tests::explicit_year_bounds_outrank_cuda_search_hints | source inventory; review pending |
| 83 | U03 | src/cli/sdks.rs:8 | Selection | source inventory; review pending |
| 84 | U03 | src/cli/sdks.rs:10 | Selection.cuda | source inventory; review pending |
| 85 | U03 | src/cli/sdks.rs:12 | Selection.optix | source inventory; review pending |
| 86 | U03 | src/cli/sdks.rs:14 | Selection.nvidia | source inventory; review pending |
| 87 | U03 | src/cli/sdks.rs:15 | Selection.vcpkg | source inventory; review pending |
| 88 | U03 | src/cli/sdks.rs:18 | invalid_root | source inventory; review pending |
| 89 | U03 | src/cli/sdks.rs:23 | configured | source inventory; review pending |
| 90 | U03 | src/cli/sdks.rs:38 | resolve | source inventory; review pending |
| 91 | U03 | src/cli/sdks.rs:102 | Selection::apply | source inventory; review pending |
| 92 | U03 | src/cli/sdks.rs:121 | Selection::cuda_bounds | source inventory; review pending |
| 93 | U03 | src/cli/sdks.rs:133 | Selection::info | source inventory; review pending |
| 94 | U03 | src/cli/sdks.rs:162 | location | source inventory; review pending |
| 95 | U03 | src/cli/sdks.rs:167 | sdk_description | source inventory; review pending |
| 96 | U03 | src/cli/sdks.rs:173 | inventory | source inventory; review pending |
| 97 | U03 | src/cli/sdks.rs:227 | tests::invalid_explicit_root_is_an_error_in_auto_and_on_modes | source inventory; review pending |
| 98 | U03 | src/cli/sdks.rs:237 | tests::absence_and_selection_follow_the_requested_mode | source inventory; review pending |
| 99 | U03 | tests/cli.rs:6 | command | source inventory; review pending |
| 100 | U03 | tests/cli.rs:28 | without_sdks | source inventory; review pending |
| 101 | U03 | tests/cli.rs:38 | missing | source inventory; review pending |
| 102 | U03 | tests/cli.rs:46 | failure | source inventory; review pending |
| 103 | U03 | tests/cli.rs:54 | help_describes_both_platforms | source inventory; review pending |
| 104 | U03 | tests/cli.rs:64 | automatic_cuda_rejects_an_invalid_explicit_root_before_any_script | source inventory; review pending |
| 105 | U03 | tests/cli.rs:72 | automatic_optix_rejects_an_invalid_explicit_root_before_any_script | source inventory; review pending |
| 106 | U03 | tests/cli.rs:82 | disabled_sdks_ignore_invalid_explicit_roots | source inventory; review pending |
| 107 | U03 | tests/cli.rs:100 | explicitly_requested_sdk_reports_unsupported_platform | source inventory; review pending |
| 108 | U03 | tests/cli.rs:113 | vcpkg_configuration_errors_do_not_emit_scripts | source inventory; review pending |
| 109 | U03 | tests/cli.rs:126 | linux_rejects_windows_options_cross_targets_and_cmd_before_output | source inventory; review pending |
| 110 | U03 | tests/cli.rs:139 | linux_json_without_sdks_uses_native_variable_names | source inventory; review pending |

## Independent review ratchet

| Question | Required review | State |
|---|---|---|
| Q1 | Unfinished code, stubs and unsupported paths | pending fresh read-only review |
| Q2 | Entry-to-leaf wiring, every flag/feature and inventory marker | pending |
| Q3 | Errors, panics and full-render-before-stdout behavior | pending |
| Q4 | Root precedence, architectures, year bounds and empty inputs | pending |
| Q5 | Runtime call boundaries and unnecessary allocation | pending |
| Q6 | Metadata/error representation and actual FFI boundaries | pending |
| Q7 | Compiler, filesystem, platform and SDK assumptions | pending |

No independent pass or zero-defect verdict exists. Repair findings and repeat a fresh review.

## Root gates and seams

- [ ] Verify D11 descriptors and D12 fallible environment/formatter contracts at every caller.
- [ ] Review all 110 manifest rows and adjacent behavior independently.
- [ ] Commit and push the coherent source to main before compiler or test commands.
- [ ] Check none/cuda/optix/nvsdk/cli/default feature combinations on Windows and Ubuntu 26.04.
- [ ] Run CLI negative/smoke tests, actual native shell round trips and a relevant seeded mutant.
- [ ] Record explicit unavailable hardware/SDK gates; ARM64 fixtures are not native execution.
- [ ] Refresh the graph and preserve exact pushed dependency identity for nv-rs.
