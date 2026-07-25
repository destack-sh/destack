use std::path::{Path, PathBuf};

use crate::{DiagnosticsRequest, Workspace};

use super::Server;
use crate::protocol::{
    FileDiagnosticsPayload, ProtocolError, ProtocolErrorCode, RootId, WorkspaceResponse,
};

impl Server {
    /// Diagnose every source file in one root.
    pub(super) fn diagnose(&self, handle: RootId) -> Result<WorkspaceResponse, ProtocolError> {
        self.require_session()?;
        let (root, workspace) = self.resolve_root(handle)?;
        let diagnostics = workspace
            .diagnose(DiagnosticsRequest::Root(root.root.clone()))
            .map_err(|error| self.workspace_error("root diagnostics", error))?;
        let diagnostics = diagnostics
            .iter()
            .map(FileDiagnosticsPayload::from)
            .collect();

        Ok(WorkspaceResponse::Diagnose(diagnostics))
    }

    /// Diagnose one source file.
    pub(super) fn diagnose_file(
        &self,
        handle: RootId,
        path: PathBuf,
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.require_session()?;
        let (root, workspace) = self.resolve_root(handle)?;
        let diagnostics = self.file_diagnostics(workspace.as_ref(), &root.root, &path)?;

        Ok(WorkspaceResponse::DiagnoseFile(diagnostics))
    }

    /// Return diagnostics for one source path.
    fn file_diagnostics(
        &self,
        workspace: &dyn Workspace,
        root: &Path,
        path: &Path,
    ) -> Result<Option<FileDiagnosticsPayload>, ProtocolError> {
        if !self.path_within_root(workspace, path, root) {
            return Err(self.protocol_error(
                ProtocolErrorCode::Forbidden,
                "diagnostic path is outside root",
            ));
        }
        let diagnostics = workspace
            .diagnose(DiagnosticsRequest::File(path.to_path_buf()))
            .map_err(|error| self.workspace_error("file diagnostics", error))?;

        match diagnostics.as_slice() {
            [] => Ok(None),
            [diagnostics] => Ok(Some(FileDiagnosticsPayload::from(diagnostics))),
            _ => Err(self.protocol_error(
                ProtocolErrorCode::Internal,
                "workspace returned multiple diagnostic files for one path",
            )),
        }
    }
}
