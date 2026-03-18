use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

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
        match json {
            StackIssuerRefJson::Service(reference) => Self {
                service: Some(reference.service.clone()),
                url: None,
            },
            StackIssuerRefJson::Url(reference) => Self {
                service: None,
                url: Some(reference.url.clone()),
            },
        }
    }
}

/// Provider attachment options.
#[derive(Debug, Clone, Default)]
pub struct StackProviderOptions {
    /// Provider identifier.
    pub name: Option<String>,
    /// Control-plane account reference.
    pub account: Option<String>,
    /// Extra provider-specific arguments.
    pub with: Option<Value>,
}

impl StackProviderOptions {
    /// Inherit unset provider settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.name.is_none() {
            self.name = parent.name.clone();
        }
        if self.account.is_none() {
            self.account = parent.account.clone();
        }
        if self.with.is_none() {
            self.with = parent.with.clone();
        }
    }
}

impl From<&StackProviderJson> for StackProviderOptions {
    fn from(json: &StackProviderJson) -> Self {
        match json {
            StackProviderJson::Name(name) => Self {
                name: Some(name.clone()),
                account: None,
                with: None,
            },
            StackProviderJson::Options(options) => Self {
                name: options.name.clone(),
                account: options.account.clone(),
                with: options.with.clone(),
            },
        }
    }
}

/// Cache mode options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackCacheMode {
    /// Public cacheable content.
    Public,
    /// Private cacheable content.
    Private,
    /// Bypass cache storage.
    NoStore,
    /// Revalidate on every use.
    NoCache,
}

impl From<StackCacheModeJson> for StackCacheMode {
    fn from(json: StackCacheModeJson) -> Self {
        match json {
            StackCacheModeJson::Public => Self::Public,
            StackCacheModeJson::Private => Self::Private,
            StackCacheModeJson::NoStore => Self::NoStore,
            StackCacheModeJson::NoCache => Self::NoCache,
        }
    }
}

/// Provider attachment JSON.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum StackProviderJson {
    /// One shorthand provider identifier.
    Name(String),
    /// One structured provider attachment.
    Options(StackProviderAttachmentJson),
}

/// Structured provider attachment JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackProviderAttachmentJson {
    /// Provider identifier.
    pub name: Option<String>,
    /// Control-plane account reference.
    pub account: Option<String>,
    /// Extra provider-specific arguments.
    pub with: Option<Value>,
}

/// Cache policy options.
#[derive(Debug, Clone, Default)]
pub struct StackCacheOptions {
    /// Cache visibility mode.
    pub mode: Option<StackCacheMode>,
    /// Default max age.
    pub max_age: Option<String>,
    /// Shared cache max age.
    pub shared_max_age: Option<String>,
    /// Stale while revalidate lifetime.
    pub stale_while_revalidate: Option<String>,
    /// Stale if error lifetime.
    pub stale_if_error: Option<String>,
    /// Whether cached content is immutable.
    pub immutable: Option<bool>,
    /// Cache variation keys.
    pub vary: Vec<String>,
}

impl StackCacheOptions {
    /// Inherit unset cache settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.mode.is_none() {
            self.mode = parent.mode;
        }
        if self.max_age.is_none() {
            self.max_age = parent.max_age.clone();
        }
        if self.shared_max_age.is_none() {
            self.shared_max_age = parent.shared_max_age.clone();
        }
        if self.stale_while_revalidate.is_none() {
            self.stale_while_revalidate = parent.stale_while_revalidate.clone();
        }
        if self.stale_if_error.is_none() {
            self.stale_if_error = parent.stale_if_error.clone();
        }
        if self.immutable.is_none() {
            self.immutable = parent.immutable;
        }
        if self.vary.is_empty() {
            self.vary = parent.vary.clone();
        }
    }
}

impl From<&StackCacheJson> for StackCacheOptions {
    fn from(json: &StackCacheJson) -> Self {
        Self {
            mode: json.mode.map(StackCacheMode::from),
            max_age: json.max_age.clone(),
            shared_max_age: json.shared_max_age.clone(),
            stale_while_revalidate: json.stale_while_revalidate.clone(),
            stale_if_error: json.stale_if_error.clone(),
            immutable: json.immutable,
            vary: json.vary.clone().unwrap_or_default(),
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

/// Cache mode JSON.
#[derive(Debug, Deserialize, Clone, Copy)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum StackCacheModeJson {
    /// Public cacheable content.
    Public,
    /// Private cacheable content.
    Private,
    /// Bypass cache storage.
    NoStore,
    /// Revalidate on every use.
    NoCache,
}

/// Cache policy JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackCacheJson {
    /// Cache visibility mode.
    pub mode: Option<StackCacheModeJson>,
    /// Default max age.
    pub max_age: Option<String>,
    /// Shared cache max age.
    pub shared_max_age: Option<String>,
    /// Stale while revalidate lifetime.
    pub stale_while_revalidate: Option<String>,
    /// Stale if error lifetime.
    pub stale_if_error: Option<String>,
    /// Whether cached content is immutable.
    pub immutable: Option<bool>,
    /// Cache variation keys.
    pub vary: Option<Vec<String>>,
}

/// Issuer reference JSON.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum StackIssuerRefJson {
    /// Internal issuer service reference.
    Service(StackIssuerServiceRefJson),
    /// External issuer URL reference.
    Url(StackIssuerUrlRefJson),
}

/// Internal issuer service reference JSON.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackIssuerServiceRefJson {
    /// Internal issuer service name.
    pub service: String,
}

/// External issuer URL reference JSON.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackIssuerUrlRefJson {
    /// External issuer URL.
    pub url: String,
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
