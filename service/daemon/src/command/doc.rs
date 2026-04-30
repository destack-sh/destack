use destack_source::DiagnosticCollection;
use serde::{Deserialize, Serialize};

use super::common::CommandMessagePayload;
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
    ) -> super::CommandResult<CommandOutcome> {
        let message = "documentation generator is not implemented yet";
        self.output.push_stderr(format!("{message}\n").into_bytes());
        let payload = CommandMessagePayload {
            message: message.to_string(),
            implemented: false,
        };
        let data = serde_json::to_value(payload)
            .map_err(|error| format!("invalid doc payload: {error}"))?;
        Ok(CommandOutcome::new(DiagnosticCollection::default(), 1, 0, 0, 0).with_data(data))
    }
}
