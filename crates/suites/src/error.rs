//! The suite runtime's error type.
//!
//! Each variant is one of `test_cabinet_core::Error`'s, with the same fields and the
//! same message, and core converts one into the other variant for variant
//! (`impl From<test_cabinet_suites::Error> for test_cabinet_core::Error`). A failure
//! raised here therefore reads exactly as it did when this code was core's, whichever
//! crate's error a caller ends up holding.

use std::io;

use thiserror::Error;

/// Convenience result alias used throughout the crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors the suite runtime raises.
///
/// The message grammar is core's: a clause naming the stage that was running, with
/// the detail from the layer below it behind a colon. Not `#[non_exhaustive]`: core
/// matches it exhaustively, so a variant added here is mapped there before it builds.
#[derive(Debug, Error)]
pub enum Error {
    /// A requested test case slug was not present in the catalog.
    #[error("test case `{slug}` not found in the catalog")]
    TestCaseNotFound {
        /// The slug that could not be resolved.
        slug: String,
    },

    /// Two catalog folders declared the same `slug`, so the identity is
    /// ambiguous. A slug is a case's stable identity and must be unique across the
    /// whole catalog.
    #[error("slug `{slug}` is declared by two test-case folders (`{folder_a}`, `{folder_b}`)")]
    DuplicateSlug {
        /// The slug declared by two folders.
        slug: String,
        /// One folder declaring it.
        folder_a: String,
        /// The other folder declaring it.
        folder_b: String,
    },

    /// A requested test case version did not exist for an existing slug.
    #[error("version `{version}` of test case `{slug}` not found")]
    TestCaseVersionNotFound {
        /// The test case slug.
        slug: String,
        /// The version that could not be resolved.
        version: String,
    },

    /// A test case version was structurally invalid (for example, missing a
    /// specification).
    #[error("test case `{slug}@{version}` is invalid: {detail}")]
    InvalidTestCase {
        /// The test case slug.
        slug: String,
        /// The test case version.
        version: String,
        /// Human-readable explanation of what was wrong.
        detail: String,
    },

    /// A requested variant did not exist for a resolved test case version.
    #[error("variant `{variant}` of test case `{slug}@{version}` not found")]
    VariantNotFound {
        /// The test case slug.
        slug: String,
        /// The test case version.
        version: String,
        /// The variant slug that could not be resolved.
        variant: String,
    },

    /// A [test suite](crate::test_suite) version could not be resolved into a
    /// runnable test case: a file it declares is missing, disagrees with the folder
    /// that holds it, or carries something resolution cannot lower.
    ///
    /// The suite counterpart of [`Self::InvalidTestCase`]. A suite is many
    /// files, so this names the one that caused the failure alongside the suite
    /// coordinate — "the suite is invalid" is not something an author can act on.
    #[error("test suite `{suite}@{version}` is invalid: {file}: {detail}")]
    InvalidTestSuite {
        /// The suite slug.
        suite: String,
        /// The suite version, as the version folder names it (carrying its
        /// leading `v`).
        version: String,
        /// The file that caused the failure, relative to the version folder.
        file: String,
        /// Human-readable explanation of what was wrong.
        detail: String,
    },

    /// Rendering a prompt template failed.
    #[error("rendering the prompt for `{slug}@{version}`: {detail}")]
    PromptRender {
        /// The test case (or suite) slug.
        slug: String,
        /// The test case (or suite) version.
        version: String,
        /// Detail describing the failure.
        detail: String,
    },

    /// Rendering a test case's `.hbs` spec template failed during seeding.
    #[error("rendering spec `{spec}` for `{slug}@{version}`: {detail}")]
    SpecRender {
        /// The test case slug.
        slug: String,
        /// The test case version.
        version: String,
        /// The spec source path that failed to render.
        spec: String,
        /// Detail describing the failure.
        detail: String,
    },

    /// A [test-case group](crate::test_case_group) could not be loaded: the
    /// `test-case-groups/` catalogue was unreadable, or a group's manifest was
    /// malformed or broke an invariant (a slug disagreeing with its directory,
    /// an empty member list, a duplicate member). The detail names the group
    /// directory and what was wrong.
    #[error("loading the test-case group: {0}")]
    TestCaseGroup(String),

    /// An [engine](crate::engine) could not be resolved: the requested slug is
    /// not one this build carries. The detail names the slug and every built-in,
    /// so a typo on `--engine` is fixable from the message alone.
    #[error("resolving the engine: {0}")]
    Engine(String),

    /// Reading what the run repository is seeded from failed: a staged package's
    /// manifest is unreadable or declares no version.
    #[error("seeding the run repository: {0}")]
    Seeding(String),

    /// An underlying I/O operation failed.
    #[error("host I/O: {0}")]
    Io(#[from] io::Error),
}
