use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use destack_artifact::ArtifactKey;
use destack_query as query;
use destack_session::{Session, SessionError};
use destack_source::{File, FileId, ProfileId, Uri};
use destack_workspace::{Repository, Revision, RevisionPin};

use super::diagnostic::diagnostics_by_file;
use super::{DiagnosticSnapshot, LanguageService, LanguageServiceError};

/// Result of executing one query.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryResult {
    /// The revision used for query execution.
    pub revision: Revision,
    /// The query response payload.
    pub response: query::QueryResponse,
}

impl LanguageService {
    /// Run one callback under a coherent session read lock.
    fn read_session<T, F>(&self, root: &Path, callback: F) -> Result<T, LanguageServiceError>
    where
        F: FnOnce(&Session, &RevisionPin) -> Result<T, LanguageServiceError>,
    {
        // resolve the session and repository
        let session = self.session(root)?;

        // pin one immutable revision for the full query
        let repository = session.repository();
        let revision = session.revision(session.head())?;
        let revision_pin = repository
            .pin(revision)
            .map_err(LanguageServiceError::from)?;

        callback(session.as_ref(), &revision_pin)
    }

    /// Return one tracked file id for a path in a revision.
    fn tracked_file_id(
        repository: &Repository,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<FileId>, LanguageServiceError> {
        let file_id = repository.file_id(path);
        let file = repository.file(revision, file_id)?;

        Ok(file.map(|_| file_id))
    }

    /// Return one tracked file by id.
    fn tracked_file(
        repository: &Repository,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Arc<File>, LanguageServiceError> {
        let file = repository
            .file(revision, file_id)?
            .ok_or(SessionError::FileNotTracked { file_id })?;

        Ok(file)
    }

    /// Run one callback with a coherent file snapshot for a path.
    pub fn read_file<T, F>(&self, path: &Path, callback: F) -> Result<T, LanguageServiceError>
    where
        F: FnOnce(&Repository, FileId, Arc<File>, Revision) -> Result<T, LanguageServiceError>,
    {
        // resolve the root once before entering the read section
        let root = self.root_at(path)?;

        self.read_session(&root, |_session, revision_pin| {
            let repository = revision_pin.repository();
            let revision = revision_pin.revision();

            // require one file for the request
            let file_id = Self::tracked_file_id(repository, revision, path)?.ok_or_else(|| {
                LanguageServiceError::FileMissing {
                    path: path.to_path_buf(),
                }
            })?;
            let file = Self::tracked_file(repository, revision, file_id)?;

            callback(repository, file_id, file, revision)
        })
    }

    /// Return a current diagnostic snapshot for one file path.
    pub fn file_diagnostics(
        &self,
        path: &Path,
    ) -> Result<Option<DiagnosticSnapshot>, LanguageServiceError> {
        let root = self.root_at(path)?;

        self.read_session(&root, |session, revision_pin| {
            let revision = revision_pin.revision();
            let repository = revision_pin.repository();
            let Some(file_id) = Self::tracked_file_id(repository, revision, path)? else {
                return Ok(None);
            };
            let file = Self::tracked_file(repository, revision, file_id)?;
            let diagnostics = diagnostics_by_file(repository, revision)?
                .remove(&file_id)
                .unwrap_or_default();
            let open_file = file.path.as_ref().and_then(|path| self.open_state(path));
            let diagnostic_uri = open_file
                .as_ref()
                .map(|file| file.uri.clone())
                .or_else(|| file.path.as_ref().map(Uri::from_file_path))
                .unwrap_or_else(|| file.uri.clone());
            let diagnostic_version = if let Some(path) = file.path.as_ref() {
                self.open_file_version_in_revision(
                    session.repository().as_ref(),
                    revision,
                    file_id,
                    path,
                )?
            } else {
                None
            };

            Ok(Some(DiagnosticSnapshot {
                file,
                diagnostic_uri,
                diagnostic_version,
                diagnostics,
            }))
        })
    }

    /// Return current diagnostic snapshots for every open root.
    pub fn diagnostics(&self) -> Result<Vec<DiagnosticSnapshot>, LanguageServiceError> {
        let mut roots: Vec<_> = self.roots.iter().map(|entry| entry.key().clone()).collect();
        roots.sort();

        let mut snapshots = Vec::new();
        for root in roots {
            let root_snapshots = self.root_diagnostics(&root)?;

            snapshots.extend(root_snapshots);
        }

        Ok(snapshots)
    }

    /// Return current diagnostic snapshots for one root.
    pub fn root_diagnostics(
        &self,
        root: &Path,
    ) -> Result<Vec<DiagnosticSnapshot>, LanguageServiceError> {
        self.read_session(root, |session, revision_pin| {
            let mut diagnostics_by_file =
                diagnostics_by_file(revision_pin.repository(), revision_pin.revision())?;

            // open files
            let mut open_files = HashMap::new();
            for (path, file) in self.open_files_under(root) {
                let Some(file_id) = Self::tracked_file_id(
                    revision_pin.repository(),
                    revision_pin.revision(),
                    &path,
                )?
                else {
                    continue;
                };
                let version = self.open_file_version_in_revision(
                    session.repository().as_ref(),
                    revision_pin.revision(),
                    file_id,
                    &path,
                )?;

                diagnostics_by_file.entry(file_id).or_insert(Vec::new());
                open_files.insert(file_id, (file.uri, version));
            }

            let mut snapshots = Vec::new();
            for (file_id, diagnostics) in diagnostics_by_file {
                let Ok(file) =
                    Self::tracked_file(revision_pin.repository(), revision_pin.revision(), file_id)
                else {
                    continue;
                };
                let open_file = open_files.get(&file_id);
                let diagnostic_uri = open_file
                    .map(|(uri, _)| uri.clone())
                    .or_else(|| file.path.as_ref().map(Uri::from_file_path))
                    .unwrap_or_else(|| file.uri.clone());
                let diagnostic_version = open_file.and_then(|(_, version)| *version);

                snapshots.push(DiagnosticSnapshot {
                    file,
                    diagnostic_uri,
                    diagnostic_version,
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
    pub fn read_file_query<F>(
        &self,
        path: &Path,
        profile_id: ProfileId,
        build_request: F,
    ) -> Result<Option<(FileId, Arc<File>, QueryResult)>, LanguageServiceError>
    where
        F: FnOnce(
            query::QueryModule,
            FileId,
            &File,
        ) -> Result<Option<query::QueryRequest>, LanguageServiceError>,
    {
        let root = self.root_at(path)?;

        self.read_session(&root, |session, revision_pin| {
            let repository = revision_pin.repository();
            let revision = revision_pin.revision();

            // require one file for the request
            let file_id = Self::tracked_file_id(repository, revision, path)?.ok_or_else(|| {
                LanguageServiceError::FileMissing {
                    path: path.to_path_buf(),
                }
            })?;
            let file = Self::tracked_file(repository, revision, file_id)?;

            // build the request from exact revision facts
            let Some(module_id) = repository
                .module_id_for_file(revision, file_id)
                .map_err(LanguageServiceError::from)?
            else {
                return Err(LanguageServiceError::Internal {
                    detail: format!("missing module for file {file_id:?}"),
                });
            };
            let module = query::QueryModule {
                module_id,
                profile_id,
            };
            let Some(request) = build_request(module, file_id, &file)? else {
                return Ok(None);
            };
            let response =
                self.execute_query_request(session, repository, revision, Some(file_id), request)?;

            Ok(Some((file_id, file, QueryResult { revision, response })))
        })
    }

    /// Run one read query for the root that owns a path.
    pub fn read_query(
        &self,
        path: &Path,
        request: query::QueryRequest,
    ) -> Result<QueryResult, LanguageServiceError> {
        let root = self.root_at(path)?;

        self.read_query_for_root(&root, request)
    }

    /// Run one read query for a root.
    pub fn read_query_for_root(
        &self,
        root: &Path,
        request: query::QueryRequest,
    ) -> Result<QueryResult, LanguageServiceError> {
        self.read_session(root, |session, revision_pin| {
            // dispatch pure read query execution
            let repository = revision_pin.repository();
            let revision = revision_pin.revision();
            let response =
                self.execute_query_request(session, repository, revision, None, request)?;

            Ok(QueryResult { revision, response })
        })
    }

    /// Run one write query for the root that owns a path.
    pub fn write_query(
        &self,
        path: &Path,
        expected_revision: Revision,
        request: query::QueryRequest,
    ) -> Result<QueryResult, LanguageServiceError> {
        let root = self.root_at(path)?;

        self.write_query_for_root(&root, expected_revision, request)
    }

    /// Run one write query for a root.
    pub fn write_query_for_root(
        &self,
        root: &Path,
        expected_revision: Revision,
        request: query::QueryRequest,
    ) -> Result<QueryResult, LanguageServiceError> {
        // resolve the owning session and current revision
        let session = self.session(root)?;

        let current_revision = session.revision(session.head())?;

        // require a matching revision precondition for write requests
        if expected_revision != current_revision {
            return Err(LanguageServiceError::StaleRevision {
                expected: expected_revision,
                current: current_revision,
            });
        }

        // root the checked revision for the full request
        let repository = session.repository();
        let revision_pin = repository
            .pin(current_revision)
            .map_err(LanguageServiceError::from)?;

        // dispatch mutating query execution
        let revision = revision_pin.revision();
        let response = self.execute_query_request(
            session.as_ref(),
            revision_pin.repository(),
            revision,
            None,
            request,
        )?;

        Ok(QueryResult { revision, response })
    }

    /// Build a query response for a request payload.
    fn execute_query_request(
        &self,
        session: &Session,
        repository: &Repository,
        revision: Revision,
        bound_file_id: Option<FileId>,
        request: query::QueryRequest,
    ) -> Result<query::QueryResponse, LanguageServiceError> {
        // dispatch by query request variant
        let response = match request {
            query::QueryRequest::Completion(params) => {
                self.assert_bound_file(bound_file_id, params.position.file_id)?;
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let workspace = self.workspace_query_context(
                    session,
                    repository,
                    revision,
                    &[params.position.module.profile_id],
                )?;
                let mut items = query::completions(
                    &context,
                    &workspace,
                    params.position.offset,
                    params.trigger,
                );

                if !params.include_imports {
                    items.retain(|item| item.additional_text_edits.is_empty());
                }

                query::QueryResponse::Completion(query::CompletionResponse {
                    items,
                    is_incomplete: false,
                })
            }
            query::QueryRequest::Hover(params) => {
                self.assert_bound_file(bound_file_id, params.position.file_id)?;
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let hover = query::hover(&context, params.position.offset);

                query::QueryResponse::Hover(query::HoverResponse { hover })
            }
            query::QueryRequest::SignatureHelp(params) => {
                self.assert_bound_file(bound_file_id, params.position.file_id)?;
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let help = query::signature_help(&context, params.position.offset);

                query::QueryResponse::SignatureHelp(query::SignatureHelpResponse { help })
            }
            query::QueryRequest::InlayHints(params) => {
                self.assert_bound_file(bound_file_id, params.range.span.file)?;
                let context =
                    self.module_query_context(session, repository, revision, params.range.module)?;
                let hints = query::inlay_hints(&context, params.range.span);

                query::QueryResponse::InlayHints(query::InlayHintsResponse { hints })
            }
            query::QueryRequest::CodeLenses(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.module)?;
                let workspace = self.workspace_query_context(
                    session,
                    repository,
                    revision,
                    &[params.module.profile_id],
                )?;
                let lenses = query::code_lenses(&context, &workspace);

                query::QueryResponse::CodeLenses(query::CodeLensesResponse { lenses })
            }
            query::QueryRequest::ResolveCodeLens(params) => {
                let lens = query::resolve_code_lens(&params.lens);
                query::QueryResponse::ResolveCodeLens(query::ResolveCodeLensResponse { lens })
            }
            query::QueryRequest::FoldingRanges(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.module)?;
                let ranges = query::folding_ranges(&context);

                query::QueryResponse::FoldingRanges(query::FoldingRangesResponse { ranges })
            }
            query::QueryRequest::SemanticTokens(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.module)?;
                let tokens = query::semantic_tokens(&context);

                query::QueryResponse::SemanticTokens(query::SemanticTokensResponse { tokens })
            }
            query::QueryRequest::SemanticTokensRange(params) => {
                self.assert_bound_file(bound_file_id, params.range.span.file)?;
                let context =
                    self.module_query_context(session, repository, revision, params.range.module)?;
                let tokens = query::semantic_tokens_range(&context, params.range.span);

                query::QueryResponse::SemanticTokensRange(query::SemanticTokensResponse { tokens })
            }
            query::QueryRequest::DocumentSymbols(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.module)?;
                let symbols = query::document_symbols(&context);

                query::QueryResponse::DocumentSymbols(query::DocumentSymbolsResponse { symbols })
            }
            query::QueryRequest::WorkspaceSymbols(params) => {
                let context = self.workspace_query_context(
                    session,
                    repository,
                    revision,
                    &params.profile_ids,
                )?;
                let symbols =
                    query::workspace_symbols(&context, &params.query, params.max_results as usize);
                query::QueryResponse::WorkspaceSymbols(query::WorkspaceSymbolsResponse { symbols })
            }
            query::QueryRequest::DocumentLinks(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.module)?;
                let links = query::document_links(&context);

                query::QueryResponse::DocumentLinks(query::DocumentLinksResponse { links })
            }
            query::QueryRequest::ResolveDocumentLink(params) => {
                let link = query::resolve_document_link(&params.link);
                query::QueryResponse::ResolveDocumentLink(query::ResolveDocumentLinkResponse {
                    link,
                })
            }
            query::QueryRequest::DocumentHighlight(params) => {
                self.assert_bound_file(bound_file_id, params.position.file_id)?;
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let highlights = query::document_highlights(&context, params.position.offset);

                query::QueryResponse::DocumentHighlight(query::DocumentHighlightResponse {
                    highlights,
                })
            }
            query::QueryRequest::SelectionRanges(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.module)?;
                let ranges = query::selection_ranges(&context, &params.offsets);

                query::QueryResponse::SelectionRanges(query::SelectionRangesResponse { ranges })
            }
            query::QueryRequest::GotoDefinition(params) => {
                self.assert_bound_file(bound_file_id, params.position.file_id)?;
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let targets = query::goto_definition(&context, params.position.offset);

                query::QueryResponse::GotoDefinition(query::GotoDefinitionResponse { targets })
            }
            query::QueryRequest::GotoDeclaration(params) => {
                self.assert_bound_file(bound_file_id, params.position.file_id)?;
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let targets = query::goto_declaration(&context, params.position.offset);

                query::QueryResponse::GotoDeclaration(query::GotoDeclarationResponse { targets })
            }
            query::QueryRequest::GotoTypeDefinition(params) => {
                self.assert_bound_file(bound_file_id, params.position.file_id)?;
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let targets = query::goto_type_definition(&context, params.position.offset);

                query::QueryResponse::GotoTypeDefinition(query::GotoTypeDefinitionResponse {
                    targets,
                })
            }
            query::QueryRequest::GotoImplementation(params) => {
                self.assert_bound_file(bound_file_id, params.position.file_id)?;
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let workspace = self.workspace_query_context(
                    session,
                    repository,
                    revision,
                    &[params.position.module.profile_id],
                )?;
                let targets =
                    query::goto_implementation(&context, &workspace, params.position.offset);

                query::QueryResponse::GotoImplementation(query::GotoImplementationResponse {
                    targets,
                })
            }
            query::QueryRequest::FindReferences(params) => {
                self.assert_bound_file(bound_file_id, params.position.file_id)?;
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let workspace = self.workspace_query_context(
                    session,
                    repository,
                    revision,
                    &[params.position.module.profile_id],
                )?;
                let references = query::find_references(
                    &context,
                    &workspace,
                    params.position.offset,
                    params.include_declaration,
                );

                query::QueryResponse::FindReferences(query::FindReferencesResponse { references })
            }
            query::QueryRequest::CallHierarchyItem(params) => {
                self.assert_bound_file(bound_file_id, params.position.file_id)?;
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let item = query::call_hierarchy_item(&context, params.position.offset);

                query::QueryResponse::CallHierarchyItem(query::CallHierarchyItemResponse { item })
            }
            query::QueryRequest::CallHierarchyIncoming(params) => {
                let context = self.workspace_query_context(
                    session,
                    repository,
                    revision,
                    &[params.item.target.module.profile_id],
                )?;
                let calls = query::incoming_calls(&context, &params.item);
                query::QueryResponse::CallHierarchyIncoming(query::CallHierarchyIncomingResponse {
                    calls,
                })
            }
            query::QueryRequest::CallHierarchyOutgoing(params) => {
                let context = self.workspace_query_context(
                    session,
                    repository,
                    revision,
                    &[params.item.target.module.profile_id],
                )?;
                let calls = query::outgoing_calls(&context, &params.item);
                query::QueryResponse::CallHierarchyOutgoing(query::CallHierarchyOutgoingResponse {
                    calls,
                })
            }
            query::QueryRequest::TypeHierarchyItem(params) => {
                self.assert_bound_file(bound_file_id, params.position.file_id)?;
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let item = query::type_hierarchy_item(&context, params.position.offset);

                query::QueryResponse::TypeHierarchyItem(query::TypeHierarchyItemResponse { item })
            }
            query::QueryRequest::TypeHierarchySupertypes(params) => {
                let context = self.workspace_query_context(
                    session,
                    repository,
                    revision,
                    &[params.item.target.module.profile_id],
                )?;
                let items = query::supertypes(&context, &params.item);
                query::QueryResponse::TypeHierarchySupertypes(
                    query::TypeHierarchySupertypesResponse { items },
                )
            }
            query::QueryRequest::TypeHierarchySubtypes(params) => {
                let context = self.workspace_query_context(
                    session,
                    repository,
                    revision,
                    &[params.item.target.module.profile_id],
                )?;
                let items = query::subtypes(&context, &params.item);
                query::QueryResponse::TypeHierarchySubtypes(query::TypeHierarchySubtypesResponse {
                    items,
                })
            }
            query::QueryRequest::RenameTarget(params) => {
                self.assert_bound_file(bound_file_id, params.position.file_id)?;
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let result = query::rename_target(&context, params.position.offset);

                query::QueryResponse::RenameTarget(query::RenameTargetResponse { result })
            }
            query::QueryRequest::Rename(params) => {
                self.assert_bound_file(bound_file_id, params.position.file_id)?;
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let workspace = self.workspace_query_context(
                    session,
                    repository,
                    revision,
                    &[params.position.module.profile_id],
                )?;
                let edit = query::rename(
                    &context,
                    &workspace,
                    params.position.offset,
                    &params.new_name,
                );

                query::QueryResponse::Rename(query::RenameResponse { edit })
            }
            query::QueryRequest::RenameFiles(params) => {
                let context = self.workspace_query_context(
                    session,
                    repository,
                    revision,
                    &params.profile_ids,
                )?;
                let edit = query::rename_files(&context, &params.renames);
                query::QueryResponse::RenameFiles(query::RenameFilesResponse { edit })
            }
            query::QueryRequest::ExtractFunction(params) => {
                self.assert_bound_file(bound_file_id, params.range.span.file)?;
                let context =
                    self.module_query_context(session, repository, revision, params.range.module)?;
                let edit = query::extract_function(&context, params.range.span, &params.new_name);

                query::QueryResponse::ExtractFunction(query::ExtractFunctionResponse { edit })
            }
            query::QueryRequest::ExtractVariable(params) => {
                self.assert_bound_file(bound_file_id, params.range.span.file)?;
                let context =
                    self.module_query_context(session, repository, revision, params.range.module)?;
                let edit = query::extract_variable(&context, params.range.span, &params.new_name);

                query::QueryResponse::ExtractVariable(query::ExtractVariableResponse { edit })
            }
            query::QueryRequest::Inline(params) => {
                self.assert_bound_file(bound_file_id, params.position.file_id)?;
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let workspace = self.workspace_query_context(
                    session,
                    repository,
                    revision,
                    &[params.position.module.profile_id],
                )?;
                let edit = query::inline_symbol(&context, &workspace, params.position.offset);

                query::QueryResponse::Inline(query::InlineResponse { edit })
            }
            query::QueryRequest::ChangeSignature(params) => {
                self.assert_bound_file(bound_file_id, params.position.file_id)?;
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let workspace = self.workspace_query_context(
                    session,
                    repository,
                    revision,
                    &[params.position.module.profile_id],
                )?;
                let edit = query::change_signature(
                    &context,
                    &workspace,
                    params.position.offset,
                    &params.new_parameters,
                    &params.new_arguments,
                );

                query::QueryResponse::ChangeSignature(query::ChangeSignatureResponse { edit })
            }
            query::QueryRequest::CodeActions(params) => {
                self.assert_bound_file(bound_file_id, params.range.span.file)?;
                let context =
                    self.module_query_context(session, repository, revision, params.range.module)?;
                let workspace = self.workspace_query_context(
                    session,
                    repository,
                    revision,
                    &[params.range.module.profile_id],
                )?;
                let diagnostics = diagnostics_by_file(repository, revision)?
                    .remove(&params.range.span.file)
                    .unwrap_or_default();
                let actions = query::code_actions(
                    &context,
                    &workspace,
                    params.range.span,
                    &diagnostics,
                    &params.context,
                );

                query::QueryResponse::CodeActions(query::CodeActionsResponse { actions })
            }
            query::QueryRequest::Annotations(params) => {
                let profile_ids = match &params.scope {
                    query::AnnotationScope::Module(module) => vec![module.profile_id],
                    query::AnnotationScope::Workspace { profile_ids } => profile_ids.clone(),
                };
                let context =
                    self.workspace_query_context(session, repository, revision, &profile_ids)?;
                let annotations =
                    query::annotations(&context, &params.scope, params.name.as_deref());

                query::QueryResponse::Annotations(query::AnnotationsResponse { annotations })
            }
        };

        Ok(response)
    }

    /// Return the module query context for one query module.
    fn module_query_context<'a>(
        &self,
        session: &Session,
        repository: &'a Repository,
        revision: Revision,
        module: query::QueryModule,
    ) -> Result<query::ModuleQueryContext<'a>, LanguageServiceError> {
        let key = ArtifactKey::dir_checked(module.module_id, module.profile_id);
        let checked_version =
            session
                .require(revision, key)
                .map_err(|error| LanguageServiceError::Internal {
                    detail: format!(
                        "failed to require query DIR for module {}: {error}",
                        module.module_id
                    ),
                })?;
        let key = ArtifactKey::global_environment(module.profile_id);
        let global_environment_version =
            session
                .require(revision, key)
                .map_err(|error| LanguageServiceError::Internal {
                    detail: format!(
                        "failed to require query global environment for profile {:?}: {error}",
                        module.profile_id
                    ),
                })?;

        query::module_query_context_from_checked(
            repository,
            revision,
            module.module_id,
            module.profile_id,
            checked_version,
            global_environment_version,
        )
        .ok_or_else(|| LanguageServiceError::Internal {
            detail: format!("missing query artifacts for module {}", module.module_id),
        })
    }

    /// Return a workspace query context for explicit profiles.
    fn workspace_query_context<'a>(
        &self,
        session: &Session,
        repository: &'a Repository,
        revision: Revision,
        profile_ids: &[ProfileId],
    ) -> Result<query::WorkspaceQueryContext<'a>, LanguageServiceError> {
        let mut indexes = Vec::with_capacity(profile_ids.len());

        for profile_id in profile_ids {
            let key = ArtifactKey::workspace_query_index(*profile_id);
            let version = session.require(revision, key).map_err(|error| {
                LanguageServiceError::Internal {
                    detail: format!(
                        "failed to require workspace query index for profile {profile_id:?}: {error}"
                    ),
                }
            })?;

            let index = repository
                .artifact_store()
                .workspace_query_index(&version)
                .ok_or_else(|| LanguageServiceError::Internal {
                    detail: format!(
                        "workspace query index payload is missing for profile {profile_id:?}"
                    ),
                })?;

            indexes.push((*profile_id, index));
        }

        query::workspace_query_context(repository, revision, indexes).ok_or_else(|| {
            LanguageServiceError::Internal {
                detail: "workspace query index references missing module indexes".to_string(),
            }
        })
    }

    /// Assert that a file-backed request stayed on its bound file.
    fn assert_bound_file(
        &self,
        bound_file_id: Option<FileId>,
        requested_file_id: FileId,
    ) -> Result<(), LanguageServiceError> {
        if bound_file_id.is_some_and(|file_id| file_id != requested_file_id) {
            return Err(LanguageServiceError::Internal {
                detail: format!("query file changed from bound file {requested_file_id:?}"),
            });
        }

        Ok(())
    }
}
