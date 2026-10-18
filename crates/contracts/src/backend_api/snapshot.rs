//! The published snapshot documents: the static gallery's data (the index, the runs index and its summary cards, the per-run document, a case's metadata, the model catalog, the groups, the comparisons and the gg document corpus).

use serde::Serialize;

use super::models::ModelOut;
use crate::run_record::RunRecord;

// --- Wire shapes (§3) -------------------------------------------------------

/// The top-level snapshot pointer (`index.json`): where the runs index, per-run
/// documents, and case documents live under this snapshot's prefix.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct SnapshotIndex {
    pub schema_version: u32,
    pub snapshot_id: String,
    pub generated_at: String,
    pub run_count: usize,
    pub runs_key: String,
    /// The shared prefix the per-run documents live under
    /// (`documents/runs/`) — snapshot-independent, so it is the same string in every
    /// generation. Informational: a reader resolves a run's document through the
    /// `documentKey` on its summary, because the key carries a content digest and so
    /// cannot be composed from the run id. Operators and
    /// `scripts/recover-run-media-from-snapshot.sh` use it to scope a listing.
    pub run_documents_prefix: String,
    pub cases_prefix: String,
    /// Where this snapshot's model catalog lives (`<prefix>/models.json`).
    pub models_key: String,
    /// Where this snapshot's comparisons index lives (`<prefix>/comparisons.json`).
    pub comparisons_key: String,
    /// The prefix each published comparison's own document lives under
    /// (`<prefix>/comparisons/<id>.json`).
    pub comparisons_prefix: String,
    /// Where this snapshot's gg document corpus lives (`<prefix>/gg-runs.json`) — the
    /// payload the public Discover surface evaluates in the browser.
    pub gg_runs_key: String,
    /// Where this snapshot's test-case-group set lives
    /// (`<prefix>/test-case-groups.json`). Optional on the wire because it
    /// postdates the other keys: a snapshot written before groups existed carries
    /// none, and a reader treats the absent key as an empty group set.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub test_case_groups_key: Option<String>,
}

/// The gg document corpus file (`gg-runs.json`): every exported gg run as one flat map
/// of dotted fields, plus the instant the export was taken.
///
/// This is the **whole** public analysis payload. The site's Discover surface runs the
/// mirrored TypeScript evaluator over these documents and makes no backend call at all,
/// which is only affordable because a document is an order of magnitude smaller than the
/// record it derives from — no source, no prompts, no model output.
///
/// It carries its own `generated_at` even though [`SnapshotIndex`] has one, because the
/// public corpus legitimately lags the console's: every figure the site renders has to be
/// labelled with the instant it was true, and a figure and its as-of time should travel
/// in the same object rather than be joined at read time.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct GgRunsFile {
    pub schema_version: u32,
    /// When this corpus was exported (RFC 3339), rendered beside every public figure.
    pub generated_at: String,
    /// The exported documents, already filtered and redacted (see
    /// `SnapshotBuilder::with_gg_documents`).
    pub documents: Vec<crate::gg_query::GgRunDoc>,
}

/// The comparisons index file (`comparisons.json`): every published harness
/// comparison as its full read model. The public site lists and renders them from
/// here (each also has its own `<prefix>/comparisons/<id>.json` for a direct fetch).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ComparisonsIndex {
    pub schema_version: u32,
    pub comparisons: Vec<crate::comparison::Comparison>,
}

/// One published comparison's own document (`comparisons/<id>.json`).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ComparisonFile {
    pub schema_version: u32,
    pub comparison: crate::comparison::Comparison,
}

/// The model catalog file (`models.json`): the composed catalog the public site
/// renders the Models section from.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ModelCatalogFile {
    pub schema_version: u32,
    pub models: Vec<ModelOut>,
}

/// The test-case-group set file (`test-case-groups.json`): the ingested groups
/// in the order `GET /test-case-groups` serves them, from which the public site
/// renders the home page's per-group leaderboards. Repo-authored catalog data,
/// uploaded as built (no scrubbing — see the builder's emission site).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct TestCaseGroupsFile {
    pub schema_version: u32,
    /// The groups, in display order — the same wire shape the live API serves.
    pub groups: Vec<super::test_cases::TestCaseGroupOut>,
}

/// The flat index of run summary cards (`runs.json`), newest first.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct RunsIndex {
    pub schema_version: u32,
    pub runs: Vec<RunSummary>,
}

/// The denormalized summary card for one published run.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct RunSummary {
    pub id: String,
    pub published_at: String,
    pub started_at: String,
    pub finished_at: String,
    pub subject: SubjectOut,
    pub case_name: String,
    pub metrics: crate::metrics::RunMetrics,
    pub validation_loaded: bool,
    pub state: crate::run_record::RunState,
    /// The run's **functional** rating. On a legacy run the worst rating any
    /// reviewer gave any domain, `None` while the run carries no reviews (an
    /// unrated console run). On a [validator-rated](Self::validator_rated) run the
    /// validators' decision as overridden by its reviews — each failing scored
    /// point caps its domains at its declared failure cap, each review's overrides
    /// overlay the validators' verdicts, the run gets the worst across the
    /// reviews' effective ratings (the validators' own figure while unreviewed),
    /// composed with the toolchain gate — which is `Some` from the moment the run
    /// completes, with or without a review. A published run always has one.
    pub rating: Option<crate::review::Rating>,
    /// The run's aggregate **aesthetic** rating: the worst run-wide tier across
    /// its reviews, or `None` when no review has rated the aesthetic channel — a
    /// validator-rated run nobody has reviewed yet, and every legacy run (its
    /// reviews carry no aesthetic ratings, so it never shows the badge).
    #[cfg_attr(feature = "contract", ts(optional = nullable))]
    pub aesthetic: Option<crate::review::AestheticRating>,
    /// Whether the run is **validator-rated**: its case version is on the engine
    /// manifest format and not a game jam, so [`rating`](Self::rating) and
    /// [`score`](Self::score) are decided by the validators (present without any
    /// review, with each review's overrides folded in), its reviewers supply the
    /// [`aesthetic`](Self::aesthetic) channel and any verdict overrides, and it
    /// publishes with zero reviews. `false` for every legacy run, whose card reads
    /// exactly as it always has. Lifted here so every consumer can branch on it
    /// without a catalog.
    pub validator_rated: bool,
    /// How many reviews the run carries. The site averages their scores on a
    /// legacy run; the aggregate sits between the harshest and most generous
    /// review. On a validator-rated run it counts the aesthetic reviews.
    pub review_count: usize,
    /// The run's aggregate reviewer score: the mean earned checklist weight across
    /// its reviews. `None` when the run has no reviews (or its case's checklist
    /// weights can't be resolved). Like `case_name`, this is enriched by the
    /// callers that hold the case catalog (the console listing and the snapshot
    /// builder); `RunSummary::from_stored` leaves it `None` as it is
    /// catalog-free.
    pub score: Option<RunScoreOut>,
    /// The correctness-and-fuel result of a performance run, lifted onto the
    /// summary card so a fuel leaderboard and a run's percentile can be computed
    /// from the bounded case-scoped summary set without loading each full record.
    /// `None` for every non-performance run (which carries no
    /// `validation.performance`). Unlike [`Self::score`] this is catalog-free —
    /// fuel needs no checklist weights — so `RunSummary::from_stored` fills it.
    #[cfg_attr(feature = "contract", ts(optional = nullable))]
    pub performance: Option<PerformanceSummaryOut>,
    /// The ranking-relevant slice of the run's [code
    /// analysis](crate::code_analysis), lifted onto the card so a
    /// "which model writes the tightest code?" ordering can be computed from the
    /// bounded summary set without loading every run's full record. See
    /// [`CodeSummaryOut`], which also explains why the provenance rides along with
    /// the figures.
    ///
    /// `None` means the run was **never analysed** — not that it wrote no code. The
    /// corpus is [not backfilled], so every run that finished before the analyzer
    /// shipped carries `None` forever, and any view that renders this must say
    /// "not measured" rather than draw a zero.
    ///
    /// Catalog-free (the figures are already on the record), so
    /// `RunSummary::from_stored` fills it.
    ///
    /// [not backfilled]: https://docs.testcabinet.ai/gg/analysis/code-analysis/#publishing-and-the-analyzer-version
    #[cfg_attr(feature = "contract", ts(optional = nullable))]
    pub code: Option<CodeSummaryOut>,
    /// Where this run's full document lives: its content-addressed
    /// `documents/runs/<id>/<digest>.json` key. The digest is over the document's own
    /// bytes, so the key cannot be composed from the run id — the summary index is how
    /// a reader learns it, and following it is the only supported way to reach a run's
    /// record from the snapshot.
    ///
    /// `None` on a console card, which has no published document at all;
    /// `RunSummary::from_stored` leaves it unset and the snapshot builder fills it.
    /// Always `Some` in `runs.json`.
    #[cfg_attr(feature = "contract", ts(optional = nullable))]
    pub document_key: Option<String>,
    pub links: LinksOut,
}

/// The performance result as a summary card carries it: the correctness gate and
/// the comparable total fuel. Enough to rank a fuel leaderboard and place one run
/// against the field without the full `PerformanceResult` breakdown. Mirrors the
/// two ranking-relevant fields of
/// [`crate::validation::PerformanceResult`].
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct PerformanceSummaryOut {
    /// Whether every scored input case produced the oracle's exact answer — the
    /// gate a run must pass before its fuel means anything.
    pub correct: bool,
    /// The total fuel a correct engine consumed across every scored case (lower is
    /// better). `None` for an incorrect run, where the fuel is meaningless and the
    /// run earns no leaderboard placement.
    pub total_fuel: Option<u64>,
}

/// The code analysis as a summary card carries it: three ranking-relevant figures,
/// plus the provenance a reader needs before comparing two of them.
///
/// The full [`CodeAnalysisSummary`](crate::code_analysis::CodeAnalysisSummary)
/// is ninety-odd leaves and already rides on the record inside the per-run document;
/// this is the part a *list* sorts on, so it stays small — the same bargain
/// [`PerformanceSummaryOut`] strikes for fuel, and catalog-free for the same reason.
///
/// **The provenance fields are not decoration.** Two things make a bare figure
/// dishonest here. Analysis is [never backfilled], so an absent `code` on a card means
/// *not measured*, and among the cards that do carry one an
/// [`allFiles`](crate::code_analysis::CodeAuthoredBasis::AllFiles) authored
/// basis or a [`postValidation`](crate::code_analysis::CodeTreeBasis::PostValidation)
/// tree basis measured a different population than the exact one — the silent-degradation
/// risk the basis fields exist for. And a
/// [truncated](crate::code_analysis::CodeAnalysisNotes::truncated) analysis is
/// excluded from aggregation by default, so a view that ranks it beside complete ones
/// ranks a partial figure that looks complete. Carrying all four alongside the numbers is
/// what lets a card say so without fetching the record.
///
/// [never backfilled]: https://docs.testcabinet.ai/gg/analysis/code-analysis/#publishing-and-the-analyzer-version
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CodeSummaryOut {
    /// The [analyzer generation](crate::code_analysis::CODE_ANALYZER_VERSION)
    /// that produced these figures, so a corpus spanning two generations is visible
    /// rather than reading as a step change in the models.
    pub analyzer_version: u32,
    /// How the authored set was resolved — how much of this tree is actually the
    /// model's work.
    pub authored_basis: crate::code_analysis::CodeAuthoredBasis,
    /// Which state of the tree was measured.
    pub tree_basis: crate::code_analysis::CodeTreeBasis,
    /// Whether a tree-wide cap stopped the analysis short. A truncated result is
    /// excluded from aggregation by default.
    pub truncated: bool,
    /// How much code the model wrote: non-blank, non-comment lines across the
    /// authored set.
    pub code_lines: u32,
    /// The Gini coefficient of code lines across files — zero when every file is the
    /// same size, approaching one when a single file holds everything. The one number
    /// that answers "did the model split the work?".
    pub gini_code_lines: f64,
    /// Mean Sonar cognitive complexity per function. Cognitive rather than cyclomatic
    /// because cyclomatic is blind to nesting, and nesting is what makes generated code
    /// unreadable.
    pub mean_cognitive: f64,
}

impl CodeSummaryOut {
    /// Lift the card's slice off the bounded summary the run record carries.
    pub fn from_summary(code: &crate::code_analysis::CodeAnalysisSummary) -> Self {
        Self {
            analyzer_version: code.analyzer_version,
            authored_basis: code.authored_basis,
            tree_basis: code.tree_basis,
            truncated: code.notes.truncated,
            code_lines: code.size.code_lines,
            gini_code_lines: code.size.gini_code_lines,
            mean_cognitive: code.complexity.mean_cognitive,
        }
    }
}

/// A run's aggregate reviewer score: mean earned checklist weight across its
/// reviews, over the shared total available. `None` when the run has no reviews
/// (or its case's checklist weights can't be resolved). The item weights live
/// only in the case catalog, so this is computed by callers that hold both the
/// reviews and the catalog (see `run_summary_score`).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct RunScoreOut {
    /// The mean weight earned across the run's reviews.
    pub earned: f64,
    /// The total weight available — identical across the run's reviews.
    pub total: u32,
    /// How many reviews the average is taken over.
    pub reviews: u32,
    /// A [game jam](crate::test_case::TestType::GameJam) run's overall
    /// game grade — the worst overall grade any reviewer gave (see
    /// `review::aggregate_overall_grade`). This is the jam's
    /// rating badge, standing in for the per-domain `rating` a jam does not carry.
    /// `None` for every non-jam run.
    #[cfg_attr(feature = "contract", ts(optional = nullable))]
    pub overall_grade: Option<crate::review::VerdictStatus>,
}

/// The run subject as a summary card carries it (the slug enum, not a string).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct SubjectOut {
    pub test_case_slug: String,
    pub test_case_version: String,
    /// The test type this run's case belongs to. The UI run-log branches on this
    /// to render the category column.
    pub test_type: crate::test_case::TestType,
    pub variant: String,
    pub harness_slug: crate::run_record::HarnessSlug,
    pub harness_version: Option<String>,
    /// The slug of the [engine](crate::engine) the produced build was
    /// written against (`none` when it supplied its own runtime). Lifted onto the
    /// card because the engine is a *run dimension* selected alongside the variant,
    /// and a result is only comparable with another result on the same engine — so
    /// every listing that shows the variant has to be able to show this beside it.
    pub engine_slug: String,
    /// The version of the engine runtime vendored into the run repository. `None`
    /// for an engine that vendors no runtime (`none` has no package), and for runs
    /// recorded before engine selection existed.
    #[cfg_attr(feature = "contract", ts(optional = nullable))]
    pub engine_version: Option<String>,
    pub model_id: String,
    /// The name of the gg **configuration** this run was launched from — the
    /// [`preset`](crate::gg::GgCapabilitySet::preset) recorded on the run's
    /// capability set. Lifted onto the card because a gg run has no single harness
    /// model to identify it by ([`model_id`](Self::model_id) is only its
    /// representative primary-slot model, one of several per-agent bindings), so the
    /// run log shows the configuration in that cell instead. `None` for every
    /// third-party-harness run (which carries no capability set) and for a gg run
    /// assembled by hand rather than from a named configuration.
    #[cfg_attr(feature = "contract", ts(optional = nullable))]
    pub gg_preset: Option<String>,
    /// The **id** of the gg configuration this run was launched from — the
    /// [`preset_id`](crate::gg::GgCapabilitySet::preset_id) recorded on the
    /// run's capability set. Lifted onto the card beside the
    /// [name](Self::gg_preset) because it is what identifies the run's [coverage
    /// cell](https://docs.testcabinet.ai/components/backend/coverage/), and so what a
    /// listing narrowed to one configuration's runs matches on: a name is display text
    /// that is rewritten freely and is unique to nothing. `None` for every
    /// third-party-harness run and for a gg run assembled by hand.
    #[cfg_attr(feature = "contract", ts(optional = nullable))]
    pub gg_config_id: Option<String>,
}

impl SubjectOut {
    pub fn from(record: &RunRecord) -> Self {
        Self {
            test_case_slug: record.subject.test_case_slug.clone(),
            test_case_version: record.subject.test_case_version.clone(),
            test_type: record.subject.test_type,
            variant: record.subject.variant.clone(),
            harness_slug: record.subject.harness_slug,
            harness_version: record.subject.harness_version.clone(),
            engine_slug: record.subject.engine_slug.clone(),
            engine_version: record.subject.engine_version.clone(),
            model_id: record.subject.model_id.clone(),
            gg_preset: record
                .subject
                .gg_capability_set
                .as_ref()
                .and_then(|set| set.preset.clone()),
            gg_config_id: record
                .subject
                .gg_capability_set
                .as_ref()
                .and_then(|set| set.preset_id.clone()),
        }
    }
}

/// A per-run document (`runs/<id>.json`): the run record, its reviews and links,
/// the recorded event stream, and the snapshot-relative keys of its media.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct PerRun {
    pub schema_version: u32,
    pub record: RunRecord,
    /// The run's reviews, oldest first. The site averages their scores and takes
    /// the worst rating across them; each entry names its reviewer.
    pub reviews: Vec<Review>,
    pub links: LinksOut,
    /// The run's recorded normalized event stream (a JSON array), omitted when the
    /// run captured none. The site emits this as a per-run static asset its Events
    /// tab fetches; raw harness output is never included.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional, type = "unknown"))]
    pub events: Option<serde_json::Value>,
    /// The run's uploaded proof-of-implementation media, named by snapshot-relative
    /// key. Empty when the run produced none.
    pub proof_media: Vec<RunProofOut>,
    /// The run's synthesized *actual* automated-validation media (the model build's
    /// per-review-item debug-script outputs), named by snapshot-relative key. Empty
    /// when the run declares no debug scripts (or none produced media). The case-scoped
    /// *baseline* counterpart rides on [`CaseMetadata::validation_baselines`]. Always
    /// emitted (possibly empty); the static gallery treats it as optional so a snapshot
    /// written before this field existed still loads.
    pub validation_media: Vec<RunValidationMediaOut>,
    /// An asset-generation run's media (regenerated/preview image + action log),
    /// named by snapshot-relative key. Empty for a non-asset-generation run.
    pub asset_media: Vec<RunAssetOut>,
    /// The run's [showcase](crate::RunShowcase) files — the carousel
    /// media plus any image the description references — named by snapshot-relative
    /// key. Empty for a run whose record carries no showcase (every record written
    /// before the field existed), and possibly a subset of the carousel when a
    /// file's bytes could not be read. Always emitted (possibly empty); the static
    /// gallery treats it as optional so a snapshot written before this field
    /// existed still loads.
    pub showcase_media: Vec<RunShowcaseOut>,
    /// The snapshot-relative key of the run's **unbounded**
    /// [code-analysis document](crate::code_analysis::CodeAnalysisDocument) —
    /// every authored file, every scored function, every import edge, cycle and clone
    /// group — published as its own object so the public Code tab can fetch it on demand
    /// rather than inflating this document (and therefore every run's page load) with a
    /// tier only one tab reads.
    ///
    /// Content-stable and **generation-keyed**
    /// (`media/runs/<id>/code-analysis/v<analyzerVersion>.json`), so a refresh that finds
    /// the object already in the bucket references it without re-reading or re-uploading
    /// the bytes — and a *re-analysis under a newer generation* mints a different key
    /// rather than silently overwriting figures a published snapshot still points at.
    ///
    /// `None` when the run was never analysed, and also when it was but the document's
    /// bytes are no longer readable (the backend store is ephemeral) — the bounded summary
    /// on the record survives either way, so the tab degrades to the figures instead of
    /// offering a link that 404s.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub code_analysis_key: Option<String>,
}

/// A proof media file exposed in a per-run document. `id` matches the proof's
/// declared id (and its `validation.proofs[].id`); `kind` is image or video;
/// `key` is the snapshot-relative object key of the media.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct RunProofOut {
    pub id: String,
    pub kind: crate::MediaKind,
    pub key: String,
}

/// An asset-generation media file exposed in a per-run document. `file` is the
/// stable served name the result view requests — a single sprite's
/// `regenerated.png`/`preview.png`/`actions.json` or a sprite sheet's per-frame
/// `regenerated-<index>.png` (etc.); `key` is its snapshot-relative object key.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct RunAssetOut {
    pub file: String,
    pub key: String,
}

/// A [showcase](crate::RunShowcase) file exposed in a per-run document —
/// a carousel media file, or an image the description references. `file` is the
/// recorded name the gallery requests (the plain file name in the produced tree's
/// `showcase/`); `key` is its snapshot-relative object key, whose bytes are the
/// media as published — a video transcoded to `.mp4`, so `key` and `file` differ in
/// extension for a clip while the name the UI requests still resolves through the
/// static gallery's map (the validation-media convention).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct RunShowcaseOut {
    pub file: String,
    pub key: String,
}

/// A synthesized *actual* validation media file exposed in a per-run document — one
/// debug-script output captured from the model's build. `file` is the flat
/// `<item>__<output>.<ext>` name the gallery requests (`.png`/`.webm`, keyed off the
/// output's kind exactly as the reviewer UI's `validationMediaFor` computes it); `key`
/// is its snapshot-relative object key, whose bytes are the media as published — a
/// video transcoded to `.mp4`, so `key` and `file` differ in extension for a clip while
/// the flat name the UI requests still resolves through the static gallery's map.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct RunValidationMediaOut {
    pub file: String,
    pub key: String,
}

/// One published review on a run: the reviewer's public identity, their per-domain
/// ratings, the writeup, and their checklist verdicts. This is the canonical
/// review wire shape (`backend-api/review.schema.json`), referenced by the per-run
/// snapshot document.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "contract",
    derive(ts_rs::TS, schemars::JsonSchema),
    ts(rename = "Review"),
    schemars(rename = "Review")
)]
pub struct Review {
    /// The reviewing account's id (stable across their reviews).
    pub reviewer_id: String,
    /// The reviewer's display name, shown beside their review.
    pub reviewer: String,
    /// The reviewer's functional rating for each scoring domain. This review's
    /// overall rating is the worst across them. Empty on a review of a
    /// validator-rated run, whose functional rating the validators decide.
    pub ratings: Vec<crate::review::DomainRating>,
    /// **Legacy:** the reviewer's per-domain aesthetic ratings, from when the
    /// channel was rated per scoring domain. No longer emitted — a stored
    /// legacy row's tiers are collapsed into [`aesthetic`](Self::aesthetic)
    /// instead — but kept in the contract so a freshly deployed site can still
    /// read a not-yet-regenerated snapshot (resolve a review's tier as
    /// `aesthetic ?? worst(aesthetics)`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aesthetics: Vec<crate::review::DomainAesthetic>,
    /// The reviewer's **run-wide** aesthetic tier, on a review of a
    /// validator-rated run (a legacy per-domain row is already collapsed to its
    /// worst tier); the run's aesthetic rating is the worst across its reviews'
    /// tiers. Absent on a legacy run's review, which has no aesthetic channel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional = nullable))]
    pub aesthetic: Option<crate::review::AestheticRating>,
    pub writeup: String,
    pub checklist: Vec<crate::review::ReviewVerdict>,
    /// RFC 3339 of when the review was **first** submitted (unchanged by later
    /// edits — see [`Self::edited_at`]).
    pub reviewed_at: String,
    /// RFC 3339 of when the review was last edited, or `None` if it has never been
    /// edited since it was first submitted. The newest [`Self::revisions`] entry
    /// carries the same timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional = nullable))]
    pub edited_at: Option<String>,
    /// The review's edit history, oldest first: one entry per edit, each with the
    /// reviewer's note and the autogenerated diff of what changed. Empty for a
    /// review that has never been edited. Public, so a reader can see how a review
    /// evolved.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub revisions: Vec<crate::review::ReviewRevision>,
    /// The snapshot-relative object key of the reviewer's profile picture
    /// (`pfp/<reviewer-id>`), or `None` when the reviewer has no picture. The
    /// bytes are exported once per reviewer under the content-stable top-level
    /// `pfp/` prefix (like run media under `MEDIA_PREFIX`); the site resolves the
    /// key against the snapshot base into an absolute avatar URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional = nullable))]
    pub picture_key: Option<String>,
}

/// A run's outbound links as the snapshot carries them.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct LinksOut {
    pub source_repo: Option<String>,
    pub playable_build: Option<String>,
}

/// A case-metadata document (`cases/<slug>/<version>.json`): the site-facing slice
/// of one ingested version — its identity, variants (with rendered prompts),
/// checks, reference baselines, reviewer checklist, and scoring domains.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CaseMetadata {
    pub schema_version: u32,
    pub slug: String,
    pub version: String,
    pub name: String,
    /// The case's test type, so the static gallery can scope its catalog tabs to
    /// a single type (E2E / asset-generation / Adversarial / Performance) exactly
    /// as the backend-connected consoles do. Without it the site cannot tell a
    /// case's type and treats every case as end-to-end.
    pub test_type: crate::TestType,
    /// Whether the version is on the **engine manifest format**, which (with the
    /// test type) makes it **validator-rated**: its runs' functional rating and
    /// score are decided by the validators — each review item's `failureCap` and
    /// `domains` below — and reviewers rate only the aesthetic channel. `false` on
    /// every legacy version, whose runs the site scores exactly as before.
    pub engine_format: bool,
    /// The asset shape an asset-generation case produces, so the gallery can
    /// partition asset cases across its 2D (sprite/paint), 3D (voxel/mesh/skinned),
    /// Particle, and Audio tabs. Defaults to `sprite` for every non-asset case
    /// (harmless — the split is only consulted for asset cases).
    pub asset_kind: crate::AssetKind,
    /// A sprite-sheet case's declared frames and named animation sequences, or
    /// `None` for every other kind (only `sprite-sheet` declares a `[sheet]`).
    ///
    /// Exported because the site renders a published **reference sheet** by playing
    /// these sequences — and the motion is most of what such a case is judged on, so
    /// a site with the frames but not the sequences would be showing the least
    /// interesting half. The live console reads the same spec from the resolved
    /// version response; this is the static mirror of it.
    pub sheet: Option<crate::test_case::SheetSpec>,
    pub difficulty: String,
    pub tags: Vec<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    /// This version's own changelog entry (its `changelog.md` body), inlined.
    /// Always present — a changelog is required on every version. The site collects
    /// every published version's entry into one newest-first changelog on the
    /// case's detail page.
    pub changelog: String,
    pub variants: Vec<CaseVariantOut>,
    /// The Test Cabinet runtime packages this case ships into every run, each with
    /// its UI-only description, so the static gallery's Inputs tab can show them.
    /// Empty for a case that declares none.
    pub packages: Vec<CasePackageOut>,
    pub checks: Vec<CaseCheckOut>,
    /// Rendered reference baselines, named by snapshot-relative key. The site
    /// resolves these to absolute URLs to show baselines on the References tab.
    pub references: Vec<CaseReferenceOut>,
    /// The case's committed **baseline** automated-validation media (a debug script's
    /// outputs driven once against the reference implementation), per variant, named by
    /// snapshot-relative key. Case-scoped (a fixed property of the version), so the
    /// static gallery resolves the reviewer's baseline side-by-side from these keyed by
    /// slug/version/variant. Always emitted (possibly empty); the static gallery treats
    /// it as optional so a snapshot written before this field existed still loads.
    pub validation_baselines: Vec<CaseValidationBaselineOut>,
    /// Reviewer checklist items shared by every variant, carrying their point
    /// weights so the site can compute run scores. A variant's own items ride on
    /// [`CaseVariantOut::review_items`].
    pub common_review_items: Vec<CaseReviewItemOut>,
    /// The case's scoring domains, rated independently; the overall rating is the
    /// worst across them.
    pub domains: Vec<CaseDomainOut>,
    /// Known-issue errata recorded for this version after it shipped, so the static
    /// gallery can show the case's Errata tab and flag known issues to reviewers.
    /// Always emitted (possibly empty); the static gallery treats it as optional so a
    /// snapshot written before this field existed still loads.
    #[serde(default)]
    pub errata: Vec<CaseErratumOut>,
}

/// A reference baseline exposed in case metadata. `variant` is `null` for a
/// common reference (shown on every variant) or the variant slug for one scoped
/// to a single variant; `kind` is how it is produced (rendered/image/video); `key`
/// is the snapshot-relative object key of the media.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CaseReferenceOut {
    pub variant: Option<String>,
    pub view: String,
    pub kind: crate::ReferenceKind,
    pub key: String,
}

/// A committed **baseline** validation media file exposed in case metadata — one
/// declared output captured once against the case's reference implementation.
/// `engine` and `variant` name the reference build it was captured from (a variant
/// has one reference implementation per engine, and their captures are not
/// interchangeable); `file` is the flat `<item>__<output>.<ext>` name the gallery
/// requests (`.png`/`.webm`/`.json.gz`); `key` is its snapshot-relative object key,
/// whose bytes are the media as published (a video transcoded to `.mp4`). The static
/// gallery keys its baseline lookup off `engine` + `variant` + `file`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CaseValidationBaselineOut {
    pub engine: String,
    pub variant: String,
    pub file: String,
    pub key: String,
}

/// One variant of a case as the gallery shows it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CaseVariantOut {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    /// The variant's prompt, rendered as a real run receives it, so the public
    /// gallery's Specifications tab shows the instruction the model was handed.
    pub prompt: String,
    /// This variant's complete seeded spec set, in seed order (the common specs
    /// first, then the variant's own), each body rendered for this variant and
    /// inlined so the static gallery shows the exact specs a run of this variant is
    /// seeded with, without a live backend. Because a template spec renders to
    /// different text per variant, every variant carries its own fully-rendered set
    /// here — the spec analogue of [`Self::prompt`] — rather than the case sharing
    /// one common list.
    pub seeded_inputs: Vec<CaseSeededInputOut>,
    /// This variant's prompt and seeded specs re-rendered for each
    /// [engine](crate::engine) the version declares that vendors a
    /// runtime, keyed by engine slug.
    ///
    /// A case's `prompt.hbs` and its `.hbs` specs branch on the selected engine, so
    /// the text a run was handed depends on which runtime its build was written
    /// against. [`Self::prompt`] and [`Self::seeded_inputs`] are the engineless
    /// rendering — what a reader browsing the *case* sees, and exactly what a run on
    /// the `none` engine was handed — and this map carries the rest, so a run's
    /// Inputs surface shows the text that run actually received.
    ///
    /// The engineless engine is deliberately absent: it is already the pair above,
    /// and duplicating every spec body for it would double the document for the many
    /// cases that support nothing else.
    #[serde(default)]
    pub engine_renderings: std::collections::BTreeMap<String, CaseVariantRenderingOut>,
    /// Reviewer checklist items additive to the common ones, with their point
    /// weights, surfaced only when this variant is selected.
    pub review_items: Vec<CaseReviewItemOut>,
    /// Scoring domains additive to the case's common ones, rated only when this
    /// variant is selected. The site rates and scores a run against the common
    /// domains plus its variant's own.
    pub domains: Vec<CaseDomainOut>,
    /// The absolute URLs of this variant's authored **reference implementations** —
    /// the correct, deployed static builds (the case-variant analogue of a run's
    /// `playableBuild`), keyed by the [engine](crate::engine) each was
    /// built for and shown on the static gallery's "Reference" tab, which lets a
    /// reader switch between them. Empty when the variant declares no
    /// `reference_implementation`, or has one that has not been deployed yet. Written
    /// out-of-band by `tcab publish-reference` into the `case_reference_build` table
    /// and folded in here at export — never resolved from the manifest and never
    /// seeded into a run.
    #[serde(default)]
    pub reference_builds: std::collections::BTreeMap<String, String>,
    /// This variant's published **reference sheet** — the asset-generation analogue of
    /// [`Self::reference_builds`], shown on the static gallery's "Reference" tab.
    /// `null` when the variant declares no `reference_implementation`, or has one that
    /// has not been published yet.
    ///
    /// An asset case's reference is a `draw.sh` script whose output is a set of
    /// rendered frames, so what is exported is which frames the snapshot bucket holds.
    /// Only the indices travel: every frame's object key is derivable from the case
    /// triple plus its index (`media/references/<slug>/<version>/<variant>/frames/<index>.png`,
    /// see `crate::asset_reference`), so the site joins them onto its own
    /// snapshot base URL rather than being handed absolute URLs it would have to trust.
    /// Written out-of-band by `tcab publish-reference` into the bucket, reconciled into
    /// the `case_reference_sheet` table at ingest, and folded in here at export — never
    /// resolved from the manifest and never seeded into a run.
    pub reference_sheet: Option<CaseReferenceSheetOut>,
    /// The variant's authored **showcase**, when it declares one: the description
    /// plus the media carousel captured from the reference implementation, shown
    /// on the static gallery's catalog preview and Play tab. `null` when the
    /// variant declares none — and treated as optional by the site, so a snapshot
    /// written before the field existed still loads.
    pub showcase: Option<CaseShowcaseOut>,
    /// The variant's effective starter-workspace files for the **engineless**
    /// rendering (what a run on the `none` engine is seeded with), each naming the
    /// published object its bytes live at, so the static gallery's Inputs tab can
    /// fetch a starter file lazily — the static mirror of the live artifact route.
    /// The per-engine sets ride on [`CaseVariantRenderingOut::workspace_files`].
    /// Empty for a case that seeds no engineless workspace.
    #[serde(default)]
    pub workspace_files: Vec<CaseWorkspaceFileOut>,
}

/// A variant's authored showcase as case metadata exports it — the case-side
/// counterpart of a run's `showcaseMedia[]`, but authored and committed with the
/// version rather than produced by a run.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CaseShowcaseOut {
    /// The showcase description — the authored `showcase.md`, verbatim markdown.
    pub description: String,
    /// The media carousel, in declared order.
    pub media: Vec<CaseShowcaseMediaOut>,
}

/// One entry of an exported case showcase: `file` is the authored file name the
/// UI keys the entry by (kept as authored even when the published bytes are a
/// transcode); `key` is the snapshot-relative object key holding the media as
/// published (a `.webm` clip transcoded to `.mp4`, everything else verbatim).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CaseShowcaseMediaOut {
    /// The media file's authored name in the showcase directory.
    pub file: String,
    /// The short caption for the entry.
    pub name: String,
    /// Whether the file is a still image, a video clip, or a replay recording.
    pub kind: crate::MediaKind,
    /// The snapshot-relative object key of the published bytes.
    pub key: String,
}

/// One starter-workspace file as case metadata exports it: the run-root-relative
/// destination the file is seeded at, and the published object key its bytes
/// live under. Only the addressing is inlined — the bytes are fetched lazily,
/// because a starter project can be large and most readers never open it.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CaseWorkspaceFileOut {
    /// The run-root-relative destination path the file is seeded at.
    pub dest: String,
    /// The snapshot-relative object key of the file's bytes.
    pub key: String,
}

/// One variant's prompt and seeded specs rendered for one engine that vendors a
/// runtime — the per-engine half of [`CaseVariantOut`].
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CaseVariantRenderingOut {
    /// The variant's prompt as a run on this engine receives it.
    pub prompt: String,
    /// The variant's complete seeded spec set in seed order, each body rendered for
    /// this variant on this engine.
    pub seeded_inputs: Vec<CaseSeededInputOut>,
    /// The variant's effective starter-workspace files for this engine, each
    /// naming the published object its bytes live at — the per-engine half of
    /// [`CaseVariantOut::workspace_files`].
    #[serde(default)]
    pub workspace_files: Vec<CaseWorkspaceFileOut>,
}

/// One variant's published reference frames, as exported in case metadata.
///
/// A named object rather than a bare array so the sheet can gain fields (a canvas
/// size, a published-at stamp) without changing the shape the site already reads,
/// matching the wire type `GET /test-cases/{slug}/versions/{version}` returns.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CaseReferenceSheetOut {
    /// The published frame indices, ascending. A single sprite (a case with no
    /// `[sheet]`) publishes exactly one frame, index `0`.
    pub frames: Vec<u32>,
}

/// A seeded spec file exposed in case metadata: the run-workspace path it lands at
/// and its inlined text body. This is the same set the console's Specifications tab
/// fetches per file — the common specs then the variant's own, in seed order — but
/// inlined here so the fully static site needs no backend to show them. Only text
/// specs are carried; a spec whose bytes are missing or not valid UTF-8 is omitted.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CaseSeededInputOut {
    /// The run-workspace-relative path the spec is seeded to (its `dest`).
    pub path: String,
    /// The spec's inlined text body.
    pub text: String,
    /// The seeded file's role (`spec`/`script`), so the static gallery's Inputs
    /// tab can tag it. Presentation only.
    pub kind: crate::SpecKind,
}

/// A runtime package a case ships into its runs, exposed in case metadata for the
/// static gallery's Inputs tab: its npm name and the UI-only description of what it
/// provides. The description is never seeded into a run — it exists only to
/// explain, on the site, what a declared package is for.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CasePackageOut {
    /// The npm package name the case declares in `packages`.
    pub name: String,
    /// The UI-only description of what the package provides.
    pub description: String,
}

/// A reviewer checklist item exposed in case metadata, carrying its point weight
/// and optional scoring domain so the site can compute and break down run scores.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CaseReviewItemOut {
    pub id: String,
    pub title: String,
    pub text: String,
    pub reference: Option<String>,
    pub proof: Option<String>,
    pub sequences: Vec<String>,
    pub frames: Vec<u32>,
    pub weight: u32,
    /// Whether the item is graded on the five-level scale (a game-jam category)
    /// rather than pass/fail. The reviewer and verdict UIs render the graded
    /// control and score `weight × 10` points for it when true.
    pub graded: bool,
    pub domain: Option<String>,
    /// Name-only sub-items this item is graded by, each an independently scored
    /// pass/fail point. Empty for an item graded as a whole.
    pub sub_items: Vec<CaseSubReviewItemOut>,
    /// On a validator-rated version, a whole-item point's **failure cap**: the
    /// highest functional rating its `domains` may reach while its validator
    /// fails. Absent on a legacy version and on a sub-divided item (whose caps sit
    /// on its sub-items).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub failure_cap: Option<crate::review::FailureCap>,
    /// On a validator-rated version, the scoring domains (by id) a failure of this
    /// whole-item point lowers. Empty on a legacy version and on a sub-divided item.
    pub domains: Vec<String>,
    /// The point's automated-validation driver, when it declares one. Absent for a
    /// human-judged point. Carried so a run-scoped surface can drop a point the
    /// run's engine does not carry (see [`CaseReviewValidationOut::engines`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub validation: Option<CaseReviewValidationOut>,
}

/// The part of a checklist point's automated-validation driver the case metadata
/// exposes: which engines the validator decides the point on. The script itself and
/// its media outputs are the driver's business and are not published here.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CaseReviewValidationOut {
    /// The engines this validator decides its point on, by slug, in declared order.
    /// Empty leaves the point on every engine the case supports; a non-empty list is
    /// the subset that carries it, and a run on any other engine has no such point.
    pub engines: Vec<String>,
}

/// A sub-item of a [`CaseReviewItemOut`] exposed in case metadata: one
/// independently graded point within the item. Legacy sub-items carry only id and
/// title; a categories-grammar review item also carries its own prose, weight, and
/// paired reference/proof.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CaseSubReviewItemOut {
    pub id: String,
    pub title: String,
    /// Optional prose for this point (categories grammar); `null` for a legacy
    /// name-only sub-item.
    pub description: Option<String>,
    /// How many points this point is worth. A category's weight is the sum of its
    /// items' weights.
    pub weight: u32,
    /// Optional reference view paired with this point as the expected target.
    pub reference: Option<String>,
    /// Optional proof id paired with this point as the submitted media.
    pub proof: Option<String>,
    /// On a validator-rated version, this point's **failure cap**: the highest
    /// functional rating its `domains` may reach while its validator fails. Absent
    /// on a legacy version.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub failure_cap: Option<crate::review::FailureCap>,
    /// On a validator-rated version, the scoring domains (by id) a failure of this
    /// point lowers. Empty on a legacy version.
    pub domains: Vec<String>,
    /// The point's automated-validation driver, when it declares one (see
    /// [`CaseReviewItemOut::validation`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub validation: Option<CaseReviewValidationOut>,
}

/// A scoring domain exposed in case metadata.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CaseDomainOut {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// A known-issue erratum exposed in case metadata (see
/// [`crate::test_case::Erratum`]).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CaseErratumOut {
    pub id: String,
    pub title: String,
    pub date: Option<String>,
    pub severity: crate::test_case::ErratumSeverity,
    pub affects_scoring: bool,
    /// Whether the linked review point is excluded from scoring for the version.
    pub exclude_from_score: bool,
    pub body: String,
    pub resolved_in: Option<String>,
    /// The variant slug the erratum is scoped to, or `null` for all variants.
    pub variant: Option<String>,
    /// The review verdict id the erratum concerns, or `null` when untied to a point.
    pub review: Option<String>,
}

/// A declared validation check exposed in case metadata.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CaseCheckOut {
    pub view: String,
    pub name: String,
    pub reference_view: String,
}
