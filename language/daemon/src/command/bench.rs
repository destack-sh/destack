use serde::{Deserialize, Serialize};

use super::CommandResult;
use super::context::CommandContext;
use super::dispatch::CommandOutcome;

/// Options for the bench command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CommandBenchOptions {}

impl CommandContext<'_> {
    /// Execute a bench command.
    pub(super) fn run_bench_command(
        &mut self,
        _options: &CommandBenchOptions,
    ) -> CommandResult<CommandOutcome> {
        self.unimplemented_command("benchmark runner is not implemented yet")
    }
}
