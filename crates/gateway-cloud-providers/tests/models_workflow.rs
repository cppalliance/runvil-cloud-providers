//! The data workflow stays pointed at the rolling `models` tag.
//!
//! `schema/pin.json` names a Gateway commit so the schema cross-check in
//! `ci.yml` has a fixed thing to compare against. The weekly data refresh
//! must not read it: a pin exists to freeze the schema's shape, and a
//! stale pin must never be able to freeze the models the sheet reports.

/// The workflow that publishes the sheet.
const MODELS_WORKFLOW: &str = include_str!("../../../.github/workflows/models.yml");

/// The rolling tag both the publish step and the previous-sheet download
/// address.
const ROLLING_RELEASE_URL: &str =
    "https://github.com/cppalliance/runvil-cloud-providers/releases/download/models";

#[test]
fn the_data_workflow_reads_the_rolling_tag_for_both_urls() {
    assert!(
        MODELS_WORKFLOW.contains(&format!(
            "MODELS_SHEET_PREVIOUS_URL: {ROLLING_RELEASE_URL}/cloud-provider-models.json"
        )),
        "the previous-sheet URL must address the rolling `models` tag"
    );
    assert!(
        MODELS_WORKFLOW.contains(&format!("{ROLLING_RELEASE_URL}/cloud-provider-models.json")),
        "the publish step must address the rolling `models` tag"
    );
    // Two places name the release: the previous-sheet env and the
    // change-detection download. The publish step uploads a local file by
    // name into the same tag, so a third URL there would be a different
    // shape of address, not a second one.
    assert_eq!(
        MODELS_WORKFLOW
            .matches("releases/download/models/cloud-provider-models.json")
            .count(),
        2,
        "the previous-sheet env and the change-detection download are the \
         two places the asset URL is written"
    );
    assert!(
        MODELS_WORKFLOW.contains("gh release upload models \"$SHEET\""),
        "the publish step must upload into the rolling `models` tag"
    );
}

#[test]
fn the_data_workflow_never_reads_the_schema_pin() {
    for forbidden in ["pin.json", "schema/pin", "schema/pin.json"] {
        assert!(
            !MODELS_WORKFLOW.contains(forbidden),
            "the data workflow must not read `{forbidden}`: a stale schema pin \
             would then be able to freeze the weekly data refresh"
        );
    }
}
