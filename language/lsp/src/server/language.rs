use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, RwLock};

use dashmap::DashSet;
#[cfg(test)]
use destack_artifact::MemoryBlobStore;
use destack_lsp_server::{Client, LanguageServer, UriExt, jsonrpc};
use destack_lsp_types as lsp;
use destack_query as query;
use destack_repository::{
    DestackLayoutOverride, Environment, Revision, Settings, open_repository_from_fs,
};
use destack_source::{FileSystem, OverlayFileSystem, PatchSet, PhysicalFileSystem, ProfileId, Uri};
use destack_workspace::protocol::{DiagnosticSnapshot, QueryResponseBody};
use destack_workspace::{FileOperation, LocalWorkspace, Workspace};
use serde::{Deserialize, Serialize};
use serde_json::{from_value, to_value};
use tokio::sync::Notify;

use crate::server::assist::{code_lens_to_lsp, inlay_hint_to_lsp};
use crate::server::completion::completion_kind_to_lsp;
use crate::server::diagnostic::{
    code_action_to_lsp, diagnostic_file_snapshot, diagnostic_path_snapshots, diagnostic_result_id,
    diagnostic_root_snapshots, diagnostic_snapshots, diagnostic_to_lsp_diagnostic,
    reload_diagnostic_snapshots,
};
use crate::server::edit::patch_set_to_workspace_edit;
use crate::server::format::{file_edit, range_edit};
use crate::server::navigation::{
    call_item_from_lsp, call_item_to_lsp, document_highlight_to_lsp, document_link_to_lsp,
    document_symbol_to_lsp, incoming_call_to_lsp, navigation_target_to_location,
    navigation_targets_to_locations, outgoing_call_to_lsp, selection_range_to_lsp,
    type_item_from_lsp, type_item_to_lsp, workspace_symbol_to_lsp,
};
use crate::server::position::{byte_span_to_range, position_to_byte, span_to_location};
use crate::server::progress::WorkDoneProgressTracker;
use crate::server::query::{FileMap, QueryFile};
use crate::server::source::{
    file_from_image, file_watchers, normalize_line_endings, text_changes_from_lsp,
    tracked_file_globs,
};
use crate::server::{token, view};
use crate::uri::{lsp_uri_for_file, lsp_uri_for_source_uri};

const PARTIAL_RESULT_CHUNK_SIZE: usize = 128;
const WORKSPACE_DIAGNOSTIC_PARTIAL_CHUNK_SIZE: usize = 128;
const AUTO_IMPORT_DETAIL_PREFIX: &str = "Auto import from ";

/// Canonicalize a root path for stable dedupe comparisons.
fn canonical_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Return true when a root list already contains the path after canonicalization.
fn contains_root(roots: &[PathBuf], path: &Path) -> bool {
    let path = canonical_path(path);
    roots.iter().any(|root| canonical_path(root) == path)
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
    /// The file path that anchored the action request.
    path: PathBuf,
    /// The workspace edits for the selected code action.
    patches: PatchSet,
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

/// The Destack language server.
#[derive(Debug)]
pub struct DestackLanguageServer {
    /// The client connection.
    pub(super) client: Client,
    /// Workspace used for semantic state.
    workspace: OnceLock<Arc<dyn Workspace>>,
    /// Notification used to wake requests waiting on progress cancellation.
    progress_cancel_notify: Arc<Notify>,
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
            workspace: OnceLock::new(),
            progress_cancel_notify: Arc::new(Notify::new()),
            cancelled_progress_tokens: DashSet::new(),
            watch_registration_id: OnceLock::new(),
            completion_label_details_supported: OnceLock::new(),
            code_action_data_supported: OnceLock::new(),
            code_action_edit_resolve_supported: OnceLock::new(),
            settings: LspSettings::default(),
        }
    }

    /// Return the workspace after initialization.
    #[inline]
    fn workspace(&self) -> jsonrpc::Result<&Arc<dyn Workspace>> {
        self.workspace
            .get()
            .ok_or_else(jsonrpc::Error::internal_error)
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
            Err(_) => return,
        };
        if values.is_empty() {
            return;
        }

        // update completion settings when parsing succeeds
        if let Some(value) = values.first().cloned()
            && let Ok(parsed) = from_value::<CompletionSettings>(value)
            && let Some(auto_imports) = parsed.auto_imports
        {
            self.settings
                .completion_auto_imports
                .store(auto_imports, Ordering::Relaxed);
        }

        // update inlay hint settings when parsing succeeds
        if let Some(value) = values.get(1).cloned()
            && let Ok(parsed) = from_value::<InlayHintSettings>(value)
        {
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

        // update query settings when parsing succeeds
        if let Some(value) = values.get(2).cloned()
            && let Ok(parsed) = from_value::<QuerySettings>(value)
        {
            self.set_query_target(parsed.target);
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

    /// Execute one query through the workspace.
    fn execute_query(
        &self,
        path: &Path,
        request: query::QueryRequest,
        revision: Revision,
    ) -> Option<QueryResponseBody> {
        let workspace = self.workspace().ok()?;
        let result = super::query::execute(workspace.as_ref(), path, request, revision);

        result.ok()
    }

    /// Return the selected query profile for one root path.
    fn query_profile(&self, path: &Path) -> Option<ProfileId> {
        let target_name = self.query_target()?;
        let workspace = self.workspace().ok()?;
        let snapshot = view::root_snapshot(workspace.as_ref(), path, Some(target_name)).ok()?;

        snapshot.profile_ids.first().copied()
    }

    /// Return the exact file state for one LSP query uri.
    fn query_file_for_uri(&self, uri: &lsp::Uri) -> Option<QueryFile> {
        let workspace = self.workspace().ok()?;

        super::query::file(workspace.as_ref(), uri, self.query_target())
    }

    /// Execute one query against an exact file revision.
    fn query_file(
        &self,
        file: &QueryFile,
        request: query::QueryRequest,
    ) -> Option<QueryResponseBody> {
        self.execute_query(&file.path, request, file.revision)
    }

    /// Return a request-local file map.
    fn file_map(&self, path: &Path, revision: Revision) -> jsonrpc::Result<FileMap> {
        let workspace = Arc::clone(self.workspace()?);
        let root = workspace
            .root(path)
            .map_err(|_| jsonrpc::Error::internal_error())?;

        Ok(FileMap::new(workspace, root, revision))
    }

    /// Resolve the current semantic revision for the root that owns a path.
    fn revision_at(&self, path: &Path) -> Option<Revision> {
        let workspace = self.workspace().ok()?;
        view::root_snapshot(workspace.as_ref(), path, self.query_target())
            .ok()
            .map(|snapshot| snapshot.revision)
    }

    /// Resolve one local file path from an LSP uri.
    fn path_from_uri(uri: &lsp::Uri) -> Option<PathBuf> {
        uri.to_file_path().map(|path| path.into_owned())
    }

    /// Return true when the uri or path belongs to an open editor document.
    fn is_open_document_uri_or_path(&self, uri: &lsp::Uri, path: &Path) -> bool {
        let _ = uri;
        self.workspace()
            .ok()
            .and_then(|workspace| workspace.is_file_open(path).ok())
            .unwrap_or(false)
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

        let watchers = file_watchers();
        let options = lsp::DidChangeWatchedFilesRegistrationOptions { watchers };
        let register_options = match to_value(options) {
            Ok(value) => Some(value),
            Err(_) => return,
        };
        let registration = lsp::Registration {
            id: "destack.watch".to_string(),
            method: "workspace/didChangeWatchedFiles".to_string(),
            register_options,
        };

        if let Ok(()) = self.client.register_capability(vec![registration]).await {
            let _ = self.watch_registration_id.set("destack.watch".to_string());
        }
    }

    /// Publish workspace diagnostic snapshots.
    async fn publish_diagnostic_snapshots(&self, snapshots: Vec<DiagnosticSnapshot>) {
        let mut files_by_id = snapshots
            .iter()
            .filter_map(|snapshot| {
                let file = file_from_image(&snapshot.file)?;

                Some((file.id, file))
            })
            .collect::<HashMap<_, _>>();

        // prefetch cross-file diagnostic labels
        for snapshot in &snapshots {
            let mut missing_file_ids = HashSet::new();
            for diagnostic in &snapshot.diagnostics {
                for label in std::iter::once(&diagnostic.primary).chain(diagnostic.labels.iter()) {
                    if !files_by_id.contains_key(&label.target.file()) {
                        missing_file_ids.insert(label.target.file());
                    }
                }
            }
            if missing_file_ids.is_empty() {
                continue;
            }

            let Some(path) = snapshot.file.path.as_ref() else {
                continue;
            };
            let Ok(workspace) = self.workspace() else {
                continue;
            };
            let Ok(images) = view::file_images(
                workspace.as_ref(),
                path,
                snapshot.revision,
                missing_file_ids.into_iter().collect(),
            ) else {
                continue;
            };
            for image in images {
                let Some(file) = file_from_image(&image) else {
                    continue;
                };
                files_by_id.insert(file.id, file);
            }
        }

        for snapshot in snapshots {
            let file = file_from_image(&snapshot.file);
            let uri = lsp_uri_for_source_uri(&snapshot.diagnostic_uri)
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
            let diagnostics = snapshot
                .diagnostics
                .into_iter()
                .filter_map(|diagnostic| diagnostic_to_lsp_diagnostic(&diagnostic, &file_for_id))
                .collect();

            self.client
                .publish_diagnostics(uri, diagnostics, snapshot.diagnostic_version)
                .await;
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
        let _ = label;
        let value = match to_value(payload) {
            Ok(value) => value,
            Err(_) => return,
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
        .map_err(|_| jsonrpc::Error::internal_error())?;
        #[cfg(test)]
        let repository = repository.with_blob_store(Arc::new(MemoryBlobStore::new()));
        let root = repository.path().to_path_buf();

        // merge discovery root with initialize roots
        let mut opened_roots = vec![root.clone()];
        for initialize_root in initialize_roots {
            if !contains_root(&opened_roots, &initialize_root) {
                opened_roots.push(initialize_root);
            }
        }

        let repository = Arc::new(repository);
        // open the semantic workspace for the editor session
        let workspace: Arc<dyn Workspace> = match LocalWorkspace::new(
            repository.clone(),
            Some(overlay_fs),
            None,
            opened_roots,
            LocalWorkspace::default_worker_count(),
            None,
        ) {
            Ok(workspace) => Arc::new(workspace),
            Err(_) => return Err(jsonrpc::Error::internal_error()),
        };
        if self.workspace.set(workspace).is_err() {
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
                        legend: token::legend(),
                        range: Some(true),
                        full: Some(lsp::SemanticTokensFullOptions::Bool(true)),
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
                commands: vec!["destack.reload".to_string(), "destack.reindex".to_string()],
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
        self.register_file_watchers().await;
        self.refresh_configuration().await;
    }

    async fn did_change_configuration(&self, _: lsp::DidChangeConfigurationParams) {
        self.refresh_configuration().await;

        // reload filesystem state so config changes refresh diagnostics
        let Ok(workspace) = self.workspace() else {
            return;
        };
        let result = reload_diagnostic_snapshots(workspace.as_ref());
        let result = match result {
            Ok(result) => result,
            Err(_) => return,
        };

        self.publish_diagnostic_snapshots(result).await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    // ------------------------------------------------------------------------
    // SYNCHRONIZATION
    // ------------------------------------------------------------------------

    async fn did_open(&self, params: lsp::DidOpenTextDocumentParams) {
        let content = normalize_line_endings(params.text_document.text);
        let version = params.text_document.version;

        let Some(path) = Self::path_from_uri(&params.text_document.uri) else {
            return;
        };

        let Ok(workspace) = self.workspace() else {
            return;
        };
        let result = workspace.file(FileOperation::OpenText {
            path: path.clone(),
            uri: Uri::from_file_path(&path),
            version,
            content,
        });
        if result.is_err() {
            return;
        }

        let Ok(result) = diagnostic_path_snapshots(workspace.as_ref(), &path) else {
            return;
        };

        self.publish_diagnostic_snapshots(result).await;
    }

    async fn did_change(&self, params: lsp::DidChangeTextDocumentParams) {
        // ignore empty change batches
        if params.content_changes.is_empty() {
            return;
        };
        let Some(path) = Self::path_from_uri(&params.text_document.uri) else {
            return;
        };
        let version = params.text_document.version;

        let Ok(workspace) = self.workspace() else {
            return;
        };
        let changes = text_changes_from_lsp(params.content_changes);
        let result = workspace.file(FileOperation::PatchText {
            path: path.clone(),
            uri: Uri::from_file_path(&path),
            version,
            changes,
        });
        if result.is_err() {
            return;
        }

        let Ok(result) = diagnostic_path_snapshots(workspace.as_ref(), &path) else {
            return;
        };

        self.publish_diagnostic_snapshots(result).await;
    }

    async fn did_save(&self, params: lsp::DidSaveTextDocumentParams) {
        let Some(path) = Self::path_from_uri(&params.text_document.uri) else {
            return;
        };

        let content = params.text.map(normalize_line_endings);

        let Ok(workspace) = self.workspace() else {
            return;
        };
        let result = workspace.file(FileOperation::SaveText {
            path: path.clone(),
            content,
        });
        if result.is_err() {
            return;
        }

        let Ok(result) = diagnostic_path_snapshots(workspace.as_ref(), &path) else {
            return;
        };

        self.publish_diagnostic_snapshots(result).await;
    }

    async fn did_close(&self, params: lsp::DidCloseTextDocumentParams) {
        let Some(path) = Self::path_from_uri(&params.text_document.uri) else {
            return;
        };

        let Ok(workspace) = self.workspace() else {
            return;
        };
        let root = workspace.root(&path);
        let result = workspace.file(FileOperation::Close { path: path.clone() });
        if result.is_err() {
            return;
        }

        let Ok(root) = root else {
            return;
        };
        let Ok(result) = diagnostic_root_snapshots(workspace.as_ref(), &root) else {
            return;
        };

        self.publish_diagnostic_snapshots(result).await;
    }

    async fn did_change_workspace_folders(&self, params: lsp::DidChangeWorkspaceFoldersParams) {
        let Ok(workspace) = self.workspace() else {
            return;
        };

        for folder in params.event.added {
            let Some(path) = folder.uri.to_file_path().map(|path| path.into_owned()) else {
                continue;
            };
            let _ = workspace.open(path);
        }

        for folder in params.event.removed {
            let Some(path) = folder.uri.to_file_path().map(|path| path.into_owned()) else {
                continue;
            };
            let _ = workspace.close(&path);
        }
    }

    async fn did_change_watched_files(&self, params: lsp::DidChangeWatchedFilesParams) {
        if params.changes.is_empty() {
            return;
        }

        let Ok(workspace) = self.workspace() else {
            return;
        };
        let Ok(result) = reload_diagnostic_snapshots(workspace.as_ref()) else {
            return;
        };

        self.publish_diagnostic_snapshots(result).await;
    }

    async fn will_rename_files(
        &self,
        params: lsp::RenameFilesParams,
    ) -> jsonrpc::Result<Option<lsp::WorkspaceEdit>> {
        // collect rename targets from uris
        let mut renames = Vec::new();
        let mut query_root: Option<PathBuf> = None;
        let mut expected_revision: Option<Revision> = None;
        let workspace = self.workspace()?;
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
            let old_root = workspace.root(&old_path).ok();
            let new_root = workspace.root(&new_path).ok();
            let root = old_root.or(new_root);
            let Some(root) = root else {
                continue;
            };

            if let Some(existing_root) = query_root.as_ref() {
                if existing_root != &root {
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
        let Some(profile_id) = self.query_profile(&query_root) else {
            return Ok(None);
        };
        let request = query::QueryRequest::RenameFiles(query::RenameFilesRequest {
            profile_ids: vec![profile_id],
            renames,
        });
        let response = self.execute_query(&query_root, request, expected_revision);
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

        let mut files = self.file_map(&query_root, revision)?;
        let workspace_edit = patch_set_to_workspace_edit(&edit, &mut |file_id| files.file(file_id));
        Ok(Some(workspace_edit))
    }

    async fn will_delete_files(
        &self,
        params: lsp::DeleteFilesParams,
    ) -> jsonrpc::Result<Option<lsp::WorkspaceEdit>> {
        let _ = params;
        Ok(None)
    }

    async fn did_create_files(&self, _params: lsp::CreateFilesParams) {
        // workspace-owned watchers observe filesystem changes
    }

    async fn did_rename_files(&self, params: lsp::RenameFilesParams) {
        // clear diagnostics for renamed files
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

            // clear diagnostics for the old uri
            self.client
                .publish_diagnostics(old_uri.clone(), Vec::new(), None)
                .await;
        }
    }

    async fn did_delete_files(&self, params: lsp::DeleteFilesParams) {
        // clear diagnostics for deleted files
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

                self.client
                    .publish_diagnostics(uri.clone(), Vec::new(), None)
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

        let path = path.clone();
        let snapshot = match diagnostic_file_snapshot(self.workspace()?.as_ref(), &path) {
            Ok(snapshot) => snapshot,
            Err(_) => return Err(jsonrpc::Error::internal_error()),
        };
        let Some(snapshot) = snapshot else {
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
        let Some(file) = file_from_image(&snapshot.file) else {
            return Err(jsonrpc::Error::internal_error());
        };
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

        let snapshots = match diagnostic_snapshots(self.workspace()?.as_ref()) {
            Ok(snapshots) => snapshots,
            Err(_) => return Err(jsonrpc::Error::internal_error()),
        };

        // collect partial results when supported
        let partial_token = params.partial_result_params.partial_result_token;

        let mut items = Vec::new();
        let mut partial_items = Vec::new();
        let files_by_id = snapshots
            .iter()
            .filter_map(|snapshot| {
                let file = file_from_image(&snapshot.file)?;

                Some((file.id, file))
            })
            .collect::<HashMap<_, _>>();

        // allow cancellation between chunks
        let mut processed = 0usize;
        for snapshot in snapshots {
            let Some(file) = file_from_image(&snapshot.file) else {
                continue;
            };
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
                let result = reload_diagnostic_snapshots(self.workspace()?.as_ref());
                let result = match result {
                    Ok(result) => result,
                    Err(_) => return Ok(None),
                };

                self.publish_diagnostic_snapshots(result).await;

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
        let position = super::query::position(query_file.module, query_file.file_id, offset);
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
        let mut files = self.file_map(&query_file.path, revision)?;
        let location = navigation_target_to_location(target, &mut |file_id| files.file(file_id));
        Ok(location.map(lsp::GotoDefinitionResponse::Scalar))
    }

    async fn goto_declaration(
        &self,
        params: lsp::request::GotoDeclarationParams,
    ) -> jsonrpc::Result<Option<lsp::request::GotoDeclarationResponse>> {
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
        let position = super::query::position(query_file.module, query_file.file_id, offset);
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

        let mut files = self.file_map(&query_file.path, revision)?;
        let location = navigation_target_to_location(target, &mut |file_id| files.file(file_id));
        Ok(location.map(lsp::GotoDefinitionResponse::Scalar))
    }

    async fn goto_type_definition(
        &self,
        params: lsp::request::GotoTypeDefinitionParams,
    ) -> jsonrpc::Result<Option<lsp::request::GotoTypeDefinitionResponse>> {
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
        let position = super::query::position(query_file.module, query_file.file_id, offset);
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

        let mut files = self.file_map(&query_file.path, revision)?;
        let location = navigation_target_to_location(target, &mut |file_id| files.file(file_id));
        Ok(location.map(lsp::GotoDefinitionResponse::Scalar))
    }

    async fn references(
        &self,
        params: lsp::ReferenceParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::Location>>> {
        let uri = &params.text_document_position.text_document.uri;
        let Some(query_file) = self.query_file_for_uri(uri) else {
            return Ok(None);
        };
        let Some(offset) =
            position_to_byte(&query_file.file, &params.text_document_position.position)
        else {
            return Ok(None);
        };
        let position = super::query::position(query_file.module, query_file.file_id, offset);
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
        let mut files = self.file_map(&query_file.path, revision)?;
        for reference in references.iter() {
            let Some(location) =
                span_to_location(reference.target.span, &mut |file_id| files.file(file_id))
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
        let request = query::QueryRequest::Outline(query::OutlineRequest {
            module: query_file.module,
        });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let query::QueryResponse::Outline(response) = response.response else {
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
                        "symbol",
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
            self.publish_partial_result(token, std::mem::take(&mut partial_symbols), "symbol")
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
        // query symbol search
        let Some(root) = self.workspace()?.roots().into_iter().next() else {
            return Ok(None);
        };
        let Some(revision) = self.revision_at(&root) else {
            return Ok(None);
        };
        let Some(profile_id) = self.query_profile(&root) else {
            return Ok(None);
        };
        let request = query::QueryRequest::SymbolSearch(query::SymbolSearchRequest {
            query: params.query.clone(),
            profile_ids: vec![profile_id],
            max_results: 100,
        });
        let Some(response) = self.execute_query(&root, request, revision) else {
            return Ok(None);
        };
        let revision = response.revision;
        let query::QueryResponse::SymbolSearch(response) = response.response else {
            return Ok(None);
        };
        let symbols = response.symbols;

        let mut progress = WorkDoneProgressTracker::start(
            self,
            params.work_done_progress_params.work_done_token,
            "Symbol search",
            "building symbols",
        )
        .await;

        // convert to LSP
        let partial_token = params.partial_result_params.partial_result_token;
        let mut lsp_symbols = Vec::new();
        let mut partial_symbols = Vec::new();
        let mut processed = 0usize;
        let mut files = self.file_map(&root, revision)?;
        for symbol in symbols.iter() {
            let Some(lsp_symbol) =
                workspace_symbol_to_lsp(symbol, &mut |file_id| files.file(file_id))
            else {
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
        let position = super::query::position(query_file.module, query_file.file_id, offset);
        let request = query::QueryRequest::Highlight(query::HighlightRequest { position });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let query::QueryResponse::Highlight(response) = response.response else {
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
                        "highlight",
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
                "highlight",
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
        let position = super::query::position(query_file.module, query_file.file_id, offset);
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
        let position = super::query::position(query_file.module, query_file.file_id, offset);
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
        let position = super::query::position(query_file.module, query_file.file_id, offset);
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
        let lsp_tokens = token::tokens_to_lsp(&query_file.file, &tokens);

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
        let Some(query_file) = self.query_file_for_uri(&params.text_document.uri) else {
            return Ok(None);
        };
        let Some(start) = position_to_byte(&query_file.file, &params.range.start) else {
            return Ok(None);
        };
        let Some(end) = position_to_byte(&query_file.file, &params.range.end) else {
            return Ok(None);
        };
        let range = super::query::range(query_file.module, query_file.file_id, start, end);
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
        let lsp_tokens = token::tokens_to_lsp(&query_file.file, &tokens);

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
        if !self
            .workspace()?
            .is_file_open(&path)
            .map_err(|_| jsonrpc::Error::internal_error())?
        {
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
        let Some(query_file) = self.query_file_for_uri(&params.text_document.uri) else {
            return Ok(None);
        };
        let edit = file_edit(&query_file.file, query_file.formatter)
            .map_err(|_| jsonrpc::Error::internal_error())?;
        let Some(edit) = edit else {
            return Ok(None);
        };

        Ok(Some(vec![edit]))
    }

    async fn range_formatting(
        &self,
        params: lsp::DocumentRangeFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TextEdit>>> {
        let Some(query_file) = self.query_file_for_uri(&params.text_document.uri) else {
            return Ok(None);
        };
        let Some(start_offset) = position_to_byte(&query_file.file, &params.range.start) else {
            return Ok(None);
        };
        let Some(end_offset) = position_to_byte(&query_file.file, &params.range.end) else {
            return Ok(None);
        };
        let edit = range_edit(
            &query_file.file,
            query_file.formatter,
            start_offset,
            end_offset,
        )
        .map_err(|_| jsonrpc::Error::internal_error())?;
        let Some(edit) = edit else {
            return Ok(None);
        };

        Ok(Some(vec![edit]))
    }

    async fn on_type_formatting(
        &self,
        params: lsp::DocumentOnTypeFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TextEdit>>> {
        let Some(query_file) =
            self.query_file_for_uri(&params.text_document_position.text_document.uri)
        else {
            return Ok(None);
        };
        let Some(end_offset) =
            position_to_byte(&query_file.file, &params.text_document_position.position)
        else {
            return Ok(None);
        };
        let trigger_width = params.ch.len() as u32;
        let start_offset = end_offset.saturating_sub(trigger_width.max(1));
        let edit = range_edit(
            &query_file.file,
            query_file.formatter,
            start_offset,
            end_offset,
        )
        .map_err(|_| jsonrpc::Error::internal_error())?;
        let Some(edit) = edit else {
            return Ok(None);
        };

        Ok(Some(vec![edit]))
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
        let position = super::query::position(query_file.module, query_file.file_id, offset);
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
        let mut files = self.file_map(&query_file.path, revision)?;
        let locations =
            navigation_targets_to_locations(&response.targets, &mut |file_id| files.file(file_id));
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
        let request = query::QueryRequest::Links(query::LinksRequest {
            module: query_file.module,
        });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let query::QueryResponse::Links(response) = response.response else {
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
                    self.publish_partial_result(token, std::mem::take(&mut partial_links), "link")
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
            self.publish_partial_result(token, std::mem::take(&mut partial_links), "link")
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
        // return eager links unchanged for clients that call despite capabilities
        Ok(params)
    }

    // ------------------------------------------------------------------------
    // CODE ACTIONS
    // ------------------------------------------------------------------------

    async fn code_action(
        &self,
        params: lsp::CodeActionParams,
    ) -> jsonrpc::Result<Option<lsp::CodeActionResponse>> {
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
        let range = super::query::range(query_file.module, query_file.file_id, start, end);
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

        // filter diagnostic linked quick fixes by requested diagnostic ids
        let diagnostic_ids: HashSet<String> = params
            .context
            .diagnostics
            .iter()
            .filter_map(|diagnostic| match diagnostic.code.as_ref() {
                Some(lsp::NumberOrString::String(value)) => Some(value.clone()),
                Some(lsp::NumberOrString::Number(value)) => Some(value.to_string()),
                _ => None,
            })
            .collect();
        if !diagnostic_ids.is_empty() {
            actions.retain(|action| {
                action
                    .diagnostic_id
                    .as_ref()
                    .is_none_or(|id| diagnostic_ids.contains(id))
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
        let mut files = self.file_map(&query_file.path, revision)?;
        for action in actions.iter() {
            let data = if prefer_lazy_code_action_edits {
                to_value(CodeActionResolveData {
                    revision,
                    path: query_file.path.clone(),
                    patches: action.patches.clone(),
                })
                .ok()
            } else {
                None
            };
            let include_edit = !prefer_lazy_code_action_edits || data.is_none();
            let Some(lsp_action) = code_action_to_lsp(action, include_edit, data, &mut |file_id| {
                files.file(file_id)
            }) else {
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

        let mut files = self.file_map(&resolved.path, resolved.revision)?;
        params.edit = Some(patch_set_to_workspace_edit(
            &resolved.patches,
            &mut |file_id| files.file(file_id),
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
        let range = super::query::range(query_file.module, query_file.file_id, start, end);
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
            .map(|hint| inlay_hint_to_lsp(&query_file.file, hint))
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
        let position = super::query::position(query_file.module, query_file.file_id, offset);
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
        let position = super::query::position(query_file.module, query_file.file_id, offset);
        let request = query::QueryRequest::Rename(query::RenameRequest {
            position,
            new_name: params.new_name.clone(),
        });
        let Some(response) = self.execute_query(&query_path, request, query_file.revision) else {
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
        let mut files = self.file_map(&query_path, revision)?;
        let workspace_edit = patch_set_to_workspace_edit(&edit, &mut |file_id| files.file(file_id));
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
        let Some(query_file) = self.query_file_for_uri(uri) else {
            return Ok(None);
        };
        let Some(offset) = position_to_byte(
            &query_file.file,
            &params.text_document_position_params.position,
        ) else {
            return Ok(None);
        };
        let position = super::query::position(query_file.module, query_file.file_id, offset);
        let request = query::QueryRequest::CallItem(query::CallItemRequest { position });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let revision = response.revision;
        let query::QueryResponse::CallItem(response) = response.response else {
            return Ok(None);
        };
        let Some(item) = response.item else {
            return Ok(None);
        };

        // convert to LSP
        let mut files = self.file_map(&query_file.path, revision)?;
        let Some(lsp_item) = call_item_to_lsp(revision, &item, &mut |file_id| files.file(file_id))
        else {
            return Ok(None);
        };

        Ok(Some(vec![lsp_item]))
    }

    async fn incoming_calls(
        &self,
        params: lsp::CallHierarchyIncomingCallsParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::CallHierarchyIncomingCall>>> {
        // extract query item from lsp data
        let Some((revision, item)) = call_item_from_lsp(&params.item) else {
            return Ok(Some(vec![]));
        };
        let Some(path) = params.item.uri.to_file_path().map(|path| path.into_owned()) else {
            return Ok(Some(vec![]));
        };

        // query incoming calls
        let request = query::QueryRequest::IncomingCalls(query::IncomingCallsRequest { item });
        let Some(response) = self.execute_query(&path, request, revision) else {
            return Ok(Some(vec![]));
        };
        let revision = response.revision;
        let query::QueryResponse::IncomingCalls(response) = response.response else {
            return Ok(Some(vec![]));
        };
        let calls = response.calls;

        // convert to LSP
        let mut files = self.file_map(&path, revision)?;
        let lsp_calls: Vec<lsp::CallHierarchyIncomingCall> = calls
            .iter()
            .filter_map(|c| incoming_call_to_lsp(revision, c, &mut |file_id| files.file(file_id)))
            .collect();

        Ok(Some(lsp_calls))
    }

    async fn outgoing_calls(
        &self,
        params: lsp::CallHierarchyOutgoingCallsParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::CallHierarchyOutgoingCall>>> {
        // extract query item from lsp data
        let Some((revision, item)) = call_item_from_lsp(&params.item) else {
            return Ok(Some(vec![]));
        };
        let Some(path) = params.item.uri.to_file_path().map(|path| path.into_owned()) else {
            return Ok(Some(vec![]));
        };

        // query outgoing calls
        let request = query::QueryRequest::OutgoingCalls(query::OutgoingCallsRequest { item });
        let Some(response) = self.execute_query(&path, request, revision) else {
            return Ok(Some(vec![]));
        };
        let revision = response.revision;
        let query::QueryResponse::OutgoingCalls(response) = response.response else {
            return Ok(Some(vec![]));
        };
        let calls = response.calls;

        // convert to LSP
        let mut files = self.file_map(&path, revision)?;
        let lsp_calls: Vec<lsp::CallHierarchyOutgoingCall> = calls
            .iter()
            .filter_map(|c| outgoing_call_to_lsp(revision, c, &mut |file_id| files.file(file_id)))
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
        let position = super::query::position(query_file.module, query_file.file_id, offset);
        let request = query::QueryRequest::TypeItem(query::TypeItemRequest { position });
        let Some(response) = self.query_file(&query_file, request) else {
            return Ok(None);
        };
        let revision = response.revision;
        let query::QueryResponse::TypeItem(response) = response.response else {
            return Ok(None);
        };
        let Some(item) = response.item else {
            return Ok(None);
        };

        let mut files = self.file_map(&query_file.path, revision)?;
        let Some(lsp_item) = type_item_to_lsp(revision, &item, &mut |file_id| files.file(file_id))
        else {
            return Ok(None);
        };

        Ok(Some(vec![lsp_item]))
    }

    async fn supertypes(
        &self,
        params: lsp::TypeHierarchySupertypesParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TypeHierarchyItem>>> {
        // extract query item from lsp data
        let Some((revision, item)) = type_item_from_lsp(&params.item) else {
            return Ok(Some(vec![]));
        };
        let Some(path) = params.item.uri.to_file_path().map(|path| path.into_owned()) else {
            return Ok(Some(vec![]));
        };

        // query supertypes
        let request = query::QueryRequest::Supertypes(query::SupertypesRequest { item });
        let Some(response) = self.execute_query(&path, request, revision) else {
            return Ok(Some(vec![]));
        };
        let revision = response.revision;
        let query::QueryResponse::Supertypes(response) = response.response else {
            return Ok(Some(vec![]));
        };
        let supertypes = response.items;

        // convert to LSP
        let mut files = self.file_map(&path, revision)?;
        let lsp_items: Vec<lsp::TypeHierarchyItem> = supertypes
            .iter()
            .filter_map(|t| type_item_to_lsp(revision, t, &mut |file_id| files.file(file_id)))
            .collect();

        Ok(Some(lsp_items))
    }

    async fn subtypes(
        &self,
        params: lsp::TypeHierarchySubtypesParams,
    ) -> jsonrpc::Result<Option<Vec<lsp::TypeHierarchyItem>>> {
        // extract query item from lsp data
        let Some((revision, item)) = type_item_from_lsp(&params.item) else {
            return Ok(Some(vec![]));
        };
        let Some(path) = params.item.uri.to_file_path().map(|path| path.into_owned()) else {
            return Ok(Some(vec![]));
        };

        // query subtypes
        let request = query::QueryRequest::Subtypes(query::SubtypesRequest { item });
        let Some(response) = self.execute_query(&path, request, revision) else {
            return Ok(Some(vec![]));
        };
        let revision = response.revision;
        let query::QueryResponse::Subtypes(response) = response.response else {
            return Ok(Some(vec![]));
        };
        let subtypes = response.items;

        // convert to LSP
        let mut files = self.file_map(&path, revision)?;
        let lsp_items: Vec<lsp::TypeHierarchyItem> = subtypes
            .iter()
            .filter_map(|t| type_item_to_lsp(revision, t, &mut |file_id| files.file(file_id)))
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
