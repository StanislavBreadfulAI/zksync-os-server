use serde::{Serialize, Serializer};
use std::time::Duration;

/// Whether the node should be accepting transactions
#[derive(Debug, Clone, PartialEq)]
pub enum TransactionAcceptanceState {
    Accepting,
    NotAccepting(Vec<NotAcceptingReason>),
}

/// Reason why the node is not accepting transactions
#[derive(Debug, Clone, PartialEq, thiserror::Error, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NotAcceptingReason {
    /// Block production has been disabled via config (`sequencer_max_blocks_to_produce`)
    #[error("Node is not currently accepting transactions: block production disabled.")]
    BlockProductionDisabled,
    /// Node is shutting down
    #[error("Node is not currently accepting transactions: terminating.")]
    Terminating,
    /// One or more pipeline components are reporting backpressure
    #[error(
        "Node is not currently accepting transactions: pipeline backpressure ({}).",
        format_backpressure_components(causes)
    )]
    PipelineBackpressure { causes: Vec<BackpressureCause> },
}

fn format_backpressure_components(causes: &[BackpressureCause]) -> String {
    let mut names: Vec<&str> = causes.iter().map(|c| c.component).collect();
    names.sort_unstable();
    names.dedup();
    names.join(", ")
}

/// A single component contributing to pipeline backpressure
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BackpressureCause {
    pub component: &'static str,
    #[serde(flatten)]
    pub trigger: BackpressureTrigger,
}

/// The condition that triggered backpressure for a component
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "trigger", rename_all = "snake_case")]
pub enum BackpressureTrigger {
    /// The number of unprocessed blocks between this component and its upstream exceeds the threshold
    BlockDiffToUpstreamTooHigh { threshold: u64, actual: u64 },
    /// The block-timestamp diff between this component and its upstream exceeds the threshold.
    /// Only evaluated when both upstream and component timestamps are available.
    TimeDiffToUpstreamTooHigh {
        #[serde(
            rename = "threshold_secs",
            serialize_with = "serialize_duration_as_secs"
        )]
        threshold: Duration,
        #[serde(rename = "actual_secs", serialize_with = "serialize_duration_as_secs")]
        actual: Duration,
    },
    /// The number of unprocessed batches between this component and its upstream exceeds the threshold
    BatchDiffToUpstreamTooHigh { threshold: u64, actual: u64 },
}

fn serialize_duration_as_secs<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_f64(d.as_secs_f64())
}
