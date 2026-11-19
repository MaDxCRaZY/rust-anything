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

/// `require("a.b.c")` -> `a/b/c`, `require("./x")` -> `./x`.
fn require_specs(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(i) = line.find("require") {
        let rest = &line[i + "require".len()..];
        let rest = rest.trim_start();
        let rest = rest.strip_prefix('(').unwrap_or(rest).trim_start();
        if rest.starts_with('"') || rest.starts_with('\'') {
            let q = rest.chars().next().unwrap();
            if let Some(end) = rest[1..].find(q) {
                let spec = unquote(&rest[..end + 2]);
                out.push(if spec.contains('.') && !spec.starts_with('.') {
                    spec.replace('.', "/")
                } else {
                    spec.to_string()
                });
            }
        }
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
        // `love.load()` is a LÖVE callback, not Lua's dynamic-code `load()`.
        let dynamic_load = trimmed.contains("load(") && !trimmed.contains(".load(");
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

pub fn resolve(
    from: &str,
    spec: &str,
    index: &HashMap<&str, usize>,
    _ws: &Workspace,
) -> super::Resolution {
    let mut candidates = Vec::new();
    if let Some(rel) = spec.strip_prefix("./") {
        let dir = from.rsplit_once('/').map_or("", |(d, _)| d);
        candidates.push(join(&dir, rel));
    } else {
        candidates.push(spec.to_string());
    }

    for base in candidates {
        for suffix in [".lua", "/init.lua"] {
            let path = format!("{base}{suffix}");
            if let Some(i) = index.get(path.as_str()) {
                return super::Resolution::File(*i);
            }
        }
    }
    if spec.starts_with("love.") || spec == "love" {
        return super::Resolution::Stdlib;
    }
    super::Resolution::Unresolved
}

/// Exports: `function name(...)`, `function M.name(...)`, `function love.load(...)`.
pub fn exports(path: &str, source: &str) -> Vec<ExportedFn> {
    if !path.ends_with(".lua") {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (n, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("function ") else {
            continue;
        };
        // Skip `local function` (already filtered by the prefix) and method definitions with `:`.
        if rest.contains(':') {
            continue;
        }
        let Some((head, tail)) = rest.split_once('(') else {
            continue;
        };
        let name = head.trim();
        if name.is_empty() || name.starts_with('(') {
            continue;
        }
        let params = tail
            .split_once(')')
            .map_or(tail, |(p, _)| p)
            .split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(|p| Param {
                name: p
                    .split('=')
                    .next()
                    .unwrap_or(p)
                    .trim()
                    .trim_start_matches("...")
                    .to_string(),
                ts_type: "unknown".to_string(),
                optional: p.contains('=') || p.starts_with("..."),
                rest: p.starts_with("..."),
                ..Param::default()
            })
            .collect::<Vec<_>>();
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
            name: name.to_string(),
            file: path.to_string(),
            line: n as u32 + 1,
            params,
            returns: "unknown".to_string(),
            is_async: false,
            prototype: Some(trimmed.to_string()),
            ..ExportedFn::default()
        });
    }
    out
}

/// The receiver and method name of a Lua export (`("M", "f")`, `("love", "load")`).
pub fn receiver(f: &ExportedFn) -> (String, String) {
    let proto = f.prototype.as_deref().unwrap_or(&f.name);
    let head = proto
        .split('(')
        .next()
        .unwrap_or(proto)
        .trim_start_matches("function ")
        .trim();
    match head.rsplit_once('.') {
        Some((recv, name)) => (recv.to_string(), name.to_string()),
        None => ("_G".into(), head.to_string()),
    }
}

pub fn signature(f: &ExportedFn) -> String {
    let proto = f.prototype.as_deref().unwrap_or("");
    proto
        .split_once('(')
        .map(|(_, rest)| rest.split_once(')').map_or(rest, |(p, _)| p).to_string())
        .unwrap_or_default()
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
        let mut index = HashMap::new();
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
}
