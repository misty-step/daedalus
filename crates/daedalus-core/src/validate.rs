//! The Rust validation kernel — one home for schema/receipt/contract checks.
//!
//! Before this module existed, four call sites grew their own `require_*`
//! dialects and their own copies of the Cerberus / launch schema-version
//! string literals (`launch.rs`, `cerberus.rs`, `cerberus_lab.rs`, plus the
//! `doctor.rs` contract walk). Backlog 045 consolidates them here so that
//! schema validation, receipt validation, and launch-contract tooling are a
//! single durable Rust-owned surface that `bin/gate` exercises.
//!
//! ## Parity contract
//!
//! These validators are parity-verified Python ports. The public error
//! *messages* and field-order behavior are load-bearing. The two require-family
//! consumers (`launch.rs` → `ContractValidationError`, `cerberus_lab.rs` →
//! `CerberusLabError`) keep their historical error types as thin newtypes with
//! `From<ValidationError>`; `cerberus.rs` only consumes the `SchemaVersion`
//! registry, so `CerberusExportError` stays untouched (no `From` impl needed).
//! Every message string lives here verbatim — do not reword one without
//! updating the parity/unit tests that pin it.
//!
//! ## Two value worlds
//!
//! Launch contracts and Cerberus reviewer-config packets are TOML
//! (`toml::Value`); Cerberus lab request/artifact records are JSON
//! (`serde_json::Value`). The kernel offers a require-family for each, with the
//! message shape each historical dialect already emitted:
//!   * TOML: `"{label} must be str"`, `"{label} must not be empty"`, … (from
//!     `launch.rs`).
//!   * JSON: `"{label} is required"` (from `cerberus_lab.rs`).

use serde_json::Value as JsonValue;

// ---------------------------------------------------------------------------
// The one error type
// ---------------------------------------------------------------------------

/// The canonical validation error. Callers wrap it in their own newtype via
/// `From<ValidationError>` so existing public APIs and error messages survive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError(pub String);

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ValidationError {}

impl ValidationError {
    /// Construct from anything string-like.
    pub fn new(message: impl Into<String>) -> Self {
        ValidationError(message.into())
    }
}

// ---------------------------------------------------------------------------
// Schema-version registry — the literals previously scattered as `const`s and
// inline string comparisons across cerberus*.rs / cerberus_lab.rs.
// ---------------------------------------------------------------------------

/// Centralized schema-version identifiers. These are the exact strings the
/// accepted records and Cerberus' own validator agree on; changing one is a
/// schema bump, not a refactor.
pub struct SchemaVersion;

impl SchemaVersion {
    /// Cerberus `ReviewRequest.v1` (`cerberus_lab` import boundary).
    pub const CERBERUS_REVIEW_REQUEST: &'static str = "cerberus.review_request.v1";
    /// Cerberus `ReviewArtifact.v1` (`cerberus_lab` import boundary).
    pub const CERBERUS_REVIEW_ARTIFACT: &'static str = "cerberus.review_artifact.v1";
    /// Daedalus cerberus-lab import summary (`cerberus_lab` compare boundary).
    pub const CERBERUS_LAB_IMPORT: &'static str = "cerberus-lab-import.v1";
    /// Daedalus cerberus-lab comparison summary (`cerberus_lab` compare output).
    pub const CERBERUS_LAB_COMPARISON: &'static str = "cerberus-lab-comparison.v1";
    /// Daedalus reviewer-config packet (`cerberus.rs` export output).
    pub const REVIEWER_CONFIG_PACKET: &'static str = "reviewer-config-packet.v1";
    /// Cerberus `ReviewConfig.v1` embedded in the reviewer-config packet.
    pub const REVIEW_CONFIG: &'static str = "review-config.v1";
    /// Launch contract schema version (the integer `contract = 1` discriminant).
    pub const LAUNCH_CONTRACT: i64 = 1;
}

// ---------------------------------------------------------------------------
// TOML require family — lifted verbatim from launch.rs:95-155.
//
// Message strings are byte-identical to the original `ContractValidationError`
// dialect; parity/unit tests pin them.
// ---------------------------------------------------------------------------

/// Require that `table` contains every key in `keys`; error names the missing
/// ones in declaration order. Mirrors launch.py `_require_keys`.
pub fn require_keys(
    table: &toml::Table,
    keys: &[&str],
    label: &str,
) -> Result<(), ValidationError> {
    let missing: Vec<&str> = keys
        .iter()
        .copied()
        .filter(|k| !table.contains_key(*k))
        .collect();
    if !missing.is_empty() {
        return Err(ValidationError(format!(
            "{label} missing required field(s): {}",
            missing.join(", ")
        )));
    }
    Ok(())
}

/// Require a non-empty TOML string. Mirrors launch.py `_require_string`.
pub fn require_string(value: Option<&toml::Value>, label: &str) -> Result<String, ValidationError> {
    match value {
        Some(toml::Value::String(s)) if !s.is_empty() => Ok(s.clone()),
        Some(toml::Value::String(_)) => Err(ValidationError(format!("{label} must not be empty"))),
        _ => Err(ValidationError(format!("{label} must be str"))),
    }
}

/// Require a TOML number (int or float), widened to `f64`. Mirrors launch.py
/// `_require_number`.
pub fn require_number(value: Option<&toml::Value>, label: &str) -> Result<f64, ValidationError> {
    match value {
        Some(toml::Value::Integer(i)) => Ok(*i as f64),
        Some(toml::Value::Float(f)) => Ok(*f),
        _ => Err(ValidationError(format!("{label} must be int|float"))),
    }
}

/// Require a TOML boolean. Mirrors launch.py `_require_bool`.
pub fn require_bool(value: Option<&toml::Value>, label: &str) -> Result<bool, ValidationError> {
    match value {
        Some(toml::Value::Boolean(b)) => Ok(*b),
        _ => Err(ValidationError(format!("{label} must be bool"))),
    }
}

/// Require a TOML array. Mirrors launch.py `_require_array`.
pub fn require_array<'a>(
    value: Option<&'a toml::Value>,
    label: &str,
) -> Result<&'a toml::value::Array, ValidationError> {
    match value {
        Some(toml::Value::Array(a)) => Ok(a),
        _ => Err(ValidationError(format!("{label} must be list"))),
    }
}

// ---------------------------------------------------------------------------
// JSON require family — lifted verbatim from cerberus_lab.rs.
//
// Message strings are byte-identical to the original `CerberusLabError`
// dialect (`"{label} is required"`); the cerberus_lab unit tests pin them.
// ---------------------------------------------------------------------------

/// Walk a nested JSON path and return the leaf string if present. Mirrors
/// cerberus_lab `optional_string`.
pub fn optional_json_string<'a>(value: &'a JsonValue, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_str()
}

/// Require a JSON string at `path`. Mirrors cerberus_lab `require_string`.
pub fn require_json_string<'a>(
    value: &'a JsonValue,
    path: &[&str],
    label: &str,
) -> Result<&'a str, ValidationError> {
    optional_json_string(value, path).ok_or_else(|| ValidationError(format!("{label} is required")))
}

/// Require a JSON string at `path` that is also non-empty after trimming.
/// Mirrors cerberus_lab `require_nonempty_string`.
pub fn require_json_nonempty_string<'a>(
    value: &'a JsonValue,
    path: &[&str],
    label: &str,
) -> Result<&'a str, ValidationError> {
    let text = require_json_string(value, path, label)?;
    if text.trim().is_empty() {
        Err(ValidationError(format!("{label} is required")))
    } else {
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn require_keys_names_missing_in_order() {
        let table: toml::Table = toml::from_str("a = 1\n").unwrap();
        let err = require_keys(&table, &["a", "b", "c"], "thing").unwrap_err();
        assert_eq!(err.to_string(), "thing missing required field(s): b, c");
        assert!(require_keys(&table, &["a"], "thing").is_ok());
    }

    #[test]
    fn require_string_messages_match_launch_dialect() {
        let v = toml::Value::String(String::new());
        assert_eq!(
            require_string(Some(&v), "x").unwrap_err().to_string(),
            "x must not be empty"
        );
        let n = toml::Value::Integer(3);
        assert_eq!(
            require_string(Some(&n), "x").unwrap_err().to_string(),
            "x must be str"
        );
        let ok = toml::Value::String("y".to_string());
        assert_eq!(require_string(Some(&ok), "x").unwrap(), "y");
    }

    #[test]
    fn require_number_bool_array_messages() {
        assert_eq!(
            require_number(None, "n").unwrap_err().to_string(),
            "n must be int|float"
        );
        assert_eq!(
            require_bool(None, "b").unwrap_err().to_string(),
            "b must be bool"
        );
        assert_eq!(
            require_array(None, "a").unwrap_err().to_string(),
            "a must be list"
        );
        assert_eq!(
            require_number(Some(&toml::Value::Integer(2)), "n").unwrap(),
            2.0
        );
    }

    #[test]
    fn json_require_messages_match_cerberus_lab_dialect() {
        let v = json!({"change": {"title": "  "}});
        assert_eq!(
            require_json_string(&v, &["request_id"], "request_id")
                .unwrap_err()
                .to_string(),
            "request_id is required"
        );
        assert_eq!(
            require_json_nonempty_string(&v, &["change", "title"], "change.title")
                .unwrap_err()
                .to_string(),
            "change.title is required"
        );
        let ok = json!({"request_id": "abc"});
        assert_eq!(
            require_json_string(&ok, &["request_id"], "request_id").unwrap(),
            "abc"
        );
    }

    #[test]
    fn schema_registry_holds_the_accepted_literals() {
        assert_eq!(
            SchemaVersion::CERBERUS_REVIEW_REQUEST,
            "cerberus.review_request.v1"
        );
        assert_eq!(
            SchemaVersion::CERBERUS_REVIEW_ARTIFACT,
            "cerberus.review_artifact.v1"
        );
        assert_eq!(
            SchemaVersion::REVIEWER_CONFIG_PACKET,
            "reviewer-config-packet.v1"
        );
        assert_eq!(SchemaVersion::REVIEW_CONFIG, "review-config.v1");
        assert_eq!(SchemaVersion::LAUNCH_CONTRACT, 1);
    }
}
