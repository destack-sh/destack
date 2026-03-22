use std::time::Duration;

use super::RunOptions;

/// Shared configuration for running one suite.
#[derive(Debug, Clone, Copy)]
pub struct RunContext<'a> {
    /// The CLI options for this run.
    pub options: &'a RunOptions,
    /// The optional per-case timeout.
    pub timeout: Option<Duration>,
}

impl<'a> RunContext<'a> {
    /// Create one run context with default shared state.
    pub fn new(options: &'a RunOptions) -> Self {
        Self {
            options,
            timeout: None,
        }
    }
}
