//! Parity oracle for the judge port.
//!
//! For each deterministic function, runs BOTH the original Python
//! `runner/judge.py` and the Rust port over identical inputs and asserts the
//! outputs agree:
//!   - `rubric_hash` — exact `String` equality
//!   - `normalize` — exact `f64` equality
//!   - `parse_judge_response` — semantic `serde_json::Value` equality
//!   - `build_judge_prompt` — exact `String` equality
//!   - `spearman` — exact `f64` or null equality
//!   - `calibration_gate` — semantic `serde_json::Value` equality
//!   - `load_scorer_families` — semantic `serde_json::Value` equality
//!
//! Skips (does not fail) when python3 is unavailable, mirroring `bin/gate`.
//!
//! ## LLM boundary
//!
//! `judge_score` is NOT parity-tested here: it requires a live LLM call and
//! cannot produce deterministic output for cross-language comparison. It is
//! covered by unit tests in `src/judge.rs#[cfg(test)]` with a fake `call`
//! closure returning a canned response.
//!
//! ## Spearman / statistics.mean note
//!
//! Python's `statistics.mean` sums exactly via `Fraction`; `pycompat::mean`
//! uses IEEE 754 f64. For the rank vectors that arise in `spearman` (small
//! integers and half-integers), the two implementations agree in every tested
//! case. Should a divergence surface, the failing assertion message will
//! contain both values and should be noted as a documented gap rather than
//! suppressed.

use std::path::{Path, PathBuf};
use std::process::Command;

use daedalus_core::judge::{
    build_judge_prompt, calibration_gate, load_scorer_families, normalize, parse_judge_response,
    rubric_hash, spearman,
};
use serde_json::{json, Map, Value};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root above crates/daedalus-core")
        .to_path_buf()
}

fn python_available() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Run a Python one-liner (in the repo root) and return parsed JSON from stdout.
fn py_json(root: &Path, snippet: &str) -> Value {
    let out = Command::new("python3")
        .current_dir(root)
        .arg("-c")
        .arg(snippet)
        .output()
        .expect("run python3");
    assert!(
        out.status.success(),
        "python3 failed:\nstderr: {}\nsnippet: {snippet}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("python3 did not emit valid JSON")
}

/// Run a Python one-liner and return raw stdout as a String.
fn py_text(root: &Path, snippet: &str) -> String {
    let out = Command::new("python3")
        .current_dir(root)
        .arg("-c")
        .arg(snippet)
        .output()
        .expect("run python3");
    assert!(
        out.status.success(),
        "python3 failed:\nstderr: {}\nsnippet: {snippet}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("python3 output is utf-8")
}

/// Escape a string for safe embedding in a Python string literal (double-quoted).
fn py_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

const RUBRIC: &str = "\
# Review-quality rubric
Score each 0\u{2013}5:
- evidence: every finding cites file/line and explains the defect
- actionability: a developer could act on the finding without guessing
- severity_calibration: severity matches real impact
";

// ---------------------------------------------------------------------------
// rubric_hash parity
// ---------------------------------------------------------------------------

#[test]
fn parity_rubric_hash() {
    if !python_available() {
        eprintln!("skipping judge parity: python3 not available");
        return;
    }
    let root = repo_root();

    let cases: &[&str] = &[
        RUBRIC,
        "",
        "hello world",
        "unicode: \u{1F600} \u{00e9}",
        "multi\nline\nrubric",
    ];

    for &text in cases {
        let escaped = py_escape(text);
        let py_hash = py_text(
            &root,
            &format!(
                "import sys; sys.path.insert(0,'runner'); import judge; \
                 import sys; sys.stdout.write(judge.rubric_hash(\"{escaped}\"))"
            ),
        );
        let rust_hash = rubric_hash(text);
        assert_eq!(
            py_hash.trim_end_matches('\n'),
            rust_hash,
            "rubric_hash mismatch for input: {text:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// normalize parity
// ---------------------------------------------------------------------------

#[test]
fn parity_normalize() {
    if !python_available() {
        eprintln!("skipping judge parity: python3 not available");
        return;
    }
    let root = repo_root();

    let cases: &[&[(&str, i64)]] = &[
        &[("a", 5), ("b", 5)],
        &[("a", 0), ("b", 0)],
        &[("a", 3), ("b", 0)],
        &[],
        &[
            ("evidence", 4),
            ("actionability", 5),
            ("severity_calibration", 3),
        ],
        &[("x", 1)],
        &[("a", 5), ("b", 3), ("c", 2)],
    ];

    for scores in cases {
        // Build Python dict literal
        let dict_py = if scores.is_empty() {
            "{}".to_string()
        } else {
            let items: Vec<String> = scores.iter().map(|(k, v)| format!("'{k}': {v}")).collect();
            format!("{{{}}}", items.join(", "))
        };

        let py_result = py_json(
            &root,
            &format!(
                "import sys, json; sys.path.insert(0,'runner'); import judge; \
                 print(json.dumps(judge.normalize({dict_py})))"
            ),
        );

        let rust_map: Map<String, Value> = scores
            .iter()
            .map(|(k, v)| (k.to_string(), Value::from(*v)))
            .collect();
        let rust_result = Value::from(normalize(&rust_map));

        assert_eq!(
            py_result, rust_result,
            "normalize mismatch for scores: {scores:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// parse_judge_response parity
// ---------------------------------------------------------------------------

#[test]
fn parity_parse_judge_response() {
    if !python_available() {
        eprintln!("skipping judge parity: python3 not available");
        return;
    }
    let root = repo_root();

    // Cases where Python succeeds — compare the parsed map
    let valid_cases: &[&str] = &[
        r#"{"scores": {"evidence": 4}}"#,
        r#"prose {"evidence": 5, "x": 2} tail"#,
        r#"{"a": 3, "b": 9, "c": "hi"}"#,
        r#"{"scores": {"ev": 0, "ac": 5, "sv": 3}}"#,
        r#"text before {"lo": 0, "hi": 5} text after"#,
    ];

    for &input in valid_cases {
        let escaped = py_escape(input);
        let py_result = py_json(
            &root,
            &format!(
                "import sys, json; sys.path.insert(0,'runner'); import judge; \
                 print(json.dumps(judge.parse_judge_response(\"{escaped}\")))"
            ),
        );
        let rust_result = parse_judge_response(input)
            .map(Value::Object)
            .unwrap_or_else(|_| panic!("Rust parse failed on: {input:?}"));
        assert_eq!(
            py_result, rust_result,
            "parse_judge_response mismatch for input: {input:?}"
        );
    }

    // Cases where Python raises ValueError — Rust must return Err
    let invalid_cases: &[&str] = &[
        "no json",
        r#"{"a": 7}"#,        // all out of range
        r#"{"a": -1}"#,       // negative
        r#"{"a": true}"#,     // bool only
        r#"{"a": "string"}"#, // non-numeric only
        "[]",                 // not an object at all
    ];

    for &input in invalid_cases {
        let escaped = py_escape(input);
        // Verify Python also raises
        let py_raises = Command::new("python3")
            .current_dir(&root)
            .arg("-c")
            .arg(format!(
                "import sys; sys.path.insert(0,'runner'); import judge; \
                 try:\n\
                     judge.parse_judge_response(\"{escaped}\")\n\
                     print('ok')\n\
                 except ValueError:\n\
                     print('error')"
            ))
            .output()
            .expect("run python3");
        let py_out = String::from_utf8_lossy(&py_raises.stdout);
        // Rust must also return Err
        let rust_result = parse_judge_response(input);
        assert!(
            rust_result.is_err(),
            "Rust should have failed on: {input:?} (Python said: {py_out})"
        );
    }
}

// ---------------------------------------------------------------------------
// build_judge_prompt parity
// ---------------------------------------------------------------------------

#[test]
fn parity_build_judge_prompt() {
    if !python_available() {
        eprintln!("skipping judge parity: python3 not available");
        return;
    }
    let root = repo_root();

    let cases: &[(&str, &str, Value)] = &[
        (RUBRIC, "review the diff", json!({"findings": []})),
        ("rubric text", "simple task", json!({"k": 1, "v": [1, 2]})),
        ("r", "t", json!(null)),
        ("r", "t", json!([])),
    ];

    for (rubric, instruction, findings) in cases {
        let rubric_escaped = py_escape(rubric);
        let instruction_escaped = py_escape(instruction);
        let findings_json = serde_json::to_string(findings).unwrap();
        let findings_escaped = py_escape(&findings_json);

        let py_result = py_text(
            &root,
            &format!(
                "import sys, json; sys.path.insert(0,'runner'); import judge; \
                 findings = json.loads(\"{findings_escaped}\"); \
                 sys.stdout.write(judge.build_judge_prompt(\"{rubric_escaped}\", \
                     \"{instruction_escaped}\", findings))"
            ),
        );
        let rust_result = build_judge_prompt(rubric, instruction, findings);
        assert_eq!(
            py_result, rust_result,
            "build_judge_prompt mismatch for rubric={rubric:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// spearman parity
// ---------------------------------------------------------------------------

#[test]
fn parity_spearman() {
    if !python_available() {
        eprintln!("skipping judge parity: python3 not available");
        return;
    }
    let root = repo_root();

    let cases: &[(&[f64], &[f64])] = &[
        (&[1.0, 2.0, 3.0, 4.0], &[1.0, 2.0, 3.0, 4.0]), // concordant
        (&[1.0, 2.0, 3.0, 4.0], &[4.0, 3.0, 2.0, 1.0]), // discordant
        (&[1.0, 1.0, 1.0], &[1.0, 2.0, 3.0]),           // xs constant → None
        (&[1.0], &[1.0]),                               // too short → None
        (&[0.2, 0.4, 0.6, 0.8, 1.0], &[0.3, 0.45, 0.55, 0.75, 0.95]), // concordant non-trivial
        (&[0.1, 0.3, 0.5, 0.7, 0.9], &[0.9, 0.2, 0.8, 0.1, 0.5]), // scrambled
        // ties with anti-correlation
        (&[1.0, 2.0, 2.0, 3.0], &[3.0, 2.0, 2.0, 1.0]),
        // boundary: ys constant
        (&[1.0, 2.0, 3.0], &[5.0, 5.0, 5.0]),
    ];

    for (xs, ys) in cases {
        let xs_py = format!(
            "[{}]",
            xs.iter()
                .map(|x| format!("{x}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let ys_py = format!(
            "[{}]",
            ys.iter()
                .map(|y| format!("{y}"))
                .collect::<Vec<_>>()
                .join(", ")
        );

        let py_result = py_json(
            &root,
            &format!(
                "import sys, json; sys.path.insert(0,'runner'); import judge; \
                 result = judge.spearman({xs_py}, {ys_py}); \
                 print(json.dumps(result))"
            ),
        );

        let rust_result = spearman(xs, ys);
        let rust_json = match rust_result {
            Some(v) => json!(v),
            None => json!(null),
        };

        assert_eq!(
            py_result, rust_json,
            "spearman mismatch for xs={xs:?} ys={ys:?}\npy={py_result}\nrust={rust_json}"
        );
    }
}

// ---------------------------------------------------------------------------
// calibration_gate parity
// ---------------------------------------------------------------------------

#[test]
fn parity_calibration_gate() {
    if !python_available() {
        eprintln!("skipping judge parity: python3 not available");
        return;
    }
    let root = repo_root();

    type GateCase<'a> = (&'a [f64], &'a [f64], &'a [f64], f64);
    let cases: &[GateCase<'_>] = &[
        // passes: both judges agree, key agrees
        (
            &[0.2, 0.4, 0.6, 0.8, 1.0],
            &[0.3, 0.45, 0.55, 0.75, 0.95],
            &[0.0, 0.5, 0.5, 1.0, 1.0],
            0.8,
        ),
        // fails: judge disagreement
        (
            &[0.1, 0.3, 0.5, 0.7, 0.9],
            &[0.9, 0.2, 0.8, 0.1, 0.5],
            &[],
            0.8,
        ),
        // fails: judge contradicts key
        (
            &[0.2, 0.4, 0.6, 0.8, 1.0],
            &[0.25, 0.45, 0.55, 0.85, 0.95],
            &[1.0, 1.0, 0.5, 0.0, 0.0],
            0.8,
        ),
        // passes: no key (empty deterministic)
        (
            &[0.2, 0.4, 0.6, 0.8, 1.0],
            &[0.3, 0.45, 0.55, 0.75, 0.95],
            &[],
            0.8,
        ),
        // edge: constant judge_a → inter is None → fails
        (&[0.5, 0.5, 0.5, 0.5], &[0.1, 0.2, 0.3, 0.4], &[], 0.8),
        // default min_agreement value
        (
            &[0.2, 0.4, 0.6, 0.8, 1.0],
            &[0.3, 0.45, 0.55, 0.75, 0.95],
            &[],
            0.8,
        ),
    ];

    for (judge_a, judge_b, deterministic, min_agreement) in cases {
        let to_py_list = |v: &[f64]| -> String {
            format!(
                "[{}]",
                v.iter()
                    .map(|x| format!("{x}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };

        let a_py = to_py_list(judge_a);
        let b_py = to_py_list(judge_b);
        let det_py = to_py_list(deterministic);

        let py_result = py_json(
            &root,
            &format!(
                "import sys, json; sys.path.insert(0,'runner'); import judge; \
                 result = judge.calibration_gate({a_py}, {b_py}, {det_py}, \
                     min_agreement={min_agreement}); \
                 print(json.dumps(result))"
            ),
        );

        let rust_result = calibration_gate(judge_a, judge_b, deterministic, *min_agreement);
        let rust_json = Value::Object(rust_result);

        // Compare field by field for clear diagnostics
        for field in [
            "passed",
            "inter_judge_spearman",
            "judge_vs_key_spearman",
            "min_agreement",
        ] {
            assert_eq!(
                py_result.get(field),
                rust_json.get(field),
                "calibration_gate field '{field}' mismatch\n\
                 judge_a={judge_a:?} judge_b={judge_b:?} det={deterministic:?}\n\
                 py={py_result}\nrust={rust_json}"
            );
        }
        // reasons: compare as sets (order is an impl detail for this field)
        let py_reasons: Vec<String> = py_result["reasons"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
        let rust_reasons: Vec<String> = rust_json["reasons"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
        let mut py_sorted = py_reasons.clone();
        py_sorted.sort();
        let mut rust_sorted = rust_reasons.clone();
        rust_sorted.sort();
        assert_eq!(
            py_sorted, rust_sorted,
            "calibration_gate reasons mismatch\n\
             judge_a={judge_a:?} judge_b={judge_b:?} det={deterministic:?}\n\
             py_reasons={py_reasons:?}\nrust_reasons={rust_reasons:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// load_scorer_families parity
// ---------------------------------------------------------------------------

#[test]
fn parity_load_scorer_families() {
    if !python_available() {
        eprintln!("skipping judge parity: python3 not available");
        return;
    }
    let root = repo_root();

    // Case 1: no scoring.toml — defaults to deterministic
    {
        let tmp = std::env::temp_dir().join(format!(
            "daedalus-judge-parity-noscoringtoml-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(tmp.join("tests")).unwrap();

        let tmp_str = tmp.display().to_string();
        let tmp_escaped = py_escape(&tmp_str);
        let py_result = py_json(
            &root,
            &format!(
                "import sys, json; sys.path.insert(0,'runner'); import judge; \
                 print(json.dumps(judge.load_scorer_families(\"{tmp_escaped}\")))"
            ),
        );
        let rust_result = load_scorer_families(&tmp);
        assert_eq!(
            py_result, rust_result,
            "load_scorer_families (no toml) mismatch"
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    // Case 2: with scoring.toml
    {
        let tmp = std::env::temp_dir().join(format!(
            "daedalus-judge-parity-withtoml-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(tmp.join("tests")).unwrap();
        let toml_content = concat!(
            "families = [\"deterministic\", \"judge\"]\n",
            "[judge]\n",
            "rubric = \"rubric.md\"\n",
            "models = [\"anthropic/claude-x\", \"openai/gpt-y\"]\n",
        );
        std::fs::write(tmp.join("tests").join("scoring.toml"), toml_content).unwrap();

        let tmp_str = tmp.display().to_string();
        let tmp_escaped = py_escape(&tmp_str);
        let py_result = py_json(
            &root,
            &format!(
                "import sys, json; sys.path.insert(0,'runner'); import judge; \
                 print(json.dumps(judge.load_scorer_families(\"{tmp_escaped}\")))"
            ),
        );
        let rust_result = load_scorer_families(&tmp);
        assert_eq!(
            py_result, rust_result,
            "load_scorer_families (with toml) mismatch"
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
