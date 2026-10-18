//! The error body every backend endpoint answers a failure with.

use serde::Serialize;

/// The JSON body of an error response: `{ "error": { "code", "message" } }`.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ErrorEnvelope {
    pub error: ErrorBody,
}

/// The `error` member of an [`ErrorEnvelope`]: a stable machine-readable code and
/// a human-readable message.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}
