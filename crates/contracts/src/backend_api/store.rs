//! Stored shapes the backend's definition store serves: the coordinate of a suite-defined test case and the adversarial contract, sandbox, simulation, match and replay of an adversarial case.

use serde::{Deserialize, Serialize};

/// Where a suite-defined test-case version came from: the
/// [suite](super::suite_store::StoredSuite) that offers it, the suite version that
/// declares it, and the definition file it was lowered from.
///
/// A version carrying one is served out of the same key space an authored version
/// is, so this is the only thing that distinguishes the two on the wire — which is
/// what lets a console group suite-derived cases under their suite.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct StoredSuiteCoordinate {
    /// The suite's slug, which is its directory in the suites checkout.
    pub suite: String,
    /// The suite version folder name, carrying its leading `v`.
    pub suite_version: String,
    /// The definition's slug, which is its file stem under `test-cases/`.
    pub definition: String,
}

/// The submission contract of an adversarial or performance case persisted in a
/// `StoredManifest`. Path fields are forward-slashed run-workspace-relative keys.
/// Adversarial carries the per-tick `world`/`action` schemas; performance carries
/// the per-scenario `input`/`output` schemas. The pairs are `Option` so the two
/// types coexist on one stored shape (only one pair is set).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct StoredContract {
    /// The exported function invoked once per tick (adversarial) or once per
    /// scenario (performance).
    pub entry: String,
    /// Adversarial only: the seeded `world` observation schema path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world: Option<String>,
    /// Adversarial only: the seeded `action` schema path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    /// Performance only: the seeded `input` (scenario) schema path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    /// Performance only: the seeded `output` (state) schema path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

/// The sandbox limits of an adversarial or performance case persisted in a
/// `StoredManifest`. Adversarial meters per tick (`fuel_per_tick`); performance
/// meters per scenario (`fuel_limit`). Both fuel fields are `Option` so the two
/// types coexist (only one is set).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct StoredSandbox {
    /// Adversarial only: the wasmtime fuel ceiling for a single tick.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fuel_per_tick: Option<u64>,
    /// Performance only: the wasmtime fuel ceiling for a whole scenario.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fuel_limit: Option<u64>,
    /// The linear-memory cap in bytes.
    pub max_memory_bytes: u64,
}

/// The simulation-loop configuration of an adversarial case persisted in a
/// `StoredManifest`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct StoredSimulation {
    /// The fixed, faked delta handed to the game logic each tick (milliseconds).
    pub timestep_ms: u32,
    /// Hard cap on match length.
    pub max_ticks: u32,
}

/// How an adversarial case pairs implementations into matches, persisted in a
/// `StoredManifest`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct StoredMatch {
    /// Controllers per match.
    pub participants: u32,
    /// How the field is paired (for example `round-robin`).
    pub structure: String,
    /// Matches played per pairing.
    pub rounds: u32,
}

/// How an adversarial case renders a recorded match for browser playback,
/// persisted in a `StoredManifest`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct StoredReplay {
    /// The run-workspace-relative path the renderer is seeded to, forward-slashed.
    pub renderer: String,
}
