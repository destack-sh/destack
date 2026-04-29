use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use destack_query as query;
use destack_query::{QueryRequestEnvelope, QueryResponseEnvelope};
use destack_session::Session;
use destack_source::{Diagnostic, DiagnosticCollection, File, FileId, Span, Uri};
use destack_workspace::{Repository, Revision, RevisionPin};

use super::{
    DocumentDiagnosticSnapshot, LanguageService, LanguageServiceError, WorkspaceDiagnosticSnapshot,
};

impl LanguageService {
    /// Run one callback under the coherent read lock for a workspace root.
    fn with_workspace_read<T, F>(&self, root: &Path, callback: F) -> Result<T, LanguageServiceError>
    where
        F: FnOnce(&Session, &RevisionPin) -> Result<T, LanguageServiceError>,
    {
        // resolve the owning workspace and repository
        let workspace = self.workspace_for_root(root)?;

        // hold the workspace read section for the full query
        let _query_guard = workspace.enter_query();
        let repository = workspace.repository();
        let revision_pin = repository
            .pin(workspace.revision())
            .map_err(LanguageServiceError::from)?;

        callback(workspace.as_ref(), &revision_pin)
    }

    /// Prepare the default query artifact root for one document path.
    pub fn prepare_default_query_for_path(&self, path: &Path) -> Result<(), LanguageServiceError> {
        let workspace = self.workspace_for_document_path(path)?;
        let module_id = workspace.admit_module_for_path(path).map_err(|error| {
            LanguageServiceError::QueryNotReady {
                detail: format!("query preparation failed for {}: {error}", path.display()),
            }
        })?;
        let revision = workspace.revision();
        let profile_id = workspace
            .repository()
            .default_profile_id_for_module(revision, module_id)
            .map_err(|error| LanguageServiceError::QueryNotReady {
                detail: format!("query preparation failed for {}: {error}", path.display()),
            })?;
        let artifact_key = query::default_document_artifact_key(module_id, profile_id);

        workspace.provide(&[artifact_key]).map_err(|error| {
            LanguageServiceError::QueryNotReady {
                detail: format!("query preparation failed for {}: {error}", path.display()),
            }
        })?;

        Ok(())
    }

    /// Run one callback with a coherent file snapshot for a path.
    pub fn with_file_for_path<T, F>(
        &self,
        path: &Path,
        callback: F,
    ) -> Result<T, LanguageServiceError>
    where
        F: FnOnce(&Repository, FileId, Arc<File>, Revision) -> Result<T, LanguageServiceError>,
    {
        // resolve the workspace root once before entering the read section
        let root = self.workspace_root_for_path(path)?;

        self.with_workspace_read(&root, |workspace, revision_pin| {
            let repository = revision_pin.repository();
            let revision = revision_pin.revision();

            // require one tracked file for the request
            let file_id = workspace.file_id_for_path(path)?.ok_or_else(|| {
                LanguageServiceError::FileNotTracked {
                    path: path.to_path_buf(),
                }
            })?;
            let file = workspace.file_for_id(file_id)?;

            callback(repository, file_id, file, revision)
        })
    }

    /// Return a current diagnostic snapshot for one document path.
    pub fn document_diagnostics_for_path(
        &self,
        path: &Path,
    ) -> Result<Option<DocumentDiagnosticSnapshot>, LanguageServiceError> {
        let root = self.workspace_root_for_path(path)?;

        self.with_workspace_read(&root, |workspace, revision_pin| {
            let Some((file_id, file)) = workspace.file_for_path(path)? else {
                return Ok(None);
            };
            let diagnostics =
                current_root_diagnostics_by_file(workspace, revision_pin.repository())?
                    .remove(&file_id)
                    .unwrap_or_default();

            Ok(Some(DocumentDiagnosticSnapshot { file, diagnostics }))
        })
    }

    /// Return current diagnostic snapshots for every workspace-owned file.
    pub fn workspace_diagnostics(
        &self,
    ) -> Result<Vec<WorkspaceDiagnosticSnapshot>, LanguageServiceError> {
        let mut roots: Vec<_> = self
            .workspaces_by_root
            .iter()
            .map(|entry| entry.key().clone())
            .collect();
        roots.sort();

        let mut snapshots = Vec::new();
        for root in roots {
            let handle_snapshots = self.diagnostics_for_root(&root)?;

            snapshots.extend(handle_snapshots);
        }

        Ok(snapshots)
    }

    /// Return current diagnostic snapshots for one workspace root.
    pub fn diagnostics_for_root(
        &self,
        root: &Path,
    ) -> Result<Vec<WorkspaceDiagnosticSnapshot>, LanguageServiceError> {
        self.with_workspace_read(root, |workspace, revision_pin| {
            let mut diagnostics_by_file =
                current_root_diagnostics_by_file(workspace, revision_pin.repository())?;

            // tracked documents
            let mut tracked_documents = std::collections::HashMap::new();
            for (path, uri, version) in workspace.open_file_identities() {
                let Some(file_id) = workspace.file_id_for_path(&path)? else {
                    continue;
                };

                diagnostics_by_file.entry(file_id).or_insert(Vec::new());
                tracked_documents.insert(file_id, (uri, version));
            }

            let mut snapshots = Vec::new();
            for (file_id, diagnostics) in diagnostics_by_file {
                let Ok(file) = workspace.file_for_id(file_id) else {
                    continue;
                };
                let tracked_document = tracked_documents.get(&file_id);
                let publish_uri = tracked_document
                    .map(|(uri, _)| uri.clone())
                    .or_else(|| file.path.as_ref().map(Uri::from_file_path))
                    .unwrap_or_else(|| file.uri.clone());
                let publish_version = tracked_document.map(|(_, version)| *version);

                snapshots.push(WorkspaceDiagnosticSnapshot {
                    file,
                    publish_uri,
                    publish_version,
                    diagnostics,
                });
            }

            snapshots.sort_by(|left, right| {
                let left_key = left
                    .file
                    .path
                    .as_ref()
                    .map(|path| path.to_string_lossy().into_owned())
                    .unwrap_or_else(|| left.file.uri.to_string());
                let right_key = right
                    .file
                    .path
                    .as_ref()
                    .map(|path| path.to_string_lossy().into_owned())
                    .unwrap_or_else(|| right.file.uri.to_string());

                left_key.cmp(&right_key)
            });

            Ok(snapshots)
        })
    }

    /// Execute one coherent file-backed read query for a path.
    pub fn execute_file_read_query_for_path<F>(
        &self,
        path: &Path,
        build_request: F,
    ) -> Result<Option<(FileId, Arc<File>, QueryResponseEnvelope)>, LanguageServiceError>
    where
        F: FnOnce(FileId, &File) -> Option<query::QueryRequest>,
    {
        self.prepare_default_query_for_path(path)?;

        self.with_file_for_path(path, |repository, file_id, file, revision| {
            // build the request from the exact file snapshot used for result conversion
            let Some(request) = build_request(file_id, &file) else {
                return Ok(None);
            };
            let response =
                self.execute_query_request(repository, revision, Some(file_id), request)?;

            Ok(Some((
                file_id,
                file,
                QueryResponseEnvelope { revision, response },
            )))
        })
    }

    /// Execute a read query for the workspace that owns the path.
    pub fn execute_read_query_for_path(
        &self,
        path: &Path,
        request: query::QueryRequest,
    ) -> Result<QueryResponseEnvelope, LanguageServiceError> {
        // read queries never carry revision preconditions
        let envelope = QueryRequestEnvelope {
            expected_revision: None,
            request,
        };

        // route through read envelope execution
        self.execute_read_query_envelope_for_path(path, envelope)
    }

    /// Execute a read query envelope for the workspace that owns the path.
    pub fn execute_read_query_envelope_for_path(
        &self,
        path: &Path,
        envelope: QueryRequestEnvelope,
    ) -> Result<QueryResponseEnvelope, LanguageServiceError> {
        let root = self.workspace_root_for_path(path)?;

        self.execute_read_query_envelope_for_workspace_root(&root, envelope)
    }

    /// Execute a read query envelope for any workspace-owned path.
    pub fn execute_read_query_envelope_for_owned_path(
        &self,
        path: &Path,
        envelope: QueryRequestEnvelope,
    ) -> Result<QueryResponseEnvelope, LanguageServiceError> {
        let root = self.workspace_root_for_path(path)?;
        self.execute_read_query_envelope_for_workspace_root(&root, envelope)
    }

    /// Execute a read query for a specific workspace root.
    pub fn execute_read_query_for_workspace_root(
        &self,
        root: &Path,
        request: query::QueryRequest,
    ) -> Result<QueryResponseEnvelope, LanguageServiceError> {
        // read queries never carry revision preconditions
        let envelope = QueryRequestEnvelope {
            expected_revision: None,
            request,
        };

        // route through read envelope execution
        self.execute_read_query_envelope_for_workspace_root(root, envelope)
    }

    /// Execute a read query envelope for a specific workspace root.
    pub fn execute_read_query_envelope_for_workspace_root(
        &self,
        root: &Path,
        envelope: QueryRequestEnvelope,
    ) -> Result<QueryResponseEnvelope, LanguageServiceError> {
        // reject any non-read requests on the read path
        let request_mode = envelope.request.execution_mode();
        if request_mode != query::QueryExecutionMode::Read {
            return Err(LanguageServiceError::QueryExecutionModeMismatch {
                method: envelope.request.method_id(),
                expected: query::QueryExecutionMode::Read,
                actual: request_mode,
            });
        }

        // reject revision preconditions on read path requests
        if let Some(expected_revision) = envelope.expected_revision {
            return Err(LanguageServiceError::UnexpectedExpectedRevisionOnRead {
                expected_revision,
            });
        }

        self.prepare_query_request(&envelope.request)?;

        self.with_workspace_read(root, |_workspace, revision_pin| {
            // dispatch pure read query execution
            let repository = revision_pin.repository();
            let revision = revision_pin.revision();
            let response =
                self.execute_query_request(repository, revision, None, envelope.request)?;

            Ok(QueryResponseEnvelope { revision, response })
        })
    }

    /// Execute a write query envelope for the workspace that owns the path.
    pub fn execute_write_query_envelope_for_path(
        &self,
        path: &Path,
        envelope: QueryRequestEnvelope,
    ) -> Result<QueryResponseEnvelope, LanguageServiceError> {
        let root = self.workspace_root_for_path(path)?;

        self.execute_write_query_envelope_for_workspace_root(&root, envelope)
    }

    /// Execute a write query envelope for any workspace-owned path.
    pub fn execute_write_query_envelope_for_owned_path(
        &self,
        path: &Path,
        envelope: QueryRequestEnvelope,
    ) -> Result<QueryResponseEnvelope, LanguageServiceError> {
        let root = self.workspace_root_for_path(path)?;
        self.execute_write_query_envelope_for_workspace_root(&root, envelope)
    }

    /// Execute a write query envelope for a specific workspace root.
    pub fn execute_write_query_envelope_for_workspace_root(
        &self,
        root: &Path,
        envelope: QueryRequestEnvelope,
    ) -> Result<QueryResponseEnvelope, LanguageServiceError> {
        // reject any non-write requests on the write path
        let request_mode = envelope.request.execution_mode();
        if request_mode != query::QueryExecutionMode::Write {
            return Err(LanguageServiceError::QueryExecutionModeMismatch {
                method: envelope.request.method_id(),
                expected: query::QueryExecutionMode::Write,
                actual: request_mode,
            });
        }

        // resolve the owning workspace session and current revision
        let workspace = self.workspace_for_root(root)?;

        // serialize write queries with all other workspace mutations
        let _mutation_guard = workspace.enter_mutation();
        let current_revision = workspace.revision();

        // require a matching revision precondition for write requests
        let expected_revision = envelope
            .expected_revision
            .ok_or(LanguageServiceError::MissingExpectedRevision)?;
        if expected_revision != current_revision {
            return Err(LanguageServiceError::StaleRevision {
                expected: expected_revision,
                current: current_revision,
            });
        }

        // root the checked revision for the full request
        let repository = workspace.repository();
        let revision_pin = repository
            .pin(workspace.revision())
            .map_err(LanguageServiceError::from)?;

        // dispatch mutating query execution
        let revision = revision_pin.revision();
        let response = self.execute_query_request(
            revision_pin.repository(),
            revision,
            None,
            envelope.request,
        )?;

        Ok(QueryResponseEnvelope { revision, response })
    }

    /// Build a query response for a request payload.
    fn execute_query_request(
        &self,
        repository: &Repository,
        revision: Revision,
        bound_file_id: Option<FileId>,
        request: query::QueryRequest,
    ) -> Result<query::QueryResponse, LanguageServiceError> {
        // dispatch by query request variant
        let response = match request {
            query::QueryRequest::Completion(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let mut items = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        query::completions(
                            repository,
                            revision,
                            file_id,
                            params.offset,
                            params.trigger,
                        )
                    }
                    None => Vec::new(),
                };

                if !params.include_imports {
                    items.retain(|item| item.additional_text_edits.is_empty());
                }

                query::QueryResponse::Completion(query::CompletionResponse {
                    items,
                    is_incomplete: false,
                })
            }
            query::QueryRequest::Hover(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let hover = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        query::hover(repository, revision, file_id, params.offset)
                    }
                    None => None,
                };

                query::QueryResponse::Hover(query::HoverResponse { hover })
            }
            query::QueryRequest::SignatureHelp(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let help = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        query::signature_help(repository, revision, file_id, params.offset)
                    }
                    None => None,
                };

                query::QueryResponse::SignatureHelp(query::SignatureHelpResponse { help })
            }
            query::QueryRequest::InlayHints(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let hints = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        let range = self.span_for_offsets(file_id, params.start, params.end);
                        query::inlay_hints(repository, revision, file_id, range)
                    }
                    None => Vec::new(),
                };

                query::QueryResponse::InlayHints(query::InlayHintsResponse { hints })
            }
            query::QueryRequest::CodeLenses(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let lenses = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        query::code_lenses(repository, revision, file_id)
                    }
                    None => Vec::new(),
                };

                query::QueryResponse::CodeLenses(query::CodeLensesResponse { lenses })
            }
            query::QueryRequest::ResolveCodeLens(params) => {
                let lens = query::resolve_code_lens(&params.lens);
                query::QueryResponse::ResolveCodeLens(query::ResolveCodeLensResponse { lens })
            }
            query::QueryRequest::FoldingRanges(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let ranges = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        query::folding_ranges(repository, revision, file_id)
                    }
                    None => Vec::new(),
                };

                query::QueryResponse::FoldingRanges(query::FoldingRangesResponse { ranges })
            }
            query::QueryRequest::SemanticTokens(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let tokens = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        query::semantic_tokens(repository, revision, file_id)
                    }
                    None => Vec::new(),
                };

                query::QueryResponse::SemanticTokens(query::SemanticTokensResponse { tokens })
            }
            query::QueryRequest::SemanticTokensRange(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let tokens = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        let range = self.span_for_offsets(file_id, params.start, params.end);
                        query::semantic_tokens_range(repository, revision, file_id, range)
                    }
                    None => Vec::new(),
                };

                query::QueryResponse::SemanticTokensRange(query::SemanticTokensResponse { tokens })
            }
            query::QueryRequest::DocumentSymbols(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let symbols = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        query::document_symbols(repository, revision, file_id)
                    }
                    None => Vec::new(),
                };

                query::QueryResponse::DocumentSymbols(query::DocumentSymbolsResponse { symbols })
            }
            query::QueryRequest::WorkspaceSymbols(params) => {
                let symbols = query::workspace_symbols(
                    repository,
                    revision,
                    &params.query,
                    params.max_results as usize,
                );
                query::QueryResponse::WorkspaceSymbols(query::WorkspaceSymbolsResponse { symbols })
            }
            query::QueryRequest::DocumentLinks(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let links = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        query::document_links(repository, revision, file_id)
                    }
                    None => Vec::new(),
                };

                query::QueryResponse::DocumentLinks(query::DocumentLinksResponse { links })
            }
            query::QueryRequest::ResolveDocumentLink(params) => {
                let link = query::resolve_document_link(&params.link);
                query::QueryResponse::ResolveDocumentLink(query::ResolveDocumentLinkResponse {
                    link,
                })
            }
            query::QueryRequest::DocumentHighlight(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let highlights = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        query::document_highlights(repository, revision, file_id, params.offset)
                    }
                    None => Vec::new(),
                };

                query::QueryResponse::DocumentHighlight(query::DocumentHighlightResponse {
                    highlights,
                })
            }
            query::QueryRequest::SelectionRanges(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let ranges = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        query::selection_ranges(repository, revision, file_id, &params.offsets)
                    }
                    None => Vec::new(),
                };

                query::QueryResponse::SelectionRanges(query::SelectionRangesResponse { ranges })
            }
            query::QueryRequest::GotoDefinition(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let result = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        query::goto_definition(repository, revision, file_id, params.offset)
                    }
                    None => None,
                };

                query::QueryResponse::GotoDefinition(query::GotoDefinitionResponse { result })
            }
            query::QueryRequest::GotoDeclaration(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let result = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        query::goto_declaration(repository, revision, file_id, params.offset)
                    }
                    None => None,
                };

                query::QueryResponse::GotoDeclaration(query::GotoDeclarationResponse { result })
            }
            query::QueryRequest::GotoTypeDefinition(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let result = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        query::goto_type_definition(repository, revision, file_id, params.offset)
                    }
                    None => None,
                };

                query::QueryResponse::GotoTypeDefinition(query::GotoTypeDefinitionResponse {
                    result,
                })
            }
            query::QueryRequest::GotoImplementation(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let result = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        query::goto_implementation(repository, revision, file_id, params.offset)
                    }
                    None => None,
                };

                query::QueryResponse::GotoImplementation(query::GotoImplementationResponse {
                    result,
                })
            }
            query::QueryRequest::FindReferences(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let result = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        query::find_references(
                            repository,
                            revision,
                            file_id,
                            params.offset,
                            params.include_declaration,
                        )
                    }
                    None => None,
                };

                query::QueryResponse::FindReferences(query::FindReferencesResponse { result })
            }
            query::QueryRequest::PrepareCallHierarchy(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let item = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        query::prepare_call_hierarchy(repository, revision, file_id, params.offset)
                    }
                    None => None,
                };

                query::QueryResponse::PrepareCallHierarchy(query::PrepareCallHierarchyResponse {
                    item,
                })
            }
            query::QueryRequest::CallHierarchyIncoming(params) => {
                let calls = query::incoming_calls(repository, revision, &params.item);
                query::QueryResponse::CallHierarchyIncoming(query::CallHierarchyIncomingResponse {
                    calls,
                })
            }
            query::QueryRequest::CallHierarchyOutgoing(params) => {
                let calls = query::outgoing_calls(repository, revision, &params.item);
                query::QueryResponse::CallHierarchyOutgoing(query::CallHierarchyOutgoingResponse {
                    calls,
                })
            }
            query::QueryRequest::PrepareTypeHierarchy(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let item = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        query::prepare_type_hierarchy(repository, revision, file_id, params.offset)
                    }
                    None => None,
                };

                query::QueryResponse::PrepareTypeHierarchy(query::PrepareTypeHierarchyResponse {
                    item,
                })
            }
            query::QueryRequest::TypeHierarchySupertypes(params) => {
                let items = query::supertypes(repository, revision, &params.item);
                query::QueryResponse::TypeHierarchySupertypes(
                    query::TypeHierarchySupertypesResponse { items },
                )
            }
            query::QueryRequest::TypeHierarchySubtypes(params) => {
                let items = query::subtypes(repository, revision, &params.item);
                query::QueryResponse::TypeHierarchySubtypes(query::TypeHierarchySubtypesResponse {
                    items,
                })
            }
            query::QueryRequest::PrepareRename(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let result = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        query::prepare_rename(repository, revision, file_id, params.offset)
                    }
                    None => None,
                };

                query::QueryResponse::PrepareRename(query::PrepareRenameResponse { result })
            }
            query::QueryRequest::Rename(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let result = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        query::rename(
                            repository,
                            revision,
                            file_id,
                            params.offset,
                            &params.new_name,
                        )
                    }
                    None => None,
                };

                query::QueryResponse::Rename(query::RenameResponse { result })
            }
            query::QueryRequest::RenameFiles(params) => {
                let result = query::rename_files(repository, revision, &params.renames);
                query::QueryResponse::RenameFiles(query::RenameFilesResponse { result })
            }
            query::QueryRequest::ExtractFunction(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let result = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        let selection = self.span_for_offsets(file_id, params.start, params.end);
                        query::extract_function(
                            repository,
                            revision,
                            file_id,
                            selection,
                            &params.new_name,
                        )
                    }
                    None => None,
                };

                query::QueryResponse::ExtractFunction(query::ExtractFunctionResponse { result })
            }
            query::QueryRequest::ExtractVariable(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let result = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        let selection = self.span_for_offsets(file_id, params.start, params.end);
                        query::extract_variable(
                            repository,
                            revision,
                            file_id,
                            selection,
                            &params.new_name,
                        )
                    }
                    None => None,
                };

                query::QueryResponse::ExtractVariable(query::ExtractVariableResponse { result })
            }
            query::QueryRequest::Inline(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let result = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        query::inline_symbol(repository, revision, file_id, params.offset)
                    }
                    None => None,
                };

                query::QueryResponse::Inline(query::InlineResponse { result })
            }
            query::QueryRequest::ChangeSignature(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let result = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        query::change_signature(
                            repository,
                            revision,
                            file_id,
                            params.offset,
                            &params.new_parameters,
                            &params.new_arguments,
                        )
                    }
                    None => None,
                };

                query::QueryResponse::ChangeSignature(query::ChangeSignatureResponse { result })
            }
            query::QueryRequest::CodeActions(params) => {
                let file_id =
                    self.resolve_request_file_id(repository, revision, &params.uri, bound_file_id);
                let actions = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_query_artifacts_for_file(repository, revision, file_id)?;
                        let range = self.span_for_offsets(file_id, params.start, params.end);
                        let diagnostics =
                            current_repository_diagnostics_by_file(repository, revision)?
                                .remove(&file_id)
                                .unwrap_or_default();
                        query::code_actions(
                            repository,
                            revision,
                            file_id,
                            range,
                            &diagnostics,
                            &params.context,
                        )
                    }
                    None => Vec::new(),
                };

                query::QueryResponse::CodeActions(query::CodeActionsResponse { actions })
            }
        };

        Ok(response)
    }

    /// Resolve the file id for one query request.
    fn resolve_request_file_id(
        &self,
        repository: &Repository,
        revision: Revision,
        uri: &Uri,
        bound_file_id: Option<FileId>,
    ) -> Option<FileId> {
        bound_file_id.or_else(|| self.resolve_file_id(repository, revision, uri))
    }

    /// Resolve a file id for a query uri.
    fn resolve_file_id(
        &self,
        repository: &Repository,
        revision: Revision,
        uri: &Uri,
    ) -> Option<FileId> {
        // prefer one module lookup by uri
        if let Ok(Some(module_id)) = repository.module_id_for_uri(revision, uri)
            && let Ok(Some(module)) = repository.module(revision, module_id)
        {
            return Some(module.file_id);
        }

        // then try direct file identity for the exact uri form
        if let Some(path) = uri.to_path_buf() {
            let file_id = repository.file_id_for_workspace_path(&path);

            return match repository.file(revision, file_id) {
                Ok(Some(_file)) => Some(file_id),
                Ok(None) => None,
                Err(_error) => None,
            };
        }

        let file_id = FileId::from_logical_str(uri.as_ref());

        match repository.file(revision, file_id) {
            Ok(Some(_file)) => Some(file_id),
            Ok(None) => None,
            Err(_error) => None,
        }
    }

    /// Ensure query artifacts are ready for a file.
    fn ensure_query_artifacts_for_file(
        &self,
        repository: &Repository,
        revision: Revision,
        file_id: FileId,
    ) -> Result<FileId, LanguageServiceError> {
        let Some(module_id) = repository
            .module_id_for_file(revision, file_id)
            .map_err(LanguageServiceError::from)?
        else {
            return Err(LanguageServiceError::QueryNotReady {
                detail: format!("missing module for file {file_id:?}"),
            });
        };

        let profile_id = repository
            .default_profile_id_for_module(revision, module_id)
            .map_err(|error| LanguageServiceError::QueryNotReady {
                detail: format!("missing query profile for module {module_id:?}: {error}"),
            })?;
        let artifact_key = query::default_document_artifact_key(module_id, profile_id);

        if repository
            .dir_checked(revision, module_id, profile_id)
            .is_some()
        {
            return Ok(file_id);
        }

        Err(LanguageServiceError::QueryNotReady {
            detail: format!("missing query artifact {artifact_key:?}"),
        })
    }

    /// Prepare one query request when it targets one document path.
    fn prepare_query_request(
        &self,
        request: &query::QueryRequest,
    ) -> Result<(), LanguageServiceError> {
        let Some(path) = request.path() else {
            return Ok(());
        };

        let workspace = self.workspace_for_document_path(&path)?;
        let module_id = workspace.admit_module_for_path(&path).map_err(|error| {
            LanguageServiceError::QueryNotReady {
                detail: format!("query preparation failed for {}: {error}", path.display()),
            }
        })?;
        let revision = workspace.revision();
        let profile_id = workspace
            .repository()
            .default_profile_id_for_module(revision, module_id)
            .map_err(|error| LanguageServiceError::QueryNotReady {
                detail: format!("query preparation failed for {}: {error}", path.display()),
            })?;
        let Some(artifact_key) = request.default_artifact_key(module_id, profile_id) else {
            return Ok(());
        };

        workspace.provide(&[artifact_key]).map_err(|error| {
            LanguageServiceError::QueryNotReady {
                detail: format!("query preparation failed for {}: {error}", path.display()),
            }
        })?;

        Ok(())
    }

    /// Build a span from offsets for a file.
    fn span_for_offsets(&self, file_id: FileId, start: u32, end: u32) -> Span {
        // normalize offset order before building a span
        let range_start = start.min(end);
        let range_end = start.max(end);
        Span::new(file_id, range_start, range_end)
    }
}

/// Return current diagnostics grouped by file for one workspace session.
fn current_root_diagnostics_by_file(
    workspace: &Session,
    repository: &Repository,
) -> Result<HashMap<FileId, Vec<Diagnostic>>, LanguageServiceError> {
    current_repository_diagnostics_by_file(repository, workspace.revision())
}

/// Return current diagnostics grouped by file for one repository revision.
fn current_repository_diagnostics_by_file(
    repository: &Repository,
    revision: Revision,
) -> Result<HashMap<FileId, Vec<Diagnostic>>, LanguageServiceError> {
    let module_ids = repository
        .module_ids(revision)
        .map_err(LanguageServiceError::from)?;
    let mut diagnostics = DiagnosticCollection::new();

    for module_id in module_ids {
        let Some(module) = repository
            .module(revision, module_id)
            .map_err(LanguageServiceError::from)?
        else {
            continue;
        };

        if module.path.is_none() {
            continue;
        }

        let profile_id = repository
            .default_profile_id_for_module(revision, module_id)
            .map_err(LanguageServiceError::from)?;
        diagnostics
            .merge_from(&repository.module_artifact_diagnostics(revision, module_id, profile_id));
    }
    let mut diagnostics_by_file = HashMap::new();

    for diagnostic in diagnostics.iter() {
        diagnostics_by_file
            .entry(diagnostic.file_id)
            .or_insert_with(Vec::new)
            .push(diagnostic);
    }

    Ok(diagnostics_by_file)
}
