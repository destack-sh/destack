use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use super::common::merge_metadata;

/// Volume options.
#[derive(Debug, Clone, Default)]
pub struct StackVolumeOptions {
    /// Capacity or size hint.
    pub size: Option<String>,
    /// Storage class or tier.
    pub class: Option<String>,
    /// Access mode for this volume.
    pub access: Option<String>,
    /// Retention policy for this volume.
    pub retention: Option<String>,
    /// Backup policy for this volume.
    pub backup: StackVolumeBackupOptions,
    /// Selection labels.
    pub labels: IndexMap<String, String>,
    /// Non-identifying metadata.
    pub annotations: IndexMap<String, String>,
    /// Provider-specific lowering overrides.
    pub provider: Option<Value>,
    /// Extra volume metadata.
    pub config: Option<Value>,
}

impl StackVolumeOptions {
    /// Inherit unset volume settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.size.is_none() {
            self.size = parent.size.clone();
        }
        if self.class.is_none() {
            self.class = parent.class.clone();
        }
        if self.access.is_none() {
            self.access = parent.access.clone();
        }
        if self.retention.is_none() {
            self.retention = parent.retention.clone();
        }
        if self.provider.is_none() {
            self.provider = parent.provider.clone();
        }
        if self.config.is_none() {
            self.config = parent.config.clone();
        }
        self.backup.extend_from(&parent.backup);

        merge_metadata(&mut self.labels, &parent.labels);
        merge_metadata(&mut self.annotations, &parent.annotations);
    }
}

impl From<&StackVolumeJson> for StackVolumeOptions {
    fn from(json: &StackVolumeJson) -> Self {
        Self {
            size: json.size.clone(),
            class: json.class.clone(),
            access: json.access.clone(),
            retention: json.retention.clone(),
            backup: StackVolumeBackupOptions::from(&json.backup),
            labels: json.labels.clone().unwrap_or_default(),
            annotations: json.annotations.clone().unwrap_or_default(),
            provider: json.provider.clone(),
            config: json.config.clone(),
        }
    }
}

/// Volume backup policy options.
#[derive(Debug, Clone, Default)]
pub struct StackVolumeBackupOptions {
    /// Whether backups are enabled.
    pub enabled: Option<bool>,
    /// Backup schedule.
    pub schedule: Option<String>,
    /// Backup retention in days.
    pub retention_days: Option<u64>,
}

impl StackVolumeBackupOptions {
    /// Inherit unset backup settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.enabled.is_none() {
            self.enabled = parent.enabled;
        }
        if self.schedule.is_none() {
            self.schedule = parent.schedule.clone();
        }
        if self.retention_days.is_none() {
            self.retention_days = parent.retention_days;
        }
    }
}

impl From<&StackVolumeBackupJson> for StackVolumeBackupOptions {
    fn from(json: &StackVolumeBackupJson) -> Self {
        Self {
            enabled: json.enabled,
            schedule: json.schedule.clone(),
            retention_days: json.retention_days,
        }
    }
}

/// A passive mountable storage node.
///
/// Inputs: size, class, and provider config.
/// Outputs: mount handles for workloads.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackVolumeJson {
    /// Capacity or size hint.
    pub size: Option<String>,
    /// Storage class or tier.
    pub class: Option<String>,
    /// Access mode for this volume.
    pub access: Option<String>,
    /// Retention policy for this volume.
    pub retention: Option<String>,
    /// Backup policy for this volume.
    #[serde(default)]
    pub backup: StackVolumeBackupJson,
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
    /// Provider-specific lowering overrides.
    pub provider: Option<Value>,
    /// Extra volume metadata.
    pub config: Option<Value>,
}

/// Volume backup policy JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackVolumeBackupJson {
    /// Whether backups are enabled.
    pub enabled: Option<bool>,
    /// Backup schedule.
    pub schedule: Option<String>,
    /// Backup retention in days.
    pub retention_days: Option<u64>,
}
