use std::path::{Path, PathBuf};

use destack_repository::Revision;
use destack_source::{Edit, FileId, TextRange};

use crate::{FileOperation, Workspace};

use super::Server;
use crate::protocol::{
    FileEditPayload, FileOperationRequest, FileOperationResponse, ProtocolError, ProtocolErrorCode,
    RootId, SourceUpdateRequest, SourceUpdateResponse, WorkspaceResponse,
};

impl Server {
    /// Check whether one source file is open.
    pub(super) fn is_file_open(
        &self,
        handle: RootId,
        path: PathBuf,
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.require_session()?;
        let (root, workspace) = self.resolve_root(handle)?;

        // require the requested path to belong to the selected root
        if !self.path_within_root(workspace.as_ref(), &path, &root.root) {
            return Err(
                self.protocol_error(ProtocolErrorCode::Forbidden, "file path is outside root")
            );
        }
        let is_open = workspace
            .is_file_open(&path)
            .map_err(|error| self.workspace_error("file open", error))?;

        Ok(WorkspaceResponse::IsFileOpen(is_open))
    }

    /// Format one source file or selected text range.
    pub(super) fn format_file(
        &self,
        handle: RootId,
        path: PathBuf,
        range: Option<TextRange>,
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.require_session()?;
        let (root, workspace) = self.resolve_root(handle)?;
        if !self.path_within_root(workspace.as_ref(), &path, &root.root) {
            return Err(
                self.protocol_error(ProtocolErrorCode::Forbidden, "file path is outside root")
            );
        }

        // format the exact source selection
        let edit = workspace
            .format_file(&root.root, path, range)
            .map_err(|error| self.workspace_error("format file", error))?;
        let edit = edit.as_ref().map(FileEditPayload::from);

        Ok(WorkspaceResponse::FormatFile(edit))
    }

    /// Read source files from one exact revision.
    pub(super) fn read_files(
        &self,
        handle: RootId,
        revision: Revision,
        file_ids: Vec<FileId>,
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.require_session()?;
        let (root, workspace) = self.resolve_root(handle)?;
        let files = workspace
            .read_files(&root.root, revision, file_ids)
            .map_err(|error| self.workspace_error("read files", error))?;
        let images = files
            .iter()
            .map(|file| crate::FileImage::from(file.as_ref()))
            .collect();

        Ok(WorkspaceResponse::ReadFiles(images))
    }

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
        edits: &[Edit],
    ) -> Result<(), ProtocolError> {
        for edit in edits {
            match edit {
                Edit::SetText { path, .. }
                | Edit::EditText { path, .. }
                | Edit::SetBytes { path, .. }
                | Edit::Remove { path } => {
                    if !self.path_within_root(workspace, path, root) {
                        return Err(self.protocol_error(
                            ProtocolErrorCode::Forbidden,
                            "update path is outside root",
                        ));
                    }
                }
                Edit::Move { from, to } => {
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
