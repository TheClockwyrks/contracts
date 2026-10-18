//! The backend's API wire types: what the console, the gallery and The Spec Cabinet
//! read from the backend and its published snapshot, and the documents it stores
//! and publishes.
//!
//! `api-codegen` emits them as `@clockwyrks/backend-api` and as the JSON Schemas under
//! `schema/backend-api/`, `schema/coverage/`, `schema/jobs-api/` and `schema/snapshot/`.
//! The backend serves them; the rules that fill them are the backend's.

pub mod comparisons;
pub mod config;
pub mod coverage;
pub mod error;
pub mod gg;
pub mod jobs;
pub mod ladders;
pub mod models;
pub mod probe;
pub mod snapshot;
pub mod stats;
pub mod store;
pub mod suite_store;
pub mod test_cases;
pub mod test_suites;
