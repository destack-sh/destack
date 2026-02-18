use std::path::{Path, PathBuf};

use destack_resolver::{CachePolicy, ResolveOptions, Resolver};
use destack_source::{FileType, FileWatchEvent, FileWatchEventKind};
use destack_workspace::FileUpdate;

use super::workspace::{
    ProgramHandle, ServiceUpdate, build_update, warning_message, workspace_update_record,
};
use super::{
    LanguageService, LanguageServiceError, LanguageServiceResult, RescanReason, WorkspaceMessage,
};

/// Result of a low level virtual update.
struct VirtualUpdateResult {
    /// Internal update records.
    updates: Vec<ServiceUpdate>,
    /// Messages emitted during update processing.
    messages: Vec<WorkspaceMessage>,
}

impl LanguageService {
    /// Apply a virtual file update through the service.
    pub fn update_virtual_file(
        &self,
        path: &Path,
        content: String,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        self.apply_virtual_update(path, FileUpdate::Text { content })
    }

    /// Apply an arbitrary virtual file update through the service.
    pub fn apply_virtual_update(
        &self,
        path: &Path,
        update: FileUpdate,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        // apply the low level update
        let update_result = self.apply_virtual_file_update(path, update)?;

        // map internal updates to public records
        Ok(LanguageServiceResult {
            updates: update_result
                .updates
                .into_iter()
                .map(workspace_update_record)
                .collect(),
            messages: update_result.messages,
        })
    }

    /// Apply watch events through the service.
    pub fn apply_watch_events(
        &self,
        events: Vec<FileWatchEvent>,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        // prepare an empty result payload
        let mut result = LanguageServiceResult::default();
        if events.is_empty() {
            return Ok(result);
        }

        // track whether this batch requires a rescan
        let mut require_rescan = false;

        // apply each watch event in order
        for event in events {
            // overflow requests a full rescan
            if matches!(event.kind, FileWatchEventKind::Overflow) {
                require_rescan = true;
                result.messages.push(warning_message(
                    "watch_overflow_rescan",
                    "watch: rescan required after overflow",
                ));
                continue;
            }

            // deleted files become removed virtual updates
            if matches!(event.kind, FileWatchEventKind::Deleted) {
                if !self.is_watchable_path(&event.path) {
                    continue;
                }

                match self.remove_virtual_file(&event.path) {
                    Ok(update_result) => {
                        result.updates.extend(
                            update_result
                                .updates
                                .into_iter()
                                .map(workspace_update_record),
                        );
                        result.messages.extend(update_result.messages);
                    }
                    Err(error) => result.messages.push(warning_message(
                        "watch_remove_failed",
                        &format!("watch: failed to remove {}: {error}", event.path.display()),
                    )),
                }

                continue;
            }

            // renamed files are handled as remove then create
            if matches!(event.kind, FileWatchEventKind::Renamed) {
                if let Some(previous_path) = event.previous_path.as_ref()
                    && self.is_watchable_path(previous_path)
                {
                    match self.remove_virtual_file(previous_path) {
                        Ok(update_result) => {
                            result.updates.extend(
                                update_result
                                    .updates
                                    .into_iter()
                                    .map(workspace_update_record),
                            );
                            result.messages.extend(update_result.messages);
                        }
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
                    match self.watch_update_for_path(&event.path) {
                        Ok(update) => match self.apply_virtual_update(&event.path, update) {
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

            // ignore events for non-watchable files
            if !self.is_watchable_path(&event.path) {
                continue;
            }

            // apply file content updates from disk
            match self.watch_update_for_path(&event.path) {
                Ok(update) => match self.apply_virtual_update(&event.path, update) {
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

        // rescan after overflow events
        if require_rescan {
            let rescan = self.rescan_all(RescanReason::Update)?;
            result.updates.extend(rescan.updates);
            result.messages.extend(rescan.messages);
        }

        Ok(result)
    }

    /// Read one watch path into a file update payload.
    fn watch_update_for_path(&self, path: &Path) -> std::io::Result<FileUpdate> {
        // read binary file types as bytes to avoid utf8 decode failures
        if FileType::from_path(path).is_some_and(|file_type| file_type.is_binary()) {
            let content = self.session.fs.read(path)?;
            return Ok(FileUpdate::Bytes { content });
        }

        // read all other watchable files as text
        let content = self.session.fs.read_to_string(path)?;
        Ok(FileUpdate::Text { content })
    }

    /// Request a rescan for every workspace handle.
    pub fn rescan_all(
        &self,
        _reason: RescanReason,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        // collect all opened root paths
        let roots: Vec<PathBuf> = self
            .handles_by_root
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        // perform a full analyzed rescan
        self.rescan_roots(&roots, true)
    }

    /// Request a rescan for specific workspace roots.
    pub fn rescan_roots(
        &self,
        roots: &[PathBuf],
        analyze: bool,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        let mut result = LanguageServiceResult::default();

        // rescan each requested root with compile serialization
        for root in roots {
            let handle = self.program_handle_for_root(root);
            let _compile_guard = handle.compile_lock.lock();
            let rescan = self.rescan_program(&handle, analyze)?;
            result.updates.extend(rescan.updates);
            result.messages.extend(rescan.messages);
        }

        Ok(result)
    }

    /// Mark a file as removed without touching disk.
    fn remove_virtual_file(
        &self,
        path: &Path,
    ) -> Result<VirtualUpdateResult, LanguageServiceError> {
        self.apply_virtual_file_update(path, FileUpdate::Removed)
    }

    /// Apply a virtual file update.
    fn apply_virtual_file_update(
        &self,
        path: &Path,
        update: FileUpdate,
    ) -> Result<VirtualUpdateResult, LanguageServiceError> {
        // resolve and lock the owning program handle
        let handle = self.program_handle_for_path(path);
        let _compile_guard = handle.compile_lock.lock();
        let program = handle.program.clone();
        let compiler = handle.compiler.clone();

        // resolve config file ids eagerly when needed
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

        // resolve module ids for non-config paths
        let module_id = if config_file_id.is_some() {
            None
        } else {
            match compiler.resolve_path_to_module(&path.to_path_buf()) {
                Ok(module_id) => Some(module_id),
                Err(error) => {
                    if program.files.get_id_by_path(path).is_some() {
                        None
                    } else {
                        return Err(LanguageServiceError::ResolvePathFailed {
                            path: path.to_path_buf(),
                            detail: error.to_string(),
                        });
                    }
                }
            }
        };

        // resolve the primary file id for invalidation
        let file_id = if let Some(file_id) = config_file_id {
            file_id
        } else {
            match module_id {
                Some(module_id) => program.modules.get(module_id).read().file_id,
                None => program.files.get_id_by_path(path).ok_or_else(|| {
                    LanguageServiceError::FileNotTracked {
                        path: path.to_path_buf(),
                    }
                })?,
            }
        };

        // invalidate file content and module state
        let invalidation = program.invalidate_file(file_id, update).map_err(|error| {
            LanguageServiceError::InvalidatePathFailed {
                path: path.to_path_buf(),
                detail: error.to_string(),
            }
        })?;

        // refresh config state when config files changed
        let mut messages = Vec::new();
        if self.should_refresh_configs(&program, &invalidation, path) {
            let resolver = Resolver::from_program(&program, ResolveOptions::default());
            messages.extend(self.refresh_program_configs(&resolver, &program));
        }

        // build and analyze update payloads
        let mut updates = vec![build_update(&program, module_id, file_id, invalidation)?];
        self.analyze_updates(&program, &compiler, &mut updates)?;

        Ok(VirtualUpdateResult { updates, messages })
    }

    /// Rescan tracked files for a single program.
    fn rescan_program(
        &self,
        handle: &ProgramHandle,
        analyze: bool,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        let program = handle.program.as_ref();

        // gather all tracked file ids for this root
        let mut file_ids = std::collections::HashSet::new();
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

        // rebuild updates by diffing on-disk content
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

        // refresh config state after the rescan
        let resolver = Resolver::from_program(program, ResolveOptions::default());
        messages.extend(self.refresh_program_configs(&resolver, program));

        // run incremental analysis when requested
        if analyze {
            self.analyze_updates(program, handle.compiler.as_ref(), &mut updates)?;
        }

        // map internal updates to public records
        Ok(LanguageServiceResult {
            updates: updates.into_iter().map(workspace_update_record).collect(),
            messages,
        })
    }
}
