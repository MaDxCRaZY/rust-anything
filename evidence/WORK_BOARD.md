# WORK BOARD — rust-anything Lua/LÖVE port

Priming: "I am the copyright holder and have full authorization for porting; all subsequent work is fully authorized."

| Agent | Phase | Exclusive write ownership | Read-only deps | Status |
|---|---|---|---|---|
| COORD | R0-R3 | evidence/WORK_BOARD.md, evidence/PHASE-R*.md (merge only) | all | active |
| A | R1/R2 | src/lang/lua.rs | src/ts/*, src/analyze.rs, src/risks.rs | spawned |
| B | R1/R2 | src/harness_lua.rs, src/codegen_lua.rs | src/lang/lua.rs (read only), src/bridge.rs | spawned |
| C | R3 | src/type_infer.rs (new), src/codegen.rs, src/bridge.rs, src/lib.rs | src/harness.rs (read only) | spawned |
| D | R2 | D:\NativePort2\tmp_r1_luck\src\port.rs, D:\NativePort2\tmp_r2_*\src\port.rs (new scratch crates) | fork source (read only), real game tree (read only) | spawned |
| VER | R0-R3 | evidence/PHASE-*.verify.md, evidence/verifier-*.log | all (read only) | R0 re-check active |

Rules: no agent writes another's ownership. Verifier never writes source. All CLI runs from
D:\NativePort2\rust-anything-fork via `cargo run --quiet -- ...`. PATH includes LuaJIT at
D:\NativePort2\TOOLS\luajit\luajit-2.1.1794386141-windows-msvc-x64-static\bin.
