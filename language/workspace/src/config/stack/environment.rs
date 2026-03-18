use indexmap::IndexMap;
use serde::Deserialize;

use super::asset::{StackAssetJson, StackAssetOptions};
use super::common::merge_metadata;
use super::component::{StackComponentJson, StackComponentOptions};
use super::config::{StackConfigJson, StackConfigOptions};
use super::domain::{StackDomainJson, StackDomainOptions};
use super::ingress::{StackIngressJson, StackIngressOptions};
use super::network::{StackNetworkJson, StackNetworkOptions};
use super::publication::{StackPublicationJson, StackPublicationOptions};
use super::secret::{StackSecretJson, StackSecretOptions};
use super::service::{StackServiceJson, StackServiceOptions};
use super::volume::{StackVolumeJson, StackVolumeOptions};
use super::workload::{StackWorkloadJson, StackWorkloadOptions};

/// Stack environment overlay options.
#[derive(Debug, Clone, Default)]
pub struct StackEnvironmentOptions {
    /// Selection labels.
    pub labels: IndexMap<String, String>,
    /// Non-identifying metadata.
    pub annotations: IndexMap<String, String>,
    /// Whether this environment is ephemeral.
    pub ephemeral: Option<bool>,
    /// Component overrides.
    pub components: IndexMap<String, StackComponentOptions>,
    /// Workload overrides.
    pub workloads: IndexMap<String, StackWorkloadOptions>,
    /// Service overrides.
    pub services: IndexMap<String, StackServiceOptions>,
    /// Volume overrides.
    pub volumes: IndexMap<String, StackVolumeOptions>,
    /// Config overrides.
    pub configs: IndexMap<String, StackConfigOptions>,
    /// Secret overrides.
    pub secrets: IndexMap<String, StackSecretOptions>,
    /// Asset overrides.
    pub assets: IndexMap<String, StackAssetOptions>,
    /// Publication overrides.
    pub publications: IndexMap<String, StackPublicationOptions>,
    /// Domain overrides.
    pub domains: IndexMap<String, StackDomainOptions>,
    /// Ingress overrides.
    pub ingress: StackIngressOptions,
    /// Network overrides.
    pub network: StackNetworkOptions,
}

impl StackEnvironmentOptions {
    /// Inherit unset environment settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        merge_metadata(&mut self.labels, &parent.labels);
        merge_metadata(&mut self.annotations, &parent.annotations);
        if self.ephemeral.is_none() {
            self.ephemeral = parent.ephemeral;
        }

        for (name, component) in &parent.components {
            if let Some(current) = self.components.get_mut(name) {
                current.extend_from(component);
            } else {
                self.components.insert(name.clone(), component.clone());
            }
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

impl From<&StackEnvironmentJson> for StackEnvironmentOptions {
    fn from(json: &StackEnvironmentJson) -> Self {
        Self {
            labels: json.labels.clone().unwrap_or_default(),
            annotations: json.annotations.clone().unwrap_or_default(),
            ephemeral: json.ephemeral,
            components: json
                .components
                .as_ref()
                .map(|components| {
                    components
                        .iter()
                        .map(|(name, component)| {
                            (name.clone(), StackComponentOptions::from(component))
                        })
                        .collect()
                })
                .unwrap_or_default(),
            workloads: json
                .workloads
                .as_ref()
                .map(|workloads| {
                    workloads
                        .iter()
                        .map(|(name, workload)| {
                            (name.clone(), StackWorkloadOptions::from(workload))
                        })
                        .collect()
                })
                .unwrap_or_default(),
            services: json
                .services
                .as_ref()
                .map(|services| {
                    services
                        .iter()
                        .map(|(name, service)| (name.clone(), StackServiceOptions::from(service)))
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
                        .map(|(name, config)| (name.clone(), StackConfigOptions::from(config)))
                        .collect()
                })
                .unwrap_or_default(),
            secrets: json
                .secrets
                .as_ref()
                .map(|secrets| {
                    secrets
                        .iter()
                        .map(|(name, secret)| (name.clone(), StackSecretOptions::from(secret)))
                        .collect()
                })
                .unwrap_or_default(),
            assets: json
                .assets
                .as_ref()
                .map(|assets| {
                    assets
                        .iter()
                        .map(|(name, asset)| (name.clone(), StackAssetOptions::from(asset)))
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
                            (name.clone(), StackPublicationOptions::from(publication))
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

/// A partial environment overlay over one named stack.
///
/// Inputs: explicit overrides for graph nodes and policy.
/// Outputs: one environment-specific view of the same stack graph.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackEnvironmentJson {
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
    /// Whether this environment is ephemeral.
    pub ephemeral: Option<bool>,
    /// Component overrides.
    pub components: Option<IndexMap<String, StackComponentJson>>,
    /// Workload overrides.
    pub workloads: Option<IndexMap<String, StackWorkloadJson>>,
    /// Service overrides.
    pub services: Option<IndexMap<String, StackServiceJson>>,
    /// Volume overrides.
    pub volumes: Option<IndexMap<String, StackVolumeJson>>,
    /// Config overrides.
    pub configs: Option<IndexMap<String, StackConfigJson>>,
    /// Secret overrides.
    pub secrets: Option<IndexMap<String, StackSecretJson>>,
    /// Asset overrides.
    pub assets: Option<IndexMap<String, StackAssetJson>>,
    /// Publication overrides.
    pub publications: Option<IndexMap<String, StackPublicationJson>>,
    /// Domain overrides.
    pub domains: Option<IndexMap<String, StackDomainJson>>,
    /// Ingress overrides.
    #[serde(default)]
    pub ingress: StackIngressJson,
    /// Network overrides.
    #[serde(default)]
    pub network: StackNetworkJson,
}
