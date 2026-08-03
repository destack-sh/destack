use destack_artifact::ArtifactKey;
use destack_repository::{Repository, Revision};
use destack_source::ProfileId;

use crate::{
    CallItemResponse, CodeActionKind, CodeActionsResponse, CodeLensesResponse, DecoratorsResponse,
    ExtractVariableResponse, FindReferencesResponse, FoldingRangesResponse,
    GotoDeclarationResponse, GotoDefinitionResponse, GotoImplementationResponse,
    GotoTypeDefinitionResponse, HighlightResponse, HoverResponse, IncomingCallsResponse,
    InlayHintsResponse, InlineResponse, LinksResponse, ModuleQueryContext, OutgoingCallsResponse,
    OutlineResponse, ProgramQueryContext, QueryError, QueryRequest, QueryResponse, QueryResult,
    RenameFilesResponse, RenameResponse, RenameTargetResponse, SearchSymbolsResponse,
    SelectionRangesResponse, SemanticTokensRangeResponse, SemanticTokensResponse,
    SignatureHelpResponse, SubtypesResponse, SupertypesResponse, TypeItemResponse, rename_files,
    search_symbols,
};

impl QueryRequest {
    /// Return artifacts scheduled before this request executes.
    pub fn initial_artifacts(
        &self,
        repository: &Repository,
        revision: Revision,
        selected_profile_ids: &[ProfileId],
    ) -> QueryResult<Vec<ArtifactKey>> {
        let mut artifacts = Vec::new();

        // schedule indexes for one program
        let profile_id = self.profile_id();
        if let Some(profile_id) = profile_id {
            for kind in self.method().index_kinds() {
                artifacts.push(ArtifactKey::program_index(profile_id, *kind));
            }
        }
        // schedule indexes for every selected program
        else {
            for profile_id in selected_profile_ids {
                for kind in self.method().index_kinds() {
                    artifacts.push(ArtifactKey::program_index(*profile_id, *kind));
                }
            }
        }

        // schedule import resolution for every program read by file renames
        if matches!(self, Self::RenameFiles(_)) {
            let module_ids = ProgramQueryContext::authored_module_ids(repository, revision)?;
            for profile_id in selected_profile_ids {
                for module_id in &module_ids {
                    artifacts.extend([
                        ArtifactKey::dir_parsed(*module_id),
                        ArtifactKey::dir_imported(*module_id, *profile_id),
                        ArtifactKey::dir_expanded(*module_id, *profile_id),
                    ]);
                }
            }
        }

        // schedule the anchored module context when one exists
        if let Some(module) = self.module() {
            artifacts.extend(ModuleQueryContext::initial_artifacts(
                module.module_id,
                module.profile_id,
            ));
        }

        artifacts.sort_unstable();
        artifacts.dedup();

        Ok(artifacts)
    }

    /// Return diagnostic artifacts read by this request.
    pub fn diagnostic_artifacts(&self) -> Vec<ArtifactKey> {
        if let Self::CodeActions(params) = self
            && params.context.includes(CodeActionKind::QuickFix)
        {
            vec![ArtifactKey::dir_checked(
                params.range.module.module_id,
                params.range.module.profile_id,
            )]
        } else {
            Vec::new()
        }
    }

    /// Execute this request against one exact repository revision.
    pub fn execute<'a>(
        self,
        repository: &'a Repository,
        revision: Revision,
        selected_profile_ids: &[ProfileId],
        require_artifacts: &'a dyn Fn(&[ArtifactKey]) -> QueryResult<()>,
    ) -> QueryResult<QueryResponse> {
        // execute requests that read every selected program
        match self {
            Self::SearchSymbols(params) => {
                let programs = Self::programs(
                    repository,
                    revision,
                    selected_profile_ids,
                    require_artifacts,
                )?;
                let symbols =
                    search_symbols(&programs, &params.query, params.max_results as usize)?;

                Ok(QueryResponse::SearchSymbols(SearchSymbolsResponse {
                    symbols,
                }))
            }
            Self::RenameFiles(params) => {
                let programs = Self::programs(
                    repository,
                    revision,
                    selected_profile_ids,
                    require_artifacts,
                )?;
                let mut modules = Vec::new();
                for program in &programs {
                    modules.extend(program.authored_modules()?);
                }
                let edit = rename_files(
                    repository,
                    revision,
                    &modules,
                    &params.renames,
                    require_artifacts,
                )?;

                Ok(QueryResponse::RenameFiles(RenameFilesResponse { edit }))
            }

            // execute every other request against its selected program
            request => {
                let profile_id = request.profile_id().ok_or_else(|| {
                    QueryError::invalid(format!(
                        "query method has no selected program: {}",
                        request.method().name()
                    ))
                })?;
                let program =
                    ProgramQueryContext::new(repository, revision, profile_id, require_artifacts)?;

                request.execute_program(&program, repository, revision)
            }
        }
    }

    /// Execute this request against its selected program.
    fn execute_program(
        self,
        program: &ProgramQueryContext<'_>,
        repository: &Repository,
        revision: Revision,
    ) -> QueryResult<QueryResponse> {
        let response = match self {
            Self::Completion(params) => {
                let context = program.module(params.position.module.module_id)?;
                let response = context.completion(
                    program,
                    params.position.file_id,
                    params.position.offset,
                    params.trigger,
                    params.include_auto_imports,
                )?;

                QueryResponse::Completion(response)
            }
            Self::Hover(params) => {
                let context = program.module(params.position.module.module_id)?;
                let hover =
                    context.hover(program, params.position.file_id, params.position.offset)?;

                QueryResponse::Hover(HoverResponse { hover })
            }
            Self::SignatureHelp(params) => {
                let context = program.module(params.position.module.module_id)?;
                let help = context.signature_help(
                    program,
                    params.position.file_id,
                    params.position.offset,
                )?;

                QueryResponse::SignatureHelp(SignatureHelpResponse { help })
            }
            Self::InlayHints(params) => {
                let context = program.module(params.range.module.module_id)?;
                let hints = context.inlay_hints(
                    program,
                    params.range.span,
                    params.type_hints,
                    params.parameter_hints,
                )?;

                QueryResponse::InlayHints(InlayHintsResponse { hints })
            }
            Self::CodeLenses(params) => {
                let context = program.module(params.module.module_id)?;
                let lenses = context.code_lenses(program, params.file_id)?;

                QueryResponse::CodeLenses(CodeLensesResponse { lenses })
            }
            Self::FoldingRanges(params) => {
                let context = program.module(params.module.module_id)?;
                let ranges = context.folding_ranges(params.file_id)?;

                QueryResponse::FoldingRanges(FoldingRangesResponse { ranges })
            }
            Self::SemanticTokens(params) => {
                let context = program.module(params.module.module_id)?;
                let tokens = context.semantic_tokens(program, params.file_id)?;

                QueryResponse::SemanticTokens(SemanticTokensResponse { tokens })
            }
            Self::SemanticTokensRange(params) => {
                let context = program.module(params.range.module.module_id)?;
                let tokens = context.semantic_tokens_range(program, params.range.span)?;

                QueryResponse::SemanticTokensRange(SemanticTokensRangeResponse { tokens })
            }
            Self::Outline(params) => {
                let context = program.module(params.module.module_id)?;
                let symbols = context.outline(program, params.file_id)?;

                QueryResponse::Outline(OutlineResponse { symbols })
            }
            Self::Links(params) => {
                let context = program.module(params.module.module_id)?;
                let links = context.links(params.file_id)?;

                QueryResponse::Links(LinksResponse { links })
            }
            Self::Highlight(params) => {
                let context = program.module(params.position.module.module_id)?;
                let highlights =
                    context.highlight(program, params.position.file_id, params.position.offset)?;

                QueryResponse::Highlight(HighlightResponse { highlights })
            }
            Self::SelectionRanges(params) => {
                let context = program.module(params.module.module_id)?;
                let ranges = context.selection_ranges(params.file_id, &params.offsets)?;

                QueryResponse::SelectionRanges(SelectionRangesResponse { ranges })
            }
            Self::GotoDefinition(params) => {
                let context = program.module(params.position.module.module_id)?;
                let targets = context.goto_definition(
                    program,
                    params.position.file_id,
                    params.position.offset,
                )?;

                QueryResponse::GotoDefinition(GotoDefinitionResponse { targets })
            }
            Self::GotoDeclaration(params) => {
                let context = program.module(params.position.module.module_id)?;
                let targets = context.goto_declaration(
                    program,
                    params.position.file_id,
                    params.position.offset,
                )?;

                QueryResponse::GotoDeclaration(GotoDeclarationResponse { targets })
            }
            Self::GotoTypeDefinition(params) => {
                let context = program.module(params.position.module.module_id)?;
                let targets = context.goto_type_definition(
                    program,
                    params.position.file_id,
                    params.position.offset,
                )?;

                QueryResponse::GotoTypeDefinition(GotoTypeDefinitionResponse { targets })
            }
            Self::GotoImplementation(params) => {
                let context = program.module(params.position.module.module_id)?;
                let targets = context.goto_implementation(
                    program,
                    params.position.file_id,
                    params.position.offset,
                )?;

                QueryResponse::GotoImplementation(GotoImplementationResponse { targets })
            }
            Self::FindReferences(params) => {
                let context = program.module(params.position.module.module_id)?;
                let references = context.find_references(
                    program,
                    params.position.file_id,
                    params.position.offset,
                    params.include_declaration,
                )?;

                QueryResponse::FindReferences(FindReferencesResponse { references })
            }
            Self::CallItem(params) => {
                let context = program.module(params.position.module.module_id)?;
                let item =
                    context.call_item(program, params.position.file_id, params.position.offset)?;

                QueryResponse::CallItem(CallItemResponse { item })
            }
            Self::IncomingCalls(params) => {
                let calls = program.incoming_calls(&params.item)?;

                QueryResponse::IncomingCalls(IncomingCallsResponse { calls })
            }
            Self::OutgoingCalls(params) => {
                let calls = program.outgoing_calls(&params.item)?;

                QueryResponse::OutgoingCalls(OutgoingCallsResponse { calls })
            }
            Self::TypeItem(params) => {
                let context = program.module(params.position.module.module_id)?;
                let item =
                    context.type_item(program, params.position.file_id, params.position.offset)?;

                QueryResponse::TypeItem(TypeItemResponse { item })
            }
            Self::Supertypes(params) => {
                let items = program.supertypes(&params.item)?;

                QueryResponse::Supertypes(SupertypesResponse { items })
            }
            Self::Subtypes(params) => {
                let items = program.subtypes(&params.item)?;

                QueryResponse::Subtypes(SubtypesResponse { items })
            }
            Self::Decorators(params) => {
                let decorators = program.decorators(&params.scope, params.name.as_deref())?;

                QueryResponse::Decorators(DecoratorsResponse { decorators })
            }
            Self::RenameTarget(params) => {
                let context = program.module(params.position.module.module_id)?;
                let target = context.rename_target(
                    program,
                    params.position.file_id,
                    params.position.offset,
                )?;

                QueryResponse::RenameTarget(RenameTargetResponse { target })
            }
            Self::Rename(params) => {
                let context = program.module(params.position.module.module_id)?;
                let edit = context.rename(
                    program,
                    params.position.file_id,
                    params.position.offset,
                    &params.new_name,
                )?;

                QueryResponse::Rename(RenameResponse { edit })
            }
            Self::ExtractVariable(params) => {
                let context = program.module(params.range.module.module_id)?;
                let edit =
                    context.extract_variable(program, params.range.span, &params.new_name)?;

                QueryResponse::ExtractVariable(ExtractVariableResponse { edit })
            }
            Self::Inline(params) => {
                let context = program.module(params.position.module.module_id)?;
                let edit =
                    context.inline(program, params.position.file_id, params.position.offset)?;

                QueryResponse::Inline(InlineResponse { edit })
            }
            Self::CodeActions(params) => {
                let context = program.module(params.range.module.module_id)?;
                let diagnostics = if params.context.includes(CodeActionKind::QuickFix) {
                    let artifact = ArtifactKey::dir_checked(
                        params.range.module.module_id,
                        params.range.module.profile_id,
                    );
                    let diagnostics = repository.diagnostics_for_keys(revision, &[artifact])?;
                    let mut diagnostics = diagnostics.group_by_file();
                    let mut file_diagnostics = Vec::new();

                    // retain diagnostics emitted for this exact source file
                    if let Some(diagnostics) = diagnostics.remove(&params.range.span.file) {
                        file_diagnostics = diagnostics;
                    }

                    file_diagnostics
                } else {
                    Vec::new()
                };
                let actions = context.code_actions(
                    program,
                    params.range.span,
                    &diagnostics,
                    &params.context,
                )?;

                QueryResponse::CodeActions(CodeActionsResponse { actions })
            }

            Self::SearchSymbols(_) | Self::RenameFiles(_) => {
                return Err(QueryError::invalid(
                    "multi-program query reached single program execution",
                ));
            }
        };

        Ok(response)
    }

    /// Build query contexts for the selected program profiles.
    fn programs<'a>(
        repository: &'a Repository,
        revision: Revision,
        profile_ids: &[ProfileId],
        require_artifacts: &'a dyn Fn(&[ArtifactKey]) -> QueryResult<()>,
    ) -> QueryResult<Vec<ProgramQueryContext<'a>>> {
        let mut programs = Vec::with_capacity(profile_ids.len());

        // build each exact program context
        for profile_id in profile_ids {
            let program =
                ProgramQueryContext::new(repository, revision, *profile_id, require_artifacts)?;
            programs.push(program);
        }

        Ok(programs)
    }
}
