use std::path::Path;

use destack_session as session;

use crate::{FileOperation, Workspace};

use super::Server;
use crate::protocol::{
    FileOperationRequest, FileOperationResponse, ProtocolError, ProtocolErrorCode,
    SourceUpdateRequest, SourceUpdateResponse, WorkspaceResponse,
};

impl Server {
    /// Handle a file operation request.
    pub(super) fn handle_file_operation(
        &self,
        request: FileOperationRequest,
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.require_session()?;
        let (entry, workspace) = self.resolve_root(request.handle)?;
        if !self.file_operation_within_root(workspace.as_ref(), &entry.root, &request.operation) {
            return Err(self.protocol_error(
                ProtocolErrorCode::Forbidden,
                "operation path is outside root",
            ));
        }

        let result = workspace
            .file(request.operation)
            .map_err(|error| self.workspace_error("file operation", error))?;

        Ok(WorkspaceResponse::FileOperationApplied(
            FileOperationResponse {
                handle: request.handle,
                updates: result,
            },
        ))
    }

    /// Handle a source update request.
    pub(super) fn handle_source_update(
        &self,
        request: SourceUpdateRequest,
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.require_session()?;
        let (entry, workspace) = self.resolve_root(request.handle)?;
        self.validate_source_update_paths(workspace.as_ref(), &entry.root, &request.update.edits)?;

        let result = workspace
            .edit(&entry.root, request.update)
            .map_err(|error| self.workspace_error("source update", error))?;

        Ok(WorkspaceResponse::SourceUpdated(SourceUpdateResponse {
            handle: request.handle,
            commit: result,
        }))
    }

    /// Validate paths in one source update.
    fn validate_source_update_paths(
        &self,
        workspace: &dyn Workspace,
        root: &Path,
        edits: &[session::Edit],
    ) -> Result<(), ProtocolError> {
        for edit in edits {
            match edit {
                session::Edit::SetText { path, .. }
                | session::Edit::EditText { path, .. }
                | session::Edit::SetBytes { path, .. }
                | session::Edit::Remove { path } => {
                    if !self.path_within_root(workspace, path, root) {
                        return Err(self.protocol_error(
                            ProtocolErrorCode::Forbidden,
                            "update path is outside root",
                        ));
                    }
                }
                session::Edit::Move { from, to } => {
                    if !self.path_within_root(workspace, from, root)
                        || !self.path_within_root(workspace, to, root)
                    {
                        return Err(self.protocol_error(
                            ProtocolErrorCode::Forbidden,
                            "update path is outside root",
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Return whether every file operation path is inside one root.
    fn file_operation_within_root(
        &self,
        workspace: &dyn Workspace,
        root: &Path,
        operation: &FileOperation,
    ) -> bool {
        match operation {
            FileOperation::Move { from, to } => {
                self.path_within_root(workspace, from, root)
                    && self.path_within_root(workspace, to, root)
            }
            operation => self.path_within_root(workspace, operation.path(), root),
        }
    }
}
