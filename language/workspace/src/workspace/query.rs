use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::{ArtifactKey, PackageIndex};
use destack_query::{
    CallItemResponse, CodeActionsResponse, CodeLensesResponse, CompletionResponse, DecoratorScope,
    DecoratorsResponse, ExtractVariableResponse, FindReferencesResponse, FoldingRangesResponse,
    GotoDeclarationResponse, GotoDefinitionResponse, GotoImplementationResponse,
    GotoTypeDefinitionResponse, HighlightResponse, HoverResponse, IncomingCallsResponse,
    InlayHintsResponse, InlineResponse, LinksResponse, Module, ModuleQueryContext,
    OutgoingCallsResponse, OutlineResponse, ProgramQueryContext, QueryPosition, QueryRange,
    QueryRequest, QueryResponse, RenameFilesResponse, RenameResponse, RenameTargetResponse,
    SearchSymbolsResponse, SelectionRangesResponse, SemanticTokensResponse, SignatureHelpResponse,
    SubtypesResponse, SupertypesResponse, TypeItemResponse, rename_files, rename_files_artifacts,
    search_symbols,
};
use destack_repository::{ArtifactReader, Package, PackageKind, Revision};
use destack_serde::Reflect;
use destack_source::{File, ProfileId, Span};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{Error, diagnostics_by_file};

use super::{LocalWorkspace, SessionPin};

/// One source file resolved for semantic queries.
#[derive(Debug, Clone)]
pub struct QueryFile {
    /// The requested source path.
    pub path: PathBuf,
    /// The exact semantic revision.
    pub revision: Revision,
    /// The module containing the file.
    pub module: Module,
    /// The source file at the exact semantic revision.
    pub file: Arc<File>,
}

impl QueryFile {
    /// Return one byte position in this query file.
    pub fn position(&self, offset: u32) -> QueryPosition {
        QueryPosition {
            module: self.module,
            file_id: self.file.id,
            offset,
        }
    }

    /// Return one exact ordered byte range in this query file.
    pub fn range(&self, start: u32, end: u32) -> Option<QueryRange> {
        if start > end {
            return None;
        }

        Some(QueryRange {
            module: self.module,
            span: Span::new(self.file.id, start, end),
        })
    }
}

impl LocalWorkspace {
    /// Resolve one source file for semantic queries.
    pub fn resolve_query_file(
        &self,
        root: &Path,
        path: PathBuf,
    ) -> Result<Option<QueryFile>, Error> {
        // require the requested path to belong to the requested root
        let owning_root = self.root_at(&path)?;
        if owning_root != root {
            return Err(Error::PathNotInRoot { path });
        }

        // pin the root and resolve the requested source
        let session = self.pin_session(root)?;
        let Some(file_id) = session.file_id(&path)? else {
            return Ok(None);
        };
        let file = session.file(file_id)?;
        let repository = session.repository();
        let revision = session.revision();

        // resolve the containing module
        let module_id = repository.module_id_for_file(revision, file_id)?;
        let Some(module_id) = module_id else {
            return Ok(None);
        };
        let module = repository
            .module(revision, module_id)?
            .ok_or_else(|| Error::Internal {
                detail: format!("module {module_id:?} is missing from revision {revision}"),
            })?;

        // select the module package profile
        let package = repository
            .package(revision, module.package_id)?
            .ok_or_else(|| Error::Internal {
                detail: format!(
                    "package {:?} is missing from revision {revision}",
                    module.package_id
                ),
            })?;
        let profile_id = session.selected_profile_id(&package)?;
        let Some(profile_id) = profile_id else {
            return Err(Error::TargetNotSelected {
                package_id: package.id,
            });
        };
        let module = Module {
            module_id,
            profile_id,
        };

        Ok(Some(QueryFile {
            path,
            revision,
            module,
            file,
        }))
    }
}

/// Response from one semantic query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RunQueryResponse {
    /// The revision used for query execution.
    pub revision: Revision,
    /// The query response.
    pub response: QueryResponse,
}

/// Request to run one semantic query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RunQueryRequest {
    /// Revision selection for this query.
    pub revision: RevisionPolicy,
    /// The query request.
    pub request: QueryRequest,
}

/// Revision selection policy for one query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum RevisionPolicy {
    /// Use the ref's latest revision when execution starts.
    Latest,
    /// Use one exact immutable revision.
    Exact(Revision),
    /// Use one revision only if the ref still points at it.
    Current(Revision),
}

impl LocalWorkspace {
    /// Run one semantic query for a root.
    pub fn run_query(
        &self,
        root: &Path,
        request: RunQueryRequest,
    ) -> Result<RunQueryResponse, Error> {
        // pin the session selected by the revision policy
        let session = match request.revision {
            // select the latest ref state
            RevisionPolicy::Latest => self.pin_session(root)?,

            // pin the exact immutable revision
            RevisionPolicy::Exact(revision) => {
                let session = self.session(root)?;
                let repository = session.repository();
                let revision = repository.pin(revision)?;

                SessionPin::new(session, revision)
            }

            // require the ref to remain at the caller's revision
            RevisionPolicy::Current(expected) => {
                let session = self.pin_session(root)?;
                let current = session.revision();
                if current != expected {
                    return Err(Error::StaleRevision { expected, current });
                }

                session
            }
        };

        // execute against the pinned state
        let revision = session.revision();
        let response = session.query(request.request)?;

        Ok(RunQueryResponse { revision, response })
    }
}

impl SessionPin {
    /// Build a query response for a request payload.
    fn query(&self, request: QueryRequest) -> Result<QueryResponse, Error> {
        // dispatch by query request variant
        let response = match request {
            QueryRequest::Completion(params) => {
                let context = self.module_context(params.position.module)?;
                let program = self.program_context(params.position.module.profile_id)?;
                let packages = self.package_index(params.position.module.profile_id)?;
                let mut items = context
                    .completions(
                        &program,
                        &packages,
                        params.position.file_id,
                        params.position.offset,
                        params.trigger,
                    )
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
                let program = self.program_context(params.position.module.profile_id)?;
                let hover =
                    context.hover(&program, params.position.file_id, params.position.offset);

                QueryResponse::Hover(HoverResponse { hover })
            }
            QueryRequest::SignatureHelp(params) => {
                let context = self.module_context(params.position.module)?;
                let program = self.program_context(params.position.module.profile_id)?;
                let help = context.signature_help(
                    &program,
                    params.position.file_id,
                    params.position.offset,
                );

                QueryResponse::SignatureHelp(SignatureHelpResponse { help })
            }
            QueryRequest::InlayHints(params) => {
                let context = self.module_context(params.range.module)?;
                let program = self.program_context(params.range.module.profile_id)?;
                let hints = context.inlay_hints(&program, params.range.span);

                QueryResponse::InlayHints(InlayHintsResponse { hints })
            }
            QueryRequest::CodeLenses(params) => {
                let context = self.module_context(params.module)?;
                let program = self.program_context(params.module.profile_id)?;
                let lenses = context.code_lenses(&program, params.file_id);

                QueryResponse::CodeLenses(CodeLensesResponse { lenses })
            }
            QueryRequest::FoldingRanges(params) => {
                let context = self.module_context(params.module)?;
                let ranges = context.folding_ranges(params.file_id);

                QueryResponse::FoldingRanges(FoldingRangesResponse { ranges })
            }
            QueryRequest::SemanticTokens(params) => {
                let context = self.module_context(params.module)?;
                let program = self.program_context(params.module.profile_id)?;
                let tokens = context.semantic_tokens(&program, params.file_id);

                QueryResponse::SemanticTokens(SemanticTokensResponse { tokens })
            }
            QueryRequest::SemanticTokensRange(params) => {
                let context = self.module_context(params.range.module)?;
                let program = self.program_context(params.range.module.profile_id)?;
                let tokens = context.semantic_tokens_range(&program, params.range.span);

                QueryResponse::SemanticTokensRange(SemanticTokensResponse { tokens })
            }
            QueryRequest::Outline(params) => {
                let context = self.module_context(params.module)?;
                let program = self.program_context(params.module.profile_id)?;
                let symbols = context.outline(&program, params.file_id);

                QueryResponse::Outline(OutlineResponse { symbols })
            }
            QueryRequest::SearchSymbols(params) => {
                let profile_ids = self.selected_profile_ids()?;
                let programs = self.program_contexts(&profile_ids)?;
                let symbols =
                    search_symbols(&programs, &params.query, params.max_results as usize)?;

                QueryResponse::SearchSymbols(SearchSymbolsResponse { symbols })
            }
            QueryRequest::Links(params) => {
                let context = self.module_context(params.module)?;
                let links = context.links(params.file_id);

                QueryResponse::Links(LinksResponse { links })
            }
            QueryRequest::Highlight(params) => {
                let context = self.module_context(params.position.module)?;
                let program = self.program_context(params.position.module.profile_id)?;
                let highlights =
                    context.highlights(&program, params.position.file_id, params.position.offset);

                QueryResponse::Highlight(HighlightResponse { highlights })
            }
            QueryRequest::SelectionRanges(params) => {
                let context = self.module_context(params.module)?;
                let ranges = context.selection_ranges(params.file_id, &params.offsets);

                QueryResponse::SelectionRanges(SelectionRangesResponse { ranges })
            }
            QueryRequest::GotoDefinition(params) => {
                let context = self.module_context(params.position.module)?;
                let program = self.program_context(params.position.module.profile_id)?;
                let targets = context.goto_definition(
                    &program,
                    params.position.file_id,
                    params.position.offset,
                );

                QueryResponse::GotoDefinition(GotoDefinitionResponse { targets })
            }
            QueryRequest::GotoDeclaration(params) => {
                let context = self.module_context(params.position.module)?;
                let program = self.program_context(params.position.module.profile_id)?;
                let targets = context.goto_declaration(
                    &program,
                    params.position.file_id,
                    params.position.offset,
                );

                QueryResponse::GotoDeclaration(GotoDeclarationResponse { targets })
            }
            QueryRequest::GotoTypeDefinition(params) => {
                let context = self.module_context(params.position.module)?;
                let program = self.program_context(params.position.module.profile_id)?;
                let targets = context.goto_type_definition(
                    &program,
                    params.position.file_id,
                    params.position.offset,
                );

                QueryResponse::GotoTypeDefinition(GotoTypeDefinitionResponse { targets })
            }
            QueryRequest::GotoImplementation(params) => {
                let context = self.module_context(params.position.module)?;
                let program = self.program_context(params.position.module.profile_id)?;
                let targets = context.goto_implementation(
                    &program,
                    params.position.file_id,
                    params.position.offset,
                );

                QueryResponse::GotoImplementation(GotoImplementationResponse { targets })
            }
            QueryRequest::FindReferences(params) => {
                let context = self.module_context(params.position.module)?;
                let program = self.program_context(params.position.module.profile_id)?;
                let references = context.find_references(
                    &program,
                    params.position.file_id,
                    params.position.offset,
                    params.include_declaration,
                );

                QueryResponse::FindReferences(FindReferencesResponse { references })
            }
            QueryRequest::CallItem(params) => {
                let context = self.module_context(params.position.module)?;
                let program = self.program_context(params.position.module.profile_id)?;
                let item =
                    context.call_item(&program, params.position.file_id, params.position.offset);

                QueryResponse::CallItem(CallItemResponse { item })
            }
            QueryRequest::IncomingCalls(params) => {
                let context = self.program_context(params.item.target.module.profile_id)?;
                let calls = context.incoming_calls(&params.item);
                QueryResponse::IncomingCalls(IncomingCallsResponse { calls })
            }
            QueryRequest::OutgoingCalls(params) => {
                let context = self.program_context(params.item.target.module.profile_id)?;
                let calls = context.outgoing_calls(&params.item);
                QueryResponse::OutgoingCalls(OutgoingCallsResponse { calls })
            }
            QueryRequest::TypeItem(params) => {
                let context = self.module_context(params.position.module)?;
                let program = self.program_context(params.position.module.profile_id)?;
                let item =
                    context.type_item(&program, params.position.file_id, params.position.offset);

                QueryResponse::TypeItem(TypeItemResponse { item })
            }
            QueryRequest::Supertypes(params) => {
                let context = self.program_context(params.item.target.module.profile_id)?;
                let items = context.supertypes(&params.item);
                QueryResponse::Supertypes(SupertypesResponse { items })
            }
            QueryRequest::Subtypes(params) => {
                let context = self.program_context(params.item.target.module.profile_id)?;
                let items = context.subtypes(&params.item);
                QueryResponse::Subtypes(SubtypesResponse { items })
            }
            QueryRequest::RenameTarget(params) => {
                let context = self.module_context(params.position.module)?;
                let program = self.program_context(params.position.module.profile_id)?;
                let target = context.rename_target(
                    &program,
                    params.position.file_id,
                    params.position.offset,
                );

                QueryResponse::RenameTarget(RenameTargetResponse { target })
            }
            QueryRequest::Rename(params) => {
                let context = self.module_context(params.position.module)?;
                let program = self.program_context(params.position.module.profile_id)?;
                let edit = context.rename(
                    &program,
                    params.position.file_id,
                    params.position.offset,
                    &params.new_name,
                );

                QueryResponse::Rename(RenameResponse { edit })
            }
            QueryRequest::RenameFiles(params) => {
                let profile_ids = self.selected_profile_ids()?;
                let programs = self.program_contexts(&profile_ids)?;
                let mut modules = Vec::new();
                for program in &programs {
                    modules.extend(program.authored_modules()?);
                }
                for module in &modules {
                    self.provide_rename_files_artifacts(*module)?;
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
                let program = self.program_context(params.range.module.profile_id)?;
                let edit = context.extract_variable(&program, params.range.span, &params.new_name);

                QueryResponse::ExtractVariable(ExtractVariableResponse { edit })
            }
            QueryRequest::Inline(params) => {
                let context = self.module_context(params.position.module)?;
                let program = self.program_context(params.position.module.profile_id)?;
                let edit = context.inline_symbol(
                    &program,
                    params.position.file_id,
                    params.position.offset,
                );

                QueryResponse::Inline(InlineResponse { edit })
            }
            QueryRequest::CodeActions(params) => {
                let context = self.module_context(params.range.module)?;
                let program = self.program_context(params.range.module.profile_id)?;
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
                    DecoratorScope::Program => self.selected_profile_ids()?,
                };
                let programs = self.program_contexts(&profile_ids)?;
                let decorators = programs
                    .iter()
                    .flat_map(|program| program.decorators(&params.scope, params.name.as_deref()))
                    .collect();

                QueryResponse::Decorators(DecoratorsResponse { decorators })
            }
        };

        Ok(response)
    }

    /// Return the profile selected for one query package.
    fn selected_profile_id(&self, package: &Package) -> Result<Option<ProfileId>, Error> {
        // resolve an unambiguous package target
        let selected = self
            .repository()
            .package_default_target(self.revision(), package.id)?;
        let Some((target_id, _)) = selected else {
            if package.targets.is_empty() {
                return Ok(None);
            }

            return Err(Error::TargetNotSelected {
                package_id: package.id,
            });
        };

        // load the selected target profile
        let profile = self
            .repository()
            .profile_for_target(self.revision(), target_id)?;

        Ok(Some(profile.id()))
    }

    /// Return the module query context for one module.
    fn module_context(&self, module: Module) -> Result<ModuleQueryContext<'_>, Error> {
        let repository = self.repository();
        let revision = self.revision();
        let required = ModuleQueryContext::artifacts(module.module_id, module.profile_id);

        // provide the queried module's exact context artifacts
        self.session()
            .provide(revision, &required)
            .map_err(|error| Error::Internal {
                detail: format!(
                    "failed to provide query artifacts for module {}: {error}",
                    module.module_id
                ),
            })?;

        // build the module context over the ready revision state
        let context =
            ModuleQueryContext::new(repository, revision, module.module_id, module.profile_id);

        Ok(context)
    }

    /// Return the semantic profiles selected for root-wide queries.
    fn selected_profile_ids(&self) -> Result<Vec<ProfileId>, Error> {
        let repository = self.repository();
        let revision = self.revision();
        let mut profile_ids = Vec::new();

        // resolve one selected target for every configured package
        for package_id in repository.package_ids(revision)? {
            let package =
                repository
                    .package(revision, package_id)?
                    .ok_or_else(|| Error::Internal {
                        detail: format!("missing query package {package_id:?}"),
                    })?;
            if matches!(package.kind, PackageKind::Builtin | PackageKind::Dependency) {
                continue;
            }

            // packages without targets do not define semantic programs
            if let Some(profile_id) = self.selected_profile_id(&package)? {
                profile_ids.push(profile_id);
            }
        }

        // collapse packages sharing one semantic profile
        profile_ids.sort_unstable();
        profile_ids.dedup();

        Ok(profile_ids)
    }

    /// Return a program query context for one exact profile.
    fn program_context(&self, profile_id: ProfileId) -> Result<ProgramQueryContext<'_>, Error> {
        let mut programs = self.program_contexts(&[profile_id])?;
        programs.pop().ok_or_else(|| Error::Internal {
            detail: format!("program context is missing for profile {profile_id:?}"),
        })
    }

    /// Return program query contexts for exact profiles.
    fn program_contexts(
        &self,
        profile_ids: &[ProfileId],
    ) -> Result<Vec<ProgramQueryContext<'_>>, Error> {
        let repository = self.repository();
        let revision = self.revision();
        let required_artifacts = profile_ids
            .iter()
            .map(|profile_id| ArtifactKey::program_index(*profile_id))
            .collect::<Vec<_>>();
        let mut programs = Vec::with_capacity(profile_ids.len());

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

            programs.push(ProgramQueryContext::new(
                repository,
                revision,
                *profile_id,
                index,
            ));
        }

        Ok(programs)
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

    /// Provide the artifacts read while rewriting one module's specifiers.
    fn provide_rename_files_artifacts(&self, module: Module) -> Result<(), Error> {
        let required_artifacts = rename_files_artifacts(module.module_id, module.profile_id);
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
