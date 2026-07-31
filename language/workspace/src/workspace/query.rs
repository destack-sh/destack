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
    QueryRange, QueryRequest, QueryResponse, QueryResult, QueryScope, RenameFilesResponse,
    RenameResponse, RenameTargetResponse, SearchSymbolsResponse, SelectionRangesResponse,
    SemanticTokensRangeResponse, SemanticTokensResponse, SignatureHelpResponse, SubtypesResponse,
    SupertypesResponse, TypeItemResponse, rename_files, search_symbols,
};
use destack_repository::{ArtifactReader, RepositoryError, Revision, Trace};
use destack_serde::Reflect;
use destack_session::{ArtifactPriority, ArtifactRun};
use destack_source::{File, ProfileId, Span};
use serde::{Deserialize, Serialize};

use crate::RunGuard;
use crate::diagnostic::Error;

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
        let selected = session.selected_target(&package)?;
        let Some((_, profile_id)) = selected else {
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

/// One scheduled semantic query at an exact revision.
pub struct QueryRun {
    /// Pinned source and artifact state.
    session: SessionPin,
    /// Query executed after its artifacts become ready.
    request: QueryRequest,
    /// Trace spanning artifact provision and query execution.
    trace: Arc<Trace>,
    /// Semantic artifacts that must provide ready payloads.
    required: ArtifactRun,
    /// Diagnostic artifacts that may have failed terminal outcomes.
    diagnostics: Option<ArtifactRun>,
}

impl std::fmt::Debug for QueryRun {
    /// Format the visible query run state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("QueryRun")
            .field("revision", &self.session.revision())
            .field("method", &self.request.method())
            .field("required", &self.required)
            .field("diagnostics", &self.diagnostics)
            .finish()
    }
}

impl QueryRun {
    /// Return the exact revision pinned by this query.
    pub fn revision(&self) -> Revision {
        self.session.revision()
    }

    /// Return this query operation trace.
    pub fn trace(&self) -> Arc<Trace> {
        self.trace.clone()
    }

    /// Cancel this query when its caller abandons the operation.
    pub fn guard(&self) -> RunGuard {
        let mut cancellations = vec![self.required.cancellation()];
        if let Some(diagnostics) = self.diagnostics.as_ref() {
            cancellations.push(diagnostics.cancellation());
        }

        RunGuard::new(cancellations)
    }

    /// Wait for ready artifacts and execute the exact query.
    pub fn wait(self) -> Result<RunQueryResponse, Error> {
        let QueryRun {
            session,
            request,
            trace,
            required,
            diagnostics,
        } = self;
        let query_trace = trace.clone();
        let result = (|| {
            required.wait_ready()?;
            if let Some(diagnostics) = diagnostics {
                diagnostics.complete()?;
            }
            let revision = session.revision();
            let require_artifacts = |artifact_keys: &[ArtifactKey]| {
                required
                    .require(artifact_keys)
                    .map_err(QueryError::artifact)
            };
            let response =
                query_trace.span("query", || session.query(request, &require_artifacts))?;

            Ok(RunQueryResponse { revision, response })
        })();
        drop(required);
        session.session().finish_trace(trace);

        result
    }
}

/// Initial artifact roots selected for one semantic query.
struct QueryRoots {
    /// Artifact roots scheduled before query execution.
    initial: Vec<ArtifactKey>,
    /// Diagnostic roots that may have failed terminal outcomes.
    diagnostics: Vec<ArtifactKey>,
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
    /// Schedule one semantic query for a root.
    pub fn start_query(&self, root: &Path, request: RunQueryRequest) -> Result<QueryRun, Error> {
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

        // schedule known query roots into one operation trace
        let roots = session.query_roots(&request.request)?;
        let trace = session.session().start_trace();
        let required = session.session().schedule_artifacts_traced(
            session.revision(),
            &roots.initial,
            ArtifactPriority::Foreground,
            trace.clone(),
        );
        let diagnostics = (!roots.diagnostics.is_empty()).then(|| {
            session.session().schedule_artifacts_traced(
                session.revision(),
                &roots.diagnostics,
                ArtifactPriority::Foreground,
                trace.clone(),
            )
        });

        Ok(QueryRun {
            session,
            request: request.request,
            trace,
            required,
            diagnostics,
        })
    }

    /// Run one semantic query for a root.
    pub fn run_query(
        &self,
        root: &Path,
        request: RunQueryRequest,
    ) -> Result<RunQueryResponse, Error> {
        self.start_query(root, request)?.wait()
    }
}

impl SessionPin {
    /// Return the artifact roots scheduled before one query.
    fn query_roots(&self, request: &QueryRequest) -> Result<QueryRoots, Error> {
        let mut initial = Vec::new();

        // schedule known roots for the request scope
        match request.scope() {
            QueryScope::Module(module) => {
                Self::push_module_roots(module, &mut initial);
            }
            QueryScope::Program(profile_id) => {
                self.push_program_roots(request, profile_id, &mut initial)?;

                if let Some(module) = request.module() {
                    Self::push_module_roots(module, &mut initial);
                }
            }
            QueryScope::Workspace => {
                for profile_id in self.selected_profile_ids()? {
                    self.push_program_roots(request, profile_id, &mut initial)?;
                }
            }
        }

        // schedule the selected global environment for completion
        if let QueryRequest::Completion(params) = request {
            initial.push(ArtifactKey::global_environment(
                params.position.module.profile_id,
            ));
        }

        // schedule exact diagnostic roots for requested quick fixes
        let mut diagnostics = if let QueryRequest::CodeActions(params) = request
            && params.context.includes(CodeActionKind::QuickFix)
        {
            self.diagnostic_artifacts(&[params.range.module.module_id])?
        } else {
            Vec::new()
        };

        initial.sort_unstable();
        initial.dedup();
        diagnostics.sort_unstable();
        diagnostics.dedup();

        Ok(QueryRoots {
            initial,
            diagnostics,
        })
    }

    /// Append one module context's artifact roots.
    fn push_module_roots(module: Module, roots: &mut Vec<ArtifactKey>) {
        roots.extend(ModuleQueryContext::initial_roots(
            module.module_id,
            module.profile_id,
        ));
    }

    /// Append one program context's artifact roots.
    fn push_program_roots(
        &self,
        request: &QueryRequest,
        profile_id: ProfileId,
        roots: &mut Vec<ArtifactKey>,
    ) -> Result<(), Error> {
        let initial = ProgramQueryContext::initial_roots(
            self.repository(),
            self.revision(),
            profile_id,
            request.method(),
        )?;
        roots.extend(initial);

        Ok(())
    }

    /// Build a query response for a request payload.
    fn query(
        &self,
        request: QueryRequest,
        require_artifacts: &dyn Fn(&[ArtifactKey]) -> QueryResult<()>,
    ) -> Result<QueryResponse, Error> {
        // dispatch by query request variant
        let response = match request {
            QueryRequest::Completion(params) => {
                let program =
                    self.program_context(params.position.module.profile_id, require_artifacts)?;
                let context = program.module(params.position.module.module_id)?;
                let environment =
                    self.global_environment(params.position.module.profile_id, require_artifacts)?;
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
                let query =
                    self.program_context(params.position.module.profile_id, require_artifacts)?;
                let context = query.module(params.position.module.module_id)?;
                let hover =
                    context.hover(&query, params.position.file_id, params.position.offset)?;

                QueryResponse::Hover(HoverResponse { hover })
            }
            QueryRequest::SignatureHelp(params) => {
                let query =
                    self.program_context(params.position.module.profile_id, require_artifacts)?;
                let context = query.module(params.position.module.module_id)?;
                let help = context.signature_help(
                    &query,
                    params.position.file_id,
                    params.position.offset,
                )?;

                QueryResponse::SignatureHelp(SignatureHelpResponse { help })
            }
            QueryRequest::InlayHints(params) => {
                let query =
                    self.program_context(params.range.module.profile_id, require_artifacts)?;
                let context = query.module(params.range.module.module_id)?;
                let hints = context.inlay_hints(
                    &query,
                    params.range.span,
                    params.type_hints,
                    params.parameter_hints,
                )?;

                QueryResponse::InlayHints(InlayHintsResponse { hints })
            }
            QueryRequest::CodeLenses(params) => {
                let program = self.program_context(params.module.profile_id, require_artifacts)?;
                let context = program.module(params.module.module_id)?;
                let lenses = context.code_lenses(&program, params.file_id)?;

                QueryResponse::CodeLenses(CodeLensesResponse { lenses })
            }
            QueryRequest::FoldingRanges(params) => {
                let context = self.module(params.module, require_artifacts)?;
                let ranges = context.folding_ranges(params.file_id)?;

                QueryResponse::FoldingRanges(FoldingRangesResponse { ranges })
            }
            QueryRequest::SemanticTokens(params) => {
                let query = self.program_context(params.module.profile_id, require_artifacts)?;
                let context = query.module(params.module.module_id)?;
                let tokens = context.semantic_tokens(&query, params.file_id)?;

                QueryResponse::SemanticTokens(SemanticTokensResponse { tokens })
            }
            QueryRequest::SemanticTokensRange(params) => {
                let query =
                    self.program_context(params.range.module.profile_id, require_artifacts)?;
                let context = query.module(params.range.module.module_id)?;
                let tokens = context.semantic_tokens_range(&query, params.range.span)?;

                QueryResponse::SemanticTokensRange(SemanticTokensRangeResponse { tokens })
            }
            QueryRequest::Outline(params) => {
                let query = self.program_context(params.module.profile_id, require_artifacts)?;
                let context = query.module(params.module.module_id)?;
                let symbols = context.outline(&query, params.file_id)?;

                QueryResponse::Outline(OutlineResponse { symbols })
            }
            QueryRequest::SearchSymbols(params) => {
                let profile_ids = self.selected_profile_ids()?;
                let programs = self.program_contexts(&profile_ids, require_artifacts)?;
                let symbols =
                    search_symbols(&programs, &params.query, params.max_results as usize)?;

                QueryResponse::SearchSymbols(SearchSymbolsResponse { symbols })
            }
            QueryRequest::Links(params) => {
                let context = self.module(params.module, require_artifacts)?;
                let links = context.links(params.file_id)?;

                QueryResponse::Links(LinksResponse { links })
            }
            QueryRequest::Highlight(params) => {
                let program =
                    self.program_context(params.position.module.profile_id, require_artifacts)?;
                let context = program.module(params.position.module.module_id)?;
                let highlights =
                    context.highlight(&program, params.position.file_id, params.position.offset)?;

                QueryResponse::Highlight(HighlightResponse { highlights })
            }
            QueryRequest::SelectionRanges(params) => {
                let context = self.module(params.module, require_artifacts)?;
                let ranges = context.selection_ranges(params.file_id, &params.offsets)?;

                QueryResponse::SelectionRanges(SelectionRangesResponse { ranges })
            }
            QueryRequest::GotoDefinition(params) => {
                let query =
                    self.program_context(params.position.module.profile_id, require_artifacts)?;
                let context = query.module(params.position.module.module_id)?;
                let targets = context.goto_definition(
                    &query,
                    params.position.file_id,
                    params.position.offset,
                )?;

                QueryResponse::GotoDefinition(GotoDefinitionResponse { targets })
            }
            QueryRequest::GotoDeclaration(params) => {
                let query =
                    self.program_context(params.position.module.profile_id, require_artifacts)?;
                let context = query.module(params.position.module.module_id)?;
                let targets = context.goto_declaration(
                    &query,
                    params.position.file_id,
                    params.position.offset,
                )?;

                QueryResponse::GotoDeclaration(GotoDeclarationResponse { targets })
            }
            QueryRequest::GotoTypeDefinition(params) => {
                let query =
                    self.program_context(params.position.module.profile_id, require_artifacts)?;
                let context = query.module(params.position.module.module_id)?;
                let targets = context.goto_type_definition(
                    &query,
                    params.position.file_id,
                    params.position.offset,
                )?;

                QueryResponse::GotoTypeDefinition(GotoTypeDefinitionResponse { targets })
            }
            QueryRequest::GotoImplementation(params) => {
                let program =
                    self.program_context(params.position.module.profile_id, require_artifacts)?;
                let context = program.module(params.position.module.module_id)?;
                let targets = context.goto_implementation(
                    &program,
                    params.position.file_id,
                    params.position.offset,
                )?;

                QueryResponse::GotoImplementation(GotoImplementationResponse { targets })
            }
            QueryRequest::FindReferences(params) => {
                let program =
                    self.program_context(params.position.module.profile_id, require_artifacts)?;
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
                let query =
                    self.program_context(params.position.module.profile_id, require_artifacts)?;
                let context = query.module(params.position.module.module_id)?;
                let item =
                    context.call_item(&query, params.position.file_id, params.position.offset)?;

                QueryResponse::CallItem(CallItemResponse { item })
            }
            QueryRequest::IncomingCalls(params) => {
                let context =
                    self.program_context(params.item.target.module.profile_id, require_artifacts)?;
                let calls = context.incoming_calls(&params.item)?;
                QueryResponse::IncomingCalls(IncomingCallsResponse { calls })
            }
            QueryRequest::OutgoingCalls(params) => {
                let context =
                    self.program_context(params.item.target.module.profile_id, require_artifacts)?;
                let calls = context.outgoing_calls(&params.item)?;
                QueryResponse::OutgoingCalls(OutgoingCallsResponse { calls })
            }
            QueryRequest::TypeItem(params) => {
                let query =
                    self.program_context(params.position.module.profile_id, require_artifacts)?;
                let context = query.module(params.position.module.module_id)?;
                let item =
                    context.type_item(&query, params.position.file_id, params.position.offset)?;

                QueryResponse::TypeItem(TypeItemResponse { item })
            }
            QueryRequest::Supertypes(params) => {
                let context =
                    self.program_context(params.item.target.module.profile_id, require_artifacts)?;
                let items = context.supertypes(&params.item)?;
                QueryResponse::Supertypes(SupertypesResponse { items })
            }
            QueryRequest::Subtypes(params) => {
                let context =
                    self.program_context(params.item.target.module.profile_id, require_artifacts)?;
                let items = context.subtypes(&params.item)?;
                QueryResponse::Subtypes(SubtypesResponse { items })
            }
            QueryRequest::RenameTarget(params) => {
                let query =
                    self.program_context(params.position.module.profile_id, require_artifacts)?;
                let context = query.module(params.position.module.module_id)?;
                let target = context.rename_target(
                    &query,
                    params.position.file_id,
                    params.position.offset,
                )?;

                QueryResponse::RenameTarget(RenameTargetResponse { target })
            }
            QueryRequest::Rename(params) => {
                let program =
                    self.program_context(params.position.module.profile_id, require_artifacts)?;
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
                let programs = self.program_contexts(&profile_ids, require_artifacts)?;
                let mut modules = Vec::new();
                for program in &programs {
                    modules.extend(program.authored_modules()?);
                }

                let edit = rename_files(
                    self.repository(),
                    self.revision(),
                    &modules,
                    &params.renames,
                    require_artifacts,
                )?;

                QueryResponse::RenameFiles(RenameFilesResponse { edit })
            }
            QueryRequest::ExtractVariable(params) => {
                let query =
                    self.program_context(params.range.module.profile_id, require_artifacts)?;
                let context = query.module(params.range.module.module_id)?;
                let edit = context.extract_variable(&query, params.range.span, &params.new_name)?;

                QueryResponse::ExtractVariable(ExtractVariableResponse { edit })
            }
            QueryRequest::Inline(params) => {
                let program =
                    self.program_context(params.position.module.profile_id, require_artifacts)?;
                let context = program.module(params.position.module.module_id)?;
                let edit =
                    context.inline(&program, params.position.file_id, params.position.offset)?;

                QueryResponse::Inline(InlineResponse { edit })
            }
            QueryRequest::CodeActions(params) => {
                let program =
                    self.program_context(params.range.module.profile_id, require_artifacts)?;
                let context = program.module(params.range.module.module_id)?;
                let includes_quick_fixes = params.context.includes(CodeActionKind::QuickFix);
                let diagnostics = if includes_quick_fixes {
                    let artifact_keys =
                        self.diagnostic_artifacts(&[params.range.module.module_id])?;
                    let diagnostics = self
                        .repository()
                        .diagnostics_for_keys(self.revision(), &artifact_keys)?;
                    let mut diagnostics = diagnostics.group_by_file();
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
                let profile_id = match &params.scope {
                    DecoratorScope::Module(module) => module.profile_id,
                    DecoratorScope::Program(profile_id) => *profile_id,
                };
                let program = self.program_context(profile_id, require_artifacts)?;
                let decorators = program.decorators(&params.scope, params.name.as_deref())?;

                QueryResponse::Decorators(DecoratorsResponse { decorators })
            }
        };

        Ok(response)
    }

    /// Return one module context at the pinned query revision.
    fn module<'a>(
        &'a self,
        module: Module,
        require_artifacts: &'a dyn Fn(&[ArtifactKey]) -> QueryResult<()>,
    ) -> Result<ModuleQueryContext<'a>, Error> {
        let context = ModuleQueryContext::new(
            self.repository(),
            self.revision(),
            module.module_id,
            module.profile_id,
            require_artifacts,
        )?;

        Ok(context)
    }

    /// Return a program query context for one exact profile.
    fn program_context<'a>(
        &'a self,
        profile_id: ProfileId,
        require_artifacts: &'a dyn Fn(&[ArtifactKey]) -> QueryResult<()>,
    ) -> Result<ProgramQueryContext<'a>, Error> {
        let repository = self.repository();
        let revision = self.revision();

        ProgramQueryContext::new(repository, revision, profile_id, require_artifacts)
            .map_err(Error::from)
    }

    /// Return program query contexts for exact profiles.
    fn program_contexts<'a>(
        &'a self,
        profile_ids: &[ProfileId],
        require_artifacts: &'a dyn Fn(&[ArtifactKey]) -> QueryResult<()>,
    ) -> Result<Vec<ProgramQueryContext<'a>>, Error> {
        let mut programs = Vec::with_capacity(profile_ids.len());

        // build each exact program context
        for profile_id in profile_ids {
            programs.push(self.program_context(*profile_id, require_artifacts)?);
        }

        Ok(programs)
    }

    /// Return the global environment for one profile.
    fn global_environment(
        &self,
        profile_id: ProfileId,
        require_artifacts: &dyn Fn(&[ArtifactKey]) -> QueryResult<()>,
    ) -> Result<Arc<GlobalEnvironment>, Error> {
        let repository = self.repository();
        let revision = self.revision();

        // require and read the exact payload
        let artifact = ArtifactKey::global_environment(profile_id);
        require_artifacts(&[artifact])?;
        let artifacts = ArtifactReader::new(repository, revision);
        let environment = artifacts
            .global_environment(profile_id)
            .map_err(QueryError::from)?;

        Ok(environment)
    }
}
