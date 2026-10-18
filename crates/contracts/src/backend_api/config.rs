//! The console's client configuration (`GET /config`).

/// The body of `GET /config`: the console's client-side configuration.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct ClientConfig {
    /// The artifact service's public base URL, or `null` when artifacts are not
    /// served separately. The console resolves a pre-publish run's build and media
    /// links against it.
    #[cfg_attr(feature = "contract", ts(optional))]
    pub artifacts_url: Option<String>,
    /// The arena service's public base URL, or `null` when adversarial execution is
    /// not served separately. The console POSTs matches/tournaments and streams live
    /// tournament progress against it; the adversarial run UI degrades when absent.
    #[cfg_attr(feature = "contract", ts(optional))]
    pub arena_url: Option<String>,
    /// Grafana's base URL, or `null` when the deployment runs no observability
    /// stack. The console uses it to link a run to the traces it emitted; absent,
    /// that link is simply not rendered.
    #[cfg_attr(feature = "contract", ts(optional))]
    pub grafana_url: Option<String>,
    /// The public snapshot bucket's **read** base URL, or `null` when the deployment
    /// publishes no public snapshot. The client joins it with the deterministic keys
    /// it derives — today an asset-generation variant's published reference frames,
    /// `media/references/<slug>/<version>/<variant>/frames/<index>.png`. This is never
    /// the S3 write endpoint (`TCAB_R2_ENDPOINT`), which is credentialed and stays
    /// server-side.
    #[cfg_attr(feature = "contract", ts(optional))]
    pub snapshot_url: Option<String>,
}
