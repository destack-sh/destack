use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;
use destack_compiler::{AnalyzeTask, Compiler, CompilerOptions};
use destack_source::{FileId, FileType, Uri};
use destack_workspace::{Session, query};
use parking_lot::RwLock;
use tower_lsp_server::{Client, LanguageServer, UriExt, jsonrpc, lsp_types as lsp};

use crate::semantic;
use crate::source::{byte_span_to_range, position_to_byte};
use crate::workspace::TRACKED_FILE_TYPES;

/// State for an open document.
#[derive(Debug)]
struct OpenDocument {
    file_id: FileId,
    #[allow(dead_code)]
    content: String,
}

#[derive(Debug)]
pub struct DestackLanguageServer {
    pub(super) client: Client,
    session: RwLock<Option<Arc<Session>>>,
    open_documents: DashMap<String, OpenDocument>,
}

impl DestackLanguageServer {
    /// Create a new language server instance.
    pub fn new(client: Client) -> Self {
        Self {
            client,
            session: RwLock::new(None),
            open_documents: DashMap::new(),
        }
    }

    fn get_session(&self) -> Option<Arc<Session>> {
        self.session.read().clone()
    }

    /// Compile a module after registration.
    fn compile_module(&self, session: &Session, path: &std::path::Path) {
        let program = session.find_program_for_path(path);
        let path_buf = path.to_path_buf();

        let compiler = Compiler::new(
            Arc::new(Session::new(session.cwd.clone()).with_fs(session.fs.clone())),
            program.clone(),
            CompilerOptions {
                workers: 1,
                ..Default::default()
            },
        );

        if let Ok(module_id) = compiler.resolve_path_to_module(&path_buf) {
            compiler.enqueue(AnalyzeTask::AnalyzeModuleValidate { module: module_id });
            compiler.compile();
        }
    }
}

// ----------------------------------------------------------------------------
// lifecycle
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

        // create session and add root
        let session = Arc::new(Session::new(cwd.clone()));
        session.add_root(cwd);
        *self.session.write() = Some(session);

        // file operation filters for workspace notifications
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

        let capabilities = lsp::ServerCapabilities {
            text_document_sync: Some(lsp::TextDocumentSyncCapability::Kind(
                lsp::TextDocumentSyncKind::FULL,
            )),
            hover_provider: Some(lsp::HoverProviderCapability::Simple(true)),
            definition_provider: Some(lsp::OneOf::Left(true)),
            references_provider: Some(lsp::OneOf::Left(true)),
            document_symbol_provider: Some(lsp::OneOf::Left(true)),
            completion_provider: Some(lsp::CompletionOptions {
                trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
                ..Default::default()
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
    // synchronization
    // ------------------------------------------------------------------------

    async fn did_open(&self, params: lsp::DidOpenTextDocumentParams) {
        let uri_str = params.text_document.uri.to_string();
        let content = params.text_document.text;

        self.client
            .log_message(lsp::MessageType::INFO, format!("did_open: {uri_str}"))
            .await;

        let Some(session) = self.get_session() else {
            return;
        };

        let Some(path) = params
            .text_document
            .uri
            .to_file_path()
            .map(|p| p.into_owned())
        else {
            return;
        };

        // register module with inline content
        let uri = Uri::from_path(&path);
        let program = session.find_program_for_path(&path);
        let module_id = program.register_inline_module(uri, content.clone(), FileType::Destack);

        let module = session.modules.get(module_id);
        let file_id = module.read().file_id;

        self.open_documents
            .insert(uri_str, OpenDocument { file_id, content });

        self.compile_module(&session, &path);
    }

    async fn did_change(&self, params: lsp::DidChangeTextDocumentParams) {
        let uri_str = params.text_document.uri.to_string();

        // full sync: we get entire new content
        let Some(change) = params.content_changes.into_iter().next() else {
            return;
        };
        let content = change.text;

        let Some(session) = self.get_session() else {
            return;
        };

        let Some(path) = params
            .text_document
            .uri
            .to_file_path()
            .map(|p| p.into_owned())
        else {
            return;
        };

        // TODO #Incomplete: re-registering creates a new module, should update existing
        let uri = Uri::from_path(&path);
        let program = session.find_program_for_path(&path);
        let module_id = program.register_inline_module(uri, content.clone(), FileType::Destack);

        let module = session.modules.get(module_id);
        let file_id = module.read().file_id;

        self.open_documents
            .insert(uri_str, OpenDocument { file_id, content });

        self.compile_module(&session, &path);
    }

    async fn did_close(&self, params: lsp::DidCloseTextDocumentParams) {
        let uri_str = params.text_document.uri.to_string();
        self.open_documents.remove(&uri_str);
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
    // language features
    // ------------------------------------------------------------------------

    async fn hover(&self, params: lsp::HoverParams) -> jsonrpc::Result<Option<lsp::Hover>> {
        let uri_str = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();

        let Some(session) = self.get_session() else {
            return Ok(None);
        };

        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };

        let file = session.files.get(doc.file_id);

        let Some(offset) = position_to_byte(&file, &params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        let Some(hover_info) = query::hover(&session, doc.file_id, offset) else {
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

    // ------------------------------------------------------------------------
    // semantic tokens
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
    // formatting
    // ------------------------------------------------------------------------

    async fn formatting(
        &self,
        _params: lsp::DocumentFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TextEdit>>> {
        // TODO #Incomplete: implement formatting
        Ok(None)
    }
}
