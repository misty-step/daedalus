use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crate lives under crates/threshold-core")
        .to_path_buf()
}

fn read_repo_file(path: &str) -> String {
    fs::read_to_string(repo_root().join(path)).unwrap_or_else(|err| {
        panic!("failed to read {path}: {err}");
    })
}

#[test]
fn canonical_gate_owns_the_rust_check_lanes() {
    let gate = read_repo_file("bin/gate");

    assert!(gate.starts_with("#!/usr/bin/env sh\n"));
    assert!(gate.contains("set -eu\n"));
    assert!(
        gate.contains("cargo fmt --check"),
        "bin/gate must keep rustfmt in the canonical local gate"
    );
    assert!(
        gate.contains("cargo test --workspace"),
        "bin/gate must keep workspace tests in the canonical local gate"
    );
    assert!(
        gate.contains("cargo clippy --workspace --all-targets"),
        "bin/gate must keep clippy in the canonical local gate"
    );
    assert!(
        gate.contains("-D warnings"),
        "clippy warnings must remain fatal"
    );
}

#[test]
fn github_actions_delegates_to_the_local_gate() {
    let workflow = read_repo_file(".github/workflows/ci.yml");

    assert!(
        workflow.contains("run: ./bin/gate"),
        "GitHub Actions should delegate to the repo-owned local gate"
    );
    assert_eq!(
        workflow.matches("run: ./bin/gate").count(),
        1,
        "the workflow should have one canonical gate invocation"
    );

    let direct_cargo_runs: Vec<&str> = workflow
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("run: cargo "))
        .collect();
    assert!(
        direct_cargo_runs.is_empty(),
        "GitHub Actions must not duplicate gate logic in YAML: {direct_cargo_runs:?}"
    );
}

#[test]
fn readme_quickstart_names_the_full_offline_gate() {
    let readme = read_repo_file("README.md");

    assert!(readme.contains("bin/gate"));
    assert!(
        readme.contains("cargo fmt --check"),
        "README quickstart should document that the offline gate includes rustfmt"
    );
    assert!(
        readme.contains("cargo test"),
        "README quickstart should document that the offline gate includes tests"
    );
    assert!(
        readme.contains("cargo clippy"),
        "README quickstart should document that the offline gate includes clippy"
    );
}
