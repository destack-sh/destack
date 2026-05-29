use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant};

use dashmap::{DashMap, DashSet};
#[cfg(test)]
use destack_artifact::MemoryCacheStore;
use destack_core::StableHasher;
use destack_lsp_server::{Client, LanguageServer, UriExt, jsonrpc};
use destack_lsp_types as lsp;
use destack_query as query;
use destack_service::{
    FileChange, LanguageService, LanguageServiceError, LanguageServiceMessage,
    LanguageServiceMessageKind as ProtocolMessageKind, LanguageServiceResult, QueryResult,
    QueryRevision, TextChange, TextPosition, TextRange,
};
use destack_session::{Session, open_repository_from_fs};
use destack_source::{
    BatchEdit, File, FileId, FileSystem, FileWatchEvent, FileWatchEventKind, ModuleId,
    OverlayFileSystem, PhysicalFileSystem, ProfileId, Span, TargetId,
};
use destack_workspace::{DestackLayoutOverride, Environment, Repository, Revision, Settings};
use serde::{Deserialize, Serialize};
use serde_json::{from_value, to_value};
use tokio::sync::Notify;

use crate::query::assist::{code_lens_to_lsp, inlay_hint_to_lsp};
use crate::query::common::{byte_span_to_range, position_to_byte, span_to_location};
use crate::query::diagnostic::{code_action_to_lsp, diagnostic_to_lsp_diagnostic};
use crate::query::navigation::{
    call_hierarchy_item_to_lsp, call_hierarchy_query_item_from_lsp, document_highlight_to_lsp,
    document_link_to_lsp, document_symbol_to_lsp, incoming_call_to_lsp,
    navigation_target_to_location, navigation_targets_to_locations, outgoing_call_to_lsp,
    selection_range_to_lsp, type_hierarchy_item_to_lsp, type_hierarchy_query_item_from_lsp,
    workspace_symbol_to_lsp,
};
use crate::query::refactor::batch_edit_to_workspace_edit;
use crate::query::semantic;
use crate::server::file::{
    build_file_watchers, completion_kind_to_lsp, create_language_service, diagnostic_result_id,
    file_from_image_for_diagnostics, format_file, format_range, normalize_line_endings,
    tracked_file_globs,
};
use crate::server::progress::WorkDoneProgressTracker;
use crate::server::token::semantic_tokens_edits;
use crate::uri::{lsp_uri_for_file, lsp_uri_for_source_uri, source_uri_from_lsp};

const PARTIAL_RESULT_CHUNK_SIZE: usize = 128;
const WORKSPACE_DIAGNOSTIC_PARTIAL_CHUNK_SIZE: usize = 128;
const AUTO_IMPORT_DETAIL_PREFIX: &str = "Auto import from ";
const SLOW_DIAGNOSTIC_WARN_DURATION: Duration = Duration::from_secs(2);

/// Convert LSP text changes into service text changes.
fn text_changes_from_lsp(changes: Vec<lsp::TextDocumentContentChangeEvent>) -> Vec<TextChange> {
    changes
        .into_iter()
        .map(|change| TextChange {
            range: change.range.map(|range| TextRange {
                start: text_position_from_lsp(range.start),
                end: text_position_from_lsp(range.end),
            }),
            text: change.text,
        })
        .collect()
}

/// Convert one LSP text position into a service text position.
fn text_position_from_lsp(position: lsp::Position) -> TextPosition {
    TextPosition {
        line: position.line,
        character: position.character,
    }
}

/// Canonicalize a root path for stable dedupe comparisons.
fn canonical_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Return true when a root list already contains the path after canonicalization.
fn contains_root(roots: &[PathBuf], path: &Path) -> bool {
    let path = canonical_path(path);
    roots.iter().any(|root| canonical_path(root) == path)
}

/// Build one query position from an LSP file callback.
fn query_position(
    module: query::QueryModule,
    file_id: FileId,
    offset: u32,
) -> query::QueryPosition {
    query::QueryPosition {
        module,
        file_id,
        offset,
    }
}

/// Build one query range from an LSP file callback.
fn query_range(
    module: query::QueryModule,
    file_id: FileId,
    start: u32,
    end: u32,
) -> query::QueryRange {
    let range_start = start.min(end);
    let range_end = start.max(end);

    query::QueryRange {
        module,
        span: Span::new(file_id, range_start, range_end),
    }
}

/// File state used by one LSP query request.
struct QueryFile {
    /// The local file path.
    path: PathBuf,
    /// The exact revision used for the query.
    revision: Revision,
    /// The query module.
    module: query::QueryModule,
    /// The source file id.
    file_id: FileId,
    /// The coherent file contents.
    file: Arc<File>,
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
    /// The semantic revision that produced these edits.
    revision: Revision,
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

/// Configuration for query behavior.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QuerySettings {
    /// The selected target name for queries.
    target: Option<String>,
}

/// LSP configuration settings.
#[derive(Debug)]
struct LspSettings {
    /// Whether auto import completions are enabled.
    completion_auto_imports: AtomicBool,
    /// Whether parameter name inlay hints are enabled.
    parameter_inlay_hints: AtomicBool,
    /// Whether inferred type inlay hints are enabled.
    type_inlay_hints: AtomicBool,
    /// The selected target name for queries.
    query_target: RwLock<Option<String>>,
}

impl Default for LspSettings {
    /// Create default LSP settings.
    fn default() -> Self {
        Self {
            completion_auto_imports: AtomicBool::new(true),
            parameter_inlay_hints: AtomicBool::new(true),
            type_inlay_hints: AtomicBool::new(true),
            query_target: RwLock::new(None),
        }
    }
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
    /// The shared repository.
    repository: OnceLock<Arc<Repository>>,
    /// The language service.
    language_service: OnceLock<Arc<LanguageService>>,
    /// Compiler worker count for the owned language service.
    workers: usize,
    /// Notification used to wake requests waiting on progress cancellation.
    progress_cancel_notify: Arc<Notify>,
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
    settings: LspSettings,
}

#[allow(clippy::too_many_arguments)]
impl DestackLanguageServer {
    /// Create a new language server instance.
    pub fn new(client: Client) -> Self {
        // initialize server state
        Self {
            client,
            repository: OnceLock::new(),
            language_service: OnceLock::new(),
            workers: Session::default_worker_count(),
            progress_cancel_notify: Arc::new(Notify::new()),
            semantic_tokens_cache: DashMap::new(),
            semantic_tokens_counter: AtomicU64::new(1),
            cancelled_progress_tokens: DashSet::new(),
            watch_registration_id: OnceLock::new(),
            completion_label_details_supported: OnceLock::new(),
            code_action_data_supported: OnceLock::new(),
            code_action_edit_resolve_supported: OnceLock::new(),
            settings: LspSettings::default(),
        }
    }

    /// Create a new language server instance with an explicit worker count.
    #[cfg(any(test, feature = "test"))]
    pub(crate) fn with_workers(client: Client, workers: usize) -> Self {
        let mut server = Self::new(client);
        server.workers = workers;

        server
    }

    /// Get the repository (must be called after initialize).
    #[inline]
    fn repository(&self) -> &Arc<Repository> {
        self.repository.get().expect("repository not initialized")
    }

    /// Get the language service (must be called after initialize).
    #[inline]
    fn language_service(&self) -> &Arc<LanguageService> {
        self.language_service
            .get()
            .expect("language service not initialized")
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
        let mut hasher = StableHasher::new();
        tokens.len().hash(&mut hasher);
        for token in tokens {
            token.delta_line.hash(&mut hasher);
            token.delta_start.hash(&mut hasher);
            token.length.hash(&mut hasher);
            token.token_type.hash(&mut hasher);
            token.token_modifiers_bitset.hash(&mut hasher);
        }
        hasher.finish_u64()
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
            .completion_auto_imports
            .load(Ordering::Relaxed)
    }

    /// Check whether parameter name inlay hints are enabled.
    fn parameter_inlay_hints_enabled(&self) -> bool {
        self.settings.parameter_inlay_hints.load(Ordering::Relaxed)
    }

    /// Check whether inferred type inlay hints are enabled.
    fn type_inlay_hints_enabled(&self) -> bool {
        self.settings.type_inlay_hints.load(Ordering::Relaxed)
    }

    /// Return the selected query target.
    fn query_target(&self) -> Option<String> {
        match self.settings.query_target.read() {
            Ok(target) => target.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    /// Set the selected query target.
    fn set_query_target(&self, target: Option<String>) {
        let mut current = match self.settings.query_target.write() {
            Ok(current) => current,
            Err(poisoned) => poisoned.into_inner(),
        };

        *current = target;
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
            lsp::ConfigurationItem {
                scope_uri: None,
                section: Some("destack.query".to_string()),
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

        // update completion settings when parsing succeeds
        if let Some(value) = values.first().cloned() {
            match from_value::<CompletionSettings>(value) {
                Ok(parsed) => {
                    if let Some(auto_imports) = parsed.auto_imports {
                        self.settings
                            .completion_auto_imports
                            .store(auto_imports, Ordering::Relaxed);
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
                        self.settings
                            .parameter_inlay_hints
                            .store(parameter_hints, Ordering::Relaxed);
                    }
                    if let Some(type_hints) = parsed.type_hints {
                        self.settings
                            .type_inlay_hints
                            .store(type_hints, Ordering::Relaxed);
                    }
                }
                Err(error) => {
                    tracing::debug!(?error, "lsp.config.parse_inlay_failed");
                }
            }
        }

        // update query settings when parsing succeeds
        if let Some(value) = values.get(2).cloned() {
            match from_value::<QuerySettings>(value) {
                Ok(parsed) => {
                    self.set_query_target(parsed.target);
                }
                Err(error) => {
                    tracing::debug!(?error, "lsp.config.parse_query_failed");
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

    /// Execute one query through the language service.
    fn execute_query(
        &self,
        path: &Path,
        request: query::QueryRequest,
        revision: QueryRevision,
    ) -> Option<QueryResult> {
        let result = self.language_service().query(path, request, revision);

        match result {
            Ok(response) => Some(response),
            Err(error) => {
                tracing::debug!(?error, path = ?path, "lsp.query.failed");
                None
            }
        }
    }

    /// Return the selected query profile for one exact revision.
    fn query_profile(
        &self,
        path: &Path,
        revision: Revision,
        module_id: Option<ModuleId>,
    ) -> Option<ProfileId> {
        let target_name = self.query_target()?;
        let repository = self.repository();

        // prefer the module package when the file is tracked
        let package_id = if let Some(module_id) = module_id {
            repository
                .module(revision, module_id)
                .ok()
                .flatten()
                .map(|module| module.package_id)?
        } else {
            repository
                .nearest_package(revision, path)
                .ok()
                .flatten()
                .map(|package| package.id)?
        };

        let target_id = TargetId::new(package_id, target_name.as_str());
        let profile = repository
            .target_profile(revision, target_id)
            .ok()
            .flatten()?;

        Some(profile.id())
    }

    /// Return the exact file state for one LSP query uri.
    fn query_file_for_uri(&self, uri: &lsp::Uri) -> Option<QueryFile> {
        let path = uri.to_file_path().map(|path| path.into_owned())?;
        let view = self.language_service().file_view(&path).ok()?;
        let repository = view.repository();
        let revision = view.revision();
        let file_id = view.file_id;
        let module_id = repository
            .module_id_for_file(revision, file_id)
            .ok()
            .flatten()?;
        let profile_id = self.query_profile(&path, revision, Some(module_id))?;
        let module = query::QueryModule {
            module_id,
            profile_id,
        };

        Some(QueryFile {
            path,
            revision,
            module,
            file_id,
            file: Arc::clone(&view.file),
        })
    }

    /// Execute one query against an exact file revision.
    fn query_file(&self, file: &QueryFile, request: query::QueryRequest) -> Option<QueryResult> {
        self.execute_query(&file.path, request, QueryRevision::Exact(file.revision))
    }

    /// Resolve the current semantic revision for the root that owns a path.
    fn revision_at(&self, path: &Path) -> Option<Revision> {
        self.language_service().revision_at(path).ok()
    }

    /// Resolve one local file path from an LSP uri.
    fn path_from_uri(uri: &lsp::Uri) -> Option<PathBuf> {
        uri.to_file_path().map(|path| path.into_owned())
    }

    /// Read normalized text from the repository file system.
    fn file_system_text(&self, path: &Path) -> Option<String> {
        self.repository()
            .file_system()
            .read_to_string(path)
            .ok()
            .map(normalize_line_endings)
    }

    /// Return true when the uri or path belongs to an open editor document.
    fn is_open_document_uri_or_path(&self, uri: &lsp::Uri, path: &Path) -> bool {
        let Some(language_service) = self.language_service.get() else {
            return false;
        };

        let _ = uri;
        language_service.has_open_file(path)
    }

    /// Convert an LSP code action kind into service query kinds.
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

        // formatter owns import organization
        if kind_name == lsp::CodeActionKind::SOURCE_ORGANIZE_IMPORTS.as_str()
            || kind_name.starts_with("source.organizeImports.")
        {
            return Vec::new();
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
                query::CodeActionKind::SourceFixAll,
            ];
        }

        Vec::new()
    }

    /// Build query code action context from LSP code action context.
    fn query_code_action_context(context: &lsp::CodeActionContext) -> query::CodeActionContext {
        let mut only = Vec::new();
        let mut seen = HashSet::new();

        // map each requested lsp kind to query code action kinds
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

    /// Publish service messages with an explicit client handle.
    async fn publish_watch_messages_for_command(
        client: &Client,
        messages: Vec<LanguageServiceMessage>,
    ) {
        for message in messages {
            let message_type = match message.kind {
                ProtocolMessageKind::Info => lsp::MessageType::INFO,
                ProtocolMessageKind::Warning => lsp::MessageType::WARNING,
                ProtocolMessageKind::Error => lsp::MessageType::ERROR,
            };
            client.log_message(message_type, message.message).await;
        }
    }

    /// Publish one complete service result.
    async fn apply_service_result(&self, result: LanguageServiceResult) {
        let files_by_id = result
            .updates
            .iter()
            .filter_map(|update| {
                let image = update.file.as_ref()?;
                let file = file_from_image_for_diagnostics(image)?;

                Some((file.id, file))
            })
            .collect::<HashMap<_, _>>();

        // publish diagnostics per updated file
        for update in result.updates {
            let file = update
                .file
                .as_ref()
                .and_then(file_from_image_for_diagnostics);
            let uri = lsp_uri_for_source_uri(&update.diagnostic_uri)
                .or_else(|| file.as_ref().and_then(|file| lsp_uri_for_file(file)));
            let Some(uri) = uri else {
                continue;
            };
            let file_for_id = |file_id| {
                if let Some(file) = file.as_ref()
                    && file_id == file.id
                {
                    return Some(file.clone());
                }

                files_by_id.get(&file_id).cloned()
            };
            let diagnostics: Vec<lsp::Diagnostic> = update
                .diagnostics
                .into_iter()
                .filter_map(|diagnostic| diagnostic_to_lsp_diagnostic(&diagnostic, &file_for_id))
                .collect();

            self.client
                .publish_diagnostics(uri, diagnostics, update.diagnostic_version)
                .await;
        }

        Self::publish_watch_messages_for_command(&self.client, result.messages).await;
    }

    /// Apply one watch-event batch through the language service and publish the result.
    async fn apply_watch_events(&self, events: Vec<FileWatchEvent>) {
        if events.is_empty() {
            return;
        }

        let result = self
            .run_service(move |service| service.apply_watch_events(events))
            .await;
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                tracing::debug!(?error, "lsp.watch.apply_failed");
                return;
            }
        };

        tracing::trace!(updates = result.updates.len(), "lsp.watch.apply");
        self.apply_service_result(result).await;
    }

    /// Yield once so tests can wait for any in-flight service task completion.
    #[cfg(any(test, feature = "test"))]
    pub(crate) async fn wait_for_mutation_idle_for_tests(&self) {
        tokio::task::yield_now().await;
    }

    /// Run one blocking service operation through the language service.
    async fn run_service<T, F>(&self, operation: F) -> Result<T, LanguageServiceError>
    where
        T: Send + 'static,
        F: FnOnce(&LanguageService) -> Result<T, LanguageServiceError> + Send + 'static,
    {
        // clone service for blocking runtime execution
        let language_service = self.language_service().clone();
        let join_result =
            tokio::task::spawn_blocking(move || operation(language_service.as_ref())).await;
        match join_result {
            Ok(result) => result,
            Err(error) => Err(LanguageServiceError::Internal {
                detail: format!("service task join failed: {error}"),
            }),
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

    /// Publish a partial root diagnostics payload.
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

        // determine root from params
        #[allow(deprecated)]
        let cwd = if let Some(path) = params
            .root_uri
            .as_ref()
            .and_then(|uri| uri.to_file_path().map(|p| p.into_owned()))
        {
            path
        } else if let Some(path) = params.root_path.clone().map(PathBuf::from) {
            path
        } else {
            std::env::current_dir().map_err(|error| {
                jsonrpc::Error::invalid_params(format!(
                    "failed to resolve current directory for initialize: {error}"
                ))
            })?
        };

        // collect initialization roots from workspace folders
        let mut initialize_roots = Vec::new();
        if let Some(workspace_folders) = params.workspace_folders.as_ref() {
            for folder in workspace_folders {
                let Some(path) = folder.uri.to_file_path().map(|path| path.into_owned()) else {
                    continue;
                };

                if !contains_root(&initialize_roots, &path) {
                    initialize_roots.push(path);
                }
            }
        }
        if initialize_roots.is_empty() {
            initialize_roots.push(cwd.clone());
        }

        // create repository with overlay filesystem
        let physical_fs = Arc::new(PhysicalFileSystem::new());
        let overlay_fs = Arc::new(OverlayFileSystem::with_inner(physical_fs));
        let repository = open_repository_from_fs(
            cwd.clone(),
            overlay_fs.clone(),
            Environment::capture_process(),
            Settings::default(),
            DestackLayoutOverride::default(),
        )
        .map_err(|error| {
            tracing::error!("lsp.initialize.repository_import_failed: {error}");
            jsonrpc::Error::internal_error()
        })?;
        #[cfg(test)]
        let repository = repository.with_cache(Arc::new(MemoryCacheStore::new()));
        let root = repository.workspace_root().to_path_buf();
        self.client
            .log_message(
                lsp::MessageType::INFO,
                format!("destack.initialize.root={}", root.display()),
            )
            .await;

        // merge discovery root with initialize roots
        let mut opened_roots = vec![root.clone()];
        for initialize_root in initialize_roots {
            if !contains_root(&opened_roots, &initialize_root) {
                opened_roots.push(initialize_root);
            }
        }

        let repository = Arc::new(repository);
        if self.repository.set(repository.clone()).is_err() {
            tracing::warn!("lsp.initialize.repository_already_set");
            return Err(jsonrpc::Error::internal_error());
        }

        // create the language service for the repository
        let language_service = match create_language_service(
            repository.clone(),
            overlay_fs,
            opened_roots,
            self.workers,
        ) {
            Ok(language_service) => Arc::new(language_service),
            Err(error) => {
                tracing::debug!(?error, "lsp.workspace.init_failed");
                return Err(jsonrpc::Error::internal_error());
            }
        };
        if self.language_service.set(language_service).is_err() {
            tracing::warn!("lsp.initialize.language_service_already_set");
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

        // build file operation filters for root notifications
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
            document_on_type_formatting_provider: Some(lsp::DocumentOnTypeFormattingOptions {
                first_trigger_character: ";".to_string(),
                more_trigger_character: Some(vec!["}".to_string()]),
            }),
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
                    "destack.reload".to_string(),
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

        // reload filesystem state so config changes refresh diagnostics
        let started_at = Instant::now();
        let result = self.run_service(|service| service.reload_all()).await;
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                tracing::debug!(?error, "lsp.config_reload.failed");
                return;
            }
        };

        self.apply_service_result(result).await;

        tracing::info!(
            elapsed_ms = started_at.elapsed().as_millis(),
            "lsp.config_reload.completed"
        );
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
        let content = normalize_line_endings(params.text_document.text);
        let version = params.text_document.version;

        self.client
            .log_message(lsp::MessageType::INFO, format!("did_open: {uri_str}"))
            .await;

        let Some(path) = Self::path_from_uri(&params.text_document.uri) else {
            return;
        };

        let query_uri = source_uri_from_lsp(&params.text_document.uri);
        let path_for_operation = path.clone();
        let result = self
            .run_service(move |service| {
                service.open_file(
                    &path_for_operation,
                    query_uri,
                    version,
                    FileChange::Text { content },
                )
            })
            .await;
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                tracing::debug!(?error, path = %path.display(), "lsp.open_document.failed");
                return;
            }
        };

        self.apply_service_result(result).await;
    }

    async fn did_change(&self, params: lsp::DidChangeTextDocumentParams) {
        // ignore empty change batches
        if params.content_changes.is_empty() {
            return;
        };
        let uri_str = params.text_document.uri.to_string();
        let Some(path) = Self::path_from_uri(&params.text_document.uri) else {
            return;
        };
        let query_uri = source_uri_from_lsp(&params.text_document.uri);
        let changes = text_changes_from_lsp(params.content_changes);

        let version = params.text_document.version;
        let path_for_operation = path.clone();
        let result = self
            .run_service(move |service| {
                service.patch_text_file(&path_for_operation, query_uri, version, changes)
            })
            .await;
        let result = match result {
            Ok(result) => result,
            Err(LanguageServiceError::StaleOpenFile {
                incoming, current, ..
            }) => {
                tracing::debug!(
                    uri = %uri_str,
                    incoming,
                    current,
                    "lsp.did_change.stale_version"
                );
                return;
            }
            Err(error) => {
                tracing::debug!(?error, path = %path.display(), "lsp.update_document.failed");
                return;
            }
        };
        self.apply_service_result(result).await;
    }

    async fn did_save(&self, params: lsp::DidSaveTextDocumentParams) {
        let uri_str = params.text_document.uri.to_string();
        let Some(path) = Self::path_from_uri(&params.text_document.uri) else {
            return;
        };

        let content = if let Some(text) = params.text {
            Some(normalize_line_endings(text))
        } else {
            self.file_system_text(&path)
        };

        let Some(content) = content else {
            tracing::debug!(uri = %uri_str, "lsp.did_save.read_failed");
            return;
        };

        let path_for_operation = path.clone();
        let result = self
            .run_service(move |service| {
                service.save_file(&path_for_operation, FileChange::Text { content })
            })
            .await;
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                tracing::debug!(?error, path = %path.display(), "lsp.save_file.failed");
                return;
            }
        };
        self.apply_service_result(result).await;
    }

    async fn did_close(&self, params: lsp::DidCloseTextDocumentParams) {
        let uri_str = params.text_document.uri.to_string();
        self.semantic_tokens_cache.remove(&uri_str);
        let Some(path) = Self::path_from_uri(&params.text_document.uri) else {
            return;
        };
        let path_for_operation = path.clone();
        let result = self
            .run_service(move |service| service.close_file(&path_for_operation))
            .await;
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                tracing::debug!(?error, path = %path.display(), "lsp.close_file.failed");
                return;
            }
        };

        self.apply_service_result(result).await;
    }

    async fn did_change_workspace_folders(&self, params: lsp::DidChangeWorkspaceFoldersParams) {
        let language_service = self.language_service().clone();

        for folder in params.event.added {
            if let Some(path) = folder.uri.to_file_path().map(|path| path.into_owned()) {
                let open_result = {
                    let language_service = language_service.clone();
                    tokio::task::spawn_blocking(move || language_service.open_root(path)).await
                };

                let open_result = match open_result {
                    Ok(result) => result,
                    Err(error) => {
                        tracing::debug!(?error, "lsp.workspace.add_join_failed");
                        continue;
                    }
                };
                if let Err(error) = open_result {
                    tracing::debug!(?error, "lsp.workspace.add_failed");
                }
            }
        }

        for folder in params.event.removed {
            if let Some(path) = folder.uri.to_file_path().map(|path| path.into_owned()) {
                let close_result = {
                    let language_service = language_service.clone();
                    let path_for_close = path.clone();
                    tokio::task::spawn_blocking(move || {
                        language_service.close_root(&path_for_close)
                    })
                    .await
                };

                let close_result = match close_result {
                    Ok(result) => result,
                    Err(error) => {
                        tracing::debug!(?error, "lsp.workspace.remove_join_failed");
                        continue;
                    }
                };
                if let Err(error) = close_result {
                    tracing::debug!(?error, "lsp.workspace.remove_failed");
                }
            }
        }
    }

    async fn did_change_watched_files(&self, params: lsp::DidChangeWatchedFilesParams) {
        // apply external file changes through the service
        let mut events = Vec::new();
        for change in params.changes {
            // resolve file path
            let Some(path) = change.uri.to_file_path().map(|path| path.into_owned()) else {
                continue;
            };

            // skip updates for files that are still open in the editor
            if self.is_open_document_uri_or_path(&change.uri, &path) {
                continue;
            }

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
        let mut query_root: Option<PathBuf> = None;
        let mut expected_revision: Option<Revision> = None;
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

            // enforce a single root for rename write consistency
            let old_root = self.language_service().root_at(&old_path).ok();
            let new_root = self.language_service().root_at(&new_path).ok();
            let root = old_root.or(new_root);
            let Some(root) = root else {
                continue;
            };

            if let Some(existing_root) = query_root.as_ref() {
                if existing_root != &root {
                    self.client
                        .log_message(
                            lsp::MessageType::WARNING,
                            format!(
                                "destack.will_rename_files.multi_root_rejected first_root={} next_root={}",
                                existing_root.display(),
                                root.display()
                            ),
                        )
                        .await;
                    tracing::warn!(
                        first_root = %existing_root.display(),
                        next_root = %root.display(),
                        "lsp.will_rename_files.multi_root_rejected"
                    );
                    return Ok(None);
                }
            } else {
                let Some(revision) = self.revision_at(&root) else {
                    return Ok(None);
                };
                query_root = Some(root);
                expected_revision = Some(revision);
            }

            renames.push(query::FileRenameEntry { old_path, new_path });
        }
        if renames.is_empty() {
            return Ok(None);
        }
        let Some(query_root) = query_root else {
            return Ok(None);
        };
        let Some(expected_revision) = expected_revision else {
            return Ok(None);
        };

        // build workspace edits for import specifiers
        let Some(profile_id) = self.query_profile(&query_root, expected_revision, None) else {
            return Ok(None);
        };
        let request = query::QueryRequest::RenameFiles(query::RenameFilesRequest {
            profile_ids: vec![profile_id],
            renames,
        });
        let response = self.execute_query(
            &query_root,
            request,
            QueryRevision::Current(expected_revision),
        );
        let Some(response) = response else {
            return Ok(None);
        };
        let revision = response.revision;
        let query::QueryResponse::RenameFiles(response) = response.response else {
            return Ok(None);
        };
        let Some(edit) = response.edit else {
            return Ok(None);
        };
        if edit.is_empty() {
            return Ok(None);
        }

        let workspace_edit = batch_edit_to_workspace_edit(self.repository(), revision, &edit);
        Ok(Some(workspace_edit))
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
        let repository = self.repository().clone();
        let mut events = Vec::new();
        for file in params.files {
            // parse the file uri
            let Ok(uri) = file.uri.parse::<lsp::Uri>() else {
                continue;
            };

            // resolve the file path
            let Some(path) = uri.to_file_path().map(|path| path.into_owned()) else {
                continue;
            };

            // skip updates for files that are still open in the editor
            if self.is_open_document_uri_or_path(&uri, &path) {
                continue;
            }

            // skip files that are not yet visible on disk
            if repository.file_system().exists(&path).ok() != Some(true) {
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
        let repository = self.repository().clone();
        let mut events = Vec::new();
        for file in params.files {
            // parse rename uris
            let Ok(old_uri) = file.old_uri.parse::<lsp::Uri>() else {
                continue;
            };
            let Ok(new_uri) = file.new_uri.parse::<lsp::Uri>() else {
                continue;
            };

            // resolve paths
            let Some(old_path) = old_uri.to_file_path().map(|path| path.into_owned()) else {
                continue;
            };
            let Some(new_path) = new_uri.to_file_path().map(|path| path.into_owned()) else {
                continue;
            };

            // skip updates for files that are still open in the editor
            if self.is_open_document_uri_or_path(&old_uri, &old_path)
                || self.is_open_document_uri_or_path(&new_uri, &new_path)
            {
                continue;
            }

            // clear diagnostics for the old uri before publishing the renamed path
            self.client
                .publish_diagnostics(old_uri.clone(), Vec::new(), None)
                .await;

            // skip files that are not yet visible on disk
            if repository.file_system().exists(&new_path).ok() != Some(true) {
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

            // resolve the file path
            if let Some(path) = uri.to_file_path().map(|path| path.into_owned()) {
                // skip updates for files that are still open in the editor
                if self.is_open_document_uri_or_path(&uri, &path) {
                    continue;
                }

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
        let uri_str = params.text_document.uri.to_string();
        let tracked_path = params
            .text_document
            .uri
            .to_file_path()
            .map(|path| path.into_owned());
        let Some(path) = tracked_path.as_ref() else {
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

        let path = path.clone();
        let path_for_query = path.clone();
        let snapshot = match self
            .run_service(move |service| service.file_diagnostics(&path_for_query))
            .await
        {
            Ok(snapshot) => snapshot,
            Err(error) => {
                tracing::error!(path = %path.display(), ?error, "lsp.diagnostic.snapshot_failed");
                return Err(jsonrpc::Error::internal_error());
            }
        };
        let Some(snapshot) = snapshot else {
            tracing::debug!(path = %path.display(), uri = %uri_str, "lsp.diagnostic.query_file_not_found");
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
        let file = snapshot.file;
        let diagnostics = snapshot.diagnostics;
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
            .filter_map(|diagnostic| {
                let file_for_id = |file_id| (file_id == file.id).then(|| file.clone());

                diagnostic_to_lsp_diagnostic(&diagnostic, &file_for_id)
            })
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
        let work_done_token = params.work_done_progress_params.work_done_token.clone();
        if let Some(token) = work_done_token.as_ref()
            && self.is_progress_cancelled(token)
        {
            self.clear_progress_cancel(token);
            return Err(jsonrpc::Error::request_cancelled());
        }

        let started_at = Instant::now();
        let previous_ids: std::collections::HashMap<String, String> = params
            .previous_result_ids
            .into_iter()
            .map(|entry| (entry.uri.to_string(), entry.value))
            .collect();

        // start work done progress before collecting diagnostics
        let mut progress = WorkDoneProgressTracker::start(
            self,
            params.work_done_progress_params.work_done_token,
            "Workspace diagnostics",
            "collecting diagnostics",
        )
        .await;
        progress
            .check_cancelled(self, "diagnostics cancelled")
            .await?;

        let snapshots = match self.run_service(|service| service.diagnostics()).await {
            Ok(snapshots) => snapshots,
            Err(error) => {
                tracing::error!(?error, "lsp.workspace_diagnostic.snapshot_failed");
                return Err(jsonrpc::Error::internal_error());
            }
        };

        // collect partial results when supported
        let partial_token = params.partial_result_params.partial_result_token;

        let mut items = Vec::new();
        let mut partial_items = Vec::new();
        let files_by_id = snapshots
            .iter()
            .map(|snapshot| (snapshot.file.id, snapshot.file.clone()))
            .collect::<HashMap<_, _>>();

        // allow cancellation between chunks
        let mut processed = 0usize;
        for snapshot in snapshots {
            let file = snapshot.file;
            let uri = lsp_uri_for_source_uri(&snapshot.diagnostic_uri)
                .or_else(|| lsp_uri_for_file(&file));
            let Some(uri) = uri else {
                continue;
            };
            let diagnostics = snapshot.diagnostics;
            let result_id = diagnostic_result_id(&diagnostics);
            let version = snapshot.diagnostic_version.map(|version| version as i64);
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
                .filter_map(|diagnostic| {
                    let file_for_id = |file_id| {
                        if file_id == file.id {
                            return Some(file.clone());
                        }

                        files_by_id.get(&file_id).cloned()
                    };

                    diagnostic_to_lsp_diagnostic(&diagnostic, &file_for_id)
                })
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

        progress
            .finish_or_cancelled(self, "diagnostics complete", "diagnostics cancelled")
            .await?;

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
            "destack.reload" | "destack.reindex" => {
                let started_at = Instant::now();
                let result = self.run_service(|service| service.reload_all()).await;
                let result = match result {
                    Ok(result) => result,
                    Err(error) => {
                        tracing::debug!(
                            ?error,
                            command = params.command.as_str(),
                            "lsp.command.failed"
                        );
                        return Ok(None);
                    }
                };

                let update_count = result.updates.len();
                let message_count = result.messages.len();
                self.apply_service_result(result).await;

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
                let result = self
                    .run_service(|service| {
                        service
                            .clear_cache_all()
                            .and_then(|()| service.reload_all())
                    })
                    .await;
                let result = match result {
                    Ok(result) => result,
                    Err(error) => {
                        tracing::debug!(?error, "lsp.clear_cache.failed");
                        return Ok(None);
                    }
                };

                let update_count = result.updates.len();
                let message_count = result.messages.len();
                self.apply_service_result(result).await;

                tracing::info!(
                    updates = update_count,
                    messages = message_count,
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "lsp.clear_cache.completed"
                );
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    async fn work_done_progress_cancel(&self, params: lsp::WorkDoneProgressCancelParams) {
        self.cancelled_progress_tokens.insert(params.token);
        self.progress_cancel_notify.notify_waiters();
    }

    // ------------------------------------------------------------------------
    // NAVIGATION
    // ------------------------------------------------------------------------

    async fn goto_definition(
        &self,
        params: lsp::GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<lsp::GotoDefinitionResponse>> {
        let repository = self.repository();
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.query_file_for_uri(uri) else {
            return Ok(None);
        };
        let Some(offset) = position_to_byte(
            &query_file.file,
            &params.text_document_position_params.position,
        ) else {
            return Ok(None);
        };
        let position = query_position(query_file.module, query_file.file_id, offset);
        let request =
            query::QueryRequest::GotoDefinition(query::GotoDefinitionRequest { position });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let revision = response.revision;
        let query::QueryResponse::GotoDefinition(response) = response.response else {
            return Ok(None);
        };
        let Some(target) = response.targets.first() else {
            return Ok(None);
        };

        // convert to LSP location
        let location = navigation_target_to_location(repository, revision, target);
        Ok(location.map(lsp::GotoDefinitionResponse::Scalar))
    }

    async fn goto_declaration(
        &self,
        params: lsp::request::GotoDeclarationParams,
    ) -> jsonrpc::Result<Option<lsp::request::GotoDeclarationResponse>> {
        let repository = self.repository();
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.query_file_for_uri(uri) else {
            return Ok(None);
        };
        let Some(offset) = position_to_byte(
            &query_file.file,
            &params.text_document_position_params.position,
        ) else {
            return Ok(None);
        };
        let position = query_position(query_file.module, query_file.file_id, offset);
        let request =
            query::QueryRequest::GotoDeclaration(query::GotoDeclarationRequest { position });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let revision = response.revision;
        let query::QueryResponse::GotoDeclaration(response) = response.response else {
            return Ok(None);
        };
        let Some(target) = response.targets.first() else {
            return Ok(None);
        };

        let location = navigation_target_to_location(repository, revision, target);
        Ok(location.map(lsp::GotoDefinitionResponse::Scalar))
    }

    async fn goto_type_definition(
        &self,
        params: lsp::request::GotoTypeDefinitionParams,
    ) -> jsonrpc::Result<Option<lsp::request::GotoTypeDefinitionResponse>> {
        let repository = self.repository();
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.query_file_for_uri(uri) else {
            return Ok(None);
        };
        let Some(offset) = position_to_byte(
            &query_file.file,
            &params.text_document_position_params.position,
        ) else {
            return Ok(None);
        };
        let position = query_position(query_file.module, query_file.file_id, offset);
        let request =
            query::QueryRequest::GotoTypeDefinition(query::GotoTypeDefinitionRequest { position });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let revision = response.revision;
        let query::QueryResponse::GotoTypeDefinition(response) = response.response else {
            return Ok(None);
        };
        let Some(target) = response.targets.first() else {
            return Ok(None);
        };

        let location = navigation_target_to_location(repository, revision, target);
        Ok(location.map(lsp::GotoDefinitionResponse::Scalar))
    }

    async fn references(
        &self,
        params: lsp::ReferenceParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::Location>>> {
        let repository = self.repository();
        let uri = &params.text_document_position.text_document.uri;
        let Some(query_file) = self.query_file_for_uri(uri) else {
            return Ok(None);
        };
        let Some(offset) =
            position_to_byte(&query_file.file, &params.text_document_position.position)
        else {
            return Ok(None);
        };
        let position = query_position(query_file.module, query_file.file_id, offset);
        let request = query::QueryRequest::FindReferences(query::FindReferencesRequest {
            position,
            include_declaration: params.context.include_declaration,
        });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let revision = response.revision;
        let query::QueryResponse::FindReferences(response) = response.response else {
            return Ok(None);
        };
        let references = response.references;
        if references.is_empty() {
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
        for reference in references.iter() {
            let Some(location) = span_to_location(repository, revision, reference.target.span)
            else {
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

        progress
            .finish_or_cancelled(self, "references complete", "references cancelled")
            .await?;

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
        let Some(query_file) = self.query_file_for_uri(&params.text_document.uri) else {
            return Ok(None);
        };
        let request = query::QueryRequest::DocumentSymbols(query::DocumentSymbolsRequest {
            module: query_file.module,
        });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let query::QueryResponse::DocumentSymbols(response) = response.response else {
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
        let partial_token = params.partial_result_params.partial_result_token;
        let mut lsp_symbols = Vec::new();
        let mut partial_symbols = Vec::new();
        let mut processed = 0usize;
        for symbol in symbols.iter() {
            let Some(lsp_symbol) = document_symbol_to_lsp(&query_file.file, symbol) else {
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

        progress
            .finish_or_cancelled(self, "symbols complete", "symbols cancelled")
            .await?;

        if lsp_symbols.is_empty() {
            Ok(None)
        } else {
            Ok(Some(lsp::DocumentSymbolResponse::Nested(lsp_symbols)))
        }
    }

    async fn symbol(
        &self,
        params: lsp::WorkspaceSymbolParams,
    ) -> jsonrpc::Result<Option<lsp::OneOf<Vec<lsp::SymbolInformation>, Vec<lsp::WorkspaceSymbol>>>>
    {
        let repository = self.repository();

        // query workspace symbols
        let root = self.repository().workspace_root();
        let Some(revision) = self.revision_at(root) else {
            return Ok(None);
        };
        let Some(profile_id) = self.query_profile(root, revision, None) else {
            return Ok(None);
        };
        let request = query::QueryRequest::WorkspaceSymbols(query::WorkspaceSymbolsRequest {
            query: params.query.clone(),
            profile_ids: vec![profile_id],
            max_results: 100,
        });
        let Some(response) = self.execute_query(root, request, QueryRevision::Exact(revision))
        else {
            return Ok(None);
        };
        let revision = response.revision;
        let query::QueryResponse::WorkspaceSymbols(response) = response.response else {
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
            let Some(lsp_symbol) = workspace_symbol_to_lsp(repository, revision, symbol) else {
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

        progress
            .finish_or_cancelled(self, "symbols complete", "symbols cancelled")
            .await?;

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
        let Some(query_file) = self.query_file_for_uri(uri) else {
            return Ok(None);
        };
        let Some(offset) = position_to_byte(
            &query_file.file,
            &params.text_document_position_params.position,
        ) else {
            return Ok(None);
        };
        let position = query_position(query_file.module, query_file.file_id, offset);
        let request =
            query::QueryRequest::DocumentHighlight(query::DocumentHighlightRequest { position });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let query::QueryResponse::DocumentHighlight(response) = response.response else {
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
            let Some(lsp_highlight) = document_highlight_to_lsp(&query_file.file, highlight) else {
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

        progress
            .finish_or_cancelled(self, "highlights complete", "highlights cancelled")
            .await?;

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
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.query_file_for_uri(uri) else {
            return Ok(None);
        };
        let Some(offset) = position_to_byte(
            &query_file.file,
            &params.text_document_position_params.position,
        ) else {
            return Ok(None);
        };
        let position = query_position(query_file.module, query_file.file_id, offset);
        let request = query::QueryRequest::Hover(query::HoverRequest { position });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let query::QueryResponse::Hover(response) = response.response else {
            return Ok(None);
        };
        let Some(hover_info) = response.hover else {
            return Ok(None);
        };

        let range = hover_info
            .range
            .map(|span| byte_span_to_range(&query_file.file, span));

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

        let uri = &params.text_document_position.text_document.uri;
        let Some(query_file) = self.query_file_for_uri(uri) else {
            return Ok(None);
        };
        let Some(offset) =
            position_to_byte(&query_file.file, &params.text_document_position.position)
        else {
            return Ok(None);
        };
        let position = query_position(query_file.module, query_file.file_id, offset);
        let request = query::QueryRequest::Completion(query::CompletionRequest {
            position,
            trigger,
            include_imports: true,
        });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let query::QueryResponse::Completion(response) = response.response else {
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
                                // keep same file completion edits
                                if edit.span.file != query_file.file_id {
                                    return None;
                                }
                                Some(lsp::TextEdit {
                                    range: byte_span_to_range(&query_file.file, edit.span),
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
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.query_file_for_uri(uri) else {
            return Ok(None);
        };
        let Some(offset) = position_to_byte(
            &query_file.file,
            &params.text_document_position_params.position,
        ) else {
            return Ok(None);
        };
        let position = query_position(query_file.module, query_file.file_id, offset);
        let request = query::QueryRequest::SignatureHelp(query::SignatureHelpRequest { position });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let query::QueryResponse::SignatureHelp(response) = response.response else {
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
        let uri_str = params.text_document.uri.to_string();
        let Some(query_file) = self.query_file_for_uri(&params.text_document.uri) else {
            return Ok(None);
        };
        let request = query::QueryRequest::SemanticTokens(query::SemanticTokensRequest {
            module: query_file.module,
        });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let query::QueryResponse::SemanticTokens(response) = response.response else {
            return Ok(None);
        };
        let tokens = response.tokens;

        // convert to LSP
        let lsp_tokens = semantic::tokens_to_lsp(&query_file.file, &tokens);
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
        let uri_str = params.text_document.uri.to_string();
        let Some(query_file) = self.query_file_for_uri(&params.text_document.uri) else {
            return Ok(None);
        };
        let request = query::QueryRequest::SemanticTokens(query::SemanticTokensRequest {
            module: query_file.module,
        });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let query::QueryResponse::SemanticTokens(response) = response.response else {
            return Ok(None);
        };
        let tokens = response.tokens;

        // convert to LSP
        let lsp_tokens = semantic::tokens_to_lsp(&query_file.file, &tokens);
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
        let Some(query_file) = self.query_file_for_uri(&params.text_document.uri) else {
            return Ok(None);
        };
        let Some(start) = position_to_byte(&query_file.file, &params.range.start) else {
            return Ok(None);
        };
        let Some(end) = position_to_byte(&query_file.file, &params.range.end) else {
            return Ok(None);
        };
        let range = query_range(query_file.module, query_file.file_id, start, end);
        let request =
            query::QueryRequest::SemanticTokensRange(query::SemanticTokensRangeRequest { range });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let query::QueryResponse::SemanticTokensRange(response) = response.response else {
            return Ok(None);
        };
        let tokens = response.tokens;

        // convert to LSP
        let lsp_tokens = semantic::tokens_to_lsp(&query_file.file, &tokens);

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
        let Some(path) = params
            .text_document
            .uri
            .to_file_path()
            .map(|path| path.into_owned())
        else {
            return Ok(None);
        };
        if !self.language_service().has_open_file(&path) {
            return Ok(None);
        }

        // query for folding ranges
        let Some(query_file) = self.query_file_for_uri(&params.text_document.uri) else {
            return Ok(None);
        };
        let request = query::QueryRequest::FoldingRanges(query::FoldingRangesRequest {
            module: query_file.module,
        });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let query::QueryResponse::FoldingRanges(response) = response.response else {
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

        progress
            .finish_or_cancelled(self, "folding ranges complete", "folding ranges cancelled")
            .await?;

        if lsp_ranges.is_empty() {
            Ok(None)
        } else {
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
        let Some(path) = params
            .text_document
            .uri
            .to_file_path()
            .map(|path| path.into_owned())
        else {
            return Ok(None);
        };
        let path_for_query = path.clone();
        let result = self
            .run_service(move |service| {
                let file_view = service.file_view(&path_for_query)?;
                let repository = file_view.repository();
                let revision = file_view.revision();
                let file_id = file_view.file_id;
                let file = file_view.file.clone();

                // format the exact file view
                {
                    let formatter =
                        super::file::formatting_options_for_path(repository, revision, &path);
                    let Some(formatted) =
                        format_file(repository, revision, file_id, &file, formatter)
                    else {
                        return Err(LanguageServiceError::Internal {
                            detail: format!("formatting failed for {}", path.display()),
                        });
                    };

                    Ok((formatted, file))
                }
            })
            .await;
        let Ok((formatted, file)) = result else {
            return Ok(None);
        };

        // return single edit replacing entire document
        let line_count = file.line_count();
        let last_line_index = line_count.saturating_sub(1);
        let last_line_len = file
            .get_line_str(last_line_index)
            .map(|l| l.len())
            .unwrap_or(0);

        Ok(Some(vec![lsp::TextEdit {
            range: lsp::Range {
                start: lsp::Position {
                    line: 0,
                    character: 0,
                },
                end: lsp::Position {
                    line: last_line_index,
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
        let Some(path) = params
            .text_document
            .uri
            .to_file_path()
            .map(|path| path.into_owned())
        else {
            return Ok(None);
        };
        let path_for_query = path.clone();
        let result = self
            .run_service(move |service| {
                let file_view = service.file_view(&path_for_query)?;
                let repository = file_view.repository();
                let revision = file_view.revision();
                let file = file_view.file.clone();

                // format the exact file range
                {
                    let formatter =
                        super::file::formatting_options_for_path(repository, revision, &path);
                    let Some(start_offset) = position_to_byte(&file, &params.range.start) else {
                        return Err(LanguageServiceError::Internal {
                            detail:
                                "range formatting start position is outside the coherent file view"
                                    .to_string(),
                        });
                    };
                    let Some(end_offset) = position_to_byte(&file, &params.range.end) else {
                        return Err(LanguageServiceError::Internal {
                            detail:
                                "range formatting end position is outside the coherent file view"
                                    .to_string(),
                        });
                    };
                    let Some((formatted, edit_range)) =
                        format_range(&file, formatter, start_offset, end_offset)
                    else {
                        return Err(LanguageServiceError::Internal {
                            detail: "range formatting did not produce an edit".to_string(),
                        });
                    };

                    Ok((formatted, edit_range, file))
                }
            })
            .await;
        let Ok((formatted, edit_range, file)) = result else {
            return Ok(None);
        };

        // convert byte range back to LSP range
        let lsp_range = byte_span_to_range(&file, edit_range);

        Ok(Some(vec![lsp::TextEdit {
            range: lsp_range,
            new_text: formatted,
        }]))
    }

    async fn on_type_formatting(
        &self,
        params: lsp::DocumentOnTypeFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TextEdit>>> {
        let Some(path) = params
            .text_document_position
            .text_document
            .uri
            .to_file_path()
            .map(|path| path.into_owned())
        else {
            return Ok(None);
        };
        let path_for_query = path.clone();
        let result = self
            .run_service(move |service| {
                let file_view = service.file_view(&path_for_query)?;
                let repository = file_view.repository();
                let revision = file_view.revision();
                let file = file_view.file.clone();

                // format the exact trigger position
                {
                    let formatter =
                        super::file::formatting_options_for_path(repository, revision, &path);
                    let Some(end_offset) =
                        position_to_byte(&file, &params.text_document_position.position)
                    else {
                        return Err(LanguageServiceError::Internal {
                            detail: "on-type formatting position is outside the coherent file view"
                                .to_string(),
                        });
                    };
                    let trigger_width = params.ch.len() as u32;
                    let start_offset = end_offset.saturating_sub(trigger_width.max(1));
                    let Some((formatted, edit_range)) =
                        format_range(&file, formatter, start_offset, end_offset)
                    else {
                        return Err(LanguageServiceError::Internal {
                            detail: "on-type formatting did not produce an edit".to_string(),
                        });
                    };

                    Ok((formatted, edit_range, file))
                }
            })
            .await;
        let Ok((formatted, edit_range, file)) = result else {
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
        let Some(query_file) = self.query_file_for_uri(&params.text_document.uri) else {
            return Ok(None);
        };
        let positions: Vec<u32> = params
            .positions
            .iter()
            .filter_map(|position| position_to_byte(&query_file.file, position))
            .collect();
        let request = query::QueryRequest::SelectionRanges(query::SelectionRangesRequest {
            module: query_file.module,
            offsets: positions,
        });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let query::QueryResponse::SelectionRanges(response) = response.response else {
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
            let lsp_range = selection_range_to_lsp(&query_file.file, range);
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

        progress
            .finish_or_cancelled(self, "ranges complete", "ranges cancelled")
            .await?;
        Ok(Some(lsp_ranges))
    }

    async fn goto_implementation(
        &self,
        params: lsp::request::GotoImplementationParams,
    ) -> jsonrpc::Result<Option<lsp::request::GotoImplementationResponse>> {
        let repository = self.repository();
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.query_file_for_uri(uri) else {
            return Ok(None);
        };
        let Some(offset) = position_to_byte(
            &query_file.file,
            &params.text_document_position_params.position,
        ) else {
            return Ok(None);
        };
        let position = query_position(query_file.module, query_file.file_id, offset);
        let request =
            query::QueryRequest::GotoImplementation(query::GotoImplementationRequest { position });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let revision = response.revision;
        let query::QueryResponse::GotoImplementation(response) = response.response else {
            return Ok(None);
        };

        // convert to LSP
        let locations = navigation_targets_to_locations(repository, revision, &response.targets);
        if locations.is_empty() {
            return Ok(None);
        }

        Ok(Some(lsp::GotoDefinitionResponse::Array(locations)))
    }

    // ------------------------------------------------------------------------
    // DOCUMENT LINKS
    // ------------------------------------------------------------------------

    async fn document_link(
        &self,
        params: lsp::DocumentLinkParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::DocumentLink>>> {
        let Some(query_file) = self.query_file_for_uri(&params.text_document.uri) else {
            return Ok(None);
        };
        let request = query::QueryRequest::DocumentLinks(query::DocumentLinksRequest {
            module: query_file.module,
        });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let query::QueryResponse::DocumentLinks(response) = response.response else {
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
            let Some(lsp_link) = document_link_to_lsp(&query_file.file, link) else {
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

        progress
            .finish_or_cancelled(self, "links complete", "links cancelled")
            .await?;

        if lsp_links.is_empty() {
            Ok(None)
        } else {
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
        let repository = self.repository();
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

        let Some(query_file) = self.query_file_for_uri(&params.text_document.uri) else {
            return Ok(None);
        };
        let Some(start) = position_to_byte(&query_file.file, &params.range.start) else {
            return Ok(None);
        };
        let Some(end) = position_to_byte(&query_file.file, &params.range.end) else {
            return Ok(None);
        };
        let range = query_range(query_file.module, query_file.file_id, start, end);
        let request =
            query::QueryRequest::CodeActions(query::CodeActionsRequest { range, context });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let revision = response.revision;
        let query::QueryResponse::CodeActions(response) = response.response else {
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
                    revision,
                    edits: action.edits.clone(),
                })
                .ok()
            } else {
                None
            };
            let include_edit = !prefer_lazy_code_action_edits || data.is_none();
            let Some(lsp_action) =
                code_action_to_lsp(repository, revision, action, include_edit, data)
            else {
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

        progress
            .finish_or_cancelled(self, "actions complete", "actions cancelled")
            .await?;

        if lsp_actions.is_empty() {
            Ok(None)
        } else {
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
            self.repository(),
            resolved.revision,
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
        let Some(query_file) = self.query_file_for_uri(&params.text_document.uri) else {
            return Ok(None);
        };
        let request = query::QueryRequest::CodeLenses(query::CodeLensesRequest {
            module: query_file.module,
        });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let query::QueryResponse::CodeLenses(response) = response.response else {
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
            let lsp_lens = code_lens_to_lsp(&query_file.file, lens);
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

        progress
            .finish_or_cancelled(self, "lenses complete", "lenses cancelled")
            .await?;
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
        let Some(query_file) = self.query_file_for_uri(&params.text_document.uri) else {
            return Ok(None);
        };
        let Some(start) = position_to_byte(&query_file.file, &params.range.start) else {
            return Ok(None);
        };
        let Some(end) = position_to_byte(&query_file.file, &params.range.end) else {
            return Ok(None);
        };
        let range = query_range(query_file.module, query_file.file_id, start, end);
        let request = query::QueryRequest::InlayHints(query::InlayHintsRequest { range });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let query::QueryResponse::InlayHints(response) = response.response else {
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
            .filter_map(|h| inlay_hint_to_lsp(&query_file.file, h))
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
        let Some(query_file) = self.query_file_for_uri(&params.text_document.uri) else {
            return Ok(None);
        };
        let Some(offset) = position_to_byte(&query_file.file, &params.position) else {
            return Ok(None);
        };
        let position = query_position(query_file.module, query_file.file_id, offset);
        let request = query::QueryRequest::RenameTarget(query::RenameTargetRequest { position });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let query::QueryResponse::RenameTarget(response) = response.response else {
            return Ok(None);
        };
        let Some(result) = response.result else {
            return Ok(None);
        };

        // convert to LSP
        let range = byte_span_to_range(&query_file.file, result.range);
        Ok(Some(lsp::PrepareRenameResponse::RangeWithPlaceholder {
            range,
            placeholder: result.placeholder,
        }))
    }

    async fn rename(
        &self,
        params: lsp::RenameParams,
    ) -> jsonrpc::Result<Option<lsp::WorkspaceEdit>> {
        let repository = self.repository();
        let Some(query_path) = params
            .text_document_position
            .text_document
            .uri
            .to_file_path()
            .map(|path| path.into_owned())
        else {
            return Ok(None);
        };
        let Some(query_file) =
            self.query_file_for_uri(&params.text_document_position.text_document.uri)
        else {
            return Ok(None);
        };
        let Some(offset) =
            position_to_byte(&query_file.file, &params.text_document_position.position)
        else {
            return Ok(None);
        };
        let position = query_position(query_file.module, query_file.file_id, offset);
        let request = query::QueryRequest::Rename(query::RenameRequest {
            position,
            new_name: params.new_name.clone(),
        });
        let Some(response) = self.execute_query(
            &query_path,
            request,
            QueryRevision::Current(query_file.revision),
        ) else {
            return Ok(None);
        };
        let revision = response.revision;
        let query::QueryResponse::Rename(response) = response.response else {
            return Ok(None);
        };
        let Some(edit) = response.edit else {
            return Ok(None);
        };

        // convert to LSP
        let workspace_edit = batch_edit_to_workspace_edit(repository, revision, &edit);
        Ok(Some(workspace_edit))
    }

    // ------------------------------------------------------------------------
    // CALL HIERARCHY
    // ------------------------------------------------------------------------

    async fn prepare_call_hierarchy(
        &self,
        params: lsp::CallHierarchyPrepareParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::CallHierarchyItem>>> {
        let repository = self.repository();
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.query_file_for_uri(uri) else {
            return Ok(None);
        };
        let Some(offset) = position_to_byte(
            &query_file.file,
            &params.text_document_position_params.position,
        ) else {
            return Ok(None);
        };
        let position = query_position(query_file.module, query_file.file_id, offset);
        let request =
            query::QueryRequest::CallHierarchyItem(query::CallHierarchyItemRequest { position });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let revision = response.revision;
        let query::QueryResponse::CallHierarchyItem(response) = response.response else {
            return Ok(None);
        };
        let Some(item) = response.item else {
            return Ok(None);
        };

        // convert to LSP
        let Some(lsp_item) = call_hierarchy_item_to_lsp(repository, revision, &item) else {
            return Ok(None);
        };

        Ok(Some(vec![lsp_item]))
    }

    async fn incoming_calls(
        &self,
        params: lsp::CallHierarchyIncomingCallsParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::CallHierarchyIncomingCall>>> {
        // extract query item from lsp data
        let repository = self.repository();
        let Some((revision, item)) = call_hierarchy_query_item_from_lsp(&params.item) else {
            return Ok(Some(vec![]));
        };
        let Some(path) = params.item.uri.to_file_path().map(|path| path.into_owned()) else {
            return Ok(Some(vec![]));
        };

        // query incoming calls
        let request =
            query::QueryRequest::CallHierarchyIncoming(query::CallHierarchyIncomingRequest {
                item,
            });
        let Some(response) = self.execute_query(&path, request, QueryRevision::Exact(revision))
        else {
            return Ok(Some(vec![]));
        };
        let revision = response.revision;
        let query::QueryResponse::CallHierarchyIncoming(response) = response.response else {
            return Ok(Some(vec![]));
        };
        let calls = response.calls;

        // convert to LSP
        let lsp_calls: Vec<lsp::CallHierarchyIncomingCall> = calls
            .iter()
            .filter_map(|c| incoming_call_to_lsp(repository, revision, c))
            .collect();

        Ok(Some(lsp_calls))
    }

    async fn outgoing_calls(
        &self,
        params: lsp::CallHierarchyOutgoingCallsParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::CallHierarchyOutgoingCall>>> {
        // extract query item from lsp data
        let repository = self.repository();
        let Some((revision, item)) = call_hierarchy_query_item_from_lsp(&params.item) else {
            return Ok(Some(vec![]));
        };
        let Some(path) = params.item.uri.to_file_path().map(|path| path.into_owned()) else {
            return Ok(Some(vec![]));
        };

        // query outgoing calls
        let request =
            query::QueryRequest::CallHierarchyOutgoing(query::CallHierarchyOutgoingRequest {
                item,
            });
        let Some(response) = self.execute_query(&path, request, QueryRevision::Exact(revision))
        else {
            return Ok(Some(vec![]));
        };
        let revision = response.revision;
        let query::QueryResponse::CallHierarchyOutgoing(response) = response.response else {
            return Ok(Some(vec![]));
        };
        let calls = response.calls;

        // convert to LSP
        let lsp_calls: Vec<lsp::CallHierarchyOutgoingCall> = calls
            .iter()
            .filter_map(|c| outgoing_call_to_lsp(repository, revision, c))
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
        let repository = self.repository();
        let uri = &params.text_document_position_params.text_document.uri;
        let Some(query_file) = self.query_file_for_uri(uri) else {
            return Ok(None);
        };
        let Some(offset) = position_to_byte(
            &query_file.file,
            &params.text_document_position_params.position,
        ) else {
            return Ok(None);
        };
        let position = query_position(query_file.module, query_file.file_id, offset);
        let request =
            query::QueryRequest::TypeHierarchyItem(query::TypeHierarchyItemRequest { position });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let revision = response.revision;
        let query::QueryResponse::TypeHierarchyItem(response) = response.response else {
            return Ok(None);
        };
        let Some(item) = response.item else {
            return Ok(None);
        };

        let Some(lsp_item) = type_hierarchy_item_to_lsp(repository, revision, &item) else {
            return Ok(None);
        };

        Ok(Some(vec![lsp_item]))
    }

    async fn supertypes(
        &self,
        params: lsp::TypeHierarchySupertypesParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TypeHierarchyItem>>> {
        // extract query item from lsp data
        let repository = self.repository();
        let Some((revision, item)) = type_hierarchy_query_item_from_lsp(&params.item) else {
            return Ok(Some(vec![]));
        };
        let Some(path) = params.item.uri.to_file_path().map(|path| path.into_owned()) else {
            return Ok(Some(vec![]));
        };

        // query supertypes
        let request =
            query::QueryRequest::TypeHierarchySupertypes(query::TypeHierarchySupertypesRequest {
                item,
            });
        let Some(response) = self.execute_query(&path, request, QueryRevision::Exact(revision))
        else {
            return Ok(Some(vec![]));
        };
        let revision = response.revision;
        let query::QueryResponse::TypeHierarchySupertypes(response) = response.response else {
            return Ok(Some(vec![]));
        };
        let supertypes = response.items;

        // convert to LSP
        let lsp_items: Vec<lsp::TypeHierarchyItem> = supertypes
            .iter()
            .filter_map(|t| type_hierarchy_item_to_lsp(repository, revision, t))
            .collect();

        Ok(Some(lsp_items))
    }

    async fn subtypes(
        &self,
        params: lsp::TypeHierarchySubtypesParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TypeHierarchyItem>>> {
        // extract query item from lsp data
        let repository = self.repository();
        let Some((revision, item)) = type_hierarchy_query_item_from_lsp(&params.item) else {
            return Ok(Some(vec![]));
        };
        let Some(path) = params.item.uri.to_file_path().map(|path| path.into_owned()) else {
            return Ok(Some(vec![]));
        };

        // query subtypes
        let request =
            query::QueryRequest::TypeHierarchySubtypes(query::TypeHierarchySubtypesRequest {
                item,
            });
        let Some(response) = self.execute_query(&path, request, QueryRevision::Exact(revision))
        else {
            return Ok(Some(vec![]));
        };
        let revision = response.revision;
        let query::QueryResponse::TypeHierarchySubtypes(response) = response.response else {
            return Ok(Some(vec![]));
        };
        let subtypes = response.items;

        // convert to LSP
        let lsp_items: Vec<lsp::TypeHierarchyItem> = subtypes
            .iter()
            .filter_map(|t| type_hierarchy_item_to_lsp(repository, revision, t))
            .collect();

        Ok(Some(lsp_items))
    }
}

#[cfg(test)]
mod tests {
    use super::DestackLanguageServer;
    use destack_lsp_types as lsp;
    use destack_query as query;

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
