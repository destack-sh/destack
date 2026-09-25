use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tspp_artifact::{BuildLinkage, BuildProfile};
use tspp_serde::Reflect;

use super::{HeapOptions, HostOptions, RuntimeDiagnosticOptions, WorkerOptions};

/// Runtime configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeOptions {
    /// Runtime topology and policy identity.
    pub identity: RuntimeIdentityOptions,
    /// Build distribution profile.
    pub profile: BuildProfile,
    /// Build payload linkage.
    pub linkage: BuildLinkage,
    /// Runtime worker configuration.
    pub worker: WorkerOptions,
    /// Runtime heap configuration.
    pub heap: HeapOptions,
    /// Runtime diagnostics configuration.
    pub diagnostic: RuntimeDiagnosticOptions,
    /// Runtime host module defaults.
    pub host: HostOptions,
}

/// Runtime identity used for topology and policy selection.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeIdentityOptions {
    /// Stable runtime name for policy selection.
    pub name: Option<String>,
    /// Stable runtime labels for topology and policy selection.
    pub labels: BTreeMap<String, String>,
}
