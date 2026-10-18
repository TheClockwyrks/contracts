//! Ladders (`/ladders`): an ordered series of rungs that harness and model combinations climb one at a time, gated at each rung by the validators' ratings of its runs.

use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;

use super::coverage::{Gate, InFlightLimit, ReviewPlanCombo, default_retry_count};
use crate::run_record::HarnessSlug;

// ---- Wire types ------------------------------------------------------------

/// Which axis a ladder's emission loop nests on — and therefore the order its runs
/// execute in, by the same mechanism a plan's axis works: emission order is queue
/// order is execution order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub enum LadderAxis {
    /// Bring every climber up one rung before anyone moves on — the board advances
    /// as a row. The default: it is what makes a ladder comparable across models.
    #[default]
    Rung,
    /// Take one climber as far up as it gets before starting the next — the board
    /// advances as a column. Answers "how far does *this* model get?" soonest.
    Combination,
}

impl LadderAxis {
    /// The stored/wire token for the axis.
    pub fn as_str(self) -> &'static str {
        match self {
            LadderAxis::Rung => "rung",
            LadderAxis::Combination => "combination",
        }
    }

    /// Parse a stored axis token, falling back to the default for anything else. The
    /// axis decides emission *order* only, so a row written by a newer build degrades
    /// to today's ordering rather than making the ladder unreadable.
    pub fn parse(token: &str) -> Self {
        match token {
            "combination" => LadderAxis::Combination,
            _ => LadderAxis::Rung,
        }
    }
}

/// One rung: exactly one [pinned case](super::coverage::ReviewPlanCase) — a slug, an exact version, a
/// variant, and the engine its runs are built on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct LadderRung {
    /// The rung's **stable opaque id**, minted when the rung is added and never
    /// reused.
    ///
    /// Emphatically not its position. Rungs get reordered and re-pinned, and every
    /// recorded verdict references this id — a positional identifier would silently
    /// reattribute a climber's verdicts to a different case the moment the ladder was
    /// rearranged.
    pub id: String,
    /// The test-case slug.
    pub slug: String,
    /// The pinned, exact version.
    pub version: String,
    /// The variant to climb.
    pub variant: String,
    /// The engine to climb on, or null for the `none` engine.
    ///
    /// Part of the rung's identity within the climb, because clearing a case with a
    /// runtime underneath is a different achievement from clearing it with nothing: one
    /// ladder holds the same case at the same version and variant twice when the two
    /// pins name different engines.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub engine: Option<String>,
    /// This rung's override of the ladder's runs-per-cell target, or null to inherit
    /// it — so one pivotal step can demand more evidence without making the whole
    /// climb more expensive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub runs: Option<u32>,
}

/// One rung in a create/update body. The id is optional: absent means "a new rung",
/// present means "this existing rung, wherever it now sits", which is what lets a
/// reorder or a version bump keep every climber's recorded progress.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct LadderRungInput {
    /// The existing rung's stable id, or null to mint one.
    #[serde(default)]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub id: Option<String>,
    /// The test-case slug.
    pub slug: String,
    /// The pinned, exact version.
    pub version: String,
    /// The variant to climb.
    pub variant: String,
    /// The engine to climb on, or null for the `none` engine.
    #[serde(default)]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub engine: Option<String>,
    /// This rung's override of the ladder's runs-per-cell target, or null to inherit.
    #[serde(default)]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub runs: Option<u32>,
}

/// A ladder's **configuration**: the climb, the climbers, the rule every rung is judged
/// by, and how a dispatch of it is fed. What a dispatch did is [`LadderProgress`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct Ladder {
    /// The ladder's opaque id (minted on create).
    pub id: String,
    /// The reviewer-chosen display name.
    pub name: String,
    /// The default target number of runs for each rung slot; a rung may raise it for
    /// itself via [`LadderRung::runs`].
    pub runs_per_cell: u32,
    /// The single parameterised rule every rung is judged by. Per ladder, not per
    /// rung: a ladder asks *one* question of an ordered series of cases, and only how
    /// many runs it takes to answer varies by rung.
    pub gate: Gate,
    /// The referenced combination groups' ids — the same `kind = "combo"` coverage
    /// groups a plan uses, so one saved set of models drives both and editing it
    /// reshapes both.
    pub combo_group_ids: Vec<String>,
    /// One-off combinations pinned directly on the ladder, unioned with the groups.
    pub combos: Vec<ReviewPlanCombo>,
    /// The rungs, low to high. The order **is** the climb.
    pub rungs: Vec<LadderRung>,
    /// Which axis a dispatch's launch order nests on.
    pub outer_axis: LadderAxis,
    /// This ladder's override of the account's runs-in-flight limit, or null to inherit
    /// it. A dispatch resolves it when it starts and keeps it.
    #[serde(default)]
    pub in_flight_limit: Option<InFlightLimit>,
    /// How many automatic retries each run a dispatch launches gets, `0..=10`. A launch
    /// that uses them up without a counted run blocks its climber. A dispatch takes it
    /// when it starts and keeps it.
    #[serde(default = "default_retry_count")]
    pub retry_count: u32,
    /// RFC 3339 of when the ladder was last saved.
    pub updated_at: String,
}

/// The create/update body for a ladder (the server assigns `id` and `updatedAt`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct LadderInput {
    /// The reviewer-chosen display name.
    pub name: String,
    /// The default target number of runs for each rung slot.
    pub runs_per_cell: u32,
    /// The rule every rung is decided by, or null for [`Gate::default`] — the gentlest
    /// gate that still stops a hopeless climb (pass as long as one run was playable at
    /// all).
    #[serde(default)]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub gate: Option<Gate>,
    /// The referenced combination groups' ids.
    #[serde(default)]
    pub combo_group_ids: Vec<String>,
    /// One-off combinations pinned directly on the ladder.
    #[serde(default)]
    pub combos: Vec<ReviewPlanCombo>,
    /// The rungs, low to high.
    #[serde(default)]
    pub rungs: Vec<LadderRungInput>,
    /// Which axis a dispatch's launch order nests on.
    #[serde(default)]
    pub outer_axis: LadderAxis,
    /// This ladder's override of the account's runs-in-flight limit, or null to inherit.
    #[serde(default)]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub in_flight_limit: Option<InFlightLimit>,
    /// How many automatic retries each run a dispatch launches gets, or null for the
    /// default of one. Clamped to the most the backend honours for any launch.
    #[serde(default)]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub retry_count: Option<u32>,
}

/// Where a ladder's latest dispatch stands. A ladder never run has no dispatch, which
/// the console reads as "Not run yet".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub enum DispatchStatus {
    /// Launch passes are still climbing it.
    Running,
    /// It is running and waiting on its owner: none of its jobs is in flight, every
    /// climber is completed, failed or blocked, and at least one is blocked. A reading of
    /// a running dispatch, derived whenever its status is reported and never stored: a
    /// climber's Retry or a run arriving for one of its slots makes it `running` again.
    NeedsAttention,
    /// Every climber completed or failed and none of its runs is in flight.
    Finished,
    /// Its owner stopped it. It launches nothing more.
    Stopped,
}

impl DispatchStatus {
    /// The stored token. A dispatch that needs attention is stored as the running
    /// dispatch it is.
    pub fn as_str(self) -> &'static str {
        match self {
            DispatchStatus::Running | DispatchStatus::NeedsAttention => "running",
            DispatchStatus::Finished => "finished",
            DispatchStatus::Stopped => "stopped",
        }
    }

    /// Parse a stored token, which is never [`NeedsAttention`](Self::NeedsAttention).
    /// Anything unknown reads as ended: a dispatch nobody can recognise as running must
    /// not keep launching.
    pub fn parse(token: &str) -> Self {
        match token {
            "running" => DispatchStatus::Running,
            "finished" => DispatchStatus::Finished,
            _ => DispatchStatus::Stopped,
        }
    }
}

/// Where one **rung slot** — one climber on one rung of a dispatch — stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub enum SlotStatus {
    /// The climber's current rung, undecided, with runs launched or waiting for room.
    Running,
    /// The climber's current rung, undecided and blocked.
    Blocked,
    /// The gate passed it.
    Passed,
    /// The gate failed it.
    Failed,
    /// Not reached yet, while the dispatch is running.
    Pending,
    /// Never to run: the climber failed an earlier rung, or the dispatch was stopped.
    Skipped,
}

/// Where one climber of a dispatch stands.
///
/// There is no "waiting on a review" state: the validators rate every completed run, so
/// a rung the dispatch can still feed is `running`, and one nothing it does can move is
/// `blocked`, with the reason in [`LadderClimber::blocked`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub enum ClimberStatus {
    /// The current rung is undecided and its runs can still launch.
    Running,
    /// The current rung is undecided and nothing the dispatch does will move it. Why,
    /// and what fixes it, is [`LadderClimber::blocked`].
    Blocked,
    /// The current rung was failed. The validators are assumed correct, so this is the
    /// climber's result.
    Failed,
    /// Every rung passed. There is nothing left to climb.
    Completed,
}

/// Why a climber is [`blocked`](ClimberStatus::Blocked), each reason naming its fix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub enum ClimberBlock {
    /// The combination cannot be launched at all, and nothing of its rung is in flight.
    /// Fix: fix the cause, then retry the climber, which resolves it again.
    Unlaunchable {
        /// Why, in the words the launch pass reports it with.
        reason: String,
    },
    /// A job the dispatch launched for the rung slot used up its automatic retries
    /// without a counted run, no later launch of the dispatch took its place, and
    /// nothing of the slot is in flight. Fix: fix the
    /// cause, then retry the climber (`POST /ladders/{id}/climbers/retry`), which
    /// relaunches it.
    Failing {
        /// How many attempts that launch made: the first, and each automatic retry.
        attempts: u32,
    },
    /// Every run the rung will get has completed and the rung is still undecided,
    /// because `runs` of them carry no validator rating (pushed while the backend did
    /// not hold the case version). Fix: re-push the runs, or stop the dispatch.
    Unrated {
        /// How many of the rung's counted runs carry no validator rating.
        runs: u32,
    },
}

/// How many of a dispatch's rung slots are in each [`SlotStatus`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct SlotCounts {
    /// Every rung slot: climbers × rungs.
    pub total: u32,
    /// Slots `running`.
    pub running: u32,
    /// Slots `blocked`.
    pub blocked: u32,
    /// Slots `passed`.
    pub passed: u32,
    /// Slots `failed`.
    pub failed: u32,
    /// Slots `pending`.
    pub pending: u32,
    /// Slots `skipped`.
    pub skipped: u32,
}

impl SlotCounts {
    /// Count one slot.
    pub fn add(&mut self, status: SlotStatus) {
        self.total += 1;
        match status {
            SlotStatus::Running => self.running += 1,
            SlotStatus::Blocked => self.blocked += 1,
            SlotStatus::Passed => self.passed += 1,
            SlotStatus::Failed => self.failed += 1,
            SlotStatus::Pending => self.pending += 1,
            SlotStatus::Skipped => self.skipped += 1,
        }
    }
}

/// How much of a dispatch's execution is behind it: the figures its progress bar is
/// drawn from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct DispatchRuns {
    /// The sum over rung slots of the rung's target runs.
    pub total: u32,
    /// The runs that need no more executing, summed per slot: a passed, failed or
    /// skipped slot's whole target less the runs the dispatch still has in flight for
    /// it, a running or blocked slot's runs, and nothing of a pending one. A run that
    /// existed before the dispatch started is done from its first pass. Equal to
    /// `total` once nothing is left to execute.
    pub done: u32,
    /// The jobs the dispatch launched that are still in flight (`queued` through
    /// `running`).
    pub in_flight: u32,
}

/// A ladder's latest dispatch, as its board reports it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct LadderDispatch {
    /// The dispatch's id: the second segment of its jobs' origins.
    pub id: String,
    /// Where it stands.
    pub status: DispatchStatus,
    /// RFC 3339 of the Run that started it.
    pub started_at: String,
    /// RFC 3339 of when it finished or was stopped, absent while running.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub ended_at: Option<String>,
    /// The gate, as it stood at Run.
    pub gate: Gate,
    /// The climb order, as it stood at Run.
    pub outer_axis: LadderAxis,
    /// The runs-in-flight limit resolved at Run.
    pub in_flight_limit: InFlightLimit,
    /// The retry limit, as it stood at Run: how many automatic retries each run the
    /// dispatch launches gets.
    pub retry_count: u32,
    /// The rung-slot counts.
    pub slots: SlotCounts,
    /// The runs: total, done, in flight.
    pub runs: DispatchRuns,
    /// How many climbers are running.
    pub climbers_running: u32,
    /// How many climbers are blocked.
    pub climbers_blocked: u32,
    /// How many climbers failed a rung.
    pub climbers_failed: u32,
    /// How many climbers passed every rung.
    pub climbers_completed: u32,
}

/// The counts one gate answer was made from, so a dashboard can say *why* a slot passed,
/// failed, or is still running without re-deriving the floor and unloaded-run rules.
///
/// The wire mirror of `GateTally`, which is an internal type of the pure core.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct RungTally {
    /// The slot's counted runs: finished runs that are the model's result.
    pub counted: u32,
    /// Counted runs the gate has a rating for — carrying a validator rating, or
    /// decided as broken because the build never loaded or the model failed.
    pub rated: u32,
    /// Counted runs with no validator rating.
    pub unrated: u32,
    /// Rated runs rated at or above the gate's floor.
    pub passing: u32,
    /// Runs the slot will still finish with: `max(target, counted + inFlight) − counted`.
    pub pending: u32,
    /// The slot's jobs still in flight.
    pub in_flight: u32,
    /// How many passing runs the threshold demands, as the whole number of runs it
    /// actually takes — a fractional bar of 2.5 means three.
    pub required: u32,
}

/// One rung slot of one climber.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct LadderSlot {
    /// Which rung, by its stable id.
    pub rung_id: String,
    /// The rung's position in the climb, from zero.
    pub position: u32,
    /// Where the slot stands.
    pub status: SlotStatus,
    /// The gate evidence behind the slot's answer: present on a slot that has been
    /// reached or has runs.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub tally: Option<RungTally>,
    /// RFC 3339 of when the gate decided the slot, on a passed or failed slot. A verdict
    /// a read computed live, which the next launch pass records, carries the read's time.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub decided_at: Option<String>,
    /// The slot's runs, oldest first: the first runs of its cell to finish, up to the
    /// rung's target, whoever launched them. Empty on a slot the climber has not
    /// reached.
    pub run_ids: Vec<String>,
    /// The jobs the dispatch launched for this slot that are still in flight, in queue
    /// order. The tally's `inFlight` also counts a job of the cell someone else
    /// launched, so it can exceed these.
    pub job_ids: Vec<String>,
}

/// One climber of a dispatch and its whole standing.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct LadderClimber {
    /// The combination's canonical key — the text the dispatch records this climber's
    /// state against.
    ///
    /// Its shape follows the shape of the combination: a harness climber's key is its
    /// `harness|model|provider` triple, and a gg climber's names the configuration and the
    /// models it binds, because two climbers running one configuration on different models
    /// are exactly the two arms a ladder exists to separate. It is written and compared,
    /// never parsed.
    pub key: String,
    /// The harness the climber runs — `gg` on a gg climber.
    pub harness: HarnessSlug,
    /// The model the climber's runs are attributed to: the harness climber's own, and for a
    /// gg climber the model its bound configuration's root agent runs.
    pub model: String,
    /// The provider for a provider-routed harness, or null.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub provider: Option<String>,
    /// The gg configuration this climber runs (`saved:<id>`), or null on a harness climber.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub gg_config_id: Option<String>,
    /// That configuration's current display name, or null on a harness climber.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub gg_config_name: Option<String>,
    /// The model bound to each of the configuration's launch slots, keyed by slot name.
    /// Empty on a harness climber.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub gg_slot_models: BTreeMap<String, String>,
    /// Why this climber cannot be launched at all, or null when it can.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub unlaunchable: Option<String>,
    /// Where the climber stands.
    pub status: ClimberStatus,
    /// Why the climber is blocked, naming the fix, or null when it is not.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub blocked: Option<ClimberBlock>,
    /// The position of the rung it stands on (or failed), absent once it completed.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub current_rung: Option<u32>,
    /// One slot per rung of the dispatch, in rung order.
    pub slots: Vec<LadderSlot>,
}

/// One rung as the progress board describes it: the declaration plus how its pin has
/// aged.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct LadderProgressRung {
    /// The rung's declaration.
    #[serde(flatten)]
    pub rung: LadderRung,
    /// Its position in the climb, from zero.
    pub position: u32,
    /// The newest ingested version of this case. Empty when the case is not ingested.
    pub latest_version: String,
    /// Whether the pinned version is not the newest ingested one.
    pub stale: bool,
}

/// The ladder's board: the rungs, its latest dispatch, and every climber of it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct LadderProgress {
    /// The ladder's id.
    pub ladder_id: String,
    /// The latest dispatch, or null for a ladder never run.
    pub dispatch: Option<LadderDispatch>,
    /// The rungs, low to high: the dispatch's when there is one, else the
    /// configuration's.
    pub rungs: Vec<LadderProgressRung>,
    /// Every climber of the dispatch, in its resolved order. Empty without one.
    pub climbers: Vec<LadderClimber>,
    /// The completed runs among the dispatch's slots' runs that the requester has not
    /// reviewed — exactly what `GET /ladders/{id}/queue` offers. Information only.
    pub runs_unreviewed: u32,
}

/// One ladder on the ladders list: its configuration's headline and its latest
/// dispatch's totals.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct LadderSummary {
    /// The ladder's id.
    pub id: String,
    /// Its display name.
    pub name: String,
    /// How many rungs the configuration holds.
    pub rungs: u32,
    /// The configuration's runs per rung slot.
    pub runs_per_cell: u32,
    /// How many climbers the configuration resolves to now.
    pub climbers: u32,
    /// The latest dispatch, or null for a ladder never run.
    pub dispatch: Option<LadderDispatchSummary>,
}

/// A dispatch's totals, for the ladders list.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct LadderDispatchSummary {
    /// The dispatch's id.
    pub id: String,
    /// Where it stands.
    pub status: DispatchStatus,
    /// RFC 3339 of the Run.
    pub started_at: String,
    /// RFC 3339 of when it ended, absent while running.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub ended_at: Option<String>,
    /// The rung-slot counts, for the whole ladder.
    pub slots: SlotCounts,
    /// The runs: total, done, in flight.
    pub runs: DispatchRuns,
}

/// The `POST /ladders/{id}/stop` body.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct LadderStopInput {
    /// Whether to cancel the dispatch's `dispatched`, `starting` and `running` jobs as
    /// well as its waiting ones. Those are partly or wholly paid for, so a client
    /// confirms first.
    #[serde(default)]
    pub cancel_running: bool,
}

/// The `POST /ladders/{id}/climbers/retry` body: the climber to retry.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct LadderRetryInput {
    /// Which climber, in either shape and either — stored or read — form: the key is
    /// taken from the member as it would be stored, so a climber read off the board can
    /// be handed straight back.
    pub combination: ReviewPlanCombo,
}

/// The `POST /ladders/{id}/rungs/order` body: the rungs' stable ids in their new
/// climb order.
///
/// Ids rather than a list of rungs, because a reorder must not be able to edit a rung
/// in passing.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct LadderRungOrderInput {
    /// Every one of the ladder's rung ids, in the new order. Must be a permutation of
    /// what the ladder currently holds: a reorder that adds or drops a rung is an edit,
    /// and edits go through `PUT /ladders/{id}` where the consequences are visible.
    pub rung_ids: Vec<String>,
}
