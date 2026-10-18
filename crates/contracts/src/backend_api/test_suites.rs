//! The ingested test suites (`/test-suites`): the listing, one version's read, and a reference build uploaded for it.

use serde::{Deserialize, Serialize};

use super::suite_store::{StoredSuite, SuiteVersionIdentity};

// --- Wire shapes ------------------------------------------------------------

/// `PUT /suites/{slug}/versions/{version}/reference-builds/{engine}` — the stored
/// build.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ReferenceBuildOut {
    /// The engine the build was uploaded for.
    pub engine: String,
    /// The URL the build is played at, relative to the backend.
    pub url: String,
}

// --- Wire shapes ------------------------------------------------------------

/// `GET /test-suites` — every ingested suite the deployment offers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct SuitesResponse {
    /// The suites, by slug.
    pub test_suites: Vec<SuiteOut>,
}

/// `GET /test-suites/{slug}/{version}` — one suite version's resolved record, with
/// the reference builds uploaded for it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct SuiteVersionResponse {
    /// The stored record, served unmapped.
    #[serde(flatten)]
    pub suite: StoredSuite,
    /// The engines with an uploaded reference build, each with the URL its build is
    /// played at, relative to the backend. An engine the version ships a reference
    /// implementation for but holds no upload of is absent.
    pub reference_builds: std::collections::BTreeMap<String, String>,
    /// The content digest the stored version was ingested from, as lowercase hex
    /// SHA-256, so a client holding the checkout can tell whether the store still
    /// matches it. Absent for a record written before digests were recorded.
    pub digest: Option<String>,
}

/// One suite in the listing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct SuiteOut {
    /// The suite's slug, which is its directory in the suites checkout.
    pub slug: String,
    /// Every visible version, oldest first — so the newest, which is the one a row
    /// describes, is last.
    pub versions: Vec<SuiteVersionIdentity>,
}
