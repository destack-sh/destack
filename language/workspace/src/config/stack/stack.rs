use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use super::common::{StackProviderJson, StackProviderOptions, merge_metadata};
use super::domain::{StackDomainJson, StackDomainOptions};
use super::ingress::{StackIngressJson, StackIngressOptions};
use super::network::{StackNetworkJson, StackNetworkOptions};
use super::publication::{StackPublicationJson, StackPublicationOptions};
use super::service::{StackServiceJson, StackServiceOptions};
use super::volume::{StackVolumeJson, StackVolumeOptions};
use super::workloads::{StackWorkloadJson, StackWorkloadOptions};
use crate::{AssetJson, AssetOptions, FeatureRefsJson, TelemetryRefsJson};

/// Destack stack configuration options.
#[derive(Debug, Clone, Default)]
pub struct StackOptions {
    /// Selection labels.
    pub labels: IndexMap<String, String>,
    /// Non-identifying metadata.
    pub annotations: IndexMap<String, String>,
    /// Provider attachment.
    pub provider: StackProviderOptions,
    /// Extra stack arguments.
    pub with: Option<Value>,
    /// Referenced runtime feature definitions.
    pub features: Vec<String>,
    /// Referenced telemetry definitions.
    pub telemetry: Vec<String>,
    /// Named deployable workloads.
    pub workloads: IndexMap<String, StackWorkloadOptions>,
    /// Named services.
    pub services: IndexMap<String, StackServiceOptions>,
    /// Named attached volumes.
    pub volumes: IndexMap<String, StackVolumeOptions>,
    /// Named asset collections.
    pub assets: IndexMap<String, AssetOptions>,
    /// Named publications over assets or target outputs.
    pub publications: IndexMap<String, StackPublicationOptions>,
    /// Named domains and DNS ownership.
    pub domains: IndexMap<String, StackDomainOptions>,
    /// External traffic and asset ingress.
    pub ingress: StackIngressOptions,
    /// Internal network topology settings.
    pub network: StackNetworkOptions,
}

impl StackOptions {
    /// Apply stack-level provider defaults across resource nouns.
    pub fn apply_provider_defaults(&mut self) {
        let provider = self.provider.clone();

        for workload in self.workloads.values_mut() {
            workload.provider.extend_from(&provider);
        }

        for service in self.services.values_mut() {
            service.provider.extend_from(&provider);
        }

        for volume in self.volumes.values_mut() {
            volume.provider.extend_from(&provider);
        }

        for publication in self.publications.values_mut() {
            publication.provider.extend_from(&provider);
        }

        for domain in self.domains.values_mut() {
            domain.provider.extend_from(&provider);
            domain.dns.provider.extend_from(&domain.provider);
        }
    }

    /// Inherit unset stack settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        merge_metadata(&mut self.labels, &parent.labels);
        merge_metadata(&mut self.annotations, &parent.annotations);
        self.provider.extend_from(&parent.provider);
        if self.with.is_none() {
            self.with = parent.with.clone();
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

impl From<&StackJson> for StackOptions {
    fn from(json: &StackJson) -> Self {
        let mut options = Self {
            labels: json.labels.clone().unwrap_or_default(),
            annotations: json.annotations.clone().unwrap_or_default(),
            provider: json
                .provider
                .as_ref()
                .map(StackProviderOptions::from)
                .unwrap_or_default(),
            with: json.with.clone(),
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
        };

        options.apply_provider_defaults();

        options
    }
}

/// A deployment topology node.
///
/// Inputs: explicit graph nodes and references to build outputs.
/// Outputs: one normalized stack graph.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackJson {
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
    /// Provider attachment.
    pub provider: Option<StackProviderJson>,
    /// Extra stack arguments.
    pub with: Option<Value>,
    /// Referenced runtime feature definitions.
    pub features: Option<FeatureRefsJson>,
    /// Referenced telemetry definitions.
    pub telemetry: Option<TelemetryRefsJson>,
    /// Named deployable workloads.
    pub workloads: Option<IndexMap<String, StackWorkloadJson>>,
    /// Named services.
    pub services: Option<IndexMap<String, StackServiceJson>>,
    /// Named attached volumes.
    pub volumes: Option<IndexMap<String, StackVolumeJson>>,
    /// Named asset collections.
    pub assets: Option<IndexMap<String, AssetJson>>,
    /// Named publications over assets or target outputs.
    pub publications: Option<IndexMap<String, StackPublicationJson>>,
    /// Named domains and DNS ownership.
    pub domains: Option<IndexMap<String, StackDomainJson>>,
    /// External traffic and asset ingress.
    #[serde(default)]
    pub ingress: StackIngressJson,
    /// Internal network topology settings.
    #[serde(default)]
    pub network: StackNetworkJson,
}
