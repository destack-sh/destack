use std::path::Path;

use destack_query as query;
use destack_query::{QueryRequestEnvelope, QueryResponseEnvelope};
use destack_source::{FileId, Span, Uri};
use destack_workspace::Program;

use super::{LanguageService, LanguageServiceError, WorkspaceHandleId};

impl LanguageService {
    /// Execute a read query for the workspace that owns the path.
    pub fn execute_read_query_for_path(
        &self,
        path: &Path,
        request: query::QueryRequest,
    ) -> Result<QueryResponseEnvelope, LanguageServiceError> {
        // read queries never carry revision preconditions
        let envelope = QueryRequestEnvelope {
            expected_revision: None,
            request,
        };

        // route through read envelope execution
        self.execute_read_query_envelope_for_path(path, envelope)
    }

    /// Execute a read query envelope for the workspace that owns the path.
    pub fn execute_read_query_envelope_for_path(
        &self,
        path: &Path,
        envelope: QueryRequestEnvelope,
    ) -> Result<QueryResponseEnvelope, LanguageServiceError> {
        // resolve the workspace handle for this path
        let handle = self.workspace_handle_id_for_path(path)?;

        // route to handle based read query execution
        self.execute_read_query_envelope_for_workspace_handle(handle, envelope)
    }

    /// Execute a read query for a specific workspace handle.
    pub fn execute_read_query_for_workspace_handle(
        &self,
        handle: WorkspaceHandleId,
        request: query::QueryRequest,
    ) -> Result<QueryResponseEnvelope, LanguageServiceError> {
        // read queries never carry revision preconditions
        let envelope = QueryRequestEnvelope {
            expected_revision: None,
            request,
        };

        // route through read envelope execution
        self.execute_read_query_envelope_for_workspace_handle(handle, envelope)
    }

    /// Execute a read query envelope for a specific workspace handle.
    pub fn execute_read_query_envelope_for_workspace_handle(
        &self,
        handle: WorkspaceHandleId,
        envelope: QueryRequestEnvelope,
    ) -> Result<QueryResponseEnvelope, LanguageServiceError> {
        // reject any non-read requests on the read path
        let request_mode = envelope.request.execution_mode();
        if request_mode != query::QueryExecutionMode::Read {
            return Err(LanguageServiceError::QueryExecutionModeMismatch {
                method: envelope.request.method_id(),
                expected: query::QueryExecutionMode::Read,
                actual: request_mode,
            });
        }

        // reject revision preconditions on read path requests
        if let Some(expected_revision) = envelope.expected_revision {
            return Err(LanguageServiceError::UnexpectedExpectedRevisionOnRead {
                expected_revision,
            });
        }

        // resolve the owning workspace handle and program
        let workspace = self.workspace_handle_for_id(handle)?;

        // fail fast when a mutation is currently active on this workspace
        let _query_guard = workspace
            .try_enter_query()
            .ok_or(LanguageServiceError::QueryBusy { handle })?;
        let program = workspace.program.as_ref();

        // dispatch pure read query execution
        let response = self.execute_query_request(program, envelope.request)?;
        let revision = workspace.revision();

        Ok(QueryResponseEnvelope { revision, response })
    }

    /// Execute a write query envelope for the workspace that owns the path.
    pub fn execute_write_query_envelope_for_path(
        &self,
        path: &Path,
        envelope: QueryRequestEnvelope,
    ) -> Result<QueryResponseEnvelope, LanguageServiceError> {
        // resolve the workspace handle for this path
        let handle = self.workspace_handle_id_for_path(path)?;

        // route to handle based write query execution
        self.execute_write_query_envelope_for_workspace_handle(handle, envelope)
    }

    /// Execute a write query envelope for a specific workspace handle.
    pub fn execute_write_query_envelope_for_workspace_handle(
        &self,
        handle: WorkspaceHandleId,
        envelope: QueryRequestEnvelope,
    ) -> Result<QueryResponseEnvelope, LanguageServiceError> {
        // reject any non-write requests on the write path
        let request_mode = envelope.request.execution_mode();
        if request_mode != query::QueryExecutionMode::Write {
            return Err(LanguageServiceError::QueryExecutionModeMismatch {
                method: envelope.request.method_id(),
                expected: query::QueryExecutionMode::Write,
                actual: request_mode,
            });
        }

        // resolve the owning workspace handle and current revision
        let workspace = self.workspace_handle_for_id(handle)?;

        // serialize write queries with all other workspace mutations
        let _mutation_guard = workspace.enter_mutation();
        let program = workspace.program.as_ref();
        let current_revision = workspace.revision();

        // require a matching revision precondition for write requests
        let expected_revision = envelope
            .expected_revision
            .ok_or(LanguageServiceError::MissingExpectedRevision)?;
        if expected_revision != current_revision {
            return Err(LanguageServiceError::StaleRevision {
                expected: expected_revision,
                current: current_revision,
            });
        }

        // dispatch mutating query execution
        let response = self.execute_query_request(program, envelope.request)?;
        let revision = workspace.revision();

        Ok(QueryResponseEnvelope { revision, response })
    }

    /// Build a query response for a request payload.
    fn execute_query_request(
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
                        query::document_highlights(session, file_id, params.offset)
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
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
                        let file_id = self.ensure_semantic_query_ready(program, file_id)?;
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
            return Some(module.file_id);
        }

        // fall back to module lookups by path
        if let Some(path) = uri.to_path_buf()
            && let Some(module_id) = program.modules.get_id_by_path(&path)
        {
            let module = program.modules.get(module_id);
            return Some(module.file_id);
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
    ) -> Result<FileId, LanguageServiceError> {
        // return early when semantic query state is already ready
        if self.semantic_query_ready(program, file_id) {
            return Ok(file_id);
        }

        // read paths must fail loudly instead of running fallback mutation
        let session = self.session_ref();

        // build a detailed failure summary for diagnostics
        let detail = session
            .modules
            .get_by_file_id(file_id)
            .map(|module| {
                let module = module.as_ref();
                let profile_id = program.default_profile_id_for_module(module.id);
                let ast_ready = program.artifacts.ast(module.id).is_some();
                let base_dir_ready = program.artifacts.dir_base(module.id).is_some();
                let dir_ready = program.artifacts.dir_analyzed(module.id, profile_id).is_some();
                let path = module
                    .path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "<none>".to_string());
                format!(
                    "file_id={file_id:?} module_id={:?} profile_id={:?} ast_ready={ast_ready} base_dir_ready={base_dir_ready} dir_ready={dir_ready} path={path}",
                    module.id,
                    profile_id
                )
            })
            .unwrap_or_else(|| format!("file_id={file_id:?} module_id=<missing>"));

        Err(LanguageServiceError::SemanticQueryNotReady { detail })
    }

    /// Check if semantic query state is ready for the file.
    fn semantic_query_ready(&self, program: &Program, file_id: FileId) -> bool {
        // resolve the module for this file id
        let session = self.session_ref();
        let Some(module) = session.modules.get_by_file_id(file_id) else {
            return false;
        };

        // require both ast and profile dir state
        let module = module.as_ref();
        let profile = program.default_profile_id_for_module(module.id);
        program.artifacts.ast(module.id).is_some()
            && program.artifacts.dir_analyzed(module.id, profile).is_some()
    }

    /// Build a span from offsets for a file.
    fn span_for_offsets(&self, file_id: FileId, start: u32, end: u32) -> Span {
        // normalize offset order before building a span
        let range_start = start.min(end);
        let range_end = start.max(end);
        Span::new(file_id, range_start, range_end)
    }
}
