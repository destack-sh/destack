use std::collections::HashMap;
use std::iter;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use destack_lsp_server::{Client, LanguageServer, LspService, Server, UriExt, jsonrpc};
use destack_lsp_types as lsp;
use destack_query as query;
use destack_repository::{Revision, Trace, TraceReport, TraceView};
use destack_source::{FileId, TextRange, Uri};
use destack_workspace::{
    DiagnosticRun, DiagnosticsRequest, FileDiagnostics, FileEdit, FileOperation, LocalWorkspace,
    QueryFile, QueryRun, ReloadReason, ReloadRequest, RevisionPolicy, RunQueryRequest,
    RunQueryResponse, Workspace,
};
use serde_json::to_value;

use super::{ClientCapabilities, ServerSession, ServerSettings, internal_error, workspace_error};
use crate::query::{
    ActionContext, ActionContinuation, DiagnosticDelivery, DiagnosticPublisher, Document,
    DocumentSet, DocumentUri, HierarchyContinuation, SemanticTokenStream, SourceSync,
};

/// Slow artifact attempts included in verbose LSP traces.
const TRACE_SLOW_ATTEMPTS: usize = 12;

/// The Destack language server.
#[derive(Debug)]
pub struct DestackLanguageServer {
    /// The client connection.
    pub(super) client: Client,
    /// State installed after initialization.
    session: OnceLock<ServerSession>,
}

#[allow(clippy::too_many_arguments)]
impl DestackLanguageServer {
    /// Create a new language server instance.
    pub fn new(client: Client) -> Self {
        Self {
            client,
            session: OnceLock::new(),
        }
    }

    /// Run the language server over stdio.
    pub async fn run_stdio() -> Result<(), Box<dyn std::error::Error>> {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let (service, socket) = LspService::new(Self::new);

        Server::new(stdin, stdout, socket).serve(service).await;

        Ok(())
    }

    /// Return the initialized server session.
    fn session(&self) -> jsonrpc::Result<&ServerSession> {
        self.session
            .get()
            .ok_or_else(|| internal_error("language server is not initialized"))
    }

    /// Return the semantic workspace that owns one source path.
    fn workspace(&self, path: &Path) -> jsonrpc::Result<Arc<LocalWorkspace>> {
        self.session()?.workspace(path)
    }

    /// Return features supported by the initialized client.
    fn client_capabilities(&self) -> jsonrpc::Result<&ClientCapabilities> {
        Ok(&self.session()?.client_capabilities)
    }

    /// Return diagnostics after initialization.
    fn diagnostics(&self) -> jsonrpc::Result<&DiagnosticDelivery> {
        Ok(&self.session()?.diagnostics)
    }

    /// Return mutable editor configuration after initialization.
    fn settings(&self) -> jsonrpc::Result<&ServerSettings> {
        Ok(&self.session()?.settings)
    }

    /// Execute one program query through the workspace.
    async fn query_program(
        &self,
        path: &Path,
        request: query::QueryRequest,
        revision: RevisionPolicy,
    ) -> jsonrpc::Result<RunQueryResponse> {
        let method = request.method();
        let workspace = self.workspace(path)?;
        let root = workspace.root(path).map_err(workspace_error)?;
        let request = RunQueryRequest { revision, request };
        let run = workspace
            .start_query(&root, request)
            .map_err(workspace_error)?;

        self.wait_query(workspace, root, method, run).await
    }

    /// Resolve one LSP document to workspace query state.
    fn resolve_query_file(&self, uri: &lsp::Uri) -> jsonrpc::Result<Option<QueryFile>> {
        let Some(path) = uri.to_file_path().map(|path| path.into_owned()) else {
            return Ok(None);
        };
        let workspace = self.workspace(&path)?;
        let root = workspace.root(&path).map_err(workspace_error)?;

        workspace
            .resolve_query_file(&root, path)
            .map_err(workspace_error)
    }

    /// Format one LSP document or selected text range.
    fn format_file(
        &self,
        uri: &lsp::Uri,
        range: Option<TextRange>,
    ) -> jsonrpc::Result<Option<FileEdit>> {
        let Some(path) = uri.to_file_path().map(|path| path.into_owned()) else {
            return Ok(None);
        };
        let workspace = self.workspace(&path)?;
        let root = workspace.root(&path).map_err(workspace_error)?;

        workspace
            .format_file(&root, path, range)
            .map_err(workspace_error)
    }

    /// Execute one module query against its file revision.
    async fn query_module(
        &self,
        file: &QueryFile,
        request: query::QueryRequest,
    ) -> jsonrpc::Result<RunQueryResponse> {
        let method = request.method();
        let workspace = self.workspace(&file.path)?;
        let root = workspace.root(&file.path).map_err(workspace_error)?;
        let request = RunQueryRequest {
            revision: RevisionPolicy::Current(file.revision),
            request,
        };
        let run = workspace
            .start_query(&root, request)
            .map_err(workspace_error)?;

        self.wait_query(workspace, root, method, run).await
    }

    /// Wait for one scheduled query without blocking the async server.
    async fn wait_query(
        &self,
        workspace: Arc<LocalWorkspace>,
        root: PathBuf,
        method: query::QueryMethod,
        run: QueryRun,
    ) -> jsonrpc::Result<RunQueryResponse> {
        let revision = run.revision();
        let trace = run.trace();
        let guard = run.guard();
        let response = tokio::task::spawn_blocking(move || run.wait()).await;
        guard.finish();
        self.report_query_trace(workspace.as_ref(), revision, method, trace)
            .await?;
        let response = response.map_err(internal_error)?.map_err(workspace_error)?;

        // reject results invalidated while the query was running
        let current = workspace.revision(&root).map_err(workspace_error)?;
        if current != response.revision {
            return Err(jsonrpc::Error::content_modified());
        }

        Ok(response)
    }

    /// Report one completed query through standard LSP tracing.
    async fn report_query_trace(
        &self,
        workspace: &LocalWorkspace,
        revision: Revision,
        method: query::QueryMethod,
        trace: Arc<Trace>,
    ) -> jsonrpc::Result<()> {
        if self.client.trace_level() != lsp::TraceValue::Verbose {
            return Ok(());
        }

        // snapshot exact artifact labels only for an explicit detailed trace
        let snapshot = workspace
            .snapshot_trace(revision, trace.as_ref(), TraceView::Detailed)
            .map_err(internal_error)?;
        let message = format!(
            "event=query.finished method={} revision={revision} duration_us={}",
            method.name(),
            snapshot.total_micros,
        );

        // render the complete artifact report in the explicit verbose field
        let verbose = Some(
            TraceReport::new()
                .row(method.name(), snapshot)
                .timelines()
                .span_totals()
                .slow_attempts(TRACE_SLOW_ATTEMPTS)
                .render(),
        );
        self.client
            .log_trace(message, verbose)
            .await
            .map_err(internal_error)
    }

    /// Load documents from one query revision.
    fn load_documents(
        &self,
        path: &Path,
        revision: Revision,
        file_ids: impl IntoIterator<Item = FileId>,
    ) -> jsonrpc::Result<DocumentSet> {
        let workspace = self.workspace(path)?;
        DocumentSet::load(workspace.as_ref(), path, revision, file_ids)
    }

    /// Read diagnostics without blocking the async server.
    async fn read_diagnostics(
        &self,
        workspace: Arc<LocalWorkspace>,
        request: DiagnosticsRequest,
    ) -> jsonrpc::Result<Vec<FileDiagnostics>> {
        let run = workspace
            .start_diagnostics(request)
            .map_err(workspace_error)?;

        self.wait_diagnostics(workspace, run).await
    }

    /// Wait for scheduled diagnostics and reject superseded revisions.
    async fn wait_diagnostics(
        &self,
        workspace: Arc<LocalWorkspace>,
        run: DiagnosticRun,
    ) -> jsonrpc::Result<Vec<FileDiagnostics>> {
        let revisions = run.revisions();
        let guard = run.guard();
        let diagnostics = tokio::task::spawn_blocking(move || run.wait())
            .await
            .map_err(internal_error)?
            .map_err(workspace_error)?;
        guard.finish();

        // reject any root invalidated while diagnostics were running
        for (root, revision) in revisions {
            let current = workspace.revision(&root).map_err(workspace_error)?;
            if current != revision {
                return Err(jsonrpc::Error::content_modified());
            }
        }

        Ok(diagnostics)
    }

    /// Register file watchers with the client.
    async fn register_file_watchers(&self) -> jsonrpc::Result<()> {
        let watchers = SourceSync::file_watchers();
        let options = lsp::DidChangeWatchedFilesRegistrationOptions { watchers };
        let register_options = Some(to_value(options).map_err(internal_error)?);
        let registration = lsp::Registration {
            id: "destack.watch".to_string(),
            method: "workspace/didChangeWatchedFiles".to_string(),
            register_options,
        };

        self.client.register_capability(vec![registration]).await?;

        Ok(())
    }

    /// Reload workspace source.
    fn reload_workspace(&self) -> jsonrpc::Result<()> {
        for workspace in self.session()?.workspaces() {
            workspace
                .reload(ReloadRequest {
                    roots: Vec::new(),
                    reason: ReloadReason::Manual,
                })
                .map_err(workspace_error)?;
        }

        Ok(())
    }

    /// Schedule diagnostics for one semantic workspace.
    fn schedule_diagnostics(&self, workspace: Arc<LocalWorkspace>) -> jsonrpc::Result<()> {
        for root in workspace.roots() {
            self.diagnostics()?
                .schedule(root, workspace.clone(), self.client.clone());
        }

        Ok(())
    }

    /// Schedule diagnostics for every semantic workspace.
    fn schedule_workspace_diagnostics(&self) -> jsonrpc::Result<()> {
        for workspace in self.session()?.workspaces() {
            self.schedule_diagnostics(workspace)?;
        }

        Ok(())
    }

    /// Reload configuration and schedule diagnostics for every root.
    async fn change_configuration(&self) -> jsonrpc::Result<()> {
        self.settings()?.refresh(&self.client).await?;
        self.reload_workspace()?;

        self.schedule_workspace_diagnostics()
    }

    /// Open one editor document and schedule diagnostics for its source root.
    async fn open_document(&self, params: lsp::DidOpenTextDocumentParams) -> jsonrpc::Result<()> {
        let path = params
            .text_document
            .uri
            .to_file_path()
            .map(|path| path.into_owned())
            .ok_or_else(|| jsonrpc::Error::invalid_params("document URI is not a file URI"))?;
        let uri = Uri::from_string(params.text_document.uri.to_string());
        let content = params.text_document.text;
        let workspace = self.session()?.open_document(&path)?;
        let result = workspace.file(FileOperation::OpenText {
            path: path.clone(),
            uri,
            version: params.text_document.version,
            content,
        });
        if let Err(error) = result {
            self.session()?.close_document(&path)?;

            return Err(workspace_error(error));
        }

        self.schedule_diagnostics(workspace)
    }

    /// Apply one editor document change and schedule diagnostics for its source root.
    async fn change_document(
        &self,
        params: lsp::DidChangeTextDocumentParams,
    ) -> jsonrpc::Result<()> {
        if params.content_changes.is_empty() {
            return Ok(());
        }

        let path = params
            .text_document
            .uri
            .to_file_path()
            .map(|path| path.into_owned())
            .ok_or_else(|| jsonrpc::Error::invalid_params("document URI is not a file URI"))?;
        let uri = Uri::from_string(params.text_document.uri.to_string());
        let changes = SourceSync::text_changes(params.content_changes);
        let workspace = self.workspace(&path)?;
        workspace
            .file(FileOperation::PatchText {
                path,
                uri,
                version: params.text_document.version,
                changes,
            })
            .map_err(workspace_error)?;

        self.schedule_diagnostics(workspace)
    }

    /// Save one editor document and schedule diagnostics for its source root.
    async fn save_document(&self, params: lsp::DidSaveTextDocumentParams) -> jsonrpc::Result<()> {
        let path = params
            .text_document
            .uri
            .to_file_path()
            .map(|path| path.into_owned())
            .ok_or_else(|| jsonrpc::Error::invalid_params("document URI is not a file URI"))?;
        let content = params.text;
        let workspace = self.workspace(&path)?;
        workspace
            .file(FileOperation::SaveText { path, content })
            .map_err(workspace_error)?;

        self.schedule_diagnostics(workspace)
    }

    /// Close one editor document and release its project ownership.
    async fn close_document(&self, params: lsp::DidCloseTextDocumentParams) -> jsonrpc::Result<()> {
        let path = params
            .text_document
            .uri
            .to_file_path()
            .map(|path| path.into_owned())
            .ok_or_else(|| jsonrpc::Error::invalid_params("document URI is not a file URI"))?;
        let workspace = self.workspace(&path)?;
        workspace
            .file(FileOperation::Close { path: path.clone() })
            .map_err(workspace_error)?;
        let removed = self.session()?.close_document(&path)?;

        // clear projects released with the final open document
        if !removed.is_empty() {
            for root in removed {
                self.diagnostics()?.remove_root(&root, &self.client).await;
            }

            Ok(())
        }
        // refresh the project still owned by another folder or document
        else {
            self.schedule_diagnostics(workspace)
        }
    }

    /// Apply one workspace-folder change.
    async fn change_workspace_folders(
        &self,
        params: lsp::DidChangeWorkspaceFoldersParams,
    ) -> jsonrpc::Result<()> {
        let mut added = Vec::with_capacity(params.event.added.len());
        for folder in params.event.added {
            let path = folder
                .uri
                .to_file_path()
                .map(|path| path.into_owned())
                .ok_or_else(|| {
                    jsonrpc::Error::invalid_params("workspace folder URI is not a file URI")
                })?;
            added.push(path);
        }

        let mut removed = Vec::with_capacity(params.event.removed.len());
        for folder in params.event.removed {
            let path = folder
                .uri
                .to_file_path()
                .map(|path| path.into_owned())
                .ok_or_else(|| {
                    jsonrpc::Error::invalid_params("workspace folder URI is not a file URI")
                })?;
            removed.push(path);
        }

        // open declared source roots selected by added editor folders
        for path in added {
            if let Some(workspace) = self.session()?.open_editor_folder(&path)? {
                self.schedule_diagnostics(workspace)?;
            }
        }

        // remove source roots no longer selected by editor folders
        for path in removed {
            for root in self.session()?.close_editor_folder(&path)? {
                self.diagnostics()?.remove_root(&root, &self.client).await;
            }
        }

        Ok(())
    }

    /// Reload watched filesystem state and schedule current diagnostics.
    async fn change_watched_files(
        &self,
        params: lsp::DidChangeWatchedFilesParams,
    ) -> jsonrpc::Result<()> {
        if params.changes.is_empty() {
            return Ok(());
        }

        self.reload_workspace()?;

        self.schedule_workspace_diagnostics()
    }

    /// Clear diagnostics for one renamed file when neither path remains open.
    async fn clear_renamed_file_diagnostics(
        &self,
        old_uri: &str,
        new_uri: &str,
    ) -> jsonrpc::Result<()> {
        let old_uri = old_uri
            .parse::<lsp::Uri>()
            .map_err(|error| jsonrpc::Error::invalid_params(format!("invalid old URI: {error}")))?;
        let new_uri = new_uri
            .parse::<lsp::Uri>()
            .map_err(|error| jsonrpc::Error::invalid_params(format!("invalid new URI: {error}")))?;
        let old_path = old_uri
            .to_file_path()
            .map(|path| path.into_owned())
            .ok_or_else(|| jsonrpc::Error::invalid_params("old URI is not a file URI"))?;
        let new_path = new_uri
            .to_file_path()
            .map(|path| path.into_owned())
            .ok_or_else(|| jsonrpc::Error::invalid_params("new URI is not a file URI"))?;

        // keep diagnostics owned by open editor documents
        let old_workspace = self.workspace(&old_path)?;
        let new_workspace = self.workspace(&new_path)?;
        let old_is_open = old_workspace
            .is_file_open(&old_path)
            .map_err(workspace_error)?;
        let new_is_open = new_workspace
            .is_file_open(&new_path)
            .map_err(workspace_error)?;
        if old_is_open || new_is_open {
            return Ok(());
        }

        self.diagnostics()?.remove_file(old_uri, &self.client).await;

        Ok(())
    }

    /// Clear diagnostics for one deleted file when it is no longer open.
    async fn clear_deleted_file_diagnostics(&self, uri: &str) -> jsonrpc::Result<()> {
        let uri = uri
            .parse::<lsp::Uri>()
            .map_err(|error| jsonrpc::Error::invalid_params(format!("invalid URI: {error}")))?;
        let path = uri
            .to_file_path()
            .map(|path| path.into_owned())
            .ok_or_else(|| jsonrpc::Error::invalid_params("URI is not a file URI"))?;

        // keep diagnostics owned by an open editor document
        let workspace = self.workspace(&path)?;
        let is_open = workspace.is_file_open(&path).map_err(workspace_error)?;
        if is_open {
            return Ok(());
        }

        self.diagnostics()?.remove_file(uri, &self.client).await;

        Ok(())
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
        self.client.set_trace(params.trace.unwrap_or_default());

        // build the complete initialized session
        let session = ServerSession::open(&params)?;
        let supports_code_lenses = session.client_capabilities.supports_code_lenses();
        let supports_pull_diagnostics = session.client_capabilities.supports_pull_diagnostics;
        self.session
            .set(session)
            .map_err(|_| internal_error("language server initialized more than once"))?;

        // build file operation filters for root notifications
        let file_operation_filters = SourceSync::file_operation_filters();

        // declare server capabilities
        let capabilities = lsp::ServerCapabilities {
            text_document_sync: Some(lsp::TextDocumentSyncCapability::Options(
                lsp::TextDocumentSyncOptions {
                    open_close: Some(true),
                    change: Some(lsp::TextDocumentSyncKind::INCREMENTAL),
                    will_save: None,
                    will_save_wait_until: None,
                    save: Some(
                        lsp::SaveOptions {
                            include_text: Some(true),
                        }
                        .into(),
                    ),
                },
            )),
            hover_provider: Some(lsp::HoverProviderCapability::Simple(true)),
            definition_provider: Some(lsp::OneOf::Left(true)),
            declaration_provider: Some(lsp::DeclarationCapability::Simple(true)),
            type_definition_provider: Some(lsp::TypeDefinitionProviderCapability::Simple(true)),
            references_provider: Some(lsp::OneOf::Left(true)),
            document_symbol_provider: Some(lsp::OneOf::Left(true)),
            workspace_symbol_provider: Some(lsp::OneOf::Left(true)),
            document_highlight_provider: Some(lsp::OneOf::Left(true)),
            completion_provider: Some(lsp::CompletionOptions {
                trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
                resolve_provider: None,
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
                        legend: SemanticTokenStream::legend(),
                        range: Some(true),
                        full: Some(lsp::SemanticTokensFullOptions::Bool(true)),
                        work_done_progress_options: lsp::WorkDoneProgressOptions::default(),
                    },
                ),
            ),
            diagnostic_provider: supports_pull_diagnostics.then_some(
                lsp::DiagnosticServerCapabilities::Options(lsp::DiagnosticOptions {
                    identifier: Some("destack".to_string()),
                    inter_file_dependencies: true,
                    workspace_diagnostics: true,
                    markup_message_support: None,
                    work_done_progress_options: lsp::WorkDoneProgressOptions::default(),
                }),
            ),
            document_formatting_provider: Some(lsp::OneOf::Left(true)),
            document_range_formatting_provider: Some(lsp::OneOf::Left(true)),
            document_on_type_formatting_provider: Some(lsp::DocumentOnTypeFormattingOptions {
                first_trigger_character: ";".to_string(),
                more_trigger_character: Some(vec!["}".to_string()]),
            }),
            folding_range_provider: Some(lsp::FoldingRangeProviderCapability::Simple(true)),
            selection_range_provider: Some(lsp::SelectionRangeProviderCapability::Simple(true)),
            document_link_provider: Some(lsp::DocumentLinkOptions {
                resolve_provider: None,
                work_done_progress_options: Default::default(),
            }),
            rename_provider: Some(lsp::OneOf::Right(lsp::RenameOptions {
                prepare_provider: Some(true),
                work_done_progress_options: Default::default(),
            })),
            code_action_provider: Some(lsp::CodeActionProviderCapability::Options(
                lsp::CodeActionOptions {
                    code_action_kinds: Some(vec![
                        lsp::CodeActionKind::QUICKFIX,
                        lsp::CodeActionKind::REFACTOR,
                        lsp::CodeActionKind::REFACTOR_EXTRACT,
                        lsp::CodeActionKind::REFACTOR_INLINE,
                    ]),
                    resolve_provider: Some(true),
                    work_done_progress_options: Default::default(),
                },
            )),
            code_lens_provider: supports_code_lenses.then_some(lsp::CodeLensOptions {
                resolve_provider: None,
            }),
            inlay_hint_provider: Some(lsp::OneOf::Left(true)),
            implementation_provider: Some(lsp::ImplementationProviderCapability::Simple(true)),
            call_hierarchy_provider: Some(lsp::CallHierarchyServerCapability::Simple(true)),
            type_hierarchy_provider: Some(lsp::OneOf::Left(true)),
            execute_command_provider: Some(lsp::ExecuteCommandOptions {
                commands: vec!["destack.reload".to_string()],
                work_done_progress_options: Default::default(),
            }),
            workspace: Some(lsp::WorkspaceServerCapabilities {
                workspace_folders: Some(lsp::WorkspaceFoldersServerCapabilities {
                    supported: Some(true),
                    change_notifications: Some(lsp::OneOf::Left(true)),
                }),
                file_operations: Some(lsp::WorkspaceFileOperationsServerCapabilities {
                    did_create: None,
                    did_rename: Some(lsp::FileOperationRegistrationOptions {
                        filters: file_operation_filters.clone(),
                    }),
                    did_delete: Some(lsp::FileOperationRegistrationOptions {
                        filters: file_operation_filters.clone(),
                    }),
                    will_create: None,
                    will_rename: Some(lsp::FileOperationRegistrationOptions {
                        filters: file_operation_filters.clone(),
                    }),
                    will_delete: None,
                }),
                text_document_content: None,
            }),
            ..Default::default()
        };

        Ok(lsp::InitializeResult {
            capabilities,
            server_info: Some(lsp::ServerInfo {
                name: "destack".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            offset_encoding: None,
        })
    }

    async fn initialized(&self, _: lsp::InitializedParams) {
        let capabilities = match self.client_capabilities() {
            Ok(capabilities) => capabilities,
            Err(error) => {
                self.client
                    .report_error("client_capabilities.read", error)
                    .await;

                return;
            }
        };

        // register watched files when the client supports dynamic registration
        if capabilities.supports_dynamic_file_watching
            && let Err(error) = self.register_file_watchers().await
        {
            self.client
                .report_error("file_watchers.register", error)
                .await;
        }

        // request settings when the client supports workspace configuration
        if capabilities.supports_configuration {
            let result = match self.settings() {
                Ok(settings) => settings.refresh(&self.client).await,
                Err(error) => Err(error),
            };
            if let Err(error) = result {
                self.client.report_error("configuration.load", error).await;
            }
        }
    }

    async fn set_trace(&self, params: lsp::SetTraceParams) {
        self.client.set_trace(params.value);
    }

    async fn did_change_configuration(&self, _: lsp::DidChangeConfigurationParams) {
        if let Err(error) = self.change_configuration().await {
            self.client.report_error("configuration.apply", error).await;
        }
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    // ------------------------------------------------------------------------
    // SYNCHRONIZATION
    // ------------------------------------------------------------------------

    async fn did_open(&self, params: lsp::DidOpenTextDocumentParams) {
        if let Err(error) = self.open_document(params).await {
            self.client.report_error("document.open", error).await;
        }
    }

    async fn did_change(&self, params: lsp::DidChangeTextDocumentParams) {
        if let Err(error) = self.change_document(params).await {
            self.client.report_error("document.change", error).await;
        }
    }

    async fn did_save(&self, params: lsp::DidSaveTextDocumentParams) {
        if let Err(error) = self.save_document(params).await {
            self.client.report_error("document.save", error).await;
        }
    }

    async fn did_close(&self, params: lsp::DidCloseTextDocumentParams) {
        if let Err(error) = self.close_document(params).await {
            self.client.report_error("document.close", error).await;
        }
    }

    async fn did_change_workspace_folders(&self, params: lsp::DidChangeWorkspaceFoldersParams) {
        if let Err(error) = self.change_workspace_folders(params).await {
            self.client
                .report_error("workspace_folders.change", error)
                .await;
        }
    }

    async fn did_change_watched_files(&self, params: lsp::DidChangeWatchedFilesParams) {
        if let Err(error) = self.change_watched_files(params).await {
            self.client
                .report_error("watched_files.reload", error)
                .await;
        }
    }

    async fn will_rename_files(
        &self,
        params: lsp::RenameFilesParams,
    ) -> jsonrpc::Result<Option<lsp::WorkspaceEdit>> {
        // collect rename targets from uris
        let mut renames = Vec::new();
        let mut query_state: Option<(PathBuf, Revision)> = None;
        for file in params.files {
            let old_uri = file
                .old_uri
                .parse::<lsp::Uri>()
                .map_err(|_| jsonrpc::Error::invalid_params("invalid old file URI"))?;
            let new_uri = file
                .new_uri
                .parse::<lsp::Uri>()
                .map_err(|_| jsonrpc::Error::invalid_params("invalid new file URI"))?;
            let old_path = old_uri
                .to_file_path()
                .map(|path| path.into_owned())
                .ok_or_else(|| jsonrpc::Error::invalid_params("old URI is not a file URI"))?;
            let new_path = new_uri
                .to_file_path()
                .map(|path| path.into_owned())
                .ok_or_else(|| jsonrpc::Error::invalid_params("new URI is not a file URI"))?;

            // enforce a single root for rename write consistency
            let workspace = self.workspace(&old_path)?;
            let new_workspace = self.workspace(&new_path)?;
            let root = workspace.root(&old_path).map_err(workspace_error)?;
            let new_root = new_workspace.root(&new_path).map_err(workspace_error)?;
            if !Arc::ptr_eq(&workspace, &new_workspace) || root != new_root {
                return Err(jsonrpc::Error::invalid_params(
                    "file rename crosses workspace roots",
                ));
            }

            if let Some((existing_root, _)) = query_state.as_ref() {
                if existing_root != &root {
                    return Err(jsonrpc::Error::invalid_params(
                        "file renames span workspace roots",
                    ));
                }
            } else {
                let revision = workspace.revision(&root).map_err(workspace_error)?;
                query_state = Some((root, revision));
            }

            renames.push(query::FileRename { old_path, new_path });
        }
        if renames.is_empty() {
            return Ok(None);
        }
        let (query_root, query_revision) =
            query_state.ok_or_else(|| internal_error("file rename has no workspace state"))?;

        // build workspace edits for import specifiers
        let request = query::QueryRequest::RenameFiles(query::RenameFilesRequest { renames });
        let response = self
            .query_program(
                &query_root,
                request,
                RevisionPolicy::Current(query_revision),
            )
            .await?;
        let revision = response.revision;
        let query::QueryResponse::RenameFiles(response) = response.response else {
            return Err(internal_error("query did not return rename files"));
        };
        let Some(edit) = response.edit else {
            return Ok(None);
        };
        if edit.is_empty() {
            return Ok(None);
        }

        let file_ids = edit.files.iter().map(|file| file.file);
        let documents = self.load_documents(&query_root, revision, file_ids)?;
        let workspace_edit = documents.workspace_edit(&edit)?;

        Ok(Some(workspace_edit))
    }

    async fn did_rename_files(&self, params: lsp::RenameFilesParams) {
        for file in params.files {
            let result = self
                .clear_renamed_file_diagnostics(&file.old_uri, &file.new_uri)
                .await;
            if let Err(error) = result {
                self.client
                    .report_error("renamed_file_diagnostics.clear", error)
                    .await;
            }
        }
    }

    async fn did_delete_files(&self, params: lsp::DeleteFilesParams) {
        for file in params.files {
            let result = self.clear_deleted_file_diagnostics(&file.uri).await;
            if let Err(error) = result {
                self.client
                    .report_error("deleted_file_diagnostics.clear", error)
                    .await;
            }
        }
    }

    // ------------------------------------------------------------------------
    // DIAGNOSTICS
    // ------------------------------------------------------------------------

    async fn diagnostic(
        &self,
        params: lsp::DocumentDiagnosticParams,
    ) -> jsonrpc::Result<lsp::DocumentDiagnosticReportResult> {
        let tracked_path = params
            .text_document
            .uri
            .to_file_path()
            .map(|path| path.into_owned());
        let Some(path) = tracked_path.as_ref() else {
            let report =
                lsp::DocumentDiagnosticReport::Full(lsp::RelatedFullDocumentDiagnosticReport {
                    related_documents: None,
                    full_document_diagnostic_report: lsp::FullDocumentDiagnosticReport {
                        result_id: None,
                        items: Vec::new(),
                    },
                });
            return Ok(lsp::DocumentDiagnosticReportResult::Report(report));
        };

        let workspace = self.workspace(path)?;
        let mut diagnostics = self
            .read_diagnostics(workspace.clone(), DiagnosticsRequest::File(path.clone()))
            .await?;
        if diagnostics.len() > 1 {
            return Err(internal_error(format!(
                "workspace returned {} diagnostic files for one path",
                diagnostics.len()
            )));
        }
        let file_diagnostics = diagnostics.pop();
        let Some(file_diagnostics) = file_diagnostics else {
            let report =
                lsp::DocumentDiagnosticReport::Full(lsp::RelatedFullDocumentDiagnosticReport {
                    related_documents: None,
                    full_document_diagnostic_report: lsp::FullDocumentDiagnosticReport {
                        result_id: None,
                        items: Vec::new(),
                    },
                });
            return Ok(lsp::DocumentDiagnosticReportResult::Report(report));
        };
        let publisher = DiagnosticPublisher::new(self.client.clone(), workspace);
        let documents = publisher.load_documents(&file_diagnostics)?;
        let diagnostics = file_diagnostics.diagnostics;
        let result_id = DiagnosticPublisher::result_id(&diagnostics);

        if params.previous_result_id.as_ref() == Some(&result_id) {
            let report = lsp::DocumentDiagnosticReport::Unchanged(
                lsp::RelatedUnchangedDocumentDiagnosticReport {
                    related_documents: None,
                    unchanged_document_diagnostic_report: lsp::UnchangedDocumentDiagnosticReport {
                        result_id,
                    },
                },
            );
            return Ok(lsp::DocumentDiagnosticReportResult::Report(report));
        }

        let items = diagnostics
            .into_iter()
            .map(|diagnostic| documents.diagnostic(&diagnostic))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        let report =
            lsp::DocumentDiagnosticReport::Full(lsp::RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: lsp::FullDocumentDiagnosticReport {
                    result_id: Some(result_id),
                    items,
                },
            });

        Ok(lsp::DocumentDiagnosticReportResult::Report(report))
    }

    async fn workspace_diagnostic(
        &self,
        params: lsp::WorkspaceDiagnosticParams,
    ) -> jsonrpc::Result<lsp::WorkspaceDiagnosticReportResult> {
        let previous_ids: std::collections::HashMap<String, String> = params
            .previous_result_ids
            .into_iter()
            .map(|entry| (entry.uri.to_string(), entry.value))
            .collect();

        let mut items = Vec::new();
        for workspace in self.session()?.workspaces() {
            let diagnostics = self
                .read_diagnostics(workspace.clone(), DiagnosticsRequest::All)
                .await?;
            let publisher = DiagnosticPublisher::new(self.client.clone(), workspace);

            // encode every diagnostic file from this semantic workspace
            for file_diagnostics in diagnostics {
                let uri = DocumentUri::source(&file_diagnostics.uri).ok_or_else(|| {
                    internal_error(format!(
                        "diagnostic URI is not representable by LSP: {}",
                        file_diagnostics.uri
                    ))
                })?;
                let documents = publisher.load_documents(&file_diagnostics)?;
                let diagnostics = file_diagnostics.diagnostics;
                let result_id = DiagnosticPublisher::result_id(&diagnostics);
                let version = file_diagnostics.version.map(|version| version as i64);
                let uri_string = uri.to_string();
                let report = if previous_ids.get(&uri_string) == Some(&result_id) {
                    lsp::WorkspaceDocumentDiagnosticReport::Unchanged(
                        lsp::WorkspaceUnchangedDocumentDiagnosticReport {
                            uri,
                            version,
                            unchanged_document_diagnostic_report:
                                lsp::UnchangedDocumentDiagnosticReport { result_id },
                        },
                    )
                } else {
                    let lsp_diagnostics = diagnostics
                        .into_iter()
                        .map(|diagnostic| documents.diagnostic(&diagnostic))
                        .collect::<jsonrpc::Result<Vec<_>>>()?;

                    lsp::WorkspaceDocumentDiagnosticReport::Full(
                        lsp::WorkspaceFullDocumentDiagnosticReport {
                            uri,
                            version,
                            full_document_diagnostic_report: lsp::FullDocumentDiagnosticReport {
                                result_id: Some(result_id),
                                items: lsp_diagnostics,
                            },
                        },
                    )
                };
                items.push(report);
            }
        }

        Ok(lsp::WorkspaceDiagnosticReportResult::Report(
            lsp::WorkspaceDiagnosticReport { items },
        ))
    }

    // ------------------------------------------------------------------------
    // COMMANDS
    // ------------------------------------------------------------------------

    async fn execute_command(
        &self,
        params: lsp::ExecuteCommandParams,
    ) -> jsonrpc::Result<Option<lsp::LSPAny>> {
        match params.command.as_str() {
            "destack.reload" => {
                self.reload_workspace()?;
                self.schedule_workspace_diagnostics()?;

                Ok(None)
            }
            command => Err(jsonrpc::Error::invalid_params(format!(
                "unsupported command: {command}"
            ))),
        }
    }

    // ------------------------------------------------------------------------
    // NAVIGATION
    // ------------------------------------------------------------------------

    async fn goto_definition(
        &self,
        params: lsp::GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<lsp::GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.resolve_query_file(uri)? else {
            return Ok(None);
        };
        let document = Document::new(query_file.file.clone());
        let offset = document.offset(&params.text_document_position_params.position)?;
        let position = query_file.position(offset);
        let request =
            query::QueryRequest::GotoDefinition(query::GotoDefinitionRequest { position });
        let response = self.query_module(&query_file, request).await?;
        let revision = response.revision;
        let query::QueryResponse::GotoDefinition(response) = response.response else {
            return Err(internal_error("query did not return goto definition"));
        };

        // build targets
        let file_ids = response
            .targets
            .iter()
            .flat_map(|target| [target.origin.span.file, target.target.span.file]);
        let documents = self.load_documents(&query_file.path, revision, file_ids)?;
        let links = response
            .targets
            .iter()
            .map(|query_target| documents.location_link(query_target))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        if links.is_empty() {
            return Ok(None);
        }

        Ok(Some(lsp::GotoDefinitionResponse::Link(links)))
    }

    async fn goto_declaration(
        &self,
        params: lsp::request::GotoDeclarationParams,
    ) -> jsonrpc::Result<Option<lsp::request::GotoDeclarationResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.resolve_query_file(uri)? else {
            return Ok(None);
        };
        let document = Document::new(query_file.file.clone());
        let offset = document.offset(&params.text_document_position_params.position)?;
        let position = query_file.position(offset);
        let request =
            query::QueryRequest::GotoDeclaration(query::GotoDeclarationRequest { position });
        let response = self.query_module(&query_file, request).await?;
        let revision = response.revision;
        let query::QueryResponse::GotoDeclaration(response) = response.response else {
            return Err(internal_error("query did not return goto declaration"));
        };

        // build targets
        let file_ids = response
            .targets
            .iter()
            .flat_map(|target| [target.origin.span.file, target.target.span.file]);
        let documents = self.load_documents(&query_file.path, revision, file_ids)?;
        let links = response
            .targets
            .iter()
            .map(|query_target| documents.location_link(query_target))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        if links.is_empty() {
            return Ok(None);
        }

        Ok(Some(lsp::GotoDefinitionResponse::Link(links)))
    }

    async fn goto_type_definition(
        &self,
        params: lsp::request::GotoTypeDefinitionParams,
    ) -> jsonrpc::Result<Option<lsp::request::GotoTypeDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.resolve_query_file(uri)? else {
            return Ok(None);
        };
        let document = Document::new(query_file.file.clone());
        let offset = document.offset(&params.text_document_position_params.position)?;
        let position = query_file.position(offset);
        let request =
            query::QueryRequest::GotoTypeDefinition(query::GotoTypeDefinitionRequest { position });
        let response = self.query_module(&query_file, request).await?;
        let revision = response.revision;
        let query::QueryResponse::GotoTypeDefinition(response) = response.response else {
            return Err(internal_error("query did not return goto type definition"));
        };

        // build targets
        let file_ids = response
            .targets
            .iter()
            .flat_map(|target| [target.origin.span.file, target.target.span.file]);
        let documents = self.load_documents(&query_file.path, revision, file_ids)?;
        let links = response
            .targets
            .iter()
            .map(|query_target| documents.location_link(query_target))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        if links.is_empty() {
            return Ok(None);
        }

        Ok(Some(lsp::GotoDefinitionResponse::Link(links)))
    }

    async fn references(
        &self,
        params: lsp::ReferenceParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::Location>>> {
        let uri = &params.text_document_position.text_document.uri;
        let Some(query_file) = self.resolve_query_file(uri)? else {
            return Ok(None);
        };
        let document = Document::new(query_file.file.clone());
        let offset = document.offset(&params.text_document_position.position)?;
        let position = query_file.position(offset);
        let request = query::QueryRequest::FindReferences(query::FindReferencesRequest {
            position,
            include_declaration: params.context.include_declaration,
        });
        let response = self.query_module(&query_file, request).await?;
        let revision = response.revision;
        let query::QueryResponse::FindReferences(response) = response.response else {
            return Err(internal_error("query did not return references"));
        };
        let references = response.references;
        if references.is_empty() {
            return Ok(None);
        }

        // build LSP locations
        let file_ids = references
            .iter()
            .map(|reference| reference.target.span.file);
        let documents = self.load_documents(&query_file.path, revision, file_ids)?;
        let locations = references
            .iter()
            .map(|reference| documents.location(reference.target.span))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        Ok(Some(locations))
    }

    async fn document_symbol(
        &self,
        params: lsp::DocumentSymbolParams,
    ) -> jsonrpc::Result<Option<lsp::DocumentSymbolResponse>> {
        let Some(query_file) = self.resolve_query_file(&params.text_document.uri)? else {
            return Ok(None);
        };
        let document = Document::new(query_file.file.clone());
        let request = query::QueryRequest::Outline(query::OutlineRequest {
            module: query_file.module,
            file_id: query_file.file.id,
        });
        let response = self.query_module(&query_file, request).await?;
        let query::QueryResponse::Outline(response) = response.response else {
            return Err(internal_error("query did not return outline"));
        };
        let symbols = response.symbols;
        if symbols.is_empty() {
            return Ok(None);
        }

        // build LSP symbols
        let lsp_symbols = symbols
            .iter()
            .map(|query_symbol| document.document_symbol(query_symbol))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        Ok(Some(lsp::DocumentSymbolResponse::Nested(lsp_symbols)))
    }

    async fn symbol(
        &self,
        params: lsp::WorkspaceSymbolParams,
    ) -> jsonrpc::Result<Option<lsp::OneOf<Vec<lsp::SymbolInformation>, Vec<lsp::WorkspaceSymbol>>>>
    {
        // query every semantic workspace independently
        let workspaces = self.session()?.workspaces();
        if workspaces.is_empty() {
            return Ok(None);
        }
        let mut symbols = Vec::new();
        for workspace in workspaces {
            for root in workspace.roots() {
                let request = query::QueryRequest::SearchSymbols(query::SearchSymbolsRequest {
                    query: params.query.clone(),
                    max_results: 100,
                });
                let response = self
                    .query_program(&root, request, RevisionPolicy::Latest)
                    .await?;
                let revision = response.revision;
                let query::QueryResponse::SearchSymbols(response) = response.response else {
                    return Err(internal_error("query did not return symbol search"));
                };

                // retain the root and revision required to project each result
                for symbol in response.symbols {
                    let Some(order) = symbol.order(&params.query) else {
                        return Err(internal_error(format!(
                            "symbol search returned nonmatching symbol {}",
                            symbol.name
                        )));
                    };
                    symbols.push((order, root.clone(), revision, symbol));
                }
            }
        }

        // apply one stable order and result bound across all roots
        symbols.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
        symbols.truncate(100);

        // build LSP symbols
        let mut lsp_symbols = Vec::new();
        let mut file_ids_by_root: HashMap<PathBuf, (Revision, Vec<FileId>)> = HashMap::new();
        for (_, root, revision, symbol) in symbols.iter() {
            let entry = file_ids_by_root
                .entry(root.clone())
                .or_insert_with(|| (*revision, Vec::new()));
            if entry.0 != *revision {
                return Err(internal_error(format!(
                    "symbol search mixed revisions for root {}",
                    root.display()
                )));
            }
            entry.1.push(symbol.target.span.file);
        }
        let mut documents_by_root = HashMap::with_capacity(file_ids_by_root.len());
        for (root, (revision, file_ids)) in file_ids_by_root {
            let documents = self.load_documents(&root, revision, file_ids)?;
            documents_by_root.insert(root, documents);
        }

        for (_, root, _, symbol) in symbols.iter() {
            let Some(documents) = documents_by_root.get(root) else {
                return Err(internal_error(format!(
                    "symbol search file map is missing root {}",
                    root.display()
                )));
            };
            let lsp_symbol = documents.symbol_information(symbol)?;
            lsp_symbols.push(lsp_symbol);
        }

        if lsp_symbols.is_empty() {
            Ok(None)
        } else {
            Ok(Some(lsp::OneOf::Left(lsp_symbols)))
        }
    }

    async fn document_highlight(
        &self,
        params: lsp::DocumentHighlightParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::DocumentHighlight>>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.resolve_query_file(uri)? else {
            return Ok(None);
        };
        let document = Document::new(query_file.file.clone());
        let offset = document.offset(&params.text_document_position_params.position)?;
        let position = query_file.position(offset);
        let request = query::QueryRequest::Highlight(query::HighlightRequest { position });
        let response = self.query_module(&query_file, request).await?;
        let query::QueryResponse::Highlight(response) = response.response else {
            return Err(internal_error("query did not return highlights"));
        };
        let highlights = response.highlights;
        if highlights.is_empty() {
            return Ok(None);
        }

        // build LSP highlights
        let lsp_highlights = highlights
            .iter()
            .map(|query_highlight| document.highlight(query_highlight))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        Ok(Some(lsp_highlights))
    }

    // ------------------------------------------------------------------------
    // ASSIST
    // ------------------------------------------------------------------------

    async fn hover(&self, params: lsp::HoverParams) -> jsonrpc::Result<Option<lsp::Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.resolve_query_file(uri)? else {
            return Ok(None);
        };
        let document = Document::new(query_file.file.clone());
        let offset = document.offset(&params.text_document_position_params.position)?;
        let position = query_file.position(offset);
        let request = query::QueryRequest::Hover(query::HoverRequest { position });
        let response = self.query_module(&query_file, request).await?;
        let revision = response.revision;
        let query::QueryResponse::Hover(response) = response.response else {
            return Err(internal_error("query did not return hover"));
        };
        let Some(hover_info) = response.hover else {
            return Ok(None);
        };

        // project every declaration target from the response revision
        let file_ids = hover_info.items.iter().map(|item| item.target.span.file);
        let documents = self.load_documents(&query_file.path, revision, file_ids)?;
        let markdown = documents.render_hover(&hover_info.items)?;
        let range = document.range(hover_info.range)?;

        Ok(Some(lsp::Hover {
            contents: lsp::HoverContents::Markup(lsp::MarkupContent {
                kind: lsp::MarkupKind::Markdown,
                value: markdown,
            }),
            range: Some(range),
        }))
    }

    async fn completion(
        &self,
        params: lsp::CompletionParams,
    ) -> jsonrpc::Result<Option<lsp::CompletionResponse>> {
        // map the LSP trigger kind to completion behavior
        let trigger = match params.context.as_ref().map(|context| context.trigger_kind) {
            Some(kind) if kind == lsp::CompletionTriggerKind::TRIGGER_CHARACTER => {
                let character = params
                    .context
                    .as_ref()
                    .and_then(|context| context.trigger_character.as_deref())
                    .ok_or_else(|| {
                        jsonrpc::Error::invalid_params("completion trigger character is missing")
                    })?;
                let mut characters = character.chars();
                let character = characters.next().ok_or_else(|| {
                    jsonrpc::Error::invalid_params("completion trigger character is empty")
                })?;
                if characters.next().is_some() {
                    return Err(jsonrpc::Error::invalid_params(
                        "completion trigger contains multiple characters",
                    ));
                }

                query::CompletionTrigger::Character(character)
            }
            Some(kind)
                if kind == lsp::CompletionTriggerKind::TRIGGER_FOR_INCOMPLETE_COMPLETIONS =>
            {
                query::CompletionTrigger::Incomplete
            }
            None | Some(lsp::CompletionTriggerKind::INVOKED) => query::CompletionTrigger::Invoked,
            Some(_) => {
                return Err(jsonrpc::Error::invalid_params(
                    "completion trigger kind is unsupported",
                ));
            }
        };

        let uri = &params.text_document_position.text_document.uri;
        let Some(query_file) = self.resolve_query_file(uri)? else {
            return Ok(None);
        };
        let document = Document::new(query_file.file.clone());
        let offset = document.offset(&params.text_document_position.position)?;
        let position = query_file.position(offset);
        let include_auto_imports = self.settings()?.completion_auto_imports();
        let request = query::QueryRequest::Completion(query::CompletionRequest {
            position,
            trigger,
            include_auto_imports,
        });
        let response = self.query_module(&query_file, request).await?;
        let query::QueryResponse::Completion(response) = response.response else {
            return Err(internal_error("query did not return completion"));
        };
        let is_incomplete = response.is_incomplete;
        let completions = response.items;
        if completions.is_empty() {
            return Ok(None);
        }

        // build LSP completion items
        let items: Vec<lsp::CompletionItem> = completions
            .into_iter()
            .enumerate()
            .map(|(index, item)| document.completion_item(index, item))
            .collect::<jsonrpc::Result<_>>()?;

        Ok(Some(lsp::CompletionResponse::List(lsp::CompletionList {
            is_incomplete,
            items,
            ..Default::default()
        })))
    }

    async fn signature_help(
        &self,
        params: lsp::SignatureHelpParams,
    ) -> jsonrpc::Result<Option<lsp::SignatureHelp>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.resolve_query_file(uri)? else {
            return Ok(None);
        };
        let document = Document::new(query_file.file.clone());
        let offset = document.offset(&params.text_document_position_params.position)?;
        let position = query_file.position(offset);
        let request = query::QueryRequest::SignatureHelp(query::SignatureHelpRequest { position });
        let response = self.query_module(&query_file, request).await?;
        let query::QueryResponse::SignatureHelp(response) = response.response else {
            return Err(internal_error("query did not return signature help"));
        };
        let Some(help) = response.help else {
            return Ok(None);
        };

        Ok(Some(document.signature_help(help)))
    }

    // ------------------------------------------------------------------------
    // SEMANTIC TOKENS
    // ------------------------------------------------------------------------

    async fn semantic_tokens_full(
        &self,
        params: lsp::SemanticTokensParams,
    ) -> jsonrpc::Result<Option<lsp::SemanticTokensResult>> {
        let Some(query_file) = self.resolve_query_file(&params.text_document.uri)? else {
            return Ok(None);
        };
        let document = Document::new(query_file.file.clone());
        let request = query::QueryRequest::SemanticTokens(query::SemanticTokensRequest {
            module: query_file.module,
            file_id: query_file.file.id,
        });
        let response = self.query_module(&query_file, request).await?;
        let query::QueryResponse::SemanticTokens(response) = response.response else {
            return Err(internal_error("query did not return semantic tokens"));
        };
        let tokens = response.tokens;

        // build LSP tokens
        let lsp_tokens = document.semantic_tokens(&tokens)?;

        Ok(Some(lsp::SemanticTokensResult::Tokens(
            lsp::SemanticTokens {
                result_id: None,
                data: lsp_tokens,
            },
        )))
    }

    async fn semantic_tokens_range(
        &self,
        params: lsp::SemanticTokensRangeParams,
    ) -> jsonrpc::Result<Option<lsp::SemanticTokensRangeResult>> {
        let Some(query_file) = self.resolve_query_file(&params.text_document.uri)? else {
            return Ok(None);
        };
        let document = Document::new(query_file.file.clone());
        let start = document.offset(&params.range.start)?;
        let end = document.offset(&params.range.end)?;
        let range = query_file
            .range(start, end)
            .ok_or_else(|| jsonrpc::Error::invalid_params("range is reversed"))?;
        let request =
            query::QueryRequest::SemanticTokensRange(query::SemanticTokensRangeRequest { range });
        let response = self.query_module(&query_file, request).await?;
        let query::QueryResponse::SemanticTokensRange(response) = response.response else {
            return Err(internal_error("query did not return range semantic tokens"));
        };
        let tokens = response.tokens;

        // build LSP tokens
        let lsp_tokens = document.semantic_tokens(&tokens)?;

        Ok(Some(lsp::SemanticTokensRangeResult::Tokens(
            lsp::SemanticTokens {
                result_id: None,
                data: lsp_tokens,
            },
        )))
    }

    // ------------------------------------------------------------------------
    // FOLDING
    // ------------------------------------------------------------------------

    async fn folding_range(
        &self,
        params: lsp::FoldingRangeParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::FoldingRange>>> {
        let Some(query_file) = self.resolve_query_file(&params.text_document.uri)? else {
            return Ok(None);
        };
        let document = Document::new(query_file.file.clone());
        let request = query::QueryRequest::FoldingRanges(query::FoldingRangesRequest {
            module: query_file.module,
            file_id: query_file.file.id,
        });
        let response = self.query_module(&query_file, request).await?;
        let query::QueryResponse::FoldingRanges(response) = response.response else {
            return Err(internal_error("query did not return folding ranges"));
        };
        let ranges = response.ranges;
        if ranges.is_empty() {
            return Ok(None);
        }

        let lsp_ranges = ranges
            .into_iter()
            .map(|range| document.folding_range(range))
            .collect();

        Ok(Some(lsp_ranges))
    }

    // ------------------------------------------------------------------------
    // FORMATTING
    // ------------------------------------------------------------------------

    async fn formatting(
        &self,
        params: lsp::DocumentFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TextEdit>>> {
        let Some(edit) = self.format_file(&params.text_document.uri, None)? else {
            return Ok(None);
        };
        let document = Document::new(edit.file.clone());
        let edit = document.text_edit(edit)?;

        Ok(Some(vec![edit]))
    }

    async fn range_formatting(
        &self,
        params: lsp::DocumentRangeFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TextEdit>>> {
        let range = SourceSync::text_range(params.range);
        let Some(edit) = self.format_file(&params.text_document.uri, Some(range))? else {
            return Ok(None);
        };
        let document = Document::new(edit.file.clone());
        let edit = document.text_edit(edit)?;

        Ok(Some(vec![edit]))
    }

    async fn on_type_formatting(
        &self,
        params: lsp::DocumentOnTypeFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TextEdit>>> {
        // build the trigger range in UTF-16 editor coordinates
        let end = params.text_document_position.position;
        let trigger_width = params.ch.encode_utf16().count() as u32;
        if trigger_width == 0 {
            return Err(jsonrpc::Error::invalid_params(
                "formatting trigger is empty",
            ));
        }
        let start_character = end.character.checked_sub(trigger_width).ok_or_else(|| {
            jsonrpc::Error::invalid_params("formatting trigger starts before its source line")
        })?;
        let start = lsp::Position {
            line: end.line,
            character: start_character,
        };
        let range = SourceSync::text_range(lsp::Range { start, end });

        // format the workspace file
        let uri = &params.text_document_position.text_document.uri;
        let Some(edit) = self.format_file(uri, Some(range))? else {
            return Ok(None);
        };

        // require the exact workspace source to contain the reported trigger
        let document = Document::new(edit.file.clone());
        let start_offset = document.offset(&start)?;
        let end_offset = document.offset(&end)?;
        if edit
            .file
            .text()
            .get(start_offset as usize..end_offset as usize)
            != Some(params.ch.as_str())
        {
            return Err(jsonrpc::Error::content_modified());
        }

        // return the selected formatting edit
        let edit = document.text_edit(edit)?;

        Ok(Some(vec![edit]))
    }

    // ------------------------------------------------------------------------
    // SELECTION & NAVIGATION
    // ------------------------------------------------------------------------

    async fn selection_range(
        &self,
        params: lsp::SelectionRangeParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::SelectionRange>>> {
        let Some(query_file) = self.resolve_query_file(&params.text_document.uri)? else {
            return Ok(None);
        };
        let document = Document::new(query_file.file.clone());
        let positions: Vec<u32> = params
            .positions
            .iter()
            .map(|position| document.offset(position))
            .collect::<jsonrpc::Result<_>>()?;
        let request = query::QueryRequest::SelectionRanges(query::SelectionRangesRequest {
            module: query_file.module,
            file_id: query_file.file.id,
            offsets: positions,
        });
        let response = self.query_module(&query_file, request).await?;
        let query::QueryResponse::SelectionRanges(response) = response.response else {
            return Err(internal_error("query did not return selection ranges"));
        };
        let ranges = response.ranges;

        // build LSP selection ranges
        let lsp_ranges = ranges
            .into_iter()
            .map(|range| document.selection_range(range))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        Ok(Some(lsp_ranges))
    }

    async fn goto_implementation(
        &self,
        params: lsp::request::GotoImplementationParams,
    ) -> jsonrpc::Result<Option<lsp::request::GotoImplementationResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.resolve_query_file(uri)? else {
            return Ok(None);
        };
        let document = Document::new(query_file.file.clone());
        let offset = document.offset(&params.text_document_position_params.position)?;
        let position = query_file.position(offset);
        let request =
            query::QueryRequest::GotoImplementation(query::GotoImplementationRequest { position });
        let response = self.query_module(&query_file, request).await?;
        let revision = response.revision;
        let query::QueryResponse::GotoImplementation(response) = response.response else {
            return Err(internal_error("query did not return goto implementation"));
        };

        // build targets
        let file_ids = response
            .targets
            .iter()
            .flat_map(|target| [target.origin.span.file, target.target.span.file]);
        let documents = self.load_documents(&query_file.path, revision, file_ids)?;
        let links = response
            .targets
            .iter()
            .map(|query_target| documents.location_link(query_target))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        if links.is_empty() {
            return Ok(None);
        }

        Ok(Some(lsp::GotoDefinitionResponse::Link(links)))
    }

    // ------------------------------------------------------------------------
    // DOCUMENT LINKS
    // ------------------------------------------------------------------------

    async fn document_link(
        &self,
        params: lsp::DocumentLinkParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::DocumentLink>>> {
        let Some(query_file) = self.resolve_query_file(&params.text_document.uri)? else {
            return Ok(None);
        };
        let request = query::QueryRequest::Links(query::LinksRequest {
            module: query_file.module,
            file_id: query_file.file.id,
        });
        let response = self.query_module(&query_file, request).await?;
        let revision = response.revision;
        let query::QueryResponse::Links(response) = response.response else {
            return Err(internal_error("query did not return links"));
        };
        let links = response.links;
        if links.is_empty() {
            return Ok(None);
        }

        // load exact source and target documents
        let file_ids = links.iter().flat_map(|link| [link.range.file, link.target]);
        let documents = self.load_documents(&query_file.path, revision, file_ids)?;

        // build LSP links
        let lsp_links = links
            .iter()
            .map(|query_link| documents.link(query_link))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        Ok(Some(lsp_links))
    }

    // ------------------------------------------------------------------------
    // CODE ACTIONS
    // ------------------------------------------------------------------------

    async fn code_action(
        &self,
        params: lsp::CodeActionParams,
    ) -> jsonrpc::Result<Option<lsp::CodeActionResponse>> {
        let Some(query_file) = self.resolve_query_file(&params.text_document.uri)? else {
            return Ok(None);
        };
        let document = Document::new(query_file.file.clone());
        let context = ActionContext::try_from(&params.context)?;

        // skip unsupported action filters
        if context.excludes_all() {
            return Ok(None);
        }

        let start = document.offset(&params.range.start)?;
        let end = document.offset(&params.range.end)?;
        let range = query_file
            .range(start, end)
            .ok_or_else(|| jsonrpc::Error::invalid_params("range is reversed"))?;
        let request = query::QueryRequest::CodeActions(query::CodeActionsRequest {
            range,
            context: context.query_context(),
        });
        let response = self.query_module(&query_file, request).await?;
        let revision = response.revision;
        let query::QueryResponse::CodeActions(response) = response.response else {
            return Err(internal_error("query did not return code actions"));
        };
        let actions = response.actions;

        if actions.is_empty() {
            return Ok(None);
        }

        // build LSP actions
        let mut lsp_actions = Vec::new();
        let client_capabilities = self.client_capabilities()?;
        let is_code_action_edit_deferred = client_capabilities.supports_code_action_data
            && client_capabilities.supports_code_action_edit_resolve;
        let file_ids = if is_code_action_edit_deferred {
            Vec::new()
        } else {
            actions
                .iter()
                .flat_map(|action| action.patches.files.iter())
                .map(|file| file.file)
                .collect()
        };
        let documents = self.load_documents(&query_file.path, revision, file_ids)?;
        for action in actions.iter() {
            let (edit, data) = if is_code_action_edit_deferred {
                let continuation =
                    ActionContinuation::new(revision, &query_file.path, action.patches.clone());
                let data = continuation.into_value()?;

                (None, Some(data))
            } else if action.patches.is_empty() {
                (None, None)
            } else {
                (Some(documents.workspace_edit(&action.patches)?), None)
            };
            let lsp_action = context.code_action(action, edit, data)?;
            lsp_actions.push(lsp_action);
        }

        Ok(Some(lsp_actions))
    }

    async fn code_action_resolve(
        &self,
        mut params: lsp::CodeAction,
    ) -> jsonrpc::Result<lsp::CodeAction> {
        if params.edit.is_some() {
            return Err(jsonrpc::Error::invalid_params(
                "code action is already resolved",
            ));
        }

        let Some(data) = params.data.take() else {
            return Err(jsonrpc::Error::invalid_params(
                "code action has no continuation",
            ));
        };

        let resolved = ActionContinuation::from_value(data)?;

        let file_ids = resolved.file_ids();
        let documents = self.load_documents(&resolved.path, resolved.revision, file_ids)?;
        params.edit = Some(documents.workspace_edit(&resolved.patches)?);

        Ok(params)
    }

    // ------------------------------------------------------------------------
    // CODE LENS
    // ------------------------------------------------------------------------

    async fn code_lens(
        &self,
        params: lsp::CodeLensParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::CodeLens>>> {
        let Some(query_file) = self.resolve_query_file(&params.text_document.uri)? else {
            return Ok(None);
        };
        let document = Document::new(query_file.file.clone());
        let request = query::QueryRequest::CodeLenses(query::CodeLensesRequest {
            module: query_file.module,
            file_id: query_file.file.id,
        });
        let response = self.query_module(&query_file, request).await?;
        let query::QueryResponse::CodeLenses(response) = response.response else {
            return Err(internal_error("query did not return code lenses"));
        };
        let lenses = response.lenses;

        // build only actions implemented by the connected client bridge
        let capabilities = self.client_capabilities()?;
        let mut lsp_lenses = Vec::new();
        for lens in &lenses {
            if capabilities.supports_code_lens(&lens.action) {
                lsp_lenses.push(document.code_lens(lens)?);
            }
        }

        Ok(Some(lsp_lenses))
    }

    // ------------------------------------------------------------------------
    // INLAY HINTS
    // ------------------------------------------------------------------------

    async fn inlay_hint(
        &self,
        params: lsp::InlayHintParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::InlayHint>>> {
        let Some(query_file) = self.resolve_query_file(&params.text_document.uri)? else {
            return Ok(None);
        };
        let document = Document::new(query_file.file.clone());
        let start = document.offset(&params.range.start)?;
        let end = document.offset(&params.range.end)?;
        let range = query_file
            .range(start, end)
            .ok_or_else(|| jsonrpc::Error::invalid_params("range is reversed"))?;
        let type_hints = self.settings()?.type_inlay_hints();
        let parameter_hints = self.settings()?.parameter_inlay_hints();
        let request = query::QueryRequest::InlayHints(query::InlayHintsRequest {
            range,
            type_hints,
            parameter_hints,
        });
        let response = self.query_module(&query_file, request).await?;
        let query::QueryResponse::InlayHints(response) = response.response else {
            return Err(internal_error("query did not return inlay hints"));
        };
        let hints = response.hints;

        // build LSP hints
        let lsp_hints = hints
            .iter()
            .map(|query_hint| document.inlay_hint(query_hint))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        Ok(Some(lsp_hints))
    }

    // ------------------------------------------------------------------------
    // RENAME
    // ------------------------------------------------------------------------

    async fn prepare_rename(
        &self,
        params: lsp::TextDocumentPositionParams,
    ) -> jsonrpc::Result<Option<lsp::PrepareRenameResponse>> {
        let Some(query_file) = self.resolve_query_file(&params.text_document.uri)? else {
            return Ok(None);
        };
        let document = Document::new(query_file.file.clone());
        let offset = document.offset(&params.position)?;
        let position = query_file.position(offset);
        let request = query::QueryRequest::RenameTarget(query::RenameTargetRequest { position });
        let response = self.query_module(&query_file, request).await?;
        let query::QueryResponse::RenameTarget(response) = response.response else {
            return Err(internal_error("query did not return rename target"));
        };
        let Some(target) = response.target else {
            return Ok(None);
        };

        // build LSP rename range
        let range = document.range(target.target.span)?;
        Ok(Some(lsp::PrepareRenameResponse::RangeWithPlaceholder {
            range,
            placeholder: target.placeholder,
        }))
    }

    async fn rename(
        &self,
        params: lsp::RenameParams,
    ) -> jsonrpc::Result<Option<lsp::WorkspaceEdit>> {
        let Some(query_file) =
            self.resolve_query_file(&params.text_document_position.text_document.uri)?
        else {
            return Ok(None);
        };
        let document = Document::new(query_file.file.clone());
        let offset = document.offset(&params.text_document_position.position)?;
        let position = query_file.position(offset);
        let request = query::QueryRequest::Rename(query::RenameRequest {
            position,
            new_name: params.new_name.clone(),
        });
        let response = self.query_module(&query_file, request).await?;
        let revision = response.revision;
        let query::QueryResponse::Rename(response) = response.response else {
            return Err(internal_error("query did not return rename"));
        };
        let Some(edit) = response.edit else {
            return Ok(None);
        };

        // build LSP workspace edit
        let file_ids = edit.files.iter().map(|file| file.file);
        let documents = self.load_documents(&query_file.path, revision, file_ids)?;
        let workspace_edit = documents.workspace_edit(&edit)?;

        Ok(Some(workspace_edit))
    }

    // ------------------------------------------------------------------------
    // CALL HIERARCHY
    // ------------------------------------------------------------------------

    async fn prepare_call_hierarchy(
        &self,
        params: lsp::CallHierarchyPrepareParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::CallHierarchyItem>>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.resolve_query_file(uri)? else {
            return Ok(None);
        };
        let document = Document::new(query_file.file.clone());
        let offset = document.offset(&params.text_document_position_params.position)?;
        let position = query_file.position(offset);
        let request = query::QueryRequest::CallItem(query::CallItemRequest { position });
        let response = self.query_module(&query_file, request).await?;
        let revision = response.revision;
        let query::QueryResponse::CallItem(response) = response.response else {
            return Err(internal_error("query did not return call item"));
        };
        let Some(item) = response.item else {
            return Ok(None);
        };

        // build LSP call item
        let documents = self.load_documents(&query_file.path, revision, [item.target.span.file])?;
        let lsp_item = documents.call_hierarchy_item(&query_file.path, revision, &item)?;

        Ok(Some(vec![lsp_item]))
    }

    async fn incoming_calls(
        &self,
        params: lsp::CallHierarchyIncomingCallsParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::CallHierarchyIncomingCall>>> {
        // read the hierarchy continuation
        let continuation =
            HierarchyContinuation::<query::CallItem>::from_value(params.item.data.as_ref())?;
        let HierarchyContinuation {
            path,
            revision,
            item,
        } = continuation;

        // query incoming calls
        let request = query::QueryRequest::IncomingCalls(query::IncomingCallsRequest { item });
        let response = self
            .query_program(&path, request, RevisionPolicy::Exact(revision))
            .await?;
        let revision = response.revision;
        let query::QueryResponse::IncomingCalls(response) = response.response else {
            return Err(internal_error("query did not return incoming calls"));
        };
        let calls = response.calls;

        // build LSP incoming calls
        let file_ids = calls.iter().map(|call| call.from.target.span.file);
        let documents = self.load_documents(&path, revision, file_ids)?;
        let lsp_calls: Vec<lsp::CallHierarchyIncomingCall> = calls
            .iter()
            .map(|call| documents.incoming_call(&path, revision, call))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        Ok(Some(lsp_calls))
    }

    async fn outgoing_calls(
        &self,
        params: lsp::CallHierarchyOutgoingCallsParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::CallHierarchyOutgoingCall>>> {
        // read the hierarchy continuation
        let continuation =
            HierarchyContinuation::<query::CallItem>::from_value(params.item.data.as_ref())?;
        let HierarchyContinuation {
            path,
            revision,
            item,
        } = continuation;
        let source_file_id = item.target.span.file;

        // query outgoing calls
        let request = query::QueryRequest::OutgoingCalls(query::OutgoingCallsRequest { item });
        let response = self
            .query_program(&path, request, RevisionPolicy::Exact(revision))
            .await?;
        let revision = response.revision;
        let query::QueryResponse::OutgoingCalls(response) = response.response else {
            return Err(internal_error("query did not return outgoing calls"));
        };
        let calls = response.calls;

        // build LSP outgoing calls
        let file_ids =
            iter::once(source_file_id).chain(calls.iter().map(|call| call.to.target.span.file));
        let documents = self.load_documents(&path, revision, file_ids)?;
        let document = documents.document(source_file_id)?;
        let lsp_calls: Vec<lsp::CallHierarchyOutgoingCall> = calls
            .iter()
            .map(|call| documents.outgoing_call(&path, revision, call, document))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        Ok(Some(lsp_calls))
    }

    // ------------------------------------------------------------------------
    // TYPE HIERARCHY
    // ------------------------------------------------------------------------

    async fn prepare_type_hierarchy(
        &self,
        params: lsp::TypeHierarchyPrepareParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TypeHierarchyItem>>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.resolve_query_file(uri)? else {
            return Ok(None);
        };
        let document = Document::new(query_file.file.clone());
        let offset = document.offset(&params.text_document_position_params.position)?;
        let position = query_file.position(offset);
        let request = query::QueryRequest::TypeItem(query::TypeItemRequest { position });
        let response = self.query_module(&query_file, request).await?;
        let revision = response.revision;
        let query::QueryResponse::TypeItem(response) = response.response else {
            return Err(internal_error("query did not return type item"));
        };
        let Some(item) = response.item else {
            return Ok(None);
        };

        let documents = self.load_documents(&query_file.path, revision, [item.target.span.file])?;
        let lsp_item = documents.type_hierarchy_item(&query_file.path, revision, &item)?;

        Ok(Some(vec![lsp_item]))
    }

    async fn supertypes(
        &self,
        params: lsp::TypeHierarchySupertypesParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TypeHierarchyItem>>> {
        // read the hierarchy continuation
        let continuation =
            HierarchyContinuation::<query::TypeItem>::from_value(params.item.data.as_ref())?;
        let HierarchyContinuation {
            path,
            revision,
            item,
        } = continuation;

        // query supertypes
        let request = query::QueryRequest::Supertypes(query::SupertypesRequest { item });
        let response = self
            .query_program(&path, request, RevisionPolicy::Exact(revision))
            .await?;
        let revision = response.revision;
        let query::QueryResponse::Supertypes(response) = response.response else {
            return Err(internal_error("query did not return supertypes"));
        };
        let supertypes = response.items;

        // build LSP supertypes
        let file_ids = supertypes.iter().map(|item| item.target.span.file);
        let documents = self.load_documents(&path, revision, file_ids)?;
        let lsp_items: Vec<lsp::TypeHierarchyItem> = supertypes
            .iter()
            .map(|item| documents.type_hierarchy_item(&path, revision, item))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        Ok(Some(lsp_items))
    }

    async fn subtypes(
        &self,
        params: lsp::TypeHierarchySubtypesParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TypeHierarchyItem>>> {
        // read the hierarchy continuation
        let continuation =
            HierarchyContinuation::<query::TypeItem>::from_value(params.item.data.as_ref())?;
        let HierarchyContinuation {
            path,
            revision,
            item,
        } = continuation;

        // query subtypes
        let request = query::QueryRequest::Subtypes(query::SubtypesRequest { item });
        let response = self
            .query_program(&path, request, RevisionPolicy::Exact(revision))
            .await?;
        let revision = response.revision;
        let query::QueryResponse::Subtypes(response) = response.response else {
            return Err(internal_error("query did not return subtypes"));
        };
        let subtypes = response.items;

        // build LSP subtypes
        let file_ids = subtypes.iter().map(|item| item.target.span.file);
        let documents = self.load_documents(&path, revision, file_ids)?;
        let lsp_items: Vec<lsp::TypeHierarchyItem> = subtypes
            .iter()
            .map(|item| documents.type_hierarchy_item(&path, revision, item))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        Ok(Some(lsp_items))
    }
}
