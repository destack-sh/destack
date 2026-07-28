use std::path::{Path, PathBuf};

use crate::Workspace;

use super::Server;
use crate::PayloadSender;
use crate::protocol::{
    ProtocolError, ProtocolErrorCode, QueryFilePayload, QueryRequestPayload, QueryResponsePayload,
    RootId, WorkspaceResponse,
};

impl Server {
    /// Resolve one source file for semantic queries.
    pub(super) fn resolve_query_file(
        &self,
        handle: RootId,
        path: PathBuf,
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.require_session()?;
        let (root, workspace) = self.resolve_root(handle)?;

        // require the requested path to belong to the selected root
        if !self.path_within_root(workspace.as_ref(), &path, &root.root) {
            return Err(
                self.protocol_error(ProtocolErrorCode::Forbidden, "query path is outside root")
            );
        }

        // resolve the exact semantic file state
        let file = workspace
            .resolve_query_file(&root.root, path)
            .map_err(|error| self.workspace_error("query file", error))?;
        let file = file.as_ref().map(QueryFilePayload::from);

        Ok(WorkspaceResponse::ResolveQueryFile(file))
    }

    /// Run one semantic query.
    pub(super) fn run_query(
        &self,
        handle: RootId,
        request: QueryRequestPayload,
        payloads: &mut PayloadSender,
    ) -> Result<WorkspaceResponse, ProtocolError> {
        self.require_session()?;
        let (root, workspace) = self.resolve_root(handle)?;
        let response = self.execute_query(workspace.as_ref(), &root.root, request, payloads)?;

        Ok(WorkspaceResponse::RunQuery(response))
    }

    /// Execute a query against the current session.
    fn execute_query(
        &self,
        workspace: &dyn Workspace,
        root: &Path,
        request: QueryRequestPayload,
        payloads: &mut PayloadSender,
    ) -> Result<QueryResponsePayload, ProtocolError> {
        // decode the query request payload
        let request = request.decode_request().map_err(|error| {
            self.protocol_error(
                ProtocolErrorCode::InvalidPayload,
                &format!("invalid query request payload: {error}"),
            )
        })?;

        // capture the request method before dispatch
        let request_method = request.request.method();

        let response = workspace
            .run_query(root, request)
            .map_err(|error| self.workspace_error("query", error))?;

        // keep query response variants aligned with query request variants
        if response.response.method() != request_method {
            return Err(self.protocol_error(
                ProtocolErrorCode::Internal,
                &format!(
                    "query response method mismatch: request={request_method:?} response={:?}",
                    response.response.method(),
                ),
            ));
        }

        // encode the query response payload
        let mut response = QueryResponsePayload::from_response(response).map_err(|error| {
            self.protocol_error(
                ProtocolErrorCode::Internal,
                &format!("failed to encode query response payload: {error}"),
            )
        })?;

        // route large query responses through deferred payload streaming
        response.payload = payloads
            .prepare(response.payload)
            .map_err(|error| self.payload_error(error))?;

        Ok(response)
    }
}
