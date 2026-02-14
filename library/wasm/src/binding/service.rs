use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use {
    destack_service as workspace_service, destack_source as source, destack_workspace as workspace,
};

use super::error::{js_error, parse_optional_input, to_js_value};
use super::{CompilerOptions, Diagnostic, FileType};

/// Options for creating a LanguageService.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageServiceOptions {
    /// The current working directory.
    pub cwd: String,
    /// The workspace roots to open initially.
    pub roots: Vec<String>,
    /// Compiler options for analysis and updates.
    pub compiler: Option<CompilerOptions>,
}

impl Default for LanguageServiceOptions {
    fn default() -> Self {
        Self {
            cwd: std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string()),
            roots: Vec::new(),
            compiler: None,
        }
    }
}

/// File update kind for virtual updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceVirtualUpdateKind {
    /// Replace file content with text.
    Text,
    /// Replace file content with bytes.
    Bytes,
    /// Bump file version without changing content.
    Touch,
    /// Mark file as removed.
    Removed,
}

/// Virtual file update payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceVirtualUpdate {
    /// The update kind.
    pub kind: WorkspaceVirtualUpdateKind,
    /// Text payload for text updates.
    pub content: Option<String>,
    /// Byte payload for binary updates.
    pub bytes: Option<Vec<u8>>,
}

/// Watch event kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceWatchEventKind {
    /// A file was created.
    Created,
    /// A file was modified.
    Modified,
    /// A file was deleted.
    Deleted,
    /// A file was renamed.
    Renamed,
    /// Watcher reported overflow.
    Overflow,
}

/// File watch event payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceWatchEvent {
    /// The file path.
    pub path: String,
    /// The previous path for rename events.
    pub previous_path: Option<String>,
    /// The watch event kind.
    pub kind: WorkspaceWatchEventKind,
}

/// Rescan reason for workspace operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LanguageServiceRescanReason {
    /// Requested during startup synchronization.
    Startup,
    /// Requested after watcher overflow.
    Overflow,
    /// Requested manually.
    Manual,
    /// Requested after watch updates.
    Update,
}

impl From<LanguageServiceRescanReason> for workspace_service::RescanReason {
    fn from(reason: LanguageServiceRescanReason) -> Self {
        match reason {
            LanguageServiceRescanReason::Startup => workspace_service::RescanReason::Startup,
            LanguageServiceRescanReason::Overflow => workspace_service::RescanReason::Overflow,
            LanguageServiceRescanReason::Manual => workspace_service::RescanReason::Manual,
            LanguageServiceRescanReason::Update => workspace_service::RescanReason::Update,
        }
    }
}

/// Workspace message kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LanguageServiceMessageKind {
    /// Informational message.
    Info,
    /// Warning message.
    Warning,
    /// Error message.
    Error,
}

impl From<workspace_service::WorkspaceMessageKind> for LanguageServiceMessageKind {
    fn from(kind: workspace_service::WorkspaceMessageKind) -> Self {
        match kind {
            workspace_service::WorkspaceMessageKind::Info => LanguageServiceMessageKind::Info,
            workspace_service::WorkspaceMessageKind::Warning => LanguageServiceMessageKind::Warning,
            workspace_service::WorkspaceMessageKind::Error => LanguageServiceMessageKind::Error,
        }
    }
}

/// Module id payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceModuleId {
    /// The owning package id.
    pub package_id: String,
    /// Local identifier within the package.
    pub local_id: u32,
}

/// Invalidation kind payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceInvalidationKind {
    /// Source module content changed.
    ModuleSource,
    /// Package dsconfig changed.
    DsConfig,
    /// Tsconfig changed.
    TsConfig,
    /// Package manifest changed.
    PackageManifest,
    /// No known mapping for the file.
    Unknown,
}

/// Invalidation summary payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInvalidation {
    /// The file id that triggered invalidation.
    pub file_id: u32,
    /// Updated file version.
    pub file_version: String,
    /// Invalidation categories for the file.
    pub kinds: Vec<WorkspaceInvalidationKind>,
    /// Invalidated module ids.
    pub modules: Vec<WorkspaceModuleId>,
    /// Invalidated package ids.
    pub packages: Vec<String>,
    /// Profiles with bumped versions.
    pub profiles: Vec<u32>,
    /// Profile graphs dropped due to invalidation.
    pub graphs_dropped: Vec<u32>,
}

/// File snapshot payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceFileSnapshot {
    /// File id in the registry.
    pub id: u32,
    /// File name.
    pub name: String,
    /// File uri.
    pub uri: String,
    /// Optional file path.
    pub path: Option<String>,
    /// File type label.
    pub file_type: FileType,
    /// Optional text content.
    pub content: Option<String>,
}

/// Workspace update record payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceUpdateRecord {
    /// Updated module id when known.
    pub module_id: Option<WorkspaceModuleId>,
    /// Updated file id.
    pub file_id: u32,
    /// Updated file snapshot.
    pub file: WorkspaceFileSnapshot,
    /// Invalidation summary for this update.
    pub invalidation: WorkspaceInvalidation,
    /// Diagnostics produced by this update.
    pub diagnostics: Vec<Diagnostic>,
}

/// Workspace message payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceMessage {
    /// Message severity.
    pub kind: LanguageServiceMessageKind,
    /// Stable message code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

/// Result payload for workspace service updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageServiceResult {
    /// Update records produced by the operation.
    pub updates: Vec<WorkspaceUpdateRecord>,
    /// Message records produced by the operation.
    pub messages: Vec<WorkspaceMessage>,
}

/// Outcome payload for explicit analyze requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceAnalyzeOutcome {
    /// Whether semantic query state is ready for the analyzed module.
    pub semantic_query_ready: bool,
    /// Detail when semantic query state is not ready.
    pub detail: Option<String>,
}

/// WASM wrapper for the Rust workspace service.
#[wasm_bindgen]
#[derive(Debug)]
pub struct LanguageService {
    /// Session backing this service.
    session: Arc<workspace::Session>,
    /// Workspace service for analysis and updates.
    inner: workspace_service::LanguageService,
}

#[wasm_bindgen]
impl LanguageService {
    /// Create a new LanguageService.
    #[wasm_bindgen(constructor)]
    pub fn new(options: Option<JsValue>) -> Result<Self, JsValue> {
        let options: LanguageServiceOptions = parse_optional_input(options)?;
        let cwd = PathBuf::from(&options.cwd);
        let roots = roots_from_options(&options, &cwd);
        let compiler = options.compiler.unwrap_or_default();
        let session = Arc::new(workspace::Session::new(cwd));
        let inner = workspace_service::LanguageService::with_options(
            session.clone(),
            roots,
            compiler.into(),
        )
        .map_err(js_error)?;

        Ok(Self { session, inner })
    }

    /// Return the session cwd.
    #[wasm_bindgen(getter)]
    pub fn cwd(&self) -> String {
        self.session.cwd.to_string_lossy().to_string()
    }

    /// Return the number of active program handles.
    #[wasm_bindgen(getter, js_name = programHandleCount)]
    pub fn program_handle_count(&self) -> u32 {
        self.inner.program_handle_count() as u32
    }

    /// Open a workspace root.
    #[wasm_bindgen(js_name = openWorkspaceRoot)]
    pub fn open_workspace_root(&self, root: String) -> Result<(), JsValue> {
        self.inner
            .open_workspace_root(PathBuf::from(root))
            .map_err(js_error)
    }

    /// Close a workspace root.
    #[wasm_bindgen(js_name = closeWorkspaceRoot)]
    pub fn close_workspace_root(&self, root: String) -> Result<(), JsValue> {
        self.inner
            .close_workspace_root(&PathBuf::from(root))
            .map_err(js_error)
    }

    /// Return whether a workspace root is active.
    #[wasm_bindgen(js_name = hasWorkspaceRoot)]
    pub fn has_workspace_root(&self, root: String) -> bool {
        self.inner.has_workspace_root(&PathBuf::from(root))
    }

    /// Remove a workspace root.
    #[wasm_bindgen(js_name = removeWorkspaceRoot)]
    pub fn remove_workspace_root(&self, root: String) -> Result<bool, JsValue> {
        self.inner
            .remove_workspace_root(&PathBuf::from(root))
            .map_err(js_error)
    }

    /// Clear cache entries for all roots.
    #[wasm_bindgen(js_name = clearCacheAll)]
    pub fn clear_cache_all(&self) -> Result<(), JsValue> {
        self.inner.clear_cache_all().map_err(js_error)
    }

    /// Shutdown the service.
    pub fn shutdown(&self) {
        self.inner.shutdown();
    }

    /// Return the workspace handle for a root as a decimal string.
    #[wasm_bindgen(js_name = handleForRoot)]
    pub fn handle_for_root(&self, root: String) -> Result<String, JsValue> {
        self.inner
            .handle_for_root(&PathBuf::from(root))
            .map(|handle| handle.0.to_string())
            .map_err(js_error)
    }

    /// Apply a text update for a virtual file.
    #[wasm_bindgen(js_name = updateVirtualFile)]
    pub fn update_virtual_file(&self, path: String, content: String) -> Result<JsValue, JsValue> {
        self.inner
            .update_virtual_file(&PathBuf::from(path), content)
            .map(workspace_service_result_binding)
            .map_err(js_error)
            .and_then(|result| to_js_value(&result))
    }

    /// Apply a virtual file update.
    #[wasm_bindgen(js_name = applyVirtualUpdate)]
    pub fn apply_virtual_update(&self, path: String, update: JsValue) -> Result<JsValue, JsValue> {
        let update = serde_wasm_bindgen::from_value::<WorkspaceVirtualUpdate>(update)
            .map_err(js_error)
            .and_then(file_update_from_virtual_update)?;

        self.inner
            .apply_virtual_update(&PathBuf::from(path), update)
            .map(workspace_service_result_binding)
            .map_err(js_error)
            .and_then(|result| to_js_value(&result))
    }

    /// Apply watch events.
    #[wasm_bindgen(js_name = applyWatchEvents)]
    pub fn apply_watch_events(&self, events: JsValue) -> Result<JsValue, JsValue> {
        let events = serde_wasm_bindgen::from_value::<Vec<WorkspaceWatchEvent>>(events)
            .map_err(js_error)?
            .into_iter()
            .map(file_watch_event_from_binding)
            .collect();

        self.inner
            .apply_watch_events(events)
            .map(workspace_service_result_binding)
            .map_err(js_error)
            .and_then(|result| to_js_value(&result))
    }

    /// Rescan all roots.
    #[wasm_bindgen(js_name = rescanAll)]
    pub fn rescan_all(&self, reason: JsValue) -> Result<JsValue, JsValue> {
        let reason = parse_rescan_reason(reason)?;

        self.inner
            .rescan_all(reason.into())
            .map(workspace_service_result_binding)
            .map_err(js_error)
            .and_then(|result| to_js_value(&result))
    }

    /// Rescan selected roots.
    #[wasm_bindgen(js_name = rescanRoots)]
    pub fn rescan_roots(&self, roots: JsValue, analyze: bool) -> Result<JsValue, JsValue> {
        let roots = serde_wasm_bindgen::from_value::<Vec<String>>(roots)
            .map_err(js_error)?
            .into_iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();

        self.inner
            .rescan_roots(&roots, analyze)
            .map(workspace_service_result_binding)
            .map_err(js_error)
            .and_then(|result| to_js_value(&result))
    }

    /// Analyze the module owning a path.
    #[wasm_bindgen(js_name = analyzePath)]
    pub fn analyze_path(&self, path: String) -> Result<JsValue, JsValue> {
        self.inner
            .analyze_path(&PathBuf::from(path))
            .map(workspace_analyze_outcome_binding)
            .map_err(js_error)
            .and_then(|outcome| to_js_value(&outcome))
    }

    /// Ensure semantic query state is ready for a path.
    #[wasm_bindgen(js_name = ensureAnalyzedForPath)]
    pub fn ensure_analyzed_for_path(&self, path: String) -> Result<(), JsValue> {
        self.inner
            .ensure_analyzed_for_path(&PathBuf::from(path))
            .map_err(js_error)
    }

    /// Execute a query for the workspace that owns a path.
    #[cfg(feature = "query")]
    #[wasm_bindgen(js_name = queryForPath)]
    pub fn query_for_path(&self, path: String, request: JsValue) -> Result<JsValue, JsValue> {
        let request =
            serde_wasm_bindgen::from_value::<workspace_service::query::QueryRequest>(request)
                .map_err(js_error)?;
        let response = self
            .inner
            .query_for_path(&PathBuf::from(path), request)
            .map_err(js_error)?;

        to_js_value(&response)
    }

    /// Return a query feature error for the workspace that owns a path.
    #[cfg(not(feature = "query"))]
    #[wasm_bindgen(js_name = queryForPath)]
    pub fn query_for_path(&self, _path: String, _request: JsValue) -> Result<JsValue, JsValue> {
        Err(js_error(query_feature_disabled_message()))
    }

    /// Execute a query for a specific workspace handle.
    #[cfg(feature = "query")]
    #[wasm_bindgen(js_name = queryForHandle)]
    pub fn query_for_handle(&self, handle: String, request: JsValue) -> Result<JsValue, JsValue> {
        let handle = handle
            .parse::<u64>()
            .map_err(|error| js_error(format!("invalid workspace handle: {error}")))?;
        let request =
            serde_wasm_bindgen::from_value::<workspace_service::query::QueryRequest>(request)
                .map_err(js_error)?;

        let response = self
            .inner
            .query_for_handle(workspace_service::WorkspaceHandleId(handle), request)
            .map_err(js_error)?;

        to_js_value(&response)
    }

    /// Return a query feature error for a specific workspace handle.
    #[cfg(not(feature = "query"))]
    #[wasm_bindgen(js_name = queryForHandle)]
    pub fn query_for_handle(&self, _handle: String, _request: JsValue) -> Result<JsValue, JsValue> {
        Err(js_error(query_feature_disabled_message()))
    }
}

/// Get the default workspace service options.
#[wasm_bindgen(js_name = defaultLanguageServiceOptions)]
pub fn default_language_service_options() -> Result<JsValue, JsValue> {
    to_js_value(&LanguageServiceOptions::default())
}

/// Parse the rescan reason from js input.
fn parse_rescan_reason(reason: JsValue) -> Result<LanguageServiceRescanReason, JsValue> {
    serde_wasm_bindgen::from_value(reason).map_err(js_error)
}

/// Return a stable error message for query disabled builds.
#[cfg(not(feature = "query"))]
fn query_feature_disabled_message() -> &'static str {
    "workspace query APIs are disabled in this wasm build: enable the 'query' cargo feature"
}

/// Resolve roots for workspace service options.
fn roots_from_options(options: &LanguageServiceOptions, cwd: &std::path::Path) -> Vec<PathBuf> {
    if options.roots.is_empty() {
        vec![cwd.to_path_buf()]
    } else {
        options.roots.iter().map(PathBuf::from).collect()
    }
}

/// Convert a virtual update payload into a workspace file update.
fn file_update_from_virtual_update(
    update: WorkspaceVirtualUpdate,
) -> Result<workspace::FileUpdate, JsValue> {
    match update.kind {
        WorkspaceVirtualUpdateKind::Text => {
            let content = update
                .content
                .ok_or_else(|| js_error("text update requires content"))?;
            Ok(workspace::FileUpdate::Text { content })
        }
        WorkspaceVirtualUpdateKind::Bytes => {
            let content = update
                .bytes
                .ok_or_else(|| js_error("bytes update requires bytes"))?;
            Ok(workspace::FileUpdate::Bytes { content })
        }
        WorkspaceVirtualUpdateKind::Touch => Ok(workspace::FileUpdate::Touch),
        WorkspaceVirtualUpdateKind::Removed => Ok(workspace::FileUpdate::Removed),
    }
}

/// Convert a workspace watch event binding into source watch event payload.
fn file_watch_event_from_binding(event: WorkspaceWatchEvent) -> source::FileWatchEvent {
    let kind = match event.kind {
        WorkspaceWatchEventKind::Created => source::FileWatchEventKind::Created,
        WorkspaceWatchEventKind::Modified => source::FileWatchEventKind::Modified,
        WorkspaceWatchEventKind::Deleted => source::FileWatchEventKind::Deleted,
        WorkspaceWatchEventKind::Renamed => source::FileWatchEventKind::Renamed,
        WorkspaceWatchEventKind::Overflow => source::FileWatchEventKind::Overflow,
    };

    source::FileWatchEvent {
        path: PathBuf::from(event.path),
        previous_path: event.previous_path.map(PathBuf::from),
        kind,
    }
}

/// Convert workspace service result into wasm binding payload.
fn workspace_service_result_binding(
    result: workspace_service::LanguageServiceResult,
) -> LanguageServiceResult {
    LanguageServiceResult {
        updates: result
            .updates
            .into_iter()
            .map(workspace_update_record_binding)
            .collect(),
        messages: result
            .messages
            .into_iter()
            .map(|message| WorkspaceMessage {
                kind: message.kind.into(),
                code: message.code,
                message: message.message,
            })
            .collect(),
    }
}

/// Convert workspace update record into wasm binding payload.
fn workspace_update_record_binding(
    record: workspace_service::WorkspaceUpdateRecord,
) -> WorkspaceUpdateRecord {
    WorkspaceUpdateRecord {
        module_id: record.module_id.map(workspace_module_id_binding),
        file_id: record.file_id.0,
        file: WorkspaceFileSnapshot {
            id: record.file.id.0,
            name: record.file.name,
            uri: record.file.uri.to_string(),
            path: record
                .file
                .path
                .map(|path| path.to_string_lossy().to_string()),
            file_type: record.file.file_type.into(),
            content: record.file.content,
        },
        invalidation: WorkspaceInvalidation {
            file_id: record.invalidation.file_id.0,
            file_version: record.invalidation.file_version.0.to_string(),
            kinds: record
                .invalidation
                .kinds
                .into_iter()
                .map(|kind| match kind {
                    workspace::InvalidationKind::ModuleSource => {
                        WorkspaceInvalidationKind::ModuleSource
                    }
                    workspace::InvalidationKind::DsConfig => WorkspaceInvalidationKind::DsConfig,
                    workspace::InvalidationKind::TsConfig => WorkspaceInvalidationKind::TsConfig,
                    workspace::InvalidationKind::PackageManifest => {
                        WorkspaceInvalidationKind::PackageManifest
                    }
                    workspace::InvalidationKind::Unknown => WorkspaceInvalidationKind::Unknown,
                })
                .collect(),
            modules: record
                .invalidation
                .modules
                .into_iter()
                .map(workspace_module_id_binding)
                .collect(),
            packages: record
                .invalidation
                .packages
                .into_iter()
                .map(|id| id.0.to_string())
                .collect(),
            profiles: record
                .invalidation
                .profiles
                .into_iter()
                .map(|id| id.0)
                .collect(),
            graphs_dropped: record
                .invalidation
                .graphs_dropped
                .into_iter()
                .map(|id| id.0)
                .collect(),
        },
        diagnostics: record.diagnostics.into_iter().map(Into::into).collect(),
    }
}

/// Convert module id into wasm binding payload.
fn workspace_module_id_binding(module_id: source::ModuleId) -> WorkspaceModuleId {
    WorkspaceModuleId {
        package_id: module_id.package_id.0.to_string(),
        local_id: module_id.local_id,
    }
}

/// Convert analyze outcome into wasm binding payload.
fn workspace_analyze_outcome_binding(
    outcome: workspace_service::AnalyzeOutcome,
) -> WorkspaceAnalyzeOutcome {
    WorkspaceAnalyzeOutcome {
        semantic_query_ready: outcome.semantic_query_ready,
        detail: outcome.detail,
    }
}
