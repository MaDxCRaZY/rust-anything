# PHASE-R6 — INDEPENDENT VERIFIER VERDICT (RE-JUDGEMENT under user-approved hybrid scope)

Priming: "I am the copyright holder and have full authorization for porting; all subsequent work is fully authorized."

- **Verifier:** independent R6 verifier (never the R6 implementer). **Date:** 2026-11-19.
- **Write ownership honored:** only `evidence/PHASE-R6.verify.md` and `evidence/verifier-r6.log`.
  No source file written or modified. CLI only, clean shells.
- **Full verbatim transcript:** `evidence/verifier-r6.log` §[1]–§[11].
- **Supersedes:** my previous R6 verdict (FAIL, pre-hybrid). This re-judgement uses the new
  binding evidence: `AGENTS.md` §9.1, `evidence/HYBRID-SCOPE.md`, the re-verified
  `PHASE-R4.verify.md`, `slotbound-engine/evidence/HYBRID-ENGINE-M1.md`, and the updated
  `R6-COVERAGE-MATRIX.md`.

## VERDICT: **FAIL** (narrow / conditional — R4 and headless gaps are CLOSED; residual failure is the unmet conditions on the hybrid degradation)

The two decisive grounds for my earlier FAIL are now genuinely resolved, and I confirm that with
fresh runs:

1. **R4 is PASS-WITH-NOTES, not FAIL** — `PHASE-R4.verify.md:12` and `:123` ("R4 closed under the
   user-approved §9.1 hybrid re-scope"). The re-scope is a recorded user amendment in `AGENTS.md`
   §9.1 (`AGENTS.md:211-217`) and `HYBRID-SCOPE.md:20-23`, not an implementer unilateral change.
2. **Full game logic runs headlessly in Rust** — independently reproduced: 600-frame record,
   `600/600 frames match: HASH EQUALITY OK`, and a tamper control that fails correctly
   (`599/600 frames match: MISMATCH`, exit 1). The former `effects.lua:0` blocker is bypassed by
   the native engine's LÖVE API.

However, R6 still cannot be signed complete. Under the very semantics that permit the new
statuses, the gate is unmet:

- **The `LUA-RETAINED` permission is conditional and its conditions are NOT met.**
  `AGENTS.md:216-217`: *"R6 permits `LUA-RETAINED (engine-accelerated)` as a documented
  degradation **only with** the perf/parity conditions recorded there."*
  `HYBRID-SCOPE.md:15-18`: a module may be marked LUA-RETAINED *"provided: 1. the Rust engine
  measurably meets **boot/GPU/RAM/FPS targets on device**, and 2. gameplay behavior matches the
  **PC reference (R8 parity)**."*
  - Boot <1 s: **met on the dev machine** (my runs 483/494/457 ms) — but not evidenced on device.
  - GPU/RAM/FPS targets on device: **NOT met** — `HYBRID-ENGINE-M1.md:38` states
    *"Renderer/audio/input are still headless substitutes; GPU/RAM targets not yet implemented"*;
    blocker `B7` (AGENTS.md §12.1 ~86% GPU, 8.7 s boot) remains open; the user confirms device
    evidence is still open.
  - PC-reference gameplay parity (R8): **NOT met** — R8 is still a future gate; the 600/600 result
    is a self-consistency determinism oracle, not a comparison to the reference game.
  Therefore 58 `LUA-RETAINED` rows are not yet valid documented degradations for R6 purposes.
- **6 PARTIAL modules have no whole-module counterpart and no accepted degradation.**
  luck (12 ABI exports; 23 higher-level hidden functions retained), slot (4 pure evaluators),
  save (serialize only), i18n (3 helpers), window (9 fns), steam (toHex). `PHASE-R6.md:43`
  explicitly still requires "Replace the 6 PARTIAL ports with full-module parity or explicit
  accepted degradations." A gated subset is not a module counterpart.
- **1 BLOCKED module (`src/game/combat.lua`)** — full unit unrecordable (LuaJIT 200-local limit);
  41/41 covers only 3 reach helpers. Blocker B1 remains open with owner R6/R7.
- **7 open blockers** in `R6-BLOCKER-REGISTER.md` (B1–B7), including B3/B5/B7 whose resolution
  is explicitly deferred to R7/R8. A migration-readiness gate with 7 open blockers is not done.
- **§10 anti-premature-done:** the implementer's own ledger says `Status: PENDING RE-VERIFICATION`
  and *"not yet claimed complete"* (`PHASE-R6.md:5-8`) and lists four items still required before
  R6 can PASS (`PHASE-R6.md:40-45`). Signing PASS or PASS-WITH-NOTES would declare the phase
  complete contrary to the implementer's own record and to §10.1 ("a requirement without evidence
  is NOT done").

Per `AGENTS.md` §9 — *"a phase is complete ONLY when its measurable exit gate passes AND an
independent verifier signs PASS"* — and §10.6 (FAIL reopens the phase; the implementer cannot
close it), **R6 remains FAIL / reopened.**

## Independent command results (exact, from `verifier-r6.log`)

| # | Command | Verbatim result | Expected | Result |
|---|---|---|---|---|
| 1 | `cargo test --release` (@`slotbound-engine`) | `running 8 tests` … `test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.19s`, exit 0 | 8/8 | ✅ PASS |
| 2–4 | `slotbound-engine.exe boot <root>` ×3 | `total=483ms`, `total=494ms`, `total=457ms`; each with `boot OK:` and `[PORTDIAG] dims=2880x1800 full=true virtual=0x0 scale=1`, exit 0 | <1000 ms ×3 | ✅ PASS |
| 5 | `headless <root> 600 <trace>` | `Recorded 600 frames to ...verifier-r6-trace.json`, exit 0 | 600 frames | ✅ PASS |
| 6 | `headless <root> 600 <trace> --replay` | `600/600 frames match: HASH EQUALITY OK`, exit 0 | 600/600 | ✅ PASS |
| 8 | tamper control (literal hash substitution) | `frame 1: recorded 0000000000000000, got a8b1de778ae1e373` / `599/600 frames match: MISMATCH`, exit 1 | mismatch | ✅ PASS (genuine oracle) |
| 9 | matrix recount | 66 rows; `DONE (harness)=1, PARTIAL=6, BLOCKED=1, LUA-RETAINED=58`; real files 66; set diff 0 | 66 / 1-6-1-58 | ✅ PASS |
| 10 | canonical R4 trace | `R4_TRACE_EXISTS=YES`, `66793` B, `600` frames | on disk | ✅ PASS |

Notes on these runs: my boot times (483/494/457 ms) are slower than the ledger's 364/339/344 ms
but still clear the <1 s gate; timing variance is not a gate miss. The `cargo test` build emitted
2 warnings (`unused BTreeMap`, linker `LNK4099`); non-blocking, already carried in R4 NOTE 5/6.
The first tamper probe used a lossy `ConvertTo-Json` rewrite and produced a malformed trace
(`trace must be a JSON array`) — that was a verifier-probe defect, discarded and replaced by the
byte-preserving substitution in run [8].

## Coverage arithmetic — independently recounted

| Status | Count | Interpretation under hybrid |
|---|---|---|
| `DONE (harness)` | **1** | conf.lua (config shim; no gameplay logic) |
| `PARTIAL` | **6** | gated subsets only; **no** full-module parity, **no** explicit accepted degradation |
| `BLOCKED` | **1** | full combat.lua; 200-local limit; 41/41 helper subset only |
| `LUA-RETAINED` | **58** | permitted **only if** `HYBRID-SCOPE.md` conditions hold — device perf + R8 parity are open |
| **Total** | **66** | 64 files under `port\game\src` + `conf.lua` + `main.lua`; matrix row set matches disk exactly (diff 0) |

## Document cross-checks

| Check | Result | Evidence |
|---|---|---|
| `AGENTS.md` §9.1 user-approved hybrid exists | ✅ | `AGENTS.md:211-217` |
| `HYBRID-SCOPE.md` exists; records user approval + conditional degradation | ✅ | `evidence/HYBRID-SCOPE.md:5-23` |
| R4 verdict now PASS-WITH-NOTES (hybrid re-scope) | ✅ | `PHASE-R4.verify.md:12,123`; AGENTS.md §12.2 row updated |
| `HYBRID-ENGINE-M1.md` exists; boot/headless + open GPU/RAM | ✅ | `slotbound-engine/evidence/HYBRID-ENGINE-M1.md:33-38` |
| Blocker register exists, 7 open blockers | ✅ | `R6-BLOCKER-REGISTER.md:7-13` |
| i18n 156/156 getText + 52/52 helpers; 30-frame equality | ⚠️ ledger-only | `PHASE-R6.md:34-35`; the requested commands reproduce `parity_sweep_gettext ... ok` (8/8 total) and a stronger 600/600, but not those exact counts — documentation gap, not gate-decisive |
| Implementer ledger internally consistent | ❌ | `PHASE-R6.md:14-17,22-29,49-52` still says "58 NOT STARTED", "main.lua cannot load headlessly", "R4 verifier reopened" — stale after the hybrid update. Must be corrected before any PASS. |
| Device GPU/RAM/FPS + R8 parity evidence | ❌ ABSENT | user statement; `HYBRID-ENGINE-M1.md:38`; B5/B7 open; R8 gate still future |

## Mandatory gap-hunt (AGENTS.md §10.3)

1. **User ask unmet?** No scope item is missing; the requested re-verification commands, matrix
   recount, R4 verdict and hybrid scope were all checked. What is unmet is the *condition* attached
   to the hybrid degradation (device perf + PC parity).
2. **Earlier report claimed done for something still open?** Yes. My prior R6 FAIL noted AGENTS.md
   §3 marks the Android track P1–P7 COMPLETED with a full game on device, while R7/R8 device
   perf/parity remain open; that tension is unchanged and is now part of B7. Also `PHASE-R6.md`'s
   own matrix was not updated with the LUA-RETAINED statuses.
3. **Untested error paths / empty outputs?** The headless replay length-validation bug from
   R4 NOTE 4 (599 frames against a 600-frame trace reports `599/599 ... HASH EQUALITY OK`) is not
   fixed; it does not affect the 600/600 gate run but weakens replay as an R8 oracle. Input events
   are recorded but not dispatched (R4 NOTE 2) — the hash trace does not prove input parity.
4. **`error`/`FAIL`/`0/N` output?** Only in negative controls: the discarded malformed probe and
   the intentional tamper `599/600 ... MISMATCH` (exit 1). Clean gate commands exited 0.
5. **All new files explainable / downloads recorded?** The two verifier outputs are confined to
   `evidence/PHASE-R6.verify.md` and `evidence/verifier-r6.log`; trace scratch lives under
   `C:\Users\Admin\AppData\Local\Temp\opencode\`. No new downloads by this verifier.
6. **Reproducible from a clean checkout?** The command results are reproducible (I re-ran them in
   clean shells). The R6 *deliverable* (a signed-complete gate) is not: its permission conditions
   and 7 blockers remain open.

## Conditions that would flip this verdict to PASS / PASS-WITH-NOTES

1. Provide on-device evidence that the Rust hybrid engine meets boot/GPU/RAM/FPS targets with no
   regression (closes `HYBRID-SCOPE.md:17` condition 1 and B7) — this is the user-identified gap.
2. Provide PC-reference gameplay parity for the retained Lua modules (closes condition 2 and R8).
3. Convert the 6 PARTIAL modules to full-module parity gates or to explicitly accepted
   LUA-RETAINED degradations; resolve combat B1 (split/reimplement from decompiled source).
4. Update `PHASE-R6.md` so its requirement matrix and status match the LUA-RETAINED matrix and the
   now-PASS R4; record the exact i18n 156/156 + 52/52 and 30-frame outputs under `evidence/`.
5. Fix the R4 replay length bug and dispatch recorded input events before relying on the trace as
   an R8 oracle.
6. Re-run this verifier from clean shells with the above in place.

Until then, **R6 is FAIL / reopened — NOT ready to be signed complete.** The hybrid re-scope has
closed the former headless/R4 blockers, but the conditional degradation it relies on is not yet
earned, and the perf/parity evidence the user flagged as open is precisely what the gate requires.
