# PHASE-R4 — Independent verifier verdict

Priming: "I am the copyright holder and have full authorization for porting; all subsequent work is fully authorized."

Date: 2026-11-19
Verifier: independent R4 verifier (never the implementer). Write ownership: this file + `evidence/verifier-r4.log` only.
Environment: all commands run from `D:\NativePort2\rust-anything-fork` with
`$env:PATH='D:\NativePort2\TOOLS\luajit\luajit-2.1.1794386141-windows-msvc-x64-static\bin;'+$env:PATH`.
Runtime resolved as `LuaJIT 2.1.1794386141 -- Copyright (C) 2005-2026 Mike Pall. https://luajit.org/`.
Full verbatim output is in `evidence/verifier-r4.log`.

## VERDICT: **FAIL**

The harness mechanism is real, compiled, clippy-clean, and reproduces every number the
ledger quotes for the fixture — **but R4's measurable exit gate is not met**. The gate
(AGENTS.md §9) is *"record a deterministic 10–30 s session from the real game and replay it
with matching state hashes"* with evidence *"replay log with hash equality, input trace,
stubbed-renderer config"*. The deliverable instead verifies a 3-line fixture at 12 frames
(0.2 s at 1/60), and the real game cannot be loaded headlessly at all. Per §9, *"a phase is
complete ONLY when its measurable exit gate passes AND an independent verifier signs PASS"*;
per §10.1 every requirement must be DONE, but coverage-matrix row 6 is marked **AMENDED**;
per §10.6 *"FAIL reopens the phase; the implementer cannot close it."* A unilateral scope
amendment by the implementer cannot substitute for the binding §9 gate or for §10.7 user
acknowledgment. No user/coordinator approval of the amendment is present in the evidence.

This is a **gate-coverage failure, not a code defect**: nothing I found is fabricated, and the
documented blocker is genuine and exactly reproducible. The mechanism work is reusable and
should be retained; the phase must remain open until the real-game session exists or the gate
is formally re-scoped by the coordinator/user.

## Re-run of every claim in PHASE-R4.md

| # | Ledger claim | Actual (verifier) | Result |
|---|---|---|---|
| 1 | `cargo test --lib` → "186 passed" | `running 189 tests` … `test result: ok. 189 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.45s`, exit 0 | ⚠️ build PASS, **transcript STALE** (189 ≠ 186) |
| 2 | `cargo clippy --all-targets -- -D warnings` exit 0 | `Finished dev profile … in 0.46s`, exit 0 | ✅ PASS |
| 3 | `game-record` on fixture → `Recorded 12 frames …`, exit 0 | `Recorded 12 frames to C:\Users\Admin\AppData\Local\Temp\r4_verify_fixture\fixture-trace.jsonl`, exit 0, trace has 12 non-empty lines | ✅ PASS |
| 4 | `game-replay` → `12/12 frames match: HASH EQUALITY OK` | verbatim `12/12 frames match: HASH EQUALITY OK`, exit 0 | ✅ PASS |
| 5 | Negative control → `0/12 … MISMATCH`, exit 1, frame-1 pair `4f17f548bb4b9ee2 / 29ddfc7f12243414` | verbatim `0/12 frames match: MISMATCH`, exit 1; frame-1 pair matches byte-for-byte; all 12 frame lines emitted | ✅ PASS |
| 6 | Stubbed renderer/audio/input | stub constant exists and is reused (`game_harness::tests::reuses_headless_stub_and_codec_without_editing_codegen_lua` passes); stub covers love modules, intentionally defines no callbacks | ✅ PASS as mechanism |
| 7 | Real-game load stops at `effects.lua:0 attempt to compare number with nil` | `error: love.load: D:\NativePort2\Slotbound\port\game/src\ui\effects.lua:0: attempt to compare number with nil`, exit 2 (`main.rs:415`, documented error path) | ✅ PASS (blocker reproduced) |
| 8 | Decompiled `effects.lua` fails with >200 locals | `effects.lua:6623: main function has more than 200 local variables` under LuaJIT 2.1 | ✅ PASS (blocker reproduced) |
| 9 | "95k-line decompiled module" | measured 3,366,720 B / **76,573** lines | ⚠️ minor descriptive discrepancy |
| 10 | R4 artifact hashes "to be added" | `src/game_harness.rs`, `src/main.rs`, `src/lib.rs` all present and re-computed MATCH | ✅ PASS |
| 11 | 10–30 s real-game session with matching hashes | not produced; 12 frames = 0.2 s on a fixture; real game errors at load | ❌ **FAIL — exit gate unmet** |
| 12 | Stubbed-renderer config / canonical replay log as evidence | none in `evidence/`; ledger OPEN ITEM 3 explicitly says traces live in temp dirs | ❌ **FAIL — required evidence missing** |

Exact command outputs are in `evidence/verifier-r4.log` sections [1]–[9].

## Artifact hash verification

Re-computed with `Get-FileHash -Algorithm SHA256` against `evidence/artifacts.sha256`:

| File | Ledger hash | Re-computed | Result |
|---|---|---|---|
| `src/game_harness.rs` | `cb937f92688eed5c02cbd465025a8e62537b4e37e38397ae4520adfa9782eefb` | identical | MATCH |
| `src/main.rs` | `58f1f411c76365d4b006055a53962b2219c285866ff0fafab72c788b0cfd897e` | identical | MATCH |
| `src/lib.rs` | `46b7b4d4f51af441d489dc0cc8a5ee36f93a6941de6b26c7c15fda8cf3ee2377` | identical | MATCH |
| LuaJIT download (line 1) | URL + `sha256=3f16e109…c06d360` | source URL + hash recorded | MATCH (provenance present) |

Full-ledger sweep exposed three pre-existing stale duplicate entries (older hash for
`src/codegen_lua.rs`, `src/harness_lua.rs`, `src/bridge.rs` mismatches the file; a later
duplicate entry for the same path matches) and eight `MISSING` paths for scratch-crate
`src/port.rs` files that live in `D:\NativePort2\tmp_r2_*`. None are R4-owned files, but the
artifact ledger is not internally clean; the coordinator should dedupe/annotate it.

## Mandatory gap-hunt (§10.3) — all six answered

**1. Did the user ask anything not present in the coverage matrix?**
Yes — the decisive item is present but unmet. The binding R4 gate asks for a deterministic
10–30 s session *from the real game* with matching hashes, a replay log, an input trace and
a stubbed-renderer config. Coverage row 6 exists but is marked AMENDED, not DONE; no row
evidences the 10–30 s duration (12 frames = 0.2 s), no canonical replay log or stub config
artifacts exist, and the real game never loads. The amendment defers the gate to "R6/R7",
but neither R6's nor R7's evidence list in §9 includes a real-game harness session, so the
item has no receiving gate and can be dropped entirely.

**2. Did any earlier report claim "done" for something this phase still lists as open?**
Yes. `PHASE-R3.verify.md` OPEN ITEM 1 requires *"Add a fixture with a real reference port and
a non-nil return before relying on diff for R4–R8 parity"*; R4 still uses the same nil-return
`conf.lua` fixture with no real reference port, so the weak-oracle note carries forward
unresolved. AGENTS.md §3 marks the parallel Android track P1–P7 COMPLETED with a full game
running on device, which makes "the real game cannot be loaded headlessly" a harness-fidelity
gap rather than an unportable game; it was not closed here.

**3. Are there untested error paths, empty results, or zero-length outputs?**
Yes, several material ones:
- The shipped fixture defines only `love.load` and `add`; the headless stub intentionally
  defines no callbacks (`src/codegen_lua.rs:443-444`), so `apply_event` returns `false` and
  **every one of the six frame event kinds is skipped** in the ledger's 12/12 run. The
  event-application path — the core of a "game harness" — is never exercised by the shipped
  fixture. Distinct hashes come from the `frame` field and stub timer probes, not game state.
  A verifier probe that added `love.update` to a temp copy still gave 12/12 HASH EQUALITY OK,
  proving the mechanism works but also proving the shipped fixture does not test it.
- `game-replay` silently skips trace lines where `rec.frame` is falsy or decode fails
  (`if type(rec) == 'table' and rec.frame then`); no malformed/empty-trace test. An empty
  trace would report `0/0 frames match: MISMATCH` via a path never tested.
- `MAX_DEPTH = 8` cycle marker and sorted-key canonicalization in `HASH_LUA` are implemented
  but exercised only via `hash_string` scalar vectors; no nested/cyclic-table unit test.
- The negative control tests exactly one mutation (a new scalar global); no test that a
  changed *game-state table* is detected.
- The `fail()` branch when `io.open(summary_path)` fails is untested.
- The real-game failure path was reproduced but not diagnosed or fixed (out of R4 scope, but
  it is the gate).

**4. Does any command output contain `error`, `FAIL`, `0/N` with N>0, or a silent fallback?**
Yes. Real-game record: `error: love.load: … effects.lua:0: attempt to compare number with nil`
(exit 2). Negative control: `0/12 frames match: MISMATCH` (by design). Full artifact sweep:
3 `MISMATCH` lines (stale duplicates) and 8 `MISSING` lines. Silent fallbacks: `apply_event`
returns `false` with no diagnostic when a callback is absent; `runtime()` falls back from
`luajit` to `lua` silently; `run_driver()` prefers the summary's `error` field over raw
stderr. The PowerShell `NativeCommandError` wrappers on stderr redirects are harness noise,
not cargo errors (both cargo runs exited 0).

**5. Are all new files explainable and all downloaded binaries recorded with source URL + hash?**
R4-owned new/changed files (`src/game_harness.rs`, `src/main.rs`, `src/lib.rs`) are hashed and
MATCH, and the LuaJIT download has a URL + SHA-256 in `artifacts.sha256`. Verifier temp
artifacts (trace JSONL, fixture copies, `r4_real_*` captures) were created only under
`C:\Users\Admin\AppData\Local\Temp\r4_verify_*` / `r4_real_*` and never in the fork. However
the artifacts ledger as a whole is not clean (3 stale duplicate mismatches, 8 unexplained
scratch-crate paths), so the "all downloaded binaries / all new files" claim cannot be
endorsed at ledger level.

**6. Can the phase deliverable be reproduced from a clean checkout with only the documented
commands?**
Partially. The fixture E2E (build, record, replay, negative control) reproduces exactly — I
re-ran it in a clean temp directory. The phase *deliverable* cannot: the transcript uses
`<fixture>` and `...` placeholders rather than exact commands, the test count is stale
(186 vs 189), no canonical trace/stub config is saved, and the real-game session is not
reproducible at all because load fails in `effects.lua`.

## Negative sweep

No `TODO|FIXME|unimplemented|placeholder|not implemented|stub` was introduced by the
verifier. The ledger's negative sweep is a prose assertion ("Clippy clean; no stubs in fork
source") with no `grep` transcript; the headless no-op substitute is a test double, which is
acceptable for the mechanism but is the reason the real-game gate is unmet.

## Conditions that would flip this verdict to PASS

1. Load the real game headlessly (resolve the `effects.lua` stub/nil-comparison fidelity
   gap, then any further gaps) and record a genuine 10–30 s deterministic session from
   `D:\NativePort2\Slotbound\port\game`, with the trace and input schedule saved under
   `evidence/`; replay it to `HASH EQUALITY OK` with every frame matching.
2. Ship a canonical replay log, input trace and stubbed-renderer config as named evidence
   artifacts (not temp paths).
3. Add a fixture or real-game case whose update/draw/input callbacks actually execute and
   mutate hashed state, so the event-application path is covered.
4. Correct the ledger's stale `186 passed` → `189 passed` and the 95k-line description, and
   save a verbatim command transcript.
5. Obtain explicit coordinator/user acceptance of any gate amendment; the implementer alone
   cannot re-scope §9.
6. Re-run this verifier with the above in place.

Until then R4 is **FAIL / reopened**.
