//! Parity oracle for the seed port: run the real Python `seed.sample_compositions`
//! and the Rust port with the same `random.Random(SEED)` / `PyRandom::new(SEED)`
//! and assert the sampled compositions agree.
//!
//! Only `sample_compositions` is parity-tested here (the deterministic core).
//! `author_packets` / `build_seeds` / `seed_population` involve file I/O and
//! optional LLM calls; their observable file-format output is unit-tested via
//! injected `call` in `seed::tests`.
//!
//! Skips (does not fail) when python3 is unavailable.

use std::path::{Path, PathBuf};
use std::process::Command;

use daedalus_core::pyrandom::PyRandom;
use daedalus_core::seed::{sample_compositions, STANCES};
use serde_json::{json, Value};

fn python_available() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root above crates/daedalus-core")
        .to_path_buf()
}

/// Python driver: given a JSON payload `{search, n, seed}` on stdin,
/// print `json.dumps(sample_compositions(search, n, rng))` to stdout.
const PY_DRIVER: &str = r#"
import sys, json, random
sys.path.insert(0, 'runner')
import seed

payload = json.load(sys.stdin)
search = payload["search"]
n = payload["n"]
rng_seed = payload["seed"]
rng = random.Random(rng_seed)
combos = seed.sample_compositions(search, n, rng)
print(json.dumps(combos))
"#;

/// Run the Python driver with the given search, n, and seed; return the list
/// of combo dicts as a `serde_json::Value` array.
fn py_sample(search: &Value, n: usize, seed: u64) -> Value {
    use std::io::Write;
    use std::process::Stdio;

    let payload = json!({"search": search, "n": n, "seed": seed});
    let mut child = Command::new("python3")
        .current_dir(repo_root())
        .arg("-c")
        .arg(PY_DRIVER)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn python3");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(serde_json::to_string(&payload).unwrap().as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "python driver failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("python driver emitted valid JSON")
}

/// Convert a `Composition` to the same JSON shape as the Python dict.
fn composition_to_value(c: &daedalus_core::seed::Composition) -> Value {
    json!({
        "model": c.model,
        "thinking": c.thinking,
        "policy_name": c.policy_name,
        "tools": c.tools,
        "system_prompt_mode": c.system_prompt_mode,
        "skill_set_name": c.skill_set_name,
        "skills": c.skills,
        "agents_md": c.agents_md,
    })
}

/// Assert that the Rust `sample_compositions` output matches Python's for given
/// search space, n, and seed.
fn check(label: &str, search: &Value, n: usize, seed: u64) {
    let search_obj = search.as_object().unwrap().clone();
    let py_combos = py_sample(search, n, seed);
    let py_arr = py_combos.as_array().unwrap();

    let mut rng = PyRandom::new(seed);
    let rust_combos = sample_compositions(&search_obj, n, &mut rng);

    assert_eq!(
        rust_combos.len(),
        py_arr.len(),
        "[{label}] combo count differs (seed={seed})"
    );

    for (i, (py, rust)) in py_arr.iter().zip(rust_combos.iter()).enumerate() {
        let rust_v = composition_to_value(rust);
        assert_eq!(
            py, &rust_v,
            "[{label}] combo[{i}] differs (seed={seed})\npy={py}\nrust={rust_v}"
        );
    }
}

// ---------------------------------------------------------------------------
// The standard SEARCH space from tests/test_seed.py
// ---------------------------------------------------------------------------

fn standard_search() -> Value {
    json!({
        "models": [
            "deepseek/deepseek-v4-flash",
            "z-ai/glm-4.7-flash",
            "openai/gpt-5-mini",
            "moonshotai/kimi-k2.6",
            "z-ai/glm-5",
            "qwen/qwen3.7-plus"
        ],
        "thinking_levels": ["off", "low", "medium", "high"],
        "tool_policies": {
            "full": ["read", "bash", "edit", "write"],
            "explore": ["read", "bash"],
            "no-exec": ["read", "edit", "write"]
        },
        "packet_stances": 3,
        "seed_count": 6
    })
}

// ---------------------------------------------------------------------------
// Parity tests
// ---------------------------------------------------------------------------

#[test]
fn sample_compositions_parity_standard_seeds() {
    if !python_available() {
        eprintln!("skipping seed parity: python3 not available");
        return;
    }

    let search = standard_search();

    // Mirror the seeds from test_seed.py::test_sampling_spans_the_axes_and_is_deterministic
    check("standard-seed-7", &search, 6, 7);
    check("standard-seed-8", &search, 6, 8);
    check("standard-seed-42", &search, 6, 42);

    // Additional seeds for breadth
    check("standard-seed-0", &search, 6, 0);
    check("standard-seed-1", &search, 6, 1);
    check("standard-seed-999", &search, 6, 999);
    check("standard-seed-2pow31", &search, 6, 2u64.pow(31));
    check("standard-seed-2pow32minus1", &search, 6, (2u64.pow(32)) - 1);
}

#[test]
fn sample_compositions_parity_varying_n() {
    if !python_available() {
        eprintln!("skipping seed parity: python3 not available");
        return;
    }

    let search = standard_search();
    const SEED: u64 = 42;

    // Small n
    check("n=1", &search, 1, SEED);
    check("n=2", &search, 2, SEED);

    // n == axes length
    check("n=6", &search, 6, SEED);

    // n > axes length — cycling kicks in
    check("n=10", &search, 10, SEED);
    check("n=20", &search, 20, SEED);
}

#[test]
fn sample_compositions_parity_single_value_axes() {
    if !python_available() {
        eprintln!("skipping seed parity: python3 not available");
        return;
    }

    // All axes have one value — shuffling a length-1 list is a no-op.
    let search = json!({
        "models": ["deepseek/deepseek-v4-flash"],
        "thinking_levels": ["medium"],
        "tool_policies": {
            "full": ["read", "bash", "edit", "write"]
        }
    });

    check("single-value-seed-0", &search, 4, 0);
    check("single-value-seed-7", &search, 4, 7);
    check("single-value-seed-42", &search, 4, 42);
}

#[test]
fn sample_compositions_parity_default_axes() {
    if !python_available() {
        eprintln!("skipping seed parity: python3 not available");
        return;
    }

    // Only models declared — thinking, policies, sp_modes etc. use defaults.
    let search = json!({
        "models": ["openai/gpt-5-mini", "z-ai/glm-5"]
    });

    check("defaults-seed-0", &search, 3, 0);
    check("defaults-seed-42", &search, 3, 42);
    check("defaults-seed-100", &search, 6, 100);
}

#[test]
fn sample_compositions_parity_optional_axes() {
    if !python_available() {
        eprintln!("skipping seed parity: python3 not available");
        return;
    }

    // With system_prompt_modes, skill_sets, and agents_md_options declared.
    let search = json!({
        "models": [
            "deepseek/deepseek-v4-flash",
            "z-ai/glm-4.7-flash",
            "openai/gpt-5-mini"
        ],
        "thinking_levels": ["low", "medium", "high"],
        "tool_policies": {
            "full": ["read", "bash", "edit", "write"],
            "explore": ["read", "bash"]
        },
        "system_prompt_modes": ["append", "replace"],
        "skill_sets": {
            "pack": ["/path/to/skill-a.md", "/path/to/skill-b.md"],
            "bare": []
        },
        "agents_md_options": ["/path/to/agents.md", null]
    });

    check("optional-seed-3", &search, 6, 3);
    check("optional-seed-7", &search, 6, 7);
    check("optional-seed-42", &search, 6, 42);
}

#[test]
fn sample_compositions_parity_large_n_cycling() {
    if !python_available() {
        eprintln!("skipping seed parity: python3 not available");
        return;
    }

    // Large n exercises the modulo cycling thoroughly.
    let search = json!({
        "models": ["a/model-1", "b/model-2", "c/model-3"],
        "thinking_levels": ["off", "medium"],
        "tool_policies": {
            "p1": ["read"],
            "p2": ["bash"]
        }
    });

    check("large-n-seed-0", &search, 50, 0);
    check("large-n-seed-17", &search, 50, 17);
}

#[test]
fn stances_constant_matches_python() {
    // Verify the STANCES constant exactly matches the Python module-level list
    // so the parity driver can look up stances by brief text.
    if !python_available() {
        eprintln!("skipping stances parity: python3 not available");
        return;
    }

    let py_stances_json = {
        use std::process::Stdio;
        let child = Command::new("python3")
            .current_dir(repo_root())
            .arg("-c")
            .arg("import sys, json; sys.path.insert(0,'runner'); import seed; print(json.dumps(seed.STANCES))")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(
            out.status.success(),
            "python stances failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice::<Value>(&out.stdout).unwrap()
    };

    let py_arr = py_stances_json.as_array().unwrap();
    assert_eq!(py_arr.len(), STANCES.len(), "STANCES length mismatch");
    for (i, (py_item, (rust_name, rust_brief))) in py_arr.iter().zip(STANCES.iter()).enumerate() {
        let py_pair = py_item.as_array().unwrap();
        let py_name = py_pair[0].as_str().unwrap();
        let py_brief = py_pair[1].as_str().unwrap();
        assert_eq!(
            py_name, *rust_name,
            "STANCES[{i}] name mismatch: py={py_name} rust={rust_name}"
        );
        assert_eq!(
            py_brief, *rust_brief,
            "STANCES[{i}] brief mismatch:\npy={py_brief}\nrust={rust_brief}"
        );
    }
}
