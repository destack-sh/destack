use std::path::Path;

use destack_session as session;

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
                .open_file(
                    uri,
                    version,
                    session::Edit::SetText {
                        path,
                        text: content,
                    },
                )
                .map_err(DaemonError::from),
            FileOperation::OpenBytes {
                path,
                uri,
                version,
                content,
            } => workspace
                .open_file(
                    uri,
                    version,
                    session::Edit::SetBytes {
                        path,
                        bytes: content,
                    },
                )
                .map_err(DaemonError::from),
            FileOperation::ChangeText {
                path,
                uri,
                version,
                content,
            } => workspace
                .change_file(
                    uri,
                    version,
                    session::Edit::SetText {
                        path,
                        text: content,
                    },
                )
                .map_err(DaemonError::from),
            FileOperation::ChangeBytes {
                path,
                uri,
                version,
                content,
            } => workspace
                .change_file(
                    uri,
                    version,
                    session::Edit::SetBytes {
                        path,
                        bytes: content,
                    },
                )
                .map_err(DaemonError::from),
            FileOperation::SaveText { path, content } => workspace.save_text_file(&path, content),
            FileOperation::SaveBytes { path, content } => workspace.save_bytes_file(&path, content),
            FileOperation::Close { path } => workspace.close_file(&path).map_err(DaemonError::from),
            FileOperation::WriteText { path, content } => {
                workspace.write_file(session::Edit::SetText {
                    path,
                    text: content,
                })
            }
            FileOperation::WriteBytes { path, content } => {
                workspace.write_file(session::Edit::SetBytes {
                    path,
                    bytes: content,
                })
            }
            FileOperation::Remove { path } => workspace.write_file(session::Edit::Remove { path }),
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

        let base = request.update.base;
        let edits = request
            .update
            .edits
            .into_iter()
            .map(session::Edit::from)
            .collect();
        let result = workspace
            .apply_source_edits(&entry.root, base, edits)
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

impl From<SourceEdit> for session::Edit {
    /// Convert a protocol source edit into a session edit.
    fn from(edit: SourceEdit) -> Self {
        match edit {
            SourceEdit::SetText { path, text } => Self::SetText { path, text },
            SourceEdit::EditText { path, edits } => Self::EditText {
                path,
                edits: edits.into_iter().map(session::TextEdit::from).collect(),
            },
            SourceEdit::SetBytes { path, bytes } => Self::SetBytes { path, bytes },
            SourceEdit::Remove { path } => Self::Remove { path },
            SourceEdit::Move { from, to } => Self::Move { from, to },
        }
    }
}

impl From<TextEdit> for session::TextEdit {
    /// Convert a protocol text edit into a session text edit.
    fn from(edit: TextEdit) -> Self {
        Self {
            range: session::TextRange::from(edit.range),
            text: edit.text,
        }
    }
}

impl From<TextRange> for session::TextRange {
    /// Convert a protocol text range into a session text range.
    fn from(range: TextRange) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }
}
