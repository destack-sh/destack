use std::sync::Arc;

use indexmap::{IndexMap, IndexSet};
use rustc_hash::{FxHashMap, FxHashSet};
use tspp_artifact::{
    ArtifactDependency, ArtifactDependencySet, ArtifactKey, ArtifactPayload,
    ArtifactProjectionFingerprint, ArtifactProjectionKey, DirResolved, Implementation, ModuleGraph,
    SourceDependencyKey,
};
use tspp_repository::{ArtifactReader, ProfileId, ProviderContext};
use tspp_source::{ModuleId, PackageId};

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

        // classify added, removed, and changed module entries
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
    /// Read the module graph provider's dependency entries.
    fn read(dependencies: &[ArtifactDependency]) -> Option<Self> {
        let mut modules = FxHashMap::default();
        let mut is_module_set_observed = false;

        // collect the edge projection for every module
        for dependency in dependencies {
            match dependency {
                ArtifactDependency::Projection(dependency) => {
                    let projection = dependency.projection();
                    let module = projection.artifact.module_id()?;
                    if projection.key != ArtifactProjectionKey::DirResolvedComponentRelations {
                        return None;
                    }
                    if modules.insert(module, dependency.fingerprint()).is_some() {
                        return None;
                    }
                }
                ArtifactDependency::Source(source)
                    if matches!(source.key(), SourceDependencyKey::PackageModules(_))
                        && !is_module_set_observed =>
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

    /// Return whether these entries cover one exact module set.
    fn matches_modules(&self, modules: &[ModuleId]) -> bool {
        self.modules.len() == modules.len()
            && modules
                .iter()
                .all(|module| self.modules.contains_key(module))
    }
}

impl Compiler {
    /// Collect inputs for the module graph of one package under one profile.
    pub(crate) fn collect_module_graph(
        &self,
        package: PackageId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();

        // read the resolved imports of every module in the package
        let modules = self
            .repository
            .package_module_ids(context.revision(), package)?;
        dependencies.observe_package_modules(package, &modules);
        for module in modules {
            dependencies.require_projection(
                ArtifactKey::dir_resolved(module, profile),
                ArtifactProjectionKey::DirResolvedComponentRelations,
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

        let graph = base
            .artifact::<ModuleGraph>()
            .ok_or_else(|| CompilerError::Internal {
                message: format!("module graph base is missing: {:?}", base.version()),
            })?;

        let dependencies =
            context
                .artifact_dependencies()
                .ok_or_else(|| CompilerError::Internal {
                    message: "module graph provider has no frozen dependencies".to_string(),
                })?;
        let base = ModuleGraphBase::new(graph, base.dependencies(), dependencies)?;

        Ok(Some(base))
    }

    /// Build the module graph of one package under one profile.
    pub(crate) fn provide_module_graph(
        &self,
        package: PackageId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifacts = self.artifact_reader(context);
        let started = self.repository.host().clock().now();
        let modules = self
            .repository
            .package_module_ids(context.revision(), package)?;
        if let Some(started) = started {
            context.record_span("provide.modules", started);
        }
        context.record_counter("graph.modules", modules.len() as u64);

        // build or reuse the profile graph
        let base = self.module_graph_base(context)?;
        let graph = self.module_graph(&artifacts, package, profile, context, &modules, base)?;

        Ok(ArtifactPayload::ModuleGraph(graph))
    }

    /// Return the module graph of one package under one profile.
    fn module_graph(
        &self,
        artifacts: &ArtifactReader<'_>,
        package: PackageId,
        profile: ProfileId,
        context: &dyn ProviderContext,
        modules: &[ModuleId],
        base: Option<ModuleGraphBase>,
    ) -> CompilerResult<Arc<ModuleGraph>> {
        let started = self.repository.host().clock().now();
        let mut edge_count = 0u64;

        // build all edges when no predecessor graph is available
        let Some(base) = base else {
            let mut implementations = Vec::new();
            let mut edges_by_module = IndexMap::with_capacity(modules.len());

            // collect every module's implementations and import edges
            for module in modules.iter().copied() {
                let selected = self.collect_implementations(artifacts, profile, module)?;
                implementations.extend(selected);
                let edges = self.module_edges(artifacts, profile, module)?;

                edge_count += edges.len() as u64;
                edges_by_module.insert(module, edges);
            }

            if let Some(started) = started {
                context.record_span("provide.edges", started);
            }
            context.record_counter("graph.edges", edge_count);

            let graph = ModuleGraph::from_edges(package, profile, edges_by_module, implementations)
                .map_err(|module| CompilerError::Internal {
                    message: format!("module graph edge references an unknown module: {module:?}"),
                })?;

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

        // derive the complete implementation list from only changed resolution entries
        let mut implementations = base
            .graph
            .implementations()
            .iter()
            .filter(|implementation| {
                !changed_modules.contains(&implementation.symbol.module_id)
                    && !removed_modules.contains(&implementation.symbol.module_id)
            })
            .copied()
            .collect::<Vec<_>>();
        for module in changed_modules.iter().copied() {
            let selected = self.collect_implementations(artifacts, profile, module)?;
            implementations.extend(selected);
        }
        implementations.sort_unstable();
        implementations.dedup();
        let is_implementations_changed = implementations.as_slice() != base.graph.implementations();

        // reread import entries with changed resolved relationships
        let mut changed_edges = IndexMap::with_capacity(changed_modules.len());
        let mut is_edges_changed =
            !base.added_modules.is_empty() || !base.removed_modules.is_empty();
        for module in changed_modules.iter().copied() {
            let edges = self.module_edges(artifacts, profile, module)?;

            edge_count += edges.len() as u64;
            is_edges_changed |= !base.graph.edges_equal(module, edges.as_ref());
            changed_edges.insert(module, edges);
        }

        let changed = changed_modules.len();
        let reused = modules.len().saturating_sub(changed);

        context.record_counter("graph.changed", changed as u64);
        context.record_counter("graph.added", base.added_modules.len() as u64);
        context.record_counter("graph.removed", base.removed_modules.len() as u64);
        context.record_counter("graph.reused", reused as u64);
        if let Some(started) = started {
            context.record_span("provide.edges", started);
        }
        context.record_counter("graph.edges", edge_count);

        // return the predecessor graph when its inputs still match
        if !is_edges_changed && !is_implementations_changed {
            return Ok(base.graph);
        }

        let graph = base
            .graph
            .derive(changed_edges, base.removed_modules, implementations)
            .map_err(|module| CompilerError::Internal {
                message: format!("derived module graph references an unknown module: {module:?}"),
            })?;

        Ok(Arc::new(graph))
    }

    /// Collect one module's interface implementations.
    fn collect_implementations(
        &self,
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        module: ModuleId,
    ) -> CompilerResult<Vec<Implementation>> {
        // select interface implementations from the resolved relationships
        artifacts
            .project::<DirResolved, _, _>((module, profile), |resolved| {
                let implementations = resolved
                    .extensions
                    .implementations()
                    .map(|(symbol, root, interface)| Implementation {
                        interface,
                        symbol,
                        root,
                    })
                    .collect::<Vec<_>>();

                (
                    implementations,
                    [ArtifactProjectionKey::DirResolvedComponentRelations],
                )
            })
            .map_err(CompilerError::from)
    }

    /// Return the modules one module depends on, deduplicated in order.
    fn module_edges(
        &self,
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        module: ModuleId,
    ) -> CompilerResult<Arc<[ModuleId]>> {
        // select defining modules from the resolved relationships
        artifacts
            .project::<DirResolved, _, _>((module, profile), |resolved| {
                let edges = resolved
                    .target_modules()
                    .filter(|target| *target != module)
                    .collect::<IndexSet<_>>();
                let edges = edges.into_iter().collect::<Arc<[ModuleId]>>();

                (
                    edges,
                    [ArtifactProjectionKey::DirResolvedComponentRelations],
                )
            })
            .map_err(CompilerError::from)
    }
}
