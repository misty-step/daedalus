//! Parity oracle for the port_harbor port.
//!
//! Runs the original Python `runner/port_harbor.port_task` and the Rust
//! [`daedalus_core::port_harbor::port_task`] over identical arena + task
//! inputs, then asserts every produced file is byte-identical (relative paths
//! AND content). Covers:
//!   - a real task from the `pr-review-v0` arena with default timeouts
//!   - a second real task from the same arena
//!   - a crafted task with non-default timeouts (agent=300, verifier=30)
//!
//! Skips gracefully when `python3` is unavailable, mirroring `bin/gate`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use daedalus_core::port_harbor::port_task;
use toml::Value as TomlValue;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

static COUNTER: AtomicU64 = AtomicU64::new(0);

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

/// Create a fresh temp directory unique per test invocation.
fn tmpdir(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "daedalus-harbor-parity-{}-{n}-{label}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Run Python `port_harbor.port_task(arena_dir, arena, task_dir, out_dir)`.
/// The arena dict is re-parsed by Python from `arena_dir/arena.toml`.
fn py_port_task(root: &Path, arena_dir: &Path, task_dir: &Path, out_dir: &Path) {
    let script = "import sys, tomllib; from pathlib import Path; \
         sys.path.insert(0, 'runner'); \
         import port_harbor; \
         arena_dir = Path(sys.argv[1]); \
         task_dir = Path(sys.argv[2]); \
         out_dir = Path(sys.argv[3]); \
         arena = tomllib.loads((arena_dir / 'arena.toml').read_text()); \
         port_harbor.port_task(arena_dir, arena, task_dir, out_dir)";
    let out = Command::new("python3")
        .current_dir(root)
        .arg("-c")
        .arg(script)
        .arg(arena_dir)
        .arg(task_dir)
        .arg(out_dir)
        .output()
        .expect("run python3");
    assert!(
        out.status.success(),
        "python port_task failed:\nstderr: {}\nstdout: {}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout),
    );
}

/// Recursively collect all files under `dir` as a map from relative path
/// (slash-separated) to file bytes.
fn collect_files(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut map = BTreeMap::new();
    collect_files_inner(dir, dir, &mut map);
    map
}

fn collect_files_inner(root: &Path, dir: &Path, map: &mut BTreeMap<String, Vec<u8>>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let ft = entry.file_type().unwrap();
        if ft.is_dir() {
            collect_files_inner(root, &path, map);
        } else {
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_slash_lossy()
                .into_owned();
            let bytes = std::fs::read(&path).unwrap();
            map.insert(rel, bytes);
        }
    }
}

/// Assert both output trees have the same files with identical contents.
fn assert_byte_parity(label: &str, py_dir: &Path, rust_dir: &Path) {
    let py_files = collect_files(py_dir);
    let rust_files = collect_files(rust_dir);

    // Same set of relative paths
    let py_paths: Vec<&str> = py_files.keys().map(String::as_str).collect();
    let rust_paths: Vec<&str> = rust_files.keys().map(String::as_str).collect();
    assert_eq!(
        py_paths, rust_paths,
        "[{label}] file tree paths differ\npy:   {py_paths:?}\nrust: {rust_paths:?}"
    );

    // Byte-identical contents for each file
    for rel in py_files.keys() {
        let py_bytes = &py_files[rel];
        let rust_bytes = &rust_files[rel];
        assert_eq!(
            py_bytes,
            rust_bytes,
            "[{label}] file '{rel}' differs\npy  ({} bytes): {}\nrust({} bytes): {}",
            py_bytes.len(),
            String::from_utf8_lossy(py_bytes),
            rust_bytes.len(),
            String::from_utf8_lossy(rust_bytes),
        );
    }
}

/// Load arena TOML from `<arena_dir>/arena.toml`.
fn load_arena(arena_dir: &Path) -> TomlValue {
    let text = std::fs::read_to_string(arena_dir.join("arena.toml")).expect("read arena.toml");
    text.parse().expect("parse arena.toml")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Case 1: real task `js-cart-total` from pr-review-v0 (default timeouts:
/// agent=600, verifier=60).
#[test]
fn parity_real_task_js_cart_total() {
    if !python_available() {
        eprintln!("skipping harbor parity: python3 not available");
        return;
    }
    let root = repo_root();
    let arena_dir = root.join("arenas/pr-review-v0");
    let task_dir = arena_dir.join("tasks/js-cart-total");
    if !task_dir.exists() {
        eprintln!("skipping: task fixture not in checkout");
        return;
    }

    let py_out = tmpdir("py-js-cart-total");
    let rust_out = tmpdir("rs-js-cart-total");

    py_port_task(&root, &arena_dir, &task_dir, &py_out);

    let arena = load_arena(&arena_dir);
    port_task(&arena_dir, &arena, &task_dir, &rust_out, &root).expect("rust port_task");

    assert_byte_parity("js-cart-total", &py_out, &rust_out);

    let _ = std::fs::remove_dir_all(&py_out);
    let _ = std::fs::remove_dir_all(&rust_out);
}

/// Case 2: real task `py-auth-sqli` from pr-review-v0 (default timeouts).
#[test]
fn parity_real_task_py_auth_sqli() {
    if !python_available() {
        eprintln!("skipping harbor parity: python3 not available");
        return;
    }
    let root = repo_root();
    let arena_dir = root.join("arenas/pr-review-v0");
    let task_dir = arena_dir.join("tasks/py-auth-sqli");
    if !task_dir.exists() {
        eprintln!("skipping: task fixture not in checkout");
        return;
    }

    let py_out = tmpdir("py-py-auth-sqli");
    let rust_out = tmpdir("rs-py-auth-sqli");

    py_port_task(&root, &arena_dir, &task_dir, &py_out);

    let arena = load_arena(&arena_dir);
    port_task(&arena_dir, &arena, &task_dir, &rust_out, &root).expect("rust port_task");

    assert_byte_parity("py-auth-sqli", &py_out, &rust_out);

    let _ = std::fs::remove_dir_all(&py_out);
    let _ = std::fs::remove_dir_all(&rust_out);
}

/// Case 3: crafted task with non-default timeouts (agent=300, verifier=30).
/// Also covers the float formatting invariant for integer-valued floats.
#[test]
fn parity_crafted_task_custom_timeouts() {
    if !python_available() {
        eprintln!("skipping harbor parity: python3 not available");
        return;
    }
    let root = repo_root();

    // Build a minimal arena + task directory tree in temp
    let tmp = tmpdir("crafted-arena");
    let arena_dir = tmp.join("arena");
    let task_dir = arena_dir.join("tasks/custom-task");
    let env_src = task_dir.join("environment");
    let tests_src = task_dir.join("tests");
    let sol_src = task_dir.join("solution");

    std::fs::create_dir_all(&arena_dir).unwrap();
    std::fs::create_dir_all(&env_src).unwrap();
    std::fs::create_dir_all(&tests_src).unwrap();
    std::fs::create_dir_all(&sol_src).unwrap();

    // arena.toml
    std::fs::write(
        arena_dir.join("arena.toml"),
        "id = \"test-arena\"\nversion = \"0.1.0\"\n[template]\nfile = \"template.md\"\n",
    )
    .unwrap();

    // template.md
    std::fs::write(
        arena_dir.join("template.md"),
        "# Task\nIntent: {intent}\nDo the thing.\n",
    )
    .unwrap();

    // task.toml — non-default timeouts
    std::fs::write(
        task_dir.join("task.toml"),
        "id = \"custom-task\"\n\n[agent]\ntimeout_sec = 300\n\n[verifier]\ntimeout_sec = 30\n",
    )
    .unwrap();

    // intent.md
    std::fs::write(task_dir.join("intent.md"), "  find the bug  \n").unwrap();

    // environment/: one file
    std::fs::write(env_src.join("main.py"), "print('hello')\n").unwrap();

    // tests/expected.json
    std::fs::write(tests_src.join("expected.json"), "{\"defects\":[]}\n").unwrap();

    // solution/findings.json
    std::fs::write(sol_src.join("findings.json"), "{\"findings\":[]}\n").unwrap();

    let py_out = tmpdir("py-crafted");
    let rust_out = tmpdir("rs-crafted");

    py_port_task(&root, &arena_dir, &task_dir, &py_out);

    let arena = load_arena(&arena_dir);
    port_task(&arena_dir, &arena, &task_dir, &rust_out, &root).expect("rust port_task crafted");

    assert_byte_parity("crafted-custom-timeouts", &py_out, &rust_out);

    // Verify the float formatting in the generated task.toml
    let task_toml_bytes = std::fs::read(rust_out.join("task.toml")).unwrap();
    let task_toml_text = String::from_utf8(task_toml_bytes).unwrap();
    assert!(
        task_toml_text.contains("timeout_sec = 300.0"),
        "expected '300.0', got:\n{task_toml_text}"
    );
    assert!(
        task_toml_text.contains("timeout_sec = 30.0"),
        "expected '30.0', got:\n{task_toml_text}"
    );

    // Verify instruction.md intent substitution (intent stripped)
    let instruction = std::fs::read_to_string(rust_out.join("instruction.md")).unwrap();
    assert!(
        instruction.contains("Intent: find the bug"),
        "intent not substituted correctly:\n{instruction}"
    );

    let _ = std::fs::remove_dir_all(&tmp);
    let _ = std::fs::remove_dir_all(&py_out);
    let _ = std::fs::remove_dir_all(&rust_out);
}

/// Case 4: verify that idempotent re-runs overwrite existing output correctly
/// (existing environment/, tests/, solution/ are removed and recreated).
#[test]
fn parity_idempotent_rerun() {
    if !python_available() {
        eprintln!("skipping harbor parity: python3 not available");
        return;
    }
    let root = repo_root();
    let arena_dir = root.join("arenas/pr-review-v0");
    let task_dir = arena_dir.join("tasks/js-cart-total");
    if !task_dir.exists() {
        eprintln!("skipping: task fixture not in checkout");
        return;
    }

    let py_out = tmpdir("py-idempotent");
    let rust_out = tmpdir("rs-idempotent");

    // Run Python twice — second run should overwrite
    py_port_task(&root, &arena_dir, &task_dir, &py_out);
    py_port_task(&root, &arena_dir, &task_dir, &py_out);

    // Run Rust twice
    let arena = load_arena(&arena_dir);
    port_task(&arena_dir, &arena, &task_dir, &rust_out, &root).expect("rust run 1");
    port_task(&arena_dir, &arena, &task_dir, &rust_out, &root).expect("rust run 2");

    assert_byte_parity("idempotent", &py_out, &rust_out);

    let _ = std::fs::remove_dir_all(&py_out);
    let _ = std::fs::remove_dir_all(&rust_out);
}

// ---------------------------------------------------------------------------
// Helper trait for to_slash_lossy
// ---------------------------------------------------------------------------

trait ToSlashLossy {
    fn to_slash_lossy(&self) -> std::borrow::Cow<'_, str>;
}

impl ToSlashLossy for Path {
    fn to_slash_lossy(&self) -> std::borrow::Cow<'_, str> {
        #[cfg(windows)]
        {
            std::borrow::Cow::Owned(self.to_string_lossy().replace('\\', "/"))
        }
        #[cfg(not(windows))]
        {
            self.to_string_lossy()
        }
    }
}
