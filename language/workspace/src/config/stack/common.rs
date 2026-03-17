use indexmap::IndexMap;
use serde::Deserialize;

/// Workload run kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackRunKind {
    /// Serve HTTP traffic.
    #[default]
    Http,
    /// Publish static assets.
    Static,
    /// Consume queue or stream messages.
    Consumer,
    /// Run on a cron-like schedule.
    Schedule,
    /// Run as a one-shot background job.
    Job,
}

impl From<StackRunKindJson> for StackRunKind {
    fn from(json: StackRunKindJson) -> Self {
        match json {
            StackRunKindJson::Http => Self::Http,
            StackRunKindJson::Static => Self::Static,
            StackRunKindJson::Consumer => Self::Consumer,
            StackRunKindJson::Schedule => Self::Schedule,
            StackRunKindJson::Job => Self::Job,
        }
    }
}

/// Scaling signal options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackScalingMetric {
    /// Scale from CPU pressure.
    #[default]
    Cpu,
    /// Scale from memory pressure.
    Memory,
    /// Scale from request latency.
    Latency,
    /// Scale from queue depth.
    QueueDepth,
    /// Scale from in-flight request count.
    Concurrency,
}

impl From<StackScalingMetricJson> for StackScalingMetric {
    fn from(json: StackScalingMetricJson) -> Self {
        match json {
            StackScalingMetricJson::Cpu => Self::Cpu,
            StackScalingMetricJson::Memory => Self::Memory,
            StackScalingMetricJson::Latency => Self::Latency,
            StackScalingMetricJson::QueueDepth => Self::QueueDepth,
            StackScalingMetricJson::Concurrency => Self::Concurrency,
        }
    }
}

/// Rollout strategy options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackRolloutStrategy {
    /// Replace instances gradually.
    #[default]
    Rolling,
    /// Replace all instances at once.
    Recreate,
    /// Shift traffic between blue and green pools.
    BlueGreen,
    /// Shift traffic incrementally.
    Canary,
}

impl From<StackRolloutStrategyJson> for StackRolloutStrategy {
    fn from(json: StackRolloutStrategyJson) -> Self {
        match json {
            StackRolloutStrategyJson::Rolling => Self::Rolling,
            StackRolloutStrategyJson::Recreate => Self::Recreate,
            StackRolloutStrategyJson::BlueGreen => Self::BlueGreen,
            StackRolloutStrategyJson::Canary => Self::Canary,
        }
    }
}

/// Restart policy options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackRestartPolicy {
    /// Never restart automatically.
    Never,
    /// Restart when the unit fails.
    #[default]
    OnFailure,
    /// Always restart.
    Always,
}

impl From<StackRestartPolicyJson> for StackRestartPolicy {
    fn from(json: StackRestartPolicyJson) -> Self {
        match json {
            StackRestartPolicyJson::Never => Self::Never,
            StackRestartPolicyJson::OnFailure => Self::OnFailure,
            StackRestartPolicyJson::Always => Self::Always,
        }
    }
}

/// TLS mode options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackTlsMode {
    /// Let the platform manage certificates automatically.
    #[default]
    Automatic,
    /// Certificates are provided explicitly.
    Manual,
    /// Disable TLS termination.
    Disabled,
}

impl From<StackTlsModeJson> for StackTlsMode {
    fn from(json: StackTlsModeJson) -> Self {
        match json {
            StackTlsModeJson::Automatic => Self::Automatic,
            StackTlsModeJson::Manual => Self::Manual,
            StackTlsModeJson::Disabled => Self::Disabled,
        }
    }
}

/// Run kind JSON.
#[derive(Debug, Deserialize, Clone, Copy)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum StackRunKindJson {
    /// Serve HTTP traffic.
    Http,
    /// Publish static assets.
    Static,
    /// Consume queue or stream messages.
    Consumer,
    /// Run on a cron-like schedule.
    Schedule,
    /// Run as a one-shot background job.
    Job,
}

/// Scaling metric JSON.
#[derive(Debug, Deserialize, Clone, Copy)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum StackScalingMetricJson {
    /// Scale from CPU pressure.
    Cpu,
    /// Scale from memory pressure.
    Memory,
    /// Scale from request latency.
    Latency,
    /// Scale from queue depth.
    QueueDepth,
    /// Scale from in-flight request count.
    Concurrency,
}

/// Rollout strategy JSON.
#[derive(Debug, Deserialize, Clone, Copy)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum StackRolloutStrategyJson {
    /// Replace instances gradually.
    Rolling,
    /// Replace all instances at once.
    Recreate,
    /// Shift traffic between blue and green pools.
    BlueGreen,
    /// Shift traffic incrementally.
    Canary,
}

/// Restart policy JSON.
#[derive(Debug, Deserialize, Clone, Copy)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum StackRestartPolicyJson {
    /// Never restart automatically.
    Never,
    /// Restart when the unit fails.
    OnFailure,
    /// Always restart.
    Always,
}

/// TLS mode JSON.
#[derive(Debug, Deserialize, Clone, Copy)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum StackTlsModeJson {
    /// Let the platform manage certificates automatically.
    Automatic,
    /// Certificates are provided explicitly.
    Manual,
    /// Disable TLS termination.
    Disabled,
}

/// Merge metadata maps with child precedence.
pub(super) fn merge_metadata(
    into: &mut IndexMap<String, String>,
    parent: &IndexMap<String, String>,
) {
    for (name, value) in parent {
        into.entry(name.clone()).or_insert_with(|| value.clone());
    }
}
