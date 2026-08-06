use std::sync::Arc;

use destack_artifact::{
    ArtifactDependency, ArtifactDependencySet, ArtifactKey, ArtifactPayload,
    ArtifactProjectionFingerprint, ArtifactProjectionKey, DirResolved, InherentExtension,
    ModuleGraph, SourceDependencyKey,
};
use destack_repository::{ArtifactReader, ProfileId, ProviderContext};
use destack_source::ModuleId;
use indexmap::{IndexMap, IndexSet};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{Compiler, CompilerError, CompilerResult};

/// Predecessor module graph used to derive one new module graph.
struct ModuleGraphBase {
    /// The predecessor module graph payload.
    graph: Arc<ModuleGraph>,
    /// Modules added since the predecessor graph.
    added_modules: Vec<ModuleId>,
    /// Modules removed since the predecessor graph.
    removed_modules: Vec<ModuleId>,
    /// Modules whose resolved component edges changed.
    changed_modules: FxHashSet<ModuleId>,
}

/// Exact dependency fingerprints for one module graph.
struct ModuleGraphDependencies {
    /// Resolved component edge fingerprints by module.
    modules: FxHashMap<ModuleId, ArtifactProjectionFingerprint>,
}

impl ModuleGraphBase {
    /// Build one predecessor graph and its changed inputs.
    fn new(
        graph: Arc<ModuleGraph>,
        previous: &[ArtifactDependency],
        current: &[ArtifactDependency],
    ) -> CompilerResult<Self> {
        let previous =
            ModuleGraphDependencies::read(previous).ok_or_else(|| CompilerError::Internal {
                message: "module graph predecessor has malformed dependencies".to_owned(),
            })?;
        let current =
            ModuleGraphDependencies::read(current).ok_or_else(|| CompilerError::Internal {
                message: "module graph provider has malformed dependencies".to_owned(),
            })?;
        if !previous.matches_modules(graph.modules()) {
            return Err(CompilerError::Internal {
                message: "module graph predecessor dependencies differ from its modules".to_owned(),
            });
        }

        // classify added, removed, and changed module rows
        let mut added_modules = current
            .modules
            .keys()
            .filter(|module| !previous.modules.contains_key(module))
            .copied()
            .collect::<Vec<_>>();
        let mut removed_modules = previous
            .modules
            .keys()
            .filter(|module| !current.modules.contains_key(module))
            .copied()
            .collect::<Vec<_>>();
        let mut changed_modules = FxHashSet::default();
        for (module, fingerprint) in &current.modules {
            let Some(previous) = previous.modules.get(module) else {
                continue;
            };
            if fingerprint != previous {
                changed_modules.insert(*module);
            }
        }
        added_modules.sort_unstable();
        removed_modules.sort_unstable();

        Ok(Self {
            graph,
            added_modules,
            removed_modules,
            changed_modules,
        })
    }
}

impl ModuleGraphDependencies {
    /// Read the module graph provider's dependency rows.
    fn read(dependencies: &[ArtifactDependency]) -> Option<Self> {
        let mut modules = FxHashMap::default();
        let mut is_module_set_observed = false;

        // collect the edge projection for every module
        for dependency in dependencies {
            match dependency {
                ArtifactDependency::Projection(dependency) => {
                    let projection = dependency.projection();
                    let module = projection.artifact.module_id()?;
                    if projection.key != ArtifactProjectionKey::ImportEdges {
                        return None;
                    }
                    if modules.insert(module, dependency.fingerprint()).is_some() {
                        return None;
                    }
                }
                ArtifactDependency::Source(source)
                    if source.key == SourceDependencyKey::Modules && !is_module_set_observed =>
                {
                    is_module_set_observed = true;
                }
                ArtifactDependency::Artifact(_) | ArtifactDependency::Source(_) => return None,
            }
        }
        if !is_module_set_observed {
            return None;
        }

        Some(Self { modules })
    }

    /// Return whether these rows cover one exact module set.
    fn matches_modules(&self, modules: &[ModuleId]) -> bool {
        self.modules.len() == modules.len()
            && modules
                .iter()
                .all(|module| self.modules.contains_key(module))
    }
}

impl Compiler {
    /// Collect inputs for the module graph of one profile.
    pub(crate) fn collect_module_graph(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();

        // import edges read every module's resolved relationships
        let modules = self.repository.module_ids(context.revision())?;
        dependencies.observe_modules(&modules);
        for module in modules {
            dependencies.require_projection(
                ArtifactKey::dir_resolved(module, profile),
                ArtifactProjectionKey::ImportEdges,
            );
        }

        Ok(dependencies)
    }

    /// Return the predecessor graph usable for one module graph build.
    fn module_graph_base(
        &self,
        context: &dyn ProviderContext,
    ) -> CompilerResult<Option<ModuleGraphBase>> {
        let Some(base) = context.artifact_base() else {
            return Ok(None);
        };

        let graph = self
            .repository
            .artifact_table()
            .artifact::<ModuleGraph>(&base.version)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("module graph base is missing: {:?}", base.version),
            })?;

        let dependencies =
            context
                .artifact_dependencies()
                .ok_or_else(|| CompilerError::Internal {
                    message: "module graph provider has no frozen dependencies".to_string(),
                })?;
        let base = ModuleGraphBase::new(graph, &base.dependencies, dependencies)?;

        Ok(Some(base))
    }

    /// Build the module graph for one profile.
    pub(crate) fn provide_module_graph(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifacts = self.artifact_reader(context);
        let started = self.repository.host().clock().now();
        let modules = self.repository.module_ids(context.revision())?;
        if let Some(started) = started {
            context.emit_span("provide.modules", started);
        }
        context.emit_counter("graph.modules", modules.len() as u64);

        // build or reuse the profile graph
        let base = self.module_graph_base(context)?;
        let graph = self.module_graph(&artifacts, profile, context, &modules, base)?;

        Ok(ArtifactPayload::ModuleGraph(graph))
    }

    /// Return the module graph for one profile.
    fn module_graph(
        &self,
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        context: &dyn ProviderContext,
        modules: &[ModuleId],
        base: Option<ModuleGraphBase>,
    ) -> CompilerResult<Arc<ModuleGraph>> {
        let started = self.repository.host().clock().now();
        let mut edge_count = 0u64;
        let module_set = modules.iter().copied().collect::<FxHashSet<_>>();

        // build all edges when no predecessor graph is available
        let Some(base) = base else {
            let mut extensions = Vec::new();
            let mut edges_by_module = IndexMap::with_capacity(modules.len());

            // collect every module's extensions and import edges
            for module in modules.iter().copied() {
                self.collect_inherent_extensions(artifacts, profile, module, &mut extensions)?;
                let edges = self.module_edges(artifacts, profile, module, &module_set)?;

                edge_count += edges.len() as u64;
                edges_by_module.insert(module, edges);
            }
            extensions.sort_unstable();
            extensions.dedup();

            if let Some(started) = started {
                context.emit_span("provide.edges", started);
            }
            context.emit_counter("graph.edges", edge_count);

            let graph = ModuleGraph::from_edges(profile, edges_by_module, extensions).map_err(
                |module| CompilerError::Internal {
                    message: format!(
                        "module graph edge references a module outside its profile: {module:?}"
                    ),
                },
            )?;

            return Ok(Arc::new(graph));
        };

        // include newly added modules in every changed input set
        let mut changed_modules = base.changed_modules;
        changed_modules.extend(base.added_modules.iter().copied());
        let removed_modules = base
            .removed_modules
            .iter()
            .copied()
            .collect::<FxHashSet<_>>();

        // derive the complete extension list from only changed resolution rows
        let mut extensions = base
            .graph
            .extensions()
            .iter()
            .filter(|extension| {
                !changed_modules.contains(&extension.symbol.module_id)
                    && !removed_modules.contains(&extension.symbol.module_id)
            })
            .copied()
            .collect::<Vec<_>>();
        for module in changed_modules.iter().copied() {
            self.collect_inherent_extensions(artifacts, profile, module, &mut extensions)?;
        }
        extensions.sort_unstable();
        extensions.dedup();
        let is_extensions_changed = extensions.as_slice() != base.graph.extensions();

        // reread import rows with changed resolved relationships
        let mut changed_edges = IndexMap::with_capacity(changed_modules.len());
        let mut is_edges_changed =
            !base.added_modules.is_empty() || !base.removed_modules.is_empty();
        for module in changed_modules.iter().copied() {
            let edges = self.module_edges(artifacts, profile, module, &module_set)?;

            edge_count += edges.len() as u64;
            is_edges_changed |= !base.graph.edges_equal(module, edges.as_ref());
            changed_edges.insert(module, edges);
        }

        let changed = changed_modules.len();
        let reused = modules.len().saturating_sub(changed);

        context.emit_counter("graph.changed", changed as u64);
        context.emit_counter("graph.added", base.added_modules.len() as u64);
        context.emit_counter("graph.removed", base.removed_modules.len() as u64);
        context.emit_counter("graph.reused", reused as u64);
        if let Some(started) = started {
            context.emit_span("provide.edges", started);
        }
        context.emit_counter("graph.edges", edge_count);

        // return the predecessor graph when its inputs still match
        if !is_edges_changed && !is_extensions_changed {
            return Ok(base.graph);
        }

        let graph = base
            .graph
            .derive(changed_edges, base.removed_modules, extensions)
            .map_err(|module| CompilerError::Internal {
                message: format!(
                    "derived module graph references a module outside its profile: {module:?}"
                ),
            })?;

        Ok(Arc::new(graph))
    }

    /// Collect one module's exported extensions kept inherent by target ownership.
    fn collect_inherent_extensions(
        &self,
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        module: ModuleId,
        inherent: &mut Vec<InherentExtension>,
    ) -> CompilerResult<()> {
        let resolved = artifacts
            .read_projection::<DirResolved>((module, profile), ArtifactProjectionKey::ImportEdges)
            .map_err(CompilerError::from)?;

        for (symbol, target) in resolved.extensions.targets() {
            // keep extensions of another package's target import-scoped
            if target.module_id.package_id != module.package_id {
                continue;
            }

            inherent.push(InherentExtension { symbol, target });
        }

        Ok(())
    }

    /// Return the modules one module depends on, deduplicated in order.
    fn module_edges(
        &self,
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        module: ModuleId,
        modules: &FxHashSet<ModuleId>,
    ) -> CompilerResult<Arc<[ModuleId]>> {
        let resolved = artifacts
            .read_projection::<DirResolved>((module, profile), ArtifactProjectionKey::ImportEdges)
            .map_err(CompilerError::from)?;

        // collect the defining modules of resolved targets
        let mut edges = IndexSet::new();
        for target in resolved.target_modules() {
            if modules.contains(&target) && target != module {
                edges.insert(target);
            }
        }

        let edges = edges.into_iter().collect::<Vec<_>>();

        Ok(Arc::from(edges))
    }
}
