use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Named toolchain or shell invocation.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    /// Toolchain command name to run.
    pub run: Option<String>,
    /// Explicit shell command to execute.
    pub exec: Option<String>,
    /// Build target selected for this task.
    pub target: Option<String>,
    /// Product selected for this task.
    pub product: Option<String>,
    /// Profile selected for this task.
    pub profile: Option<String>,
    /// Active source graph modes added by this task.
    pub modes: Vec<String>,
    /// Active source graph roles added by this task.
    pub roles: Vec<String>,
    /// Active source graph features added by this task.
    pub features: Vec<String>,
    /// Active source graph tags added by this task.
    pub tags: Vec<String>,
    /// Environment variables passed to this task.
    pub env: IndexMap<String, String>,
    /// Structured command arguments.
    pub arguments: IndexMap<String, Value>,
    /// Tasks that must complete before this task.
    pub depends_on: Vec<String>,
}
