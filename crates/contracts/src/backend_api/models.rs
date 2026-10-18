//! The model catalog (`/models`): models, aliases, prices, the candidates a provider lists, and model probes.

use serde::{Deserialize, Serialize};

use crate::run_record::HarnessFamily;

/// The `GET /models/{slug}/candidates` response: the candidate list the next gg enqueue of the
/// model would build, for an agent that sets no reasoning.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ModelCandidatesOut {
    /// The OpenRouter id the endpoints listing was read under.
    pub model_id: String,
    /// The native quantization the filter kept: the catalog entry's, else the highest level any
    /// endpoint declares. Null when neither names one, which the list survives only with the
    /// catalog entry's quantization filter off.
    pub native_quantization: Option<String>,
    /// The candidates, in the order a run tries them. Empty exactly when
    /// [`refusal`](Self::refusal) is set.
    pub candidates: Vec<CandidateOut>,
    /// Why the list is empty, naming the filter that emptied it, or null when it is not.
    pub refusal: Option<String>,
}

/// One candidate of a model's list.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CandidateOut {
    /// The provider, spelled as OpenRouter's endpoints listing spells its `provider_name`.
    pub provider: String,
    /// The quantization its endpoint declares.
    pub quantization: String,
    /// Whether this is the model developer's own endpoint.
    pub developer: bool,
    /// The input price, USD per million tokens.
    pub input_price: f64,
    /// The output price, USD per million tokens.
    pub output_price: f64,
    /// The cache-read price, USD per million tokens.
    pub cache_read_price: f64,
    /// The provider's recorded fault rate for the model: stalls, unexpected cache misses and
    /// turns that ended on its failed model calls, over its calls. Null when no recorded run of
    /// the model used the provider, which the order reads as zero.
    pub fault_rate: Option<f64>,
}

/// The `POST /models/{slug}/probes` request body. Everything is optional: an
/// empty body probes every language arm over the default route with the
/// default sampling.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ProbeTriggerInput {
    /// Pin every call to this provider (`provider.order` with fallbacks
    /// disabled). Absent probes the default route.
    pub provider: Option<String>,
    /// Probe this one program-language arm, by its wire id (`typescript`,
    /// `rust`, …). Absent probes every arm.
    pub language: Option<String>,
    /// Completion calls per input prompt (default 8, at most 128).
    pub samples: Option<i32>,
    /// Completion-token cap per call (default 3500).
    pub max_tokens: Option<i32>,
}

/// One probe, as every probe read returns it (the detail read adds the items
/// and the per-case requests).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ModelProbeOut {
    pub id: String,
    /// The catalog slug the probe was triggered from.
    pub model_slug: String,
    /// The OpenRouter slug the completions were requested under.
    pub openrouter_slug: String,
    /// The pinned provider, or null for the default route.
    pub provider: Option<String>,
    /// The probed program-language arm's wire id, or null for every language.
    pub language: Option<String>,
    /// Completion calls requested per input prompt.
    pub samples: i32,
    pub max_tokens: i32,
    /// `running`, `complete`, or `failed`.
    pub status: String,
    /// Why the probe failed, or null.
    pub error: Option<String>,
    /// `ready` or `not-ready`; null until the probe completes.
    pub verdict: Option<String>,
    /// The probe's overall case-check pass rate (0..=1), or null.
    pub pass_rate: Option<f64>,
    /// Total USD spend across the probe's calls, as OpenRouter reported it.
    pub spend: f64,
    pub created_at: String,
    pub finished_at: Option<String>,
}

/// One completion call inside a probe: which case it sampled, which provider
/// served it, how it finished, whether the submitted program passed its case's
/// check, and the raw reply.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ModelProbeItemOut {
    pub id: String,
    /// The program-language arm's wire id this call probed.
    pub language: String,
    /// The case's scenario: `baseline` or `missing-docview`.
    pub scenario: String,
    /// The case's input prompt id.
    pub prompt: String,
    /// The sample index within the case, from 0.
    pub sample: i32,
    /// The provider OpenRouter reported serving the call, or null on error.
    pub provider: Option<String>,
    pub finish_reason: Option<String>,
    pub native_finish_reason: Option<String>,
    /// The classified outcome (`correct-calls`, `docview-first`,
    /// `called-undocumented`, `fenced`, …), or null when the call errored.
    pub label: Option<String>,
    /// Whether the submitted program passed its case's check.
    pub pass: bool,
    /// The program string the reply's first `submit_program` call carried, or
    /// null.
    pub program_text: Option<String>,
    /// The reply's text content beside the call, verbatim.
    pub response_text: String,
    /// The reply's separate reasoning stream, or null.
    pub reasoning_text: Option<String>,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    /// The call's USD cost, or null.
    pub cost: Option<f64>,
    pub duration_ms: i64,
    /// The transport or gateway error that voided the call, or null.
    pub error: Option<String>,
    pub created_at: String,
}

/// The `POST /models/{slug}/probes` response: the probe row, already running.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ProbeTriggerResponse {
    pub probe: ModelProbeOut,
}

/// The `GET /models/{slug}/probes` response, newest first.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ModelProbesResponse {
    pub probes: Vec<ModelProbeOut>,
}

/// The `GET /model-probes/{id}` response: the probe with everything the console
/// shows — every case's request messages exactly as sent, every call's
/// classification, the submitted programs, and the raw replies. Each request
/// also carried the case's `submit_program` tool definition with `tool_choice`
/// forced to it; that pair is part of the embedded fixture rather than a
/// response field.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ModelProbeDetailResponse {
    pub probe: ModelProbeOut,
    pub items: Vec<ModelProbeItemOut>,
    /// The per-case requests exactly as sent, one entry per probed
    /// (language, scenario, prompt).
    pub requests: Vec<super::probe::ProbeRequestOut>,
}

/// The `GET /models/{slug}/probe-providers` response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ProbeProvidersResponse {
    /// The OpenRouter slug the providers were enumerated for.
    pub openrouter_slug: String,
    pub providers: Vec<ProbeProviderOut>,
}

/// One provider route OpenRouter lists for the model.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ProbeProviderOut {
    /// The provider's display name — the value a probe pins with.
    pub name: String,
    /// The route's context window in tokens, or null when unreported.
    pub context_length: Option<u64>,
}

/// `GET /models` — the merged model catalog.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ModelCatalogResponse {
    pub models: Vec<ModelOut>,
}

/// One catalog entry: a curated model merged with its runs and its observed
/// catalog facts, or a model derived from runs with no curated config.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ModelOut {
    /// The curated slug, or the canonical model id for a derived model.
    pub slug: String,
    /// The display name (a derived model uses its canonical id).
    pub name: String,
    /// The provider (guessed from the id for a derived model).
    pub provider: String,
    /// Whether this entry has curated config, as opposed to being derived from
    /// runs alone.
    pub curated: bool,
    /// `https://openrouter.ai/<slug>` when the model is on OpenRouter, else null.
    pub openrouter_url: Option<String>,
    /// Curated description markdown, or null.
    pub description: Option<String>,
    /// The curated, sanitized provider-logo SVG, or null.
    pub logo_svg: Option<String>,
    /// The raw `subject.modelId` strings from runs this entry absorbs — what the
    /// console matches a run against.
    pub covered_model_ids: Vec<String>,
    /// The canonical model ids this entry claims, each tagged with the harness
    /// family it is usable with (so a run form can offer only the slugs the
    /// selected harness can launch).
    pub aliases: Vec<AliasOut>,
    /// The model's list price (per-token USD), the only price a model has and
    /// what a run's comparable cost is computed from — all-or-nothing: `Some`
    /// only when all three prices are set. Null for a derived or unpriced model.
    pub list_price: Option<ModelPricesOut>,
    /// The date the list-price figures were taken (`YYYY-MM-DD`): the day they
    /// were last read from OpenRouter, or the date recorded with a set entered
    /// by hand. Null when the model carries no list price.
    pub list_price_as_of: Option<String>,
    /// Where the list price came from: `openrouter` for a set read from
    /// OpenRouter (by a refresh of every entry, see
    /// `bootstrap::refresh_list_prices`, or at enqueue), `hand` for a
    /// set an operator entered, which stands until the next refresh resolves a
    /// rate for the model, or null when the model carries none.
    pub list_price_source: Option<String>,
    /// The latest observed context window in tokens, or null.
    pub context_length: Option<u64>,
    /// The developer provider: the OpenRouter provider name of the model developer's own
    /// endpoint. The hand-set one when the catalog entry sets it, otherwise the one last observed
    /// on the endpoints listing. Null means none is known; a gg run's
    /// [candidate list](https://docs.testcabinet.ai/gg/overview/#the-candidate-list) then takes
    /// the catalog entry's price ceiling in place of the developer's rates.
    pub provider_pin: Option<String>,
    /// Whether [`provider_pin`](Self::provider_pin) is set by hand on the catalog entry rather
    /// than observed on the listing.
    pub provider_pin_set_by_hand: bool,
    /// Whether a gg run's candidate list is filtered to the native quantization. False accepts
    /// every endpoint at whatever level it declares, `unknown` included, for a model every
    /// provider serves at one precision nobody discloses. Always true for a derived model.
    pub quantization_filter: bool,
    /// The native quantization set by hand (`fp8`, `bf16`, …), or null to take the highest
    /// level any endpoint declares. Always null for a derived model.
    pub native_quantization: Option<String>,
    /// The input half of the price ceiling, USD per million tokens, used when OpenRouter lists no
    /// developer endpoint. Null when no ceiling is set, and always for a derived model.
    pub max_input_price: Option<f64>,
    /// The output half of the price ceiling, USD per million tokens.
    pub max_output_price: Option<f64>,
    /// The providers a gg run of the model never uses. Empty for a derived model.
    pub banned_providers: Vec<String>,
    /// The providers accepted despite declaring `unknown` quantization. Empty for a derived
    /// model.
    pub unknown_quantization_providers: Vec<String>,
    /// The latest observed release date (RFC 3339), or null.
    pub released_at: Option<String>,
    /// The input modalities OpenRouter reports the model accepts (`text`,
    /// `image`, `file`, …), lowercased. **Empty means unobserved**, not "text
    /// only" — the catalog has simply not recorded a modality list for this model
    /// yet, and a consumer deciding whether it may send an image treats that as
    /// unknown rather than as a refusal.
    pub input_modalities: Vec<String>,
}

/// One canonical model id a catalog entry claims, with the harness family it is
/// usable with. The run form filters the models it offers a harness by matching
/// the harness's family against these.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct AliasOut {
    /// The canonical model id (an OpenRouter id like `anthropic/claude-opus-4.8`,
    /// or a provider-native id like `claude-opus-4-8`).
    pub slug: String,
    /// The harness family this slug is usable with.
    pub harness_family: HarnessFamily,
}

/// A per-token price triple, in USD: a model's list price.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ModelPricesOut {
    pub uncached_input: Option<f64>,
    pub cached_input: Option<f64>,
    pub output: Option<f64>,
}

/// One slug ↔ harness-family pairing in a config write. The operator supplies a
/// slug and picks the harness family it belongs to; a model with slugs across
/// several families carries one entry per (slug, family).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct AliasInput {
    /// The canonical model id (the `openrouter/` routing prefix is stripped on
    /// write so it matches a run's canonical id).
    pub slug: String,
    /// The harness family this slug is usable with.
    pub harness_family: HarnessFamily,
}

/// The `POST /models` / `PUT /models/{slug}` request body.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ModelConfigInput {
    /// The curated slug (used on create; the path wins on update).
    pub slug: String,
    pub name: String,
    pub provider: String,
    /// The canonical model ids this model covers, each paired with the harness
    /// family it is usable with (at least one).
    pub aliases: Vec<AliasInput>,
    pub openrouter_slug: Option<String>,
    /// The developer provider: the OpenRouter provider name of the model developer's own
    /// endpoint, set by hand where the endpoints listing's name does not match the model id's
    /// author segment. Absent or blank takes the provider the listing names for that segment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub provider_pin: Option<String>,
    /// Whether a gg run's candidate list is filtered to the native quantization. Absent is true.
    /// False accepts every endpoint at whatever level it declares, `unknown` included, for a
    /// model every provider serves at one precision nobody discloses; the other filters still
    /// apply.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub quantization_filter: Option<bool>,
    /// The native quantization every provider of a gg run's candidate list must serve the model
    /// at (`fp8`, `bf16`, …) while the filter is on. Absent or blank takes the highest level any
    /// endpoint declares; any other value must be a level OpenRouter declares.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub native_quantization: Option<String>,
    /// The input half of the price ceiling, USD per million tokens, used when OpenRouter lists no
    /// developer endpoint. Set together with [`max_output_price`](Self::max_output_price) or not
    /// at all, and positive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub max_input_price: Option<f64>,
    /// The output half of the price ceiling, USD per million tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub max_output_price: Option<f64>,
    /// The providers a gg run of the model never uses. Names are trimmed and blanks dropped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub banned_providers: Option<Vec<String>>,
    /// The providers accepted despite declaring `unknown` quantization. Names are trimmed and
    /// blanks dropped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub unknown_quantization_providers: Option<Vec<String>>,
    /// The list price per **Mtok** of input, in USD — the unit every pricing
    /// page publishes; the store carries per token. The list-price write is
    /// all-or-nothing: all three prices (plus `list_price_as_of`) or none;
    /// absent on update preserves the stored set. A set written here is sourced
    /// `hand` and stands until the next refresh resolves a rate for the model
    /// from OpenRouter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub list_price_input_per_mtok: Option<f64>,
    /// The list price per **Mtok** of cached input, in USD.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub list_price_cached_input_per_mtok: Option<f64>,
    /// The list price per **Mtok** of output, in USD.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub list_price_output_per_mtok: Option<f64>,
    /// The date the operator took the list-price figures (trimmed; empty means
    /// none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub list_price_as_of: Option<String>,
    pub description: Option<String>,
    /// The stored provider-logo SVG (already fetched via `POST /models/logo`).
    pub logo_svg: Option<String>,
    /// The svgl.app URL the logo was fetched from, kept for reference.
    pub provider_logo_url: Option<String>,
}

/// The `GET /models/seed` response: a blank-form seed derived from a run.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ModelSeedOut {
    /// Suggested slug (the canonical id), which the operator may change.
    pub slug: String,
    /// Empty — the operator must set a display name explicitly.
    pub name: String,
    /// Provider guessed from the model id, possibly empty.
    pub provider: String,
    /// Suggested aliases (the canonical and raw forms), deduped, each tagged with
    /// the family of the harness the seed run used.
    pub aliases: Vec<AliasOut>,
    /// The canonical id as an OpenRouter slug, when it looks like one.
    pub openrouter_slug: Option<String>,
}

/// The `GET /models/openrouter` response: the descriptive facts OpenRouter
/// publishes about a model, for the config form to fill itself in with, plus the
/// model's list price as OpenRouter publishes it, scaled to per Mtok — the seed
/// figures for the form's list-price fields. The rate is resolved by the same
/// flow every list price read
/// from OpenRouter is, so the three figures are all present or all null. A model
/// with no rate is not an error: the fields are null and the form leaves them for
/// the operator. The context window and the modalities remain deliberately absent:
/// the backend records those itself from the same catalog (on save, on launch,
/// and on the 24-hour refresh), so they are never form state to begin with.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ModelListingOut {
    /// The display name, with OpenRouter's `Provider: ` prefix stripped.
    pub name: String,
    /// The provider's presentational name (`Anthropic`), from that same prefix.
    pub provider: String,
    /// OpenRouter's prose description, or null when it publishes none.
    pub description: Option<String>,
    /// The list price of input per Mtok in USD, or null.
    pub input_per_mtok: Option<f64>,
    /// The list price of cached input per Mtok in USD, or null.
    pub cached_input_per_mtok: Option<f64>,
    /// The list price of output per Mtok in USD, or null.
    pub output_per_mtok: Option<f64>,
}

/// The `POST /models/list-prices/refresh` response: what the refresh of every
/// catalog entry's list price did, counted in entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ListPriceRefreshOut {
    /// Every curated catalog entry: the sum of the other three.
    #[cfg_attr(feature = "contract", ts(type = "number"))]
    pub total: u64,
    /// The entries whose rate changed, or that had no list price and now have one.
    #[cfg_attr(feature = "contract", ts(type = "number"))]
    pub updated: u64,
    /// The entries whose stored rate OpenRouter confirmed. Their date was rewritten.
    #[cfg_attr(feature = "contract", ts(type = "number"))]
    pub unchanged: u64,
    /// The entries OpenRouter gave no rate for, which keep what they hold.
    #[cfg_attr(feature = "contract", ts(type = "number"))]
    pub unresolved: u64,
}

/// The `POST /models/logo` request/response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct LogoFetchInput {
    pub url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct LogoFetchOut {
    pub logo_svg: String,
}
