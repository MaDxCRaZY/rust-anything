# PHASE-R5 — INDEPENDENT VERIFIER VERDICT (re-verification, 2nd pass)

Priming: "I am the copyright holder and have full authorization for porting; all subsequent work is fully authorized."

Date: 2026-11-19. Clean shell; CLI only from `D:\NativePort2\rust-anything-fork`; PATH prefixed with
`D:\NativePort2\TOOLS\luajit\luajit-2.1.1794386141-windows-msvc-x64-static\bin`.
Verifier wrote **no source**; scratch shadows/drivers/fixtures regenerated under
`C:\Users\Admin\AppData\Local\Temp\opencode\verifier-r5-rerun\` (existing scratch crates untouched
except the requested `record`/`build`/`diff` re-run and read-only `diff` spot checks).
Exact outputs: `evidence/verifier-r5.log`. This supersedes the 1st-pass **FAIL** verdict.

## Verdict: **PASS-WITH-NOTES**

The 1st-pass FAIL (batch wrapper overwritten by `build`) is **fixed and re-verified**:
`src/harness_lua.rs::build` now regenerates the base Lua wrapper and injects
`codegen_batch_native::lua_wrapper` before `return M`. A clean `record → build → diff` on
`D:\NativePort2\tmp_r5_batch\final` yields **12/12 cases match (exit 0)**, and
`lua/final_rs.lua` is a usable self-contained combined wrapper exposing both `M.score` and
`ScoreBatch`. All three R5 requirements now reproduce; gates, hashes, shadow, profile and all
parity crates pass. Notes are documentation-only (see below); none affects an exit gate.

## Re-verified claim matrix

| # | Claim | Ledger value | Reproduced? | Evidence (2nd pass) |
|---|---|---|---|---|
| G1 | `cargo test --lib` | 189/189 | ✅ YES | `test result: ok. 189 passed; 0 failed; ... finished in 0.54s`, exit 0 |
| G2 | `cargo clippy --all-targets -- -D warnings` | clean | ✅ YES | `Finished \`dev\` profile ...`, exit 0 |
| A | artifact hashes | R5 sources pinned | ✅ YES | 8/8 MATCH, incl. `harness_lua.rs a1f4d90d…`, `codegen_lua.rs d33af385…` (new ledger entries) |
| 1 | shadow, fixture | 12/0 | ✅ YES | `[rust-anything] shadow: 12 calls, 0 mismatches (mode=shadow)` |
| 1 | shadow, luck | 1543/0 | ✅ YES | `[rust-anything] shadow: 1543 calls, 0 mismatches (mode=shadow)` |
| 1 | mode dispatch old/new/shadow | results=1,0; calls 2/0/0 | ✅ YES | all three lines reproduced via fresh `mode_driver.lua` |
| 1 | kill switch `RUST_ANYTHING_UNIT_LUCK=original` | calls=0 | ✅ YES | `mode=old(kill-switch): results=1,0 calls=0 mismatches=0 seen=0` |
| 2 | batch `record` | `Recorded 12 cases (0 threw)` | ✅ YES | exact line + `score: 12`, exit 0 |
| 2 | batch `build` | native .so built | ✅ YES | `Built ...\lua\final_rs_native.so`, exit 0 |
| 2 | batch `diff` | 12/12 | ✅ YES (FIXED) | `12/12 cases match`, exit 0 (was 0/12 on 1st pass) |
| 2 | combined wrapper survives build | `ScoreBatch` in `final_rs.lua` | ✅ YES (FIXED) | 2,225 B / 60 lines; `local ffi`, `native = ffi.load`, `codec`, `M = {}`, `function M.score`, `ScoreBatch_mt:run/rank`, `return M`; loads OK; `score=function`, `ScoreBatch=table`, `score(0)=1` |
| 2 | C ABI symbols in `src/lib.rs` | present | ✅ YES | all 7 `extern "C"` functions present |
| 3 | `profile` ranked hot functions | add/spin tables | ✅ YES | tables emitted for `-jp` and fallback paths, exit 0 |
| 3 | reported profiler method | `-jp` / instrumentation | ✅ YES | exact method strings reproduced for both paths |
| — | negative sweep | no real TODO/FIXME/unimplemented | ✅ YES | 14 hits, all benign (gate.rs detector strings/tests, `todo` worklist) |
| — | parity crates | all re-pass | ✅ YES | 12/12, 1543/1543, 56/56, 33/33, 68/68, 41/41 (reach_crate), 14/14 — all exit 0 |

### Fix confirmation (source-level)
`harness_lua.rs::build` (hash `a1f4d90d…`) now does:
`let mut wrapper = codegen_lua::wrapper(m)` → if `!m.batches.is_empty()`, parse `src/port.rs`,
`let batches = codegen_batch_native::lua_wrapper(m, &api)`, then
`wrapper = wrapper.replacen("\nreturn M\n", &format!("\n{batches}\nreturn M\n"), 1)` →
write `wrapper` to `wrapper_path(m)`. This mirrors `bridge.rs:1456-1471` and eliminates the
1st-pass defect where the bare fragment was written as the wrapper.

`record`/`build`/`diff` did not mutate `src/port.rs` or `src/lib.rs` (SHA-256 identical before and
after), confirming the pass is due to the fix, not test contamination.

## Gap-hunt (AGENTS.md §10.3) — re-answered

1. **Anything asked but absent from the coverage matrix?** YES (documentation only). PHASE-R5.md's
   matrix still has only 3 rows; it omits `cargo test --lib`, clippy, mode-dispatch/kill-switch and
   artifact-hash items that the verifier was explicitly asked to check. All were independently
   re-run and pass, so this does not change the verdict, but the matrix should be expanded.
2. **Earlier report claimed done for something still open?** NO functional case remains. The
   1st-pass FAIL item (batch gate) is now genuinely fixed. R5 OPEN ITEM 3 is still open:
   `docs/STATUS.md` line 21 (Lua/LÖVE row) still does not mention shadow/batch/profile — carried as
   a note. R0's deferred "Lua shadow" / "bridge --batch" items are closed by this phase.
3. **Untested error paths / empty results / zero-length outputs?** None material on 2nd pass. The
   1st-pass empty-output failure is gone. The profile fallback's sensitivity to very short
   workloads (`os.clock()` ≈ 0 → empty hot table) remains a known edge; the heavier workload
   confirms correct behavior.
4. **Output contains `error`, `FAIL`, `0/N` (N>0), or silent fallback?** NO for R5 deliverables on
   this pass: all reported counts are 12/12, 1543/1543, 189/189, etc. (PowerShell printed benign
   `NativeCommandError` wrappers around cargo's stderr; cargo exit codes were 0.)
5. **New files explainable / downloads recorded?** YES. LuaJIT zip recorded with URL+hash
   (`artifacts.sha256` line 1). All new local files are verifier scratch under the approved temp
   dir; no unrecorded binary downloaded.
6. **Reproducible from a clean checkout with documented commands?** YES. The documented
   `record → build → diff` sequence on `final` now yields 12/12 from a clean shell, with the built
   wrapper loadable and exposing both `score` and `ScoreBatch`.

## Notes (PASS-WITH-NOTES reasons)

- R5 OPEN ITEM 3 remains open: update `docs/STATUS.md` Lua/LÖVE row to mention shadow, batch and
  profile after this PASS.
- PHASE-R5.md coverage matrix should gain rows for the test/clippy gates, mode-dispatch/kill-switch,
  and artifact hashes (the verifier re-ran all of them; they are missing only from the matrix).
- `evidence/artifacts.sha256` accumulates duplicate historical entries per file with `(R5/R4 fix)`
  annotations; the current on-disk hashes match the newest entry for every R5-relevant file.
- OPEN ITEM 1 (shadow bakes absolute source paths) and ITEM 4 (profile used a Lua fixture, no real
  game CLI workload for `-jp`) remain accurate, explicitly disclosed limitations; neither is a gate.
- Negative-sweep hits are intentional strings in `gate.rs`'s stub detector and its tests, plus a
  local `todo` worklist variable.
- `git status` at end shows only a pre-existing concurrent `src/codegen_lua.rs` modification
  (R4 headless-stub getters), which the verifier did not write.

## Conclusion

All R5 exit gates pass on re-verification. Verdict **PASS-WITH-NOTES**; the notes are documentation
follow-ups that do not reopen the phase.
