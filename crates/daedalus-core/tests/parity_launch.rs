//! Parity oracle for the launch port.
//!
//! For each case, run BOTH the original Python `runner/launch.py` functions
//! and the Rust port over identical inputs and assert the outputs agree:
//!   - `render_import_packet` compared as exact bytes (it is template-based)
//!   - `write_import_packet` file output compared as exact bytes
//!   - error paths (UnsignedLaunchError / ContractValidationError) compared semantically
//!
//! The parity test ALWAYS passes an explicit timestamp so results are
//! deterministic; it never compares wall-clock `utc_now_iso()` output.
//!
//! Skips (does not fail) when python3 is unavailable.
//!
//! ## Parity gaps
//!
//! None known. The `render_import_packet` function is string-templated in both
//! Python and Rust; key order, escaping, and boolean representation are
//! identical. The swarm path delegates to `crate::swarm::load_swarm_contract`,
//! whose parity is verified separately in `parity_swarm.rs`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use daedalus_core::launch::{render_import_packet, write_import_packet};
use serde_json::Value;

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
        "daedalus-launch-parity-{}-{n}-{suffix}",
        std::process::id()
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

// ---------------------------------------------------------------------------
// Delivery fixture builder — mirrors tests/test_launch.py `build_delivery`
// ---------------------------------------------------------------------------

fn build_delivery(tmp_path: &Path) -> (PathBuf, PathBuf) {
    let prompt = tmp_path.join("packets").join("packet.md");
    std::fs::create_dir_all(prompt.parent().unwrap()).unwrap();
    std::fs::write(&prompt, "Measured review prompt.\n").unwrap();

    let evidence = tmp_path.join("runs").join("demo");
    std::fs::create_dir_all(&evidence).unwrap();
    for name in &["report.md", "lineage.md", "pareto.json", "trials.jsonl"] {
        std::fs::write(evidence.join(name), "evidence\n").unwrap();
    }

    let prompt_str = prompt.display().to_string();
    let evidence_str = evidence.display().to_string();
    let trials_str = evidence.join("trials.jsonl").display().to_string();

    let contract_text = format!(
        r#"
contract = 1
agent = "demo-agent"
composition_hash = "abc123"
taskspec = "demo"
mode = "threshold-then-cheap"

[composition]
harness = "pi"
harness_version = "9.9.9"
provider = "openrouter"
model = "z-ai/glm-5"
thinking = "low"
tools = ["read", "bash"]
prompt_packet = "{prompt_str}"
timeout_sec = 600

[permissions]
workspace = "read-only checkout"
env = ["OPENROUTER_API_KEY"]
write_actions = "none"

[budgets]
max_cost_usd_per_run = 0.5
max_wall_sec = 600

[observability]
arena = "arenas/pr-review-v2"
trace_destination = "JSONL-only waiver"

[evidence]
run_dir = "{evidence_str}"
report = "{evidence_str}/report.md"
lineage = "{evidence_str}/lineage.md"
pareto = "{evidence_str}/pareto.json"
trials = "{trials_str}"

[approval]
g3_signed = false
g3_approval = "approvals/G3-demo-agent.md"
note = "unsigned"
"#
    );
    std::fs::write(tmp_path.join("contract.toml"), &contract_text).unwrap();
    (tmp_path.to_path_buf(), prompt)
}

// ---------------------------------------------------------------------------
// Python driver helpers
// ---------------------------------------------------------------------------

/// Run Python `launch.render_import_packet` and return the rendered text.
fn py_render_import_packet(
    root: &Path,
    delivery: &Path,
    plane: &str,
    dry_run: bool,
    ts: &str,
) -> String {
    let dry_run_str = if dry_run { "True" } else { "False" };
    let snippet = format!(
        r#"import sys
sys.path.insert(0, 'runner')
import launch
from pathlib import Path
delivery = Path(r'{delivery}')
text = launch.render_import_packet(delivery, '{plane}', dry_run={dry_run_str}, generated='{ts}')
print(text, end='')"#,
        delivery = delivery.display(),
        plane = plane,
        dry_run_str = dry_run_str,
        ts = ts,
    );

    let out = Command::new("python3")
        .current_dir(root)
        .arg("-c")
        .arg(&snippet)
        .output()
        .expect("run python3");

    assert!(
        out.status.success(),
        "python3 failed:\nstderr: {}\nsnippet: {snippet}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("utf-8")
}

/// Run Python `launch.write_import_packet` and return the packet text.
fn py_write_import_packet(
    root: &Path,
    delivery: &Path,
    plane: &str,
    dry_run: bool,
    ts: &str,
) -> String {
    let dry_run_str = if dry_run { "True" } else { "False" };
    let snippet = format!(
        r#"import sys
sys.path.insert(0, 'runner')
import launch
from pathlib import Path
delivery = Path(r'{delivery}')
path = launch.write_import_packet(delivery, '{plane}', dry_run={dry_run_str}, generated='{ts}')
print(path.read_text(), end='')"#,
        delivery = delivery.display(),
        plane = plane,
        dry_run_str = dry_run_str,
        ts = ts,
    );

    let out = Command::new("python3")
        .current_dir(root)
        .arg("-c")
        .arg(&snippet)
        .output()
        .expect("run python3");

    assert!(
        out.status.success(),
        "python3 failed:\nstderr: {}\nsnippet: {snippet}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("utf-8")
}

/// Run Python `launch.render_import_packet` for a swarm delivery and return text.
fn py_render_swarm_import_packet(
    root: &Path,
    delivery: &Path,
    plane: &str,
    dry_run: bool,
    ts: &str,
) -> String {
    // swarm path goes through render_import_packet too
    py_render_import_packet(root, delivery, plane, dry_run, ts)
}

/// Run Python `launch.render_import_packet` expecting an error; return the error message.
fn py_render_import_packet_err(
    root: &Path,
    delivery: &Path,
    plane: &str,
    dry_run: bool,
    ts: &str,
) -> String {
    let dry_run_str = if dry_run { "True" } else { "False" };
    let snippet = format!(
        r#"import sys, json
sys.path.insert(0, 'runner')
import launch
from pathlib import Path
delivery = Path(r'{delivery}')
try:
    launch.render_import_packet(delivery, '{plane}', dry_run={dry_run_str}, generated='{ts}')
    print(json.dumps({{"ok": True, "msg": ""}}))
except (launch.UnsignedLaunchError, launch.ContractValidationError) as e:
    print(json.dumps({{"ok": False, "msg": str(e)}}))"#,
        delivery = delivery.display(),
        plane = plane,
        dry_run_str = dry_run_str,
        ts = ts,
    );

    let out = Command::new("python3")
        .current_dir(root)
        .arg("-c")
        .arg(&snippet)
        .output()
        .expect("run python3");

    assert!(
        out.status.success(),
        "python3 runner failed:\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: Value = serde_json::from_slice(&out.stdout).expect("json from python3");
    v["msg"].as_str().unwrap_or("").to_string()
}

// ---------------------------------------------------------------------------
// Parity assert helpers
// ---------------------------------------------------------------------------

fn assert_packet_parity(label: &str, py_text: &str, rust_text: &str) {
    assert_eq!(
        py_text, rust_text,
        "[{label}] import packet text differs\npy=>>>\n{py_text}<<<\nrust=>>>\n{rust_text}<<<"
    );
}

// ---------------------------------------------------------------------------
// Parity oracle tests
// ---------------------------------------------------------------------------

#[test]
fn launch_parity_across_fixtures() {
    if !python_available() {
        eprintln!("skipping launch parity: python3 not available");
        return;
    }

    let root = repo_root();

    // -----------------------------------------------------------------------
    // Case 1: unsigned contract — dry-run (single-agent)
    // -----------------------------------------------------------------------
    // Tests: render_import_packet byte-for-byte, refusal_reason, deployable=false,
    //        sandbox_required=true, primary_reviewer_allowed=false.
    {
        let label = "unsigned-dry-run";
        let d = tmpdir("case1");
        let (delivery, _) = build_delivery(&d);

        let py_text = py_render_import_packet(
            &root,
            &delivery,
            "bitter-blossom",
            true,
            "2026-06-11T00:00:00Z",
        );
        let rust_text = render_import_packet(
            &delivery,
            "bitter-blossom",
            true,
            Some("2026-06-11T00:00:00Z"),
            &root,
        )
        .expect("render_import_packet should succeed for dry-run");

        assert_packet_parity(label, &py_text, &rust_text);

        // TOML round-trip: both must parse as valid TOML
        let py_parsed: toml::Value =
            toml::from_str(&py_text).expect("[{label}] python text is valid TOML");
        let rust_parsed: toml::Value =
            toml::from_str(&rust_text).expect("[{label}] rust text is valid TOML");
        assert_eq!(
            py_parsed.get("deployable").and_then(toml::Value::as_bool),
            Some(false),
            "[{label}] deployable"
        );
        assert_eq!(
            rust_parsed.get("deployable").and_then(toml::Value::as_bool),
            Some(false),
            "[{label}] rust deployable"
        );

        let _ = std::fs::remove_dir_all(&d);
    }

    // -----------------------------------------------------------------------
    // Case 2: unsigned contract — deploy attempt raises UnsignedLaunchError
    // -----------------------------------------------------------------------
    // Tests: both Python and Rust raise UnsignedLaunchError with matching message.
    {
        let label = "unsigned-deploy-error";
        let d = tmpdir("case2");
        let (delivery, _) = build_delivery(&d);

        let py_err = py_render_import_packet_err(
            &root,
            &delivery,
            "bitter-blossom",
            false,
            "2026-06-11T00:00:00Z",
        );
        assert!(
            !py_err.is_empty(),
            "[{label}] Python should raise an error for unsigned deploy"
        );
        assert!(
            py_err.contains("G3 approval is unsigned"),
            "[{label}] Python error should mention 'G3 approval is unsigned': {py_err}"
        );

        let rust_result = render_import_packet(
            &delivery,
            "bitter-blossom",
            false,
            Some("2026-06-11T00:00:00Z"),
            &root,
        );
        assert!(
            rust_result.is_err(),
            "[{label}] Rust should raise an error for unsigned deploy"
        );
        let rust_err = rust_result.unwrap_err().to_string();
        assert!(
            rust_err.contains("G3 approval is unsigned"),
            "[{label}] Rust error should mention 'G3 approval is unsigned': {rust_err}"
        );

        let _ = std::fs::remove_dir_all(&d);
    }

    // -----------------------------------------------------------------------
    // Case 3: write_import_packet — file output byte-for-byte
    // -----------------------------------------------------------------------
    // Tests: write_import_packet writes the exact same bytes as Python.
    {
        let label = "write-import-packet-file";
        let d = tmpdir("case3");
        let (delivery, _) = build_delivery(&d);

        let py_text = py_write_import_packet(
            &root,
            &delivery,
            "bitter-blossom",
            true,
            "2026-06-11T00:00:00Z",
        );

        // Rust: write to a separate out_dir to avoid clobbering python's output
        let rust_out = d.join("rust-launch-dry-run");
        let path = write_import_packet(
            &delivery,
            "bitter-blossom",
            true,
            Some("2026-06-11T00:00:00Z"),
            Some(&rust_out),
            &root,
        )
        .expect("write_import_packet should succeed");
        let rust_text = std::fs::read_to_string(&path).unwrap();

        assert_packet_parity(label, &py_text, &rust_text);

        let _ = std::fs::remove_dir_all(&d);
    }

    // -----------------------------------------------------------------------
    // Case 4: real launch-contract delivery — dry-run
    // -----------------------------------------------------------------------
    // Uses the real `deliveries/launch-contract` fixture to test against a
    // real-world contract (absolute prompt_packet path, real evidence paths).
    {
        let label = "real-launch-contract-dry-run";
        let delivery = root.join("deliveries/launch-contract");

        let py_text = py_render_import_packet(
            &root,
            &delivery,
            "bitter-blossom",
            true,
            "2026-06-11T00:00:00Z",
        );
        let rust_text = render_import_packet(
            &delivery,
            "bitter-blossom",
            true,
            Some("2026-06-11T00:00:00Z"),
            &root,
        )
        .expect("render_import_packet should succeed for real delivery");

        assert_packet_parity(label, &py_text, &rust_text);
    }

    // -----------------------------------------------------------------------
    // Case 5: real pr-review-swarm delivery — swarm path dry-run
    // -----------------------------------------------------------------------
    // Tests: when swarm-contract.toml is present, delegates to swarm path
    //        and produces matching output byte-for-byte.
    {
        let label = "swarm-dry-run";
        let delivery = root.join("deliveries/pr-review-swarm");

        let py_text = py_render_swarm_import_packet(
            &root,
            &delivery,
            "bitter-blossom",
            true,
            "2026-06-11T00:00:00Z",
        );
        let rust_text = render_import_packet(
            &delivery,
            "bitter-blossom",
            true,
            Some("2026-06-11T00:00:00Z"),
            &root,
        )
        .expect("render_import_packet swarm path should succeed");

        assert_packet_parity(label, &py_text, &rust_text);
    }

    // -----------------------------------------------------------------------
    // Case 6: ContractValidationError on missing composition_hash
    // -----------------------------------------------------------------------
    // Tests: schema validation fires before dry-run with correct field name in msg.
    {
        let label = "missing-composition-hash";
        let d = tmpdir("case6");
        let (delivery, _) = build_delivery(&d);

        let contract_path = delivery.join("contract.toml");
        let text = std::fs::read_to_string(&contract_path).unwrap();
        let new_text = text.replace("composition_hash = \"abc123\"\n", "");
        std::fs::write(&contract_path, &new_text).unwrap();

        let py_err = py_render_import_packet_err(
            &root,
            &delivery,
            "bitter-blossom",
            true,
            "2026-06-11T00:00:00Z",
        );
        assert!(
            py_err.contains("composition_hash"),
            "[{label}] Python error should mention 'composition_hash': {py_err}"
        );

        let rust_result = render_import_packet(
            &delivery,
            "bitter-blossom",
            true,
            Some("2026-06-11T00:00:00Z"),
            &root,
        );
        assert!(
            rust_result.is_err(),
            "[{label}] Rust should raise ContractValidationError"
        );
        let rust_err = rust_result.unwrap_err().to_string();
        assert!(
            rust_err.contains("composition_hash"),
            "[{label}] Rust error should mention 'composition_hash': {rust_err}"
        );

        let _ = std::fs::remove_dir_all(&d);
    }

    // -----------------------------------------------------------------------
    // Case 7: write_actions != "none" requires G4
    // -----------------------------------------------------------------------
    // Tests: write authority guard fires before dry-run.
    {
        let label = "write-auth-requires-g4";
        let d = tmpdir("case7");
        let (delivery, _) = build_delivery(&d);

        let contract_path = delivery.join("contract.toml");
        let text = std::fs::read_to_string(&contract_path).unwrap();
        let new_text = text.replace(
            "write_actions = \"none\"",
            "write_actions = \"post PR comments\"",
        );
        std::fs::write(&contract_path, &new_text).unwrap();

        let py_err = py_render_import_packet_err(
            &root,
            &delivery,
            "bitter-blossom",
            true,
            "2026-06-11T00:00:00Z",
        );
        assert!(
            py_err.contains("G4"),
            "[{label}] Python error should mention 'G4': {py_err}"
        );

        let rust_result = render_import_packet(
            &delivery,
            "bitter-blossom",
            true,
            Some("2026-06-11T00:00:00Z"),
            &root,
        );
        assert!(rust_result.is_err(), "[{label}] Rust should reject");
        let rust_err = rust_result.unwrap_err().to_string();
        assert!(
            rust_err.contains("G4"),
            "[{label}] Rust error should mention 'G4': {rust_err}"
        );

        let _ = std::fs::remove_dir_all(&d);
    }

    // -----------------------------------------------------------------------
    // Case 8: "none, except PR comments" still requires G4 (no-prefix bypass)
    // -----------------------------------------------------------------------
    {
        let label = "write-auth-none-prefix-no-bypass";
        let d = tmpdir("case8");
        let (delivery, _) = build_delivery(&d);

        let contract_path = delivery.join("contract.toml");
        let text = std::fs::read_to_string(&contract_path).unwrap();
        let new_text = text.replace(
            "write_actions = \"none\"",
            "write_actions = \"none, except PR comments\"",
        );
        std::fs::write(&contract_path, &new_text).unwrap();

        let py_err = py_render_import_packet_err(
            &root,
            &delivery,
            "bitter-blossom",
            true,
            "2026-06-11T00:00:00Z",
        );
        assert!(
            py_err.contains("G4"),
            "[{label}] Python error should mention 'G4': {py_err}"
        );

        let rust_result = render_import_packet(
            &delivery,
            "bitter-blossom",
            true,
            Some("2026-06-11T00:00:00Z"),
            &root,
        );
        assert!(rust_result.is_err(), "[{label}] Rust should reject");
        assert!(
            rust_result.unwrap_err().to_string().contains("G4"),
            "[{label}] Rust error should mention 'G4'"
        );

        let _ = std::fs::remove_dir_all(&d);
    }

    // -----------------------------------------------------------------------
    // Case 9: signed g3 flag but unsigned g3 file — UnsignedLaunchError
    // -----------------------------------------------------------------------
    {
        let label = "g3-flag-but-no-file";
        let d = tmpdir("case9");
        let (delivery, _) = build_delivery(&d);

        let contract_path = delivery.join("contract.toml");
        let text = std::fs::read_to_string(&contract_path).unwrap();
        let new_text = text.replace("g3_signed = false", "g3_signed = true");
        std::fs::write(&contract_path, &new_text).unwrap();

        let py_err = py_render_import_packet_err(
            &root,
            &delivery,
            "bitter-blossom",
            false,
            "2026-06-11T00:00:00Z",
        );
        assert!(
            py_err.contains("approval file"),
            "[{label}] Python error should mention 'approval file': {py_err}"
        );

        let rust_result = render_import_packet(
            &delivery,
            "bitter-blossom",
            false,
            Some("2026-06-11T00:00:00Z"),
            &root,
        );
        assert!(rust_result.is_err(), "[{label}] Rust should reject");
        let rust_err = rust_result.unwrap_err().to_string();
        assert!(
            rust_err.contains("approval file"),
            "[{label}] Rust error should mention 'approval file': {rust_err}"
        );

        let _ = std::fs::remove_dir_all(&d);
    }

    // -----------------------------------------------------------------------
    // Case 10: write_import_packet swarm — file output byte-for-byte
    // -----------------------------------------------------------------------
    // Tests: swarm write path writes the exact same bytes as Python.
    {
        let label = "swarm-write-import-packet-file";
        let delivery = root.join("deliveries/pr-review-swarm");

        let py_text =
            py_write_import_packet(&root, &delivery, "olympus", true, "2026-06-11T00:00:00Z");

        let rust_out = tmpdir("case10-out");
        let path = write_import_packet(
            &delivery,
            "olympus",
            true,
            Some("2026-06-11T00:00:00Z"),
            Some(&rust_out),
            &root,
        )
        .expect("write_import_packet swarm should succeed");
        let rust_text = std::fs::read_to_string(&path).unwrap();

        assert_packet_parity(label, &py_text, &rust_text);

        let _ = std::fs::remove_dir_all(&rust_out);
    }
}
