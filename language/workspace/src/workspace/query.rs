use std::path::Path;

use destack_artifact::ArtifactKey;
use destack_query as query;
use destack_repository::{Repository, Revision};
use destack_session::Session;
use destack_source::ProfileId;

use crate::diagnostic::{Error, diagnostics_by_file};

use super::{Snapshot, Workspace};

/// Result of executing one query.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryResult {
    /// The revision used for query execution.
    pub revision: Revision,
    /// The query response payload.
    pub response: query::QueryResponse,
}

/// Revision selection policy for one query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevisionPolicy {
    /// Use the ref's latest revision when execution starts.
    Latest,
    /// Use one exact immutable revision.
    Exact(Revision),
    /// Use one revision only if the ref still points at it.
    Current(Revision),
}

impl Workspace {
    /// Run one query for the root that owns a path.
    pub fn query(
        &self,
        path: &Path,
        request: query::QueryRequest,
        revision: RevisionPolicy,
    ) -> Result<QueryResult, Error> {
        let root = self.root_at(path)?;

        self.query_root(&root, request, revision)
    }

    /// Run one query for a root.
    pub fn query_root(
        &self,
        root: &Path,
        request: query::QueryRequest,
        revision: RevisionPolicy,
    ) -> Result<QueryResult, Error> {
        let session = self.query_snapshot(root, revision)?;
        let revision = session.revision();

        // dispatch query execution
        let response =
            self.execute_query_request(session.session(), session.repository(), revision, request)?;

        Ok(QueryResult { revision, response })
    }

    /// Return the snapshot selected by one query revision policy.
    fn query_snapshot(&self, root: &Path, revision: RevisionPolicy) -> Result<Snapshot, Error> {
        match revision {
            // latest ref state
            RevisionPolicy::Latest => self.snapshot(root),

            // exact immutable revision state
            RevisionPolicy::Exact(revision) => {
                let session = self.session(root)?;
                let repository = session.repository();
                let revision = repository.pin(revision)?;

                Ok(Snapshot::new(session, revision))
            }

            // current ref state with caller precondition
            RevisionPolicy::Current(expected) => {
                let session = self.snapshot(root)?;
                let current = session.revision();
                if current != expected {
                    return Err(Error::StaleRevision { expected, current });
                }

                Ok(session)
            }
        }
    }

    /// Build a query response for a request payload.
    fn execute_query_request(
        &self,
        session: &Session,
        repository: &Repository,
        revision: Revision,
        request: query::QueryRequest,
    ) -> Result<query::QueryResponse, Error> {
        // dispatch by query request variant
        let response = match request {
            query::QueryRequest::Completion(params) => {
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
                let mut items =
                    context.completions(&workspace, params.position.offset, params.trigger);

                if !params.include_imports {
                    items.retain(|item| item.additional_text_edits.is_empty());
                }

                query::QueryResponse::Completion(query::CompletionResponse {
                    items,
                    is_incomplete: false,
                })
            }
            query::QueryRequest::Hover(params) => {
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let hover = context.hover(params.position.offset);

                query::QueryResponse::Hover(query::HoverResponse { hover })
            }
            query::QueryRequest::SignatureHelp(params) => {
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let help = context.signature_help(params.position.offset);

                query::QueryResponse::SignatureHelp(query::SignatureHelpResponse { help })
            }
            query::QueryRequest::InlayHints(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.range.module)?;
                let hints = context.inlay_hints(params.range.span);

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
                let lenses = context.code_lenses(&workspace);

                query::QueryResponse::CodeLenses(query::CodeLensesResponse { lenses })
            }
            query::QueryRequest::ResolveCodeLens(params) => {
                let lens = query::resolve_code_lens(&params.lens);
                query::QueryResponse::ResolveCodeLens(query::ResolveCodeLensResponse { lens })
            }
            query::QueryRequest::FoldingRanges(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.module)?;
                let ranges = context.folding_ranges();

                query::QueryResponse::FoldingRanges(query::FoldingRangesResponse { ranges })
            }
            query::QueryRequest::SemanticTokens(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.module)?;
                let tokens = context.semantic_tokens();

                query::QueryResponse::SemanticTokens(query::SemanticTokensResponse { tokens })
            }
            query::QueryRequest::SemanticTokensRange(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.range.module)?;
                let tokens = context.semantic_tokens_range(params.range.span);

                query::QueryResponse::SemanticTokensRange(query::SemanticTokensResponse { tokens })
            }
            query::QueryRequest::DocumentSymbols(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.module)?;
                let symbols = context.document_symbols();

                query::QueryResponse::DocumentSymbols(query::DocumentSymbolsResponse { symbols })
            }
            query::QueryRequest::WorkspaceSymbols(params) => {
                let context = self.workspace_query_context(
                    session,
                    repository,
                    revision,
                    &params.profile_ids,
                )?;
                let symbols = context.workspace_symbols(&params.query, params.max_results as usize);
                query::QueryResponse::WorkspaceSymbols(query::WorkspaceSymbolsResponse { symbols })
            }
            query::QueryRequest::DocumentLinks(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.module)?;
                let links = context.document_links();

                query::QueryResponse::DocumentLinks(query::DocumentLinksResponse { links })
            }
            query::QueryRequest::DocumentHighlight(params) => {
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let highlights = context.document_highlights(params.position.offset);

                query::QueryResponse::DocumentHighlight(query::DocumentHighlightResponse {
                    highlights,
                })
            }
            query::QueryRequest::SelectionRanges(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.module)?;
                let ranges = context.selection_ranges(&params.offsets);

                query::QueryResponse::SelectionRanges(query::SelectionRangesResponse { ranges })
            }
            query::QueryRequest::GotoDefinition(params) => {
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let targets = context.goto_definition(params.position.offset);

                query::QueryResponse::GotoDefinition(query::GotoDefinitionResponse { targets })
            }
            query::QueryRequest::GotoDeclaration(params) => {
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let targets = context.goto_declaration(params.position.offset);

                query::QueryResponse::GotoDeclaration(query::GotoDeclarationResponse { targets })
            }
            query::QueryRequest::GotoTypeDefinition(params) => {
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let targets = context.goto_type_definition(params.position.offset);

                query::QueryResponse::GotoTypeDefinition(query::GotoTypeDefinitionResponse {
                    targets,
                })
            }
            query::QueryRequest::GotoImplementation(params) => {
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
                let targets = context.goto_implementation(&workspace, params.position.offset);

                query::QueryResponse::GotoImplementation(query::GotoImplementationResponse {
                    targets,
                })
            }
            query::QueryRequest::FindReferences(params) => {
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
                let references = context.find_references(
                    &workspace,
                    params.position.offset,
                    params.include_declaration,
                );

                query::QueryResponse::FindReferences(query::FindReferencesResponse { references })
            }
            query::QueryRequest::CallHierarchyItem(params) => {
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let item = context.call_hierarchy_item(params.position.offset);

                query::QueryResponse::CallHierarchyItem(query::CallHierarchyItemResponse { item })
            }
            query::QueryRequest::CallHierarchyIncoming(params) => {
                let context = self.workspace_query_context(
                    session,
                    repository,
                    revision,
                    &[params.item.target.module.profile_id],
                )?;
                let calls = context.incoming_calls(&params.item);
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
                let calls = context.outgoing_calls(&params.item);
                query::QueryResponse::CallHierarchyOutgoing(query::CallHierarchyOutgoingResponse {
                    calls,
                })
            }
            query::QueryRequest::TypeHierarchyItem(params) => {
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let item = context.type_hierarchy_item(params.position.offset);

                query::QueryResponse::TypeHierarchyItem(query::TypeHierarchyItemResponse { item })
            }
            query::QueryRequest::TypeHierarchySupertypes(params) => {
                let context = self.workspace_query_context(
                    session,
                    repository,
                    revision,
                    &[params.item.target.module.profile_id],
                )?;
                let items = context.supertypes(&params.item);
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
                let items = context.subtypes(&params.item);
                query::QueryResponse::TypeHierarchySubtypes(query::TypeHierarchySubtypesResponse {
                    items,
                })
            }
            query::QueryRequest::RenameTarget(params) => {
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let result = context.rename_target(params.position.offset);

                query::QueryResponse::RenameTarget(query::RenameTargetResponse { result })
            }
            query::QueryRequest::Rename(params) => {
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
                let edit = context.rename(&workspace, params.position.offset, &params.new_name);

                query::QueryResponse::Rename(query::RenameResponse { edit })
            }
            query::QueryRequest::RenameFiles(params) => {
                let context = self.workspace_query_context(
                    session,
                    repository,
                    revision,
                    &params.profile_ids,
                )?;
                let edit = context.rename_files(&params.renames);
                query::QueryResponse::RenameFiles(query::RenameFilesResponse { edit })
            }
            query::QueryRequest::ExtractFunction(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.range.module)?;
                let edit = context.extract_function(params.range.span, &params.new_name);

                query::QueryResponse::ExtractFunction(query::ExtractFunctionResponse { edit })
            }
            query::QueryRequest::ExtractVariable(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.range.module)?;
                let edit = context.extract_variable(params.range.span, &params.new_name);

                query::QueryResponse::ExtractVariable(query::ExtractVariableResponse { edit })
            }
            query::QueryRequest::Inline(params) => {
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
                let edit = context.inline_symbol(&workspace, params.position.offset);

                query::QueryResponse::Inline(query::InlineResponse { edit })
            }
            query::QueryRequest::ChangeSignature(params) => {
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
                let edit = context.change_signature(
                    &workspace,
                    params.position.offset,
                    &params.new_parameters,
                    &params.new_arguments,
                );

                query::QueryResponse::ChangeSignature(query::ChangeSignatureResponse { edit })
            }
            query::QueryRequest::CodeActions(params) => {
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
                let actions = context.code_actions(
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
                let annotations = context.annotations(&params.scope, params.name.as_deref());

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
    ) -> Result<query::ModuleQueryContext<'a>, Error> {
        let key = ArtifactKey::dir_checked(module.module_id, module.profile_id);
        let checked_version = session
            .require(revision, key)
            .map_err(|error| Error::Internal {
                detail: format!(
                    "failed to require query DIR for module {}: {error}",
                    module.module_id
                ),
            })?;
        let key = ArtifactKey::global_environment(module.profile_id);
        let global_environment_version =
            session
                .require(revision, key)
                .map_err(|error| Error::Internal {
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
        .ok_or_else(|| Error::Internal {
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
    ) -> Result<query::WorkspaceQueryContext<'a>, Error> {
        let mut indexes = Vec::with_capacity(profile_ids.len());

        for profile_id in profile_ids {
            let key = ArtifactKey::workspace_query_index(*profile_id);
            let version = session.require(revision, key).map_err(|error| {
                Error::Internal {
                    detail: format!(
                        "failed to require workspace query index for profile {profile_id:?}: {error}"
                    ),
                }
            })?;

            let index = repository
                .artifact_store()
                .workspace_query_index(&version)
                .ok_or_else(|| Error::Internal {
                    detail: format!(
                        "workspace query index payload is missing for profile {profile_id:?}"
                    ),
                })?;

            indexes.push((*profile_id, index));
        }

        query::workspace_query_context(repository, revision, indexes).ok_or_else(|| {
            Error::Internal {
                detail: "workspace query index references missing module indexes".to_string(),
            }
        })
    }
}
