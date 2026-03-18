use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use super::asset::StackAssetRefsJson;
use super::common::{StackProviderJson, StackProviderOptions, merge_metadata};
use super::workload::{StackDeliveryJson, StackTriggerJson};

/// Builtin client component type id.
pub const DESTACK_COMPONENT_CLIENT: &str = "destack.sh/component/client";
/// Builtin service component type id.
pub const DESTACK_COMPONENT_SERVICE: &str = "destack.sh/component/service";
/// Builtin worker component type id.
pub const DESTACK_COMPONENT_WORKER: &str = "destack.sh/component/worker";
/// Builtin job component type id.
pub const DESTACK_COMPONENT_JOB: &str = "destack.sh/component/job";
/// Builtin site component type id.
pub const DESTACK_COMPONENT_SITE: &str = "destack.sh/component/site";

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
    /// Static or asset-backed site convenience.
    Site,
}

impl StackBuiltinComponentKind {
    /// Resolve one builtin component kind from one type id.
    pub fn from_type_id(type_id: &str) -> Option<Self> {
        match type_id {
            DESTACK_COMPONENT_CLIENT => Some(Self::Client),
            DESTACK_COMPONENT_SERVICE => Some(Self::Service),
            DESTACK_COMPONENT_WORKER => Some(Self::Worker),
            DESTACK_COMPONENT_JOB => Some(Self::Job),
            DESTACK_COMPONENT_SITE => Some(Self::Site),
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
    /// Site component parameters.
    Site(StackSiteComponentJson),
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
    /// Provider attachment.
    pub provider: StackProviderOptions,
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
        self.provider.extend_from(&parent.provider);
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
            StackBuiltinComponentKind::Site => {
                StackBuiltinComponentParameters::Site(serde_json::from_value(with_json)?)
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
            provider: json
                .provider
                .as_ref()
                .map(StackProviderOptions::from)
                .unwrap_or_default(),
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
    /// Optional public environment binding name for the bound service URL.
    pub service_env: Option<String>,
    /// Referenced asset collections.
    pub assets: Option<StackAssetRefsJson>,
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
    /// Extra component arguments.
    pub with: Option<Value>,
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
    /// Extra component arguments.
    pub with: Option<Value>,
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
    /// Extra component arguments.
    pub with: Option<Value>,
}

/// Builtin site component parameters.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackSiteComponentJson {
    /// Primary site target.
    pub target: Option<String>,
    /// Optional public domain for the lowered site service.
    pub domain: Option<String>,
    /// Optional route match for the lowered ingress rule.
    pub r#match: Option<String>,
    /// Referenced asset collections.
    pub assets: Option<StackAssetRefsJson>,
    /// Optional default site index document.
    pub index: Option<String>,
    /// Optional default site error document.
    pub error: Option<String>,
    /// Extra component arguments.
    pub with: Option<Value>,
}

/// A typed graph expansion.
///
/// Inputs: component type, optional version, and typed `with` parameters.
/// Outputs: named graph nodes in the lowered stack.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackComponentJson {
    /// Open component type identifier.
    ///
    /// Builtin component types currently blessed by Destack are:
    /// `destack.sh/component/client`,
    /// `destack.sh/component/service`,
    /// `destack.sh/component/worker`,
    /// `destack.sh/component/job`,
    /// and `destack.sh/component/site`.
    pub r#type: Option<String>,
    /// Optional component version selector.
    pub version: Option<String>,
    /// Typed component inputs.
    ///
    /// For builtin component types this object should match the corresponding
    /// client, service, worker, job, or site parameter shape.
    /// Runtime and platform remain target-scoped rather than component-scoped.
    pub with: Option<Value>,
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
    /// Provider attachment.
    pub provider: Option<StackProviderJson>,
}
