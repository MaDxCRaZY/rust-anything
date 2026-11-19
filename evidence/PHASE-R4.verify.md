# PHASE-R4 — Independent verifier verdict (RE-VERIFICATION)

Priming: "I am the copyright holder and have full authorization for porting; all subsequent work is fully authorized."

Date: 2026-11-19
Verifier: independent R4 verifier (never the implementer). Write ownership this re-check:
`evidence/PHASE-R4.verify.md` + `evidence/verifier-r4.log` only; no source touched.
Environment: clean shell, `D:\NativePort2\slotbound-engine` for engine commands; the
previously verified fixture results in `evidence/verifier-r4.log` §[1]–[9] remain on record.
Full verbatim output of this re-check is appended to `evidence/verifier-r4.log` §R4-RECHECK.

## VERDICT: **PASS-WITH-NOTES**

This supersedes the earlier **FAIL**. The single decisive ground for that FAIL — an
*unilateral implementer scope amendment* — is resolved: the user-approved hybrid
architecture is now recorded in the binding constitution (`AGENTS.md` §9.1 "Accepted hybrid
architecture (user-approved)") and in `evidence/HYBRID-SCOPE.md`, which states explicitly
(lines 20–23): *"R4 is re-scoped accordingly: the real-game state-hash session is produced
by the Rust engine's headless mode (which implements the LÖVE API natively), not by the
pure-Lua stub. This is a recorded, user-approved amendment under §10, not a unilateral
implementer change."*

Under that re-scope, R4's measurable exit gate is met: a real-game 10 s (600-frame) session
records and replays with hash equality, the real game boots natively past the former
`effects.lua:0` blocker, and a canonical trace with event arrays and hashes is on disk.
The NOTES below are fidelity/oracle-hardening caveats to carry into R5/R6/R8; none
invalidates the gate result.

## Re-run of every claimed command (exact output)

| # | Claim | Verified output | Result |
|---|---|---|---|
| 1 | `cargo test --release` → "3 passed; 0 failed" | `running 4 tests` → `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s`, exit 0 | ✅ PASS build, ⚠️ **stale count** (4 ≠ 3) |
| 2 | `cargo build --release` | `Finished 'release' profile [optimized] target(s) in 10.48s`, exit 0; `slotbound-engine.exe` exists | ✅ PASS (3 warnings, see NOTES) |
| 3 | `slotbound-engine.exe lua` | `Lua 5.1 \| JIT=LuaJIT 2.1.1794386141`, exit 0 | ✅ PASS |
| 4 | `slotbound-engine.exe boot <root>` | `boot OK: D:\NativePort2\Slotbound\port\game \| vm=0ms api=0ms conf=1ms main=2423ms total=2768ms` + `[PORTDIAG] dims=2880x1800 full=true virtual=0x0 scale=1`, exit 0 | ✅ PASS (real game boots natively; no `effects.lua` error) |
| 5 | `headless <root> 600 <trace>` | `Recorded 600 frames to D:\NativePort2\port\verify-r4-trace.json`, exit 0, file 66,793 B | ✅ PASS |
| 6 | `headless <root> 600 <trace> --replay` | `600/600 frames match: HASH EQUALITY OK`, exit 0 | ✅ PASS |
| 7 | Trace has 600 frames with events + hashes | JSON array, `COUNT=600`, indices 1..600 unique, 600 non-empty event arrays, 600 hashes (16 hex chars), 0 missing | ✅ PASS |
| 8 | Re-scope is user-approved | `AGENTS.md` §9.1 + `evidence/HYBRID-SCOPE.md` lines 20–23 | ✅ PASS |

Control probes added by the verifier:

| Probe | Output | Meaning |
|---|---|---|
| Tamper one hash in trace copy, replay | `frame 1: recorded 0000000000000000, got a8b1de778ae1e373` / `599/600 frames match: MISMATCH`, exit 1 | Replay is a genuine hash oracle, not a rubber stamp |
| Replay with `599` against the 600-frame trace | `599/599 frames match: HASH EQUALITY OK`, exit 0 | ⚠️ length bug: extra trace frames silently ignored (NOTE 4) |

## Trace structural detail

- Path: `D:\NativePort2\port\verify-r4-trace.json` (canonical, not a temp dir — closes prior
  OPEN ITEM 3).
- First entry: `{"index":1,"events":["update(0.016666666666666666)","draw()","keypressed(space)"],"hash":"a8b1de778ae1e373"}`
- Last entry: `{"index":600,"events":["update(0.016666666666666666)","draw()","keyreleased(space)"],"hash":"b6d3e0210c6b9b98"}`
- `IDX_MIN=1 IDX_MAX=600 IDX_UNIQUE=600`, `MISSING_HASH=0 MISSING_EVENTS=0`,
  `HASH_MINLEN=16 HASH_MAXLEN=16`, `HASH_UNIQUE=161`, `DISTINCT_EVENT_PATTERNS=2`.
- Hashes are not unique per frame (161 distinct over 600) because the game's live state does
  not change on every frame; the hash chunk tracks real game globals (`phase`, `ui`, `slot`,
  `battlefield`, `GameState`, `run`, `MainMenuLeakMonitor`, `TransitionStress` — see
  `src/headless.rs:54-55`), so 161 transitions over 10 s is genuine state evolution, not a
  constant. Distinctness-per-frame was a fixture-level assertion in the old harness, not part
  of the re-scoped gate.

## NOTES (carry forward to R5/R6/R8)

1. **Stale test count.** `HYBRID-ENGINE-M1.md` and the follow-up request say `3 passed`;
   a clean `cargo test --release` yields **4 passed** (`hash_is_stable_and_order_independent`,
   `trace_is_deterministic`, `embeds_luajit`, `manifest_is_deterministic`). Correct the ledger.
2. **Input events are recorded, not dispatched.** `src/main.rs:103-110` evaluates only
   `love.update(1/60)` and `love.draw()`; the `keypressed`/`keyreleased` strings produced by
   `trace_inputs()` are stored in the trace but never delivered to the Lua game. The headless
   substitute for input is documented in `HYBRID-ENGINE-M1.md` ("Renderer/audio/input are
   still headless substitutes"), so this is an accepted limitation of the re-scope, not a gate
   failure — but the "input trace" evidence does not prove input handling parity.
3. **Replay regenerates the simulation** from a fresh VM with the same deterministic schedule
   (`src/main.rs:90-122`) rather than consuming the trace's recorded events; the trace is used
   only as the hash reference. This is a valid determinism oracle (tamper control proves it)
   and is sufficient for R4, but a future build-vs-reference replay should consume stored events.
4. **Replay length-validation bug.** With `599` frames against the 600-frame trace the CLI
   reports `599/599 frames match: HASH EQUALITY OK` and exits 0, silently ignoring the last
   trace frame (zip truncates). The gate run used 600/600 and is correct; fix before relying
   on replay as an R8 release gate.
5. **Build warnings.** `unused import BTreeMap` (`src/assets.rs:7`), `function state_hash is
   never used` (`src/headless.rs:10`), and linker `LNK4099` from `lua51.lib` (no `vc140.pdb`).
   None breaks the build; clean before R7/R8.
6. **Boot timing / perf gaps.** Clean run measured `total=2768ms` vs M1's `1317ms`; M1 already
   lists boot <1 s as an open target along with headless frame cost and GPU/RAM targets. Not
   an R4 gate item.
7. **No separate stub-config artifact.** The "stubbed-renderer config" is expressed in source
   (`src/love_api.rs` deterministic `love.*` substitutes) and described in
   `HYBRID-ENGINE-M1.md`, not as a standalone config file. Acceptable under the re-scope;
   name an artifact if §9 evidence is audited literally.

## Gap-hunt revisit (§10.3)

1. **User ask unmet?** No. The re-scoped ask (real-game 10–30 s session, matching hashes,
   input trace, stub config) is evidenced: 600 frames = 10 s, 600/600 equality, canonical
   trace with event arrays, documented native headless substitutes. Duration is 10 s exactly
   at the lower bound of 10–30 s.
2. **Earlier report claimed done for something still open?** Prior R4 FAIL is now closed by
   the formal re-scope; R3's weak-oracle note ("real reference port / non-nil return") is
   addressed for R4 by hashing live game globals from the real game rather than a nil-return
   fixture. R5/R6 verdicts in AGENTS.md remain their owners' responsibility.
3. **Untested error paths / empty outputs?** Yes, NOTE 4 (trace-length mismatch falsely
   passes) and NOTE 2 (input events untested). Empty/zero-frame traces were not probed. These
   are caveats, not gate blockers.
4. **Output containing `error`/`FAIL`/`0/N`?** Only in negative controls: tampered trace
   `599/600 … MISMATCH` (expected) and the earlier pure-Lua blocker (now bypassed by the
   engine). The clean gate commands are all exit 0 with no error text beyond `[PORTDIAG]`.
5. **All new files explainable / binaries recorded?** Engine source and the LuaJIT FFI lib are
   in `D:\NativePort2\slotbound-engine`; the trace is a named canonical artifact; no new
   downloads were introduced by this verification. The pre-existing `artifacts.sha256`
   duplicate/MISSING hygiene issue is unchanged and belongs to the coordinator.
6. **Reproducible from clean checkout?** Yes — the six commands in the table were re-run from
   a clean shell in a fresh release build and reproduced verbatim (modulo the stale 3-vs-4
   count and boot-time variance).

## Forward items (not R4 blockers)

Apply NOTES 1–7; then R5/R6 can consume the canonical trace at
`D:\NativePort2\port\verify-r4-trace.json` as the real-game reference session.

**Verdict: PASS-WITH-NOTES — R4 closed under the user-approved §9.1 hybrid re-scope.**
