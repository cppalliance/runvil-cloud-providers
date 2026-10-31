//! The sheet gate: the rendered JSON is checked against this repository's
//! `sheet.schema.json` before it is written anywhere.
//!
//! The publish step overwrites one stable URL that shipped Gateways
//! download, so a sheet that violates the schema must never reach it. Not
//! writing the file is also what keeps change detection from ever seeing a
//! bad sheet: the workflow compares the file that exists.
//!
//! A failed provider fetch is not a schema violation. Those degrade to
//! `stale` or `unavailable` and the build continues; only a document that
//! does not match the schema fails the run.

use std::fmt;

use serde_json::Value;

/// This repository's sheet schema, embedded so a built binary can never
/// validate against a missing file.
const SCHEMA: &str = include_str!("../../../schema/sheet.schema.json");

/// One violated schema keyword, and where in the instance it was violated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// The JSON pointer of the offending instance, empty at the document
    /// root.
    pub pointer: String,
    /// The schema keyword that failed, e.g. `const` or `type`.
    pub keyword: String,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pointer = if self.pointer.is_empty() {
            "(document root)"
        } else {
            self.pointer.as_str()
        };
        write!(f, "{}: {}", pointer, self.keyword)
    }
}

/// Why a sheet could not be checked at all, as distinct from a sheet that
/// was checked and failed.
#[derive(Debug, thiserror::Error)]
pub enum CheckError {
    /// The embedded schema did not compile. A build defect, not a data
    /// defect: the schema is a compile-time constant.
    #[error("the embedded sheet schema did not compile: {0}")]
    Schema(String),
    /// The document did not parse as JSON.
    #[error("the document is not valid JSON: {0}")]
    NotJson(String),
}

/// Checks sheet JSON against the embedded schema.
///
/// # Errors
///
/// Returns [`CheckError::NotJson`] when the text does not parse, and
/// [`CheckError::Schema`] when the embedded schema itself will not
/// compile. A document that parses and violates the schema is
/// `Ok(violations)` with a non-empty list.
pub fn check_json(json: &str) -> Result<Vec<Violation>, CheckError> {
    let instance: Value =
        serde_json::from_str(json).map_err(|error| CheckError::NotJson(error.to_string()))?;
    let schema: Value =
        serde_json::from_str(SCHEMA).map_err(|error| CheckError::Schema(error.to_string()))?;
    let validator = jsonschema::validator_for(&schema)
        .map_err(|error| CheckError::Schema(error.to_string()))?;
    let mut violations = Vec::new();
    for error in validator.iter_errors(&instance) {
        let violation = Violation {
            pointer: error.instance_path().to_string(),
            keyword: error.kind().keyword().to_owned(),
        };
        // One keyword can be reported more than once at the same pointer
        // (an `anyOf` arm per type, for example); a reader wants the
        // distinct findings only.
        if !violations.contains(&violation) {
            violations.push(violation);
        }
    }
    Ok(violations)
}

/// Renders every violation as one indented line, for stderr. No leading or
/// trailing newline: the caller supplies the separator.
#[must_use]
pub fn report(violations: &[Violation]) -> String {
    let mut text = String::new();
    for (index, violation) in violations.iter().enumerate() {
        if index > 0 {
            text.push('\n');
        }
        text.push_str("  ");
        text.push_str(&violation.to_string());
    }
    text
}

#[cfg(test)]
#[path = "validate-tests.rs"]
mod tests;
