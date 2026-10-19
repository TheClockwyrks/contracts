//! The run container's layout as both sides of it name it: the workspace a run is
//! built in, the folders seeding places beside the submission, which stream a
//! captured output line came from, and the message of the commit seeding makes.
//!
//! The container runtime, seeding and artifact collection that use these are the
//! driver's.

use serde::{Deserialize, Serialize};

/// The directory the seeded run repository is copied into inside the run
/// container, and the working directory the harness builds in. Spec `dest` paths
/// are relative to this, so the rendered prompt can point the model at absolute
/// in-container paths.
pub const WORKSPACE_DIR: &str = "/work";

/// The workspace-relative folder a game-jam run's *previous entries* are seeded
/// into: the gameplay READMEs of earlier runs of the same jam with the same harness
/// and model. It is reference material for building something distinct, not part of
/// the submission, so seeding git-ignores it (see `test_cabinet_driver::seeding`). Both the
/// seeder (which writes it) and the prompt (which points the model at it) name it
/// through this constant so they never drift.
pub const GAME_JAM_PRIOR_ENTRIES_DIR: &str = "previous-entries";

/// The workspace-relative folder the selected [engine](crate::engine)'s own
/// documentation is seeded into, copied out of the engine package's declared
/// `docs` directory.
///
/// It sits at the run root rather than under `.vendor/` because it is material the
/// model is *meant to read*: the engine documents itself from its own package, so
/// a case's specs never restate it and the rendered prompt points at
/// `/work/engine` instead. Both the seeder (which writes it) and the prompt (which
/// points the model at it) name it through this constant so they never drift —
/// the same reason [`GAME_JAM_PRIOR_ENTRIES_DIR`] exists. Empty of meaning for a
/// run with no engine, which seeds nothing here.
///
/// The engine's manifest names the documentation directory *inside its package*
/// (`docs`, for `@clockwyrks/simple-2d`), but seeding flattens it to this one fixed
/// place, so every engine's documentation is found at the same path and a prompt
/// template never has to know the package's internal layout: the `{{engine.docs}}`
/// path a template sees depends only on *whether* the engine declares
/// documentation, never on what it called the directory.
pub const ENGINE_DOCS_DIR: &str = "engine";

/// Which standard stream a captured output line came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OutputStream {
    /// Standard output.
    Stdout,
    /// Standard error.
    Stderr,
}

/// The message on the commit the seeder makes after laying the workspace down.
///
/// Exported because it is a **contract with the code analyzer**, not an implementation
/// detail. When a run record carries no seed commit, the analyzer's authored-set ladder
/// falls back to the tree's single root commit — and this message is the *only* thing
/// distinguishing a seed root from one the model created with its own `git init`. Taking
/// the root unconditionally would fail in the worst direction: the seeded set would
/// swallow the model's own work and the run would report near-zero authored code, stamped
/// as an exact measurement. Changing this string retires that fallback for every tree
/// seeded afterwards, so it lives in one place where both sides can see it: the
/// driver's seeder writes it and the code analyzer reads it.
pub const SEED_COMMIT_MESSAGE: &str = "Seed test case";

/// The message on the commit the driver makes at run end, over everything the model
/// left uncommitted on top of the history the run holds, before it writes the run's
/// bundle for the repos service.
///
/// Fixed, so every bundle names its implementation commit the same way. It must differ
/// from [`SEED_COMMIT_MESSAGE`]: the code analyzer tells a seed root by that message
/// alone, and a run whose model never committed has this commit as the only one above
/// the seed.
pub const IMPLEMENTATION_COMMIT_MESSAGE: &str = "Implement test case";

#[cfg(test)]
#[path = "execution.test.rs"]
mod tests;
