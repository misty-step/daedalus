//! Parity oracle for the prompt_packet port: the Rust `is_sane_prompt_packet`
//! must agree with Python's over a corpus that exercises every branch (length
//! floor/ceiling, long runs, alpha-ratio, unique-ratio, Unicode counting).
//!
//! Skips (does not fail) when python3 is unavailable, mirroring `bin/gate`.

use std::path::{Path, PathBuf};
use std::process::Command;

use daedalus_core::prompt_packet::is_sane_prompt_packet;

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

#[test]
fn is_sane_matches_python_over_corpus() {
    if !python_available() {
        eprintln!("skipping prompt_packet parity: python3 not available");
        return;
    }
    let root = repo_root();

    let corpus: Vec<String> = vec![
        "too short".to_string(),
        "a".repeat(20),
        "a".repeat(30),
        "abcdefghij".repeat(401), // 4010 chars: over the size cap
        "Review the diff for correctness and security issues; cite file and line \
         for every finding you report."
            .to_string(),
        "1234567890!@#$%^&*()".repeat(10), // low alpha ratio
        "ab".repeat(100),                  // low unique ratio
        "é".repeat(10),                    // 10 code points / 20 bytes -> too short
        "é".repeat(25),                    // 25 code points, run 25 -> long run
        format!(
            "  {}  ",
            "Cite evidence for every finding and avoid nitpicks."
        ), // strip
        "x".repeat(24) + " and then some additional explanatory words here", // run==24 ok
        String::new(),
        "   ".to_string(),
        "The quick brown fox jumps over the lazy dog. ".repeat(5),
    ];

    let dir = std::env::temp_dir().join(format!("daedalus-pp-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let corpus_path = dir.join("corpus.json");
    std::fs::write(&corpus_path, serde_json::to_string(&corpus).unwrap()).unwrap();

    let out = Command::new("python3")
        .current_dir(&root)
        .arg("-c")
        .arg(
            "import sys, json; sys.path.insert(0, 'runner'); \
             from prompt_packet import is_sane_prompt_packet as f; \
             print(json.dumps([f(s) for s in json.load(open(sys.argv[1]))]))",
        )
        .arg(&corpus_path)
        .output()
        .expect("run python prompt_packet");
    assert!(
        out.status.success(),
        "python prompt_packet failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let py: Vec<bool> = serde_json::from_slice(&out.stdout).expect("python emitted json array");
    assert_eq!(py.len(), corpus.len());
    for (i, expected) in py.iter().enumerate() {
        let got = is_sane_prompt_packet(&corpus[i]);
        assert_eq!(
            *expected, got,
            "case {i} differs: py={expected} rust={got} for {:?}",
            corpus[i]
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}
