use std::path::PathBuf;

use destack_repository::Revision;
use destack_source::{FileId, TextRange};

use super::{Client, ClientError};
use crate::protocol::{
    FileOperationRequest, FileOperationResponse, RootId, SourceUpdateRequest, SourceUpdateResponse,
    WorkspaceRequest, WorkspaceResponse,
};
use crate::{FileEdit, FileImage, FileOperation, SourceUpdate};

impl Client {
    /// Return whether one source file is open.
    pub fn is_file_open(&self, handle: RootId, path: PathBuf) -> Result<bool, ClientError> {
        // send the file state request
        let response = self.send_request(WorkspaceRequest::IsFileOpen { handle, path })?;

        // decode the file open response
        match response {
            WorkspaceResponse::IsFileOpen(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("is file open", other)),
        }
    }

    /// Format one source file or selected text range.
    pub fn format_file(
        &self,
        handle: RootId,
        path: PathBuf,
        range: Option<TextRange>,
    ) -> Result<Option<FileEdit>, ClientError> {
        // send the editor formatting request
        let response = self.send_request(WorkspaceRequest::FormatFile {
            handle,
            path,
            range,
        })?;

        // decode the editor formatting response
        match response {
            WorkspaceResponse::FormatFile(response) => response
                .map(|edit| {
                    FileEdit::try_from(edit).map_err(|error| {
                        ClientError::UnexpectedResponse(format!(
                            "file edit payload is invalid: {error}"
                        ))
                    })
                })
                .transpose(),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("format file", other)),
        }
    }

    /// Read source file images from one exact revision.
    pub fn read_files(
        &self,
        handle: RootId,
        revision: Revision,
        file_ids: Vec<FileId>,
    ) -> Result<Vec<FileImage>, ClientError> {
        // send the file read request
        let response = self.send_request(WorkspaceRequest::ReadFiles {
            handle,
            revision,
            file_ids,
        })?;

        // decode the file response
        match response {
            WorkspaceResponse::ReadFiles(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("read files", other)),
        }
    }

    /// Apply a file operation to a workspace root handle.
    pub fn apply_file_operation(
        &self,
        handle: RootId,
        operation: FileOperation,
    ) -> Result<FileOperationResponse, ClientError> {
        // send the file operation request
        let request = FileOperationRequest { handle, operation };
        let response = self.send_request(WorkspaceRequest::ApplyFileOperation(request))?;

        // decode the file operation response
        match response {
            WorkspaceResponse::FileOperationApplied(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("file operation applied", other)),
        }
    }

    /// Apply a source update to a workspace root handle.
    pub fn apply_source_update(
        &self,
        handle: RootId,
        update: SourceUpdate,
    ) -> Result<SourceUpdateResponse, ClientError> {
        // send the source update request
        let request = SourceUpdateRequest { handle, update };
        let response = self.send_request(WorkspaceRequest::ApplySourceUpdate(request))?;

        // decode the source update response
        match response {
            WorkspaceResponse::SourceUpdated(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("source update applied", other)),
        }
    }
}
