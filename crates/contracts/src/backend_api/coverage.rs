//! The reviewer coverage surface: reusable groups, declarative plans, the coverage matrix a plan expands into, the runs-in-flight limit and the controls around a fill, and the rating gate a ladder's rung is judged by.

use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;

use crate::review::Rating;
use crate::run_record::HarnessSlug;

/// How many of a rung's runs must clear the [floor](Gate::floor) for the climber to
/// pass the rung.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub enum GateThreshold {
    /// An absolute number of runs, independent of how many the rung ran. `1` is
    /// "any run clearing the floor is enough", which is how "stop only when
    /// everything is broken" is expressed.
    Count {
        /// The number of runs that must clear the floor.
        runs: u32,
    },
    /// A share of the runs the rung finishes with, compared as
    /// `count >= fraction * runs`. Values outside `0.0..=1.0` are clamped into
    /// it, and a non-finite value (which cannot come from valid JSON but can from a
    /// corrupt row) reads as `0.0`.
    ///
    /// Clamping is not the same as neutralising: below the range it degrades to
    /// "always pass", but above it degrades to `1.0`, the strictest bar the rule
    /// can express — every run must clear the floor. That is deliberate.
    /// A fraction over one is a typo, not an instruction, and the nearest expressible
    /// reading of "more than all of them" is "all of them"; inventing a permissive
    /// answer instead would let a mistyped gate wave every climber through.
    Fraction {
        /// The share of the rung's runs that must clear the floor.
        fraction: f64,
    },
}

/// The rule one ladder applies at every rung.
///
/// Stored per ladder (not per rung): a ladder is a single question asked of an
/// ordered series of cases, so the bar it sets is the ladder's, and a rung only
/// varies how many runs it takes to answer.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct Gate {
    /// The worst rating that still counts as clearing the rung. A run rated this
    /// or better passes; anything worse does not.
    pub floor: Rating,
    /// How many runs must clear [`Self::floor`].
    pub threshold: GateThreshold,
    /// Whether a run whose build never loaded counts as [`Rating::Broken`] outright,
    /// whatever its validator rating says. On by default: there was nothing to play.
    #[serde(default = "unloaded_counts_as_broken_default")]
    pub unloaded_counts_as_broken: bool,
    /// Whether the gate may decide on partial results, the caller then cancelling the
    /// slot's jobs that have not started yet. **Off** by default — the runs are evidence in
    /// their own right, so a rung finishes what it started even when the verdict is
    /// already certain.
    #[serde(default)]
    pub early_stop: bool,
}

impl Default for Gate {
    /// The gentlest gate that still stops a hopeless climb: pass as long as a single
    /// run was playable at all, fail only when the whole rung is broken.
    fn default() -> Self {
        Self {
            floor: Rating::Scuffed,
            threshold: GateThreshold::Count { runs: 1 },
            unloaded_counts_as_broken: unloaded_counts_as_broken_default(),
            early_stop: false,
        }
    }
}

/// What a rung's evidence says about one climber.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub enum GateOutcome {
    /// The rung is passed; the climber moves to the next one.
    Passed,
    /// The rung is failed; the climber stops here. The validators are assumed correct,
    /// so this is the climber's result for that version of the case.
    Failed,
    /// Not enough evidence yet: runs are still to come, or finished runs carry no
    /// validator rating. The climber stays on the rung.
    Undecided,
}

/// How many of a plan's or a ladder dispatch's own jobs may be in flight at once before a
/// launch pass stops emitting — or no bound at all.
///
/// The unbounded shape is its own variant rather than a large bound so that it is
/// stored, reported, and shown as the instruction it is, and so that no bound the owner
/// types can be mistaken for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub enum InFlightLimit {
    /// Keep at most `runs` jobs in flight. `0` is a legitimate bound — "launch nothing"
    /// — and is not the same instruction as no bound.
    Bounded {
        /// The most jobs that may be in flight before a launch pass stops.
        runs: u32,
    },
    /// No bound: a launch pass emits every missing cell it can. The per-cell target and
    /// the harness parallelism cap still apply.
    Unbounded,
}

impl InFlightLimit {
    /// Whether `in_flight` jobs already fill this limit, so a launch pass must stop
    /// before emitting another cell. An unbounded limit is never full.
    pub fn is_full(self, in_flight: u32) -> bool {
        match self {
            InFlightLimit::Bounded { runs } => in_flight >= runs,
            InFlightLimit::Unbounded => false,
        }
    }

    /// The bound, or `None` when there is none. For callers that want to show or
    /// compare the number without matching on the shape.
    pub fn bound(self) -> Option<u32> {
        match self {
            InFlightLimit::Bounded { runs } => Some(runs),
            InFlightLimit::Unbounded => None,
        }
    }
}

/// One **pinned case** in a plan or a case group: a slug, an exact version, a variant,
/// and the [engine](crate::engine) its runs are built on. Coverage is counted
/// against exactly this pin; the matrix flags it when a newer version has since been
/// ingested.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ReviewPlanCase {
    /// The test-case slug (e.g. `caldera`).
    pub slug: String,
    /// The pinned, exact version (e.g. `v1.2.0`).
    pub version: String,
    /// The variant to cover (e.g. `base`).
    pub variant: String,
    /// The engine to cover (e.g. `simple-2d`), or null for the `none` engine — the
    /// engineless run every case supports, and exactly what a plan scheduled before the
    /// pin carried an engine asked for.
    ///
    /// The engine is in the pin because a result is only comparable with another result
    /// on the same engine: a model handed a runtime and a documented API is doing
    /// different work from the same model starting from nothing, so one case at one
    /// version and variant on two engines is two pinned cases and two sets of cells.
    ///
    /// A pin naming an engine the version does not declare support for is accepted here
    /// and reported by the run, exactly as an uningested version is. The catalogue moves
    /// under a standing plan, so the check belongs where a run executes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub engine: Option<String>,
}

impl ReviewPlanCase {
    /// The pin's engine as the cell key segments it: the slug it
    /// names, or `none` when it names nothing. Absent and `none` are the same pin, so
    /// they must never be two cells.
    pub fn engine_slug(&self) -> String {
        self.launch_engine()
            .unwrap_or_else(|| crate::engine::NONE_SLUG.to_string())
    }

    /// The pin's engine as a launch request carries it: the slug it names, or `None`
    /// where it names nothing, which is the key a launch omits to ask for the engineless
    /// run. A pin holding only whitespace is a pin holding nothing.
    pub fn launch_engine(&self) -> Option<String> {
        self.engine
            .as_deref()
            .map(str::trim)
            .filter(|slug| !slug.is_empty())
            .map(str::to_string)
    }
}

/// One **combination** in a plan, a ladder, or a combo group: what a cell's runs are
/// executed by.
///
/// One type carrying two shapes, exactly as
/// [`ComparisonArm`](crate::comparison::ComparisonArm) already does. A
/// member naming a [`gg_config_id`](Self::gg_config_id) is a **gg combination** — a
/// saved [gg configuration](https://docs.testcabinet.ai/gg/configurations/) plus a model
/// for each launch slot it declares — and one without it is a **harness combination**:
/// a harness, the model it runs, and a provider for a provider-routed harness. They are
/// unioned into one list rather than split across two, so a plan crossing both against
/// its cases needs no second axis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ReviewPlanCombo {
    /// The agent harness to drive — `gg` on a gg member.
    pub harness: HarnessSlug,
    /// The opaque model id passed to the harness. Empty in storage on a gg member,
    /// which binds a model per agent instead; a read fills it with the model the bound
    /// set's root agent runs on, so a client with no configuration in hand still has
    /// something to show.
    #[serde(default)]
    pub model: String,
    /// The provider for a provider-routed harness, or null.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub provider: Option<String>,
    /// The launcher's key for the gg configuration this member runs (`saved:<id>`), or
    /// null on a harness member. This is what makes a member a gg member.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub gg_config_id: Option<String>,
    /// A model per [launch slot](https://docs.testcabinet.ai/gg/configurations/) the
    /// configuration declares, keyed by slot name. Empty on a harness member.
    ///
    /// Ordered (a `BTreeMap`) because it is compared and keyed, not merely read: two
    /// members binding the same slots must produce the same
    /// `combination_key` whatever order they arrived in.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub gg_slot_models: BTreeMap<String, String>,
    /// The configuration's current display name, filled on read and **never stored**.
    ///
    /// A configuration is renamed in one place, and a member that had stored the name it
    /// bore at the time would go on showing the old one forever.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub gg_config_name: Option<String>,
}

impl ReviewPlanCombo {
    /// The bare id of the gg configuration this member names, or `None` on a harness
    /// member.
    ///
    /// The stored value is whatever arrived, so the normalization happens here, on every
    /// read, rather than being written into the row — and it is
    /// the same rule (`api::gg::saved_config_id`) a launch's `presetId` goes through, so a
    /// member and the run it schedules name one configuration by one string.
    pub fn gg_config_ref(&self) -> Option<&str> {
        super::gg::saved_config_id(self.gg_config_id.as_deref()?)
    }

    /// This member's slot bindings in **canonical** form: every slot name and every model id
    /// trimmed.
    ///
    /// The bindings are not merely read, they are compared: two members are the same member
    /// when they bind the same models, and this is the one form that comparison is made in —
    /// by the de-dupe key, by the launch that binds them, and by the check
    /// that every declared slot is filled. Surrounding space in a pasted model id would
    /// otherwise make one written-down decision look like two: two members, two cells' worth
    /// of runs enqueued, and one cell to count them in.
    pub fn gg_bindings(&self) -> BTreeMap<String, String> {
        self.gg_slot_models
            .iter()
            .map(|(slot, model)| (slot.trim().to_string(), model.trim().to_string()))
            .collect()
    }

    /// This member as it is **stored**: unchanged for a harness member, and for a gg member
    /// stripped of every value a read derives — its `model`, its `provider`, and the
    /// configuration name — with its harness forced to [`Gg`](HarnessSlug::Gg).
    ///
    /// What a gg member *is* is the configuration it names and the models it binds to that
    /// configuration's launch slots; everything else about it is re-derived from the
    /// configuration on every read. Without this normalization a console that reads a plan
    /// and saves it back would store the name and the root model as they stood at that
    /// moment, and the member would go on reporting them after the configuration had been
    /// renamed or re-bound — a stale value nothing would ever correct, because nothing else
    /// writes those fields.
    pub fn for_storage(&self) -> ReviewPlanCombo {
        if self.gg_config_ref().is_none() {
            return self.clone();
        }
        ReviewPlanCombo {
            harness: HarnessSlug::Gg,
            model: String::new(),
            provider: None,
            gg_config_id: self.gg_config_id.clone(),
            gg_slot_models: self.gg_bindings(),
            gg_config_name: None,
        }
    }
}

/// Which kind of members a coverage group holds: harness+model combinations or
/// version-pinned cases. A group holds one kind; a plan references groups of both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub enum CoverageGroupKind {
    /// A group of harness+model combinations.
    Combo,
    /// A group of version-pinned test cases.
    Case,
}

impl CoverageGroupKind {
    /// The stored/wire token for the kind.
    pub fn as_str(self) -> &'static str {
        match self {
            CoverageGroupKind::Combo => "combo",
            CoverageGroupKind::Case => "case",
        }
    }

    /// Parse a stored kind token, or `None` for an unknown value (a corrupt row).
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "combo" => Some(CoverageGroupKind::Combo),
            "case" => Some(CoverageGroupKind::Case),
            _ => None,
        }
    }
}

/// Which axis a coverage plan's cell loop nests on — and therefore the order its
/// runs execute in, since a launch pass emits cells in this order, `job.queue_seq` is
/// monotonic, and the dispatcher claims in ascending order.
///
/// The console labels these "One case at a time" and "One model at a time".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub enum CoverageAxis {
    /// Finish one case across every combination before starting the next case. The
    /// default, and what every plan did before the axis was selectable.
    #[default]
    Case,
    /// Finish one combination across every case before starting the next
    /// combination — "take this model all the way through the plan".
    Combination,
}

impl CoverageAxis {
    /// The stored/wire token for the axis.
    pub fn as_str(self) -> &'static str {
        match self {
            CoverageAxis::Case => "case",
            CoverageAxis::Combination => "combination",
        }
    }

    /// Parse a stored axis token. An unrecognized value falls back to the default
    /// rather than erroring: the axis only decides emission *order*, so a row written
    /// by a newer build degrades to today's ordering instead of making the plan
    /// unreadable.
    pub fn parse(token: &str) -> Self {
        match token {
            "combination" => CoverageAxis::Combination,
            _ => CoverageAxis::Case,
        }
    }
}

/// An owner's saved, reusable group of combinations or cases. Referenced by plans
/// as a pointer; editing the group reshapes every plan that references it. Exactly
/// one of `combos`/`cases` is populated, per `kind`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CoverageGroup {
    /// The group's opaque id (minted on create).
    pub id: String,
    /// The reviewer-chosen display name.
    pub name: String,
    /// The member kind.
    pub kind: CoverageGroupKind,
    /// The harness+model combinations, when `kind` is `combo` (else empty).
    pub combos: Vec<ReviewPlanCombo>,
    /// The version-pinned cases, when `kind` is `case` (else empty).
    pub cases: Vec<ReviewPlanCase>,
    /// RFC 3339 of when the group was last saved.
    pub updated_at: String,
}

/// The create/update body for a coverage group (the server assigns `id` and
/// `updatedAt`). Only the members matching `kind` are kept.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CoverageGroupInput {
    /// The reviewer-chosen display name.
    pub name: String,
    /// The member kind.
    pub kind: CoverageGroupKind,
    /// The harness+model combinations (kept only when `kind` is `combo`).
    #[serde(default)]
    pub combos: Vec<ReviewPlanCombo>,
    /// The version-pinned cases (kept only when `kind` is `case`).
    #[serde(default)]
    pub cases: Vec<ReviewPlanCase>,
}

/// An owner's named coverage plan: the groups it references, any one-off members, the
/// target runs-per-cell, the order it launches in, and its runs-in-flight limit
/// override. Persisted whole; one account may hold many.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CoveragePlan {
    /// The plan's opaque id (minted on create).
    pub id: String,
    /// The owner-chosen display name.
    pub name: String,
    /// The target number of runs desired for each `case × combination` cell.
    pub runs_per_cell: u32,
    /// The referenced combination groups' ids.
    pub combo_group_ids: Vec<String>,
    /// The referenced case groups' ids.
    pub case_group_ids: Vec<String>,
    /// One-off combinations pinned directly on the plan (unioned with the groups).
    pub combos: Vec<ReviewPlanCombo>,
    /// One-off cases pinned directly on the plan (unioned with the groups).
    pub cases: Vec<ReviewPlanCase>,
    /// Which axis the cell loop nests on, and therefore the order runs launch in.
    #[serde(default)]
    pub outer_axis: CoverageAxis,
    /// This plan's override of the account's runs-in-flight limit, or null to inherit
    /// it. Null, a bound of `0`, and `unbounded` are three different instructions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub in_flight_limit: Option<InFlightLimit>,
    /// How many automatic retries each run the plan launches gets, `0..=10`. A launch
    /// that uses them up without a counted run blocks its cell.
    #[serde(default = "default_retry_count")]
    pub retry_count: u32,
    /// RFC 3339 of when the plan was last saved.
    pub updated_at: String,
}

/// One plan as a reader sees it: its configuration, and whether it is filling — which
/// only the fill and halt endpoints change.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CoveragePlanOut {
    /// The plan's configuration.
    #[serde(flatten)]
    pub plan: CoveragePlan,
    /// Whether the plan is filling: launching its missing runs under its limit until
    /// every launchable cell is filled.
    pub filling: bool,
}

/// The create/update body for a coverage plan (the server assigns `id` and
/// `updatedAt`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CoveragePlanInput {
    /// The owner-chosen display name.
    pub name: String,
    /// The target number of runs desired for each `case × combination` cell.
    /// Rejected outside `MIN_RUNS_PER_CELL..=MAX_RUNS_PER_CELL` rather than corrected
    /// into range.
    pub runs_per_cell: u32,
    /// The referenced combination groups' ids.
    #[serde(default)]
    pub combo_group_ids: Vec<String>,
    /// The referenced case groups' ids.
    #[serde(default)]
    pub case_group_ids: Vec<String>,
    /// One-off combinations pinned directly on the plan.
    #[serde(default)]
    pub combos: Vec<ReviewPlanCombo>,
    /// One-off cases pinned directly on the plan.
    #[serde(default)]
    pub cases: Vec<ReviewPlanCase>,
    /// Which axis the cell loop nests on. Defaults to `case`.
    #[serde(default)]
    pub outer_axis: CoverageAxis,
    /// The plan's runs-in-flight limit override, or null to inherit the account's. A
    /// bound is clamped to `MAX_IN_FLIGHT_LIMIT`.
    #[serde(default)]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub in_flight_limit: Option<InFlightLimit>,
    /// How many automatic retries each run the plan launches gets, or null for the
    /// default of one. Clamped to `MAX_RETRY_COUNT`.
    #[serde(default)]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub retry_count: Option<u32>,
}

/// One cell of the coverage matrix: a plan case (at its pinned version) crossed
/// with a resolved combination, with the run/job counts that say how close it is to
/// the target.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CoverageCell {
    /// The test-case slug.
    pub slug: String,
    /// The pinned version this cell counts against.
    pub version: String,
    /// The variant.
    pub variant: String,
    /// The engine this cell counts against, resolved: `none` where the pin names none.
    /// Always concrete, because a run recorded with no engine is a `none` run and the
    /// two must land in one cell.
    pub engine: String,
    /// The harness — `gg` on a gg cell.
    pub harness: HarnessSlug,
    /// The model id: the one a harness cell's runs are launched with, and on a gg cell the
    /// one its bound set's root agent runs.
    pub model: String,
    /// The provider for a provider-routed harness, or null.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub provider: Option<String>,
    /// The gg configuration this cell's runs are launched from, as the member names it, or
    /// null on a harness cell.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub gg_config_id: Option<String>,
    /// That configuration's current display name — what the cell is labelled by, and the
    /// first gg segment of its identity. Null on a harness cell, and
    /// on a gg cell whose configuration no longer exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub gg_config_name: Option<String>,
    /// The model bound to each of the configuration's launch slots. Empty on a harness cell.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub gg_slot_models: BTreeMap<String, String>,
    /// Why a launch pass cannot launch this cell, or null when it can.
    ///
    /// A cell whose member cannot be resolved is still counted and still reported — it keeps
    /// its place in the matrix carrying the reason — because a plan that silently got
    /// smaller is a plan whose missing runs nobody can explain.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub unlaunchable: Option<String>,
    /// The target run count (the plan's `runs_per_cell`).
    pub desired: u32,
    /// The [cell's runs](https://docs.testcabinet.ai/components/backend/coverage/#a-cells-runs):
    /// the first `desired` counted runs of the cell to land, whoever launched them, in the
    /// order they landed. Every other figure on the cell is computed over these.
    pub run_ids: Vec<String>,
    /// How many runs the cell holds — the length of [`Self::run_ids`], so never more than
    /// `desired`. A counted run is the model's own result, a retried attempt once.
    pub counted: u32,
    /// Whether the cell is filled: `counted >= desired`. Runs in flight do not fill it.
    pub filled: bool,
    /// Whether the cell is [blocked](https://docs.testcabinet.ai/components/backend/coverage/#a-blocked-cell):
    /// it is missing a run and one of its jobs used up its automatic retries without a
    /// counted run with no later launch in its place, so a launch pass skips it until its
    /// owner retries it. A blocked cell can still have jobs in flight.
    pub blocked: bool,
    /// In-flight jobs (queued / pending / dispatched / starting / running) for this
    /// cell, counted globally and read only up to what the cell still needs
    /// (`desired - counted`), so a filled cell has none.
    pub in_flight: u32,
    /// How many of [`Self::in_flight`] are `pending` — deliberately held back rather
    /// than merely waiting to be claimed, because their harness is at its parallelism
    /// cap or (for a game jam) another run of the same jam is already going on that
    /// model. A **subset** of `inFlight`, not an addition to it.
    pub pending: u32,
    /// How many of the cell's runs are completed and not reviewed by the **requesting
    /// account**. Informational: it changes nothing about what the cell needs.
    pub unreviewed: u32,
    /// How many more runs to launch: `max(0, desired - (counted + in_flight))`.
    pub remaining: u32,
    /// The newest ingested version of this case (may differ from `version` when
    /// the pin is stale). Empty when the case is not ingested.
    pub latest_version: String,
    /// Whether the pinned `version` is not the newest ingested one — a hint to the
    /// reviewer that they may want to bump the pin.
    pub stale: bool,
}

/// The coverage matrix `GET /coverage-plans/{id}/coverage` returns: every cell plus
/// the rollups the plan dashboard header shows.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CoverageMatrix {
    /// Every `case × combination` cell, in the plan's own emission order — which
    /// axis is outer is the plan's [`CoveragePlan::outer_axis`], echoed below so
    /// a reader knows what the order means without fetching the plan again.
    pub cells: Vec<CoverageCell>,
    /// The axis the cells above are ordered on.
    pub outer_axis: CoverageAxis,
    /// How many cells are filled (`counted >= desired`; runs in flight do not count).
    pub cells_filled: u32,
    /// The total number of cells.
    pub cells_total: u32,
    /// How many cells are blocked on infrastructure failures.
    pub cells_blocked: u32,
    /// The plan's progress in runs: the sum of every cell's `counted`.
    pub runs_done: u32,
    /// The runs the plan asks for: the sum of every cell's `desired`.
    pub runs_total: u32,
    /// The plan's own jobs in flight — every job whose origin names the plan — which is
    /// what its limit counts.
    pub runs_in_flight: u32,
    /// The sum of every cell's `remaining` — the total runs still to launch.
    pub runs_missing: u32,
    /// The sum of every cell's `pending` — runs deliberately held back by the queue.
    pub runs_pending: u32,
    /// The sum of every cell's `unreviewed` — completed runs the requester has not
    /// reviewed. Informational.
    pub runs_unreviewed: u32,
    /// The runs-in-flight limit in force for this plan (its own override, else the
    /// account's setting, else the backend default).
    pub in_flight_limit: InFlightLimit,
    /// Whether the plan is filling.
    pub filling: bool,
    /// Whether the filling plan is waiting on its owner: none of its own jobs is in
    /// flight, nothing a launch pass could launch is left, and a cell is blocked.
    pub needs_attention: bool,
}

/// One plan's coverage roll-up for the plans list and the Home widget: the cell
/// counts without the per-cell detail the dashboard fetches on open.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CoveragePlanSummary {
    /// The plan's id.
    pub id: String,
    /// The plan's display name.
    pub name: String,
    /// The plan's target runs-per-cell.
    pub runs_per_cell: u32,
    /// How many cells are filled.
    pub cells_filled: u32,
    /// The total number of cells.
    pub cells_total: u32,
    /// How many cells are blocked on infrastructure failures.
    pub cells_blocked: u32,
    /// The plan's progress in runs: the sum of every cell's `counted`.
    pub runs_done: u32,
    /// The runs the plan asks for.
    pub runs_total: u32,
    /// The plan's own jobs in flight.
    pub runs_in_flight: u32,
    /// The total runs still to launch across the plan.
    pub runs_missing: u32,
    /// The completed runs across the plan the requester has not reviewed.
    pub runs_unreviewed: u32,
    /// Whether the plan is filling.
    pub filling: bool,
    /// Whether the filling plan is waiting on its owner: none of its own jobs is in
    /// flight, nothing a launch pass could launch is left, and a cell is blocked.
    pub needs_attention: bool,
}

/// The account-wide coverage settings `GET`/`PUT /coverage-settings` read and write: the
/// account's default runs-in-flight limit, which a plan or ladder may override.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CoverageSettings {
    /// How many of one plan's or one ladder dispatch's jobs may be in flight at once, or
    /// `unbounded`.
    pub in_flight_limit: InFlightLimit,
    /// Whether [`Self::in_flight_limit`] is the account's own choice or the backend's
    /// compiled-in default because they have never chosen one. A `PUT` always makes it
    /// a choice.
    pub is_default: bool,
}

/// The `PUT /coverage-settings` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CoverageSettingsInput {
    /// The limit to store. A bound is clamped to `MAX_IN_FLIGHT_LIMIT`; a bound of `0` is
    /// a legitimate value — "launch nothing" — and `unbounded` is stored as itself.
    pub in_flight_limit: InFlightLimit,
}

/// Why a launch pass did no work. "busy" is a moment that will pass; the others are
/// states the owner changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub enum LaunchSkipped {
    /// The plan is not filling.
    NotFilling,
    /// The ladder has no running dispatch.
    NotRunning,
    /// Another launch pass of the same plan or ladder holds the claim; the holder runs
    /// one more pass for this request before it lets go.
    Busy,
}

/// One cell a launch pass launched, and the jobs it enqueued for it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct LaunchedCell {
    /// The ladder rung this cell belongs to, or null for a coverage plan (which has
    /// no rungs). Shared shape, because plans and ladders launch through the same code
    /// path.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub rung_id: Option<String>,
    /// The test-case slug.
    pub slug: String,
    /// The pinned version.
    pub version: String,
    /// The variant.
    pub variant: String,
    /// The engine the enqueued runs are built on, resolved: `none` where the cell's pin
    /// names none.
    pub engine: String,
    /// The harness.
    pub harness: HarnessSlug,
    /// The combination's canonical model id (not the launched one — see
    /// `model_id::launch_model_id`).
    pub model: String,
    /// The provider for a provider-routed harness, or null.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub provider: Option<String>,
    /// The gg configuration the launched member names, or null on a harness member.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub gg_config_id: Option<String>,
    /// That configuration's current display name, when it still exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub gg_config_name: Option<String>,
    /// The model bound to each of the configuration's launch slots. Empty on a harness member.
    ///
    /// Reported for the same reason the blocked list reports it: `gg` and a root model are not
    /// a name — two configurations, or two arms of one, read identically without it.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub gg_slot_models: BTreeMap<String, String>,
    /// How many runs were enqueued for this cell — always the cell's whole shortfall,
    /// never a partial cell.
    pub runs: u32,
    /// The enqueued jobs' ids, in queue order, so a console can follow them straight
    /// into its in-progress list.
    pub job_ids: Vec<String>,
}

/// One cell a launch pass **could not** launch, and why: a member that cannot be
/// resolved, or a cell blocked on infrastructure failures.
///
/// Reported per cell, beside the launches rather than instead of them: a pass that
/// enqueued four cells and skipped two broken ones is working, and the owner needs to see
/// both numbers to know that.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct BlockedCell {
    /// The ladder rung this cell belongs to, or null for a coverage plan.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub rung_id: Option<String>,
    /// The test-case slug.
    pub slug: String,
    /// The pinned version.
    pub version: String,
    /// The variant.
    pub variant: String,
    /// The engine the cell would have launched on, resolved: `none` where the pin names
    /// none.
    pub engine: String,
    /// The harness — `gg` on a gg member.
    pub harness: HarnessSlug,
    /// The combination's model id, empty when the member could not be resolved far enough
    /// to have one.
    pub model: String,
    /// The provider for a provider-routed harness, or null.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub provider: Option<String>,
    /// The gg configuration the member names, or null on a harness member.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub gg_config_id: Option<String>,
    /// That configuration's current display name, when it still exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub gg_config_name: Option<String>,
    /// The model bound to each of the configuration's launch slots. Empty on a harness member.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub gg_slot_models: BTreeMap<String, String>,
    /// Why the cell could not be launched, in the words the owner has to act on.
    pub reason: String,
}

/// What a launch pass did, reported in enough detail that an idle plan or ladder is
/// never a mystery: whether it ran at all, what the limit allowed, and exactly what it
/// enqueued.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct LaunchPassResult {
    /// Why nothing was attempted, or null when the scheduler ran. A pass that ran and
    /// enqueued nothing (a full limit, or nothing missing) reports null here with
    /// `enqueued` zero.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub skipped: Option<LaunchSkipped>,
    /// The runs-in-flight limit in force.
    pub in_flight_limit: InFlightLimit,
    /// The plan's or dispatch's own jobs in flight as the scheduler saw them, or null
    /// when it never ran.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub in_flight: Option<u32>,
    /// How many runs were enqueued in total.
    pub enqueued: u32,
    /// The cells that were launched, in the order they were emitted — which is the
    /// order they will execute in.
    pub cells: Vec<LaunchedCell>,
    /// The cells the pass wanted to launch and could not, each with its reason. One
    /// broken member never stops the rest being launched, so this list and
    /// [`Self::cells`] are routinely both non-empty.
    pub unlaunchable: Vec<BlockedCell>,
    /// How many not-yet-started jobs (`queued` or `pending`) a ladder whose gate stops
    /// early cancelled because the rung they belonged to was decided. Always `0` on a
    /// coverage plan, and on a ladder with `earlyStop` off.
    pub early_stop_canceled: u32,
}

/// One run in a scoped review queue: a completed run of this plan (or ladder dispatch)
/// the requesting account has not reviewed.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CoverageQueueEntry {
    /// The run's id — what the console opens to review it.
    pub run_id: String,
    /// The ladder rung this run belongs to, or null for a coverage plan.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub rung_id: Option<String>,
    /// The test-case slug.
    pub slug: String,
    /// The test-case version.
    pub version: String,
    /// The variant.
    pub variant: String,
    /// The engine the run was built on, resolved: `none` where the cell's pin names none.
    pub engine: String,
    /// The harness.
    pub harness: HarnessSlug,
    /// The model id the run was launched with.
    pub model: String,
    /// The gg configuration the run's cell names, or null on a harness cell.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub gg_config_id: Option<String>,
    /// That configuration's current display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub gg_config_name: Option<String>,
    /// The model bound to each of the configuration's launch slots. Empty on a harness cell.
    ///
    /// The queue carries the member's whole identity rather than the run's harness and model,
    /// because on a gg cell those two are `gg` and a root model — the same two values for every
    /// configuration the plan crosses and for every arm of each. A worklist whose rows cannot
    /// be told apart is a worklist a reviewer cannot open in an informed order, and telling
    /// exactly those arms apart is what the plan was built for.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub gg_slot_models: BTreeMap<String, String>,
    /// RFC 3339 of when the run finished.
    pub finished_at: String,
}

/// The plan's (or ladder's) unreviewed-by-me runs, in its own order.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CoverageQueue {
    /// The runs to review, in the plan's or ladder's own order — **not** newest-first
    /// like the global Unreviewed page — so a case's repeats sit together.
    pub runs: Vec<CoverageQueueEntry>,
    /// Whether the listing was cut short at the cap. A queue is walked from the
    /// front, not paged, so this is a "there is more behind this" flag rather than a
    /// cursor.
    pub truncated: bool,
}

/// The runs a plan's cells hold, as `GET /coverage-plans/{id}/runs` returns them.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CoveragePlanRuns {
    /// The summary card of every run the plan's cells hold — each cell's
    /// [`CoverageCell::run_ids`] — in the matrix's cell order, and in the order the runs
    /// landed within a cell. A run beyond a cell's target is not here.
    pub runs: Vec<super::snapshot::RunSummary>,
}

/// The `POST /coverage-plans/{id}/cells/retry` body: the blocked cell to retry.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct PlanCellRetryInput {
    /// The cell's case, at its pinned version and engine.
    pub case: ReviewPlanCase,
    /// The cell's combination, as the plan holds it.
    pub combination: ReviewPlanCombo,
}

/// What a plan's halt or a ladder's stop did.
///
/// The **count is the point**, not a nicety: a halt that reports only success cannot
/// be told apart from a halt whose scope was wrong. The plan always stops filling (and
/// the dispatch always ends), which is why that is stated here in prose rather than
/// reported as a field that could only ever say `true`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct HaltResult {
    /// How many jobs were moved to `canceled`.
    pub canceled: u32,
    /// Whether the halt also reached jobs that were already executing (`halt all`)
    /// rather than only the ones that had cost nothing yet.
    pub included_active: bool,
}

impl LaunchPassResult {
    /// A launch pass that never ran, and why. The limit is still reported: the owner's
    /// next question after "it did nothing" is "what was it aiming for?".
    pub fn skipped_by(reason: LaunchSkipped, in_flight_limit: InFlightLimit) -> Self {
        Self {
            skipped: Some(reason),
            in_flight_limit,
            in_flight: None,
            enqueued: 0,
            cells: Vec::new(),
            unlaunchable: Vec::new(),
            early_stop_canceled: 0,
        }
    }

    /// Two passes' results as one: what both enqueued and could not launch, the later
    /// pass's view of the limit, and every early-stop cancel.
    pub fn merged_with(self, later: LaunchPassResult) -> LaunchPassResult {
        let mut cells = self.cells;
        cells.extend(later.cells);
        let mut unlaunchable = self.unlaunchable;
        unlaunchable.extend(later.unlaunchable);
        LaunchPassResult {
            skipped: later.skipped,
            in_flight_limit: later.in_flight_limit,
            in_flight: later.in_flight.or(self.in_flight),
            enqueued: self.enqueued + later.enqueued,
            cells,
            unlaunchable,
            early_stop_canceled: self.early_stop_canceled + later.early_stop_canceled,
        }
    }
}

/// The retry limit a plan or ladder saved without one launches with: the count a launch
/// request that names none gets.
pub fn default_retry_count() -> u32 {
    super::jobs::DEFAULT_RETRY_COUNT
}

/// The default for [`Gate::unloaded_counts_as_broken`], as a function so serde can
/// name it when the field is absent from a stored gate.
fn unloaded_counts_as_broken_default() -> bool {
    true
}
