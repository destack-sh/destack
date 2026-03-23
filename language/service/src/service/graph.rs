use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use destack_artifact::ArtifactKey;
use destack_compiler::Compiler;
use destack_source::{Diagnostic, DiagnosticStoreUpdate, FileId, ModuleId};
use destack_workspace::{InvalidationKind, InvalidationPlan, Program};

use super::workspace::{ServiceUpdate, build_update};
use super::{AnalyzeOutcome, LanguageService, LanguageServiceError};

impl LanguageService {
    /// Ensure the path is analyzed and semantic-query ready.
    pub fn ensure_analyzed_for_path(&self, path: &Path) -> Result<(), LanguageServiceError> {
        // run a focused analysis for the path
        let outcome = self.analyze_path(path)?;

        // return early when semantic query state is ready
        if outcome.semantic_query_ready {
            return Ok(());
        }

        // report a detailed failure message otherwise
        let detail = outcome
            .detail
            .unwrap_or_else(|| "semantic query state not ready after analyze".to_string());
        Err(LanguageServiceError::AnalyzeFailed { detail })
    }

    /// Analyze a path and update diagnostics.
    pub fn analyze_path(&self, path: &Path) -> Result<AnalyzeOutcome, LanguageServiceError> {
        // resolve and lock the owning workspace handle
        let handle = self.workspace_handle_for_path(path)?;
        let _compile_guard = handle.enter_mutation();
        let program = handle.program.clone();
        let compiler = handle.compiler.clone();

        // resolve the module for the requested path
        let module_id = compiler
            .resolve_path_to_module(&path.to_path_buf())
            .map_err(|error| LanguageServiceError::ResolvePathFailed {
                path: path.to_path_buf(),
                detail: error.to_string(),
            })?;

        // clear pending diagnostics before analysis
        let _ = program.diagnostics.drain();

        // enqueue and run the module analysis task
        let profile_id = program.default_profile_id_for_module(module_id);
        compiler.enqueue(ArtifactKey::dir_analyzed(module_id, profile_id));
        compiler.compile();

        // group fresh diagnostics by file id
        let diagnostics = program.diagnostics.collect();
        let mut diagnostics_by_file: HashMap<FileId, Vec<Diagnostic>> = HashMap::new();
        for diagnostic in diagnostics.iter() {
            diagnostics_by_file
                .entry(diagnostic.file_id)
                .or_default()
                .push(diagnostic.clone());
        }

        // compute semantic query readiness for the analyzed module
        let module = program.modules.get(module_id);
        let module = module.as_ref();
        let module_file_id = module.file_id;
        let module_source_version = program.modules.source_version(module.id);
        let ast_ready = compiler.artifacts.ast(module_id).is_some();
        let dir_ready = compiler
            .artifacts
            .dir_analyzed(module_id, profile_id)
            .is_some();
        let semantic_query_ready = ast_ready && dir_ready;
        let detail = if semantic_query_ready {
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

        // build diagnostic store updates for the primary module file
        let mut store_updates = Vec::new();
        store_updates.push(DiagnosticStoreUpdate::new(
            module_file_id,
            module_source_version,
            diagnostics_by_file
                .remove(&module_file_id)
                .unwrap_or_default(),
        ));

        // build diagnostic store updates for related files
        for (file_id, diagnostics) in diagnostics_by_file {
            let file = program
                .files
                .get_maybe(file_id)
                .ok_or(LanguageServiceError::FileIdNotTracked { file_id })?;
            store_updates.push(DiagnosticStoreUpdate::new(
                file_id,
                file.version,
                diagnostics,
            ));
        }

        // commit diagnostic updates
        program.diagnostic_store.apply_updates(store_updates);

        Ok(AnalyzeOutcome {
            semantic_query_ready,
            detail,
        })
    }

    /// Analyze updates and attach diagnostics.
    pub(super) fn analyze_updates(
        &self,
        program: &Program,
        compiler: &Compiler,
        updates: &mut Vec<ServiceUpdate>,
    ) -> Result<(), LanguageServiceError> {
        // collect directly updated modules and files
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

        // collect modules reported by invalidation fanout
        let mut extra_updates = Vec::new();
        for update in updates.iter() {
            for module_id in update.invalidation.modules.iter().copied() {
                if !self.is_workspace_module_id(program, module_id) {
                    continue;
                }

                module_ids.insert(module_id);
                let module = program.modules.get(module_id);
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

        // clear diagnostics when at least one module will be analyzed
        if !module_ids.is_empty() {
            let _ = program.diagnostics.drain();
        }

        // ensure module graph state is ready for dependency fanout
        self.ensure_module_graphs_ready(program, compiler, &module_ids);

        // include dependent modules with matching graph versions
        if module_ids.len() < program.modules.len() {
            let mut queue: VecDeque<ModuleId> = module_ids.iter().copied().collect();
            while let Some(module_id) = queue.pop_front() {
                let profile_id = program.default_profile_id_for_module(module_id);
                let Some(graph) = compiler.artifacts.module_graph(profile_id) else {
                    continue;
                };

                let module = program.modules.get(module_id);
                let module_version = program.modules.version(module.id);
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

        // materialize update records for dependency files
        if !updates.is_empty() {
            let merged_invalidation = merge_invalidation_plans(updates);
            let mut dependency_updates = Vec::new();
            for module_id in module_ids.iter().copied() {
                let module = program.modules.get(module_id);
                let file_id = module.file_id;
                if file_ids.insert(file_id) {
                    let file = program
                        .files
                        .get_maybe(file_id)
                        .ok_or(LanguageServiceError::FileIdNotTracked { file_id })?;
                    let invalidation = InvalidationPlan {
                        file_id,
                        file_version: file.version,
                        kinds: merged_invalidation.kinds.clone(),
                        modules: merged_invalidation.modules.clone(),
                        packages: merged_invalidation.packages.clone(),
                        profiles: merged_invalidation.profiles.clone(),
                        graphs_dropped: merged_invalidation.graphs_dropped.clone(),
                    };
                    let dependency_update =
                        build_update(program, Some(module_id), file_id, invalidation)?;
                    dependency_updates.push(dependency_update);
                }
            }
            updates.extend(dependency_updates);
        }

        // run analysis for the full affected module set
        if !module_ids.is_empty() {
            for module_id in &module_ids {
                let profile = program.default_profile_id_for_module(*module_id);
                compiler.enqueue(ArtifactKey::dir_analyzed(*module_id, profile));
            }
            compiler.compile();

            // group emitted diagnostics by file id
            let mut diagnostics_by_file: HashMap<FileId, Vec<Diagnostic>> = HashMap::new();
            for diagnostic in program.diagnostics.iter() {
                if file_ids.contains(&diagnostic.file_id) {
                    diagnostics_by_file
                        .entry(diagnostic.file_id)
                        .or_default()
                        .push(diagnostic.clone());
                }
            }

            // attach grouped diagnostics to each update
            for update in updates.iter_mut() {
                update.diagnostics = diagnostics_by_file
                    .remove(&update.file_id)
                    .unwrap_or_default();
            }
        }

        // publish diagnostic store updates for all touched files
        let mut store_updates = Vec::new();
        for update in updates.iter() {
            let file = program.files.get_maybe(update.file_id).ok_or(
                LanguageServiceError::FileIdNotTracked {
                    file_id: update.file_id,
                },
            )?;
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
        // return early when there are no modules to resolve
        if module_ids.is_empty() {
            return;
        }

        // collect resolve tasks for stale or missing graphs
        let mut resolve_tasks = Vec::new();
        let mut queued = HashSet::new();
        for module_id in module_ids.iter().copied() {
            let profile_id = program.default_profile_id_for_module(module_id);
            if let Some(graph) = compiler.artifacts.module_graph(profile_id) {
                let module = program.modules.get(module_id);
                let module = module.as_ref();
                let graph_version = graph.module_versions.get(&module_id).copied();
                if graph_version == Some(program.modules.version(module.id)) {
                    continue;
                }

                if queued.insert((module_id, profile_id)) {
                    resolve_tasks.push(ArtifactKey::dir_resolved(module_id, profile_id));
                }

                continue;
            }

            for module in program.modules.iter() {
                let module = module.as_ref();
                if !self.is_workspace_module(&module) {
                    continue;
                }
                if program.default_profile_id_for_module(module.id) != profile_id {
                    continue;
                }

                if queued.insert((module.id, profile_id)) {
                    resolve_tasks.push(ArtifactKey::dir_resolved(module.id, profile_id));
                }
            }
        }

        // return when no graph refresh is needed
        if resolve_tasks.is_empty() {
            return;
        }

        // enqueue and run resolve tasks
        for task in resolve_tasks {
            compiler.enqueue(task);
        }
        compiler.compile();
    }
}

/// Merge invalidation metadata from a set of updates.
fn merge_invalidation_plans(updates: &[ServiceUpdate]) -> InvalidationPlan {
    let mut kinds = HashSet::new();
    let mut modules = HashSet::new();
    let mut packages = HashSet::new();
    let mut profiles = HashSet::new();
    let mut graphs_dropped = HashSet::new();

    // aggregate invalidation sets from all updates
    for update in updates {
        kinds.extend(update.invalidation.kinds.iter().copied());
        modules.extend(update.invalidation.modules.iter().copied());
        packages.extend(update.invalidation.packages.iter().copied());
        profiles.extend(update.invalidation.profiles.iter().copied());
        graphs_dropped.extend(update.invalidation.graphs_dropped.iter().copied());
    }

    // preserve a stable default kind when no kinds are present
    if kinds.is_empty() {
        kinds.insert(InvalidationKind::Unknown);
    }

    // seed identity values from the first update
    let first = &updates[0].invalidation;
    InvalidationPlan {
        file_id: first.file_id,
        file_version: first.file_version,
        kinds: kinds.into_iter().collect(),
        modules: modules.into_iter().collect(),
        packages: packages.into_iter().collect(),
        profiles: profiles.into_iter().collect(),
        graphs_dropped: graphs_dropped.into_iter().collect(),
    }
}
