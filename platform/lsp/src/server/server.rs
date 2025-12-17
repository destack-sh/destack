use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use dashmap::DashMap;
use destack_compiler::{AnalyzeTask, Compiler, CompilerOptions};
use destack_source::{FileId, FileSystem, FileType, OverlayFileSystem, PhysicalFileSystem, Uri};
use destack_workspace::{Session, query};
use tower_lsp_server::{Client, LanguageServer, UriExt, jsonrpc, lsp_types as lsp};

use crate::query::common::{byte_span_to_range, position_to_byte, span_to_location};
use crate::query::navigation::{
    definition_to_location, document_highlight_to_lsp, document_symbol_to_lsp,
};
use crate::query::semantic;

pub const TRACKED_FILE_TYPES: [FileType; 8] = [
    FileType::Destack,
    FileType::DestackText,
    FileType::DestackBinary,
    FileType::JavaScript,
    FileType::JavaScriptXml,
    FileType::TypeScript,
    FileType::TypeScriptXml,
    FileType::TypeScriptDeclaration,
];

/// State for an open document.
#[derive(Debug)]
struct OpenDocument {
    file_id: FileId,
}

/// The Destack language server.
#[derive(Debug)]
pub struct DestackLanguageServer {
    pub(super) client: Client,
    overlay_fs: Arc<OverlayFileSystem>,
    session: OnceLock<Arc<Session>>,
    open_documents: DashMap<String, OpenDocument>,
}

impl DestackLanguageServer {
    /// Create a new language server instance.
    pub fn new(client: Client) -> Self {
        let physical_fs = Arc::new(PhysicalFileSystem::new());
        let overlay_fs = Arc::new(OverlayFileSystem::with_inner(physical_fs));
        Self {
            client,
            overlay_fs,
            session: OnceLock::new(),
            open_documents: DashMap::new(),
        }
    }

    /// Get the session (must be called after initialize).
    fn session(&self) -> &Arc<Session> {
        self.session.get().expect("session not initialized")
    }

    /// Invalidate a module at the given path.
    ///
    /// nocheckin TODO #Incomplete: incremental recompilation
    fn invalidate_path(&self, path: &std::path::Path) {
        let session = self.session().clone();
        let program = session.find_program_for_path(path);

        // create compiler with existing session (reuses all state...?)
        let compiler = Compiler::new(
            session,
            program.clone(),
            CompilerOptions {
                workers: 1,
                ..Default::default()
            },
        );

        // resolve path to module and enqueue analysis
        let Ok(module_id) = compiler.resolve_path_to_module(&path.to_path_buf()) else {
            return;
        };
        compiler.enqueue(AnalyzeTask::AnalyzeModuleValidate { module: module_id });
        compiler.compile();
    }
}

// ----------------------------------------------------------------------------
// LIFECYCLE
// ----------------------------------------------------------------------------

impl LanguageServer for DestackLanguageServer {
    async fn initialize(
        &self,
        params: lsp::InitializeParams,
    ) -> jsonrpc::Result<lsp::InitializeResult> {
        self.client
            .log_message(lsp::MessageType::INFO, "destack.initialize.start")
            .await;

        // determine workspace root from params
        #[allow(deprecated)]
        let cwd = params
            .root_uri
            .as_ref()
            .and_then(|uri| uri.to_file_path().map(|p| p.into_owned()))
            .or_else(|| params.root_path.clone().map(PathBuf::from))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        // create session with overlay filesystem
        let session = Arc::new(Session::new(cwd.clone()).with_fs(self.overlay_fs.clone()));
        session.add_root(cwd);
        let _ = self.session.set(session);

        // build file operation filters for workspace notifications
        let file_operation_filters: Vec<lsp::FileOperationFilter> = TRACKED_FILE_TYPES
            .iter()
            .map(|file_type| lsp::FileOperationFilter {
                scheme: None,
                pattern: lsp::FileOperationPattern {
                    glob: file_type.glob().expect("unknown file type").to_string(),
                    matches: Some(lsp::FileOperationPatternKind::File),
                    options: Some(lsp::FileOperationPatternOptions {
                        ignore_case: Some(true),
                    }),
                },
            })
            .collect();

        // declare server capabilities
        let capabilities = lsp::ServerCapabilities {
            text_document_sync: Some(lsp::TextDocumentSyncCapability::Kind(
                lsp::TextDocumentSyncKind::FULL,
            )),
            hover_provider: Some(lsp::HoverProviderCapability::Simple(true)),
            definition_provider: Some(lsp::OneOf::Left(true)),
            declaration_provider: Some(lsp::DeclarationCapability::Simple(true)),
            type_definition_provider: Some(lsp::TypeDefinitionProviderCapability::Simple(true)),
            references_provider: Some(lsp::OneOf::Left(true)),
            document_symbol_provider: Some(lsp::OneOf::Left(true)),
            document_highlight_provider: Some(lsp::OneOf::Left(true)),
            completion_provider: Some(lsp::CompletionOptions {
                trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
                ..Default::default()
            }),
            signature_help_provider: Some(lsp::SignatureHelpOptions {
                trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                retrigger_characters: None,
                work_done_progress_options: Default::default(),
            }),
            semantic_tokens_provider: Some(
                lsp::SemanticTokensServerCapabilities::SemanticTokensOptions(
                    lsp::SemanticTokensOptions {
                        legend: semantic::legend(),
                        range: Some(true),
                        full: Some(lsp::SemanticTokensFullOptions::Bool(true)),
                        work_done_progress_options: lsp::WorkDoneProgressOptions::default(),
                    },
                ),
            ),
            document_formatting_provider: Some(lsp::OneOf::Left(true)),
            folding_range_provider: Some(lsp::FoldingRangeProviderCapability::Simple(true)),
            workspace: Some(lsp::WorkspaceServerCapabilities {
                workspace_folders: Some(lsp::WorkspaceFoldersServerCapabilities {
                    supported: Some(true),
                    change_notifications: Some(lsp::OneOf::Left(true)),
                }),
                file_operations: Some(lsp::WorkspaceFileOperationsServerCapabilities {
                    did_create: Some(lsp::FileOperationRegistrationOptions {
                        filters: file_operation_filters.clone(),
                    }),
                    did_rename: Some(lsp::FileOperationRegistrationOptions {
                        filters: file_operation_filters.clone(),
                    }),
                    did_delete: Some(lsp::FileOperationRegistrationOptions {
                        filters: file_operation_filters.clone(),
                    }),
                    will_create: None,
                    will_rename: None,
                    will_delete: None,
                }),
            }),
            ..Default::default()
        };

        self.client
            .log_message(lsp::MessageType::INFO, "destack.initialize.done")
            .await;

        Ok(lsp::InitializeResult {
            capabilities,
            server_info: Some(lsp::ServerInfo {
                name: "destack".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: lsp::InitializedParams) {
        self.client
            .log_message(lsp::MessageType::INFO, "destack.initialized")
            .await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        self.client
            .log_message(lsp::MessageType::INFO, "destack.shutdown")
            .await;
        Ok(())
    }

    // ------------------------------------------------------------------------
    // SYNCHRONIZATION
    // ------------------------------------------------------------------------

    async fn did_open(&self, params: lsp::DidOpenTextDocumentParams) {
        let uri_str = params.text_document.uri.to_string();
        let content = params.text_document.text;

        self.client
            .log_message(lsp::MessageType::INFO, format!("did_open: {uri_str}"))
            .await;

        let session = self.session();

        let Some(path) = params
            .text_document
            .uri
            .to_file_path()
            .map(|p| p.into_owned())
        else {
            return;
        };

        // set overlay so subsequent reads use editor content
        self.overlay_fs.set_overlay(&path, content.clone());

        // register module with inline content
        let uri = Uri::from_path(&path);
        let program = session.find_program_for_path(&path);
        let module_id = program.register_inline_module(uri, content, FileType::Destack);

        // track open document
        let module = session.modules.get(module_id);
        let file_id = module.read().file_id;
        self.open_documents
            .insert(uri_str, OpenDocument { file_id });

        // invalidate the module
        self.invalidate_path(&path);
    }

    async fn did_change(&self, params: lsp::DidChangeTextDocumentParams) {
        // full sync: we receive entire new content
        let Some(change) = params.content_changes.into_iter().next() else {
            return;
        };
        let content = change.text;

        let Some(path) = params
            .text_document
            .uri
            .to_file_path()
            .map(|p| p.into_owned())
        else {
            return;
        };

        // update overlay with new content
        self.overlay_fs.set_overlay(&path, content);

        // invalidate the module
        self.invalidate_path(&path);
    }

    async fn did_close(&self, params: lsp::DidCloseTextDocumentParams) {
        let uri_str = params.text_document.uri.to_string();
        self.open_documents.remove(&uri_str);

        // remove overlay to fall back to disk content
        if let Some(path) = params
            .text_document
            .uri
            .to_file_path()
            .map(|p| p.into_owned())
        {
            self.overlay_fs.remove_overlay(&path);
        }
    }

    async fn did_change_workspace_folders(&self, _params: lsp::DidChangeWorkspaceFoldersParams) {
        // TODO #Incomplete: handle workspace folder changes
    }

    async fn did_change_watched_files(&self, _params: lsp::DidChangeWatchedFilesParams) {
        // TODO #Incomplete: handle external file changes
    }

    async fn did_create_files(&self, _params: lsp::CreateFilesParams) {
        // TODO #Incomplete: handle file creation
    }

    async fn did_rename_files(&self, _params: lsp::RenameFilesParams) {
        // TODO #Incomplete: handle file renames
    }

    async fn did_delete_files(&self, _params: lsp::DeleteFilesParams) {
        // TODO #Incomplete: handle file deletion
    }

    // ------------------------------------------------------------------------
    // NAVIGATION
    // ------------------------------------------------------------------------

    async fn goto_definition(
        &self,
        params: lsp::GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<lsp::GotoDefinitionResponse>> {
        // resolve file and position
        let uri_str = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = session.files.get(doc.file_id);
        let Some(offset) = position_to_byte(&file, &params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        // query for definition
        let Some(result) = query::goto_definition(session, doc.file_id, offset) else {
            return Ok(None);
        };

        // convert to LSP location
        let location = definition_to_location(session, &result);
        Ok(location.map(lsp::GotoDefinitionResponse::Scalar))
    }

    async fn goto_declaration(
        &self,
        params: lsp::request::GotoDeclarationParams,
    ) -> jsonrpc::Result<Option<lsp::request::GotoDeclarationResponse>> {
        // resolve file and position
        let uri_str = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = session.files.get(doc.file_id);
        let Some(offset) = position_to_byte(&file, &params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        // query for declaration (same as definition for now)
        let Some(result) = query::goto_declaration(session, doc.file_id, offset) else {
            return Ok(None);
        };

        let location = definition_to_location(session, &result);
        Ok(location.map(lsp::GotoDefinitionResponse::Scalar))
    }

    async fn goto_type_definition(
        &self,
        params: lsp::request::GotoTypeDefinitionParams,
    ) -> jsonrpc::Result<Option<lsp::request::GotoTypeDefinitionResponse>> {
        // resolve file and position
        let uri_str = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = session.files.get(doc.file_id);
        let Some(offset) = position_to_byte(&file, &params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        // query for type definition
        let Some(result) = query::goto_type_definition(session, doc.file_id, offset) else {
            return Ok(None);
        };

        let location = definition_to_location(session, &result);
        Ok(location.map(lsp::GotoDefinitionResponse::Scalar))
    }

    async fn references(
        &self,
        params: lsp::ReferenceParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::Location>>> {
        // resolve file and position
        let uri_str = params.text_document_position.text_document.uri.to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = session.files.get(doc.file_id);
        let Some(offset) = position_to_byte(&file, &params.text_document_position.position) else {
            return Ok(None);
        };

        // query for references (include declaration in results)
        let Some(refs) = query::find_references(session, doc.file_id, offset, true) else {
            return Ok(None);
        };
        if refs.is_empty() {
            return Ok(None);
        }

        // convert to LSP locations
        let locations: Vec<lsp::Location> = refs
            .references
            .iter()
            .filter_map(|span| span_to_location(session, *span))
            .collect();

        if locations.is_empty() {
            Ok(None)
        } else {
            Ok(Some(locations))
        }
    }

    async fn document_symbol(
        &self,
        params: lsp::DocumentSymbolParams,
    ) -> jsonrpc::Result<Option<lsp::DocumentSymbolResponse>> {
        // resolve file
        let uri_str = params.text_document.uri.to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };

        // query for document symbols
        let symbols = query::document_symbols(session, doc.file_id);
        if symbols.is_empty() {
            return Ok(None);
        }

        // convert to LSP symbols
        let file = session.files.get(doc.file_id);
        let lsp_symbols: Vec<lsp::DocumentSymbol> = symbols
            .iter()
            .filter_map(|s| document_symbol_to_lsp(&file, s))
            .collect();

        if lsp_symbols.is_empty() {
            Ok(None)
        } else {
            Ok(Some(lsp::DocumentSymbolResponse::Nested(lsp_symbols)))
        }
    }

    async fn document_highlight(
        &self,
        params: lsp::DocumentHighlightParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::DocumentHighlight>>> {
        // resolve file and position
        let uri_str = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = session.files.get(doc.file_id);
        let Some(offset) = position_to_byte(&file, &params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        // query for highlights
        let highlights = query::document_highlight(session, doc.file_id, offset);
        if highlights.is_empty() {
            return Ok(None);
        }

        // convert to LSP highlights
        let lsp_highlights: Vec<lsp::DocumentHighlight> = highlights
            .iter()
            .filter_map(|h| document_highlight_to_lsp(&file, h))
            .collect();

        if lsp_highlights.is_empty() {
            Ok(None)
        } else {
            Ok(Some(lsp_highlights))
        }
    }

    // ------------------------------------------------------------------------
    // ASSIST
    // ------------------------------------------------------------------------

    async fn hover(&self, params: lsp::HoverParams) -> jsonrpc::Result<Option<lsp::Hover>> {
        // resolve file and position
        let uri_str = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = session.files.get(doc.file_id);
        let Some(offset) = position_to_byte(&file, &params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        // query for hover info
        let Some(hover_info) = query::hover(session, doc.file_id, offset) else {
            return Ok(None);
        };

        let range = hover_info.range.map(|span| byte_span_to_range(&file, span));

        Ok(Some(lsp::Hover {
            contents: lsp::HoverContents::Markup(lsp::MarkupContent {
                kind: lsp::MarkupKind::Markdown,
                value: hover_info.to_markdown(),
            }),
            range,
        }))
    }

    async fn completion(
        &self,
        params: lsp::CompletionParams,
    ) -> jsonrpc::Result<Option<lsp::CompletionResponse>> {
        // resolve file and position
        let uri_str = params.text_document_position.text_document.uri.to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = session.files.get(doc.file_id);
        let Some(offset) = position_to_byte(&file, &params.text_document_position.position) else {
            return Ok(None);
        };

        // query for completions
        let trigger = query::CompletionTrigger::Invoked;
        let completions = query::completions(session, doc.file_id, offset, trigger);
        if completions.is_empty() {
            return Ok(None);
        }

        // convert to LSP completion items
        let items: Vec<lsp::CompletionItem> = completions
            .into_iter()
            .map(|c| lsp::CompletionItem {
                label: c.label,
                kind: Some(completion_kind_to_lsp(c.kind)),
                detail: c.detail,
                documentation: c.documentation.map(|d| {
                    lsp::Documentation::MarkupContent(lsp::MarkupContent {
                        kind: lsp::MarkupKind::Markdown,
                        value: d,
                    })
                }),
                insert_text: c.insert_text,
                ..Default::default()
            })
            .collect();

        Ok(Some(lsp::CompletionResponse::Array(items)))
    }

    async fn signature_help(
        &self,
        params: lsp::SignatureHelpParams,
    ) -> jsonrpc::Result<Option<lsp::SignatureHelp>> {
        // resolve file and position
        let uri_str = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let file = session.files.get(doc.file_id);
        let Some(offset) = position_to_byte(&file, &params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        // query for signature help
        let Some(help) = query::signature_help(session, doc.file_id, offset) else {
            return Ok(None);
        };

        // convert to LSP signature help
        let signatures: Vec<lsp::SignatureInformation> = help
            .signatures
            .into_iter()
            .map(|s| lsp::SignatureInformation {
                label: s.label,
                documentation: s.documentation.map(|d| {
                    lsp::Documentation::MarkupContent(lsp::MarkupContent {
                        kind: lsp::MarkupKind::Markdown,
                        value: d,
                    })
                }),
                parameters: Some(
                    s.parameters
                        .into_iter()
                        .map(|p| lsp::ParameterInformation {
                            label: lsp::ParameterLabel::Simple(p.label),
                            documentation: p.documentation.map(|d| {
                                lsp::Documentation::MarkupContent(lsp::MarkupContent {
                                    kind: lsp::MarkupKind::Markdown,
                                    value: d,
                                })
                            }),
                        })
                        .collect(),
                ),
                // note: active_parameter is on SignatureHelp, not per-signature
                active_parameter: None,
            })
            .collect();

        Ok(Some(lsp::SignatureHelp {
            signatures,
            active_signature: Some(help.active_signature as u32),
            active_parameter: Some(help.active_parameter as u32),
        }))
    }

    // ------------------------------------------------------------------------
    // SEMANTIC TOKENS
    // ------------------------------------------------------------------------

    async fn semantic_tokens_full(
        &self,
        _params: lsp::SemanticTokensParams,
    ) -> jsonrpc::Result<Option<lsp::SemanticTokensResult>> {
        // TODO #Incomplete: implement semantic tokens
        Ok(None)
    }

    async fn semantic_tokens_range(
        &self,
        _params: lsp::SemanticTokensRangeParams,
    ) -> jsonrpc::Result<Option<lsp::SemanticTokensRangeResult>> {
        // TODO #Incomplete: implement semantic tokens range
        Ok(None)
    }

    // ------------------------------------------------------------------------
    // FOLDING
    // ------------------------------------------------------------------------

    async fn folding_range(
        &self,
        params: lsp::FoldingRangeParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::FoldingRange>>> {
        // resolve file
        let uri_str = params.text_document.uri.to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };

        // query for folding ranges
        let ranges = query::folding_ranges(session, doc.file_id);
        if ranges.is_empty() {
            return Ok(None);
        }

        // convert to LSP folding ranges
        let lsp_ranges: Vec<lsp::FoldingRange> = ranges
            .into_iter()
            .map(|r| lsp::FoldingRange {
                start_line: r.start_line,
                start_character: r.start_character,
                end_line: r.end_line,
                end_character: r.end_character,
                kind: r.kind.map(|k| match k {
                    query::FoldingRangeKind::Comment => lsp::FoldingRangeKind::Comment,
                    query::FoldingRangeKind::Imports => lsp::FoldingRangeKind::Imports,
                    query::FoldingRangeKind::Region => lsp::FoldingRangeKind::Region,
                }),
                collapsed_text: r.collapsed_text,
            })
            .collect();

        Ok(Some(lsp_ranges))
    }

    // ------------------------------------------------------------------------
    // FORMATTING
    // ------------------------------------------------------------------------

    async fn formatting(
        &self,
        _params: lsp::DocumentFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TextEdit>>> {
        // TODO #Incomplete: implement formatting
        Ok(None)
    }
}

// ----------------------------------------------------------------------------
// HELPERS
// ----------------------------------------------------------------------------

/// Convert completion kind to LSP completion item kind.
fn completion_kind_to_lsp(kind: query::CompletionKind) -> lsp::CompletionItemKind {
    match kind {
        query::CompletionKind::Text => lsp::CompletionItemKind::TEXT,
        query::CompletionKind::Method => lsp::CompletionItemKind::METHOD,
        query::CompletionKind::Function => lsp::CompletionItemKind::FUNCTION,
        query::CompletionKind::Constructor => lsp::CompletionItemKind::CONSTRUCTOR,
        query::CompletionKind::Field => lsp::CompletionItemKind::FIELD,
        query::CompletionKind::Variable => lsp::CompletionItemKind::VARIABLE,
        query::CompletionKind::Class => lsp::CompletionItemKind::CLASS,
        query::CompletionKind::Interface => lsp::CompletionItemKind::INTERFACE,
        query::CompletionKind::Module => lsp::CompletionItemKind::MODULE,
        query::CompletionKind::Property => lsp::CompletionItemKind::PROPERTY,
        query::CompletionKind::Unit => lsp::CompletionItemKind::UNIT,
        query::CompletionKind::Value => lsp::CompletionItemKind::VALUE,
        query::CompletionKind::Enum => lsp::CompletionItemKind::ENUM,
        query::CompletionKind::Keyword => lsp::CompletionItemKind::KEYWORD,
        query::CompletionKind::Snippet => lsp::CompletionItemKind::SNIPPET,
        query::CompletionKind::Color => lsp::CompletionItemKind::COLOR,
        query::CompletionKind::File => lsp::CompletionItemKind::FILE,
        query::CompletionKind::Reference => lsp::CompletionItemKind::REFERENCE,
        query::CompletionKind::Folder => lsp::CompletionItemKind::FOLDER,
        query::CompletionKind::EnumMember => lsp::CompletionItemKind::ENUM_MEMBER,
        query::CompletionKind::Constant => lsp::CompletionItemKind::CONSTANT,
        query::CompletionKind::Struct => lsp::CompletionItemKind::STRUCT,
        query::CompletionKind::Event => lsp::CompletionItemKind::EVENT,
        query::CompletionKind::Operator => lsp::CompletionItemKind::OPERATOR,
        query::CompletionKind::TypeParameter => lsp::CompletionItemKind::TYPE_PARAMETER,
    }
}
