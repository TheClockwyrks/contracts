//! The statistics reads (`/stats/*`): per-provider call and probe figures, model accuracy and the cabinet's token and cost totals.

use serde::Serialize;

use std::collections::BTreeMap;

/// The `GET /stats/providers` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ProviderStatsResponse {
    /// Every stored gg run with a recorded session summary, whether or not it
    /// recorded providers. A launch that died before a single turn records no
    /// summary and is not scanned.
    pub runs_scanned: u64,
    /// The runs among them whose summary carries provider slices — the
    /// denominator that makes sparse provider coverage read as sparse rather
    /// than as zero.
    pub runs_with_provider_data: u64,
    /// Run evidence: one entry per observed provider, providerless last.
    pub providers: Vec<ProviderStatsOut>,
    /// Probe evidence, strictly separate from the run evidence: one entry per
    /// provider observed on model-probe items.
    pub probes: Vec<ProbeProviderStatsOut>,
}

/// One provider's run evidence: its per-model rows and their total.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ProviderStatsOut {
    /// The provider's OpenRouter name, or null for the slice of calls that
    /// named none — a gateway that stamps no provider, or a turn whose call
    /// produced no reply to name one.
    pub provider: Option<String>,
    /// The per-model rows, largest first by calls, a modelless row last.
    pub models: Vec<ProviderModelStatsOut>,
    /// The rows summed (`runs` counts distinct runs, not a sum of rows).
    pub totals: ProviderCallStatsOut,
}

/// One provider's evidence for one model.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ProviderModelStatsOut {
    /// The model id, or null for a slice recorded before the agent's first
    /// usage delta named one, on a run more than one model served.
    pub model_id: Option<String>,
    /// The row's figures.
    pub stats: ProviderCallStatsOut,
}

/// The call/turn figures one provider row carries.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ProviderCallStatsOut {
    /// Distinct runs contributing to this row.
    pub runs: u64,
    /// Model calls that reported usage.
    pub calls: u64,
    /// Every token class those calls reported, summed.
    pub total_tokens: u64,
    /// Their comparable USD cost, summed — null only when no contributing
    /// slice reported one, so unreported stays unreported.
    pub cost: Option<f64>,
    /// Length-capped replies the provider served.
    pub rejected: u64,
    /// Turns attributed to the provider.
    pub turns: u64,
    /// The turns among them that worked (progressed or finished).
    pub working: u64,
    /// The errored turns, keyed by turn error type wire id.
    pub errors: BTreeMap<String, u64>,
    /// Streams that stalled on this provider.
    pub stalls: u64,
    /// Unexpected cache misses this provider's replies produced.
    pub cache_misses: u64,
}

/// One provider's probe evidence.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ProbeProviderStatsOut {
    /// The provider OpenRouter reported serving the items, or null for calls
    /// that errored before any provider served them.
    pub provider: Option<String>,
    /// The per-model rows, largest first by items.
    pub models: Vec<ProbeProviderModelOut>,
}

/// One provider's probe evidence for one probed model.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ProbeProviderModelOut {
    /// The catalog slug the probe was triggered from.
    pub model_slug: String,
    /// Completion calls on this (provider, model).
    pub items: u64,
    /// The calls whose submitted program passed its case's check.
    pub passes: u64,
    /// The calls that errored before classification.
    pub errored: u64,
}

/// The `GET /stats/model-accuracy` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ModelAccuracyResponse {
    /// One entry per model with any evidence, largest evidence first.
    pub models: Vec<ModelAccuracyOut>,
    /// Runs that could not be attributed to any single model: an older
    /// multi-model run with no per-model slices, in either execution mode.
    pub unattributable_runs: u64,
}

/// One model's accuracy figures, split by execution mode. Either half is null
/// when no run of that mode contributed.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ModelAccuracyOut {
    /// The model id, as the runs recorded it.
    pub model_id: String,
    /// The responses-as-code figures.
    pub rac: Option<RacAccuracyOut>,
    /// The tool-calling figures.
    pub tool_calling: Option<ToolCallingAccuracyOut>,
}

/// A model's responses-as-code turn accounting.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct RacAccuracyOut {
    /// Distinct contributing runs.
    pub runs: u64,
    /// Turns attributed to the model, whatever their outcome.
    pub turns: u64,
    /// The turns that worked. Exact where per-model slices exist; on an older
    /// record it is turns minus errors, which counts a fatal turn as valid —
    /// an overcount of at most one turn per agent, tallied under
    /// [`approximate_runs`](Self::approximate_runs).
    pub valid: u64,
    /// Turns the compiler rejected (the transpile kind).
    pub compile: u64,
    /// Turns the program failed at runtime (program faults plus sandbox
    /// limits).
    pub runtime: u64,
    /// Turns lost to the model API (the model-api kind).
    pub model_errors: u64,
    /// Turns that produced no completion at all.
    pub missing: u64,
    /// The same errors keyed by turn error type wire id — the open breakdown
    /// the named groups above are derived from.
    pub by_type: BTreeMap<String, u64>,
    /// The contributing runs whose figures came from the run-level rollup
    /// rather than per-model slices.
    pub approximate_runs: u64,
}

/// A model's tool-calling dispatch accounting.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ToolCallingAccuracyOut {
    /// Distinct runs contributing dispatch totals.
    pub runs: u64,
    /// Dispatched tool calls across them.
    pub calls: u64,
    /// The dispatches that succeeded (calls minus the failures below).
    pub ok: u64,
    /// The failed dispatches, keyed by call failure class wire id.
    pub failures: BTreeMap<String, u64>,
    /// Tool-calling runs with failure evidence but no recorded dispatch total
    /// — records that predate the total, whose rate cannot be stated.
    pub runs_without_call_totals: u64,
}

/// The `GET /stats/cabinet` response: the cabinet's headline totals plus the
/// weekly activity series the home page charts.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CabinetStatsResponse {
    /// Every recorded run, whatever its state or publication.
    pub runs: u64,
    /// The summed token totals, with the honesty counter beside the sum.
    pub tokens: CabinetTokensOut,
    /// The summed comparable USD cost, with the honesty counter beside the sum.
    pub cost: CabinetCostOut,
    /// Distinct test-case slugs across the corpus.
    pub test_cases: u64,
    /// Distinct model ids across the corpus.
    pub models: u64,
    /// Runs per ISO week (UTC Mondays), the last 52 weeks up to
    /// now inclusive, ascending, with explicit zero entries for empty weeks so a
    /// consumer charts the series without filling gaps.
    pub weekly: Vec<CabinetWeekOut>,
}

/// The cabinet's token total and its unreported-run counter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CabinetTokensOut {
    /// Total tokens across the runs that reported any.
    pub total: u64,
    /// Runs whose metrics reported no tokens; they contribute nothing to the
    /// total.
    pub unreported_runs: u64,
}

/// The cabinet's comparable-cost total and its unreported-run counter.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CabinetCostOut {
    /// Summed comparable cost (USD) across the runs whose cost is known.
    pub total: f64,
    /// Runs whose comparable cost is unknown (a `NULL` lifted column); they
    /// contribute nothing to the total.
    pub unreported_runs: u64,
}

/// One week of the cabinet's activity series.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CabinetWeekOut {
    /// The week's UTC Monday, as `YYYY-MM-DD`.
    pub week_start: String,
    /// Runs whose `started_at` falls in that ISO week.
    pub runs: u64,
}
