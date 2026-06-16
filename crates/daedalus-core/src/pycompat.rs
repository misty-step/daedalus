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
}
