//! Step 4: a drop-in module that runs the TS original and the Rust port side by side.
//!
//! Import the generated module where the app imported the original. Every call returns the
//! original's result; the Rust result is compared and mismatches are reported. Once production
//! traffic shows no mismatches, `RUST_ANYTHING_MODE=new` switches to Rust, still without a code change.

use crate::walk::Canonical;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::bridge::Manifest;
use crate::harness::CODEC_JS;

#[derive(Debug, Serialize)]
pub struct ShadowOutcome {
    pub path: PathBuf,
    pub exports: Vec<String>,
}

pub fn shadow(crate_dir: &Path, out: Option<&Path>) -> Result<ShadowOutcome> {
    let crate_dir = crate_dir
        .canonical()
        .with_context(|| format!("resolving {}", crate_dir.display()))?;
    let manifest = Manifest::load(&crate_dir)?;
    if manifest.language == crate::lang::Lang::Python {
        return shadow_python(&crate_dir, &manifest, out);
    }
    if manifest.language == crate::lang::Lang::Lua {
        return shadow_lua(&crate_dir, &manifest, out);
    }
    if manifest.kind == crate::bridge::Kind::CAbi {
        return shadow_c(&crate_dir, &manifest, out);
    }
    if manifest.language == crate::lang::Lang::Java {
        let dir = out.map_or_else(|| crate_dir.join("java"), Path::to_path_buf);
        let port = std::fs::read_to_string(crate_dir.join("src/port.rs")).unwrap_or_default();
        let mut api = crate::codegen::parse_port(&port);
        crate::bridge::match_export_names(&manifest, &mut api);
        let files = crate::codegen_java::shadows(&manifest, &api);
        for (path, _) in &files {
            let target = dir.join(path.trim_start_matches("shadow/"));
            if target.exists() {
                bail!("{} exists; remove it first or pass --out", target.display());
            }
        }
        let mut first = None;
        for (path, text) in &files {
            let target = dir.join(path.trim_start_matches("shadow/"));
            std::fs::create_dir_all(target.parent().context("no folder")?)?;
            std::fs::write(&target, with_kill_switch(text, &manifest.crate_name))?;
            first.get_or_insert(target);
        }
        return Ok(ShadowOutcome {
            path: first.context("no class to shadow")?,
            exports: manifest.exports.iter().map(|f| f.name.clone()).collect(),
        });
    }
    if manifest.language == crate::lang::Lang::Ruby {
        let path = match out {
            Some(p) => p.to_path_buf(),
            None => crate_dir
                .join("ruby")
                .join(format!("{}_shadow.rb", manifest.lib_name())),
        };
        if path.exists() {
            bail!("{} exists; remove it first or pass --out", path.display());
        }
        std::fs::create_dir_all(path.parent().context("output path has no folder")?)?;
        std::fs::write(
            &path,
            with_kill_switch(
                &crate::codegen_ruby::shadow(&manifest),
                &manifest.crate_name,
            ),
        )?;
        return Ok(ShadowOutcome {
            path,
            exports: manifest.exports.iter().map(|f| f.name.clone()).collect(),
        });
    }
    if manifest.language == crate::lang::Lang::Go {
        let pkg = crate::codegen_go::package_name(&manifest);
        let path = match out {
            Some(p) => p.to_path_buf(),
            None => crate_dir
                .join("go")
                .join("shadow")
                .join(format!("{pkg}.go")),
        };
        if path.exists() {
            bail!("{} exists; remove it first or pass --out", path.display());
        }
        std::fs::create_dir_all(path.parent().context("output path has no folder")?)?;
        let original = crate::harness_go::import_path(&manifest)?;
        std::fs::write(
            &path,
            with_kill_switch(
                &crate::codegen_go::shadow(&manifest, &original),
                &manifest.crate_name,
            ),
        )?;
        let _ = crate::process::output(
            std::process::Command::new("gofmt").arg("-w").arg(&path),
            crate::process::QUICK,
        );
        return Ok(ShadowOutcome {
            path,
            exports: manifest.exports.iter().map(|f| f.name.clone()).collect(),
        });
    }
    let mut files: Vec<&str> = manifest
        .exports
        .iter()
        .map(|f| f.file.as_str())
        .chain(manifest.js_only.iter().map(|j| j.file.as_str()))
        .chain(manifest.classes.iter().map(|c| c.file.as_str()))
        .collect();
    files.sort_unstable();
    files.dedup();
    let stem = manifest.crate_name.trim_end_matches("-rs");
    let path = match out {
        Some(p) => p.to_path_buf(),
        None => crate_dir.join("shadow").join(format!("{stem}.shadow.ts")),
    };
    if path.exists() {
        bail!("{} exists; remove it first or pass --out", path.display());
    }
    let dir = path.parent().context("output path has no folder")?;
    std::fs::create_dir_all(dir)?;
    let dir = dir.canonical()?;

    let mut code = String::from(
        "// Generated by rust-anything: runs the TS original and the Rust port side by side.\n\
         // RUST_ANYTHING_MODE=old: original only. shadow (default): original result, Rust compared.\n\
         // new: Rust only. Report mismatches with `onMismatch`; counts are in `shadowStats`.\n\
         /* eslint-disable */\n\
         import { createRequire } from 'node:module'\n",
    );
    for (i, file) in files.iter().enumerate() {
        code.push_str(&format!(
            "import * as legacy{i} from '{}'\n",
            relative(&dir, &manifest.root.join(file))
        ));
    }
    code.push_str(&format!(
        "\ntype Mode = 'old' | 'shadow' | 'new'\n\
         export interface Mismatch {{ fn: string, args: unknown[], expected: unknown, actual: unknown }}\n\
         const mode: Mode = ((globalThis as any).process?.env?.RUST_ANYTHING_MODE ?? 'shadow') as Mode\n\
         let native: any = null\n\
         if (mode !== 'old') {{\n\
         \x20 try {{\n\
         \x20   native = createRequire(import.meta.url)('{native}')\n\
         \x20 }}\n\
         \x20 catch (error) {{\n\
         \x20   if (mode === 'new') throw error\n\
         \x20   console.warn('[rust-anything] Rust module not loaded, running the original only:', (error as Error).message)\n\
         \x20 }}\n\
         }}\n\
         export const shadowStats = {{ calls: 0, mismatches: 0 }}\n\
         let report = (m: Mismatch): void => {{\n\
         \x20 if (shadowStats.mismatches <= 10) console.warn('[rust-anything] mismatch in', m.fn, JSON.stringify(enc(m.args)), 'expected', JSON.stringify(m.expected), 'got', JSON.stringify(m.actual))\n\
         }}\n\
         /** Replace the default reporter (console.warn for the first 10 mismatches). */\n\
         export function onMismatch(handler: (m: Mismatch) => void): void {{\n\
         \x20 report = handler\n\
         }}\n\
         {codec}\n\
         function compare(name: string, args: unknown[], expected: unknown, actual: unknown): void {{\n\
         \x20 const want = {{ ok: enc(expected) }}\n\
         \x20 if (!same(want, actual)) {{\n\
         \x20   shadowStats.mismatches++\n\
         \x20   report({{ fn: name, args, expected: want, actual }})\n\
         \x20 }}\n\
         }}\n\
         function check(name: string, args: unknown[], expected: unknown, run: () => unknown): void {{\n\
         \x20 shadowStats.calls++\n\
         \x20 let result: unknown\n\
         \x20 try {{\n\
         \x20   result = run()\n\
         \x20 }}\n\
         \x20 catch (error) {{\n\
         \x20   return compare(name, args, expected, {{ error: String((error as Error)?.message ?? error) }})\n\
         \x20 }}\n\
         \x20 // An async port: compare when it settles; callers never wait for Rust.\n\
         \x20 if (result && typeof (result as any).then === 'function') {{\n\
         \x20   (result as Promise<unknown>).then(\n\
         \x20     value => compare(name, args, expected, {{ ok: enc(value) }}),\n\
         \x20     error => compare(name, args, expected, {{ error: String(error?.message ?? error) }}),\n\
         \x20   )\n\
         \x20   return\n\
         \x20 }}\n\
         \x20 compare(name, args, expected, {{ ok: enc(result) }})\n\
         }}\n\
         function call(name: string, legacy: any, args: unknown[]): any {{\n\
         \x20 if (mode === 'old' || !native) return legacy(...args)\n\
         \x20 if (mode === 'new') return native[name](...args)\n\
         \x20 // The original's result is always what callers get; it runs first and may throw.\n\
         \x20 const expected = legacy(...args)\n\
         \x20 if (expected && typeof (expected as any).then === 'function') {{\n\
         \x20   return (expected as Promise<unknown>).then((value) => {{\n\
         \x20     check(name, args, value, () => native[name](...args))\n\
         \x20     return value\n\
         \x20   }})\n\
         \x20 }}\n\
         \x20 check(name, args, expected, () => native[name](...args))\n\
         \x20 return expected\n\
         }}\n",
        native = relative(&dir, &crate_dir.join("index.cjs")),
        codec = CODEC_JS.trim().replace("const enc = (v, stack = [])", "const enc = (v: any, stack: any[] = []): any")
            .replace("const dec = (v)", "const dec = (v: any): any")
            .replace("const canon = (v, sort)", "const canon = (v: any, sort: boolean): any")
            .replace("const same = (a, b, sort = false)", "const same = (a: unknown, b: unknown, sort = false): boolean"),
    ));
    for f in &manifest.exports {
        let i = files.iter().position(|file| *file == f.file).unwrap_or(0);
        code.push_str(&format!(
            "\nexport const {name}: typeof legacy{i}.{name} = ((...args: unknown[]) => call('{name}', legacy{i}.{name}, args)) as any\n",
            name = f.name
        ));
    }
    for c in &manifest.classes {
        let i = files.iter().position(|file| *file == c.file).unwrap_or(0);
        let methods: String = c
            .methods
            .iter()
            .map(|m| format!("  {name}(...args: any[]): any {{\n    return this.#call('{name}', args)\n  }}\n", name = m.name))
            .collect();
        code.push_str(&format!(
            "\n/** `{name}`: a TS instance and a Rust instance side by side; callers get the TS results. */\n\
             export class {name} {{\n\
             \x20 #legacy: any = null\n\
             \x20 #native: any = null\n\
             \x20 constructor(...args: any[]) {{\n\
             \x20   if (mode === 'new' && native) {{\n\
             \x20     this.#native = new native.{name}(...args)\n\
             \x20     return\n\
             \x20   }}\n\
             \x20   this.#legacy = new (legacy{i} as any).{name}(...args)\n\
             \x20   if (mode === 'old' || !native) return\n\
             \x20   try {{\n\
             \x20     this.#native = new native.{name}(...args)\n\
             \x20   }}\n\
             \x20   catch (error) {{\n\
             \x20     shadowStats.mismatches++\n\
             \x20     report({{ fn: '{name} constructor', args, expected: {{ ok: true }}, actual: {{ error: String((error as Error)?.message ?? error) }} }})\n\
             \x20   }}\n\
             \x20 }}\n\
             {methods}             \x20 #call(method: string, args: unknown[]): any {{\n\
             \x20   if (!this.#legacy) return this.#native[method](...args)\n\
             \x20   if (!this.#native) return this.#legacy[method](...args)\n\
             \x20   const expected = this.#legacy[method](...args)\n\
             \x20   check(`{name}.${{method}}`, args, expected, () => this.#native[method](...args))\n\
             \x20   return expected\n\
             \x20 }}\n\
             }}\n",
            name = c.name
        ));
    }
    for j in &manifest.js_only {
        let i = files.iter().position(|file| *file == j.file).unwrap_or(0);
        code.push_str(&format!(
            "\n/** Stays in TS ({}). */\nexport const {name} = legacy{i}.{name}\n",
            j.reason,
            name = j.name
        ));
    }
    std::fs::write(&path, with_kill_switch(&code, &manifest.crate_name))
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(ShadowOutcome {
        path,
        exports: manifest
            .exports
            .iter()
            .map(|f| f.name.clone())
            .chain(manifest.classes.iter().map(|c| format!("class {}", c.name)))
            .collect(),
    })
}

/// An ES import specifier for `to`, relative to the folder `from`.
/// Makes the shadow honor the unit's kill switch, `RUST_ANYTHING_UNIT_<NAME>=original`, before
/// `RUST_ANYTHING_MODE`: the original package can send one unit back on its own.
fn with_kill_switch(text: &str, crate_name: &str) -> String {
    let switch = crate::front_door::kill_switch(crate_name);
    text.replacen(
        "((globalThis as any).process?.env?.RUST_ANYTHING_MODE ?? 'shadow')",
        &format!(
            "((globalThis as any).process?.env?.{switch} === 'original' ? 'old' : ((globalThis as any).process?.env?.RUST_ANYTHING_MODE ?? 'shadow'))"
        ),
        1,
    )
    .replacen(
        "local MODE = os.getenv('RUST_ANYTHING_MODE') or 'shadow'",
        &format!(
            "local MODE = (os.getenv('{switch}') == 'original') and 'old' or (os.getenv('RUST_ANYTHING_MODE') or 'shadow')"
        ),
        1,
    )
    .replacen(
        "_MODE = os.environ.get(\"RUST_ANYTHING_MODE\", \"shadow\")",
        &format!(
            "_MODE = \"old\" if os.environ.get(\"{switch}\") == \"original\" else os.environ.get(\"RUST_ANYTHING_MODE\", \"shadow\")"
        ),
        1,
    )
    .replacen(
        "MODE = ENV.fetch(\"RUST_ANYTHING_MODE\", \"shadow\")",
        &format!(
            "MODE = ENV[\"{switch}\"] == \"original\" ? \"old\" : ENV.fetch(\"RUST_ANYTHING_MODE\", \"shadow\")"
        ),
        1,
    )
    .replacen(
        "var mode = func() string {\n",
        &format!(
            "var mode = func() string {{\n\tif os.Getenv(\"{switch}\") == \"original\" {{\n\t\treturn \"old\"\n\t}}\n"
        ),
        1,
    )
    .replace(
        "System.getProperty(\"rust-anything.mode\", \"shadow\")",
        &format!(
            "(\"original\".equals(System.getenv(\"{switch}\")) ? \"old\" : System.getProperty(\"rust-anything.mode\", \"shadow\"))"
        ),
    )
}

fn relative(from: &Path, to: &Path) -> String {
    let from: Vec<Component> = from.components().collect();
    let to_parts: Vec<Component> = to.components().collect();
    let common = from
        .iter()
        .zip(&to_parts)
        .take_while(|(a, b)| a == b)
        .count();
    let mut parts: Vec<String> =
        std::iter::repeat_n("..".to_string(), from.len() - common).collect();
    parts.extend(
        to_parts[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    let joined = parts.join("/");
    if joined.starts_with("..") {
        joined
    } else {
        format!("./{joined}")
    }
}

/// The Python shadow module: same functions and classes, original first, Rust compared.
fn shadow_python(
    crate_dir: &Path,
    manifest: &Manifest,
    out: Option<&Path>,
) -> Result<ShadowOutcome> {
    let stem = manifest
        .crate_name
        .trim_end_matches("-rs")
        .replace('-', "_");
    let path = match out {
        Some(p) => p.to_path_buf(),
        None => crate_dir.join("shadow").join(format!("{stem}_shadow.py")),
    };
    if path.exists() {
        bail!("{} exists; remove it first or pass --out", path.display());
    }
    std::fs::create_dir_all(path.parent().context("output path has no folder")?)?;
    let mut files: Vec<&str> = manifest
        .exports
        .iter()
        .map(|f| f.file.as_str())
        .chain(manifest.classes.iter().map(|c| c.file.as_str()))
        .chain(manifest.js_only.iter().map(|j| j.file.as_str()))
        .collect();
    files.sort_unstable();
    files.dedup();
    let module_of = |file: &str| files.iter().position(|f| *f == file).unwrap_or(0);
    let sources = format!(
        "[{}]",
        files
            .iter()
            .map(|f| format!("{:?}", manifest.root.join(f).to_string_lossy()))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let mut code = format!(
        "\"\"\"Generated by rust-anything: runs the Python original and the Rust port side by side.\n\n\
         RUST_ANYTHING_MODE=old: original only. shadow (default): original result, Rust compared.\n\
         new: Rust only. Report mismatches with `on_mismatch`; counts are in `stats`.\n\"\"\"\n\
         import importlib\nimport json\nimport math\nimport os\nimport sys\n\n\
         _MODE = os.environ.get(\"RUST_ANYTHING_MODE\", \"shadow\")\n\
         _SOURCES = {sources}\n\
         _PACKAGE_DIR = {package_dir:?}\n{driver_helpers}\n\
         _legacy = [load(p) for p in _SOURCES]\n\
         _native = None\n\
         if _MODE != \"old\":\n    try:\n        sys.path.insert(0, _PACKAGE_DIR)\n        _native = importlib.import_module({lib:?})\n\
         \x20   except ImportError as error:\n        if _MODE == \"new\":\n            raise\n\
         \x20       print(\"[rust-anything] Rust module not loaded, running the original only:\", error, file=sys.stderr)\n\n\
         stats = {{\"calls\": 0, \"mismatches\": 0}}\n\n\n\
         def _default_report(m):\n    if stats[\"mismatches\"] <= 10:\n        print(\"[rust-anything] mismatch in\", m[\"fn\"], m, file=sys.stderr)\n\n\n\
         _report = _default_report\n\n\n\
         def on_mismatch(handler):\n    \"\"\"Replace the default reporter (stderr for the first 10 mismatches).\"\"\"\n    global _report\n    _report = handler\n\n\n\
         def _check(name, args, expected, run):\n    stats[\"calls\"] += 1\n    try:\n        actual = {{\"ok\": enc(run())}}\n\
         \x20   except (KeyboardInterrupt, SystemExit):\n        raise\n    except BaseException as e:  # noqa: BLE001\n        actual = {{\"error\": error_text(e)}}\n\
         \x20   want = {{\"ok\": enc(expected)}}\n    if canon(want, False) != canon(actual, False):\n\
         \x20       stats[\"mismatches\"] += 1\n        _report({{\"fn\": name, \"args\": args, \"expected\": want, \"actual\": actual}})\n\n\n\
         def _call(module, name, args, kwargs):\n    legacy = getattr(_legacy[module], name)\n    if _native is None:\n        return legacy(*args, **kwargs)\n\
         \x20   if _MODE == \"new\":\n        return getattr(_native, name)(*args, **kwargs)\n    expected = legacy(*args, **kwargs)\n\
         \x20   _check(name, list(args), expected, lambda: getattr(_native, name)(*args, **kwargs))\n    return expected\n",
        sources = sources,
        package_dir = crate_dir.join("python").to_string_lossy(),
        lib = manifest.lib_name(),
        driver_helpers = PY_HELPERS,
    );
    for f in &manifest.exports {
        code.push_str(&format!(
            "\n\ndef {name}(*args, **kwargs):\n    return _call({m}, {name:?}, args, kwargs)\n",
            name = f.name,
            m = module_of(&f.file)
        ));
    }
    for c in &manifest.classes {
        let methods: String = c
            .methods
            .iter()
            .map(|m| {
                format!(
                    "\n    def {name}(self, *args, **kwargs):\n        return self._call({name:?}, args, kwargs)\n",
                    name = m.name
                )
            })
            .collect();
        code.push_str(&format!(
            "\n\nclass {name}:\n    \"\"\"A Python instance and a Rust instance side by side; callers get the Python results.\"\"\"\n\n\
             \x20   def __init__(self, *args, **kwargs):\n        self._legacy = None\n        self._native = None\n\
             \x20       if _MODE == \"new\" and _native is not None:\n            self._native = _native.{name}(*args, **kwargs)\n            return\n\
             \x20       self._legacy = getattr(_legacy[{m}], {name:?})(*args, **kwargs)\n        if _native is None or _MODE == \"old\":\n            return\n\
             \x20       try:\n            self._native = _native.{name}(*args, **kwargs)\n        except (KeyboardInterrupt, SystemExit):\n            raise\n        except BaseException as e:  # noqa: BLE001\n\
             \x20           stats[\"mismatches\"] += 1\n            _report({{\"fn\": \"{name} constructor\", \"args\": list(args), \"expected\": {{\"ok\": True}}, \"actual\": {{\"error\": error_text(e)}}}})\n\n\
             \x20   def _call(self, method, args, kwargs):\n        if self._legacy is None:\n            return getattr(self._native, method)(*args, **kwargs)\n\
             \x20       if self._native is None:\n            return getattr(self._legacy, method)(*args, **kwargs)\n\
             \x20       expected = getattr(self._legacy, method)(*args, **kwargs)\n\
             \x20       _check(f\"{name}.{{method}}\", list(args), expected, lambda: getattr(self._native, method)(*args, **kwargs))\n        return expected\n{methods}",
            name = c.name,
            m = module_of(&c.file)
        ));
    }
    for j in &manifest.js_only {
        code.push_str(&format!(
            "\n\n# Stays in Python ({}).\n{name} = _legacy[{m}].{name}\n",
            j.reason,
            name = j.name,
            m = module_of(&j.file)
        ));
    }
    std::fs::write(&path, with_kill_switch(&code, &manifest.crate_name))
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(ShadowOutcome {
        path,
        exports: manifest
            .exports
            .iter()
            .map(|f| f.name.clone())
            .chain(manifest.classes.iter().map(|c| format!("class {}", c.name)))
            .collect(),
    })
}

/// `enc`, `canon`, `load` and `error_text` from the Python driver, for the shadow module.
const PY_HELPERS: &str = {
    // Kept in one place: the driver defines them first, before its entry points.
    r#"
BIG = 2 ** 53


def enc(v, stack=()):
    if v is None or isinstance(v, (bool, str)):
        return v
    if isinstance(v, int):
        return v if -BIG <= v <= BIG else {"$int": str(v)}
    if isinstance(v, float):
        if math.isnan(v):
            return {"$num": "NaN"}
        if math.isinf(v):
            return {"$num": "Infinity" if v > 0 else "-Infinity"}
        if v == 0 and math.copysign(1, v) < 0:
            return {"$num": "-0"}
        return {"$float": repr(v)} if v.is_integer() else v
    if any(v is s for s in stack):
        return {"$cycle": True}
    stack = stack + (v,)
    if isinstance(v, list):
        return [enc(x, stack) for x in v]
    if isinstance(v, tuple):
        return {"$tuple": [enc(x, stack) for x in v]}
    if isinstance(v, (set, frozenset)):
        return {"$set": sorted((enc(x, stack) for x in v), key=json.dumps)}
    if isinstance(v, (bytes, bytearray)):
        return {"$bytes": list(v)}
    if isinstance(v, dict):
        if all(isinstance(k, str) for k in v):
            return {k: enc(x, stack) for k, x in v.items()}
        return {"$dict": [[enc(k, stack), enc(x, stack)] for k, x in v.items()]}
    if hasattr(v, "__dict__"):
        return {"$object": type(v).__name__, "attrs": enc(vars(v), stack)}
    return {"$repr": type(v).__name__}


def canon(v, sort):
    return json.dumps(v, sort_keys=sort, ensure_ascii=False)


def error_text(e):
    return f"{type(e).__name__}: {e}"


def load(path):
    path = os.path.abspath(path)
    directory, filename = os.path.split(path)
    parts = [os.path.splitext(filename)[0]]
    while os.path.exists(os.path.join(directory, "__init__.py")):
        directory, package = os.path.split(directory)
        parts.insert(0, package)
    if parts[-1] == "__init__":
        parts.pop()
    if directory not in sys.path:
        sys.path.insert(0, directory)
    return importlib.import_module(".".join(parts))
"#
};

/// Quote a string for a Lua source literal. Paths are forward-slashed by the caller; escape
/// the characters Lua would otherwise interpret inside a double-quoted string.
fn lua_str(s: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\{:03}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The Lua/LÖVE shadow module: a drop-in `M` whose functions run the original and the Rust port
/// side by side. The original is located with the same upvalue/table resolution the record/diff
/// driver uses (see `codegen_lua.rs::DRIVER_LUA`), the port is the generated FFI wrapper
/// (`codegen_lua::wrapper`). `RUST_ANYTHING_MODE=old|shadow|new` selects which result callers get;
/// `shadowStats.mismatches` and `onMismatch` expose the comparison.
fn shadow_lua(crate_dir: &Path, manifest: &Manifest, out: Option<&Path>) -> Result<ShadowOutcome> {
    let lib = manifest.lib_name();
    let path = match out {
        Some(p) => p.to_path_buf(),
        None => crate_dir.join("lua").join(format!("{lib}_shadow.lua")),
    };
    if path.exists() {
        bail!("{} exists; remove it first or pass --out", path.display());
    }
    std::fs::create_dir_all(path.parent().context("output path has no folder")?)?;
    let source = lua_shadow_source(crate_dir, manifest);
    std::fs::write(&path, with_kill_switch(&source, &manifest.crate_name))
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(ShadowOutcome {
        path,
        exports: manifest.exports.iter().map(|f| f.name.clone()).collect(),
    })
}

/// Build the generated Lua shadow source for one crate.
fn lua_shadow_source(crate_dir: &Path, m: &Manifest) -> String {
    use crate::lang::lua::receiver;

    let lib = lua_str(&m.lib_name());
    let root = lua_str(&m.root.to_string_lossy().replace('\\', "/"));
    let cases = lua_str(
        &crate_dir
            .join(crate::harness::CASES)
            .to_string_lossy()
            .replace('\\', "/"),
    );

    let mut files: Vec<&str> = m
        .exports
        .iter()
        .map(|f| f.file.as_str())
        .chain(m.js_only.iter().map(|j| j.file.as_str()))
        .collect();
    files.sort_unstable();
    files.dedup();
    let files_lua = files
        .iter()
        .map(|f| lua_str(&m.root.join(f).to_string_lossy().replace('\\', "/")))
        .collect::<Vec<_>>()
        .join(", ");

    let mut targets = String::new();
    let mut natives = String::new();
    for f in &m.exports {
        let (recv, name) = receiver(f);
        targets.push_str(&format!(
            "  [{}] = {{ {}, {}, {} }},\n",
            lua_str(&f.name),
            lua_str(&recv),
            lua_str(&name),
            lua_str(&f.file)
        ));
        // The FFI wrapper nests exports as M.<receiver>.<name> (or M.<name> for globals).
        let mut parts: Vec<String> = if recv == "_G" {
            Vec::new()
        } else {
            recv.split('.')
                .filter(|p| !p.is_empty())
                .map(str::to_string)
                .collect()
        };
        parts.push(name);
        natives.push_str(&format!(
            "  [{}] = {{ {} }},\n",
            lua_str(&f.name),
            parts
                .iter()
                .map(|p| lua_str(p))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    let mut out = String::new();
    out.push_str(&format!(
        r#"-- Generated by rust-anything: runs the original Lua function and the Rust port side by side.
-- RUST_ANYTHING_MODE=old: original only. shadow (default): original result, Rust compared.
-- new: Rust only. Report mismatches with `onMismatch`; counts are in `shadowStats`.
local codec = require('rust_anything_codec')
pcall(require, 'rust_anything_headless_stub')
if type(_G.love) ~= 'table' then _G.love = {{}} end
local unpack_ = table.unpack or unpack

local MODE = os.getenv('RUST_ANYTHING_MODE') or 'shadow'
local source_root = {root}
local cases_path = {cases}
local alias_cache = {{}}

local native = nil
if MODE ~= 'old' then
  local ok, mod = pcall(require, {lib})
  if ok then
    native = mod
  elseif MODE == 'new' then
    error(mod, 0)
  else
    print('[rust-anything] Rust module not loaded, running the original only: ' .. tostring(mod))
  end
end

local modules = {{}}
local function load_files(...)
  for _, path in ipairs({{...}}) do
    local chunk = loadfile(path)
    if chunk then
      local ok, mod = pcall(chunk)
      if ok and type(mod) == 'table' then modules[path] = mod end
    end
  end
end
load_files({files_lua})

local targets = {{
{targets}}}
local native_paths = {{
{natives}}}
"#,
        root = root,
        cases = cases,
        lib = lib,
        files_lua = files_lua,
        targets = targets,
        natives = natives,
    ));
    out.push_str(LUA_SHADOW_HELPERS);
    out.push_str(LUA_SHADOW_RUNTIME);

    out.push_str("local M = {}\n");
    for f in &m.exports {
        let (recv, _) = receiver(f);
        let parts: Vec<&str> = recv.split('.').filter(|p| !p.is_empty()).collect();
        let mut prefix = String::new();
        for part in &parts {
            prefix = if prefix.is_empty() {
                (*part).to_string()
            } else {
                format!("{prefix}.{part}")
            };
            out.push_str(&format!("M.{prefix} = M.{prefix} or {{}}\n"));
        }
        let full = if parts.is_empty() {
            receiver(f).1
        } else {
            format!("{}.{}", parts.join("."), receiver(f).1)
        };
        out.push_str(&format!(
            "function M.{full}(...) return call({}, {{...}}) end\n",
            lua_str(&f.name)
        ));
    }
    out.push_str(
        r#"
M.shadowStats = shadowStats
M.onMismatch = onMismatch
M.runCases = runCases

-- Running the file directly (rather than `require`-ing it) replays the recorded cases and
-- prints the mismatch count; `require` returns the drop-in module without running anything.
local first = ...
if type(first) ~= 'string' then
  local f = io.open(cases_path)
  if f then
    f:close()
    local stats = runCases(cases_path)
    print(string.format('[rust-anything] shadow: %d calls, %d mismatches (mode=%s)', stats.calls, stats.mismatches, MODE))
  end
end

return M
"#,
    );
    out
}

/// Resolution and canonical-comparison helpers, adapted from `codegen_lua.rs::DRIVER_LUA` so the
/// shadow reaches exactly the closures `record`/`diff` reach (nested tables and function
/// upvalues), and compares results the same way.
const LUA_SHADOW_HELPERS: &str = r#"
local function split_path(s)
  local out = {}
  for part in tostring(s or ''):gmatch('[^%.]+') do out[#out + 1] = part end
  return out
end

local function path_get(root, path)
  local cur = root
  for i = 1, #path do
    if type(cur) ~= 'table' then return nil end
    cur = cur[path[i]]
  end
  return cur
end

local function deepcopy(v)
  if type(v) ~= 'table' then return v end
  local out = {}
  for k, value in pairs(v) do out[deepcopy(k)] = deepcopy(value) end
  return out
end

local function find_named(tbl, path, depth, seen)
  if type(tbl) ~= 'table' or depth > 12 or seen[tbl] then return nil end
  seen[tbl] = true
  local direct = path_get(tbl, path)
  if type(direct) == 'function' then return direct end
  for _, value in pairs(tbl) do
    if type(value) == 'table' then
      local found = find_named(value, path, depth + 1, seen)
      if found then return found end
    end
  end
  return nil
end

local function find_upvalue(root, path, depth, seen)
  if depth > 12 or seen[root] then return nil end
  seen[root] = true
  if type(root) == 'function' then
    local i = 1
    while true do
      local upname, value = debug.getupvalue(root, i)
      if upname == nil then break end
      local direct = path_get(value, path)
      if type(direct) == 'function' then return direct end
      local found = find_upvalue(value, path, depth + 1, seen)
      if found then return found end
      i = i + 1
    end
  elseif type(root) == 'table' then
    for _, value in pairs(root) do
      local found = find_upvalue(value, path, depth + 1, seen)
      if found then return found end
    end
  end
  return nil
end

local function source_aliases(file)
  if not file or not source_root then return nil end
  if alias_cache[file] ~= nil then return alias_cache[file] end
  local map = {}
  local f = io.open(source_root .. '/' .. file)
  if f then
    local text = f:read('*a')
    f:close()
    for alias, target in text:gmatch('local%s+([%w_]+)%s*=%s*([%w_]+)[^\n]*') do
      if alias ~= target then map[alias] = target end
    end
    local ret
    for block in text:gmatch('return%s*{([^}]*)}') do ret = block end
    if ret then
      for key, alias in ret:gmatch('([%w_]+)%s*=%s*([%w_]+)') do
        local cur = alias
        for _ = 1, 8 do
          local parent = map[cur]
          if not parent or parent == cur then break end
          cur = parent
        end
        if map[cur] == nil then map[cur] = key end
      end
    end
  end
  alias_cache[file] = map
  return map
end

local function aliased_recv(file, recv)
  local map = source_aliases(file)
  if not map then return nil end
  local cur = recv
  for _ = 1, 8 do
    local parent = map[cur]
    if not parent or parent == cur then break end
    cur = parent
  end
  return cur ~= recv and cur or nil
end

local function resolve(t)
  local recv, name, file = t[1], t[2], t[3]
  local recv_path, name_path = split_path(recv), split_path(name)

  local holders = {}
  local function add(h)
    if type(h) == 'table' then holders[#holders + 1] = h end
  end
  if file then
    add(modules[file])
    if #holders == 0 then
      for path, mod in pairs(modules) do
        if path:sub(-#file) == file then add(mod) break end
      end
    end
  end
  for _, mod in pairs(modules) do add(mod) end
  if recv == '_G' then
    add(_G)
  elseif recv == 'love' then
    add(_G.love)
  end

  for _, mod in pairs(modules) do
    local fn = find_upvalue(mod, name_path, 0, {})
    if fn then return fn end
  end

  if recv == '_G' then
    local global = path_get(_G, name_path)
    if type(global) == 'function' then return global end
    for _, holder in ipairs(holders) do
      local fn = path_get(holder, name_path)
      if type(fn) == 'function' then return fn end
    end
    return nil
  end

  local alias = aliased_recv(file, recv)
  local recv_paths = { recv_path }
  if alias then recv_paths[#recv_paths + 1] = split_path(alias) end
  for _, holder in ipairs(holders) do
    for _, rp in ipairs(recv_paths) do
      local sub = path_get(holder, rp)
      if type(sub) == 'table' then
        local fn = path_get(sub, name_path)
        if type(fn) == 'function' then return fn end
      end
    end
    local fn = path_get(holder, name_path)
    if type(fn) == 'function' then return fn end
  end

  for _, holder in ipairs(holders) do
    local fn = find_named(holder, name_path, 0, {})
    if fn then return fn end
  end
  return nil
end

local function canonical(v, level)
  if type(v) ~= 'table' then return codec.encode(v) end
  local keys = {}
  for k in pairs(v) do keys[#keys + 1] = k end
  table.sort(keys, function(a, b)
    if type(a) == 'number' and type(b) == 'number' then return a < b end
    return tostring(a) < tostring(b)
  end)
  local parts = {}
  for _, k in ipairs(keys) do
    parts[#parts + 1] = codec.encode(tostring(k)) .. ':' .. canonical(v[k], (level or 0) + 1)
  end
  return '{' .. table.concat(parts, ',') .. '}'
end

local function normerr(s)
  return (tostring(s or ''):gsub('\\', '/'))
end

local function same(expected, got)
  if tostring(got.error or ''):find('Rust panic:', 1, true) then return false end
  if expected.error ~= nil then
    return got.error ~= nil and normerr(expected.error) == normerr(got.error)
  end
  return got.error == nil and canonical(expected.ok, 0) == canonical(got.ok, 0)
end
"#;

/// The shadow dispatcher and stats API, assembled after the helpers and before the exports.
const LUA_SHADOW_RUNTIME: &str = r#"
local shadowStats = { calls = 0, mismatches = 0 }
local report = function(m)
  if shadowStats.mismatches <= 10 then
    print('[rust-anything] mismatch in ' .. tostring(m.fn) ..
      ' expected ' .. tostring(codec.encode(m.expected)) ..
      ' got ' .. tostring(codec.encode(m.actual)))
  end
end
local function onMismatch(handler) report = handler end

local function path_get_native(path)
  local cur = native
  for i = 1, #path do
    if type(cur) ~= 'table' then return nil end
    cur = cur[path[i]]
  end
  return cur
end

local function invoke_legacy(name, args)
  local fn = resolve(targets[name] or {})
  if type(fn) ~= 'function' then error('no such original: ' .. tostring(name), 0) end
  return fn(unpack_(args, 1, #args))
end

local function invoke_native(name, args)
  local path = native_paths[name]
  local fn = path and path_get_native(path)
  if type(fn) ~= 'function' then error('no such native: ' .. tostring(name), 0) end
  return fn(unpack_(args, 1, #args))
end

local function compare(name, args, expected, actual)
  shadowStats.calls = shadowStats.calls + 1
  if not same(expected, actual) then
    shadowStats.mismatches = shadowStats.mismatches + 1
    report({ fn = name, args = args, expected = expected, actual = actual })
  end
end

local function call(name, args)
  if MODE == 'new' then return invoke_native(name, args) end
  if MODE == 'old' or not native then return invoke_legacy(name, args) end
  -- The original gets the caller's real table (so in-place mutations still reach the caller);
  -- the port gets a pristine snapshot. Passing the same table to both would let a stateful
  -- original (e.g. SbPityForce) mutate the port's inputs and manufacture a mismatch.
  local native_args = deepcopy(args)
  local lok, expected = pcall(invoke_legacy, name, args)
  local nok, actual = pcall(invoke_native, name, native_args)
  local exp = lok and { ok = expected } or { error = tostring(expected) }
  local act = nok and { ok = actual } or { error = tostring(actual) }
  compare(name, args, exp, act)
  if not lok then error(expected, 0) end
  return expected
end

local function runCases(path)
  path = path or cases_path
  local f = io.open(path)
  if not f then error('no cases at ' .. tostring(path), 0) end
  local text = f:read('*a')
  f:close()
  for line in text:gmatch('[^\n]+') do
    if line ~= '' then
      local c = codec.decode(line)
      local args = {}
      for i, v in ipairs(c.args or {}) do args[i] = codec.normalize(v) end
      pcall(call, c.fn, args)
    end
  end
  return shadowStats
end
"#;

/// C: a shadow translation unit, and the `-D` flags that rename the originals beside it.
fn shadow_c(crate_dir: &Path, manifest: &Manifest, out: Option<&Path>) -> Result<ShadowOutcome> {
    let stem = manifest
        .crate_name
        .trim_end_matches("-rs")
        .replace('-', "_");
    let path = match out {
        Some(p) => p.to_path_buf(),
        None => crate_dir.join("shadow").join(format!("{stem}_shadow.c")),
    };
    if path.exists() {
        bail!("{} exists; remove it first or pass --out", path.display());
    }
    let dir = path.parent().context("output path has no folder")?;
    std::fs::create_dir_all(dir)?;
    std::fs::write(&path, crate::codegen_c::shadow_c(manifest))?;
    std::fs::write(
        dir.join("flags.txt"),
        crate::codegen_c::shadow_flags(manifest),
    )?;
    Ok(ShadowOutcome {
        path,
        exports: manifest.exports.iter().map(|f| f.name.clone()).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::{lua_shadow_source, lua_str, relative, with_kill_switch};
    use crate::bridge::Manifest;
    use std::path::Path;

    fn lua_manifest() -> Manifest {
        serde_json::from_str(
            r#"{"schema":"rust-anything/bridge@1","root":".","unit":"x.lua","language":"lua",
                 "kind":"ffi","crate_name":"x-rs","exports":[
                   {"name":"love.conf","file":"conf.lua","line":1,"params":[],"returns":"unknown","is_async":false},
                   {"name":"api.run","file":"src/x.lua","line":2,"params":[],"returns":"unknown","is_async":false}]}"#,
        )
        .unwrap()
    }

    #[test]
    fn lua_shadow_wraps_original_and_native_with_stats() {
        let src = lua_shadow_source(Path::new("crate"), &lua_manifest());
        for piece in [
            "local codec = require('rust_anything_codec')",
            "pcall(require, 'rust_anything_headless_stub')",
            "local MODE = os.getenv('RUST_ANYTHING_MODE') or 'shadow'",
            "RUST_ANYTHING_MODE=old: original only",
            "find_upvalue",
            "local function resolve(t)",
            "local function canonical(v, level)",
            "local function deepcopy(v)",
            "local native_args = deepcopy(args)",
            "local function invoke_legacy(name, args)",
            "local function invoke_native(name, args)",
            "shadowStats.mismatches = shadowStats.mismatches + 1",
            "M.shadowStats = shadowStats",
            "M.onMismatch = onMismatch",
            "M.runCases = runCases",
            "function M.love.conf(...) return call(\"love.conf\", {...}) end",
            "function M.api.run(...) return call(\"api.run\", {...}) end",
            "[\"love.conf\"] = { \"love\", \"conf\", \"conf.lua\" }",
            "[\"api.run\"] = { \"api\", \"run\" }",
            "return M",
        ] {
            assert!(src.contains(piece), "shadow is missing {piece}");
        }
    }

    #[test]
    fn lua_shadow_honors_the_unit_kill_switch() {
        let src = lua_shadow_source(Path::new("crate"), &lua_manifest());
        let out = with_kill_switch(&src, "x-rs");
        assert!(out.contains("os.getenv('RUST_ANYTHING_UNIT_X') == 'original'"));
        assert!(!out.contains("local MODE = os.getenv('RUST_ANYTHING_MODE') or 'shadow'"));
    }

    #[test]
    fn lua_str_escapes_sources() {
        assert_eq!(lua_str("a\\b\"c"), "\"a\\\\b\\\"c\"");
        assert_eq!(lua_str("a\nb"), "\"a\\nb\"");
    }

    #[test]
    fn builds_relative_specifiers() {
        assert_eq!(
            relative(Path::new("/r/rust/x/shadow"), Path::new("/r/src/a.ts")),
            "../../../src/a.ts"
        );
        assert_eq!(
            relative(Path::new("/r/rust/x"), Path::new("/r/rust/x/index.cjs")),
            "./index.cjs"
        );
    }
}
