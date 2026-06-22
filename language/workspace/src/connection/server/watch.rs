use super::Server;
use crate::protocol::{
    ProtocolError, ProtocolErrorCode, WatchBatchResponse, WatchNextRequest, WatchStartRequest,
    WatchStartedResponse, WatchStopRequest, WatchStoppedResponse, WorkspaceResponse,
};

impl Server {
    /// Handle a watch start request.
    pub(super) fn handle_start_watch(
        &self,
        request: WatchStartRequest,
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.require_session()?;
        let (entry, workspace) = self.resolve_root(request.handle)?;
        let roots = if request.roots.is_empty() {
            vec![entry.root.clone()]
        } else {
            request.roots
        };
        for root in &roots {
            if !self.path_within_root(workspace.as_ref(), root, &entry.root) {
                return Err(
                    self.protocol_error(ProtocolErrorCode::Forbidden, "watch root is outside root")
                );
            }
        }
        workspace
            .watch(roots, request.options.policy())
            .map_err(|error| self.workspace_error("watch start", error))?;

        Ok(WorkspaceResponse::WatchStarted(WatchStartedResponse {
            handle: request.handle,
        }))
    }

    /// Handle a watch next request.
    pub(super) fn handle_next_watch_batch(
        &self,
        request: WatchNextRequest,
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.require_session()?;
        let (entry, workspace) = self.resolve_root(request.handle)?;
        let Some(update) = workspace
            .next_watch(&entry.root)
            .map_err(|error| self.workspace_error("watch batch", error))?
        else {
            return Ok(WorkspaceResponse::WatchBatchReady(
                WatchBatchResponse::empty(request.handle),
            ));
        };

        Ok(WorkspaceResponse::WatchBatchReady(WatchBatchResponse {
            handle: request.handle,
            batch: Some(update.batch),
            updates: update.updates,
        }))
    }

    /// Handle a watch stop request.
    pub(super) fn handle_stop_watch(
        &self,
        request: WatchStopRequest,
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.require_session()?;
        let (entry, workspace) = self.resolve_root(request.handle)?;
        workspace
            .unwatch(&entry.root)
            .map_err(|error| self.workspace_error("watch stop", error))?;

        Ok(WorkspaceResponse::WatchStopped(WatchStoppedResponse {
            handle: request.handle,
        }))
    }
}
