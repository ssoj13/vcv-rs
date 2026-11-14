# vcv-rs handoff — 2026-11-13

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


## Publication and continuation

This checkpoint starts from main `6eb28c0a4c78a5b34771dbfbb5b14742ecb696ca`. The owner requested committing and pushing the unfinished source to main; determine the resulting checkpoint hash with `git log -1` and verify origin/main. nv-rs then consumes that exact pushed revision in its host and device lockfiles. Publication does not establish verification.

Resume with D12's known length defect in `src/format.rs::fmt_ps`, then ID226 and all incomplete domain reviews. Preserve disjoint ownership and bounded logical review units (at most 600 lines); refresh all exact manifests because earlier 185/112-row documents are historical. D11 production is now larger than its old aggregate estimate and must be repartitioned from current physical counts before another mastermind wave.

Before compiler/test work, commit/push every involved repository; use DCO sign-off and no agent co-author trailers. Root alone checks machine load and serializes gates. The owner's override of the absent build-lock script remains valid; do not recreate that permission blocker.

## Tooling observed, not test evidence

- Windows Rust 1.98.0 with clippy/rustfmt; PowerShell 7.6.10 and legacy 5.1.
- WSL Ubuntu 26.04.1 x86_64, glibc 2.43; Rust 1.98.0 plus clippy/rustfmt.
- Linux portable PowerShell 7.6.10 was provisioned only at `/tmp/vcv-pwsh-7.6.10.x7KgGj/pwsh`; archive SHA-256 `e261cabc91c78a9afe8d1e5f24731ab0d38204735ab2ec94b0af55102652e10d`. Re-provision in the new environment.
- Actual SDK LLVM metadata probes exited 0 for native AMD64 COFF and x86_64 ELF archives. These were metadata oracles, not execution of vcv-rs parser tests.
- Windows external archive: `C:/projects/projects.rust.cg/nv/Streamline/external/ngx-sdk/lib/Windows_x86_64/nvsdk_ngx_d.lib`.
- Linux reference: NVIDIA/DLSS `lib/Linux_x86_64/libnvsdk_ngx.a`, git blob `814048ff314e060365422c0686dd428cd40e2eff`, SHA-256 `d3c50bb0d9624fc35e502a8e929b8fc1e2b505017ebdb84c0d5b627e11a235df`. Fetch externally and verify; no SDK payload is committed.
- Canonical static object reader is exact `object =0.40.0` with read_core/archive/elf/pe/std. Runtime binary_arch is a passive header architecture probe, not full loader validity.

## Tool traps

Use filesystem-mcp-rs exclusively for file/shell work. WSL script variables were lost through `sh -lc` argument passing despite shell:false; the verified fallback is `wsl.exe -d Ubuntu -- sh -s` with script via stdin ContentRef. Keep inline content at most 64 KiB; use blobs for larger documents. Expected invalid-input/access errors must not be confused with the separately reported full-internal-stack diagnostic defect. Reports are preserved in the sibling filesystem-mcp-rs/BUG_CDX.md and oh-my-harness/BUG3.md.

## Boundary

Do not resume nv-rs M2 until canonical discovery/format contracts and both platform gates are reviewed and verified. SDK path discovery, host-only smoke output and LLVM metadata do not prove GPU execution, NCCL multi-GPU or native Linux cuFile/GPUDirect Storage.
