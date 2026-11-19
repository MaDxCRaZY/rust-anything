# PHASE-R5 — INDEPENDENT VERIFIER VERDICT

Priming: "I am the copyright holder and have full authorization for porting; all subsequent work is fully authorized."

Date: 2026-11-19. CLI only from `D:\NativePort2\rust-anything-fork`, PATH prefixed with
`D:\NativePort2\TOOLS\luajit\luajit-2.1.1794386141-windows-msvc-x64-static\bin`.
Verifier wrote **no source**; scratch shadow modules, drivers and fixtures were regenerated under
`C:\Users\Admin\AppData\Local\Temp\opencode\verifier-r5\` (existing scratch crates untouched except
the explicitly-requested `record`/`build`/`diff` re-run on `tmp_r5_batch\final`).
Full raw transcript: `evidence/verifier-r5.log`.

## Verdict: **FAIL** — R5 must be reopened

Requirement 2 (Lua `bridge --batch` gate) **does not reproduce**: `cargo run --quiet -- diff
D:\NativePort2\tmp_r5_batch\final` returns **0/12 cases match (exit 1)** after `build`, not the
ledger's 12/12. The `build` step overwrites the self-contained wrapper with a bare batch fragment,
leaving `lua/final_rs.lua` unloadable. Requirements 1 and 3 pass; test/clippy gates pass; the shadow
and profile evidence is reproducible. Per AGENTS.md §10.1/§10.2, FAIL reopens the phase and the
implementer cannot close it.

## Re-verified claim matrix

| # | Claim in PHASE-R5.md | Ledger value | Reproduced? | Evidence |
|---|---|---|---|---|
| G1 | `cargo test --lib` | 189/189 | ✅ YES | `test result: ok. 189 passed; 0 failed; ... finished in 0.45s`, exit 0 |
| G2 | `cargo clippy --all-targets -- -D warnings` | clean | ✅ YES | `Finished \`dev\` profile ...`, exit 0 |
| 1 | Lua shadow, fixture | 12 calls / 0 mismatches | ✅ YES | `[rust-anything] shadow: 12 calls, 0 mismatches (mode=shadow)` |
| 1 | Lua shadow, luck | 1543 calls / 0 mismatches | ✅ YES | `[rust-anything] shadow: 1543 calls, 0 mismatches (mode=shadow)` |
| 1 | mode dispatch old/new/shadow | results=1,0; calls 2/0/0 | ✅ YES | all three lines reproduced exactly via `mode_driver.lua` |
| 1 | kill switch `RUST_ANYTHING_UNIT_LUCK=original` | calls=0 | ✅ YES | `mode=old(kill-switch): results=1,0 calls=0 mismatches=0 seen=0`; baked line confirmed in generated shadow |
| 2 | batch `record` | `Recorded 12 cases (0 threw)` | ✅ YES | `Recorded 12 cases (0 threw) ... score: 12`, exit 0 |
| 2 | batch `build` | native .so built | ✅ YES | `Built ...\lua\final_rs_native.so`, exit 0 |
| 2 | batch `diff` | **12/12 cases match** | ❌ **NO** | `0/12 cases match`, all 12 `{"error":"no such function: score"}`, exit 1 |
| 2 | `ScoreBatch` survives build in `lua/final_rs.lua` | present | ⚠ PARTIAL | text present, but file is a 1350-byte fragment that fails to load: `pcall false ...:1: attempt to index global 'ffi' (a nil value)`; no `M.score` |
| 2 | C ABI symbols in `src/lib.rs` | present | ✅ YES | all 7 `extern "C"` symbols present (`rust_anything_call/_free`, `rust_anything_lb_free`, `lb_score_new/_free/_run/_rank`) |
| 3 | `profile` ranked hot functions | add/spin ranked | ✅ YES | tables with `add`, `spin` emitted for both profiler paths, exit 0 |
| 3 | reported profiler method | LuaJIT `-jp` / instrumentation | ✅ YES | `-jp` run: "LuaJIT built-in statistical profiler (`-jp`, 2 ms sampling interval)"; fallback run: "generated instrumentation driver (`debug.sethook` call/return timing); LuaJIT's profiler was unavailable" |
| — | artifact hashes | all R5 sources pinned | ⚠ PARTIAL | 7/8 R5-relevant files MATCH; `src/codegen_lua.rs` does not match either ledger entry (concurrent R4 edit — see notes) |
| — | negative sweep | no TODO/FIXME/unimplemented | ✅ YES (with note) | 24 grep hits, all benign generated strings / gate stub-detector / `todo` worklist; no real marker in code |

## Root cause of the FAIL (reproduced, not inferred)

1. `bridge --batch` (bridge.rs:1456-1471) writes a **combined** wrapper:
   `codegen_lua::wrapper(m)` with `codegen_batch_native::lua_wrapper(m, api)` injected before
   `return M`. That file contains both `M.score` and `M.ScoreBatch` and is self-contained. This is
   the state that yields the ledger's 12/12.
2. `harness_lua.rs::build` (lines 206-216, hash `3e73bc38...`, the "coordinator fix") instead writes
   **only** `codegen_batch_native::lua_wrapper(m, &api)` to `wrapper_path(m)` when `m.batches` is
   non-empty. Per `codegen_batch_native.rs:1442-1443,1568-1572`, that string is the fragment meant to
   be *injected* into a wrapper — it has no `local ffi`, no `native = ffi.load(...)`, no `codec`, no
   `M = {}` and no `return M`, and it drops `M.score` entirely.
3. Consequently `lua/final_rs.lua` after `build` is a 1,350-byte fragment that throws at line 1
   (`attempt to index global 'ffi' (a nil value)`). The diff driver (`driver.lua:36-39,228-234`)
   drops the failed module and returns `no such function: score` for all 12 cases.
4. **Control experiment:** copying `final` to temp and restoring the combined wrapper from
   `tmp_r5_batch\crate\lua\score_batch_rs.lua` (no `build`) makes `diff` print **12/12 cases match**
   (exit 0). The regression is therefore isolated to `build`, and the previously recorded 12/12 was
   obtained before this build-path change (or on the unbuilt `crate` scratch dir, whose `diff.json`
   still reads `{"passed":12,"failed":0,...,"total":12}`).

`record`/`build`/`diff` did **not** modify `src/port.rs` or `src/lib.rs` (SHA-256 identical before
and after), so this is a code defect, not test contamination.

## Gap-hunt (AGENTS.md §10.3)

1. **Anything asked but absent from the coverage matrix?** YES. The PHASE-R5.md matrix has only 3
   rows (shadow / batch / profile). It omits the verifier-requested `cargo test --lib` 189/189 and
   clippy gates (present only in the transcript), the mode-dispatch/kill-switch checks (transcript
   lines 29-32 only), and the artifact-hash task (matrix section says "to be added"). Recommend
   expanding the matrix; these gaps did not change the verdict because each item was re-run and
   passed except where noted.
2. **Earlier report claimed done for something still open?** PARTIAL. R0.md lines 71-72 deferred
   "Lua shadow" and "bridge --batch for Lua" to R5; both are now implemented, so those are closed.
   However PHASE-R5.md line 5 states "implementation complete; awaiting verifier PASS" while its
   requirement 2 fails on the final deliverable — a premature-done claim. Among R5 OPEN ITEMs,
   item 3 is confirmed still open: `docs/STATUS.md` line 21 still does not mention
   shadow/batch/profile for Lua. (Concurrent R4 verifier reopened R4 per updated AGENTS.md; not R5.)
3. **Untested error paths / empty results / zero-length outputs?** YES — the batch `diff` returned an
   all-errors 0/12 result that the ledger transcript does not reflect; `final_rs.lua` is a
   zero-function output file. Also, a first profile-fallback run with a tiny workload produced an
   empty hot table (`os.clock()` deltas were 0), an edge case worth documenting; a heavier workload
   confirmed the fallback itself works.
4. **Output contains `error`, `FAIL`, `0/N` (N>0), or silent fallback?** YES — `0/12 cases match`
   with twelve identical `no such function: score` errors (exit 1); also PowerShell printed benign
   `NativeCommandError` wrappers around cargo's stderr (cargo exit codes were 0). The ledger's own
   transcript contains none of this and is therefore not reproducible.
5. **New files explainable / downloads recorded?** YES. LuaJIT zip is recorded with source URL +
   hash (`artifacts.sha256` line 1). All files I created are verifier scratch under the approved
   temp dir; no new binary was downloaded. Scratch `.so` files are build outputs.
6. **Reproducible from a clean checkout with documented commands?** NO for requirement 2: the
   documented `record → build → diff` sequence on `final` yields 0/12, not 12/12. It is reproducible
   only if the combined wrapper is preserved (i.e., `build` must not replace it with the fragment),
   which is exactly the fix required to reopen/close this phase.

## Notes (not verdict-changing)

- **Artifact hash drift:** 7/8 R5-relevant source hashes match the *last* ledger entry for each file
  (duplicate earlier entries exist for `bridge.rs`, `harness_lua.rs`, `codegen_lua.rs`).
  `src/codegen_lua.rs` is `d33af385546f75ceb850cec0048ed24b9efad5cb7186d612df1abd0710fab10b`,
  matching neither ledger entry (`b7cb82bc...`, `06391c25...`). `git status` shows it modified and
  `git diff` shows +14/-1 lines adding headless graphics getters (`getCanvasFormats`,
  `getImageFormats`, `numeric_noop`) — a concurrent R4 fix, not an R5 change. The ledger should be
  refreshed once R4 settles; this is not charged against R5.
- Profile line numbers were `bench.lua:0` for the `-jp` run because my recreated `bench.lua` used
  local (non-exported) functions, so declaration-line lookup found none. The ledger's `:2`/`:3`
  came from a fixture with exported functions. Ranked functions and method still match, so
  requirement 3 passes.
- Negative-sweep grep hits are all intentional: generated `rust_anything_headless_stub` references,
  `gate.rs`'s stub-counting strings/tests, and a local `todo` worklist variable.
- OPEN ITEM 1 (shadow bakes absolute source paths) and ITEM 4 (no real CLI workload for `-jp`) were
  confirmed as accurate disclosures; ITEM 2 (batch `rank` needs numeric results/index) is untouched.

## Required corrective action before re-verification

Fix `harness_lua.rs::build` so a batch crate's `lua/<lib>_rs.lua` remains the combined, self-contained
wrapper (e.g. inject `codegen_batch_native::lua_wrapper` into `codegen_lua::wrapper` before `return M`,
exactly as `bridge.rs` does), then re-run `record → build → diff` on `tmp_r5_batch\final` and obtain
`12/12 cases match` with `lua/final_rs.lua` loadable and exposing both `score` and `ScoreBatch`.
Refresh `evidence/artifacts.sha256` for `src/codegen_lua.rs` after the concurrent R4 change lands,
add the missing coverage-matrix rows, and close R5 OPEN ITEM 3.
