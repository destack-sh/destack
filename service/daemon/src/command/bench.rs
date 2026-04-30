use destack_source::DiagnosticCollection;
use serde::{Deserialize, Serialize};

use super::common::CommandMessagePayload;
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
    ) -> super::CommandResult<CommandOutcome> {
        let message = "benchmark runner is not implemented yet";
        self.output.push_stderr(format!("{message}\n").into_bytes());
        let payload = CommandMessagePayload {
            message: message.to_string(),
            implemented: false,
        };
        let data = serde_json::to_value(payload)
            .map_err(|error| format!("invalid bench payload: {error}"))?;
        Ok(CommandOutcome::new(DiagnosticCollection::default(), 1, 0, 0, 0).with_data(data))
    }
}
