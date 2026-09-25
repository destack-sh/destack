use std::sync::Arc;

use tspp_source::{DiagnosticCollection, File};

use super::CommandMessagePayload;
use crate::Message;

/// Command outcome used for response assembly.
#[derive(Debug)]
pub(crate) struct CommandOutcome<T = ()> {
    /// Diagnostics captured during execution.
    pub(crate) diagnostics: DiagnosticCollection,
    /// Exit code for the command.
    pub(crate) exit_code: i32,
    /// Messages produced by command execution.
    pub(crate) messages: Vec<Message>,
    /// Source files referenced by command data.
    pub(crate) files: Vec<Arc<File>>,
    /// Command payload data.
    pub(crate) data: T,
    /// Number of modules included in the command.
    pub(crate) module_count: usize,
    /// Number of profiles included in the command.
    pub(crate) profile_count: usize,
    /// Number of targets included in the command.
    pub(crate) target_count: usize,
}

impl CommandOutcome<()> {
    /// Build a command outcome.
    pub(super) fn new(
        diagnostics: DiagnosticCollection,
        exit_code: i32,
        module_count: usize,
        profile_count: usize,
        target_count: usize,
    ) -> Self {
        Self {
            diagnostics,
            exit_code,
            messages: Vec::new(),
            files: Vec::new(),
            data: (),
            module_count,
            profile_count,
            target_count,
        }
    }

    /// Build a standard unimplemented command outcome.
    pub(super) fn unimplemented(message: &str) -> CommandOutcome<CommandMessagePayload> {
        let payload = CommandMessagePayload {
            message: message.to_string(),
            implemented: false,
        };

        Self::new(DiagnosticCollection::default(), 1, 0, 0, 0)
            .with_message(Message::error("unimplemented", message))
            .with_data(payload)
    }
}

impl<T> CommandOutcome<T> {
    /// Attach command-specific data.
    pub(super) fn with_data<U>(self, data: U) -> CommandOutcome<U> {
        CommandOutcome {
            diagnostics: self.diagnostics,
            exit_code: self.exit_code,
            messages: self.messages,
            files: self.files,
            data,
            module_count: self.module_count,
            profile_count: self.profile_count,
            target_count: self.target_count,
        }
    }

    /// Attach one command message.
    pub(super) fn with_message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Attach source files referenced by command data.
    pub(super) fn with_files(mut self, files: Vec<Arc<File>>) -> Self {
        self.files = files;
        self
    }
}
