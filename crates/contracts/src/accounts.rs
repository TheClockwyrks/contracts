//! The wire shapes of the standalone auth service: an account and the register and
//! log-in requests and the token response.
//!
//! See `apps/docs/src/content/docs/components/auth/overview.md`. The auth service is
//! the only holder of the account store; the backend verifies each request's bearer
//! token against it and attributes the resolved [`Account`] to the review. The HTTP
//! client over these shapes is its caller's.

use serde::{Deserialize, Serialize};

/// A user account as the auth service represents it on the wire: a stable id, the
/// login `username`, and the human-facing `display_name` shown beside a review.
/// The password hash never leaves the service, so it is not part of this shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct Account {
    /// The account's stable id (a UUID), the key a review is attributed to.
    pub id: String,
    /// The unique login handle.
    pub username: String,
    /// The human-facing name shown beside the account's reviews.
    pub display_name: String,
    /// RFC 3339 of when the account's profile picture was last set, or `None` when
    /// the account has no picture. Doubles as a "has picture" flag and a cache-bust
    /// version: the picture bytes themselves are served separately
    /// (`GET /auth/users/{id}/picture`), never carried inline on this shape.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "contract", ts(optional = nullable))]
    pub picture_updated_at: Option<String>,
}

/// A request to create a new account (`POST /auth/register`). Self-registration
/// is open on the private network: anyone who can reach the auth service may
/// create an account.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct RegisterRequest {
    /// The desired unique login handle.
    pub username: String,
    /// The plaintext password, hashed with Argon2id by the service and never
    /// stored or logged in the clear.
    pub password: String,
    /// The human-facing display name.
    pub display_name: String,
}

/// A request to exchange credentials for a bearer token (`POST /auth/login`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct LoginRequest {
    /// The account's login handle.
    pub username: String,
    /// The account's plaintext password, verified against the stored hash.
    pub password: String,
}

/// The auth service's response to a successful register or login: the freshly
/// minted bearer `token` (shown once; the service stores only its hash) and the
/// resolved [`Account`]. Clients persist the token and send it as
/// `Authorization: Bearer <token>` on every mutating request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "contract",
    derive(ts_rs::TS, schemars::JsonSchema),
    ts(rename = "AuthResult"),
    schemars(rename = "AuthResult")
)]
pub struct AuthnResponse {
    /// The opaque bearer token to authenticate subsequent requests with.
    pub token: String,
    /// The account the token authenticates as.
    pub account: Account,
}
