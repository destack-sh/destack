use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use super::common::{
    StackAccessMode, StackAccessModeJson, StackIssuerRefJson, StackIssuerRefOptions,
    StackProviderJson, StackProviderOptions, merge_metadata,
};
use super::workload::{StackPlacementJson, StackPlacementOptions};

/// Destack service configuration options.
#[derive(Debug, Clone, Default)]
pub struct StackServiceOptions {
    /// Open semantic service type identifier.
    pub r#type: Option<String>,
    /// Selection labels.
    pub labels: IndexMap<String, String>,
    /// Non-identifying metadata.
    pub annotations: IndexMap<String, String>,
    /// Provider attachment.
    pub provider: StackProviderOptions,
    /// Workloads that provide this service.
    pub workloads: Vec<String>,
    /// External endpoint for unmanaged services.
    pub endpoint: Option<String>,
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

impl StackServiceOptions {
    /// Inherit unset service settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.r#type.is_none() {
            self.r#type = parent.r#type.clone();
        }
        self.provider.extend_from(&parent.provider);
        if self.workloads.is_empty() {
            self.workloads = parent.workloads.clone();
        }
        if self.endpoint.is_none() {
            self.endpoint = parent.endpoint.clone();
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

        self.placement.extend_from(&parent.placement);
        self.access.extend_from(&parent.access);
        merge_metadata(&mut self.labels, &parent.labels);
        merge_metadata(&mut self.annotations, &parent.annotations);
    }
}

impl From<&StackServiceJson> for StackServiceOptions {
    fn from(json: &StackServiceJson) -> Self {
        Self {
            r#type: json.r#type.clone(),
            labels: json.labels.clone().unwrap_or_default(),
            annotations: json.annotations.clone().unwrap_or_default(),
            provider: json
                .provider
                .as_ref()
                .map(StackProviderOptions::from)
                .unwrap_or_default(),
            workloads: json.workloads.clone().unwrap_or_default(),
            endpoint: json.endpoint.clone(),
            protocol: json.protocol.clone(),
            port: json.port,
            target_port: json.target_port,
            placement: StackPlacementOptions::from(&json.placement),
            access: StackServiceAccessOptions::from(&json.access),
            with: json.with.clone(),
        }
    }
}

/// Service access options.
#[derive(Debug, Clone, Default)]
pub struct StackServiceAccessOptions {
    /// Access mode.
    pub mode: Option<StackAccessMode>,
    /// Accepted issuer reference.
    pub issuer: Option<StackIssuerRefOptions>,
    /// Accepted audience names.
    pub audiences: Vec<String>,
    /// Allowed calling workload or service identities.
    pub callers: Vec<String>,
}

impl StackServiceAccessOptions {
    /// Inherit unset service access settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.mode.is_none() {
            self.mode = parent.mode;
        }
        if let Some(parent_issuer) = &parent.issuer {
            if let Some(issuer) = self.issuer.as_mut() {
                issuer.extend_from(parent_issuer);
            } else {
                self.issuer = Some(parent_issuer.clone());
            }
        }
        if self.audiences.is_empty() {
            self.audiences = parent.audiences.clone();
        }
        if self.callers.is_empty() {
            self.callers = parent.callers.clone();
        }
    }
}

impl From<&StackServiceAccessJson> for StackServiceAccessOptions {
    fn from(json: &StackServiceAccessJson) -> Self {
        Self {
            mode: json.mode.map(StackAccessMode::from),
            issuer: json.issuer.as_ref().map(StackIssuerRefOptions::from),
            audiences: json.audiences.clone().unwrap_or_default(),
            callers: json.callers.clone().unwrap_or_default(),
        }
    }
}

/// A stable bindable capability or endpoint.
///
/// Inputs: either named workloads or one external endpoint, plus protocol, placement, and service config.
/// Outputs: addressable fields such as `url` for bindings and ingress.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackServiceJson {
    /// Open semantic service type identifier.
    pub r#type: Option<String>,
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
    /// Provider attachment.
    pub provider: Option<StackProviderJson>,
    /// Workloads that provide this service.
    ///
    /// Exactly one of `workloads` or `endpoint` should be set.
    pub workloads: Option<Vec<String>>,
    /// External endpoint for unmanaged services.
    ///
    /// Exactly one of `workloads` or `endpoint` should be set.
    pub endpoint: Option<String>,
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

/// Service access JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackServiceAccessJson {
    /// Access mode.
    pub mode: Option<StackAccessModeJson>,
    /// Accepted issuer reference.
    pub issuer: Option<StackIssuerRefJson>,
    /// Accepted audience names.
    pub audiences: Option<Vec<String>>,
    /// Allowed calling workload or service identities.
    pub callers: Option<Vec<String>>,
}
