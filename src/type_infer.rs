//! Infer concrete Rust types for a Lua port from the cases `record` wrote to `cases.jsonl`.
//!
//! Lua has no type annotations (`lang::lua` gives every parameter and return the TS type
//! `unknown`, which the port generator maps to `serde_json::Value`). A recording, though,
//! carries real values: `{"fn": name, "args": [...], "ok": value}` for a successful call and
//! `{"fn": name, "args": [...], "error": value}` for one that raised. This module collapses
//! the values seen for each argument position and for each return into one Rust type:
//!
//! - JSON integers -> `i64`, real numbers (and `$num` markers) -> `f64`, `bool`, `string`.
//! - Arrays -> `Vec<T>` when their elements agree, `Vec<serde_json::Value>` when they do not.
//! - Objects -> `serde_json::Map<String, serde_json::Value>` (a struct-like map; `preserve_order`
//!   keeps Lua's insertion order, which matters for table parity).
//! - A slot with no values, or with values that do not agree, -> `serde_json::Value`.
//!
//! Inference is best-effort and never fails a build: a missing, empty or unreadable
//! `cases.jsonl` simply yields no types, and the caller keeps the signature it had before.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

/// One recorded case. `fn` is a Rust keyword, so it is renamed on the wire.
#[derive(Clone, Debug, Deserialize)]
pub struct Case {
    #[serde(rename = "fn")]
    pub name: String,
    #[serde(default)]
    pub args: Vec<Value>,
    /// Present (even as `null`) when the original returned. `null` means "no value" in Lua
    /// and carries no type information.
    #[serde(default)]
    pub ok: Option<Value>,
    /// Present when the original raised; such cases still inform argument types but say
    /// nothing about the return type.
    #[serde(default)]
    pub error: Option<Value>,
}

/// The Rust type guessed for one value slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InferredType {
    /// A JSON integer that fits an `i64`.
    Int,
    /// A JSON real, a large unsigned integer, or a `$num` marker (NaN/Infinity/-0).
    Float,
    Bool,
    Str,
    /// A JSON array. The element type is `Json` when elements disagree or the array is empty.
    Array(Box<InferredType>),
    /// A JSON object: a struct-like map keyed by string.
    Map,
    /// The fallback: no information, or conflicting information.
    Json,
}

impl InferredType {
    /// The Rust type to write in a `port.rs` signature.
    pub fn rust(&self) -> String {
        match self {
            Self::Int => "i64".into(),
            Self::Float => "f64".into(),
            Self::Bool => "bool".into(),
            Self::Str => "String".into(),
            Self::Array(elem) => format!("Vec<{}>", elem.rust()),
            Self::Map => "serde_json::Map<String, serde_json::Value>".into(),
            Self::Json => "serde_json::Value".into(),
        }
    }
}

/// Two observations agree only when they infer the same type. Anything else, including a
/// `Json` fallback meeting a concrete type, stays a fallback so an inferred signature is
/// never narrower than the evidence for it.
fn agree(a: &InferredType, b: &InferredType) -> InferredType {
    if a == b {
        a.clone()
    } else {
        InferredType::Json
    }
}

/// The codec's marker for a number JSON itself cannot hold (`NaN`, `Infinity`, `-Infinity`,
/// `-0`): such a value is a number, not the object its JSON spelling suggests.
fn is_number_marker(map: &serde_json::Map<String, Value>) -> bool {
    if map.len() != 1 {
        return false;
    }
    map.get("$num")
        .and_then(Value::as_str)
        .is_some_and(|s| matches!(s, "NaN" | "Infinity" | "-Infinity" | "-0"))
}

/// Classify one recorded value. `null` is a Lua nil and carries no information, so it maps to
/// the `Json` fallback (a slot made only of nulls is therefore left as `serde_json::Value`).
pub fn classify(value: &Value) -> InferredType {
    match value {
        Value::Null => InferredType::Json,
        Value::Bool(_) => InferredType::Bool,
        Value::Number(n) => {
            if n.is_i64() || n.as_u64().is_some_and(|u| u <= i64::MAX as u64) {
                InferredType::Int
            } else {
                // A real, or an unsigned integer too large for `i64`.
                InferredType::Float
            }
        }
        Value::String(_) => InferredType::Str,
        Value::Array(items) => {
            let mut elem = InferredType::Json;
            let mut seen = false;
            for item in items {
                // `null` inside an array is still a value: an array of nulls is `Vec<Value>`.
                let t = classify(item);
                elem = if seen { agree(&elem, &t) } else { t };
                seen = true;
            }
            InferredType::Array(Box::new(elem))
        }
        Value::Object(map) => {
            if is_number_marker(map) {
                InferredType::Float
            } else {
                InferredType::Map
            }
        }
    }
}

/// Types inferred for one export: one entry per argument position, plus a return type.
#[derive(Clone, Debug, Default)]
pub struct FunctionTypes {
    pub params: Vec<InferredType>,
    pub ret: Option<InferredType>,
}

/// Types inferred for every export in a recording, keyed by the case's `fn` name.
#[derive(Clone, Debug, Default)]
pub struct Inferred {
    pub funcs: HashMap<String, FunctionTypes>,
}

impl Inferred {
    pub fn is_empty(&self) -> bool {
        self.funcs.is_empty()
    }

    /// The type seen at argument `index` of `name`, when the recording had a value there.
    pub fn param(&self, name: &str, index: usize) -> Option<&InferredType> {
        self.funcs.get(name)?.params.get(index)
    }

    /// The return type seen for `name`, when a successful case recorded a value.
    pub fn ret(&self, name: &str) -> Option<&InferredType> {
        self.funcs.get(name)?.ret.as_ref()
    }
}

/// Fold one observed value into a slot. `None` means "no evidence yet"; `null` carries no
/// information and is skipped.
fn fold(observed: &mut Option<InferredType>, value: Option<&Value>) {
    let Some(value) = value else { return };
    if value.is_null() {
        return;
    }
    let t = classify(value);
    *observed = Some(match observed.take() {
        Some(prev) => agree(&prev, &t),
        None => t,
    });
}

/// Per-export accumulator while cases are being read.
#[derive(Default)]
struct Accum {
    /// One slot per argument position; `None` means nothing recorded there yet.
    params: Vec<Option<InferredType>>,
    ret: Option<InferredType>,
}

/// Infer from already-parsed cases (also used by the unit tests).
pub fn infer<'a>(cases: impl IntoIterator<Item = &'a Case>) -> Inferred {
    let mut accums: HashMap<String, Accum> = HashMap::new();
    for case in cases {
        let acc = accums.entry(case.name.clone()).or_default();
        for (i, arg) in case.args.iter().enumerate() {
            if acc.params.len() <= i {
                acc.params.resize(i + 1, None);
            }
            fold(&mut acc.params[i], Some(arg));
        }
        // Only successful calls tell us the return type; a raised call has no return value.
        if case.error.is_none() {
            fold(&mut acc.ret, case.ok.as_ref());
        }
    }
    let funcs = accums
        .into_iter()
        .map(|(name, acc)| {
            (
                name,
                FunctionTypes {
                    params: acc
                        .params
                        .into_iter()
                        .map(|t| t.unwrap_or(InferredType::Json))
                        .collect(),
                    ret: acc.ret,
                },
            )
        })
        .collect();
    Inferred { funcs }
}

/// Read `<crate_dir>/cases.jsonl` and infer types. A missing, empty or partially malformed
/// recording is not an error: malformed lines are skipped and an empty result is returned.
pub fn from_cases_dir(crate_dir: &Path) -> Inferred {
    let Ok(text) = std::fs::read_to_string(crate_dir.join(crate::harness::CASES)) else {
        return Inferred::default();
    };
    let cases: Vec<Case> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Case>(l).ok())
        .collect();
    infer(&cases)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn case(args: Vec<Value>, ok: Value) -> Case {
        Case {
            name: "f".into(),
            args,
            ok: Some(ok),
            error: None,
        }
    }

    fn param_of(cases: &[Case], i: usize) -> InferredType {
        infer(cases).param("f", i).cloned().unwrap()
    }

    #[test]
    fn infers_integers_and_floats_separately() {
        let ints = [
            case(vec![json!(1)], json!(null)),
            case(vec![json!(-7)], json!(null)),
        ];
        assert_eq!(param_of(&ints, 0), InferredType::Int);
        let floats = [
            case(vec![json!(0.1)], json!(null)),
            case(vec![json!(1e21)], json!(null)),
        ];
        assert_eq!(param_of(&floats, 0), InferredType::Float);
        // A large unsigned integer does not fit `i64`, so it maps to `f64`.
        let big = [case(vec![json!(u64::MAX)], json!(null))];
        assert_eq!(param_of(&big, 0), InferredType::Float);
    }

    #[test]
    fn mixed_int_and_float_falls_back_to_value() {
        let cases = [
            case(vec![json!(1)], json!(null)),
            case(vec![json!(2.5)], json!(null)),
        ];
        assert_eq!(param_of(&cases, 0), InferredType::Json);
    }

    #[test]
    fn infers_strings_and_bools() {
        let s = [case(vec![json!("hi"), json!(true)], json!(null))];
        assert_eq!(param_of(&s, 0), InferredType::Str);
        assert_eq!(param_of(&s, 1), InferredType::Bool);
    }

    #[test]
    fn infers_arrays_and_mixed_arrays() {
        let nums = [case(vec![json!([1, 2, 3])], json!(null))];
        assert_eq!(
            param_of(&nums, 0),
            InferredType::Array(Box::new(InferredType::Int))
        );
        let mixed = [case(vec![json!(["a", 1])], json!(null))];
        assert_eq!(
            param_of(&mixed, 0),
            InferredType::Array(Box::new(InferredType::Json))
        );
        let empty = [case(vec![json!([])], json!(null))];
        assert_eq!(
            param_of(&empty, 0),
            InferredType::Array(Box::new(InferredType::Json))
        );
    }

    #[test]
    fn infers_objects_as_maps() {
        let cases = [case(vec![json!({"window": {"width": 1280}})], json!(null))];
        assert_eq!(param_of(&cases, 0), InferredType::Map);
        assert_eq!(
            param_of(&cases, 0).rust(),
            "serde_json::Map<String, serde_json::Value>"
        );
    }

    #[test]
    fn recognizes_number_markers_as_floats() {
        for marker in ["NaN", "Infinity", "-Infinity", "-0"] {
            let cases = [case(vec![json!({"$num": marker})], json!(null))];
            assert_eq!(param_of(&cases, 0), InferredType::Float, "{marker}");
        }
        // An unrelated one-key object is still a map.
        let cases = [case(vec![json!({"a": 1})], json!(null))];
        assert_eq!(param_of(&cases, 0), InferredType::Map);
    }

    #[test]
    fn inconsistent_values_fall_back_to_value() {
        let cases = [
            case(vec![json!("x")], json!(null)),
            case(vec![json!({"a": 1})], json!(null)),
        ];
        assert_eq!(param_of(&cases, 0), InferredType::Json);
    }

    #[test]
    fn null_and_empty_carry_no_information() {
        let nulls = [case(vec![json!(null)], json!(null))];
        assert_eq!(param_of(&nulls, 0), InferredType::Json);
        assert!(infer(&[] as &[Case]).is_empty());
    }

    #[test]
    fn records_a_return_type_only_from_successes() {
        let cases = [
            case(vec![], json!(1)),
            Case {
                name: "f".into(),
                args: vec![],
                ok: None,
                error: Some(json!("boom")),
            },
        ];
        assert_eq!(infer(&cases).ret("f"), Some(&InferredType::Int));
        // A function whose only recorded result is nil keeps the Value fallback.
        let nils = [case(vec![], json!(null))];
        assert_eq!(infer(&nils).ret("f"), None);
    }

    #[test]
    fn reads_cases_from_a_directory() {
        let dir = std::env::temp_dir().join(format!("ra-infer-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(crate::harness::CASES);
        std::fs::write(
            &path,
            "{\"fn\":\"love.conf\",\"args\":[{\"window\":{\"width\":1280}}]}\n\
             {\"fn\":\"love.conf\",\"args\":[{\"window\":{\"width\":1280}}]}\n",
        )
        .unwrap();
        let inferred = from_cases_dir(&dir);
        assert_eq!(inferred.param("love.conf", 0), Some(&InferredType::Map));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
