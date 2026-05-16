use serde::{Deserialize, Serialize};

/// Runtime worker configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct WorkerOptions {
    /// Maximum number of live workers in one runtime.
    pub limit: Option<u64>,
    /// Maximum number of host threads allocated to workers.
    pub thread_limit: Option<u64>,
    /// Stack reservation in bytes for one worker.
    pub stack_bytes: Option<u64>,
}
