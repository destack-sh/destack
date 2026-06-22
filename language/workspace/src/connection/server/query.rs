use std::path::Path;

use crate::{DiagnosticsRequest, QueryRequest, RevisionPolicy, ViewRequest, ViewResult, Workspace};

use super::Server;
use crate::PayloadWriter;
use crate::protocol::{
    DiagnosticBatch, DiagnosticSnapshot, ProtocolError, ProtocolErrorCode, QueryRequestPayload,
    QueryResponsePayload, WorkspaceQuery, WorkspaceQueryResponse, WorkspaceResponse,
};

impl Server {
    /// Handle a query request.
    pub(super) fn handle_query(
        &self,
        query: WorkspaceQuery,
        payloads: &mut PayloadWriter,
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.require_session()?;
        let response = match query {
            WorkspaceQuery::Diagnostics { handle } => {
                let (root, workspace) = self.resolve_root(handle)?;
                let diagnostics = self.diagnostics(workspace.as_ref(), &root.root)?;
                WorkspaceQueryResponse::Diagnostics(diagnostics)
            }
            WorkspaceQuery::DiagnosticSnapshots { handle } => {
                let (root, workspace) = self.resolve_root(handle)?;
                let diagnostics = self.diagnostic_snapshots(workspace.as_ref(), &root.root)?;
                WorkspaceQueryResponse::DiagnosticSnapshots(diagnostics)
            }
            WorkspaceQuery::FileDiagnostics { handle, path } => {
                let (root, workspace) = self.resolve_root(handle)?;
                let diagnostics = self.file_diagnostics(workspace.as_ref(), &root.root, &path)?;
                WorkspaceQueryResponse::FileDiagnostics(diagnostics)
            }
            WorkspaceQuery::CurrentRevision { handle } => {
                let (root, workspace) = self.resolve_root(handle)?;
                let revision = workspace
                    .revision(&root.root)
                    .map_err(|error| self.workspace_error("current revision", error))?;
                WorkspaceQueryResponse::CurrentRevision(revision)
            }
            WorkspaceQuery::RootSnapshot { handle, target } => {
                let (root, workspace) = self.resolve_root(handle)?;
                let response = workspace
                    .view(&root.root, ViewRequest::Root { target })
                    .map_err(|error| self.workspace_error("root snapshot", error))?;
                let ViewResult::Root(snapshot) = response else {
                    return Err(self.protocol_error(
                        ProtocolErrorCode::Internal,
                        "workspace returned a non-root view",
                    ));
                };

                WorkspaceQueryResponse::RootSnapshot(snapshot)
            }
            WorkspaceQuery::FileSnapshot { handle, request } => {
                let (root, workspace) = self.resolve_root(handle)?;
                let response = workspace
                    .view(&root.root, ViewRequest::File(request))
                    .map_err(|error| self.workspace_error("file snapshot", error))?;
                let ViewResult::File(snapshot) = response else {
                    return Err(self.protocol_error(
                        ProtocolErrorCode::Internal,
                        "workspace returned a non-file view",
                    ));
                };

                WorkspaceQueryResponse::FileSnapshot(snapshot)
            }
            WorkspaceQuery::FileImages { handle, request } => {
                let (root, workspace) = self.resolve_root(handle)?;
                let response = workspace
                    .view(&root.root, ViewRequest::FileImages(request))
                    .map_err(|error| self.workspace_error("file images", error))?;
                let ViewResult::FileImages(images) = response else {
                    return Err(self.protocol_error(
                        ProtocolErrorCode::Internal,
                        "workspace returned a non-file-images view",
                    ));
                };

                WorkspaceQueryResponse::FileImages(images)
            }
            WorkspaceQuery::Execute { handle, request } => {
                let (root, workspace) = self.resolve_root(handle)?;
                let response =
                    self.execute_query(workspace.as_ref(), &root.root, request, payloads)?;
                WorkspaceQueryResponse::Query(response)
            }
            WorkspaceQuery::ExecuteBatch { handle, requests } => {
                let (root, workspace) = self.resolve_root(handle)?;
                let responses = requests
                    .into_iter()
                    .map(|request| {
                        self.execute_query(workspace.as_ref(), &root.root, request, payloads)
                    })
                    .collect::<Result<Vec<_>, ProtocolError>>()?;
                WorkspaceQueryResponse::QueryBatch(responses)
            }
        };

        Ok(WorkspaceResponse::QueryResult(response))
    }

    /// Return diagnostics for one open root.
    pub(super) fn diagnostics(
        &self,
        workspace: &dyn Workspace,
        root: &Path,
    ) -> Result<Vec<DiagnosticBatch>, ProtocolError> {
        let diagnostics = workspace
            .diagnostics(DiagnosticsRequest::Root(root.to_path_buf()))
            .map_err(|error| self.workspace_error("diagnostics", error))?;
        let diagnostics: Vec<_> = diagnostics
            .iter()
            .flat_map(|image| image.diagnostics.iter().cloned())
            .collect();

        Ok(DiagnosticBatch::group(&diagnostics))
    }

    /// Execute a query against the current session.
    fn execute_query(
        &self,
        workspace: &dyn Workspace,
        root: &Path,
        request: QueryRequestPayload,
        payloads: &mut PayloadWriter,
    ) -> Result<QueryResponsePayload, ProtocolError> {
        // decode the query request payload
        let request = request.decode_request().map_err(|error| {
            self.protocol_error(
                ProtocolErrorCode::InvalidPayload,
                &format!("invalid query request payload: {error}"),
            )
        })?;

        // capture request kind before dispatch
        let request_method_id = request.request.method_id();

        // require callers to choose one coherent revision
        let revision = request.expected_revision.ok_or_else(|| {
            self.protocol_error(
                ProtocolErrorCode::InvalidRequest,
                "missing expected revision for query",
            )
        })?;
        let revision = RevisionPolicy::Current(revision);
        let response = workspace
            .query(
                root,
                QueryRequest {
                    expected_revision: Some(match revision {
                        RevisionPolicy::Current(revision) | RevisionPolicy::Exact(revision) => {
                            revision
                        }
                        RevisionPolicy::Latest => {
                            return Err(self.protocol_error(
                                ProtocolErrorCode::InvalidRequest,
                                "latest revision cannot be used for protocol query execution",
                            ));
                        }
                    }),
                    request: request.request,
                },
            )
            .map_err(|error| self.workspace_error("query", error))?;

        // keep query response variants aligned with query request variants
        if response.response.method_id() != request_method_id {
            return Err(self.protocol_error(
                ProtocolErrorCode::Internal,
                &format!(
                    "query response kind mismatch: request={request_method_id:?} response={:?}",
                    response.response.method_id(),
                ),
            ));
        }

        // encode the query response payload
        let mut response =
            QueryResponsePayload::from_response(response.revision, response.response).map_err(
                |error| {
                    self.protocol_error(
                        ProtocolErrorCode::Internal,
                        &format!("failed to encode query response payload: {error}"),
                    )
                },
            )?;

        // route large query responses through deferred payload streaming
        response.payload = payloads
            .prepare(response.payload)
            .map_err(|error| self.payload_error(error))?;

        Ok(response)
    }

    /// Return rich diagnostics for one root.
    fn diagnostic_snapshots(
        &self,
        workspace: &dyn Workspace,
        root: &Path,
    ) -> Result<Vec<DiagnosticSnapshot>, ProtocolError> {
        let revision = workspace
            .revision(root)
            .map_err(|error| self.workspace_error("diagnostics", error))?;
        let diagnostics = workspace
            .diagnostics(DiagnosticsRequest::Root(root.to_path_buf()))
            .map_err(|error| self.workspace_error("diagnostics", error))?;

        Ok(diagnostics
            .iter()
            .map(|view| DiagnosticSnapshot::new(revision, view))
            .collect())
    }

    /// Return rich diagnostics for one source path.
    fn file_diagnostics(
        &self,
        workspace: &dyn Workspace,
        root: &Path,
        path: &Path,
    ) -> Result<Option<DiagnosticSnapshot>, ProtocolError> {
        if !self.path_within_root(workspace, path, root) {
            return Err(
                self.protocol_error(ProtocolErrorCode::Forbidden, "query path is outside root")
            );
        }
        let revision = workspace
            .revision(root)
            .map_err(|error| self.workspace_error("file diagnostics", error))?;

        let diagnostics = workspace
            .diagnostics(DiagnosticsRequest::File(path.to_path_buf()))
            .map_err(|error| self.workspace_error("file diagnostics", error))?;

        Ok(diagnostics
            .first()
            .map(|view| DiagnosticSnapshot::new(revision, view)))
    }
}
