use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use super::super::common::{StackProviderJson, StackProviderOptions, merge_metadata};
use crate::config::runtime::{RuntimeConfigJson, RuntimeOptionsJson};
use crate::config::{FeatureRefsJson, TelemetryRefsJson};

use super::{
    StackAvailabilityJson, StackAvailabilityOptions, StackBindingJson, StackBindingOptions,
    StackCapacityJson, StackCapacityOptions, StackEnvVarJson, StackEnvVarOptions, StackHealthJson,
    StackHealthOptions, StackIdentityJson, StackIdentityOptions, StackMountJson, StackMountOptions,
    StackPlacementJson, StackPlacementOptions, StackRestartJson, StackRestartOptions,
    StackRolloutJson, StackRolloutOptions, StackTimeoutJson, StackTimeoutOptions,
};

/// Destack workload configuration options.
#[derive(Debug, Clone, Default)]
pub struct StackWorkloadOptions {
    /// Artifact target consumed by this workload.
    pub target: Option<String>,
    /// Referenced runtime feature definitions.
    pub features: Vec<String>,
    /// Referenced telemetry definitions.
    pub telemetry: Vec<String>,
    /// Cron schedule for scheduled workloads.
    pub schedule: Option<String>,
    /// Typed bindings exposed to this workload.
    pub bindings: IndexMap<String, StackBindingOptions>,
    /// Environment variable transport overrides.
    pub env: IndexMap<String, StackEnvVarOptions>,
    /// Named mounts exposed to this workload.
    pub mounts: IndexMap<String, StackMountOptions>,
    /// Selection labels.
    pub labels: IndexMap<String, String>,
    /// Non-identifying metadata.
    pub annotations: IndexMap<String, String>,
    /// Provider attachment.
    pub provider: StackProviderOptions,
    /// Workload health checks.
    pub health: StackHealthOptions,
    /// Placement constraints and preferences.
    pub placement: StackPlacementOptions,
    /// Workload identity.
    pub identity: StackIdentityOptions,
    /// Compute envelope and scaling behavior.
    pub capacity: StackCapacityOptions,
    /// Generic workload and runtime time budgets.
    pub timeouts: StackTimeoutOptions,
    /// Destack runtime shorthand or overrides for this workload.
    pub runtime: Option<RuntimeConfigJson>,
    /// Extra workload arguments.
    pub with: Option<Value>,
    /// Rollout policy.
    pub rollout: StackRolloutOptions,
    /// Restart policy.
    pub restart: StackRestartOptions,
    /// Availability policy.
    pub availability: StackAvailabilityOptions,
}

impl StackWorkloadOptions {
    /// Inherit unset workload settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.target.is_none() {
            self.target = parent.target.clone();
        }
        if self.features.is_empty() {
            self.features = parent.features.clone();
        }
        if self.telemetry.is_empty() {
            self.telemetry = parent.telemetry.clone();
        }
        self.provider.extend_from(&parent.provider);

        if self.schedule.is_none() {
            self.schedule = parent.schedule.clone();
        }

        for (name, binding) in &parent.bindings {
            if let Some(current) = self.bindings.get_mut(name) {
                current.extend_from(binding);
            } else {
                self.bindings.insert(name.clone(), binding.clone());
            }
        }

        for (name, value) in &parent.env {
            if let Some(current) = self.env.get_mut(name) {
                current.extend_from(value);
            } else {
                self.env.insert(name.clone(), value.clone());
            }
        }

        for (name, mount) in &parent.mounts {
            if let Some(current) = self.mounts.get_mut(name) {
                current.extend_from(mount);
            } else {
                self.mounts.insert(name.clone(), mount.clone());
            }
        }

        merge_metadata(&mut self.labels, &parent.labels);
        merge_metadata(&mut self.annotations, &parent.annotations);
        self.health.extend_from(&parent.health);
        self.placement.extend_from(&parent.placement);
        self.identity.extend_from(&parent.identity);
        self.capacity.extend_from(&parent.capacity);
        self.timeouts.extend_from(&parent.timeouts);
        if self.runtime.is_none() {
            self.runtime = parent.runtime.clone();
        }
        if self.with.is_none() {
            self.with = parent.with.clone();
        }
        self.rollout.extend_from(&parent.rollout);
        self.restart.extend_from(&parent.restart);
        self.availability.extend_from(&parent.availability);
    }

    /// Return one effective runtime override set for this workload.
    pub fn effective_runtime_overrides(&self) -> Option<RuntimeOptionsJson> {
        let inferred = self.capacity.inferred_runtime_overrides();
        let explicit = self
            .runtime
            .as_ref()
            .map(RuntimeConfigJson::as_options_json);

        match (explicit, inferred) {
            (Some(mut explicit), Some(inferred)) => {
                explicit.extend_from(&inferred);
                Some(explicit)
            }
            (Some(explicit), None) => Some(explicit),
            (None, Some(inferred)) => Some(inferred),
            (None, None) => None,
        }
    }
}

impl From<&StackWorkloadJson> for StackWorkloadOptions {
    fn from(json: &StackWorkloadJson) -> Self {
        Self {
            target: json.target.clone(),
            features: json
                .features
                .as_ref()
                .map(FeatureRefsJson::names)
                .unwrap_or_default(),
            telemetry: json
                .telemetry
                .as_ref()
                .map(TelemetryRefsJson::names)
                .unwrap_or_default(),
            schedule: json.schedule.clone(),
            bindings: json
                .bindings
                .as_ref()
                .map(|bindings| {
                    bindings
                        .iter()
                        .map(|(name, binding)| (name.clone(), StackBindingOptions::from(binding)))
                        .collect()
                })
                .unwrap_or_default(),
            env: json
                .env
                .as_ref()
                .map(|env| {
                    env.iter()
                        .map(|(name, value)| (name.clone(), StackEnvVarOptions::from(value)))
                        .collect()
                })
                .unwrap_or_default(),
            mounts: json
                .mounts
                .as_ref()
                .map(|mounts| {
                    mounts
                        .iter()
                        .map(|(name, mount)| (name.clone(), StackMountOptions::from(mount)))
                        .collect()
                })
                .unwrap_or_default(),
            labels: json.labels.clone().unwrap_or_default(),
            annotations: json.annotations.clone().unwrap_or_default(),
            provider: json
                .provider
                .as_ref()
                .map(StackProviderOptions::from)
                .unwrap_or_default(),
            health: StackHealthOptions::from(&json.health),
            placement: StackPlacementOptions::from(&json.placement),
            identity: StackIdentityOptions::from(&json.identity),
            capacity: StackCapacityOptions::from(&json.capacity),
            timeouts: StackTimeoutOptions::from(&json.timeouts),
            runtime: json.runtime.clone(),
            with: json.with.clone(),
            rollout: StackRolloutOptions::from(&json.rollout),
            restart: StackRestartOptions::from(&json.restart),
            availability: StackAvailabilityOptions::from(&json.availability),
        }
    }
}

/// An executable compute node.
///
/// Inputs: one target, bindings, mounts, env transport, schedule, and runtime policy.
/// Outputs: runtime instances that may back one or more services.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackWorkloadJson {
    /// Artifact target consumed by this workload.
    pub target: Option<String>,
    /// Referenced runtime feature definitions.
    pub features: Option<FeatureRefsJson>,
    /// Referenced telemetry definitions.
    pub telemetry: Option<TelemetryRefsJson>,
    /// Cron schedule for scheduled workloads.
    pub schedule: Option<String>,
    /// Typed bindings exposed to this workload.
    pub bindings: Option<IndexMap<String, StackBindingJson>>,
    /// Environment variable transport overrides.
    pub env: Option<IndexMap<String, StackEnvVarJson>>,
    /// Named mounts exposed to this workload.
    pub mounts: Option<IndexMap<String, StackMountJson>>,
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
    /// Provider attachment.
    pub provider: Option<StackProviderJson>,
    /// Workload health checks.
    #[serde(default)]
    pub health: StackHealthJson,
    /// Placement constraints and preferences.
    #[serde(default)]
    pub placement: StackPlacementJson,
    /// Workload identity.
    #[serde(default)]
    pub identity: StackIdentityJson,
    /// Compute envelope and scaling behavior.
    #[serde(default)]
    pub capacity: StackCapacityJson,
    /// Generic workload and runtime time budgets.
    #[serde(default)]
    pub timeouts: StackTimeoutJson,
    /// Destack runtime shorthand or overrides for this workload.
    pub runtime: Option<RuntimeConfigJson>,
    /// Extra workload arguments.
    pub with: Option<Value>,
    /// Rollout policy.
    #[serde(default)]
    pub rollout: StackRolloutJson,
    /// Restart policy.
    #[serde(default)]
    pub restart: StackRestartJson,
    /// Availability policy.
    #[serde(default)]
    pub availability: StackAvailabilityJson,
}
