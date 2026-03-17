use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use super::common::merge_metadata;
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
            config: json.config.clone(),
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
    /// Extra service metadata.
    pub config: Option<Value>,
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
