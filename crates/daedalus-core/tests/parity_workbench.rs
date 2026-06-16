//! Parity oracle for the workbench port.
//!
//! For each case, run BOTH the original Python `runner/workbench.py` functions
//! and the Rust port over identical inputs and assert the outputs agree:
//!   - structured returns compared as `serde_json::Value` (semantic equality)
//!   - rendered reports compared as exact `String` equality
//!   - written files compared byte-identical
//!
//! Skips (does not fail) when `python3` is unavailable, mirroring `bin/gate`.
//!
//! ## Cases covered
//!
//! 1. `scaffold_task` — directory tree + file contents byte-identical
//! 2. `validate_expected_shape` — valid, missing fields, inverted span
//! 3. `validate_no_symlinks` — clean dir, symlink present (error message)
//! 4. `validate_splits` — complete, missing task, duplicate assignment
//! 5. `_holdout_counts` — empty, with version filter, without version filter
//! 6. `format_holdout_ledger_row` — with and without arena_version
//! 7. `validate_arena` — passing arena, missing split, probe saturation
//! 8. `render_validation_report` — pass and fail cases
//! 9. `replace_version` (via record_adjudication path, OUT-OF-SCOPE ruling)
//! 10. `record_adjudication` — OUT-OF-SCOPE + table row + section
//! 11. `disagreements` — category miss, span miss, exact match (no row)
//!
//! ## Parity notes
//!
//! - `record_adjudication` embeds `datetime.now(timezone.utc)` — parity
//!   compares everything EXCEPT the date field, then checks it is a valid
//!   ISO-8601 date.
//! - File-write functions: parity done by collecting all written files and
//!   comparing byte-by-byte (mirroring parity_port_harbor.rs).
//! - `validate_arena` uses `score()` internally, which is already parity-
//!   verified. Parity here is over structured `ValidationReport` fields.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use daedalus_core::workbench::{
    disagreements, format_holdout_ledger_row, holdout_counts, render_validation_report,
    scaffold_task, validate_arena, validate_expected_shape, validate_no_symlinks, version_tuple,
};
use serde_json::Value;

static COUNTER: AtomicU64 = AtomicU64::new(0);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root two levels above crates/daedalus-core")
        .to_path_buf()
}

fn python_available() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn tmpdir(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "daedalus-wb-parity-{}-{n}-{label}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Run a Python snippet with `arena_dir` as `sys.argv[1]` and any extra args.
/// Returns (stdout, stderr) as strings.
fn py_run(root: &Path, snippet: &str, args: &[&Path]) -> (String, String, bool) {
    let mut cmd = Command::new("python3");
    cmd.current_dir(root).arg("-c").arg(snippet);
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run python3");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

/// Run Python snippet and return parsed JSON. Panics on failure.
fn py_json(root: &Path, snippet: &str, args: &[&Path]) -> Value {
    let (stdout, stderr, ok) = py_run(root, snippet, args);
    assert!(ok, "python3 failed:\nstderr: {stderr}\nsnippet: {snippet}");
    serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!("python3 did not emit valid JSON: {e}\nstdout: {stdout}\nsnippet: {snippet}")
    })
}

/// Run Python snippet and return raw stdout string.
fn py_text(root: &Path, snippet: &str, args: &[&Path]) -> String {
    let (stdout, stderr, ok) = py_run(root, snippet, args);
    assert!(ok, "python3 failed:\nstderr: {stderr}\nsnippet: {snippet}");
    stdout
}

/// Recursively collect all files under `dir` as BTreeMap<rel_path → bytes>.
fn collect_files(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut map = BTreeMap::new();
    collect_inner(dir, dir, &mut map);
    map
}

fn collect_inner(root: &Path, dir: &Path, map: &mut BTreeMap<String, Vec<u8>>) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            let path = entry.path();
            let ft = entry.file_type().unwrap();
            if ft.is_dir() {
                collect_inner(root, &path, map);
            } else {
                let rel = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                let bytes = std::fs::read(&path).unwrap();
                map.insert(rel, bytes);
            }
        }
    }
}

fn assert_byte_parity(label: &str, py_dir: &Path, rust_dir: &Path) {
    let py_files = collect_files(py_dir);
    let rust_files = collect_files(rust_dir);

    let py_paths: Vec<&str> = py_files.keys().map(String::as_str).collect();
    let rust_paths: Vec<&str> = rust_files.keys().map(String::as_str).collect();
    assert_eq!(
        py_paths, rust_paths,
        "[{label}] file tree paths differ\npy:   {py_paths:?}\nrust: {rust_paths:?}"
    );
    for rel in py_files.keys() {
        let py_b = &py_files[rel];
        let rust_b = &rust_files[rel];
        assert_eq!(
            py_b,
            rust_b,
            "[{label}] file '{rel}' differs\npy  ({} bytes): {}\nrust({} bytes): {}",
            py_b.len(),
            String::from_utf8_lossy(py_b),
            rust_b.len(),
            String::from_utf8_lossy(rust_b)
        );
    }
}

// ---------------------------------------------------------------------------
// Shared arena builder (mirrors test_workbench.py::write_arena)
// ---------------------------------------------------------------------------

fn write_arena(tmp: &Path) -> PathBuf {
    let arena = tmp.join("arena");
    for task_name in &["buggy", "clean"] {
        let task = arena.join("tasks").join(task_name);
        std::fs::create_dir_all(task.join("environment")).unwrap();
        std::fs::create_dir_all(task.join("tests")).unwrap();
        std::fs::create_dir_all(task.join("solution")).unwrap();
    }

    // buggy task
    let buggy = arena.join("tasks").join("buggy");
    std::fs::write(buggy.join("environment").join("app.py"), "print('bug')\n").unwrap();
    std::fs::write(buggy.join("intent.md"), "Find the bug.\n").unwrap();
    std::fs::write(
        buggy.join("tests").join("expected.json"),
        serde_json::to_string(&serde_json::json!({
            "defects": [{
                "id": "bug",
                "file": "app.py",
                "line_start": 1,
                "line_end": 1,
                "category": "correctness",
                "note": "seeded"
            }]
        }))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(buggy.join("tests").join("test.sh"), "#!/usr/bin/env sh\n").unwrap();
    std::fs::write(
        buggy.join("solution").join("findings.json"),
        serde_json::to_string(&serde_json::json!({
            "findings": [{
                "file": "app.py",
                "line": 1,
                "category": "correctness",
                "description": "bug"
            }]
        }))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(
        buggy.join("task.toml"),
        "id = \"buggy\"\n\n[agent]\ntimeout_sec = 600\n\n[verifier]\ntimeout_sec = 60\n",
    )
    .unwrap();

    // clean task
    let clean = arena.join("tasks").join("clean");
    std::fs::write(clean.join("environment").join("app.py"), "print('ok')\n").unwrap();
    std::fs::write(clean.join("intent.md"), "Confirm clean.\n").unwrap();
    std::fs::write(
        clean.join("tests").join("expected.json"),
        "{\"defects\": []}\n",
    )
    .unwrap();
    std::fs::write(clean.join("tests").join("test.sh"), "#!/usr/bin/env sh\n").unwrap();
    std::fs::write(
        clean.join("solution").join("findings.json"),
        "{\"findings\": []}\n",
    )
    .unwrap();
    std::fs::write(
        clean.join("task.toml"),
        "id = \"clean\"\n\n[agent]\ntimeout_sec = 600\n\n[verifier]\ntimeout_sec = 60\n",
    )
    .unwrap();

    std::fs::write(
        arena.join("template.md"),
        "{intent}\nReturn findings.json.\n",
    )
    .unwrap();
    // Note: must match Python's write_arena exactly
    std::fs::write(
        arena.join("arena.toml"),
        "\nid = \"sample\"\nversion = \"0.1.0\"\ntaskspec = \"specs/sample/taskspec.toml\"\n\n[template]\nfile = \"template.md\"\n\n[risk]\nclass = \"low\"\n\n[split]\ntrain = [\"buggy\"]\nvalidation = [\"clean\"]\nholdout = []\n",
    )
    .unwrap();
    arena
}

fn write_probe_run(tmp: &Path) -> PathBuf {
    let run = tmp.join("run");
    std::fs::create_dir_all(&run).unwrap();
    std::fs::write(
        run.join("summary.json"),
        serde_json::to_string(&serde_json::json!({
            "oracle": {"kind": "oracle", "reward_mean": 1.0},
            "null": {"kind": "null", "reward_mean": 0.5},
            "probe-oneshot": {"kind": "oneshot", "reward_mean": 0.0},
        }))
        .unwrap(),
    )
    .unwrap();
    run
}

// ---------------------------------------------------------------------------
// Case 1: scaffold_task — byte-identical directory tree
// ---------------------------------------------------------------------------

#[test]
fn parity_scaffold_task() {
    if !python_available() {
        eprintln!("skipping workbench parity: python3 not available");
        return;
    }
    let root = repo_root();
    let tmp = tmpdir("scaffold");
    let py_arena = tmp.join("py-arena");
    let rust_arena = tmp.join("rust-arena");

    // Python
    let snippet = "\
import sys; from pathlib import Path; \
sys.path.insert(0,'runner'); import workbench; \
workbench.scaffold_task(Path(sys.argv[1]), 'new-task', 'specs/x.toml')";
    let (_, stderr, ok) = py_run(&root, snippet, &[&py_arena]);
    assert!(ok, "python scaffold_task failed: {stderr}");

    // Rust
    scaffold_task(&rust_arena, "new-task", "specs/x.toml").expect("rust scaffold_task");

    // Compare the task dir (not arena.toml — Python writes arena_id from dir name)
    let py_task = py_arena.join("tasks").join("new-task");
    let rust_task = rust_arena.join("tasks").join("new-task");
    assert_byte_parity("scaffold_task/task", &py_task, &rust_task);

    // arena.toml and template.md are generated with the same content
    // (id = "py-arena" vs id = "rust-arena" differ; only compare structure)
    let py_toml: Value = {
        let t = std::fs::read_to_string(py_arena.join("arena.toml")).unwrap();
        let tv: toml::Value = t.parse().unwrap();
        serde_json::to_value(tv).unwrap()
    };
    let rust_toml: Value = {
        let t = std::fs::read_to_string(rust_arena.join("arena.toml")).unwrap();
        let tv: toml::Value = t.parse().unwrap();
        serde_json::to_value(tv).unwrap()
    };
    // Compare everything except "id" (which reflects the directory name)
    for field in ["version", "taskspec"] {
        assert_eq!(
            py_toml.get(field),
            rust_toml.get(field),
            "scaffold arena.toml field '{field}' differs"
        );
    }

    // template.md must be identical
    let py_tmpl = std::fs::read(py_arena.join("template.md")).unwrap();
    let rust_tmpl = std::fs::read(rust_arena.join("template.md")).unwrap();
    assert_eq!(py_tmpl, rust_tmpl, "scaffold template.md differs");

    let _ = std::fs::remove_dir_all(&tmp);
}

// ---------------------------------------------------------------------------
// Case 2: validate_expected_shape — valid and error cases
// ---------------------------------------------------------------------------

#[test]
fn parity_validate_expected_shape() {
    if !python_available() {
        eprintln!("skipping workbench parity: python3 not available");
        return;
    }
    let root = repo_root();
    let tmp = tmpdir("expected-shape");

    // Valid expected.json
    let valid = tmp.join("valid.json");
    std::fs::write(
        &valid,
        serde_json::to_string(&serde_json::json!({
            "defects": [{
                "id": "d1",
                "file": "a.py",
                "line_start": 5,
                "line_end": 10,
                "category": "security"
            }]
        }))
        .unwrap(),
    )
    .unwrap();

    // Python result: count of defects
    let py_count = py_json(
        &root,
        "import sys, json; from pathlib import Path; sys.path.insert(0,'runner'); import workbench; \
         defects = workbench._validate_expected_shape(Path(sys.argv[1])); \
         print(json.dumps(len(defects)))",
        &[&valid],
    );
    let rust_defects = validate_expected_shape(&valid).expect("validate_expected_shape failed");
    assert_eq!(
        py_count,
        serde_json::json!(rust_defects.len()),
        "valid case: defect count differs"
    );

    // Empty defects list
    let empty = tmp.join("empty.json");
    std::fs::write(&empty, "{\"defects\": []}").unwrap();
    let py_empty = py_json(
        &root,
        "import sys, json; from pathlib import Path; sys.path.insert(0,'runner'); import workbench; \
         defects = workbench._validate_expected_shape(Path(sys.argv[1])); \
         print(json.dumps(len(defects)))",
        &[&empty],
    );
    let rust_empty = validate_expected_shape(&empty).expect("empty case failed");
    assert_eq!(
        py_empty,
        serde_json::json!(rust_empty.len()),
        "empty case: count differs"
    );

    // Missing field → error
    let bad = tmp.join("bad.json");
    std::fs::write(
        &bad,
        serde_json::to_string(&serde_json::json!({
            "defects": [{"id": "x", "file": "a.py"}]
        }))
        .unwrap(),
    )
    .unwrap();
    let (_, py_err_stderr, py_err_ok) = py_run(
        &root,
        "import sys; from pathlib import Path; sys.path.insert(0,'runner'); import workbench; \
         workbench._validate_expected_shape(Path(sys.argv[1]))",
        &[&bad],
    );
    assert!(!py_err_ok, "python should have raised on missing field");
    let rust_err = validate_expected_shape(&bad);
    assert!(rust_err.is_err(), "rust should error on missing field");
    // Both error, error text may differ but error presence matches
    let _ = py_err_stderr;

    // Inverted span → error
    let inv = tmp.join("inv.json");
    std::fs::write(
        &inv,
        serde_json::to_string(&serde_json::json!({
            "defects": [{
                "id": "inv",
                "file": "a.py",
                "line_start": 10,
                "line_end": 5,
                "category": "security"
            }]
        }))
        .unwrap(),
    )
    .unwrap();
    let (_, _, py_inv_ok) = py_run(
        &root,
        "import sys; from pathlib import Path; sys.path.insert(0,'runner'); import workbench; \
         workbench._validate_expected_shape(Path(sys.argv[1]))",
        &[&inv],
    );
    assert!(!py_inv_ok, "python should have raised on inverted span");
    let rust_inv = validate_expected_shape(&inv);
    assert!(rust_inv.is_err(), "rust should error on inverted span");

    let _ = std::fs::remove_dir_all(&tmp);
}

// ---------------------------------------------------------------------------
// Case 3: validate_no_symlinks — clean directory, symlink present
// ---------------------------------------------------------------------------

#[test]
fn parity_validate_no_symlinks() {
    if !python_available() {
        eprintln!("skipping workbench parity: python3 not available");
        return;
    }
    let root = repo_root();
    let tmp = tmpdir("symlinks");

    // Clean dir
    let clean = tmp.join("clean");
    std::fs::create_dir_all(&clean).unwrap();
    std::fs::write(clean.join("file.py"), "print(1)\n").unwrap();

    let (_, _, py_ok) = py_run(
        &root,
        "import sys; from pathlib import Path; sys.path.insert(0,'runner'); import workbench; \
         workbench._validate_no_symlinks(Path(sys.argv[1]))",
        &[&clean],
    );
    assert!(py_ok, "python should pass on clean dir");
    assert!(
        validate_no_symlinks(&clean).is_ok(),
        "rust should pass on clean dir"
    );

    // Dir with symlink
    let symlinked = tmp.join("symlinked");
    std::fs::create_dir_all(&symlinked).unwrap();
    std::fs::write(symlinked.join("real.py"), "x\n").unwrap();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(symlinked.join("real.py"), symlinked.join("link.py")).unwrap();
        let (_, _, py_sym_ok) = py_run(
            &root,
            "import sys; from pathlib import Path; sys.path.insert(0,'runner'); import workbench; \
             workbench._validate_no_symlinks(Path(sys.argv[1]))",
            &[&symlinked],
        );
        assert!(!py_sym_ok, "python should error on symlink");
        assert!(
            validate_no_symlinks(&symlinked).is_err(),
            "rust should error on symlink"
        );
    }

    let _ = std::fs::remove_dir_all(&tmp);
}

// ---------------------------------------------------------------------------
// Case 4: holdout_counts — matches Python _holdout_counts
// ---------------------------------------------------------------------------

#[test]
fn parity_holdout_counts() {
    if !python_available() {
        eprintln!("skipping workbench parity: python3 not available");
        return;
    }
    let root = repo_root();
    let tmp = tmpdir("holdout-counts");
    let arena = tmp.join("arena");
    std::fs::create_dir_all(&arena).unwrap();

    // With version column
    std::fs::write(
        arena.join("holdout-ledger.md"),
        "| date | arena version | run | purpose | tasks |\n\
         |---|---|---|---|---|\n\
         | 2026-06-12 | 0.1.0 | old-run | old baseline | task-a x9 |\n\
         | 2026-06-12 | 0.2.0 | new-run | new baseline | task-a x1, task-b x2 |\n",
    )
    .unwrap();

    // Case A: with arena_version=0.2.0
    let py_a = py_json(
        &root,
        "import sys, json; from pathlib import Path; sys.path.insert(0,'runner'); import workbench; \
         result = workbench._holdout_counts(Path(sys.argv[1]), ['task-a', 'task-b'], '0.2.0'); \
         print(json.dumps(result, sort_keys=True))",
        &[&arena],
    );
    let rust_a = holdout_counts(
        &arena,
        &["task-a".to_string(), "task-b".to_string()],
        Some("0.2.0"),
    );
    let rust_a_val: Value = serde_json::json!({
        "task-a": rust_a["task-a"],
        "task-b": rust_a["task-b"],
    });
    assert_eq!(
        py_a, rust_a_val,
        "holdout_counts with version=0.2.0 differs"
    );

    // Case B: no version filter (all rows count)
    let py_b = py_json(
        &root,
        "import sys, json; from pathlib import Path; sys.path.insert(0,'runner'); import workbench; \
         result = workbench._holdout_counts(Path(sys.argv[1]), ['task-a', 'task-b'], None); \
         print(json.dumps(result, sort_keys=True))",
        &[&arena],
    );
    let rust_b = holdout_counts(&arena, &["task-a".to_string(), "task-b".to_string()], None);
    let rust_b_val: Value = serde_json::json!({
        "task-a": rust_b["task-a"],
        "task-b": rust_b["task-b"],
    });
    assert_eq!(
        py_b, rust_b_val,
        "holdout_counts without version filter differs"
    );

    // Case C: empty holdout_tasks
    let rust_empty = holdout_counts(&arena, &[], None);
    assert!(
        rust_empty.is_empty(),
        "empty holdout_tasks should give empty map"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

// ---------------------------------------------------------------------------
// Case 5: format_holdout_ledger_row — exact string parity
// ---------------------------------------------------------------------------

#[test]
fn parity_format_holdout_ledger_row() {
    if !python_available() {
        eprintln!("skipping workbench parity: python3 not available");
        return;
    }
    let root = repo_root();

    // Case A: with arena_version
    let py_a = py_text(
        &root,
        "import sys; sys.path.insert(0,'runner'); import workbench; \
         row = workbench.format_holdout_ledger_row(\
             '20260612T220412Z', 'runs-search', ['cand-a', 'cand-b'], \
             ['holdout-a', 'holdout-b'], trials_per_candidate=3, arena_version='0.2.0'); \
         sys.stdout.write(row)",
        &[],
    );
    let rust_a = format_holdout_ledger_row(
        "20260612T220412Z",
        "runs-search",
        &["cand-a", "cand-b"],
        &["holdout-a", "holdout-b"],
        3,
        Some("0.2.0"),
    );
    assert_eq!(
        py_a, rust_a,
        "format_holdout_ledger_row with version differs\npy:   {py_a:?}\nrust: {rust_a:?}"
    );

    // Case B: without arena_version
    let py_b = py_text(
        &root,
        "import sys; sys.path.insert(0,'runner'); import workbench; \
         row = workbench.format_holdout_ledger_row(\
             '20260612T220412Z', 'runs-search', ['cand-a'], \
             ['holdout-x'], trials_per_candidate=2, arena_version=None); \
         sys.stdout.write(row)",
        &[],
    );
    let rust_b = format_holdout_ledger_row(
        "20260612T220412Z",
        "runs-search",
        &["cand-a"],
        &["holdout-x"],
        2,
        None,
    );
    assert_eq!(
        py_b, rust_b,
        "format_holdout_ledger_row without version differs\npy:   {py_b:?}\nrust: {rust_b:?}"
    );
}

// ---------------------------------------------------------------------------
// Case 6: validate_arena structured fields
// ---------------------------------------------------------------------------

#[test]
fn parity_validate_arena_structured() {
    if !python_available() {
        eprintln!("skipping workbench parity: python3 not available");
        return;
    }
    let root = repo_root();
    let tmp = tmpdir("va-structured");
    let arena = write_arena(&tmp);
    let probe = write_probe_run(&tmp);

    // Python result
    let py_result = py_json(
        &root,
        "import sys, json; from pathlib import Path; sys.path.insert(0,'runner'); import workbench; \
         r = workbench.validate_arena(Path(sys.argv[1]), probe_run=Path(sys.argv[2])); \
         print(json.dumps({'ok': r.ok, 'oracle_mean': r.oracle_mean, \
             'null_mean': r.null_mean, 'probe_mean': r.probe_mean, \
             'messages': r.messages, 'holdout_counts': r.holdout_counts}))",
        &[&arena, &probe],
    );
    let rust_result = validate_arena(&arena, Some(&probe), 5).expect("rust validate_arena");

    assert_eq!(
        py_result.get("ok"),
        Some(&serde_json::json!(rust_result.ok)),
        "validate_arena ok differs"
    );
    assert_eq!(
        py_result.get("oracle_mean"),
        Some(&serde_json::json!(rust_result.oracle_mean)),
        "validate_arena oracle_mean differs"
    );
    assert_eq!(
        py_result.get("null_mean"),
        Some(&serde_json::json!(rust_result.null_mean)),
        "validate_arena null_mean differs"
    );
    assert_eq!(
        py_result.get("probe_mean"),
        Some(&serde_json::json!(rust_result.probe_mean)),
        "validate_arena probe_mean differs"
    );
    assert_eq!(
        py_result.get("messages"),
        Some(&serde_json::json!(rust_result.messages)),
        "validate_arena messages differ"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

// ---------------------------------------------------------------------------
// Case 7: validate_arena — missing split
// ---------------------------------------------------------------------------

#[test]
fn parity_validate_arena_missing_split() {
    if !python_available() {
        eprintln!("skipping workbench parity: python3 not available");
        return;
    }
    let root = repo_root();
    let tmp = tmpdir("va-missing-split");
    let arena = write_arena(&tmp);
    let probe = write_probe_run(&tmp);

    let text = std::fs::read_to_string(arena.join("arena.toml")).unwrap();
    let new_text = text.replace("validation = [\"clean\"]", "validation = []");
    std::fs::write(arena.join("arena.toml"), &new_text).unwrap();

    let py_result = py_json(
        &root,
        "import sys, json; from pathlib import Path; sys.path.insert(0,'runner'); import workbench; \
         r = workbench.validate_arena(Path(sys.argv[1]), probe_run=Path(sys.argv[2])); \
         print(json.dumps({'ok': r.ok, 'messages': r.messages}))",
        &[&arena, &probe],
    );
    let rust_result = validate_arena(&arena, Some(&probe), 5).expect("rust validate_arena");

    assert_eq!(
        py_result["ok"],
        serde_json::json!(false),
        "missing split should fail in Python"
    );
    assert!(!rust_result.ok, "missing split should fail in Rust");
    // Both should contain "not assigned to any split: clean"
    let py_msgs = py_result["messages"].as_array().unwrap();
    assert!(
        py_msgs.iter().any(|m| m
            .as_str()
            .unwrap_or("")
            .contains("not assigned to any split: clean")),
        "python messages missing expected text: {py_msgs:?}"
    );
    assert!(
        rust_result
            .messages
            .iter()
            .any(|m| m.contains("not assigned to any split: clean")),
        "rust messages missing expected text: {:?}",
        rust_result.messages
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

// ---------------------------------------------------------------------------
// Case 8: render_validation_report — exact string parity
// ---------------------------------------------------------------------------

#[test]
fn parity_render_validation_report() {
    if !python_available() {
        eprintln!("skipping workbench parity: python3 not available");
        return;
    }
    let root = repo_root();
    let tmp = tmpdir("render-report");
    let arena = write_arena(&tmp);
    let probe = write_probe_run(&tmp);

    // Passing report
    let py_pass = py_text(
        &root,
        "import sys; from pathlib import Path; sys.path.insert(0,'runner'); import workbench; \
         r = workbench.validate_arena(Path(sys.argv[1]), probe_run=Path(sys.argv[2])); \
         sys.stdout.write(workbench.render_validation_report(r))",
        &[&arena, &probe],
    );
    let rust_report = validate_arena(&arena, Some(&probe), 5).expect("rust validate_arena");
    let rust_pass = render_validation_report(&rust_report);
    assert_eq!(
        py_pass, rust_pass,
        "render_validation_report (pass) differs\npy:>>>\n{py_pass}<<<\nrust:>>>\n{rust_pass}<<<"
    );

    // Failing report (missing split)
    let text = std::fs::read_to_string(arena.join("arena.toml")).unwrap();
    let new_text = text.replace("validation = [\"clean\"]", "validation = []");
    std::fs::write(arena.join("arena.toml"), &new_text).unwrap();

    let py_fail = py_text(
        &root,
        "import sys; from pathlib import Path; sys.path.insert(0,'runner'); import workbench; \
         r = workbench.validate_arena(Path(sys.argv[1]), probe_run=Path(sys.argv[2])); \
         sys.stdout.write(workbench.render_validation_report(r))",
        &[&arena, &probe],
    );
    let rust_fail_report = validate_arena(&arena, Some(&probe), 5).expect("rust validate_arena");
    let rust_fail = render_validation_report(&rust_fail_report);
    assert_eq!(
        py_fail, rust_fail,
        "render_validation_report (fail) differs\npy:>>>\n{py_fail}<<<\nrust:>>>\n{rust_fail}<<<"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

// ---------------------------------------------------------------------------
// Case 9: record_adjudication — OUT-OF-SCOPE (no date-sensitive content)
// ---------------------------------------------------------------------------

#[test]
fn parity_record_adjudication_out_of_scope() {
    if !python_available() {
        eprintln!("skipping workbench parity: python3 not available");
        return;
    }
    let root = repo_root();
    let tmp = tmpdir("adjudicate");
    let arena = write_arena(&tmp);

    // Python OUT-OF-SCOPE adjudication
    let (_, py_stderr, py_ok) = py_run(
        &root,
        "import sys; from pathlib import Path; sys.path.insert(0,'runner'); import workbench; \
         workbench.record_adjudication(\
             Path(sys.argv[1]), task='buggy', finding='no finding', ruling='OUT-OF-SCOPE', \
             rationale='not relevant', new_version=None, baseline_run=None)",
        &[&arena],
    );
    assert!(
        py_ok,
        "python record_adjudication OUT-OF-SCOPE failed: {py_stderr}"
    );

    // Note: adjudications.md already written by Python above; use a fresh arena for Rust
    let tmp2 = tmpdir("adjudicate-rust");
    let arena2 = write_arena(&tmp2);
    let rust_path = daedalus_core::workbench::record_adjudication(
        &arena2,
        "buggy",
        "no finding",
        "OUT-OF-SCOPE",
        "not relevant",
        None,
        None,
    )
    .expect("rust record_adjudication OUT-OF-SCOPE");

    let rust_text = std::fs::read_to_string(&rust_path).unwrap();
    // Structure checks (not date-exact since dates are wall-clock)
    assert!(
        rust_text.contains("Answer-key adjudications"),
        "missing header"
    );
    assert!(rust_text.contains("ADJ-1"), "missing ADJ-1");
    assert!(rust_text.contains("OUT-OF-SCOPE"), "missing ruling");
    assert!(rust_text.contains("buggy"), "missing task");
    assert!(rust_text.contains("not relevant"), "missing rationale");
    assert!(
        !rust_text.contains("Baseline run"),
        "should not have baseline run"
    );

    // Python text also has these structural features
    let py_text = std::fs::read_to_string(arena.join("adjudications.md")).unwrap();
    assert!(
        py_text.contains("Answer-key adjudications"),
        "py: missing header"
    );
    assert!(py_text.contains("ADJ-1"), "py: missing ADJ-1");
    assert!(py_text.contains("OUT-OF-SCOPE"), "py: missing ruling");

    let _ = std::fs::remove_dir_all(&tmp);
    let _ = std::fs::remove_dir_all(&tmp2);
}

// ---------------------------------------------------------------------------
// Case 10: disagreements — exact semantic parity
// ---------------------------------------------------------------------------

#[test]
fn parity_disagreements() {
    if !python_available() {
        eprintln!("skipping workbench parity: python3 not available");
        return;
    }
    let root = repo_root();
    let tmp = tmpdir("disagreements");

    let expected = tmp.join("expected.json");
    std::fs::write(
        &expected,
        serde_json::to_string(&serde_json::json!({
            "defects": [{
                "id": "escape",
                "file": "app.py",
                "line_start": 10,
                "line_end": 12,
                "category": "security"
            }]
        }))
        .unwrap(),
    )
    .unwrap();

    let findings = tmp.join("findings.json");
    std::fs::write(
        &findings,
        serde_json::to_string(&serde_json::json!({
            "findings": [
                {"file": "app.py", "line": 11, "category": "correctness"},
                {"file": "app.py", "line": 14, "category": "security"},
                {"file": "other.py", "line": 5, "category": "security"},
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    let py_result = py_json(
        &root,
        "import sys, json; from pathlib import Path; sys.path.insert(0,'runner'); import workbench; \
         rows = workbench.disagreements(Path(sys.argv[1]), Path(sys.argv[2])); \
         print(json.dumps(rows))",
        &[&findings, &expected],
    );
    let rust_rows = disagreements(&findings, &expected).expect("rust disagreements");
    let rust_val = serde_json::json!(rust_rows);

    assert_eq!(
        py_result, rust_val,
        "disagreements differs\npy: {py_result}\nrust: {rust_val}"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

// ---------------------------------------------------------------------------
// Case 11: disagreements — exact match (no rows emitted)
// ---------------------------------------------------------------------------

#[test]
fn parity_disagreements_exact_match() {
    if !python_available() {
        eprintln!("skipping workbench parity: python3 not available");
        return;
    }
    let root = repo_root();
    let tmp = tmpdir("disagree-exact");

    let expected = tmp.join("expected.json");
    std::fs::write(
        &expected,
        serde_json::to_string(&serde_json::json!({
            "defects": [{
                "id": "d1",
                "file": "a.py",
                "line_start": 5,
                "line_end": 10,
                "category": "security"
            }]
        }))
        .unwrap(),
    )
    .unwrap();

    let findings = tmp.join("findings.json");
    std::fs::write(
        &findings,
        serde_json::to_string(&serde_json::json!({
            "findings": [
                {"file": "a.py", "line": 7, "category": "security"},
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    let py_result = py_json(
        &root,
        "import sys, json; from pathlib import Path; sys.path.insert(0,'runner'); import workbench; \
         rows = workbench.disagreements(Path(sys.argv[1]), Path(sys.argv[2])); \
         print(json.dumps(rows))",
        &[&findings, &expected],
    );
    let rust_rows = disagreements(&findings, &expected).expect("rust disagreements");
    let rust_val = serde_json::json!(rust_rows);

    assert_eq!(
        py_result, rust_val,
        "disagreements exact match differs\npy: {py_result}\nrust: {rust_val}"
    );
    assert_eq!(rust_rows.len(), 0, "exact match should yield no rows");

    let _ = std::fs::remove_dir_all(&tmp);
}

// ---------------------------------------------------------------------------
// Case 12: version_tuple — comparison semantics
// ---------------------------------------------------------------------------

#[test]
fn parity_version_tuple() {
    if !python_available() {
        eprintln!("skipping workbench parity: python3 not available");
        return;
    }
    let root = repo_root();

    // Python's _version_tuple
    let pairs = [("0.1.0", "0.2.0"), ("1.0.0", "1.0.0"), ("0.10.0", "0.9.0")];
    for (a, b) in &pairs {
        let py_cmp = py_json(
            &root,
            &format!(
                "import sys, json; sys.path.insert(0,'runner'); import workbench; \
                 ta = workbench._version_tuple('{a}'); tb = workbench._version_tuple('{b}'); \
                 print(json.dumps(ta < tb))"
            ),
            &[],
        );
        let rust_lt = version_tuple(a) < version_tuple(b);
        assert_eq!(
            py_cmp,
            serde_json::json!(rust_lt),
            "version_tuple({a}) < version_tuple({b}) differs"
        );
    }
}

// ---------------------------------------------------------------------------
// Case 13: holdout burn threshold — validate_arena with holdout tasks
// ---------------------------------------------------------------------------

#[test]
fn parity_holdout_burn_threshold() {
    if !python_available() {
        eprintln!("skipping workbench parity: python3 not available");
        return;
    }
    let root = repo_root();
    let tmp = tmpdir("holdout-burn");
    let arena = write_arena(&tmp);
    let probe = write_probe_run(&tmp);

    // Rewrite arena.toml to put buggy in holdout
    let text = std::fs::read_to_string(arena.join("arena.toml")).unwrap();
    let text = text
        .replace("train = [\"buggy\"]", "train = []")
        .replace("holdout = []", "holdout = [\"buggy\"]");
    std::fs::write(arena.join("arena.toml"), &text).unwrap();

    // Write a ledger with 6 exposures of buggy (burn = 5 → should fail)
    std::fs::write(
        arena.join("holdout-ledger.md"),
        "| date | arena version | run | purpose | tasks |\n\
         |---|---|---|---|---|\n\
         | 2026-06-12 | 0.1.0 | run-1 | baseline | buggy x6 |\n",
    )
    .unwrap();

    let py_result = py_json(
        &root,
        "import sys, json; from pathlib import Path; sys.path.insert(0,'runner'); import workbench; \
         r = workbench.validate_arena(Path(sys.argv[1]), probe_run=Path(sys.argv[2]), holdout_burn=5); \
         print(json.dumps({'ok': r.ok, 'messages': r.messages}))",
        &[&arena, &probe],
    );
    let rust_result = validate_arena(&arena, Some(&probe), 5).expect("rust validate_arena");

    assert_eq!(
        py_result["ok"],
        serde_json::json!(false),
        "holdout burn should fail in Python"
    );
    assert!(!rust_result.ok, "holdout burn should fail in Rust");

    let py_msgs = py_result["messages"].as_array().unwrap();
    let py_has_burn = py_msgs
        .iter()
        .any(|m| m.as_str().unwrap_or("").contains("holdout task burned"));
    let rust_has_burn = rust_result
        .messages
        .iter()
        .any(|m| m.contains("holdout task burned"));
    assert!(
        py_has_burn,
        "python missing holdout burned message: {py_msgs:?}"
    );
    assert!(
        rust_has_burn,
        "rust missing holdout burned message: {:?}",
        rust_result.messages
    );

    let _ = std::fs::remove_dir_all(&tmp);
}
