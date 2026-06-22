use std::path::PathBuf;

use super::{Client, ClientError};
use crate::protocol::{
    RootId, WatchBatchResponse, WatchNextRequest, WatchStartOptions, WatchStartRequest,
    WatchStartedResponse, WatchStopRequest, WatchStoppedResponse, WorkspaceRequest,
    WorkspaceResponse,
};

impl Client {
    /// Start watching a workspace root handle.
    pub fn start_watch(
        &self,
        handle: RootId,
        roots: Vec<PathBuf>,
        options: WatchStartOptions,
    ) -> Result<WatchStartedResponse, ClientError> {
        // send the watch start request
        let request = WatchStartRequest {
            handle,
            roots,
            options,
        };
        let response = self.send_request(WorkspaceRequest::StartWatch(request))?;

        // decode the watch start response
        match response {
            WorkspaceResponse::WatchStarted(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("watch started", other)),
        }
    }

    /// Receive and apply the next watch batch for a workspace root handle.
    pub fn next_watch_batch(&self, handle: RootId) -> Result<WatchBatchResponse, ClientError> {
        // send the watch next request
        let request = WatchNextRequest { handle };
        let response = self.send_request(WorkspaceRequest::NextWatchBatch(request))?;

        // decode the watch batch response
        match response {
            WorkspaceResponse::WatchBatchReady(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("watch batch ready", other)),
        }
    }

    /// Stop watching a workspace root handle.
    pub fn stop_watch(&self, handle: RootId) -> Result<WatchStoppedResponse, ClientError> {
        // send the watch stop request
        let request = WatchStopRequest { handle };
        let response = self.send_request(WorkspaceRequest::StopWatch(request))?;

        // decode the watch stop response
        match response {
            WorkspaceResponse::WatchStopped(response) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("watch stopped", other)),
        }
    }
}
