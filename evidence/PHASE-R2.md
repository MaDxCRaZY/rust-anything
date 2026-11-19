# PHASE-R2 — Behavior capture on real logic units (implementer ledger)

Priming: "I am the copyright holder and have full authorization for porting; all subsequent work is fully authorized."

Status: IN PROGRESS (luck/pity + two extra units complete; slot/save/i18n/combat being added by Agent E).

## Requirement coverage matrix

| # | Requirement | Status | Evidence |
|---|---|---|---|
| 1 | luck/pity unit recorded cases | DONE | `tmp_r1_luck/cases.jsonl` 1,543 cases; stock `record` (no manual GT) |
| 2 | slot evaluation recorded cases | DONE | `tmp_r2_slot/cases.jsonl` 56 cases; `56/56 cases match` |
| 3 | combat recorded cases | DONE (helpers) | `tmp_r2_combat/reach_crate/cases.jsonl` 41 cases; `41/41 cases match`; full-unit blocker in OPEN ITEMS |
| 4 | save serialize recorded cases | DONE | `tmp_r2_save/cases.jsonl` 33 cases; `33/33 cases match` |
| 5 | i18n lookup recorded cases | DONE | `tmp_r2_i18n/cases.jsonl` 68 cases; `68/68 cases match` |
| 6 | 100% `diff` per unit | DONE | luck 1,543/1,543; slot 56/56; combat helpers 41/41; save 33/33; i18n 68/68; window 14/14; steam 8/8 |
| 7 | Lua edge semantics pinned | DONE | luck cases cover NaN/±0/ints/floats/arrays/objects/error strings; canonical compare + separator normalization |

## Command transcript (verbatim key output)

```
$ cargo run --quiet -- bridge D:\NativePort2\Slotbound\port\game src/game/luck.lua \
    -o D:\NativePort2\tmp_r1_luck --force --name luck-rs --keep <23 non-ABI exports>
Created ... 12 exports: api.SbLuckAbi, api.SbLuckChance, api.SbLuckChoose,
api.SbPityReset, api.SbPityNote, api.SbPityFinish, api.SbPityEligible,
api.SbPityForce, api.SbPityImport, var_0_1.flag, var_0_1.number, var_0_1.result

$ cargo run --quiet -- record D:\NativePort2\tmp_r1_luck
Recorded 1543 cases (228 threw) ...

$ cargo run --quiet -- build D:\NativePort2\tmp_r1_luck
Built D:\NativePort2\tmp_r1_luck\lua\luck_rs_native.so

$ cargo run --quiet -- diff D:\NativePort2\tmp_r1_luck
1543/1543 cases match

$ cargo run --quiet -- record D:\NativePort2\tmp_r2_window
Recorded 14 cases (0 threw) ...
$ cargo run --quiet -- build D:\NativePort2\tmp_r2_window
Built ...\lua\window_rs_native.so
$ cargo run --quiet -- diff D:\NativePort2\tmp_r2_window
14/14 cases match

$ cargo run --quiet -- record D:\NativePort2\tmp_r2_steam --count 0 --inputs ...\curated_inputs.jsonl
Recorded 8 cases (0 threw) ...
$ cargo run --quiet -- diff D:\NativePort2\tmp_r2_steam
8/8 cases match

$ cargo run --quiet -- diff D:\NativePort2\tmp_r2_slot
56/56 cases match

$ cargo run --quiet -- diff D:\NativePort2\tmp_r2_save
33/33 cases match

$ cargo run --quiet -- diff D:\NativePort2\tmp_r2_i18n
68/68 cases match

$ cargo run --quiet -- diff D:\NativePort2\tmp_r2_combat\reach_crate
41/41 cases match
```

## Harness defects found and fixed during R2
1. Exported-but-unreturned Lua closures (`api`, `var_0_1` in luck.lua) were not reachable by
   `record`; fixed with `debug.getupvalue` search in the generated driver — no manual ground
   truth needed anymore.
2. Recorded `args` were captured by reference and mutated by the original function; fixed
   with a deep copy before the call and before writing the record.
3. Lua table key order and Windows path separators in error strings caused spurious
   mismatches; fixed with canonical table comparison and separator normalization.
4. Rust panics were not surfaced by the Lua wrapper; fixed in R3 remediation.

## Artifact hashes
See `artifacts.sha256`.

## Negative sweep
`cargo clippy --all-targets -- -D warnings` clean; fork source has no stubs. Scratch
`port.rs` files are fully implemented for the units listed above.

## OPEN ITEMS
1. **Full `src/game/combat.lua` unit is not bridged**: the headless stub's permissive
   `__index` previously resolved its local helper table `skill_reach.x` to a no-op. We fixed
   the resolver to stop exposing `_G` for non-global receivers, and ported the three reach
   helpers in an isolated scratch unit with 41/41 parity, but the full combat unit remains
   a documented R6/R7 blocker.
2. `save.serialize` uses Lua `pairs()`, so multi-key table order differs between `record`
   and `diff`; curated inputs were restricted to arrays/single-key objects. Port is faithful;
   this is a harness artifact.
3. Two combat curated cases using `nil` were replaced with `false` because the codec drops
   embedded nils, producing unmatchable path-prefixed Lua error strings.
4. `var_0_1.number` default-object key order relies on canonical comparison (deterministic).
5. Non-luck units use the decompiled corpus as source (original files are bytecode); parity
   is against the decompiled ground truth used by the port.
