//! Tests for the sheet gate: the schema compiles, a real sheet passes, and
//! a corrupted instance is reported with its pointer and keyword.

use super::{check_json, report};

/// The smallest document the schema accepts: the three required envelope
/// keys and one provider slice with no models.
const MINIMAL: &str = r#"{
  "schema_version": 1,
  "generated_at": "2026-10-31T00:00:00Z",
  "providers": {
    "anthropic": {
      "display_name": "Anthropic",
      "tier": "prime",
      "status": "ok",
      "fetched_at": null,
      "openai_base_url": null,
      "env_vars": [],
      "models": []
    }
  }
}"#;

#[test]
fn the_embedded_schema_compiles_and_accepts_a_minimal_sheet() {
    let violations = check_json(MINIMAL).expect("the embedded schema must compile");
    assert!(
        violations.is_empty(),
        "a minimal sheet must pass: {}",
        report(&violations)
    );
}

#[test]
fn a_wrong_schema_version_is_reported_at_its_pointer() {
    let corrupted = MINIMAL.replace("\"schema_version\": 1", "\"schema_version\": 2");
    let violations = check_json(&corrupted).expect("the embedded schema must compile");
    let pointers: Vec<&str> = violations.iter().map(|v| v.pointer.as_str()).collect();
    assert!(
        pointers.contains(&"/schema_version"),
        "the violation must name the field: {}",
        report(&violations)
    );
    let keywords: Vec<&str> = violations.iter().map(|v| v.keyword.as_str()).collect();
    assert!(
        keywords.contains(&"const"),
        "the violated keyword is the `const` on the version: {}",
        report(&violations)
    );
}

#[test]
fn a_missing_required_field_is_reported_at_its_pointer() {
    let corrupted = MINIMAL.replace("\"tier\": \"prime\",", "");
    let violations = check_json(&corrupted).expect("the embedded schema must compile");
    assert!(
        violations
            .iter()
            .any(|v| v.pointer == "/providers/anthropic" && v.keyword == "required"),
        "the slice must report `required`: {}",
        report(&violations)
    );
}

#[test]
fn a_model_entry_with_a_bad_kind_is_reported_at_its_index() {
    let corrupted = MINIMAL.replace(
        "\"models\": []",
        r#""models": [{
          "id": "m1",
          "display_name": "M1",
          "family": "m",
          "variant_of": null,
          "variant": null,
          "languages": [],
          "kind": "telepathy",
          "released_at": null,
          "context_window": null,
          "max_output": null,
          "images": false,
          "pdf_input": false,
          "video_input": false,
          "audio_input": false,
          "batch": false,
          "citations": false,
          "code_execution": false,
          "structured_outputs": false,
          "tool_calling": false,
          "thinking": { "supported": false, "enabled": false, "adaptive": false },
          "effort_levels": [],
          "default_effort": null,
          "pricing": null,
          "deprecation": null
        }]"#,
    );
    let violations = check_json(&corrupted).expect("the embedded schema must compile");
    assert!(
        violations
            .iter()
            .any(|v| v.pointer == "/providers/anthropic/models/0/kind"),
        "the violation must point into the model array: {}",
        report(&violations)
    );
}

#[test]
fn a_non_json_document_is_not_a_schema_verdict() {
    let error = check_json("this is not a sheet").expect_err("non-JSON must be its own error");
    assert!(
        error.to_string().contains("not valid JSON"),
        "the error must say the document does not parse: {error}"
    );
}

#[test]
fn the_report_renders_one_line_per_violation() {
    let corrupted = MINIMAL.replace("\"schema_version\": 1", "\"schema_version\": 2");
    let violations = check_json(&corrupted).expect("the embedded schema must compile");
    let text = report(&violations);
    assert_eq!(
        text.lines().count(),
        violations.len(),
        "one line per violation: {text}"
    );
    assert!(
        report(&violations).contains("/schema_version"),
        "the pointer is printed: {}",
        report(&violations)
    );
}
