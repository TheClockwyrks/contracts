//! The run-queue control plane's outputs the backend defines beside the job API: a bulk cancel's answer and the live stream's control frames.

/// The response to one of the three global cancel controls: how many runs the sweep
/// stopped, and how far into the queue it reached.
///
/// The **count is the point**. A sweep that reported only success would leave the
/// operator unable to tell "the queue was already empty" from "nothing matched", and
/// those call for opposite next moves. The two scope flags let the console phrase what
/// it just did ("stopped 12 runs, including 3 already executing") from the response
/// alone, rather than from which button it happens to have called.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct BulkCancelOut {
    /// How many jobs moved to the terminal `canceled` state.
    pub canceled: u32,
    /// Whether the sweep reached runs that had not started yet (`queued`, `pending`)
    /// — the ones that had cost nothing.
    pub included_waiting: bool,
    /// Whether the sweep reached runs that were already executing (`dispatched`,
    /// `starting`, `running`, `validating` or `storing`) — the ones whose work was
    /// discarded.
    pub included_active: bool,
}

/// The console stream's first frame: the id this client quotes back to
/// `PUT /notifications/{stream}/topics`.
///
/// The id is minted per **connection**, not per client, and deliberately so. An
/// `EventSource` reconnects on its own after a drop, and the reconnected stream is a
/// new subscriber with default topics — so the client must be told the new id and
/// re-apply what it wanted. Handing out a client-chosen or long-lived id would hide
/// that transition and leave a console silently subscribed to nothing.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct StreamOpened {
    /// The connected stream's id.
    pub stream_id: String,
}

/// The console stream's "you fell behind" frame: this client was lagged and the
/// named number of messages were dropped for it.
///
/// It carries the count only as a diagnostic — a client's response is the same
/// whatever the number: re-read the active list (and, if it is showing them, the
/// produced runs). The frame exists because the stream keeps no backlog, so there is
/// nothing to replay and silence would be indistinguishable from an idle queue.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct StreamResync {
    /// How many messages were dropped for this client.
    pub dropped: u64,
}

/// The default `retryCount` when a launch request omits it: one retry after a
/// failure, so the total attempts allowed is `1 + retry_count` = 2.
pub const DEFAULT_RETRY_COUNT: u32 = 1;
