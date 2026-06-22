use destack_session as session;

use super::{Client, ClientError};
use crate::FileOperation;
use crate::protocol::{
    FileOperationRequest, FileOperationResponse, RootId, SourceUpdateRequest, SourceUpdateResponse,
    WorkspaceRequest, WorkspaceResponse,
};

impl Client {
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
        update: session::Update,
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
