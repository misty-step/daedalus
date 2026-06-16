//! Parity oracle for the swarm port.
//!
//! For each case, run BOTH the original Python `runner/swarm.py` functions
//! and the Rust port over identical inputs and assert the outputs agree:
//!   - `render_swarm_contract` compared as exact bytes (it is template-based)
//!   - `render_handoff` compared as exact bytes
//!   - `validate_summary` result fields compared semantically
//!   - `validate_swarm_contract` success/failure and loaded contract semantics
//!
//! The parity test ALWAYS passes an explicit timestamp so results are
//! deterministic; it never compares wall-clock `utc_now_iso()` output.
//!
//! Skips (does not fail) when python3 is unavailable.
//!
//! ## Parity gaps
//!
//! None known. Number formatting follows Python's `str(float)` exactly:
//! - TOML integers (e.g. `1200`, `1`) print without a decimal point.
//! - TOML floats and measured values (e.g. `2.0`, `900.0`) print with one.
//! - `json.dumps(sort_keys=True)` is replicated recursively.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use daedalus_core::swarm::{
    export_suite, load_swarm_contract, render_swarm_contract, validate_swarm_contract,
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
        "daedalus-swarm-parity-{}-{n}-{suffix}",
        std::process::id()
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

// ---------------------------------------------------------------------------
// Suite spec loader — loads the real taskspec as a JSON Value
// ---------------------------------------------------------------------------

fn suite_spec_json() -> Value {
    let repo = repo_root();
    let text = std::fs::read_to_string(repo.join("specs/pr-review-suite/taskspec.toml"))
        .expect("taskspec.toml exists");
    let tv: toml::Value = toml::from_str(&text).expect("taskspec.toml is valid TOML");
    toml_to_json(tv)
}

fn toml_to_json(tv: toml::Value) -> Value {
    match tv {
        toml::Value::String(s) => Value::String(s),
        toml::Value::Integer(i) => Value::from(i),
        toml::Value::Float(f) => Value::from(f),
        toml::Value::Boolean(b) => Value::Bool(b),
        toml::Value::Array(a) => Value::Array(a.into_iter().map(toml_to_json).collect()),
        toml::Value::Table(t) => {
            let mut m = serde_json::Map::new();
            for (k, v) in t {
                m.insert(k, toml_to_json(v));
            }
            Value::Object(m)
        }
        toml::Value::Datetime(dt) => Value::String(dt.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Fixture builder helpers
// ---------------------------------------------------------------------------

fn write_run_evidence(delivery: &Path, hashes: &[&str]) {
    let run_dir = delivery.join("evidence/run");
    let comp_dir = run_dir.join("compositions");
    std::fs::create_dir_all(&comp_dir).unwrap();
    std::fs::write(
        run_dir.join("trials.jsonl"),
        "{\"candidate_id\":\"suite\"}\n",
    )
    .unwrap();
    for (i, hash) in hashes.iter().enumerate() {
        std::fs::write(
            comp_dir.join(format!("c{}.json", i + 1)),
            serde_json::to_string(&json!({"composition_hash": hash})).unwrap(),
        )
        .unwrap();
    }
    std::fs::write(
        delivery.join("evidence/replay.json"),
        serde_json::to_string(&json!({"passed": true, "source": "test"})).unwrap(),
    )
    .unwrap();
}

/// Write a standard-fixture summary with optional field overrides.
/// Returns the summary as a JSON Value (after applying overrides).
fn write_summary(delivery: &Path, overrides: Option<Value>) -> Value {
    std::fs::create_dir_all(delivery).unwrap();
    let run_dir = "evidence/run";
    let hashes = ["generalhash", "correcthash", "securityhash", "masterhash"];
    write_run_evidence(delivery, &hashes);

    let mut summary = json!({
        "suite": {"total_cost_usd": 1.25, "total_wall_sec": 900},
        "waivers": {},
        "metrics": {
            "master_recall": 0.95,
            "blocking_recall": 1.0,
            "false_positive_carry": 1,
            "duplicate_collapse": 0.95
        },
        "members": {
            "general": {
                "contract": "members/general/contract.toml",
                "composition_hash": "generalhash",
                "evidence": {"run_dir": run_dir, "trials": format!("{run_dir}/trials.jsonl")}
            },
            "correctness": {
                "contract": "members/correctness/contract.toml",
                "composition_hash": "correcthash",
                "evidence": {"run_dir": run_dir, "trials": format!("{run_dir}/trials.jsonl")}
            },
            "security": {
                "contract": "members/security/contract.toml",
                "composition_hash": "securityhash",
                "evidence": {"run_dir": run_dir, "trials": format!("{run_dir}/trials.jsonl")}
            }
        },
        "master": {
            "contract": "master/contract.toml",
            "composition_hash": "masterhash",
            "evidence": {"run_dir": run_dir, "trials": format!("{run_dir}/trials.jsonl")},
            "real_member_replay": {"passed": true, "evidence": "evidence/replay.json"}
        },
        "handoff": {"mode": "full-swarm"}
    });

    if let Some(ov) = overrides {
        if let (Some(s_obj), Some(ov_obj)) = (summary.as_object_mut(), ov.as_object()) {
            for (k, v) in ov_obj {
                s_obj.insert(k.clone(), v.clone());
            }
        }
    }

    std::fs::write(
        delivery.join("summary.json"),
        serde_json::to_string(&summary).unwrap(),
    )
    .unwrap();

    summary
}

// ---------------------------------------------------------------------------
// Python driver helpers
// ---------------------------------------------------------------------------

/// Run Python `export_suite` over a delivery dir.
/// Returns (contract_text, handoff_text).
fn py_export_suite(root: &Path, delivery: &Path, ts: &str) -> (String, String) {
    let snippet = format!(
        r#"import sys, json, tomllib
sys.path.insert(0, 'runner')
import swarm
from pathlib import Path
delivery = Path(sys.argv[1])
spec = tomllib.loads((Path('specs/pr-review-suite/taskspec.toml')).read_text())
paths = swarm.export_suite(delivery, spec, generated="{ts}")
print(paths['contract'].read_text(), end='')
print('\x00SPLIT\x00', end='')
print(paths['handoff'].read_text(), end='')"#
    );

    let out = Command::new("python3")
        .current_dir(root)
        .arg("-c")
        .arg(&snippet)
        .arg(delivery)
        .output()
        .expect("run python3");

    assert!(
        out.status.success(),
        "python3 failed:\nstderr: {}\nsnippet: {snippet}",
        String::from_utf8_lossy(&out.stderr)
    );

    let combined = String::from_utf8(out.stdout).expect("utf-8");
    let parts: Vec<&str> = combined.split("\x00SPLIT\x00").collect();
    assert_eq!(parts.len(), 2, "expected two parts from python output");
    (parts[0].to_string(), parts[1].to_string())
}

/// Run Python `swarm.validate_swarm_contract` and return Ok(()) or Err(msg).
fn py_validate_swarm_contract(
    root: &Path,
    contract_path: &Path,
    delivery: &Path,
) -> Result<(), String> {
    let snippet = r#"import sys, json, tomllib
sys.path.insert(0, 'runner')
import swarm
from pathlib import Path
contract = swarm._load_toml(sys.argv[1])
try:
    swarm.validate_swarm_contract(contract, Path(sys.argv[2]))
    print(json.dumps({"ok": True, "msg": ""}))
except swarm.SwarmValidationError as e:
    print(json.dumps({"ok": False, "msg": str(e)}))"#;

    let out = Command::new("python3")
        .current_dir(root)
        .arg("-c")
        .arg(snippet)
        .arg(contract_path)
        .arg(delivery)
        .output()
        .expect("run python3");

    assert!(
        out.status.success(),
        "python3 failed:\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let v: Value = serde_json::from_slice(&out.stdout).expect("json");
    if v["ok"].as_bool().unwrap_or(false) {
        Ok(())
    } else {
        Err(v["msg"].as_str().unwrap_or("").to_string())
    }
}

// ---------------------------------------------------------------------------
// Parity helpers
// ---------------------------------------------------------------------------

fn assert_contract_parity(label: &str, py_text: &str, rust_text: &str) {
    assert_eq!(
        py_text, rust_text,
        "[{label}] swarm contract text differs\npy=>>>\n{py_text}<<<\nrust=>>>\n{rust_text}<<<"
    );
}

fn assert_handoff_parity(label: &str, py_text: &str, rust_text: &str) {
    assert_eq!(
        py_text, rust_text,
        "[{label}] handoff text differs\npy=>>>\n{py_text}<<<\nrust=>>>\n{rust_text}<<<"
    );
}

// ---------------------------------------------------------------------------
// Parity oracle tests
// ---------------------------------------------------------------------------

#[test]
fn swarm_parity_across_fixtures() {
    if !python_available() {
        eprintln!("skipping swarm parity: python3 not available");
        return;
    }

    let root = repo_root();

    // --- Case 1: full-swarm happy path ---
    // Tests: contract text, handoff text, TOML round-trip.
    {
        let label = "full-swarm-happy-path";
        let delivery = tmpdir("case1");
        write_summary(&delivery, None);

        let (py_contract, py_handoff) = py_export_suite(&root, &delivery, "2026-06-12T00:00:00Z");

        let spec = suite_spec_json();
        let result = export_suite(&delivery, &spec, Some("2026-06-12T00:00:00Z"), &root)
            .expect("Rust export_suite");
        let rust_contract = std::fs::read_to_string(&result.contract).unwrap();
        let rust_handoff = std::fs::read_to_string(&result.handoff).unwrap();

        assert_contract_parity(label, &py_contract, &rust_contract);
        assert_handoff_parity(label, &py_handoff, &rust_handoff);

        // TOML round-trip: Rust-generated contract must be loadable by
        // validate_swarm_contract and Python's validate_swarm_contract.
        let contract = load_swarm_contract(&delivery).expect("Rust load_swarm_contract");
        assert_eq!(
            contract
                .get("swarm_contract")
                .and_then(toml::Value::as_integer),
            Some(1),
            "[{label}] swarm_contract version"
        );

        let py_vc = py_validate_swarm_contract(&root, &result.contract, &delivery);
        assert!(
            py_vc.is_ok(),
            "[{label}] Python validate_swarm_contract failed: {:?}",
            py_vc
        );

        let _ = std::fs::remove_dir_all(&delivery);
    }

    // --- Case 2: member-only mode ---
    {
        let label = "member-only";
        let delivery = tmpdir("case2");
        write_summary(
            &delivery,
            Some(json!({
                "master": {
                    "contract": "master/contract.toml",
                    "composition_hash": "masterhash",
                    "evidence": {"run_dir": "evidence/run", "trials": "evidence/run/trials.jsonl"},
                    "real_member_replay": {"passed": false, "evidence": "evidence/replay.json"}
                },
                "handoff": {"mode": "member-only"}
            })),
        );

        let (py_contract, py_handoff) = py_export_suite(&root, &delivery, "2026-06-12T00:00:00Z");

        let spec = suite_spec_json();
        let result = export_suite(&delivery, &spec, Some("2026-06-12T00:00:00Z"), &root)
            .expect("Rust export_suite member-only");
        let rust_contract = std::fs::read_to_string(&result.contract).unwrap();
        let rust_handoff = std::fs::read_to_string(&result.handoff).unwrap();

        assert_contract_parity(label, &py_contract, &rust_contract);
        assert_handoff_parity(label, &py_handoff, &rust_handoff);

        let _ = std::fs::remove_dir_all(&delivery);
    }

    // --- Case 3: cost-waiver path ---
    {
        let label = "cost-waiver";
        let delivery = tmpdir("case3");
        write_summary(
            &delivery,
            Some(json!({
                "suite": {"total_cost_usd": 2.5, "total_wall_sec": 900},
                "waivers": {"cost_ceiling": true}
            })),
        );

        let (py_contract, py_handoff) = py_export_suite(&root, &delivery, "2026-06-12T00:00:00Z");

        let spec = suite_spec_json();
        let result = export_suite(&delivery, &spec, Some("2026-06-12T00:00:00Z"), &root)
            .expect("Rust export_suite cost-waiver");
        let rust_contract = std::fs::read_to_string(&result.contract).unwrap();
        let rust_handoff = std::fs::read_to_string(&result.handoff).unwrap();

        assert_contract_parity(label, &py_contract, &rust_contract);
        assert_handoff_parity(label, &py_handoff, &rust_handoff);

        let _ = std::fs::remove_dir_all(&delivery);
    }

    // --- Case 4: validate_swarm_contract rejects bad version ---
    {
        let label = "bad-version";
        let delivery = tmpdir("case4");
        write_summary(&delivery, None);

        // First export a valid contract, then corrupt the version.
        let spec = suite_spec_json();
        let result = export_suite(&delivery, &spec, Some("2026-06-12T00:00:00Z"), &root)
            .expect("export for bad-version case");

        let bad_contract_text = std::fs::read_to_string(&result.contract)
            .unwrap()
            .replace("swarm_contract = 1", "swarm_contract = 99");
        let bad_contract_path = delivery.join("swarm-contract-bad.toml");
        std::fs::write(&bad_contract_path, &bad_contract_text).unwrap();

        let bad_contract: toml::Value = toml::from_str(&bad_contract_text).unwrap();
        let rust_err = validate_swarm_contract(&bad_contract, &delivery);
        assert!(
            rust_err.is_err(),
            "[{label}] Rust should reject bad version"
        );

        let py_err = py_validate_swarm_contract(&root, &bad_contract_path, &delivery);
        assert!(
            py_err.is_err(),
            "[{label}] Python should reject bad version"
        );

        let _ = std::fs::remove_dir_all(&delivery);
    }

    // --- Case 5: cost ceiling rejection (no waiver) ---
    // Both Python and Rust should raise / return Err with "cost ceiling" in msg.
    {
        let label = "cost-ceiling-reject";
        let delivery = tmpdir("case5");
        write_summary(
            &delivery,
            Some(json!({
                "suite": {"total_cost_usd": 2.5, "total_wall_sec": 900}
            })),
        );

        let spec = suite_spec_json();
        let rust_err = export_suite(&delivery, &spec, None, &root);
        assert!(rust_err.is_err(), "[{label}] Rust should reject");
        assert!(
            rust_err.unwrap_err().0.contains("cost ceiling"),
            "[{label}] Rust error should mention 'cost ceiling'"
        );

        // Python error check via a snippet that returns error message.
        let snippet = r#"import sys, json, tomllib
sys.path.insert(0, 'runner')
import swarm
from pathlib import Path
delivery = Path(sys.argv[1])
spec = tomllib.loads(Path('specs/pr-review-suite/taskspec.toml').read_text())
try:
    swarm.export_suite(delivery, spec)
    print(json.dumps({"ok": True, "msg": ""}))
except swarm.SwarmValidationError as e:
    print(json.dumps({"ok": False, "msg": str(e)}))"#;

        let out = Command::new("python3")
            .current_dir(&root)
            .arg("-c")
            .arg(snippet)
            .arg(&delivery)
            .output()
            .expect("run python3");
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert!(
            !v["ok"].as_bool().unwrap_or(true),
            "[{label}] Python should reject"
        );
        assert!(
            v["msg"].as_str().unwrap_or("").contains("cost ceiling"),
            "[{label}] Python error should mention 'cost ceiling': {}",
            v["msg"]
        );

        let _ = std::fs::remove_dir_all(&delivery);
    }

    // --- Case 6: missing required member ---
    {
        let label = "missing-required-member";
        let delivery = tmpdir("case6");
        write_summary(
            &delivery,
            Some(json!({
                "members": {
                    "general": {
                        "contract": "members/general/contract.toml",
                        "composition_hash": "generalhash",
                        "evidence": {"run_dir": "evidence/run", "trials": "evidence/run/trials.jsonl"}
                    },
                    "correctness": {
                        "contract": "members/correctness/contract.toml",
                        "composition_hash": "correcthash",
                        "evidence": {"run_dir": "evidence/run", "trials": "evidence/run/trials.jsonl"}
                    }
                }
            })),
        );

        let spec = suite_spec_json();
        let rust_err = export_suite(&delivery, &spec, None, &root);
        assert!(rust_err.is_err(), "[{label}] Rust should reject");
        assert!(
            rust_err.unwrap_err().0.contains("required member missing"),
            "[{label}] Rust error should mention 'required member missing'"
        );

        let snippet = r#"import sys, json, tomllib
sys.path.insert(0, 'runner')
import swarm
from pathlib import Path
delivery = Path(sys.argv[1])
spec = tomllib.loads(Path('specs/pr-review-suite/taskspec.toml').read_text())
try:
    swarm.export_suite(delivery, spec)
    print(json.dumps({"ok": True, "msg": ""}))
except swarm.SwarmValidationError as e:
    print(json.dumps({"ok": False, "msg": str(e)}))"#;

        let out = Command::new("python3")
            .current_dir(&root)
            .arg("-c")
            .arg(snippet)
            .arg(&delivery)
            .output()
            .expect("run python3");
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert!(
            !v["ok"].as_bool().unwrap_or(true),
            "[{label}] Python should reject"
        );
        assert!(
            v["msg"]
                .as_str()
                .unwrap_or("")
                .contains("required member missing"),
            "[{label}] Python error mismatch: {}",
            v["msg"]
        );

        let _ = std::fs::remove_dir_all(&delivery);
    }

    // --- Case 7: bad composition_hash ---
    {
        let label = "bad-composition-hash";
        let delivery = tmpdir("case7");
        write_summary(
            &delivery,
            Some(json!({
                "members": {
                    "general": {
                        "contract": "members/general/contract.toml",
                        "composition_hash": "fabricated",
                        "evidence": {"run_dir": "evidence/run", "trials": "evidence/run/trials.jsonl"}
                    },
                    "correctness": {
                        "contract": "members/correctness/contract.toml",
                        "composition_hash": "correcthash",
                        "evidence": {"run_dir": "evidence/run", "trials": "evidence/run/trials.jsonl"}
                    },
                    "security": {
                        "contract": "members/security/contract.toml",
                        "composition_hash": "securityhash",
                        "evidence": {"run_dir": "evidence/run", "trials": "evidence/run/trials.jsonl"}
                    }
                }
            })),
        );

        let spec = suite_spec_json();
        let rust_err = export_suite(&delivery, &spec, None, &root);
        assert!(rust_err.is_err(), "[{label}] Rust should reject");
        assert!(
            rust_err.unwrap_err().0.contains("composition_hash"),
            "[{label}] Rust error should mention 'composition_hash'"
        );

        let snippet = r#"import sys, json, tomllib
sys.path.insert(0, 'runner')
import swarm
from pathlib import Path
delivery = Path(sys.argv[1])
spec = tomllib.loads(Path('specs/pr-review-suite/taskspec.toml').read_text())
try:
    swarm.export_suite(delivery, spec)
    print(json.dumps({"ok": True, "msg": ""}))
except swarm.SwarmValidationError as e:
    print(json.dumps({"ok": False, "msg": str(e)}))"#;

        let out = Command::new("python3")
            .current_dir(&root)
            .arg("-c")
            .arg(snippet)
            .arg(&delivery)
            .output()
            .expect("run python3");
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert!(
            !v["ok"].as_bool().unwrap_or(true),
            "[{label}] Python should reject"
        );
        assert!(
            v["msg"].as_str().unwrap_or("").contains("composition_hash"),
            "[{label}] Python error mismatch: {}",
            v["msg"]
        );

        let _ = std::fs::remove_dir_all(&delivery);
    }

    // --- Case 8: full-swarm replay failed → error ---
    {
        let label = "full-swarm-replay-failed";
        let delivery = tmpdir("case8");
        write_summary(
            &delivery,
            Some(json!({
                "master": {
                    "contract": "master/contract.toml",
                    "composition_hash": "masterhash",
                    "evidence": {"run_dir": "evidence/run", "trials": "evidence/run/trials.jsonl"},
                    "real_member_replay": {"passed": false, "evidence": "evidence/replay.json"}
                },
                "handoff": {"mode": "full-swarm"}
            })),
        );

        let spec = suite_spec_json();
        let rust_err = export_suite(&delivery, &spec, None, &root);
        assert!(rust_err.is_err(), "[{label}] Rust should reject");
        assert!(
            rust_err.unwrap_err().0.contains("real-member replay"),
            "[{label}] Rust error should mention 'real-member replay'"
        );

        let snippet = r#"import sys, json, tomllib
sys.path.insert(0, 'runner')
import swarm
from pathlib import Path
delivery = Path(sys.argv[1])
spec = tomllib.loads(Path('specs/pr-review-suite/taskspec.toml').read_text())
try:
    swarm.export_suite(delivery, spec)
    print(json.dumps({"ok": True, "msg": ""}))
except swarm.SwarmValidationError as e:
    print(json.dumps({"ok": False, "msg": str(e)}))"#;

        let out = Command::new("python3")
            .current_dir(&root)
            .arg("-c")
            .arg(snippet)
            .arg(&delivery)
            .output()
            .expect("run python3");
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert!(
            !v["ok"].as_bool().unwrap_or(true),
            "[{label}] Python should reject"
        );
        assert!(
            v["msg"]
                .as_str()
                .unwrap_or("")
                .contains("real-member replay"),
            "[{label}] Python error mismatch: {}",
            v["msg"]
        );

        let _ = std::fs::remove_dir_all(&delivery);
    }

    // --- Case 9: quality threshold failure (master_recall below min) ---
    {
        let label = "quality-threshold-master-recall";
        let delivery = tmpdir("case9");
        write_summary(
            &delivery,
            Some(json!({
                "metrics": {
                    "master_recall": 0.5,
                    "blocking_recall": 1.0,
                    "false_positive_carry": 1,
                    "duplicate_collapse": 0.95
                }
            })),
        );

        let spec = suite_spec_json();
        let rust_err = export_suite(&delivery, &spec, None, &root);
        assert!(rust_err.is_err(), "[{label}] Rust should reject");
        assert!(
            rust_err.unwrap_err().0.contains("master_recall"),
            "[{label}] Rust error should mention 'master_recall'"
        );

        let snippet = r#"import sys, json, tomllib
sys.path.insert(0, 'runner')
import swarm
from pathlib import Path
delivery = Path(sys.argv[1])
spec = tomllib.loads(Path('specs/pr-review-suite/taskspec.toml').read_text())
try:
    swarm.export_suite(delivery, spec)
    print(json.dumps({"ok": True, "msg": ""}))
except swarm.SwarmValidationError as e:
    print(json.dumps({"ok": False, "msg": str(e)}))"#;

        let out = Command::new("python3")
            .current_dir(&root)
            .arg("-c")
            .arg(snippet)
            .arg(&delivery)
            .output()
            .expect("run python3");
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert!(
            !v["ok"].as_bool().unwrap_or(true),
            "[{label}] Python should reject"
        );
        assert!(
            v["msg"].as_str().unwrap_or("").contains("master_recall"),
            "[{label}] Python error mismatch: {}",
            v["msg"]
        );

        let _ = std::fs::remove_dir_all(&delivery);
    }

    // --- Case 10: member-only mode skips quality threshold check ---
    // Even with master_recall=0.5, member-only should succeed.
    {
        let label = "member-only-skips-quality";
        let delivery = tmpdir("case10");
        write_summary(
            &delivery,
            Some(json!({
                "metrics": {
                    "master_recall": 0.5,
                    "blocking_recall": 1.0,
                    "false_positive_carry": 1,
                    "duplicate_collapse": 0.95
                },
                "master": {
                    "contract": "master/contract.toml",
                    "composition_hash": "masterhash",
                    "evidence": {"run_dir": "evidence/run", "trials": "evidence/run/trials.jsonl"},
                    "real_member_replay": {"passed": false, "evidence": "evidence/replay.json"}
                },
                "handoff": {"mode": "member-only"}
            })),
        );

        let (py_contract, py_handoff) = py_export_suite(&root, &delivery, "2026-06-12T00:00:00Z");

        let spec = suite_spec_json();
        let result = export_suite(&delivery, &spec, Some("2026-06-12T00:00:00Z"), &root)
            .expect("Rust member-only skips quality check");
        let rust_contract = std::fs::read_to_string(&result.contract).unwrap();
        let rust_handoff = std::fs::read_to_string(&result.handoff).unwrap();

        assert_contract_parity(label, &py_contract, &rust_contract);
        assert_handoff_parity(label, &py_handoff, &rust_handoff);

        let _ = std::fs::remove_dir_all(&delivery);
    }

    // --- Case 11: render_swarm_contract standalone (no file I/O) ---
    // Pass summary and spec directly; compare string output.
    {
        let label = "render-standalone";
        let delivery = tmpdir("case11");
        let summary_val = write_summary(&delivery, None);
        let spec = suite_spec_json();

        // Python standalone render
        let snippet = r#"import sys, json, tomllib
sys.path.insert(0, 'runner')
import swarm
from pathlib import Path
import json as json_mod
delivery = Path(sys.argv[1])
spec = tomllib.loads(Path('specs/pr-review-suite/taskspec.toml').read_text())
summary = json_mod.loads((delivery / 'summary.json').read_text())
print(swarm.render_swarm_contract(spec, summary, generated='2026-06-12T00:00:00Z', delivery_dir=delivery), end='')"#;

        let out = Command::new("python3")
            .current_dir(&root)
            .arg("-c")
            .arg(snippet)
            .arg(&delivery)
            .output()
            .expect("run python3");
        assert!(
            out.status.success(),
            "python3 failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let py_text = String::from_utf8(out.stdout).unwrap();

        let rust_text = render_swarm_contract(
            &spec,
            &summary_val,
            Some("2026-06-12T00:00:00Z"),
            Some(&delivery),
            &root,
        )
        .expect("Rust render_swarm_contract");

        assert_contract_parity(label, &py_text, &rust_text);

        let _ = std::fs::remove_dir_all(&delivery);
    }

    // --- Case 12: wall-time waiver ---
    {
        let label = "wall-time-waiver";
        let delivery = tmpdir("case12");
        write_summary(
            &delivery,
            Some(json!({
                "suite": {"total_cost_usd": 1.25, "total_wall_sec": 1500},
                "waivers": {"wall_time": true}
            })),
        );

        let (py_contract, py_handoff) = py_export_suite(&root, &delivery, "2026-06-12T00:00:00Z");

        let spec = suite_spec_json();
        let result = export_suite(&delivery, &spec, Some("2026-06-12T00:00:00Z"), &root)
            .expect("Rust export_suite wall-time-waiver");
        let rust_contract = std::fs::read_to_string(&result.contract).unwrap();
        let rust_handoff = std::fs::read_to_string(&result.handoff).unwrap();

        assert_contract_parity(label, &py_contract, &rust_contract);
        assert_handoff_parity(label, &py_handoff, &rust_handoff);

        let _ = std::fs::remove_dir_all(&delivery);
    }
}
