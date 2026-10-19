//! The Access credential a caller outside the cluster presents to a deployed
//! environment's edge, and the local identity the local stack takes in its place.
//!
//! A deployed environment sits behind Cloudflare Access, which lets a request
//! through to the origin only when it carries a credential Access accepts: a
//! person's user token, sent as `cf-access-token`, or a service token, sent as
//! the `CF-Access-Client-Id` and `CF-Access-Client-Secret` pair. The local stack
//! has no edge, and its services take the identity a request names in
//! `Cf-Access-Authenticated-User-Email` as presented. The edge mints the
//! assertion the services verify, so none of these is a credential a service
//! reads itself.
//!
//! The values are secrets: [`Debug`] prints the kind of credential and never its
//! values, every header value is marked sensitive, and nothing here traces.

use std::fmt;

/// The header a person's Access user token is sent in.
pub const ACCESS_TOKEN_HEADER: &str = "cf-access-token";
/// The header an Access service token's client id is sent in.
pub const ACCESS_CLIENT_ID_HEADER: &str = "CF-Access-Client-Id";
/// The header an Access service token's client secret is sent in.
pub const ACCESS_CLIENT_SECRET_HEADER: &str = "CF-Access-Client-Secret";
/// The header the local stack reads the caller's identity from, in place of an
/// assertion the edge would mint.
pub const LOCAL_IDENTITY_HEADER: &str = "Cf-Access-Authenticated-User-Email";

/// One credential a client presents to an environment's edge on every request.
#[derive(Clone, PartialEq, Eq)]
pub enum AccessCredential {
    /// A person's Access user token, from `cloudflared access login` on the
    /// environment's origin. The assertion the edge mints carries the person's
    /// `email`.
    UserToken(String),
    /// An Access service token, for automation. The assertion the edge mints
    /// carries the token's `common_name`.
    ServiceToken {
        /// The token's client id.
        client_id: String,
        /// The token's client secret.
        client_secret: String,
    },
    /// The identity the local stack, which has no edge, records the call under
    /// (`dev@local` by default).
    LocalIdentity(String),
}

impl AccessCredential {
    /// Add this credential's headers to `headers`, each value marked sensitive.
    ///
    /// A value that is not a valid header value is left out rather than sent
    /// mangled, so the edge refuses the request as it would one with no
    /// credential.
    pub fn apply(&self, headers: &mut http::HeaderMap) {
        match self {
            Self::UserToken(token) => insert(headers, ACCESS_TOKEN_HEADER, token),
            Self::ServiceToken {
                client_id,
                client_secret,
            } => {
                insert(headers, ACCESS_CLIENT_ID_HEADER, client_id);
                insert(headers, ACCESS_CLIENT_SECRET_HEADER, client_secret);
            }
            Self::LocalIdentity(email) => insert(headers, LOCAL_IDENTITY_HEADER, email),
        }
    }

    /// The kind of credential, for a message that must name it without its value.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::UserToken(_) => "an Access user token",
            Self::ServiceToken { .. } => "an Access service token",
            Self::LocalIdentity(_) => "the local identity header",
        }
    }
}

impl fmt::Debug for AccessCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The local identity is an email the caller chose, not a secret, but it is
        // left out too so that no credential's value ever reaches a log.
        let kind = match self {
            Self::UserToken(_) => "UserToken",
            Self::ServiceToken { .. } => "ServiceToken",
            Self::LocalIdentity(_) => "LocalIdentity",
        };
        write!(f, "AccessCredential::{kind}(<redacted>)")
    }
}

/// Insert one sensitive header, skipping a value that is not a valid header value.
/// Header names are case-insensitive on the wire; `from_bytes` lower-cases them.
fn insert(headers: &mut http::HeaderMap, name: &str, value: &str) {
    if let (Ok(name), Ok(mut value)) = (
        http::HeaderName::from_bytes(name.as_bytes()),
        http::HeaderValue::from_str(value),
    ) {
        value.set_sensitive(true);
        headers.insert(name, value);
    }
}

#[cfg(test)]
#[path = "access.test.rs"]
mod tests;
