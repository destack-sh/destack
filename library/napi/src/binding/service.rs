use std::path::PathBuf;
use std::sync::Arc;

use napi::Error;
use napi_derive::napi;

use {
    destack_source as source, destack_workspace as workspace,
    destack_workspace_service as workspace_service,
};

use super::{CompilerOptions, Diagnostic};

/// Options for creating a WorkspaceService.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct WorkspaceServiceOptions {
    /// The current working directory.
    pub cwd: String,
    /// The workspace roots to open initially.
    pub roots: Vec<String>,
    /// Compiler options for analysis and updates.
    pub compiler: Option<CompilerOptions>,
}

impl Default for WorkspaceServiceOptions {
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
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[napi(object)]
#[derive(Debug, Clone)]
pub struct WorkspaceVirtualUpdate {
    /// The update kind.
    pub kind: WorkspaceVirtualUpdateKind,
    /// Text payload for text updates.
    pub content: Option<String>,
    /// Byte payload for binary updates.
    pub bytes: Option<Vec<u8>>,
}

/// Watch event kind.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[napi(object)]
#[derive(Debug, Clone)]
pub struct WorkspaceWatchEvent {
    /// The file path.
    pub path: String,
    /// The previous path for rename events.
    pub previous_path: Option<String>,
    /// The watch event kind.
    pub kind: WorkspaceWatchEventKind,
}

/// Rescan reason for workspace operations.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceServiceRescanReason {
    /// Requested during startup synchronization.
    Startup,
    /// Requested after watcher overflow.
    Overflow,
    /// Requested manually.
    Manual,
    /// Requested after watch updates.
    Update,
}

impl From<WorkspaceServiceRescanReason> for workspace_service::RescanReason {
    fn from(reason: WorkspaceServiceRescanReason) -> Self {
        match reason {
            WorkspaceServiceRescanReason::Startup => workspace_service::RescanReason::Startup,
            WorkspaceServiceRescanReason::Overflow => workspace_service::RescanReason::Overflow,
            WorkspaceServiceRescanReason::Manual => workspace_service::RescanReason::Manual,
            WorkspaceServiceRescanReason::Update => workspace_service::RescanReason::Update,
        }
    }
}

/// Workspace message kind.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceServiceMessageKind {
    /// Informational message.
    Info,
    /// Warning message.
    Warning,
    /// Error message.
    Error,
}

impl From<workspace_service::WorkspaceMessageKind> for WorkspaceServiceMessageKind {
    fn from(kind: workspace_service::WorkspaceMessageKind) -> Self {
        match kind {
            workspace_service::WorkspaceMessageKind::Info => WorkspaceServiceMessageKind::Info,
            workspace_service::WorkspaceMessageKind::Warning => {
                WorkspaceServiceMessageKind::Warning
            }
            workspace_service::WorkspaceMessageKind::Error => WorkspaceServiceMessageKind::Error,
        }
    }
}

/// Module id payload.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct WorkspaceModuleId {
    /// The owning package id.
    pub package_id: String,
    /// Local identifier within the package.
    pub local_id: u32,
}

/// Invalidation kind payload.
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[napi(object)]
#[derive(Debug, Clone)]
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
#[napi(object)]
#[derive(Debug, Clone)]
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
    pub file_type: String,
    /// Optional text content.
    pub content: Option<String>,
}

/// Workspace update record payload.
#[napi(object)]
#[derive(Debug, Clone)]
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
#[napi(object)]
#[derive(Debug, Clone)]
pub struct WorkspaceMessage {
    /// Message severity.
    pub kind: WorkspaceServiceMessageKind,
    /// Stable message code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

/// Result payload for workspace service updates.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct WorkspaceServiceResult {
    /// Update records produced by the operation.
    pub updates: Vec<WorkspaceUpdateRecord>,
    /// Message records produced by the operation.
    pub messages: Vec<WorkspaceMessage>,
}

/// Outcome payload for explicit analyze requests.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct WorkspaceAnalyzeOutcome {
    /// Whether semantic query state is ready for the analyzed module.
    pub semantic_query_ready: bool,
    /// Detail when semantic query state is not ready.
    pub detail: Option<String>,
}

/// NAPI wrapper for the Rust workspace service.
#[napi]
#[derive(Debug)]
pub struct WorkspaceService {
    /// Session backing this service.
    session: Arc<workspace::Session>,
    /// Workspace service for analysis and updates.
    inner: workspace_service::WorkspaceService,
}

#[napi]
impl WorkspaceService {
    /// Create a new WorkspaceService.
    #[napi(constructor)]
    pub fn new(options: WorkspaceServiceOptions) -> napi::Result<Self> {
        // normalize options
        let cwd = PathBuf::from(&options.cwd);
        let roots = roots_from_options(&options, &cwd);
        let compiler = options.compiler.unwrap_or_default();

        // create backing session and service
        let session = Arc::new(workspace::Session::new(cwd));
        let inner = workspace_service::WorkspaceService::with_options(
            session.clone(),
            roots,
            compiler.into(),
        )
        .map_err(napi_error_from_workspace_service)?;

        // assemble wrapper
        Ok(Self { session, inner })
    }

    /// Return the session cwd.
    #[napi(getter)]
    pub fn cwd(&self) -> String {
        self.session.cwd.to_string_lossy().to_string()
    }

    /// Return the number of active program handles.
    #[napi(getter)]
    pub fn program_handle_count(&self) -> u32 {
        self.inner.program_handle_count() as u32
    }

    /// Open a workspace root.
    #[napi]
    pub fn open_workspace_root(&self, root: String) -> napi::Result<()> {
        self.inner
            .open_workspace_root(PathBuf::from(root))
            .map_err(napi_error_from_workspace_service)
    }

    /// Close a workspace root.
    #[napi]
    pub fn close_workspace_root(&self, root: String) -> napi::Result<()> {
        self.inner
            .close_workspace_root(&PathBuf::from(root))
            .map_err(napi_error_from_workspace_service)
    }

    /// Return whether a workspace root is active.
    #[napi]
    pub fn has_workspace_root(&self, root: String) -> bool {
        self.inner.has_workspace_root(&PathBuf::from(root))
    }

    /// Remove a workspace root.
    #[napi]
    pub fn remove_workspace_root(&self, root: String) -> napi::Result<bool> {
        self.inner
            .remove_workspace_root(&PathBuf::from(root))
            .map_err(napi_error_from_workspace_service)
    }

    /// Clear cache entries for all roots.
    #[napi]
    pub fn clear_cache_all(&self) -> napi::Result<()> {
        self.inner
            .clear_cache_all()
            .map_err(napi_error_from_workspace_service)
    }

    /// Shutdown the service.
    #[napi]
    pub fn shutdown(&self) {
        self.inner.shutdown();
    }

    /// Return the workspace handle for a root as a decimal string.
    #[napi]
    pub fn handle_for_root(&self, root: String) -> napi::Result<String> {
        self.inner
            .handle_for_root(&PathBuf::from(root))
            .map(|handle| handle.0.to_string())
            .map_err(napi_error_from_workspace_service)
    }

    /// Apply a text update for a virtual file.
    #[napi]
    pub fn update_virtual_file(
        &self,
        path: String,
        content: String,
    ) -> napi::Result<WorkspaceServiceResult> {
        self.inner
            .update_virtual_file(&PathBuf::from(path), content)
            .map(workspace_service_result_binding)
            .map_err(napi_error_from_workspace_service)
    }

    /// Apply a virtual file update.
    #[napi]
    pub fn apply_virtual_update(
        &self,
        path: String,
        update: WorkspaceVirtualUpdate,
    ) -> napi::Result<WorkspaceServiceResult> {
        // convert binding update payload
        let update = file_update_from_virtual_update(update)?;

        // apply update and map result
        self.inner
            .apply_virtual_update(&PathBuf::from(path), update)
            .map(workspace_service_result_binding)
            .map_err(napi_error_from_workspace_service)
    }

    /// Apply watch events.
    #[napi]
    pub fn apply_watch_events(
        &self,
        events: Vec<WorkspaceWatchEvent>,
    ) -> napi::Result<WorkspaceServiceResult> {
        // map watch events into core payloads
        let events = events
            .into_iter()
            .map(file_watch_event_from_binding)
            .collect();

        // apply events and map result
        self.inner
            .apply_watch_events(events)
            .map(workspace_service_result_binding)
            .map_err(napi_error_from_workspace_service)
    }

    /// Rescan all roots.
    #[napi]
    pub fn rescan_all(
        &self,
        reason: WorkspaceServiceRescanReason,
    ) -> napi::Result<WorkspaceServiceResult> {
        self.inner
            .rescan_all(reason.into())
            .map(workspace_service_result_binding)
            .map_err(napi_error_from_workspace_service)
    }

    /// Rescan selected roots.
    #[napi]
    pub fn rescan_roots(
        &self,
        roots: Vec<String>,
        analyze: bool,
    ) -> napi::Result<WorkspaceServiceResult> {
        // map root paths
        let roots = roots.into_iter().map(PathBuf::from).collect::<Vec<_>>();

        // execute root rescan
        self.inner
            .rescan_roots(&roots, analyze)
            .map(workspace_service_result_binding)
            .map_err(napi_error_from_workspace_service)
    }

    /// Analyze the module owning a path.
    #[napi]
    pub fn analyze_path(&self, path: String) -> napi::Result<WorkspaceAnalyzeOutcome> {
        self.inner
            .analyze_path(&PathBuf::from(path))
            .map(workspace_analyze_outcome_binding)
            .map_err(napi_error_from_workspace_service)
    }

    /// Ensure semantic query state is ready for a path.
    #[napi]
    pub fn ensure_analyzed_for_path(&self, path: String) -> napi::Result<()> {
        self.inner
            .ensure_analyzed_for_path(&PathBuf::from(path))
            .map_err(napi_error_from_workspace_service)
    }

    /// Execute a query for the workspace that owns a path using a JSON payload.
    #[napi]
    pub fn query_for_path_json(&self, path: String, request_json: String) -> napi::Result<String> {
        // decode query request
        let request = serde_json::from_str::<workspace::query::QueryRequest>(&request_json)
            .map_err(|error| Error::from_reason(format!("invalid query request json: {error}")))?;

        // execute query
        let response = self
            .inner
            .query_for_path(&PathBuf::from(path), request)
            .map_err(napi_error_from_workspace_service)?;

        // encode response json
        serde_json::to_string(&response).map_err(|error| {
            Error::from_reason(format!("failed to encode query response: {error}"))
        })
    }

    /// Execute a query for a specific workspace handle using a JSON payload.
    #[napi]
    pub fn query_for_handle_json(
        &self,
        handle: String,
        request_json: String,
    ) -> napi::Result<String> {
        // parse handle and request
        let handle = handle
            .parse::<u64>()
            .map_err(|error| Error::from_reason(format!("invalid workspace handle: {error}")))?;
        let request = serde_json::from_str::<workspace::query::QueryRequest>(&request_json)
            .map_err(|error| Error::from_reason(format!("invalid query request json: {error}")))?;

        // execute query
        let response = self
            .inner
            .query_for_handle(workspace_service::WorkspaceHandleId(handle), request)
            .map_err(napi_error_from_workspace_service)?;

        // encode response json
        serde_json::to_string(&response).map_err(|error| {
            Error::from_reason(format!("failed to encode query response: {error}"))
        })
    }
}

/// Get the default workspace service options.
#[napi(js_name = "defaultWorkspaceServiceOptions")]
pub fn default_workspace_service_options() -> WorkspaceServiceOptions {
    WorkspaceServiceOptions::default()
}

/// Build workspace roots for service options.
fn roots_from_options(options: &WorkspaceServiceOptions, cwd: &std::path::Path) -> Vec<PathBuf> {
    if options.roots.is_empty() {
        vec![cwd.to_path_buf()]
    } else {
        options.roots.iter().map(PathBuf::from).collect()
    }
}

/// Convert a virtual update payload into a core file update.
fn file_update_from_virtual_update(
    update: WorkspaceVirtualUpdate,
) -> napi::Result<workspace::FileUpdate> {
    match update.kind {
        WorkspaceVirtualUpdateKind::Text => {
            let content = update
                .content
                .ok_or_else(|| Error::from_reason("text update requires content".to_string()))?;
            Ok(workspace::FileUpdate::Text { content })
        }
        WorkspaceVirtualUpdateKind::Bytes => {
            let content = update
                .bytes
                .ok_or_else(|| Error::from_reason("bytes update requires bytes".to_string()))?;
            Ok(workspace::FileUpdate::Bytes { content })
        }
        WorkspaceVirtualUpdateKind::Touch => Ok(workspace::FileUpdate::Touch),
        WorkspaceVirtualUpdateKind::Removed => Ok(workspace::FileUpdate::Removed),
    }
}

/// Convert a watch event payload into a core watch event.
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

/// Convert a core workspace service result into a binding payload.
fn workspace_service_result_binding(
    result: workspace_service::WorkspaceServiceResult,
) -> WorkspaceServiceResult {
    WorkspaceServiceResult {
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

/// Convert a core workspace update record into a binding payload.
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
            file_type: format!("{:?}", record.file.file_type),
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

/// Convert a core module id into a binding payload.
fn workspace_module_id_binding(module_id: source::ModuleId) -> WorkspaceModuleId {
    WorkspaceModuleId {
        package_id: module_id.package_id.0.to_string(),
        local_id: module_id.local_id,
    }
}

/// Convert analyze outcomes into binding payloads.
fn workspace_analyze_outcome_binding(
    outcome: workspace_service::AnalyzeOutcome,
) -> WorkspaceAnalyzeOutcome {
    WorkspaceAnalyzeOutcome {
        semantic_query_ready: outcome.semantic_query_ready,
        detail: outcome.detail,
    }
}

/// Convert workspace service errors into NAPI errors.
fn napi_error_from_workspace_service(error: workspace_service::WorkspaceServiceError) -> Error {
    Error::from_reason(error.to_string())
}
