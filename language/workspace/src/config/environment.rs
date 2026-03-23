use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use crate::config::asset::{AssetJson, AssetOptions};
use crate::config::config::{ConfigJson, ConfigOptions};
use crate::config::feature::FeatureRefsJson;
use crate::config::runtime::RuntimeConfigJson;
use crate::config::secret::{SecretJson, SecretOptions};
use crate::config::stack::{
    StackAvailabilityJson, StackAvailabilityOptions, StackBindingJson, StackBindingOptions,
    StackCacheJson, StackCacheOptions, StackCapacityJson, StackCapacityOptions, StackDomainJson,
    StackDomainOptions, StackEnvVarJson, StackEnvVarOptions, StackHealthJson, StackHealthOptions,
    StackIdentityJson, StackIdentityOptions, StackIngressJson, StackIngressOptions, StackMountJson,
    StackMountOptions, StackNetworkJson, StackNetworkOptions, StackPlacementJson,
    StackPlacementOptions, StackProviderJson, StackProviderOptions, StackPublicationOriginJson,
    StackPublicationOriginOptions, StackRestartJson, StackRestartOptions, StackRolloutJson,
    StackRolloutOptions, StackServiceAccessJson, StackServiceAccessOptions, StackTimeoutJson,
    StackTimeoutOptions, StackVolumeJson, StackVolumeOptions, merge_metadata,
};
use crate::config::telemetry::TelemetryRefsJson;

/// Workload refinement options.
#[derive(Debug, Clone, Default)]
struct WorkloadOptions {
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

impl WorkloadOptions {
    /// Inherit unset workload refinement settings from one parent config.
    fn extend_from(&mut self, parent: &Self) {
        if self.features.is_empty() {
            self.features = parent.features.clone();
        }
        if self.telemetry.is_empty() {
            self.telemetry = parent.telemetry.clone();
        }
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
        self.provider.extend_from(&parent.provider);
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
}

impl From<&WorkloadJson> for WorkloadOptions {
    fn from(json: &WorkloadJson) -> Self {
        Self {
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

/// Service refinement options.
#[derive(Debug, Clone, Default)]
struct ServiceOptions {
    /// Referenced runtime feature definitions.
    pub features: Vec<String>,
    /// Referenced telemetry definitions.
    pub telemetry: Vec<String>,
    /// Selection labels.
    pub labels: IndexMap<String, String>,
    /// Non-identifying metadata.
    pub annotations: IndexMap<String, String>,
    /// Provider attachment.
    pub provider: StackProviderOptions,
    /// Service protocol.
    pub protocol: Option<String>,
    /// Externally visible service port.
    pub port: Option<u16>,
    /// Target workload port.
    pub target_port: Option<u16>,
    /// Placement constraints and preferences.
    pub placement: StackPlacementOptions,
    /// Service access expectations.
    pub access: StackServiceAccessOptions,
    /// Extra service arguments.
    pub with: Option<Value>,
}

impl ServiceOptions {
    /// Inherit unset service refinement settings from one parent config.
    fn extend_from(&mut self, parent: &Self) {
        if self.features.is_empty() {
            self.features = parent.features.clone();
        }
        if self.telemetry.is_empty() {
            self.telemetry = parent.telemetry.clone();
        }
        if self.protocol.is_none() {
            self.protocol = parent.protocol.clone();
        }
        if self.port.is_none() {
            self.port = parent.port;
        }
        if self.target_port.is_none() {
            self.target_port = parent.target_port;
        }
        if self.with.is_none() {
            self.with = parent.with.clone();
        }

        merge_metadata(&mut self.labels, &parent.labels);
        merge_metadata(&mut self.annotations, &parent.annotations);
        self.provider.extend_from(&parent.provider);
        self.placement.extend_from(&parent.placement);
        self.access.extend_from(&parent.access);
    }
}

impl From<&ServiceJson> for ServiceOptions {
    fn from(json: &ServiceJson) -> Self {
        Self {
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
            labels: json.labels.clone().unwrap_or_default(),
            annotations: json.annotations.clone().unwrap_or_default(),
            provider: json
                .provider
                .as_ref()
                .map(StackProviderOptions::from)
                .unwrap_or_default(),
            protocol: json.protocol.clone(),
            port: json.port,
            target_port: json.target_port,
            placement: StackPlacementOptions::from(&json.placement),
            access: StackServiceAccessOptions::from(&json.access),
            with: json.with.clone(),
        }
    }
}

/// Publication refinement options.
#[derive(Debug, Clone, Default)]
struct PublicationOptions {
    /// Provider attachment.
    pub provider: StackProviderOptions,
    /// Backing origin configuration.
    pub origin: StackPublicationOriginOptions,
    /// Cache policy for the published content.
    pub cache: StackCacheOptions,
    /// Selection labels.
    pub labels: IndexMap<String, String>,
    /// Non-identifying metadata.
    pub annotations: IndexMap<String, String>,
    /// Extra publication arguments.
    pub with: Option<Value>,
}

impl PublicationOptions {
    /// Inherit unset publication refinement settings from one parent config.
    fn extend_from(&mut self, parent: &Self) {
        self.provider.extend_from(&parent.provider);
        self.origin.extend_from(&parent.origin);
        self.cache.extend_from(&parent.cache);
        merge_metadata(&mut self.labels, &parent.labels);
        merge_metadata(&mut self.annotations, &parent.annotations);
        if self.with.is_none() {
            self.with = parent.with.clone();
        }
    }
}

impl From<&PublicationJson> for PublicationOptions {
    fn from(json: &PublicationJson) -> Self {
        Self {
            provider: json
                .provider
                .as_ref()
                .map(StackProviderOptions::from)
                .unwrap_or_default(),
            origin: StackPublicationOriginOptions::from(&json.origin),
            cache: StackCacheOptions::from(&json.cache),
            labels: json.labels.clone().unwrap_or_default(),
            annotations: json.annotations.clone().unwrap_or_default(),
            with: json.with.clone(),
        }
    }
}

/// Environment overlay options.
#[derive(Debug, Clone, Default)]
pub struct EnvironmentOptions {
    /// Selection labels.
    pub labels: IndexMap<String, String>,
    /// Non-identifying metadata.
    pub annotations: IndexMap<String, String>,
    /// Whether this environment is ephemeral.
    pub ephemeral: Option<bool>,
    /// Referenced runtime feature definitions.
    pub features: Vec<String>,
    /// Referenced telemetry definitions.
    pub telemetry: Vec<String>,
    /// Workload refinements.
    workloads: IndexMap<String, WorkloadOptions>,
    /// Service refinements.
    services: IndexMap<String, ServiceOptions>,
    /// Volume overrides.
    pub volumes: IndexMap<String, StackVolumeOptions>,
    /// Config overrides.
    pub configs: IndexMap<String, ConfigOptions>,
    /// Secret overrides.
    pub secrets: IndexMap<String, SecretOptions>,
    /// Asset overrides.
    pub assets: IndexMap<String, AssetOptions>,
    /// Publication refinements.
    publications: IndexMap<String, PublicationOptions>,
    /// Domain overrides.
    pub domains: IndexMap<String, StackDomainOptions>,
    /// Ingress overrides.
    pub ingress: StackIngressOptions,
    /// Network overrides.
    pub network: StackNetworkOptions,
}

impl EnvironmentOptions {
    /// Inherit unset environment refinement settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        merge_metadata(&mut self.labels, &parent.labels);
        merge_metadata(&mut self.annotations, &parent.annotations);
        if self.ephemeral.is_none() {
            self.ephemeral = parent.ephemeral;
        }
        if self.features.is_empty() {
            self.features = parent.features.clone();
        }
        if self.telemetry.is_empty() {
            self.telemetry = parent.telemetry.clone();
        }

        for (name, workload) in &parent.workloads {
            if let Some(current) = self.workloads.get_mut(name) {
                current.extend_from(workload);
            } else {
                self.workloads.insert(name.clone(), workload.clone());
            }
        }

        for (name, service) in &parent.services {
            if let Some(current) = self.services.get_mut(name) {
                current.extend_from(service);
            } else {
                self.services.insert(name.clone(), service.clone());
            }
        }

        for (name, volume) in &parent.volumes {
            if let Some(current) = self.volumes.get_mut(name) {
                current.extend_from(volume);
            } else {
                self.volumes.insert(name.clone(), volume.clone());
            }
        }

        for (name, config) in &parent.configs {
            if let Some(current) = self.configs.get_mut(name) {
                current.extend_from(config);
            } else {
                self.configs.insert(name.clone(), config.clone());
            }
        }

        for (name, secret) in &parent.secrets {
            if let Some(current) = self.secrets.get_mut(name) {
                current.extend_from(secret);
            } else {
                self.secrets.insert(name.clone(), secret.clone());
            }
        }

        for (name, asset) in &parent.assets {
            if let Some(current) = self.assets.get_mut(name) {
                current.extend_from(asset);
            } else {
                self.assets.insert(name.clone(), asset.clone());
            }
        }

        for (name, publication) in &parent.publications {
            if let Some(current) = self.publications.get_mut(name) {
                current.extend_from(publication);
            } else {
                self.publications.insert(name.clone(), publication.clone());
            }
        }

        for (name, domain) in &parent.domains {
            if let Some(current) = self.domains.get_mut(name) {
                current.extend_from(domain);
            } else {
                self.domains.insert(name.clone(), domain.clone());
            }
        }

        self.ingress.extend_from(&parent.ingress);
        self.network.extend_from(&parent.network);
    }
}

impl From<&EnvironmentJson> for EnvironmentOptions {
    fn from(json: &EnvironmentJson) -> Self {
        Self {
            labels: json.labels.clone().unwrap_or_default(),
            annotations: json.annotations.clone().unwrap_or_default(),
            ephemeral: json.ephemeral,
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
            workloads: json
                .workloads
                .as_ref()
                .map(|workloads| {
                    workloads
                        .iter()
                        .map(|(name, workload)| (name.clone(), WorkloadOptions::from(workload)))
                        .collect()
                })
                .unwrap_or_default(),
            services: json
                .services
                .as_ref()
                .map(|services| {
                    services
                        .iter()
                        .map(|(name, service)| (name.clone(), ServiceOptions::from(service)))
                        .collect()
                })
                .unwrap_or_default(),
            volumes: json
                .volumes
                .as_ref()
                .map(|volumes| {
                    volumes
                        .iter()
                        .map(|(name, volume)| (name.clone(), StackVolumeOptions::from(volume)))
                        .collect()
                })
                .unwrap_or_default(),
            configs: json
                .configs
                .as_ref()
                .map(|configs| {
                    configs
                        .iter()
                        .map(|(name, config)| (name.clone(), ConfigOptions::from(config)))
                        .collect()
                })
                .unwrap_or_default(),
            secrets: json
                .secrets
                .as_ref()
                .map(|secrets| {
                    secrets
                        .iter()
                        .map(|(name, secret)| (name.clone(), SecretOptions::from(secret)))
                        .collect()
                })
                .unwrap_or_default(),
            assets: json
                .assets
                .as_ref()
                .map(|assets| {
                    assets
                        .iter()
                        .map(|(name, asset)| (name.clone(), AssetOptions::from(asset)))
                        .collect()
                })
                .unwrap_or_default(),
            publications: json
                .publications
                .as_ref()
                .map(|publications| {
                    publications
                        .iter()
                        .map(|(name, publication)| {
                            (name.clone(), PublicationOptions::from(publication))
                        })
                        .collect()
                })
                .unwrap_or_default(),
            domains: json
                .domains
                .as_ref()
                .map(|domains| {
                    domains
                        .iter()
                        .map(|(name, domain)| (name.clone(), StackDomainOptions::from(domain)))
                        .collect()
                })
                .unwrap_or_default(),
            ingress: StackIngressOptions::from(&json.ingress),
            network: StackNetworkOptions::from(&json.network),
        }
    }
}

/// Convert environment declarations into normalized options.
pub fn environment_options_from_json(
    json: &Option<IndexMap<String, EnvironmentJson>>,
) -> IndexMap<String, EnvironmentOptions> {
    json.as_ref()
        .map(|environments| {
            environments
                .iter()
                .map(|(name, environment)| (name.clone(), EnvironmentOptions::from(environment)))
                .collect()
        })
        .unwrap_or_default()
}

/// Inherit one environment map from a parent config.
pub fn extend_environment_options(
    current: &mut IndexMap<String, EnvironmentOptions>,
    parent: &IndexMap<String, EnvironmentOptions>,
) {
    for (name, environment) in parent {
        if let Some(existing) = current.get_mut(name) {
            existing.extend_from(environment);
        } else {
            current.insert(name.clone(), environment.clone());
        }
    }
}

/// Workload refinement JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
struct WorkloadJson {
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

/// Service refinement JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
struct ServiceJson {
    /// Referenced runtime feature definitions.
    pub features: Option<FeatureRefsJson>,
    /// Referenced telemetry definitions.
    pub telemetry: Option<TelemetryRefsJson>,
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
    /// Provider attachment.
    pub provider: Option<StackProviderJson>,
    /// Service protocol.
    pub protocol: Option<String>,
    /// Externally visible service port.
    pub port: Option<u16>,
    /// Target workload port.
    pub target_port: Option<u16>,
    /// Placement constraints and preferences.
    #[serde(default)]
    pub placement: StackPlacementJson,
    /// Service access expectations.
    #[serde(default)]
    pub access: StackServiceAccessJson,
    /// Extra service arguments.
    pub with: Option<Value>,
}

/// Publication refinement JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
struct PublicationJson {
    /// Provider attachment.
    pub provider: Option<StackProviderJson>,
    /// Backing origin configuration.
    #[serde(default)]
    pub origin: StackPublicationOriginJson,
    /// Cache policy for the published content.
    #[serde(default)]
    pub cache: StackCacheJson,
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
    /// Extra publication arguments.
    pub with: Option<Value>,
}

/// A reusable environment overlay.
///
/// Inputs: refinements for existing graph nodes and policy.
/// Outputs: one environment-specific view of the same stack graph.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentJson {
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
    /// Whether this environment is ephemeral.
    pub ephemeral: Option<bool>,
    /// Referenced runtime feature definitions.
    pub features: Option<FeatureRefsJson>,
    /// Referenced telemetry definitions.
    pub telemetry: Option<TelemetryRefsJson>,
    /// Workload refinements.
    workloads: Option<IndexMap<String, WorkloadJson>>,
    /// Service refinements.
    services: Option<IndexMap<String, ServiceJson>>,
    /// Volume overrides.
    pub volumes: Option<IndexMap<String, StackVolumeJson>>,
    /// Config overrides.
    pub configs: Option<IndexMap<String, ConfigJson>>,
    /// Secret overrides.
    pub secrets: Option<IndexMap<String, SecretJson>>,
    /// Asset overrides.
    pub assets: Option<IndexMap<String, AssetJson>>,
    /// Publication refinements.
    publications: Option<IndexMap<String, PublicationJson>>,
    /// Domain overrides.
    pub domains: Option<IndexMap<String, StackDomainJson>>,
    /// Ingress overrides.
    #[serde(default)]
    pub ingress: StackIngressJson,
    /// Network overrides.
    #[serde(default)]
    pub network: StackNetworkJson,
}
