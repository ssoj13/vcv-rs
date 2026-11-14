# vcv-rs — Windows and Linux work queue

## Transfer checkpoint — 2026-11-13 (Codex)

**Status: paused by owner; switch environments.** The owner stopped implementation and all task agents, requested progress updates, and explicitly requested a push to main. Only documentation, evidence preservation, dependency-lock metadata and checkpoint publication continue. Do not resume automatically.

- D11: initial 195-row review exposed four additional selection/archive defects; repairs and archive/member extent closure reached 225 statically reviewed IDs. The final extent reviewer approved the affected IDs 201–225 and all seven questions, source-only. Physical format/architecture proof does not establish link, loader, symbol/relocation or ABI correctness.
- D11 ID 226, `discovery.rs::tests::actual_sdk_archives_require_native_x64_members` (attributes 1040–1041, body 1042–1065), was added last: **not formatted after insertion, not independently reviewed, not executed**. Source SHA-256: `96548facd5a14bbb45ced776514161a6ea2764620b7aaebc1cd7f86321b8ab72`. The ignored test requires real COFF/ELF files through `VCV_TEST_COFF_ARCHIVE` and `VCV_TEST_ELF_ARCHIVE`; missing inputs fail, no silent skip.
- D12: 125 manifest IDs (E49 + F39 + H24 + A13). Empty-scalar typed rejection, Windows native ordinal key comparison, scratch ownership and native shell harness repairs are written. Focused external Unicode review approved ten symbols, source-only. Full F/H 76-row review was interrupted without a final verdict; E49 independent review was not started.
- **Confirmed open D12 defect:** `fmt_ps` accepts oversized Windows/.NET environment keys/values/joined lists; a later setter can fail after earlier mutations. Repair with the documented UTF-16 length boundary and preflight of known generated/apply-time joined values. No universal Linux ARG_MAX or OOM transaction guarantee is implied.
- D13: 110-ID source map prepared; no confirmed production defect in mapping, but independent review not started (80/30 partition). Compiler compatibility remains VS-year hints plus executable existence, not an executed CUDA compatibility proof.
- Minor queued source-doc discrepancy: `src/cuda.rs` calls the nvcc path absolute although relative input roots can remain relative.
- **No build, check, test, shell round trip, mutant or GPU gate has run for this wave. Windows/Linux gates: 0/2.** Earlier rustfmt and Cargo lock metadata resolution are not execution evidence. The final ID226 insertion has no formatter receipt.
- Task agents have stopped/completed; the recovery documentation agent failed OAuth and supplied no result. External focused reviewer run18 is stopped; no own active external child run remains. No new agents or chat wait were started after stop.

The durable JSON receipts and prepared validation scripts are in `docs/mastermind/evidence/2026-11-13-transfer/`. They are historical per-freeze records; the transfer checkpoint supersedes their stale running/pending metadata. Prepared scripts are **unexecuted** and contain paths from the old machine. NVIDIA SDK archives/headers were excluded. Re-establish tools, paths, manifests and fresh source hashes in the next environment.


## Resume plan (only after owner resumes)

- [x] Preserve written D11/D12/D13 repairs and durable source-only receipts.
- [x] Record remaining defects, interrupted reviews and unexecuted gates.
- [x] Stop task agents for environment transfer.
- [ ] Finish ID226 formatting and independent review; regenerate current bounded manifests.
- [ ] Repair/review the D12 Windows/.NET length failure and review all E/F/H/A rows.
- [ ] Independently review D13's 110 rows and all live consumer seams.
- [ ] Commit/push all further changes to main before any build/check/test, with DCO sign-off.
- [ ] Root monitors CPU/RAM/GPU/other builds and runs serialized Windows and Linux gates: all 16 feature powersets, check/clippy/full tests, native shell round trips, seeded mutant, actual SDK archive and host C/CUDA smoke gates.
- [ ] Verify nv-rs host/device dependency pins and affected generator/SDK consumers.
- [ ] Refresh GitNexus after changes; mark complete only with separate Windows/Linux evidence.

See [HANDOFF.md](HANDOFF.md) for exact continuation and evidence limits.

## Historical recovery queue (superseded by transfer checkpoint)

Updated: 2026-11-13. Priority: complete Windows/Linux support before resuming nv-rs M2 verification.
See the [domain index](docs/mastermind/MASTERMIND_INDEX.md), [seam matrix](docs/mastermind/SEAM_MATRIX.md)
and [current README](README.md).

## Current step

Status: in_progress — complete Windows/Linux support under sysgo/mastermind, repair confirmed findings,
then obtain independent review, push and verify. Recovery confirmed local `main` at `6eb28c0`
and nv-rs `main` at `8b7e13c`, both with uncommitted work. D11/D12/D13 source ownership is
partitioned; D11 discovery and D12 formatter/harness repairs are active after the prior source freeze.
No source push, compiler/test gate or completed fresh independent Inquisitor verdict exists for this wave.
The README describes current source APIs and migration; its examples are not execution receipts.

## Current source checkpoint

- D11: `src/cuda.rs`, `src/optix.rs`, `src/nvsdk.rs` and `src/discovery.rs` contain authoritative explicit-root selection, target-aware Windows/Linux paths, passive native ELF/PE checks, NCCL and separate runtime/link metadata. The prior source-freeze report counted 2,273 physical lines, including separate CUDA runtime proof; repairs are active and this is not a passed platform gate.
- D12: explicit-target/raw-OsString environments, fallible formatters and the expanded native shell harness had a prior source-freeze report of 1,582 physical lines and 112 individual manifest rows; formatter/harness repairs are active. See the [environment/formatter ledger](docs/mastermind/domains/D12_env_format.md). All fixtures are unexecuted.
- D13: common Arch mapping, optional CLI dependencies, real Windows/Linux entry points and shared SDK selection are wired. Static mapping has identified no production defect so far; this is not approval. All 110 rows in the [CLI ledger](docs/mastermind/domains/D13_cli.md) still require independent review.
- Root's nv-rs adapter selects one SDK snapshot for headers/runtime/link paths, checks the Cargo target architecture and path representation, and tracks discovery aliases as rebuild inputs. The new vcv-rs revision has not been pushed or consumed by a checked nv-rs build.

## Open source repairs

- [ ] Review and execute the written CUDA runtime_dir/add_cuda repair; static-only link metadata must not imply runtime availability.
- [ ] Review and execute selected VCPKG_DEFAULT_TRIPLET propagation and its downstream-selection regression.
- [ ] Independently review the written CMD list-snapshot harness after its ownership repair, then run its five snapshot outcomes and 32 unsafe inherited-list cases natively after the source push.
- [ ] Execute the pinned CMD empty-component fixture; the formatter documents at-most-once retention of inherited empty components and rejection of generated empty entries.
- [ ] Independently review the written D11/D12/D13 live callers and repair every finding before approval.
- [ ] Complete repairs/re-review for the four confirmed D11 root causes: OptiX fixture ownership, NGX runtime names/package paths, nested Windows NGX import libraries, and case-insensitive Windows CUDA .LIB suffixes. The OptiX worker has written owned-RAII fixture cleanup and a regression; independent review and execution are pending. The final 185-row audit/verdict remains pending.
- [ ] Repair the confirmed D11/D12 seam: runtime-only CUDA `bin/x64` must not reach PATH through `bin_dirs` without target proof. D11 owns this repair.
- [ ] Replace D12 shell Scratch's predictable PID/counter `create_dir_all` ownership: a pre-existing stale directory can be claimed and deleted. Repair and regression evidence are pending.
- [ ] Preserve empty scalar environment values or reject them explicitly in portable formatters. CMD `set name=` and Windows PowerShell 5.1 `SetEnvironmentVariable` delete the variable; modern PowerShell 7.5+ differs. The confirmed representability repair remains pending.
- [ ] Root monitors CPU/RAM/GPU load and other build processes, then runs Cargo gates serially under the owner's 2026-11-13 instruction; an absent build-lock script is not a blocker for this task.

## Execution rule

Every repository involved in this wave must be committed and pushed to `main` before **any**
build, check or test command. This includes vcv-rs, nv-rs and changed shared dependencies.
Only root runs serialized compiler/test gates according to machine load and other active builds.
Owner instruction on 2026-11-13 overrides the absent build-lock requirement for this task: root monitors CPU/RAM/GPU load and other build processes, then serializes Cargo/check/test runs. No replacement lock or further permission is required.
Finish edits first, preserve others' changes, and use DCO sign-off without agent co-author footers.

## Discovery and contracts

- [ ] Read applicable instructions and map all Windows-only gates, Linux stubs and consumers.
- [ ] Assign exact disjoint domain write scopes and numbered symbol manifests; keep domain/micro-unit size limits.
- [ ] Define supported Windows and Linux architecture/toolchain combinations from actual install layouts.
- [ ] Verify environment-variable precedence, explicit roots and installed-version selection.
- [ ] Fix CUDA and OptiX selection so version sorting cannot override an explicitly selected root; add regression fixtures.
- [ ] Verify CUDA header version and host-compiler compatibility detection against installed headers.
- [ ] Resolve CUDA executable, include, library and runtime paths on Windows and Linux, including multiarch layouts and symlinks.
- [ ] Make vcv-rs the single discovery owner consumed by nv-rs; avoid duplicate SDK scans.

## Implementation

- [ ] Complete Linux CUDA discovery and environment construction without a success-shaped stub.
- [ ] Preserve and verify Windows Visual Studio/MSVC, SDK/UCRT and CUDA selection behavior.
- [ ] Implement consistent Windows/Linux OptiX and separately shipped NVIDIA SDK discovery.
- [ ] Cover nvCOMP, NCCL and cuFile locations needed by nv-rs; preserve explicit unsupported-platform errors.
- [ ] Provide real CLI behavior on both systems with concise help, aliases and honest selected-toolchain output.
- [ ] Preserve user environment values while applying documented path precedence and deduplication.
- [ ] Verify PowerShell, CMD, POSIX shell and JSON emission, including spaces, quotes, separators and metacharacters.
- [ ] Verify all relevant feature combinations, including library-only and individual CUDA/OptiX/SDK features.
- [ ] Remove obsolete platform stubs and duplicated paths only after their replacements are wired end to end.

## Verification gates

- [ ] Add reference/fixture tests for discovery, version selection, path layouts and missing/invalid installations.
- [ ] Add negative tests for explicit unsupported requests and malformed environment input.
- [ ] Add quoting/environment round-trip tests for every supported formatter.
- [ ] Run a fresh independent reviewer per domain against its numbered manifest and all seven Inquisitor questions; repair every finding and repeat review.
- [ ] Verify discovery, CLI, environment and formatter contracts in the seam matrix.
- [ ] Commit and push all involved repositories to main before starting any build/check/test.
- [ ] Run narrow package/feature gates after complete edits, with captured exit codes and logs.
- [ ] Run all 16 feature powersets and final serialized check, clippy and full-test gates on native Windows.
- [ ] Run the same 16 feature powersets and final gates on Ubuntu 26.04 / glibc 2.43; record architecture, CUDA and SDK versions.
- [ ] Run native shell round trips on Windows CMD, Windows PowerShell 5.1, PowerShell 7.6.10 and Git Bash, and on Linux sh plus portable PowerShell 7.6.10. The portable download is hash-verified; no shell gate has run.
- [ ] Prove a relevant invariant test fails under a seeded mutant, restore the implementation, then verify normal behavior.
- [ ] Verify nv-rs consumes the pushed vcv-rs revision and rerun its affected discovery/binding gates.
- [ ] Re-index changed code and update README/instructions from verified behavior.
- [ ] Mark the wave verified only after both platform gates and independent reviews have evidence.

## Evidence limits

Windows and Ubuntu results are separate gates. SDK header discovery does not prove runtime loading
or GPU execution. NCCL multi-GPU and cuFile native-Linux GPUDirect Storage validation remain nv-rs
gates; this discovery work cannot establish them. Static findings and written repairs do not establish a completed independent review or an executed test gate.
