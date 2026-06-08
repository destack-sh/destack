use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::Workspace;

use super::connection::OpenRoot;
use super::{
    DaemonResponse, ProtocolError, ProtocolErrorCode, ReloadRootRequest, RootClosedResponse,
    RootHandleId, RootOpenedResponse, RootReloadResponse, Server,
};

impl Server {
    /// Handle opening a root.
    pub(super) fn handle_open_root(
        &self,
        request: super::OpenRootRequest,
    ) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;

        let workspace = self
            .daemon
            .open_workspace(&request.workspace)
            .map_err(|error| self.daemon_error(error))?;
        let root = self.normalize_root(workspace.as_ref(), &request.root)?;
        let entry = OpenRoot {
            workspace: workspace.root().to_path_buf(),
            root: root.clone(),
        };

        // open or reuse the root handle
        let (handle, inserted) = self.open_root(workspace.as_ref(), entry)?;
        if inserted {
            self.control.register_handle();
        }

        // load diagnostics only when requested
        let diagnostics = if request.options.load_index {
            self.diagnostics(workspace.as_ref(), &root)?
        } else {
            Vec::new()
        };

        Ok(DaemonResponse::RootOpened(RootOpenedResponse {
            handle,
            diagnostics,
            messages: Vec::new(),
        }))
    }

    /// Handle closing a root handle.
    pub(super) fn handle_close_root(
        &self,
        request: super::CloseRootRequest,
    ) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let mut state = self.state.lock();
        let entry = state
            .remove_root(request.handle)
            .ok_or_else(|| self.missing_root(request.handle))?;
        drop(state);

        // release the root handle
        let workspace = self.daemon.workspace(&entry.workspace).ok_or_else(|| {
            self.protocol_error(ProtocolErrorCode::NotFound, "workspace is closed")
        })?;
        workspace
            .release_root(&entry.root)
            .map_err(|error| self.daemon_error(error))?;
        self.control.unregister_handle();

        Ok(DaemonResponse::RootClosed(RootClosedResponse {
            handle: request.handle,
        }))
    }

    /// Handle a root reload request.
    pub(super) fn handle_reload_root(
        &self,
        request: ReloadRootRequest,
    ) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let (entry, workspace) = self.resolve_root(request.handle)?;
        let reload = workspace.reload_roots(&[entry.root]);

        Ok(DaemonResponse::RootReloaded(RootReloadResponse::new(
            request.handle,
            &reload,
        )))
    }

    /// Normalize roots.
    pub(super) fn normalize_root(
        &self,
        workspace: &Workspace,
        root: &Path,
    ) -> Result<PathBuf, ProtocolError> {
        workspace
            .repository
            .file_system()
            .canonicalize(root)
            .map_err(|error| {
                self.protocol_error(
                    ProtocolErrorCode::InvalidRequest,
                    &format!(
                        "root canonicalization failed for {}: {error}",
                        root.display()
                    ),
                )
            })
    }

    /// Check whether a path is within a root.
    pub(super) fn path_within_root(&self, workspace: &Workspace, path: &Path, root: &Path) -> bool {
        if path.starts_with(root) {
            return true;
        }

        let Some(canonical) = self.canonical_path(workspace, path) else {
            return false;
        };
        canonical.starts_with(root)
    }

    /// Open or reuse a root handle for a root.
    fn open_root(
        &self,
        workspace: &Workspace,
        entry: OpenRoot,
    ) -> Result<(RootHandleId, bool), ProtocolError> {
        let state = self.state.lock();
        if let Some(existing) = state.handle(&entry) {
            return Ok((existing, false));
        }
        drop(state);

        workspace
            .acquire_root(&entry.root)
            .map_err(|error| self.daemon_error(error))?;

        let mut state = self.state.lock();
        if let Some(existing) = state.handle(&entry) {
            let _ = workspace.release_root(&entry.root);
            return Ok((existing, false));
        }
        let handle = state.insert_root(entry);

        Ok((handle, true))
    }

    /// Release root handles tied to this connection.
    pub(super) fn cleanup_connection(&self) {
        // drain roots and clear subscriptions for this connection
        let roots = {
            let mut state = self.state.lock();
            state.drain_roots()
        };

        // release root leases for the drained roots
        for entry in roots {
            if let Some(workspace) = self.daemon.workspace(&entry.workspace) {
                let _ = workspace.release_root(&entry.root);
            }
            self.control.unregister_handle();
        }
    }

    /// Return the open root for one handle.
    pub(super) fn opened_root(&self, handle: RootHandleId) -> Result<OpenRoot, ProtocolError> {
        let state = self.state.lock();
        state.root(handle).ok_or_else(|| self.missing_root(handle))
    }

    /// Resolve one root handle with its workspace.
    pub(super) fn resolve_root(
        &self,
        handle: RootHandleId,
    ) -> Result<(OpenRoot, Arc<Workspace>), ProtocolError> {
        let entry = self.opened_root(handle)?;
        let workspace = self.daemon.workspace(&entry.workspace).ok_or_else(|| {
            self.protocol_error(ProtocolErrorCode::NotFound, "workspace is closed")
        })?;

        Ok((entry, workspace))
    }

    /// Create a protocol error for missing root handles.
    fn missing_root(&self, handle: RootHandleId) -> ProtocolError {
        self.protocol_error(
            ProtocolErrorCode::NotFound,
            &format!("unknown root handle {handle:?}"),
        )
    }

    /// Return a canonical path without requiring the file to exist.
    fn canonical_path(&self, workspace: &Workspace, path: &Path) -> Option<PathBuf> {
        if let Ok(canonical) = workspace.repository.file_system().canonicalize(path) {
            return Some(canonical);
        }

        let parent = path.parent()?;
        let file_name = path.file_name()?;
        let canonical_parent = workspace
            .repository
            .file_system()
            .canonicalize(parent)
            .ok()?;
        Some(canonical_parent.join(file_name))
    }
}
