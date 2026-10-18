//! The test case catalog (`/test-cases`): the catalog, a case's versions, the fully resolved definition of one version, its specifications, and the test-case groups.

use serde::Serialize;

use super::store::{
    StoredContract, StoredMatch, StoredReplay, StoredSandbox, StoredSimulation,
    StoredSuiteCoordinate,
};
use crate::test_case::{
    AssetDimension, AssetKind, AudioSpec, ErratumSeverity, MaterialSpec, ModelSpec, ParticleSpec,
    SheetSpec, SpecKind, TestType, UiSpec, VoxelSpec,
};

/// The `GET /test-case-groups` envelope: the groups under a wrapping key so the
/// response can grow new fields without breaking readers, like the catalog's.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct TestCaseGroupsResponse {
    /// Every ingested group, in display order (rank ascending then name, with
    /// ranked groups before unranked ones — resolved at ingest).
    pub groups: Vec<TestCaseGroupOut>,
}

/// One test-case group as served — the manifest minus `rank`, which orders the
/// listing server-side and deliberately does not ride the wire.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct TestCaseGroupOut {
    /// The group's stable slug.
    pub slug: String,
    /// Display name, heading the group's home-page leaderboard.
    pub name: String,
    /// Optional one-line description, or null.
    pub summary: Option<String>,
    /// The ordered member test-case/game-jam slugs, by manifest-declared
    /// identity (the slug run records carry).
    pub cases: Vec<String>,
}

// --- Wire shapes (§1.2) -----------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CatalogResponse {
    pub test_cases: Vec<CatalogCase>,
}

/// One entry of the catalog listing: a case, its visible versions, and the
/// display metadata a card renders — read from the latest visible version. This
/// is the *summary* half of the catalog contract; the detail half is
/// [`VersionResponse`], which a client fetches only for a case it opens.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CatalogCase {
    pub slug: String,
    /// Every visible version, oldest first.
    pub versions: Vec<String>,
    /// The case's display name, from its latest visible version.
    pub name: String,
    /// The case's test type, which drives how a listing groups it.
    pub test_type: TestType,
    /// For an asset-generation case, the asset shape it produces — the catalog
    /// partitions its 2D / 3D / Particle / Audio tabs on this.
    pub asset_kind: AssetKind,
    /// Relative difficulty (`easy` / `medium` / `hard`). A game jam is
    /// unclassified and reports whatever its manifest carries.
    pub difficulty: String,
    pub tags: Vec<String>,
    /// The short plain-text abstract a card shows, when the case declares one.
    pub summary: Option<String>,
    /// The case's catalog **showcase preview**, when its latest visible version
    /// has one: the first variant (manifest order) of that version that declares
    /// a showcase, with the media list a card's preview stage loops. `null` when
    /// no variant of the latest version declares one. Only the addressing rides
    /// here (the description lives on the resolved version's variant); each
    /// media file is fetched from
    /// `/test-cases/{slug}/versions/{version}/showcase/{variant}/{file}`.
    pub showcase: Option<CatalogShowcaseOut>,
    /// The [test suite](super::suite_store) coordinate this case was lowered from,
    /// read from its latest visible version. Absent for an authored case, which is
    /// how a listing tells the two apart and groups the suite-derived ones under
    /// the suite that offers them.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub suite: Option<StoredSuiteCoordinate>,
}

/// A case's catalog showcase preview: which version and variant the media
/// belongs to, plus the carousel entries themselves — everything a listing card
/// needs to address the media without resolving the full version.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CatalogShowcaseOut {
    /// The version the showcase was read from (the case's latest visible one).
    pub version: String,
    /// The variant that declares it.
    pub variant: String,
    /// The media carousel, in declared order.
    pub media: Vec<ShowcaseMediaOut>,
}

#[derive(Serialize)]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct VersionsResponse {
    pub slug: String,
    pub versions: Vec<String>,
}

/// The resolved, fully-rendered definition of one test-case version: the body of
/// `GET /test-cases/{slug}/{version}`. Everything a reporter needs to present the
/// case — its metadata, the per-test-type contract/sandbox shapes, the rendered
/// prompt and seeded specs, and every variant, reference, proof, check, and
/// reviewer item — resolved as a real run receives them.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct VersionResponse {
    pub slug: String,
    pub version: String,
    pub name: String,
    pub difficulty: String,
    pub tags: Vec<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    /// The version's own changelog entry (its `changelog.md` body), inlined.
    /// Always present — a changelog is required on every version. The console
    /// aggregates every version's entry into the case's changelog tab.
    pub changelog: String,
    pub max_runtime_seconds: u64,
    pub test_type: TestType,
    /// Whether the version is on the **engine manifest format**, which (with the
    /// test type) makes it **validator-rated**: a run's functional rating and score
    /// are decided by its validators — every review point declares its `failureCap`
    /// and `domains` — and a reviewer rates only the aesthetic channel. `false` on
    /// every legacy version, whose runs are reviewed exactly as before.
    pub engine_format: bool,
    /// The engines a run of this version may select. Never empty — a version that
    /// declares none supports the engineless run.
    pub engines: Vec<EngineOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<BuildOut>,
    /// The case's TypeScript toolchain commands, when it declares a `[toolchain]`
    /// table. Absent for every version that predates it, which is what leaves those
    /// versions unchecked and ungated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toolchain: Option<ToolchainOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canvas: Option<CanvasOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<ToolOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract: Option<StoredContract>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<StoredSandbox>,
    /// A performance case's held-out scored set — each case's `input` scenario and
    /// `expected` oracle state, by store-relative key. Empty (and omitted) for
    /// every other type. The runner fetches these like assets and the performance
    /// validator scores the engine against them; they are never seeded into a run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cases: Vec<CaseOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub simulation: Option<StoredSimulation>,
    #[serde(rename = "match", skip_serializing_if = "Option::is_none")]
    pub r#match: Option<StoredMatch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replay: Option<StoredReplay>,
    pub asset_kind: AssetKind,
    /// Which full-stack run image the version's runs execute in. Always serialized
    /// (never skipped) because the runner deserializes this body into the resolved
    /// version it resolves the run image from.
    pub asset_dimension: AssetDimension,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet: Option<SheetSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voxel: Option<VoxelSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui: Option<UiSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub material: Option<MaterialSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub particle: Option<ParticleSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<AudioSpec>,
    /// The audio packs a run of this version is staged with, in declaration order.
    /// The driver reads it to stage the run container's palette, so it is served for
    /// every test type, not just the audio kinds — always present, empty for a
    /// version that declares none, like the other list-valued keys beside it.
    pub audio_packs: Vec<String>,
    pub prompt_template: String,
    pub common_specs: Vec<SpecOut>,
    /// The Test Cabinet runtime packages this case ships into every run, each with
    /// a UI-only description. Shown on the console's Inputs tab; empty for a case
    /// that declares none.
    pub packages: Vec<PackageOut>,
    pub workspace: std::collections::BTreeMap<String, Vec<WorkspaceOut>>,
    pub init: Option<String>,
    pub assets: Vec<AssetOut>,
    pub variants: Vec<VariantOut>,
    pub common_references: Vec<ReferenceOut>,
    pub common_proofs: Vec<ProofOut>,
    pub checks: Vec<CheckOut>,
    pub common_review_items: Vec<ReviewItemOut>,
    pub domains: Vec<DomainOut>,
    /// The case's `[instrumentation]` debug-API handle, when it mandates one for
    /// automated validation. Reporter-side (never seeded into a run): served so the
    /// driver's validator knows which `window` handle to drive. Absent for a case with
    /// no auto-validated items.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instrumentation: Option<InstrumentationOut>,
    /// Known-issue errata recorded for this version after it shipped. Site-facing:
    /// shown on the case's Errata tab and, where relevant, to reviewers scoring a
    /// run of the version. Empty when the version has none.
    pub errata: Vec<ErratumOut>,
    /// The [test suite](super::suite_store) coordinate this version was lowered
    /// from: the suite, the suite version, and the definition inside it. Absent for
    /// an authored case, so a client that finds one here knows the version is
    /// served out of a suite and can reach the suite's own read endpoints with it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suite: Option<StoredSuiteCoordinate>,
}

#[derive(Serialize)]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct BuildOut {
    pub install: String,
    pub build: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
}

/// The case's declared TypeScript toolchain, served so a backend-driven run can run
/// it: the gating `typecheck` and the recorded `lint`/`format`/`test`.
#[derive(Serialize)]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ToolchainOut {
    pub typecheck: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CanvasOut {
    pub width: u32,
    pub height: u32,
    pub background: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ToolOut {
    pub binary: String,
    pub preview: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct OutputOut {
    pub actions: String,
}

#[derive(Serialize)]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct SpecOut {
    pub source: String,
    pub dest: String,
    pub template: bool,
    /// The seeded file's role (`spec`/`script`), so the console's Inputs tab can
    /// tag it. Presentation only.
    pub kind: SpecKind,
}

/// A variant's seeded spec set with every body rendered for that variant, the
/// response of `variant_specs`. Unlike [`SpecOut`] (a descriptor pointing at the
/// raw artifact key) this carries the finished, handlebars-free `body` the reader
/// shows.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct SpecsResponse {
    pub slug: String,
    pub version: String,
    pub variant: String,
    /// The version's site-facing description (never seeded), carried so the Inputs
    /// tab has the same context the resolved version does. `null` when none.
    pub description: Option<String>,
    pub specs: Vec<SpecDocumentOut>,
}

/// One seeded spec with its body already rendered for the selected variant.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct SpecDocumentOut {
    /// The run-workspace-relative path the spec seeds to (its `dest`).
    pub dest: String,
    /// The spec's body, rendered for this variant — a template spec's conditionals
    /// resolved, a plain spec verbatim.
    pub body: String,
    /// The seeded file's role (`spec`/`script`), for the Inputs-tab tag.
    pub kind: SpecKind,
}

/// One [engine](crate::engine) a run of this version may select, with
/// the version range the case accepts it at. A launcher offers exactly this set,
/// and the runner holds a run's selection against it before any container work.
///
/// Always the table spelling, so one shape answers both readers: a case that pins
/// nothing carries the slug alone and omits the bounds.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct EngineOut {
    pub slug: String,
    /// The lowest engine version a run may select, inclusive. Absent when the case
    /// pinned no floor.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub min_version: Option<String>,
    /// The version support stops at, exclusive. Absent when the range is unbounded
    /// above, which is the default.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub max_version: Option<String>,
}

/// A runtime package a case ships into its runs, exposed for the console's Inputs
/// tab: its npm name and the UI-only description of what it provides (never seeded
/// into a run).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct PackageOut {
    pub name: String,
    pub description: String,
}

#[derive(Serialize)]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct AssetOut {
    pub source: String,
    pub dest: String,
}

/// One held-out scored case of a performance case: the store-relative keys of the
/// `input` scenario fed to the engine and the `expected` oracle state its output
/// is checked against. Mirrors core's wire `CaseBody`, so a resolved version
/// round-trips its scored set through to the runner's `materialize_version`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CaseOut {
    pub input: String,
    pub expected: String,
    /// The case's resolved run ceiling (`fuel_limit * fuel_runway`), carried so the
    /// driver grades against the same ceiling the manifest declared.
    pub fuel_ceiling: u64,
    /// Which phase the case belongs to — a correctness pre-flight `smoke` test or a
    /// scored `stress` case — carried so the driver runs the smoke gate before the
    /// stress cases. Defaults to `stress` for versions ingested before smoke tests.
    #[serde(default)]
    pub kind: crate::validation::PerformanceCaseKind,
}

#[derive(Serialize)]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct WorkspaceOut {
    pub source: String,
    pub dest: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct VariantOut {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    /// The variant's prompt, rendered as a real run receives it.
    pub prompt: String,
    pub specs: Vec<SpecOut>,
    pub workspace: Option<std::collections::BTreeMap<String, Vec<WorkspaceOut>>>,
    pub references: Vec<ReferenceOut>,
    pub proofs: Vec<ProofOut>,
    pub review_items: Vec<ReviewItemOut>,
    /// The variant's own scoring domains, on top of the case's common
    /// [`VersionResponse::domains`]. Rated only when this variant is selected.
    pub domains: Vec<DomainOut>,
    /// The variant's bounding-volume override, when it declares its own `[voxel]`
    /// (the size axis behind a case's half/base/double variants). `None` inherits
    /// the case's common [`VersionResponse::voxel`].
    pub voxel: Option<VoxelSpec>,
    /// The absolute URLs of this variant's authored **reference implementations** —
    /// the correct, deployed static builds (the case-variant analogue of a run's
    /// `playableBuild`), keyed by the [engine](crate::engine) each was
    /// built for and served on the console's "Reference" tab, which lets a reader
    /// switch between them. Empty when the variant declares no
    /// `reference_implementation`, or has one that has not been deployed yet.
    /// Written out-of-band by `tcab publish-reference` and read from the
    /// `case_reference_build` table — never resolved from the manifest and never
    /// seeded into a run. A suite-defined version carries instead the builds
    /// uploaded for its suite version, each URL relative to the backend.
    #[serde(default)]
    pub reference_builds: std::collections::BTreeMap<String, String>,
    /// This variant's published **reference sheet** — the asset-generation analogue of
    /// [`Self::reference_builds`]. An asset case's reference is a `draw.sh` script, not
    /// a site, so what is recorded is which of its rendered frames were published to
    /// the public snapshot bucket. `None` when the variant declares no
    /// `reference_implementation`, or has one that has not been published yet.
    ///
    /// Written out-of-band by `tcab publish-reference` (which uploads the frames) and
    /// read from the `case_reference_sheet` table, which the backend reconciles by
    /// listing the bucket at ingest — never resolved from the manifest and never
    /// seeded into a run. Only the indices travel: each frame's URL is derivable from
    /// the case triple and its index (see `crate::asset_reference`), so
    /// the client builds them against the `snapshotUrl` from `GET /config`.
    pub reference_sheet: Option<ReferenceSheetOut>,
    /// The variant's authored **showcase**, when it declares one: the description
    /// plus the media carousel captured from the reference implementation, shown
    /// on the case's Play tab. `null` when the variant declares none. Each media
    /// file is fetched from
    /// `/test-cases/{slug}/versions/{version}/showcase/{variant}/{file}`.
    pub showcase: Option<ShowcaseOut>,
}

/// A variant's authored showcase as the resolved version serves it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ShowcaseOut {
    /// The showcase description — the authored `showcase.md`, verbatim markdown.
    pub description: String,
    /// The media carousel, in declared order.
    pub media: Vec<ShowcaseMediaOut>,
}

/// One entry of a served showcase carousel: the file name the showcase route
/// addresses the bytes by, its caption, and the kind of media it holds.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ShowcaseMediaOut {
    /// The media file's name in the showcase directory (a plain file name).
    pub file: String,
    /// The short caption for the entry.
    pub name: String,
    /// Whether the file is a still image, a video clip, or a replay recording.
    pub kind: crate::MediaKind,
}

/// One variant's published reference frames.
///
/// A struct rather than a bare `Vec<u32>` because the frame indices are not the whole
/// story a reference sheet will want to tell — a sheet may later carry, say, the
/// canvas size or a published-at stamp — and a named object can grow those fields
/// without a breaking change to the wire shape.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ReferenceSheetOut {
    /// The published frame indices, ascending. A single sprite (a case with no
    /// `[sheet]`) publishes exactly one frame, index `0`.
    pub frames: Vec<u32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ReviewItemOut {
    pub id: String,
    pub title: String,
    pub text: String,
    pub reference: Option<String>,
    pub proof: Option<String>,
    pub weight: u32,
    /// Whether the item is graded on the five-level scale (a game-jam category)
    /// rather than pass/fail. False for every other test type. The reviewer UI keys
    /// its editor control (emoji grade scale vs. pass/fail) off this flag.
    #[serde(default)]
    pub graded: bool,
    pub domain: Option<String>,
    /// Name-only sub-items the reviewer grades this item by, each an
    /// independently scored pass/fail point. Empty for an item graded as a whole.
    pub sub_items: Vec<SubReviewItemOut>,
    /// The item's automated-validation driver, when it opts into auto-validation:
    /// the reporter-side debug script (by its version-folder-relative key) and its
    /// declared media outputs. Served so the driver's validator can fetch, materialize,
    /// and run it against the build's debug API. Absent for a human-judged item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<ReviewValidationOut>,
    /// On a validator-rated version, a whole-item point's **failure cap**: the
    /// highest functional rating its `domains` may reach while its validator fails.
    /// Absent on a legacy version and on a sub-divided item (whose caps sit on its
    /// sub-items).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub failure_cap: Option<crate::review::FailureCap>,
    /// On a validator-rated version, the scoring domains (by id) a failure of this
    /// whole-item point lowers. Empty on a legacy version and on a sub-divided item.
    pub domains: Vec<String>,
}

/// The `[instrumentation]` handle in the §1.2 wire shape.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct InstrumentationOut {
    /// The `window` property name the debug API is installed on (no `window.` prefix).
    pub handle: String,
    /// The case's fixed simulation rate in whole ticks per second, when it declares
    /// one — what lets the validation runtime relate exact stepping to real time.
    /// Omitted for a real-time-clocked case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tick_hz: Option<u32>,
}

/// A review item's automated-validation driver in the §1.2 wire shape: the
/// version-folder-relative debug `script` key the driver fetches (like an asset) and
/// its declared media `outputs`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ReviewValidationOut {
    pub script: String,
    /// Whether `script` names a suite inside a validator project — one per engine,
    /// and it ships in the project of every engine
    /// [`ReviewValidationOut::engines`] covers — rather than one file under the
    /// version folder.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub per_engine: bool,
    /// The engines this validator decides its point on, by slug, in declared order.
    /// Omitted when the case declares no restriction, which leaves the point active
    /// on every engine the case supports.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub engines: Vec<String>,
    pub outputs: Vec<ReviewOutputOut>,
}

/// One media output of a [`ReviewValidationOut`] script in the §1.2 wire shape.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ReviewOutputOut {
    pub id: String,
    pub name: String,
    pub kind: crate::MediaKind,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct SubReviewItemOut {
    pub id: String,
    pub title: String,
    /// Optional prose for this point (categories grammar); absent for a legacy
    /// name-only sub-item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// How many points this point is worth. A category's weight is the sum of its
    /// items' weights.
    pub weight: u32,
    /// Optional expected reference paired with this point.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// Optional submitted proof paired with this point.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<String>,
    /// The sub-item's automated-validation driver, when it opts into auto-validation.
    /// Same shape as [`ReviewItemOut::validation`] but keyed to this sub-item's verdict.
    /// Absent for a human-judged sub-item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<ReviewValidationOut>,
    /// On a validator-rated version, this point's **failure cap**: the highest
    /// functional rating its `domains` may reach while its validator fails. Absent on
    /// a legacy version.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub failure_cap: Option<crate::review::FailureCap>,
    /// On a validator-rated version, the scoring domains (by id) a failure of this
    /// point lowers. Empty on a legacy version.
    pub domains: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct DomainOut {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// A known-issue erratum in the §1.2 wire shape (see
/// [`crate::test_case::Erratum`]).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ErratumOut {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    pub severity: ErratumSeverity,
    pub affects_scoring: bool,
    /// Whether the linked review point is excluded from scoring for the version.
    pub exclude_from_score: bool,
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_in: Option<String>,
    /// The variant slug the erratum is scoped to, or absent for all variants.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    /// The review verdict id the erratum concerns, or absent when untied to a point.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ReferenceOut {
    pub view: String,
    /// How the reference is produced (`rendered`, `image`, or `video`).
    pub kind: crate::ReferenceKind,
    /// The URL path the reference media is served under.
    pub media_url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ProofOut {
    pub id: String,
    pub name: String,
    pub kind: crate::MediaKind,
    pub dest: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct CheckOut {
    pub view: String,
    pub name: String,
    pub reference_view: String,
    pub actions: Vec<crate::test_case::CheckAction>,
}
