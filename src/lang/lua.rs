//! Lua and LÖVE: discovery, `require` resolution, risks and exported functions.
//!
//! Lua is dynamic, so types are recorded at runtime by the harness rather than read from
//! annotations. LÖVE projects are recognised by `conf.lua` + `main.lua` and their `love.*`
//! callbacks are exported as units, so a game loop can be ported one callback at a time.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use super::syntax::loc;
use crate::risks::Risk;
use crate::ts::deps::CrateMatch;
use crate::ts::discover::{FileKind, Package, SourceFile, Workspace};
use crate::ts::exports::{ExportedFn, Param};
use crate::ts::scan::{FileFacts, Import};

const SKIP_DIRS: &[&str] = &[
    "target",
    ".git",
    "node_modules",
    "vendor",
    ".love",
    "build",
    "out",
];

fn join(dir: &str, rest: &str) -> String {
    if dir.is_empty() {
        rest.to_string()
    } else {
        format!("{dir}/{rest}")
    }
}

/// A directory containing both `conf.lua` and `main.lua` is a LÖVE game root.
fn is_love_project(root: &Path) -> bool {
    root.join("conf.lua").exists() && root.join("main.lua").exists()
}

pub fn discover(root: &Path) -> Result<Workspace> {
    let mut files = Vec::new();
    let walker = crate::walk::filtered(root, |e| {
        !(e.file_type().is_some_and(|t| t.is_dir())
            && SKIP_DIRS.contains(&e.file_name().to_string_lossy().as_ref()))
    })
    .hidden(true)
    .require_git(false)
    .build();

    for entry in walker.flatten() {
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let Ok(rel) = entry.path().strip_prefix(root) else {
            continue;
        };
        let rel = rel.to_string_lossy().replace('\\', "/");
        if !rel.ends_with(".lua") {
            continue;
        }
        let is_test = rel.contains("/test") || rel.contains("/spec") || rel.starts_with("test_");
        files.push((
            rel,
            if is_test {
                FileKind::Test
            } else {
                FileKind::Source
            },
        ));
    }

    let love = is_love_project(root);
    let name = root
        .file_name()
        .map_or_else(|| "root".into(), |n| n.to_string_lossy().into_owned());
    let packages = vec![Package {
        name: if love { format!("{name}-love") } else { name },
        src_root: String::new(),
        dir: String::new(),
        deps: if love {
            vec!["love".to_string()]
        } else {
            Vec::new()
        },
        dev_deps: Vec::new(),
    }];

    files.sort_by(|a, b| a.0.cmp(&b.0));
    let files = files
        .into_iter()
        .map(|(path, kind)| SourceFile {
            path,
            package: 0,
            kind,
        })
        .collect();
    Ok(Workspace { packages, files })
}

/// Strip a Lua string literal's quotes.
fn unquote(s: &str) -> &str {
    let s = s.trim();
    if s.len() >= 2
        && ((s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')))
    {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// Normalise a `require` spec: dotted module names become path separators, while a
/// leading `./` or `../` is preserved so `resolve` can anchor it at the importer.
fn normalize_spec(spec: &str) -> String {
    let (prefix, rest) = if let Some(r) = spec.strip_prefix("../") {
        ("../", r)
    } else if let Some(r) = spec.strip_prefix("./") {
        ("./", r)
    } else {
        ("", spec)
    };
    let tail = if rest.contains('.') {
        rest.replace('.', "/")
    } else {
        rest.to_string()
    };
    format!("{prefix}{tail}")
}

/// Every `require("...")` literal on a line, normalised by [`normalize_spec`].
///
/// Real game files put several requires on one line (`local a = require("x"); local b =
/// require("y")`), so this returns one entry per call instead of only the first. A bare
/// `require` identifier (a user-defined shim) or a non-literal argument yields nothing.
fn require_specs(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(i) = rest.find("require") {
        let after_ident = &rest[i + "require".len()..];
        let mut after = after_ident.trim_start();
        let Some(paren) = after.strip_prefix('(') else {
            rest = after_ident;
            continue;
        };
        after = paren.trim_start();
        let Some(q) = after.chars().next().filter(|c| *c == '"' || *c == '\'') else {
            rest = after_ident;
            continue;
        };
        let Some(end) = after[1..].find(q) else {
            rest = after_ident;
            continue;
        };
        out.push(normalize_spec(unquote(&after[..end + 2])));
        rest = &after[end + 2..];
    }
    out
}

pub fn scan(source: &str) -> FileFacts {
    let mut facts = FileFacts {
        loc: loc(source),
        ..FileFacts::default()
    };
    let add = |risk: Risk, line: u32, facts: &mut FileFacts| {
        facts.risks.entry(risk).or_default().push(line);
    };

    for (n, line) in source.lines().enumerate() {
        let line_no = n as u32 + 1;
        let trimmed = line.trim();

        for spec in require_specs(line) {
            facts.imports.push(Import {
                specifier: spec,
                type_only: false,
                dynamic: false,
            });
        }

        if trimmed.starts_with("local function ") || trimmed.contains("function ") {
            // Lua has no annotations: every function's types are learned by recording.
            add(Risk::DynamicTyping, line_no, &mut facts);
        }
        if trimmed.starts_with("for ")
            || trimmed.starts_with("while ")
            || trimmed.contains(" ipairs(")
        {
            facts.loops += 1;
        }
        if trimmed.contains("pairs(") {
            add(Risk::UnorderedIteration, line_no, &mut facts);
        }
        // `love.load()` / `M.load()` are function definitions/method calls, not Lua's
        // dynamic-code `load()`. A function *named* `load` is a declaration too.
        let load_declaration = trimmed
            .strip_prefix("local function ")
            .or_else(|| trimmed.strip_prefix("function "))
            .and_then(|r| r.split_once('('))
            .is_some_and(|(head, _)| head.trim() == "load");
        let dynamic_load =
            trimmed.contains("load(") && !trimmed.contains(".load(") && !load_declaration;
        if trimmed.contains("loadstring(") || dynamic_load || trimmed.contains("dofile(") {
            add(Risk::DynamicCode, line_no, &mut facts);
        }
        if trimmed.contains("require(\"ffi\")")
            || trimmed.contains("ffi.")
            || trimmed.contains("ffi.C")
        {
            add(Risk::PlatformSpecific, line_no, &mut facts);
            add(Risk::Pointers, line_no, &mut facts);
        }
        if trimmed.contains("love.thread") || trimmed.contains("love.timer.sleep") {
            add(Risk::Concurrency, line_no, &mut facts);
        }
        if trimmed.contains("jit.off") || trimmed.contains("jit.on") {
            add(Risk::PlatformSpecific, line_no, &mut facts);
        }
        if trimmed.contains("love.graphics")
            || trimmed.contains("love.window")
            || trimmed.contains("love.audio")
        {
            facts.platforms.insert(crate::ts::scan::Platform::Ui);
        }
        if trimmed.contains("love.filesystem") || trimmed.contains("os.getenv") {
            facts.platforms.insert(crate::ts::scan::Platform::Node);
        }
        if trimmed.contains("error(") || trimmed.contains("pcall(") || trimmed.contains("assert(") {
            add(Risk::Exceptions, line_no, &mut facts);
        }
        if trimmed.starts_with("function love.") {
            // LÖVE callbacks are the whole-app entry points.
            add(Risk::Callbacks, line_no, &mut facts);
        }
        if trimmed.contains("math.random") || trimmed.contains("bit.") {
            add(Risk::NumberSemantics, line_no, &mut facts);
        }
        if trimmed.contains("json") || trimmed.contains("love.data.encode") {
            add(Risk::Json, line_no, &mut facts);
        }
        // Reassigning a global or a love module method changes it for every caller.
        if trimmed.starts_with("function love.") && trimmed.contains("=") {
            add(Risk::Monkeypatching, line_no, &mut facts);
        }
    }
    facts
}

/// Repository-relative base paths (no extension) a Lua `require` spec can refer to.
///
/// - `require("src.data.constants")` -> `src/data/constants`
/// - `require("./helper")` in `src/game/luck.lua` -> `src/game/helper`
/// - `require("../save")` in `src/game/luck.lua` -> `src/save`
fn module_base(from: &str, spec: &str) -> Vec<String> {
    let mut dir = from
        .rsplit_once('/')
        .map_or(String::new(), |(d, _)| d.to_string());
    let mut rest = spec;
    let mut ups = 0usize;
    loop {
        if let Some(r) = rest.strip_prefix("../") {
            ups += 1;
            rest = r;
        } else if let Some(r) = rest.strip_prefix("./") {
            rest = r;
        } else {
            break;
        }
    }
    for _ in 0..ups {
        dir = dir
            .rsplit_once('/')
            .map_or(String::new(), |(parent, _)| parent.to_string());
    }
    let tail = rest.replace('.', "/");
    let base = join(&dir, &tail);
    let mut out = vec![base.clone()];
    // Keep the literal spec as a fallback for files whose name really contains a dot.
    if spec.contains('.') && !spec.starts_with('.') && spec != base {
        out.push(spec.to_string());
    }
    out
}

pub fn resolve(
    from: &str,
    spec: &str,
    index: &HashMap<&str, usize>,
    _ws: &Workspace,
) -> super::Resolution {
    // LÖVE/LuaJIT built-ins are platform modules. Check the original spec first so a
    // dotted `love.graphics` is not rewritten into a path and mistaken for a file.
    if spec == "love"
        || spec.starts_with("love.")
        || spec.starts_with("love/")
        || spec == "ffi"
        || spec.starts_with("ffi.")
    {
        return super::Resolution::Stdlib;
    }

    for base in module_base(from, spec) {
        for suffix in ["", ".lua", "/init.lua"] {
            let path = format!("{base}{suffix}");
            if let Some(i) = index.get(path.as_str()) {
                return super::Resolution::File(*i);
            }
        }
    }
    super::Resolution::Unresolved
}

/// A function head is a valid Lua name when it starts with a letter/underscore and only
/// contains word characters, dots (table fields) and at most one `:` (a method).
fn valid_head(head: &str) -> bool {
    let Some(first) = head.chars().next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && head.matches(':').count() <= 1
        && head
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == ':')
}

/// True once the parameter tail (which starts just after the opening `(`) has seen its
/// matching `)`, ignoring brackets and quoted strings. Used to join multi-line headers.
fn has_closing_paren(tail: &str) -> bool {
    let mut depth = 1i32;
    let mut quote = None;
    for c in tail.chars() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                depth -= 1;
                if depth == 0 {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// Parse the text beginning at `(` into parameters, honouring nesting, quoted defaults
/// and varargs. Names are kept verbatim (without `...`), with `rest`/`optional` set.
fn parse_params(tail: &str) -> Vec<Param> {
    let Some(inner) = tail.strip_prefix('(') else {
        return Vec::new();
    };
    let mut depth = 0i32;
    let mut quote = None;
    let mut end = inner.len();
    for (i, c) in inner.char_indices() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                if depth == 0 && c == ')' {
                    end = i;
                    break;
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    let inner = &inner[..end];

    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth = 0i32;
    let mut quote = None;
    for (i, c) in inner.char_indices() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&inner[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&inner[start..]);

    parts
        .into_iter()
        .filter_map(|raw| {
            let p = raw.trim();
            if p.is_empty() {
                return None;
            }
            let (name, optional, rest) = if p == "..." {
                (String::new(), true, true)
            } else if let Some(r) = p.strip_prefix("...") {
                (r.trim().to_string(), true, true)
            } else {
                let (name, optional) = p
                    .split_once('=')
                    .map_or((p, false), |(name, _)| (name, true));
                (name.trim().to_string(), optional, false)
            };
            Some(Param {
                name,
                ts_type: "unknown".to_string(),
                optional,
                rest,
                ..Param::default()
            })
        })
        .collect()
}

/// Exports: `function name(...)`, `function M.name(...)`, `function M:method(...)` and
/// multi-line variants. A dotted head is a table export; a `:` head is a method whose
/// implicit `self` becomes the first parameter so the bridged signature matches Lua.
pub fn exports(path: &str, source: &str) -> Vec<ExportedFn> {
    if !path.ends_with(".lua") {
        return Vec::new();
    }
    let mut out = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut n = 0usize;
    while n < lines.len() {
        let trimmed = lines[n].trim_start();
        let Some(rest) = trimmed.strip_prefix("function ") else {
            n += 1;
            continue;
        };
        let Some((head, tail)) = rest.split_once('(') else {
            n += 1;
            continue;
        };
        let head = head.trim();
        if !valid_head(head) {
            n += 1;
            continue;
        }
        let start_line = n;
        let mut header = trimmed.to_string();
        let mut tail_owned = tail.to_string();
        while !has_closing_paren(&tail_owned) && n + 1 < lines.len() {
            n += 1;
            let next = lines[n].trim();
            header.push(' ');
            header.push_str(next);
            tail_owned.push(' ');
            tail_owned.push_str(next);
        }

        let is_method = head.contains(':');
        let name = if is_method {
            head.replace(':', ".")
        } else {
            head.to_string()
        };
        let mut params = parse_params(&format!("({tail_owned}"));
        if is_method {
            params.insert(
                0,
                Param {
                    name: "self".to_string(),
                    ts_type: "unknown".to_string(),
                    ..Param::default()
                },
            );
        }
        // Give synthetic parameters a stable name when the source omitted them.
        let params = params
            .into_iter()
            .enumerate()
            .map(|(i, mut p)| {
                if p.name.is_empty() {
                    p.name = format!("arg{i}");
                }
                p
            })
            .collect();
        out.push(ExportedFn {
            name,
            file: path.to_string(),
            line: start_line as u32 + 1,
            params,
            returns: "unknown".to_string(),
            is_async: false,
            prototype: Some(header),
            ..ExportedFn::default()
        });
        n += 1;
    }
    out
}

/// The receiver and method name of a Lua export (`("M", "f")`, `("love", "load")`).
/// Both `function M.f()` and `function M:f()` address `M.f`.
pub fn receiver(f: &ExportedFn) -> (String, String) {
    let proto = f.prototype.as_deref().unwrap_or(&f.name);
    let head = proto
        .split('(')
        .next()
        .unwrap_or(proto)
        .trim_start_matches("function ")
        .trim();
    if let Some((recv, name)) = head.rsplit_once(':') {
        return (recv.trim().to_string(), name.trim().to_string());
    }
    match head.rsplit_once('.') {
        Some((recv, name)) => (recv.to_string(), name.to_string()),
        None => ("_G".into(), head.to_string()),
    }
}

pub fn signature(f: &ExportedFn) -> String {
    let proto = f.prototype.as_deref().unwrap_or("");
    let is_method = proto.split('(').next().unwrap_or("").contains(':');
    let params = proto
        .split_once('(')
        .map(|(_, rest)| rest.split_once(')').map_or(rest, |(p, _)| p).to_string())
        .unwrap_or_default();
    if is_method {
        if params.trim().is_empty() {
            "self".to_string()
        } else {
            format!("self, {params}")
        }
    } else {
        params
    }
}

/// LÖVE callbacks, in the order the engine calls them.
pub const LOVE_CALLBACKS: &[&str] = &[
    "load",
    "update",
    "draw",
    "keypressed",
    "keyreleased",
    "mousepressed",
    "mousereleased",
    "mousemoved",
    "wheelmoved",
    "touchpressed",
    "touchmoved",
    "touchreleased",
    "gamepadpressed",
    "gamepadreleased",
    "gamepadaxis",
    "joystickadded",
    "joystickremoved",
    "focus",
    "quit",
    "resize",
    "visible",
    "errorhandler",
    "threaderror",
];

pub fn is_love_callback(name: &str) -> bool {
    LOVE_CALLBACKS.contains(&name)
}

/// Crate hints for `require`d Lua modules.
pub fn lookup(name: &str) -> Option<CrateMatch> {
    let (crates, note) = if name.starts_with("love") {
        ("love2d", "LÖVE engine API; port with a Rust engine layer")
    } else if name == "ffi" {
        ("libloading", "LuaJIT FFI calls a C ABI; use a Rust cdylib")
    } else {
        return None;
    };
    Some(CrateMatch {
        crates,
        fit: crate::ts::deps::Fit::Partial,
        note,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_index() -> HashMap<&'static str, usize> {
        HashMap::new()
    }

    #[test]
    fn detects_requires_and_love_callbacks() {
        let src = "function love.load()\n  local s = require('src.data.constants')\nend\n";
        let facts = scan(src);
        assert_eq!(facts.imports.len(), 1);
        assert_eq!(facts.imports[0].specifier, "src/data/constants");
        let f = exports("main.lua", src);
        assert_eq!(f.len(), 1);
        assert_eq!(receiver(&f[0]), ("love".into(), "load".into()));
        assert!(is_love_callback("load"));
    }

    #[test]
    fn resolves_dotted_require() {
        let mut index = empty_index();
        index.insert("src/data/constants.lua", 7usize);
        let r = resolve(
            "main.lua",
            "src/data/constants",
            &index,
            &Workspace {
                packages: vec![],
                files: vec![],
            },
        );
        assert!(matches!(r, crate::lang::Resolution::File(7)));
    }

    #[test]
    fn detects_dotted_and_method_exports() {
        let src = concat!(
            "local M = {}\n",
            "function M.reset()\n",
            "  return 0\n",
            "end\n",
            "function M:step(dt, opts)\n",
            "  return dt\n",
            "end\n",
            "function love.load()\nend\n",
        );
        let f = exports("src/game/luck.lua", src);
        assert_eq!(f.len(), 3, "all three declaration forms are exports");

        assert_eq!(f[0].name, "M.reset");
        assert_eq!(f[0].line, 2);
        assert_eq!(f[0].file, "src/game/luck.lua");
        assert_eq!(f[0].prototype.as_deref(), Some("function M.reset()"));
        assert_eq!(receiver(&f[0]), ("M".into(), "reset".into()));
        assert!(f[0].params.is_empty());

        assert_eq!(f[1].name, "M.step");
        assert_eq!(f[1].line, 5);
        assert_eq!(f[1].prototype.as_deref(), Some("function M:step(dt, opts)"));
        assert_eq!(receiver(&f[1]), ("M".into(), "step".into()));
        assert_eq!(signature(&f[1]), "self, dt, opts");
        assert_eq!(
            f[1].params
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            vec!["self", "dt", "opts"]
        );

        // A decompiled table like `var_0_9` is just another dotted receiver.
        let bytecode_style = "function var_0_9.getState(arg_1_0, ...)\nend\n";
        let g = exports("src/game/luck.lua", bytecode_style);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].name, "var_0_9.getState");
        assert_eq!(receiver(&g[0]), ("var_0_9".into(), "getState".into()));
        assert_eq!(
            g[0].params
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            vec!["arg_1_0", "arg1"]
        );
        assert!(g[0].params[1].rest);
    }

    #[test]
    fn joins_multiline_function_headers() {
        let src = "function M.compute(\n  a,\n  b = 2\n)\n  return a\nend\n";
        let f = exports("src/x.lua", src);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].name, "M.compute");
        assert_eq!(f[0].line, 1);
        assert_eq!(
            f[0].prototype.as_deref(),
            Some("function M.compute( a, b = 2 )")
        );
        assert_eq!(f[0].params.len(), 2);
        assert!(f[0].params[1].optional);
    }

    #[test]
    fn resolves_dotted_relative_and_parent_requires() {
        let mut index = empty_index();
        index.insert("src/data/constants.lua", 1usize);
        index.insert("src/data/constants/init.lua", 2usize);
        index.insert("src/game/helper.lua", 3usize);
        index.insert("src/save.lua", 4usize);
        let ws = Workspace {
            packages: vec![],
            files: vec![],
        };
        // A macro, not a closure: each expansion borrows `index` only for its statement,
        // so the test can mutate the index in between to exercise the `/init.lua` fallback.
        macro_rules! resolved {
            ($spec:expr, $from:expr) => {
                resolve($from, $spec, &index, &ws)
            };
        }

        // Dotted module name -> both the flat file and the directory module.
        assert!(matches!(
            resolved!("src.data.constants", "main.lua"),
            crate::lang::Resolution::File(1)
        ));
        assert!(matches!(
            resolved!("src/data/constants", "main.lua"),
            crate::lang::Resolution::File(1)
        ));
        index.remove("src/data/constants.lua");
        assert!(matches!(
            resolved!("src.data.constants", "main.lua"),
            crate::lang::Resolution::File(2)
        ));

        // Relative `./` is anchored at the importer's directory; `../` walks up.
        assert!(matches!(
            resolved!("./helper", "src/game/luck.lua"),
            crate::lang::Resolution::File(3)
        ));
        assert!(matches!(
            resolved!("./constants", "src/game/luck.lua"),
            crate::lang::Resolution::Unresolved
        ));
        assert!(matches!(
            resolved!("../save", "src/game/luck.lua"),
            crate::lang::Resolution::File(4)
        ));
        assert!(matches!(
            resolved!("./sub.thing", "src/game/luck.lua"),
            crate::lang::Resolution::Unresolved
        ));

        // Engine modules stay platform imports, never unresolved internal files.
        assert!(matches!(
            resolved!("love.graphics", "main.lua"),
            crate::lang::Resolution::Stdlib
        ));
        assert!(matches!(
            resolved!("ffi", "main.lua"),
            crate::lang::Resolution::Stdlib
        ));
    }

    #[test]
    fn require_specs_reads_every_call_on_a_line() {
        let specs = require_specs(
            "local a = require('src.data.constants'); local b = require(\"./helper\")",
        );
        assert_eq!(specs, vec!["src/data/constants", "./helper"]);
        assert!(require_specs("function require(arg)").is_empty());
    }

    #[test]
    fn scan_keeps_love_callbacks_distinct_from_dynamic_load() {
        let ok = scan("function love.load()\nend\nfunction M.load()\nend\n");
        assert!(!ok.risks.contains_key(&Risk::DynamicCode));

        let risky = scan("function load(s)\nend\nlocal x = load(s)\nloadstring(s)\n");
        assert!(risky.risks.contains_key(&Risk::DynamicCode));
    }
}
