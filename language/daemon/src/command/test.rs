use serde::{Deserialize, Serialize};

use super::CommandResult;
use super::context::CommandContext;
use super::dispatch::CommandOutcome;

/// Options for the test command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CommandTestOptions {}

impl CommandContext<'_> {
    /// Execute a test command.
    pub(super) fn run_test_command(
        &mut self,
        _options: &CommandTestOptions,
    ) -> CommandResult<CommandOutcome> {
        self.unimplemented_command("test runner is not implemented yet")
    }
}
