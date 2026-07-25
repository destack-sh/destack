use std::path::PathBuf;

use super::{Client, ClientError};
use crate::protocol::{FileDiagnosticsPayload, RootId, WorkspaceRequest, WorkspaceResponse};

impl Client {
    /// Diagnose every source file in one workspace root.
    pub fn diagnose(&self, handle: RootId) -> Result<Vec<FileDiagnosticsPayload>, ClientError> {
        // send the diagnostic request
        let response = self.send_request(WorkspaceRequest::Diagnose { handle })?;

        // decode the diagnostics response
        match response {
            WorkspaceResponse::Diagnose(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("diagnose", other)),
        }
    }

    /// Diagnose one source file.
    pub fn diagnose_file(
        &self,
        handle: RootId,
        path: PathBuf,
    ) -> Result<Option<FileDiagnosticsPayload>, ClientError> {
        // send the file diagnostic request
        let response = self.send_request(WorkspaceRequest::DiagnoseFile { handle, path })?;

        // decode the file diagnostics response
        match response {
            WorkspaceResponse::DiagnoseFile(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("diagnose file", other)),
        }
    }
}
