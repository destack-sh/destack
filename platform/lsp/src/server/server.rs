use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant};

use dashmap::{DashMap, DashSet};
use destack_lsp_server::{Client, LanguageServer, UriExt, jsonrpc};
use destack_lsp_types as lsp;
use destack_resolver::{ResolveOptions, Resolver};
use destack_service::{
    LanguageService as LspLanguageService, RescanReason, WorkspaceMessage,
    WorkspaceMessageKind as ProtocolMessageKind, WorkspaceUpdateRecord, query,
};
use destack_source::{
    BatchEdit, Diagnostic, File, FileId, FileSystem, FileWatchEvent, FileWatchEventKind,
    OverlayFileSystem, PhysicalFileSystem, Uri,
};
use destack_workspace::{Session, Workspace, WorkspaceKind};
use serde::{Deserialize, Serialize};
use serde_json::{from_value, to_value};

use crate::query::assist::{code_lens_to_lsp, inlay_hint_to_lsp};
use crate::query::common::{byte_span_to_range, position_to_byte, span_to_location};
use crate::query::diagnostic::{code_action_to_lsp, diagnostic_to_lsp_diagnostic};
use crate::query::navigation::{
    call_hierarchy_item_to_lsp, definition_to_location, document_highlight_to_lsp,
    document_link_to_lsp, document_symbol_to_lsp, implementation_to_location, incoming_call_to_lsp,
    outgoing_call_to_lsp, query_call_hierarchy_item_from_lsp, query_type_hierarchy_item_from_lsp,
    selection_range_to_lsp, type_hierarchy_item_to_lsp, workspace_symbol_to_lsp,
};
use crate::query::refactor::batch_edit_to_workspace_edit;
use crate::query::semantic;
use crate::server::file::{
    apply_text_changes, build_file_watchers, completion_kind_to_lsp, create_workspace_service,
    diagnostic_result_id, format_file, format_range, normalize_line_endings, tracked_file_globs,
    upsert_file_from_snapshot,
};
use crate::server::progress::WorkDoneProgressTracker;
use crate::server::token::semantic_tokens_edits;
use crate::uri::lsp_uri_for_file;

const PARTIAL_RESULT_CHUNK_SIZE: usize = 128;
const WORKSPACE_DIAGNOSTIC_PARTIAL_CHUNK_SIZE: usize = 128;
const AUTO_IMPORT_DETAIL_PREFIX: &str = "Auto import from ";
const SLOW_DIAGNOSTIC_WARN_DURATION: Duration = Duration::from_secs(2);

/// State for an open document.
#[derive(Debug)]
struct OpenDocument {
    /// The file id in the session registry.
    file_id: FileId,
    /// The current in editor document text normalized to LF.
    text: String,
    /// The current LSP version for the document.
    version: i32,
}

/// Additional information used when resolving completion items.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompletionResolveData {
    /// The documentation payload for the completion item.
    documentation: Option<String>,
}

/// Additional information used when resolving code actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodeActionResolveData {
    /// The workspace edits for the selected code action.
    edits: BatchEdit,
}

/// Configuration for completion behavior.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompletionSettings {
    /// Whether auto import completions are enabled.
    auto_imports: Option<bool>,
}

impl Default for CompletionSettings {
    fn default() -> Self {
        Self {
            auto_imports: Some(true),
        }
    }
}

/// Configuration for inlay hint behavior.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InlayHintSettings {
    /// Whether parameter name hints are enabled.
    parameter_hints: Option<bool>,
    /// Whether inferred type hints are enabled.
    type_hints: Option<bool>,
}

impl Default for InlayHintSettings {
    fn default() -> Self {
        Self {
            parameter_hints: Some(true),
            type_hints: Some(true),
        }
    }
}

/// LSP configuration settings.
#[derive(Debug, Clone, Default)]
struct LspSettings {
    /// Completion-related settings.
    completion: CompletionSettings,
    /// Inlay hint related settings.
    inlay_hints: InlayHintSettings,
}

/// Cached semantic tokens for a document.
#[derive(Debug, Clone)]
struct SemanticTokensCache {
    /// The current result id for the cached tokens.
    result_id: String,
    /// The last token payload.
    data: Vec<lsp::SemanticToken>,
    /// The hash of the cached token payload.
    hash: u64,
}

/// The Destack language server.
#[derive(Debug)]
pub struct DestackLanguageServer {
    /// The client connection.
    pub(super) client: Client,
    /// The overlay file system.
    overlay_fs: Arc<OverlayFileSystem>,
    /// The session.
    session: OnceLock<Arc<Session>>,
    /// The workspace service.
    workspace_service: OnceLock<Arc<LspLanguageService>>,
    /// The open documents.
    open_documents: DashMap<String, OpenDocument>,
    /// The latest diagnostics for open virtual documents.
    virtual_diagnostics: DashMap<FileId, Vec<Diagnostic>>,
    /// Cached semantic tokens per document.
    semantic_tokens_cache: DashMap<String, SemanticTokensCache>,
    /// Monotonic counter for semantic token result ids.
    semantic_tokens_counter: AtomicU64,
    /// Progress tokens canceled by the client.
    cancelled_progress_tokens: DashSet<lsp::ProgressToken>,
    /// The watch registration ID.
    watch_registration_id: OnceLock<String>,
    /// Whether completion label details are supported by the client.
    completion_label_details_supported: OnceLock<bool>,
    /// Whether code action data payloads are supported by the client.
    code_action_data_supported: OnceLock<bool>,
    /// Whether code action edit payloads can be resolved lazily.
    code_action_edit_resolve_supported: OnceLock<bool>,
    /// LSP configuration settings.
    settings: RwLock<LspSettings>,
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
            workspace_service: OnceLock::new(),
            open_documents: DashMap::new(),
            virtual_diagnostics: DashMap::new(),
            semantic_tokens_cache: DashMap::new(),
            semantic_tokens_counter: AtomicU64::new(1),
            cancelled_progress_tokens: DashSet::new(),
            watch_registration_id: OnceLock::new(),
            completion_label_details_supported: OnceLock::new(),
            code_action_data_supported: OnceLock::new(),
            code_action_edit_resolve_supported: OnceLock::new(),
            settings: RwLock::new(LspSettings::default()),
        }
    }

    /// Get the session (must be called after initialize).
    #[inline]
    fn session(&self) -> &Arc<Session> {
        self.session.get().expect("session not initialized")
    }

    /// Get the workspace service (must be called after initialize).
    #[inline]
    fn workspace_service(&self) -> &Arc<LspLanguageService> {
        self.workspace_service
            .get()
            .expect("workspace service not initialized")
    }

    /// Allocate the next semantic tokens result id.
    #[inline]
    fn next_semantic_tokens_result_id(&self) -> String {
        self.semantic_tokens_counter
            .fetch_add(1, Ordering::Relaxed)
            .to_string()
    }

    /// Compute a hash for semantic tokens.
    fn semantic_tokens_hash(tokens: &[lsp::SemanticToken]) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        tokens.len().hash(&mut hasher);
        for token in tokens {
            token.delta_line.hash(&mut hasher);
            token.delta_start.hash(&mut hasher);
            token.length.hash(&mut hasher);
            token.token_type.hash(&mut hasher);
            token.token_modifiers_bitset.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Check whether completion label details are supported.
    fn completion_label_details_supported(&self) -> bool {
        self.completion_label_details_supported
            .get()
            .copied()
            .unwrap_or(false)
    }

    /// Check whether code action data payloads are supported.
    fn code_action_data_supported(&self) -> bool {
        self.code_action_data_supported
            .get()
            .copied()
            .unwrap_or(false)
    }

    /// Check whether code action edit payloads can be resolved lazily.
    fn code_action_edit_resolve_supported(&self) -> bool {
        self.code_action_edit_resolve_supported
            .get()
            .copied()
            .unwrap_or(false)
    }

    /// Check whether auto import completions are enabled.
    fn completion_auto_imports_enabled(&self) -> bool {
        self.settings
            .read()
            .map(|settings| settings.completion.auto_imports.unwrap_or(true))
            .unwrap_or(true)
    }

    /// Check whether parameter name inlay hints are enabled.
    fn parameter_inlay_hints_enabled(&self) -> bool {
        self.settings
            .read()
            .map(|settings| settings.inlay_hints.parameter_hints.unwrap_or(true))
            .unwrap_or(true)
    }

    /// Check whether inferred type inlay hints are enabled.
    fn type_inlay_hints_enabled(&self) -> bool {
        self.settings
            .read()
            .map(|settings| settings.inlay_hints.type_hints.unwrap_or(true))
            .unwrap_or(true)
    }

    /// Build commit characters for completion items.
    fn completion_commit_characters(kind: query::CompletionKind) -> Option<Vec<String>> {
        let commit_characters: &[&str] = match kind {
            query::CompletionKind::Method
            | query::CompletionKind::Function
            | query::CompletionKind::Constructor
            | query::CompletionKind::Field
            | query::CompletionKind::Variable
            | query::CompletionKind::Class
            | query::CompletionKind::Interface
            | query::CompletionKind::Module
            | query::CompletionKind::Property
            | query::CompletionKind::Enum
            | query::CompletionKind::EnumMember
            | query::CompletionKind::Struct
            | query::CompletionKind::Constant
            | query::CompletionKind::TypeParameter => &[".", ",", ";", "("],
            query::CompletionKind::Keyword => &[" ", ";"],
            _ => &[],
        };

        if commit_characters.is_empty() {
            return None;
        }

        Some(
            commit_characters
                .iter()
                .map(|character| (*character).to_string())
                .collect(),
        )
    }

    /// Refresh configuration from the client.
    async fn refresh_configuration(&self) {
        let items = vec![
            lsp::ConfigurationItem {
                scope_uri: None,
                section: Some("destack.completion".to_string()),
            },
            lsp::ConfigurationItem {
                scope_uri: None,
                section: Some("destack.inlayHints".to_string()),
            },
        ];
        let values = match self.client.configuration(items).await {
            Ok(values) => values,
            Err(error) => {
                tracing::debug!(?error, "lsp.config.fetch_failed");
                return;
            }
        };
        if values.is_empty() {
            return;
        }

        let Ok(mut settings) = self.settings.write() else {
            return;
        };

        // update completion settings when parsing succeeds
        if let Some(value) = values.first().cloned() {
            match from_value::<CompletionSettings>(value) {
                Ok(parsed) => {
                    if let Some(auto_imports) = parsed.auto_imports {
                        settings.completion.auto_imports = Some(auto_imports);
                    }
                }
                Err(error) => {
                    tracing::debug!(?error, "lsp.config.parse_completion_failed");
                }
            }
        }

        // update inlay hint settings when parsing succeeds
        if let Some(value) = values.get(1).cloned() {
            match from_value::<InlayHintSettings>(value) {
                Ok(parsed) => {
                    if let Some(parameter_hints) = parsed.parameter_hints {
                        settings.inlay_hints.parameter_hints = Some(parameter_hints);
                    }
                    if let Some(type_hints) = parsed.type_hints {
                        settings.inlay_hints.type_hints = Some(type_hints);
                    }
                }
                Err(error) => {
                    tracing::debug!(?error, "lsp.config.parse_inlay_failed");
                }
            }
        }
    }

    /// Check if a progress token has been cancelled.
    pub(super) fn is_progress_cancelled(&self, token: &lsp::ProgressToken) -> bool {
        self.cancelled_progress_tokens.contains(token)
    }

    /// Clear a cancelled progress token.
    pub(super) fn clear_progress_cancel(&self, token: &lsp::ProgressToken) {
        self.cancelled_progress_tokens.remove(token);
    }

    /// Load a file snapshot for query operations.
    fn get_query_file(&self, file_id: FileId) -> Option<Arc<File>> {
        let file = self.session().files.get_maybe(file_id);
        if file.is_none() {
            tracing::debug!(?file_id, "lsp.query.file_not_found");
        }

        file
    }

    /// Load a file snapshot for an open document.
    fn get_query_file_for_open_document(&self, doc: &OpenDocument) -> Option<Arc<File>> {
        self.get_query_file(doc.file_id)
    }

    /// Execute a workspace query through the workspace for a filesystem path.
    fn query_for_path(
        &self,
        path: &Path,
        request: query::QueryRequest,
    ) -> Option<query::QueryResponse> {
        match self.workspace_service().query_for_path(path, request) {
            Ok(response) => Some(response.response),
            Err(error) => {
                tracing::debug!(?error, path = ?path, "lsp.query.workspace_failed");
                None
            }
        }
    }

    /// Execute a workspace query through the workspace for an lsp uri.
    fn query_for_uri(
        &self,
        uri: &lsp::Uri,
        request: query::QueryRequest,
    ) -> Option<query::QueryResponse> {
        let path = uri.to_file_path().map(|path| path.into_owned())?;
        self.query_for_path(&path, request)
    }

    /// Execute a workspace query through the workspace using the workspace root.
    fn query_for_workspace(&self, request: query::QueryRequest) -> Option<query::QueryResponse> {
        let root = self.session().workspace_root();
        self.query_for_path(&root, request)
    }

    /// Build a source uri from an lsp uri.
    fn query_uri(uri: &lsp::Uri) -> Uri {
        if let Some(path) = uri.to_file_path().map(|path| path.into_owned()) {
            return Uri::from_file_path(path);
        }

        Uri::from_string(uri.to_string())
    }

    /// Convert an LSP code action kind into workspace query kinds.
    fn query_code_action_kinds(kind: &lsp::CodeActionKind) -> Vec<query::CodeActionKind> {
        let kind_name = kind.as_str();

        // quick fixes and sub kinds
        if kind_name == lsp::CodeActionKind::QUICKFIX.as_str() || kind_name.starts_with("quickfix.")
        {
            return vec![query::CodeActionKind::QuickFix];
        }

        // extract refactors and sub kinds
        if kind_name == lsp::CodeActionKind::REFACTOR_EXTRACT.as_str()
            || kind_name.starts_with("refactor.extract.")
        {
            return vec![query::CodeActionKind::RefactorExtract];
        }

        // inline refactors and sub kinds
        if kind_name == lsp::CodeActionKind::REFACTOR_INLINE.as_str()
            || kind_name.starts_with("refactor.inline.")
        {
            return vec![query::CodeActionKind::RefactorInline];
        }

        // rewrite refactors and sub kinds
        if kind_name == lsp::CodeActionKind::REFACTOR_REWRITE.as_str()
            || kind_name.starts_with("refactor.rewrite.")
        {
            return vec![query::CodeActionKind::RefactorRewrite];
        }

        // umbrella refactor kinds
        if kind_name == lsp::CodeActionKind::REFACTOR.as_str() || kind_name.starts_with("refactor.")
        {
            return vec![
                query::CodeActionKind::Refactor,
                query::CodeActionKind::RefactorExtract,
                query::CodeActionKind::RefactorInline,
                query::CodeActionKind::RefactorRewrite,
            ];
        }

        // organize import source kinds
        if kind_name == lsp::CodeActionKind::SOURCE_ORGANIZE_IMPORTS.as_str()
            || kind_name.starts_with("source.organizeImports.")
        {
            return vec![query::CodeActionKind::SourceOrganizeImports];
        }

        // fix all source kinds
        if kind_name == lsp::CodeActionKind::SOURCE_FIX_ALL.as_str()
            || kind_name.starts_with("source.fixAll.")
        {
            return vec![query::CodeActionKind::SourceFixAll];
        }

        // umbrella source kinds
        if kind_name == lsp::CodeActionKind::SOURCE.as_str() || kind_name.starts_with("source.") {
            return vec![
                query::CodeActionKind::Source,
                query::CodeActionKind::SourceOrganizeImports,
                query::CodeActionKind::SourceFixAll,
            ];
        }

        Vec::new()
    }

    /// Build query code action context from LSP code action context.
    fn query_code_action_context(context: &lsp::CodeActionContext) -> query::CodeActionContext {
        let mut only = Vec::new();
        let mut seen = HashSet::new();

        // map each requested lsp kind to workspace code action kinds
        if let Some(kinds) = context.only.as_ref() {
            for kind in kinds {
                for query_kind in Self::query_code_action_kinds(kind) {
                    if seen.insert(query_kind) {
                        only.push(query_kind);
                    }
                }
            }
        }

        query::CodeActionContext {
            only,
            include_disabled: context.only.as_ref().is_some_and(|kinds| !kinds.is_empty()),
        }
    }

    /// Register file watchers with the client.
    async fn register_file_watchers(&self) {
        if self.watch_registration_id.get().is_some() {
            return;
        }

        let watchers = build_file_watchers();
        let options = lsp::DidChangeWatchedFilesRegistrationOptions { watchers };
        let register_options = match to_value(options) {
            Ok(value) => Some(value),
            Err(error) => {
                tracing::debug!(?error, "lsp.watch.register.serialize_failed");
                return;
            }
        };
        let registration = lsp::Registration {
            id: "destack.watch".to_string(),
            method: "workspace/didChangeWatchedFiles".to_string(),
            register_options,
        };

        match self.client.register_capability(vec![registration]).await {
            Ok(()) => {
                if self
                    .watch_registration_id
                    .set("destack.watch".to_string())
                    .is_err()
                {
                    tracing::debug!("lsp.watch.register_id_already_set");
                }
            }
            Err(error) => {
                tracing::debug!(?error, "lsp.watch.register_failed");
            }
        }
    }

    /// Apply watch events and publish diagnostics.
    async fn apply_watch_events(&self, events: Vec<FileWatchEvent>) {
        // skip empty batches
        if events.is_empty() {
            return;
        }

        // apply updates through the workspace
        let workspace_service = self.workspace_service().clone();
        let result = match workspace_service.apply_watch_events(events) {
            Ok(result) => result,
            Err(error) => {
                tracing::debug!(?error, "lsp.watch.apply_failed");
                return;
            }
        };

        tracing::trace!(
            updates = result.updates.len(),
            rescan = false,
            "lsp.watch.apply"
        );

        // publish diagnostics for updates
        self.publish_watch_updates(result.updates).await;
        self.publish_watch_messages(result.messages).await;

        if let Err(error) = self.client.workspace_diagnostic_refresh().await {
            tracing::debug!(?error, "lsp.watch.refresh_diagnostics_failed");
        }
        if let Err(error) = self.client.semantic_tokens_refresh().await {
            tracing::debug!(?error, "lsp.watch.refresh_semantic_tokens_failed");
        }
    }

    /// Publish diagnostics for a batch of workspace updates.
    async fn publish_watch_updates(&self, updates: Vec<WorkspaceUpdateRecord>) {
        // publish diagnostics per updated file
        let session = self.session().clone();
        for update in updates {
            let Some(file) = upsert_file_from_snapshot(session.as_ref(), &update.file) else {
                continue;
            };
            let Some(uri) = lsp_uri_for_file(&file) else {
                continue;
            };
            let diagnostics: Vec<lsp::Diagnostic> = update
                .diagnostics
                .into_iter()
                .map(|diagnostic| diagnostic_to_lsp_diagnostic(&diagnostic, &file))
                .collect();
            self.client
                .publish_diagnostics(uri, diagnostics, None)
                .await;
        }
    }

    /// Publish watch warnings for a batch.
    async fn publish_watch_messages(&self, messages: Vec<WorkspaceMessage>) {
        for message in messages {
            let message_type = match message.kind {
                ProtocolMessageKind::Info => lsp::MessageType::INFO,
                ProtocolMessageKind::Warning => lsp::MessageType::WARNING,
                ProtocolMessageKind::Error => lsp::MessageType::ERROR,
            };
            self.client.log_message(message_type, message.message).await;
        }
    }

    /// Publish a partial result payload.
    async fn publish_partial_result<T: Serialize>(
        &self,
        token: &lsp::ProgressToken,
        payload: T,
        label: &str,
    ) {
        // serialize the partial result
        let value = match to_value(payload) {
            Ok(value) => value,
            Err(error) => {
                tracing::debug!(?error, label, "lsp.partial.serialize_failed");
                return;
            }
        };

        // dispatch the progress notification
        self.client
            .send_notification::<lsp::notification::Progress>(lsp::ProgressParams {
                token: token.clone(),
                value: lsp::ProgressParamsValue::PartialResult(value),
            })
            .await;
    }

    /// Publish a partial workspace diagnostics payload.
    async fn publish_workspace_diagnostic_partial(
        &self,
        token: &lsp::ProgressToken,
        items: Vec<lsp::WorkspaceDocumentDiagnosticReport>,
    ) {
        // skip empty payloads
        if items.is_empty() {
            return;
        }

        let report = lsp::WorkspaceDiagnosticReportPartialResult { items };
        self.publish_partial_result(token, report, "diagnostic")
            .await;
    }

    /// Invalidate a module at the given path and publish diagnostics.
    async fn invalidate_and_publish(
        &self,
        uri: &lsp::Uri,
        path: &std::path::Path,
        content: String,
    ) {
        // apply the virtual file update through the workspace
        let result = match self.workspace_service().update_virtual_file(path, content) {
            Ok(updates) => updates,
            Err(error) => {
                tracing::debug!(?error, "lsp.invalidate.file");
                return;
            }
        };

        // gather diagnostics for updated files
        let session = self.session().clone();
        let program = session.find_program_for_path(path);
        let mut diagnostics_by_file = program.diagnostic_store.snapshot_by_file();

        // publish diagnostics for the updated files
        let primary_path = path;
        for mut update in result.updates {
            let Some(file) = upsert_file_from_snapshot(session.as_ref(), &update.file) else {
                continue;
            };
            if update.diagnostics.is_empty() {
                update.diagnostics = diagnostics_by_file.remove(&file.id).unwrap_or_default();
            }

            // cache diagnostics for pull based clients while the document is open
            self.virtual_diagnostics
                .insert(file.id, update.diagnostics.clone());

            let uri = if update
                .file
                .path
                .as_ref()
                .is_some_and(|update_path| update_path == primary_path)
            {
                Some(uri.clone())
            } else {
                lsp_uri_for_file(&file)
            };
            let Some(uri) = uri else {
                continue;
            };
            let diagnostics: Vec<lsp::Diagnostic> = update
                .diagnostics
                .into_iter()
                .map(|diagnostic| diagnostic_to_lsp_diagnostic(&diagnostic, &file))
                .collect();
            self.client
                .publish_diagnostics(uri, diagnostics, None)
                .await;
        }

        self.publish_watch_messages(result.messages).await;
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
        let session = Session::new(cwd.clone()).with_fs(self.overlay_fs.clone());

        // discover workspace and attach configuration
        let resolver = Resolver::from_session(&session, ResolveOptions::default());
        let workspace = match resolver.discover_workspace(&cwd) {
            Ok(workspace) => workspace,
            Err(error) => {
                self.client
                    .log_message(
                        lsp::MessageType::WARNING,
                        format!(
                            "destack.initialize.workspace_discovery_failed cwd={} error={error}",
                            cwd.display()
                        ),
                    )
                    .await;
                Workspace::single_package(cwd.clone())
            }
        };
        let workspace_kind = match workspace.kind {
            WorkspaceKind::Monorepo => "monorepo",
            WorkspaceKind::SinglePackage => "single-package",
        };
        let package_count = workspace.package_paths.len();
        let package_paths = workspace
            .package_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>();
        let root = workspace.root.clone();
        self.client
            .log_message(
                lsp::MessageType::INFO,
                format!(
                    "destack.initialize.workspace root={} kind={workspace_kind} packages={package_count}",
                    root.display()
                ),
            )
            .await;
        for package_path in &package_paths {
            self.client
                .log_message(
                    lsp::MessageType::INFO,
                    format!("destack.initialize.package {package_path}"),
                )
                .await;
        }
        let session = Arc::new(session.with_workspace(workspace));
        session.add_root(root.clone());
        if self.session.set(session.clone()).is_err() {
            tracing::warn!("lsp.initialize.session_already_set");
            return Err(jsonrpc::Error::internal_error());
        }

        // create workspace service for the session
        let workspace_service = match create_workspace_service(session.clone(), root.clone()) {
            Ok(workspace_service) => Arc::new(workspace_service),
            Err(error) => {
                tracing::debug!(?error, "lsp.workspace.init_failed");
                return Err(jsonrpc::Error::internal_error());
            }
        };
        if self.workspace_service.set(workspace_service).is_err() {
            tracing::warn!("lsp.initialize.workspace_already_set");
            return Err(jsonrpc::Error::internal_error());
        }

        // record client completion capabilities
        let label_details_supported = params
            .capabilities
            .text_document
            .as_ref()
            .and_then(|text| text.completion.as_ref())
            .and_then(|completion| completion.completion_item.as_ref())
            .and_then(|item| item.label_details_support)
            .unwrap_or(false);
        let _ = self
            .completion_label_details_supported
            .set(label_details_supported);

        // record client code action capabilities
        let code_action_capabilities = params
            .capabilities
            .text_document
            .as_ref()
            .and_then(|text| text.code_action.as_ref());
        let code_action_data_supported = code_action_capabilities
            .and_then(|capabilities| capabilities.data_support)
            .unwrap_or(false);
        let code_action_edit_resolve_supported = code_action_capabilities
            .and_then(|capabilities| capabilities.resolve_support.as_ref())
            .is_some_and(|resolve| resolve.properties.iter().any(|property| property == "edit"));
        let _ = self
            .code_action_data_supported
            .set(code_action_data_supported);
        let _ = self
            .code_action_edit_resolve_supported
            .set(code_action_edit_resolve_supported);

        // build file operation filters for workspace notifications
        let file_operation_filters: Vec<lsp::FileOperationFilter> = tracked_file_globs()
            .into_iter()
            .map(|glob| lsp::FileOperationFilter {
                scheme: None,
                pattern: lsp::FileOperationPattern {
                    glob: glob.to_string(),
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
                lsp::TextDocumentSyncKind::INCREMENTAL,
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
                resolve_provider: Some(true),
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
                        full: Some(lsp::SemanticTokensFullOptions::Delta { delta: Some(true) }),
                        work_done_progress_options: lsp::WorkDoneProgressOptions::default(),
                    },
                ),
            ),
            diagnostic_provider: Some(lsp::DiagnosticServerCapabilities::Options(
                lsp::DiagnosticOptions {
                    identifier: Some("destack".to_string()),
                    inter_file_dependencies: true,
                    workspace_diagnostics: true,
                    markup_message_support: None,
                    work_done_progress_options: lsp::WorkDoneProgressOptions::default(),
                },
            )),
            document_formatting_provider: Some(lsp::OneOf::Left(true)),
            document_range_formatting_provider: Some(lsp::OneOf::Left(true)),
            folding_range_provider: Some(lsp::FoldingRangeProviderCapability::Simple(true)),
            selection_range_provider: Some(lsp::SelectionRangeProviderCapability::Simple(true)),
            document_link_provider: Some(lsp::DocumentLinkOptions {
                resolve_provider: Some(true),
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
                        lsp::CodeActionKind::REFACTOR_REWRITE,
                        lsp::CodeActionKind::SOURCE,
                        lsp::CodeActionKind::SOURCE_ORGANIZE_IMPORTS,
                        lsp::CodeActionKind::SOURCE_FIX_ALL,
                    ]),
                    resolve_provider: Some(true),
                    work_done_progress_options: Default::default(),
                },
            )),
            code_lens_provider: Some(lsp::CodeLensOptions {
                resolve_provider: Some(true),
            }),
            inlay_hint_provider: Some(lsp::OneOf::Left(true)),
            implementation_provider: Some(lsp::ImplementationProviderCapability::Simple(true)),
            call_hierarchy_provider: Some(lsp::CallHierarchyServerCapability::Simple(true)),
            type_hierarchy_provider: Some(lsp::OneOf::Left(true)),
            execute_command_provider: Some(lsp::ExecuteCommandOptions {
                commands: vec![
                    "destack.rescan".to_string(),
                    "destack.reindex".to_string(),
                    "destack.clearCache".to_string(),
                ],
                work_done_progress_options: Default::default(),
            }),
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
                    will_rename: Some(lsp::FileOperationRegistrationOptions {
                        filters: file_operation_filters.clone(),
                    }),
                    will_delete: Some(lsp::FileOperationRegistrationOptions {
                        filters: file_operation_filters.clone(),
                    }),
                }),
                text_document_content: None,
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
            offset_encoding: None,
        })
    }

    async fn initialized(&self, _: lsp::InitializedParams) {
        self.client
            .log_message(lsp::MessageType::INFO, "destack.initialized")
            .await;

        self.register_file_watchers().await;
        self.refresh_configuration().await;
    }

    async fn did_change_configuration(&self, _: lsp::DidChangeConfigurationParams) {
        self.refresh_configuration().await;

        let Some(workspace_service) = self.workspace_service.get() else {
            return;
        };

        // rescan workspaces so config changes refresh diagnostics
        let result = match workspace_service.rescan_all(RescanReason::Manual) {
            Ok(result) => result,
            Err(error) => {
                tracing::debug!(?error, "lsp.config.rescan_failed");
                return;
            }
        };

        self.publish_watch_updates(result.updates).await;
        self.publish_watch_messages(result.messages).await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        self.client
            .log_message(lsp::MessageType::INFO, "destack.shutdown")
            .await;
        if let Some(workspace_service) = self.workspace_service.get() {
            workspace_service.shutdown();
        }
        Ok(())
    }

    // ------------------------------------------------------------------------
    // SYNCHRONIZATION
    // ------------------------------------------------------------------------

    async fn did_open(&self, params: lsp::DidOpenTextDocumentParams) {
        let uri_str = params.text_document.uri.to_string();
        let content = normalize_line_endings(params.text_document.text);
        let version = params.text_document.version;

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

        // invalidate and publish diagnostics
        self.invalidate_and_publish(&params.text_document.uri, &path, content.clone())
            .await;

        // track open document after the file registry is updated
        let Some(file_id) = session.files.get_id_by_path(&path) else {
            return;
        };
        self.open_documents.insert(
            uri_str,
            OpenDocument {
                file_id,
                text: content,
                version,
            },
        );
    }

    async fn did_change(&self, params: lsp::DidChangeTextDocumentParams) {
        // incremental sync: apply change ranges to stored text
        if params.content_changes.is_empty() {
            return;
        };
        let uri_str = params.text_document.uri.to_string();
        let content = {
            let Some(mut entry) = self.open_documents.get_mut(&uri_str) else {
                return;
            };
            if params.text_document.version <= entry.version {
                tracing::debug!(
                    uri = %uri_str,
                    incoming = params.text_document.version,
                    current = entry.version,
                    "lsp.did_change.stale_version"
                );
                return;
            }
            let applied = apply_text_changes(&mut entry.text, &params.content_changes);
            if !applied {
                tracing::debug!(uri = %uri_str, "lsp.did_change.apply_failed");
                return;
            }
            entry.version = params.text_document.version;
            entry.text.clone()
        };
        let Some(path) = params
            .text_document
            .uri
            .to_file_path()
            .map(|p| p.into_owned())
        else {
            return;
        };

        // update overlay with new content
        self.overlay_fs.set_overlay(&path, content.clone());

        // invalidate and publish diagnostics
        self.invalidate_and_publish(&params.text_document.uri, &path, content)
            .await;
    }

    async fn did_save(&self, params: lsp::DidSaveTextDocumentParams) {
        let uri_str = params.text_document.uri.to_string();
        let Some(path) = params
            .text_document
            .uri
            .to_file_path()
            .map(|p| p.into_owned())
        else {
            return;
        };

        let content = if let Some(text) = params.text {
            Some(normalize_line_endings(text))
        } else {
            std::fs::read_to_string(&path)
                .ok()
                .map(normalize_line_endings)
        };

        let Some(content) = content else {
            tracing::debug!(uri = %uri_str, "lsp.did_save.read_failed");
            return;
        };

        if let Some(mut entry) = self.open_documents.get_mut(&uri_str) {
            entry.text = content.clone();
        }

        self.overlay_fs.set_overlay(&path, content.clone());
        self.invalidate_and_publish(&params.text_document.uri, &path, content)
            .await;
    }

    async fn did_close(&self, params: lsp::DidCloseTextDocumentParams) {
        let uri_str = params.text_document.uri.to_string();
        if let Some((_, document)) = self.open_documents.remove(&uri_str) {
            self.virtual_diagnostics.remove(&document.file_id);
        }
        self.semantic_tokens_cache.remove(&uri_str);

        // remove overlay to fall back to disk content
        if let Some(path) = params
            .text_document
            .uri
            .to_file_path()
            .map(|p| p.into_owned())
        {
            self.overlay_fs.remove_overlay(&path);
        }

        // clear diagnostics for closed file
        self.client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }

    async fn did_change_workspace_folders(&self, params: lsp::DidChangeWorkspaceFoldersParams) {
        let session = self.session();
        let workspace_service = self.workspace_service();

        for folder in params.event.added {
            if let Some(path) = folder.uri.to_file_path().map(|path| path.into_owned()) {
                session.get_or_create_program(path.clone());
                if let Err(error) = workspace_service.open_workspace_root(path) {
                    tracing::debug!(?error, "lsp.workspace.add_failed");
                }
            }
        }

        for folder in params.event.removed {
            if let Some(path) = folder.uri.to_file_path().map(|path| path.into_owned()) {
                let _ = session.remove_root(&path);
                if let Err(error) = workspace_service.close_workspace_root(&path) {
                    tracing::debug!(?error, "lsp.workspace.remove_failed");
                }
            }
        }
    }

    async fn did_change_watched_files(&self, params: lsp::DidChangeWatchedFilesParams) {
        // apply external file changes to the workspace
        let mut events = Vec::new();
        for change in params.changes {
            // resolve file path
            let Some(path) = change.uri.to_file_path().map(|path| path.into_owned()) else {
                continue;
            };

            // skip updates for open documents
            let uri_str = change.uri.to_string();
            if self.open_documents.contains_key(&uri_str) {
                continue;
            }

            // remove any stale overlay for disk changes
            self.overlay_fs.remove_overlay(&path);

            // translate change into a watch event
            let kind = match change.typ {
                lsp::FileChangeType::CREATED => FileWatchEventKind::Created,
                lsp::FileChangeType::CHANGED => FileWatchEventKind::Modified,
                lsp::FileChangeType::DELETED => FileWatchEventKind::Deleted,
                _ => FileWatchEventKind::Modified,
            };
            events.push(FileWatchEvent {
                path,
                previous_path: None,
                kind,
            });
        }

        // apply watch updates
        self.apply_watch_events(events).await;
    }

    async fn will_rename_files(
        &self,
        params: lsp::RenameFilesParams,
    ) -> jsonrpc::Result<Option<lsp::WorkspaceEdit>> {
        // collect rename targets from uris
        let mut renames = Vec::new();
        let mut query_path = None;
        for file in params.files {
            let Ok(old_uri) = file.old_uri.parse::<lsp::Uri>() else {
                continue;
            };
            let Ok(new_uri) = file.new_uri.parse::<lsp::Uri>() else {
                continue;
            };

            let Some(old_path) = old_uri.to_file_path().map(|path| path.into_owned()) else {
                continue;
            };
            let Some(new_path) = new_uri.to_file_path().map(|path| path.into_owned()) else {
                continue;
            };

            if query_path.is_none() {
                query_path = Some(old_path.clone());
            }
            renames.push(query::FileRenameEntry { old_path, new_path });
        }

        // build workspace edits for import specifiers
        let request = query::QueryRequest::RenameFiles(query::RenameFilesRequest { renames });
        let response = if let Some(path) = query_path.as_ref() {
            self.query_for_path(path, request)
        } else {
            self.query_for_workspace(request)
        };
        let Some(query::QueryResponse::RenameFiles(response)) = response else {
            return Ok(None);
        };
        let Some(result) = response.result else {
            return Ok(None);
        };
        if result.is_empty() {
            return Ok(None);
        }

        let edit = batch_edit_to_workspace_edit(self.session(), &result.edits);
        Ok(Some(edit))
    }

    async fn will_delete_files(
        &self,
        params: lsp::DeleteFilesParams,
    ) -> jsonrpc::Result<Option<lsp::WorkspaceEdit>> {
        let _ = params;
        Ok(None)
    }

    async fn did_create_files(&self, params: lsp::CreateFilesParams) {
        // apply created file updates
        let session = self.session().clone();
        let mut events = Vec::new();
        for file in params.files {
            // parse the file uri
            let Ok(uri) = file.uri.parse::<lsp::Uri>() else {
                continue;
            };

            // skip open documents
            let uri_str = uri.to_string();
            if self.open_documents.contains_key(&uri_str) {
                continue;
            }

            // resolve the file path
            let Some(path) = uri.to_file_path().map(|path| path.into_owned()) else {
                continue;
            };

            // clear overlay so we read from disk
            self.overlay_fs.remove_overlay(&path);

            // skip files that are not yet visible on disk
            if session.fs.exists(&path).ok() != Some(true) {
                continue;
            }

            // record the create event
            events.push(FileWatchEvent {
                path,
                previous_path: None,
                kind: FileWatchEventKind::Created,
            });
        }

        // apply watch updates
        self.apply_watch_events(events).await;
    }

    async fn did_rename_files(&self, params: lsp::RenameFilesParams) {
        // apply renamed file updates
        let session = self.session().clone();
        let mut events = Vec::new();
        for file in params.files {
            // parse rename uris
            let Ok(old_uri) = file.old_uri.parse::<lsp::Uri>() else {
                continue;
            };
            let Ok(new_uri) = file.new_uri.parse::<lsp::Uri>() else {
                continue;
            };

            // skip open documents
            let old_uri_str = old_uri.to_string();
            let new_uri_str = new_uri.to_string();
            if self.open_documents.contains_key(&old_uri_str)
                || self.open_documents.contains_key(&new_uri_str)
            {
                continue;
            }

            // resolve paths
            let Some(old_path) = old_uri.to_file_path().map(|path| path.into_owned()) else {
                continue;
            };
            let Some(new_path) = new_uri.to_file_path().map(|path| path.into_owned()) else {
                continue;
            };

            // clear overlays so we re-read from disk
            self.overlay_fs.remove_overlay(&old_path);
            self.overlay_fs.remove_overlay(&new_path);

            // skip files that are not yet visible on disk
            if session.fs.exists(&new_path).ok() != Some(true) {
                continue;
            }

            // record the rename event
            events.push(FileWatchEvent {
                path: new_path,
                previous_path: Some(old_path),
                kind: FileWatchEventKind::Renamed,
            });
        }

        // apply watch updates
        self.apply_watch_events(events).await;
    }

    async fn did_delete_files(&self, params: lsp::DeleteFilesParams) {
        // clear diagnostics for deleted files
        let mut events = Vec::new();
        for file in params.files {
            // parse the file uri
            let Ok(uri) = file.uri.parse::<lsp::Uri>() else {
                continue;
            };
            let uri_str = uri.to_string();
            if self.open_documents.contains_key(&uri_str) {
                continue;
            }

            // resolve the file path
            if let Some(path) = uri.to_file_path().map(|path| path.into_owned()) {
                // clear overlay so we read from disk
                self.overlay_fs.remove_overlay(&path);

                // record the delete event
                events.push(FileWatchEvent {
                    path,
                    previous_path: None,
                    kind: FileWatchEventKind::Deleted,
                });
            }
        }

        // apply watch updates
        self.apply_watch_events(events).await;
    }

    // ------------------------------------------------------------------------
    // DIAGNOSTICS
    // ------------------------------------------------------------------------

    async fn diagnostic(
        &self,
        params: lsp::DocumentDiagnosticParams,
    ) -> jsonrpc::Result<lsp::DocumentDiagnosticReportResult> {
        let session = self.session();
        let uri_str = params.text_document.uri.to_string();
        let doc_entry = self.open_documents.get(&uri_str);
        let file_id = doc_entry
            .as_ref()
            .map(|doc| doc.file_id)
            .or_else(|| {
                params
                    .text_document
                    .uri
                    .to_file_path()
                    .map(|path| path.into_owned())
                    .and_then(|path| session.files.get_id_by_path(&path))
            })
            .or_else(|| {
                let uri = Self::query_uri(&params.text_document.uri);
                session.files.get_id_by_uri(&uri)
            });

        let Some(file_id) = file_id else {
            tracing::debug!(uri = %uri_str, "lsp.diagnostic.file_not_found");
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

        let Some(file) = self.get_query_file(file_id) else {
            tracing::debug!(?file_id, uri = %uri_str, "lsp.diagnostic.query_file_not_found");
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
        let diagnostics = if let Some(open_document) = doc_entry.as_ref()
            && let Some(cached) = self.virtual_diagnostics.get(&open_document.file_id)
        {
            cached.value().clone()
        } else {
            let source_file = session.files.get(file_id);
            let program = if let Some(path) = source_file.path.as_ref() {
                session.find_program_for_path(path)
            } else {
                session.get_or_create_program(session.cwd.clone())
            };
            program.diagnostic_store.diagnostics_for_file(file_id)
        };
        let result_id = diagnostic_result_id(&diagnostics);

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

        let items: Vec<lsp::Diagnostic> = diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic_to_lsp_diagnostic(&diagnostic, &file))
            .collect();

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
        let started_at = Instant::now();
        let session = self.session();
        let previous_ids: std::collections::HashMap<String, String> = params
            .previous_result_ids
            .into_iter()
            .map(|entry| (entry.uri.to_string(), entry.value))
            .collect();

        let mut diagnostics_by_file = std::collections::HashMap::new();
        for entry in session.programs.iter() {
            let program = entry.value();
            for (file_id, diagnostics) in program.diagnostic_store.snapshot_by_file() {
                diagnostics_by_file.entry(file_id).or_insert(diagnostics);
            }
        }

        let mut open_versions = std::collections::HashMap::new();
        for entry in self.open_documents.iter() {
            let doc = entry.value();
            open_versions.insert(doc.file_id, doc.version);
            if let Some(cached) = self.virtual_diagnostics.get(&doc.file_id) {
                diagnostics_by_file.insert(doc.file_id, cached.value().clone());
            } else {
                diagnostics_by_file.entry(doc.file_id).or_insert(Vec::new());
            }
        }

        // collect partial results when supported
        let partial_token = params.partial_result_params.partial_result_token;

        let mut items = Vec::new();
        let mut partial_items = Vec::new();

        // start work done progress when supported
        let mut progress = WorkDoneProgressTracker::start(
            self,
            params.work_done_progress_params.work_done_token,
            "Workspace diagnostics",
            "collecting diagnostics",
        )
        .await;

        // allow cancellation between chunks
        let mut processed = 0usize;
        for (file_id, diagnostics) in diagnostics_by_file {
            let Some(file) = session.files.get_maybe(file_id) else {
                tracing::debug!(?file_id, "lsp.workspace_diagnostic.file_not_found");
                continue;
            };
            let Some(uri) = lsp_uri_for_file(&file) else {
                continue;
            };
            let result_id = diagnostic_result_id(&diagnostics);
            let version = open_versions.get(&file_id).map(|version| *version as i64);
            let uri_str = uri.to_string();
            if previous_ids.get(&uri_str) == Some(&result_id) {
                let report = lsp::WorkspaceDocumentDiagnosticReport::Unchanged(
                    lsp::WorkspaceUnchangedDocumentDiagnosticReport {
                        uri,
                        version,
                        unchanged_document_diagnostic_report:
                            lsp::UnchangedDocumentDiagnosticReport { result_id },
                    },
                );
                items.push(report);
                continue;
            }

            let lsp_diagnostics: Vec<lsp::Diagnostic> = diagnostics
                .into_iter()
                .map(|diagnostic| diagnostic_to_lsp_diagnostic(&diagnostic, &file))
                .collect();
            let report = lsp::WorkspaceDocumentDiagnosticReport::Full(
                lsp::WorkspaceFullDocumentDiagnosticReport {
                    uri,
                    version,
                    full_document_diagnostic_report: lsp::FullDocumentDiagnosticReport {
                        result_id: Some(result_id),
                        items: lsp_diagnostics,
                    },
                },
            );
            if let Some(token) = partial_token.as_ref() {
                partial_items.push(report.clone());
                if partial_items.len() >= WORKSPACE_DIAGNOSTIC_PARTIAL_CHUNK_SIZE {
                    self.publish_workspace_diagnostic_partial(
                        token,
                        std::mem::take(&mut partial_items),
                    )
                    .await;
                }
            }
            items.push(report);

            processed += 1;
            progress
                .report_chunk(
                    self,
                    processed,
                    WORKSPACE_DIAGNOSTIC_PARTIAL_CHUNK_SIZE,
                    |count| format!("scanned {count} files"),
                    "diagnostics cancelled",
                )
                .await?;
        }

        if let Some(token) = partial_token.as_ref() {
            self.publish_workspace_diagnostic_partial(token, std::mem::take(&mut partial_items))
                .await;
        }

        progress.finish(self, "diagnostics complete").await;

        let elapsed = started_at.elapsed();
        if elapsed >= SLOW_DIAGNOSTIC_WARN_DURATION {
            tracing::warn!(
                files = items.len(),
                elapsed_ms = elapsed.as_millis(),
                "lsp.workspace_diagnostic.slow"
            );
        } else {
            tracing::debug!(
                files = items.len(),
                elapsed_ms = elapsed.as_millis(),
                "lsp.workspace_diagnostic.ok"
            );
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
            "destack.rescan" | "destack.reindex" => {
                let started_at = Instant::now();
                let workspace_service = self.workspace_service();
                let result = match workspace_service.rescan_all(RescanReason::Manual) {
                    Ok(result) => result,
                    Err(error) => {
                        tracing::debug!(?error, "lsp.command.rescan_failed");
                        return Err(jsonrpc::Error::internal_error());
                    }
                };
                let update_count = result.updates.len();
                let message_count = result.messages.len();
                self.publish_watch_updates(result.updates).await;
                self.publish_watch_messages(result.messages).await;
                if let Err(error) = self.client.workspace_diagnostic_refresh().await {
                    tracing::debug!(?error, "lsp.command.refresh_diagnostics_failed");
                }
                if let Err(error) = self.client.semantic_tokens_refresh().await {
                    tracing::debug!(?error, "lsp.command.refresh_semantic_tokens_failed");
                }
                tracing::info!(
                    command = params.command.as_str(),
                    updates = update_count,
                    messages = message_count,
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "lsp.command.completed"
                );
                Ok(None)
            }
            "destack.clearCache" => {
                let started_at = Instant::now();
                let workspace_service = self.workspace_service();
                if let Err(error) = workspace_service.clear_cache_all() {
                    tracing::debug!(?error, "lsp.command.clear_cache_failed");
                    return Err(jsonrpc::Error::internal_error());
                }

                let result = match workspace_service.rescan_all(RescanReason::Manual) {
                    Ok(result) => result,
                    Err(error) => {
                        tracing::debug!(?error, "lsp.command.rescan_failed");
                        return Err(jsonrpc::Error::internal_error());
                    }
                };
                let update_count = result.updates.len();
                let message_count = result.messages.len();
                self.publish_watch_updates(result.updates).await;
                self.publish_watch_messages(result.messages).await;
                if let Err(error) = self.client.workspace_diagnostic_refresh().await {
                    tracing::debug!(?error, "lsp.command.refresh_diagnostics_failed");
                }
                if let Err(error) = self.client.semantic_tokens_refresh().await {
                    tracing::debug!(?error, "lsp.command.refresh_semantic_tokens_failed");
                }
                tracing::info!(
                    command = "destack.clearCache",
                    updates = update_count,
                    messages = message_count,
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "lsp.command.completed"
                );
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    async fn work_done_progress_cancel(&self, params: lsp::WorkDoneProgressCancelParams) {
        self.cancelled_progress_tokens.insert(params.token);
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

        // keep refactor inputs on a fresh analyzed snapshot
        let Some(file) = self.get_query_file_for_open_document(&doc) else {
            return Ok(None);
        };
        let Some(offset) = position_to_byte(&file, &params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        // query for definition
        let query_uri = Self::query_uri(&params.text_document_position_params.text_document.uri);
        let request = query::QueryRequest::GotoDefinition(query::GotoDefinitionRequest {
            uri: query_uri,
            offset,
        });
        let Some(query::QueryResponse::GotoDefinition(response)) = self.query_for_uri(
            &params.text_document_position_params.text_document.uri,
            request,
        ) else {
            return Ok(None);
        };
        let Some(result) = response.result else {
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
        let Some(file) = self.get_query_file_for_open_document(&doc) else {
            return Ok(None);
        };
        let Some(offset) = position_to_byte(&file, &params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        // query for declaration
        let query_uri = Self::query_uri(&params.text_document_position_params.text_document.uri);
        let request = query::QueryRequest::GotoDeclaration(query::GotoDeclarationRequest {
            uri: query_uri,
            offset,
        });
        let Some(query::QueryResponse::GotoDeclaration(response)) = self.query_for_uri(
            &params.text_document_position_params.text_document.uri,
            request,
        ) else {
            return Ok(None);
        };
        let Some(result) = response.result else {
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
        let Some(file) = self.get_query_file_for_open_document(&doc) else {
            return Ok(None);
        };
        let Some(offset) = position_to_byte(&file, &params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        // query for type definition
        let query_uri = Self::query_uri(&params.text_document_position_params.text_document.uri);
        let request = query::QueryRequest::GotoTypeDefinition(query::GotoTypeDefinitionRequest {
            uri: query_uri,
            offset,
        });
        let Some(query::QueryResponse::GotoTypeDefinition(response)) = self.query_for_uri(
            &params.text_document_position_params.text_document.uri,
            request,
        ) else {
            return Ok(None);
        };
        let Some(result) = response.result else {
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
        let Some(file) = self.get_query_file_for_open_document(&doc) else {
            return Ok(None);
        };
        let Some(offset) = position_to_byte(&file, &params.text_document_position.position) else {
            return Ok(None);
        };

        // query for references
        let query_uri = Self::query_uri(&params.text_document_position.text_document.uri);
        let request = query::QueryRequest::FindReferences(query::FindReferencesRequest {
            uri: query_uri,
            offset,
            include_declaration: params.context.include_declaration,
        });
        let Some(query::QueryResponse::FindReferences(response)) =
            self.query_for_uri(&params.text_document_position.text_document.uri, request)
        else {
            return Ok(None);
        };
        let Some(refs) = response.result else {
            return Ok(None);
        };
        if refs.is_empty() {
            return Ok(None);
        }

        let mut progress = WorkDoneProgressTracker::start(
            self,
            params.work_done_progress_params.work_done_token,
            "References",
            "resolving references",
        )
        .await;

        // convert to LSP locations
        let partial_token = params.partial_result_params.partial_result_token;
        let mut locations = Vec::new();
        let mut partial_locations = Vec::new();
        let mut processed = 0usize;
        for span in refs.references.iter() {
            let Some(location) = span_to_location(session, *span) else {
                continue;
            };

            if let Some(token) = partial_token.as_ref() {
                partial_locations.push(location.clone());
                if partial_locations.len() >= PARTIAL_RESULT_CHUNK_SIZE {
                    self.publish_partial_result(
                        token,
                        std::mem::take(&mut partial_locations),
                        "references",
                    )
                    .await;
                }
            }
            locations.push(location);

            processed += 1;
            progress
                .report_chunk(
                    self,
                    processed,
                    PARTIAL_RESULT_CHUNK_SIZE,
                    |count| format!("resolved {count} references"),
                    "references cancelled",
                )
                .await?;
        }

        if let Some(token) = partial_token.as_ref() {
            self.publish_partial_result(
                token,
                std::mem::take(&mut partial_locations),
                "references",
            )
            .await;
        }

        if locations.is_empty() {
            progress.finish(self, "references complete").await;
            Ok(None)
        } else {
            progress.finish(self, "references complete").await;
            Ok(Some(locations))
        }
    }

    async fn document_symbol(
        &self,
        params: lsp::DocumentSymbolParams,
    ) -> jsonrpc::Result<Option<lsp::DocumentSymbolResponse>> {
        // resolve file
        let uri_str = params.text_document.uri.to_string();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };

        // query for document symbols
        let query_uri = Self::query_uri(&params.text_document.uri);
        let request =
            query::QueryRequest::DocumentSymbols(query::DocumentSymbolsRequest { uri: query_uri });
        let Some(query::QueryResponse::DocumentSymbols(response)) =
            self.query_for_uri(&params.text_document.uri, request)
        else {
            return Ok(None);
        };
        let symbols = response.symbols;
        if symbols.is_empty() {
            return Ok(None);
        }

        let mut progress = WorkDoneProgressTracker::start(
            self,
            params.work_done_progress_params.work_done_token,
            "Document symbols",
            "building symbols",
        )
        .await;

        // convert to LSP symbols
        let Some(file) = self.get_query_file_for_open_document(&doc) else {
            return Ok(None);
        };
        let partial_token = params.partial_result_params.partial_result_token;
        let mut lsp_symbols = Vec::new();
        let mut partial_symbols = Vec::new();
        let mut processed = 0usize;
        for symbol in symbols.iter() {
            let Some(lsp_symbol) = document_symbol_to_lsp(&file, symbol) else {
                continue;
            };
            if let Some(token) = partial_token.as_ref() {
                partial_symbols.push(lsp_symbol.clone());
                if partial_symbols.len() >= PARTIAL_RESULT_CHUNK_SIZE {
                    self.publish_partial_result(
                        token,
                        std::mem::take(&mut partial_symbols),
                        "document_symbol",
                    )
                    .await;
                }
            }
            lsp_symbols.push(lsp_symbol);

            processed += 1;
            progress
                .report_chunk(
                    self,
                    processed,
                    PARTIAL_RESULT_CHUNK_SIZE,
                    |count| format!("built {count} symbols"),
                    "symbols cancelled",
                )
                .await?;
        }

        if let Some(token) = partial_token.as_ref() {
            self.publish_partial_result(
                token,
                std::mem::take(&mut partial_symbols),
                "document_symbol",
            )
            .await;
        }

        if lsp_symbols.is_empty() {
            progress.finish(self, "symbols complete").await;
            Ok(None)
        } else {
            progress.finish(self, "symbols complete").await;
            Ok(Some(lsp::DocumentSymbolResponse::Nested(lsp_symbols)))
        }
    }

    async fn symbol(
        &self,
        params: lsp::WorkspaceSymbolParams,
    ) -> jsonrpc::Result<Option<lsp::OneOf<Vec<lsp::SymbolInformation>, Vec<lsp::WorkspaceSymbol>>>>
    {
        let session = self.session();

        // query workspace symbols
        let request = query::QueryRequest::WorkspaceSymbols(query::WorkspaceSymbolsRequest {
            query: params.query.clone(),
            max_results: 100,
        });
        let Some(query::QueryResponse::WorkspaceSymbols(response)) =
            self.query_for_workspace(request)
        else {
            return Ok(None);
        };
        let symbols = response.symbols;

        let mut progress = WorkDoneProgressTracker::start(
            self,
            params.work_done_progress_params.work_done_token,
            "Workspace symbols",
            "building symbols",
        )
        .await;

        // convert to LSP
        let partial_token = params.partial_result_params.partial_result_token;
        let mut lsp_symbols = Vec::new();
        let mut partial_symbols = Vec::new();
        let mut processed = 0usize;
        for symbol in symbols.iter() {
            let Some(lsp_symbol) = workspace_symbol_to_lsp(session, symbol) else {
                continue;
            };
            if let Some(token) = partial_token.as_ref() {
                partial_symbols.push(lsp_symbol.clone());
                if partial_symbols.len() >= PARTIAL_RESULT_CHUNK_SIZE {
                    self.publish_partial_result(
                        token,
                        std::mem::take(&mut partial_symbols),
                        "workspace_symbol",
                    )
                    .await;
                }
            }
            lsp_symbols.push(lsp_symbol);

            processed += 1;
            progress
                .report_chunk(
                    self,
                    processed,
                    PARTIAL_RESULT_CHUNK_SIZE,
                    |count| format!("built {count} symbols"),
                    "symbols cancelled",
                )
                .await?;
        }

        if let Some(token) = partial_token.as_ref() {
            self.publish_partial_result(
                token,
                std::mem::take(&mut partial_symbols),
                "workspace_symbol",
            )
            .await;
        }

        if lsp_symbols.is_empty() {
            progress.finish(self, "symbols complete").await;
            Ok(None)
        } else {
            progress.finish(self, "symbols complete").await;
            Ok(Some(lsp::OneOf::Left(lsp_symbols)))
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
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let Some(file) = self.get_query_file_for_open_document(&doc) else {
            return Ok(None);
        };
        let Some(offset) = position_to_byte(&file, &params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        // query for highlights
        let query_uri = Self::query_uri(&params.text_document_position_params.text_document.uri);
        let request = query::QueryRequest::DocumentHighlight(query::DocumentHighlightRequest {
            uri: query_uri,
            offset,
        });
        let Some(query::QueryResponse::DocumentHighlight(response)) = self.query_for_uri(
            &params.text_document_position_params.text_document.uri,
            request,
        ) else {
            return Ok(None);
        };
        let highlights = response.highlights;
        if highlights.is_empty() {
            return Ok(None);
        }

        let mut progress = WorkDoneProgressTracker::start(
            self,
            params.work_done_progress_params.work_done_token,
            "Document highlights",
            "building highlights",
        )
        .await;

        // convert to LSP highlights
        let partial_token = params.partial_result_params.partial_result_token;
        let mut lsp_highlights = Vec::new();
        let mut partial_highlights = Vec::new();
        let mut processed = 0usize;
        for highlight in highlights.iter() {
            let Some(lsp_highlight) = document_highlight_to_lsp(&file, highlight) else {
                continue;
            };
            if let Some(token) = partial_token.as_ref() {
                partial_highlights.push(lsp_highlight.clone());
                if partial_highlights.len() >= PARTIAL_RESULT_CHUNK_SIZE {
                    self.publish_partial_result(
                        token,
                        std::mem::take(&mut partial_highlights),
                        "document_highlight",
                    )
                    .await;
                }
            }
            lsp_highlights.push(lsp_highlight);

            processed += 1;
            progress
                .report_chunk(
                    self,
                    processed,
                    PARTIAL_RESULT_CHUNK_SIZE,
                    |count| format!("built {count} highlights"),
                    "highlights cancelled",
                )
                .await?;
        }

        if let Some(token) = partial_token.as_ref() {
            self.publish_partial_result(
                token,
                std::mem::take(&mut partial_highlights),
                "document_highlight",
            )
            .await;
        }

        if lsp_highlights.is_empty() {
            progress.finish(self, "highlights complete").await;
            Ok(None)
        } else {
            progress.finish(self, "highlights complete").await;
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
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let Some(file) = self.get_query_file_for_open_document(&doc) else {
            return Ok(None);
        };
        let Some(offset) = position_to_byte(&file, &params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        // query hover info
        let query_uri = Self::query_uri(&params.text_document_position_params.text_document.uri);
        let request = query::QueryRequest::Hover(query::HoverRequest {
            uri: query_uri,
            offset,
        });
        let Some(query::QueryResponse::Hover(response)) = self.query_for_uri(
            &params.text_document_position_params.text_document.uri,
            request,
        ) else {
            return Ok(None);
        };
        let Some(hover_info) = response.hover else {
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
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let Some(file) = self.get_query_file_for_open_document(&doc) else {
            return Ok(None);
        };
        let Some(offset) = position_to_byte(&file, &params.text_document_position.position) else {
            return Ok(None);
        };

        // map the LSP trigger kind to completion behavior
        let trigger = match params.context.as_ref().map(|ctx| ctx.trigger_kind) {
            Some(kind) if kind == lsp::CompletionTriggerKind::TRIGGER_CHARACTER => params
                .context
                .as_ref()
                .and_then(|ctx| ctx.trigger_character.as_ref())
                .and_then(|s| s.chars().next())
                .map(query::CompletionTrigger::Character)
                .unwrap_or(query::CompletionTrigger::Invoked),
            Some(kind)
                if kind == lsp::CompletionTriggerKind::TRIGGER_FOR_INCOMPLETE_COMPLETIONS =>
            {
                query::CompletionTrigger::Incomplete
            }
            _ => query::CompletionTrigger::Invoked,
        };

        // query for completions
        let query_uri = Self::query_uri(&params.text_document_position.text_document.uri);
        let request = query::QueryRequest::Completion(query::CompletionRequest {
            uri: query_uri,
            offset,
            trigger,
            include_imports: true,
        });
        let Some(query::QueryResponse::Completion(response)) =
            self.query_for_uri(&params.text_document_position.text_document.uri, request)
        else {
            return Ok(None);
        };
        let mut completions = response.items;
        if completions.is_empty() {
            return Ok(None);
        }

        // filter auto imports when disabled
        if !self.completion_auto_imports_enabled() {
            completions.retain(|completion| !completion.is_auto_import);
        }

        // convert to LSP completion items
        let items: Vec<lsp::CompletionItem> = completions
            .into_iter()
            .map(|c| {
                let insert_text_format = if c.is_snippet {
                    Some(lsp::InsertTextFormat::SNIPPET)
                } else {
                    None
                };
                let insert_text_mode = if c.is_snippet {
                    Some(lsp::InsertTextMode::ADJUST_INDENTATION)
                } else {
                    None
                };
                let commit_characters = Self::completion_commit_characters(c.kind);

                // convert additional text edits
                let additional_text_edits = if c.additional_text_edits.is_empty() {
                    None
                } else {
                    Some(
                        c.additional_text_edits
                            .iter()
                            .filter_map(|edit| {
                                // for now, only handle edits to the same file
                                if edit.span.file != doc.file_id {
                                    return None;
                                }
                                Some(lsp::TextEdit {
                                    range: byte_span_to_range(&file, edit.span),
                                    new_text: edit.new_text.clone(),
                                })
                            })
                            .collect(),
                    )
                };

                // handle deprecated items
                let (deprecated, tags) = if c.deprecated {
                    (Some(true), Some(vec![lsp::CompletionItemTag::DEPRECATED]))
                } else {
                    (None, None)
                };

                let sort_text = c
                    .sort_text
                    .clone()
                    .or_else(|| Some(format!("{:04}:{}", c.sort_order, c.label.as_str())));

                let (detail, label_details) = if c.is_auto_import
                    && self.completion_label_details_supported()
                    && let Some(detail_text) = c.detail.as_ref()
                    && let Some(path) = detail_text.strip_prefix(AUTO_IMPORT_DETAIL_PREFIX)
                {
                    (
                        None,
                        Some(lsp::CompletionItemLabelDetails {
                            detail: None,
                            description: Some(path.to_string()),
                        }),
                    )
                } else {
                    (c.detail.clone(), None)
                };

                let data = c.documentation.as_ref().and_then(|doc| {
                    to_value(CompletionResolveData {
                        documentation: Some(doc.clone()),
                    })
                    .ok()
                });

                lsp::CompletionItem {
                    label: c.label,
                    label_details,
                    kind: Some(completion_kind_to_lsp(c.kind)),
                    detail,
                    documentation: None,
                    insert_text: c.insert_text,
                    insert_text_format,
                    insert_text_mode,
                    sort_text,
                    preselect: if c.preselect { Some(true) } else { None },
                    deprecated,
                    tags,
                    commit_characters,
                    additional_text_edits,
                    data,
                    ..Default::default()
                }
            })
            .collect();

        let is_incomplete = response.is_incomplete;
        Ok(Some(lsp::CompletionResponse::List(lsp::CompletionList {
            is_incomplete,
            items,
            ..Default::default()
        })))
    }

    async fn completion_resolve(
        &self,
        mut params: lsp::CompletionItem,
    ) -> jsonrpc::Result<lsp::CompletionItem> {
        if params.documentation.is_some() {
            return Ok(params);
        }

        let Some(data) = params.data.take() else {
            return Ok(params);
        };

        let resolved = match from_value::<CompletionResolveData>(data.clone()) {
            Ok(resolved) => resolved,
            Err(_) => {
                params.data = Some(data);
                return Ok(params);
            }
        };

        if let Some(doc) = resolved.documentation {
            params.documentation = Some(lsp::Documentation::MarkupContent(lsp::MarkupContent {
                kind: lsp::MarkupKind::Markdown,
                value: doc,
            }));
        }

        Ok(params)
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
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let Some(file) = self.get_query_file_for_open_document(&doc) else {
            return Ok(None);
        };
        let Some(offset) = position_to_byte(&file, &params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        // query for signature help
        let query_uri = Self::query_uri(&params.text_document_position_params.text_document.uri);
        let request = query::QueryRequest::SignatureHelp(query::SignatureHelpRequest {
            uri: query_uri,
            offset,
        });
        let Some(query::QueryResponse::SignatureHelp(response)) = self.query_for_uri(
            &params.text_document_position_params.text_document.uri,
            request,
        ) else {
            return Ok(None);
        };
        let Some(help) = response.help else {
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
        params: lsp::SemanticTokensParams,
    ) -> jsonrpc::Result<Option<lsp::SemanticTokensResult>> {
        // look up file
        let uri_str = params.text_document.uri.to_string();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let Some(file) = self.get_query_file_for_open_document(&doc) else {
            return Ok(None);
        };

        // query semantic tokens
        let query_uri = Self::query_uri(&params.text_document.uri);
        let request =
            query::QueryRequest::SemanticTokens(query::SemanticTokensRequest { uri: query_uri });
        let Some(query::QueryResponse::SemanticTokens(response)) =
            self.query_for_uri(&params.text_document.uri, request)
        else {
            return Ok(None);
        };
        let tokens = response.tokens;

        // convert to LSP
        let lsp_tokens = semantic::tokens_to_lsp(&file, &tokens);
        let tokens_hash = Self::semantic_tokens_hash(&lsp_tokens);
        let mut result_id = self.next_semantic_tokens_result_id();
        if let Some(cache) = self.semantic_tokens_cache.get(&uri_str)
            && cache.hash == tokens_hash
        {
            result_id = cache.result_id.clone();
        }
        self.semantic_tokens_cache.insert(
            uri_str.clone(),
            SemanticTokensCache {
                result_id: result_id.clone(),
                data: lsp_tokens.clone(),
                hash: tokens_hash,
            },
        );

        Ok(Some(lsp::SemanticTokensResult::Tokens(
            lsp::SemanticTokens {
                result_id: Some(result_id),
                data: lsp_tokens,
            },
        )))
    }

    async fn semantic_tokens_full_delta(
        &self,
        params: lsp::SemanticTokensDeltaParams,
    ) -> jsonrpc::Result<Option<lsp::SemanticTokensFullDeltaResult>> {
        // look up file
        let uri_str = params.text_document.uri.to_string();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let Some(file) = self.get_query_file_for_open_document(&doc) else {
            return Ok(None);
        };

        // query semantic tokens
        let query_uri = Self::query_uri(&params.text_document.uri);
        let request =
            query::QueryRequest::SemanticTokens(query::SemanticTokensRequest { uri: query_uri });
        let Some(query::QueryResponse::SemanticTokens(response)) =
            self.query_for_uri(&params.text_document.uri, request)
        else {
            return Ok(None);
        };
        let tokens = response.tokens;

        // convert to LSP
        let lsp_tokens = semantic::tokens_to_lsp(&file, &tokens);
        let tokens_hash = Self::semantic_tokens_hash(&lsp_tokens);

        let mut result_id = self.next_semantic_tokens_result_id();
        let mut edits = None;
        if let Some(cache) = self.semantic_tokens_cache.get(&uri_str) {
            if cache.hash == tokens_hash {
                result_id = cache.result_id.clone();
                if cache.result_id == params.previous_result_id {
                    edits = Some(Vec::new());
                }
            } else if cache.result_id == params.previous_result_id {
                edits = Some(semantic_tokens_edits(&cache.data, &lsp_tokens));
            }
        }

        self.semantic_tokens_cache.insert(
            uri_str.clone(),
            SemanticTokensCache {
                result_id: result_id.clone(),
                data: lsp_tokens.clone(),
                hash: tokens_hash,
            },
        );

        if let Some(edits) = edits {
            let delta = lsp::SemanticTokensDelta {
                result_id: Some(result_id),
                edits,
            };
            return Ok(Some(lsp::SemanticTokensFullDeltaResult::TokensDelta(delta)));
        }

        Ok(Some(lsp::SemanticTokensFullDeltaResult::Tokens(
            lsp::SemanticTokens {
                result_id: Some(result_id),
                data: lsp_tokens,
            },
        )))
    }

    async fn semantic_tokens_range(
        &self,
        params: lsp::SemanticTokensRangeParams,
    ) -> jsonrpc::Result<Option<lsp::SemanticTokensRangeResult>> {
        // look up file
        let uri_str = params.text_document.uri.to_string();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let Some(file) = self.get_query_file_for_open_document(&doc) else {
            return Ok(None);
        };

        // convert range to span
        let Some(start) = position_to_byte(&file, &params.range.start) else {
            return Ok(None);
        };
        let Some(end) = position_to_byte(&file, &params.range.end) else {
            return Ok(None);
        };
        // query semantic tokens for range
        let query_uri = Self::query_uri(&params.text_document.uri);
        let request = query::QueryRequest::SemanticTokensRange(query::SemanticTokensRangeRequest {
            uri: query_uri,
            start,
            end,
        });
        let Some(query::QueryResponse::SemanticTokensRange(response)) =
            self.query_for_uri(&params.text_document.uri, request)
        else {
            return Ok(None);
        };
        let tokens = response.tokens;

        // convert to LSP
        let lsp_tokens = semantic::tokens_to_lsp(&file, &tokens);

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
        // resolve file
        let uri_str = params.text_document.uri.to_string();
        let Some(_doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };

        // query for folding ranges
        let query_uri = Self::query_uri(&params.text_document.uri);
        let request =
            query::QueryRequest::FoldingRanges(query::FoldingRangesRequest { uri: query_uri });
        let Some(query::QueryResponse::FoldingRanges(response)) =
            self.query_for_uri(&params.text_document.uri, request)
        else {
            return Ok(None);
        };
        let ranges = response.ranges;
        if ranges.is_empty() {
            return Ok(None);
        }

        let mut progress = WorkDoneProgressTracker::start(
            self,
            params.work_done_progress_params.work_done_token,
            "Folding ranges",
            "building folding ranges",
        )
        .await;

        // convert to LSP folding ranges
        let partial_token = params.partial_result_params.partial_result_token;
        let mut lsp_ranges = Vec::new();
        let mut partial_ranges = Vec::new();
        let mut processed = 0usize;
        for range in ranges.into_iter() {
            let lsp_range = lsp::FoldingRange {
                start_line: range.start_line,
                start_character: range.start_character,
                end_line: range.end_line,
                end_character: range.end_character,
                kind: range.kind.map(|kind| match kind {
                    query::FoldingRangeKind::Comment => lsp::FoldingRangeKind::Comment,
                    query::FoldingRangeKind::Imports => lsp::FoldingRangeKind::Imports,
                    query::FoldingRangeKind::Region => lsp::FoldingRangeKind::Region,
                }),
                collapsed_text: range.collapsed_text,
            };
            if let Some(token) = partial_token.as_ref() {
                partial_ranges.push(lsp_range.clone());
                if partial_ranges.len() >= PARTIAL_RESULT_CHUNK_SIZE {
                    self.publish_partial_result(
                        token,
                        std::mem::take(&mut partial_ranges),
                        "folding_range",
                    )
                    .await;
                }
            }
            lsp_ranges.push(lsp_range);

            processed += 1;
            progress
                .report_chunk(
                    self,
                    processed,
                    PARTIAL_RESULT_CHUNK_SIZE,
                    |count| format!("built {count} folding ranges"),
                    "folding ranges cancelled",
                )
                .await?;
        }

        if let Some(token) = partial_token.as_ref() {
            self.publish_partial_result(
                token,
                std::mem::take(&mut partial_ranges),
                "folding_range",
            )
            .await;
        }

        if lsp_ranges.is_empty() {
            progress.finish(self, "folding ranges complete").await;
            Ok(None)
        } else {
            progress.finish(self, "folding ranges complete").await;
            Ok(Some(lsp_ranges))
        }
    }

    // ------------------------------------------------------------------------
    // FORMATTING
    // ------------------------------------------------------------------------

    async fn formatting(
        &self,
        params: lsp::DocumentFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TextEdit>>> {
        // resolve file
        let uri_str = params.text_document.uri.to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let Some(file) = self.get_query_file_for_open_document(&doc) else {
            return Ok(None);
        };

        // get formatter options from program (respects dsconfig.json)
        let formatter = file
            .path
            .as_ref()
            .map(|p| session.find_program_for_path(p).formatter)
            .unwrap_or_default();

        // format the file
        let Some(formatted) = format_file(session, doc.file_id, &file, formatter) else {
            return Ok(None);
        };

        // return single edit replacing entire document
        let line_count = file.line_count();
        let last_line_len = file
            .get_line_str(line_count.saturating_sub(1))
            .map(|l| l.len())
            .unwrap_or(0);

        Ok(Some(vec![lsp::TextEdit {
            range: lsp::Range {
                start: lsp::Position {
                    line: 0,
                    character: 0,
                },
                end: lsp::Position {
                    line: line_count,
                    character: last_line_len as u32,
                },
            },
            new_text: formatted,
        }]))
    }

    async fn range_formatting(
        &self,
        params: lsp::DocumentRangeFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TextEdit>>> {
        // resolve file
        let uri_str = params.text_document.uri.to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let Some(file) = self.get_query_file_for_open_document(&doc) else {
            return Ok(None);
        };

        // get formatter options from program
        let formatter = file
            .path
            .as_ref()
            .map(|p| session.find_program_for_path(p).formatter)
            .unwrap_or_default();

        // convert range to byte offsets
        let Some(start_offset) = position_to_byte(&file, &params.range.start) else {
            return Ok(None);
        };
        let Some(end_offset) = position_to_byte(&file, &params.range.end) else {
            return Ok(None);
        };

        // format range
        let Some((formatted, edit_range)) =
            format_range(&file, formatter, start_offset, end_offset)
        else {
            return Ok(None);
        };

        // convert byte range back to LSP range
        let lsp_range = byte_span_to_range(&file, edit_range);

        Ok(Some(vec![lsp::TextEdit {
            range: lsp_range,
            new_text: formatted,
        }]))
    }

    // ------------------------------------------------------------------------
    // SELECTION & NAVIGATION
    // ------------------------------------------------------------------------

    async fn selection_range(
        &self,
        params: lsp::SelectionRangeParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::SelectionRange>>> {
        // look up file
        let uri_str = params.text_document.uri.to_string();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let Some(file) = self.get_query_file_for_open_document(&doc) else {
            return Ok(None);
        };

        // convert positions to byte offsets
        let positions: Vec<u32> = params
            .positions
            .iter()
            .filter_map(|p| position_to_byte(&file, p))
            .collect();

        // query selection ranges
        let query_uri = Self::query_uri(&params.text_document.uri);
        let request = query::QueryRequest::SelectionRanges(query::SelectionRangesRequest {
            uri: query_uri,
            offsets: positions,
        });
        let Some(query::QueryResponse::SelectionRanges(response)) =
            self.query_for_uri(&params.text_document.uri, request)
        else {
            return Ok(None);
        };
        let ranges = response.ranges;

        let mut progress = WorkDoneProgressTracker::start(
            self,
            params.work_done_progress_params.work_done_token,
            "Selection ranges",
            "building ranges",
        )
        .await;

        // convert to LSP
        let partial_token = params.partial_result_params.partial_result_token;
        let mut lsp_ranges = Vec::new();
        let mut partial_ranges = Vec::new();
        let mut processed = 0usize;
        for range in ranges {
            let lsp_range = selection_range_to_lsp(&file, range);
            if let Some(token) = partial_token.as_ref() {
                partial_ranges.push(lsp_range.clone());
                if partial_ranges.len() >= PARTIAL_RESULT_CHUNK_SIZE {
                    self.publish_partial_result(
                        token,
                        std::mem::take(&mut partial_ranges),
                        "selection_range",
                    )
                    .await;
                }
            }
            lsp_ranges.push(lsp_range);

            processed += 1;
            progress
                .report_chunk(
                    self,
                    processed,
                    PARTIAL_RESULT_CHUNK_SIZE,
                    |count| format!("built {count} ranges"),
                    "ranges cancelled",
                )
                .await?;
        }

        if let Some(token) = partial_token.as_ref() {
            self.publish_partial_result(
                token,
                std::mem::take(&mut partial_ranges),
                "selection_range",
            )
            .await;
        }

        progress.finish(self, "ranges complete").await;
        Ok(Some(lsp_ranges))
    }

    async fn goto_implementation(
        &self,
        params: lsp::request::GotoImplementationParams,
    ) -> jsonrpc::Result<Option<lsp::request::GotoImplementationResponse>> {
        // look up file and position
        let uri_str = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let Some(file) = self.get_query_file_for_open_document(&doc) else {
            return Ok(None);
        };
        let Some(offset) = position_to_byte(&file, &params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        // query implementation
        let query_uri = Self::query_uri(&params.text_document_position_params.text_document.uri);
        let request = query::QueryRequest::GotoImplementation(query::GotoImplementationRequest {
            uri: query_uri,
            offset,
        });
        let Some(query::QueryResponse::GotoImplementation(response)) = self.query_for_uri(
            &params.text_document_position_params.text_document.uri,
            request,
        ) else {
            return Ok(None);
        };
        let Some(result) = response.result else {
            return Ok(None);
        };

        // convert to LSP
        let Some(location) = implementation_to_location(session, &result) else {
            return Ok(None);
        };

        Ok(Some(lsp::GotoDefinitionResponse::Scalar(location)))
    }

    // ------------------------------------------------------------------------
    // DOCUMENT LINKS
    // ------------------------------------------------------------------------

    async fn document_link(
        &self,
        params: lsp::DocumentLinkParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::DocumentLink>>> {
        // look up file
        let uri_str = params.text_document.uri.to_string();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let Some(file) = self.get_query_file_for_open_document(&doc) else {
            return Ok(None);
        };

        // query document links
        let query_uri = Self::query_uri(&params.text_document.uri);
        let request =
            query::QueryRequest::DocumentLinks(query::DocumentLinksRequest { uri: query_uri });
        let Some(query::QueryResponse::DocumentLinks(response)) =
            self.query_for_uri(&params.text_document.uri, request)
        else {
            return Ok(None);
        };
        let links = response.links;
        if links.is_empty() {
            return Ok(None);
        }

        let mut progress = WorkDoneProgressTracker::start(
            self,
            params.work_done_progress_params.work_done_token,
            "Document links",
            "building links",
        )
        .await;

        // convert to LSP
        let partial_token = params.partial_result_params.partial_result_token;
        let mut lsp_links = Vec::new();
        let mut partial_links = Vec::new();
        let mut processed = 0usize;
        for link in links.iter() {
            let Some(lsp_link) = document_link_to_lsp(&file, link) else {
                continue;
            };
            if let Some(token) = partial_token.as_ref() {
                partial_links.push(lsp_link.clone());
                if partial_links.len() >= PARTIAL_RESULT_CHUNK_SIZE {
                    self.publish_partial_result(
                        token,
                        std::mem::take(&mut partial_links),
                        "document_link",
                    )
                    .await;
                }
            }
            lsp_links.push(lsp_link);

            processed += 1;
            progress
                .report_chunk(
                    self,
                    processed,
                    PARTIAL_RESULT_CHUNK_SIZE,
                    |count| format!("built {count} links"),
                    "links cancelled",
                )
                .await?;
        }

        if let Some(token) = partial_token.as_ref() {
            self.publish_partial_result(token, std::mem::take(&mut partial_links), "document_link")
                .await;
        }

        if lsp_links.is_empty() {
            progress.finish(self, "links complete").await;
            Ok(None)
        } else {
            progress.finish(self, "links complete").await;
            Ok(Some(lsp_links))
        }
    }

    async fn document_link_resolve(
        &self,
        params: lsp::DocumentLink,
    ) -> jsonrpc::Result<lsp::DocumentLink> {
        // links are already resolved in document_link
        Ok(params)
    }

    // ------------------------------------------------------------------------
    // CODE ACTIONS
    // ------------------------------------------------------------------------

    async fn code_action(
        &self,
        params: lsp::CodeActionParams,
    ) -> jsonrpc::Result<Option<lsp::CodeActionResponse>> {
        // look up file
        let uri_str = params.text_document.uri.to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let Some(file) = self.get_query_file_for_open_document(&doc) else {
            return Ok(None);
        };

        // convert range to span
        let Some(start) = position_to_byte(&file, &params.range.start) else {
            return Ok(None);
        };
        let Some(end) = position_to_byte(&file, &params.range.end) else {
            return Ok(None);
        };
        // query code actions
        let has_only_filter = params
            .context
            .only
            .as_ref()
            .is_some_and(|kinds| !kinds.is_empty());
        let context = Self::query_code_action_context(&params.context);

        // skip when lsp only filters were provided but none map to workspace kinds
        if has_only_filter && context.only.is_empty() {
            return Ok(None);
        }

        let query_uri = Self::query_uri(&params.text_document.uri);
        let request = query::QueryRequest::CodeActions(query::CodeActionsRequest {
            uri: query_uri,
            start,
            end,
            context,
        });
        let Some(query::QueryResponse::CodeActions(response)) =
            self.query_for_uri(&params.text_document.uri, request)
        else {
            return Ok(None);
        };
        let mut actions = response.actions;

        // filter diagnostic linked quick fixes by requested diagnostic codes
        let diagnostic_codes: HashSet<String> = params
            .context
            .diagnostics
            .iter()
            .filter_map(|diagnostic| match diagnostic.code.as_ref() {
                Some(lsp::NumberOrString::String(value)) => Some(value.clone()),
                Some(lsp::NumberOrString::Number(value)) => Some(value.to_string()),
                _ => None,
            })
            .collect();
        if !diagnostic_codes.is_empty() {
            actions.retain(|action| {
                action
                    .diagnostic_code
                    .as_ref()
                    .is_none_or(|code| diagnostic_codes.contains(code))
            });
        }

        if actions.is_empty() {
            return Ok(None);
        }

        let mut progress = WorkDoneProgressTracker::start(
            self,
            params.work_done_progress_params.work_done_token,
            "Code actions",
            "building actions",
        )
        .await;

        // convert to LSP
        let partial_token = params.partial_result_params.partial_result_token;
        let mut lsp_actions = Vec::new();
        let mut partial_actions = Vec::new();
        let mut processed = 0usize;
        let prefer_lazy_code_action_edits =
            self.code_action_data_supported() && self.code_action_edit_resolve_supported();
        for action in actions.iter() {
            let data = if prefer_lazy_code_action_edits {
                to_value(CodeActionResolveData {
                    edits: action.edits.clone(),
                })
                .ok()
            } else {
                None
            };
            let include_edit = !prefer_lazy_code_action_edits || data.is_none();
            let Some(lsp_action) = code_action_to_lsp(session, action, include_edit, data) else {
                continue;
            };
            if let Some(token) = partial_token.as_ref() {
                partial_actions.push(lsp_action.clone());
                if partial_actions.len() >= PARTIAL_RESULT_CHUNK_SIZE {
                    self.publish_partial_result(
                        token,
                        std::mem::take(&mut partial_actions),
                        "code_action",
                    )
                    .await;
                }
            }
            lsp_actions.push(lsp_action);

            processed += 1;
            progress
                .report_chunk(
                    self,
                    processed,
                    PARTIAL_RESULT_CHUNK_SIZE,
                    |count| format!("built {count} actions"),
                    "actions cancelled",
                )
                .await?;
        }

        if let Some(token) = partial_token.as_ref() {
            self.publish_partial_result(token, std::mem::take(&mut partial_actions), "code_action")
                .await;
        }

        if lsp_actions.is_empty() {
            progress.finish(self, "actions complete").await;
            Ok(None)
        } else {
            progress.finish(self, "actions complete").await;
            Ok(Some(lsp_actions))
        }
    }

    async fn code_action_resolve(
        &self,
        mut params: lsp::CodeAction,
    ) -> jsonrpc::Result<lsp::CodeAction> {
        if params.edit.is_some() {
            return Ok(params);
        }

        let Some(data) = params.data.take() else {
            return Ok(params);
        };

        let resolved = match from_value::<CodeActionResolveData>(data.clone()) {
            Ok(resolved) => resolved,
            Err(_) => {
                params.data = Some(data);
                return Ok(params);
            }
        };

        params.edit = Some(batch_edit_to_workspace_edit(
            self.session(),
            &resolved.edits,
        ));

        Ok(params)
    }

    // ------------------------------------------------------------------------
    // CODE LENS
    // ------------------------------------------------------------------------

    async fn code_lens(
        &self,
        params: lsp::CodeLensParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::CodeLens>>> {
        // look up file
        let uri_str = params.text_document.uri.to_string();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let Some(file) = self.get_query_file_for_open_document(&doc) else {
            return Ok(None);
        };

        // query code lenses
        let query_uri = Self::query_uri(&params.text_document.uri);
        let request = query::QueryRequest::CodeLenses(query::CodeLensesRequest { uri: query_uri });
        let Some(query::QueryResponse::CodeLenses(response)) =
            self.query_for_uri(&params.text_document.uri, request)
        else {
            return Ok(None);
        };
        let lenses = response.lenses;

        let mut progress = WorkDoneProgressTracker::start(
            self,
            params.work_done_progress_params.work_done_token,
            "Code lens",
            "building lenses",
        )
        .await;

        // convert to LSP
        let partial_token = params.partial_result_params.partial_result_token;
        let mut lsp_lenses = Vec::new();
        let mut partial_lenses = Vec::new();
        let mut processed = 0usize;
        for lens in lenses.iter() {
            let lsp_lens = code_lens_to_lsp(&file, lens);
            if let Some(token) = partial_token.as_ref() {
                partial_lenses.push(lsp_lens.clone());
                if partial_lenses.len() >= PARTIAL_RESULT_CHUNK_SIZE {
                    self.publish_partial_result(
                        token,
                        std::mem::take(&mut partial_lenses),
                        "code_lens",
                    )
                    .await;
                }
            }
            lsp_lenses.push(lsp_lens);

            processed += 1;
            progress
                .report_chunk(
                    self,
                    processed,
                    PARTIAL_RESULT_CHUNK_SIZE,
                    |count| format!("built {count} lenses"),
                    "lenses cancelled",
                )
                .await?;
        }

        if let Some(token) = partial_token.as_ref() {
            self.publish_partial_result(token, std::mem::take(&mut partial_lenses), "code_lens")
                .await;
        }

        progress.finish(self, "lenses complete").await;
        Ok(Some(lsp_lenses))
    }

    async fn code_lens_resolve(&self, params: lsp::CodeLens) -> jsonrpc::Result<lsp::CodeLens> {
        // lenses are already resolved
        Ok(params)
    }

    // ------------------------------------------------------------------------
    // INLAY HINTS
    // ------------------------------------------------------------------------

    async fn inlay_hint(
        &self,
        params: lsp::InlayHintParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::InlayHint>>> {
        let uri_str = params.text_document.uri.to_string();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let Some(file) = self.get_query_file_for_open_document(&doc) else {
            return Ok(None);
        };

        // convert range to span
        let Some(start) = position_to_byte(&file, &params.range.start) else {
            return Ok(None);
        };
        let Some(end) = position_to_byte(&file, &params.range.end) else {
            return Ok(None);
        };
        // query inlay hints
        let query_uri = Self::query_uri(&params.text_document.uri);
        let request = query::QueryRequest::InlayHints(query::InlayHintsRequest {
            uri: query_uri,
            start,
            end,
        });
        let Some(query::QueryResponse::InlayHints(response)) =
            self.query_for_uri(&params.text_document.uri, request)
        else {
            return Ok(None);
        };
        let hints = response.hints;
        let parameter_hints_enabled = self.parameter_inlay_hints_enabled();
        let type_hints_enabled = self.type_inlay_hints_enabled();

        // convert to LSP
        let lsp_hints: Vec<lsp::InlayHint> = hints
            .iter()
            .filter(|hint| match hint.kind {
                query::InlayHintKind::Parameter => parameter_hints_enabled,
                query::InlayHintKind::Type => type_hints_enabled,
            })
            .filter_map(|h| inlay_hint_to_lsp(&file, h))
            .collect();

        Ok(Some(lsp_hints))
    }

    // ------------------------------------------------------------------------
    // RENAME
    // ------------------------------------------------------------------------

    async fn prepare_rename(
        &self,
        params: lsp::TextDocumentPositionParams,
    ) -> jsonrpc::Result<Option<lsp::PrepareRenameResponse>> {
        // look up file and position
        let uri_str = params.text_document.uri.to_string();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let Some(file) = self.get_query_file_for_open_document(&doc) else {
            return Ok(None);
        };
        let Some(offset) = position_to_byte(&file, &params.position) else {
            return Ok(None);
        };

        // query prepare rename
        let query_uri = Self::query_uri(&params.text_document.uri);
        let request = query::QueryRequest::PrepareRename(query::PrepareRenameRequest {
            uri: query_uri,
            offset,
        });
        let Some(query::QueryResponse::PrepareRename(response)) =
            self.query_for_uri(&params.text_document.uri, request)
        else {
            return Ok(None);
        };
        let Some(result) = response.result else {
            return Ok(None);
        };

        // convert to LSP
        let range = byte_span_to_range(&file, result.range);
        Ok(Some(lsp::PrepareRenameResponse::RangeWithPlaceholder {
            range,
            placeholder: result.placeholder,
        }))
    }

    async fn rename(
        &self,
        params: lsp::RenameParams,
    ) -> jsonrpc::Result<Option<lsp::WorkspaceEdit>> {
        // look up file and position
        let uri_str = params.text_document_position.text_document.uri.to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let Some(file) = self.get_query_file_for_open_document(&doc) else {
            return Ok(None);
        };
        let Some(offset) = position_to_byte(&file, &params.text_document_position.position) else {
            return Ok(None);
        };

        // query rename
        let query_uri = Self::query_uri(&params.text_document_position.text_document.uri);
        let request = query::QueryRequest::Rename(query::RenameRequest {
            uri: query_uri,
            offset,
            new_name: params.new_name.clone(),
        });
        let Some(query::QueryResponse::Rename(response)) =
            self.query_for_uri(&params.text_document_position.text_document.uri, request)
        else {
            return Ok(None);
        };
        let Some(result) = response.result else {
            return Ok(None);
        };

        // convert to LSP
        let workspace_edit = batch_edit_to_workspace_edit(session, &result.edits);
        Ok(Some(workspace_edit))
    }

    // ------------------------------------------------------------------------
    // CALL HIERARCHY
    // ------------------------------------------------------------------------

    async fn prepare_call_hierarchy(
        &self,
        params: lsp::CallHierarchyPrepareParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::CallHierarchyItem>>> {
        // look up file and position
        let uri_str = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let Some(file) = self.get_query_file_for_open_document(&doc) else {
            return Ok(None);
        };
        let Some(offset) = position_to_byte(&file, &params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        // query prepare call hierarchy
        let query_uri = Self::query_uri(&params.text_document_position_params.text_document.uri);
        let request =
            query::QueryRequest::PrepareCallHierarchy(query::PrepareCallHierarchyRequest {
                uri: query_uri,
                offset,
            });
        let Some(query::QueryResponse::PrepareCallHierarchy(response)) = self.query_for_uri(
            &params.text_document_position_params.text_document.uri,
            request,
        ) else {
            return Ok(None);
        };
        let Some(item) = response.item else {
            return Ok(None);
        };

        // convert to LSP
        let Some(lsp_item) = call_hierarchy_item_to_lsp(session, &item) else {
            return Ok(None);
        };

        Ok(Some(vec![lsp_item]))
    }

    async fn incoming_calls(
        &self,
        params: lsp::CallHierarchyIncomingCallsParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::CallHierarchyIncomingCall>>> {
        // extract query item from lsp data
        let session = self.session();
        let Some(item) = query_call_hierarchy_item_from_lsp(&params.item) else {
            return Ok(Some(vec![]));
        };

        // query incoming calls
        let request =
            query::QueryRequest::CallHierarchyIncoming(query::CallHierarchyIncomingRequest {
                item,
            });
        let Some(query::QueryResponse::CallHierarchyIncoming(response)) =
            self.query_for_uri(&params.item.uri, request)
        else {
            return Ok(Some(vec![]));
        };
        let calls = response.calls;

        // convert to LSP
        let lsp_calls: Vec<lsp::CallHierarchyIncomingCall> = calls
            .iter()
            .filter_map(|c| incoming_call_to_lsp(session, c))
            .collect();

        Ok(Some(lsp_calls))
    }

    async fn outgoing_calls(
        &self,
        params: lsp::CallHierarchyOutgoingCallsParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::CallHierarchyOutgoingCall>>> {
        // extract query item from lsp data
        let session = self.session();
        let Some(item) = query_call_hierarchy_item_from_lsp(&params.item) else {
            return Ok(Some(vec![]));
        };

        // query outgoing calls
        let request =
            query::QueryRequest::CallHierarchyOutgoing(query::CallHierarchyOutgoingRequest {
                item,
            });
        let Some(query::QueryResponse::CallHierarchyOutgoing(response)) =
            self.query_for_uri(&params.item.uri, request)
        else {
            return Ok(Some(vec![]));
        };
        let calls = response.calls;

        // convert to LSP
        let lsp_calls: Vec<lsp::CallHierarchyOutgoingCall> = calls
            .iter()
            .filter_map(|c| outgoing_call_to_lsp(session, c))
            .collect();

        Ok(Some(lsp_calls))
    }

    // ------------------------------------------------------------------------
    // TYPE HIERARCHY
    // ------------------------------------------------------------------------

    async fn prepare_type_hierarchy(
        &self,
        params: lsp::TypeHierarchyPrepareParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TypeHierarchyItem>>> {
        let uri_str = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let session = self.session();
        let Some(doc) = self.open_documents.get(&uri_str) else {
            return Ok(None);
        };
        let Some(file) = self.get_query_file_for_open_document(&doc) else {
            return Ok(None);
        };
        let Some(offset) = position_to_byte(&file, &params.text_document_position_params.position)
        else {
            return Ok(None);
        };

        let query_uri = Self::query_uri(&params.text_document_position_params.text_document.uri);
        let request =
            query::QueryRequest::PrepareTypeHierarchy(query::PrepareTypeHierarchyRequest {
                uri: query_uri,
                offset,
            });
        let Some(query::QueryResponse::PrepareTypeHierarchy(response)) = self.query_for_uri(
            &params.text_document_position_params.text_document.uri,
            request,
        ) else {
            return Ok(None);
        };
        let Some(item) = response.item else {
            return Ok(None);
        };

        let Some(lsp_item) = type_hierarchy_item_to_lsp(session, &item) else {
            return Ok(None);
        };

        Ok(Some(vec![lsp_item]))
    }

    async fn supertypes(
        &self,
        params: lsp::TypeHierarchySupertypesParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TypeHierarchyItem>>> {
        // extract query item from lsp data
        let session = self.session();
        let Some(item) = query_type_hierarchy_item_from_lsp(&params.item) else {
            return Ok(Some(vec![]));
        };

        // query supertypes
        let request =
            query::QueryRequest::TypeHierarchySupertypes(query::TypeHierarchySupertypesRequest {
                item,
            });
        let Some(query::QueryResponse::TypeHierarchySupertypes(response)) =
            self.query_for_uri(&params.item.uri, request)
        else {
            return Ok(Some(vec![]));
        };
        let supertypes = response.items;

        // convert to LSP
        let lsp_items: Vec<lsp::TypeHierarchyItem> = supertypes
            .iter()
            .filter_map(|t| type_hierarchy_item_to_lsp(session, t))
            .collect();

        Ok(Some(lsp_items))
    }

    async fn subtypes(
        &self,
        params: lsp::TypeHierarchySubtypesParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TypeHierarchyItem>>> {
        // extract query item from lsp data
        let session = self.session();
        let Some(item) = query_type_hierarchy_item_from_lsp(&params.item) else {
            return Ok(Some(vec![]));
        };

        // query subtypes
        let request =
            query::QueryRequest::TypeHierarchySubtypes(query::TypeHierarchySubtypesRequest {
                item,
            });
        let Some(query::QueryResponse::TypeHierarchySubtypes(response)) =
            self.query_for_uri(&params.item.uri, request)
        else {
            return Ok(Some(vec![]));
        };
        let subtypes = response.items;

        // convert to LSP
        let lsp_items: Vec<lsp::TypeHierarchyItem> = subtypes
            .iter()
            .filter_map(|t| type_hierarchy_item_to_lsp(session, t))
            .collect();

        Ok(Some(lsp_items))
    }
}

#[cfg(test)]
mod tests {
    use super::DestackLanguageServer;
    use destack_lsp_types as lsp;
    use destack_service::query;

    /// Map umbrella refactor kinds to all workspace refactor buckets.
    #[test]
    fn test_query_code_action_kinds_maps_refactor_umbrella() {
        let mapped = DestackLanguageServer::query_code_action_kinds(&lsp::CodeActionKind::REFACTOR);

        assert_eq!(
            mapped,
            vec![
                query::CodeActionKind::Refactor,
                query::CodeActionKind::RefactorExtract,
                query::CodeActionKind::RefactorInline,
                query::CodeActionKind::RefactorRewrite,
            ]
        );
    }

    /// Map source kind sub prefixes to organize imports.
    #[test]
    fn test_query_code_action_kinds_maps_source_sub_prefix() {
        let kind = lsp::CodeActionKind::new("source.organizeImports.destack");
        let mapped = DestackLanguageServer::query_code_action_kinds(&kind);

        assert_eq!(mapped, vec![query::CodeActionKind::SourceOrganizeImports]);
    }

    /// Build code action mapping with deduplicated mapped kinds.
    #[test]
    fn test_query_code_action_context_deduplicates_mapped_kinds() {
        let context = lsp::CodeActionContext {
            diagnostics: Vec::new(),
            only: Some(vec![
                lsp::CodeActionKind::REFACTOR,
                lsp::CodeActionKind::REFACTOR_INLINE,
            ]),
            trigger_kind: Some(lsp::CodeActionTriggerKind::INVOKED),
        };

        let mapped = DestackLanguageServer::query_code_action_context(&context);

        assert_eq!(
            mapped.only,
            vec![
                query::CodeActionKind::Refactor,
                query::CodeActionKind::RefactorExtract,
                query::CodeActionKind::RefactorInline,
                query::CodeActionKind::RefactorRewrite,
            ]
        );
        assert!(mapped.include_disabled);
    }

    /// Keep include disabled false when no kind filter exists.
    #[test]
    fn test_query_code_action_context_without_only_filter() {
        let context = lsp::CodeActionContext {
            diagnostics: Vec::new(),
            only: None,
            trigger_kind: Some(lsp::CodeActionTriggerKind::INVOKED),
        };

        let mapped = DestackLanguageServer::query_code_action_context(&context);

        assert!(mapped.only.is_empty());
        assert!(!mapped.include_disabled);
    }

    /// Offer identifier-style commit characters for callable items.
    #[test]
    fn test_completion_commit_characters_for_function() {
        let commit_characters =
            DestackLanguageServer::completion_commit_characters(query::CompletionKind::Function);

        assert_eq!(
            commit_characters,
            Some(vec![
                ".".to_string(),
                ",".to_string(),
                ";".to_string(),
                "(".to_string(),
            ])
        );
    }

    /// Skip commit characters for snippet-only pseudo items.
    #[test]
    fn test_completion_commit_characters_for_snippet() {
        let commit_characters =
            DestackLanguageServer::completion_commit_characters(query::CompletionKind::Snippet);

        assert!(commit_characters.is_none());
    }
}
