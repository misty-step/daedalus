//! Score review findings against a seeded-defect answer key.
//!
//! Port of `runner/score.py`. Contract (DESIGN.md):
//! `reward = max(0, recall - 0.2 * false_positives)`, except on a clean task
//! (empty answer key) where any finding at all scores 0 — inventing defects on
//! a sound change fails the task's whole point.
//!
//! A finding matches a defect when `file` and `category` are equal and the
//! finding's `line` falls inside `[line_start, line_end]`. Each defect matches
//! at most once (greedy, in finding order). Findings that match nothing are
//! false positives. A missing or malformed findings.json scores 0 — failing to
//! follow the output contract is a failure.

use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::pycompat::round_half_even;

const FP_PENALTY: f64 = 0.2;

/// Rank of a severity label; lower is stricter. `None` for unknown labels,
/// matching the Python `SEVERITY_RANK` membership test.
fn severity_rank(label: &str) -> Option<u8> {
    match label {
        "blocking" => Some(0),
        "serious" => Some(1),
        "minor" => Some(2),
        _ => None,
    }
}

/// One seeded defect from the answer key (`expected.json` → `defects[]`).
#[derive(Debug, Clone, Deserialize)]
struct Defect {
    id: String,
    file: String,
    line_start: i64,
    line_end: i64,
    category: String,
    #[serde(default)]
    severity: Option<String>,
}

/// The grader verdict. Field order matches `runner/score.py`'s result dict so
/// the CLI's pretty JSON lines up with the Python tool.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScoreResult {
    pub reward: f64,
    pub recall: f64,
    pub matched: Vec<String>,
    pub false_positives: u64,
    pub expected_defects: usize,
    pub error: Option<String>,
}

/// Failure to load the answer key. Python lets a bad answer key raise uncaught
/// (it is repo-controlled and always well-formed); we surface it as `Err` so
/// the caller exits non-zero the same way. A bad *findings* file is **not** an
/// error — it yields `Ok(ScoreResult)` with `error` set and reward 0, matching
/// the Python try/except that scores malformed output 0.
#[derive(Debug)]
pub enum ScoreError {
    AnswerKey(String),
}

impl fmt::Display for ScoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScoreError::AnswerKey(m) => write!(f, "invalid answer key: {m}"),
        }
    }
}

impl std::error::Error for ScoreError {}

/// Replicate Python's `int(value)` applied to a finding's `line`. Returns
/// `None` exactly where Python would raise `TypeError`/`ValueError` — which the
/// scorer treats as a false positive.
fn coerce_line(value: Option<&Value>) -> Option<i64> {
    match value? {
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(i)
            } else if let Some(u) = n.as_u64() {
                Some(u as i64)
            } else {
                // int(7.5) == 7: truncate toward zero.
                n.as_f64().map(|f| f.trunc() as i64)
            }
        }
        // int("7") parses; int("7.5") raises. Whitespace is stripped.
        Value::String(s) => s.trim().parse::<i64>().ok(),
        // bool is an int subclass in Python: int(True) == 1.
        Value::Bool(b) => Some(i64::from(*b)),
        _ => None,
    }
}

/// True if `finding`'s severity is at least as strict as `defect` requires.
/// A defect with no `severity` ignores severity entirely.
fn severity_matches(finding: &Value, defect: &Defect) -> bool {
    let expected = match defect.severity.as_deref() {
        None => return true,
        Some(label) => label,
    };
    let found = finding.get("severity").and_then(Value::as_str);
    match (severity_rank(expected), found.and_then(severity_rank)) {
        (Some(exp_rank), Some(found_rank)) => found_rank <= exp_rank,
        _ => false,
    }
}

fn load_expected(path: &Path) -> Result<Vec<Defect>, ScoreError> {
    let raw = std::fs::read_to_string(path).map_err(|e| ScoreError::AnswerKey(e.to_string()))?;
    let value: Value =
        serde_json::from_str(&raw).map_err(|e| ScoreError::AnswerKey(e.to_string()))?;
    let defects = value
        .get("defects")
        .ok_or_else(|| ScoreError::AnswerKey("missing 'defects' key".to_string()))?;
    serde_json::from_value(defects.clone()).map_err(|e| ScoreError::AnswerKey(e.to_string()))
}

/// Load `findings.json` → the `findings` array. `Err(msg)` carries the reason a
/// malformed file scores 0 (mirrors the strings Python's except sees).
fn load_findings(path: &Path) -> Result<Vec<Value>, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let value: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    match value.get("findings") {
        Some(Value::Array(items)) => Ok(items.clone()),
        Some(_) => Err("findings is not a list".to_string()),
        None => Err("'findings'".to_string()),
    }
}

/// Score `findings_path` against the answer key at `expected_path`.
pub fn score(findings_path: &Path, expected_path: &Path) -> Result<ScoreResult, ScoreError> {
    let expected = load_expected(expected_path)?;
    let findings = match load_findings(findings_path) {
        Ok(items) => items,
        Err(msg) => {
            return Ok(ScoreResult {
                reward: 0.0,
                recall: 0.0,
                matched: Vec::new(),
                false_positives: 0,
                expected_defects: expected.len(),
                error: Some(format!("invalid findings: {msg}")),
            });
        }
    };
    Ok(score_defects(&expected, &findings))
}

/// The matching + reward core, over already-loaded defects and findings. Shared
/// by [`score`] and [`redteam_audit`].
fn score_defects(expected: &[Defect], findings: &[Value]) -> ScoreResult {
    let mut result = ScoreResult {
        reward: 0.0,
        recall: 0.0,
        matched: Vec::new(),
        false_positives: 0,
        expected_defects: expected.len(),
        error: None,
    };

    let mut matched_flags = vec![false; expected.len()];
    let mut false_positives: u64 = 0;

    for finding in findings {
        // file and category must be present (Python KeyError → FP); line must
        // be int-coercible (Python int() → ValueError/TypeError → FP).
        let (file, category) = match (finding.get("file"), finding.get("category")) {
            (Some(f), Some(c)) => (f, c),
            _ => {
                false_positives += 1;
                continue;
            }
        };
        let line = match coerce_line(finding.get("line")) {
            Some(n) => n,
            None => {
                false_positives += 1;
                continue;
            }
        };

        let hit = expected.iter().enumerate().position(|(i, d)| {
            !matched_flags[i]
                && file.as_str() == Some(d.file.as_str())
                && category.as_str() == Some(d.category.as_str())
                && d.line_start <= line
                && line <= d.line_end
                && severity_matches(finding, d)
        });

        match hit {
            Some(idx) => {
                matched_flags[idx] = true;
                result.matched.push(expected[idx].id.clone());
            }
            None => false_positives += 1,
        }
    }

    let recall = if expected.is_empty() {
        1.0
    } else {
        result.matched.len() as f64 / expected.len() as f64
    };
    result.recall = round_half_even(recall, 4);
    result.false_positives = false_positives;
    result.reward = if expected.is_empty() && false_positives > 0 {
        0.0
    } else {
        round_half_even((recall - FP_PENALTY * false_positives as f64).max(0.0), 4)
    };

    result
}

/// A red-team audit of an answer key (backlog 040 item 3). The scorer matches a
/// finding to a defect on `file == file && category == category && line ∈
/// [line_start, line_end]`, so a *wide* span lets a candidate score a hit by
/// guessing only the file+category and emitting at any in-span line — without
/// locating the defect. This measures that spatial slack.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RedteamAudit {
    pub n_defects: usize,
    /// Widest span (`line_end − line_start + 1`).
    pub max_span: i64,
    pub mean_span: f64,
    /// `(defect id, span)` for spans wider than the threshold — the gameable
    /// keys where the line constraint is weak.
    pub wide_defects: Vec<(String, i64)>,
    /// Reward earned by a structure-aware, **zero-localization** candidate that
    /// emits file+category at each defect's span edge. 1.0 means the line
    /// constraint adds no discrimination once the key structure is known — the
    /// arena then measures file+category identification, not defect location.
    pub gaming_reward: f64,
}

/// Audit an `expected.json` answer key for scorer-gaming exposure.
pub fn redteam_audit(
    expected_path: &Path,
    wide_threshold: i64,
) -> Result<RedteamAudit, ScoreError> {
    let expected = load_expected(expected_path)?;
    let spans: Vec<i64> = expected
        .iter()
        .map(|d| (d.line_end - d.line_start + 1).max(0))
        .collect();
    let max_span = spans.iter().copied().max().unwrap_or(0);
    let mean_span = if spans.is_empty() {
        0.0
    } else {
        round_half_even(spans.iter().sum::<i64>() as f64 / spans.len() as f64, 2)
    };
    let wide_defects: Vec<(String, i64)> = expected
        .iter()
        .zip(&spans)
        .filter(|(_, &s)| s > wide_threshold)
        .map(|(d, &s)| (d.id.clone(), s))
        .collect();

    // Structure-aware, zero-precision gaming candidate: file+category at the
    // span edge for every defect. If this scores high, the line constraint is
    // not discriminating.
    let gaming: Vec<Value> = expected
        .iter()
        .map(|d| {
            let mut m = serde_json::Map::new();
            m.insert("file".into(), Value::String(d.file.clone()));
            m.insert("category".into(), Value::String(d.category.clone()));
            m.insert("line".into(), Value::from(d.line_start));
            if let Some(sev) = &d.severity {
                m.insert("severity".into(), Value::String(sev.clone()));
            }
            Value::Object(m)
        })
        .collect();
    let gaming_reward = score_defects(&expected, &gaming).reward;

    Ok(RedteamAudit {
        n_defects: expected.len(),
        max_span,
        mean_span,
        wide_defects,
        gaming_reward,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// A throwaway fixture directory, mirroring pytest's `tmp_path`.
    struct Case {
        dir: PathBuf,
    }

    impl Case {
        fn new() -> Self {
            let n = COUNTER.fetch_add(1, Ordering::SeqCst);
            let dir =
                std::env::temp_dir().join(format!("daedalus-score-{}-{n}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            Case { dir }
        }

        fn write(&self, name: &str, contents: &str) -> PathBuf {
            let p = self.dir.join(name);
            std::fs::write(&p, contents).unwrap();
            p
        }

        fn expected_two(&self) -> PathBuf {
            self.write(
                "expected.json",
                r#"{"defects":[
                  {"id":"d1","file":"a.py","line_start":5,"line_end":10,"category":"security"},
                  {"id":"d2","file":"a.py","line_start":20,"line_end":22,"category":"correctness"}
                ]}"#,
            )
        }

        fn expected_clean(&self) -> PathBuf {
            self.write("expected.json", r#"{"defects":[]}"#)
        }

        fn findings(&self, items: &str) -> PathBuf {
            self.write("findings.json", &format!(r#"{{"findings":{items}}}"#))
        }
    }

    impl Drop for Case {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    fn run(f: &Path, e: &Path) -> ScoreResult {
        score(f, e).unwrap()
    }

    #[test]
    fn redteam_gaming_candidate_scores_full_without_localization() {
        // A candidate that knows only file+category and emits at each span edge
        // (no real localization) earns full reward — proof the line constraint
        // adds no discrimination once the key structure is known.
        let c = Case::new();
        let exp = c.expected_two();
        let audit = redteam_audit(&exp, 8).unwrap();
        assert_eq!(audit.gaming_reward, 1.0);
        assert_eq!(audit.n_defects, 2);
    }

    #[test]
    fn redteam_audit_flags_wide_spans() {
        let c = Case::new();
        // d1 span 25 lines (gameable), d2 span 2 lines (tight).
        let exp = c.write(
            "expected.json",
            r#"{"defects":[
              {"id":"d1","file":"a.py","line_start":1,"line_end":25,"category":"correctness"},
              {"id":"d2","file":"a.py","line_start":40,"line_end":41,"category":"security"}
            ]}"#,
        );
        let audit = redteam_audit(&exp, 8).unwrap();
        assert_eq!(audit.max_span, 25);
        assert_eq!(audit.mean_span, 13.5); // (25 + 2) / 2
        assert_eq!(audit.wide_defects, vec![("d1".to_string(), 25)]);
    }

    #[test]
    fn redteam_audit_on_a_clean_key_is_empty() {
        let c = Case::new();
        let exp = c.expected_clean();
        let audit = redteam_audit(&exp, 8).unwrap();
        assert_eq!(audit.n_defects, 0);
        assert_eq!(audit.max_span, 0);
        assert!(audit.wide_defects.is_empty());
    }

    #[test]
    fn perfect_recall_no_fp() {
        let c = Case::new();
        let f = c.findings(
            r#"[{"file":"a.py","line":7,"category":"security"},
                {"file":"a.py","line":21,"category":"correctness"}]"#,
        );
        let r = run(&f, &c.expected_two());
        assert_eq!(r.reward, 1.0);
        let mut m = r.matched.clone();
        m.sort();
        assert_eq!(m, ["d1", "d2"]);
        assert_eq!(r.false_positives, 0);
    }

    #[test]
    fn half_recall_plus_one_fp_is_point_three() {
        let c = Case::new();
        let f = c.findings(
            r#"[{"file":"a.py","line":7,"category":"security"},
                {"file":"a.py","line":99,"category":"concurrency"}]"#,
        );
        let r = run(&f, &c.expected_two());
        assert_eq!(r.reward, 0.3);
        assert_eq!(r.recall, 0.5);
        assert_eq!(r.false_positives, 1);
    }

    #[test]
    fn empty_findings_on_defective_task_scores_zero() {
        let c = Case::new();
        let f = c.findings("[]");
        let r = run(&f, &c.expected_two());
        assert_eq!(r.reward, 0.0);
        assert_eq!(r.recall, 0.0);
    }

    #[test]
    fn clean_task_silence_scores_one() {
        let c = Case::new();
        let f = c.findings("[]");
        let r = run(&f, &c.expected_clean());
        assert_eq!(r.reward, 1.0);
    }

    #[test]
    fn clean_task_any_invented_finding_is_hard_zero() {
        let c = Case::new();
        let f = c.findings(r#"[{"file":"a.py","line":1,"category":"correctness"}]"#);
        let r = run(&f, &c.expected_clean());
        assert_eq!(r.reward, 0.0);
        assert_eq!(r.false_positives, 1);
    }

    #[test]
    fn category_mismatch_is_fp_not_match() {
        let c = Case::new();
        let f = c.findings(r#"[{"file":"a.py","line":7,"category":"correctness"}]"#);
        let r = run(&f, &c.expected_two());
        assert!(r.matched.is_empty());
        assert_eq!(r.false_positives, 1);
    }

    #[test]
    fn line_outside_range_is_fp() {
        let c = Case::new();
        let f = c.findings(r#"[{"file":"a.py","line":11,"category":"security"}]"#);
        let r = run(&f, &c.expected_two());
        assert!(r.matched.is_empty());
        assert_eq!(r.false_positives, 1);
    }

    #[test]
    fn one_defect_matches_at_most_once() {
        let c = Case::new();
        let f = c.findings(
            r#"[{"file":"a.py","line":6,"category":"security"},
                {"file":"a.py","line":8,"category":"security"}]"#,
        );
        let r = run(&f, &c.expected_two());
        assert_eq!(r.matched, ["d1"]);
        assert_eq!(r.false_positives, 1);
        assert_eq!(r.reward, 0.3);
    }

    #[test]
    fn missing_findings_file_scores_zero_with_error() {
        let c = Case::new();
        let r = run(&c.dir.join("nope.json"), &c.expected_two());
        assert_eq!(r.reward, 0.0);
        assert!(r.error.is_some());
    }

    #[test]
    fn malformed_findings_scores_zero_with_error() {
        let c = Case::new();
        let p = c.write("findings.json", "not json {");
        let r = run(&p, &c.expected_two());
        assert_eq!(r.reward, 0.0);
        assert!(r.error.is_some());
    }

    #[test]
    fn findings_not_a_list_scores_zero() {
        let c = Case::new();
        let p = c.write("findings.json", r#"{"findings":"lots of issues"}"#);
        let r = run(&p, &c.expected_two());
        assert_eq!(r.reward, 0.0);
        assert!(r.error.is_some());
    }

    #[test]
    fn finding_missing_fields_counts_as_fp() {
        let c = Case::new();
        let f = c.findings(r#"[{"file":"a.py"},{"line":7,"category":"security"}]"#);
        let r = run(&f, &c.expected_two());
        assert_eq!(r.false_positives, 2);
        assert_eq!(r.reward, 0.0);
    }

    #[test]
    fn reward_never_negative() {
        let c = Case::new();
        let items: Vec<String> = (1..=8)
            .map(|i| format!(r#"{{"file":"z.py","line":{i},"category":"correctness"}}"#))
            .collect();
        let f = c.findings(&format!("[{}]", items.join(",")));
        let r = run(&f, &c.expected_two());
        assert_eq!(r.reward, 0.0);
        assert_eq!(r.false_positives, 8);
    }

    #[test]
    fn expected_severity_requires_at_least_that_strict() {
        let c = Case::new();
        let expected = c.write(
            "expected.json",
            r#"{"defects":[{"id":"d1","file":"a.py","line_start":1,"line_end":1,
               "category":"credential-exposure","severity":"blocking"}]}"#,
        );
        let weak = c.write(
            "weak.json",
            r#"{"findings":[{"file":"a.py","line":1,"category":"credential-exposure","severity":"serious"}]}"#,
        );
        assert!(run(&weak, &expected).matched.is_empty());
        let strict = c.write(
            "strict.json",
            r#"{"findings":[{"file":"a.py","line":1,"category":"credential-exposure","severity":"blocking"}]}"#,
        );
        assert_eq!(run(&strict, &expected).matched, ["d1"]);
    }

    #[test]
    fn expected_serious_accepts_stricter_blocking_severity() {
        let c = Case::new();
        let expected = c.write(
            "expected.json",
            r#"{"defects":[{"id":"d1","file":"a.py","line_start":1,"line_end":1,
               "category":"logic-invariant","severity":"serious"}]}"#,
        );
        let f = c.findings(
            r#"[{"file":"a.py","line":1,"category":"logic-invariant","severity":"blocking"}]"#,
        );
        assert_eq!(run(&f, &expected).matched, ["d1"]);
    }

    #[test]
    fn expected_severity_rejects_missing_finding_severity() {
        let c = Case::new();
        let expected = c.write(
            "expected.json",
            r#"{"defects":[{"id":"d1","file":"a.py","line_start":1,"line_end":1,
               "category":"credential-exposure","severity":"blocking"}]}"#,
        );
        let f = c.findings(r#"[{"file":"a.py","line":1,"category":"credential-exposure"}]"#);
        let r = run(&f, &expected);
        assert!(r.matched.is_empty());
        assert_eq!(r.false_positives, 1);
    }

    #[test]
    fn expected_severity_rejects_unknown_finding_severity() {
        let c = Case::new();
        let expected = c.write(
            "expected.json",
            r#"{"defects":[{"id":"d1","file":"a.py","line_start":1,"line_end":1,
               "category":"credential-exposure","severity":"blocking"}]}"#,
        );
        let f = c.findings(
            r#"[{"file":"a.py","line":1,"category":"credential-exposure","severity":"critical"}]"#,
        );
        let r = run(&f, &expected);
        assert!(r.matched.is_empty());
        assert_eq!(r.false_positives, 1);
    }
}
