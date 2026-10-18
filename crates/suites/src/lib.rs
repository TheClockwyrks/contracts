//! # test-cabinet-suites
//!
//! The test-suite runtime and the manifest loaders: what reads a `test-cases/` or
//! `test-suites/` checkout and turns an authored case or an offered definition into
//! the [`TestCaseVersion`](test_case::TestCaseVersion) a run executes, on the
//! filesystem, in a process or in a browser. See
//! `docs/components/contracts/overview.md`.
//!
//! - [`test_case`](mod@test_case), [`test_case_group`] and [`test_suite`]: the manifest loaders,
//!   with the validators and catalogs over them. [`color`] holds the background
//!   grammars a case manifest is checked with. The suite catalog lowers a
//!   definition onto a test case version, builds previews, and runs a suite's
//!   validator project.
//! - [`resolver`]: the one place an identity and a version become a resolved case,
//!   whether authored or suite-defined.
//! - [`prompt`]: an authored case's prompt and `.hbs` specs, and a suite
//!   definition's prompt.
//! - [`engine`]: the engine catalog over a table of manifests it is handed, and the
//!   version each engine's staged package reports.
//! - [`vitest_validator`], [`validator`] and [`browser`]: a case's or a suite's vitest
//!   project, the verdict units a checklist flattens into, and the static server and
//!   browser driver the validators use. A browser call takes the function naming
//!   the current span's `traceparent` from its caller; this crate links no
//!   OpenTelemetry.
//! - [`content_digest`] and [`content_labels`]: what an ingest keys a version by.
//!
//! The shapes these read and write are `test_cabinet_contracts`'. The crate names no
//! engine table: everything that checks a declared engine slug takes an
//! [`EngineCatalog`](engine::EngineCatalog), and a caller that needs the built-in
//! one takes `test_cabinet_engines::built_in_catalog()` from the engines
//! repository.

pub mod browser;
pub mod color;
pub mod content_digest;
pub mod content_labels;
pub mod engine;
pub mod error;
pub mod execution;
pub mod fs;
pub mod prompt;
pub mod resolver;
pub mod seeding;
pub mod test_case;
pub mod test_case_group;
pub mod test_suite;
pub mod validator;
pub mod vitest_validator;

/// What a test driving a real browser needs, and the `TCAB_REQUIRE_BROWSER`
/// rule for when it is missing. Exposed to other crates' tests by the
/// `test-support` feature.
#[cfg(any(test, feature = "test-support"))]
pub mod test_browser;

#[cfg(test)]
mod test_engines;

pub use error::{Error, Result};

// The contract modules the moved code names by their `crate::` paths, as it did in
// the run engine, where they are re-exported at the same names.
use test_cabinet_contracts::{toolchain, validation};
