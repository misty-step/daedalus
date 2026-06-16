//! Parity oracle for the scorer port.
//!
//! For each case, run BOTH the original Python `runner/score.py` and the Rust
//! `score()` over identical inputs and assert the verdicts agree. The grader is
//! gospel: a silent divergence here would poison every future experiment, so
//! this is the verification loop the port ships behind.
//!
//! Error *text* is implementation-defined (Python's `FileNotFoundError` string
//! differs from Rust's `io::Error`), so we compare error *presence*; every
//! other field must be exactly equal. Skips (does not fail) when python3 is
//! unavailable, mirroring `bin/gate`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use daedalus_core::score::score;
use serde_json::Value;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn repo_root() -> PathBuf {
    // crates/daedalus-core -> crates -> <repo root>
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

fn fresh_dir() -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("daedalus-parity-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn py_score(root: &Path, findings: &Path, expected: &Path) -> Value {
    let out = Command::new("python3")
        .arg(root.join("runner/score.py"))
        .arg(findings)
        .arg(expected)
        .output()
        .expect("run python score.py");
    assert!(
        out.status.success(),
        "python score.py failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("python score.py emitted valid json")
}

/// Compare a Python verdict and a Rust verdict, ignoring error *text*.
fn assert_parity(label: &str, py: &Value, rust: &Value) {
    assert_eq!(
        py["error"].is_null(),
        rust["error"].is_null(),
        "[{label}] error presence differs\n  py={py}\n  rust={rust}"
    );
    for field in [
        "reward",
        "recall",
        "matched",
        "false_positives",
        "expected_defects",
    ] {
        assert_eq!(
            py[field], rust[field],
            "[{label}] field `{field}` differs\n  py={py}\n  rust={rust}"
        );
    }
}

/// Run one case through both implementations. `findings` is the raw
/// findings.json body; `expected` the raw expected.json body. A `findings` of
/// `None` means "don't write the file" (the missing-file path).
fn check(label: &str, root: &Path, findings: Option<&str>, expected: &str) {
    let dir = fresh_dir();
    let expected_path = dir.join("expected.json");
    std::fs::write(&expected_path, expected).unwrap();

    let findings_path = dir.join("findings.json");
    if let Some(body) = findings {
        std::fs::write(&findings_path, body).unwrap();
    }

    let py = py_score(root, &findings_path, &expected_path);
    let rust = serde_json::to_value(score(&findings_path, &expected_path).unwrap()).unwrap();
    assert_parity(label, &py, &rust);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn scorer_matches_python_across_cases() {
    if !python_available() {
        eprintln!("skipping scorer parity: python3 not available");
        return;
    }
    let root = repo_root();

    let two = r#"{"defects":[
        {"id":"d1","file":"a.py","line_start":5,"line_end":10,"category":"security"},
        {"id":"d2","file":"a.py","line_start":20,"line_end":22,"category":"correctness"}
    ]}"#;
    let clean = r#"{"defects":[]}"#;
    let sev = r#"{"defects":[{"id":"d1","file":"a.py","line_start":1,"line_end":1,
        "category":"credential-exposure","severity":"blocking"}]}"#;

    // --- behavioral cases mirroring the unit suite ---
    check(
        "perfect",
        &root,
        Some(
            r#"{"findings":[{"file":"a.py","line":7,"category":"security"},{"file":"a.py","line":21,"category":"correctness"}]}"#,
        ),
        two,
    );
    check(
        "partial+fp",
        &root,
        Some(
            r#"{"findings":[{"file":"a.py","line":7,"category":"security"},{"file":"a.py","line":99,"category":"concurrency"}]}"#,
        ),
        two,
    );
    check(
        "silence-on-defective",
        &root,
        Some(r#"{"findings":[]}"#),
        two,
    );
    check("silence-on-clean", &root, Some(r#"{"findings":[]}"#), clean);
    check(
        "invented-on-clean",
        &root,
        Some(r#"{"findings":[{"file":"a.py","line":1,"category":"correctness"}]}"#),
        clean,
    );
    check(
        "at-most-once",
        &root,
        Some(
            r#"{"findings":[{"file":"a.py","line":6,"category":"security"},{"file":"a.py","line":8,"category":"security"}]}"#,
        ),
        two,
    );

    // --- int() coercion edge cases ---
    check(
        "float-line-truncates",
        &root,
        Some(r#"{"findings":[{"file":"a.py","line":7.9,"category":"security"}]}"#),
        two,
    );
    check(
        "string-line-parses",
        &root,
        Some(r#"{"findings":[{"file":"a.py","line":"7","category":"security"}]}"#),
        two,
    );
    check(
        "decimal-string-line-is-fp",
        &root,
        Some(r#"{"findings":[{"file":"a.py","line":"7.5","category":"security"}]}"#),
        two,
    );
    check(
        "nonnumeric-line-is-fp",
        &root,
        Some(r#"{"findings":[{"file":"a.py","line":"abc","category":"security"}]}"#),
        two,
    );
    check(
        "null-line-is-fp",
        &root,
        Some(r#"{"findings":[{"file":"a.py","line":null,"category":"security"}]}"#),
        two,
    );

    // --- severity gate ---
    check(
        "severity-too-weak",
        &root,
        Some(
            r#"{"findings":[{"file":"a.py","line":1,"category":"credential-exposure","severity":"serious"}]}"#,
        ),
        sev,
    );
    check(
        "severity-strong-enough",
        &root,
        Some(
            r#"{"findings":[{"file":"a.py","line":1,"category":"credential-exposure","severity":"blocking"}]}"#,
        ),
        sev,
    );
    check(
        "severity-unknown",
        &root,
        Some(
            r#"{"findings":[{"file":"a.py","line":1,"category":"credential-exposure","severity":"critical"}]}"#,
        ),
        sev,
    );

    // --- rounding stress: non-terminating recall denominators ---
    let three = r#"{"defects":[
        {"id":"d1","file":"a.py","line_start":1,"line_end":1,"category":"c"},
        {"id":"d2","file":"a.py","line_start":2,"line_end":2,"category":"c"},
        {"id":"d3","file":"a.py","line_start":3,"line_end":3,"category":"c"}]}"#;
    check(
        "one-of-three-recall",
        &root,
        Some(r#"{"findings":[{"file":"a.py","line":1,"category":"c"}]}"#),
        three,
    );
    check(
        "two-of-three-recall",
        &root,
        Some(
            r#"{"findings":[{"file":"a.py","line":1,"category":"c"},{"file":"a.py","line":2,"category":"c"}]}"#,
        ),
        three,
    );

    let seven = {
        let defects: Vec<String> = (1..=7)
            .map(|i| format!(r#"{{"id":"d{i}","file":"a.py","line_start":{i},"line_end":{i},"category":"c"}}"#))
            .collect();
        format!(r#"{{"defects":[{}]}}"#, defects.join(","))
    };
    check(
        "one-of-seven-recall",
        &root,
        Some(r#"{"findings":[{"file":"a.py","line":1,"category":"c"}]}"#),
        &seven,
    );

    // --- malformed output paths (compare error presence + zeroed fields) ---
    check("missing-file", &root, None, two);
    check("malformed-json", &root, Some("not json {"), two);
    check(
        "findings-not-a-list",
        &root,
        Some(r#"{"findings":"oops"}"#),
        two,
    );
    check(
        "missing-fields",
        &root,
        Some(r#"{"findings":[{"file":"a.py"},{"line":7,"category":"security"}]}"#),
        two,
    );
}
