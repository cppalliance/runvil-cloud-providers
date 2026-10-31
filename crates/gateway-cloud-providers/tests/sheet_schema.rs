//! The sheet schema check: the committed `schema/sheet.schema.json` must
//! equal the schema derived from the sheet types.
//!
//! `BLESS_SCHEMA=1 cargo test --test sheet_schema` rewrites the artifact.
//!
//! The Gateway derives the same schema from its own copy of these types, and
//! CI compares the two `$defs` objects. A type that drifts here is a schema
//! that drifts there, which is what the comparison exists to catch.

use std::env;
use std::fs;
use std::path::PathBuf;

use gateway_cloud_providers::Sheet;
use serde_json::Value;

/// The one command that rewrites the artifact.
const REGENERATE: &str = "BLESS_SCHEMA=1 cargo test --test sheet_schema";

/// The committed artifact, resolved from the manifest so the check does not
/// depend on the working directory.
fn artifact_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("schema")
        .join("sheet.schema.json")
}

/// `value` with every object's keys in sorted order, so the artifact renders
/// the same whatever order the generator and the JSON map give.
fn canonical(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut keys: Vec<&String> = object.keys().collect();
            keys.sort();
            Value::Object(
                keys.into_iter()
                    .map(|key| (key.clone(), canonical(&object[key])))
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(items.iter().map(canonical).collect()),
        scalar => scalar.clone(),
    }
}

/// The artifact text: two-space indent, sorted keys, a trailing newline, and
/// no `$schema` URL, so a schemars bump does not churn the file.
fn render(schema: &Value) -> String {
    let mut value = canonical(schema);
    if let Value::Object(object) = &mut value {
        object.remove("$schema");
    }
    let mut text = serde_json::to_string_pretty(&value)
        .unwrap_or_else(|error| panic!("the derived schema must serialize: {error}"));
    text.push('\n');
    text
}

#[test]
fn the_committed_schema_matches_the_derived_one() {
    let derived_text = render(&schemars::schema_for!(Sheet).to_value());
    let path = artifact_path();
    let bless = env::var_os("BLESS_SCHEMA").is_some();
    let committed_text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            assert!(
                bless,
                "{} is missing; write it with: {REGENERATE}",
                path.display()
            );
            fs::write(&path, &derived_text)
                .unwrap_or_else(|error| panic!("{}: unwritable: {error}", path.display()));
            return;
        }
        Err(error) => panic!("{}: unreadable: {error}", path.display()),
    };
    if committed_text == derived_text {
        return;
    }
    if bless {
        fs::write(&path, &derived_text)
            .unwrap_or_else(|error| panic!("{}: unwritable: {error}", path.display()));
        return;
    }
    panic!(
        "{} is stale: it does not match the schema derived from the sheet types.\n\
         Rewrite it with: {REGENERATE}",
        path.display()
    );
}
