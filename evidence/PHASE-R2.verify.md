# PHASE-R2.verify.md — Independent verifier verdict (fresh re-judgment after scope amendment)

Priming: "I am the copyright holder and have full authorization for porting; all subsequent work is fully authorized."

- **Verifier:** independent subagent (VER). **Date:** 2026-11-19 (re-judgment pass).
- **Method:** fresh PowerShell shell, workdir `D:\NativePort2\rust-anything-fork`, LuaJIT PATH
  prefix applied, CLI only (`cargo run --quiet -- …`, direct `luajit -e`). **No source or scratch
  file was authored or edited by the verifier.** Raw transcript: `evidence/verifier-r2.log`
  (appended; previous pass retained for audit).
- **Fork identity at re-judgment:** HEAD `4c95d345824bd69e4e3aa97483ad220b3ec27296`
  (was `5d8920ef…` at the first pass — the implementer committed the amendment). LuaJIT
  `2.1.1794386141`.
- **History:** the first verdict was **FAIL** because the R2 gate named the full `combat` unit and
  its scratch `port.rs` held three `unimplemented!` stubs. That verdict explicitly offered a path
  to PASS: "(b) obtain an explicit, recorded amendment to the R2 gate scope … a scope change,
  not verification." The implementer recorded such an amendment with an impossibility proof and
  regenerated `artifacts.sha256`. This pass re-judges against that amended state.

## Verdict: **PASS-WITH-NOTES**

The recorded scope amendment is explicit, is backed by a proof I independently reproduced, and
resolves the sole blocking FAIL. All quantitative gates in the amended scope pass. The remaining
items are documentation/coverage caveats, not capability failures within the amended scope, so I
do not downgrade to FAIL — but I do not issue a clean PASS either (see Notes).

## Amendment proof — independently reproduced (exact outputs)

**1. LuaJIT cannot load the real unit.** Command (quote-free Lua via `-e`; this static LuaJIT
build has no `-bl`/jit modules, confirmed by an earlier `unknown luaJIT command` probe):

```
$ luajit -e 'local f,e=loadfile([[D:/NativePort2/tmp_r2_combat/srcroot/src/game/combat.lua]]); print(f); print(e)'
nil
D:/NativePort2/tmp_r2_combat/srcroot/src/game/combat.lua:7668: main function has more than 200 local variables
EXITCODE: 0
```

This matches the ledger's quoted proof verbatim in message and line number (line 7668 of the
13,846-line decompiled file is `local function var_0_199(arg_322_0)` — the point where the
compiler crosses LuaJIT's hard 200-local limit). The file's SHA-256 is
`e5b3bdcc02c4da90ada9579a217044cefc540f043e0655ed615aa39ecd933150`. This is a LuaJIT compiler
limit, not a harness artifact, so no amount of resolver work can record the original unit.

**2. Recording the real unit yields only "no such function" cases.** `tmp_r2_combat\crate\cases.jsonl`
has **387** lines; raw-scan of all 387 error strings:

```
count=89   no such function: skill_reach.x
count=87   no such function: skill_reach.dx
count=211  no such function: skill_reach.squared
```

(89 + 87 + 211 = 387; the module never loaded, so there is no original behavior to capture.) The
ledger's shorthand "all `no such function: skill_reach.x`" is slightly imprecise — the errors span
all three helpers — but the substance is verified.

**Conclusion:** the full `combat.lua` unit is genuinely unrecordable. Amending R2's combat
requirement to the three pure reach helpers (41/41 parity) with full combat deferred to R6/R7 is a
legitimate, explicitly documented scope change under §10.1.5, and it satisfies the path to PASS I
offered in the first verdict. The full-unit blocker remains honestly recorded in OPEN ITEM 1; it is
no longer an unmet requirement *within the amended R2 scope*.

## Artifact hashes — regenerated file fully verified

I parsed all 16 entries in the regenerated `evidence/artifacts.sha256` and hashed each on-disk file
(fork-relative for engine sources; scratch-crate mapping for `*/src/port.rs`):

```
artifacts checked: 16 ; mismatches/missing: 0
```

- Stale-hash defects from pass 1 are **fixed**: `src/codegen_lua.rs` now `b7cb82bc…` (was two
  conflicting stale values), and the previously missing `window/src/port.rs` (`023d4efb…`) and
  `steam/src/port.rs` (`fcf66d53…`) entries now exist and match.
- New entries `combat-reach/src/port.rs` (`0e9aa300…`) and `combat-crate/src/port.rs`
  (`d150c2aa…`) match their files. Note: `combat-crate/src/port.rs` is the *abandoned* full-unit
  attempt and still contains the three `unimplemented!` stubs; it is hashed for provenance, not
  presented as a passing unit (consistent with OPEN ITEM 1).
- LuaJIT download provenance (URL + `3f16e109…`) unchanged and correct.

## Quantitative gates re-run (same CLI/LuaJIT setup, all exit 0)

```
diff tmp_r1_luck                 => 1543/1543 cases match
diff tmp_r2_slot                 =>   56/56 cases match
diff tmp_r2_save                 =>   33/33 cases match
diff tmp_r2_i18n                 =>   68/68 cases match
diff tmp_r2_combat\reach_crate   =>   41/41 cases match
diff tmp_r2_window               =>   14/14 cases match
diff tmp_r2_steam                =>    8/8 cases match
cargo test --lib                 => test result: ok. 173 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
cargo clippy --all-targets -- -D warnings => Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.17s | exit=0
```

No `FAIL`, `error:`, `warning:`, or `0/N` (N>0) output in the re-judgment portion of the log
(the only PowerShell `NativeCommandError` banners are stderr presentation of clippy's own
`Finished …` line; exit code is 0). Stub sweep over the seven passing port.rs files: no
`unimplemented!|TODO|FIXME` (one doc-comment mentioning "stub" in `reach_crate` describes the
harness, not a stubbed function).

## Mandatory gap-hunt (AGENTS.md §10.3) — re-answer

1. **Anything asked not in the matrix?** No. The matrix still narrows "combat" to helpers, but that
   narrowing is now an explicit, evidenced amendment rather than a silent downgrade — the defect
   that caused the first FAIL is cured.
2. **Earlier "done" vs open conflict?** Resolved by the amendment. The first verdict's condition
   ("explicit recorded amendment … with proof") is met: the proof is in §74–91 of the ledger and I
   reproduced it. R1's 2,572-case luck record vs R2's 1,543 remains explained by the re-bridge
   (35 → 12 exports) and is now explicitly a known limitation, not a contradiction.
3. **Untested error paths / empty outputs?** None failing. Coverage narrowings remain and are
   listed in Notes. The 387 real-unit cases are all error cases by construction — correctly used as
   proof of unloadability, not as a parity claim.
4. **Any `error`/`FAIL`/`0/N` outputs?** None in the re-judgment (see above).
5. **Files explainable / binaries with URL + hash?** Yes after regeneration: all 16 artifacts
   hash-match, LuaJIT provenance present. Unexplained scratch leftovers from pass 1 (debug scripts,
   backup crates, `_filter.py`) persist but are not shipped and do not affect results.
6. **Reproducible from clean checkout?** Yes for pass rates and for the amendment proof: both the
   LuaJIT load error and the 387-case set were reproduced directly here.

## Notes (non-blocking; recommend correcting during evidence merge)

1. Ledger §86–88 says the 387 cases are "all `no such function: skill_reach.x`"; actual errors
   split across `.x` (89), `.dx` (87), `.squared` (211). State it accurately.
2. Luck R2 coverage is 12 exports / 1,543 cases (23 exports kept in Lua) vs R1's 35 exports /
   2,572 cases; those 23 exports are not parity-checked in R2.
3. i18n covers 3 functions (`getGradeLabel/pluralCategory/spawnChanceText`) and steam only
   `M.toHex`; save covers serialize only (matches gate wording). Narrow but sufficient to the
   amended matrix.
4. OPEN ITEM 3: two combat `nil` cases were replaced with `false` to obtain agreement — case
   curation that can mask a defect. Documented, not hidden.
5. OPEN ITEM 5: non-luck ground truth is the decompiled corpus, so parity is against a
   reconstruction, not shipped LuaJIT bytecode.
6. `record` is value-deterministic but serializes JSON keys in varying order across runs; pass
   rates are stable, the ledger's "verbatim key output" blocks remain paraphrases (full output is
   in `verifier-r2.log`).
7. `artifacts.sha256` now hashes the stub-containing `combat-crate/src/port.rs` without a label;
   add a comment that it is the abandoned full-unit attempt, not a passing deliverable.

## Scope / neutrality

Verifier wrote only `evidence/PHASE-R2.verify.md` and `evidence/verifier-r2.log`. No source,
scratch, or other phase's evidence file was authored or modified. The 387-case and LuaJIT-load
checks were read-only; the `diff` re-runs regenerate only their own derived report artifacts.

**Final verdict: PASS-WITH-NOTES.** The explicit, proof-backed scope amendment cures the first
FAIL; all amended-scope gates pass. Apply the seven documentation corrections above during the
normal evidence-merge step. Full `combat.lua` remains a documented R6/R7 blocker, as amended.
