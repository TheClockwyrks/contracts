//! The gg run mode's enqueue request, the saved gg configurations and agents, the TCQ batch envelope and the saved views over the corpus.

use serde::{Deserialize, Serialize};

use crate::gg::{GgAgentConfig, GgCapabilitySet};
use crate::gg_query::{GgQuery, GgQueryResponse};

/// The body of `POST /gg/runs`: a **gg-native** run request. Only the test case,
/// version, and variant carry over from a conventional run (those are
/// test-case-level); everything a third-party run expresses as `(model,
/// orchestrator)` is instead expressed by the [capability set](GgCapabilitySet),
/// which binds the models (to [slots](crate::gg::GgSlotBinding)) and
/// selects the capabilities. There is deliberately no `harness` field — this
/// endpoint runs the gg harness by construction — and no `orchestrator` field: gg
/// is its own executor and the orchestrator dimension does not apply.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct GgRunRequest {
    /// Test-case slug to run (e.g. `carom`).
    pub test_case: String,
    /// Exact, immutable test-case version (e.g. `v1.0.0`).
    pub version: String,
    /// Variant to run. Defaults to `base` when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub variant: Option<String>,
    /// The declarative capability set configuring the run: its agent profiles (each
    /// with its own capabilities, model binding, and delegation graph) and the
    /// run-level model slots and limits. Must bind a model to its
    /// [root agent](GgCapabilitySet::root).
    pub capability_set: GgCapabilitySet,
    /// Optional override for the maximum harness runtime, in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub max_runtime_seconds: Option<u64>,
    /// How many times to automatically retry this run after a terminal failure The
    /// Test Cabinet (or a catastrophic build) is responsible for. Defaults to `1`
    /// (one retry) when omitted; `0` disables retries. The backend clamps it to a
    /// sane maximum. Same semantics as a conventional run's `retryCount`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub retry_count: Option<u32>,
    /// Built-in [engine](crate::engine) slug the produced build is
    /// written against. Omit for the `none` default, which supplies no runtime.
    ///
    /// A gg run seeds and builds a workspace like any other run, so it carries the
    /// engine dimension on the same terms: the slug must be one the engine
    /// catalogue knows and one the requested case version declares support for,
    /// both checked when the run executes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub engine: Option<String>,
}

/// The bare id of the saved gg configuration a textual reference names, or `None` when it
/// names none.
///
/// One rule for the two places a configuration is named by text: the
/// [member](super::coverage::ReviewPlanCombo::gg_config_ref) a plan or a ladder stores, and the
/// [`preset_id`](GgCapabilitySet::preset_id) a launch records on the set it runs. The
/// console's picker values a configuration as `saved:<id>` and a client may just as
/// reasonably send the id alone; both name the same configuration, so both must key
/// identically — a member written one way and a run launched the other way are one
/// [coverage cell](https://docs.testcabinet.ai/components/backend/coverage/) or the plan
/// never fills.
///
/// A blank reference names nothing. Recorded as an id it would be indistinguishable from the
/// empty segment a set assembled by hand carries, quietly filing the run into the cell those
/// share.
pub fn saved_config_id(reference: &str) -> Option<&str> {
    let id = reference.trim();
    let id = id.strip_prefix("saved:").unwrap_or(id);
    (!id.is_empty()).then_some(id)
}

/// An operator's saved, reusable agent profile: one agent authored on its own, ready to
/// be imported into any number of gg configurations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct GgSavedAgent {
    /// The saved agent's opaque id (minted on create). This is what an importing
    /// configuration points at, so renaming the agent never breaks an import.
    pub id: String,
    /// The profile's name — the agent's own [`name`](GgAgentConfig::name), lifted out so
    /// the library can be listed and ordered by it. Display text: two saved agents may
    /// carry one name, and the [id](Self::id) is what tells them apart. There is
    /// deliberately no second name: the thing an operator names in the editor is the
    /// agent.
    pub name: String,
    /// A one-line note on what the agent is for. Empty when unset. The library's own
    /// note — not the caller-scoped description a roster entry carries, which says when
    /// *one particular* agent should put this one to work.
    pub description: String,
    /// The profile itself, in exactly the form a configuration's agent list holds,
    /// including the [model slots](GgAgentConfig::model_slots) its bindings defer to.
    pub agent: GgAgentConfig,
    /// RFC 3339 of when the agent was last saved.
    pub updated_at: String,
}

/// The create/update body for a saved agent (the server assigns `id` and `updatedAt`,
/// and lifts `name` off the agent itself).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct GgSavedAgentInput {
    /// A one-line note on what the agent is for.
    #[serde(default)]
    pub description: String,
    /// The profile to save. Its `name` becomes the library entry's name.
    pub agent: GgAgentConfig,
}

/// An operator's saved, reusable gg configuration: a named
/// [capability set](GgCapabilitySet) the new-run form can launch as-is.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct GgConfig {
    /// The configuration's opaque id (minted on create).
    pub id: String,
    /// The operator-chosen display name.
    pub name: String,
    /// A one-line note on what the configuration is for. Empty when unset.
    pub description: String,
    /// The capability set a run launched from this configuration carries. Every agent
    /// is written out in full, whether it was declared inline or imported from a
    /// [saved agent](GgSavedAgent): gg is handed a configuration whose agents are
    /// already whole and resolves no reference of its own.
    pub capability_set: GgCapabilitySet,
    /// Where each imported agent came from. A profile declared inline has no entry, so
    /// a configuration that imports nothing carries an empty list.
    pub agent_sources: Vec<GgAgentSource>,
    /// RFC 3339 of when the configuration was last saved.
    pub updated_at: String,
}

/// The provenance of one agent in a configuration: the [saved
/// agent](GgSavedAgent) it was imported from, and the fields this
/// configuration pins itself.
///
/// This is what makes an import a live reference rather than a copy. A field named in
/// [`overrides`](Self::overrides) is taken from the configuration's own resolved agent;
/// every other field is taken from the saved agent as it stands, so editing the saved
/// agent reshapes each configuration that imported it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct GgAgentSource {
    /// The [id](crate::gg::GgAgentConfig::id) the imported profile carries in
    /// this configuration's capability set — the reference that ties this source to its
    /// profile. An id is never rewritten, so an operator renaming either side leaves the
    /// import following what it always followed.
    pub profile_id: String,
    /// The [id](GgSavedAgent::id) of the saved agent it follows. An id
    /// no longer on the account leaves the profile as the ordinary inline agent the
    /// capability set already holds.
    pub agent_id: String,
    /// The agent fields this configuration overrides, each a path into the profile:
    /// `model`, `tools`, `operations`, `customInstructions`, `systemPromptTemplate`,
    /// `promptCacheTtl`, `loopDetection`, `subagents`, `hooks`, or
    /// `capabilities.<capability id>` for one capability.
    pub overrides: Vec<String>,
}

/// The create/update body for a gg configuration (the server assigns `id` and
/// `updatedAt`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct GgConfigInput {
    /// The operator-chosen display name.
    pub name: String,
    /// A one-line note on what the configuration is for.
    #[serde(default)]
    pub description: String,
    /// The capability set to save, with every agent resolved.
    pub capability_set: GgCapabilitySet,
    /// Where each imported agent came from.
    #[serde(default)]
    pub agent_sources: Vec<GgAgentSource>,
}

/// The body of `POST /gg/query/batch`: several queries answered from **one** read of
/// the document index.
///
/// A dashboard's panels almost always share a filter and differ only in their
/// aggregation, so answering them one request at a time re-resolves the same corpus
/// N times and — because the index refreshes on a timer — can even answer two panels
/// of the same board from two different corpora, which reads as a data bug. One
/// batch, one snapshot, one consistent board.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct GgQueryBatch {
    /// The queries to evaluate, at most `GG_QUERY_MAX_BATCH` of them.
    #[serde(default)]
    pub queries: Vec<GgQuery>,
}

/// The response to `POST /gg/query/batch`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct GgQueryBatchResponse {
    /// One result per requested query, **in request order** — the only binding
    /// between a panel and its answer, so the list is never filtered or reordered.
    #[serde(default)]
    pub results: Vec<GgQueryResponse>,
}

/// An operator's saved TCQ query: a question worth keeping, with the range it was
/// asked over.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct GgSavedQuery {
    /// The query's opaque id (minted on create).
    pub id: String,
    /// The operator-chosen display name.
    pub name: String,
    /// A one-line note on what the query answers. Empty when unset.
    pub description: String,
    /// The query as TCQ **source text**, exactly as it was typed — never its
    /// compiled form, so `now-30d` re-resolves on every run.
    pub query: String,
    /// The time range the query was saved with, as the picker's token (`24h`, `7d`,
    /// `30d`, `90d`, `1y`, `all`). Beside the text rather than inside it, because the
    /// range is a control rather than an edit.
    pub range_id: String,
    /// RFC 3339 of when the query was last saved.
    pub updated_at: String,
}

/// The create/update body for a saved query (the server assigns `id` and
/// `updatedAt`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct GgSavedQueryInput {
    /// The operator-chosen display name.
    pub name: String,
    /// A one-line note on what the query answers.
    #[serde(default)]
    pub description: String,
    /// The query as TCQ source text.
    pub query: String,
    /// The time range token to save with it. Defaults to the whole corpus.
    #[serde(default = "default_range")]
    pub range_id: String,
}

/// One panel of a [dashboard](GgDashboard): a heading, a query, and how wide it sits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct GgDashboardPanel {
    /// The panel's heading.
    pub title: String,
    /// The panel's query, as TCQ **source text**. A copy, even when the panel was
    /// built from a saved query — see the module docs.
    pub query: String,
    /// How many of the twelve grid columns the panel spans.
    /// A width outside `1..=12` is rejected on save, not corrected into range.
    pub width: u32,
}

/// An operator's saved dashboard: panels of TCQ over one board-level range.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct GgDashboard {
    /// The dashboard's opaque id (minted on create).
    pub id: String,
    /// The operator-chosen display name.
    pub name: String,
    /// A one-line note on what the board is for. Empty when unset.
    pub description: String,
    /// The board's panels, in render order, at most `MAX_DASHBOARD_PANELS`.
    pub panels: Vec<GgDashboardPanel>,
    /// The board-level time range, as the picker's token. **One per board** — a panel
    /// never gets its own.
    pub range_id: String,
    /// RFC 3339 of when the dashboard was last saved.
    pub updated_at: String,
}

/// The create/update body for a dashboard (the server assigns `id` and `updatedAt`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct GgDashboardInput {
    /// The operator-chosen display name.
    pub name: String,
    /// A one-line note on what the board is for.
    #[serde(default)]
    pub description: String,
    /// The board's panels, in render order.
    #[serde(default)]
    pub panels: Vec<GgDashboardPanel>,
    /// The board-level time range token. Defaults to the whole corpus.
    #[serde(default = "default_range")]
    pub range_id: String,
}

/// The range token a view carries when none was sent: the whole corpus.
///
/// The same default Discover opens on, and for the same reason — gg runs are
/// experiment material recorded in bursts, so a view that silently defaulted to a
/// recent window would greet most operators with an empty result and no clue that the
/// corpus is fine.
fn default_range() -> String {
    "all".to_string()
}
