//! Judge scorer family: calibrated 0–5 rubric scoring for qualities a
//! seeded-defect key cannot capture (finding quality, severity calibration,
//! actionability).
//!
//! Port of `runner/judge.py`. The module is pure scoring + calibration logic;
//! the judge LLM call is injected so the family is testable offline.
//!
//! ## LLM boundary
//!
//! `judge_score` takes an injected `call` closure `FnMut(&str, &str) ->
//! (String, f64)` (prompt, model) → (text, cost_usd). This cannot be
//! parity-tested across languages because it requires a live LLM. It is
//! unit-tested in `#[cfg(test)]` with a fake closure returning a canned
//! judge response; the deterministic helpers (`rubric_hash`, `normalize`,
//! `parse_judge_response`, `build_judge_prompt`, `spearman`,
//! `calibration_gate`, `load_scorer_families`) are parity-tested in
//! `tests/parity_judge.rs`.
//!
//! ## Spearman / statistics.mean note
//!
//! Python's `statistics.mean` sums exactly via `Fraction`; `pycompat::mean`
//! uses IEEE 754 f64 addition and can differ by 1 ULP on adversarial inputs.
//! For the rank vectors produced by `spearman` (small integers and half-integers)
//! the two implementations agree in all tested cases. Any divergence at the
//! parity-test level is noted there.

use std::path::Path;

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::pycompat::{mean, round_half_even};

/// 0–5 per-criterion scale (6-point scale maximises human–LLM agreement).
const SCALE_MAX: f64 = 5.0;

// ---------------------------------------------------------------------------
// rubric_hash
// ---------------------------------------------------------------------------

/// SHA-256 of the rubric text (UTF-8), hex-encoded, first 16 characters.
/// Mirrors `hashlib.sha256(rubric_text.encode()).hexdigest()[:16]`.
pub fn rubric_hash(rubric_text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(rubric_text.as_bytes());
    let digest = hasher.finalize();
    // hex-encode (lowercase) and take first 16 chars
    let hex = format!("{digest:x}");
    hex[..16].to_string()
}

// ---------------------------------------------------------------------------
// normalize
// ---------------------------------------------------------------------------

/// Mean of per-criterion 0–5 scores, normalized to [0,1] and rounded to 4dp.
/// Returns 0.0 when the map is empty (mirrors Python: `if not vals: return 0.0`).
pub fn normalize(criterion_scores: &Map<String, Value>) -> f64 {
    let vals: Vec<f64> = criterion_scores
        .values()
        .filter_map(Value::as_f64)
        .collect();
    if vals.is_empty() {
        return 0.0;
    }
    let sum: f64 = vals.iter().sum();
    let n = vals.len() as f64;
    round_half_even(sum / n / SCALE_MAX, 4)
}

// ---------------------------------------------------------------------------
// parse_judge_response
// ---------------------------------------------------------------------------

/// Pull `{criterion: score}` out of a judge model's JSON reply.
///
/// Scans `text` for the first balanced `{…}` that contains at least one valid
/// entry: a non-bool numeric value in [0, SCALE_MAX]. Scores outside that range,
/// booleans, and non-numeric values are dropped. Returns an insertion-ordered
/// map.
///
/// Mirrors the Python nested-scan approach exactly: outer loop advances `start`
/// with `text.find('{', start + 1)` on each failure; inner loop tracks brace
/// depth to find the matching `}`.
///
/// # Errors
/// Returns `Err(msg)` when no parseable 0–5 rubric scores can be extracted.
pub fn parse_judge_response(text: &str) -> Result<Map<String, Value>, String> {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut start = 0;

    // find next '{' from position `start`
    while let Some(rel) = text[start..].find('{') {
        let abs_start = start + rel;
        // walk the brace tree to find the matching '}'
        let mut depth: i32 = 0;
        let mut i = abs_start;
        while i < len {
            match bytes[i] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        // attempt to parse text[abs_start..=i] as JSON
                        let candidate = &text[abs_start..=i];
                        if let Ok(obj) = serde_json::from_str::<Value>(candidate) {
                            // scores = obj.get("scores", obj)
                            let scores_val = match obj.get("scores") {
                                Some(s) => s,
                                None => &obj,
                            };
                            if let Some(scores_obj) = scores_val.as_object() {
                                let mut out: Map<String, Value> = Map::new();
                                for (k, v) in scores_obj {
                                    // drop booleans (bool is NOT int in Rust/JSON)
                                    if v.is_boolean() {
                                        continue;
                                    }
                                    if let Some(num) = v.as_f64() {
                                        if (0.0..=SCALE_MAX).contains(&num) {
                                            // store as integer (matching Python `int(v)`)
                                            out.insert(
                                                k.clone(),
                                                Value::Number(serde_json::Number::from(num as i64)),
                                            );
                                        }
                                    }
                                }
                                if !out.is_empty() {
                                    return Ok(out);
                                }
                            }
                        }
                        // parsing failed or empty out — advance past this '{'
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        // advance start past the current '{' so we look for the next one
        start = abs_start + 1;
    }

    Err("no parseable 0\u{2013}5 rubric scores in judge response".to_string())
}

// ---------------------------------------------------------------------------
// build_judge_prompt
// ---------------------------------------------------------------------------

/// Build the judge prompt: rubric text + task + findings + scoring instruction.
/// Mirrors `build_judge_prompt(rubric_text, instruction, findings)` exactly,
/// including `json.dumps(findings, indent=2)` for the findings block.
pub fn build_judge_prompt(rubric_text: &str, instruction: &str, findings: &Value) -> String {
    let findings_json = serde_json::to_string_pretty(findings).unwrap_or_else(|_| "{}".to_string());
    format!(
        "{rubric_text}\n\n## Task the agent was reviewing\n{instruction}\
\n\n## The agent's findings to score\n{findings_json}\
\n\nScore each rubric criterion from 0 to 5. Respond with ONLY a \
JSON object: {{\"scores\": {{\"<criterion>\": <0-5>, ...}}}}."
    )
}

// ---------------------------------------------------------------------------
// judge_score
// ---------------------------------------------------------------------------

/// One judge model's normalized score for a set of findings.
///
/// `call` is the injected optimizer/judge LLM call:
///   `FnMut(prompt: &str, model: &str) -> (text: String, cost_usd: f64)`
///
/// Returns an insertion-ordered JSON object matching Python's result dict.
///
/// **Cannot be parity-tested** across languages — requires a live LLM call.
/// Tested in `#[cfg(test)]` with a fake `call` closure.
pub fn judge_score<F>(
    rubric_text: &str,
    instruction: &str,
    findings: &Value,
    judge_model: &str,
    mut call: F,
) -> Result<Map<String, Value>, String>
where
    F: FnMut(&str, &str) -> (String, f64),
{
    let prompt = build_judge_prompt(rubric_text, instruction, findings);
    let (text, cost) = call(&prompt, judge_model);
    let criterion_scores = parse_judge_response(&text)?;
    let score = normalize(&criterion_scores);
    let hash = rubric_hash(rubric_text);

    let mut out: Map<String, Value> = Map::new();
    out.insert("score".into(), Value::from(score));
    out.insert("criterion_scores".into(), Value::Object(criterion_scores));
    out.insert("judge_model".into(), Value::String(judge_model.to_string()));
    out.insert("rubric_hash".into(), Value::String(hash));
    out.insert("cost_usd".into(), Value::from(cost));
    Ok(out)
}

// ---------------------------------------------------------------------------
// spearman
// ---------------------------------------------------------------------------

/// Compute average ranks for a slice; ties get the average of their positions.
/// Mirrors the Python `ranks(vals)` inner function.
fn ranks(vals: &[f64]) -> Vec<f64> {
    let n = vals.len();
    // order[i] = index of the i-th smallest value
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        vals[a]
            .partial_cmp(&vals[b])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut r = vec![0.0f64; n];
    let mut i = 0;
    while i < order.len() {
        let mut j = i;
        // advance j while values are equal (ties)
        while j + 1 < order.len() && vals[order[j + 1]] == vals[order[i]] {
            j += 1;
        }
        // avg = (i + j) / 2 + 1  (1-based rank, average of tied positions)
        let avg = (i + j) as f64 / 2.0 + 1.0;
        for k in i..=j {
            r[order[k]] = avg;
        }
        i = j + 1;
    }
    r
}

/// Spearman rank correlation; 1.0 for perfectly concordant rankings.
/// Ties get average ranks. Returns `None` when either series is constant,
/// len < 2, or the denominator is 0.
///
/// Mirrors `spearman(xs, ys)` in `runner/judge.py` exactly, including use of
/// `statistics.mean` (via `pycompat::mean`).
pub fn spearman(xs: &[f64], ys: &[f64]) -> Option<f64> {
    if xs.len() != ys.len() || xs.len() < 2 {
        return None;
    }
    // constant series: len(set(xs)) == 1
    let xs_unique: std::collections::HashSet<u64> = xs.iter().map(|&x| x.to_bits()).collect();
    let ys_unique: std::collections::HashSet<u64> = ys.iter().map(|&y| y.to_bits()).collect();
    if xs_unique.len() == 1 || ys_unique.len() == 1 {
        return None;
    }

    let rx = ranks(xs);
    let ry = ranks(ys);
    let n = xs.len();

    // Python: mean_rx = statistics.mean(rx); mean_ry = statistics.mean(ry)
    let mean_rx = mean(&rx);
    let mean_ry = mean(&ry);

    let num: f64 = (0..n).map(|i| (rx[i] - mean_rx) * (ry[i] - mean_ry)).sum();
    let den_a: f64 = (0..n).map(|i| (rx[i] - mean_rx).powi(2)).sum();
    let den_b: f64 = (0..n).map(|i| (ry[i] - mean_ry).powi(2)).sum();
    let den = (den_a * den_b).sqrt();

    if den == 0.0 {
        return None;
    }
    Some(round_half_even(num / den, 4))
}

// ---------------------------------------------------------------------------
// calibration_gate
// ---------------------------------------------------------------------------

/// Decide whether a judge family may count toward keep/discard.
///
/// Passes only when two independent judges agree (Spearman ≥ min_agreement)
/// AND, where a deterministic key exists, the judge ranking agrees with it.
///
/// Returns an insertion-ordered JSON object matching Python's result dict.
pub fn calibration_gate(
    judge_a: &[f64],
    judge_b: &[f64],
    deterministic: &[f64],
    min_agreement: f64,
) -> Map<String, Value> {
    let inter = spearman(judge_a, judge_b);
    let vs_key = if deterministic.is_empty() {
        None
    } else {
        spearman(judge_a, deterministic)
    };

    let mut reasons: Vec<String> = Vec::new();
    match inter {
        None => reasons.push("inter-judge agreement undefined (constant scores)".to_string()),
        Some(v) if v < min_agreement => {
            reasons.push(format!("inter-judge Spearman {v} < {min_agreement}"));
        }
        _ => {}
    }
    if !deterministic.is_empty() {
        match vs_key {
            None => reasons.push("judge-vs-key agreement undefined".to_string()),
            Some(v) if v < min_agreement => {
                reasons.push(format!("judge-vs-key Spearman {v} < {min_agreement}"));
            }
            _ => {}
        }
    }

    let passed = reasons.is_empty();

    let mut out: Map<String, Value> = Map::new();
    out.insert("passed".into(), Value::Bool(passed));
    out.insert(
        "inter_judge_spearman".into(),
        match inter {
            Some(v) => Value::from(v),
            None => Value::Null,
        },
    );
    out.insert(
        "judge_vs_key_spearman".into(),
        match vs_key {
            Some(v) => Value::from(v),
            None => Value::Null,
        },
    );
    out.insert("min_agreement".into(), Value::from(min_agreement));
    out.insert(
        "reasons".into(),
        Value::Array(reasons.into_iter().map(Value::String).collect()),
    );
    out
}

// ---------------------------------------------------------------------------
// load_scorer_families
// ---------------------------------------------------------------------------

/// A task declares which families apply in `scoring.toml`; absence means
/// deterministic-only. Mirrors `load_scorer_families(task_dir)`.
///
/// Returns an insertion-ordered JSON Value (TOML structure → JSON).
pub fn load_scorer_families(task_dir: &Path) -> Value {
    let cfg_path = task_dir.join("tests").join("scoring.toml");
    if !cfg_path.exists() {
        let mut m: Map<String, Value> = Map::new();
        m.insert(
            "families".into(),
            Value::Array(vec![Value::String("deterministic".into())]),
        );
        return Value::Object(m);
    }
    let text = match std::fs::read_to_string(&cfg_path) {
        Ok(t) => t,
        Err(_) => {
            let mut m: Map<String, Value> = Map::new();
            m.insert(
                "families".into(),
                Value::Array(vec![Value::String("deterministic".into())]),
            );
            return Value::Object(m);
        }
    };
    let toml_val: toml::Value = match text.parse() {
        Ok(v) => v,
        Err(_) => {
            let mut m: Map<String, Value> = Map::new();
            m.insert(
                "families".into(),
                Value::Array(vec![Value::String("deterministic".into())]),
            );
            return Value::Object(m);
        }
    };
    // Convert toml::Value → serde_json::Value
    toml_to_json(toml_val)
}

/// Recursively convert a `toml::Value` to a `serde_json::Value`.
fn toml_to_json(v: toml::Value) -> Value {
    match v {
        toml::Value::String(s) => Value::String(s),
        toml::Value::Integer(i) => Value::Number(serde_json::Number::from(i)),
        toml::Value::Float(f) => {
            Value::Number(serde_json::Number::from_f64(f).unwrap_or_else(|| 0.into()))
        }
        toml::Value::Boolean(b) => Value::Bool(b),
        toml::Value::Array(arr) => Value::Array(arr.into_iter().map(toml_to_json).collect()),
        toml::Value::Table(t) => {
            let mut m: Map<String, Value> = Map::new();
            for (k, val) in t {
                m.insert(k, toml_to_json(val));
            }
            Value::Object(m)
        }
        toml::Value::Datetime(dt) => Value::String(dt.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Unit tests (porting tests/test_judge.py; fake `call` for judge_score)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const RUBRIC: &str = "\
# Review-quality rubric
Score each 0–5:
- evidence: every finding cites file/line and explains the defect
- actionability: a developer could act on the finding without guessing
- severity_calibration: severity matches real impact
";

    // -----------------------------------------------------------------------
    // rubric_hash
    // -----------------------------------------------------------------------

    #[test]
    fn rubric_hash_is_16_char_hex() {
        let h = rubric_hash(RUBRIC);
        assert_eq!(h.len(), 16);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn rubric_hash_empty_string() {
        // e3b0c44298fc1c14 = sha256("") truncated to 16
        assert_eq!(rubric_hash(""), "e3b0c44298fc1c14");
    }

    #[test]
    fn rubric_hash_deterministic() {
        assert_eq!(rubric_hash("abc"), rubric_hash("abc"));
    }

    // -----------------------------------------------------------------------
    // normalize
    // -----------------------------------------------------------------------

    #[test]
    fn normalize_all_max() {
        let m: Map<String, Value> = [("a", 5), ("b", 5)]
            .iter()
            .map(|(k, v)| (k.to_string(), Value::from(*v as i64)))
            .collect();
        assert_eq!(normalize(&m), 1.0);
    }

    #[test]
    fn normalize_all_zero() {
        let m: Map<String, Value> = [("a", 0), ("b", 0)]
            .iter()
            .map(|(k, v)| (k.to_string(), Value::from(*v as i64)))
            .collect();
        assert_eq!(normalize(&m), 0.0);
    }

    #[test]
    fn normalize_mixed() {
        // mean(3, 0) / 5 = 1.5 / 5 = 0.3
        let m: Map<String, Value> = [("a", 3), ("b", 0)]
            .iter()
            .map(|(k, v)| (k.to_string(), Value::from(*v as i64)))
            .collect();
        assert_eq!(normalize(&m), 0.3);
    }

    #[test]
    fn normalize_empty_returns_zero() {
        let m: Map<String, Value> = Map::new();
        assert_eq!(normalize(&m), 0.0);
    }

    // -----------------------------------------------------------------------
    // parse_judge_response
    // -----------------------------------------------------------------------

    #[test]
    fn parse_scores_wrapper() {
        let out = parse_judge_response(r#"{"scores": {"evidence": 4}}"#).unwrap();
        assert_eq!(out["evidence"], json!(4));
    }

    #[test]
    fn parse_bare_object_with_prose() {
        // bare object without the scores wrapper, surrounded by prose
        let out = parse_judge_response(r#"prose {"evidence": 5, "x": 2} tail"#).unwrap();
        assert_eq!(out["evidence"], json!(5));
        assert_eq!(out["x"], json!(2));
    }

    #[test]
    fn parse_drops_out_of_range_and_non_numeric() {
        let out = parse_judge_response(r#"{"a": 3, "b": 9, "c": "hi"}"#).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out["a"], json!(3));
    }

    #[test]
    fn parse_raises_on_no_json() {
        assert!(parse_judge_response("no json").is_err());
    }

    #[test]
    fn parse_raises_when_all_out_of_range() {
        assert!(parse_judge_response(r#"{"a": 7}"#).is_err());
    }

    #[test]
    fn parse_drops_boolean_values() {
        // booleans are not valid scores even though bool is int subclass in Python
        let result = parse_judge_response(r#"{"a": true, "b": 3}"#);
        let out = result.unwrap();
        assert!(!out.contains_key("a"));
        assert_eq!(out["b"], json!(3));
    }

    #[test]
    fn parse_accepts_boundary_values_0_and_5() {
        let out = parse_judge_response(r#"{"lo": 0, "hi": 5}"#).unwrap();
        assert_eq!(out["lo"], json!(0));
        assert_eq!(out["hi"], json!(5));
    }

    #[test]
    fn parse_malformed_json_advances_to_next() {
        // first object has bad JSON, second is valid
        let out = parse_judge_response(r#"{bad} {"evidence": 3}"#).unwrap();
        assert_eq!(out["evidence"], json!(3));
    }

    // -----------------------------------------------------------------------
    // build_judge_prompt
    // -----------------------------------------------------------------------

    #[test]
    fn build_judge_prompt_structure() {
        let findings = json!({"findings": []});
        let prompt = build_judge_prompt(RUBRIC, "review the diff", &findings);
        assert!(prompt.starts_with(RUBRIC));
        assert!(prompt.contains("## Task the agent was reviewing"));
        assert!(prompt.contains("review the diff"));
        assert!(prompt.contains("## The agent's findings to score"));
        assert!(prompt.contains("Score each rubric criterion from 0 to 5"));
        assert!(prompt.contains(r#"{"scores": {"<criterion>": <0-5>, ...}}"#));
    }

    #[test]
    fn build_judge_prompt_findings_indented() {
        let findings = json!({"k": 1});
        let prompt = build_judge_prompt("rubric", "task", &findings);
        // serde_json pretty-prints with 2-space indent, same as json.dumps(indent=2)
        assert!(prompt.contains("  \"k\": 1"));
    }

    // -----------------------------------------------------------------------
    // judge_score (fake call — unit test only, no parity)
    // -----------------------------------------------------------------------

    #[test]
    fn judge_score_normalizes_and_meters_cost() {
        // fake call returning a canned judge response
        let fake_call = |_prompt: &str, _model: &str| -> (String, f64) {
            (
                serde_json::to_string(&json!({
                    "scores": {
                        "evidence": 4,
                        "actionability": 5,
                        "severity_calibration": 3
                    }
                }))
                .unwrap(),
                0.0009,
            )
        };

        let findings = json!({"findings": []});
        let out = judge_score(
            RUBRIC,
            "review the diff",
            &findings,
            "anthropic/claude-x",
            fake_call,
        )
        .unwrap();

        // (4 + 5 + 3) / 3 / 5 = 4 / 5 = 0.8
        let expected_score = round_half_even((4.0 + 5.0 + 3.0) / 3.0 / 5.0, 4);
        assert_eq!(out["score"], json!(expected_score));
        assert_eq!(out["cost_usd"], json!(0.0009));
        assert_eq!(out["rubric_hash"], json!(rubric_hash(RUBRIC)));
        assert_eq!(out["judge_model"], json!("anthropic/claude-x"));
    }

    #[test]
    fn judge_score_propagates_parse_error() {
        let bad_call = |_: &str, _: &str| -> (String, f64) { ("not json".to_string(), 0.0) };
        let findings = json!({});
        assert!(judge_score(RUBRIC, "task", &findings, "m", bad_call).is_err());
    }

    // -----------------------------------------------------------------------
    // spearman
    // -----------------------------------------------------------------------

    #[test]
    fn spearman_concordant() {
        assert_eq!(
            spearman(&[1.0, 2.0, 3.0, 4.0], &[1.0, 2.0, 3.0, 4.0]),
            Some(1.0)
        );
    }

    #[test]
    fn spearman_discordant() {
        assert_eq!(
            spearman(&[1.0, 2.0, 3.0, 4.0], &[4.0, 3.0, 2.0, 1.0]),
            Some(-1.0)
        );
    }

    #[test]
    fn spearman_constant_series_is_none() {
        assert_eq!(spearman(&[1.0, 1.0, 1.0], &[1.0, 2.0, 3.0]), None);
    }

    #[test]
    fn spearman_single_element_is_none() {
        assert_eq!(spearman(&[1.0], &[1.0]), None);
    }

    #[test]
    fn spearman_mismatched_lengths_is_none() {
        assert_eq!(spearman(&[1.0, 2.0], &[1.0, 2.0, 3.0]), None);
    }

    #[test]
    fn spearman_ties_average() {
        // partial ties in xs and ys — NOT a constant series, so result is defined.
        // Python: spearman([1.0, 1.0, 2.0], [1.0, 1.0, 2.0]) == 1.0
        // ranks([1,1,2]) → order=[0,1,2]; tie at indices 0&1 → avg rank 1.5
        // rx = [1.5, 1.5, 3.0]; ry = [1.5, 1.5, 3.0] → perfectly concordant
        assert_eq!(spearman(&[1.0, 1.0, 2.0], &[1.0, 1.0, 2.0]), Some(1.0));

        // fully constant xs → returns None
        assert_eq!(spearman(&[1.0, 1.0, 1.0], &[1.0, 2.0, 3.0]), None);
    }

    #[test]
    fn spearman_anti_correlated() {
        let xs = vec![0.1, 0.3, 0.5, 0.7, 0.9];
        let ys = vec![0.9, 0.7, 0.5, 0.3, 0.1];
        assert_eq!(spearman(&xs, &ys), Some(-1.0));
    }

    // -----------------------------------------------------------------------
    // calibration_gate
    // -----------------------------------------------------------------------

    #[test]
    fn calibration_gate_passes_when_judges_and_key_agree() {
        let a = vec![0.2, 0.4, 0.6, 0.8, 1.0];
        let b = vec![0.3, 0.45, 0.55, 0.75, 0.95];
        let key = vec![0.0, 0.5, 0.5, 1.0, 1.0];
        let gate = calibration_gate(&a, &b, &key, 0.8);
        assert_eq!(gate["passed"], json!(true));
        let inter = gate["inter_judge_spearman"].as_f64().unwrap();
        assert!(inter >= 0.8);
    }

    #[test]
    fn calibration_gate_fails_on_judge_disagreement() {
        let a = vec![0.1, 0.3, 0.5, 0.7, 0.9];
        let b = vec![0.9, 0.2, 0.8, 0.1, 0.5];
        let gate = calibration_gate(&a, &b, &[], 0.8);
        assert_eq!(gate["passed"], json!(false));
        let reasons = gate["reasons"].as_array().unwrap();
        assert!(reasons
            .iter()
            .any(|r| r.as_str().unwrap_or("").contains("inter-judge")));
    }

    #[test]
    fn calibration_gate_fails_when_judge_contradicts_key() {
        let a = vec![0.2, 0.4, 0.6, 0.8, 1.0];
        let b = vec![0.25, 0.45, 0.55, 0.85, 0.95];
        let key = vec![1.0, 1.0, 0.5, 0.0, 0.0];
        let gate = calibration_gate(&a, &b, &key, 0.8);
        assert_eq!(gate["passed"], json!(false));
        let reasons = gate["reasons"].as_array().unwrap();
        assert!(reasons
            .iter()
            .any(|r| r.as_str().unwrap_or("").contains("judge-vs-key")));
    }

    #[test]
    fn calibration_gate_empty_deterministic_skips_vs_key() {
        let a = vec![0.2, 0.4, 0.6, 0.8, 1.0];
        let b = vec![0.3, 0.45, 0.55, 0.75, 0.95];
        let gate = calibration_gate(&a, &b, &[], 0.8);
        assert_eq!(gate["passed"], json!(true));
        assert_eq!(gate["judge_vs_key_spearman"], json!(null));
    }

    #[test]
    fn calibration_gate_boundary_agreement_at_min() {
        // exactly at min_agreement — must pass
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let b = vec![1.0, 2.0, 3.0, 4.0, 5.0]; // spearman = 1.0
        let gate = calibration_gate(&a, &b, &[], 1.0);
        assert_eq!(gate["passed"], json!(true));
    }

    // -----------------------------------------------------------------------
    // load_scorer_families
    // -----------------------------------------------------------------------

    #[test]
    fn load_scorer_families_defaults_when_no_toml() {
        let tmp = std::env::temp_dir().join(format!("threshold-judge-test-{}", std::process::id()));
        std::fs::create_dir_all(tmp.join("tests")).unwrap();
        let fam = load_scorer_families(&tmp);
        assert_eq!(fam["families"], json!(["deterministic"]));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn load_scorer_families_reads_toml() {
        let tmp =
            std::env::temp_dir().join(format!("threshold-judge-test-toml-{}", std::process::id()));
        std::fs::create_dir_all(tmp.join("tests")).unwrap();
        let toml = concat!(
            "families = [\"deterministic\", \"judge\"]\n",
            "[judge]\n",
            "rubric = \"rubric.md\"\n",
            "models = [\"anthropic/claude-x\", \"openai/gpt-y\"]\n",
        );
        std::fs::write(tmp.join("tests").join("scoring.toml"), toml).unwrap();
        let fam = load_scorer_families(&tmp);
        assert_eq!(fam["families"], json!(["deterministic", "judge"]));
        assert_eq!(fam["judge"]["rubric"], json!("rubric.md"));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
