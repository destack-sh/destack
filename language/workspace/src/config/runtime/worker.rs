use serde::{Deserialize, Serialize};

/// Runtime worker configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct WorkerOptions {
    /// Maximum number of live workers in one runtime.
    pub limit: Option<u64>,
    /// Maximum number of host threads allocated to workers.
    pub thread_limit: Option<u64>,
    /// Stack reservation in bytes for one worker.
    pub stack_bytes: Option<u64>,
}

/// Runtime worker configuration for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct WorkerOptionsJson {
    /// Maximum number of live workers in one runtime.
    pub limit: Option<u64>,
    /// Maximum number of host threads allocated to workers.
    pub thread_limit: Option<u64>,
    /// Stack reservation in bytes for one worker.
    pub stack_bytes: Option<u64>,
}

impl WorkerOptionsJson {
    /// Inherit unset worker settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.limit.is_none() {
            self.limit = parent.limit;
        }

        if self.thread_limit.is_none() {
            self.thread_limit = parent.thread_limit;
        }

        if self.stack_bytes.is_none() {
            self.stack_bytes = parent.stack_bytes;
        }
    }

    /// Apply worker overrides to a base set of options.
    pub fn apply_to(&self, options: &mut WorkerOptions) {
        if let Some(limit) = self.limit {
            options.limit = Some(limit);
        }

        if let Some(thread_limit) = self.thread_limit {
            options.thread_limit = Some(thread_limit);
        }

        if let Some(stack_bytes) = self.stack_bytes {
            options.stack_bytes = Some(stack_bytes);
        }
    }
}
