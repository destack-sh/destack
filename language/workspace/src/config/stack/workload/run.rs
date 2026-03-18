use serde::Deserialize;
use serde_json::Value;

use super::super::asset::{StackAssetsJson, StackAssetsOptions};
use super::super::common::{StackRunKind, StackRunKindJson};

/// Builtin queue trigger kind id.
pub const DESTACK_TRIGGER_QUEUE: &str = "destack.sh/trigger/queue";
/// Builtin schedule trigger kind id.
pub const DESTACK_TRIGGER_SCHEDULE: &str = "destack.sh/trigger/schedule";
/// Builtin manual trigger kind id.
pub const DESTACK_TRIGGER_MANUAL: &str = "destack.sh/trigger/manual";

/// Workload run configuration options.
#[derive(Debug, Clone, Default)]
pub struct StackRunOptions {
    /// Run kind.
    pub kind: Option<StackRunKind>,
    /// Run protocol.
    pub protocol: Option<String>,
    /// Optional mount path or handler path.
    pub mount: Option<String>,
    /// Static asset source or asset publishing configuration.
    pub assets: StackAssetsOptions,
    /// Trigger configuration.
    pub trigger: StackTriggerOptions,
    /// Delivery configuration.
    pub delivery: StackDeliveryOptions,
    /// Extra run arguments.
    pub with: Option<Value>,
}

impl StackRunOptions {
    /// Inherit unset run settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.kind.is_none() {
            self.kind = parent.kind;
        }
        if self.protocol.is_none() {
            self.protocol = parent.protocol.clone();
        }
        if self.mount.is_none() {
            self.mount = parent.mount.clone();
        }
        if self.with.is_none() {
            self.with = parent.with.clone();
        }

        self.assets.extend_from(&parent.assets);
        self.trigger.extend_from(&parent.trigger);
        self.delivery.extend_from(&parent.delivery);
    }
}

impl From<&StackRunJson> for StackRunOptions {
    fn from(json: &StackRunJson) -> Self {
        Self {
            kind: json.kind.map(StackRunKind::from),
            protocol: json.protocol.clone(),
            mount: json.mount.clone(),
            assets: json
                .assets
                .as_ref()
                .map(StackAssetsOptions::from)
                .unwrap_or_default(),
            trigger: StackTriggerOptions::from(&json.trigger),
            delivery: StackDeliveryOptions::from(&json.delivery),
            with: json.with.clone(),
        }
    }
}

/// Trigger configuration options.
#[derive(Debug, Clone, Default)]
pub struct StackTriggerOptions {
    /// Open trigger kind identifier.
    pub kind: Option<String>,
    /// Upstream service that activates this workload.
    pub service: Option<String>,
    /// Cron schedule for scheduled workloads.
    pub schedule: Option<String>,
}

impl StackTriggerOptions {
    /// Inherit unset trigger settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.kind.is_none() {
            self.kind = parent.kind.clone();
        }
        if self.service.is_none() {
            self.service = parent.service.clone();
        }
        if self.schedule.is_none() {
            self.schedule = parent.schedule.clone();
        }
    }
}

impl From<&StackTriggerJson> for StackTriggerOptions {
    fn from(json: &StackTriggerJson) -> Self {
        Self {
            kind: json.kind.clone(),
            service: json.service.clone(),
            schedule: json.schedule.clone(),
        }
    }
}

/// Delivery configuration options.
#[derive(Debug, Clone, Default)]
pub struct StackDeliveryOptions {
    /// Batch delivery settings.
    pub batch: StackBatchOptions,
    /// Retry delivery settings.
    pub retry: StackRetryOptions,
    /// Visibility timeout in seconds.
    pub visibility_timeout_seconds: Option<u64>,
}

impl StackDeliveryOptions {
    /// Inherit unset delivery settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.visibility_timeout_seconds.is_none() {
            self.visibility_timeout_seconds = parent.visibility_timeout_seconds;
        }

        self.batch.extend_from(&parent.batch);
        self.retry.extend_from(&parent.retry);
    }
}

impl From<&StackDeliveryJson> for StackDeliveryOptions {
    fn from(json: &StackDeliveryJson) -> Self {
        Self {
            batch: StackBatchOptions::from(&json.batch),
            retry: StackRetryOptions::from(&json.retry),
            visibility_timeout_seconds: json.visibility_timeout_seconds,
        }
    }
}

/// Batch delivery options.
#[derive(Debug, Clone, Default)]
pub struct StackBatchOptions {
    /// Maximum batch size.
    pub max_size: Option<u64>,
    /// Maximum batch wait in seconds.
    pub max_wait_seconds: Option<u64>,
}

impl StackBatchOptions {
    /// Inherit unset batch settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.max_size.is_none() {
            self.max_size = parent.max_size;
        }
        if self.max_wait_seconds.is_none() {
            self.max_wait_seconds = parent.max_wait_seconds;
        }
    }
}

impl From<&StackBatchJson> for StackBatchOptions {
    fn from(json: &StackBatchJson) -> Self {
        Self {
            max_size: json.max_size,
            max_wait_seconds: json.max_wait_seconds,
        }
    }
}

/// Retry delivery options.
#[derive(Debug, Clone, Default)]
pub struct StackRetryOptions {
    /// Maximum delivery attempts.
    pub max_attempts: Option<u64>,
    /// Retry backoff in seconds.
    pub backoff_seconds: Option<u64>,
}

impl StackRetryOptions {
    /// Inherit unset retry settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.max_attempts.is_none() {
            self.max_attempts = parent.max_attempts;
        }
        if self.backoff_seconds.is_none() {
            self.backoff_seconds = parent.backoff_seconds;
        }
    }
}

impl From<&StackRetryJson> for StackRetryOptions {
    fn from(json: &StackRetryJson) -> Self {
        Self {
            max_attempts: json.max_attempts,
            backoff_seconds: json.backoff_seconds,
        }
    }
}

/// Workload run configuration JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackRunJson {
    /// Run kind.
    pub kind: Option<StackRunKindJson>,
    /// Run protocol.
    pub protocol: Option<String>,
    /// Optional mount path or handler path.
    pub mount: Option<String>,
    /// Static asset source or asset publishing configuration.
    pub assets: Option<StackAssetsJson>,
    /// Trigger configuration.
    #[serde(default)]
    pub trigger: StackTriggerJson,
    /// Delivery configuration.
    #[serde(default)]
    pub delivery: StackDeliveryJson,
    /// Extra run arguments.
    pub with: Option<Value>,
}

/// Trigger configuration JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackTriggerJson {
    /// Open trigger kind identifier.
    pub kind: Option<String>,
    /// Upstream service that activates this workload.
    pub service: Option<String>,
    /// Cron schedule for scheduled workloads.
    pub schedule: Option<String>,
}

/// Delivery configuration JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackDeliveryJson {
    /// Batch delivery settings.
    #[serde(default)]
    pub batch: StackBatchJson,
    /// Retry delivery settings.
    #[serde(default)]
    pub retry: StackRetryJson,
    /// Visibility timeout in seconds.
    pub visibility_timeout_seconds: Option<u64>,
}

/// Batch delivery JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackBatchJson {
    /// Maximum batch size.
    pub max_size: Option<u64>,
    /// Maximum batch wait in seconds.
    pub max_wait_seconds: Option<u64>,
}

/// Retry delivery JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackRetryJson {
    /// Maximum delivery attempts.
    pub max_attempts: Option<u64>,
    /// Retry backoff in seconds.
    pub backoff_seconds: Option<u64>,
}
