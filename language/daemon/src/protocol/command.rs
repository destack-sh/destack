use std::path::Path;
use std::sync::Arc;

use crate::{CommandErrorKind, CommandRevision, DaemonCommandError, Workspace};

use super::{
    BinaryPayload, CommandRequest, CommandResponse, DaemonResponse, PayloadWriter, ProtocolError,
    ProtocolErrorCode, Server, diagnostic_file_images, diagnostics_to_batches,
};

impl Server {
    /// Handle a command request.
    pub(super) fn handle_command(
        &self,
        request: CommandRequest,
        payloads: &mut PayloadWriter,
    ) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let (entry, workspace) = self.resolve_root(request.handle)?;
        self.require_command_revision(workspace.as_ref(), &entry.root, request.revision)?;

        let result = self
            .daemon
            .run_root_command(
                workspace.as_ref(),
                &entry.root,
                &request.common,
                &request.payload,
                request.revision,
            )
            .map_err(|error| self.command_error(error))?;
        let data = match result.data {
            Some(payload) => {
                let binary_payload = BinaryPayload::from_json_value(&payload).map_err(|error| {
                    self.protocol_error(
                        ProtocolErrorCode::Internal,
                        &format!("failed to encode command payload: {error}"),
                    )
                })?;
                Some(
                    payloads
                        .prepare(binary_payload)
                        .map_err(|error| self.payload_error(error))?,
                )
            }
            None => None,
        };
        let repository = Arc::clone(&workspace.repository);
        let diagnostics = diagnostics_to_batches(&result.diagnostics);
        let files = diagnostic_file_images(&repository, result.revision, &result.diagnostics);

        Ok(DaemonResponse::CommandResult(CommandResponse {
            handle: request.handle,
            success: result.success,
            exit_code: result.exit_code,
            diagnostics,
            files,
            messages: Vec::new(),
            output: result.output,
            outputs: Vec::new(),
            module_count: result.module_count,
            profile_count: result.profile_count,
            target_count: result.target_count,
            data,
        }))
    }

    /// Require the requested command revision.
    fn require_command_revision(
        &self,
        workspace: &Workspace,
        root: &Path,
        revision: CommandRevision,
    ) -> Result<(), ProtocolError> {
        let CommandRevision::Exact(expected) = revision else {
            return Ok(());
        };

        // compare the root against the requested content identity
        let actual = workspace
            .revision(root)
            .map_err(|error| self.workspace_error("command revision", error))?;
        if actual == expected {
            return Ok(());
        }

        Err(self.protocol_error(
            ProtocolErrorCode::Conflict,
            &format!("command expected revision {expected}, current revision is {actual}"),
        ))
    }

    /// Convert a command error into a protocol error.
    fn command_error(&self, error: DaemonCommandError) -> ProtocolError {
        let code = match error.kind {
            CommandErrorKind::InvalidInput
            | CommandErrorKind::Config
            | CommandErrorKind::Resolve
            | CommandErrorKind::Payload => ProtocolErrorCode::InvalidRequest,
            CommandErrorKind::Compiler | CommandErrorKind::Runtime => ProtocolErrorCode::Conflict,
            CommandErrorKind::Internal => ProtocolErrorCode::Internal,
        };

        self.protocol_error(code, &error.to_string())
    }
}
