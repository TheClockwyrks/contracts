//! The descriptive-statistics shapes of a harness comparison: honest summaries of an
//! arm's `N` runs.
//!
//! The governing rule (see `docs/comparisons/statistics.md`) is: **present the
//! distribution and its sample size; never emit a verdict.** Every summary carries
//! its `n` and its spread, and the reader draws the conclusion. The statistics that
//! fill these shapes are the backend's.
//!
//! - **Cost, tokens, and session duration are right-skewed** — a floor with a long
//!   upper tail. They are summarized with the [`MetricSummary`] (median + IQR +
//!   min/max, alongside the mean), with a bootstrap confidence interval on the
//!   median that makes no normality assumption.
//! - **Pass rate is a proportion** — summarized with a [`PassRate`] carrying a
//!   Wilson score interval, not a normal one.

use serde::{Deserialize, Serialize};

/// A descriptive summary of one numeric metric (comparable cost, total tokens,
/// session duration) over an arm's `N` runs.
///
/// The bar a view draws is the [`median`](Self::median) — the honest "typical"
/// run for a skewed metric — but the summary carries the full spread so the view
/// can show the raw shape: the [quartiles](Self::q1) for a box, the
/// [extremes](Self::min) for whiskers, the [`mean`](Self::mean) (which sits above
/// the median for a right-skewed metric — that gap is informative, not redundant),
/// and a bootstrap [confidence interval](Self::ci_low) on the median that widens
/// honestly as `n` shrinks.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct MetricSummary {
    /// The number of runs summarized (always shown; never implied).
    pub n: usize,
    /// The median — the value a view plots for the arm.
    pub median: f64,
    /// The arithmetic mean, shown alongside the median (their gap reveals skew).
    pub mean: f64,
    /// The smallest observed value.
    pub min: f64,
    /// The largest observed value.
    pub max: f64,
    /// The first quartile (25th percentile), the box's lower edge.
    pub q1: f64,
    /// The third quartile (75th percentile), the box's upper edge.
    pub q3: f64,
    /// The interquartile range, `q3 - q1`.
    pub iqr: f64,
    /// The lower bound of the bootstrap confidence interval on the median.
    pub ci_low: f64,
    /// The upper bound of the bootstrap confidence interval on the median.
    pub ci_high: f64,
}

/// An arm's pass rate — the proportion of its runs whose automated validators all
/// passed — with a **Wilson score interval** rather than a normal one, because a
/// plain average understates the uncertainty of a small sample.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "contract", derive(ts_rs::TS, schemars::JsonSchema))]
pub struct PassRate {
    /// The number of runs (the denominator).
    pub n: u64,
    /// How many of them passed.
    pub passed: u64,
    /// The observed proportion, `passed / n`.
    pub rate: f64,
    /// The lower bound of the 95% Wilson score interval.
    pub wilson_low: f64,
    /// The upper bound of the 95% Wilson score interval.
    pub wilson_high: f64,
}
