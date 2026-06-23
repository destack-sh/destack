use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::Workspace;
use crate::file::normalize_path;
use crate::workspace::ReloadRequest;

use super::Server;
use super::connection::OpenRoot;
use crate::protocol::{
    CloseRootRequest, OpenRootRequest, ProtocolError, ProtocolErrorCode, ReloadRootRequest,
    RootClosedResponse, RootId, RootOpenedResponse, RootReloadResponse, WorkspaceResponse,
};

impl Server {
    /// Handle opening a root.
    pub(super) fn handle_open_root(
        &self,
        request: OpenRootRequest,
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.require_session()?;

        let root = self.normalize_root(self.workspace.as_ref(), &request.root)?;
        let entry = OpenRoot { root: root.clone() };

        // open or reuse the root handle
        let (handle, inserted) = self.open_root(entry)?;
        if inserted {
            self.lifecycle.register_handle();
        }

        // load diagnostics only when requested
        let diagnostics = if request.options.load_index {
            self.diagnostics(self.workspace.as_ref(), &root)?
        } else {
            Vec::new()
        };

        Ok(WorkspaceResponse::RootOpened(RootOpenedResponse {
            handle,
            root,
            diagnostics,
            messages: Vec::new(),
        }))
    }

    /// Handle closing a root handle.
    pub(super) fn handle_close_root(
        &self,
        request: CloseRootRequest,
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.require_session()?;
        let mut state = self.connection.lock();
        let entry = state
            .remove_root(request.handle)
            .ok_or_else(|| self.missing_root(request.handle))?;
        drop(state);

        // release the root handle
        self.root_lease
            .release(self.workspace.as_ref(), &entry.root)
            .map_err(|error| self.workspace_error("release root", error))?;
        self.lifecycle.unregister_handle();

        Ok(WorkspaceResponse::RootClosed(RootClosedResponse {
            handle: request.handle,
        }))
    }

    /// Handle a root reload request.
    pub(super) fn handle_reload_root(
        &self,
        request: ReloadRootRequest,
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.require_session()?;
        let (entry, workspace) = self.resolve_root(request.handle)?;
        let reload = workspace
            .reload(ReloadRequest {
                roots: vec![entry.root],
                reason: request.reason,
            })
            .map_err(|error| self.workspace_error("reload root", error))?;

        Ok(WorkspaceResponse::RootReloaded(RootReloadResponse {
            handle: request.handle,
            updates: reload,
        }))
    }

    /// Normalize roots.
    pub(super) fn normalize_root(
        &self,
        workspace: &dyn Workspace,
        root: &Path,
    ) -> Result<PathBuf, ProtocolError> {
        workspace.canonicalize(root).map_err(|error| {
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
    pub(super) fn path_within_root(
        &self,
        workspace: &dyn Workspace,
        path: &Path,
        root: &Path,
    ) -> bool {
        let Some(canonical) = self.canonical_path(workspace, path) else {
            return false;
        };

        canonical.starts_with(root)
    }

    /// Open or reuse a root handle for a root.
    fn open_root(&self, entry: OpenRoot) -> Result<(RootId, bool), ProtocolError> {
        let state = self.connection.lock();
        if let Some(existing) = state.handle(&entry) {
            return Ok((existing, false));
        }
        drop(state);

        self.root_lease
            .acquire(self.workspace.as_ref(), &entry.root)
            .map_err(|error| self.workspace_error("open root", error))?;

        let mut state = self.connection.lock();
        if let Some(existing) = state.handle(&entry) {
            self.root_lease
                .release(self.workspace.as_ref(), &entry.root)
                .map_err(|error| self.workspace_error("release duplicate root", error))?;

            return Ok((existing, false));
        }
        let handle = state.insert_root(entry);

        Ok((handle, true))
    }

    /// Release root handles tied to this connection.
    pub(super) fn cleanup_connection(&self) {
        // drain roots and clear subscriptions for this connection
        let roots = {
            let mut state = self.connection.lock();
            state.drain_roots()
        };

        // release root handles for the drained roots
        for entry in roots {
            let _ = self
                .root_lease
                .release(self.workspace.as_ref(), &entry.root);
            self.lifecycle.unregister_handle();
        }
    }

    /// Return the open root for one handle.
    pub(super) fn opened_root(&self, handle: RootId) -> Result<OpenRoot, ProtocolError> {
        let state = self.connection.lock();
        state.root(handle).ok_or_else(|| self.missing_root(handle))
    }

    /// Resolve one root handle with its workspace.
    pub(super) fn resolve_root(
        &self,
        handle: RootId,
    ) -> Result<(OpenRoot, Arc<dyn Workspace>), ProtocolError> {
        let entry = self.opened_root(handle)?;
        let workspace = self.workspace.clone();

        Ok((entry, workspace))
    }

    /// Create a protocol error for missing root handles.
    fn missing_root(&self, handle: RootId) -> ProtocolError {
        self.protocol_error(
            ProtocolErrorCode::NotFound,
            &format!("unknown root handle {handle:?}"),
        )
    }

    /// Return a canonical path without requiring the file to exist.
    fn canonical_path(&self, workspace: &dyn Workspace, path: &Path) -> Option<PathBuf> {
        if let Ok(canonical) = workspace.canonicalize(path) {
            return Some(canonical);
        }

        if let (Some(parent), Some(file_name)) = (path.parent(), path.file_name())
            && let Ok(parent) = workspace.canonicalize(parent)
        {
            return Some(parent.join(file_name));
        }

        Some(normalize_path(path))
    }
}
