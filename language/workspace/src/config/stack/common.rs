use indexmap::IndexMap;
use serde::Deserialize;

/// Default first-party issuer service name.
pub const DESTACK_ISSUER_UNIVERSE_SERVICE: &str = "universe";

/// Workload run kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackRunKind {
    /// Serve HTTP traffic.
    #[default]
    Http,
    /// Publish static assets.
    Static,
    /// Consume queue or stream messages.
    Consumer,
    /// Run on a cron-like schedule.
    Schedule,
    /// Run as a one-shot background job.
    Job,
}

impl From<StackRunKindJson> for StackRunKind {
    fn from(json: StackRunKindJson) -> Self {
        match json {
            StackRunKindJson::Http => Self::Http,
            StackRunKindJson::Static => Self::Static,
            StackRunKindJson::Consumer => Self::Consumer,
            StackRunKindJson::Schedule => Self::Schedule,
            StackRunKindJson::Job => Self::Job,
        }
    }
}

/// Scaling signal options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackScalingMetric {
    /// Scale from CPU pressure.
    #[default]
    Cpu,
    /// Scale from memory pressure.
    Memory,
    /// Scale from request latency.
    Latency,
    /// Scale from queue depth.
    QueueDepth,
    /// Scale from in-flight request count.
    Concurrency,
}

impl From<StackScalingMetricJson> for StackScalingMetric {
    fn from(json: StackScalingMetricJson) -> Self {
        match json {
            StackScalingMetricJson::Cpu => Self::Cpu,
            StackScalingMetricJson::Memory => Self::Memory,
            StackScalingMetricJson::Latency => Self::Latency,
            StackScalingMetricJson::QueueDepth => Self::QueueDepth,
            StackScalingMetricJson::Concurrency => Self::Concurrency,
        }
    }
}

/// Rollout strategy options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackRolloutStrategy {
    /// Replace instances gradually.
    #[default]
    Rolling,
    /// Replace all instances at once.
    Recreate,
    /// Shift traffic between blue and green pools.
    BlueGreen,
    /// Shift traffic incrementally.
    Canary,
}

impl From<StackRolloutStrategyJson> for StackRolloutStrategy {
    fn from(json: StackRolloutStrategyJson) -> Self {
        match json {
            StackRolloutStrategyJson::Rolling => Self::Rolling,
            StackRolloutStrategyJson::Recreate => Self::Recreate,
            StackRolloutStrategyJson::BlueGreen => Self::BlueGreen,
            StackRolloutStrategyJson::Canary => Self::Canary,
        }
    }
}

/// Restart policy options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackRestartPolicy {
    /// Never restart automatically.
    Never,
    /// Restart when the unit fails.
    #[default]
    OnFailure,
    /// Always restart.
    Always,
}

impl From<StackRestartPolicyJson> for StackRestartPolicy {
    fn from(json: StackRestartPolicyJson) -> Self {
        match json {
            StackRestartPolicyJson::Never => Self::Never,
            StackRestartPolicyJson::OnFailure => Self::OnFailure,
            StackRestartPolicyJson::Always => Self::Always,
        }
    }
}

/// TLS mode options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackTlsMode {
    /// Let the platform manage certificates automatically.
    #[default]
    Automatic,
    /// Certificates are provided explicitly.
    Manual,
    /// Disable TLS termination.
    Disabled,
}

impl From<StackTlsModeJson> for StackTlsMode {
    fn from(json: StackTlsModeJson) -> Self {
        match json {
            StackTlsModeJson::Automatic => Self::Automatic,
            StackTlsModeJson::Manual => Self::Manual,
            StackTlsModeJson::Disabled => Self::Disabled,
        }
    }
}

/// Access mode options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackAccessMode {
    /// No authentication is required.
    #[default]
    Public,
    /// Authentication is accepted when present.
    Optional,
    /// Authentication is required.
    Required,
}

impl From<StackAccessModeJson> for StackAccessMode {
    fn from(json: StackAccessModeJson) -> Self {
        match json {
            StackAccessModeJson::Public => Self::Public,
            StackAccessModeJson::Optional => Self::Optional,
            StackAccessModeJson::Required => Self::Required,
        }
    }
}

/// Issuer reference options.
#[derive(Debug, Clone, Default)]
pub struct StackIssuerRefOptions {
    /// Internal issuer service name.
    pub service: Option<String>,
    /// External issuer URL.
    pub url: Option<String>,
}

impl StackIssuerRefOptions {
    /// Inherit unset issuer settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.service.is_none() {
            self.service = parent.service.clone();
        }
        if self.url.is_none() {
            self.url = parent.url.clone();
        }
    }
}

impl From<&StackIssuerRefJson> for StackIssuerRefOptions {
    fn from(json: &StackIssuerRefJson) -> Self {
        Self {
            service: json.service.clone(),
            url: json.url.clone(),
        }
    }
}

/// Run kind JSON.
#[derive(Debug, Deserialize, Clone, Copy)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum StackRunKindJson {
    /// Serve HTTP traffic.
    Http,
    /// Publish static assets.
    Static,
    /// Consume queue or stream messages.
    Consumer,
    /// Run on a cron-like schedule.
    Schedule,
    /// Run as a one-shot background job.
    Job,
}

/// Scaling metric JSON.
#[derive(Debug, Deserialize, Clone, Copy)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum StackScalingMetricJson {
    /// Scale from CPU pressure.
    Cpu,
    /// Scale from memory pressure.
    Memory,
    /// Scale from request latency.
    Latency,
    /// Scale from queue depth.
    QueueDepth,
    /// Scale from in-flight request count.
    Concurrency,
}

/// Rollout strategy JSON.
#[derive(Debug, Deserialize, Clone, Copy)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum StackRolloutStrategyJson {
    /// Replace instances gradually.
    Rolling,
    /// Replace all instances at once.
    Recreate,
    /// Shift traffic between blue and green pools.
    BlueGreen,
    /// Shift traffic incrementally.
    Canary,
}

/// Restart policy JSON.
#[derive(Debug, Deserialize, Clone, Copy)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum StackRestartPolicyJson {
    /// Never restart automatically.
    Never,
    /// Restart when the unit fails.
    OnFailure,
    /// Always restart.
    Always,
}

/// TLS mode JSON.
#[derive(Debug, Deserialize, Clone, Copy)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum StackTlsModeJson {
    /// Let the platform manage certificates automatically.
    Automatic,
    /// Certificates are provided explicitly.
    Manual,
    /// Disable TLS termination.
    Disabled,
}

/// Access mode JSON.
#[derive(Debug, Deserialize, Clone, Copy)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum StackAccessModeJson {
    /// No authentication is required.
    Public,
    /// Authentication is accepted when present.
    Optional,
    /// Authentication is required.
    Required,
}

/// Issuer reference JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StackIssuerRefJson {
    /// Internal issuer service name.
    pub service: Option<String>,
    /// External issuer URL.
    pub url: Option<String>,
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for StackIssuerRefJson {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "StackIssuerRefJson".into()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        concat!(module_path!(), "::StackIssuerRefJson").into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        let _ = generator;

        schemars::json_schema!({
            "description": "Issuer reference JSON.",
            "type": "object",
            "properties": {
                "service": {
                    "description": "Internal issuer service name.",
                    "type": ["string", "null"]
                },
                "url": {
                    "description": "External issuer URL.",
                    "type": ["string", "null"]
                }
            },
            "oneOf": [
                {
                    "required": ["service"],
                    "not": {
                        "required": ["url"]
                    },
                    "properties": {
                        "service": {
                            "type": "string"
                        }
                    }
                },
                {
                    "required": ["url"],
                    "not": {
                        "required": ["service"]
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

/// Merge metadata maps with child precedence.
pub(super) fn merge_metadata(
    into: &mut IndexMap<String, String>,
    parent: &IndexMap<String, String>,
) {
    for (name, value) in parent {
        into.entry(name.clone()).or_insert_with(|| value.clone());
    }
}
