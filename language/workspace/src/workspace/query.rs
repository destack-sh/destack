use std::path::Path;

use destack_artifact::ArtifactKey;
use destack_repository::{Repository, Revision};
use destack_serde::Reflect;
use destack_session::Session;
use destack_source::ProfileId;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{Error, diagnostics_by_file};

use super::{LocalWorkspace, Snapshot};

/// Result of executing one query.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryResult {
    /// The revision used for query execution.
    pub revision: Revision,
    /// The query response payload.
    pub response: destack_query::QueryResponse,
}

/// Request to run one semantic query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct QueryRequest {
    /// Expected revision for this query.
    pub expected_revision: Option<Revision>,
    /// Query request payload.
    pub request: destack_query::QueryRequest,
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

impl LocalWorkspace {
    /// Run one query for the root that owns a path.
    pub fn query(
        &self,
        path: &Path,
        request: destack_query::QueryRequest,
        revision: RevisionPolicy,
    ) -> Result<QueryResult, Error> {
        let root = self.root_at(path)?;

        self.query_root(&root, request, revision)
    }

    /// Run one query for a root.
    pub fn query_root(
        &self,
        root: &Path,
        request: destack_query::QueryRequest,
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
        request: destack_query::QueryRequest,
    ) -> Result<destack_query::QueryResponse, Error> {
        // dispatch by query request variant
        let response = match request {
            destack_query::QueryRequest::Completion(params) => {
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let program = self.program_query_context(
                    session,
                    repository,
                    revision,
                    &[params.position.module.profile_id],
                )?;
                let mut items =
                    context.completions(&program, params.position.offset, params.trigger);

                if !params.include_imports {
                    items.retain(|item| item.additional_text_edits.is_empty());
                }

                destack_query::QueryResponse::Completion(destack_query::CompletionResponse {
                    items,
                    is_incomplete: false,
                })
            }
            destack_query::QueryRequest::Hover(params) => {
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let hover = context.hover(params.position.offset);

                destack_query::QueryResponse::Hover(destack_query::HoverResponse { hover })
            }
            destack_query::QueryRequest::SignatureHelp(params) => {
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let help = context.signature_help(params.position.offset);

                destack_query::QueryResponse::SignatureHelp(destack_query::SignatureHelpResponse {
                    help,
                })
            }
            destack_query::QueryRequest::InlayHints(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.range.module)?;
                let hints = context.inlay_hints(params.range.span);

                destack_query::QueryResponse::InlayHints(destack_query::InlayHintsResponse {
                    hints,
                })
            }
            destack_query::QueryRequest::CodeLenses(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.module)?;
                let program = self.program_query_context(
                    session,
                    repository,
                    revision,
                    &[params.module.profile_id],
                )?;
                let lenses = context.code_lenses(&program);

                destack_query::QueryResponse::CodeLenses(destack_query::CodeLensesResponse {
                    lenses,
                })
            }
            destack_query::QueryRequest::ResolveCodeLens(params) => {
                let lens = destack_query::resolve_code_lens(&params.lens);
                destack_query::QueryResponse::ResolveCodeLens(
                    destack_query::ResolveCodeLensResponse { lens },
                )
            }
            destack_query::QueryRequest::FoldingRanges(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.module)?;
                let ranges = context.folding_ranges();

                destack_query::QueryResponse::FoldingRanges(destack_query::FoldingRangesResponse {
                    ranges,
                })
            }
            destack_query::QueryRequest::SemanticTokens(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.module)?;
                let tokens = context.semantic_tokens();

                destack_query::QueryResponse::SemanticTokens(
                    destack_query::SemanticTokensResponse { tokens },
                )
            }
            destack_query::QueryRequest::SemanticTokensRange(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.range.module)?;
                let tokens = context.semantic_tokens_range(params.range.span);

                destack_query::QueryResponse::SemanticTokensRange(
                    destack_query::SemanticTokensResponse { tokens },
                )
            }
            destack_query::QueryRequest::Outline(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.module)?;
                let symbols = context.outline();

                destack_query::QueryResponse::Outline(destack_query::OutlineResponse { symbols })
            }
            destack_query::QueryRequest::SymbolSearch(params) => {
                let context =
                    self.program_query_context(session, repository, revision, &params.profile_ids)?;
                let symbols = context.search_symbols(&params.query, params.max_results as usize);
                destack_query::QueryResponse::SymbolSearch(destack_query::SymbolSearchResponse {
                    symbols,
                })
            }
            destack_query::QueryRequest::Links(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.module)?;
                let links = context.links();

                destack_query::QueryResponse::Links(destack_query::LinksResponse { links })
            }
            destack_query::QueryRequest::Highlight(params) => {
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let highlights = context.highlights(params.position.offset);

                destack_query::QueryResponse::Highlight(destack_query::HighlightResponse {
                    highlights,
                })
            }
            destack_query::QueryRequest::SelectionRanges(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.module)?;
                let ranges = context.selection_ranges(&params.offsets);

                destack_query::QueryResponse::SelectionRanges(
                    destack_query::SelectionRangesResponse { ranges },
                )
            }
            destack_query::QueryRequest::GotoDefinition(params) => {
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let targets = context.goto_definition(params.position.offset);

                destack_query::QueryResponse::GotoDefinition(
                    destack_query::GotoDefinitionResponse { targets },
                )
            }
            destack_query::QueryRequest::GotoDeclaration(params) => {
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let targets = context.goto_declaration(params.position.offset);

                destack_query::QueryResponse::GotoDeclaration(
                    destack_query::GotoDeclarationResponse { targets },
                )
            }
            destack_query::QueryRequest::GotoTypeDefinition(params) => {
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let targets = context.goto_type_definition(params.position.offset);

                destack_query::QueryResponse::GotoTypeDefinition(
                    destack_query::GotoTypeDefinitionResponse { targets },
                )
            }
            destack_query::QueryRequest::GotoImplementation(params) => {
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let program = self.program_query_context(
                    session,
                    repository,
                    revision,
                    &[params.position.module.profile_id],
                )?;
                let targets = context.goto_implementation(&program, params.position.offset);

                destack_query::QueryResponse::GotoImplementation(
                    destack_query::GotoImplementationResponse { targets },
                )
            }
            destack_query::QueryRequest::FindReferences(params) => {
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let program = self.program_query_context(
                    session,
                    repository,
                    revision,
                    &[params.position.module.profile_id],
                )?;
                let references = context.find_references(
                    &program,
                    params.position.offset,
                    params.include_declaration,
                );

                destack_query::QueryResponse::FindReferences(
                    destack_query::FindReferencesResponse { references },
                )
            }
            destack_query::QueryRequest::CallItem(params) => {
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let item = context.call_item(params.position.offset);

                destack_query::QueryResponse::CallItem(destack_query::CallItemResponse { item })
            }
            destack_query::QueryRequest::IncomingCalls(params) => {
                let context = self.program_query_context(
                    session,
                    repository,
                    revision,
                    &[params.item.target.module.profile_id],
                )?;
                let calls = context.incoming_calls(&params.item);
                destack_query::QueryResponse::IncomingCalls(destack_query::IncomingCallsResponse {
                    calls,
                })
            }
            destack_query::QueryRequest::OutgoingCalls(params) => {
                let context = self.program_query_context(
                    session,
                    repository,
                    revision,
                    &[params.item.target.module.profile_id],
                )?;
                let calls = context.outgoing_calls(&params.item);
                destack_query::QueryResponse::OutgoingCalls(destack_query::OutgoingCallsResponse {
                    calls,
                })
            }
            destack_query::QueryRequest::TypeItem(params) => {
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let item = context.type_item(params.position.offset);

                destack_query::QueryResponse::TypeItem(destack_query::TypeItemResponse { item })
            }
            destack_query::QueryRequest::Supertypes(params) => {
                let context = self.program_query_context(
                    session,
                    repository,
                    revision,
                    &[params.item.target.module.profile_id],
                )?;
                let items = context.supertypes(&params.item);
                destack_query::QueryResponse::Supertypes(destack_query::SupertypesResponse {
                    items,
                })
            }
            destack_query::QueryRequest::Subtypes(params) => {
                let context = self.program_query_context(
                    session,
                    repository,
                    revision,
                    &[params.item.target.module.profile_id],
                )?;
                let items = context.subtypes(&params.item);
                destack_query::QueryResponse::Subtypes(destack_query::SubtypesResponse { items })
            }
            destack_query::QueryRequest::RenameTarget(params) => {
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let result = context.rename_target(params.position.offset);

                destack_query::QueryResponse::RenameTarget(destack_query::RenameTargetResponse {
                    result,
                })
            }
            destack_query::QueryRequest::Rename(params) => {
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let program = self.program_query_context(
                    session,
                    repository,
                    revision,
                    &[params.position.module.profile_id],
                )?;
                let edit = context.rename(&program, params.position.offset, &params.new_name);

                destack_query::QueryResponse::Rename(destack_query::RenameResponse { edit })
            }
            destack_query::QueryRequest::RenameFiles(params) => {
                let context =
                    self.program_query_context(session, repository, revision, &params.profile_ids)?;
                let edit = context.rename_files(&params.renames);
                destack_query::QueryResponse::RenameFiles(destack_query::RenameFilesResponse {
                    edit,
                })
            }
            destack_query::QueryRequest::ExtractFunction(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.range.module)?;
                let edit = context.extract_function(params.range.span, &params.new_name);

                destack_query::QueryResponse::ExtractFunction(
                    destack_query::ExtractFunctionResponse { edit },
                )
            }
            destack_query::QueryRequest::ExtractVariable(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.range.module)?;
                let edit = context.extract_variable(params.range.span, &params.new_name);

                destack_query::QueryResponse::ExtractVariable(
                    destack_query::ExtractVariableResponse { edit },
                )
            }
            destack_query::QueryRequest::Inline(params) => {
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let program = self.program_query_context(
                    session,
                    repository,
                    revision,
                    &[params.position.module.profile_id],
                )?;
                let edit = context.inline_symbol(&program, params.position.offset);

                destack_query::QueryResponse::Inline(destack_query::InlineResponse { edit })
            }
            destack_query::QueryRequest::ChangeSignature(params) => {
                let context = self.module_query_context(
                    session,
                    repository,
                    revision,
                    params.position.module,
                )?;
                let program = self.program_query_context(
                    session,
                    repository,
                    revision,
                    &[params.position.module.profile_id],
                )?;
                let edit = context.change_signature(
                    &program,
                    params.position.offset,
                    &params.new_parameters,
                    &params.new_arguments,
                );

                destack_query::QueryResponse::ChangeSignature(
                    destack_query::ChangeSignatureResponse { edit },
                )
            }
            destack_query::QueryRequest::CodeActions(params) => {
                let context =
                    self.module_query_context(session, repository, revision, params.range.module)?;
                let program = self.program_query_context(
                    session,
                    repository,
                    revision,
                    &[params.range.module.profile_id],
                )?;
                let diagnostics = diagnostics_by_file(repository, revision)?
                    .remove(&params.range.span.file)
                    .unwrap_or_default();
                let actions = context.code_actions(
                    &program,
                    params.range.span,
                    &diagnostics,
                    &params.context,
                );

                destack_query::QueryResponse::CodeActions(destack_query::CodeActionsResponse {
                    actions,
                })
            }
            destack_query::QueryRequest::Decorators(params) => {
                let profile_ids = match &params.scope {
                    destack_query::DecoratorScope::Module(module) => vec![module.profile_id],
                    destack_query::DecoratorScope::Program { profile_ids } => profile_ids.clone(),
                };
                let context =
                    self.program_query_context(session, repository, revision, &profile_ids)?;
                let decorators = context.decorators(&params.scope, params.name.as_deref());

                destack_query::QueryResponse::Decorators(destack_query::DecoratorsResponse {
                    decorators,
                })
            }
        };

        Ok(response)
    }

    /// Return the module query context for one module.
    fn module_query_context<'a>(
        &self,
        session: &Session,
        repository: &'a Repository,
        revision: Revision,
        module: destack_query::Module,
    ) -> Result<destack_query::ModuleQueryContext<'a>, Error> {
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

        let context = destack_query::module_query_context_exact(
            repository,
            revision,
            module.module_id,
            module.profile_id,
            checked_version,
            global_environment_version,
        );

        Ok(context)
    }

    /// Return a program query context for explicit profiles.
    fn program_query_context<'a>(
        &self,
        session: &Session,
        repository: &'a Repository,
        revision: Revision,
        profile_ids: &[ProfileId],
    ) -> Result<destack_query::ProgramQueryContext<'a>, Error> {
        let mut indexes = Vec::with_capacity(profile_ids.len());

        for profile_id in profile_ids {
            let key = ArtifactKey::program_index(*profile_id);
            let version = session
                .require(revision, key)
                .map_err(|error| Error::Internal {
                    detail: format!(
                        "failed to require program index for profile {profile_id:?}: {error}"
                    ),
                })?;

            let index = repository
                .artifact_table()
                .program_index(&version)
                .ok_or_else(|| Error::Internal {
                    detail: format!("program index payload is missing for profile {profile_id:?}"),
                })?;

            indexes.push((*profile_id, index));
        }

        Ok(destack_query::program_query_context(
            repository, revision, indexes,
        ))
    }
}
