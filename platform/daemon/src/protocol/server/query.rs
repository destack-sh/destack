use std::collections::HashSet;

use destack_compiler::{AnalyzeTask, ResolveTask, TaskOutcome};
use destack_source::{FileId, Span, Uri};
use destack_workspace::{ModuleContent, Program, query};

use super::ProtocolServer;
use crate::protocol::{
    DaemonQuery, DaemonQueryResponse, DaemonResponse, ProtocolError, ProtocolErrorCode,
    WorkspaceHandleId,
};

impl ProtocolServer {
    /// Handle a query request.
    pub(super) fn handle_query(&self, query: DaemonQuery) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let response = match query {
            DaemonQuery::WorkspaceIndex { handle } => {
                let root = self.root_for_handle(handle)?;
                let payload = self.prepare_payload(self.workspace_index_payload(&root)?)?;
                DaemonQueryResponse::WorkspaceIndex(payload)
            }
            DaemonQuery::ModuleGraph { handle, profile } => {
                let root = self.root_for_handle(handle)?;
                let payload = self.prepare_payload(self.module_graph_payload(&root, profile)?)?;
                DaemonQueryResponse::ModuleGraph(payload)
            }
            DaemonQuery::ModuleSignature {
                handle,
                module_id,
                profile,
            } => {
                let root = self.root_for_handle(handle)?;
                let payload = self
                    .prepare_payload(self.module_signature_payload(&root, module_id, profile)?)?;
                DaemonQueryResponse::ModuleSignature(payload)
            }
            DaemonQuery::Diagnostics { handle } => {
                let root = self.root_for_handle(handle)?;
                let diagnostics = self.diagnostics_for_root(&root)?;
                DaemonQueryResponse::Diagnostics(diagnostics)
            }
            DaemonQuery::CacheStats { handle } => {
                let root = self.root_for_handle(handle)?;
                let stats = self.cache_stats_for_root(&root)?;
                DaemonQueryResponse::CacheStats(stats)
            }
            DaemonQuery::WorkspaceQuery { handle, request } => {
                let _root = self.root_for_handle(handle)?;
                let response = self.execute_workspace_query(handle, request)?;
                DaemonQueryResponse::WorkspaceQuery(response)
            }
            DaemonQuery::WorkspaceQueryBatch { handle, requests } => {
                let _root = self.root_for_handle(handle)?;
                let responses = requests
                    .into_iter()
                    .map(|request| self.execute_workspace_query(handle, request))
                    .collect::<Result<Vec<_>, ProtocolError>>()?;
                DaemonQueryResponse::WorkspaceQueryBatch(responses)
            }
        };

        Ok(DaemonResponse::QueryResult(response))
    }

    /// Execute a workspace query against the current session.
    fn execute_workspace_query(
        &self,
        handle: WorkspaceHandleId,
        request: query::QueryRequestEnvelope,
    ) -> Result<query::QueryResponseEnvelope, ProtocolError> {
        // resolve the workspace program
        let root = self.root_for_handle(handle)?;
        let program = self.program_for_root(&root)?;

        // build the response payload
        let response = self.query_response_for_request(
            &program,
            request.request,
            request.options.allow_stale,
        )?;

        // select the snapshot id for the response
        let snapshot_id = request
            .snapshot_id
            .unwrap_or_else(|| self.snapshot_id_for_handle(handle));

        Ok(query::QueryResponseEnvelope {
            snapshot_id,
            response,
        })
    }

    /// Build a query response for a request payload.
    fn query_response_for_request(
        &self,
        program: &Program,
        request: query::QueryRequest,
        allow_stale: bool,
    ) -> Result<query::QueryResponse, ProtocolError> {
        // resolve the session reference
        let session = self.daemon.session.as_ref();

        // dispatch the query request
        let response = session.with_query_context_mode(allow_stale, || {
            Ok(match request {
                query::QueryRequest::Completion(params) => {
                    // resolve the file id
                    let file_id = self.resolve_file_id(program, &params.uri);

                    // collect completion items when available
                    let mut items = match file_id {
                        Some(file_id) => {
                            let file_id = self.ensure_query_context(
                                program,
                                file_id,
                                &params.uri,
                                allow_stale,
                            )?;
                            query::completions(session, file_id, params.offset, params.trigger)
                        }
                        None => Vec::new(),
                    };

                    // drop auto import entries when disabled
                    if !params.include_imports {
                        items.retain(|item| item.additional_text_edits.is_empty());
                    }

                    query::QueryResponse::Completion(query::CompletionResponse {
                        items,
                        is_incomplete: false,
                    })
                }
                query::QueryRequest::Hover(params) => {
                    // resolve the file id
                    let file_id = self.resolve_file_id(program, &params.uri);

                    // compute hover info when available
                    let hover = match file_id {
                        Some(file_id) => {
                            let file_id = self.ensure_query_context(
                                program,
                                file_id,
                                &params.uri,
                                allow_stale,
                            )?;
                            query::hover(session, file_id, params.offset)
                        }
                        None => None,
                    };

                    query::QueryResponse::Hover(query::HoverResponse { hover })
                }
                query::QueryRequest::SignatureHelp(params) => {
                    // resolve the file id
                    let file_id = self.resolve_file_id(program, &params.uri);

                    // compute signature help when available
                    let help = match file_id {
                        Some(file_id) => {
                            let file_id = self.ensure_query_context(
                                program,
                                file_id,
                                &params.uri,
                                allow_stale,
                            )?;
                            query::signature_help(session, file_id, params.offset)
                        }
                        None => None,
                    };

                    query::QueryResponse::SignatureHelp(query::SignatureHelpResponse { help })
                }
                query::QueryRequest::InlayHints(params) => {
                    // resolve the file id
                    let file_id = self.resolve_file_id(program, &params.uri);

                    // compute inlay hints when available
                    let hints = match file_id {
                        Some(file_id) => {
                            let file_id = self.ensure_query_context(
                                program,
                                file_id,
                                &params.uri,
                                allow_stale,
                            )?;
                            let range = self.span_for_offsets(file_id, params.start, params.end);
                            query::inlay_hints(session, file_id, range)
                        }
                        None => Vec::new(),
                    };

                    query::QueryResponse::InlayHints(query::InlayHintsResponse { hints })
                }
                query::QueryRequest::CodeLenses(params) => {
                    // resolve the file id
                    let file_id = self.resolve_file_id(program, &params.uri);

                    // compute code lenses when available
                    let lenses = match file_id {
                        Some(file_id) => {
                            let file_id = self.ensure_query_context(
                                program,
                                file_id,
                                &params.uri,
                                allow_stale,
                            )?;
                            query::code_lenses(session, file_id)
                        }
                        None => Vec::new(),
                    };

                    query::QueryResponse::CodeLenses(query::CodeLensesResponse { lenses })
                }
                query::QueryRequest::ResolveCodeLens(params) => {
                    // resolve the code lens
                    let lens = query::resolve_code_lens(session, &params.lens);

                    query::QueryResponse::ResolveCodeLens(query::ResolveCodeLensResponse { lens })
                }
                query::QueryRequest::FoldingRanges(params) => {
                    // resolve the file id
                    let file_id = self.resolve_file_id(program, &params.uri);

                    // compute folding ranges when available
                    let ranges = match file_id {
                        Some(file_id) => {
                            let file_id = self.ensure_query_context(
                                program,
                                file_id,
                                &params.uri,
                                allow_stale,
                            )?;
                            query::folding_ranges(session, file_id)
                        }
                        None => Vec::new(),
                    };

                    query::QueryResponse::FoldingRanges(query::FoldingRangesResponse { ranges })
                }
                query::QueryRequest::SemanticTokens(params) => {
                    // resolve the file id
                    let file_id = self.resolve_file_id(program, &params.uri);

                    // compute semantic tokens when available
                    let tokens = match file_id {
                        Some(file_id) => {
                            let file_id = self.ensure_query_context(
                                program,
                                file_id,
                                &params.uri,
                                allow_stale,
                            )?;
                            query::semantic_tokens(session, file_id)
                        }
                        None => Vec::new(),
                    };

                    query::QueryResponse::SemanticTokens(query::SemanticTokensResponse { tokens })
                }
                query::QueryRequest::SemanticTokensRange(params) => {
                    // resolve the file id
                    let file_id = self.resolve_file_id(program, &params.uri);

                    // compute semantic tokens when available
                    let tokens = match file_id {
                        Some(file_id) => {
                            let file_id = self.ensure_query_context(
                                program,
                                file_id,
                                &params.uri,
                                allow_stale,
                            )?;
                            let range = self.span_for_offsets(file_id, params.start, params.end);
                            query::semantic_tokens_range(session, file_id, range)
                        }
                        None => Vec::new(),
                    };

                    query::QueryResponse::SemanticTokensRange(query::SemanticTokensResponse {
                        tokens,
                    })
                }
                query::QueryRequest::DocumentSymbols(params) => {
                    // resolve the file id
                    let file_id = self.resolve_file_id(program, &params.uri);

                    // compute document symbols when available
                    let symbols = match file_id {
                        Some(file_id) => {
                            let file_id = self.ensure_query_context(
                                program,
                                file_id,
                                &params.uri,
                                allow_stale,
                            )?;
                            query::document_symbols(session, file_id)
                        }
                        None => Vec::new(),
                    };

                    query::QueryResponse::DocumentSymbols(query::DocumentSymbolsResponse {
                        symbols,
                    })
                }
                query::QueryRequest::WorkspaceSymbols(params) => {
                    // compute workspace symbols
                    let symbols = query::workspace_symbols(
                        session,
                        &params.query,
                        params.max_results as usize,
                    );

                    query::QueryResponse::WorkspaceSymbols(query::WorkspaceSymbolsResponse {
                        symbols,
                    })
                }
                query::QueryRequest::DocumentLinks(params) => {
                    // resolve the file id
                    let file_id = self.resolve_file_id(program, &params.uri);

                    // compute document links when available
                    let links = match file_id {
                        Some(file_id) => {
                            let file_id = self.ensure_query_context(
                                program,
                                file_id,
                                &params.uri,
                                allow_stale,
                            )?;
                            query::document_links(session, file_id)
                        }
                        None => Vec::new(),
                    };

                    query::QueryResponse::DocumentLinks(query::DocumentLinksResponse { links })
                }
                query::QueryRequest::ResolveDocumentLink(params) => {
                    // resolve the document link
                    let link = query::resolve_document_link(session, &params.link);

                    query::QueryResponse::ResolveDocumentLink(query::ResolveDocumentLinkResponse {
                        link,
                    })
                }
                query::QueryRequest::DocumentHighlight(params) => {
                    // resolve the file id
                    let file_id = self.resolve_file_id(program, &params.uri);

                    // compute document highlights when available
                    let highlights = match file_id {
                        Some(file_id) => {
                            let file_id = self.ensure_query_context(
                                program,
                                file_id,
                                &params.uri,
                                allow_stale,
                            )?;
                            query::document_highlight(session, file_id, params.offset)
                        }
                        None => Vec::new(),
                    };

                    query::QueryResponse::DocumentHighlight(query::DocumentHighlightResponse {
                        highlights,
                    })
                }
                query::QueryRequest::SelectionRanges(params) => {
                    // resolve the file id
                    let file_id = self.resolve_file_id(program, &params.uri);

                    // compute selection ranges when available
                    let ranges = match file_id {
                        Some(file_id) => {
                            let file_id = self.ensure_query_context(
                                program,
                                file_id,
                                &params.uri,
                                allow_stale,
                            )?;
                            query::selection_ranges(session, file_id, &params.offsets)
                        }
                        None => Vec::new(),
                    };

                    query::QueryResponse::SelectionRanges(query::SelectionRangesResponse { ranges })
                }
                query::QueryRequest::GotoDefinition(params) => {
                    // resolve the file id
                    let file_id = self.resolve_file_id(program, &params.uri);

                    // compute definition when available
                    let result = match file_id {
                        Some(file_id) => {
                            let file_id = self.ensure_query_context(
                                program,
                                file_id,
                                &params.uri,
                                allow_stale,
                            )?;
                            query::goto_definition(session, file_id, params.offset)
                        }
                        None => None,
                    };

                    query::QueryResponse::GotoDefinition(query::GotoDefinitionResponse { result })
                }
                query::QueryRequest::GotoDeclaration(params) => {
                    // resolve the file id
                    let file_id = self.resolve_file_id(program, &params.uri);

                    // compute declaration when available
                    let result = match file_id {
                        Some(file_id) => {
                            let file_id = self.ensure_query_context(
                                program,
                                file_id,
                                &params.uri,
                                allow_stale,
                            )?;
                            query::goto_declaration(session, file_id, params.offset)
                        }
                        None => None,
                    };

                    query::QueryResponse::GotoDeclaration(query::GotoDeclarationResponse { result })
                }
                query::QueryRequest::GotoTypeDefinition(params) => {
                    // resolve the file id
                    let file_id = self.resolve_file_id(program, &params.uri);

                    // compute type definition when available
                    let result = match file_id {
                        Some(file_id) => {
                            let file_id = self.ensure_query_context(
                                program,
                                file_id,
                                &params.uri,
                                allow_stale,
                            )?;
                            query::goto_type_definition(session, file_id, params.offset)
                        }
                        None => None,
                    };

                    query::QueryResponse::GotoTypeDefinition(query::GotoTypeDefinitionResponse {
                        result,
                    })
                }
                query::QueryRequest::GotoImplementation(params) => {
                    // resolve the file id
                    let file_id = self.resolve_file_id(program, &params.uri);

                    // compute implementations when available
                    let result = match file_id {
                        Some(file_id) => {
                            let file_id = self.ensure_query_context(
                                program,
                                file_id,
                                &params.uri,
                                allow_stale,
                            )?;
                            query::goto_implementation(session, file_id, params.offset)
                        }
                        None => None,
                    };

                    query::QueryResponse::GotoImplementation(query::GotoImplementationResponse {
                        result,
                    })
                }
                query::QueryRequest::FindReferences(params) => {
                    // resolve the file id
                    let file_id = self.resolve_file_id(program, &params.uri);

                    // compute references when available
                    let result = match file_id {
                        Some(file_id) => {
                            let file_id = self.ensure_query_context(
                                program,
                                file_id,
                                &params.uri,
                                allow_stale,
                            )?;
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
                    // resolve the file id
                    let file_id = self.resolve_file_id(program, &params.uri);

                    // compute call hierarchy item when available
                    let item = match file_id {
                        Some(file_id) => {
                            let file_id = self.ensure_query_context(
                                program,
                                file_id,
                                &params.uri,
                                allow_stale,
                            )?;
                            query::prepare_call_hierarchy(session, file_id, params.offset)
                        }
                        None => None,
                    };

                    query::QueryResponse::PrepareCallHierarchy(
                        query::PrepareCallHierarchyResponse { item },
                    )
                }
                query::QueryRequest::CallHierarchyIncoming(params) => {
                    // compute incoming calls
                    let calls = query::incoming_calls(session, &params.item);

                    query::QueryResponse::CallHierarchyIncoming(
                        query::CallHierarchyIncomingResponse { calls },
                    )
                }
                query::QueryRequest::CallHierarchyOutgoing(params) => {
                    // compute outgoing calls
                    let calls = query::outgoing_calls(session, &params.item);

                    query::QueryResponse::CallHierarchyOutgoing(
                        query::CallHierarchyOutgoingResponse { calls },
                    )
                }
                query::QueryRequest::PrepareTypeHierarchy(params) => {
                    // resolve the file id
                    let file_id = self.resolve_file_id(program, &params.uri);

                    // compute type hierarchy item when available
                    let item = match file_id {
                        Some(file_id) => {
                            let file_id = self.ensure_query_context(
                                program,
                                file_id,
                                &params.uri,
                                allow_stale,
                            )?;
                            query::prepare_type_hierarchy(session, file_id, params.offset)
                        }
                        None => None,
                    };

                    query::QueryResponse::PrepareTypeHierarchy(
                        query::PrepareTypeHierarchyResponse { item },
                    )
                }
                query::QueryRequest::TypeHierarchySupertypes(params) => {
                    // compute type hierarchy supertypes
                    let items = query::supertypes(session, &params.item);

                    query::QueryResponse::TypeHierarchySupertypes(
                        query::TypeHierarchySupertypesResponse { items },
                    )
                }
                query::QueryRequest::TypeHierarchySubtypes(params) => {
                    // compute type hierarchy subtypes
                    let items = query::subtypes(session, &params.item);

                    query::QueryResponse::TypeHierarchySubtypes(
                        query::TypeHierarchySubtypesResponse { items },
                    )
                }
                query::QueryRequest::PrepareRename(params) => {
                    // resolve the file id
                    let file_id = self.resolve_file_id(program, &params.uri);

                    // compute prepare rename when available
                    let result = match file_id {
                        Some(file_id) => {
                            let file_id = self.ensure_query_context(
                                program,
                                file_id,
                                &params.uri,
                                allow_stale,
                            )?;
                            query::prepare_rename(session, file_id, params.offset)
                        }
                        None => None,
                    };

                    query::QueryResponse::PrepareRename(query::PrepareRenameResponse { result })
                }
                query::QueryRequest::Rename(params) => {
                    // resolve the file id
                    let file_id = self.resolve_file_id(program, &params.uri);

                    // compute rename when available
                    let result = match file_id {
                        Some(file_id) => {
                            let file_id = self.ensure_query_context(
                                program,
                                file_id,
                                &params.uri,
                                allow_stale,
                            )?;
                            query::rename(session, file_id, params.offset, &params.new_name)
                        }
                        None => None,
                    };

                    query::QueryResponse::Rename(query::RenameResponse { result })
                }
                query::QueryRequest::CodeActions(params) => {
                    // resolve the file id
                    let file_id = self.resolve_file_id(program, &params.uri);

                    // compute code actions when available
                    let actions = match file_id {
                        Some(file_id) => {
                            let file_id = self.ensure_query_context(
                                program,
                                file_id,
                                &params.uri,
                                allow_stale,
                            )?;
                            let range = self.span_for_offsets(file_id, params.start, params.end);
                            query::code_actions(session, file_id, range, &params.context)
                        }
                        None => Vec::new(),
                    };

                    query::QueryResponse::CodeActions(query::CodeActionsResponse { actions })
                }
            })
        })?;

        Ok(response)
    }

    /// Resolve a file id for a query uri.
    fn resolve_file_id(&self, program: &Program, uri: &Uri) -> Option<FileId> {
        // prefer module lookups so we get the correct file id for code modules
        if let Some(module_id) = program.modules.get_id_by_uri(uri) {
            let module = program.modules.get(module_id);
            return Some(module.read().file_id);
        }

        if let Some(path) = uri.to_path_buf()
            && let Some(module_id) = program.modules.get_id_by_path(&path)
        {
            let module = program.modules.get(module_id);
            return Some(module.read().file_id);
        }

        // fall back to the file registry
        if let Some(file_id) = program.files.get_id_by_uri(uri) {
            return Some(file_id);
        }

        let path = uri.to_path_buf()?;
        program.files.get_id_by_path(&path)
    }

    /// Ensure query context is ready for a file unless stale results are allowed.
    fn ensure_query_context(
        &self,
        program: &Program,
        file_id: FileId,
        uri: &Uri,
        allow_stale: bool,
    ) -> Result<FileId, ProtocolError> {
        if allow_stale || self.query_context_ready(file_id) {
            return Ok(file_id);
        }

        let mut builtins_outcome = None;
        let mut libs_outcome = None;
        let mut prepare_outcome = None;
        let mut direct_outcome = None;
        let mut canonical_outcome = None;
        let mut analyze_outcome = None;

        // attempt to force analysis for the resolved module id
        let session = self.daemon.session.as_ref();
        if let Some(module_id) = session.modules.get_id_by_file_id(file_id) {
            let profile_id = session.default_profile_for_module(module_id);
            let handle = self.daemon.program_handle_for_path(&program.cwd);
            let _compile_guard = handle.compile_lock.lock();

            let module = handle.compiler.module_stamp(module_id);
            let profile = handle.compiler.profile_stamp(profile_id);
            let graph = handle.compiler.module_graph_stamp(profile_id);

            let builtins_task = ResolveTask::ResolveBuiltins { profile };
            let libs_task = ResolveTask::ResolveLibs { profile };
            let prepare_task = ResolveTask::ResolveModulePrepare { module, profile };
            let direct_task = ResolveTask::ResolveModuleDirect { module, profile };
            let canonical_task = ResolveTask::ResolveModuleCanonical {
                module,
                profile,
                graph,
            };
            let analyze_task = AnalyzeTask::AnalyzeModuleValidate { module, profile };

            handle.compiler.enqueue(builtins_task.clone());
            handle.compiler.enqueue(libs_task.clone());
            handle.compiler.enqueue(prepare_task.clone());
            handle.compiler.enqueue(direct_task.clone());
            handle.compiler.enqueue(canonical_task.clone());
            handle.compiler.enqueue(analyze_task.clone());
            if let Some(builtins) = program.builtins.as_ref() {
                let mut builtin_module_ids = HashSet::new();
                builtin_module_ids.insert(builtins.prelude_module_id);
                for module_id in builtins.core_module_by_path.values() {
                    builtin_module_ids.insert(*module_id);
                }
                for module_id in builtin_module_ids {
                    let module = handle.compiler.module_stamp(module_id);
                    let profile = handle.compiler.profile_stamp(profile_id);
                    let graph = handle.compiler.module_graph_stamp(profile_id);
                    handle.compiler.enqueue(ResolveTask::ResolveModule {
                        module,
                        profile,
                        graph,
                    });
                    handle
                        .compiler
                        .enqueue(ResolveTask::ResolveModulePrepare { module, profile });
                    handle
                        .compiler
                        .enqueue(ResolveTask::ResolveModuleDirect { module, profile });
                    handle
                        .compiler
                        .enqueue(ResolveTask::ResolveModuleCanonical {
                            module,
                            profile,
                            graph,
                        });
                }
            }

            let mut passes = 0;
            loop {
                passes += 1;
                handle.compiler.compile();

                builtins_outcome = handle.compiler.get_outcome(builtins_task.clone());
                libs_outcome = handle.compiler.get_outcome(libs_task.clone());
                prepare_outcome = handle.compiler.get_outcome(prepare_task.clone());
                direct_outcome = handle.compiler.get_outcome(direct_task.clone());
                canonical_outcome = handle.compiler.get_outcome(canonical_task.clone());
                analyze_outcome = handle.compiler.get_outcome(analyze_task.clone());

                let needs_retry = [
                    builtins_outcome.as_ref(),
                    libs_outcome.as_ref(),
                    prepare_outcome.as_ref(),
                    direct_outcome.as_ref(),
                    canonical_outcome.as_ref(),
                    analyze_outcome.as_ref(),
                ]
                .into_iter()
                .any(|outcome| matches!(outcome, Some(TaskOutcome::Yield { .. })));
                if !needs_retry || passes >= 3 {
                    break;
                }
            }
            if self.query_context_ready(file_id) {
                return Ok(file_id);
            }
        }

        let path = program
            .files
            .get_maybe(file_id)
            .and_then(|file| file.path.clone().or_else(|| file.uri.to_path_buf()))
            .or_else(|| uri.to_path_buf());

        let Some(path) = path else {
            return Err(self.protocol_error(
                ProtocolErrorCode::NotReady,
                "query context is not ready for the requested uri",
            ));
        };

        if let Err(error) = self.daemon.analyze_path(&path) {
            return Err(self.protocol_error_from_daemon(error));
        }

        if self.query_context_ready(file_id) {
            return Ok(file_id);
        }

        if let Some(resolved_file_id) = self.resolve_file_id(program, uri)
            && self.query_context_ready(resolved_file_id)
        {
            return Ok(resolved_file_id);
        }

        // provide context in error details for debugging readiness gaps
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
                    ModuleContent::Code(code) => code
                        .dirs
                        .iter()
                        .filter_map(|dir| dir.profile_id)
                        .collect(),
                    ModuleContent::Data { dirs, .. }
                    | ModuleContent::Text { dirs, .. }
                    | ModuleContent::Binary { dirs, .. } => dirs
                        .iter()
                        .filter_map(|dir| dir.profile_id)
                        .collect(),
                    ModuleContent::Unloaded => Vec::new(),
                };
                let path = module
                    .path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "<none>".to_string());
                format!(
                    "file_id={file_id:?} module_id={:?} profile_id={:?} ast_ready={ast_ready} base_dir_ready={base_dir_ready} dir_ready={dir_ready} dir_profiles={dir_profiles:?} builtins_outcome={builtins_outcome:?} libs_outcome={libs_outcome:?} prepare_outcome={prepare_outcome:?} direct_outcome={direct_outcome:?} canonical_outcome={canonical_outcome:?} analyze_outcome={analyze_outcome:?} path={path}",
                    module.id,
                    profile_id
                )
            })
            .or_else(|| Some(format!("file_id={file_id:?} module_id=<missing>")));

        Err(ProtocolError {
            code: ProtocolErrorCode::NotReady,
            message: "analysis did not produce a query context".to_string(),
            detail,
            retryable: false,
            retry_after_ms: None,
        })
    }

    /// Check if the query context is ready for the given file id.
    fn query_context_ready(&self, file_id: FileId) -> bool {
        // resolve the module for the file
        let session = self.daemon.session.as_ref();
        let Some(module) = session.modules.get_by_file_id(file_id) else {
            return false;
        };

        // verify AST and DIR are available for the default profile
        let module = module.read();
        let profile = session.default_profile_for_module(module.id);
        module.ast_maybe().is_some() && module.dir_maybe(profile).is_some()
    }

    /// Build a span from offsets for a file.
    fn span_for_offsets(&self, file_id: FileId, start: u32, end: u32) -> Span {
        // normalize the offset order
        let range_start = start.min(end);
        let range_end = start.max(end);

        Span::new(file_id, range_start, range_end)
    }

    /// Build a snapshot identifier for a workspace handle.
    fn snapshot_id_for_handle(&self, handle: WorkspaceHandleId) -> String {
        // format a handle based snapshot id
        format!("handle:{}", handle.0)
    }
}
