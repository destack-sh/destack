use std::path::PathBuf;

use destack_repository::Revision;

use super::{Client, ClientError};
use crate::ReloadReason;
use crate::protocol::{
    CloseRootRequest, OpenRootRequest, ReloadRootRequest, RootClosedResponse, RootId,
    RootOpenedResponse, RootReloadResponse, WorkspaceRequest, WorkspaceResponse,
};

impl Client {
    /// Read the current semantic revision for one workspace root.
    pub fn read_revision(&self, handle: RootId) -> Result<Revision, ClientError> {
        // send the revision request
        let response = self.send_request(WorkspaceRequest::ReadRevision { handle })?;

        // decode the revision response
        match response {
            WorkspaceResponse::ReadRevision(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("read revision", other)),
        }
    }

    /// Open a workspace root handle.
    pub fn open_root(&self, root: PathBuf) -> Result<RootOpenedResponse, ClientError> {
        // send the open root request
        let request = OpenRootRequest { root };
        let response = self.send_request(WorkspaceRequest::OpenRoot(request))?;

        // decode the root response
        match response {
            WorkspaceResponse::RootOpened(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("root opened", other)),
        }
    }

    /// Close a workspace root handle.
    pub fn close_root(&self, handle: RootId) -> Result<RootClosedResponse, ClientError> {
        // send the close root request
        let request = CloseRootRequest { handle };
        let response = self.send_request(WorkspaceRequest::CloseRoot(request))?;

        // decode the close response
        match response {
            WorkspaceResponse::RootClosed(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("root closed", other)),
        }
    }

    /// Reload a workspace root handle.
    pub fn reload_root(
        &self,
        handle: RootId,
        reason: ReloadReason,
    ) -> Result<RootReloadResponse, ClientError> {
        // send the reload request
        let request = ReloadRootRequest { handle, reason };
        let response = self.send_request(WorkspaceRequest::ReloadRoot(request))?;

        // decode the reload response
        match response {
            WorkspaceResponse::RootReloaded(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("root reloaded", other)),
        }
    }
}
