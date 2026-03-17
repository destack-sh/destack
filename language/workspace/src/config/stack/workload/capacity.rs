use serde::Deserialize;

use super::super::common::{
    StackRestartPolicy, StackRestartPolicyJson, StackRolloutStrategy, StackRolloutStrategyJson,
    StackScalingMetric, StackScalingMetricJson,
};

/// Placement configuration options.
#[derive(Debug, Clone, Default)]
pub struct StackPlacementOptions {
    /// Regions where this workload may run.
    pub regions: Vec<String>,
    /// Placement class or pool name.
    pub class: Option<String>,
}

impl StackPlacementOptions {
    /// Inherit unset placement settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.regions.is_empty() {
            self.regions = parent.regions.clone();
        }
        if self.class.is_none() {
            self.class = parent.class.clone();
        }
    }
}

impl From<&StackPlacementJson> for StackPlacementOptions {
    fn from(json: &StackPlacementJson) -> Self {
        Self {
            regions: json.regions.clone().unwrap_or_default(),
            class: json.class.clone(),
        }
    }
}

/// Capacity configuration options.
#[derive(Debug, Clone, Default)]
pub struct StackCapacityOptions {
    /// Requested compute resources.
    pub requests: StackComputeResourcesOptions,
    /// Hard compute limits.
    pub limits: StackComputeResourcesOptions,
    /// Scaling policy.
    pub scaling: StackScalingOptions,
    /// Concurrency policy.
    pub concurrency: StackConcurrencyOptions,
}

impl StackCapacityOptions {
    /// Inherit unset capacity settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        self.requests.extend_from(&parent.requests);
        self.limits.extend_from(&parent.limits);
        self.scaling.extend_from(&parent.scaling);
        self.concurrency.extend_from(&parent.concurrency);
    }
}

impl From<&StackCapacityJson> for StackCapacityOptions {
    fn from(json: &StackCapacityJson) -> Self {
        Self {
            requests: StackComputeResourcesOptions::from(&json.requests),
            limits: StackComputeResourcesOptions::from(&json.limits),
            scaling: StackScalingOptions::from(&json.scaling),
            concurrency: StackConcurrencyOptions::from(&json.concurrency),
        }
    }
}

/// Compute resource quantity options.
#[derive(Debug, Clone, Default)]
pub struct StackComputeResourcesOptions {
    /// CPU quantity.
    pub cpu: Option<String>,
    /// Memory quantity.
    pub memory: Option<String>,
    /// Storage quantity.
    pub storage: Option<String>,
}

impl StackComputeResourcesOptions {
    /// Inherit unset compute resource quantities from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.cpu.is_none() {
            self.cpu = parent.cpu.clone();
        }
        if self.memory.is_none() {
            self.memory = parent.memory.clone();
        }
        if self.storage.is_none() {
            self.storage = parent.storage.clone();
        }
    }
}

impl From<&StackComputeResourcesJson> for StackComputeResourcesOptions {
    fn from(json: &StackComputeResourcesJson) -> Self {
        Self {
            cpu: json.cpu.clone(),
            memory: json.memory.clone(),
            storage: json.storage.clone(),
        }
    }
}

/// Scaling configuration options.
#[derive(Debug, Clone, Default)]
pub struct StackScalingOptions {
    /// Minimum workload count.
    pub min: Option<u64>,
    /// Maximum workload count.
    pub max: Option<u64>,
    /// Scaling signal name.
    pub metric: Option<StackScalingMetric>,
}

impl StackScalingOptions {
    /// Inherit unset scaling settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.min.is_none() {
            self.min = parent.min;
        }
        if self.max.is_none() {
            self.max = parent.max;
        }
        if self.metric.is_none() {
            self.metric = parent.metric;
        }
    }
}

impl From<&StackScalingJson> for StackScalingOptions {
    fn from(json: &StackScalingJson) -> Self {
        Self {
            min: json.min,
            max: json.max,
            metric: json.metric.map(StackScalingMetric::from),
        }
    }
}

/// Concurrency configuration options.
#[derive(Debug, Clone, Default)]
pub struct StackConcurrencyOptions {
    /// Maximum concurrent work in flight.
    pub max: Option<u64>,
}

impl StackConcurrencyOptions {
    /// Inherit unset concurrency settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.max.is_none() {
            self.max = parent.max;
        }
    }
}

impl From<&StackConcurrencyJson> for StackConcurrencyOptions {
    fn from(json: &StackConcurrencyJson) -> Self {
        Self { max: json.max }
    }
}

/// Rollout configuration options.
#[derive(Debug, Clone, Default)]
pub struct StackRolloutOptions {
    /// Rollout strategy name.
    pub strategy: Option<StackRolloutStrategy>,
    /// Maximum unavailable instances.
    pub max_unavailable: Option<u64>,
    /// Maximum surge instances.
    pub max_surge: Option<u64>,
    /// Connection drain interval in seconds.
    pub drain_seconds: Option<u64>,
}

impl StackRolloutOptions {
    /// Inherit unset rollout settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.strategy.is_none() {
            self.strategy = parent.strategy;
        }
        if self.max_unavailable.is_none() {
            self.max_unavailable = parent.max_unavailable;
        }
        if self.max_surge.is_none() {
            self.max_surge = parent.max_surge;
        }
        if self.drain_seconds.is_none() {
            self.drain_seconds = parent.drain_seconds;
        }
    }
}

impl From<&StackRolloutJson> for StackRolloutOptions {
    fn from(json: &StackRolloutJson) -> Self {
        Self {
            strategy: json.strategy.map(StackRolloutStrategy::from),
            max_unavailable: json.max_unavailable,
            max_surge: json.max_surge,
            drain_seconds: json.drain_seconds,
        }
    }
}

/// Restart policy options.
#[derive(Debug, Clone, Default)]
pub struct StackRestartOptions {
    /// Restart policy name.
    pub policy: Option<StackRestartPolicy>,
    /// Maximum retry count.
    pub max_retries: Option<u64>,
}

impl StackRestartOptions {
    /// Inherit unset restart settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.policy.is_none() {
            self.policy = parent.policy;
        }
        if self.max_retries.is_none() {
            self.max_retries = parent.max_retries;
        }
    }
}

impl From<&StackRestartJson> for StackRestartOptions {
    fn from(json: &StackRestartJson) -> Self {
        Self {
            policy: json.policy.map(StackRestartPolicy::from),
            max_retries: json.max_retries,
        }
    }
}

/// Availability policy options.
#[derive(Debug, Clone, Default)]
pub struct StackAvailabilityOptions {
    /// Minimum available instances during voluntary disruption.
    pub min_available: Option<u64>,
}

impl StackAvailabilityOptions {
    /// Inherit unset availability settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.min_available.is_none() {
            self.min_available = parent.min_available;
        }
    }
}

impl From<&StackAvailabilityJson> for StackAvailabilityOptions {
    fn from(json: &StackAvailabilityJson) -> Self {
        Self {
            min_available: json.min_available,
        }
    }
}

/// Placement configuration JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackPlacementJson {
    /// Regions where this workload may run.
    pub regions: Option<Vec<String>>,
    /// Placement class or pool name.
    pub class: Option<String>,
}

/// Capacity configuration JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackCapacityJson {
    /// Requested compute resources.
    #[serde(default)]
    pub requests: StackComputeResourcesJson,
    /// Hard compute limits.
    #[serde(default)]
    pub limits: StackComputeResourcesJson,
    /// Scaling policy.
    #[serde(default)]
    pub scaling: StackScalingJson,
    /// Concurrency policy.
    #[serde(default)]
    pub concurrency: StackConcurrencyJson,
}

/// Compute resource quantity JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackComputeResourcesJson {
    /// CPU quantity.
    pub cpu: Option<String>,
    /// Memory quantity.
    pub memory: Option<String>,
    /// Storage quantity.
    pub storage: Option<String>,
}

/// Scaling configuration JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackScalingJson {
    /// Minimum workload count.
    pub min: Option<u64>,
    /// Maximum workload count.
    pub max: Option<u64>,
    /// Scaling signal name.
    pub metric: Option<StackScalingMetricJson>,
}

/// Concurrency configuration JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackConcurrencyJson {
    /// Maximum concurrent work in flight.
    pub max: Option<u64>,
}

/// Rollout configuration JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackRolloutJson {
    /// Rollout strategy name.
    pub strategy: Option<StackRolloutStrategyJson>,
    /// Maximum unavailable instances.
    pub max_unavailable: Option<u64>,
    /// Maximum surge instances.
    pub max_surge: Option<u64>,
    /// Connection drain interval in seconds.
    pub drain_seconds: Option<u64>,
}

/// Restart policy JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackRestartJson {
    /// Restart policy name.
    pub policy: Option<StackRestartPolicyJson>,
    /// Maximum retry count.
    pub max_retries: Option<u64>,
}

/// Availability policy JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackAvailabilityJson {
    /// Minimum available instances during voluntary disruption.
    pub min_available: Option<u64>,
}
