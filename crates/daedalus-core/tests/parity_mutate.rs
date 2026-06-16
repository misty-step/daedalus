//! Parity oracle for the mutate port.
//!
//! For each deterministic function, runs BOTH the original Python
//! `runner/mutate.py` and the Rust port over identical inputs and asserts
//! the outputs agree:
//!
//!   - `proposal_instructions` — exact `String` equality
//!   - `parse_proposal` — semantic `serde_json::Value` equality (success)
//!     and matching `Err` (failure)
//!   - `normalize_predicted_effect` — semantic equality of returned map + flag
//!   - `resolve_donor` — semantic equality (success) and matching error (failure)
//!   - `worst_trials` — semantic `serde_json::Value` equality
//!   - `build_prompt` — exact `String` equality
//!   - `validate_proposal` — matching success (slot/value) and Err presence
//!   - `build_child` (scalar + list slots only) — semantic equality
//!   - `write_manifest` — exact byte/string equality
//!
//! Skips (does not fail) when `python3` is unavailable, mirroring `bin/gate`.
//!
//! ## LLM boundary
//!
//! `call_optimizer` and `propose` are NOT parity-tested: they require a live
//! OpenRouter endpoint. `propose` is covered by `src/mutate.rs #[cfg(test)]`
//! with a fake injected `call`.
//!
//! ## Parity gaps
//!
//! None documented at this time. All deterministic outputs match exactly.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use daedalus_core::mutate::{
    build_child, build_prompt, normalize_predicted_effect, parse_proposal, proposal_instructions,
    resolve_donor, validate_proposal, worst_trials, write_manifest,
};
use serde_json::{json, Map, Value};

// ---------------------------------------------------------------------------
// Helpers shared across tests
// ---------------------------------------------------------------------------

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

/// Run a Python snippet and return raw stdout as a String.
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

/// Run a Python snippet and return parsed JSON from stdout.
fn py_json(root: &Path, snippet: &str) -> Value {
    let raw = py_text(root, snippet);
    serde_json::from_str(raw.trim())
        .unwrap_or_else(|e| panic!("python3 did not emit valid JSON: {e}\nraw: {raw:?}"))
}

/// Escape a string for safe embedding in a Python double-quoted string literal.
fn py_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Temporary directory unique per test invocation.
fn tmp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "daedalus-mutate-parity-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A minimal parent manifest used across multiple test cases.
fn parent_manifest() -> Map<String, Value> {
    serde_json::from_value(json!({
        "composition": 1,
        "id": "pi-kimi",
        "kind": "pi",
        "model": "moonshotai/kimi-k2.6",
        "prompt_packet": "packets/reviewer-v1.md",
        "thinking": "medium",
        "tools": ["read", "bash", "edit", "write"],
        "timeout_sec": 600
    }))
    .unwrap()
}

/// Prelude to insert into every Python snippet: imports + sys.path.
const PY_PRELUDE: &str = "import sys, json; sys.path.insert(0,'runner'); import mutate; ";

// ---------------------------------------------------------------------------
// proposal_instructions parity
// ---------------------------------------------------------------------------

#[test]
fn parity_proposal_instructions() {
    if !python_available() {
        eprintln!("skipping mutate parity: python3 not available");
        return;
    }
    let root = repo_root();

    // Case 1: no options (default mode)
    {
        let py = py_text(
            &root,
            &format!("{PY_PRELUDE}sys.stdout.write(mutate.proposal_instructions())"),
        );
        let rust = proposal_instructions(None, None, None, &[], None, None, None);
        assert_eq!(
            py.trim_end_matches('\n'),
            rust,
            "proposal_instructions (defaults) mismatch"
        );
    }

    // Case 2: with allowed_models, allowed_thinking, mode, donors, avoid_slots
    {
        let py = py_text(
            &root,
            &format!(
                "{PY_PRELUDE}sys.stdout.write(mutate.proposal_instructions(\
                    allowed_models=['z-ai/glm-5','moonshotai/kimi-k2.6'], \
                    allowed_thinking=['off','low','medium'], \
                    avoid_slots=('thinking',), \
                    mode='max-quality', \
                    donors=['seed2-cheap','seed1']))"
            ),
        );
        let allowed_models = vec!["z-ai/glm-5".to_string(), "moonshotai/kimi-k2.6".to_string()];
        let allowed_thinking = vec!["off".to_string(), "low".to_string(), "medium".to_string()];
        let avoid_slots = vec!["thinking".to_string()];
        let donors = vec!["seed2-cheap".to_string(), "seed1".to_string()];
        let rust = proposal_instructions(
            None,
            Some(&allowed_models),
            Some(&allowed_thinking),
            &avoid_slots,
            None,
            Some("max-quality"),
            Some(&donors),
        );
        assert_eq!(
            py.trim_end_matches('\n'),
            rust,
            "proposal_instructions (with options) mismatch"
        );
    }

    // Case 3: with tool_policies and skill_sets
    {
        let py = py_text(
            &root,
            &format!(
                "{PY_PRELUDE}sys.stdout.write(mutate.proposal_instructions(\
                    tool_policies={{'full': ['r','b','e','w'], 'explore': ['r','b']}}, \
                    skill_sets={{'review-pack': ['a.md'], 'bare': []}}))"
            ),
        );
        let mut tool_policies: HashMap<String, Vec<String>> = HashMap::new();
        tool_policies.insert(
            "full".into(),
            vec!["r".into(), "b".into(), "e".into(), "w".into()],
        );
        tool_policies.insert("explore".into(), vec!["r".into(), "b".into()]);
        let mut skill_sets: HashMap<String, Vec<String>> = HashMap::new();
        skill_sets.insert("review-pack".into(), vec!["a.md".into()]);
        skill_sets.insert("bare".into(), vec![]);
        let rust = proposal_instructions(
            Some(&tool_policies),
            None,
            None,
            &[],
            Some(&skill_sets),
            None,
            None,
        );
        assert_eq!(
            py.trim_end_matches('\n'),
            rust,
            "proposal_instructions (tool_policies + skill_sets) mismatch"
        );
    }
}

// ---------------------------------------------------------------------------
// parse_proposal parity
// ---------------------------------------------------------------------------

#[test]
fn parity_parse_proposal() {
    if !python_available() {
        eprintln!("skipping mutate parity: python3 not available");
        return;
    }
    let root = repo_root();

    // Cases where Python succeeds — compare the parsed objects
    let valid_cases: &[&str] = &[
        r#"{"slot": "model", "value": "x/y", "hypothesis": "h"}"#,
        "Reasoning...\\n```json\\n{\"slot\": \"thinking\", \"value\": \"high\", \"hypothesis\": \"more specific\"}\\n```",
        "Let me think.\\n\\n{\"slot\": \"model\", \"value\": \"a/b\", \"hypothesis\": \"cross-file\"}\\n\\nDone.",
        r#"{"slot": "prompt_packet", "value": "Use {curly} braces always.", "hypothesis": "h"}"#,
        r#"text {"slot": "agents_md", "value": "brief {agents}", "hypothesis": "h"} tail"#,
    ];

    for &input in valid_cases {
        let escaped = py_escape(input);
        let py_result = py_json(
            &root,
            &format!(
                "{PY_PRELUDE}\
                 result = mutate.parse_proposal(\"{escaped}\"); \
                 print(json.dumps(result))"
            ),
        );
        let rust_result = parse_proposal(input)
            .map(Value::Object)
            .unwrap_or_else(|e| panic!("Rust parse_proposal failed on {input:?}: {e}"));
        assert_eq!(
            py_result, rust_result,
            "parse_proposal mismatch for {input:?}"
        );
    }

    // Cases where Python raises ValueError — Rust must return Err
    let invalid_cases: &[&str] = &["no json here", "just prose", "[]", ""];

    for &input in invalid_cases {
        let escaped = py_escape(input);
        // Python: can't mix semicolon-separated statements with try: on same line.
        // Use a proper multi-line snippet.
        let py_raises_snippet = format!(
            "import sys, json\nsys.path.insert(0,'runner')\nimport mutate\n\
             try:\n    mutate.parse_proposal(\"{escaped}\")\n    print('ok')\n\
             except ValueError:\n    print('error')"
        );
        let py_out = py_text(&root, &py_raises_snippet);
        if py_out.trim() == "error" {
            let rust_result = parse_proposal(input);
            assert!(
                rust_result.is_err(),
                "Rust parse_proposal should have Err'd on: {input:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// normalize_predicted_effect parity
// ---------------------------------------------------------------------------

#[test]
fn parity_normalize_predicted_effect() {
    if !python_available() {
        eprintln!("skipping mutate parity: python3 not available");
        return;
    }
    let root = repo_root();

    // (proposal JSON, expect_ok: true/false)
    let cases: &[(&str, bool)] = &[
        (r#"{}"#, true),
        (
            r#"{"predicted_effect": {"reward": "up", "cost": "hold"}}"#,
            true,
        ),
        (
            r#"{"predicted_effect": {"reward": "hold", "cost": "down"}}"#,
            true,
        ),
        (
            r#"{"predicted_effect": {"reward": "hold", "cost": "up"}}"#,
            true,
        ),
        // Bad values → error
        (
            r#"{"predicted_effect": {"reward": "sideways", "cost": "down"}}"#,
            false,
        ),
        (r#"{"predicted_effect": "bad"}"#, false),
    ];

    for (proposal_json, expect_ok) in cases {
        let escaped = py_escape(proposal_json);
        let py_snippet = format!(
            "import sys, json\nsys.path.insert(0,'runner')\nimport mutate\n\
             proposal = json.loads(\"{escaped}\")\n\
             try:\n    pe, d = mutate.normalize_predicted_effect(proposal)\n    print(json.dumps([pe, d]))\n\
             except (ValueError, TypeError):\n    print(json.dumps(None))"
        );
        let py_result = py_json(&root, &py_snippet);

        let proposal: Map<String, Value> = serde_json::from_str(proposal_json).unwrap();
        let rust_result = normalize_predicted_effect(&proposal);

        if *expect_ok {
            let py_arr = py_result.as_array().expect("[pe, defaulted]");
            let py_pe = &py_arr[0];
            let py_d = py_arr[1].as_bool().unwrap_or(false);
            let (rust_pe, rust_d) = rust_result
                .unwrap_or_else(|e| panic!("Rust should succeed on {proposal_json}: {e}"));
            assert_eq!(
                rust_d, py_d,
                "normalize_predicted_effect defaulted mismatch for {proposal_json}"
            );
            assert_eq!(
                py_pe,
                &Value::Object(rust_pe),
                "normalize_predicted_effect pe mismatch for {proposal_json}"
            );
        } else {
            assert_eq!(
                py_result,
                Value::Null,
                "Python should have errored for {proposal_json}"
            );
            assert!(rust_result.is_err(), "Rust should Err for {proposal_json}");
        }
    }
}

// ---------------------------------------------------------------------------
// resolve_donor parity
// ---------------------------------------------------------------------------

#[test]
fn parity_resolve_donor() {
    if !python_available() {
        eprintln!("skipping mutate parity: python3 not available");
        return;
    }
    let root = repo_root();

    let archive_json = json!({
        "seed2-cheap": {
            "id": "seed2-cheap",
            "kind": "pi",
            "model": "z-ai/glm-4.7-flash",
            "tools": ["read", "bash"],
            "prompt_packet_text": "A winning packet about tracing callers and citing evidence."
        }
    });

    let archive_str = py_escape(&serde_json::to_string(&archive_json).unwrap());

    let cases: &[(&str, bool)] = &[
        (
            r#"{"slot": "model", "donor": "seed2-cheap", "hypothesis": "h"}"#,
            true,
        ),
        (
            r#"{"slot": "prompt_packet", "donor": "seed2-cheap", "hypothesis": "h"}"#,
            true,
        ),
        (
            r#"{"slot": "tools", "donor": "seed2-cheap", "hypothesis": "h"}"#,
            true,
        ),
        (
            r#"{"slot": "model", "donor": "ghost", "hypothesis": "h"}"#,
            false,
        ),
    ];

    for (proposal_json, expect_ok) in cases {
        let prop_esc = py_escape(proposal_json);
        let py_snippet = format!(
            "import sys, json\nsys.path.insert(0,'runner')\nimport mutate\n\
             archive = json.loads(\"{archive_str}\")\n\
             proposal = json.loads(\"{prop_esc}\")\n\
             parent = {{}}\n\
             try:\n    result = mutate.resolve_donor(proposal, archive, parent)\n    print(json.dumps(result))\n\
             except ValueError:\n    print(json.dumps(None))"
        );
        let py_result = py_json(&root, &py_snippet);

        let mut archive: HashMap<String, Map<String, Value>> = HashMap::new();
        if let Some(obj) = archive_json.as_object() {
            for (k, v) in obj {
                if let Some(inner) = v.as_object() {
                    archive.insert(k.clone(), inner.clone());
                }
            }
        }
        let proposal: Map<String, Value> = serde_json::from_str(proposal_json).unwrap();
        let parent = Map::new();
        let rust_result = resolve_donor(&proposal, &archive, &parent);

        if *expect_ok {
            let rust_map = rust_result
                .unwrap_or_else(|e| panic!("Rust resolve_donor failed on {proposal_json}: {e}"));
            assert_eq!(
                py_result,
                Value::Object(rust_map),
                "resolve_donor mismatch for {proposal_json}"
            );
        } else {
            assert_eq!(
                py_result,
                Value::Null,
                "Python should have errored for {proposal_json}"
            );
            assert!(rust_result.is_err(), "Rust should Err for {proposal_json}");
        }
    }
}

// ---------------------------------------------------------------------------
// worst_trials parity
// ---------------------------------------------------------------------------

#[test]
fn parity_worst_trials() {
    if !python_available() {
        eprintln!("skipping mutate parity: python3 not available");
        return;
    }
    let root = repo_root();

    let records_json = json!([
        {"candidate_id": "x", "reward": 1.0, "wall_ms": 100, "run_id": "a"},
        {"candidate_id": "x", "reward": 0.0, "wall_ms": 200, "run_id": "b"},
        {"candidate_id": "y", "reward": 0.0, "wall_ms": 50, "run_id": "c"},
        {"candidate_id": "x", "reward": 0.5, "wall_ms": 300, "run_id": "d"},
        {"candidate_id": "x", "reward": 0.0, "wall_ms": 400, "run_id": "e"},
    ]);
    let records_esc = py_escape(&serde_json::to_string(&records_json).unwrap());

    for (cid, n) in [("x", 2usize), ("x", 3usize), ("y", 1usize), ("x", 10usize)] {
        let py_result = py_json(
            &root,
            &format!(
                "{PY_PRELUDE}\
                 records = json.loads(\"{records_esc}\"); \
                 result = mutate.worst_trials(records, \"{cid}\", n={n}); \
                 print(json.dumps(result))"
            ),
        );
        let records: Vec<Map<String, Value>> =
            serde_json::from_value(records_json.clone()).unwrap();
        let rust_result: Vec<Value> = worst_trials(&records, cid, n)
            .into_iter()
            .map(Value::Object)
            .collect();
        assert_eq!(
            py_result,
            Value::Array(rust_result),
            "worst_trials mismatch for cid={cid:?} n={n}"
        );
    }
}

// ---------------------------------------------------------------------------
// build_prompt parity
// ---------------------------------------------------------------------------

#[test]
fn parity_build_prompt() {
    if !python_available() {
        eprintln!("skipping mutate parity: python3 not available");
        return;
    }
    let root = repo_root();

    let taskspec = json!({"goal": "find bugs in the diff", "mode": "max-quality"});
    let parent_snapshot = json!({
        "id": "pi-kimi",
        "kind": "pi",
        "model": "moonshotai/kimi-k2.6",
        "thinking": "medium",
        "tools": ["read", "bash"],
        "prompt_packet_text": "review carefully and cite line numbers"
    });

    let cases: &[(&str, &str, &str, &str)] = &[
        (
            &serde_json::to_string(&taskspec).unwrap(),
            &serde_json::to_string(&parent_snapshot).unwrap(),
            "no evidence",
            "{}",
        ),
        (
            &serde_json::to_string(&taskspec).unwrap(),
            &serde_json::to_string(&parent_snapshot).unwrap(),
            "### Trial r1\nreward: 0.5",
            r#"{"gen1": {"model": "x/y"}}"#,
        ),
    ];

    for (ts_str, ps_str, evidence, archive_str) in cases {
        let ts_esc = py_escape(ts_str);
        let ps_esc = py_escape(ps_str);
        let ev_esc = py_escape(evidence);
        let ar_esc = py_escape(archive_str);
        let py_snippet = format!(
            "{PY_PRELUDE}\
             taskspec = json.loads(\"{ts_esc}\"); \
             parent_snapshot = json.loads(\"{ps_esc}\"); \
             archive_summary = json.loads(\"{ar_esc}\"); \
             result = mutate.build_prompt(taskspec, parent_snapshot, \
                 \"{ev_esc}\", archive_summary); \
             sys.stdout.write(result)"
        );
        let py_result = py_text(&root, &py_snippet);

        let ts: Map<String, Value> = serde_json::from_str(ts_str).unwrap();
        let ps: Map<String, Value> = serde_json::from_str(ps_str).unwrap();
        let archive: Value = serde_json::from_str(archive_str).unwrap();
        let rust_result = build_prompt(&ts, &ps, evidence, &archive, None);

        assert_eq!(
            py_result.trim_end_matches('\n'),
            rust_result,
            "build_prompt mismatch for evidence={evidence:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// validate_proposal parity
// ---------------------------------------------------------------------------

#[test]
fn parity_validate_proposal() {
    if !python_available() {
        eprintln!("skipping mutate parity: python3 not available");
        return;
    }
    let root = repo_root();

    let parent_esc = py_escape(&serde_json::to_string(&parent_manifest()).unwrap());
    let policies_str =
        r#"{"full": ["read", "bash", "edit", "write"], "explore": ["read", "bash"]}"#;
    let sets_str = r#"{"review-pack": ["packets/skill-a.md"], "bare": []}"#;
    let allowed_models_str = r#"["z-ai/glm-5", "moonshotai/kimi-k2.6"]"#;
    let allowed_thinking_str = r#"["off", "low", "medium"]"#;

    // Cases: (proposal JSON, tag for which kwargs to inject, expect_ok)
    // Tags: "tool_policies", "skill_sets", "allowed_models", "allowed_thinking", "avoid_thinking"
    let cases: &[(&str, &[&str], bool)] = &[
        // valid mutations
        (
            r#"{"slot": "thinking", "value": "high", "hypothesis": "h"}"#,
            &[],
            true,
        ),
        (
            r#"{"slot": "model", "value": "z-ai/glm-5", "hypothesis": "h"}"#,
            &["allowed_models"],
            true,
        ),
        (
            r#"{"slot": "system_prompt_mode", "value": "replace", "hypothesis": "h"}"#,
            &[],
            true,
        ),
        (
            r#"{"slot": "tools", "value": "explore", "hypothesis": "h"}"#,
            &["tool_policies"],
            true,
        ),
        (
            r#"{"slot": "agents_md", "value": "Workspace briefing: trace callers before judging.", "hypothesis": "h"}"#,
            &[],
            true,
        ),
        (
            r#"{"slot": "skills", "value": "review-pack", "hypothesis": "h"}"#,
            &["skill_sets"],
            true,
        ),
        // invalid: unknown/frozen slot
        (
            r#"{"slot": "temperature", "value": 0.5, "hypothesis": "h"}"#,
            &[],
            false,
        ),
        // invalid: no-op (same as parent)
        (
            r#"{"slot": "thinking", "value": "medium", "hypothesis": "h"}"#,
            &[],
            false,
        ),
        // invalid: missing hypothesis
        (
            r#"{"slot": "thinking", "value": "high", "hypothesis": "   "}"#,
            &[],
            false,
        ),
        // invalid: model not in search space
        (
            r#"{"slot": "model", "value": "bad/model", "hypothesis": "h"}"#,
            &["allowed_models"],
            false,
        ),
        // invalid: thinking not in declared space
        (
            r#"{"slot": "thinking", "value": "high", "hypothesis": "h"}"#,
            &["allowed_thinking"],
            false,
        ),
        // invalid: tools without policy map
        (
            r#"{"slot": "tools", "value": "explore", "hypothesis": "h"}"#,
            &[],
            false,
        ),
        // invalid: thin prompt_packet
        (
            r#"{"slot": "prompt_packet", "value": "be good", "hypothesis": "h"}"#,
            &[],
            false,
        ),
        // invalid: avoid_slots collision
        (
            r#"{"slot": "thinking", "value": "high", "hypothesis": "h"}"#,
            &["avoid_slots"],
            false,
        ),
    ];

    let policies_esc = py_escape(policies_str);
    let sets_esc = py_escape(sets_str);

    for (proposal_json, tags, expect_ok) in cases {
        let prop_esc = py_escape(proposal_json);

        // Build Python kwargs string
        let mut py_kwargs_parts: Vec<String> = vec![];
        if tags.contains(&"tool_policies") {
            py_kwargs_parts.push(format!("tool_policies=json.loads(\"{policies_esc}\")"));
        }
        if tags.contains(&"skill_sets") {
            py_kwargs_parts.push(format!("skill_sets=json.loads(\"{sets_esc}\")"));
        }
        if tags.contains(&"allowed_models") {
            py_kwargs_parts.push(format!("allowed_models={allowed_models_str}"));
        }
        if tags.contains(&"allowed_thinking") {
            py_kwargs_parts.push(format!("allowed_thinking={allowed_thinking_str}"));
        }
        if tags.contains(&"avoid_slots") {
            py_kwargs_parts.push("avoid_slots=(\"thinking\",)".to_string());
        }
        let py_kwargs = if py_kwargs_parts.is_empty() {
            String::new()
        } else {
            format!(", {}", py_kwargs_parts.join(", "))
        };

        let py_snippet = format!(
            "import sys, json\nsys.path.insert(0,'runner')\nimport mutate\n\
             parent = json.loads(\"{parent_esc}\")\n\
             proposal = json.loads(\"{prop_esc}\")\n\
             try:\n    slot, value, hyp = mutate.validate_proposal(proposal, parent{py_kwargs})\n    print(json.dumps([slot, value]))\n\
             except ValueError:\n    print(json.dumps(None))"
        );
        let py_result = py_json(&root, &py_snippet);

        let parent = parent_manifest();
        let proposal: Map<String, Value> = serde_json::from_str(proposal_json).unwrap();

        let mut tool_policies_opt: Option<HashMap<String, Vec<String>>> = None;
        let mut skill_sets_opt: Option<HashMap<String, Vec<String>>> = None;
        let mut allowed_models_opt: Option<Vec<String>> = None;
        let mut allowed_thinking_opt: Option<Vec<String>> = None;
        let mut avoid_slots_vec: Vec<String> = vec![];

        if tags.contains(&"tool_policies") {
            tool_policies_opt = Some(serde_json::from_str(policies_str).unwrap());
        }
        if tags.contains(&"skill_sets") {
            skill_sets_opt = Some(serde_json::from_str(sets_str).unwrap());
        }
        if tags.contains(&"allowed_models") {
            allowed_models_opt = Some(serde_json::from_str(allowed_models_str).unwrap());
        }
        if tags.contains(&"allowed_thinking") {
            allowed_thinking_opt = Some(serde_json::from_str(allowed_thinking_str).unwrap());
        }
        if tags.contains(&"avoid_slots") {
            avoid_slots_vec = vec!["thinking".to_string()];
        }

        let rust_result = validate_proposal(
            &proposal,
            &parent,
            tool_policies_opt.as_ref(),
            allowed_models_opt.as_deref(),
            allowed_thinking_opt.as_deref(),
            &avoid_slots_vec,
            skill_sets_opt.as_ref(),
            None,
        );

        if *expect_ok {
            assert!(
                py_result != Value::Null,
                "Python should have succeeded for proposal={proposal_json}"
            );
            let (rust_slot, rust_value, _) = rust_result.unwrap_or_else(|e| {
                panic!("Rust validate_proposal failed on {proposal_json}: {e}")
            });
            let py_arr = py_result.as_array().expect("[slot, value]");
            let py_slot = py_arr[0].as_str().unwrap_or("");
            assert_eq!(py_slot, rust_slot, "slot mismatch for {proposal_json}");
            assert_eq!(
                py_arr[1], rust_value,
                "value mismatch for proposal={proposal_json}"
            );
        } else {
            assert_eq!(
                py_result,
                Value::Null,
                "Python should have errored for {proposal_json}"
            );
            assert!(
                rust_result.is_err(),
                "Rust should Err for {proposal_json}, got: {rust_result:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// build_child parity (scalar and list slots)
// ---------------------------------------------------------------------------

#[test]
fn parity_build_child() {
    if !python_available() {
        eprintln!("skipping mutate parity: python3 not available");
        return;
    }
    let root = repo_root();

    let parent_esc = py_escape(&serde_json::to_string(&parent_manifest()).unwrap());
    let policies_esc =
        py_escape(r#"{"full": ["read", "bash", "edit", "write"], "explore": ["read", "bash"]}"#);

    // (slot, value JSON, Python extra kwargs for tool_policies, child_id)
    // We omit text slots (prompt_packet, agents_md) as their paths differ per tmp dir.
    let scalar_cases: &[(&str, &str, &str, &str)] = &[
        ("thinking", r#""high""#, "", "gen-think"),
        ("model", r#""z-ai/glm-5""#, "", "gen-model"),
        ("system_prompt_mode", r#""replace""#, "", "gen-spm"),
        // tools with policy name → resolves to list
        (
            "tools",
            r#""explore""#,
            &format!("tool_policies=json.loads(\"{policies_esc}\")"),
            "gen-tools",
        ),
    ];

    let tmp = tmp_dir("build-child");

    for (slot, value_json, extra_kwargs, child_id) in scalar_cases {
        let val_esc = py_escape(value_json);
        let py_kwargs = if extra_kwargs.is_empty() {
            String::new()
        } else {
            format!(", {extra_kwargs}")
        };
        // Python: build_child(parent, slot, value, child_id, packets_dir, ...)
        // We strip file-path slots from comparison since tmp paths differ.
        let py_snippet = format!(
            "{PY_PRELUDE}\
             import pathlib, tempfile; \
             parent = json.loads(\"{parent_esc}\"); \
             value = json.loads(\"{val_esc}\"); \
             tmp = pathlib.Path(tempfile.mkdtemp()); \
             child = mutate.build_child(parent, \"{slot}\", value, \"{child_id}\", tmp{py_kwargs}); \
             out = {{k: v for k, v in child.items() \
                 if not (k in ('prompt_packet', 'agents_md') and isinstance(v, str) and '/' in str(v))}}; \
             print(json.dumps(out))"
        );
        let py_result = py_json(&root, &py_snippet);

        let parent = parent_manifest();
        let value: Value = serde_json::from_str(value_json).unwrap();
        let tool_policies: Option<HashMap<String, Vec<String>>> =
            if extra_kwargs.contains("tool_policies") {
                Some(serde_json::from_str(
                    r#"{"full": ["read", "bash", "edit", "write"], "explore": ["read", "bash"]}"#,
                ).unwrap())
            } else {
                None
            };

        let rust_child = build_child(
            &parent,
            slot,
            &value,
            child_id,
            &tmp,
            tool_policies.as_ref(),
            None,
        )
        .unwrap_or_else(|e| panic!("build_child failed for slot={slot}: {e}"));

        // Filter out file-path slots (those written to disk — paths differ)
        let rust_filtered: Map<String, Value> = rust_child
            .into_iter()
            .filter(|(k, v)| {
                if k == "prompt_packet" || k == "agents_md" {
                    !v.as_str().map(|s| s.contains('/')).unwrap_or(false)
                } else {
                    true
                }
            })
            .collect();

        assert_eq!(
            py_result,
            Value::Object(rust_filtered),
            "build_child mismatch for slot={slot}"
        );
    }
}

// ---------------------------------------------------------------------------
// write_manifest parity
// ---------------------------------------------------------------------------

#[test]
fn parity_write_manifest() {
    if !python_available() {
        eprintln!("skipping mutate parity: python3 not available");
        return;
    }
    let root = repo_root();

    let manifests: &[Value] = &[
        json!({
            "id": "gen1",
            "kind": "pi",
            "model": "x/y",
            "thinking": "high",
            "tools": ["read", "bash"],
            "timeout_sec": 600,
            "flag": true
        }),
        json!({
            "id": "gen2",
            "enabled": false,
            "score": 0.75,
            "tools": []
        }),
        // null values: Python json.dumps(str(None)) → "\"None\""
        json!({
            "id": "gen3",
            "agents_md": null
        }),
    ];

    let tmp = tmp_dir("write-manifest");

    for (i, manifest_val) in manifests.iter().enumerate() {
        let manifest_obj = manifest_val.as_object().unwrap();
        let manifest_esc = py_escape(&serde_json::to_string(manifest_val).unwrap());

        let py_snippet = format!(
            "{PY_PRELUDE}\
             import pathlib, tempfile; \
             child = json.loads(\"{manifest_esc}\"); \
             tmp = pathlib.Path(tempfile.mkdtemp()); \
             path = mutate.write_manifest(child, tmp / 'out.toml'); \
             sys.stdout.write(path.read_text())"
        );
        let py_text_out = py_text(&root, &py_snippet);

        let out_path = tmp.join(format!("manifest-{i}.toml"));
        let rust_path = write_manifest(manifest_obj, &out_path)
            .unwrap_or_else(|e| panic!("write_manifest failed: {e}"));
        let rust_text = std::fs::read_to_string(&rust_path).unwrap();

        assert_eq!(
            py_text_out, rust_text,
            "write_manifest mismatch for manifest #{i}\npy: {py_text_out:?}\nrust: {rust_text:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Sanity-check: validate_proposal rejects bad prompt_packet via is_sane_prompt_packet
// ---------------------------------------------------------------------------

#[test]
fn parity_validate_rejects_bad_prompt_packet() {
    if !python_available() {
        eprintln!("skipping mutate parity: python3 not available");
        return;
    }
    let root = repo_root();

    let parent_esc = py_escape(&serde_json::to_string(&parent_manifest()).unwrap());

    let bad_values: &[String] = &[
        "be good".to_string(),                 // too short
        "a".repeat(5001),                      // too long
        "The".to_string() + &"!".repeat(5000), // degenerate
    ];

    for bad in bad_values {
        let proposal_json = serde_json::json!({
            "slot": "prompt_packet",
            "value": bad,
            "hypothesis": "try this packet"
        });
        let proposal_esc = py_escape(&serde_json::to_string(&proposal_json).unwrap());
        let py_snippet = format!(
            "import sys, json\nsys.path.insert(0,'runner')\nimport mutate\n\
             parent = json.loads(\"{parent_esc}\")\n\
             proposal = json.loads(\"{proposal_esc}\")\n\
             try:\n    mutate.validate_proposal(proposal, parent)\n    print('ok')\n\
             except ValueError:\n    print('error')"
        );
        let py_out = py_text(&root, &py_snippet);

        if py_out.trim() == "error" {
            let proposal: Map<String, Value> = serde_json::from_value(proposal_json).unwrap();
            let parent = parent_manifest();
            let rust_result =
                validate_proposal(&proposal, &parent, None, None, None, &[], None, None);
            assert!(
                rust_result.is_err(),
                "Rust should reject bad packet (len={})",
                bad.len()
            );
        }
    }
}
