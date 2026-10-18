//! A test suite version as the backend stores and serves it: the resolved record one version's read returns, and its identity in a listing.

use serde::{Deserialize, Serialize};

use crate::test_suite::{
    AssetManifest, DemoManifest, ShowcaseManifest, SpecificationManifest, SuiteManifest,
    SuiteTestCaseDefinition, VersionManifest,
};

/// One ingested suite version, resolved: every entity the version folder declares,
/// with the prose it references inlined.
///
/// This is both the stored record and the body
/// `GET /test-suites/{slug}/{version}` serves. The bytes the record points at —
/// the showcase media and the asset files — are copied into the same stored
/// version and served by the byte routes, addressed by the paths carried here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct StoredSuite {
    /// The suite's slug, which is its directory in the suites checkout and what
    /// `suite.toml` declares.
    pub slug: String,
    /// The version folder name, carrying its leading `v`.
    pub version: String,
    /// `suite.toml`: the identity of the suite this version belongs to, read from the
    /// suite folder at ingest, so a renamed suite presents its new name on every
    /// version the next ingest writes.
    pub suite: SuiteManifest,
    /// `version.toml`: the version's identity and the paths of the prose describing
    /// it.
    pub manifest: VersionManifest,
    /// The body of the manifest's `description.md`, inlined — the prose the
    /// manifest names by path, read once at ingest so a reader needs no second
    /// fetch.
    pub description: String,
    /// The body of the manifest's `changelog.md`, inlined for the same reason.
    pub changelog: String,
    /// Every specification the version declares, flattened out of the folder tree
    /// in walk order (a folder before the folders nested inside it).
    pub specifications: Vec<StoredSuiteSpecification>,
    /// The definitions the version offers, in file-stem order.
    pub test_cases: Vec<StoredSuiteDefinition>,
    /// The bundled assets, one per folder under `assets/`.
    pub assets: Vec<StoredSuiteAsset>,
    /// The demonstrations, one per folder under `demos/`.
    pub demos: Vec<StoredSuiteDemo>,
    /// The engines the version ships a reference implementation for, in folder
    /// order. The projects themselves stay in the checkout; the static builds of
    /// them are uploaded separately (see `reference_builds`).
    pub reference_implementations: Vec<String>,
    /// The showcase, when the version holds one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional))]
    pub showcase: Option<StoredSuiteShowcase>,
}

/// One specification of a stored suite version: its manifest, the folder that
/// declares it, and its prose inlined.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct StoredSuiteSpecification {
    /// The specification folder, relative to the version folder. Specification
    /// folders nest freely, so this is what records where a nested one sits.
    pub dir: String,
    /// `specification.toml`: the identity, the seeded path, and the requirements
    /// in declaration order, each carrying the validator modules it claims.
    pub manifest: SpecificationManifest,
    /// The body of the `specification.md` beside it, inlined.
    pub prose: String,
}

/// One test case definition of a stored suite version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct StoredSuiteDefinition {
    /// The definition's slug, which is its file stem under `test-cases/`.
    pub slug: String,
    /// The catalog identity the definition is ingested under — what a reader
    /// follows to reach the test case this definition became.
    pub id: String,
    /// The definition itself.
    pub definition: SuiteTestCaseDefinition,
}

/// One bundled asset of a stored suite version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct StoredSuiteAsset {
    /// The asset folder, relative to the version folder. Its last segment is the
    /// asset's id, and it is the prefix the asset byte route serves its files
    /// under.
    pub dir: String,
    /// `asset.toml`.
    pub manifest: AssetManifest,
}

/// One demonstration of a stored suite version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct StoredSuiteDemo {
    /// The demonstration folder, relative to the version folder.
    pub dir: String,
    /// `demo.toml`.
    pub manifest: DemoManifest,
}

/// The showcase of a stored suite version: the carousel, and the prose above it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct StoredSuiteShowcase {
    /// `showcase/showcase.toml`: the carousel, in declared order.
    pub manifest: ShowcaseManifest,
    /// The body of `showcase/showcase.md`, inlined.
    pub description: String,
    /// The media files the showcase directory holds, by their name inside it —
    /// exactly the path the showcase byte route takes.
    pub media: Vec<String>,
}

impl StoredSuite {
    /// The listing entry for this version: what a suite listing shows without
    /// reading anything else.
    pub fn identity(&self) -> SuiteVersionIdentity {
        SuiteVersionIdentity {
            version: self.version.clone(),
            name: self.suite.name.clone(),
            summary: self.manifest.summary.clone(),
            tags: self.manifest.tags.clone(),
            experimental: self.manifest.experimental,
        }
    }
}

/// One version's identity as a listing presents it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct SuiteVersionIdentity {
    /// The version folder name, carrying its leading `v`.
    pub version: String,
    /// The suite's display name at this version.
    pub name: String,
    /// The one-line abstract a row shows.
    pub summary: String,
    /// Classification tags.
    pub tags: Vec<String>,
    /// Whether the version is still being iterated on. A deployment that has not
    /// opted in is never served one, so a client shows what it is served.
    pub experimental: bool,
}
