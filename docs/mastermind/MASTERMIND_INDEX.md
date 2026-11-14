# Windows/Linux domain index

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


Current continuation: [HANDOFF.md](../../HANDOFF.md), [TODO.md](../../TODO.md).

| Domain | Transfer status |
|---|---|
| D11 | 225-ID static closure; added ID226 unreviewed/unexecuted |
| D12 | 125-ID manifest; confirmed PowerShell length defect open; full review incomplete |
| D13 | 110-ID map; independent review pending |
| Windows / Linux | 0/2 executed platform gates |

## Historical index (superseded)

Updated: 2026-11-13. Priority: finish the canonical vcv-rs Windows/Linux implementation.
See [TODO.md](../../TODO.md), [README.md](../../README.md) and the [seam matrix](SEAM_MATRIX.md).

## Current evidence

Recovery on 2026-11-13 confirmed vcv-rs local `main` at `6eb28c0` and nv-rs local `main` at
`8b7e13c`, both with uncommitted work. The owner authorized completing Windows/Linux support
under sysgo/mastermind. D11/D12/D13 ownership is partitioned; D11 SDK and D12 environment/formatter/harness
repairs are active after their prior source freeze. No new source push, compiler, test, mutant
or completed fresh independent-review gate has passed. Windows and Ubuntu platform gates remain
0/2. README examples remain unexecuted.

D11 writes explicit-root precedence in the discovery owner rather than duplicating consumer searches.
Its prior freeze report counted 2,273 physical lines across four files, including CUDA runtime proof. D12 contains explicit
target identity, separate executable/runtime paths, raw `OsString` values and fallible formatters.
Source presence establishes neither feature-matrix compilation nor native shell correctness.

CUDA runtime_dir and selected vcpkg-triplet propagation repairs are written and unexecuted.
CMD empty-component behavior is pinned in formatter source and its extended native list-snapshot
harness has bounded source readback. D12 records 112 individual manifest rows; D13 records 110. Fresh reviews of all three
domains and the cross-domain callers remain pending. The absent build-lock script is not a blocker:
root follows the owner's machine-load-based serialization instruction for this task.

## D11 fresh review checkpoint

Four confirmed root causes need repairs and re-review: OptiX fixed temporary roots/recursive
pre-delete ownership; NGX runtime filenames and native package paths; NGX nested Windows
import-library layout; and case-insensitive Windows CUDA `.LIB` suffix handling. The OptiX
worker has written owned-RAII fixture cleanup and a regression, but neither independent review
nor execution is complete. The final independent 185-row audit table/verdict remains pending.

The confirmed D11/D12 seam lets runtime-only CUDA `bin/x64` reach PATH through `bin_dirs`
without target proof; D11 plans the repair. D12 shell Scratch uses predictable PID/counter
`create_dir_all` ownership and can claim/delete a stale directory; its repair is pending.
D12 also must preserve or explicitly reject empty scalar values: CMD `set name=` and Windows
PowerShell 5.1 `SetEnvironmentVariable` delete them, while PowerShell 7.5+ differs. The portable
formatter repair remains pending.

D13 static mapping has identified no production defect so far. Independent review of all
110 manifest rows remains pending; no domain approval follows from that static result.

## Domain assignment

Root assigned the following disjoint source ownership. These rows record source checkpoints, not approval:

| Area | Contract | Owner / exact files | Status |
|---|---|---|---|
| [D11 SDK discovery](domains/D11_sdk.md) | Authoritative CUDA/OptiX roots; target-aware NVIDIA SDK paths, versions, NCCL and runtime/link metadata | `vcv_sdk`: src/cuda.rs, src/optix.rs, src/nvsdk.rs, src/discovery.rs | repairs active; review/execution pending |
| [D12 environment and formatting](domains/D12_env_format.md) | Explicit target, native lists, vcpkg metadata, fallible shell/JSON output and round trips | `vcv_env`: src/env.rs, src/format.rs, tests/format_roundtrip.rs | repairs active; review/execution pending |
| [D13 CLI and feature integration](domains/D13_cli.md) | Platform entry points, selection/listing, help/aliases and feature matrix | `vcv_cli`: src/cli.rs, src/cli/*.rs, src/main.rs, src/lib.rs, Cargo.toml, Cargo.lock, tests/cli.rs; src/detect.rs clap cfg attributes only | source frozen; review/execution pending |

Keep each domain at or below 2,500 implementation lines and each numbered micro-unit at or below
600 lines or 5–10 functions. Require a symbol-by-symbol ledger and a fresh independent Inquisitor
review answering all seven questions. Do not use filesystem `extract_symbols` as a parser: it is
a character-cutting mutation tool. Use current GitNexus symbols/context and bounded source reads.

## Root-only gates

- [ ] Exact domains, file ownership and numbered manifests assigned.
- [ ] Explicit-root precedence and complete platform path contracts locked by fixtures.
- [ ] Implementation and all live consumers wired; obsolete stubs removed.
- [ ] Independent reviewer per domain completed every numbered manifest row and all seven Inquisitor questions; repairs and re-review left no unresolved defects.
- [ ] [Seams](SEAM_MATRIX.md) verified with source and test evidence.
- [ ] Every involved repository committed and pushed to main before any build/check/test.
- [ ] Serialized narrow gates, all 16 feature powersets and final check/clippy/full tests passed separately on Windows and Ubuntu 26.04 / glibc 2.43.
- [ ] Native shell gates passed on Windows CMD, Windows PowerShell 5.1, PowerShell 7.6.10 and Git Bash, plus Linux sh and portable PowerShell 7.6.10; the portable download is hash-verified, execution remains pending.
- [ ] Seeded mutant made a relevant test fail, then the original invariant passed.
- [ ] nv-rs consumed the pushed vcv-rs revision and its affected gates passed.
- [ ] Graph refresh and documentation/instructions reflect verified behavior.

## Execution and evidence rules

Root alone executes build/check/test commands serially after checking CPU/RAM/GPU load and other
active builds. Owner instruction on 2026-11-13 overrides the absent build-lock requirement for this task: root monitors CPU/RAM/GPU load and other build processes, then serializes Cargo/check/test runs. No replacement lock or further permission is required.
Source edits finish first. The user's push-to-main requirement applies to all involved repositories before
any such command, including tests for a seeded mutant.

Keep Windows and Ubuntu evidence separate. A discovery result proves paths and metadata only;
runtime loading, GPU execution, NCCL multi-GPU and cuFile GPUDirect Storage need their own gates.
Preserve exact SDK/toolkit, architecture, glibc and dependency revisions in the final receipts.
