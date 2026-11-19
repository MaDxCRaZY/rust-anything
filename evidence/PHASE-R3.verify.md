# PHASE-R3 — independent verifier report

Priming: "I am the copyright holder and have full authorization for porting; all subsequent work is fully authorized."

Verifier: independent of the R3 implementer. Wrote only this file and `evidence/verifier-r3.log`; no source files were modified. All scratch crates were created **outside** the fork under `D:\NativePort2\verifier_r3_*`.

## Verdict: **PASS-WITH-NOTES**

The R3 gate as stated is met: (a) `port.rs` uses concrete types when the recording is consistent, (b) `build` + `diff` still report 100% (12/12, and 8/8 on the typed crate), (c) non-Lua behavior is unchanged. The notes below are evidence-quality/harness caveats, not functional failures of the inference feature.

---

## Method / environment

Every command ran from a clean shell with the required PATH prefix:

```
$env:PATH='D:\NativePort2\TOOLS\luajit\luajit-2.1.1794386141-windows-msvc-x64-static\bin;'+$env:PATH
```

True native exit codes were captured with `cmd /c "... > log 2>&1"` followed by `$LASTEXITCODE` (a first PowerShell-pipeline run of `cargo test --lib` displayed a tool-level "Exit code 1" caused by stderr being converted to error records; re-run via `cmd /c` gave the real code 0 — this is a harness artifact, not a cargo failure).

While verifying, HEAD moved from `caf0d3f6157236ddeacf9aea4a017b4ef58b0e2d` ("Complete Lua LÖVE backend…") to `ccfdab3da400a094acb094980f72ffcefb6f9647` ("Add R1, R3 evidence ledgers and remediated R0 record") — a concurrent **evidence-only** commit by another agent in the same checkout. Final `git status --porcelain` showed only `evidence/*` changes/untracked files; **no source file changed** during this verification. Commands ran against the current working tree.

---

## Requirement-by-requirement verification

### 1 & 5 — `src/type_infer.rs` exists; inference tests cover int/float/string/bool/array/object/inconsistent

File exists (`D:\NativePort2\rust-anything-fork\src\type_infer.rs`, 370 lines, SHA-256 `46e6d4693d5c2e9a94a8c52efb8f8f2ad6c0fec0f61d27c340af75e02e940036`, added in commit `caf0d3f`). I read the tests rather than trusting the ledger; all ten are real assertions:

| Category | Test (line) | Assertion |
|---|---|---|
| int / float | `infers_integers_and_floats_separately` (250) | `1,-7` → `Int`; `0.1,1e21,u64::MAX` → `Float` |
| mixed int+float | `mixed_int_and_float_falls_back_to_value` (267) | → `Json` |
| string / bool | `infers_strings_and_bools` (276) | slot 0 → `Str`, slot 1 → `Bool` |
| array | `infers_arrays_and_mixed_arrays` (283) | `[1,2,3]` → `Array(Int)`; `["a",1]`, `[]` → `Array(Json)` |
| object | `infers_objects_as_maps` (302) | → `Map`, `.rust() == "serde_json::Map<String, serde_json::Value>"` |
| `$num` marker | `recognizes_number_markers_as_floats` (312) | `NaN/Infinity/-Infinity/-0` → `Float`; `{"a":1}` → `Map` |
| inconsistent | `inconsistent_values_fall_back_to_value` (323) | string + object → `Json` |
| null / empty | `null_and_empty_carry_no_information` (332) | nulls → `Json`; empty case set → `is_empty()` |
| return source | `records_a_return_type_only_from_successes` (339) | only successful case yields `ret`; nil → `None` |
| file reader | `reads_cases_from_a_directory` (356) | reads `cases.jsonl` from a temp dir → `Map` |

Verbatim run:

```
$ cargo test --lib type_infer::tests
running 10 tests
test type_infer::tests::inconsistent_values_fall_back_to_value ... ok
test type_infer::tests::null_and_empty_carry_no_information ... ok
test type_infer::tests::infers_objects_as_maps ... ok
test type_infer::tests::infers_integers_and_floats_separately ... ok
test type_infer::tests::infers_arrays_and_mixed_arrays ... ok
test type_infer::tests::mixed_int_and_float_falls_back_to_value ... ok
test type_infer::tests::infers_strings_and_bools ... ok
test type_infer::tests::recognizes_number_markers_as_floats ... ok
test type_infer::tests::records_a_return_type_only_from_successes ... ok
test type_infer::tests::reads_cases_from_a_directory ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 163 filtered out; finished in 0.00s
EXIT=0
```

Negative sweep of `src/type_infer.rs` for `TODO|FIXME|unimplemented|placeholder|not implemented|stub`: **no hits**.

### 2 — concrete inference reproduced verbatim (before / mixed / object-only)

**(a) First bridge, no cases → `Value`:**

```
$ cargo run --quiet -- bridge tests/fixtures/lua_tiny conf.lua -o D:\NativePort2\verifier_r3_fresh --force
BRIDGE_EXIT=0
Created D:\NativePort2\verifier_r3_fresh (Ffi) for love.conf
  Cargo.toml
  src/port.rs
  .gitignore
  README.md
  rust-anything.json
  src/lib.rs
  lua/conf_rs.lua
  lua/rust_anything_codec.lua

port.rs signature: pub fn love_conf(t: serde_json::Value) -> Result<serde_json::Value, String> {
cases.jsonl present? False
```

**(b) Record, then bridge with the 12 mixed cases → still `Value`:**

```
$ cargo run --quiet -- record D:\NativePort2\verifier_r3_fresh
RECORD_EXIT=0
Recorded 12 cases (0 threw) in D:\NativePort2\verifier_r3_fresh\cases.jsonl
  love.conf: 12
```

The recorded argument shapes are mixed (string, 8 objects, null, int, bool, no-arg) — e.g. `{"fn":"love.conf","args":["x"]}`, `{"fn":"love.conf","args":[0]}`, `{"fn":"love.conf","args":[]}`.

A literal in-place re-bridge is refused **by design** (`bridge` protects a port in progress):

```
$ cargo run --quiet -- bridge tests/fixtures/lua_tiny conf.lua -o D:\NativePort2\verifier_r3_fresh --force
REBRIDGE_EXIT=2
error: D:\NativePort2\verifier_r3_fresh already exists and is not empty; rust-anything never overwrites a port in progress
```

The implementation commit explicitly supports "a folder containing only the artifacts `record` writes has no port to overwrite; leaving it bridgeable lets Lua type inference read `cases.jsonl`". So I reproduced the re-bridge by seeding fresh output dirs with the recorded `cases.jsonl` (this is the step the implementer ledger omits):

```
ALL_CASES=12 OBJECT_ONLY_CASES=8
BRIDGE_MIXED_EXIT=0   -> Created D:\NativePort2\verifier_r3_mixed (Ffi) for love.conf
BRIDGE_OBJECTS_EXIT=0 -> Created D:\NativePort2\verifier_r3_objects (Ffi) for love.conf

signature (12 mixed recorded cases):
pub fn love_conf(t: serde_json::Value) -> Result<serde_json::Value, String> {
```

**(c) Object-only consistent cases → `Map`** (derived by filtering the recorded file to the 8 lines whose first argument is a JSON object — all verbatim):

```
{"fn":"love.conf","args":[{"a":"b","window":{"width":1280}}]}
{"fn":"love.conf","args":[{"window":{"width":1280}}]}
{"fn":"love.conf","args":[{"1":"a","2":1,"window":{"width":1280}}]}
{"fn":"love.conf","args":[{"a":{"b":"c"},"window":{"width":1280}}]}
{"fn":"love.conf","args":[{"window":{"width":1280}}]}
{"fn":"love.conf","args":[{"apple":2,"Ac":5,"10":3,"2":4,"zebra":1,"window":{"width":1280},"Apple":6}]}
{"fn":"love.conf","args":[{"window":{"width":1280},"t":0.10000000000000001,"f":1e+21,"n":9007199254740992,"neg":-7}]}
{"fn":"love.conf","args":[{"1":3,"2":1,"3":2,"window":{"width":1280}}]}
```

Generated `port.rs` (verbatim — note the ledger's one-line rendering is normalized; the generator wraps a long type across three lines):

```
/// Port of `love.conf` (`conf.lua:3`).
pub fn love_conf(
    t: serde_json::Map<String, serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let _ = &t;
    unimplemented!("port `love.conf` from conf.lua")
}
```

Return remains `serde_json::Value` because `love.conf` returns nil (no `ok` value recorded) — consistent with `records_a_return_type_only_from_successes`.

SHA-256 of the three generated `port.rs` files: no-cases `4444df4b…d29`, mixed `4444df4b…d29`, object-only `ffa5552a…df8` (full hashes in `verifier-r3.log`).

### 3 — non-Lua unchanged

- Both inference call sites are guarded: `src/bridge.rs:1248 if lang == Lang::Lua`, `src/bridge.rs:1471 if manifest.language == Lang::Lua`; `src/codegen.rs:96 let lua = m.language == Lang::Lua`, with `port_rs(m)` delegating to `port_rs_with(m, &Inferred::default())` so the pre-R3 path is preserved for every other language.
- Dedicated regression test `lua_port_uses_inferred_types_and_other_languages_do_not` (`src/bridge.rs:1828`) asserts the Lua port becomes `Map` while a `Lang::Ts` manifest with the same inferred evidence stays `serde_json::Value`.
- Full suite and lint:

```
$ cargo test --lib          -> CARGO_TEST_EXIT=0
test result: ok. 173 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.43s

$ cargo clippy --all-targets -- -D warnings -> CARGO_CLIPPY_EXIT=0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.27s
```

The clippy log contains one line and zero `warning`/`error` matches. (R0's earlier `153/153` is not a contradiction: R3 added 20 lib tests.)

### 4 — fixture E2E: record → build → diff = 12/12 (also the R0 remediation)

Clean crate `D:\NativePort2\verifier_r3_e2e` (deleted before the run):

```
===== STEP bridge ===== EXIT=0
Created D:\NativePort2\verifier_r3_e2e (Ffi) for love.conf
===== STEP record ===== EXIT=0
Recorded 12 cases (0 threw) in D:\NativePort2\verifier_r3_e2e\cases.jsonl
  love.conf: 12
===== STEP build ===== EXIT=0
Built D:\NativePort2\verifier_r3_e2e\lua/conf_rs_native.so
===== STEP diff ===== EXIT=0
12/12 cases match
```

`.rust-anything/diff.json` = `{"mismatches":[],"total":12,"passed":12,"failed":0,"missing":[]}`.

Additionally, the typed `Map` crate (`D:\NativePort2\verifier_r3_objects`, 8 object-only cases) builds and diffs with its concrete signature:

```
===== STEP build (typed object-only crate) ===== EXIT=0
Built D:\NativePort2\verifier_r3_objects\lua/conf_rs_native.so
===== STEP diff (typed object-only crate) ===== EXIT=0
8/8 cases match
t: serde_json::Map<String, serde_json::Value>,
```

So the inferred signature compiles through the generated `lib.rs` argument decoder and the generated Lua wrapper, and parity is unchanged.

**Caveat (see OPEN ITEMS 1):** this fixture's function returns nil, and the generated bridge does not check the Rust `{"panic": ...}` response key. A direct FFI probe of the freshly built library returned:

```
thread '<unnamed>' (15812) panicked at src\port.rs:13:5:
not implemented: port `love.conf` from conf.lua
{"panic":"not implemented: port `love.conf` from conf.lua"}
```

`lua/conf_rs.lua` only turns `answer.error` into a Lua error, so the panic becomes a nil return; `driver`'s `same()` then compares `null` to `null` and counts a match. The literal `12/12` is therefore a harness-level pass with an unimplemented stub, not proof that a typed port computes correct values. This does not change the R3 verdict but must be carried into R4–R8 parity planning.

---

## Mandatory gap hunt

1. **Anything asked that is missing from the coverage matrix?** No functional requirement is missing. Two evidence obligations from the porting constitution §10.1 are only nominally met: (i) "Artifact hashes" points to `artifacts.sha256`, which contains a single line — the LuaJIT download — and **no SHA-256 of any R3 deliverable**; (ii) the negative sweep and the diff caveat are asserted but not itemized in the ledger.
2. **Earlier "done" claims contradicting open items?** None. R0's `12/12` and R3's requirement 4 agree; R1/R2 make no typed-inference claims. `docs/STATUS.md` still has no Lua row even though the project convention says to update it when functionality changes — documentation drift, not a contradiction.
3. **Untested error paths / empty or zero-length outputs?** Yes, several (all minor or deferred): `from_cases_dir`'s documented "missing / empty / partially malformed lines are skipped" branch has no unit test (only `reads_cases_from_a_directory` with two valid lines); no explicit zero-length `cases.jsonl` bridge test; `inferred_param_type`'s rest/optional wrapping is not unit-tested; no non-nil-return fixture exercises a typed port through `build`/`diff`.
4. **Command output containing `error`, `FAIL`, `0/N` with N>0, or a silent fallback?** All phase commands exited 0 with no warnings. Two findings: (a) the design-intended in-place re-bridge refusal (`EXIT=2`, shown above), which the ledger does not mention; (b) a **silent fallback** — Rust panics are serialized as `{"panic": ...}` and swallowed by the generated Lua wrapper, so a stub passes `diff` (demonstrated above). The driver's `Rust panic:` guard does not catch this shape.
5. **New files explainable / downloads recorded?** `src/type_infer.rs`, the `bridge.rs` test, and the `Lang::Lua` guards are all in commit `caf0d3f` and explainable. The implementer's four scratch dirs (`lua_tiny_infer_demo`, `lua_tiny_fresh_rs`, `lua_tiny_rs`, `lua_tiny_rs_backup_r3`) are disclosed in PHASE-R3.md OPEN ITEM 3; my four `verifier_r3_*` dirs are created solely for this verification. Only downloaded binary is LuaJIT `2.1.1794386141`, recorded in `artifacts.sha256` with URL and `sha256=3f16e1091f8252a88c5484acfc65d4fe34a12043dee588809a23f5e44c06d360`; no unrecorded downloads found.
6. **Reproducible from a clean checkout with only documented commands?** Tests, clippy and the E2E: yes. Inference signatures: yes, with the undocumented-but-required step of seeding a fresh output dir with `cases.jsonl` (direct re-bridge into a populated crate is refused). The "see artifacts.sha256" hash claim is **not** reproducible/verifiable because that file has no deliverable hashes.

---

## OPEN ITEMS / notes for the coordinator

1. **Weak parity oracle:** `12/12` is achieved with an `unimplemented!` stub because `love.conf` returns nil and the wrapper ignores `answer.panic`. Add a fixture with a real reference port and a non-nil return before relying on `diff` for R4–R8 parity. (Harness/R0-scope issue, carried forward.)
2. **Artifact hashes missing:** `evidence/artifacts.sha256` records only the LuaJIT download, not R3 deliverables. Add hashes of `src/type_infer.rs`, the generated `port.rs` variants and the evidence files.
3. **Ledger precision:** the transcript's re-bridge step needs the fresh-dir-with-cases nuance; the object-only signature is generated over three lines, not one.
4. **Concurrent checkout activity:** HEAD moved during verification (evidence-only commit). Source was untouched; re-check HEAD before any R4 work.
5. `docs/STATUS.md` has not been updated with a Lua row despite the project convention.
6. Carry-forward items already in PHASE-R3.md (mixed params → `Value`; objects → `Map` not named structs; scratch dirs outside the fork) remain accurate.

## Final decision

**PASS-WITH-NOTES.** R3's measurable gate is satisfied: consistent evidence produces `serde_json::Map<String, serde_json::Value>` (verbatim shown), inconsistent/no evidence correctly keeps `serde_json::Value`, `build`+`diff` remain 100% (12/12 and 8/8), non-Lua code is gated and all 173 lib tests plus clippy pass. The notes are evidence-quality and downstream-parity risks, not reasons to reopen R3.
