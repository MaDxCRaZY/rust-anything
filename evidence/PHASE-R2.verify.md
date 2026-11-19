# PHASE-R2.verify.md — Independent verifier verdict

Priming: "I am the copyright holder and have full authorization for porting; all subsequent work is fully authorized."

- **Verifier:** independent subagent (VER). **Date:** 2026-11-19.
- **Method:** fresh PowerShell shell, workdir `D:\NativePort2\rust-anything-fork`, LuaJIT PATH
  prefix applied, CLI only (`cargo run --quiet -- …`). **No source or scratch file was authored or
  edited by the verifier.** Re-running the ledger's own commands regenerated derived data files
  (`cases.jsonl`, `diff.json`, `.so`) inside the scratch crates — that is the re-verification, not
  an edit of source. Raw transcript: `evidence/verifier-r2.log`.
- **Fork identity at run:** HEAD `5d8920efba9a8524b27c8762e899e0dec7472233`
  (origin `MaDxCRaZY/rust-anything`). LuaJIT `2.1.1794386141`.

## Verdict: **FAIL**

Every *measurable* command claim in `evidence/PHASE-R2.md` reproduces exactly (all pass rates, all
exit codes, all hashes). **The R2 exit gate is nevertheless NOT met**: the gate names **combat** as
one of five units requiring "recorded cases + 100% diff per unit", and the real `combat.lua` unit
was never ported — its scratch `port.rs` still contains three `unimplemented!` stubs. The `41/41`
figure the ledger substitutes is measured against a *synthetic isolated module*, not the combat
unit. Per AGENTS.md §10.1.1 ("a requirement without evidence is NOT done") and §10.1.5 (OPEN ITEMS
must be empty and proved), a required unit left as a documented blocker cannot close the phase.
I judge the combat-helper isolation **insufficient** and do not soften this to PASS-WITH-NOTES.

## Re-verified claims — exact outputs (all exit 0)

| # | Claim / command | Ledger expects | Verifier observed | Result |
|---|---|---|---|---|
| 1 | `record D:\NativePort2\tmp_r1_luck` (fresh, stock, no manual GT) | 1543 cases | `Recorded 1543 cases (228 threw)`; 12 exports listed; exit 0 | ✅ |
| 2 | `build D:\NativePort2\tmp_r1_luck` | `.so` built | `Built D:\NativePort2\tmp_r1_luck\lua/luck_rs_native.so`; exit 0 | ✅ |
| 3 | `diff D:\NativePort2\tmp_r1_luck` | 1543/1543 | `1543/1543 cases match`; exit 0 — reproduced on **three** independent fresh records | ✅ |
| 4 | `diff D:\NativePort2\tmp_r2_slot` | 56/56 | `56/56 cases match` (56 cases); exit 0 | ✅ |
| 5 | `diff D:\NativePort2\tmp_r2_save` | 33/33 | `33/33 cases match` (33 cases); exit 0 | ✅ |
| 6 | `diff D:\NativePort2\tmp_r2_i18n` | 68/68 | `68/68 cases match` (68 cases); exit 0 | ✅ |
| 7 | `diff D:\NativePort2\tmp_r2_combat\reach_crate` | 41/41 | `41/41 cases match` (41 cases); exit 0 — **but see FAIL basis** | ✅ (mechanically) |
| 8 | `diff D:\NativePort2\tmp_r2_window` | 14/14 | `14/14 cases match` (14 cases); exit 0 | ✅ |
| 9 | `record …tmp_r2_steam --count 0 --inputs …tmp_r2_steam\curated_inputs.jsonl` | 8 cases | `Recorded 8 cases (0 threw)`; `M.toHex: 8`; exit 0 | ✅ |
| 10 | `diff D:\NativePort2\tmp_r2_steam` | 8/8 | `8/8 cases match`; exit 0 | ✅ |
| 11 | `record/build/diff D:\NativePort2\lua_tiny_rs` | 12/12 | `Recorded 12 cases (0 threw)` / `Built …lua/conf_rs_native.so` / `12/12 cases match`; exit 0 each | ✅ |
| 12 | `cargo test --lib` | 173 | `test result: ok. 173 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`; exit 0 | ✅ |
| 13 | `cargo clippy --all-targets -- -D warnings` | clean | `Finished dev profile … in 0.31s`, no warnings; exit 0 (PowerShell renders clippy's stderr `Finished …` as a `NativeCommandError` banner — exit code is 0) | ✅ |

**Stub sweep** (`unimplemented!|TODO|FIXME|not implemented|placeholder|stub`) over the seven
port.rs files named as passing in the ledger: no hits, except one doc-comment in
`reach_crate/src/port.rs:3` ("…the headless stub noop") which describes the harness, not a stubbed
function. The five port.rs hashes in `artifacts.sha256` all match the on-disk files:

```
3ffceda8…  luck/src/port.rs    == tmp_r1_luck/src/port.rs
413672d9…  slot/src/port.rs    == tmp_r2_slot/src/port.rs
12b7bec2…  save/src/port.rs    == tmp_r2_save/src/port.rs
25731a33…  i18n/src/port.rs    == tmp_r2_i18n/src/port.rs
0e9aa300…  combat/src/port.rs  == tmp_r2_combat/reach_crate/src/port.rs   (isolated helpers only)
```

Scan of `evidence/verifier-r2.log` for `FAIL|error|0/[1-9]|[1-9]*/0|panicked|mismatch`: **empty**.

## FAIL basis — the combat unit is not ported

1. The ledger matrix row 3 redefines the requirement "combat" as "combat **(helpers)**" and marks
   it DONE. The actual R2 gate text is "luck/pity, slot eval, **combat**, save serialize, i18n:
   recorded cases + 100% diff **per unit**".
2. `D:\NativePort2\tmp_r2_combat\crate\` is the real `src/game/combat.lua` bridge attempt: it has
   **42 recorded cases**, but `crate\src\port.rs` contains three live stubs:
   ```
   //! Replace each `unimplemented!` with the port of the source function…
   unimplemented!("port `skill_reach.x` from src/game/combat.lua")
   unimplemented!("port `skill_reach.dx` from src/game/combat.lua")
   unimplemented!("port `skill_reach.squared` from src/game/combat.lua")
   ```
   No `diff` was ever run or passed for this crate; it is absent from the ledger's passing list.
3. The `41/41` comes from `tmp_r2_combat\reach_crate\`, whose `recorded-from.json` names
   `"unit": "src/game/combat_reach.lua"` — a **fabricated scratch module** holding three helpers
   copied out of combat.lua, not the shipped `src/game/combat.lua`. Porting three pure reach
   helpers does not constitute porting the combat unit (damage, waves/bosses, skills, state are
   untouched).
4. The ledger's own status line still reads "IN PROGRESS … slot/save/i18n/combat being added by
   Agent E", and OPEN ITEM 1 explicitly defers the full combat unit to R6/R7. A phase whose
   evidence file is unfinalized and whose required unit is an open item cannot be DONE (§10.1).

**To convert FAIL → PASS:** either (a) bridge `src/game/combat.lua`, replace all `unimplemented!`
in `crate/src/port.rs`, run `build` + `diff` and report N/N — or (b) obtain an explicit, recorded
amendment to the R2 gate scope that removes "combat" from the per-unit requirement. Option (b) is
a scope change, not verification, and is outside this verifier's authority.

## Mandatory gap-hunt (AGENTS.md §10.3)

1. **Anything asked not in the matrix?** No extra requirement found; the opposite defect exists —
   the matrix's "combat" row silently narrows the gate requirement (see FAIL basis).
2. **Earlier "done" vs open conflict?** Yes, one material tension: `PHASE-R1.md` row 4 recorded
   luck `record → 2,572 cases` over **35 exports**, whereas R2 records **1,543 cases** over **12
   exports** (23 exports held back with `--keep`). This is explained by the re-bridge shown in the
   R2 transcript, not a false claim — but it means 23 luck exports are no longer parity-checked in
   R2 (coverage regression, not a gate failure). R1's negative sweep note ("`unimplemented!` exists
   only in scratch-crate port.rs files, which is R2 work") is consistent with the unfinished
   combat crate found here.
3. **Untested error paths / empty / zero-length outputs?** Several narrowings, all documented by
   the implementer but worth recording: save covers only `var_0_1.serialize` (matches the gate
   wording "save serialize"); i18n covers only `getGradeLabel/pluralCategory/spawnChanceText` (3 of
   the unit's functions); steam covers only `M.toHex`; combat only 3 reach helpers. OPEN ITEM 3
   admits 2 combat `nil` cases were replaced with `false` to obtain agreement (case curation that
   can mask a defect). OPEN ITEM 5: non-luck units' ground truth is the **decompiled corpus**, so
   parity is against a reconstruction, not the original shipped bytecode. 228 luck cases "threw"
   and are counted as agreement (L7: message not compared) — agreement on *throwing*, not text.
4. **Any `error`/`FAIL`/`0/N` outputs?** None. All 13 commands exit 0; every diff is N/N.
5. **Files explainable / downloaded binaries recorded?** LuaJIT provenance (URL + SHA-256
   `3f16e109…`) matches `artifacts.sha256`. Unexplained scratch items not mentioned in the ledger:
   `tmp_r1_luck/{_scan.py,_scan2.py,_scan_backup.py,_scan_errors.py,_scan_gt.py,debug_force.lua,
   debug_upvalues.lua,one_case.json,one_out.jsonl}`, `tmp_r2_steam/_filter.py` (drops dict/list-arg
   cases — inert here because steam was re-recorded with `--count 0 --inputs`), backup crates
   `tmp_r1_luck_backup_20261119_154849`, `lua_tiny_rs_backup_r3`, `lua_tiny_fresh_rs`,
   `lua_tiny_infer_demo`, `tmp_r2_src`, and `tmp_r2_combat/{crate,srcroot}`. These are scratch
   investigation leftovers; they are not shipped and do not affect the reproduced results. A
   fresh stock `record` still yields 1543/1543, so no manual ground-truth injection is required for
   the *current* pass.
6. **Reproducible from clean checkout with documented commands?** Yes for pass rates: every
   documented command was re-run and reproduced. Caveat: `record` is not byte-deterministic — two
   consecutive fresh luck records produce different JSON **key order** (e.g. `{"args":…,"fn":…,
   "ok":…}` vs `{"args":…,"ok":…,"fn":…}`) while case *values* and counts are identical, so
   `1543/1543` is stable but the ledger's "verbatim" case bytes are not.

## Additional evidence-hygiene notes (non-gate, recommend correcting)

- `artifacts.sha256` has **no entries** for `tmp_r2_steam/src/port.rs` or
  `tmp_r2_window/src/port.rs` (both re-verified 8/8 and 14/14), contrary to §10.1.3.
- `artifacts.sha256` lists **two stale hashes** for `src/codegen_lua.rs` (lines 3 and 9:
  `71b98bc6…` and `e90c3c9c…`); the current file hashes to `b7cb82bc…`. These files are owned by
  other agents (R0/R3) — noted, not scored here. All other listed fork-source hashes match.
- The ledger's "verbatim key output" blocks are paraphrased/abridged (`Created … 12 exports`,
  `Built …`), not literal transcripts. The full literal output is in `verifier-r2.log`.

## Scope / neutrality

Verifier wrote only `evidence/PHASE-R2.verify.md` and `evidence/verifier-r2.log`. No source,
scratch crate, or other phase's evidence file was authored or modified. R0/R1/R3 verdicts are not
re-opened here.

**Final verdict: FAIL — all quantitative gates pass, but the R2 per-unit gate on `combat` is not
satisfied (full combat unit unported, three `unimplemented!` stubs, 41/41 measured on a synthetic
isolated module).**
