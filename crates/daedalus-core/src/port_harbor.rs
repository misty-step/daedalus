//! Harbor task-format adapter.
//!
//! Port of `runner/port_harbor.py`. Reads `task.toml`/`arena.toml` with the
//! `toml` crate and writes a complete Harbor build directory by string
//! templating — byte-for-byte identical to the Python reference implementation.
//!
//! Entry point: [`port_task`].

use std::fs;
use std::path::Path;

use toml::Value as TomlValue;

// ---------------------------------------------------------------------------
// Constant template strings — must match `runner/port_harbor.py` exactly.
// ---------------------------------------------------------------------------

const DOCKERFILE: &str = "\
FROM python:3.12-slim
WORKDIR /app
COPY . /app/
";

const DOCKERIGNORE: &str = "Dockerfile\n.dockerignore\n";

const TEST_SH: &str = "\
#!/bin/bash
# Daedalus verifier: score findings.json against the answer key and emit the
# Harbor reward file.
set -u
/tests/daedalus-score /app/findings.json /tests/expected.json \\
    > /logs/verifier/score.json
python3 -c 'import json,sys; print(json.load(sys.stdin)[\"reward\"])' \\
    < /logs/verifier/score.json > /logs/verifier/reward.txt
";

const SOLVE_SH: &str = "\
#!/bin/bash
# Oracle: replay the reference findings.
cp /solution/findings.json /app/findings.json
";

const TASK_TOML_TEMPLATE: &str = "\
version = \"1.0\"

[agent]
timeout_sec = {agent_timeout}

[verifier]
timeout_sec = {verifier_timeout}
";

// ---------------------------------------------------------------------------
// Scorer binary: port_task accepts the prebuilt `daedalus-score` musl binary
// path from the caller and copies it into tests/. The caller (daedalus CLI
// port-harbor subcommand or bin/harbor-run) must build it first.
// ---------------------------------------------------------------------------

/// Render the arena instruction for a task by substituting `{intent}` in the
/// template file.
///
/// Mirrors Python:
/// ```python
/// def render_instruction(arena_dir, arena, task_dir):
///     template = (arena_dir / arena["template"]["file"]).read_text()
///     intent = (task_dir / "intent.md").read_text().strip()
///     return template.replace("{intent}", intent)
/// ```
pub fn render_instruction(
    arena_dir: &Path,
    arena: &TomlValue,
    task_dir: &Path,
) -> Result<String, String> {
    let template_file = arena
        .get("template")
        .and_then(|t| t.get("file"))
        .and_then(|f| f.as_str())
        .ok_or_else(|| "arena missing template.file".to_string())?;
    let template = fs::read_to_string(arena_dir.join(template_file))
        .map_err(|e| format!("read template: {e}"))?;
    let intent = fs::read_to_string(task_dir.join("intent.md"))
        .map_err(|e| format!("read intent.md: {e}"))?;
    let intent = intent.trim();
    Ok(template.replace("{intent}", intent))
}

/// Format a timeout value as Python's `str(float(x))` does:
/// `float(600) → "600.0"`, `float(30.5) → "30.5"`.
///
/// Python `str(float(n))` for an integer-valued float always has a trailing
/// `.0`; Rust's default Display omits it (e.g. `600.0_f64` formats as `"600"`).
/// We detect integer-valued floats and append `.0` to match Python.
fn format_py_float(v: f64) -> String {
    // Python's str(float(x)) always has at least one decimal digit.
    // For finite integer-valued floats this means a trailing ".0".
    // For non-integer floats the minimal representation already has a decimal.
    if v.is_finite() && v.fract() == 0.0 {
        format!("{v}.0")
    } else {
        // Non-integer: Rust's Display gives the same minimal decimal as Python.
        format!("{v}")
    }
}

/// Recursively copy `src` to `dst`, creating `dst` and all intermediate
/// directories. Mirrors Python `shutil.copytree(src, dst)`.
fn copy_tree(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("create_dir_all {}: {e}", dst.display()))?;
    for entry in fs::read_dir(src).map_err(|e| format!("read_dir {}: {e}", src.display()))? {
        let entry = entry.map_err(|e| format!("dir entry: {e}"))?;
        let ft = entry.file_type().map_err(|e| format!("file_type: {e}"))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ft.is_dir() {
            copy_tree(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| {
                format!("copy {} → {}: {e}", src_path.display(), dst_path.display())
            })?;
        }
    }
    Ok(())
}

/// Set a file executable (chmod 755).
///
/// Mirrors Python `path.chmod(0o755)`.
fn make_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o755);
        fs::set_permissions(path, perms).map_err(|e| format!("chmod {}: {e}", path.display()))?;
    }
    #[cfg(not(unix))]
    {
        // No-op on non-Unix; Harbor runs on Linux so this path is never hit.
        let _ = path;
    }
    Ok(())
}

/// Port a single task into Harbor format under `out_dir`.
///
/// Mirrors Python `port_task(arena_dir, arena, task_dir, out_dir)`.
///
/// `scorer_bin` must be the path to the prebuilt `daedalus-score` musl binary.
/// It is copied into `tests/daedalus-score` and the generated `test.sh` runs
/// `/tests/daedalus-score` inside the Harbor container.
///
/// # Errors
///
/// Returns an error string on any I/O or parse failure.
pub fn port_task(
    arena_dir: &Path,
    arena: &TomlValue,
    task_dir: &Path,
    out_dir: &Path,
    scorer_bin: &Path,
) -> Result<(), String> {
    // Read the source task.toml
    let src_toml_text = fs::read_to_string(task_dir.join("task.toml"))
        .map_err(|e| format!("read task.toml: {e}"))?;
    let src_cfg: TomlValue = src_toml_text
        .parse()
        .map_err(|e| format!("parse task.toml: {e}"))?;

    fs::create_dir_all(out_dir)
        .map_err(|e| format!("create out_dir {}: {e}", out_dir.display()))?;

    // instruction.md
    let instruction = render_instruction(arena_dir, arena, task_dir)?;
    fs::write(out_dir.join("instruction.md"), &instruction)
        .map_err(|e| format!("write instruction.md: {e}"))?;

    // task.toml — Python-flavoured float formatting
    let agent_timeout = src_cfg
        .get("agent")
        .and_then(|a| a.get("timeout_sec"))
        .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
        .unwrap_or(600.0);
    let verifier_timeout = src_cfg
        .get("verifier")
        .and_then(|v| v.get("timeout_sec"))
        .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
        .unwrap_or(120.0);
    let task_toml_out = TASK_TOML_TEMPLATE
        .replace("{agent_timeout}", &format_py_float(agent_timeout))
        .replace("{verifier_timeout}", &format_py_float(verifier_timeout));
    fs::write(out_dir.join("task.toml"), &task_toml_out)
        .map_err(|e| format!("write task.toml: {e}"))?;

    // environment/ — copy then overwrite Dockerfile + .dockerignore
    let env_out = out_dir.join("environment");
    if env_out.exists() {
        fs::remove_dir_all(&env_out).map_err(|e| format!("rmtree environment: {e}"))?;
    }
    copy_tree(&task_dir.join("environment"), &env_out)?;
    fs::write(env_out.join("Dockerfile"), DOCKERFILE)
        .map_err(|e| format!("write Dockerfile: {e}"))?;
    fs::write(env_out.join(".dockerignore"), DOCKERIGNORE)
        .map_err(|e| format!("write .dockerignore: {e}"))?;

    // tests/
    let tests_out = out_dir.join("tests");
    if tests_out.exists() {
        fs::remove_dir_all(&tests_out).map_err(|e| format!("rmtree tests: {e}"))?;
    }
    fs::create_dir_all(&tests_out).map_err(|e| format!("create tests/: {e}"))?;
    fs::copy(
        task_dir.join("tests").join("expected.json"),
        tests_out.join("expected.json"),
    )
    .map_err(|e| format!("copy expected.json: {e}"))?;
    let scorer_dst = tests_out.join("daedalus-score");
    fs::copy(scorer_bin, &scorer_dst).map_err(|e| format!("copy daedalus-score binary: {e}"))?;
    make_executable(&scorer_dst)?;
    let test_sh_path = tests_out.join("test.sh");
    fs::write(&test_sh_path, TEST_SH).map_err(|e| format!("write test.sh: {e}"))?;
    make_executable(&test_sh_path)?;

    // solution/
    let sol_out = out_dir.join("solution");
    if sol_out.exists() {
        fs::remove_dir_all(&sol_out).map_err(|e| format!("rmtree solution: {e}"))?;
    }
    fs::create_dir_all(&sol_out).map_err(|e| format!("create solution/: {e}"))?;
    fs::copy(
        task_dir.join("solution").join("findings.json"),
        sol_out.join("findings.json"),
    )
    .map_err(|e| format!("copy findings.json: {e}"))?;
    let solve_sh_path = sol_out.join("solve.sh");
    fs::write(&solve_sh_path, SOLVE_SH).map_err(|e| format!("write solve.sh: {e}"))?;
    make_executable(&solve_sh_path)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_py_float_integer_values() {
        // Python: str(float(x)) for integer-valued floats always has ".0"
        assert_eq!(format_py_float(600.0), "600.0");
        assert_eq!(format_py_float(120.0), "120.0");
        assert_eq!(format_py_float(60.0), "60.0");
        assert_eq!(format_py_float(0.0), "0.0");
        assert_eq!(format_py_float(1.0), "1.0");
        assert_eq!(format_py_float(300.0), "300.0");
    }

    #[test]
    fn format_py_float_noninteger_values() {
        // Python: str(float(x)) for non-integer floats uses minimal decimal repr
        assert_eq!(format_py_float(30.5), "30.5");
        assert_eq!(format_py_float(0.5), "0.5");
        assert_eq!(format_py_float(1.25), "1.25");
        assert_eq!(format_py_float(1.1), "1.1");
    }

    #[test]
    fn task_toml_template_substitution() {
        let out = TASK_TOML_TEMPLATE
            .replace("{agent_timeout}", "600.0")
            .replace("{verifier_timeout}", "120.0");
        let expected = "version = \"1.0\"\n\n[agent]\ntimeout_sec = 600.0\n\n[verifier]\ntimeout_sec = 120.0\n";
        assert_eq!(out, expected);
    }

    #[test]
    fn dockerfile_and_dockerignore_constants() {
        assert!(DOCKERFILE.starts_with("FROM python:3.12-slim\n"));
        assert_eq!(DOCKERIGNORE, "Dockerfile\n.dockerignore\n");
    }

    #[test]
    fn test_sh_is_bash_and_executable_content() {
        assert!(TEST_SH.starts_with("#!/bin/bash\n"));
        assert!(TEST_SH.contains("daedalus-score"));
        assert!(TEST_SH.contains("reward.txt"));
    }

    #[test]
    fn solve_sh_constant() {
        assert!(SOLVE_SH.starts_with("#!/bin/bash\n"));
        assert!(SOLVE_SH.contains("findings.json"));
    }
}
