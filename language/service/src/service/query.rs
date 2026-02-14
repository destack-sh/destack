use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

use crate::query::{self, QueryResponseEnvelope};
use destack_compiler::TaskOutcome;
use destack_source::{FileId, Span, Uri};
use destack_workspace::{ModuleContent, Program};

use super::{LanguageService, LanguageServiceError, WorkspaceHandleId};

impl LanguageService {
    /// Execute a workspace query for the workspace that owns the path.
    pub fn query_for_path(
        &self,
        path: &Path,
        request: query::QueryRequest,
    ) -> Result<QueryResponseEnvelope, LanguageServiceError> {
        // resolve the workspace handle for this path
        let handle = self.handle_for_path(path)?;

        // route to handle based query execution
        self.query_for_handle(handle, request)
    }

    /// Execute a workspace query for a specific workspace handle.
    pub fn query_for_handle(
        &self,
        handle: WorkspaceHandleId,
        request: query::QueryRequest,
    ) -> Result<QueryResponseEnvelope, LanguageServiceError> {
        // resolve the root and owning program
        let root = self.root_for_handle(handle)?;
        let program = self.program_for_root(&root);

        // dispatch query execution
        let response = self.query_response_for_request(&program, request)?;

        // build a snapshot id from current module state
        let snapshot_id = self.snapshot_id_for_program(handle, &program);

        Ok(QueryResponseEnvelope {
            snapshot_id,
            response,
        })
    }

    /// Build a query response for a request payload.
    fn query_response_for_request(
        &self,
        program: &Program,
        request: query::QueryRequest,
    ) -> Result<query::QueryResponse, LanguageServiceError> {
        // resolve the shared session for query helpers
        let session = self.session_ref();

        // dispatch by query request variant
        let response = match request {
            query::QueryRequest::Completion(params) => {
                let file_id = self.resolve_file_id(program, &params.uri);
                let mut items = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        query::completions(session, file_id, params.offset, params.trigger)
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
                let file_id = self.resolve_file_id(program, &params.uri);
                let hover = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        query::hover(session, file_id, params.offset)
                    }
                    None => None,
                };

                query::QueryResponse::Hover(query::HoverResponse { hover })
            }
            query::QueryRequest::SignatureHelp(params) => {
                let file_id = self.resolve_file_id(program, &params.uri);
                let help = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        query::signature_help(session, file_id, params.offset)
                    }
                    None => None,
                };

                query::QueryResponse::SignatureHelp(query::SignatureHelpResponse { help })
            }
            query::QueryRequest::InlayHints(params) => {
                let file_id = self.resolve_file_id(program, &params.uri);
                let hints = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        let range = self.span_for_offsets(file_id, params.start, params.end);
                        query::inlay_hints(session, file_id, range)
                    }
                    None => Vec::new(),
                };

                query::QueryResponse::InlayHints(query::InlayHintsResponse { hints })
            }
            query::QueryRequest::CodeLenses(params) => {
                let file_id = self.resolve_file_id(program, &params.uri);
                let lenses = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        query::code_lenses(session, file_id)
                    }
                    None => Vec::new(),
                };

                query::QueryResponse::CodeLenses(query::CodeLensesResponse { lenses })
            }
            query::QueryRequest::ResolveCodeLens(params) => {
                let lens = query::resolve_code_lens(session, &params.lens);
                query::QueryResponse::ResolveCodeLens(query::ResolveCodeLensResponse { lens })
            }
            query::QueryRequest::FoldingRanges(params) => {
                let file_id = self.resolve_file_id(program, &params.uri);
                let ranges = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        query::folding_ranges(session, file_id)
                    }
                    None => Vec::new(),
                };

                query::QueryResponse::FoldingRanges(query::FoldingRangesResponse { ranges })
            }
            query::QueryRequest::SemanticTokens(params) => {
                let file_id = self.resolve_file_id(program, &params.uri);
                let tokens = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        query::semantic_tokens(session, file_id)
                    }
                    None => Vec::new(),
                };

                query::QueryResponse::SemanticTokens(query::SemanticTokensResponse { tokens })
            }
            query::QueryRequest::SemanticTokensRange(params) => {
                let file_id = self.resolve_file_id(program, &params.uri);
                let tokens = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        let range = self.span_for_offsets(file_id, params.start, params.end);
                        query::semantic_tokens_range(session, file_id, range)
                    }
                    None => Vec::new(),
                };

                query::QueryResponse::SemanticTokensRange(query::SemanticTokensResponse { tokens })
            }
            query::QueryRequest::DocumentSymbols(params) => {
                let file_id = self.resolve_file_id(program, &params.uri);
                let symbols = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        query::document_symbols(session, file_id)
                    }
                    None => Vec::new(),
                };

                query::QueryResponse::DocumentSymbols(query::DocumentSymbolsResponse { symbols })
            }
            query::QueryRequest::WorkspaceSymbols(params) => {
                let symbols =
                    query::workspace_symbols(session, &params.query, params.max_results as usize);
                query::QueryResponse::WorkspaceSymbols(query::WorkspaceSymbolsResponse { symbols })
            }
            query::QueryRequest::DocumentLinks(params) => {
                let file_id = self.resolve_file_id(program, &params.uri);
                let links = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        query::document_links(session, file_id)
                    }
                    None => Vec::new(),
                };

                query::QueryResponse::DocumentLinks(query::DocumentLinksResponse { links })
            }
            query::QueryRequest::ResolveDocumentLink(params) => {
                let link = query::resolve_document_link(session, &params.link);
                query::QueryResponse::ResolveDocumentLink(query::ResolveDocumentLinkResponse {
                    link,
                })
            }
            query::QueryRequest::DocumentHighlight(params) => {
                let file_id = self.resolve_file_id(program, &params.uri);
                let highlights = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        query::document_highlight(session, file_id, params.offset)
                    }
                    None => Vec::new(),
                };

                query::QueryResponse::DocumentHighlight(query::DocumentHighlightResponse {
                    highlights,
                })
            }
            query::QueryRequest::SelectionRanges(params) => {
                let file_id = self.resolve_file_id(program, &params.uri);
                let ranges = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        query::selection_ranges(session, file_id, &params.offsets)
                    }
                    None => Vec::new(),
                };

                query::QueryResponse::SelectionRanges(query::SelectionRangesResponse { ranges })
            }
            query::QueryRequest::GotoDefinition(params) => {
                let file_id = self.resolve_file_id(program, &params.uri);
                let result = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        query::goto_definition(session, file_id, params.offset)
                    }
                    None => None,
                };

                query::QueryResponse::GotoDefinition(query::GotoDefinitionResponse { result })
            }
            query::QueryRequest::GotoDeclaration(params) => {
                let file_id = self.resolve_file_id(program, &params.uri);
                let result = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        query::goto_declaration(session, file_id, params.offset)
                    }
                    None => None,
                };

                query::QueryResponse::GotoDeclaration(query::GotoDeclarationResponse { result })
            }
            query::QueryRequest::GotoTypeDefinition(params) => {
                let file_id = self.resolve_file_id(program, &params.uri);
                let result = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        query::goto_type_definition(session, file_id, params.offset)
                    }
                    None => None,
                };

                query::QueryResponse::GotoTypeDefinition(query::GotoTypeDefinitionResponse {
                    result,
                })
            }
            query::QueryRequest::GotoImplementation(params) => {
                let file_id = self.resolve_file_id(program, &params.uri);
                let result = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        query::goto_implementation(session, file_id, params.offset)
                    }
                    None => None,
                };

                query::QueryResponse::GotoImplementation(query::GotoImplementationResponse {
                    result,
                })
            }
            query::QueryRequest::FindReferences(params) => {
                let file_id = self.resolve_file_id(program, &params.uri);
                let result = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        query::find_references(
                            session,
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
                let file_id = self.resolve_file_id(program, &params.uri);
                let item = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        query::prepare_call_hierarchy(session, file_id, params.offset)
                    }
                    None => None,
                };

                query::QueryResponse::PrepareCallHierarchy(query::PrepareCallHierarchyResponse {
                    item,
                })
            }
            query::QueryRequest::CallHierarchyIncoming(params) => {
                let calls = query::incoming_calls(session, &params.item);
                query::QueryResponse::CallHierarchyIncoming(query::CallHierarchyIncomingResponse {
                    calls,
                })
            }
            query::QueryRequest::CallHierarchyOutgoing(params) => {
                let calls = query::outgoing_calls(session, &params.item);
                query::QueryResponse::CallHierarchyOutgoing(query::CallHierarchyOutgoingResponse {
                    calls,
                })
            }
            query::QueryRequest::PrepareTypeHierarchy(params) => {
                let file_id = self.resolve_file_id(program, &params.uri);
                let item = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        query::prepare_type_hierarchy(session, file_id, params.offset)
                    }
                    None => None,
                };

                query::QueryResponse::PrepareTypeHierarchy(query::PrepareTypeHierarchyResponse {
                    item,
                })
            }
            query::QueryRequest::TypeHierarchySupertypes(params) => {
                let items = query::supertypes(session, &params.item);
                query::QueryResponse::TypeHierarchySupertypes(
                    query::TypeHierarchySupertypesResponse { items },
                )
            }
            query::QueryRequest::TypeHierarchySubtypes(params) => {
                let items = query::subtypes(session, &params.item);
                query::QueryResponse::TypeHierarchySubtypes(query::TypeHierarchySubtypesResponse {
                    items,
                })
            }
            query::QueryRequest::PrepareRename(params) => {
                let file_id = self.resolve_file_id(program, &params.uri);
                let result = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        query::prepare_rename(session, file_id, params.offset)
                    }
                    None => None,
                };

                query::QueryResponse::PrepareRename(query::PrepareRenameResponse { result })
            }
            query::QueryRequest::Rename(params) => {
                let file_id = self.resolve_file_id(program, &params.uri);
                let result = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        query::rename(session, file_id, params.offset, &params.new_name)
                    }
                    None => None,
                };

                query::QueryResponse::Rename(query::RenameResponse { result })
            }
            query::QueryRequest::RenameFiles(params) => {
                let result = query::rename_files(session, &params.renames);
                query::QueryResponse::RenameFiles(query::RenameFilesResponse { result })
            }
            query::QueryRequest::ExtractFunction(params) => {
                let file_id = self.resolve_file_id(program, &params.uri);
                let result = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        let selection = self.span_for_offsets(file_id, params.start, params.end);
                        query::extract_function(session, file_id, selection, &params.new_name)
                    }
                    None => None,
                };

                query::QueryResponse::ExtractFunction(query::ExtractFunctionResponse { result })
            }
            query::QueryRequest::ExtractVariable(params) => {
                let file_id = self.resolve_file_id(program, &params.uri);
                let result = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        let selection = self.span_for_offsets(file_id, params.start, params.end);
                        query::extract_variable(session, file_id, selection, &params.new_name)
                    }
                    None => None,
                };

                query::QueryResponse::ExtractVariable(query::ExtractVariableResponse { result })
            }
            query::QueryRequest::Inline(params) => {
                let file_id = self.resolve_file_id(program, &params.uri);
                let result = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        query::inline_symbol(session, file_id, params.offset)
                    }
                    None => None,
                };

                query::QueryResponse::Inline(query::InlineResponse { result })
            }
            query::QueryRequest::ChangeSignature(params) => {
                let file_id = self.resolve_file_id(program, &params.uri);
                let result = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        query::change_signature(
                            session,
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
                let file_id = self.resolve_file_id(program, &params.uri);
                let actions = match file_id {
                    Some(file_id) => {
                        let file_id =
                            self.ensure_semantic_query_ready(program, file_id, &params.uri)?;
                        let range = self.span_for_offsets(file_id, params.start, params.end);
                        query::code_actions(session, file_id, range, &params.context)
                    }
                    None => Vec::new(),
                };

                query::QueryResponse::CodeActions(query::CodeActionsResponse { actions })
            }
        };

        Ok(response)
    }

    /// Resolve a file id for a query uri.
    fn resolve_file_id(&self, program: &Program, uri: &Uri) -> Option<FileId> {
        // prefer module lookups by uri
        if let Some(module_id) = program.modules.get_id_by_uri(uri) {
            let module = program.modules.get(module_id);
            return Some(module.read().file_id);
        }

        // fall back to module lookups by path
        if let Some(path) = uri.to_path_buf()
            && let Some(module_id) = program.modules.get_id_by_path(&path)
        {
            let module = program.modules.get(module_id);
            return Some(module.read().file_id);
        }

        // fall back to file registry lookups by uri
        if let Some(file_id) = program.files.get_id_by_uri(uri) {
            return Some(file_id);
        }

        // finally try file registry lookups by path
        let path = uri.to_path_buf()?;
        program.files.get_id_by_path(&path)
    }

    /// Ensure semantic query state is ready for a file.
    fn ensure_semantic_query_ready(
        &self,
        program: &Program,
        file_id: FileId,
        uri: &Uri,
    ) -> Result<FileId, LanguageServiceError> {
        // return early when semantic query state is already ready
        if self.semantic_query_ready(file_id) {
            return Ok(file_id);
        }

        // run cheap validation first
        let session = self.session_ref();
        let validate_outcome = self.validate_semantic_query_module(program, file_id);
        if validate_outcome.is_some() {
            // check original file id after validation
            if self.semantic_query_ready(file_id) {
                return Ok(file_id);
            }

            // check resolved file id after validation
            if let Some(resolved_file_id) = self.resolve_file_id(program, uri)
                && self.semantic_query_ready(resolved_file_id)
            {
                return Ok(resolved_file_id);
            }
        }

        // resolve a path for explicit analyze fallback
        let path = program
            .files
            .get_maybe(file_id)
            .and_then(|file| file.path.clone().or_else(|| file.uri.to_path_buf()))
            .or_else(|| uri.to_path_buf());

        let Some(path) = path else {
            return Err(LanguageServiceError::SemanticQueryNotReady {
                detail: "semantic query state is not ready for the requested uri".to_string(),
            });
        };

        // run analysis fallback for this path
        let analyze_result = self.analyze_path(&path)?;

        // surface analyze failure details directly
        if !analyze_result.semantic_query_ready {
            let detail = analyze_result
                .detail
                .unwrap_or_else(|| "analysis did not produce a semantic query state".to_string());
            return Err(LanguageServiceError::SemanticQueryNotReady { detail });
        }

        // check original file id after analyze fallback
        if self.semantic_query_ready(file_id) {
            return Ok(file_id);
        }

        // check resolved file id after analyze fallback
        if let Some(resolved_file_id) = self.resolve_file_id(program, uri)
            && self.semantic_query_ready(resolved_file_id)
        {
            return Ok(resolved_file_id);
        }

        // build a detailed failure summary for diagnostics
        let detail = session
            .modules
            .get_by_file_id(file_id)
            .map(|module| {
                let module = module.read();
                let profile_id = session.default_profile_for_module(module.id);
                let ast_ready = module.ast_maybe().is_some();
                let base_dir_ready = module.dir_base_maybe().is_some();
                let dir_ready = module.dir_maybe(profile_id).is_some();
                let dir_profiles: Vec<_> = match &module.content {
                    ModuleContent::Code(code) => {
                        code.dirs.iter().filter_map(|dir| dir.profile_id).collect()
                    }
                    ModuleContent::Data { dirs, .. }
                    | ModuleContent::Text { dirs, .. }
                    | ModuleContent::Binary { dirs, .. } => {
                        dirs.iter().filter_map(|dir| dir.profile_id).collect()
                    }
                    ModuleContent::Unloaded => Vec::new(),
                };
                let path = module
                    .path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "<none>".to_string());
                let validate_state = validate_outcome
                    .as_ref()
                    .map(Self::task_outcome_label)
                    .unwrap_or("unavailable");
                let analyze_state = if analyze_result.semantic_query_ready {
                    "ready"
                } else {
                    "not_ready"
                };
                let analyze_detail = analyze_result.detail.as_deref().unwrap_or("none");
                format!(
                    "file_id={file_id:?} module_id={:?} profile_id={:?} ast_ready={ast_ready} base_dir_ready={base_dir_ready} dir_ready={dir_ready} dir_profiles={dir_profiles:?} validate_state={validate_state} analyze_state={analyze_state} analyze_detail={analyze_detail} path={path}",
                    module.id,
                    profile_id
                )
            })
            .unwrap_or_else(|| format!("file_id={file_id:?} module_id=<missing>"));

        Err(LanguageServiceError::SemanticQueryNotReady { detail })
    }

    /// Check if semantic query state is ready for the file.
    fn semantic_query_ready(&self, file_id: FileId) -> bool {
        // resolve the module for this file id
        let session = self.session_ref();
        let Some(module) = session.modules.get_by_file_id(file_id) else {
            return false;
        };

        // require both ast and profile dir state
        let module = module.read();
        let profile = session.default_profile_for_module(module.id);
        module.ast_maybe().is_some() && module.dir_maybe(profile).is_some()
    }

    /// Build a span from offsets for a file.
    fn span_for_offsets(&self, file_id: FileId, start: u32, end: u32) -> Span {
        // normalize offset order before building a span
        let range_start = start.min(end);
        let range_end = start.max(end);
        Span::new(file_id, range_start, range_end)
    }

    /// Return a stable label for task outcomes in diagnostics.
    fn task_outcome_label(outcome: &TaskOutcome) -> &'static str {
        match outcome {
            TaskOutcome::Yield { .. } => "yield",
            TaskOutcome::Error { .. } => "error",
            TaskOutcome::Skipped { .. } => "skipped",
            TaskOutcome::Complete => "complete",
        }
    }

    /// Build a snapshot id from current module state.
    fn snapshot_id_for_program(&self, handle: WorkspaceHandleId, program: &Program) -> String {
        let mut hasher = DefaultHasher::new();
        handle.0.hash(&mut hasher);
        program.modules.len().hash(&mut hasher);

        for module in program.modules.iter() {
            let module = module.read();
            module.id.hash(&mut hasher);
            module.version.hash(&mut hasher);
            module.source_version.hash(&mut hasher);
        }

        format!("handle:{}:{:x}", handle.0, hasher.finish())
    }
}
