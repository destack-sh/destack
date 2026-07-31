use std::sync::Arc;

use destack_artifact::{
    ArtifactDependency, ArtifactDependencySet, ArtifactKey, ArtifactPayload,
    ArtifactProjectionFingerprint, ArtifactProjectionKey, ComponentGraph, InherentExtension,
    SourceDependencyKey,
};
use destack_dir as dir;
use destack_repository::{ArtifactReader, ProfileId, ProviderContext};
use destack_source::ModuleId;
use indexmap::{IndexMap, IndexSet};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{Compiler, CompilerError, CompilerResult};

/// Predecessor component graph used to derive one new component graph.
struct ComponentGraphBase {
    /// The predecessor component graph payload.
    graph: Arc<ComponentGraph>,
    /// Modules added since the predecessor graph.
    added_modules: Vec<ModuleId>,
    /// Modules removed since the predecessor graph.
    removed_modules: Vec<ModuleId>,
    /// Modules whose resolved component edges changed.
    changed_modules: FxHashSet<ModuleId>,
    /// Whether any inference export projection changed.
    is_inference_exports_changed: bool,
}

/// Exact dependency fingerprints for one component graph.
struct ComponentGraphDependencies {
    /// Dependency fingerprints by module.
    modules: FxHashMap<ModuleId, ModuleGraphDependencies>,
}

/// Exact component graph dependency fingerprints for one module.
#[derive(Clone, Copy)]
struct ModuleGraphDependencies {
    /// The resolved component edge fingerprint.
    component_edges: ArtifactProjectionFingerprint,
    /// The exported inference fingerprint.
    inference_exports: ArtifactProjectionFingerprint,
}

impl ComponentGraphBase {
    /// Build one predecessor graph and its changed inputs.
    fn new(
        graph: Arc<ComponentGraph>,
        previous: &[ArtifactDependency],
        current: &[ArtifactDependency],
    ) -> CompilerResult<Self> {
        let previous =
            ComponentGraphDependencies::read(previous).ok_or_else(|| CompilerError::Internal {
                message: "component graph predecessor has malformed dependencies".to_owned(),
            })?;
        let current =
            ComponentGraphDependencies::read(current).ok_or_else(|| CompilerError::Internal {
                message: "component graph provider has malformed dependencies".to_owned(),
            })?;
        if !previous.matches_modules(graph.modules()) {
            return Err(CompilerError::Internal {
                message: "component graph predecessor dependencies differ from its modules"
                    .to_owned(),
            });
        }

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
        let mut is_inference_exports_changed = false;

        // classify changed module rows
        for (module, dependencies) in &current.modules {
            let Some(previous) = previous.modules.get(module) else {
                continue;
            };
            if dependencies.component_edges != previous.component_edges {
                changed_modules.insert(*module);
            }
            if dependencies.inference_exports != previous.inference_exports {
                is_inference_exports_changed = true;
            }
        }
        added_modules.sort_unstable();
        removed_modules.sort_unstable();

        Ok(Self {
            graph,
            added_modules,
            removed_modules,
            changed_modules,
            is_inference_exports_changed,
        })
    }
}

impl ComponentGraphDependencies {
    /// Read the component graph provider's dependency rows.
    fn read(dependencies: &[ArtifactDependency]) -> Option<Self> {
        let mut component_edges = Vec::new();
        let mut inference_exports = Vec::new();
        let mut is_module_set_observed = false;

        // collect both projections for every module
        for dependency in dependencies {
            match dependency {
                ArtifactDependency::Projection(dependency) => {
                    let projection = dependency.projection();
                    let module = projection.artifact.module_id()?;
                    let fingerprints = match projection.key {
                        ArtifactProjectionKey::DirComponentEdges => &mut component_edges,
                        ArtifactProjectionKey::DirInferenceExports => &mut inference_exports,
                        _ => return None,
                    };
                    fingerprints.push((module, dependency.fingerprint()));
                }
                ArtifactDependency::Source(source)
                    if source.key == SourceDependencyKey::Modules && !is_module_set_observed =>
                {
                    is_module_set_observed = true;
                }
                ArtifactDependency::Artifact(_) | ArtifactDependency::Source(_) => return None,
            }
        }
        if !is_module_set_observed || component_edges.len() != inference_exports.len() {
            return None;
        }

        component_edges.sort_unstable_by_key(|(module, _fingerprint)| *module);
        inference_exports.sort_unstable_by_key(|(module, _fingerprint)| *module);

        // join each module's exact projection fingerprints
        let mut modules = FxHashMap::default();
        for ((module, component_edges), (inference_module, inference_exports)) in
            component_edges.into_iter().zip(inference_exports)
        {
            if module != inference_module {
                return None;
            }
            let dependencies = ModuleGraphDependencies {
                component_edges,
                inference_exports,
            };
            if modules.insert(module, dependencies).is_some() {
                return None;
            }
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
    /// Collect inputs for the component partition of one profile.
    pub(crate) fn collect_component_graph(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();

        // inference edges read every module's resolution and exports
        let modules = self.repository.module_ids(context.revision())?;
        dependencies.observe_modules(&modules);
        for module in modules {
            dependencies.require_projection(
                ArtifactKey::dir_resolved(module, profile),
                ArtifactProjectionKey::DirComponentEdges,
            );
            dependencies.require_projection(
                ArtifactKey::dir_exported(module, profile),
                ArtifactProjectionKey::DirInferenceExports,
            );
        }

        Ok(dependencies)
    }

    /// Return the predecessor graph usable for one component graph build.
    fn component_graph_base(
        &self,
        context: &dyn ProviderContext,
    ) -> CompilerResult<Option<ComponentGraphBase>> {
        let Some(base) = context.artifact_base() else {
            return Ok(None);
        };

        let graph = self
            .repository
            .artifact_table()
            .component_graph(&base.version)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("component graph base is missing: {:?}", base.version),
            })?;

        let dependencies =
            context
                .artifact_dependencies()
                .ok_or_else(|| CompilerError::Internal {
                    message: "component graph provider has no frozen dependencies".to_string(),
                })?;
        let base = ComponentGraphBase::new(graph, &base.dependencies, dependencies)?;

        Ok(Some(base))
    }

    /// Build the component partition for one profile.
    pub(crate) fn provide_component_graph(
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
        let base = self.component_graph_base(context)?;
        let graph = self.component_graph(&artifacts, profile, context, &modules, base)?;

        Ok(ArtifactPayload::ComponentGraph(graph))
    }

    /// Return the component graph for one profile.
    fn component_graph(
        &self,
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        context: &dyn ProviderContext,
        modules: &[ModuleId],
        base: Option<ComponentGraphBase>,
    ) -> CompilerResult<Arc<ComponentGraph>> {
        let started = self.repository.host().clock().now();
        let mut edge_count = 0u64;
        let module_set = modules.iter().copied().collect::<FxHashSet<_>>();

        // build all edges when no predecessor graph is available
        let Some(base) = base else {
            let mut extensions = Vec::new();
            let mut inference_edges = IndexMap::with_capacity(modules.len());
            let mut edges_by_module = IndexMap::with_capacity(modules.len());

            // collect every extension before classifying inference consumers
            for module in modules.iter().copied() {
                self.collect_inherent_extensions(artifacts, profile, module, &mut extensions)?;
            }
            extensions.sort_unstable();
            extensions.dedup();

            // classify every module's inference and reference edges
            for module in modules.iter().copied() {
                let inference = self.module_inference_edges(
                    artifacts,
                    profile,
                    module,
                    extensions.as_slice(),
                    &module_set,
                )?;
                let edges = self.module_edges(artifacts, profile, module, &module_set)?;

                edge_count += edges.len() as u64;
                inference_edges.insert(module, inference);
                edges_by_module.insert(module, edges);
            }

            if let Some(started) = started {
                context.emit_span("provide.edges", started);
            }
            context.emit_counter("graph.edges", edge_count);

            let graph =
                ComponentGraph::from_edges(profile, edges_by_module, inference_edges, extensions)
                    .map_err(|module| CompilerError::Internal {
                    message: format!(
                        "component graph edge references a module outside its profile: \
                             {module:?}"
                    ),
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

        // reread inference rows affected by resolution, exports, or extensions
        let is_all_inference_dirty = base.is_inference_exports_changed || is_extensions_changed;
        let mut inference_edges = IndexMap::with_capacity(modules.len());
        let mut is_inference_changed = false;
        for module in modules.iter().copied() {
            let edges = if is_all_inference_dirty || changed_modules.contains(&module) {
                self.module_inference_edges(
                    artifacts,
                    profile,
                    module,
                    extensions.as_slice(),
                    &module_set,
                )?
            } else {
                base.graph
                    .inference_edges(module)
                    .ok_or_else(|| CompilerError::Internal {
                        message: format!(
                            "component graph predecessor does not contain module {module:?}"
                        ),
                    })?
            };
            is_inference_changed |= !base.graph.inference_edges_equal(module, edges.as_ref());
            inference_edges.insert(module, edges);
        }

        // reread reference rows with changed resolved relationships
        let mut changed_edges = IndexMap::with_capacity(changed_modules.len());
        let mut is_reference_changed =
            !base.added_modules.is_empty() || !base.removed_modules.is_empty();
        for module in changed_modules.iter().copied() {
            let edges = self.module_edges(artifacts, profile, module, &module_set)?;

            edge_count += edges.len() as u64;
            is_reference_changed |= !base.graph.reference_edges_equal(module, edges.as_ref());
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
        if !is_reference_changed && !is_inference_changed && !is_extensions_changed {
            return Ok(base.graph);
        }

        let graph = base
            .graph
            .derive(
                changed_edges,
                base.removed_modules,
                inference_edges,
                extensions,
            )
            .map_err(|module| CompilerError::Internal {
                message: format!(
                    "derived component graph references a module outside its profile: {module:?}"
                ),
            })?;

        Ok(Arc::new(graph))
    }

    /// Return the modules whose inference one module's checking consumes.
    fn module_inference_edges(
        &self,
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        module: ModuleId,
        inherent: &[InherentExtension],
        modules: &FxHashSet<ModuleId>,
    ) -> CompilerResult<Arc<[ModuleId]>> {
        let resolved = artifacts
            .dir_resolved_projection(module, profile, ArtifactProjectionKey::DirComponentEdges)
            .map_err(CompilerError::from)?;
        let mut edges = IndexSet::new();
        let mut referenced_symbols = IndexSet::new();

        // referenced symbols depend on inferred export forms
        for (source, reference) in &resolved.references.target_by_node {
            // dependency declarations identify names without consuming their types
            if source.local_id.ty == dir::NodeType::DependencyItem {
                continue;
            }

            match reference {
                dir::Reference::Bound(symbols) => {
                    for symbol in symbols {
                        referenced_symbols.insert(*symbol);
                        if modules.contains(&symbol.module_id)
                            && symbol.module_id != module
                            && self.export_requires_inference(artifacts, profile, *symbol)?
                        {
                            edges.insert(symbol.module_id);
                        }
                    }
                }
                dir::Reference::Projected { base, .. } => {
                    referenced_symbols.insert(*base);
                    if modules.contains(&base.module_id)
                        && base.module_id != module
                        && self.export_requires_inference(artifacts, profile, *base)?
                    {
                        edges.insert(base.module_id);
                    }
                }
                dir::Reference::Namespace(namespace) => {
                    if modules.contains(namespace)
                        && *namespace != module
                        && self.namespace_requires_inference(artifacts, profile, *namespace)?
                    {
                        edges.insert(*namespace);
                    }
                }
                dir::Reference::Ambiguous(_) | dir::Reference::Missing => {}
            }
        }

        // inferred inherent extensions add dependencies from target consumers
        for extension in inherent {
            if extension.symbol.module_id != module
                && referenced_symbols.contains(&extension.target)
                && self.export_requires_inference(artifacts, profile, extension.symbol)?
            {
                edges.insert(extension.symbol.module_id);
            }
        }

        let edges = edges.into_iter().collect::<Vec<_>>();

        Ok(Arc::from(edges))
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
            .dir_resolved_projection(module, profile, ArtifactProjectionKey::DirComponentEdges)
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

    /// Return whether one exported symbol carries inference to its consumers.
    fn export_requires_inference(
        &self,
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let exported = artifacts
            .dir_exported_projection(
                symbol.module_id,
                profile,
                ArtifactProjectionKey::DirInferenceExports,
            )
            .map_err(CompilerError::from)?;

        // require every resolved cross-module symbol to carry one canonical form
        let form = exported
            .exports
            .symbol_form(symbol.local_id)
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "resolved cross-module symbol {symbol:?} has no local export form"
                ),
            })?;

        Ok(form.requires_inference())
    }

    /// Return whether one namespace object carries any inference.
    fn namespace_requires_inference(
        &self,
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        namespace: ModuleId,
    ) -> CompilerResult<bool> {
        let exported = artifacts
            .dir_exported_projection(
                namespace,
                profile,
                ArtifactProjectionKey::DirInferenceExports,
            )
            .map_err(CompilerError::from)?;

        // any inferred local export requires namespace inference
        Ok(exported.exports.exports().any(|(_, export)| match export {
            dir::NamedExport::Local(local) => local.form.requires_inference(),
            dir::NamedExport::Indirect(_) => false,
        }))
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
            .dir_resolved_projection(module, profile, ArtifactProjectionKey::DirComponentEdges)
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
