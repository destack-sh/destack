use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use super::common::{
    StackAccessMode, StackAccessModeJson, StackIssuerRefJson, StackIssuerRefOptions, merge_metadata,
};
use super::workload::{StackPlacementJson, StackPlacementOptions};

#[cfg(feature = "schema")]
use std::borrow::Cow;

#[cfg(feature = "schema")]
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};

/// Destack service configuration options.
#[derive(Debug, Clone, Default)]
pub struct StackServiceOptions {
    /// Open semantic service type identifier.
    pub r#type: Option<String>,
    /// Selection labels.
    pub labels: IndexMap<String, String>,
    /// Non-identifying metadata.
    pub annotations: IndexMap<String, String>,
    /// Provider-specific lowering overrides.
    pub provider: Option<Value>,
    /// Workloads behind this service.
    pub workloads: Vec<String>,
    /// External endpoint for unmanaged services.
    pub url: Option<String>,
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
    /// Extra service metadata.
    pub config: Option<Value>,
}

impl StackServiceOptions {
    /// Inherit unset service settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.r#type.is_none() {
            self.r#type = parent.r#type.clone();
        }
        if self.provider.is_none() {
            self.provider = parent.provider.clone();
        }
        if self.workloads.is_empty() {
            self.workloads = parent.workloads.clone();
        }
        if self.url.is_none() {
            self.url = parent.url.clone();
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
        if self.config.is_none() {
            self.config = parent.config.clone();
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
            provider: json.provider.clone(),
            workloads: json.workloads.clone().unwrap_or_default(),
            url: json.url.clone(),
            protocol: json.protocol.clone(),
            port: json.port,
            target_port: json.target_port,
            placement: StackPlacementOptions::from(&json.placement),
            access: StackServiceAccessOptions::from(&json.access),
            config: json.config.clone(),
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
/// Inputs: either named workloads or one external url, plus protocol, placement, and service config.
/// Outputs: addressable fields such as `url` for bindings and ingress.
#[derive(Debug, Default, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StackServiceJson {
    /// Open semantic service type identifier.
    pub r#type: Option<String>,
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
    /// Provider-specific lowering overrides.
    pub provider: Option<Value>,
    /// Workloads behind this service.
    ///
    /// Exactly one of `workloads` or `url` should be set.
    pub workloads: Option<Vec<String>>,
    /// External endpoint for unmanaged services.
    ///
    /// Exactly one of `workloads` or `url` should be set.
    pub url: Option<String>,
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
    /// Extra service metadata.
    pub config: Option<Value>,
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

#[cfg(feature = "schema")]
impl JsonSchema for StackServiceJson {
    fn schema_name() -> Cow<'static, str> {
        "StackServiceJson".into()
    }

    fn schema_id() -> Cow<'static, str> {
        concat!(module_path!(), "::StackServiceJson").into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        let placement_schema = generator.subschema_for::<StackPlacementJson>();
        let access_schema = generator.subschema_for::<StackServiceAccessJson>();

        json_schema!({
            "description": "A stable bindable capability or endpoint.\n\nInputs: either named workloads or one external url, plus protocol, placement, and service config.\nOutputs: addressable fields such as `url` for bindings and ingress.",
            "type": "object",
            "properties": {
                "type": {
                    "description": "Open semantic service type identifier.",
                    "type": ["string", "null"]
                },
                "labels": {
                    "description": "Selection labels.",
                    "type": ["object", "null"],
                    "additionalProperties": {
                        "type": "string"
                    }
                },
                "annotations": {
                    "description": "Non-identifying metadata.",
                    "type": ["object", "null"],
                    "additionalProperties": {
                        "type": "string"
                    }
                },
                "provider": {
                    "description": "Provider-specific lowering overrides."
                },
                "workloads": {
                    "description": "Workloads behind this service.\n\nExactly one of `workloads` or `url` should be set.",
                    "type": ["array", "null"],
                    "items": {
                        "type": "string"
                    }
                },
                "url": {
                    "description": "External endpoint for unmanaged services.\n\nExactly one of `workloads` or `url` should be set.",
                    "type": ["string", "null"]
                },
                "protocol": {
                    "description": "Service protocol.",
                    "type": ["string", "null"]
                },
                "port": {
                    "description": "Externally visible service port.",
                    "type": ["integer", "null"],
                    "format": "uint16",
                    "minimum": 0,
                    "maximum": 65535
                },
                "targetPort": {
                    "description": "Target workload port.",
                    "type": ["integer", "null"],
                    "format": "uint16",
                    "minimum": 0,
                    "maximum": 65535
                },
                "placement": {
                    "description": "Placement constraints and preferences.",
                    "allOf": [placement_schema]
                },
                "access": {
                    "description": "Service access expectations.",
                    "allOf": [access_schema]
                },
                "config": {
                    "description": "Extra service metadata."
                }
            },
            "oneOf": [
                {
                    "required": ["workloads"],
                    "not": {
                        "required": ["url"]
                    },
                    "properties": {
                        "workloads": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            }
                        }
                    }
                },
                {
                    "required": ["url"],
                    "not": {
                        "required": ["workloads"]
                    },
                    "properties": {
                        "url": {
                            "type": "string"
                        }
                    }
                }
            ]
        })
    }
}
