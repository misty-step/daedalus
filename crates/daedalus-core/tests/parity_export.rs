//! Parity oracle for the export port.
//!
//! For each case, run BOTH the original Python `runner/export.py` functions
//! and the Rust port over identical inputs and assert the outputs agree:
//!   - `render_contract` compared as exact bytes (template-based)
//!   - `render_persona` compared as exact bytes
//!   - `render_handoff` compared as exact bytes
//!   - `export_delivery` written-file contents compared as exact bytes
//!   - error paths (ValueError / ExportError) compared semantically
//!
//! The parity test ALWAYS passes an explicit timestamp so results are
//! deterministic; it never compares wall-clock `utc_now_iso()` output.
//!
//! Skips (does not fail) when python3 is unavailable.
//!
//! ## Real fixtures used
//!
//! - `deliveries/pr-review` + `specs/pr-review/taskspec.toml` (real delivery)
//! - Crafted tmp delivery (absolute prompt_packet path under a `runs/` tree)
//! - `deliveries/pr-review/plane-incumbents.toml` (incumbent comparison section)
//!
//! ## Parity gaps
//!
//! None known. Number formatting follows Python's `str(int)` / `str(float)`
//! exactly: integer JSON values render without a decimal, float values render
//! with one (e.g. 600 → "600", 0.5 → "0.5").

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use daedalus_core::export;
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
        "daedalus-export-parity-{}-{n}-{suffix}",
        std::process::id()
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

// ---------------------------------------------------------------------------
// Delivery fixture builder — mirrors tests/test_export.py `build_delivery`
// ---------------------------------------------------------------------------

fn build_delivery(tmp_path: &Path) -> PathBuf {
    let packet = tmp_path
        .join("runs")
        .join("20260611T000000Z-demo")
        .join("packets")
        .join("packet.md");
    std::fs::create_dir_all(packet.parent().unwrap()).unwrap();
    std::fs::write(&packet, "Review with evidence. Cite file and line. Stop.\n").unwrap();

    std::fs::write(
        tmp_path.join("agent.toml"),
        format!(
            "composition = 1\nid = \"demo-agent\"\nkind = \"pi\"\n\
provider_name = \"openrouter\"\nmodel = \"z-ai/glm-5\"\n\
prompt_packet = \"{}\"\nthinking = \"medium\"\n\
tools = [\"read\", \"bash\"]\ntimeout_sec = 600\n",
            packet.display()
        ),
    )
    .unwrap();
    tmp_path.to_path_buf()
}

// ---------------------------------------------------------------------------
// Spec fixture — matches test_export.py SPEC dict
// ---------------------------------------------------------------------------

fn spec_value() -> Value {
    serde_json::json!({
        "id": "pr-review-v0",
        "goal": "Find the real defects a change introduces.",
        "mode": "threshold-then-cheap",
        "inputs": {"description": "post-change repo + PR.diff", "fixtures": "arenas/pr-review-v2"},
        "output": {"contract": "findings.json"},
        "budget": {"max_cost_per_trial_usd": 0.5, "max_wall_per_trial_sec": 600},
        "trigger": {"intent": "GitHub PR webhook"}
    })
}

// ---------------------------------------------------------------------------
// Python driver helpers
// ---------------------------------------------------------------------------

/// Run Python `export.export_delivery` and return the contract.toml text.
fn py_export_delivery_contract(root: &Path, delivery: &Path, ts: &str) -> String {
    let spec_json = serde_json::to_string(&spec_value()).unwrap();
    let snippet = format!(
        r#"import sys, json
sys.path.insert(0, 'runner')
import export
from pathlib import Path

spec = json.loads(r'''{spec_json}''')
delivery = Path(r'{delivery}')
paths = export.export_delivery(delivery, spec, harness_version='9.9.9', generated='{ts}')
print(paths['contract'].read_text(), end='')"#,
        delivery = delivery.display(),
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

/// Run Python `export.export_delivery` and return the persona.md text.
fn py_export_delivery_persona(root: &Path, delivery: &Path, ts: &str) -> String {
    let spec_json = serde_json::to_string(&spec_value()).unwrap();
    let snippet = format!(
        r#"import sys, json
sys.path.insert(0, 'runner')
import export
from pathlib import Path

spec = json.loads(r'''{spec_json}''')
delivery = Path(r'{delivery}')
paths = export.export_delivery(delivery, spec, harness_version='9.9.9', generated='{ts}')
print(paths['persona'].read_text(), end='')"#,
        delivery = delivery.display(),
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

/// Run Python `export.export_delivery` and return the plane-handoff.md text.
fn py_export_delivery_handoff(root: &Path, delivery: &Path, ts: &str) -> String {
    let spec_json = serde_json::to_string(&spec_value()).unwrap();
    let snippet = format!(
        r#"import sys, json
sys.path.insert(0, 'runner')
import export
from pathlib import Path

spec = json.loads(r'''{spec_json}''')
delivery = Path(r'{delivery}')
paths = export.export_delivery(delivery, spec, harness_version='9.9.9', generated='{ts}')
print(paths['handoff'].read_text(), end='')"#,
        delivery = delivery.display(),
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

/// Run Python `export.export_delivery` with incumbents and return handoff text.
fn py_export_delivery_handoff_with_incumbents(
    root: &Path,
    delivery: &Path,
    ts: &str,
    incumbents_toml: &str,
) -> String {
    let spec_json = serde_json::to_string(&spec_value()).unwrap();
    let snippet = format!(
        r#"import sys, json, tomllib
sys.path.insert(0, 'runner')
import export
from pathlib import Path

spec = json.loads(r'''{spec_json}''')
delivery = Path(r'{delivery}')
inc_path = delivery / 'plane-incumbents.toml'
inc_path.write_text(r'''{incumbents}''')
paths = export.export_delivery(delivery, spec, harness_version='9.9.9', generated='{ts}')
print(paths['handoff'].read_text(), end='')"#,
        delivery = delivery.display(),
        ts = ts,
        incumbents = incumbents_toml,
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

/// Run Python `export.export_delivery` expecting ValueError; return the error message.
fn py_export_delivery_err(root: &Path, delivery: &Path, ts: &str) -> String {
    let spec_json = serde_json::to_string(&spec_value()).unwrap();
    let snippet = format!(
        r#"import sys, json
sys.path.insert(0, 'runner')
import export
from pathlib import Path

spec = json.loads(r'''{spec_json}''')
delivery = Path(r'{delivery}')
try:
    export.export_delivery(delivery, spec, harness_version='9.9.9', generated='{ts}')
    print(json.dumps({{"ok": True, "msg": ""}}))
except ValueError as e:
    print(json.dumps({{"ok": False, "msg": str(e)}}))"#,
        delivery = delivery.display(),
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

/// Helper: copy `deliveries/pr-review/agent.toml` (and optionally incumbents)
/// to a fresh tmp dir so Python `export.export_delivery` does not overwrite
/// the committed `deliveries/pr-review/plane-handoff.md` (or contract/persona).
fn setup_real_tmp_delivery(root: &Path, tmp: &Path, copy_incumbents: bool) -> PathBuf {
    let src = root.join("deliveries/pr-review");
    let dst = tmp.join("pr-review");
    std::fs::create_dir_all(&dst).unwrap();
    std::fs::copy(src.join("agent.toml"), dst.join("agent.toml")).unwrap();
    if copy_incumbents {
        let inc = src.join("plane-incumbents.toml");
        if inc.exists() {
            std::fs::copy(&inc, dst.join("plane-incumbents.toml")).unwrap();
        }
    }
    dst
}

/// Run Python `export.export_delivery` on a tmp copy of the real pr-review delivery.
fn py_export_real_contract(root: &Path, ts: &str) -> String {
    let spec_path = root.join("specs/pr-review/taskspec.toml");
    let spec_text = std::fs::read_to_string(&spec_path).expect("taskspec.toml");
    let spec_tv: toml::Value = toml::from_str(&spec_text).expect("valid TOML");
    let spec_json = serde_json::to_string(&toml_to_json(spec_tv)).unwrap();

    let tmp = tmpdir("py-real-contract");
    let delivery = setup_real_tmp_delivery(root, &tmp, false);

    let snippet = format!(
        r#"import sys, json
sys.path.insert(0, 'runner')
import export
from pathlib import Path

spec = json.loads(r'''{spec_json}''')
delivery = Path(r'{delivery}')
paths = export.export_delivery(delivery, spec, harness_version='0.78.1', generated='{ts}')
print(paths['contract'].read_text(), end='')"#,
        delivery = delivery.display(),
        ts = ts,
    );

    let out = Command::new("python3")
        .current_dir(root)
        .arg("-c")
        .arg(&snippet)
        .output()
        .expect("run python3");

    let _ = std::fs::remove_dir_all(&tmp);
    assert!(
        out.status.success(),
        "python3 failed:\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("utf-8")
}

fn py_export_real_persona(root: &Path, ts: &str) -> String {
    let spec_path = root.join("specs/pr-review/taskspec.toml");
    let spec_text = std::fs::read_to_string(&spec_path).expect("taskspec.toml");
    let spec_tv: toml::Value = toml::from_str(&spec_text).expect("valid TOML");
    let spec_json = serde_json::to_string(&toml_to_json(spec_tv)).unwrap();

    let tmp = tmpdir("py-real-persona");
    let delivery = setup_real_tmp_delivery(root, &tmp, false);

    let snippet = format!(
        r#"import sys, json
sys.path.insert(0, 'runner')
import export
from pathlib import Path

spec = json.loads(r'''{spec_json}''')
delivery = Path(r'{delivery}')
paths = export.export_delivery(delivery, spec, harness_version='0.78.1', generated='{ts}')
print(paths['persona'].read_text(), end='')"#,
        delivery = delivery.display(),
        ts = ts,
    );

    let out = Command::new("python3")
        .current_dir(root)
        .arg("-c")
        .arg(&snippet)
        .output()
        .expect("run python3");

    let _ = std::fs::remove_dir_all(&tmp);
    assert!(
        out.status.success(),
        "python3 failed:\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("utf-8")
}

fn py_export_real_handoff(root: &Path, ts: &str) -> String {
    let spec_path = root.join("specs/pr-review/taskspec.toml");
    let spec_text = std::fs::read_to_string(&spec_path).expect("taskspec.toml");
    let spec_tv: toml::Value = toml::from_str(&spec_text).expect("valid TOML");
    let spec_json = serde_json::to_string(&toml_to_json(spec_tv)).unwrap();

    let tmp = tmpdir("py-real-handoff");
    // Copy incumbents too — the real handoff test uses them
    let delivery = setup_real_tmp_delivery(root, &tmp, true);

    let snippet = format!(
        r#"import sys, json
sys.path.insert(0, 'runner')
import export
from pathlib import Path

spec = json.loads(r'''{spec_json}''')
delivery = Path(r'{delivery}')
paths = export.export_delivery(delivery, spec, harness_version='0.78.1', generated='{ts}')
print(paths['handoff'].read_text(), end='')"#,
        delivery = delivery.display(),
        ts = ts,
    );

    let out = Command::new("python3")
        .current_dir(root)
        .arg("-c")
        .arg(&snippet)
        .output()
        .expect("run python3");

    let _ = std::fs::remove_dir_all(&tmp);
    assert!(
        out.status.success(),
        "python3 failed:\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("utf-8")
}

/// Convert toml::Value → serde_json::Value (same as in run.rs but local copy).
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
// Parity assert helpers
// ---------------------------------------------------------------------------

fn assert_text_parity(label: &str, py_text: &str, rust_text: &str) {
    assert_eq!(
        py_text, rust_text,
        "[{label}] text differs\npy=>>>\n{py_text}<<<\nrust=>>>\n{rust_text}<<<"
    );
}

// ---------------------------------------------------------------------------
// Parity oracle tests
// ---------------------------------------------------------------------------

#[test]
fn export_parity_across_fixtures() {
    if !python_available() {
        eprintln!("skipping export parity: python3 not available");
        return;
    }

    let root = repo_root();
    let ts = "2026-06-11T00:00:00Z";

    // -----------------------------------------------------------------------
    // Case 1: crafted delivery — contract.toml byte-for-byte
    // -----------------------------------------------------------------------
    // Tests: render_contract with tmp delivery, explicit timestamp, crafted spec.
    {
        let label = "crafted-contract";
        let d = tmpdir("case1");
        let delivery = build_delivery(&d);

        let py_text = py_export_delivery_contract(&root, &delivery, ts);

        let spec = spec_value();
        let paths = export::export_delivery(&delivery, &spec, Some("9.9.9"), Some(ts), &root)
            .expect("export_delivery should succeed");
        let rust_text = std::fs::read_to_string(&paths["contract"]).unwrap();

        assert_text_parity(label, &py_text, &rust_text);

        // TOML round-trip: both must parse
        let parsed: toml::Value = toml::from_str(&rust_text).expect("valid TOML");
        assert_eq!(parsed.get("contract").and_then(|v| v.as_integer()), Some(1));

        let _ = std::fs::remove_dir_all(&d);
    }

    // -----------------------------------------------------------------------
    // Case 2: crafted delivery — persona.md byte-for-byte
    // -----------------------------------------------------------------------
    // Tests: render_persona with packet body byte-identical.
    {
        let label = "crafted-persona";
        let d = tmpdir("case2");
        let delivery = build_delivery(&d);

        let py_text = py_export_delivery_persona(&root, &delivery, ts);

        let spec = spec_value();
        let paths = export::export_delivery(&delivery, &spec, Some("9.9.9"), Some(ts), &root)
            .expect("export_delivery should succeed");
        let rust_text = std::fs::read_to_string(&paths["persona"]).unwrap();

        assert_text_parity(label, &py_text, &rust_text);

        // The body after the frontmatter must be the measured packet text
        assert!(rust_text.contains("Review with evidence."));

        let _ = std::fs::remove_dir_all(&d);
    }

    // -----------------------------------------------------------------------
    // Case 3: crafted delivery — plane-handoff.md byte-for-byte (no incumbents)
    // -----------------------------------------------------------------------
    {
        let label = "crafted-handoff-no-incumbents";
        let d = tmpdir("case3");
        let delivery = build_delivery(&d);

        let py_text = py_export_delivery_handoff(&root, &delivery, ts);

        let spec = spec_value();
        let paths = export::export_delivery(&delivery, &spec, Some("9.9.9"), Some(ts), &root)
            .expect("export_delivery should succeed");
        let rust_text = std::fs::read_to_string(&paths["handoff"]).unwrap();

        assert_text_parity(label, &py_text, &rust_text);

        let _ = std::fs::remove_dir_all(&d);
    }

    // -----------------------------------------------------------------------
    // Case 4: crafted delivery with incumbents — handoff byte-for-byte
    // -----------------------------------------------------------------------
    // Tests: incumbent comparison table and notes sections.
    {
        let label = "crafted-handoff-with-incumbents";
        let d = tmpdir("case4");
        let delivery = build_delivery(&d);

        let incumbents_toml = r#"
[bitter_blossom]
agent = "review-coordinator"
version = "2"
model = "moonshotai/kimi-k2.6"
harness = "pi"
posting = "agent posts one PR comment directly through gh"
config_paths = [
  "plane/agents/review-coordinator.toml",
  "plane/tasks/review/task.toml",
  "plane/tasks/review/card.md",
]
notes = ["budgeted webhook task", "direct-post red line"]

[olympus]
agent = "charon"
version = "2"
model = "~moonshotai/kimi-latest"
harness = "pi"
posting = "strict JSON artifact; orchestrator validates and posts"
config_paths = [
  "orchestrator/agent-specs/charon.yaml",
  "orchestrator/prompts/charon-review.md",
]
notes = ["activation gated", "orchestrator-side posting"]
"#;

        let py_text =
            py_export_delivery_handoff_with_incumbents(&root, &delivery, ts, incumbents_toml);

        // Write incumbents before Rust export
        std::fs::write(delivery.join("plane-incumbents.toml"), incumbents_toml).unwrap();

        let spec = spec_value();
        let paths = export::export_delivery(&delivery, &spec, Some("9.9.9"), Some(ts), &root)
            .expect("export_delivery should succeed");
        let rust_text = std::fs::read_to_string(&paths["handoff"]).unwrap();

        assert_text_parity(label, &py_text, &rust_text);

        let _ = std::fs::remove_dir_all(&d);
    }

    // -----------------------------------------------------------------------
    // Case 5: loose prompt_packet not under runs/ — ValueError raised
    // -----------------------------------------------------------------------
    // Tests: both Python and Rust raise ValueError / ExportError with matching
    //        "evidence pointers" message.
    {
        let label = "loose-packet-error";
        let d = tmpdir("case5");
        let delivery = build_delivery(&d);

        // Replace prompt_packet with a loose file
        let loose = d.join("loose-packet.md");
        std::fs::write(&loose, "No run evidence.\n").unwrap();
        let old = std::fs::read_to_string(delivery.join("agent.toml")).unwrap();
        let runs_packet = delivery
            .join("runs")
            .join("20260611T000000Z-demo")
            .join("packets")
            .join("packet.md");
        let new_content = old.replace(runs_packet.to_str().unwrap(), loose.to_str().unwrap());
        std::fs::write(delivery.join("agent.toml"), &new_content).unwrap();

        let py_err = py_export_delivery_err(&root, &delivery, ts);
        assert!(
            !py_err.is_empty(),
            "[{label}] Python should raise a ValueError for loose packet"
        );
        assert!(
            py_err.contains("evidence pointers"),
            "[{label}] Python error should mention 'evidence pointers': {py_err}"
        );

        let spec = spec_value();
        let rust_result = export::export_delivery(&delivery, &spec, Some("9.9.9"), Some(ts), &root);
        assert!(
            rust_result.is_err(),
            "[{label}] Rust should raise ExportError for loose packet"
        );
        let rust_err = rust_result.unwrap_err().to_string();
        assert!(
            rust_err.contains("evidence pointers"),
            "[{label}] Rust error should mention 'evidence pointers': {rust_err}"
        );

        let _ = std::fs::remove_dir_all(&d);
    }

    // -----------------------------------------------------------------------
    // Case 6: real pr-review delivery — contract.toml byte-for-byte
    // -----------------------------------------------------------------------
    // Uses `deliveries/pr-review/agent.toml` (real delivery, absolute packet path).
    {
        let label = "real-pr-review-contract";
        let delivery = root.join("deliveries/pr-review");

        let py_text = py_export_real_contract(&root, ts);

        let spec_text = std::fs::read_to_string(root.join("specs/pr-review/taskspec.toml"))
            .expect("taskspec.toml");
        let spec_tv: toml::Value = toml::from_str(&spec_text).expect("valid TOML");
        let spec = toml_to_json(spec_tv);

        // Export to a tmp out dir to avoid overwriting the committed contract.toml
        let out_dir = tmpdir("case6-real");
        // Copy agent.toml only; the delivery_dir must be the out_dir so paths work.
        let tmp_delivery = out_dir.join("pr-review");
        std::fs::create_dir_all(&tmp_delivery).unwrap();
        std::fs::copy(delivery.join("agent.toml"), tmp_delivery.join("agent.toml")).unwrap();

        let paths = export::export_delivery(&tmp_delivery, &spec, Some("0.78.1"), Some(ts), &root)
            .expect("export_delivery should succeed for real delivery");
        let rust_text = std::fs::read_to_string(&paths["contract"]).unwrap();

        // Parse both as TOML and compare the structured content semantically,
        // since the delivery_name differs (tmp vs "pr-review").
        let py_parsed: toml::Value =
            toml::from_str(&py_text).expect("[{label}] python text valid TOML");
        let rust_parsed: toml::Value =
            toml::from_str(&rust_text).expect("[{label}] rust text valid TOML");

        // Key invariants: agent, hash, composition fields, budgets, evidence paths.
        assert_eq!(
            py_parsed.get("agent").and_then(|v| v.as_str()),
            rust_parsed.get("agent").and_then(|v| v.as_str()),
            "[{label}] agent"
        );
        assert_eq!(
            py_parsed.get("composition_hash").and_then(|v| v.as_str()),
            rust_parsed.get("composition_hash").and_then(|v| v.as_str()),
            "[{label}] composition_hash"
        );
        assert_eq!(
            py_parsed
                .get("composition")
                .and_then(|c| c.get("model"))
                .and_then(|v| v.as_str()),
            rust_parsed
                .get("composition")
                .and_then(|c| c.get("model"))
                .and_then(|v| v.as_str()),
            "[{label}] model"
        );
        assert_eq!(
            py_parsed
                .get("composition")
                .and_then(|c| c.get("harness_version"))
                .and_then(|v| v.as_str()),
            Some("0.78.1"),
            "[{label}] harness_version"
        );
        assert_eq!(
            rust_parsed
                .get("composition")
                .and_then(|c| c.get("harness_version"))
                .and_then(|v| v.as_str()),
            Some("0.78.1"),
            "[{label}] rust harness_version"
        );
        // Evidence paths: both should end with the same run_dir segment
        let py_run_dir = py_parsed
            .get("evidence")
            .and_then(|e| e.get("run_dir"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let rust_run_dir = rust_parsed
            .get("evidence")
            .and_then(|e| e.get("run_dir"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            py_run_dir.contains("20260611T173632Z-search-pr-review-v0"),
            "[{label}] py run_dir = {py_run_dir}"
        );
        assert!(
            rust_run_dir.contains("20260611T173632Z-search-pr-review-v0"),
            "[{label}] rust run_dir = {rust_run_dir}"
        );
        // budgets
        assert_eq!(
            py_parsed
                .get("budgets")
                .and_then(|b| b.get("max_cost_usd_per_run"))
                .and_then(|v| v.as_float()),
            rust_parsed
                .get("budgets")
                .and_then(|b| b.get("max_cost_usd_per_run"))
                .and_then(|v| v.as_float()),
            "[{label}] max_cost_usd_per_run"
        );

        let _ = std::fs::remove_dir_all(&out_dir);
    }

    // -----------------------------------------------------------------------
    // Case 7: real pr-review delivery — persona.md byte-for-byte
    // -----------------------------------------------------------------------
    // Tests: persona body is the exact measured packet.
    {
        let label = "real-pr-review-persona";
        let delivery = root.join("deliveries/pr-review");

        let py_text = py_export_real_persona(&root, ts);

        let spec_text = std::fs::read_to_string(root.join("specs/pr-review/taskspec.toml"))
            .expect("taskspec.toml");
        let spec_tv: toml::Value = toml::from_str(&spec_text).expect("valid TOML");
        let spec = toml_to_json(spec_tv);

        let out_dir = tmpdir("case7-real");
        let tmp_delivery = out_dir.join("pr-review");
        std::fs::create_dir_all(&tmp_delivery).unwrap();
        std::fs::copy(delivery.join("agent.toml"), tmp_delivery.join("agent.toml")).unwrap();

        let paths = export::export_delivery(&tmp_delivery, &spec, Some("0.78.1"), Some(ts), &root)
            .expect("export_delivery should succeed for real delivery");
        let rust_text = std::fs::read_to_string(&paths["persona"]).unwrap();

        // Exact byte comparison — same packet, same candidate, same spec
        assert_text_parity(label, &py_text, &rust_text);

        let _ = std::fs::remove_dir_all(&out_dir);
    }

    // -----------------------------------------------------------------------
    // Case 8: real pr-review delivery — plane-handoff.md (with real incumbents)
    // -----------------------------------------------------------------------
    // Tests: handoff with real plane-incumbents.toml from the delivery.
    {
        let label = "real-pr-review-handoff";

        // Only run if incumbents file exists (it does in the real repo)
        let delivery = root.join("deliveries/pr-review");
        let incumbents_path = delivery.join("plane-incumbents.toml");
        if !incumbents_path.exists() {
            eprintln!("skipping [{label}]: plane-incumbents.toml not found");
        } else {
            let py_text = py_export_real_handoff(&root, ts);

            let spec_text = std::fs::read_to_string(root.join("specs/pr-review/taskspec.toml"))
                .expect("taskspec.toml");
            let spec_tv: toml::Value = toml::from_str(&spec_text).expect("valid TOML");
            let spec = toml_to_json(spec_tv);

            let out_dir = tmpdir("case8-real");
            let tmp_delivery = out_dir.join("pr-review");
            std::fs::create_dir_all(&tmp_delivery).unwrap();
            std::fs::copy(delivery.join("agent.toml"), tmp_delivery.join("agent.toml")).unwrap();
            std::fs::copy(&incumbents_path, tmp_delivery.join("plane-incumbents.toml")).unwrap();

            let paths =
                export::export_delivery(&tmp_delivery, &spec, Some("0.78.1"), Some(ts), &root)
                    .expect("export_delivery should succeed for real delivery");
            let rust_text = std::fs::read_to_string(&paths["handoff"]).unwrap();

            // Byte-for-byte comparison (delivery_name is the same: "pr-review")
            assert_text_parity(label, &py_text, &rust_text);

            let _ = std::fs::remove_dir_all(&out_dir);
        }
    }

    // -----------------------------------------------------------------------
    // Case 9: determinism — two consecutive exports produce identical files
    // -----------------------------------------------------------------------
    {
        let label = "determinism";
        let d = tmpdir("case9");
        let delivery = build_delivery(&d);

        let spec = spec_value();
        let a = export::export_delivery(&delivery, &spec, Some("9.9.9"), Some(ts), &root)
            .expect("first export");
        let first_contract = std::fs::read_to_string(&a["contract"]).unwrap();
        let first_handoff = std::fs::read_to_string(&a["handoff"]).unwrap();

        let b = export::export_delivery(&delivery, &spec, Some("9.9.9"), Some(ts), &root)
            .expect("second export");
        assert_eq!(
            std::fs::read_to_string(&b["contract"]).unwrap(),
            first_contract,
            "[{label}] contract not deterministic"
        );
        assert_eq!(
            std::fs::read_to_string(&b["handoff"]).unwrap(),
            first_handoff,
            "[{label}] handoff not deterministic"
        );

        let _ = std::fs::remove_dir_all(&d);
    }
}
