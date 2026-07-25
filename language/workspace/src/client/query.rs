use std::path::PathBuf;

use super::{Client, ClientError};
use crate::protocol::{QueryRequestPayload, RootId, WorkspaceRequest, WorkspaceResponse};
use crate::{QueryFile, RunQueryRequest, RunQueryResponse};

impl Client {
    /// Resolve one source file for semantic queries.
    pub fn resolve_query_file(
        &self,
        handle: RootId,
        path: PathBuf,
    ) -> Result<Option<QueryFile>, ClientError> {
        // send the query file request
        let response = self.send_request(WorkspaceRequest::ResolveQueryFile { handle, path })?;

        // decode the query file response
        match response {
            WorkspaceResponse::ResolveQueryFile(response) => response
                .map(|file| {
                    QueryFile::try_from(file).map_err(|error| {
                        ClientError::UnexpectedResponse(format!(
                            "query file payload is invalid: {error}"
                        ))
                    })
                })
                .transpose(),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("resolve query file", other)),
        }
    }

    /// Run one semantic query for a workspace root.
    pub fn run_query(
        &self,
        handle: RootId,
        request: RunQueryRequest,
    ) -> Result<RunQueryResponse, ClientError> {
        // encode the query request
        let request =
            QueryRequestPayload::from_request(request).map_err(ClientError::QueryPayload)?;

        // send the semantic query
        let response = self.send_request(WorkspaceRequest::RunQuery { handle, request })?;

        // decode the query response
        match response {
            WorkspaceResponse::RunQuery(response) => response
                .decode_response()
                .map_err(ClientError::QueryPayload),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("run query", other)),
        }
    }
}
