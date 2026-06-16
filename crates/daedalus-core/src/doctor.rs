//! Cold-start readiness checks for Daedalus operators.
//!
//! Port of `runner/doctor.py`. All deterministic checks accept injected `today`
//! (as a `(year, month, day)` tuple) so they are parity-testable without a live
//! clock. The `_check_run_artifacts` git path is a live subprocess boundary that
//! is not parity-tested (it is unit-tested instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::pycompat::days_from_civil;

// ---------------------------------------------------------------------------
// Public data structures
// ---------------------------------------------------------------------------

/// A single readiness check result.  Status is one of "ok", "warn", "fail".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub name: String,
    pub status: String,
    pub message: String,
}

impl Check {
    fn new(name: &str, status: &str, message: &str) -> Self {
        Check {
            name: name.to_string(),
            status: status.to_string(),
            message: message.to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Extract the verified-live date from `docs/primitives.md`.
/// Mirrors `_primitive_date(text)` in Python:
///   find "Verified live", then the first `**...**` pair after it,
///   parse the content as `%Y-%m-%d`.
fn primitive_date(text: &str) -> Option<(i64, u32, u32)> {
    let marker = "Verified live";
    let start = text.find(marker)?;
    let tail = &text[start..];
    let star1 = tail.find("**")?;
    let after1 = star1 + 2;
    let star2 = tail[after1..].find("**")?;
    let date_str = &tail[after1..after1 + star2];
    parse_date_ymd(date_str)
}

/// Parse `YYYY-MM-DD` → `(year, month, day)`.  Returns `None` on any error.
fn parse_date_ymd(s: &str) -> Option<(i64, u32, u32)> {
    let s = s.trim();
    let parts: Vec<&str> = s.splitn(3, '-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y: i64 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    let d: u32 = parts[2].parse().ok()?;
    // Validate ranges, mirroring Python datetime.strptime
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some((y, m, d))
}

/// `(today - verified).days` — mirrors Python `(today - verified).days`.
fn date_diff_days(today: (i64, u32, u32), verified: (i64, u32, u32)) -> i64 {
    days_from_civil(today.0, today.1, today.2) - days_from_civil(verified.0, verified.1, verified.2)
}

/// Glob for `deliveries/*/contract.toml`, sorted.
fn delivery_contracts(repo: &Path) -> Vec<PathBuf> {
    let deliveries = repo.join("deliveries");
    if !deliveries.exists() {
        return Vec::new();
    }
    let Ok(entries) = std::fs::read_dir(&deliveries) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path().join("contract.toml"))
        .filter(|p| p.exists())
        .collect();
    paths.sort();
    paths
}

/// Load a TOML file as `toml::Value`.  Returns `None` on read/parse error.
fn load_toml(path: &Path) -> Option<toml::Value> {
    let text = std::fs::read_to_string(path).ok()?;
    text.parse::<toml::Value>().ok()
}

// ---------------------------------------------------------------------------
// Individual checks (public so the parity oracle can call them directly)
// ---------------------------------------------------------------------------

/// Check whether `docs/primitives.md` is present and recently verified.
/// `today` is injected as `(year, month, day)` for determinism.
pub fn check_primitives(repo: &Path, today: (i64, u32, u32), stale_days: i64) -> Check {
    let path = repo.join("docs").join("primitives.md");
    if !path.exists() {
        return Check::new("model-primitives", "fail", "docs/primitives.md missing");
    }
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => {
            return Check::new(
                "model-primitives",
                "fail",
                "could not find verified model-primitives date",
            )
        }
    };
    let verified = match primitive_date(&text) {
        Some(d) => d,
        None => {
            return Check::new(
                "model-primitives",
                "fail",
                "could not find verified model-primitives date",
            )
        }
    };
    let age = date_diff_days(today, verified);
    let verified_str = format!("{:04}-{:02}-{:02}", verified.0, verified.1, verified.2);
    if age > stale_days {
        return Check::new(
            "model-primitives",
            "fail",
            &format!("model primitives are stale: {verified_str} ({age} days old)"),
        );
    }
    Check::new(
        "model-primitives",
        "ok",
        &format!("model primitives verified {verified_str} ({age} days old)"),
    )
}

/// Check delivery contract approvals.
/// Mirrors `_check_approvals(repo)` in Python.
pub fn check_approvals(repo: &Path) -> Check {
    let mut missing: Vec<String> = Vec::new();
    let mut unsigned: Vec<String> = Vec::new();

    for path in delivery_contracts(repo) {
        let Some(contract) = load_toml(&path) else {
            continue;
        };
        let approval = contract
            .get("approval")
            .and_then(|v| v.as_table())
            .cloned()
            .unwrap_or_default();

        let ref_val = approval.get("g3_approval");
        let ref_str: Option<&str> = ref_val.and_then(|v| v.as_str());

        if ref_str.map(|s| s.is_empty()).unwrap_or(true) {
            // No g3_approval key, or empty string → missing
            let rel = path
                .strip_prefix(repo)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| path.to_string_lossy().into_owned());
            missing.push(rel);
            continue;
        }
        let ref_s = ref_str.unwrap();

        // Resolve path: if not absolute, resolve relative to repo root
        let approval_path = {
            let p = PathBuf::from(ref_s);
            if p.is_absolute() {
                p
            } else {
                repo.join(&p)
            }
        };
        if !approval_path.exists() {
            missing.push(ref_s.to_string());
        }

        // g3_signed check (independent of file existence)
        let signed = approval
            .get("g3_signed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !signed {
            unsigned.push(ref_s.to_string());
        }
    }

    if !missing.is_empty() {
        return Check::new(
            "approvals",
            "fail",
            &format!("missing approval file(s): {}", missing.join(", ")),
        );
    }
    if !unsigned.is_empty() {
        return Check::new(
            "approvals",
            "warn",
            &format!(
                "launch approval unsigned; lab evidence only: {}",
                unsigned.join(", ")
            ),
        );
    }
    Check::new("approvals", "ok", "delivery launch approvals are signed")
}

/// Check that all delivery contracts have a known harness version.
/// Mirrors `_check_harness_versions(repo)` in Python.
pub fn check_harness_versions(repo: &Path) -> Check {
    let mut unknown: Vec<String> = Vec::new();
    for path in delivery_contracts(repo) {
        let Some(contract) = load_toml(&path) else {
            continue;
        };
        let version = contract
            .get("composition")
            .and_then(|v| v.get("harness_version"))
            .map(|v| v.to_string().trim_matches('"').to_string())
            .unwrap_or_default();
        // `str((...).get("harness_version", ""))` — empty string or "unknown"
        if version.is_empty() || version == "unknown" {
            let rel = path
                .strip_prefix(repo)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| path.to_string_lossy().into_owned());
            unknown.push(rel);
        }
    }
    if !unknown.is_empty() {
        return Check::new(
            "harness-versions",
            "fail",
            &format!("unknown harness version in {}", unknown.join(", ")),
        );
    }
    Check::new(
        "harness-versions",
        "ok",
        "delivery harness versions are pinned",
    )
}

/// Check that `docs/primitives.md` documents the pi concurrency constraint.
/// Mirrors `_check_parallel_pi(repo)` in Python.
pub fn check_parallel_pi(repo: &Path) -> Check {
    let path = repo.join("docs").join("primitives.md");
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => {
            return Check::new(
                "parallel-pi",
                "fail",
                "docs/primitives.md does not warn about sequential pi trials",
            )
        }
    };
    let lower = text.to_lowercase();
    if !lower.contains("sequential") || !lower.contains("deadlock") {
        return Check::new(
            "parallel-pi",
            "fail",
            "docs/primitives.md does not warn about sequential pi trials",
        );
    }
    Check::new(
        "parallel-pi",
        "ok",
        "pi concurrency constraint is documented",
    )
}

/// Check for local run artifacts (un-committed experiment output).
/// When `use_git` is true, also runs `git status` to find dirty run records.
/// Mirrors `_check_run_artifacts(repo, use_git)` in Python.
///
/// The git subprocess path is a live I/O boundary; it is not parity-tested.
pub fn check_run_artifacts(repo: &Path, use_git: bool) -> Check {
    let runs_dir = repo.join("runs");

    // Collect files under runs/*/artifacts/**/
    let mut artifact_files: Vec<PathBuf> = Vec::new();
    if runs_dir.exists() {
        if let Ok(run_entries) = std::fs::read_dir(&runs_dir) {
            for run_entry in run_entries.flatten() {
                let artifacts_dir = run_entry.path().join("artifacts");
                if !artifacts_dir.exists() {
                    continue;
                }
                collect_files_recursive(&artifacts_dir, &mut artifact_files);
            }
        }
    }
    artifact_files.sort();

    if !artifact_files.is_empty() {
        let sample: Vec<String> = artifact_files
            .iter()
            .take(3)
            .map(|p| {
                p.strip_prefix(repo)
                    .map(|r| r.to_string_lossy().into_owned())
                    .unwrap_or_else(|_| p.to_string_lossy().into_owned())
            })
            .collect();
        return Check::new(
            "run-artifacts",
            "warn",
            &format!("local run artifacts present: {}", sample.join(", ")),
        );
    }

    if use_git {
        let result = Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "status",
                "--short",
                "--untracked-files=all",
                "--",
                "runs",
            ])
            .output();
        if let Ok(proc) = result {
            let stdout = String::from_utf8_lossy(&proc.stdout);
            let trimmed = stdout.trim();
            if !trimmed.is_empty() {
                return Check::new(
                    "run-artifacts",
                    "fail",
                    &format!("dirty run records/artifacts:\n{trimmed}"),
                );
            }
        }
    }

    Check::new("run-artifacts", "ok", "no dirty run artifacts detected")
}

/// Recursively collect files from a directory.
fn collect_files_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            out.push(path);
        } else if path.is_dir() {
            collect_files_recursive(&path, out);
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run all checks.  `today` is `(year, month, day)` for determinism;
/// pass `None` to use the current local date.
pub fn run_checks(
    repo: &Path,
    today: Option<(i64, u32, u32)>,
    stale_days: i64,
    use_git: bool,
) -> Vec<Check> {
    let today = today.unwrap_or_else(|| {
        // Derive today from Unix time (UTC), matching Python's date.today() in UTC
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let days = secs.div_euclid(86_400);
        let (y, m, d) = crate::pycompat::civil_from_days(days);
        (y, m, d)
    });
    vec![
        check_primitives(repo, today, stale_days),
        check_approvals(repo),
        check_harness_versions(repo),
        check_parallel_pi(repo),
        check_run_artifacts(repo, use_git),
    ]
}

/// Render checks as a Markdown table.
/// Mirrors `render(checks)` in Python.
pub fn render(checks: &[Check]) -> String {
    let mut lines = vec![
        "# Daedalus doctor".to_string(),
        String::new(),
        "| check | status | message |".to_string(),
        "|---|---|---|".to_string(),
    ];
    for check in checks {
        lines.push(format!(
            "| {} | {} | {} |",
            check.name, check.status, check.message
        ));
    }
    lines.join("\n") + "\n"
}

/// Return true if any check has status "fail".
/// Mirrors `has_failures(checks)` in Python.
pub fn has_failures(checks: &[Check]) -> bool {
    checks.iter().any(|c| c.status == "fail")
}

// ---------------------------------------------------------------------------
// Unit tests (porting test_doctor.py assertions where deterministic)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Write a minimal repo fixture (mirrors `write_minimal_repo` in test_doctor.py).
    fn write_minimal_repo(tmp: &Path, primitive_date: &str, harness: &str) -> PathBuf {
        let docs = tmp.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(
            docs.join("primitives.md"),
            format!(
                "Verified live against the OpenRouter `/api/v1/models` endpoint on\n\
                 **{primitive_date}**.\n\n\
                 Run pi trials **sequentially** per machine; parallel pi can deadlock.\n"
            ),
        )
        .unwrap();
        let delivery = tmp.join("deliveries").join("demo");
        fs::create_dir_all(&delivery).unwrap();
        let approvals = tmp.join("approvals");
        fs::create_dir_all(&approvals).unwrap();
        fs::write(approvals.join("G3-demo.md"), "**Status:** pending\n").unwrap();
        fs::write(
            delivery.join("contract.toml"),
            format!(
                r#"
contract = 1
agent = "demo"

[composition]
harness = "pi"
harness_version = "{harness}"

[approval]
g3_signed = false
g3_approval = "approvals/G3-demo.md"
"#
            ),
        )
        .unwrap();
        let runs = tmp.join("runs");
        fs::create_dir_all(&runs).unwrap();
        tmp.to_path_buf()
    }

    fn status_by_name(checks: &[Check]) -> std::collections::HashMap<String, String> {
        checks
            .iter()
            .map(|c| (c.name.clone(), c.status.clone()))
            .collect()
    }

    #[test]
    fn fresh_primitives_and_known_harness() {
        let tmp = tempdir();
        let repo = write_minimal_repo(&tmp, "2026-06-10", "0.78.1");
        let checks = run_checks(&repo, Some((2026, 6, 12)), 30, false);
        let s = status_by_name(&checks);
        assert_eq!(s["model-primitives"], "ok");
        assert_eq!(s["harness-versions"], "ok");
        assert_eq!(s["parallel-pi"], "ok");
        assert_eq!(s["approvals"], "warn");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn stale_primitives_and_unknown_harness() {
        let tmp = tempdir();
        let repo = write_minimal_repo(&tmp, "2026-04-01", "unknown");
        let checks = run_checks(&repo, Some((2026, 6, 12)), 30, false);
        let s = status_by_name(&checks);
        assert_eq!(s["model-primitives"], "fail");
        assert_eq!(s["harness-versions"], "fail");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn dirty_run_artifacts_without_git() {
        let tmp = tempdir();
        let repo = write_minimal_repo(&tmp, "2026-06-10", "0.78.1");
        let artifacts = repo.join("runs").join("exp").join("artifacts");
        fs::create_dir_all(&artifacts).unwrap();
        fs::write(artifacts.join("response.txt"), "raw\n").unwrap();
        let checks = run_checks(&repo, Some((2026, 6, 12)), 30, false);
        let s = status_by_name(&checks);
        assert_eq!(s["run-artifacts"], "warn");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn has_failures_returns_true_on_fail() {
        let checks = vec![
            Check::new("a", "ok", "fine"),
            Check::new("b", "fail", "broken"),
        ];
        assert!(has_failures(&checks));
    }

    #[test]
    fn has_failures_returns_false_when_no_fail() {
        let checks = vec![
            Check::new("a", "ok", "fine"),
            Check::new("b", "warn", "sorta"),
        ];
        assert!(!has_failures(&checks));
    }

    #[test]
    fn render_produces_markdown_table() {
        let checks = vec![
            Check::new("model-primitives", "ok", "verified 2026-06-10 (2 days old)"),
            Check::new("approvals", "warn", "unsigned"),
        ];
        let out = render(&checks);
        assert!(out.contains("# Daedalus doctor"));
        assert!(out.contains("| check | status | message |"));
        assert!(out.contains("| model-primitives | ok |"));
        assert!(out.contains("| approvals | warn |"));
        assert!(out.ends_with('\n'));
    }

    #[test]
    fn primitive_date_parses_correctly() {
        let text = "Verified live against the OpenRouter endpoint on\n**2026-06-10**.\n";
        let d = primitive_date(text);
        assert_eq!(d, Some((2026, 6, 10)));
    }

    #[test]
    fn primitive_date_missing_marker_returns_none() {
        assert!(primitive_date("no marker here").is_none());
    }

    #[test]
    fn date_diff_mirrors_python_timedelta() {
        // (2026-06-12) - (2026-06-10) = 2
        assert_eq!(date_diff_days((2026, 6, 12), (2026, 6, 10)), 2);
        // (2026-06-12) - (2026-04-01) = 72
        assert_eq!(date_diff_days((2026, 6, 12), (2026, 4, 1)), 72);
    }

    // ---------------------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------------------

    fn tempdir() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("daedalus-doctor-test-{}-{n}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
