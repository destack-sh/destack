use serde::Deserialize;
use serde_json::Value;

use crate::config::target::TargetOutputName;

/// Stack attachment source kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum StackAttachmentSourceKind {
    /// Read from one named workload.
    Workload,
    /// Read from one named service.
    Service,
    /// Read from one named config.
    Config,
    /// Read from one named secret.
    Secret,
    /// Read from one named volume.
    Volume,
    /// Read from one named asset collection.
    Asset,
    /// Read from one named target.
    Target,
}

/// Stack attachment source options.
#[derive(Debug, Clone)]
pub struct StackAttachmentSourceOptions {
    /// Attachment source kind.
    pub kind: StackAttachmentSourceKind,
    /// Source locator, usually one named stack or target object.
    pub locator: Option<String>,
    /// Optional named output within the source object.
    pub output: Option<String>,
    /// Optional named field or member within the source object.
    pub field: Option<String>,
    /// Extra source arguments.
    pub with: Option<Value>,
}

impl StackAttachmentSourceOptions {
    /// Inherit unset source settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.locator.is_none() {
            self.locator = parent.locator.clone();
        }

        if self.output.is_none() {
            self.output = parent.output.clone();
        }

        if self.field.is_none() {
            self.field = parent.field.clone();
        }

        if self.with.is_none() {
            self.with = parent.with.clone();
        }
    }
}

impl From<&StackAttachmentSourceJson> for StackAttachmentSourceOptions {
    fn from(json: &StackAttachmentSourceJson) -> Self {
        match json {
            StackAttachmentSourceJson::Workload {
                workload,
                field,
                with,
            } => Self {
                kind: StackAttachmentSourceKind::Workload,
                locator: Some(workload.clone()),
                output: None,
                field: field.clone(),
                with: with.clone(),
            },
            StackAttachmentSourceJson::Service {
                service,
                field,
                with,
            } => Self {
                kind: StackAttachmentSourceKind::Service,
                locator: Some(service.clone()),
                output: None,
                field: field.clone(),
                with: with.clone(),
            },
            StackAttachmentSourceJson::Config {
                config,
                field,
                with,
            } => Self {
                kind: StackAttachmentSourceKind::Config,
                locator: Some(config.clone()),
                output: None,
                field: field.clone(),
                with: with.clone(),
            },
            StackAttachmentSourceJson::Secret {
                secret,
                field,
                with,
            } => Self {
                kind: StackAttachmentSourceKind::Secret,
                locator: Some(secret.clone()),
                output: None,
                field: field.clone(),
                with: with.clone(),
            },
            StackAttachmentSourceJson::Volume {
                volume,
                field,
                with,
            } => Self {
                kind: StackAttachmentSourceKind::Volume,
                locator: Some(volume.clone()),
                output: None,
                field: field.clone(),
                with: with.clone(),
            },
            StackAttachmentSourceJson::Asset {
                asset,
                output,
                with,
            } => Self {
                kind: StackAttachmentSourceKind::Asset,
                locator: Some(asset.clone()),
                output: output.clone(),
                field: None,
                with: with.clone(),
            },
            StackAttachmentSourceJson::Target {
                target,
                output,
                with,
            } => Self {
                kind: StackAttachmentSourceKind::Target,
                locator: Some(target.clone()),
                output: output.map(|output| output.as_str().to_string()),
                field: None,
                with: with.clone(),
            },
        }
    }
}

/// Stack attachment source JSON.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StackAttachmentSourceJson {
    /// Read from one named workload.
    Workload {
        /// Workload name.
        workload: String,
        /// Optional named field or member within the source object.
        field: Option<String>,
        /// Extra source arguments.
        with: Option<Value>,
    },
    /// Read from one named service.
    Service {
        /// Service name.
        service: String,
        /// Optional named field or member within the source object.
        field: Option<String>,
        /// Extra source arguments.
        with: Option<Value>,
    },
    /// Read from one named config.
    Config {
        /// Config name.
        config: String,
        /// Optional named field or member within the source object.
        field: Option<String>,
        /// Extra source arguments.
        with: Option<Value>,
    },
    /// Read from one named secret.
    Secret {
        /// Secret name.
        secret: String,
        /// Optional named field or member within the source object.
        field: Option<String>,
        /// Extra source arguments.
        with: Option<Value>,
    },
    /// Read from one named volume.
    Volume {
        /// Volume name.
        volume: String,
        /// Optional named field or member within the source object.
        field: Option<String>,
        /// Extra source arguments.
        with: Option<Value>,
    },
    /// Read from one named asset collection.
    Asset {
        /// Asset name.
        asset: String,
        /// Optional named output within the source object.
        output: Option<String>,
        /// Extra source arguments.
        with: Option<Value>,
    },
    /// Read from one named target.
    Target {
        /// Target name.
        target: String,
        /// Optional named output within the source object.
        output: Option<TargetOutputName>,
        /// Extra source arguments.
        with: Option<Value>,
    },
}
