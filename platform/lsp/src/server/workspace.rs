use std::collections::{HashMap, HashSet, VecDeque};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use destack_compiler::{AnalyzeTask, Compiler, CompilerOptions, ResolveTask};
use destack_resolver::{CachePolicy, ResolveError, ResolveOptions, Resolver};
use destack_source::{
    Diagnostic, DiagnosticStoreUpdate, File, FileContent, FileId, FileType, FileWatchEvent,
    FileWatchEventKind, ModuleId, ModuleStamp, Span, Uri,
};
use destack_workspace::{
    FileUpdate, InvalidationKind, InvalidationPlan, Module, ModuleContent, ModuleGraphKey, Program,
    QueryResponseEnvelope, Session, TargetId, query,
};
use parking_lot::Mutex;

/// Workspace handle identifier used by the local driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkspaceHandleId(pub u64);

/// Reason for a workspace rescan request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RescanReason {
    /// Requested manually.
    Manual,
    /// Requested after watch updates.
    Update,
}

/// Message severity for local workspace records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceMessageKind {
    /// Warning message.
    Warning,
}

/// Message payload emitted by the local workspace driver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceMessageRecord {
    /// Message severity.
    pub kind: WorkspaceMessageKind,
    /// Stable message code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

/// Snapshot for updated files.
#[derive(Debug, Clone, PartialEq)]
pub struct FileSnapshot {
    /// File id in the registry.
    pub id: FileId,
    /// File name.
    pub name: String,
    /// File uri.
    pub uri: Uri,
    /// Optional file path.
    pub path: Option<PathBuf>,
    /// File type.
    pub file_type: FileType,
    /// Optional text content.
    pub content: Option<String>,
}

/// Update record emitted by the local workspace driver.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceUpdateRecord {
    /// Updated file snapshot.
    pub file: FileSnapshot,
    /// Diagnostics for this file.
    pub diagnostics: Vec<Diagnostic>,
}

/// Result of applying local workspace driver updates.
#[derive(Debug, Default)]
pub struct LspWorkspaceResult {
    /// Update records produced by the operation.
    pub updates: Vec<WorkspaceUpdateRecord>,
    /// Message records produced by the operation.
    pub messages: Vec<WorkspaceMessageRecord>,
}

/// Per program local driver handle.
#[derive(Debug)]
struct ProgramHandle {
    /// Program for this root.
    program: Arc<Program>,
    /// Compiler for this root.
    compiler: Arc<Compiler>,
    /// Serialize compilation per root.
    compile_lock: Mutex<()>,
}

/// Internal update with invalidation metadata.
#[derive(Debug, Clone)]
struct DriverUpdate {
    /// Updated module id when known.
    module_id: Option<ModuleId>,
    /// Updated file id.
    file_id: FileId,
    /// Updated file snapshot.
    file: FileSnapshot,
    /// Invalidation summary.
    invalidation: InvalidationPlan,
    /// Diagnostics for the updated file.
    diagnostics: Vec<Diagnostic>,
}

/// Local workspace backed driver used by LSP.
#[derive(Debug)]
pub struct LspWorkspaceDriver {
    /// Session for workspace resolution.
    session: Arc<Session>,
    /// Compiler options for local analysis.
    compiler_options: CompilerOptions,
    /// Program handles keyed by root path.
    program_handles: DashMap<PathBuf, Arc<ProgramHandle>>,
    /// Handle ids keyed by root path.
    handles_by_root: DashMap<PathBuf, WorkspaceHandleId>,
    /// Root paths keyed by handle id.
    roots_by_handle: DashMap<WorkspaceHandleId, PathBuf>,
    /// Next handle id.
    next_handle_id: AtomicU64,
}

impl LspWorkspaceDriver {
    /// Create a local workspace driver for the provided roots.
    pub fn new(session: Arc<Session>, roots: Vec<PathBuf>) -> Result<Self, String> {
        Self::new_in_process(session, roots)
    }

    /// Create a local workspace driver for the provided roots.
    pub fn new_in_process(session: Arc<Session>, roots: Vec<PathBuf>) -> Result<Self, String> {
        let workspace = Self {
            session,
            compiler_options: CompilerOptions::default(),
            program_handles: DashMap::new(),
            handles_by_root: DashMap::new(),
            roots_by_handle: DashMap::new(),
            next_handle_id: AtomicU64::new(1),
        };

        for root in roots {
            workspace.open_workspace_root(root)?;
        }

        Ok(workspace)
    }

    /// Ensure the workspace root is opened.
    pub fn open_workspace_root(&self, root: PathBuf) -> Result<(), String> {
        if self.handles_by_root.contains_key(&root) {
            return Ok(());
        }

        self.session.get_or_create_program(root.clone());
        let handle_id = self.next_handle_id.fetch_add(1, Ordering::Relaxed);
        let handle = WorkspaceHandleId(handle_id);
        self.handles_by_root.insert(root.clone(), handle);
        self.roots_by_handle.insert(handle, root);
        Ok(())
    }

    /// Close an opened workspace root.
    pub fn close_workspace_root(&self, root: &Path) -> Result<(), String> {
        let Some((_, handle)) = self.handles_by_root.remove(root) else {
            return Ok(());
        };

        self.roots_by_handle.remove(&handle);
        self.program_handles.remove(root);
        Ok(())
    }

    /// Ensure the path is analyzed and query-ready.
    pub fn ensure_analyzed_for_path(&self, path: &Path) -> Result<(), String> {
        let outcome = self.analyze_path(path)?;
        if outcome.query_context_ready {
            return Ok(());
        }

        let detail = outcome
            .detail
            .unwrap_or_else(|| "query context not ready after analyze".to_string());
        Err(format!("analyze request failed: {detail}"))
    }

    /// Apply a virtual file update through the local driver.
    pub fn update_virtual_file(
        &self,
        path: &Path,
        content: String,
    ) -> Result<LspWorkspaceResult, String> {
        let updates = self.apply_virtual_file_update(path, FileUpdate::Text { content })?;
        Ok(LspWorkspaceResult {
            updates: updates
                .into_iter()
                .map(|update| WorkspaceUpdateRecord {
                    file: update.file,
                    diagnostics: update.diagnostics,
                })
                .collect(),
            messages: Vec::new(),
        })
    }

    /// Apply watch events through the local driver.
    pub fn apply_watch_events(
        &self,
        events: Vec<FileWatchEvent>,
    ) -> Result<LspWorkspaceResult, String> {
        let mut result = LspWorkspaceResult::default();
        if events.is_empty() {
            return Ok(result);
        }

        let mut require_rescan = false;
        for event in events {
            if matches!(event.kind, FileWatchEventKind::Overflow) {
                require_rescan = true;
                result.messages.push(warning_message(
                    "watch_overflow_rescan",
                    "watch: rescan required after overflow",
                ));
                continue;
            }

            if matches!(event.kind, FileWatchEventKind::Deleted) {
                if !self.is_watchable_path(&event.path) {
                    continue;
                }

                match self.remove_virtual_file(&event.path) {
                    Ok(updates) => result.updates.extend(updates.into_iter().map(|update| {
                        WorkspaceUpdateRecord {
                            file: update.file,
                            diagnostics: update.diagnostics,
                        }
                    })),
                    Err(error) => result.messages.push(warning_message(
                        "watch_remove_failed",
                        &format!("watch: failed to remove {}: {error}", event.path.display()),
                    )),
                }
                continue;
            }

            if matches!(event.kind, FileWatchEventKind::Renamed) {
                if let Some(previous_path) = event.previous_path.as_ref()
                    && self.is_watchable_path(previous_path)
                {
                    match self.remove_virtual_file(previous_path) {
                        Ok(updates) => result.updates.extend(updates.into_iter().map(|update| {
                            WorkspaceUpdateRecord {
                                file: update.file,
                                diagnostics: update.diagnostics,
                            }
                        })),
                        Err(error) => result.messages.push(warning_message(
                            "watch_remove_failed",
                            &format!(
                                "watch: failed to remove {}: {error}",
                                previous_path.display()
                            ),
                        )),
                    }
                }

                if self.is_watchable_path(&event.path) {
                    match self.session.fs.read_to_string(&event.path) {
                        Ok(content) => match self.update_virtual_file(&event.path, content) {
                            Ok(update_result) => {
                                result.updates.extend(update_result.updates);
                                result.messages.extend(update_result.messages);
                            }
                            Err(error) => result.messages.push(warning_message(
                                "watch_update_failed",
                                &format!(
                                    "watch: failed to update {}: {error}",
                                    event.path.display()
                                ),
                            )),
                        },
                        Err(error) => result.messages.push(warning_message(
                            "watch_read_failed",
                            &format!("watch: failed to read {}: {error}", event.path.display()),
                        )),
                    }
                }

                continue;
            }

            if !self.is_watchable_path(&event.path) {
                continue;
            }

            match self.session.fs.read_to_string(&event.path) {
                Ok(content) => match self.update_virtual_file(&event.path, content) {
                    Ok(update_result) => {
                        result.updates.extend(update_result.updates);
                        result.messages.extend(update_result.messages);
                    }
                    Err(error) => result.messages.push(warning_message(
                        "watch_update_failed",
                        &format!("watch: failed to update {}: {error}", event.path.display()),
                    )),
                },
                Err(error) => result.messages.push(warning_message(
                    "watch_read_failed",
                    &format!("watch: failed to read {}: {error}", event.path.display()),
                )),
            }
        }

        if require_rescan {
            let rescan = self.rescan_all(RescanReason::Update)?;
            result.updates.extend(rescan.updates);
            result.messages.extend(rescan.messages);
        }

        Ok(result)
    }

    /// Request a rescan for every workspace handle.
    pub fn rescan_all(&self, _reason: RescanReason) -> Result<LspWorkspaceResult, String> {
        let mut result = LspWorkspaceResult::default();
        let roots: Vec<PathBuf> = self
            .handles_by_root
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        for root in roots {
            let handle = self.program_handle_for_root(&root);
            let _compile_guard = handle.compile_lock.lock();
            let rescan = self.rescan_program(&handle)?;
            result.updates.extend(rescan.updates);
            result.messages.extend(rescan.messages);
        }

        Ok(result)
    }

    /// Clear all cache entries for every workspace handle.
    pub fn clear_cache_all(&self) -> Result<(), String> {
        let cache_dir = self.session.workspace_cache_dir();
        if !cache_dir.exists() {
            return Ok(());
        }

        std::fs::remove_dir_all(&cache_dir)
            .map_err(|error| format!("cache clear failed at {}: {error}", cache_dir.display()))?;
        Ok(())
    }

    /// Execute a workspace query for the workspace that owns the path.
    pub fn query_for_path(
        &self,
        path: &Path,
        request: query::QueryRequest,
    ) -> Result<QueryResponseEnvelope, String> {
        let handle = self.handle_for_path(path)?;
        self.query_for_handle(handle, request)
    }

    /// Execute a workspace query for a specific workspace handle.
    pub fn query_for_handle(
        &self,
        handle: WorkspaceHandleId,
        request: query::QueryRequest,
    ) -> Result<QueryResponseEnvelope, String> {
        let root = self
            .roots_by_handle
            .get(&handle)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| format!("unknown workspace handle: {handle:?}"))?;
        let program = self.session.get_or_create_program(root);
        let response = self.query_response_for_request(&program, request)?;
        let snapshot_id = format!("handle:{}", handle.0);

        Ok(QueryResponseEnvelope {
            snapshot_id,
            response,
        })
    }

    /// Shutdown the local driver.
    pub fn shutdown(&self) {
        // no persistent connection to shut down
    }

    /// Resolve the workspace handle for a path.
    fn handle_for_path(&self, path: &Path) -> Result<WorkspaceHandleId, String> {
        let program = self.session.find_program_for_path(path);
        let root = program.cwd.clone();

        if let Some(handle) = self.handles_by_root.get(&root) {
            return Ok(*handle.value());
        }

        self.open_workspace_root(root.clone())?;
        self.handles_by_root
            .get(&root)
            .map(|entry| *entry.value())
            .ok_or_else(|| "workspace handle missing after open".to_string())
    }

    /// Resolve or create a program handle for a root.
    fn program_handle_for_root(&self, root: &Path) -> Arc<ProgramHandle> {
        match self.program_handles.entry(root.to_path_buf()) {
            Entry::Occupied(entry) => Arc::clone(entry.get()),
            Entry::Vacant(entry) => {
                let program = self.session.get_or_create_program(root.to_path_buf());
                let compiler = Arc::new(Compiler::new(
                    self.session.clone(),
                    program.clone(),
                    self.compiler_options.clone(),
                ));
                let handle = Arc::new(ProgramHandle {
                    program,
                    compiler,
                    compile_lock: Mutex::new(()),
                });
                entry.insert(handle.clone());
                handle
            }
        }
    }

    /// Resolve or create a program handle for a path.
    fn program_handle_for_path(&self, path: &Path) -> Arc<ProgramHandle> {
        let program = self.session.find_program_for_path(path);
        self.program_handle_for_root(&program.cwd)
    }

    /// Mark a file as removed without touching disk.
    fn remove_virtual_file(&self, path: &Path) -> Result<Vec<DriverUpdate>, String> {
        self.apply_virtual_file_update(path, FileUpdate::Removed)
    }

    /// Apply a virtual file update.
    fn apply_virtual_file_update(
        &self,
        path: &Path,
        update: FileUpdate,
    ) -> Result<Vec<DriverUpdate>, String> {
        let handle = self.program_handle_for_path(path);
        let _compile_guard = handle.compile_lock.lock();
        let program = handle.program.clone();
        let compiler = handle.compiler.clone();

        let config_file_id = if self.is_config_filename(path) {
            let mut file_id = program.files.get_id_by_path(path);
            if file_id.is_none() {
                let resolver = Resolver::from_program(&program, ResolveOptions::default());
                if path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name == "dsconfig.json")
                {
                    let _ = resolver.load_dsconfig(path, CachePolicy::Reload);
                } else {
                    let _ = resolver.reload_tsconfig(path);
                }
                file_id = program.files.get_id_by_path(path);
            }
            file_id
        } else {
            None
        };

        let module_id = if config_file_id.is_some() {
            None
        } else {
            match compiler.resolve_path_to_module(&path.to_path_buf()) {
                Ok(module_id) => Some(module_id),
                Err(error) => {
                    if program.files.get_id_by_path(path).is_some() {
                        None
                    } else {
                        return Err(format!("resolve failed for {}: {error}", path.display()));
                    }
                }
            }
        };

        let file_id = if let Some(file_id) = config_file_id {
            file_id
        } else {
            match module_id {
                Some(module_id) => program.modules.get(module_id).read().file_id,
                None => program
                    .files
                    .get_id_by_path(path)
                    .ok_or_else(|| format!("file not tracked: {}", path.display()))?,
            }
        };

        let invalidation = program
            .invalidate_file(file_id, update)
            .map_err(|error| format!("invalidate failed for {}: {error}", path.display()))?;

        if self.should_refresh_configs(&program, &invalidation, path) {
            let resolver = Resolver::from_program(&program, ResolveOptions::default());
            let _ = self.refresh_program_configs(&resolver, &program);
        }

        let mut updates = vec![build_update(&program, module_id, file_id, invalidation)?];
        self.analyze_updates(&program, &compiler, &mut updates)?;

        Ok(updates)
    }

    /// Rescan tracked files for a single program.
    fn rescan_program(&self, handle: &ProgramHandle) -> Result<LspWorkspaceResult, String> {
        let program = handle.program.as_ref();

        let mut file_ids = HashSet::new();
        for module in program.modules.iter() {
            let module = module.read();
            if !self.is_workspace_module(&module) {
                continue;
            }
            file_ids.insert(module.file_id);
        }

        if let Some(workspace_config) = self.session.workspace_config() {
            file_ids.insert(workspace_config.file_id);
        }

        for package in program.packages.iter() {
            let package = package.read();
            if let Some(dsconfig) = package.dsconfig.as_ref() {
                file_ids.insert(dsconfig.file_id);
            }
        }

        for tsconfig in program.tsconfigs.iter() {
            let tsconfig = tsconfig.read();
            file_ids.insert(tsconfig.file_id);
        }

        let mut updates = Vec::new();
        let mut messages = Vec::new();
        for file_id in file_ids {
            let update = match self.rescan_file_update(program, file_id) {
                Ok(Some(update)) => update,
                Ok(None) => continue,
                Err(message) => {
                    messages.push(message);
                    continue;
                }
            };

            let invalidation = match program.invalidate_file(file_id, update) {
                Ok(invalidation) => invalidation,
                Err(error) => {
                    messages.push(warning_message(
                        "rescan_invalidation_failed",
                        &format!("watch: failed to rescan {file_id:?}: {error}"),
                    ));
                    continue;
                }
            };

            let module_id = program.modules.get_id_by_file_id(file_id);
            match build_update(program, module_id, file_id, invalidation) {
                Ok(update) => updates.push(update),
                Err(error) => messages.push(warning_message(
                    "rescan_invalidation_failed",
                    &format!("watch: failed to rescan {file_id:?}: {error}"),
                )),
            }
        }

        let resolver = Resolver::from_program(program, ResolveOptions::default());
        messages.extend(self.refresh_program_configs(&resolver, program));
        self.analyze_updates(program, handle.compiler.as_ref(), &mut updates)?;

        Ok(LspWorkspaceResult {
            updates: updates
                .into_iter()
                .map(|update| WorkspaceUpdateRecord {
                    file: update.file,
                    diagnostics: update.diagnostics,
                })
                .collect(),
            messages,
        })
    }

    /// Analyze a path and update diagnostics.
    fn analyze_path(&self, path: &Path) -> Result<AnalyzeOutcome, String> {
        let handle = self.program_handle_for_path(path);
        let _compile_guard = handle.compile_lock.lock();
        let program = handle.program.clone();
        let compiler = handle.compiler.clone();

        let module_id = compiler
            .resolve_path_to_module(&path.to_path_buf())
            .map_err(|error| format!("resolve failed for {}: {error}", path.display()))?;

        let _ = program.diagnostics.drain();

        let profile_id = program.default_profile_id_for_module(module_id);
        let module = compiler.module_stamp(module_id);
        let profile = compiler.profile_stamp(profile_id);
        compiler.enqueue(AnalyzeTask::AnalyzeModule { module, profile });
        compiler.compile();

        let diagnostics = program.diagnostics.collect();
        let mut diagnostics_by_file: HashMap<FileId, Vec<Diagnostic>> = HashMap::new();
        for diagnostic in diagnostics.iter() {
            diagnostics_by_file
                .entry(diagnostic.file_id)
                .or_default()
                .push(diagnostic.clone());
        }

        let mut store_updates = Vec::new();
        let module = program.modules.get(module_id);
        let module = module.read();
        let module_file_id = module.file_id;
        let module_source_version = module.source_version;
        let ast_ready = module.ast_maybe().is_some();
        let dir_ready = module.dir_maybe(profile_id).is_some();
        let query_context_ready = ast_ready && dir_ready;
        let detail = if query_context_ready {
            None
        } else {
            let module_path = module
                .path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "<none>".to_string());
            Some(format!(
                "file_id={module_file_id:?} module_id={module_id:?} profile_id={profile_id:?} ast_ready={ast_ready} dir_ready={dir_ready} path={module_path}",
            ))
        };

        store_updates.push(DiagnosticStoreUpdate::new(
            module_file_id,
            module_source_version,
            diagnostics_by_file
                .remove(&module_file_id)
                .unwrap_or_default(),
        ));

        for (file_id, diagnostics) in diagnostics_by_file {
            let file = program
                .files
                .get_maybe(file_id)
                .ok_or_else(|| format!("file not tracked for diagnostics update: {file_id:?}"))?;
            store_updates.push(DiagnosticStoreUpdate::new(
                file_id,
                file.version,
                diagnostics,
            ));
        }

        program.diagnostic_store.apply_updates(store_updates);

        Ok(AnalyzeOutcome {
            query_context_ready,
            detail,
        })
    }

    /// Analyze updates and attach diagnostics.
    fn analyze_updates(
        &self,
        program: &Program,
        compiler: &Compiler,
        updates: &mut Vec<DriverUpdate>,
    ) -> Result<(), String> {
        let mut module_ids = HashSet::new();
        let mut file_ids = HashSet::new();
        for update in updates.iter() {
            if let Some(module_id) = update.module_id
                && self.is_workspace_module_id(program, module_id)
            {
                module_ids.insert(module_id);
            }
            file_ids.insert(update.file_id);
        }

        let mut extra_updates = Vec::new();
        for update in updates.iter() {
            for module_id in update.invalidation.modules.iter().copied() {
                if !self.is_workspace_module_id(program, module_id) {
                    continue;
                }

                module_ids.insert(module_id);
                let module = program.modules.get(module_id);
                let file_id = module.read().file_id;
                if file_ids.insert(file_id) {
                    let extra_update = build_update(
                        program,
                        Some(module_id),
                        file_id,
                        update.invalidation.clone(),
                    )?;
                    extra_updates.push(extra_update);
                }
            }
        }
        updates.extend(extra_updates);

        if !module_ids.is_empty() {
            let _ = program.diagnostics.drain();
        }

        self.ensure_module_graphs_ready(program, compiler, &module_ids);

        if module_ids.len() < program.modules.len() {
            let mut queue: VecDeque<ModuleId> = module_ids.iter().copied().collect();
            while let Some(module_id) = queue.pop_front() {
                let profile_id = program.default_profile_id_for_module(module_id);
                let graph_key = ModuleGraphKey::new(profile_id);
                let Some(graph) = program.index.module_graphs.get(&graph_key) else {
                    continue;
                };

                let module = program.modules.get(module_id);
                let module_version = module.read().version;
                let Some(graph_version) = graph.module_versions.get(&module_id) else {
                    continue;
                };
                if *graph_version != module_version {
                    continue;
                }

                for dependent in graph.dependents_for(module_id) {
                    if !self.is_workspace_module_id(program, dependent) {
                        continue;
                    }

                    if module_ids.insert(dependent) {
                        queue.push_back(dependent);
                    }
                }
            }
        }

        let invalidation_template = updates.first().map(|update| update.invalidation.clone());
        if let Some(invalidation) = invalidation_template {
            let mut dependency_updates = Vec::new();
            for module_id in module_ids.iter().copied() {
                let module = program.modules.get(module_id);
                let file_id = module.read().file_id;
                if file_ids.insert(file_id) {
                    let dependency_update =
                        build_update(program, Some(module_id), file_id, invalidation.clone())?;
                    dependency_updates.push(dependency_update);
                }
            }
            updates.extend(dependency_updates);
        }

        if !module_ids.is_empty() {
            for module_id in &module_ids {
                let profile = program.default_profile_id_for_module(*module_id);
                let module = compiler.module_stamp(*module_id);
                let profile = compiler.profile_stamp(profile);
                compiler.enqueue(AnalyzeTask::AnalyzeModule { module, profile });
            }

            compiler.compile();

            let mut diagnostics_by_file: HashMap<FileId, Vec<Diagnostic>> = HashMap::new();
            for diagnostic in program.diagnostics.iter() {
                if file_ids.contains(&diagnostic.file_id) {
                    diagnostics_by_file
                        .entry(diagnostic.file_id)
                        .or_default()
                        .push(diagnostic.clone());
                }
            }

            for update in updates.iter_mut() {
                update.diagnostics = diagnostics_by_file
                    .remove(&update.file_id)
                    .unwrap_or_default();
            }
        }

        let mut store_updates = Vec::new();
        for update in updates.iter() {
            let file = program.files.get_maybe(update.file_id).ok_or_else(|| {
                format!(
                    "file not tracked for diagnostics update: {:?}",
                    update.file_id
                )
            })?;
            store_updates.push(DiagnosticStoreUpdate::new(
                update.file_id,
                file.version,
                update.diagnostics.clone(),
            ));
        }

        program.diagnostic_store.apply_updates(store_updates);
        Ok(())
    }

    /// Ensure module graph entries exist for dependency fanout.
    fn ensure_module_graphs_ready(
        &self,
        program: &Program,
        compiler: &Compiler,
        module_ids: &HashSet<ModuleId>,
    ) {
        if module_ids.is_empty() {
            return;
        }

        let mut resolve_tasks = Vec::new();
        let mut queued = HashSet::new();
        for module_id in module_ids.iter().copied() {
            let profile_id = program.default_profile_id_for_module(module_id);
            let graph_key = ModuleGraphKey::new(profile_id);
            if let Some(graph) = program.index.module_graphs.get(&graph_key) {
                let module = program.modules.get(module_id);
                let module = module.read();
                let graph_version = graph.module_versions.get(&module_id).copied();
                if graph_version == Some(module.version) {
                    continue;
                }

                if queued.insert((module_id, profile_id)) {
                    let module = ModuleStamp::new(module_id, module.version);
                    let profile = compiler.profile_stamp(profile_id);
                    let graph = compiler.module_graph_stamp(profile_id);
                    resolve_tasks.push(ResolveTask::ResolveModuleCanonical {
                        module,
                        profile,
                        graph,
                    });
                }

                continue;
            }

            for module in program.modules.iter() {
                let module = module.read();
                if !self.is_workspace_module(&module) {
                    continue;
                }
                if program.default_profile_id_for_module(module.id) != profile_id {
                    continue;
                }

                if queued.insert((module.id, profile_id)) {
                    let module = ModuleStamp::new(module.id, module.version);
                    let profile = compiler.profile_stamp(profile_id);
                    let graph = compiler.module_graph_stamp(profile_id);
                    resolve_tasks.push(ResolveTask::ResolveModuleCanonical {
                        module,
                        profile,
                        graph,
                    });
                }
            }
        }

        if resolve_tasks.is_empty() {
            return;
        }

        for task in resolve_tasks {
            compiler.enqueue(task);
        }
        compiler.compile();
    }

    /// Build a file update by reading the latest content from disk.
    fn rescan_file_update(
        &self,
        program: &Program,
        file_id: FileId,
    ) -> Result<Option<FileUpdate>, WorkspaceMessageRecord> {
        let Some(file) = program.files.get_maybe(file_id) else {
            return Ok(None);
        };

        let Some(path) = file.path.clone().or_else(|| file.uri.to_path_buf()) else {
            return Ok(None);
        };

        if file.ty.is_binary() {
            let bytes = match program.fs.read(&path) {
                Ok(bytes) => bytes,
                Err(error) => {
                    return self.handle_rescan_read_error(&file, &path, error);
                }
            };

            if self.should_update_bytes(&file, &bytes) {
                Ok(Some(FileUpdate::Bytes { content: bytes }))
            } else {
                Ok(None)
            }
        } else {
            let content = match program.fs.read_to_string(&path) {
                Ok(content) => content,
                Err(error) => {
                    return self.handle_rescan_read_error(&file, &path, error);
                }
            };

            if self.should_update_text(&file, &content) {
                Ok(Some(FileUpdate::Text { content }))
            } else {
                Ok(None)
            }
        }
    }

    /// Decide whether a text update should be applied.
    fn should_update_text(&self, file: &File, content: &str) -> bool {
        match &file.content {
            FileContent::Text { content: current } => current != content,
            FileContent::Json {
                content: current, ..
            } => current != content,
            FileContent::Missing | FileContent::Unloaded => true,
            FileContent::Binary { .. } => true,
        }
    }

    /// Decide whether a byte update should be applied.
    fn should_update_bytes(&self, file: &File, bytes: &[u8]) -> bool {
        match &file.content {
            FileContent::Binary { content } => content.as_slice() != bytes,
            FileContent::Missing | FileContent::Unloaded => true,
            FileContent::Text { .. } | FileContent::Json { .. } => true,
        }
    }

    /// Handle file read errors during rescan.
    fn handle_rescan_read_error(
        &self,
        file: &File,
        path: &Path,
        error: io::Error,
    ) -> Result<Option<FileUpdate>, WorkspaceMessageRecord> {
        if error.kind() == io::ErrorKind::NotFound {
            if matches!(file.content, FileContent::Missing) {
                return Ok(None);
            }

            return Ok(Some(FileUpdate::Removed));
        }

        Err(warning_message(
            "rescan_read_failed",
            &format!("watch: failed to read {}: {error}", path.display()),
        ))
    }

    /// Refresh package and workspace configuration for a program.
    fn refresh_program_configs(
        &self,
        resolver: &Resolver,
        program: &Program,
    ) -> Vec<WorkspaceMessageRecord> {
        let mut messages = Vec::new();

        let workspace_root = self.session.workspace_root();
        match resolver.load_dsconfig(&workspace_root, CachePolicy::Reload) {
            Ok(dsconfig) => {
                self.session.update_workspace_config(Some(dsconfig));
            }
            Err(ResolveError::DsConfigNotFound { .. }) => {
                self.session.update_workspace_config(None);
            }
            Err(error) => {
                messages.push(warning_message(
                    "config_reload_workspace_failed",
                    &format!("config: failed to refresh workspace config: {error}"),
                ));
            }
        }

        for package in program.packages.iter() {
            let package_guard = package.read();
            let package_id = package_guard.id;
            let package_path = package_guard.path.clone();
            drop(package_guard);

            let Some(package_path) = package_path else {
                continue;
            };

            let next_dsconfig = match resolver.load_dsconfig(&package_path, CachePolicy::Reload) {
                Ok(dsconfig) => Some(dsconfig),
                Err(ResolveError::DsConfigNotFound { .. }) => None,
                Err(error) => {
                    messages.push(warning_message(
                        "config_reload_package_failed",
                        &format!(
                            "config: failed to refresh package {}: {error}",
                            package_path.display()
                        ),
                    ));
                    continue;
                }
            };

            let package = program.packages.get(package_id);
            let mut package = package.write();
            package.dsconfig = next_dsconfig.clone();
            package.targets.clear();
            if let Some(dsconfig) = next_dsconfig {
                for (name, options) in dsconfig.options.targets.iter() {
                    let target = options.to_target(name);
                    let target_id = TargetId::new(package_id, name);
                    package.targets.insert(target_id, target);
                }
            }
        }

        let mut tsconfig_paths = Vec::new();
        for tsconfig in program.tsconfigs.iter() {
            tsconfig_paths.push(tsconfig.read().path.clone());
        }
        for path in tsconfig_paths {
            if let Err(error) = resolver.reload_tsconfig(&path) {
                if matches!(error, ResolveError::TsConfigNotFound { .. }) {
                    continue;
                }
                messages.push(warning_message(
                    "config_reload_tsconfig_failed",
                    &format!(
                        "config: failed to refresh tsconfig {}: {error}",
                        path.display()
                    ),
                ));
            }
        }

        let mut module_updates = Vec::new();
        for module in program.modules.iter() {
            let module = module.read();
            let Some(path) = module.path.as_ref() else {
                continue;
            };
            let next_tsconfig = resolver.find_tsconfig(path);
            if module.tsconfig_id != next_tsconfig {
                module_updates.push((module.id, next_tsconfig));
            }
        }

        for (module_id, tsconfig_id) in module_updates {
            let module = program.modules.get(module_id);
            let mut module = module.write();
            module.tsconfig_id = tsconfig_id;
        }

        messages
    }

    /// Check if a path should be handled by watch mode.
    fn is_watchable_path(&self, path: &Path) -> bool {
        let Some(file_type) = FileType::from_path(path) else {
            return false;
        };

        file_type.is_code() || file_type.is_data() || file_type.is_text()
    }

    /// Return true when a module is part of the user workspace surface.
    fn is_workspace_module(&self, module: &Module) -> bool {
        module.is_user() && module.path.is_some()
    }

    /// Return true when the module id maps to a workspace module.
    fn is_workspace_module_id(&self, program: &Program, module_id: ModuleId) -> bool {
        let module = program.modules.get(module_id);
        let module = module.read();
        self.is_workspace_module(&module)
    }

    /// Check if a path is a config filename.
    fn is_config_filename(&self, path: &Path) -> bool {
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            return false;
        };

        if file_name == "dsconfig.json" {
            return true;
        }

        file_name.starts_with("tsconfig") && file_name.ends_with(".json")
    }

    /// Check if a config file is already tracked by this program.
    fn is_tracked_config_path(&self, program: &Program, path: &Path) -> bool {
        if let Some(workspace_config) = self.session.workspace_config()
            && workspace_config.path.as_path() == path
        {
            return true;
        }

        for package in program.packages.iter() {
            let package = package.read();
            let Some(dsconfig) = package.dsconfig.as_ref() else {
                continue;
            };
            if dsconfig.path.as_path() == path {
                return true;
            }
        }

        for tsconfig in program.tsconfigs.iter() {
            let tsconfig = tsconfig.read();
            if tsconfig.path.as_path() == path {
                return true;
            }
        }

        false
    }

    /// Check whether a config refresh is required for an invalidation.
    fn should_refresh_configs(
        &self,
        program: &Program,
        invalidation: &InvalidationPlan,
        path: &Path,
    ) -> bool {
        if invalidation.kinds.iter().any(|kind| {
            matches!(
                kind,
                InvalidationKind::DsConfig | InvalidationKind::TsConfig
            )
        }) {
            return true;
        }

        self.is_tracked_config_path(program, path)
    }

    /// Build a query response for a request payload.
    fn query_response_for_request(
        &self,
        program: &Program,
        request: query::QueryRequest,
    ) -> Result<query::QueryResponse, String> {
        let session = self.session.as_ref();

        let response = match request {
            query::QueryRequest::Completion(params) => {
                let file_id = self.resolve_file_id(program, &params.uri);
                let mut items = match file_id {
                    Some(file_id) => {
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
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
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
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
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
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
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
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
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
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
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
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
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
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
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
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
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
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
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
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
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
                        query::document_highlight(session, file_id, params.offset)
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
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
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
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
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
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
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
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
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
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
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
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
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
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
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
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
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
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
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
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
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
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
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
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
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
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
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
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
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
                        let file_id = self.ensure_query_context(program, file_id, &params.uri)?;
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

        if let Some(file_id) = program.files.get_id_by_uri(uri) {
            return Some(file_id);
        }

        let path = uri.to_path_buf()?;
        program.files.get_id_by_path(&path)
    }

    /// Ensure query context is ready for a file.
    fn ensure_query_context(
        &self,
        program: &Program,
        file_id: FileId,
        uri: &Uri,
    ) -> Result<FileId, String> {
        if self.query_context_ready(file_id) {
            return Ok(file_id);
        }

        let mut validate_outcome = None;
        let session = self.session.as_ref();

        if let Some(module_id) = session.modules.get_id_by_file_id(file_id) {
            let profile_id = session.default_profile_for_module(module_id);
            let handle = self.program_handle_for_root(&program.cwd);
            let _compile_guard = handle.compile_lock.lock();

            let module = handle.compiler.module_stamp(module_id);
            let profile = handle.compiler.profile_stamp(profile_id);
            let analyze_task = AnalyzeTask::AnalyzeModuleValidate { module, profile };
            validate_outcome = Some(handle.compiler.run_task(analyze_task));
            if self.query_context_ready(file_id) {
                return Ok(file_id);
            }

            if let Some(resolved_file_id) = self.resolve_file_id(program, uri)
                && self.query_context_ready(resolved_file_id)
            {
                return Ok(resolved_file_id);
            }
        }

        let path = program
            .files
            .get_maybe(file_id)
            .and_then(|file| file.path.clone().or_else(|| file.uri.to_path_buf()))
            .or_else(|| uri.to_path_buf());

        let Some(path) = path else {
            return Err("query context is not ready for the requested uri".to_string());
        };

        let analyze_result = self.analyze_path(&path)?;

        if !analyze_result.query_context_ready {
            let detail = analyze_result
                .detail
                .unwrap_or_else(|| "analysis did not produce a query context".to_string());
            return Err(detail);
        }

        if self.query_context_ready(file_id) {
            return Ok(file_id);
        }

        if let Some(resolved_file_id) = self.resolve_file_id(program, uri)
            && self.query_context_ready(resolved_file_id)
        {
            return Ok(resolved_file_id);
        }

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
                    ModuleContent::Code(code) => {
                        code.dirs.iter().filter_map(|dir| dir.profile_id).collect()
                    }
                    ModuleContent::Data { dirs, .. }
                    | ModuleContent::Text { dirs, .. }
                    | ModuleContent::Binary { dirs, .. } => {
                        dirs.iter().filter_map(|dir| dir.profile_id).collect()
                    }
                    ModuleContent::Unloaded => Vec::new(),
                };
                let path = module
                    .path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "<none>".to_string());
                format!(
                    "file_id={file_id:?} module_id={:?} profile_id={:?} ast_ready={ast_ready} base_dir_ready={base_dir_ready} dir_ready={dir_ready} dir_profiles={dir_profiles:?} validate_outcome={validate_outcome:?} analyze_result={analyze_result:?} path={path}",
                    module.id,
                    profile_id
                )
            })
            .unwrap_or_else(|| format!("file_id={file_id:?} module_id=<missing>"));

        Err(detail)
    }

    /// Check if query context is ready for the file.
    fn query_context_ready(&self, file_id: FileId) -> bool {
        let session = self.session.as_ref();
        let Some(module) = session.modules.get_by_file_id(file_id) else {
            return false;
        };

        let module = module.read();
        let profile = session.default_profile_for_module(module.id);
        module.ast_maybe().is_some() && module.dir_maybe(profile).is_some()
    }

    /// Build a span from offsets for a file.
    fn span_for_offsets(&self, file_id: FileId, start: u32, end: u32) -> Span {
        let range_start = start.min(end);
        let range_end = start.max(end);
        Span::new(file_id, range_start, range_end)
    }
}

/// Outcome of an explicit analyze request.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AnalyzeOutcome {
    /// Whether query context is ready for the analyzed module.
    query_context_ready: bool,
    /// Optional detail when query context is not ready.
    detail: Option<String>,
}

/// Build an internal update from program state.
fn build_update(
    program: &Program,
    module_id: Option<ModuleId>,
    file_id: FileId,
    invalidation: InvalidationPlan,
) -> Result<DriverUpdate, String> {
    let file = file_snapshot_for_id(program, file_id)?;
    Ok(DriverUpdate {
        module_id,
        file_id,
        file,
        invalidation,
        diagnostics: Vec::new(),
    })
}

/// Build a file snapshot for a program file id.
fn file_snapshot_for_id(program: &Program, file_id: FileId) -> Result<FileSnapshot, String> {
    let file = program
        .files
        .get_maybe(file_id)
        .ok_or_else(|| format!("file not tracked for id: {file_id:?}"))?;

    Ok(file_snapshot_from_file(&file))
}

/// Build a file snapshot payload.
fn file_snapshot_from_file(file: &File) -> FileSnapshot {
    let content = match &file.content {
        FileContent::Text { content } => Some(content.clone()),
        FileContent::Json { content, .. } => Some(content.clone()),
        FileContent::Binary { .. } => None,
        FileContent::Missing => None,
        FileContent::Unloaded => None,
    };

    FileSnapshot {
        id: file.id,
        name: file.name.clone(),
        uri: file.uri.clone(),
        path: file.path.clone(),
        file_type: file.ty,
        content,
    }
}

/// Build a warning message record.
fn warning_message(code: &str, message: &str) -> WorkspaceMessageRecord {
    WorkspaceMessageRecord {
        kind: WorkspaceMessageKind::Warning,
        code: code.to_string(),
        message: message.to_string(),
    }
}
