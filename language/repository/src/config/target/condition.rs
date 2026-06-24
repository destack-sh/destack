use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::Stage;

/// Target contribution to the active source graph condition set.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct TargetConditionSet {
    /// Explicit profile name for this target.
    pub profile: Option<String>,
    /// Release stage for this target.
    pub stage: Option<Stage>,
    /// Active source graph modes for this target.
    pub modes: Vec<String>,
    /// Active source graph roles for this target.
    pub roles: Vec<String>,
    /// Active optional features for this target.
    pub features: Vec<String>,
    /// Active source graph tags for this target.
    pub tags: Vec<String>,
}
