//! The request body of a harness comparison (`/comparisons`).

use serde::Deserialize;

use crate::comparison::ComparisonConfig;

/// The create/update body for a comparison (the server assigns the id and
/// timestamps, and owns the published state).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ComparisonInput {
    /// The operator-chosen display name.
    pub name: String,
    /// A one-line note on what is being compared and why.
    #[serde(default)]
    pub description: String,
    /// The controls, varied dimension, and arms to save.
    pub config: ComparisonConfig,
}
