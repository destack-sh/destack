use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::{ArtifactKey, GlobalEnvironment};
use destack_query::{
    CallItemResponse, CodeActionKind, CodeActionsResponse, CodeLensesResponse, DecoratorScope,
    DecoratorsResponse, ExtractVariableResponse, FindReferencesResponse, FoldingRangesResponse,
    GotoDeclarationResponse, GotoDefinitionResponse, GotoImplementationResponse,
    GotoTypeDefinitionResponse, HighlightResponse, HoverResponse, IncomingCallsResponse,
    InlayHintsResponse, InlineResponse, LinksResponse, Module, ModuleQueryContext,
    OutgoingCallsResponse, OutlineResponse, ProgramQueryContext, QueryError, QueryPosition,
    QueryRange, QueryRequest, QueryResponse, RenameFilesResponse, RenameResponse,
    RenameTargetResponse, SearchSymbolsResponse, SelectionRangesResponse,
    SemanticTokensRangeResponse, SemanticTokensResponse, SignatureHelpResponse, SubtypesResponse,
    SupertypesResponse, TypeItemResponse, rename_files, search_symbols,
};
use destack_repository::{ArtifactReader, Package, PackageKind, RepositoryError, Revision};
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
            .ok_or(RepositoryError::MissingModule { module: module_id })?;

        // select the module package profile
        let package_id = module.package_id;
        let package =
            repository
                .package(revision, package_id)?
                .ok_or(RepositoryError::MissingPackage {
                    package: package_id,
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
                let program = self.program_context(params.position.module.profile_id)?;
                let context = program.module(params.position.module.module_id)?;
                let environment = self.global_environment(params.position.module.profile_id)?;
                let response = context.completion(
                    &program,
                    &environment,
                    params.position.file_id,
                    params.position.offset,
                    params.trigger,
                    params.include_auto_imports,
                )?;

                QueryResponse::Completion(response)
            }
            QueryRequest::Hover(params) => {
                let program = self.program_context(params.position.module.profile_id)?;
                let context = program.module(params.position.module.module_id)?;
                let hover =
                    context.hover(&program, params.position.file_id, params.position.offset)?;

                QueryResponse::Hover(HoverResponse { hover })
            }
            QueryRequest::SignatureHelp(params) => {
                let program = self.program_context(params.position.module.profile_id)?;
                let context = program.module(params.position.module.module_id)?;
                let help = context.signature_help(
                    &program,
                    params.position.file_id,
                    params.position.offset,
                )?;

                QueryResponse::SignatureHelp(SignatureHelpResponse { help })
            }
            QueryRequest::InlayHints(params) => {
                let program = self.program_context(params.range.module.profile_id)?;
                let context = program.module(params.range.module.module_id)?;
                let hints = context.inlay_hints(
                    &program,
                    params.range.span,
                    params.type_hints,
                    params.parameter_hints,
                )?;

                QueryResponse::InlayHints(InlayHintsResponse { hints })
            }
            QueryRequest::CodeLenses(params) => {
                let program = self.program_context(params.module.profile_id)?;
                let context = program.module(params.module.module_id)?;
                let lenses = context.code_lenses(&program, params.file_id)?;

                QueryResponse::CodeLenses(CodeLensesResponse { lenses })
            }
            QueryRequest::FoldingRanges(params) => {
                let context = self.module_context(params.module)?;
                let ranges = context.folding_ranges(params.file_id)?;

                QueryResponse::FoldingRanges(FoldingRangesResponse { ranges })
            }
            QueryRequest::SemanticTokens(params) => {
                let program = self.program_context(params.module.profile_id)?;
                let context = program.module(params.module.module_id)?;
                let tokens = context.semantic_tokens(&program, params.file_id)?;

                QueryResponse::SemanticTokens(SemanticTokensResponse { tokens })
            }
            QueryRequest::SemanticTokensRange(params) => {
                let program = self.program_context(params.range.module.profile_id)?;
                let context = program.module(params.range.module.module_id)?;
                let tokens = context.semantic_tokens_range(&program, params.range.span)?;

                QueryResponse::SemanticTokensRange(SemanticTokensRangeResponse { tokens })
            }
            QueryRequest::Outline(params) => {
                let program = self.program_context(params.module.profile_id)?;
                let context = program.module(params.module.module_id)?;
                let symbols = context.outline(&program, params.file_id)?;

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
                let links = context.links(params.file_id)?;

                QueryResponse::Links(LinksResponse { links })
            }
            QueryRequest::Highlight(params) => {
                let program = self.program_context(params.position.module.profile_id)?;
                let context = program.module(params.position.module.module_id)?;
                let highlights =
                    context.highlight(&program, params.position.file_id, params.position.offset)?;

                QueryResponse::Highlight(HighlightResponse { highlights })
            }
            QueryRequest::SelectionRanges(params) => {
                let context = self.module_context(params.module)?;
                let ranges = context.selection_ranges(params.file_id, &params.offsets)?;

                QueryResponse::SelectionRanges(SelectionRangesResponse { ranges })
            }
            QueryRequest::GotoDefinition(params) => {
                let program = self.program_context(params.position.module.profile_id)?;
                let context = program.module(params.position.module.module_id)?;
                let targets = context.goto_definition(
                    &program,
                    params.position.file_id,
                    params.position.offset,
                )?;

                QueryResponse::GotoDefinition(GotoDefinitionResponse { targets })
            }
            QueryRequest::GotoDeclaration(params) => {
                let program = self.program_context(params.position.module.profile_id)?;
                let context = program.module(params.position.module.module_id)?;
                let targets = context.goto_declaration(
                    &program,
                    params.position.file_id,
                    params.position.offset,
                )?;

                QueryResponse::GotoDeclaration(GotoDeclarationResponse { targets })
            }
            QueryRequest::GotoTypeDefinition(params) => {
                let program = self.program_context(params.position.module.profile_id)?;
                let context = program.module(params.position.module.module_id)?;
                let targets = context.goto_type_definition(
                    &program,
                    params.position.file_id,
                    params.position.offset,
                )?;

                QueryResponse::GotoTypeDefinition(GotoTypeDefinitionResponse { targets })
            }
            QueryRequest::GotoImplementation(params) => {
                let program = self.program_context(params.position.module.profile_id)?;
                let context = program.module(params.position.module.module_id)?;
                let targets = context.goto_implementation(
                    &program,
                    params.position.file_id,
                    params.position.offset,
                )?;

                QueryResponse::GotoImplementation(GotoImplementationResponse { targets })
            }
            QueryRequest::FindReferences(params) => {
                let program = self.program_context(params.position.module.profile_id)?;
                let context = program.module(params.position.module.module_id)?;
                let references = context.find_references(
                    &program,
                    params.position.file_id,
                    params.position.offset,
                    params.include_declaration,
                )?;

                QueryResponse::FindReferences(FindReferencesResponse { references })
            }
            QueryRequest::CallItem(params) => {
                let program = self.program_context(params.position.module.profile_id)?;
                let context = program.module(params.position.module.module_id)?;
                let item =
                    context.call_item(&program, params.position.file_id, params.position.offset)?;

                QueryResponse::CallItem(CallItemResponse { item })
            }
            QueryRequest::IncomingCalls(params) => {
                let context = self.program_context(params.item.target.module.profile_id)?;
                let calls = context.incoming_calls(&params.item)?;
                QueryResponse::IncomingCalls(IncomingCallsResponse { calls })
            }
            QueryRequest::OutgoingCalls(params) => {
                let context = self.program_context(params.item.target.module.profile_id)?;
                let calls = context.outgoing_calls(&params.item)?;
                QueryResponse::OutgoingCalls(OutgoingCallsResponse { calls })
            }
            QueryRequest::TypeItem(params) => {
                let program = self.program_context(params.position.module.profile_id)?;
                let context = program.module(params.position.module.module_id)?;
                let item =
                    context.type_item(&program, params.position.file_id, params.position.offset)?;

                QueryResponse::TypeItem(TypeItemResponse { item })
            }
            QueryRequest::Supertypes(params) => {
                let context = self.program_context(params.item.target.module.profile_id)?;
                let items = context.supertypes(&params.item)?;
                QueryResponse::Supertypes(SupertypesResponse { items })
            }
            QueryRequest::Subtypes(params) => {
                let context = self.program_context(params.item.target.module.profile_id)?;
                let items = context.subtypes(&params.item)?;
                QueryResponse::Subtypes(SubtypesResponse { items })
            }
            QueryRequest::RenameTarget(params) => {
                let program = self.program_context(params.position.module.profile_id)?;
                let context = program.module(params.position.module.module_id)?;
                let target = context.rename_target(
                    &program,
                    params.position.file_id,
                    params.position.offset,
                )?;

                QueryResponse::RenameTarget(RenameTargetResponse { target })
            }
            QueryRequest::Rename(params) => {
                let program = self.program_context(params.position.module.profile_id)?;
                let context = program.module(params.position.module.module_id)?;
                let edit = context.rename(
                    &program,
                    params.position.file_id,
                    params.position.offset,
                    &params.new_name,
                )?;

                QueryResponse::Rename(RenameResponse { edit })
            }
            QueryRequest::RenameFiles(params) => {
                let profile_ids = self.selected_profile_ids()?;
                let programs = self.program_contexts(&profile_ids)?;
                let mut modules = Vec::new();
                for program in &programs {
                    modules.extend(program.authored_modules()?);
                }

                let edit = rename_files(
                    self.repository(),
                    self.revision(),
                    &modules,
                    &params.renames,
                )?;

                QueryResponse::RenameFiles(RenameFilesResponse { edit })
            }
            QueryRequest::ExtractVariable(params) => {
                let program = self.program_context(params.range.module.profile_id)?;
                let context = program.module(params.range.module.module_id)?;
                let edit =
                    context.extract_variable(&program, params.range.span, &params.new_name)?;

                QueryResponse::ExtractVariable(ExtractVariableResponse { edit })
            }
            QueryRequest::Inline(params) => {
                let program = self.program_context(params.position.module.profile_id)?;
                let context = program.module(params.position.module.module_id)?;
                let edit =
                    context.inline(&program, params.position.file_id, params.position.offset)?;

                QueryResponse::Inline(InlineResponse { edit })
            }
            QueryRequest::CodeActions(params) => {
                let program = self.program_context(params.range.module.profile_id)?;
                let context = program.module(params.range.module.module_id)?;
                let includes_quick_fixes = params.context.includes(CodeActionKind::QuickFix);
                let diagnostics = if includes_quick_fixes {
                    let mut diagnostics = diagnostics_by_file(self.repository(), self.revision())?;
                    let mut file_diagnostics = Vec::new();

                    // use the diagnostics emitted for this exact source file
                    if let Some(diagnostics) = diagnostics.remove(&params.range.span.file) {
                        file_diagnostics = diagnostics;
                    }

                    file_diagnostics
                }
                // skip diagnostic collection for requests without quick fixes
                else {
                    Vec::new()
                };
                let actions = context.code_actions(
                    &program,
                    params.range.span,
                    &diagnostics,
                    &params.context,
                )?;

                QueryResponse::CodeActions(CodeActionsResponse { actions })
            }
            QueryRequest::Decorators(params) => {
                let profile_ids = match &params.scope {
                    DecoratorScope::Module(module) => vec![module.profile_id],
                    DecoratorScope::Program => self.selected_profile_ids()?,
                };
                let programs = self.program_contexts(&profile_ids)?;
                let mut decorators = Vec::new();
                for program in &programs {
                    decorators.extend(program.decorators(&params.scope, params.name.as_deref())?);
                }

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
        let required = ModuleQueryContext::artifact_keys(module.module_id, module.profile_id);

        // provide the queried module's exact context artifacts
        self.session().provide(revision, &required)?;

        // build the module context over the ready revision state
        let context =
            ModuleQueryContext::new(repository, revision, module.module_id, module.profile_id)
                .map_err(QueryError::from)?;

        Ok(context)
    }

    /// Return the semantic profiles selected for root-wide queries.
    fn selected_profile_ids(&self) -> Result<Vec<ProfileId>, Error> {
        let repository = self.repository();
        let revision = self.revision();
        let mut profile_ids = Vec::new();

        // resolve one selected target for every configured package
        for package_id in repository.package_ids(revision)? {
            let package = repository.package(revision, package_id)?.ok_or(
                RepositoryError::MissingPackage {
                    package: package_id,
                },
            )?;
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
        let mut required_artifacts = Vec::new();
        let mut programs = Vec::with_capacity(profile_ids.len());

        // request every artifact read by each complete program context
        for profile_id in profile_ids {
            let artifacts = ProgramQueryContext::artifact_keys(repository, revision, *profile_id)
                .map_err(QueryError::from)?;
            required_artifacts.extend(artifacts);
        }
        required_artifacts.sort_unstable();
        required_artifacts.dedup();

        // provide every program context in one artifact run
        self.session().provide(revision, &required_artifacts)?;

        // read each exact ready payload
        let artifacts = ArtifactReader::new(repository, revision);
        for profile_id in profile_ids {
            let index = artifacts
                .program_index(*profile_id)
                .map_err(QueryError::from)?;
            let package_graph = artifacts
                .package_graph(*profile_id)
                .map_err(QueryError::from)?;

            let program =
                ProgramQueryContext::new(repository, revision, *profile_id, index, package_graph)?;
            programs.push(program);
        }

        Ok(programs)
    }

    /// Return the global environment for one profile.
    fn global_environment(&self, profile_id: ProfileId) -> Result<Arc<GlobalEnvironment>, Error> {
        let repository = self.repository();
        let revision = self.revision();
        let key = ArtifactKey::global_environment(profile_id);
        self.session().provide(revision, &[key])?;

        // read the exact ready payload
        let artifacts = ArtifactReader::new(repository, revision);
        let environment = artifacts
            .global_environment(profile_id)
            .map_err(QueryError::from)?;

        Ok(environment)
    }
}
