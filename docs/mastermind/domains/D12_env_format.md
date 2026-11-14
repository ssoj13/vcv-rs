# D12 — environment and formatting

## Transfer status — 2026-11-13

Paused by owner for environment transfer. The [current handoff](../../../HANDOFF.md) supersedes the historical counts and statuses below. D11 has 225 statically reviewed IDs plus added unreviewed ID226; D12 has 125 IDs and an open Windows PowerShell length defect; D13's 110-ID independent review remains pending. No platform or runtime gate has run. Preserved JSON receipts and unexecuted scripts are in ../evidence/2026-11-13-transfer/.

## Historical source ledger


Updated: 2026-11-13. Status: SOURCE FROZEN — NOT VERIFIED.
See the [domain index](../MASTERMIND_INDEX.md), [seam matrix](../SEAM_MATRIX.md) and [work queue](../../../TODO.md).

## Scope and source checkpoint

Source ownership: `src/env.rs` (757 lines), `src/format.rs` (465) and
`tests/format_roundtrip.rs` (360); coordinator-reported total 1,582 physical lines.
The coordinator completed bounded source readback. No compiler, test, formatter, shell or git
gate was executed by this domain, and no fresh independent review verdict exists.

Review logical units separately: env foundation/build_env (1–249), SDK/vcpkg composition
(250–443), env fixtures (444–757), formatter (1–465), and shell harness (1–360).
Each window stays below 600 lines; these suggested review windows are not prior Inquisitor passes.

## Written contracts and repairs

- `Env::new(target)` carries explicit target identity, separate executable/runtime paths and raw `OsString` variables.
- `add_cuda`, `add_nvsdk` and `add_vcpkg` return `Result<(), EnvError>`; `add_optix` adds headers/aliases without a fallible return.
- CUDA runtime metadata comes only from `CudaInfo::runtime_dir(target)`, independently of link metadata. Selected vcpkg triplets are emitted through `VCPKG_DEFAULT_TRIPLET`.
- All four formatters return `Result<String, FormatError>`; validation precedes complete CLI output.
- Linux maps executable/runtime/include/link paths to PATH/LD_LIBRARY_PATH/CPATH/LIBRARY_PATH. Windows uses PATH/INCLUDE/LIB/LIBPATH.
- CMD captures generator-time inherited lists, removes exact-string duplicates, and retains at most one preexisting empty component. It rejects generated empty paths, unsafe expansion syntax, non-ASCII values and overlong assignments.
- PowerShell/POSIX output merges apply-time values and preserves literal quoting; the committed test-only mutation branch perturbs generated test scripts.
- The CMD harness uses an isolated renderer child and a different apply-time environment: five snapshot outcomes, all nonempty native list groups, PATH/runtime deduplication and 32 unsafe inherited-list cases are written but unexecuted.
- Test fixture cleanup reports failures without a second panic during unwinding. That test-only fallback needs explicit Q3 review.

## Numbered manifests

The following 112 individual rows reproduce the coordinator's final manifests: 49 environment,
39 formatter and 24 harness rows. Every verdict remains pending; line anchors may move in repairs.

### Environment — 49 rows

| # | Source | Symbol | Gate |
|---|---|---|---|
| 1 | src/env.rs:37 | Env | written; independent review/execution pending |
| 2 | src/env.rs:48 | Env::new | written; independent review/execution pending |
| 3 | src/env.rs:60 | Env::add_if_exists | written; independent review/execution pending |
| 4 | src/env.rs:71 | EnvError | written; independent review/execution pending |
| 5 | src/env.rs:72 | EnvError::TargetMismatch | written; independent review/execution pending |
| 6 | src/env.rs:73 | EnvError::MissingCudaLibrary | written; independent review/execution pending |
| 7 | src/env.rs:74 | EnvError::UnsupportedSdk | written; independent review/execution pending |
| 8 | src/env.rs:75 | EnvError::UnsupportedVcpkgTriplet | written; independent review/execution pending |
| 9 | src/env.rs:79 | EnvError Display::fmt | written; independent review/execution pending |
| 10 | src/env.rs:93 | EnvError Error implementation | written; independent review/execution pending |
| 11 | src/env.rs:96 | with_trailing_backslash | written; independent review/execution pending |
| 12 | src/env.rs:104 | build_env | written; independent review/execution pending |
| 13 | src/env.rs:236 | program_files_if_missing | written; independent review/execution pending |
| 14 | src/env.rs:253 | add_cuda | written; independent review/execution pending |
| 15 | src/env.rs:271 | add_optix | written; independent review/execution pending |
| 16 | src/env.rs:280 | add_nvsdk | written; independent review/execution pending |
| 17 | src/env.rs:303 | VcpkgInfo | written; independent review/execution pending |
| 18 | src/env.rs:313 | requested_vcpkg_root | written; independent review/execution pending |
| 19 | src/env.rs:318 | requested_vcpkg_triplet | written; independent review/execution pending |
| 20 | src/env.rs:323 | detect_vcpkg | written; independent review/execution pending |
| 21 | src/env.rs:328 | vcpkg_triplet_candidates | written; independent review/execution pending |
| 22 | src/env.rs:350 | probe_vcpkg | written; independent review/execution pending |
| 23 | src/env.rs:358 | probe_vcpkg_with | written; independent review/execution pending |
| 24 | src/env.rs:386 | vcpkg_runtime_candidate | written; independent review/execution pending |
| 25 | src/env.rs:407 | has_vcpkg_runtime | written; independent review/execution pending |
| 26 | src/env.rs:418 | add_vcpkg | written; independent review/execution pending |
| 27 | src/env.rs:444 | tests::write_dir | written; independent review/execution pending |
| 28 | src/env.rs:448 | tests::installed | written; independent review/execution pending |
| 29 | src/env.rs:457 | tests::probe_prefers_default_triplet_when_installed | written; independent review/execution pending |
| 30 | src/env.rs:468 | tests::probe_falls_back_to_x64_windows | written; independent review/execution pending |
| 31 | src/env.rs:478 | tests::program_files_only_when_unset_and_present | written; independent review/execution pending |
| 32 | src/env.rs:485 | tests::explicit_triplets_never_fall_back | written; independent review/execution pending |
| 33 | src/env.rs:498 | tests::linux_triplets_are_target_and_platform_specific | written; independent review/execution pending |
| 34 | src/env.rs:511 | tests::probe_requires_headers_and_libraries | written; independent review/execution pending |
| 35 | src/env.rs:519 | tests::runtime_requires_native_shared_binary_evidence | written; independent review/execution pending |
| 36 | src/env.rs:542 | tests::raw_root | written; independent review/execution pending |
| 37 | src/env.rs:558 | tests::non_unicode_triplet_is_invalid | written; independent review/execution pending |
| 38 | src/env.rs:563 | tests::vcpkg_fixture | written; independent review/execution pending |
| 39 | src/env.rs:575 | tests::vcpkg_mismatch_does_not_mutate | written; independent review/execution pending |
| 40 | src/env.rs:586 | tests::vcpkg_add_deduplicates_and_preserves_raw_root | written; independent review/execution pending |
| 41 | src/env.rs:610 | tests::vcpkg_invalid_triplet_does_not_mutate | written; independent review/execution pending |
| 42 | src/env.rs:623 | tests::nvsdk_mismatch_does_not_mutate | written; independent review/execution pending |
| 43 | src/env.rs:636 | tests::nvsdk_header_only_and_runtime_only_stay_distinct | written; independent review/execution pending |
| 44 | src/env.rs:658 | tests::optix_sets_every_alias_without_loss | written; independent review/execution pending |
| 45 | src/env.rs:672 | tests::cuda_runtime_requires_matching_shared_library | written; independent review/execution pending |
| 46 | src/env.rs:674 | local NEXT_FIXTURE | written; independent review/execution pending |
| 47 | src/env.rs:675 | local Fixture | written; independent review/execution pending |
| 48 | src/env.rs:677 | Fixture::drop | written; independent review/execution pending |
| 49 | src/env.rs:745 | tests::cuda_missing_target_library_does_not_mutate | written; independent review/execution pending |

### Formatter — 39 rows

| # | Source | Symbol | Gate |
|---|---|---|---|
| 1 | src/format.rs:14 | FormatError | written; independent review/execution pending |
| 2 | src/format.rs:15 | FormatError::Unsupported | written; independent review/execution pending |
| 3 | src/format.rs:16 | FormatError::NonUnicode | written; independent review/execution pending |
| 4 | src/format.rs:17 | FormatError::InvalidKey | written; independent review/execution pending |
| 5 | src/format.rs:18 | FormatError::InvalidValue | written; independent review/execution pending |
| 6 | src/format.rs:19 | FormatError::Json | written; independent review/execution pending |
| 7 | src/format.rs:23 | FormatError Display::fmt | written; independent review/execution pending |
| 8 | src/format.rs:35 | FormatError Error::source | written; independent review/execution pending |
| 9 | src/format.rs:43 | invalid | written; independent review/execution pending |
| 10 | src/format.rs:47 | unicode | written; independent review/execution pending |
| 11 | src/format.rs:51 | validate_text | written; independent review/execution pending |
| 12 | src/format.rs:58 | validate_key | written; independent review/execution pending |
| 13 | src/format.rs:65 | dedup | written; independent review/execution pending |
| 14 | src/format.rs:70 | path_values | written; independent review/execution pending |
| 15 | src/format.rs:86 | NativeEnv | written; independent review/execution pending |
| 16 | src/format.rs:92 | native_env | written; independent review/execution pending |
| 17 | src/format.rs:132 | native_platform | written; independent review/execution pending |
| 18 | src/format.rs:150 | fmt_cmd | written; independent review/execution pending |
| 19 | src/format.rs:154 | fmt_cmd_with | written; independent review/execution pending |
| 20 | src/format.rs:183 | cmd_assignment | written; independent review/execution pending |
| 21 | src/format.rs:196 | ps_quote | written; independent review/execution pending |
| 22 | src/format.rs:209 | fmt_ps | written; independent review/execution pending |
| 23 | src/format.rs:232 | win_to_unix | written; independent review/execution pending |
| 24 | src/format.rs:253 | sh_quote | written; independent review/execution pending |
| 25 | src/format.rs:257 | sh_key | written; independent review/execution pending |
| 26 | src/format.rs:264 | fmt_sh | written; independent review/execution pending |
| 27 | src/format.rs:293 | fmt_json | written; independent review/execution pending |
| 28 | src/format.rs:312 | tests::cmd_env | written; independent review/execution pending |
| 29 | src/format.rs:319 | tests::cmd_snapshot_handles_empty_and_deduplicates_existing | written; independent review/execution pending |
| 30 | src/format.rs:336 | tests::cmd_rejects_unsafe_inherited_snapshot | written; independent review/execution pending |
| 31 | src/format.rs:345 | tests::cmd_quoted_metacharacters_and_program_files_key_are_literal | written; independent review/execution pending |
| 32 | src/format.rs:353 | tests::cmd_line_length_includes_assignment_syntax | written; independent review/execution pending |
| 33 | src/format.rs:362 | tests::drive_conversion_is_checked_and_unicode_safe | written; independent review/execution pending |
| 34 | src/format.rs:376 | tests::powershell_doubles_ascii_and_smart_single_quotes | written; independent review/execution pending |
| 35 | src/format.rs:381 | tests::native_mapping_separates_linux_runtime_and_rejects_libpath | written; independent review/execution pending |
| 36 | src/format.rs:400 | tests::powershell_on_linux_uses_native_names_and_separator | written; independent review/execution pending |
| 37 | src/format.rs:413 | tests::scalar_aliases_and_generated_keys_are_rejected_even_for_empty_lists | written; independent review/execution pending |
| 38 | src/format.rs:427 | tests::keys_paths_and_control_values_have_explicit_errors | written; independent review/execution pending |
| 39 | src/format.rs:447 | tests::non_unicode_paths_and_scalars_are_rejected_without_loss | written; independent review/execution pending |

### Native shell harness — 24 rows

| # | Source | Symbol | Gate |
|---|---|---|---|
| 1 | tests/format_roundtrip.rs:9 | NEXT | written; independent review/execution pending |
| 2 | tests/format_roundtrip.rs:11 | Scratch | written; independent review/execution pending |
| 3 | tests/format_roundtrip.rs:14 | Scratch::new | written; independent review/execution pending |
| 4 | tests/format_roundtrip.rs:26 | Scratch::drop | written; independent review/execution pending |
| 5 | tests/format_roundtrip.rs:34 | fixture | written; independent review/execution pending |
| 6 | tests/format_roundtrip.rs:56 | ps_quote | written; independent review/execution pending |
| 7 | tests/format_roundtrip.rs:61 | sh_quote | written; independent review/execution pending |
| 8 | tests/format_roundtrip.rs:65 | read_dump | written; independent review/execution pending |
| 9 | tests/format_roundtrip.rs:69 | child_command | written; independent review/execution pending |
| 10 | tests/format_roundtrip.rs:81 | dump_environment | written; independent review/execution pending |
| 11 | tests/format_roundtrip.rs:92 | json_has_native_paths_and_literal_values | written; independent review/execution pending |
| 12 | tests/format_roundtrip.rs:110 | format_rejects_reserved_keys_and_empty_path_components | written; independent review/execution pending |
| 13 | tests/format_roundtrip.rs:126 | format_rejects_control_values_and_separator_paths | written; independent review/execution pending |
| 14 | tests/format_roundtrip.rs:140 | linux_rejects_cmd_assemblies_and_non_unicode | written; independent review/execution pending |
| 15 | tests/format_roundtrip.rs:162 | windows_non_unicode_path_is_rejected | written; independent review/execution pending |
| 16 | tests/format_roundtrip.rs:174 | windows_key_rules_and_cmd_unsafe_values | written; independent review/execution pending |
| 17 | tests/format_roundtrip.rs:192 | sh_shell_roundtrip | written; independent review/execution pending |
| 18 | tests/format_roundtrip.rs:221 | cmd_fixture | written; independent review/execution pending |
| 19 | tests/format_roundtrip.rs:233 | render_cmd_environment | written; independent review/execution pending |
| 20 | tests/format_roundtrip.rs:243 | render_cmd_snapshot | written; independent review/execution pending |
| 21 | tests/format_roundtrip.rs:260 | cmd_shell_roundtrip | written; independent review/execution pending |
| 22 | tests/format_roundtrip.rs:302 | cmd_rejects_unsafe_generator_lists_without_script | written; independent review/execution pending |
| 23 | tests/format_roundtrip.rs:316 | windows_dump_normalizes_environment_key_case | written; independent review/execution pending |
| 24 | tests/format_roundtrip.rs:334 | powershell_shell_roundtrip | written; independent review/execution pending |

## Independent review ratchet

- [ ] Q1: unfinished code, simplified behavior and unsupported output paths.
- [ ] Q2: every environment field, formatter, alias and CLI consumer is wired.
- [ ] Q3: errors/panics, optional runtime evidence read failures, and test-only best-effort cleanup.
- [ ] Q4: target mismatch, invalid triplets/roots, empty lists, separators, Unicode and command-line limits.
- [ ] Q5: runtime boundary allocation and unnecessary intermediate work.
- [ ] Q6: metadata/error layouts and actual binary boundary assumptions.
- [ ] Q7: native shell quoting, process inheritance, filesystem evidence and explicit unsupported requests.

Review all 112 rows independently; repair and repeat review. No zero-defect approval is implied
by closing the source-repair queue.

## Root gates and seams

- [ ] Verify D11 runtime/link metadata and D13 selection/application/full-render callers.
- [ ] Commit and push coherent main source before compiler/test commands.
- [ ] Run the feature matrix and separate Windows/Ubuntu 26.04 gates.
- [ ] Execute native CMD/PowerShell/POSIX round trips and JSON/negative cases.
- [ ] Run `VCV_TEST_QUOTE_MUTANT=1` for expected red, then the normal gate for green.
- [ ] Refresh the graph and record exact logs, exit codes and pushed revisions.
- [ ] Keep ARM64 fixtures separate from native hardware execution.
