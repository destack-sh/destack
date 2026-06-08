use std::path::Path;

use destack_session as session;
use destack_workspace::FileChange;

use crate::{DaemonError, Workspace};

use super::{
    DaemonResponse, FileOperation, FileOperationRequest, FileOperationResponse, ProtocolError,
    ProtocolErrorCode, Server, SourceEdit, SourceUpdateRequest, SourceUpdateResponse, TextEdit,
    TextRange,
};

impl Server {
    /// Handle a file operation request.
    pub(super) fn handle_file_operation(
        &self,
        request: FileOperationRequest,
    ) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let (entry, workspace) = self.resolve_root(request.handle)?;
        if !self.path_within_root(workspace.as_ref(), request.operation.path(), &entry.root) {
            return Err(self.protocol_error(
                ProtocolErrorCode::Forbidden,
                "operation path is outside root",
            ));
        }

        let result = match request.operation {
            FileOperation::OpenText {
                path,
                uri,
                version,
                content,
            } => workspace
                .open_file(&path, uri, version, FileChange::Text { content })
                .map_err(DaemonError::from),
            FileOperation::OpenBytes {
                path,
                uri,
                version,
                content,
            } => workspace
                .open_file(&path, uri, version, FileChange::Bytes { content })
                .map_err(DaemonError::from),
            FileOperation::ChangeText {
                path,
                uri,
                version,
                content,
            } => workspace
                .change_file(&path, uri, version, FileChange::Text { content })
                .map_err(DaemonError::from),
            FileOperation::ChangeBytes {
                path,
                uri,
                version,
                content,
            } => workspace
                .change_file(&path, uri, version, FileChange::Bytes { content })
                .map_err(DaemonError::from),
            FileOperation::SaveText { path, content } => workspace.save_text_file(&path, content),
            FileOperation::SaveBytes { path, content } => workspace.save_bytes_file(&path, content),
            FileOperation::Close { path } => workspace.close_file(&path).map_err(DaemonError::from),
            FileOperation::WriteText { path, content } => {
                workspace.write_file(&path, FileChange::Text { content })
            }
            FileOperation::WriteBytes { path, content } => {
                workspace.write_file(&path, FileChange::Bytes { content })
            }
            FileOperation::Remove { path } => workspace.write_file(&path, FileChange::Removed),
        }
        .map_err(|error| self.daemon_error(error))?;

        Ok(DaemonResponse::FileOperationApplied(
            FileOperationResponse::new(request.handle, &result),
        ))
    }

    /// Handle a source update request.
    pub(super) fn handle_source_update(
        &self,
        request: SourceUpdateRequest,
    ) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let (entry, workspace) = self.resolve_root(request.handle)?;
        self.validate_source_update_paths(workspace.as_ref(), &entry.root, &request.update.edits)?;

        let result = workspace
            .apply_source_update(&entry.root, session_source_update(request.update))
            .map_err(|error| self.daemon_error(error))?;

        Ok(DaemonResponse::SourceUpdated(SourceUpdateResponse::new(
            request.handle,
            &result,
        )))
    }

    /// Validate paths in one source update.
    fn validate_source_update_paths(
        &self,
        workspace: &Workspace,
        root: &Path,
        edits: &[SourceEdit],
    ) -> Result<(), ProtocolError> {
        for edit in edits {
            match edit {
                SourceEdit::SetText { path, .. }
                | SourceEdit::EditText { path, .. }
                | SourceEdit::SetBytes { path, .. }
                | SourceEdit::Remove { path } => {
                    if !self.path_within_root(workspace, path, root) {
                        return Err(self.protocol_error(
                            ProtocolErrorCode::Forbidden,
                            "update path is outside root",
                        ));
                    }
                }
                SourceEdit::Move { from, to } => {
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
}

/// Build a session source update from a protocol payload.
fn session_source_update(update: super::SourceUpdate) -> session::SourceUpdate {
    session::SourceUpdate {
        base: update.base,
        edits: update.edits.into_iter().map(session_source_edit).collect(),
    }
}

/// Build a session source edit from a protocol payload.
fn session_source_edit(edit: SourceEdit) -> session::SourceEdit {
    match edit {
        SourceEdit::SetText { path, text } => session::SourceEdit::SetText { path, text },
        SourceEdit::EditText { path, edits } => session::SourceEdit::EditText {
            path,
            edits: edits.into_iter().map(session_text_edit).collect(),
        },
        SourceEdit::SetBytes { path, bytes } => session::SourceEdit::SetBytes { path, bytes },
        SourceEdit::Remove { path } => session::SourceEdit::Remove { path },
        SourceEdit::Move { from, to } => session::SourceEdit::Move { from, to },
    }
}

/// Build a session text edit from a protocol payload.
fn session_text_edit(edit: TextEdit) -> session::TextEdit {
    session::TextEdit {
        range: session_text_range(edit.range),
        text: edit.text,
    }
}

/// Build a session text range from a protocol payload.
fn session_text_range(range: TextRange) -> session::TextRange {
    session::TextRange {
        start: range.start,
        end: range.end,
    }
}
