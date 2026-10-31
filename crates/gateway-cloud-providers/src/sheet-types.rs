//! The sheet vocabulary: the wire types of `cloud-provider-models.json`,
//! the JSON Schema derived from them, and the schema version this writer
//! stamps.
//!
//! The Gateway derives the same schema from its own copy of these types and
//! the two `$defs` objects are compared, so a change here that is not made
//! there shows up as a mismatch rather than as a sheet a shipped Gateway
//! cannot read.
//!
//! `schemars` has no `time` support, and it reads `#[serde(with = ...)]` as a
//! type path, so the four instant and date fields state their shape through
//! `#[schemars(schema_with = ...)]` instead.

use std::collections::BTreeMap;
use std::fmt;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

/// The sheet schema version this writer stamps: the Gateway's reader gates
/// on it. Bump only on removals and renames; additive fields are marked
/// `#[serde(default)]` so a lagging reader survives them.
pub const ACCEPTED_SHEET_SCHEMA_VERSION: u32 = 1;

/// The schema of `Sheet::generated_at`: one RFC 3339 instant, the shape
/// `time::serde::rfc3339` writes.
///
/// `schemars` has no `time` support, and it reads `#[serde(with = "...")]`
/// as a type path, so the shape is stated here instead.
fn generated_at_schema(_: &mut SchemaGenerator) -> Schema {
    schemars::json_schema!({ "type": "string", "format": "date-time" })
}

/// The schema of `ProviderSlice::fetched_at`: an RFC 3339 instant or null.
/// The key is always written, so it stays required and only the value is
/// nullable.
fn fetched_at_schema(_: &mut SchemaGenerator) -> Schema {
    schemars::json_schema!({
        "anyOf": [
            { "type": "string", "format": "date-time" },
            { "type": "null" }
        ]
    })
}

/// The schema of `ModelEntry::released_at`: a calendar date or null, the
/// shape `time`'s `serde-human-readable` rendering gives a `Date`, as in
/// `"2026-07-24"`.
fn released_at_schema(_: &mut SchemaGenerator) -> Schema {
    schemars::json_schema!({
        "anyOf": [
            { "type": "string", "format": "date" },
            { "type": "null" }
        ]
    })
}

/// The schema of `Deprecation::date`: a calendar date or null, in the same
/// rendering as [`released_at_schema`].
fn deprecation_date_schema(_: &mut SchemaGenerator) -> Schema {
    schemars::json_schema!({
        "anyOf": [
            { "type": "string", "format": "date" },
            { "type": "null" }
        ]
    })
}

/// The sheet envelope: one atomic snapshot of every provider's models.
/// Future additive fields are marked `#[serde(default)]` so a lagging
/// reader survives them; `schema_version` bumps only on removals and
/// renames.
#[derive(Debug, Clone, JsonSchema, Serialize, Deserialize)]
pub struct Sheet {
    /// Bumped on breaking change. Stays 1: additive fields are marked
    /// `#[serde(default)]`; the bump is reserved for removals and renames.
    //
    // The `1` is a literal on purpose. `extend` emits its token stream as a
    // JSON value, so a path to the constant would not be evaluated; the
    // schema test fails if this literal and the constant ever disagree.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// RFC 3339; always this run's time.
    #[serde(with = "time::serde::rfc3339")]
    #[schemars(schema_with = "generated_at_schema")]
    pub generated_at: OffsetDateTime,
    /// Keyed by provider name, e.g. "anthropic".
    pub providers: BTreeMap<String, ProviderSlice>,
}

/// One provider's slice of the sheet. Self-describing: the descriptor's
/// public fields are copied in at build time so consumers can render a
/// provider dropdown from the sheet alone. Future additive fields are
/// marked `#[serde(default)]`; the schema version bump is reserved for
/// removals and renames.
#[derive(Debug, Clone, JsonSchema, Serialize, Deserialize)]
pub struct ProviderSlice {
    /// UI-facing name, e.g. "Anthropic".
    pub display_name: String,
    /// The provider's tier, a curated product opinion.
    pub tier: Tier,
    /// Freshness of this slice.
    pub status: SliceStatus,
    /// Last fresh fetch; absent for `static` slices.
    #[serde(with = "time::serde::rfc3339::option")]
    #[schemars(schema_with = "fetched_at_schema")]
    pub fetched_at: Option<OffsetDateTime>,
    /// The base URL of the provider's OpenAI-compatible chat API - the
    /// value an `[[endpoint]]` needs - or `None` when the provider has
    /// no such API.
    pub openai_base_url: Option<String>,
    /// The provider's environment variables, copied from the descriptor
    /// at build time so a consumer can render from the sheet alone.
    pub env_vars: Vec<EnvVar>,
    /// The provider's normalized model entries.
    pub models: Vec<ModelEntry>,
}

/// One environment variable a provider reads.
#[derive(Debug, Clone, JsonSchema, Serialize, Deserialize)]
pub struct EnvVar {
    /// The variable name, e.g. "ANTHROPIC_API_KEY".
    pub name: String,
    /// How the variable is used.
    pub role: EnvRole,
    /// The value the provider assumes when the variable is unset.
    pub default: Option<String>,
}

/// How a provider uses an environment variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum EnvRole {
    /// A credential: the variable holds API key material.
    Key,
    /// Configuration: the variable selects a region, endpoint, or similar.
    Config,
}

/// Curated product opinion, not a vendor fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Tier {
    /// The frontier providers.
    Prime,
    /// Credible challengers.
    Subprime,
    /// Specialized or regional providers.
    Niche,
    /// Resellers of other providers' models.
    Aggregator,
}

/// Freshness of one provider's slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum SliceStatus {
    /// Fetched fresh this run.
    Ok,
    /// Copied verbatim from the previous sheet after a failed fetch.
    Stale,
    /// No previous slice and the fetch failed; `models` is empty.
    Unavailable,
    /// Curated by hand; never fetched.
    Static,
}

/// One normalized model entry. Future additive fields are marked
/// `#[serde(default)]`; the schema version bump is reserved for removals
/// and renames.
#[expect(
    clippy::struct_excessive_bools,
    reason = "the modality and capability booleans are the sheet schema itself; a builder or sub-struct would only obscure the wire shape"
)]
#[derive(Debug, Clone, JsonSchema, Serialize, Deserialize)]
pub struct ModelEntry {
    /// The upstream slug.
    pub id: String,
    /// UI-facing name.
    pub display_name: String,
    /// The UI's first grouping level, e.g. "claude-opus", "gpt-5.4".
    pub family: String,
    /// The canonical alias this entry is a dated snapshot or SKU variant
    /// of; set only when that alias exists in the same provider's list.
    pub variant_of: Option<String>,
    /// The stripped suffix, e.g. `2026-03-05`, `free`, `batch`, for the
    /// UI's expander label.
    pub variant: Option<String>,
    /// The provider's own language codes, passed through as reported;
    /// empty when unknown or not applicable.
    pub languages: Vec<String>,
    /// The workload: chat, embedding, classifier, speech (TTS),
    /// transcription (STT), image, video.
    #[serde(default)]
    pub kind: ModelKind,
    /// Optional: not every provider reports it. Absent reads as `None`, so
    /// the field is not required; the writer still emits the key.
    #[serde(default)]
    #[schemars(schema_with = "released_at_schema")]
    pub released_at: Option<Date>,
    /// Optional: the IDs-only providers omit it.
    pub context_window: Option<u32>,
    /// Optional maximum completion tokens.
    pub max_output: Option<u32>,
    // Modalities.
    /// Accepts image input.
    pub images: bool,
    /// Accepts PDF input.
    pub pdf_input: bool,
    /// Accepts video input.
    pub video_input: bool,
    /// Accepts audio input.
    pub audio_input: bool,
    // Capabilities.
    /// Supports batch submission.
    pub batch: bool,
    /// Returns grounded citations.
    pub citations: bool,
    /// Can execute code server-side.
    pub code_execution: bool,
    /// Honors response schemas.
    pub structured_outputs: bool,
    /// Emits tool calls.
    pub tool_calling: bool,
    /// Reasoning capability.
    pub thinking: Thinking,
    /// The provider's own level names, e.g. `["low", "high", "max"]`;
    /// never mapped to a cross-provider scale.
    pub effort_levels: Vec<String>,
    /// The provider's own default level name.
    pub default_effort: Option<String>,
    /// Normalized to per-million-token units.
    pub pricing: Option<Pricing>,
    /// Sunset information, when the provider reports it.
    pub deprecation: Option<Deprecation>,
}

/// The workload a model serves: chat completions, embeddings,
/// classification, speech synthesis, transcription, or image or video
/// generation.
///
/// The kind scopes which configuration fields are meaningful: chat-only
/// fields (for example `thinking`, `default_max_tokens`,
/// `chat_template_file`) are rejected for non-chat kinds at validation,
/// while `context` applies to every kind. The catalog includes the kind so
/// clients can filter before building a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, JsonSchema, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ModelKind {
    /// Chat completions (`POST /v1/chat/completions`). The default.
    #[default]
    Chat,
    /// Text embeddings.
    Embedding,
    /// Classification / reranking.
    Classifier,
    /// Speech synthesis (`POST /v1/audio/speech`).
    Speech,
    /// Speech-to-text transcription.
    Transcription,
    /// Image generation.
    Image,
    /// Video generation.
    Video,
}

impl fmt::Display for ModelKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let spelling = match self {
            ModelKind::Chat => "chat",
            ModelKind::Embedding => "embedding",
            ModelKind::Classifier => "classifier",
            ModelKind::Speech => "speech",
            ModelKind::Transcription => "transcription",
            ModelKind::Image => "image",
            ModelKind::Video => "video",
        };
        f.write_str(spelling)
    }
}

/// Reasoning capability.
#[derive(Debug, Clone, Copy, Default, JsonSchema, Serialize, Deserialize)]
pub struct Thinking {
    /// Any reasoning capability at all.
    pub supported: bool,
    /// Manual budget mode (Anthropic "enabled").
    pub enabled: bool,
    /// Model-chosen thinking depth.
    pub adaptive: bool,
}

/// Token pricing, normalized to per-million-token units.
#[derive(Debug, Clone, JsonSchema, Serialize, Deserialize)]
pub struct Pricing {
    /// ISO 4217, e.g. "USD", "CNY".
    pub currency: String,
    /// Prompt price per million tokens.
    pub prompt_per_mtok: f64,
    /// Completion price per million tokens.
    pub completion_per_mtok: f64,
}

/// Sunset information, when the provider reports it.
#[derive(Debug, Clone, JsonSchema, Serialize, Deserialize)]
pub struct Deprecation {
    /// Provider's own lifecycle label, e.g. "LEGACY".
    pub status: String,
    /// The sunset date, when known. Absent reads as `None`, so the field is
    /// not required; the writer still emits the key.
    #[serde(default)]
    #[schemars(schema_with = "deprecation_date_schema")]
    pub date: Option<Date>,
    /// The successor model id, when named.
    pub replacement: Option<String>,
}
