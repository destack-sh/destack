use destack_source::{Content, ContentId};

use destack_artifact::ArtifactReference;

use crate::ExportRequest;

use super::Server;
use crate::protocol::{ArtifactBlob, ProtocolError, ProtocolErrorCode, RootId, WorkspaceResponse};

impl Server {
    /// Handle an artifact read request.
    pub(super) fn handle_artifact(
        &self,
        handle: RootId,
        artifact: ArtifactReference,
    ) -> Result<WorkspaceResponse, ProtocolError> {
        let (entry, workspace) = self.resolve_root(handle)?;
        let result = workspace
            .artifact(&entry.root, artifact)
            .map_err(|error| self.workspace_error("artifact", error))?;
        let bytes = destack_serde::to_vec(&result).map_err(|error| {
            self.protocol_error(
                ProtocolErrorCode::Internal,
                &format!("failed to serialize artifact payload: {error}"),
            )
        })?;
        let result = ArtifactBlob {
            version: artifact.version,
            bytes,
        };

        Ok(WorkspaceResponse::ArtifactResult(result))
    }

    /// Handle a content store request.
    pub(super) fn handle_store(
        &self,
        handle: RootId,
        content: Content,
    ) -> Result<WorkspaceResponse, ProtocolError> {
        let (_entry, workspace) = self.resolve_root(handle)?;
        let result = workspace
            .store(content)
            .map_err(|error| self.workspace_error("store content", error))?;

        Ok(WorkspaceResponse::StoreResult(result))
    }

    /// Handle a content load request.
    pub(super) fn handle_load(
        &self,
        handle: RootId,
        content: ContentId,
    ) -> Result<WorkspaceResponse, ProtocolError> {
        let (_entry, workspace) = self.resolve_root(handle)?;
        let result = workspace
            .load(content)
            .map_err(|error| self.workspace_error("content", error))?;

        Ok(WorkspaceResponse::LoadResult(result))
    }

    /// Handle an export request.
    pub(super) fn handle_export(
        &self,
        handle: RootId,
        request: ExportRequest,
    ) -> Result<WorkspaceResponse, ProtocolError> {
        let (entry, workspace) = self.resolve_root(handle)?;
        let result = workspace
            .export(&entry.root, request)
            .map_err(|error| self.workspace_error("export", error))?;

        Ok(WorkspaceResponse::ExportResult(result))
    }
}
