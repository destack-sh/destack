use serde::{Deserialize, Serialize};

use super::CommandResult;
use super::context::CommandContext;
use super::dispatch::CommandOutcome;

/// Options for the doc command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CommandDocOptions {}

impl CommandContext<'_> {
    /// Execute a doc command.
    pub(super) fn run_doc_command(
        &mut self,
        _options: &CommandDocOptions,
    ) -> CommandResult<CommandOutcome> {
        self.unimplemented_command("documentation generator is not implemented yet")
    }
}
