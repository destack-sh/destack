use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use destack_artifact::{ArtifactKey, PackageIndex};
use destack_query::{
    CallItemResponse, CodeActionsResponse, CodeLensesResponse, CompletionResponse, DecoratorScope,
    DecoratorsResponse, ExtractVariableResponse, FindReferencesResponse, FoldingRangesResponse,
    GotoDeclarationResponse, GotoDefinitionResponse, GotoImplementationResponse,
    GotoTypeDefinitionResponse, HighlightResponse, HoverResponse, IncomingCallsResponse,
    InlayHintsResponse, InlineResponse, LinksResponse, Module, ModuleQueryContext,
    OutgoingCallsResponse, OutlineResponse, ProgramQueryContext, QueryRequest, QueryResponse,
    RenameFilesResponse, RenameResponse, RenameTargetResponse, ResolveCodeLensResponse,
    SelectionRangesResponse, SemanticTokensResponse, SignatureHelpResponse, SubtypesResponse,
    SupertypesResponse, SymbolSearchResponse, TypeItemResponse, module_query_artifacts,
    module_query_context, program_query_context, rename_files, resolve_code_lens,
    specifier_artifacts,
};
use destack_repository::{ArtifactReader, Revision};
use destack_serde::Reflect;
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
    pub response: QueryResponse,
}

/// Request to run one semantic query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct WorkspaceQueryRequest {
    /// Expected revision for this query.
    pub expected_revision: Option<Revision>,
    /// Query request payload.
    pub request: QueryRequest,
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
        request: QueryRequest,
        revision: RevisionPolicy,
    ) -> Result<QueryResult, Error> {
        let root = self.root_at(path)?;

        self.query_root(&root, request, revision)
    }

    /// Run one query for a root.
    pub fn query_root(
        &self,
        root: &Path,
        request: QueryRequest,
        revision: RevisionPolicy,
    ) -> Result<QueryResult, Error> {
        let snapshot = self.query_snapshot(root, revision)?;
        let revision = snapshot.revision();
        let response = snapshot.query(request)?;

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
}

impl Snapshot {
    /// Build a query response for a request payload.
    fn query(&self, request: QueryRequest) -> Result<QueryResponse, Error> {
        // dispatch by query request variant
        let response = match request {
            QueryRequest::Completion(params) => {
                let context = self.module_context(params.position.module)?;
                let program = self.program_context(&[params.position.module.profile_id])?;
                let packages = self.package_index(params.position.module.profile_id)?;
                let mut items = context
                    .completions(&program, &packages, params.position.offset, params.trigger)
                    .map_err(|error| Error::Internal {
                        detail: format!("failed to complete import path: {error}"),
                    })?;

                if !params.include_imports {
                    items.retain(|item| item.additional_text_edits.is_empty());
                }

                QueryResponse::Completion(CompletionResponse {
                    items,
                    is_incomplete: false,
                })
            }
            QueryRequest::Hover(params) => {
                let context = self.module_context(params.position.module)?;
                let hover = context.hover(params.position.offset);

                QueryResponse::Hover(HoverResponse { hover })
            }
            QueryRequest::SignatureHelp(params) => {
                let context = self.module_context(params.position.module)?;
                let help = context.signature_help(params.position.offset);

                QueryResponse::SignatureHelp(SignatureHelpResponse { help })
            }
            QueryRequest::InlayHints(params) => {
                let context = self.module_context(params.range.module)?;
                let hints = context.inlay_hints(params.range.span);

                QueryResponse::InlayHints(InlayHintsResponse { hints })
            }
            QueryRequest::CodeLenses(params) => {
                let context = self.module_context(params.module)?;
                let program = self.program_context(&[params.module.profile_id])?;
                let lenses = context.code_lenses(&program);

                QueryResponse::CodeLenses(CodeLensesResponse { lenses })
            }
            QueryRequest::ResolveCodeLens(params) => {
                let lens = resolve_code_lens(&params.lens);
                QueryResponse::ResolveCodeLens(ResolveCodeLensResponse { lens })
            }
            QueryRequest::FoldingRanges(params) => {
                let context = self.module_context(params.module)?;
                let ranges = context.folding_ranges();

                QueryResponse::FoldingRanges(FoldingRangesResponse { ranges })
            }
            QueryRequest::SemanticTokens(params) => {
                let context = self.module_context(params.module)?;
                let tokens = context.semantic_tokens();

                QueryResponse::SemanticTokens(SemanticTokensResponse { tokens })
            }
            QueryRequest::SemanticTokensRange(params) => {
                let context = self.module_context(params.range.module)?;
                let tokens = context.semantic_tokens_range(params.range.span);

                QueryResponse::SemanticTokensRange(SemanticTokensResponse { tokens })
            }
            QueryRequest::Outline(params) => {
                let context = self.module_context(params.module)?;
                let symbols = context.outline();

                QueryResponse::Outline(OutlineResponse { symbols })
            }
            QueryRequest::SymbolSearch(params) => {
                let context = self.program_context(&params.profile_ids)?;
                let symbols = context.search_symbols(&params.query, params.max_results as usize);
                QueryResponse::SymbolSearch(SymbolSearchResponse { symbols })
            }
            QueryRequest::Links(params) => {
                let context = self.module_context(params.module)?;
                let links = context.links();

                QueryResponse::Links(LinksResponse { links })
            }
            QueryRequest::Highlight(params) => {
                let context = self.module_context(params.position.module)?;
                let program = self.program_context(&[params.position.module.profile_id])?;
                let highlights = context.highlights(&program, params.position.offset);

                QueryResponse::Highlight(HighlightResponse { highlights })
            }
            QueryRequest::SelectionRanges(params) => {
                let context = self.module_context(params.module)?;
                let ranges = context.selection_ranges(&params.offsets);

                QueryResponse::SelectionRanges(SelectionRangesResponse { ranges })
            }
            QueryRequest::GotoDefinition(params) => {
                let context = self.module_context(params.position.module)?;
                let targets = context.goto_definition(params.position.offset);

                QueryResponse::GotoDefinition(GotoDefinitionResponse { targets })
            }
            QueryRequest::GotoDeclaration(params) => {
                let context = self.module_context(params.position.module)?;
                let targets = context.goto_declaration(params.position.offset);

                QueryResponse::GotoDeclaration(GotoDeclarationResponse { targets })
            }
            QueryRequest::GotoTypeDefinition(params) => {
                let context = self.module_context(params.position.module)?;
                let targets = context.goto_type_definition(params.position.offset);

                QueryResponse::GotoTypeDefinition(GotoTypeDefinitionResponse { targets })
            }
            QueryRequest::GotoImplementation(params) => {
                let context = self.module_context(params.position.module)?;
                let program = self.program_context(&[params.position.module.profile_id])?;
                let targets = context.goto_implementation(&program, params.position.offset);

                QueryResponse::GotoImplementation(GotoImplementationResponse { targets })
            }
            QueryRequest::FindReferences(params) => {
                let context = self.module_context(params.position.module)?;
                let program = self.program_context(&[params.position.module.profile_id])?;
                let references = context.find_references(
                    &program,
                    params.position.offset,
                    params.include_declaration,
                );

                QueryResponse::FindReferences(FindReferencesResponse { references })
            }
            QueryRequest::CallItem(params) => {
                let context = self.module_context(params.position.module)?;
                let item = context.call_item(params.position.offset);

                QueryResponse::CallItem(CallItemResponse { item })
            }
            QueryRequest::IncomingCalls(params) => {
                let context = self.program_context(&[params.item.target.module.profile_id])?;
                let calls = context.incoming_calls(&params.item);
                QueryResponse::IncomingCalls(IncomingCallsResponse { calls })
            }
            QueryRequest::OutgoingCalls(params) => {
                let context = self.program_context(&[params.item.target.module.profile_id])?;
                let calls = context.outgoing_calls(&params.item);
                QueryResponse::OutgoingCalls(OutgoingCallsResponse { calls })
            }
            QueryRequest::TypeItem(params) => {
                let context = self.module_context(params.position.module)?;
                let item = context.type_item(params.position.offset);

                QueryResponse::TypeItem(TypeItemResponse { item })
            }
            QueryRequest::Supertypes(params) => {
                let context = self.program_context(&[params.item.target.module.profile_id])?;
                let items = context.supertypes(&params.item);
                QueryResponse::Supertypes(SupertypesResponse { items })
            }
            QueryRequest::Subtypes(params) => {
                let context = self.program_context(&[params.item.target.module.profile_id])?;
                let items = context.subtypes(&params.item);
                QueryResponse::Subtypes(SubtypesResponse { items })
            }
            QueryRequest::RenameTarget(params) => {
                let context = self.module_context(params.position.module)?;
                let result = context.rename_target(params.position.offset);

                QueryResponse::RenameTarget(RenameTargetResponse { result })
            }
            QueryRequest::Rename(params) => {
                let context = self.module_context(params.position.module)?;
                let program = self.program_context(&[params.position.module.profile_id])?;
                let edit = context.rename(&program, params.position.offset, &params.new_name);

                QueryResponse::Rename(RenameResponse { edit })
            }
            QueryRequest::RenameFiles(params) => {
                let modules = self.modules(&params.profile_ids)?;
                for module in &modules {
                    self.require_specifiers(*module)?;
                }

                let edit = rename_files(
                    self.repository(),
                    self.revision(),
                    &modules,
                    &params.renames,
                )
                .map_err(|error| Error::Internal {
                    detail: format!("failed to rename module specifiers: {error}"),
                })?;

                QueryResponse::RenameFiles(RenameFilesResponse { edit })
            }
            QueryRequest::ExtractVariable(params) => {
                let context = self.module_context(params.range.module)?;
                let edit = context.extract_variable(params.range.span, &params.new_name);

                QueryResponse::ExtractVariable(ExtractVariableResponse { edit })
            }
            QueryRequest::Inline(params) => {
                let context = self.module_context(params.position.module)?;
                let program = self.program_context(&[params.position.module.profile_id])?;
                let edit = context.inline_symbol(&program, params.position.offset);

                QueryResponse::Inline(InlineResponse { edit })
            }
            QueryRequest::CodeActions(params) => {
                let context = self.module_context(params.range.module)?;
                let program = self.program_context(&[params.range.module.profile_id])?;
                let diagnostics = diagnostics_by_file(self.repository(), self.revision())?
                    .remove(&params.range.span.file)
                    .unwrap_or_default();
                let actions = context.code_actions(
                    &program,
                    params.range.span,
                    &diagnostics,
                    &params.context,
                );

                QueryResponse::CodeActions(CodeActionsResponse { actions })
            }
            QueryRequest::Decorators(params) => {
                let profile_ids = match &params.scope {
                    DecoratorScope::Module(module) => vec![module.profile_id],
                    DecoratorScope::Program { profile_ids } => profile_ids.clone(),
                };
                let context = self.program_context(&profile_ids)?;
                let decorators = context.decorators(&params.scope, params.name.as_deref());

                QueryResponse::Decorators(DecoratorsResponse { decorators })
            }
        };

        Ok(response)
    }

    /// Return the module query context for one module.
    fn module_context(&self, module: Module) -> Result<ModuleQueryContext<'_>, Error> {
        let repository = self.repository();
        let revision = self.revision();
        let artifacts = ArtifactReader::new(repository, revision);
        let mut pending = vec![module.module_id];
        let mut required = HashSet::new();

        // require every artifact the reachable module contexts may read
        while let Some(module_id) = pending.pop() {
            if !required.insert(module_id) {
                continue;
            }

            let required_artifacts = module_query_artifacts(module_id, module.profile_id);
            self.session()
                .provide(revision, &required_artifacts)
                .map_err(|error| Error::Internal {
                    detail: format!(
                        "failed to provide query artifacts for module {module_id}: {error}"
                    ),
                })?;

            // extend through the authoritative resolved module edges
            let resolved = artifacts
                .dir_resolved(module_id, module.profile_id)
                .map_err(|error| Error::Internal {
                    detail: format!(
                        "failed to read resolved query DIR for module {module_id}: {error}"
                    ),
                })?;
            pending.extend(resolved.target_modules());
        }

        // build a read view over the required revision state
        let context =
            module_query_context(repository, revision, module.module_id, module.profile_id);

        Ok(context)
    }

    /// Return a program query context for explicit profiles.
    fn program_context(&self, profile_ids: &[ProfileId]) -> Result<ProgramQueryContext<'_>, Error> {
        let repository = self.repository();
        let revision = self.revision();
        let required_artifacts = profile_ids
            .iter()
            .map(|profile_id| ArtifactKey::program_index(*profile_id))
            .collect::<Vec<_>>();
        let mut indexes = Vec::with_capacity(profile_ids.len());

        // provide all program indexes in one artifact run
        self.session()
            .provide(revision, &required_artifacts)
            .map_err(|error| Error::Internal {
                detail: format!("failed to provide program indexes: {error}"),
            })?;

        // read each exact ready payload
        let artifacts = ArtifactReader::new(repository, revision);
        for profile_id in profile_ids {
            let index = artifacts
                .program_index(*profile_id)
                .map_err(|error| Error::Internal {
                    detail: format!(
                        "failed to read program index for profile {profile_id:?}: {error}"
                    ),
                })?;

            indexes.push((*profile_id, index));
        }

        Ok(program_query_context(repository, revision, indexes))
    }

    /// Return the active package index for one profile.
    fn package_index(&self, profile_id: ProfileId) -> Result<Arc<PackageIndex>, Error> {
        let repository = self.repository();
        let revision = self.revision();
        let key = ArtifactKey::package_index(profile_id);
        self.session()
            .provide(revision, &[key])
            .map_err(|error| Error::Internal {
                detail: format!(
                    "failed to provide package index for profile {profile_id:?}: {error}"
                ),
            })?;

        // read the exact ready payload
        let artifacts = ArtifactReader::new(repository, revision);
        let packages = artifacts
            .package_index(profile_id)
            .map_err(|error| Error::Internal {
                detail: format!("failed to read package index for profile {profile_id:?}: {error}"),
            })?;

        Ok(packages)
    }

    /// Return every module in the selected profiles.
    fn modules(&self, profile_ids: &[ProfileId]) -> Result<Vec<Module>, Error> {
        let repository = self.repository();
        let revision = self.revision();
        let module_ids = repository.module_ids(revision)?;
        let mut modules = Vec::new();

        // select each module that participates in each profile
        for profile_id in profile_ids {
            for module_id in &module_ids {
                let profile = repository.module_profile_by_id(revision, *module_id, *profile_id)?;
                if profile.is_none() {
                    continue;
                }

                modules.push(Module {
                    module_id: *module_id,
                    profile_id: *profile_id,
                });
            }
        }

        Ok(modules)
    }

    /// Require the artifacts read while rewriting one module's specifiers.
    fn require_specifiers(&self, module: Module) -> Result<(), Error> {
        let required_artifacts = specifier_artifacts(module.module_id, module.profile_id);
        self.session()
            .provide(self.revision(), &required_artifacts)
            .map_err(|error| Error::Internal {
                detail: format!(
                    "failed to provide specifier artifacts for module {}: {error}",
                    module.module_id
                ),
            })?;

        Ok(())
    }
}
