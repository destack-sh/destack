use std::collections::{HashMap, HashSet, VecDeque};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::mapref::entry::Entry;
use destack_compiler::{AnalyzeTask, Compiler, ResolveTask};
use destack_resolver::{CachePolicy, ResolveError, ResolveOptions, Resolver};
use destack_source::{
    Diagnostic, DiagnosticStoreUpdate, File, FileContent, FileId, FileType, FileWatchEvent,
    FileWatchEventKind, FileWatchRescanReason, FileWatchStatus, ModuleId, ModuleStamp,
};
use destack_workspace::{
    FileUpdate, InvalidationKind, InvalidationPlan, Module, ModuleGraphKey, Program, TargetId,
};
use parking_lot::Mutex;

use crate::protocol::FileSnapshot;
use crate::{
    AnalyzeOutcome, DaemonError, DaemonMessage, DaemonRescanResult, DaemonUpdate,
    DaemonWatchBatchResult, DaemonWatchEventResult,
};

use super::Daemon;
use super::program::ProgramHandle;

impl Daemon {
    /// Apply a text update and reanalyze the owning module.
    pub fn update_file(
        &self,
        path: &Path,
        content: String,
    ) -> Result<Vec<DaemonUpdate>, DaemonError> {
        self.apply_file_update(path, FileUpdate::Text { content }, true)
    }

    /// Apply a text update without writing to the filesystem.
    pub fn update_virtual_file(
        &self,
        path: &Path,
        content: String,
    ) -> Result<Vec<DaemonUpdate>, DaemonError> {
        self.apply_virtual_file_update(path, FileUpdate::Text { content })
    }

    /// Apply a watch event through the daemon.
    pub fn apply_watch_event(&self, event: &FileWatchEvent) -> DaemonWatchEventResult {
        // handle overflow events by forcing a rescan
        if matches!(event.kind, FileWatchEventKind::Overflow) {
            return DaemonWatchEventResult {
                updates: Vec::new(),
                rescan: true,
                messages: vec![DaemonMessage::WatchOverflowRescan],
            };
        }

        let requires_rescan = self.config_change_requires_rescan(&event.path);

        // handle delete by marking the file missing
        if matches!(event.kind, FileWatchEventKind::Deleted) {
            // guard on non watchable deletes and config rescans
            if !self.is_watchable_path(&event.path) {
                return DaemonWatchEventResult {
                    updates: Vec::new(),
                    rescan: requires_rescan,
                    messages: Vec::new(),
                };
            }
            if requires_rescan {
                return DaemonWatchEventResult {
                    updates: Vec::new(),
                    rescan: true,
                    messages: Vec::new(),
                };
            }

            // apply delete invalidation
            let updates = match self.remove_virtual_file(&event.path) {
                Ok(updates) => updates,
                Err(error) => {
                    return DaemonWatchEventResult {
                        updates: Vec::new(),
                        rescan: requires_rescan,
                        messages: vec![DaemonMessage::WatchRemoveFailed {
                            path: event.path.clone(),
                            error: error.to_string(),
                        }],
                    };
                }
            };

            return DaemonWatchEventResult {
                updates,
                rescan: requires_rescan,
                messages: Vec::new(),
            };
        }

        // handle rename as a delete plus create
        if matches!(event.kind, FileWatchEventKind::Renamed) {
            // track rename updates and rescan state
            let mut requires_rescan = false;
            let mut updates = Vec::new();
            let mut messages = Vec::new();
            let new_requires_rescan = self.config_change_requires_rescan(&event.path);

            // remove the previous path while tracking rescan state
            if let Some(previous) = event.previous_path.as_ref() {
                requires_rescan |= self.config_change_requires_rescan(previous);
                if self.is_watchable_path(previous) {
                    match self.remove_virtual_file(previous) {
                        Ok(mut previous_updates) => updates.append(&mut previous_updates),
                        Err(error) => {
                            messages.push(DaemonMessage::WatchRemoveFailed {
                                path: previous.clone(),
                                error: error.to_string(),
                            });
                        }
                    }
                }
            } else {
                requires_rescan = true;
            }

            // apply the new path when watchable
            requires_rescan |= new_requires_rescan;
            if self.is_watchable_path(&event.path) && !new_requires_rescan {
                let content = match self.session.fs.read_to_string(&event.path) {
                    Ok(content) => content,
                    Err(error) => {
                        messages.push(DaemonMessage::WatchReadFailed {
                            path: event.path.clone(),
                            error: error.to_string(),
                        });
                        return DaemonWatchEventResult {
                            updates,
                            rescan: requires_rescan,
                            messages,
                        };
                    }
                };

                match self.update_virtual_file(&event.path, content) {
                    Ok(mut new_updates) => updates.append(&mut new_updates),
                    Err(error) => {
                        if self.error_requires_rescan(&error) {
                            requires_rescan = true;
                        }
                        messages.push(DaemonMessage::WatchUpdateFailed {
                            path: event.path.clone(),
                            error: error.to_string(),
                        });
                    }
                }
            }

            return DaemonWatchEventResult {
                updates,
                rescan: requires_rescan,
                messages,
            };
        }

        // guard on non watchable paths and config rescans
        if !self.is_watchable_path(&event.path) {
            return DaemonWatchEventResult {
                updates: Vec::new(),
                rescan: requires_rescan,
                messages: Vec::new(),
            };
        }
        if requires_rescan {
            return DaemonWatchEventResult {
                updates: Vec::new(),
                rescan: true,
                messages: Vec::new(),
            };
        }

        // read the updated content from disk
        let content = match self.session.fs.read_to_string(&event.path) {
            Ok(content) => content,
            Err(error) => {
                return DaemonWatchEventResult {
                    updates: Vec::new(),
                    rescan: requires_rescan,
                    messages: vec![DaemonMessage::WatchReadFailed {
                        path: event.path.clone(),
                        error: error.to_string(),
                    }],
                };
            }
        };

        // update via the daemon
        let updates = match self.update_virtual_file(&event.path, content) {
            Ok(updates) => updates,
            Err(error) => {
                let rescan = requires_rescan || self.error_requires_rescan(&error);
                return DaemonWatchEventResult {
                    updates: Vec::new(),
                    rescan,
                    messages: vec![DaemonMessage::WatchUpdateFailed {
                        path: event.path.clone(),
                        error: error.to_string(),
                    }],
                };
            }
        };

        DaemonWatchEventResult {
            updates,
            rescan: requires_rescan,
            messages: Vec::new(),
        }
    }

    /// Apply a watch batch through the daemon.
    pub fn apply_watch_batch(&self, batch: &crate::WatchBatch) -> DaemonWatchBatchResult {
        // track results from the batch
        let mut result = DaemonWatchBatchResult::default();

        // emit rescan requests for overflow batches without explicit events
        let has_overflow_event = batch
            .events
            .iter()
            .any(|event| matches!(event.kind, FileWatchEventKind::Overflow));
        if batch.overflowed && !has_overflow_event {
            result.rescan = true;
            result.messages.push(DaemonMessage::WatchOverflowRescan);
        }

        // handle status updates
        for status in &batch.status {
            let status_result = self.handle_watch_status(status);
            if let Some(rescan) = status_result.rescan {
                result.rescan = result.rescan || rescan;
            }
            if let Some(message) = status_result.message {
                result.messages.push(message);
            }
        }

        // apply event updates
        for event in &batch.events {
            let event_result = self.apply_watch_event(event);
            result.rescan = result.rescan || event_result.rescan;
            result.updates.extend(event_result.updates);
            result.messages.extend(event_result.messages);
        }

        result
    }

    /// Rescan tracked files for the provided roots.
    pub fn rescan_roots(&self, roots: &[PathBuf]) -> DaemonRescanResult {
        self.rescan_roots_with_mode(roots, false)
    }

    /// Rescan tracked files and analyze updated modules.
    pub fn rescan_roots_with_analysis(&self, roots: &[PathBuf]) -> DaemonRescanResult {
        self.rescan_roots_with_mode(roots, true)
    }

    /// Rescan tracked files for the provided roots and analyze updated modules.
    fn rescan_roots_with_mode(&self, roots: &[PathBuf], analyze: bool) -> DaemonRescanResult {
        // collect rescan results across roots
        let mut result = DaemonRescanResult::default();
        let mut visited = HashSet::new();

        for root in roots {
            let handle = self.program_handle_for_path(root);
            let _compile_guard = handle.compile_lock.lock();
            let key = handle.program.cwd.clone();
            if !visited.insert(key) {
                continue;
            }

            let root_result = self.rescan_program(&handle, analyze);
            result.updates.extend(root_result.updates);
            result.messages.extend(root_result.messages);
        }

        result
    }

    /// Refresh tracked files for a single program.
    fn rescan_program(&self, handle: &ProgramHandle, analyze: bool) -> DaemonRescanResult {
        // unpack the program handle
        let program = handle.program.as_ref();
        let compiler = handle.compiler.as_ref();

        // collect user module file ids
        let mut file_ids = HashSet::new();
        for module in program.modules.iter() {
            let module = module.read();
            if !self.is_workspace_module(&module) {
                continue;
            }
            file_ids.insert(module.file_id);
        }

        // collect workspace config file ids
        if let Some(workspace_config) = self.session.workspace_config() {
            file_ids.insert(workspace_config.file_id);
        }

        // collect package dsconfig file ids
        for package in program.packages.iter() {
            let package = package.read();
            if let Some(dsconfig) = package.dsconfig.as_ref() {
                file_ids.insert(dsconfig.file_id);
            }
        }

        // collect tsconfig file ids
        for tsconfig in program.tsconfigs.iter() {
            let tsconfig = tsconfig.read();
            file_ids.insert(tsconfig.file_id);
        }

        // refresh files from disk
        let mut result = DaemonRescanResult::default();
        for file_id in file_ids {
            let update = match self.rescan_file_update(program, file_id) {
                Ok(Some(update)) => update,
                Ok(None) => continue,
                Err(message) => {
                    result.messages.push(message);
                    continue;
                }
            };

            let invalidation = match program.invalidate_file(file_id, update) {
                Ok(invalidation) => invalidation,
                Err(error) => {
                    result
                        .messages
                        .push(DaemonMessage::RescanInvalidationFailed {
                            file_id,
                            error: error.to_string(),
                        });
                    continue;
                }
            };

            let module_id = program.modules.get_id_by_file_id(file_id);
            let update = match build_update(program, module_id, file_id, invalidation) {
                Ok(update) => update,
                Err(error) => {
                    result
                        .messages
                        .push(DaemonMessage::RescanInvalidationFailed {
                            file_id,
                            error: error.to_string(),
                        });
                    continue;
                }
            };
            result.updates.push(update);
        }

        // refresh configs after a rescan to pick up new configuration files
        let resolver = Resolver::from_program(program, ResolveOptions::default());
        result
            .messages
            .extend(self.refresh_program_configs(&resolver, program));

        // analyze updated modules when requested
        if analyze && let Err(error) = self.analyze_updates(program, compiler, &mut result.updates)
        {
            result.messages.push(DaemonMessage::RescanAnalyzeFailed {
                error: error.to_string(),
            });
        }

        result
    }

    /// Mark a file as removed without touching the filesystem.
    pub fn remove_virtual_file(&self, path: &Path) -> Result<Vec<DaemonUpdate>, DaemonError> {
        self.apply_virtual_file_update(path, FileUpdate::Removed)
    }

    /// Apply a file update and optionally write to the filesystem.
    pub fn apply_file_update(
        &self,
        path: &Path,
        update: FileUpdate,
        write_to_disk: bool,
    ) -> Result<Vec<DaemonUpdate>, DaemonError> {
        // write the update to disk when requested
        if write_to_disk {
            self.write_update_to_disk(path, &update)?;
        }

        self.apply_virtual_file_update(path, update)
    }

    /// Return the compiler for a workspace root.
    pub(crate) fn compiler_for_root(&self, root: &Path) -> Arc<Compiler> {
        let handle = self.program_handle_for_path(root);
        handle.compiler.clone()
    }

    /// Apply a file update without writing to the filesystem.
    fn apply_virtual_file_update(
        &self,
        path: &Path,
        update: FileUpdate,
    ) -> Result<Vec<DaemonUpdate>, DaemonError> {
        // locate daemon handle for the path
        let handle = self.program_handle_for_path(path);
        let _compile_guard = handle.compile_lock.lock();
        let program = handle.program.clone();
        let compiler = handle.compiler.clone();

        // resolve config file ids before registering modules
        let config_file_id = if self.is_config_filename(path) {
            let mut file_id = program.files.get_id_by_path(path);
            if file_id.is_none() {
                // load config files to preserve stable file ids
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

        // resolve module id for tracked paths
        let module_id = if config_file_id.is_some() {
            None
        } else {
            match compiler.resolve_path_to_module(&path.to_path_buf()) {
                Ok(module_id) => Some(module_id),
                Err(error) => {
                    if program.files.get_id_by_path(path).is_some() {
                        None
                    } else {
                        return Err(DaemonError::Resolve {
                            path: path.to_path_buf(),
                            error: Box::new(error),
                        });
                    }
                }
            }
        };

        // resolve file id for invalidation
        let file_id = if let Some(file_id) = config_file_id {
            file_id
        } else {
            match module_id {
                Some(module_id) => program.modules.get(module_id).read().file_id,
                None => match program.files.get_id_by_path(path) {
                    Some(file_id) => file_id,
                    None => {
                        return Err(DaemonError::FileNotTracked {
                            path: path.to_path_buf(),
                        });
                    }
                },
            }
        };
        let invalidation = program.invalidate_file(file_id, update).map_err(|error| {
            DaemonError::Invalidation {
                path: path.to_path_buf(),
                error: Box::new(error),
            }
        })?;

        // refresh configs when config files change
        if self.should_refresh_configs(&program, &invalidation, path) {
            let resolver = Resolver::from_program(&program, ResolveOptions::default());
            for message in self.refresh_program_configs(&resolver, &program) {
                tracing::warn!(code = message.code(), message = %message, "daemon.config.refresh_failed");
            }
        }

        let mut updates = Vec::new();
        let update = build_update(&program, module_id, file_id, invalidation)?;
        updates.push(update);

        // analyze the updated module when available
        self.analyze_updates(&program, &compiler, &mut updates)?;

        Ok(updates)
    }

    /// Write a file update to disk before applying it.
    fn write_update_to_disk(&self, path: &Path, update: &FileUpdate) -> Result<(), DaemonError> {
        let parent = path.parent();
        if let Some(parent) = parent {
            self.session
                .fs
                .create_dir_all(parent)
                .map_err(|error| DaemonError::FileWrite {
                    path: parent.to_path_buf(),
                    error,
                })?;
        }

        match update {
            FileUpdate::Text { content } => {
                self.session
                    .fs
                    .write_string(path, content)
                    .map_err(|error| DaemonError::FileWrite {
                        path: path.to_path_buf(),
                        error,
                    })?;
            }
            FileUpdate::Bytes { content } => {
                self.session
                    .fs
                    .write(path, content)
                    .map_err(|error| DaemonError::FileWrite {
                        path: path.to_path_buf(),
                        error,
                    })?;
            }
            FileUpdate::Removed => {
                if let Err(error) = self.session.fs.remove_file(path)
                    && error.kind() != io::ErrorKind::NotFound
                {
                    return Err(DaemonError::FileWrite {
                        path: path.to_path_buf(),
                        error,
                    });
                }
            }
            FileUpdate::Touch => {}
        }

        Ok(())
    }

    /// Ensure a module for the given path is analyzed.
    pub fn analyze_path(&self, path: &Path) -> Result<AnalyzeOutcome, DaemonError> {
        // locate daemon handle for the path
        let handle = self.program_handle_for_path(path);
        let _compile_guard = handle.compile_lock.lock();
        let program = handle.program.clone();
        let compiler = handle.compiler.clone();

        // resolve the module id for the path
        let module_id = compiler
            .resolve_path_to_module(&path.to_path_buf())
            .map_err(|error| DaemonError::Resolve {
                path: path.to_path_buf(),
                error: Box::new(error),
            })?;

        // clear diagnostics collector before recompiling
        let _ = program.diagnostics.drain();

        // enqueue analysis task and compile
        let profile_id = program.default_profile_id_for_module(module_id);
        let module = compiler.module_stamp(module_id);
        let profile = compiler.profile_stamp(profile_id);
        compiler.enqueue(AnalyzeTask::AnalyzeModule { module, profile });
        compiler.compile();

        // group diagnostics by file id
        let diagnostics = program.diagnostics.collect();
        let mut diagnostics_by_file: HashMap<FileId, Vec<Diagnostic>> = HashMap::new();
        for diagnostic in diagnostics.iter() {
            diagnostics_by_file
                .entry(diagnostic.file_id)
                .or_default()
                .push(diagnostic);
        }

        // collect file versions for the module and diagnostics
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
                "file_id={module_file_id:?} module_id={module_id:?} profile_id={profile_id:?} ast_ready={ast_ready} dir_ready={dir_ready} path={module_path}"
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
                .ok_or(DaemonError::FileIdNotTracked { file_id })?;
            store_updates.push(DiagnosticStoreUpdate::new(
                file_id,
                file.version,
                diagnostics,
            ));
        }

        // commit diagnostics to the store
        program.diagnostic_store.apply_updates(store_updates);

        Ok(AnalyzeOutcome {
            query_context_ready,
            detail,
        })
    }

    /// Analyze a collection of updates and attach diagnostics.
    fn analyze_updates(
        &self,
        program: &Program,
        compiler: &Compiler,
        updates: &mut Vec<DaemonUpdate>,
    ) -> Result<(), DaemonError> {
        // collect modules and file ids that require analysis
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

        // add invalidated modules as diagnostic updates
        let mut extra_updates = Vec::new();
        for update in updates.iter() {
            for module_id in update.invalidation.modules.iter().copied() {
                if !self.is_workspace_module_id(program, module_id) {
                    continue;
                }

                module_ids.insert(module_id);
                let module = program.modules.get(module_id);
                let module = module.read();
                let file_id = module.file_id;
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

        // clear diagnostics before any compilation work
        if !module_ids.is_empty() {
            let _ = program.diagnostics.drain();
        }

        // rebuild missing or stale module graph entries before dependent fanout
        self.ensure_module_graphs_ready(program, compiler, &module_ids);

        // expand updates using module graph dependents when available
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

        // compile modules when analysis is required
        if !module_ids.is_empty() {
            // enqueue analysis tasks
            for module_id in &module_ids {
                let profile = program.default_profile_id_for_module(*module_id);
                let module = compiler.module_stamp(*module_id);
                let profile = compiler.profile_stamp(profile);
                compiler.enqueue(AnalyzeTask::AnalyzeModule { module, profile });
            }

            // run the analysis pass
            compiler.compile();

            // attach diagnostics for each update
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

        // build store updates for changed files
        let mut store_updates = Vec::new();
        for update in updates.iter() {
            let file =
                program
                    .files
                    .get_maybe(update.file_id)
                    .ok_or(DaemonError::FileIdNotTracked {
                        file_id: update.file_id,
                    })?;
            store_updates.push(DiagnosticStoreUpdate::new(
                update.file_id,
                file.version,
                update.diagnostics.clone(),
            ));
        }

        // commit diagnostics to the store
        program.diagnostic_store.apply_updates(store_updates);

        Ok(())
    }

    /// Ensure module graph entries exist for the modules used in fanout.
    /// TODO #Cleanup #Architecture:  daemon's ensure_module_graphs_ready seems partially redundant?
    fn ensure_module_graphs_ready(
        &self,
        program: &Program,
        compiler: &Compiler,
        module_ids: &HashSet<ModuleId>,
    ) {
        // skip when there is no module work
        if module_ids.is_empty() {
            return;
        }

        // collect canonical resolve tasks needed to seed graph entries
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

            // the graph is missing: seed it from user modules in this profile
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

        // run graph rebuild tasks
        if resolve_tasks.is_empty() {
            return;
        }
        for task in resolve_tasks {
            compiler.enqueue(task);
        }
        compiler.compile();
    }

    /// Refresh package and workspace configuration for a program.
    fn refresh_program_configs(
        &self,
        resolver: &Resolver,
        program: &Program,
    ) -> Vec<DaemonMessage> {
        // collect refresh warnings
        let mut messages = Vec::new();

        // refresh workspace config
        let workspace_root = self.session.workspace_root();
        match resolver.load_dsconfig(&workspace_root, CachePolicy::Reload) {
            Ok(dsconfig) => {
                self.session.update_workspace_config(Some(dsconfig));
            }
            Err(ResolveError::DsConfigNotFound { .. }) => {
                self.session.update_workspace_config(None);
            }
            Err(error) => {
                messages.push(DaemonMessage::ConfigReloadWorkspaceFailed {
                    error: error.to_string(),
                });
            }
        }

        // refresh package configs
        for package in program.packages.iter() {
            let package_guard = package.read();
            let package_id = package_guard.id;
            let package_path = package_guard.path.clone();
            drop(package_guard);

            // skip packages without roots
            let Some(package_path) = package_path else {
                continue;
            };

            // resolve the dsconfig for this package
            let next_dsconfig = match resolver.load_dsconfig(&package_path, CachePolicy::Reload) {
                Ok(dsconfig) => Some(dsconfig),
                Err(ResolveError::DsConfigNotFound { .. }) => None,
                Err(error) => {
                    messages.push(DaemonMessage::ConfigReloadPackageFailed {
                        path: package_path.clone(),
                        error: error.to_string(),
                    });
                    continue;
                }
            };

            // update package config and targets
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

        // refresh tsconfig entries
        let mut tsconfig_paths = Vec::new();
        for tsconfig in program.tsconfigs.iter() {
            tsconfig_paths.push(tsconfig.read().path.clone());
        }
        for path in tsconfig_paths {
            if let Err(error) = resolver.reload_tsconfig(&path) {
                if matches!(error, ResolveError::TsConfigNotFound { .. }) {
                    continue;
                }
                messages.push(DaemonMessage::ConfigReloadTsconfigFailed {
                    path,
                    error: error.to_string(),
                });
            }
        }

        // refresh module tsconfig assignments
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

        // apply module tsconfig updates
        for (module_id, tsconfig_id) in module_updates {
            let module = program.modules.get(module_id);
            let mut module = module.write();
            module.tsconfig_id = tsconfig_id;
        }

        messages
    }

    /// Resolve the daemon program handle for a path.
    pub(crate) fn program_handle_for_path(&self, path: &Path) -> Arc<ProgramHandle> {
        // resolve the program first
        let program = self.session.find_program_for_path(path);
        let key = program.cwd.clone();

        // return existing handle when present
        match self.program_handles.entry(key) {
            Entry::Occupied(entry) => {
                // reuse the existing handle
                Arc::clone(entry.get())
            }
            Entry::Vacant(entry) => {
                // build a new compiler for the program
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

    /// Build a file update by reading the latest content from disk.
    fn rescan_file_update(
        &self,
        program: &Program,
        file_id: FileId,
    ) -> Result<Option<FileUpdate>, DaemonMessage> {
        // load the tracked file
        let Some(file) = program.files.get_maybe(file_id) else {
            return Ok(None);
        };

        // resolve the file path
        let Some(path) = file.path.clone().or_else(|| file.uri.to_path_buf()) else {
            return Ok(None);
        };

        // read file contents from disk and decide whether to update
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
    ) -> Result<Option<FileUpdate>, DaemonMessage> {
        if error.kind() == io::ErrorKind::NotFound {
            if matches!(file.content, FileContent::Missing) {
                return Ok(None);
            }

            return Ok(Some(FileUpdate::Removed));
        }

        Err(DaemonMessage::RescanReadFailed {
            path: path.to_path_buf(),
            error: error.to_string(),
        })
    }

    /// Check if a path should be handled by watch mode.
    fn is_watchable_path(&self, path: &Path) -> bool {
        // skip unknown or non file paths
        let Some(file_type) = FileType::from_path(path) else {
            return false;
        };

        // allow code, data, and text files
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
        // detect dsconfig.json or tsconfig json variants
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            return false;
        };

        // dsconfig.json
        if file_name == "dsconfig.json" {
            return true;
        }

        // tsconfig*.json
        file_name.starts_with("tsconfig") && file_name.ends_with(".json")
    }

    /// Check if a config file is already tracked by this program.
    fn is_tracked_config_path(&self, program: &Program, path: &Path) -> bool {
        if let Some(workspace_config) = self.session.workspace_config()
            && workspace_config.path.as_path() == path
        {
            return true;
        }

        // dsconfig.json in packages
        for package in program.packages.iter() {
            let package = package.read();
            let Some(dsconfig) = package.dsconfig.as_ref() else {
                continue;
            };
            if dsconfig.path.as_path() == path {
                return true;
            }
        }

        // tsconfig*.json in packages
        for tsconfig in program.tsconfigs.iter() {
            let tsconfig = tsconfig.read();
            if tsconfig.path.as_path() == path {
                return true;
            }
        }

        false
    }

    /// Return true when a config change requires a full rescan.
    fn config_change_requires_rescan(&self, path: &Path) -> bool {
        if !self.is_config_filename(path) {
            return false;
        }

        let program = self.session.find_program_for_path(path);
        !self.is_tracked_config_path(&program, path)
    }

    /// Check whether a config refresh is required for an invalidation.
    fn should_refresh_configs(
        &self,
        program: &Program,
        invalidation: &InvalidationPlan,
        path: &Path,
    ) -> bool {
        // refresh when invalidation kinds include config changes
        if invalidation.kinds.iter().any(|kind| {
            matches!(
                kind,
                InvalidationKind::DsConfig | InvalidationKind::TsConfig
            )
        }) {
            return true;
        }

        // refresh when the path is a tracked config
        self.is_tracked_config_path(program, path)
    }

    /// Check whether a daemon error should force a rescan.
    fn error_requires_rescan(&self, error: &DaemonError) -> bool {
        matches!(
            error,
            DaemonError::FileNotTracked { .. }
                | DaemonError::FileIdNotTracked { .. }
                | DaemonError::Resolve { .. }
        )
    }

    /// Handle watch status updates and report whether a rescan is required.
    fn handle_watch_status(&self, status: &FileWatchStatus) -> WatchStatusResult {
        // report watcher errors
        if let FileWatchStatus::Error { message } = status {
            return WatchStatusResult {
                rescan: Some(false),
                message: Some(DaemonMessage::WatchStatusError {
                    message: message.to_string(),
                }),
            };
        }

        // ignore startup rescan because the initial compile already ran
        if let FileWatchStatus::RescanRequested { reason, .. } = status
            && matches!(reason, FileWatchRescanReason::Startup)
        {
            return WatchStatusResult {
                rescan: Some(false),
                message: None,
            };
        }

        // report rescan requests that require a full compile
        if let FileWatchStatus::RescanRequested { reason, .. } = status {
            return WatchStatusResult {
                rescan: Some(true),
                message: Some(DaemonMessage::WatchRescanRequested {
                    reason: reason.clone(),
                }),
            };
        }

        WatchStatusResult::default()
    }
}

/// Result from handling a watch status update.
#[derive(Debug, Default)]
struct WatchStatusResult {
    /// Optional rescan flag.
    rescan: Option<bool>,
    /// Optional message payload.
    message: Option<DaemonMessage>,
}

/// Build a daemon update from program state and invalidation metadata.
fn build_update(
    program: &Program,
    module_id: Option<ModuleId>,
    file_id: FileId,
    invalidation: InvalidationPlan,
) -> Result<DaemonUpdate, DaemonError> {
    // resolve the file snapshot for the update
    let file = file_snapshot_for_id(program, file_id)?;

    Ok(DaemonUpdate {
        module_id,
        file_id,
        file,
        invalidation,
        diagnostics: Vec::new(),
    })
}

/// Build a file snapshot for a program file id.
fn file_snapshot_for_id(program: &Program, file_id: FileId) -> Result<FileSnapshot, DaemonError> {
    // resolve the file from the registry
    let file = program
        .files
        .get_maybe(file_id)
        .ok_or(DaemonError::FileIdNotTracked { file_id })?;

    Ok(file_snapshot_from_file(&file))
}

/// Build a file snapshot for protocol updates.
fn file_snapshot_from_file(file: &File) -> FileSnapshot {
    // capture the file content when available
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
