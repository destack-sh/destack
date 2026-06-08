use std::path::Path;

use destack_query::QueryModule;
use destack_repository::Revision;
use destack_source::{File, PackageId, ProfileId, TargetId};
use destack_workspace::{DiagnosticView, Error, FileImage, RevisionPolicy};

use crate::Workspace;

use super::{
    DaemonQuery, DaemonQueryResponse, DaemonResponse, DiagnosticBatch, DiagnosticSnapshot,
    FileImagesRequest, FileSnapshot, FileSnapshotRequest, FileUpdateImage, PayloadWriter,
    ProtocolError, ProtocolErrorCode, QueryRequestPayload, QueryResponsePayload, RootSnapshot,
    Server, diagnostics_to_batches,
};

impl Server {
    /// Handle a query request.
    pub(super) fn handle_query(
        &self,
        query: DaemonQuery,
        payloads: &mut PayloadWriter,
    ) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let response = match query {
            DaemonQuery::Diagnostics { handle } => {
                let (root, workspace) = self.resolve_root(handle)?;
                let diagnostics = self.diagnostics(workspace.as_ref(), &root.root)?;
                DaemonQueryResponse::Diagnostics(diagnostics)
            }
            DaemonQuery::DiagnosticSnapshots { handle } => {
                let (root, workspace) = self.resolve_root(handle)?;
                let diagnostics = self.diagnostic_snapshots(workspace.as_ref(), &root.root)?;
                DaemonQueryResponse::DiagnosticSnapshots(diagnostics)
            }
            DaemonQuery::FileDiagnostics { handle, path } => {
                let (root, workspace) = self.resolve_root(handle)?;
                let diagnostics = self.file_diagnostics(workspace.as_ref(), &root.root, &path)?;
                DaemonQueryResponse::FileDiagnostics(diagnostics)
            }
            DaemonQuery::CurrentRevision { handle } => {
                let (root, workspace) = self.resolve_root(handle)?;
                let revision = workspace
                    .revision(&root.root)
                    .map_err(|error| self.workspace_error("current revision", error))?;
                DaemonQueryResponse::CurrentRevision(revision)
            }
            DaemonQuery::RootSnapshot { handle, target } => {
                let (root, workspace) = self.resolve_root(handle)?;
                let snapshot =
                    self.root_snapshot(workspace.as_ref(), &root.root, target.as_deref())?;
                DaemonQueryResponse::RootSnapshot(snapshot)
            }
            DaemonQuery::FileSnapshot { handle, request } => {
                let (root, workspace) = self.resolve_root(handle)?;
                let snapshot = self.file_snapshot(workspace.as_ref(), &root.root, request)?;
                DaemonQueryResponse::FileSnapshot(snapshot)
            }
            DaemonQuery::FileImages { handle, request } => {
                let (_, workspace) = self.resolve_root(handle)?;
                let images = self.file_images(workspace.as_ref(), request)?;
                DaemonQueryResponse::FileImages(images)
            }
            DaemonQuery::Execute { handle, request } => {
                let (root, workspace) = self.resolve_root(handle)?;
                let response =
                    self.execute_query(workspace.as_ref(), &root.root, request, payloads)?;
                DaemonQueryResponse::Query(response)
            }
            DaemonQuery::ExecuteBatch { handle, requests } => {
                let (root, workspace) = self.resolve_root(handle)?;
                let responses = requests
                    .into_iter()
                    .map(|request| {
                        self.execute_query(workspace.as_ref(), &root.root, request, payloads)
                    })
                    .collect::<Result<Vec<_>, ProtocolError>>()?;
                DaemonQueryResponse::QueryBatch(responses)
            }
        };

        Ok(DaemonResponse::QueryResult(response))
    }

    /// Return diagnostics for one open root.
    pub(super) fn diagnostics(
        &self,
        workspace: &Workspace,
        root: &Path,
    ) -> Result<Vec<DiagnosticBatch>, ProtocolError> {
        let images = workspace
            .root_diagnostics(root)
            .map_err(|error| self.workspace_error("diagnostics", error))?;
        let diagnostics: Vec<_> = images
            .into_iter()
            .flat_map(|image| image.diagnostics)
            .collect();

        Ok(diagnostics_to_batches(&diagnostics))
    }

    /// Execute a query against the current session.
    fn execute_query(
        &self,
        workspace: &Workspace,
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
            .query_root(root, request.request, revision)
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
        workspace: &Workspace,
        root: &Path,
    ) -> Result<Vec<DiagnosticSnapshot>, ProtocolError> {
        let revision = workspace
            .revision(root)
            .map_err(|error| self.workspace_error("diagnostics", error))?;
        let diagnostics = workspace
            .root_diagnostics(root)
            .map_err(|error| self.workspace_error("diagnostics", error))?;

        Ok(diagnostics
            .iter()
            .map(|view| diagnostic_snapshot(view, revision))
            .collect())
    }

    /// Return rich diagnostics for one source path.
    fn file_diagnostics(
        &self,
        workspace: &Workspace,
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
            .file_diagnostics(path)
            .map_err(|error| self.workspace_error("file diagnostics", error))?;

        Ok(diagnostics
            .as_ref()
            .map(|view| diagnostic_snapshot(view, revision)))
    }

    /// Return query context for one root.
    fn root_snapshot(
        &self,
        workspace: &Workspace,
        root: &Path,
        target: Option<&str>,
    ) -> Result<RootSnapshot, ProtocolError> {
        let revision = workspace
            .revision(root)
            .map_err(|error| self.workspace_error("root snapshot", error))?;
        let package_id = workspace
            .repository
            .nearest_package(revision, root)
            .map_err(|error| self.repository_error("root snapshot", error))?
            .map(|package| package.id);
        let profile_ids = if let Some(package_id) = package_id {
            self.package_profiles(workspace, revision, package_id, target)?
        } else {
            Vec::new()
        };

        Ok(RootSnapshot {
            revision,
            profile_ids,
        })
    }

    /// Return one source file snapshot.
    fn file_snapshot(
        &self,
        workspace: &Workspace,
        root: &Path,
        request: FileSnapshotRequest,
    ) -> Result<Option<FileSnapshot>, ProtocolError> {
        if !self.path_within_root(workspace, &request.path, root) {
            return Err(
                self.protocol_error(ProtocolErrorCode::Forbidden, "query path is outside root")
            );
        }

        let view = match workspace.file_view(&request.path) {
            Ok(view) => view,
            Err(Error::FileMissing { .. }) => return Ok(None),
            Err(error) => return Err(self.workspace_error("file snapshot", error)),
        };
        let repository = view.repository();
        let revision = view.revision();
        let file_id = view.file_id;
        let module_id = repository
            .module_id_for_file(revision, file_id)
            .map_err(|error| self.repository_error("file snapshot", error))?;
        let module = if let Some(module_id) = module_id {
            let module = repository
                .module(revision, module_id)
                .map_err(|error| self.repository_error("file snapshot", error))?
                .ok_or_else(|| {
                    self.protocol_error(ProtocolErrorCode::NotFound, "query module is missing")
                })?;
            let profile_ids = self.package_profiles(
                workspace,
                revision,
                module.package_id,
                request.target.as_deref(),
            )?;
            profile_ids.first().copied().map(|profile_id| QueryModule {
                module_id,
                profile_id,
            })
        } else {
            None
        };
        let file = protocol_image_from_file(view.file.as_ref());

        Ok(Some(FileSnapshot {
            revision,
            file_id,
            module,
            file,
        }))
    }

    /// Return source file images for one revision.
    fn file_images(
        &self,
        workspace: &Workspace,
        request: FileImagesRequest,
    ) -> Result<Vec<FileUpdateImage>, ProtocolError> {
        let mut images = Vec::new();
        for file_id in request.file_ids {
            let Some(file) = workspace
                .repository
                .file(request.revision, file_id)
                .map_err(|error| self.repository_error("file images", error))?
            else {
                continue;
            };
            images.push(protocol_image_from_file(file.as_ref()));
        }

        Ok(images)
    }

    /// Return selected profile ids for one package.
    fn package_profiles(
        &self,
        workspace: &Workspace,
        revision: Revision,
        package_id: PackageId,
        target: Option<&str>,
    ) -> Result<Vec<ProfileId>, ProtocolError> {
        // use the explicit target when supplied by the client
        if let Some(target) = target {
            let target_id = TargetId::new(package_id, target);
            let profile = workspace
                .repository
                .profile_for_target(revision, target_id)
                .map_err(|error| self.repository_error("profile", error))?;

            return Ok(vec![profile.id()]);
        }

        // otherwise use the package default target when one is unambiguous
        let default_target = workspace
            .repository
            .package_default_target(revision, package_id)
            .map_err(|error| self.repository_error("profile", error))?;
        let Some((target_id, _)) = default_target else {
            return Ok(Vec::new());
        };
        let profile = workspace
            .repository
            .profile_for_target(revision, target_id)
            .map_err(|error| self.repository_error("profile", error))?;

        Ok(vec![profile.id()])
    }

    /// Convert a repository error to a protocol error.
    fn repository_error(
        &self,
        context: &str,
        error: destack_repository::RepositoryError,
    ) -> ProtocolError {
        self.protocol_error(
            ProtocolErrorCode::Internal,
            &format!("{context} failed: {error}"),
        )
    }
}

/// Convert a diagnostic view into protocol shape.
fn diagnostic_snapshot(view: &DiagnosticView, revision: Revision) -> DiagnosticSnapshot {
    DiagnosticSnapshot {
        revision,
        file: protocol_image_from_file(view.file.as_ref()),
        diagnostic_uri: view.diagnostic_uri.clone(),
        diagnostic_version: view.diagnostic_version,
        diagnostics: view.diagnostics.clone(),
    }
}

/// Convert one source file into protocol shape.
fn protocol_image_from_file(file: &File) -> FileUpdateImage {
    let image = FileImage::from(file);

    FileUpdateImage {
        id: image.id,
        name: image.name,
        uri: image.uri,
        path: image.path,
        file_type: image.file_type,
        content: image.content,
    }
}
