//! Parity oracle for the `run` deterministic-core port.
//!
//! For each function, run BOTH the original Python `runner/run.py` and the
//! Rust port over identical inputs and assert the outputs agree:
//!   - hash strings compared as exact bytes (`tree_digest`, `_hash`)
//!   - `extract_pi_usage` fields compared semantically as `serde_json::Value`
//!   - `summarize` compared semantically as `serde_json::Value`
//!   - `build_pi_cmd` compared element-by-element
//!   - `validate_*` / `select_tasks` / `task_instruction` / `extract_json_object`
//!     compared by outcome (Ok/Err + content)
//!
//! Skips (does not fail) when python3 is unavailable, mirroring `bin/gate`.
//!
//! ## Parity gaps
//!
//! None known. The composition hash uses `pycompat::py_json_dumps` to match
//! Python's `json.dumps(sort_keys=True)` exactly (ensure_ascii, separators).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use daedalus_core::run::{
    build_pi_cmd, extract_json_object, extract_pi_usage, load_candidate, load_toml, select_tasks,
    summarize, task_instruction, tree_digest, validate_arena_for_local_execution,
};
use serde_json::{json, Value};

static COUNTER: AtomicU64 = AtomicU64::new(0);

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

fn tmpdir(suffix: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!(
        "daedalus-run-parity-{}-{n}-{suffix}",
        std::process::id()
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Run a Python snippet; `args` are passed as sys.argv[1..]. Returns stdout.
fn py_run(root: &Path, snippet: &str, args: &[&str]) -> String {
    let out = Command::new("python3")
        .current_dir(root)
        .arg("-c")
        .arg(snippet)
        .args(args)
        .output()
        .expect("run python3");
    assert!(
        out.status.success(),
        "python3 failed:\nstderr: {}\nstdout: {}\nsnippet: {snippet}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout),
    );
    String::from_utf8(out.stdout).expect("python3 output is utf-8")
}

/// Run Python and parse stdout as JSON.
fn py_eval_json(root: &Path, snippet: &str, args: &[&str]) -> Value {
    serde_json::from_str(&py_run(root, snippet, args)).expect("python3 did not emit valid JSON")
}

// ---------------------------------------------------------------------------
// Parity: tree_digest
// ---------------------------------------------------------------------------

#[test]
fn parity_tree_digest_matches_python() {
    if !python_available() {
        eprintln!("skipping tree_digest parity: python3 not available");
        return;
    }
    let root = repo_root();
    let dir = tmpdir("tree-digest");
    let sub = dir.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(dir.join("a.txt"), "hello").unwrap();
    std::fs::write(dir.join("b.txt"), "world").unwrap();
    std::fs::write(sub.join("c.txt"), "nested").unwrap();

    let rust_digest = tree_digest(&[&dir]);

    let snippet = "import sys, hashlib; from pathlib import Path
root = Path(sys.argv[1])
h = hashlib.sha256()
for f in sorted(root.rglob('*')):
    if f.is_file():
        h.update(str(f.relative_to(root)).encode())
        h.update(f.read_bytes())
print(h.hexdigest(), end='')";
    let py_digest = py_run(&root, snippet, &[&dir.to_string_lossy()]);

    assert_eq!(
        rust_digest, py_digest,
        "tree_digest: Rust={rust_digest} Python={py_digest}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn parity_tree_digest_multiple_roots() {
    if !python_available() {
        eprintln!("skipping multi-root tree_digest parity: python3 not available");
        return;
    }
    let root = repo_root();
    let dir = tmpdir("tree-multi");
    let r1 = dir.join("r1");
    let r2 = dir.join("r2");
    std::fs::create_dir_all(&r1).unwrap();
    std::fs::create_dir_all(&r2).unwrap();
    std::fs::write(r1.join("x.txt"), "aaa").unwrap();
    std::fs::write(r2.join("y.txt"), "bbb").unwrap();

    let rust_digest = tree_digest(&[&r1, &r2]);

    let snippet = "import sys, hashlib; from pathlib import Path
roots = sys.argv[1:]
h = hashlib.sha256()
for root in roots:
    root = Path(root)
    for f in sorted(root.rglob('*')):
        if f.is_file():
            h.update(str(f.relative_to(root)).encode())
            h.update(f.read_bytes())
print(h.hexdigest(), end='')";
    let py_digest = py_run(
        &root,
        snippet,
        &[&r1.to_string_lossy(), &r2.to_string_lossy()],
    );

    assert_eq!(
        rust_digest, py_digest,
        "multi-root tree_digest: Rust={rust_digest} Python={py_digest}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Parity: extract_json_object
// ---------------------------------------------------------------------------

#[test]
fn parity_extract_json_object() {
    if !python_available() {
        eprintln!("skipping extract_json_object parity: python3 not available");
        return;
    }
    let root = repo_root();
    let cases: &[(&str, bool)] = &[
        (r#"{"findings": []}"#, true),
        (
            r#"Sure!\n```json\n{"findings": [{"a": 1}]}\n```\nDone."#,
            true,
        ),
        (r#"{broken {"findings": []}"#, true),
        ("no json here", false),
        (r#"{"x": 1} and {"y": 2}"#, true),
        (r#"prefix {nested: {}} {"valid": true}"#, true),
    ];
    let snippet = "import sys, json
text = sys.argv[1]
start = text.find('{')
result = None
while start != -1:
    depth = 0
    for i in range(start, len(text)):
        if text[i] == '{':
            depth += 1
        elif text[i] == '}':
            depth -= 1
            if depth == 0:
                try:
                    result = json.loads(text[start:i+1])
                    break
                except json.JSONDecodeError:
                    break
    if result is not None:
        break
    start = text.find('{', start + 1)
if result is None:
    print('ERROR', end='')
else:
    print(json.dumps(result, sort_keys=True), end='')";
    for (text, expect_ok) in cases {
        let py_raw = py_run(&root, snippet, &[text]);
        let rust_result = extract_json_object(text);
        if *expect_ok {
            let rust_val = rust_result
                .unwrap_or_else(|e| panic!("expected Ok for {:?} but got Err: {e}", text));
            let py_val: Value = serde_json::from_str(&py_raw)
                .unwrap_or_else(|_| panic!("python returned non-JSON for {:?}: {py_raw}", text));
            assert_eq!(
                rust_val, py_val,
                "extract_json_object({text:?}): Rust={rust_val} Python={py_val}"
            );
        } else {
            assert!(
                rust_result.is_err(),
                "expected Err for {:?} but got Ok: {:?}",
                text,
                rust_result.unwrap()
            );
            assert_eq!(py_raw, "ERROR", "python expected error for {text:?}");
        }
    }
}

// ---------------------------------------------------------------------------
// Parity: extract_pi_usage
// ---------------------------------------------------------------------------

const PI_TRANSCRIPT: &str = r#"{"type":"session","version":3,"id":"x","timestamp":"t","cwd":"/tmp"}
{"type":"message_end","message":{"role":"user","content":[]}}
{"type":"message_end","message":{"role":"assistant","provider":"openrouter","usage":{"input":397,"output":26,"cacheRead":10,"cacheWrite":0,"totalTokens":423,"cost":{"input":0.0002,"output":0.00008,"total":0.00028}}}}
{"type":"message_end","message":{"role":"assistant","provider":"openrouter","usage":{"input":500,"output":40,"cacheRead":0,"cacheWrite":0,"totalTokens":540,"cost":{"total":0.0005}}}}
"#;

#[test]
fn parity_extract_pi_usage() {
    if !python_available() {
        eprintln!("skipping extract_pi_usage parity: python3 not available");
        return;
    }
    let root = repo_root();
    let dir = tmpdir("pi-usage");
    let transcript_file = dir.join("transcript.txt");

    // Case 1: standard transcript with two assistant message_ends
    std::fs::write(&transcript_file, PI_TRANSCRIPT).unwrap();
    let snippet = "import sys, json; sys.path.insert(0,'runner'); import run
text = open(sys.argv[1]).read()
print(json.dumps(run.extract_pi_usage(text)), end='')";
    let py_val = py_eval_json(&root, snippet, &[&transcript_file.to_string_lossy()]);
    let rust_map = extract_pi_usage(PI_TRANSCRIPT);
    let rust_val = Value::Object(rust_map);
    assert_eq!(
        rust_val, py_val,
        "extract_pi_usage: Rust={rust_val} Python={py_val}"
    );

    // Case 2: no events → empty dict
    std::fs::write(&transcript_file, "plain text\n{\"type\":\"other\"}").unwrap();
    let py_empty = py_eval_json(&root, snippet, &[&transcript_file.to_string_lossy()]);
    let rust_empty = extract_pi_usage("plain text\n{\"type\":\"other\"}");
    let rust_empty_val = Value::Object(rust_empty);
    assert_eq!(
        rust_empty_val, py_empty,
        "extract_pi_usage empty: Rust={rust_empty_val} Python={py_empty}"
    );

    // Case 3: cost accumulation across many lines
    let multi = r#"{"type":"message_end","message":{"role":"assistant","provider":"a","usage":{"input":100,"output":10,"cacheRead":5,"cost":{"total":0.001}}}}
{"type":"message_end","message":{"role":"assistant","provider":"b","usage":{"input":200,"output":20,"cacheRead":0,"cost":{"total":0.002}}}}
{"type":"message_end","message":{"role":"assistant","provider":"c","usage":{"input":300,"output":30,"cacheRead":15,"cost":{"total":0.003}}}}
"#;
    std::fs::write(&transcript_file, multi).unwrap();
    let py_multi = py_eval_json(&root, snippet, &[&transcript_file.to_string_lossy()]);
    let rust_multi = extract_pi_usage(multi);
    let rust_multi_val = Value::Object(rust_multi);
    assert_eq!(
        rust_multi_val, py_multi,
        "extract_pi_usage multi: Rust={rust_multi_val} Python={py_multi}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Parity: validate_arena_for_local_execution
// ---------------------------------------------------------------------------

#[test]
fn parity_validate_arena_for_local_execution() {
    if !python_available() {
        eprintln!("skipping validate_arena parity: python3 not available");
        return;
    }
    let root = repo_root();
    let snippet = "import sys, json; sys.path.insert(0,'runner'); import run
arena = json.loads(sys.argv[1])
try:
    run.validate_arena_for_local_execution(arena)
    print(json.dumps({'ok': True}), end='')
except run.LocalExecutionRefused as e:
    print(json.dumps({'ok': False, 'msg': str(e)}), end='')";

    let cases: &[(&str, bool)] = &[
        (r#"{"id": "x", "version": "0.1.0"}"#, false), // missing risk → refused
        (
            r#"{"risk": {"class": "sensitive", "needs_network": true}}"#,
            false,
        ),
        (r#"{"risk": {"class": "low"}}"#, true),
        (
            r#"{"risk": {"class": "low", "needs_network": true}}"#,
            false,
        ),
        (
            r#"{"risk": {"class": "low", "needs_secrets": false}}"#,
            true,
        ),
        (r#"{"risk": {}}"#, false), // empty risk: no class → refused
    ];

    for (arena_json, expect_ok) in cases {
        let arena: Value = serde_json::from_str(arena_json).unwrap();
        let py_result = py_eval_json(&root, snippet, &[arena_json]);
        let rust_result = validate_arena_for_local_execution(&arena);

        let py_ok = py_result["ok"].as_bool().unwrap();
        assert_eq!(
            py_ok, *expect_ok,
            "python result for arena {arena_json} unexpected"
        );

        match (rust_result, py_ok) {
            (Ok(()), true) => {} // both ok
            (Err(e), false) => {
                // Both refused; check key message fragments match
                let py_msg = py_result["msg"].as_str().unwrap_or("");
                // Both should mention "Harbor/Docker isolation"
                assert!(
                    e.to_string().contains("Harbor/Docker isolation"),
                    "rust error missing 'Harbor/Docker isolation': {e}"
                );
                assert!(
                    py_msg.contains("Harbor/Docker isolation"),
                    "python error missing 'Harbor/Docker isolation': {py_msg}"
                );
            }
            (Ok(()), false) => {
                panic!(
                    "Rust accepted arena {arena_json} but Python refused: {}",
                    py_result["msg"].as_str().unwrap_or("")
                );
            }
            (Err(e), true) => {
                panic!("Rust refused arena {arena_json} but Python accepted: {e}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Parity: select_tasks
// ---------------------------------------------------------------------------

fn write_arena_for_select(dir: &Path) -> Value {
    let tasks_dir = dir.join("tasks");
    for name in ["t-alpha", "t-beta", "t-gamma"] {
        std::fs::create_dir_all(tasks_dir.join(name)).unwrap();
    }
    let toml_content = "id = \"test\"\nversion = \"0.1.0\"\n\
[risk]\nclass = \"low\"\n\
[split]\ntrain = [\"t-alpha\", \"t-beta\"]\nvalidation = [\"t-gamma\"]\nholdout = []\n";
    std::fs::write(dir.join("arena.toml"), toml_content).unwrap();
    load_toml(&dir.join("arena.toml")).unwrap()
}

#[test]
fn parity_select_tasks() {
    if !python_available() {
        eprintln!("skipping select_tasks parity: python3 not available");
        return;
    }
    let root = repo_root();
    let dir = tmpdir("select-tasks");
    let arena = write_arena_for_select(&dir);

    let snippet = "import sys, json; from pathlib import Path
sys.path.insert(0,'runner'); import run
arena_dir = Path(sys.argv[1])
arena = run.load_toml(arena_dir / 'arena.toml')
split = sys.argv[2]
final_ = sys.argv[3] == 'true'
task_dirs = run.select_tasks(arena_dir, arena, split, None, final_)
print(json.dumps([d.name for d in task_dirs]), end='')";

    for (split, is_final) in [("train", false), ("validation", false), ("all", true)] {
        let py_val = py_eval_json(
            &root,
            snippet,
            &[
                &dir.to_string_lossy(),
                split,
                if is_final { "true" } else { "false" },
            ],
        );
        let py_names: Vec<String> = py_val
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .map(String::from)
            .collect();

        let rust_dirs = select_tasks(&dir, &arena, split, None, is_final).unwrap();
        let rust_names: Vec<String> = rust_dirs
            .iter()
            .filter_map(|p| p.file_name()?.to_str().map(String::from))
            .collect();

        assert_eq!(
            rust_names, py_names,
            "select_tasks(split={split}, final={is_final}): Rust={rust_names:?} Python={py_names:?}"
        );
    }

    // Unknown split → both error
    let py_bad = py_eval_json(
        &root,
        "import sys, json; from pathlib import Path
sys.path.insert(0,'runner'); import run
arena_dir = Path(sys.argv[1])
arena = run.load_toml(arena_dir / 'arena.toml')
try:
    run.select_tasks(arena_dir, arena, 'nonsense', None, False)
    print(json.dumps({'ok': True}), end='')
except SystemExit as e:
    print(json.dumps({'ok': False, 'msg': str(e)}), end='')",
        &[&dir.to_string_lossy()],
    );
    assert!(!py_bad["ok"].as_bool().unwrap());
    let rust_bad = select_tasks(&dir, &arena, "nonsense", None, false);
    assert!(
        rust_bad.is_err(),
        "Rust should reject unknown split 'nonsense'"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Parity: task_instruction
// ---------------------------------------------------------------------------

#[test]
fn parity_task_instruction() {
    if !python_available() {
        eprintln!("skipping task_instruction parity: python3 not available");
        return;
    }
    let root = repo_root();

    // Case 1: real arena with template (pr-review-v0)
    let arena_dir = root.join("arenas").join("pr-review-v0");
    if arena_dir.exists() {
        let arena = load_toml(&arena_dir.join("arena.toml")).unwrap();
        let task_dir = arena_dir.join("tasks").join("py-pagination");
        if task_dir.exists() {
            let snippet = "import sys; from pathlib import Path
sys.path.insert(0,'runner'); import run
arena_dir = Path(sys.argv[1])
arena = run.load_toml(arena_dir / 'arena.toml')
text = run.task_instruction(arena_dir, arena, arena_dir / 'tasks' / 'py-pagination')
sys.stdout.write(text)";
            let py_text = py_run(&root, snippet, &[&arena_dir.to_string_lossy()]);
            let rust_text = task_instruction(&arena_dir, &arena, &task_dir).unwrap();
            assert_eq!(
                rust_text, py_text,
                "task_instruction(pr-review-v0/py-pagination) differs"
            );
        }
    }

    // Case 2: fallback to instruction.md
    let dir = tmpdir("task-instr");
    let task_dir = dir.join("tasks").join("t1");
    std::fs::create_dir_all(&task_dir).unwrap();
    std::fs::write(task_dir.join("instruction.md"), "Direct instruction body.").unwrap();
    let arena = json!({"id": "x", "version": "0.1.0"});
    let rust_fallback = task_instruction(&dir, &arena, &task_dir).unwrap();
    assert_eq!(rust_fallback, "Direct instruction body.");

    // Case 3: template with {intent} substitution
    let dir2 = tmpdir("task-instr2");
    let task_dir2 = dir2.join("tasks").join("t2");
    std::fs::create_dir_all(&task_dir2).unwrap();
    std::fs::write(task_dir2.join("intent.md"), "  find the race condition  ").unwrap();
    std::fs::write(dir2.join("template.md"), "Review:\n{intent}\nEnd.").unwrap();
    let arena2 = json!({"template": {"file": "template.md"}});
    let py_text2 = py_run(
        &root,
        "import sys; from pathlib import Path
sys.path.insert(0,'runner'); import run
arena_dir = Path(sys.argv[1])
task_dir = Path(sys.argv[2])
arena = {'template': {'file': 'template.md'}}
sys.stdout.write(run.task_instruction(arena_dir, arena, task_dir))",
        &[&dir2.to_string_lossy(), &task_dir2.to_string_lossy()],
    );
    let rust_text2 = task_instruction(&dir2, &arena2, &task_dir2).unwrap();
    assert_eq!(
        rust_text2, py_text2,
        "task_instruction template: Rust={rust_text2:?} Python={py_text2:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir2);
}

// ---------------------------------------------------------------------------
// Parity: build_pi_cmd
// ---------------------------------------------------------------------------

#[test]
fn parity_build_pi_cmd() {
    if !python_available() {
        eprintln!("skipping build_pi_cmd parity: python3 not available");
        return;
    }
    let root = repo_root();
    let dir = tmpdir("pi-cmd");

    // Case 1: default isolation (no skills, no agents_md, append mode)
    let packet1 = dir.join("p1.md");
    std::fs::write(&packet1, "Review carefully and thoroughly.").unwrap();
    let manifest1 = dir.join("c1.toml");
    std::fs::write(
        &manifest1,
        format!(
            "id = \"x\"\nkind = \"pi\"\nmodel = \"m\"\nprompt_packet = \"{}\"\n",
            packet1.display()
        ),
    )
    .unwrap();

    let snippet = "import sys, json; from pathlib import Path
sys.path.insert(0,'runner'); import run
cand = run.load_candidate(Path(sys.argv[1]))
print(json.dumps(run.build_pi_cmd(cand)), end='')";

    let py_cmd1 = py_eval_json(&root, snippet, &[&manifest1.to_string_lossy()]);
    let py_cmd1: Vec<String> = py_cmd1
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .map(String::from)
        .collect();
    let rust_cand1 = load_candidate(&manifest1, &root).unwrap();
    let rust_cmd1 = build_pi_cmd(&rust_cand1);
    assert_eq!(
        rust_cmd1, py_cmd1,
        "build_pi_cmd (default): Rust={rust_cmd1:?} Python={py_cmd1:?}"
    );

    // Case 2: skills + agents_md + replace mode
    let packet2 = dir.join("p2.md");
    std::fs::write(&packet2, "You are the whole system prompt.").unwrap();
    let skill2 = dir.join("skill.md");
    std::fs::write(&skill2, "# a pi skill").unwrap();
    let agents2 = dir.join("agents.md");
    std::fs::write(
        &agents2,
        "Repo briefing: run bin/gate before claiming done.",
    )
    .unwrap();
    let manifest2 = dir.join("c2.toml");
    std::fs::write(
        &manifest2,
        format!(
            "id = \"x\"\nkind = \"pi\"\nmodel = \"m\"\n\
prompt_packet = \"{packet}\"\nsystem_prompt_mode = \"replace\"\n\
skills = [\"{skill}\"]\nagents_md = \"{agents}\"\n",
            packet = packet2.display(),
            skill = skill2.display(),
            agents = agents2.display(),
        ),
    )
    .unwrap();

    let py_cmd2 = py_eval_json(&root, snippet, &[&manifest2.to_string_lossy()]);
    let py_cmd2: Vec<String> = py_cmd2
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .map(String::from)
        .collect();
    let rust_cand2 = load_candidate(&manifest2, &root).unwrap();
    let rust_cmd2 = build_pi_cmd(&rust_cand2);
    assert_eq!(
        rust_cmd2, py_cmd2,
        "build_pi_cmd (skills+agents+replace): Rust={rust_cmd2:?} Python={py_cmd2:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Parity: load_candidate composition hash — against a REAL candidate TOML
// ---------------------------------------------------------------------------

#[test]
fn parity_composition_hash_on_real_candidate() {
    if !python_available() {
        eprintln!("skipping composition hash parity: python3 not available");
        return;
    }
    let root = repo_root();

    // pi-kimi.toml is the most complex real candidate (has prompt_packet +
    // composition key). oracle.toml is minimal.
    let candidates = [
        "oracle.toml",
        "null.toml",
        "probe-oneshot.toml",
        "pi-kimi.toml",
    ];
    let snippet = "import sys, json; from pathlib import Path
sys.path.insert(0,'runner'); import run
cand = run.load_candidate(Path(sys.argv[1]))
print(cand['_hash'], end='')";

    for name in &candidates {
        let path = root.join("candidates").join(name);
        if !path.exists() {
            continue;
        }
        let py_hash = py_run(&root, snippet, &[&path.to_string_lossy()]);
        let rust_cand = load_candidate(&path, &root)
            .unwrap_or_else(|e| panic!("load_candidate failed for {name}: {e}"));
        let rust_hash = rust_cand["_hash"].as_str().unwrap();
        assert_eq!(
            rust_hash, py_hash,
            "composition hash mismatch for {name}: Rust={rust_hash} Python={py_hash}"
        );
    }
}

// ---------------------------------------------------------------------------
// Parity: summarize — against a REAL trials.jsonl
// ---------------------------------------------------------------------------

fn write_trials(records: &[Value], dir: &Path) -> PathBuf {
    let path = dir.join("trials.jsonl");
    let body: String = records
        .iter()
        .map(|r| serde_json::to_string(r).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    std::fs::write(&path, body).unwrap();
    path
}

fn rec(cand: &str, task: &str, reward: f64, cost: Option<f64>, wall_ms: f64) -> Value {
    json!({
        "candidate_id": cand,
        "candidate_kind": "pi",
        "composition_hash": format!("hash-{cand}"),
        "task_id": task,
        "reward": reward,
        "cost_usd": cost,
        "wall_ms": wall_ms,
        "error": null,
    })
}

fn rec_err(cand: &str, task: &str, reward: f64, cost: Option<f64>, wall_ms: f64) -> Value {
    json!({
        "candidate_id": cand,
        "candidate_kind": "pi",
        "composition_hash": format!("hash-{cand}"),
        "task_id": task,
        "reward": reward,
        "cost_usd": cost,
        "wall_ms": wall_ms,
        "error": "boom",
    })
}

#[test]
fn parity_summarize_synthetic() {
    if !python_available() {
        eprintln!("skipping summarize parity: python3 not available");
        return;
    }
    let root = repo_root();
    let dir = tmpdir("summarize");

    let cases: &[(&str, Vec<Value>)] = &[
        (
            "single-candidate",
            vec![
                rec("a", "t1", 1.0, Some(0.01), 1000.0),
                rec("a", "t2", 0.5, Some(0.02), 2000.0),
            ],
        ),
        (
            "multi-candidate",
            vec![
                rec("a", "t1", 1.0, Some(0.01), 1000.0),
                rec("b", "t1", 0.5, Some(0.02), 2000.0),
                rec("a", "t1", 0.8, Some(0.01), 1500.0),
            ],
        ),
        (
            "with-errors",
            vec![
                rec("c", "t1", 1.0, Some(0.01), 1000.0),
                rec_err("c", "t1", 0.0, Some(0.01), 500.0),
            ],
        ),
        (
            "unknown-cost",
            vec![
                rec("d", "t1", 0.8, None, 1000.0),
                rec("d", "t1", 0.6, Some(0.01), 1200.0),
            ],
        ),
        (
            "oracle-null",
            vec![
                json!({"candidate_id": "oracle", "candidate_kind": "oracle",
                       "composition_hash": "abc", "task_id": "t1", "reward": 1.0,
                       "cost_usd": null, "wall_ms": 100.0, "error": null}),
                json!({"candidate_id": "null", "candidate_kind": "null",
                       "composition_hash": "def", "task_id": "t1", "reward": 0.0,
                       "cost_usd": null, "wall_ms": 50.0, "error": null}),
            ],
        ),
    ];

    let snippet = "import sys, json; from pathlib import Path
sys.path.insert(0,'runner'); import run
path = Path(sys.argv[1])
result = run.summarize(path)
print(json.dumps(result), end='')";

    for (label, records) in cases {
        let case_dir = dir.join(label);
        std::fs::create_dir_all(&case_dir).unwrap();
        let path = write_trials(records, &case_dir);

        let py_val = py_eval_json(&root, snippet, &[&path.to_string_lossy()]);
        let rust_map =
            summarize(&path).unwrap_or_else(|e| panic!("summarize failed for {label}: {e}"));
        let rust_val = Value::Object(rust_map);

        // Compare the summary semantically
        assert_eq!(
            rust_val, py_val,
            "[{label}] summarize differs:\nRust={rust_val}\nPython={py_val}"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn parity_summarize_on_real_trials() {
    if !python_available() {
        return;
    }
    let root = repo_root();
    // Find any real trials.jsonl to parity-test against
    let runs_dir = root.join("runs");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&runs_dir) {
        for entry in rd.flatten() {
            let p = entry.path().join("trials.jsonl");
            if p.exists() {
                candidates.push(p);
                if candidates.len() >= 2 {
                    break;
                }
            }
        }
    }
    if candidates.is_empty() {
        eprintln!("skipping real-trials parity: no runs/*/trials.jsonl present");
        return;
    }

    let snippet = "import sys, json; from pathlib import Path
sys.path.insert(0,'runner'); import run
path = Path(sys.argv[1])
result = run.summarize(path)
print(json.dumps(result), end='')";

    for path in &candidates {
        let py_val = py_eval_json(&root, snippet, &[&path.to_string_lossy()]);
        let rust_map = summarize(path)
            .unwrap_or_else(|e| panic!("summarize failed for {}: {e}", path.display()));
        let rust_val = Value::Object(rust_map);
        assert_eq!(
            rust_val,
            py_val,
            "real summarize differs for {}:\nRust={rust_val}\nPython={py_val}",
            path.display()
        );
    }
}

// ---------------------------------------------------------------------------
// Parity: validate_no_hidden_absolute_paths
// ---------------------------------------------------------------------------

#[test]
fn parity_validate_no_hidden_absolute_paths() {
    if !python_available() {
        eprintln!("skipping validate_no_hidden_absolute_paths parity: python3 not available");
        return;
    }
    let root = repo_root();
    let dir = tmpdir("hidden-paths");
    let task_dir = dir.join("task");
    let tests_dir = task_dir.join("tests");
    let solution_dir = task_dir.join("solution");
    let env_dir = task_dir.join("environment");
    std::fs::create_dir_all(&tests_dir).unwrap();
    std::fs::create_dir_all(&solution_dir).unwrap();
    std::fs::create_dir_all(&env_dir).unwrap();
    std::fs::write(tests_dir.join("expected.json"), "{\"defects\": []}").unwrap();
    std::fs::write(solution_dir.join("findings.json"), "{\"findings\": []}").unwrap();
    std::fs::write(env_dir.join("PR.diff"), "diff content").unwrap();

    // Case 1: safe instruction — should pass
    let safe_instruction = "Review the diff in the workspace.";
    let snippet_ok = "import sys, json; from pathlib import Path
sys.path.insert(0,'runner'); import run
task_dir = Path(sys.argv[1])
env_dir = task_dir / 'environment'
candidate = {}
try:
    run.validate_no_hidden_absolute_paths(candidate, task_dir, sys.argv[2], env_dir)
    print('ok', end='')
except run.GraderPathLeak as e:
    print('leak:' + str(e), end='')";
    let py_ok = py_run(
        &root,
        snippet_ok,
        &[&task_dir.to_string_lossy(), safe_instruction],
    );
    assert_eq!(py_ok, "ok", "python should accept safe instruction");
    let candidate = serde_json::Map::new();
    let result_ok = daedalus_core::run::validate_no_hidden_absolute_paths(
        &candidate,
        &task_dir,
        safe_instruction,
        &env_dir,
    );
    assert!(result_ok.is_ok(), "rust should accept safe instruction");

    // Case 2: leaky instruction — contains absolute path to tests/
    let leaked = tests_dir
        .canonicalize()
        .unwrap_or_else(|_| tests_dir.clone());
    let leaky_instruction = format!("Read {} before writing findings.", leaked.display());
    let py_leak = py_run(
        &root,
        snippet_ok,
        &[&task_dir.to_string_lossy(), &leaky_instruction],
    );
    assert!(
        py_leak.starts_with("leak:"),
        "python should detect path leak: {py_leak}"
    );
    let result_leak = daedalus_core::run::validate_no_hidden_absolute_paths(
        &candidate,
        &task_dir,
        &leaky_instruction,
        &env_dir,
    );
    assert!(
        result_leak.is_err(),
        "rust should detect path leak for {leaky_instruction:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
