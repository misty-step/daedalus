//! Small helpers that reproduce Python semantics exactly, so ports can match
//! the reference `runner/` implementation bit-for-bit. Shared by every module
//! that does Python-flavored arithmetic, truthiness, or stringification.

use serde_json::Value;

/// Replicate Python's `round(x, ndigits)`: correctly-rounded, half-to-even on
/// the *exact* binary value of `x`.
///
/// We format to `ndigits` decimals and parse back: Rust's float formatter
/// rounds half-to-even on the exact value, exactly like CPython's `round()`.
/// The naive `(x * 10^n).round() / 10^n` perturbs `x` before rounding and
/// diverges from Python at decimal half-points, so it is not used.
pub fn round_half_even(x: f64, ndigits: u32) -> f64 {
    if !x.is_finite() {
        return x;
    }
    format!("{x:.*}", ndigits as usize)
        .parse()
        .expect("formatted finite float parses")
}

/// Replicate Python truthiness (`bool(value)`) for a JSON value: `None`,
/// `false`, `0`, `""`, `[]`, and `{}` are falsy; everything else is truthy.
pub fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(true),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// Replicate Python's `str(value)` for the scalar JSON values that appear in
/// id/label fields (used inside f-strings). Collections fall back to a JSON
/// rendering — Python `repr` differs there, but those fields are never
/// collections in practice.
pub fn py_str(value: &Value) -> String {
    match value {
        Value::Null => "None".to_string(),
        Value::Bool(true) => "True".to_string(),
        Value::Bool(false) => "False".to_string(),
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

/// Arithmetic mean of a non-empty slice. Mirrors `statistics.mean` for typical
/// small float sequences. (CPython's `statistics.mean` sums exactly via
/// `Fraction`, so results can differ by 1 ULP on adversarial inputs; ports that
/// need bit-exactness against it should parity-test and note any divergence.)
pub fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

/// Current UTC time formatted as `%Y-%m-%dT%H:%M:%SZ` — the `generated`-field
/// default used by swarm/export when no timestamp is supplied
/// (`value or datetime.now(timezone.utc).strftime(...)`). Wall-clock, so it is
/// never parity-tested; the deterministic formatting is.
pub fn utc_now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    format_unix_utc(secs)
}

/// Format Unix seconds as `%Y-%m-%dT%H:%M:%SZ` (UTC).
fn format_unix_utc(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (y, mo, d) = civil_from_days(days);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

/// Howard Hinnant's days-from-civil inverse: days since 1970-01-01 → (y, m, d).
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (y + i64::from(m <= 2), m as u32, d)
}

/// Howard Hinnant's days-from-civil: (y, m, d) → days since 1970-01-01.
/// Use for date arithmetic, e.g. `days_from_civil(today) - days_from_civil(d)`
/// reproduces Python's `(today - d).days`.
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m as i64 - 3 } else { m as i64 + 9 };
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rounds_half_to_even() {
        assert_eq!(round_half_even(0.5, 0), 0.0);
        assert_eq!(round_half_even(1.5, 0), 2.0);
        assert_eq!(round_half_even(2.5, 0), 2.0);
        assert_eq!(round_half_even(0.125, 2), 0.12);
    }

    #[test]
    fn truthiness_matches_python() {
        assert!(!is_truthy(&Value::Null));
        assert!(!is_truthy(&json!(0)));
        assert!(!is_truthy(&json!(0.0)));
        assert!(!is_truthy(&json!("")));
        assert!(!is_truthy(&json!([])));
        assert!(!is_truthy(&json!({})));
        assert!(is_truthy(&json!(0.01)));
        assert!(is_truthy(&json!("x")));
        assert!(!is_truthy(&json!(false)));
    }

    #[test]
    fn py_str_matches_python() {
        assert_eq!(py_str(&Value::Null), "None");
        assert_eq!(py_str(&json!(true)), "True");
        assert_eq!(py_str(&json!(false)), "False");
        assert_eq!(py_str(&json!("seed1")), "seed1");
        assert_eq!(py_str(&json!(3)), "3");
    }

    #[test]
    fn formats_unix_utc_at_known_epochs() {
        assert_eq!(format_unix_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_unix_utc(946_684_800), "2000-01-01T00:00:00Z");
        assert_eq!(format_unix_utc(1_000_000_000), "2001-09-09T01:46:40Z");
    }

    #[test]
    fn days_from_civil_known_and_roundtrip() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 1, 1), 10_957);
        assert_eq!(days_from_civil(2001, 9, 9), 11_574);
        // day diff in days, like Python (today - verified).days
        assert_eq!(
            days_from_civil(2026, 6, 16) - days_from_civil(2026, 6, 1),
            15
        );
        for z in [-1000_i64, 0, 1, 10_957, 20_000, 100_000] {
            let (y, m, d) = civil_from_days(z);
            assert_eq!(days_from_civil(y, m, d), z);
        }
    }
}
