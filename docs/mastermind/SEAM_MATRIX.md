# Windows/Linux seam matrix

## Transfer status — 2026-11-13

Paused by owner for environment transfer. The [current handoff](../../HANDOFF.md) supersedes the historical counts and statuses below. D11 has 225 statically reviewed IDs plus added unreviewed ID226; D12 has 125 IDs and an open Windows PowerShell length defect; D13's 110-ID independent review remains pending. No platform or runtime gate has run. Preserved JSON receipts and unexecuted scripts are in evidence/2026-11-13-transfer/.

## Historical source ledger


Updated: 2026-11-13. Source checkpoints are recorded below; every seam remains unverified.
See the [domain index](MASTERMIND_INDEX.md) and [work queue](../../TODO.md).

| Seam | Producer | Consumer | Required contract | Status |
|---|---|---|---|---|
| V01 | cuda.rs:requested_root/detect_cuda; optix.rs:requested_root/detect_optix | Discovery selection and CLI | First nonempty configured root is authoritative, including invalid roots; inventory sorting cannot replace it | written; review/tests pending |
| V02 | cuda.rs:probe/lib_dir/runtime_dir; discovery.rs:binary_arch | env.rs:add_cuda and CLI | Version, target and native library evidence agree; static-only link metadata must not advertise runtime | runtime-proof repair written; review/execution pending |
| V03 | nvsdk.rs:detect(kind,target)/SdkInfo; optix.rs:detect_optix | env.rs:add_nvsdk; nv-rs nv-bindgen.rs:sdk_paths | One owner selects headers, target, runtime_dir and lib_dir; runtime-only sonames do not imply link artifacts | written; consumer/review/tests pending |
| V04 | Selected SDK/VcpkgInfo metadata | env.rs:add_cuda/add_optix/add_nvsdk/add_vcpkg | Preserve raw values, target identity and chosen aliases; normalize the selected vcpkg triplet | selected-triplet/runtime repairs written; review/execution pending |
| V05 | env.rs:Env | format.rs:fmt_cmd/fmt_ps/fmt_sh/fmt_json; tests/format_roundtrip.rs | Native names/separators, literal values and typed rejection; CMD generation-time snapshot versus PS/sh apply-time merge | source frozen; independent review/native execution pending |
| V06 | lib.rs:Arch/common exports; Cargo features | main.rs and CLI platform/selection paths | Real Windows/Linux entry points and optional clap; unsupported explicit requests error | D13 source frozen; independent review/feature matrix pending |
| V07 | Pushed vcv-rs main revision | nv-rs discovery/generation | Consumer resolves the reviewed revision and uses the same SDK/path semantics as the CLI | pending |
| V08 | Reviewed source and fixtures | Root Windows/Ubuntu gate receipts | All involved repos pushed first; root checks CPU/RAM/GPU/other builds and serializes gates by machine load; exact exit codes, features and environment versions recorded | pending |

## Required evidence

Owner instruction on 2026-11-13 overrides the absent build-lock requirement for this task: root monitors CPU/RAM/GPU load and other build processes, then serializes Cargo/check/test runs. No replacement lock or further permission is required.

- [ ] Replace producer/consumer descriptions with exact file/symbol locations after domain assignment.
- [ ] Verify explicit selection, invalid selection, missing installation and version-boundary fixtures.
- [ ] Verify symlink and architecture-specific Linux layouts against installed references.
- [ ] Verify Windows SDK/MSVC selection remains compatible with CUDA's header constraints.
- [ ] Verify formatter outputs by round-trip tests in their actual supported shells.
- [ ] Verify no fallback hides a failed explicit request or unsupported platform.
- [ ] Verify feature-gated callers and pushed cross-repository dependency identity.
- [ ] Attach fresh independent review, seeded-mutant and both-platform gate receipts before approval.

Path descriptors and environment strings have no coordinate or GPU-row-pitch contract. Check type
identity, ownership and platform representation at these boundaries; apply FFI layout checks only
where a real binary API payload crosses the seam. No test result is asserted here.
