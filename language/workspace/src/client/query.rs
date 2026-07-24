use std::path::PathBuf;

use destack_repository::Revision;

use super::{Client, ClientError};
use crate::FileImage;
use crate::protocol::{
    DiagnosticBatch, DiagnosticSnapshot, FileImagesRequest, FileSnapshot, FileSnapshotRequest,
    QueryRequestBody, QueryRequestPayload, QueryResponseBody, RootId, WorkspaceQuery,
    WorkspaceQueryResponse, WorkspaceRequest, WorkspaceResponse,
};

impl Client {
    /// Request diagnostics for a workspace root handle.
    pub fn diagnostics(&self, handle: RootId) -> Result<Vec<DiagnosticBatch>, ClientError> {
        // send the diagnostics query
        let response = self.send_request(WorkspaceRequest::Query(WorkspaceQuery::Diagnostics {
            handle,
        }))?;

        // decode the diagnostics response
        match response {
            WorkspaceResponse::QueryResult(WorkspaceQueryResponse::Diagnostics(response)) => {
                Ok(response)
            }
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("diagnostics query", other)),
        }
    }

    /// Request rich diagnostics for a workspace root handle.
    pub fn diagnostic_snapshots(
        &self,
        handle: RootId,
    ) -> Result<Vec<DiagnosticSnapshot>, ClientError> {
        // send the diagnostics query
        let response = self.send_request(WorkspaceRequest::Query(
            WorkspaceQuery::DiagnosticSnapshots { handle },
        ))?;

        // decode the diagnostics response
        match response {
            WorkspaceResponse::QueryResult(WorkspaceQueryResponse::DiagnosticSnapshots(
                response,
            )) => Ok(response),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response(
                "diagnostic snapshots query",
                other,
            )),
        }
    }

    /// Request diagnostics for one file path.
    pub fn file_diagnostics(
        &self,
        handle: RootId,
        path: PathBuf,
    ) -> Result<Option<DiagnosticSnapshot>, ClientError> {
        // send the file diagnostics query
        let response =
            self.send_request(WorkspaceRequest::Query(WorkspaceQuery::FileDiagnostics {
                handle,
                path,
            }))?;

        // decode the file diagnostics response
        match response {
            WorkspaceResponse::QueryResult(WorkspaceQueryResponse::FileDiagnostics(response)) => {
                Ok(response)
            }
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("file diagnostics query", other)),
        }
    }

    /// Request whether one file is open.
    pub fn file_open(&self, handle: RootId, path: PathBuf) -> Result<bool, ClientError> {
        // send the file open query
        let response = self.send_request(WorkspaceRequest::Query(WorkspaceQuery::FileOpen {
            handle,
            path,
        }))?;

        // decode the file open response
        match response {
            WorkspaceResponse::QueryResult(WorkspaceQueryResponse::FileOpen(response)) => {
                Ok(response)
            }
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("file open query", other)),
        }
    }

    /// Request the current semantic revision for a workspace root handle.
    pub fn current_revision(&self, handle: RootId) -> Result<Revision, ClientError> {
        // send the revision query
        let response =
            self.send_request(WorkspaceRequest::Query(WorkspaceQuery::CurrentRevision {
                handle,
            }))?;

        // decode the revision response
        match response {
            WorkspaceResponse::QueryResult(WorkspaceQueryResponse::CurrentRevision(response)) => {
                Ok(response)
            }
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("revision query", other)),
        }
    }

    /// Request a source file snapshot.
    pub fn file_snapshot(
        &self,
        handle: RootId,
        request: FileSnapshotRequest,
    ) -> Result<Option<FileSnapshot>, ClientError> {
        // send the file snapshot query
        let response =
            self.send_request(WorkspaceRequest::Query(WorkspaceQuery::FileSnapshot {
                handle,
                request,
            }))?;

        // decode the file snapshot response
        match response {
            WorkspaceResponse::QueryResult(WorkspaceQueryResponse::FileSnapshot(response)) => {
                Ok(response)
            }
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("file snapshot query", other)),
        }
    }

    /// Request source file images for one revision.
    pub fn file_images(
        &self,
        handle: RootId,
        request: FileImagesRequest,
    ) -> Result<Vec<FileImage>, ClientError> {
        // send the file images query
        let response = self.send_request(WorkspaceRequest::Query(WorkspaceQuery::FileImages {
            handle,
            request,
        }))?;

        // decode the file images response
        match response {
            WorkspaceResponse::QueryResult(WorkspaceQueryResponse::FileImages(response)) => {
                Ok(response)
            }
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("file images query", other)),
        }
    }

    /// Execute one semantic query for a workspace root handle.
    pub fn execute_query(
        &self,
        handle: RootId,
        request: QueryRequestBody,
    ) -> Result<QueryResponseBody, ClientError> {
        // encode the query request
        let request = QueryRequestPayload::from_body(request).map_err(ClientError::QueryPayload)?;

        // send the semantic query
        let response = self.send_request(WorkspaceRequest::Query(WorkspaceQuery::Execute {
            handle,
            request,
        }))?;

        // decode the query response
        match response {
            WorkspaceResponse::QueryResult(WorkspaceQueryResponse::Query(response)) => response
                .decode_response()
                .map_err(ClientError::QueryPayload),
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("query", other)),
        }
    }

    /// Execute semantic queries for a workspace root handle.
    pub fn execute_query_batch(
        &self,
        handle: RootId,
        requests: Vec<QueryRequestBody>,
    ) -> Result<Vec<QueryResponseBody>, ClientError> {
        // encode query requests
        let requests = requests
            .into_iter()
            .map(QueryRequestPayload::from_body)
            .collect::<Result<Vec<_>, _>>()
            .map_err(ClientError::QueryPayload)?;

        // send the semantic query batch
        let response =
            self.send_request(WorkspaceRequest::Query(WorkspaceQuery::ExecuteBatch {
                handle,
                requests,
            }))?;

        // decode the query response batch
        match response {
            WorkspaceResponse::QueryResult(WorkspaceQueryResponse::QueryBatch(responses)) => {
                responses
                    .into_iter()
                    .map(|response| response.decode_response())
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(ClientError::QueryPayload)
            }
            WorkspaceResponse::Error(error) => Err(ClientError::Server(error)),
            other => Err(Self::unexpected_response("query batch", other)),
        }
    }
}
