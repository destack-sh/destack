use tspp_artifact::ArtifactKey;
use tspp_repository::{Repository, Revision};
use tspp_source::ProfileId;

use crate::{
    CodeActionKind, ProgramQueryContext, QueryError, QueryRequest, QueryResponse, QueryResult,
    rename_files, search_symbols,
};

impl QueryRequest {
    /// Return diagnostic artifacts read by this request.
    pub fn diagnostic_artifacts(&self) -> Vec<ArtifactKey> {
        if let Self::CodeActions(request) = self
            && request.context.includes(CodeActionKind::QuickFix)
        {
            vec![ArtifactKey::dir_checked(
                request.range.module.module_id,
                request.range.module.profile_id,
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
        require_artifacts: &'a (dyn Fn(&[ArtifactKey]) -> QueryResult<()> + Sync),
    ) -> QueryResult<QueryResponse> {
        // execute requests that read every selected program
        match self {
            Self::SearchSymbols(request) => {
                let programs = Self::programs(
                    repository,
                    revision,
                    selected_profile_ids,
                    require_artifacts,
                )?;
                let response = search_symbols(request, &programs)?;

                Ok(QueryResponse::SearchSymbols(response))
            }
            Self::RenameFiles(request) => {
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
                let response =
                    rename_files(request, repository, revision, &modules, require_artifacts)?;

                Ok(QueryResponse::RenameFiles(response))
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

                request.execute_program(&program)
            }
        }
    }

    /// Execute this request against its selected program.
    fn execute_program(self, program: &ProgramQueryContext<'_>) -> QueryResult<QueryResponse> {
        let response = match self {
            Self::Completion(request) => {
                let context = program.module(request.position.module.module_id)?;
                let response = context.completion(request, program)?;

                QueryResponse::Completion(response)
            }
            Self::CompletionDetails(request) => {
                let context = program.module(request.module.module_id)?;
                let response = context.completion_details(request, program)?;

                QueryResponse::CompletionDetails(response)
            }
            Self::Hover(request) => {
                let context = program.module(request.position.module.module_id)?;
                let response = context.hover(request, program)?;

                QueryResponse::Hover(response)
            }
            Self::SignatureHelp(request) => {
                let context = program.module(request.position.module.module_id)?;
                let response = context.signature_help(request, program)?;

                QueryResponse::SignatureHelp(response)
            }
            Self::InlayHints(request) => {
                let context = program.module(request.range.module.module_id)?;
                let response = context.inlay_hints(request, program)?;

                QueryResponse::InlayHints(response)
            }
            Self::CodeLenses(request) => {
                let context = program.module(request.module.module_id)?;
                let response = context.code_lenses(request, program)?;

                QueryResponse::CodeLenses(response)
            }
            Self::FoldingRanges(request) => {
                let context = program.module(request.module.module_id)?;
                let response = context.folding_ranges(request)?;

                QueryResponse::FoldingRanges(response)
            }
            Self::SemanticTokens(request) => {
                let context = program.module(request.module.module_id)?;
                let response = context.semantic_tokens(request, program)?;

                QueryResponse::SemanticTokens(response)
            }
            Self::SemanticTokensRange(request) => {
                let context = program.module(request.range.module.module_id)?;
                let response = context.semantic_tokens_range(request, program)?;

                QueryResponse::SemanticTokensRange(response)
            }
            Self::Outline(request) => {
                let context = program.module(request.module.module_id)?;
                let response = context.outline(request, program)?;

                QueryResponse::Outline(response)
            }
            Self::Links(request) => {
                let context = program.module(request.module.module_id)?;
                let response = context.links(request)?;

                QueryResponse::Links(response)
            }
            Self::Highlight(request) => {
                let context = program.module(request.position.module.module_id)?;
                let response = context.highlight(request, program)?;

                QueryResponse::Highlight(response)
            }
            Self::SelectionRanges(request) => {
                let context = program.module(request.module.module_id)?;
                let response = context.selection_ranges(request)?;

                QueryResponse::SelectionRanges(response)
            }
            Self::GotoDefinition(request) => {
                let context = program.module(request.position.module.module_id)?;
                let response = context.goto_definition(request, program)?;

                QueryResponse::GotoDefinition(response)
            }
            Self::GotoDeclaration(request) => {
                let context = program.module(request.position.module.module_id)?;
                let response = context.goto_declaration(request, program)?;

                QueryResponse::GotoDeclaration(response)
            }
            Self::GotoTypeDefinition(request) => {
                let context = program.module(request.position.module.module_id)?;
                let response = context.goto_type_definition(request, program)?;

                QueryResponse::GotoTypeDefinition(response)
            }
            Self::GotoImplementation(request) => {
                let context = program.module(request.position.module.module_id)?;
                let response = context.goto_implementation(request, program)?;

                QueryResponse::GotoImplementation(response)
            }
            Self::FindReferences(request) => {
                let context = program.module(request.position.module.module_id)?;
                let response = context.find_references(request, program)?;

                QueryResponse::FindReferences(response)
            }
            Self::CallItem(request) => {
                let context = program.module(request.position.module.module_id)?;
                let response = context.call_item(request, program)?;

                QueryResponse::CallItem(response)
            }
            Self::IncomingCalls(request) => {
                let response = program.incoming_calls(request)?;

                QueryResponse::IncomingCalls(response)
            }
            Self::OutgoingCalls(request) => {
                let response = program.outgoing_calls(request)?;

                QueryResponse::OutgoingCalls(response)
            }
            Self::TypeItem(request) => {
                let context = program.module(request.position.module.module_id)?;
                let response = context.type_item(request, program)?;

                QueryResponse::TypeItem(response)
            }
            Self::Supertypes(request) => {
                let response = program.supertypes(request)?;

                QueryResponse::Supertypes(response)
            }
            Self::Subtypes(request) => {
                let response = program.subtypes(request)?;

                QueryResponse::Subtypes(response)
            }
            Self::Decorators(request) => {
                let response = program.decorators(request)?;

                QueryResponse::Decorators(response)
            }
            Self::RenameTarget(request) => {
                let context = program.module(request.position.module.module_id)?;
                let response = context.rename_target(request, program)?;

                QueryResponse::RenameTarget(response)
            }
            Self::Rename(request) => {
                let context = program.module(request.position.module.module_id)?;
                let response = context.rename(request, program)?;

                QueryResponse::Rename(response)
            }
            Self::ExtractVariable(request) => {
                let context = program.module(request.range.module.module_id)?;
                let response = context.extract_variable(request, program)?;

                QueryResponse::ExtractVariable(response)
            }
            Self::Inline(request) => {
                let context = program.module(request.position.module.module_id)?;
                let response = context.inline(request, program)?;

                QueryResponse::Inline(response)
            }
            Self::CodeActions(request) => {
                let context = program.module(request.range.module.module_id)?;
                let response = context.code_actions(request, program)?;

                QueryResponse::CodeActions(response)
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
        require_artifacts: &'a (dyn Fn(&[ArtifactKey]) -> QueryResult<()> + Sync),
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
