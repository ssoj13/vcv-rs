# D11 — SDK discovery

## Transfer status — 2026-11-13

Paused by owner for environment transfer. The [current handoff](../../../HANDOFF.md) supersedes the historical counts and statuses below. D11 has 225 statically reviewed IDs plus added unreviewed ID226; D12 has 125 IDs and an open Windows PowerShell length defect; D13's 110-ID independent review remains pending. No platform or runtime gate has run. Preserved JSON receipts and unexecuted scripts are in ../evidence/2026-11-13-transfer/.

## Historical source ledger


Updated: 2026-11-13. Status: SOURCE FROZEN — NOT VERIFIED.
See the [domain index](../MASTERMIND_INDEX.md), [seam matrix](../SEAM_MATRIX.md) and [work queue](../../../TODO.md).

## Scope and source checkpoint

Source ownership: `src/cuda.rs`, `src/optix.rs`, `src/nvsdk.rs` and `src/discovery.rs`.
The latest coordinator freeze reports 2,273 physical lines. The earlier baseline manifest
contained 173 rows; the refreshed canonical manifest below contains 185 individual rows.
The 17-row runtime repair manifest below overlaps existing symbols and must not be added to
173 as a count of unique symbols. No fresh independent Inquisitor verdict exists.

## Written contracts

- The first nonempty explicit root alias remains authoritative even when invalid. Inventory may retain alternatives; selection never silently substitutes one.
- CUDA/OptiX root aliases are exported by their owners. Version macro encoding is checked with `Option<u32>`.
- CUDA executable discovery resolves an nvcc symlink before deriving its root. Link/runtime paths are target-aware and separate.
- Passive ELF/PE metadata identifies native architecture without loading libraries. Tagged archive evidence can establish link metadata without runtime availability.
- `CudaInfo::runtime_dir(target)` accepts matching shared binaries, including versioned Linux sonames. Legacy `lib64` is native-target-only.
- NVIDIA SDK APIs take explicit targets; `SdkInfo` includes target, headers, runtime_dir and lib_dir. NCCL runtime-only sonames do not imply link artifacts.
- NVAPI is Windows-only and NCCL Linux-only. CUDA-major nvCOMP and Linux cuDNN layouts are represented without borrowing another target's libraries.

## Runtime repair manifest

These anchors reproduce the coordinator's bounded repair report. Each row needs independent review.

| # | Source | Symbol | Gate |
|---|---|---|---|
| 1 | src/cuda.rs:148 | bin_dirs | source written; review pending |
| 2 | src/cuda.rs:167 | lib_dir | source written; review pending |
| 3 | src/cuda.rs:207 | runtime_dir | source written; review pending |
| 4 | src/cuda.rs:459 | is_cuda_family | source written; review pending |
| 5 | src/cuda.rs:474 | is_cuda_library | source written; review pending |
| 6 | src/cuda.rs:483 | is_cuda_runtime_file | source written; review pending |
| 7 | src/cuda.rs:496 | has_cuda_runtime_file | source written; review pending |
| 8 | src/cuda.rs:509 | has_cuda_link_file | source written; review pending |
| 9 | src/cuda.rs:532 | has_native_cuda_library | source written; review pending |
| 10 | src/cuda.rs:782 | windows_libraries_require_matching_target_and_link_files | fixture written; not run |
| 11 | src/cuda.rs:805 | linux_tagged_libraries_do_not_fall_back_across_targets | fixture written; not run |
| 12 | src/cuda.rs:835 | tests::elf | fixture helper; review pending |
| 13 | src/cuda.rs:852 | linux_legacy_lib64_requires_native_elf_link_evidence | fixture written; not run |
| 14 | src/cuda.rs:881 | tests::pe | fixture helper; review pending |
| 15 | src/cuda.rs:897 | linux_tagged_runtime_evidence_is_separate_from_link_evidence | fixture written; not run |
| 16 | src/cuda.rs:928 | linux_legacy_runtime_requires_native_target_and_binary | fixture written; not run |
| 17 | src/cuda.rs:953 | windows_runtime_requires_matching_pe_and_eligible_bin_candidate | fixture written; not run |


## Refreshed canonical manifest — 185 rows

The coordinator supplied this refreshed inventory after the runtime repair. It supersedes the
old 173-row checkpoint as the current manifest, including four test modules and Fixture.0.
A fresh read-only Inquisitor is active; findings and verdict remain pending.

| Review unit | Source range |
|---|---|
| U11a | cuda.rs:1–545 production |
| U11b | cuda.rs:546–1006 fixtures |
| U11c | optix.rs:1–341 and discovery.rs:1–223 (564 total) |
| U11d | nvsdk.rs:1–350 |
| U11e | nvsdk.rs:351–703 |

| # | Source | Symbol | Gate |
|---|---|---|---|
| 1 | src/cuda.rs:44 | CudaVersion | independent review pending |
| 2 | src/cuda.rs:45 | CudaVersion.major | independent review pending |
| 3 | src/cuda.rs:46 | CudaVersion.minor | independent review pending |
| 4 | src/cuda.rs:51 | CudaVersion::from_macro | independent review pending |
| 5 | src/cuda.rs:59 | CudaVersion::to_macro | independent review pending |
| 6 | src/cuda.rs:76 | CudaVersion::fmt | independent review pending |
| 7 | src/cuda.rs:90 | MsvcRange | independent review pending |
| 8 | src/cuda.rs:91 | MsvcRange.min | independent review pending |
| 9 | src/cuda.rs:92 | MsvcRange.max_exclusive | independent review pending |
| 10 | src/cuda.rs:97 | MsvcRange::accepts | independent review pending |
| 11 | src/cuda.rs:106 | MsvcRange::max_vs_year | independent review pending |
| 12 | src/cuda.rs:111 | MsvcRange::min_vs_year | independent review pending |
| 13 | src/cuda.rs:121 | vs_year_for_msc | independent review pending |
| 14 | src/cuda.rs:133 | CudaInfo | independent review pending |
| 15 | src/cuda.rs:135 | CudaInfo.root | independent review pending |
| 16 | src/cuda.rs:136 | CudaInfo.version | independent review pending |
| 17 | src/cuda.rs:138 | CudaInfo.nvcc | independent review pending |
| 18 | src/cuda.rs:141 | CudaInfo.msvc | independent review pending |
| 19 | src/cuda.rs:148 | CudaInfo::bin_dirs | independent review pending |
| 20 | src/cuda.rs:158 | CudaInfo::include_dir | independent review pending |
| 21 | src/cuda.rs:167 | CudaInfo::lib_dir | independent review pending |
| 22 | src/cuda.rs:207 | CudaInfo::runtime_dir | independent review pending |
| 23 | src/cuda.rs:243 | CudaInfo::versioned_var | independent review pending |
| 24 | src/cuda.rs:254 | ROOT_VARS | independent review pending |
| 25 | src/cuda.rs:262 | root_vars | independent review pending |
| 26 | src/cuda.rs:267 | requested_root | independent review pending |
| 27 | src/cuda.rs:276 | probe | independent review pending |
| 28 | src/cuda.rs:294 | NVCC Windows | independent review pending |
| 29 | src/cuda.rs:296 | NVCC otherhosts | independent review pending |
| 30 | src/cuda.rs:299 | read_version | independent review pending |
| 31 | src/cuda.rs:316 | read_msvc_range | independent review pending |
| 32 | src/cuda.rs:348 | list_toolkits | independent review pending |
| 33 | src/cuda.rs:352 | searched_roots | independent review pending |
| 34 | src/cuda.rs:360 | inventory_with | independent review pending |
| 35 | src/cuda.rs:377 | detect_cuda | independent review pending |
| 36 | src/cuda.rs:381 | detect_with | independent review pending |
| 37 | src/cuda.rs:392 | standard_roots | independent review pending |
| 38 | src/cuda.rs:443 | nvcc_on_path | independent review pending |
| 39 | src/cuda.rs:450 | nvcc_root_from_file | independent review pending |
| 40 | src/cuda.rs:459 | is_cuda_family | independent review pending |
| 41 | src/cuda.rs:474 | is_cuda_library | independent review pending |
| 42 | src/cuda.rs:483 | is_cuda_runtime_file | independent review pending |
| 43 | src/cuda.rs:496 | has_cuda_runtime_file | independent review pending |
| 44 | src/cuda.rs:509 | has_cuda_link_file | independent review pending |
| 45 | src/cuda.rs:532 | has_native_cuda_library | independent review pending |
| 46 | src/cuda.rs:545 | tests | independent review pending |
| 47 | src/cuda.rs:549 | tests::NEXT_FIXTURE | independent review pending |
| 48 | src/cuda.rs:551 | tests::fixture | independent review pending |
| 49 | src/cuda.rs:565 | tests::version_macro_round_trips | independent review pending |
| 50 | src/cuda.rs:583 | tests::msvc_range_is_half_open | independent review pending |
| 51 | src/cuda.rs:600 | tests::msvc_guard_is_parsed_from_the_header_form | independent review pending |
| 52 | src/cuda.rs:625 | tests::fake_toolkit | independent review pending |
| 53 | src/cuda.rs:644 | tests::a_root_without_nvcc_is_not_a_toolkit | independent review pending |
| 54 | src/cuda.rs:660 | tests::any_major_version_is_read_the_same_way | independent review pending |
| 55 | src/cuda.rs:688 | tests::the_header_outranks_the_directory_name | independent review pending |
| 56 | src/cuda.rs:704 | tests::version_macro_name_must_match_exactly | independent review pending |
| 57 | src/cuda.rs:719 | tests::explicit_older_root_outranks_newer_inventory | independent review pending |
| 58 | src/cuda.rs:738 | tests::invalid_explicit_root_never_uses_valid_search_or_lower_alias | independent review pending |
| 59 | src/cuda.rs:754 | tests::root_alias_priority_skips_only_empty_values | independent review pending |
| 60 | src/cuda.rs:768 | tests::bin_directories_follow_the_host_platform | independent review pending |
| 61 | src/cuda.rs:782 | tests::windows_libraries_require_matching_target_and_link_files | independent review pending |
| 62 | src/cuda.rs:805 | tests::linux_tagged_libraries_do_not_fall_back_across_targets | independent review pending |
| 63 | src/cuda.rs:835 | tests::elf | independent review pending |
| 64 | src/cuda.rs:852 | tests::linux_legacy_lib64_requires_native_elf_link_evidence | independent review pending |
| 65 | src/cuda.rs:881 | tests::pe | independent review pending |
| 66 | src/cuda.rs:897 | tests::linux_tagged_runtime_evidence_is_separate_from_link_evidence | independent review pending |
| 67 | src/cuda.rs:928 | tests::linux_legacy_runtime_requires_native_target_and_binary | independent review pending |
| 68 | src/cuda.rs:953 | tests::windows_runtime_requires_matching_pe_and_eligible_bin_candidate | independent review pending |
| 69 | src/cuda.rs:993 | tests::nvcc_file_symlink_resolves_before_deriving_the_root | independent review pending |
| 70 | src/optix.rs:40 | OptixVersion | independent review pending |
| 71 | src/optix.rs:41 | OptixVersion.major | independent review pending |
| 72 | src/optix.rs:42 | OptixVersion.minor | independent review pending |
| 73 | src/optix.rs:43 | OptixVersion.micro | independent review pending |
| 74 | src/optix.rs:48 | OptixVersion::from_macro | independent review pending |
| 75 | src/optix.rs:57 | OptixVersion::to_macro | independent review pending |
| 76 | src/optix.rs:78 | OptixVersion::fmt | independent review pending |
| 77 | src/optix.rs:85 | OptixInfo | independent review pending |
| 78 | src/optix.rs:87 | OptixInfo.root | independent review pending |
| 79 | src/optix.rs:88 | OptixInfo.version | independent review pending |
| 80 | src/optix.rs:90 | OptixInfo.abi | independent review pending |
| 81 | src/optix.rs:92 | OptixInfo.device_impl | independent review pending |
| 82 | src/optix.rs:97 | OptixInfo::include_dir | independent review pending |
| 83 | src/optix.rs:105 | ROOT_VARS | independent review pending |
| 84 | src/optix.rs:108 | root_vars | independent review pending |
| 85 | src/optix.rs:114 | requested_root | independent review pending |
| 86 | src/optix.rs:123 | probe | independent review pending |
| 87 | src/optix.rs:137 | read_define | independent review pending |
| 88 | src/optix.rs:152 | list_sdks | independent review pending |
| 89 | src/optix.rs:156 | inventory_with | independent review pending |
| 90 | src/optix.rs:178 | detect_optix | independent review pending |
| 91 | src/optix.rs:182 | detect_with | independent review pending |
| 92 | src/optix.rs:193 | standard_roots | independent review pending |
| 93 | src/optix.rs:213 | subdirs_matching | independent review pending |
| 94 | src/optix.rs:225 | tests | independent review pending |
| 95 | src/optix.rs:229 | tests::fake_sdk | independent review pending |
| 96 | src/optix.rs:250 | tests::explicit_older_root_wins_over_newer_inventory | independent review pending |
| 97 | src/optix.rs:267 | tests::invalid_explicit_root_rejects_selection_but_inventory_keeps_alternatives | independent review pending |
| 98 | src/optix.rs:281 | tests::version_macro_round_trips | independent review pending |
| 99 | src/optix.rs:300 | tests::a_define_prefix_does_not_match_a_longer_name | independent review pending |
| 100 | src/optix.rs:321 | tests::a_root_without_the_function_table_header_is_not_an_sdk | independent review pending |
| 101 | src/optix.rs:332 | tests::the_header_outranks_the_directory_name_and_device_impl_is_reported | independent review pending |
| 102 | src/nvsdk.rs:8 | SEARCH_DEPTH | independent review pending |
| 103 | src/nvsdk.rs:12 | Kind | independent review pending |
| 104 | src/nvsdk.rs:13 | Kind::Cudnn | independent review pending |
| 105 | src/nvsdk.rs:14 | Kind::VideoCodec | independent review pending |
| 106 | src/nvsdk.rs:15 | Kind::OpticalFlow | independent review pending |
| 107 | src/nvsdk.rs:16 | Kind::Nvapi | independent review pending |
| 108 | src/nvsdk.rs:17 | Kind::Ngx | independent review pending |
| 109 | src/nvsdk.rs:18 | Kind::Nvcomp | independent review pending |
| 110 | src/nvsdk.rs:19 | Kind::Nccl | independent review pending |
| 111 | src/nvsdk.rs:24 | Kind::ALL | independent review pending |
| 112 | src/nvsdk.rs:26 | Kind::name | independent review pending |
| 113 | src/nvsdk.rs:39 | Kind::root_vars | independent review pending |
| 114 | src/nvsdk.rs:52 | Kind::requested_root | independent review pending |
| 115 | src/nvsdk.rs:57 | Kind::is_supported | independent review pending |
| 116 | src/nvsdk.rs:65 | Kind::marker | independent review pending |
| 117 | src/nvsdk.rs:77 | Kind::version_define | independent review pending |
| 118 | src/nvsdk.rs:89 | Kind::include_subdirs | independent review pending |
| 119 | src/nvsdk.rs:101 | SdkInfo | independent review pending |
| 120 | src/nvsdk.rs:102 | SdkInfo.kind | independent review pending |
| 121 | src/nvsdk.rs:103 | SdkInfo.target | independent review pending |
| 122 | src/nvsdk.rs:104 | SdkInfo.root | independent review pending |
| 123 | src/nvsdk.rs:105 | SdkInfo.include_dir | independent review pending |
| 124 | src/nvsdk.rs:106 | SdkInfo.runtime_dir | independent review pending |
| 125 | src/nvsdk.rs:107 | SdkInfo.lib_dir | independent review pending |
| 126 | src/nvsdk.rs:108 | SdkInfo.version | independent review pending |
| 127 | src/nvsdk.rs:112 | probe | independent review pending |
| 128 | src/nvsdk.rs:116 | selected_cuda | independent review pending |
| 129 | src/nvsdk.rs:128 | probe_with | independent review pending |
| 130 | src/nvsdk.rs:138 | include_dir | independent review pending |
| 131 | src/nvsdk.rs:161 | pick_cuda_versioned | independent review pending |
| 132 | src/nvsdk.rs:178 | native_tags | independent review pending |
| 133 | src/nvsdk.rs:189 | relevant_library | independent review pending |
| 134 | src/nvsdk.rs:202 | has_library | independent review pending |
| 135 | src/nvsdk.rs:222 | runtime_dirs | independent review pending |
| 136 | src/nvsdk.rs:287 | read_version | independent review pending |
| 137 | src/nvsdk.rs:294 | read_define | independent review pending |
| 138 | src/nvsdk.rs:306 | search_parents | independent review pending |
| 139 | src/nvsdk.rs:326 | list | independent review pending |
| 140 | src/nvsdk.rs:331 | list_with | independent review pending |
| 141 | src/nvsdk.rs:335 | inventory | independent review pending |
| 142 | src/nvsdk.rs:367 | detect | independent review pending |
| 143 | src/nvsdk.rs:371 | detect_with | independent review pending |
| 144 | src/nvsdk.rs:381 | matches_installed_cuda | independent review pending |
| 145 | src/nvsdk.rs:396 | numeric_part | independent review pending |
| 146 | src/nvsdk.rs:404 | version_key | independent review pending |
| 147 | src/nvsdk.rs:409 | walk | independent review pending |
| 148 | src/nvsdk.rs:427 | tests | independent review pending |
| 149 | src/nvsdk.rs:431 | tests::NEXT_FIXTURE | independent review pending |
| 150 | src/nvsdk.rs:433 | tests::Fixture | independent review pending |
| 151 | src/nvsdk.rs:433 | tests::Fixture.0 | independent review pending |
| 152 | src/nvsdk.rs:436 | tests::Fixture::new | independent review pending |
| 153 | src/nvsdk.rs:446 | tests::Fixture::drop | independent review pending |
| 154 | src/nvsdk.rs:451 | tests::write | independent review pending |
| 155 | src/nvsdk.rs:455 | tests::write_bytes | independent review pending |
| 156 | src/nvsdk.rs:460 | tests::elf | independent review pending |
| 157 | src/nvsdk.rs:471 | tests::pe | independent review pending |
| 158 | src/nvsdk.rs:481 | tests::cudnn | independent review pending |
| 159 | src/nvsdk.rs:489 | tests::cudnn_versioned_layout_is_found_under_a_parent | independent review pending |
| 160 | src/nvsdk.rs:502 | tests::header_only_sdks_report_their_header_directory_and_version | independent review pending |
| 161 | src/nvsdk.rs:521 | tests::a_define_prefix_does_not_match_a_longer_name | independent review pending |
| 162 | src/nvsdk.rs:538 | tests::invalid_explicit_root_never_selects_an_alternative | independent review pending |
| 163 | src/nvsdk.rs:553 | tests::named_roots_and_cuda_compatibility_have_separate_precedence | independent review pending |
| 164 | src/nvsdk.rs:568 | tests::nvcomp_major_layout_uses_selected_cuda_and_verified_architecture | independent review pending |
| 165 | src/nvsdk.rs:587 | tests::flat_libraries_require_physical_target_headers | independent review pending |
| 166 | src/nvsdk.rs:616 | tests::tagged_dynamic_files_still_require_native_binary_headers | independent review pending |
| 167 | src/nvsdk.rs:643 | tests::linux_cudnn_uses_flat_or_multiarch_headers | independent review pending |
| 168 | src/nvsdk.rs:658 | tests::tagged_sdk_folders_respect_platform_and_target | independent review pending |
| 169 | src/nvsdk.rs:678 | tests::nccl_versioned_soname_is_sufficient_native_library_evidence | independent review pending |
| 170 | src/nvsdk.rs:696 | tests::nvapi_is_unsupported_on_linux_even_with_headers | independent review pending |
| 171 | src/discovery.rs:13 | RootRequest | independent review pending |
| 172 | src/discovery.rs:15 | RootRequest.variable | independent review pending |
| 173 | src/discovery.rs:17 | RootRequest.path | independent review pending |
| 174 | src/discovery.rs:21 | first_root | independent review pending |
| 175 | src/discovery.rs:27 | first_root_with | independent review pending |
| 176 | src/discovery.rs:47 | binary_arch | independent review pending |
| 177 | src/discovery.rs:99 | tests | independent review pending |
| 178 | src/discovery.rs:103 | tests::NEXT_FIXTURE | independent review pending |
| 179 | src/discovery.rs:105 | tests::check_binary | independent review pending |
| 180 | src/discovery.rs:118 | tests::elf | independent review pending |
| 181 | src/discovery.rs:128 | tests::pe | independent review pending |
| 182 | src/discovery.rs:138 | tests::elf_machine_class_and_encoding_must_agree | independent review pending |
| 183 | src/discovery.rs:160 | tests::pe_machine_and_checked_header_bounds_are_required | independent review pending |
| 184 | src/discovery.rs:182 | tests::first_nonempty_root_is_authoritative_even_when_invalid | independent review pending |
| 185 | src/discovery.rs:202 | tests::root_values_are_preserved_without_trimming_or_unicode_conversion | independent review pending |

## Fresh review findings

| Finding | Reported source | Required action | State |
|---|---|---|---|
| D11-R1 | optix.rs:302–303, 324–325, 333–334 | Replace globally fixed temporary roots and recursive pre-delete with unique process/time/counter-owned fixtures; clean up only owned roots and report failures | open; repair and re-review pending |

The fresh reviewer reports that concurrent binaries/worktrees can remove each other's OptiX
fixtures. This is an open finding, not an approved fixture design. The complete 185-symbol audit
and any further findings remain pending; later repair/verdict updates belong to a separate checkpoint.

## Review and root gates

- [ ] Independently review the refreshed 185-row manifest, preserving every original symbol and repair.
- [ ] Run a fresh read-only Inquisitor through Q1 stubs, Q2 wiring, Q3 errors/panics, Q4 boundaries, Q5 allocation, Q6 layout and Q7 external assumptions.
- [ ] Review D12 consumption of runtime_dir independently from lib_dir and D13 reuse of selected descriptors.
- [ ] Compare actual install layouts and library headers against fixtures; passive metadata is not ABI/runtime proof.
- [ ] Commit/push main before any compiler/test command.
- [ ] Run independent Windows and Ubuntu 26.04 feature, fixture and native execution gates with exact receipts.
- [ ] Keep ARM64 fixture coverage distinct from native hardware execution.
- [ ] Refresh the final graph and verify the nv-rs dependency consumes the pushed revision.
