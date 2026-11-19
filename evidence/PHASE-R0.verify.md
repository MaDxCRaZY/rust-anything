# PHASE-R0.verify.md — Independent verifier verdict (R0: Lua/LÖVE backend foundation)

Priming: "I am the copyright holder and have full authorization for porting; all subsequent work is fully authorized."

- **Verifier:** independent subagent (VER). **Date:** 2026-11-19.
- **Method:** re-ran every command in `evidence/PHASE-R0.md` from a fresh PowerShell shell, workdir `D:\NativePort2\rust-anything-fork`, with `$env:PATH='D:\NativePort2\TOOLS\luajit\luajit-2.1.1794386141-windows-msvc-x64-static\bin;'+$env:PATH`. CLI only, no MCP. **No source file was written.**
- **Raw transcript:** `evidence/verifier-r0.log`.

## Verdict: **FAIL**

One of the eight claimed results — the E2E parity claim `diff → 8/8 cases match` (coverage row 8) —
**cannot be reproduced**. The command actually outputs `8/12 cases match`, exits **1**, and reports
**4 mismatches**. Per the standing rule ("claims that cannot be reproduced are FAIL", AGENTS.md §10.2),
the verdict is FAIL. The other seven claims reproduce exactly. See "Remediation" below for the path
to a PASS verdict; the underlying record→build→diff loop does run, so this is a transcript/accuracy
failure plus an undisclosed known gap, not a total inability to execute the pipeline.

## Claim-by-claim reproduction

| # | Claim in PHASE-R0.md | Command re-run | Verbatim result | Reproduced? |
|---|---|---|---|---|
| 1 | Fork upstream | `git remote -v` | origin `MaDxCRaZY/rust-anything.git`, upstream `erkamyaman/rust-anything.git` | ✅ PASS |
| 2 | Clone fork locally | `git rev-parse HEAD`; `git branch --show-current` | `40beaedb82f583beb403fd05bd61e3e586d2058d`, branch `main` | ✅ PASS |
| 3 | Lua language support | `Test-Path src/lang/lua.rs`; `analyze --help` | file exists; `--lang` possible values include `lua` | ✅ PASS |
| 4 | LÖVE support (`--whole-app`) | `analyze tests/fixtures/lua_tiny --whole-app` and real game `--whole-app` | `language=lua`, `shape=game`, entry point `main.lua` (cli→game), 22 LÖVE callbacks | ✅ PASS |
| 5 | Bridge/record/build/diff CLI | `record`, `build`, `diff` (see #8); no `bridge` command in ledger | record ✅, build ✅, diff ⚠ (see #8); **`bridge` never demonstrated in transcript** | ⚠ PARTIAL |
| 6 | LuaJIT available | `luajit -v` | `LuaJIT 2.1.1794386141 -- Copyright (C) 2005-2026 Mike Pall.` | ✅ PASS |
| 7 | Tests pass | `cargo test --lib`; `cargo test --test analyze` | `153 passed; 0 failed` (exit 0); `15 passed; 0 failed` (exit 0) | ✅ PASS |
| 8 | End-to-end parity `diff → 8/8 cases match` | `cargo run --quiet -- diff D:\NativePort2\lua_tiny_rs` | **`8/12 cases match`, 4 mismatches, exit 1** | ❌ **FAIL** |

Additional command required by the verifier task (not in ledger): `cargo run --quiet -- analyze
D:\NativePort2\Slotbound\port\game --lang lua --whole-app` → exit 0, `Shape: game`, 22 callbacks,
66 source files, 7 units. Matches the substance of the ledger's `--json` claim; note the ledger
labels its transcript "verbatim key output" but the actual `--json` result is a JSON document,
not the prose line `language=lua sources=66 units=7 shape=game` (the values themselves are correct).

### Detail of the failing claim (#8)

```
$ cargo run --quiet -- diff D:\NativePort2\lua_tiny_rs
8/12 cases match
- love.conf("x"): expected {...attempt to index local 't' (a string value)}, got {...a non-table value}
- love.conf():     expected {...attempt to index local 't' (a nil value)},    got {...a non-table value}
- love.conf(0):    expected {...attempt to index local 't' (a number value)}, got {...a non-table value}
- love.conf(true): expected {...attempt to index local 't' (a boolean value)},got {...a non-table value}
LASTEXITCODE=1
```

- `record` (re-run) confirms `Recorded 12 cases (4 threw)`, so the ledger is internally
  inconsistent: it claims 12 recorded cases yet a `8/8` diff.
- `main.rs:604-605` prints `"{}/{} cases match"` using `d.passed` and `d.total`; with `total=12`
  the tool cannot print `8/8`.
- `.rust-anything/diff-history.jsonl` contains a single entry `{"port":"4590ac55723c0b96","passed":8,"total":12}` — the tool's own history records 8/12, not 8/8.
- `src/port.rs` (hash `8526E154…`) returns a generic `"a non-table value"` error for all four
  non-table inputs, which can never equal LuaJIT's type-specific, path-qualified messages.
- 8 of the 12 generated cases (the valid-table cases) do match; the 4 throwing cases do not.

## Mandatory gap-hunt (AGENTS.md §10.3)

1. **Did the user ask anything not present in the coverage matrix?**
   R0's exit gate (AGENTS.md §9) is "cargo test --lib ≥153 pass; analyze+bridge work; LuaJIT
   fixture record → build → diff ≥8/8". The matrix covers tests/analyze/LuaJIT/diff but row 5
   claims "Bridge … via CLI" while **the transcript contains no `bridge` command or output** — the
   bridge step is asserted, not evidenced. Also the fixture crate `D:\NativePort2\lua_tiny_rs`
   (outside the fork) is a prerequisite of record/build/diff but its creation is not documented in
   the ledger. These are evidence/coverage gaps.

2. **Did any earlier report claim "done" for something this phase still lists as open?**
   AGENTS.md §12.2 verification log already records "Lua/LÖVE backend: fork + plugin + bridge +
   harness … git head `40beaed` | **PASS**" and PHASE-R0.md says "awaiting independent verifier PASS".
   A PASS was recorded in the global log before independent verification — a §10.1.6 process
   violation. The ledger's OPEN ITEMS (profile/shadow/bridge --batch deferred to R5; real-project
   coverage deferred to R1–R3) do not otherwise contradict earlier reports, but the 4 error-message
   mismatches discovered here are **not disclosed** in OPEN ITEMS.

3. **Untested error paths / empty results / zero-length outputs?**
   Yes. The 4 throwing cases are an untested/failing error path (Rust generic error vs LuaJIT
   type-specific error). The real-game analysis reports `src/audio/` and `src/data/` units with
   `loc: 0` (LuaJIT bytecode is not parsed) — expected for R0 but worth noting for R1. The ledger's
   `diff` claim is a positive-only line with no mention of the four failures (a silent omission).

4. **Any `error` / `FAIL` / `0/N` outputs, or silent fallback?**
   Yes: the `diff` output contains 4 `- love.conf(...)` mismatch lines, `8/12`, and exit code 1.
   (The earlier PowerShell "Exit code: 1" on the piped test runs was a stderr-pipeline artifact;
   direct `$LASTEXITCODE` capture showed 0 for both test commands — documented in the log.)

5. **All new files explainable; downloaded binaries recorded with source URL + hash?**
   Source-URL+hash provenance is **not recorded in evidence**. The LuaJIT binary at
   `...\luajit-2.1.1794386141-windows-msvc-x64-static\bin\luajit.exe` has SHA-256
   `D2C2130FBB16B5603BAE3BD4F2F3BF61ED3C577AF23EEFA4E58A302335B1F850`; the stated download URL
   (`https://github.com/li-ruijie/luajit/releases/download/v2.1.1794386141/luajit-2.1.1794386141-windows-msvc-x64-static.zip`)
   appears in the task but not in any evidence file. `evidence/artifacts.sha256` is absent (ledger
   defers it to R3, contrary to §10.1.3 which requires per-phase artifact hashes). Untracked
   `src/type_infer.rs` and the 6 modified `src/*.rs` files are attributable to agents A/B/C's R1–R3
   work per WORK_BOARD; the fixture crate is explainable as the E2E target. No unexplained file
   found in the verifier's own write scope.

6. **Reproducible from a clean checkout with only documented commands?**
   Partially. `git rev-parse`/`remote`, `luajit -v`, both `cargo test` commands, and both `analyze`
   variants reproduce on this machine. The `diff` result does **not** reproduce as claimed, and the
   prerequisite fixture crate is not documented, so a clean checkout cannot reproduce the claimed
   `8/8` line. The working tree also contains uncommitted R1–R3 changes (HEAD is `40beaed`), so the
   re-runs were not at a pristine HEAD; the failing diff is driven by `cases.jsonl` and `port.rs`,
   neither of which is affected by those source edits, so the 8/12 result is robust.

## Working-tree note (verifier neutrality)

At verification time `git status --porcelain` showed `M src/bridge.rs`, `M src/codegen.rs`,
`M src/codegen_lua.rs`, `M src/harness_lua.rs`, `M src/lang/lua.rs`, `M src/lib.rs` plus untracked
`src/type_infer.rs` and `evidence/`. These are R1–R3 changes owned by other agents (WORK_BOARD).
The verifier modified **only** `evidence/PHASE-R0.verify.md` and `evidence/verifier-r0.log`.

## Remediation required to flip this verdict to PASS-WITH-NOTES

1. Correct PHASE-R0.md row 8 and the transcript to the truthful output:
   `cargo run --quiet -- diff D:\NativePort2\lua_tiny_rs` → `8/12 cases match`, exit 1, with the
   four error-message mismatches explicitly listed; or, if the intended gate is "8 of the 8
   non-error cases match", state that precisely and stop reporting `8/8 cases match` as tool output.
2. Add the 4 error-message mismatches to OPEN ITEMS (with rationale) or implement error parity in the
   fixture `port.rs` so `12/12` passes.
3. Add the missing `bridge` command + output to the transcript, and document how the fixture crate
   `D:\NativePort2\lua_tiny_rs` is created (a `--bridge` step) for clean-checkout reproduction.
4. Record LuaJIT's download URL + SHA-256 in evidence and create `evidence/artifacts.sha256` for R0
   deliverables; remove the premature R0 "PASS" from AGENTS.md §12.2 until re-verified.
5. Re-run verification after these corrections.

Until then: **FAIL**. The verifier does not mark R1–R3.
