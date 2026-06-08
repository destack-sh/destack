use std::path::Path;
use std::sync::Arc;

use destack_source::{FileType, FileWatchFilter, FileWatchOptions};

use crate::{Watch, WatchPolicy};

use super::{
    DaemonResponse, ProtocolError, ProtocolErrorCode, RootHandleId, Server, WatchBatchResponse,
    WatchNextRequest, WatchStartRequest, WatchStartedResponse, WatchStopRequest,
    WatchStoppedResponse,
};

impl Server {
    /// Handle a watch start request.
    pub(super) fn handle_start_watch(
        &self,
        request: WatchStartRequest,
    ) -> Result<DaemonResponse, ProtocolError> {
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
        let policy = WatchPolicy::from(&request.options);
        let watch = Watch::new(
            self.daemon.file_watcher.clone(),
            roots,
            daemon_watch_options(),
            policy,
        );

        let mut state = self.state.lock();
        state.start_watch(request.handle, Arc::new(watch));

        Ok(DaemonResponse::WatchStarted(WatchStartedResponse {
            handle: request.handle,
        }))
    }

    /// Handle a watch next request.
    pub(super) fn handle_next_watch_batch(
        &self,
        request: WatchNextRequest,
    ) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let (_, workspace) = self.resolve_root(request.handle)?;
        let watch = self.watch(request.handle)?;
        let Some(batch) = watch.next_batch() else {
            return Ok(DaemonResponse::WatchBatchReady(WatchBatchResponse::empty(
                request.handle,
            )));
        };

        let result = workspace.apply_watch_batch(&batch);

        Ok(DaemonResponse::WatchBatchReady(WatchBatchResponse::new(
            request.handle,
            super::WatchBatch::from(&batch),
            &result,
        )))
    }

    /// Handle a watch stop request.
    pub(super) fn handle_stop_watch(
        &self,
        request: WatchStopRequest,
    ) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        self.opened_root(request.handle)?;

        let mut state = self.state.lock();
        state.stop_watch(request.handle);

        Ok(DaemonResponse::WatchStopped(WatchStoppedResponse {
            handle: request.handle,
        }))
    }

    /// Resolve the watch for a root handle.
    fn watch(&self, handle: RootHandleId) -> Result<Arc<Watch>, ProtocolError> {
        let state = self.state.lock();
        state.watch(handle).ok_or_else(|| {
            self.protocol_error(ProtocolErrorCode::NotFound, "root handle is not watched")
        })
    }
}

/// Build daemon watch options.
fn daemon_watch_options() -> FileWatchOptions {
    let filter: FileWatchFilter = Arc::new(is_watch_source_path);

    FileWatchOptions {
        filter: Some(filter),
        ..Default::default()
    }
}

/// Return whether a path should trigger daemon watch processing.
fn is_watch_source_path(path: &Path) -> bool {
    let Some(file_type) = FileType::from_path(path) else {
        return false;
    };

    file_type.is_code() || file_type.is_data() || file_type.is_text()
}
