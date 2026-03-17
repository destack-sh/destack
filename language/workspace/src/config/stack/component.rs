use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use super::common::merge_metadata;
use super::workload::{StackDeliveryJson, StackTriggerJson};

#[cfg(feature = "schema")]
use std::borrow::Cow;

#[cfg(feature = "schema")]
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};

/// Builtin client component type id.
pub const DESTACK_COMPONENT_CLIENT: &str = "destack.sh/component/client";
/// Builtin service component type id.
pub const DESTACK_COMPONENT_SERVICE: &str = "destack.sh/component/service";
/// Builtin worker component type id.
pub const DESTACK_COMPONENT_WORKER: &str = "destack.sh/component/worker";
/// Builtin job component type id.
pub const DESTACK_COMPONENT_JOB: &str = "destack.sh/component/job";

/// Builtin component archetype.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackBuiltinComponentKind {
    /// User-facing client surface.
    Client,
    /// Long-lived bindable service.
    Service,
    /// Long-lived background worker.
    Worker,
    /// Bounded one-off or scheduled job.
    Job,
}

impl StackBuiltinComponentKind {
    /// Resolve one builtin component kind from one type id.
    pub fn from_type_id(type_id: &str) -> Option<Self> {
        match type_id {
            DESTACK_COMPONENT_CLIENT => Some(Self::Client),
            DESTACK_COMPONENT_SERVICE => Some(Self::Service),
            DESTACK_COMPONENT_WORKER => Some(Self::Worker),
            DESTACK_COMPONENT_JOB => Some(Self::Job),
            _ => None,
        }
    }
}

/// Typed builtin component parameters.
#[derive(Debug, Clone)]
pub enum StackBuiltinComponentParameters {
    /// Client component parameters.
    Client(StackClientComponentJson),
    /// Service component parameters.
    Service(StackServiceComponentJson),
    /// Worker component parameters.
    Worker(StackWorkerComponentJson),
    /// Job component parameters.
    Job(StackJobComponentJson),
}

/// Stack component options.
#[derive(Debug, Clone, Default)]
pub struct StackComponentOptions {
    /// Open component type identifier.
    pub r#type: Option<String>,
    /// Optional component version selector.
    pub version: Option<String>,
    /// Typed component inputs.
    pub with: Option<Value>,
    /// Selection labels.
    pub labels: IndexMap<String, String>,
    /// Non-identifying metadata.
    pub annotations: IndexMap<String, String>,
    /// Provider-specific lowering overrides.
    pub provider: Option<Value>,
}

impl StackComponentOptions {
    /// Inherit unset component settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.r#type.is_none() {
            self.r#type = parent.r#type.clone();
        }
        if self.version.is_none() {
            self.version = parent.version.clone();
        }
        if self.with.is_none() {
            self.with = parent.with.clone();
        }
        if self.provider.is_none() {
            self.provider = parent.provider.clone();
        }

        merge_metadata(&mut self.labels, &parent.labels);
        merge_metadata(&mut self.annotations, &parent.annotations);
    }

    /// Parse builtin component parameters when this component uses one blessed type id.
    pub fn builtin_parameters(
        &self,
    ) -> Result<Option<StackBuiltinComponentParameters>, serde_json::Error> {
        let Some(type_id) = self.r#type.as_deref() else {
            return Ok(None);
        };
        let Some(with_json) = self.with.clone() else {
            return Ok(None);
        };

        let builtin = match StackBuiltinComponentKind::from_type_id(type_id) {
            Some(builtin) => builtin,
            None => return Ok(None),
        };

        let builtin = match builtin {
            StackBuiltinComponentKind::Client => {
                StackBuiltinComponentParameters::Client(serde_json::from_value(with_json)?)
            }
            StackBuiltinComponentKind::Service => {
                StackBuiltinComponentParameters::Service(serde_json::from_value(with_json)?)
            }
            StackBuiltinComponentKind::Worker => {
                StackBuiltinComponentParameters::Worker(serde_json::from_value(with_json)?)
            }
            StackBuiltinComponentKind::Job => {
                StackBuiltinComponentParameters::Job(serde_json::from_value(with_json)?)
            }
        };

        Ok(Some(builtin))
    }
}

impl From<&StackComponentJson> for StackComponentOptions {
    fn from(json: &StackComponentJson) -> Self {
        Self {
            r#type: json.r#type.clone(),
            version: json.version.clone(),
            with: json.with.clone(),
            labels: json.labels.clone().unwrap_or_default(),
            annotations: json.annotations.clone().unwrap_or_default(),
            provider: json.provider.clone(),
        }
    }
}

/// Builtin client component parameters.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackClientComponentJson {
    /// Primary client target.
    pub target: Option<String>,
    /// Optional public domain for the lowered client service.
    pub domain: Option<String>,
    /// Optional route match for the lowered ingress rule.
    pub r#match: Option<String>,
    /// Optional backing service bound into the client.
    pub service: Option<String>,
    /// Optional public environment binding name for the bound service url.
    pub service_env: Option<String>,
    /// Static asset directory or asset binding.
    pub assets: Option<String>,
    /// Extra component metadata.
    pub config: Option<Value>,
}

/// Builtin service component parameters.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackServiceComponentJson {
    /// Primary service target.
    pub target: Option<String>,
    /// Optional semantic service type id.
    pub r#type: Option<String>,
    /// Optional service protocol.
    pub protocol: Option<String>,
    /// Optional workload mount or handler path.
    pub mount: Option<String>,
    /// Optional public domain for ingress.
    pub domain: Option<String>,
    /// Optional route match for the lowered ingress rule.
    pub r#match: Option<String>,
    /// Optional target port override.
    pub target_port: Option<u16>,
    /// Extra component metadata.
    pub config: Option<Value>,
}

/// Builtin worker component parameters.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackWorkerComponentJson {
    /// Primary worker target.
    pub target: Option<String>,
    /// Optional trigger configuration.
    #[serde(default)]
    pub trigger: StackTriggerJson,
    /// Optional delivery configuration.
    #[serde(default)]
    pub delivery: StackDeliveryJson,
    /// Extra component metadata.
    pub config: Option<Value>,
}

/// Builtin job component parameters.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackJobComponentJson {
    /// Primary job target.
    pub target: Option<String>,
    /// Optional trigger configuration.
    #[serde(default)]
    pub trigger: StackTriggerJson,
    /// Optional delivery configuration.
    #[serde(default)]
    pub delivery: StackDeliveryJson,
    /// Extra component metadata.
    pub config: Option<Value>,
}

/// A typed graph expansion.
///
/// Inputs: component type, optional version, and typed `with` parameters.
/// Outputs: named graph nodes in the lowered stack.
#[derive(Debug, Default, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StackComponentJson {
    /// Open component type identifier.
    ///
    /// Builtin archetypes currently blessed by Destack are:
    /// `destack.sh/component/client`,
    /// `destack.sh/component/service`,
    /// `destack.sh/component/worker`,
    /// and `destack.sh/component/job`.
    pub r#type: Option<String>,
    /// Optional component version selector.
    pub version: Option<String>,
    /// Typed component inputs.
    ///
    /// For builtin archetypes this object should match the corresponding
    /// client, service, worker, or job parameter shape.
    /// Runtime and platform remain target-scoped rather than component-scoped.
    pub with: Option<Value>,
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
    /// Provider-specific lowering overrides.
    pub provider: Option<Value>,
}

#[cfg(feature = "schema")]
impl JsonSchema for StackComponentJson {
    fn schema_name() -> Cow<'static, str> {
        "StackComponentJson".into()
    }

    fn schema_id() -> Cow<'static, str> {
        concat!(module_path!(), "::StackComponentJson").into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        let client_with = generator.subschema_for::<StackClientComponentJson>();
        let service_with = generator.subschema_for::<StackServiceComponentJson>();
        let worker_with = generator.subschema_for::<StackWorkerComponentJson>();
        let job_with = generator.subschema_for::<StackJobComponentJson>();

        json_schema!({
            "description": "A typed graph expansion.\n\nInputs: component type, optional version, and typed `with` parameters.\nOutputs: named graph nodes in the lowered stack.",
            "type": "object",
            "properties": {
                "type": {
                    "description": "Open component type identifier.\n\nBuiltin archetypes currently blessed by Destack are:\n`destack.sh/component/client`,\n`destack.sh/component/service`,\n`destack.sh/component/worker`,\nand `destack.sh/component/job`.",
                    "type": ["string", "null"]
                },
                "version": {
                    "description": "Optional component version selector.",
                    "type": ["string", "null"]
                },
                "with": {
                    "description": "Typed component inputs.\n\nFor builtin archetypes this object should match the corresponding\nclient, service, worker, or job parameter shape.\nRuntime and platform remain target-scoped rather than component-scoped."
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
                }
            },
            "oneOf": [
                {
                    "properties": {
                        "type": {
                            "const": DESTACK_COMPONENT_CLIENT
                        },
                        "with": client_with
                    }
                },
                {
                    "properties": {
                        "type": {
                            "const": DESTACK_COMPONENT_SERVICE
                        },
                        "with": service_with
                    }
                },
                {
                    "properties": {
                        "type": {
                            "const": DESTACK_COMPONENT_WORKER
                        },
                        "with": worker_with
                    }
                },
                {
                    "properties": {
                        "type": {
                            "const": DESTACK_COMPONENT_JOB
                        },
                        "with": job_with
                    }
                },
                {
                    "not": {
                        "properties": {
                            "type": {
                                "enum": [
                                    DESTACK_COMPONENT_CLIENT,
                                    DESTACK_COMPONENT_SERVICE,
                                    DESTACK_COMPONENT_WORKER,
                                    DESTACK_COMPONENT_JOB
                                ]
                            }
                        },
                        "required": ["type"]
                    },
                    "properties": {
                        "with": {
                            "type": ["object", "null"]
                        }
                    }
                }
            ]
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        DESTACK_COMPONENT_CLIENT, DESTACK_COMPONENT_JOB, DESTACK_COMPONENT_SERVICE,
        DESTACK_COMPONENT_WORKER, StackBuiltinComponentParameters, StackComponentJson,
        StackComponentOptions,
    };

    /// Parse builtin client component parameters.
    #[test]
    fn test_component_builtin_parameters_parse_client() {
        let json: StackComponentJson = serde_json::from_value(json!({
            "type": DESTACK_COMPONENT_CLIENT,
            "with": {
                "target": "web",
                "domain": "app.example.com",
                "service": "api",
                "serviceEnv": "PUBLIC_API_ORIGIN"
            }
        }))
        .expect("component should parse");

        let component = StackComponentOptions::from(&json);
        let builtin = component
            .builtin_parameters()
            .expect("builtin parse should succeed")
            .expect("builtin params should exist");

        let StackBuiltinComponentParameters::Client(client) = builtin else {
            panic!("expected client builtin parameters");
        };

        assert_eq!(client.target.as_deref(), Some("web"));
        assert_eq!(client.domain.as_deref(), Some("app.example.com"));
        assert_eq!(client.service.as_deref(), Some("api"));
        assert_eq!(client.service_env.as_deref(), Some("PUBLIC_API_ORIGIN"));
    }

    /// Parse builtin service component parameters.
    #[test]
    fn test_component_builtin_parameters_parse_service() {
        let json: StackComponentJson = serde_json::from_value(json!({
            "type": DESTACK_COMPONENT_SERVICE,
            "with": {
                "target": "api",
                "type": "destack.sh/service/http",
                "protocol": "http",
                "mount": "/api"
            }
        }))
        .expect("component should parse");

        let component = StackComponentOptions::from(&json);
        let builtin = component
            .builtin_parameters()
            .expect("builtin parse should succeed")
            .expect("builtin params should exist");

        let StackBuiltinComponentParameters::Service(service) = builtin else {
            panic!("expected service builtin parameters");
        };

        assert_eq!(service.target.as_deref(), Some("api"));
        assert_eq!(service.r#type.as_deref(), Some("destack.sh/service/http"));
        assert_eq!(service.protocol.as_deref(), Some("http"));
        assert_eq!(service.mount.as_deref(), Some("/api"));
    }

    /// Parse builtin worker component parameters.
    #[test]
    fn test_component_builtin_parameters_parse_worker() {
        let json: StackComponentJson = serde_json::from_value(json!({
            "type": DESTACK_COMPONENT_WORKER,
            "with": {
                "target": "worker",
                "trigger": {
                    "kind": "destack.sh/trigger/queue",
                    "service": "emailQueue"
                },
                "delivery": {
                    "batch": {
                        "maxSize": 100
                    }
                }
            }
        }))
        .expect("component should parse");

        let component = StackComponentOptions::from(&json);
        let builtin = component
            .builtin_parameters()
            .expect("builtin parse should succeed")
            .expect("builtin params should exist");

        let StackBuiltinComponentParameters::Worker(worker) = builtin else {
            panic!("expected worker builtin parameters");
        };

        assert_eq!(worker.target.as_deref(), Some("worker"));
        assert_eq!(
            worker.trigger.kind.as_deref(),
            Some("destack.sh/trigger/queue")
        );
        assert_eq!(worker.trigger.service.as_deref(), Some("emailQueue"));
        assert_eq!(worker.delivery.batch.max_size, Some(100));
    }

    /// Parse builtin job component parameters.
    #[test]
    fn test_component_builtin_parameters_parse_job() {
        let json: StackComponentJson = serde_json::from_value(json!({
            "type": DESTACK_COMPONENT_JOB,
            "with": {
                "target": "reconcile",
                "trigger": {
                    "kind": "destack.sh/trigger/schedule",
                    "schedule": "0 * * * *"
                }
            }
        }))
        .expect("component should parse");

        let component = StackComponentOptions::from(&json);
        let builtin = component
            .builtin_parameters()
            .expect("builtin parse should succeed")
            .expect("builtin params should exist");

        let StackBuiltinComponentParameters::Job(job) = builtin else {
            panic!("expected job builtin parameters");
        };

        assert_eq!(job.target.as_deref(), Some("reconcile"));
        assert_eq!(
            job.trigger.kind.as_deref(),
            Some("destack.sh/trigger/schedule")
        );
        assert_eq!(job.trigger.schedule.as_deref(), Some("0 * * * *"));
    }
}
