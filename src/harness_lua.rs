//! The Lua/LÖVE side of record, build, diff and bench. A generated Lua driver calls the
//! original functions (or the LuaJIT FFI wrapper over the Rust cdylib) by name.

use crate::walk::Canonical;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::bridge::Manifest;
use crate::harness::{BenchRow, CASES, DiffOutcome};

const WORK_DIR: &str = ".rust-anything";

fn runtime() -> Result<&'static str> {
    for name in ["luajit", "lua"] {
        if Command::new(name).arg("-v").output().is_ok() {
            return Ok(name);
        }
    }
    bail!("no Lua runtime found: install LuaJIT 2.1 (preferred) or Lua 5.1+ on PATH")
}

fn run(cmd: &mut Command, what: &str) -> Result<()> {
    let output = cmd.output().with_context(|| format!("running {what}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail: Vec<&str> = stderr.lines().rev().take(30).collect();
        bail!(
            "{what} failed ({}):\n{}",
            output.status,
            tail.into_iter().rev().collect::<Vec<_>>().join("\n")
        );
    }
    Ok(())
}

pub fn wrapper_path(m: &Manifest) -> String {
    format!("lua/{}.lua", m.lib_name())
}

pub fn native_path(m: &Manifest) -> String {
    let ext = if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    };
    format!("lua/{}_native.{ext}", m.lib_name())
}

fn files(crate_dir: &Path, m: &Manifest, rust: bool) -> Result<Vec<PathBuf>> {
    if rust {
        let path = crate_dir.join(wrapper_path(m)).canonical()?;
        return Ok(vec![path]);
    }
    let mut files: Vec<PathBuf> = m
        .exports
        .iter()
        .map(|f| m.root.join(&f.file))
        .chain(m.js_only.iter().map(|j| m.root.join(&j.file)))
        .collect();
    files.sort();
    files.dedup();
    Ok(files)
}

fn driver_command(
    crate_dir: &Path,
    m: &Manifest,
    mode: &str,
    cases: &Path,
    out: &Path,
    min_ms: u64,
    rust: bool,
) -> Result<Command> {
    let work = crate_dir.join(WORK_DIR);
    std::fs::create_dir_all(work.join("lua"))?;
    std::fs::write(
        work.join("rust_anything_codec.lua"),
        crate::codegen_lua::CODEC_LUA,
    )?;
    // The driver requires this before any original file, so load-time LÖVE calls are safe
    // while LÖVE callbacks can still be assigned onto the same global table.
    std::fs::write(
        work.join("rust_anything_headless_stub.lua"),
        crate::codegen_lua::HEADLESS_STUB_LUA,
    )?;
    std::fs::write(work.join("driver.lua"), crate::codegen_lua::driver())?;
    let targets = work.join("targets.json");
    std::fs::write(&targets, crate::codegen_lua::targets(m).to_string())?;

    let mut cmd = Command::new(runtime()?);
    // `require('rust_anything_codec')` lives in the work dir; original sources resolve
    // their own `require` calls relative to the repository root.
    let lua_path = format!(
        "{}/?.lua;{}/?/init.lua;{}/?.lua;{}/?/init.lua",
        work.display(),
        work.display(),
        m.root.display(),
        m.root.display()
    );
    cmd.env("LUA_PATH", lua_path)
        // Lets the driver read the original source to map local variable names onto the
        // keys of the table a module returns (disambiguates nested-table exports).
        .env("RUST_ANYTHING_LUA_ROOT", m.root.as_os_str())
        .arg(work.join("driver.lua"))
        .arg(mode)
        .arg(targets)
        .arg(cases)
        .arg(out)
        .arg(min_ms.to_string())
        .args(files(crate_dir, m, rust)?);
    if rust {
        let lib = crate_dir.join(native_path(m)).canonical()?;
        cmd.env("RUST_ANYTHING_LUA_LIB", lib);
    }
    Ok(cmd)
}

fn driver(
    crate_dir: &Path,
    m: &Manifest,
    mode: &str,
    cases: &Path,
    out: &Path,
    min_ms: u64,
    rust: bool,
) -> Result<()> {
    let mut cmd = driver_command(crate_dir, m, mode, cases, out, min_ms, rust)?;
    run(&mut cmd, &format!("lua driver ({mode})"))
}

pub fn record(crate_dir: &Path, m: &Manifest, cases: &[Value]) -> Result<PathBuf> {
    let out = crate_dir.join(CASES);
    let pending = crate_dir.join(WORK_DIR).join("recording.jsonl");
    crate::harness::record_cases(
        cases,
        &crate_dir.join(WORK_DIR),
        &pending,
        "lua driver (record)",
        |inputs, part| driver_command(crate_dir, m, "record", inputs, part, 0, false),
    )?;
    let recorded: std::collections::HashSet<String> = std::fs::read_to_string(&pending)?
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter_map(|c| c["fn"].as_str().map(str::to_string))
        .collect();
    let mut asked: Vec<&str> = cases.iter().filter_map(|c| c["fn"].as_str()).collect();
    asked.sort_unstable();
    asked.dedup();
    if let Some(name) = asked.into_iter().find(|name| !recorded.contains(*name)) {
        bail!(
            "cannot encode recorded result for {name}: no case could be written as JSON; \
             kept the previous {CASES}"
        );
    }
    std::fs::rename(&pending, &out).with_context(|| format!("publishing {}", out.display()))?;
    Ok(out)
}

pub fn diff(crate_dir: &Path, m: &Manifest) -> Result<DiffOutcome> {
    let cases = crate_dir.join(CASES);
    if !cases.exists() {
        bail!(
            "no {CASES} in {} (run `rust-anything record` first)",
            crate_dir.display()
        );
    }
    let out = crate_dir.join(WORK_DIR).join("diff.json");
    driver(crate_dir, m, "diff", &cases, &out, 0, true)?;
    Ok(serde_json::from_str(&std::fs::read_to_string(&out)?)?)
}

pub fn bench(crate_dir: &Path, m: &Manifest, min_ms: u64) -> Result<Vec<BenchRow>> {
    let cases = crate_dir.join(CASES);
    if !cases.exists() {
        bail!(
            "no {CASES} in {} (run `rust-anything record` first)",
            crate_dir.display()
        );
    }
    let mut timings = Vec::new();
    for rust in [false, true] {
        let out = crate_dir.join(WORK_DIR).join(if rust {
            "bench-rust.json"
        } else {
            "bench-original.json"
        });
        driver(crate_dir, m, "bench", &cases, &out, min_ms, rust)?;
        timings.push(serde_json::from_str(&std::fs::read_to_string(&out)?)?);
    }
    Ok(crate::harness::bench_rows(m, &timings[0], &timings[1]))
}

pub fn build(crate_dir: &Path, m: &Manifest) -> Result<PathBuf> {
    let lib = crate::harness_c::build(crate_dir, m)?;
    let ext = if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    };
    let target = crate_dir.join(format!("lua/{}_native.{ext}", m.lib_name()));
    std::fs::create_dir_all(crate_dir.join("lua"))?;
    std::fs::copy(&lib, &target)?;
    // The generated wrapper and codec sit beside the native library.
    std::fs::write(
        crate_dir.join(wrapper_path(m)),
        crate::codegen_lua::wrapper(m).map_err(anyhow::Error::msg)?,
    )?;
    std::fs::write(
        crate_dir.join("lua/rust_anything_codec.lua"),
        crate::codegen_lua::CODEC_LUA,
    )?;
    Ok(target)
}
