use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_query::RepositoryQueryIndexExt;
use destack_resolver::{CachePolicy, ResolveOptions, Resolver};
use destack_source::{
    FileId, FileType, FileWatchEvent, FileWatchEventKind, ModuleId, ProfileId, Uri,
};
use destack_workspace::{Repository, Revision};

use super::workspace::{ServiceUpdate, WorkspaceSession, build_update, warning_message};
use super::{
    FileUpdate, LanguageService, LanguageServiceError, LanguageServiceResult, RescanReason,
    UpdateImpact, UpdateImpactKind, WorkspaceMessage,
};

/// Result of a low level virtual update.
struct VirtualUpdateResult {
    /// Repository state paired with these updates.
    repository: Arc<Repository>,
    /// Internal update records.
    updates: Vec<ServiceUpdate>,
    /// Semantic revision after the update.
    revision: Revision,
    /// Deferred query-index work for these updates.
    indexes: VirtualUpdateIndexes,
    /// Messages emitted during update processing.
    messages: Vec<WorkspaceMessage>,
}

/// Prepared query-index inputs for one virtual update.
struct VirtualUpdateIndexes {
    /// Modules that need fresh per-module query index slices.
    module_ids: Vec<ModuleId>,
}

impl LanguageService {
    /// Set one tracked document to its current editor text.
    pub fn set_document(
        &self,
        path: &Path,
        uri: Uri,
        version: i32,
        text: String,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        // resolve and lock the owning workspace session
        let session = self.workspace_for_document_path(path)?;
        let _mutation_guard = session.enter_mutation();

        // reject stale editor updates at the service boundary
        if let Some((_, current_version)) = session.tracked_document_identity_for_path(path)
            && version <= current_version
        {
            return Err(LanguageServiceError::StaleDocumentVersion {
                path: path.to_path_buf(),
                incoming: version,
                current: current_version,
            });
        }

        // apply the semantic update before committing the tracked document state
        let update = FileUpdate::Text {
            content: text.clone(),
        };
        let update_result = self.apply_virtual_file_update_locked(&session, path, update)?;
        self.finish_document_update(&session, path, Some((uri, version, text)), update_result)
    }

    /// Sync one tracked document to explicit saved text.
    pub fn sync_document(
        &self,
        path: &Path,
        text: String,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        // resolve and lock the owning workspace session
        let session = self.workspace_for_document_path(path)?;
        let _mutation_guard = session.enter_mutation();

        // preserve tracked editor identity for still-open documents
        let tracked_document = session
            .tracked_document_identity_for_path(path)
            .map(|(uri, version)| (uri, version, text.clone()));

        // apply the semantic update before mutating tracked state
        let update = FileUpdate::Text {
            content: text.clone(),
        };
        let update_result = self.apply_virtual_file_update_locked(&session, path, update)?;
        self.finish_document_update(&session, path, tracked_document, update_result)
    }

    /// Close one tracked document and restore filesystem backed source truth.
    pub fn close_document(
        &self,
        path: &Path,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        // resolve and lock the owning workspace session
        let session = self.workspace_for_document_path(path)?;
        let _mutation_guard = session.enter_mutation();

        // ignore closes for documents that are no longer tracked
        let Some((uri, version, text)) = session.tracked_document_for_path(path) else {
            return Ok(LanguageServiceResult::default());
        };

        // drop the overlay before reading and reanalyzing filesystem truth
        let _closed_document = session.untrack_document(path);
        session.remove_overlay_for_path(path);

        let update = match self.watch_update_for_path(path) {
            Ok(update) => update,
            Err(error) if error.kind() == ErrorKind::NotFound => FileUpdate::Removed,
            Err(error) => {
                session.set_tracked_document(path, uri.clone(), version);
                session.set_overlay_for_path(path, text);

                return Err(LanguageServiceError::Internal {
                    detail: format!(
                        "failed to restore closed document {}: {error}",
                        path.display()
                    ),
                });
            }
        };

        let update_result = match self.apply_virtual_file_update_locked(&session, path, update) {
            Ok(update_result) => update_result,
            Err(error) => {
                session.set_tracked_document(path, uri.clone(), version);
                session.set_overlay_for_path(path, text);

                return Err(error);
            }
        };

        self.finish_document_close(&session, uri, version, update_result)
    }

    /// Classify coarse impact kinds for one updated path.
    fn impact_kinds_for_path(&self, path: &Path) -> Vec<UpdateImpactKind> {
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            return vec![UpdateImpactKind::Unknown];
        };

        if file_name == "destack.json" {
            return vec![UpdateImpactKind::Destack];
        }

        if file_name.starts_with("tsconfig") && file_name.ends_with(".json") {
            return vec![UpdateImpactKind::TsConfig];
        }

        vec![UpdateImpactKind::Unknown]
    }

    /// Build one service-local impact summary after one file update.
    fn build_update_impact(
        &self,
        repository: &Repository,
        revision: Revision,
        path: &Path,
        file_id: FileId,
    ) -> Result<UpdateImpact, LanguageServiceError> {
        let module_id = repository
            .module_id_for_file(revision, file_id)
            .map_err(LanguageServiceError::from)?;
        let packages = module_id
            .map(|module_id| vec![module_id.package_id])
            .unwrap_or_default();

        Ok(UpdateImpact {
            file_id,
            kinds: self.impact_kinds_for_path(path),
            modules: module_id.into_iter().collect(),
            packages,
            profiles: Vec::<ProfileId>::new(),
            graphs_dropped: Vec::<ProfileId>::new(),
        })
    }

    /// Build one resolver against the current repository owned semantic state.
    fn resolver_for_repository(
        &self,
        repository: &Repository,
        options: ResolveOptions,
    ) -> Resolver {
        Resolver::from_repository(repository, options)
    }

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
        // resolve and lock the owning workspace session
        let session = self.workspace_for_document_path(path)?;
        let _mutation_guard = session.enter_mutation();

        // apply the low level update
        let update_result = self.apply_virtual_file_update_locked(&session, path, update)?;
        let VirtualUpdateResult {
            repository,
            updates,
            revision,
            indexes,
            messages,
            ..
        } = update_result;

        // finish secondary query-index warmup before releasing the mutation lane
        self.finish_virtual_update_indexes(&repository, revision, indexes);
        session.publish_revision(revision)?;

        // map internal updates to public records
        Ok(LanguageServiceResult {
            updates: updates
                .into_iter()
                .map(|update| session.workspace_update_record(update, None))
                .collect(),
            messages,
        })
    }

    /// Apply watch events through the service.
    pub fn apply_watch_events(
        &self,
        events: Vec<FileWatchEvent>,
        _preferred_uris: HashMap<PathBuf, Uri>,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        let (mut result, require_rescan) = self.apply_watch_events_immediate(events)?;

        if require_rescan {
            let rescan = self.rescan_all(RescanReason::Update)?;
            result.updates.extend(rescan.updates);
            result.messages.extend(rescan.messages);
        }

        Ok(result)
    }

    /// Apply watch events and return immediate updates plus rescan intent.
    fn apply_watch_events_immediate(
        &self,
        events: Vec<FileWatchEvent>,
    ) -> Result<(LanguageServiceResult, bool), LanguageServiceError> {
        // prepare an empty result payload
        let mut result = LanguageServiceResult::default();
        if events.is_empty() {
            return Ok((result, false));
        }

        // track whether this batch requires a rescan
        let mut require_rescan = false;

        // apply each watch event in order
        for event in events {
            // tracked documents own source truth: stale disk watch events must not override them
            let is_path_tracked = self.has_tracked_document_for_path(&event.path);
            let is_previous_path_tracked = event
                .previous_path
                .as_ref()
                .is_some_and(|path| self.has_tracked_document_for_path(path));
            if is_path_tracked || is_previous_path_tracked {
                continue;
            }

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
                // structural deletes require graph-level rediscovery
                require_rescan = true;

                if !self.is_watchable_path(&event.path) {
                    continue;
                }

                match self.apply_virtual_update(&event.path, FileUpdate::Removed) {
                    Ok(update_result) => {
                        result.updates.extend(update_result.updates);
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
                // structural renames require graph-level rediscovery
                require_rescan = true;

                if let Some(previous_path) = event.previous_path.as_ref()
                    && self.is_watchable_path(previous_path)
                {
                    match self.apply_virtual_update(previous_path, FileUpdate::Removed) {
                        Ok(update_result) => {
                            result.updates.extend(update_result.updates);
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

            // structural creates require graph-level rediscovery
            if matches!(event.kind, FileWatchEventKind::Created) {
                require_rescan = true;
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

        Ok((result, require_rescan))
    }

    /// Read one watch path into a file update payload.
    fn watch_update_for_path(&self, path: &Path) -> std::io::Result<FileUpdate> {
        // read binary file types as bytes to avoid utf8 decode failures
        if FileType::from_path(path).is_some_and(|file_type| file_type.is_binary()) {
            let content = self.repository.file_system().read(path)?;
            return Ok(FileUpdate::Bytes { content });
        }

        // read all other watchable files as text
        let content = self.repository.file_system().read_to_string(path)?;
        Ok(FileUpdate::Text { content })
    }

    /// Request a rescan for every workspace root.
    pub fn rescan_all(
        &self,
        _reason: RescanReason,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        // collect all opened root paths
        let roots: Vec<PathBuf> = self
            .workspaces_by_root
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
            let session = self.workspace_for_root(root)?;
            let _compile_guard = session.enter_mutation();
            let rescan = self.rescan_repository(&session, analyze)?;
            result.updates.extend(rescan.updates);
            result.messages.extend(rescan.messages);
        }

        Ok(result)
    }

    /// Apply a virtual file update while already holding the workspace mutation lock.
    fn apply_virtual_file_update_locked(
        &self,
        session: &Arc<WorkspaceSession>,
        path: &Path,
        update: FileUpdate,
    ) -> Result<VirtualUpdateResult, LanguageServiceError> {
        let repository = session.repository().clone();
        let revision = session.revision();
        let compiler = session.compiler().clone();

        // resolve config file ids eagerly when needed
        let config_file_id = if self.is_config_filename(path) {
            let mut file_id = {
                let file_id = repository.file_id_for_workspace_path(path);
                repository
                    .file(revision, file_id)
                    .map_err(LanguageServiceError::from)?
                    .map(|_| file_id)
            };
            if file_id.is_none() {
                let workspace_options = self.repository.workspace_options(revision)?;
                let resolver = self.resolver_for_repository(
                    &repository,
                    ResolveOptions::default_for_workspace(
                        repository.cwd.clone(),
                        workspace_options.as_ref(),
                    ),
                );
                if path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name == "destack.json")
                {
                    let _ = resolver.read_destack_config(path, CachePolicy::Reload);
                } else {
                    let _ = resolver.reload_tsconfig(path);
                }
                let candidate_file_id = repository.file_id_for_workspace_path(path);
                file_id = repository
                    .file(revision, candidate_file_id)
                    .map_err(LanguageServiceError::from)?
                    .map(|_| candidate_file_id);
            }
            file_id
        } else {
            None
        };

        // admit newly visible workspace modules before impact and analysis work
        let should_discover_module = !matches!(&update, FileUpdate::Removed)
            && self.should_discover_workspace_module_path(path);

        // defer module identity to the post update revision
        let module_id = None;

        // resolve the primary file id for impact
        let file_id = if let Some(file_id) = config_file_id {
            file_id
        } else {
            repository.file_id_for_workspace_path(path)
        };

        // publish the updated file into the workspace revision
        let revision = session.stage_file_update(path, update)?;

        // discover the updated file as a workspace module when needed
        let mut messages = Vec::new();
        if should_discover_module
            && repository
                .module_id_for_path(revision, path)
                .map_err(LanguageServiceError::from)?
                .is_none()
            && let Err(error) = compiler.resolve_path_to_module(revision, &path.to_path_buf())
        {
            messages.push(warning_message(
                "workspace_discovery_module_failed",
                &format!(
                    "workspace: failed to discover module {}: {error}",
                    path.display(),
                ),
            ));
        }

        let impact = self.build_update_impact(&repository, revision, path, file_id)?;

        // drop stale query index slices before any follow up analysis
        repository.remove_query_modules(impact.modules.iter().copied());

        // refresh config state when config files changed
        if self.should_refresh_configs(&repository, revision, &impact, path) {
            messages.extend(self.refresh_repository_configs(&repository, revision));
        }

        // build and analyze update payloads
        let mut updates = vec![build_update(
            &repository,
            revision,
            module_id,
            file_id,
            impact,
        )?];
        let revision =
            self.finalize_updates(&session, &repository, &compiler, revision, &mut updates)?;

        // collect the affected modules for deferred query-index warmup
        let module_ids = updates
            .iter()
            .filter_map(|update| update.module_id)
            .collect();
        let indexes = VirtualUpdateIndexes { module_ids };

        Ok(VirtualUpdateResult {
            repository,
            updates,
            revision,
            indexes,
            messages,
        })
    }

    /// Rescan tracked files for a single repository.
    pub(super) fn rescan_repository(
        &self,
        session: &WorkspaceSession,
        analyze: bool,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        let repository = session.repository();
        let repository = repository.as_ref();
        let mut revision = session.revision();
        let mut messages = self.discover_workspace_modules(session);

        // gather all tracked file ids for this root
        let mut file_ids = std::collections::HashSet::new();
        for module_id in repository
            .workspace_module_ids(revision)
            .map_err(LanguageServiceError::from)?
        {
            let Some(module) = repository
                .module(revision, module_id)
                .map_err(LanguageServiceError::from)?
            else {
                continue;
            };
            if !self.is_workspace_module(&module) {
                continue;
            }
            file_ids.insert(module.file_id);
        }

        if let Ok(Some(workspace_config)) = self.repository.workspace_destack_declaration(revision)
        {
            file_ids.insert(workspace_config.file_id);
        }

        for module_id in repository
            .workspace_module_ids(revision)
            .map_err(LanguageServiceError::from)?
        {
            let Some(module) = repository
                .module(revision, module_id)
                .map_err(LanguageServiceError::from)?
            else {
                continue;
            };
            let package_id = module_id.package_id;
            if let Some(package) = repository
                .package(revision, package_id)
                .map_err(LanguageServiceError::from)?
                && let Some(config) = repository
                    .package_destack_declaration(revision, &package)
                    .map_err(LanguageServiceError::from)?
            {
                file_ids.insert(config.file_id);
            }

            if let Some(tsconfig_file_id) = module.tsconfig_file_id
                && let Some(tsconfig) = repository
                    .tsconfig_declaration(revision, tsconfig_file_id)
                    .map_err(LanguageServiceError::from)?
            {
                file_ids.insert(tsconfig.file_id);
            }
        }

        // rebuild updates by diffing on-disk content
        let mut updates = Vec::new();
        for file_id in file_ids {
            let update = match self.rescan_file_update(repository, revision, file_id) {
                Ok(Some(update)) => update,
                Ok(None) => continue,
                Err(message) => {
                    messages.push(message);
                    continue;
                }
            };

            let Some(file) = repository
                .file(revision, file_id)
                .map_err(LanguageServiceError::from)?
            else {
                continue;
            };
            let Some(path) = file.path.clone().or_else(|| file.uri.to_path_buf()) else {
                continue;
            };
            revision = session.apply_file_update(path.as_path(), update)?;
            let impact = self.build_update_impact(repository, revision, path.as_path(), file_id)?;

            // drop stale query index slices before follow up analysis
            repository.remove_query_modules(impact.modules.iter().copied());

            let module_id = repository
                .module_id_for_file(revision, file_id)
                .map_err(LanguageServiceError::from)?;
            match build_update(repository, revision, module_id, file_id, impact) {
                Ok(update) => updates.push(update),
                Err(error) => messages.push(warning_message(
                    "rescan_invalidation_failed",
                    &format!("watch: failed to rescan {file_id:?}: {error}"),
                )),
            }
        }

        // refresh config state after the rescan
        messages.extend(self.refresh_repository_configs(repository, revision));

        // run incremental analysis when requested
        if analyze {
            revision = self.finalize_updates(
                session,
                repository,
                session.compiler().as_ref(),
                revision,
                &mut updates,
            )?;

            // warm query indexes for the analyzed modules
            let module_ids = updates.iter().filter_map(|update| update.module_id);
            repository.index_query_modules(revision, module_ids);
            repository.index_query_imports(revision);
        }

        // map internal updates to public records
        Ok(LanguageServiceResult {
            updates: updates
                .into_iter()
                .map(|update| session.workspace_update_record(update, None))
                .collect(),
            messages,
        })
    }
}

impl LanguageService {
    /// Finish query-index warmup for one virtual update.
    fn finish_virtual_update_indexes(
        &self,
        repository: &Repository,
        revision: Revision,
        indexes: VirtualUpdateIndexes,
    ) {
        // warm the touched module query indexes
        repository.index_query_modules(revision, indexes.module_ids);
    }

    /// Commit one successful document mutation after semantic update work finished.
    fn finish_document_update(
        &self,
        session: &Arc<WorkspaceSession>,
        path: &Path,
        tracked_document: Option<(Uri, i32, String)>,
        update_result: VirtualUpdateResult,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        let VirtualUpdateResult {
            repository,
            updates,
            revision,
            indexes,
            messages,
            ..
        } = update_result;

        // finish query-index warmup before exposing the new document state
        self.finish_virtual_update_indexes(&repository, revision, indexes);
        session.publish_revision(revision)?;

        // commit tracked editor state only after the semantic update succeeds
        let preferred_uri = if let Some((uri, version, text)) = tracked_document {
            session.set_tracked_document(path, uri.clone(), version);
            session.set_overlay_for_path(path, text);

            Some(uri)
        } else {
            None
        };

        Ok(LanguageServiceResult {
            updates: updates
                .into_iter()
                .map(|update| session.workspace_update_record(update, preferred_uri.as_ref()))
                .collect(),
            messages,
        })
    }

    /// Commit one successful document close after filesystem truth was restored.
    fn finish_document_close(
        &self,
        session: &Arc<WorkspaceSession>,
        uri: Uri,
        _version: i32,
        update_result: VirtualUpdateResult,
    ) -> Result<LanguageServiceResult, LanguageServiceError> {
        let VirtualUpdateResult {
            repository,
            updates,
            revision,
            indexes,
            messages,
            ..
        } = update_result;

        // finish query-index warmup before exposing the closed state
        self.finish_virtual_update_indexes(&repository, revision, indexes);
        session.publish_revision(revision)?;

        Ok(LanguageServiceResult {
            updates: updates
                .into_iter()
                .map(|update| session.workspace_update_record(update, Some(&uri)))
                .collect(),
            messages,
        })
    }
}
