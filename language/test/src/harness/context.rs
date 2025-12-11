use std::time::Duration;

use super::TestOptions;

/// Shared configuration for running a test suite.
#[derive(Debug, Clone, Copy)]
pub struct RunContext<'a> {
    /// CLI options that control filtering, parallelism, listing, and verbosity.
    pub options: &'a TestOptions,
    /// Optional per-test timeout.
    pub timeout: Option<Duration>,
}

impl<'a> RunContext<'a> {
    pub fn new(options: &'a TestOptions) -> Self {
        Self {
            options,
            timeout: None,
        }
    }
}
