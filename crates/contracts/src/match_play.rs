//! The match-play shapes: the controllers a match is played between, a match's
//! summary, a tournament's standings and its persisted record, the opponents a
//! case's baselines and hidden references offer, and the names a match and its
//! replay are stored under.
//!
//! The runner that plays a match over the foray and lattice hosts is the
//! engines repository's; the arena, the backend and the console read and serve
//! what it records in these shapes, and the `@clockwyrks/backend-api` package's
//! tournament types are generated from them.

use serde::{Deserialize, Serialize};

use crate::validation::AdversarialOutcome;

/// The run-root-relative filename a match's replay is written to (the validator's
/// run asset, and the per-match artifact name a tournament stores).
pub const REPLAY_JSON: &str = "replay.json";

/// The ids of the **model-facing** baseline controllers a case commits under
/// `references/<id>.wasm`. These three are documented to models and baked into the
/// run container; `border-soldier` is also the canonical scoring opponent the
/// validator uses. The arena offers a wider set (see [`ARENA_OPPONENT_IDS`]).
pub const BASELINE_IDS: &[&str] = &["border-soldier", "greedy-raider", "random"];

/// The ids of the **hidden** reference opponents a case commits under
/// `references/<id>.wasm` but deliberately keeps out of the model's view (not
/// documented, not seeded into the run container). `fuel-probe` is a non-trivial
/// adversary that defeats every baseline; it is used to auto-replay a run against
/// a strong opponent and is selectable in the arena, but never shown to a model.
pub const HIDDEN_OPPONENT_IDS: &[&str] = &["fuel-probe"];

/// Every opponent the arena and the validator may load: the model-facing
/// baselines plus the hidden references. This is the allowlist a host resolves a
/// "baseline" controller against — it is intentionally wider than [`BASELINE_IDS`]
/// (which stays the model-facing set). Kept in sync with `BASELINE_IDS` ++
/// `HIDDEN_OPPONENT_IDS` (asserted in tests).
pub const ARENA_OPPONENT_IDS: &[&str] =
    &["border-soldier", "greedy-raider", "random", "fuel-probe"];

/// The opponents a finished adversarial run is auto-replayed against, in order,
/// paired with whether that match's outcome is **scored** (recorded in the run
/// record's `replays`). The first entry is the canonical opponent: its replay is
/// written to [`REPLAY_JSON`] and mirrored to the run result's top-level fields.
/// `random` is an unscored exhibition — its replay is kept so a reviewer can watch
/// it, but its outcome is excluded from the recorded set.
pub const AUTO_REPLAY_OPPONENTS: &[(&str, bool)] = &[
    ("border-soldier", true),
    ("greedy-raider", true),
    ("fuel-probe", true),
    ("random", false),
];

/// The run-root-relative filename of the replay at `index` in an adversarial run's
/// `replays` list. Index 0 is the canonical [`REPLAY_JSON`] (`replay.json`); the
/// rest are `replay-<index>.json`, a single path segment so they route through the
/// one-segment `/asset/{file}` endpoint unchanged.
pub fn replay_filename(index: usize) -> String {
    if index == 0 {
        REPLAY_JSON.to_string()
    } else {
        format!("replay-{index}.json")
    }
}

/// Where a controller came from — surfaced so the arena UI can group baselines
/// apart from controllers built by prior runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub enum ControllerKind {
    /// A baseline committed with the case (`references/<id>.wasm`).
    Baseline,
    /// A controller a prior run produced, resolved from *this* host's run output
    /// dir. Local to the host that ran it — selectable only while browsing through
    /// that worker (the dropdown picks which worker contributes these).
    Run,
    /// A controller a **pushed** run produced, resolved from the backend's stored
    /// `controller.wasm`. Always selectable from any host (the backend is the
    /// shared source), so a reviewer can watch a pushed implementation play
    /// without having produced it locally.
    #[serde(rename = "pushed")]
    PushedRun,
}

/// A controller a match can be played with, identified but not yet loaded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ControllerRef {
    /// The stable id: a baseline name, or a prior run's id.
    pub id: String,
    /// Where it came from.
    pub kind: ControllerKind,
    /// An optional human-facing label (e.g. the model id of the run that built it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub label: Option<String>,
}

/// One match's result, summarized so a tournament list can show the outcome
/// without loading (and replaying) the match. Serializes `camelCase` to mirror the
/// `@clockwyrks/backend-api` TypeScript contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct MatchSummary {
    /// Stable id for this match within its tournament (also the replay's storage
    /// segment): `"{redId}__vs__{blueId}"`.
    pub match_id: String,
    /// The controller that played Red (the lower-sorted id in a tournament pair).
    pub red_id: String,
    /// The controller that played Blue.
    pub blue_id: String,
    /// The winning controller's id, or `None` for a draw. On a level-score match
    /// this is the more fuel-efficient controller (the tie-break), so a draw means
    /// the scores *and* the fuel totals were level.
    pub winner: Option<String>,
    /// How the match was decided: `swept`, `time_limit` (a decisive score at the
    /// cap), `efficiency` (a level score broken by lower total fuel), or `forfeit`.
    pub win_type: String,
    /// The outcome from Red's perspective.
    pub outcome_for_red: AdversarialOutcome,
    /// Red's banked score (the points Red earned this match).
    pub red_score: u32,
    /// Blue's banked score (the points Blue earned this match).
    pub blue_score: u32,
    /// How many ticks the match ran for.
    pub ticks: u32,
    /// Enemy raiders Red tagged ("kills").
    pub red_kills: u32,
    /// Enemy raiders Blue tagged.
    pub blue_kills: u32,
    /// Red's total fuel consumed over the match — the efficiency figure that breaks
    /// a level-score draw. `0` when no match ran (a load forfeit).
    pub red_fuel: u64,
    /// Blue's total fuel consumed over the match.
    pub blue_fuel: u64,
    /// The storage segment the replay is kept under, or `None` when no match ran
    /// (a controller failed to load, so there is nothing to play back).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub replay_key: Option<String>,
    /// Why a controller lost on a technicality, or `None` for a clean result. This
    /// is set both when a controller failed to *load* (no match ran) and when one
    /// *forfeited mid-match* (fuel/memory/trap/contract-invalid action) — in the
    /// latter case the replay records only `Ended::Forfeit`, so this carries the
    /// reason a player would otherwise have no way to read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub detail: Option<String>,
}

/// One row of a tournament's standings: a controller's win/loss/draw record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct Standing {
    /// The controller this row ranks.
    pub participant_id: String,
    /// Matches won — the standings are ranked by this, highest first. A win is a
    /// decided match (higher score, a sweep, the leaner side of a level-score draw,
    /// or an opponent forfeit), so the efficiency tie-break feeds straight into this
    /// count.
    pub wins: u32,
    /// Matches lost.
    pub losses: u32,
    /// Matches drawn (level score *and* level fuel, or a double forfeit).
    pub draws: u32,
    /// 1-based rank (1 is best), assigned after sorting.
    pub rank: u32,
}

/// A persisted tournament: the field, the ranked standings, and every match's
/// summary. The per-match replays are stored alongside (keyed by
/// [`MatchSummary::replay_key`]); this record carries only the summaries so it
/// loads cheaply. Serializes `camelCase` to mirror the TypeScript contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct TournamentRecord {
    /// Unique tournament id (caller-assigned).
    pub id: String,
    /// RFC 3339 timestamp the tournament was run at.
    pub created_at: String,
    /// The test case the field competed under.
    pub test_case_slug: String,
    /// The exact case version.
    pub test_case_version: String,
    /// The case variant the canonical match used.
    pub variant: String,
    /// The competing controllers (identity only, no bytes).
    pub participants: Vec<ControllerRef>,
    /// The standings, ranked by wins, highest first.
    pub standings: Vec<Standing>,
    /// Every match's summary, in the order they were played.
    pub matches: Vec<MatchSummary>,
}

/// Where a recorded tournament is in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub enum TournamentState {
    /// Recorded, with no arena playing it yet.
    Queued,
    /// An arena is playing its pairings.
    Running,
    /// Every pairing has a recorded result, and the standings are recorded.
    Succeeded,
    /// It cannot complete; the recorded tournament's `detail` carries the reason.
    Failed,
}

impl TournamentState {
    /// The wire spelling, which is also the backend's stored value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }

    /// Parse the wire spelling.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "queued" => Some(Self::Queued),
            "running" => Some(Self::Running),
            "succeeded" => Some(Self::Succeeded),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }

    /// Whether the tournament has finished, successfully or not.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed)
    }
}

/// One pairing of a tournament's field: a match recorded before it is played.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct TournamentPairing {
    /// The match id, [`match_id`] of the two seats.
    pub match_id: String,
    /// The controller that plays Red.
    pub red_id: String,
    /// The controller that plays Blue.
    pub blue_id: String,
}

/// A tournament as the arena records it on acceptance: the field and every
/// pairing, with no result. (`PUT /tournaments/{id}`)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct PendingTournament {
    /// Unique tournament id (caller-assigned).
    pub id: String,
    /// RFC 3339 timestamp the tournament was accepted at.
    pub created_at: String,
    /// The test case the field competes under.
    pub test_case_slug: String,
    /// The exact case version.
    pub test_case_version: String,
    /// The case variant the canonical match uses.
    pub variant: String,
    /// The competing controllers (identity only, no bytes).
    pub participants: Vec<ControllerRef>,
    /// Every match of the field, in the order it is played.
    pub pairings: Vec<TournamentPairing>,
}

/// A tournament as the backend holds it, in any state. (`GET /tournaments/{id}`)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct RecordedTournament {
    /// The record. Its `matches` holds the summary of each pairing with a
    /// recorded result, in pairing order, and its `standings` are empty until the
    /// tournament succeeds.
    pub record: TournamentRecord,
    /// Where the tournament is in its lifecycle.
    pub state: TournamentState,
    /// The failure reason, present when `state` is `failed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub detail: Option<String>,
    /// Every match of the field, in the order it is played.
    pub pairings: Vec<TournamentPairing>,
    /// RFC 3339 of when the tournament was recorded, and once it succeeds, of when
    /// it succeeded. The gallery's newest-first order.
    pub published_at: String,
}

impl RecordedTournament {
    /// The pairings without a recorded result, in pairing order.
    pub fn unplayed_pairings(&self) -> Vec<TournamentPairing> {
        self.pairings
            .iter()
            .filter(|pairing| {
                !self
                    .record
                    .matches
                    .iter()
                    .any(|summary| summary.match_id == pairing.match_id)
            })
            .cloned()
            .collect()
    }
}

/// A move of a recorded tournament's state. (`POST /tournaments/{id}/state`)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub enum TournamentTransition {
    /// An arena has started playing the tournament.
    Running,
    /// Every pairing has a result; these are the standings they come to.
    Succeeded {
        /// The standings, ranked by wins, highest first.
        standings: Vec<Standing>,
    },
    /// The tournament cannot complete.
    Failed {
        /// Why.
        detail: String,
    },
}

impl TournamentTransition {
    /// The state this transition moves to.
    pub fn state(&self) -> TournamentState {
        match self {
            Self::Running => TournamentState::Running,
            Self::Succeeded { .. } => TournamentState::Succeeded,
            Self::Failed { .. } => TournamentState::Failed,
        }
    }
}

/// The stable match id (and replay storage segment) for a pair.
pub fn match_id(red_id: &str, blue_id: &str) -> String {
    format!("{red_id}__vs__{blue_id}")
}

#[cfg(test)]
#[path = "match_play.test.rs"]
mod tests;
