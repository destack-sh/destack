use destack_artifact::{ArtifactPayload, ArtifactReference};
use destack_source::{Content, ContentId};

use super::{Client, ClientError};
use crate::protocol::{RootId, WorkspaceRequest, WorkspaceResponse};
use crate::{ExportRequest, ExportResult};

impl Client {
    /// Return one artifact payload for a workspace root handle.
    pub fn artifact(
        &self,
        handle: RootId,
        artifact: ArtifactReference,
    ) -> Result<ArtifactPayload, ClientError> {
        let response = self.send_request(WorkspaceRequest::Artifact { handle, artifact })?;

        match response {
            WorkspaceResponse::ArtifactResult(response) => {
                destack_serde::from_slice(&response.bytes).map_err(ClientError::ArtifactPayload)
            }
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("artifact result", other)),
        }
    }

    /// Store one content payload for a workspace root handle.
    pub fn store(&self, handle: RootId, content: Content) -> Result<ContentId, ClientError> {
        let response = self.send_request(WorkspaceRequest::Store { handle, content })?;

        match response {
            WorkspaceResponse::StoreResult(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("content stored", other)),
        }
    }

    /// Return one content payload for a workspace root handle.
    pub fn load(&self, handle: RootId, content: ContentId) -> Result<Content, ClientError> {
        let response = self.send_request(WorkspaceRequest::Load { handle, content })?;

        match response {
            WorkspaceResponse::LoadResult(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("content result", other)),
        }
    }

    /// Materialize derived outputs for a workspace root handle.
    pub fn export(
        &self,
        handle: RootId,
        request: ExportRequest,
    ) -> Result<ExportResult, ClientError> {
        let response = self.send_request(WorkspaceRequest::Export { handle, request })?;

        match response {
            WorkspaceResponse::ExportResult(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("export result", other)),
        }
    }
}
