//! The backend client's error type.

/// What a call through the backend client can fail with.
///
/// The variants keep the messages the run engine reported before the client was
/// its own crate, so a failure reads the same whichever caller surfaces it. A
/// caller with an error type of its own converts from this one.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A request to the backend failed: it could not be reached, it refused the
    /// request, its answer could not be read, or this client does not serve the
    /// call at all.
    #[error("publishing the run: {0}")]
    Publish(String),

    /// Ingesting a checkout into a backend's definition store failed: the backend
    /// could not be reached, refused the request, or reported the scan aborted on
    /// the progress feed it had already answered 200 on.
    #[error("ingesting the checkout: {0}")]
    Ingest(String),

    /// The references of a variant the resolved version does not declare were
    /// asked for.
    #[error(transparent)]
    VariantNotFound(#[from] test_cabinet_contracts::test_case::VariantNotFound),
}

/// The result of a backend client call.
pub type Result<T, E = Error> = std::result::Result<T, E>;
