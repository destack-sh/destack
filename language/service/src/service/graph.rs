use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use destack_artifact::ArtifactKey;
use destack_compiler::Compiler;
use destack_source::{Diagnostic, DiagnosticCollection, FileId, ModuleId};
use destack_workspace::{Repository, Revision};

use super::workspace::{ServiceUpdate, build_update};
use super::{LanguageService, LanguageServiceError, UpdateImpact, UpdateImpactKind};

impl LanguageService {
    /// Return true when update finalization should expand through the full workspace dependency graph.
    fn should_expand_workspace_dependencies(
        &self,
        session: &super::workspace::WorkspaceSession,
        repository: &Repository,
    ) -> bool {
        session.root == repository.workspace_root()
    }

    /// Ensure the query artifact frontier for a path is available.
    pub fn ensure_query_artifacts_for_path(&self, path: &Path) -> Result<(), LanguageServiceError> {
        // resolve and lock the owning workspace session
        let session = self.workspace_for_path(path)?;
        let _compile_guard = session.enter_mutation();
        let repository = session.repository().clone();
        let compiler = session.compiler().clone();
        let revision = session.revision();

        self.ensure_query_artifacts_in_repository(&repository, &compiler, revision, path)
    }

    /// Ensure the query artifact frontier for one path inside one repository.
    pub(super) fn ensure_query_artifacts_in_repository(
        &self,
        repository: &Repository,
        compiler: &Compiler,
        revision: Revision,
        path: &Path,
    ) -> Result<(), LanguageServiceError> {
        // resolve the module for the requested path
        let module_id = compiler
            .resolve_path_to_module(revision, &path.to_path_buf())
            .map_err(|error: destack_compiler::ImportError| {
                LanguageServiceError::ResolvePathFailed {
                    path: path.to_path_buf(),
                    detail: error.to_string(),
                }
            })?;

        // enqueue and run the module analysis task
        let context = compiler
            .context(revision)
            .map_err(LanguageServiceError::from)?;
        let profile_id = context.default_profile_id_for_module(module_id);
        let profile_id = compiler
            .run_to_completion(revision, |compiler, _context| {
                compiler.require_dir_analyzed(revision, module_id, profile_id)?;

                Ok::<_, destack_compiler::RequirementError>(profile_id)
            })
            .map_err(|error| LanguageServiceError::QueryNotReady {
                detail: format!("query artifact preparation failed: {error:?}"),
            })?;

        // validate the realized query frontier
        let module = repository
            .module(revision, module_id)
            .map_err(LanguageServiceError::from)?
            .ok_or(LanguageServiceError::ModuleIdNotTracked { module_id })?;
        let module_file_id = module.file_id;
        let _module_file = repository
            .file(revision, module_file_id)
            .map_err(LanguageServiceError::from)?
            .ok_or(LanguageServiceError::FileIdNotTracked {
                file_id: module_file_id,
            })?;
        let ast_ready = repository.ast(revision, module_id).is_some();
        let dir_ready = repository
            .dir_analyzed(revision, module_id, profile_id)
            .is_some();
        if ast_ready && dir_ready {
            return Ok(());
        }

        let module_path = module
            .path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<none>".to_string());
        Err(LanguageServiceError::QueryNotReady {
            detail: format!(
                "file_id={module_file_id:?} module_id={module_id:?} profile_id={profile_id:?} ast_ready={ast_ready} dir_ready={dir_ready} path={module_path}",
            ),
        })
    }

    /// Finalize staged updates and attach diagnostics.
    pub(super) fn finalize_updates(
        &self,
        session: &super::workspace::WorkspaceSession,
        repository: &Repository,
        compiler: &Compiler,
        revision: Revision,
        updates: &mut Vec<ServiceUpdate>,
    ) -> Result<Revision, LanguageServiceError> {
        // direct update scope
        let (mut module_ids, mut file_ids) =
            self.collect_direct_update_scope(repository, revision, updates);

        // expansion policy
        let should_expand_dependencies =
            self.should_expand_workspace_dependencies(session, repository);

        if should_expand_dependencies {
            // direct impact fanout
            self.expand_direct_impacts(
                repository,
                revision,
                updates,
                &mut module_ids,
                &mut file_ids,
            )?;

            // dependent module fanout
            self.expand_dependent_modules(repository, compiler, revision, &mut module_ids)?;

            // publishable dependency updates
            self.append_dependency_updates(
                repository,
                revision,
                updates,
                &module_ids,
                &mut file_ids,
            )?;
        }

        // artifact frontier
        self.realize_query_artifacts_for_modules(repository, compiler, revision, &module_ids)?;

        // publish projection
        self.refresh_update_files(repository, revision, updates)?;
        self.attach_update_diagnostics(repository, revision, updates, &module_ids, &file_ids)?;

        Ok(revision)
    }

    /// Collect directly touched modules and files from one update set.
    fn collect_direct_update_scope(
        &self,
        repository: &Repository,
        revision: Revision,
        updates: &[ServiceUpdate],
    ) -> (HashSet<ModuleId>, HashSet<FileId>) {
        let mut module_ids = HashSet::new();
        let mut file_ids = HashSet::new();

        for update in updates {
            if let Some(module_id) = update.module_id
                && self.is_workspace_module_id(repository, revision, module_id)
            {
                module_ids.insert(module_id);
            }

            file_ids.insert(update.file_id);
        }

        (module_ids, file_ids)
    }

    /// Expand direct impact metadata into concrete workspace updates.
    fn expand_direct_impacts(
        &self,
        repository: &Repository,
        revision: Revision,
        updates: &mut Vec<ServiceUpdate>,
        module_ids: &mut HashSet<ModuleId>,
        file_ids: &mut HashSet<FileId>,
    ) -> Result<(), LanguageServiceError> {
        let mut extra_updates = Vec::new();

        for update in updates.iter() {
            for module_id in update.impact.modules.iter().copied() {
                if !self.is_workspace_module_id(repository, revision, module_id) {
                    continue;
                }

                module_ids.insert(module_id);
                let module = repository
                    .module(revision, module_id)
                    .map_err(LanguageServiceError::from)?
                    .ok_or(LanguageServiceError::ModuleIdNotTracked { module_id })?;
                let file_id = module.file_id;
                if file_ids.insert(file_id) {
                    let extra_update = build_update(
                        repository,
                        revision,
                        Some(module_id),
                        file_id,
                        update.impact.clone(),
                    )?;
                    extra_updates.push(extra_update);
                }
            }
        }

        updates.extend(extra_updates);

        Ok(())
    }

    /// Expand workspace dependents through the current module graphs.
    fn expand_dependent_modules(
        &self,
        repository: &Repository,
        compiler: &Compiler,
        revision: Revision,
        module_ids: &mut HashSet<ModuleId>,
    ) -> Result<(), LanguageServiceError> {
        self.ensure_module_graphs_ready(repository, compiler, revision, module_ids)?;

        let workspace_module_ids = repository
            .workspace_module_ids(revision)
            .map_err(LanguageServiceError::from)?;
        if module_ids.len() >= workspace_module_ids.len() {
            return Ok(());
        }

        let mut queue: VecDeque<ModuleId> = module_ids.iter().copied().collect();
        while let Some(module_id) = queue.pop_front() {
            let profile_id = repository
                .default_profile_id_for_module(revision, module_id)
                .map_err(LanguageServiceError::from)?;
            let Some(graph) = repository.module_graph(revision, profile_id) else {
                continue;
            };

            for dependent in graph.dependents_for(module_id) {
                if !self.is_workspace_module_id(repository, revision, dependent) {
                    continue;
                }

                if module_ids.insert(dependent) {
                    queue.push_back(dependent);
                }
            }
        }

        Ok(())
    }

    /// Append publishable updates for dependency-expanded modules.
    fn append_dependency_updates(
        &self,
        repository: &Repository,
        revision: Revision,
        updates: &mut Vec<ServiceUpdate>,
        module_ids: &HashSet<ModuleId>,
        file_ids: &mut HashSet<FileId>,
    ) -> Result<(), LanguageServiceError> {
        if updates.is_empty() {
            return Ok(());
        }

        let merged_impact = merge_update_impacts(updates);
        let mut dependency_updates = Vec::new();

        for module_id in module_ids.iter().copied() {
            let module = repository
                .module(revision, module_id)
                .map_err(LanguageServiceError::from)?
                .ok_or(LanguageServiceError::ModuleIdNotTracked { module_id })?;
            let file_id = module.file_id;
            if file_ids.insert(file_id) {
                let _file = repository
                    .file(revision, file_id)
                    .map_err(LanguageServiceError::from)?
                    .ok_or(LanguageServiceError::FileIdNotTracked { file_id })?;
                let impact = UpdateImpact {
                    file_id,
                    kinds: merged_impact.kinds.clone(),
                    modules: merged_impact.modules.clone(),
                    packages: merged_impact.packages.clone(),
                    profiles: merged_impact.profiles.clone(),
                    graphs_dropped: merged_impact.graphs_dropped.clone(),
                };
                let dependency_update =
                    build_update(repository, revision, Some(module_id), file_id, impact)?;
                dependency_updates.push(dependency_update);
            }
        }

        updates.extend(dependency_updates);

        Ok(())
    }

    /// Refresh update file snapshots against one finalized revision.
    fn refresh_update_files(
        &self,
        repository: &Repository,
        revision: Revision,
        updates: &mut [ServiceUpdate],
    ) -> Result<(), LanguageServiceError> {
        for update in updates.iter_mut() {
            let rebuilt = build_update(
                repository,
                revision,
                update.module_id,
                update.file_id,
                update.impact.clone(),
            )?;
            update.file = rebuilt.file;
        }

        Ok(())
    }

    /// Attach current artifact diagnostics to one update set.
    fn attach_update_diagnostics(
        &self,
        repository: &Repository,
        revision: Revision,
        updates: &mut [ServiceUpdate],
        module_ids: &HashSet<ModuleId>,
        file_ids: &HashSet<FileId>,
    ) -> Result<(), LanguageServiceError> {
        if module_ids.is_empty() {
            return Ok(());
        }

        let mut diagnostics = DiagnosticCollection::new();
        for module_id in module_ids {
            let profile_id = repository
                .default_profile_id_for_module(revision, *module_id)
                .map_err(LanguageServiceError::from)?;
            diagnostics.merge_from(
                &repository.module_artifact_diagnostics(revision, *module_id, profile_id),
            );
        }
        let mut diagnostics_by_file = diagnostics_by_file(&diagnostics);

        for update in updates.iter_mut() {
            let diagnostics = diagnostics_by_file
                .remove(&update.file_id)
                .unwrap_or_default();
            if file_ids.contains(&update.file_id) {
                update.diagnostics = diagnostics;
            }
        }

        Ok(())
    }

    /// Ensure module graph entries exist for dependency fanout.
    fn ensure_module_graphs_ready(
        &self,
        repository: &Repository,
        compiler: &Compiler,
        revision: Revision,
        module_ids: &HashSet<ModuleId>,
    ) -> Result<(), LanguageServiceError> {
        // return early when there are no modules to resolve
        if module_ids.is_empty() {
            return Ok(());
        }

        let context = compiler
            .context(revision)
            .map_err(LanguageServiceError::from)?;

        // collect resolve tasks for stale or missing graphs
        let mut resolve_tasks = Vec::new();
        let mut queued = HashSet::new();
        for module_id in module_ids.iter().copied() {
            let profile_id = context.default_profile_id_for_module(module_id);

            // keep the existing graph when it already covers this module
            if let Some(graph) = repository.module_graph(revision, profile_id)
                && (graph.dependents.contains_key(&module_id)
                    || graph.dependencies.contains_key(&module_id))
            {
                continue;
            }

            // otherwise rebuild the profile graph from the current workspace slice
            let Ok(workspace_module_ids) = repository.workspace_module_ids(revision) else {
                continue;
            };
            for workspace_module_id in workspace_module_ids {
                let Ok(Some(module)) = repository.module(revision, workspace_module_id) else {
                    continue;
                };
                if !self.is_workspace_module(&module) {
                    continue;
                }
                let module_profile_id = context.default_profile_id_for_module(module.id);
                if module_profile_id != profile_id {
                    continue;
                }

                if queued.insert((module.id, profile_id)) {
                    resolve_tasks.push(module.id);
                }
            }
        }

        // return when no graph refresh is needed
        if resolve_tasks.is_empty() {
            return Ok(());
        }

        // enqueue and run resolve tasks
        for module_id in resolve_tasks {
            let profile_id = context.default_profile_id_for_module(module_id);
            let artifact_key = ArtifactKey::dir_resolved(module_id, profile_id);

            compiler.enqueue(revision, artifact_key);
        }

        compiler.compile();

        Ok(())
    }

    /// Realize the query artifact frontier for one module set.
    fn realize_query_artifacts_for_modules(
        &self,
        repository: &Repository,
        compiler: &Compiler,
        revision: Revision,
        module_ids: &HashSet<ModuleId>,
    ) -> Result<(), LanguageServiceError> {
        if module_ids.is_empty() {
            return Ok(());
        }

        for module_id in module_ids {
            let profile_id = repository
                .default_profile_id_for_module(revision, *module_id)
                .map_err(LanguageServiceError::from)?;
            compiler.enqueue(revision, ArtifactKey::dir_analyzed(*module_id, profile_id));
        }

        compiler.compile();

        Ok(())
    }
}

/// Group diagnostics by file id.
fn diagnostics_by_file(diagnostics: &DiagnosticCollection) -> HashMap<FileId, Vec<Diagnostic>> {
    let mut diagnostics_by_file = HashMap::new();

    for diagnostic in diagnostics.iter() {
        diagnostics_by_file
            .entry(diagnostic.file_id)
            .or_insert_with(Vec::new)
            .push(diagnostic);
    }

    diagnostics_by_file
}

/// Merge impact metadata from a set of updates.
fn merge_update_impacts(updates: &[ServiceUpdate]) -> UpdateImpact {
    let mut kinds = HashSet::new();
    let mut modules = HashSet::new();
    let mut packages = HashSet::new();
    let mut profiles = HashSet::new();
    let mut graphs_dropped = HashSet::new();

    // aggregate impact sets from all updates
    for update in updates {
        kinds.extend(update.impact.kinds.iter().copied());
        modules.extend(update.impact.modules.iter().copied());
        packages.extend(update.impact.packages.iter().copied());
        profiles.extend(update.impact.profiles.iter().copied());
        graphs_dropped.extend(update.impact.graphs_dropped.iter().copied());
    }

    // preserve a stable default kind when no kinds are present
    if kinds.is_empty() {
        kinds.insert(UpdateImpactKind::Unknown);
    }

    // seed identity values from the first update
    let first = &updates[0].impact;
    UpdateImpact {
        file_id: first.file_id,
        kinds: kinds.into_iter().collect(),
        modules: modules.into_iter().collect(),
        packages: packages.into_iter().collect(),
        profiles: profiles.into_iter().collect(),
        graphs_dropped: graphs_dropped.into_iter().collect(),
    }
}
