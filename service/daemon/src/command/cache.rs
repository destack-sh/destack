use destack_source::DiagnosticCollection;
use serde::{Deserialize, Serialize};

use super::CommandResult;
use super::context::CommandContext;
use super::dispatch::CommandOutcome;

/// Options for the cache command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CommandCacheOptions;

/// Cache entry payload for cache command output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandCacheEntry {
    /// Cache directory path.
    pub directory: String,
    /// Cache kind.
    pub kind: String,
}

/// Cache payload for cache command output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandCachePayload {
    /// Cache entries for the workspace.
    pub caches: Vec<CommandCacheEntry>,
}

impl CommandContext<'_> {
    /// Execute one cache command.
    pub(super) fn run_cache_command(
        &mut self,
        _options: &CommandCacheOptions,
    ) -> CommandResult<CommandOutcome> {
        // read the repository layout resolved at launch
        let layout = self.repository.layout();

        // build the cache payload
        let payload = CommandCachePayload {
            caches: vec![CommandCacheEntry {
                directory: layout.workspace_cache.display().to_string(),
                kind: "workspace".to_string(),
            }],
        };

        // serialize command payload
        let data = serde_json::to_value(payload)
            .map_err(|error| format!("invalid cache payload: {error}"))?;

        Ok(CommandOutcome::new(DiagnosticCollection::default(), 0, 0, 0, 0).with_data(data))
    }
}
