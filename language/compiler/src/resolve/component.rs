use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, ArtifactVersion, ComponentGraph,
};
use destack_core::DenseGraph;
use destack_repository::{ArtifactReader, ProfileId, ProviderContext};
use destack_source::{ComponentId, ModuleId};
use indexmap::{IndexMap, IndexSet};
use std::sync::Arc;

use crate::{Compiler, CompilerError, CompilerResult};

/// Predecessor component graph used to derive one new component graph.
struct ComponentGraphBase {
    /// The predecessor artifact version.
    version: ArtifactVersion,
    /// The predecessor component graph payload.
    graph: Arc<ComponentGraph>,
    /// Modules whose resolved imports changed since the predecessor revision.
    changed_modules: Vec<ModuleId>,
}

impl ComponentGraphBase {
    /// Return predecessor imports from one module.
    fn imports(&self, module: ModuleId) -> CompilerResult<&Arc<[ModuleId]>> {
        self.graph
            .imports
            .get(&module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("module {module:?} is absent from base component graph imports"),
            })
    }
}

/// Import rows for one component graph build.
enum ComponentGraphInput {
    /// The predecessor graph is still valid.
    Unchanged(Arc<ComponentGraph>),
    /// Import rows changed and graph partitioning must run.
    Changed(IndexMap<ModuleId, Arc<[ModuleId]>>),
}

impl Compiler {
    /// Collect inputs for the component partition of one profile.
    pub(crate) fn collect_component_graph(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let base = self.component_graph_base(context)?;

        let mut dependencies = ArtifactDependencySet::default();
        if let Some(base) = &base {
            dependencies.derive_from(base.version);
        }

        // reuse the predecessor dependency set when the module set is stable
        if let Some(base) = base {
            for module in base.changed_modules {
                dependencies.require(ArtifactKey::dir_resolved(module, profile));
            }
        } else {
            // collect every import row when no predecessor graph is usable
            let modules = self
                .repository
                .module_ids(context.revision())
                .map_err(|error| CompilerError::Internal {
                    message: format!("failed to enumerate profile modules: {error}"),
                })?;

            for module in modules {
                dependencies.require(ArtifactKey::dir_resolved(module, profile));
            }
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

        let Some(graph) = self
            .repository
            .artifact_table()
            .component_graph(&base.version)
        else {
            return Ok(None);
        };

        let Some(changed) = self
            .repository
            .changed_module_ids_between(context.revision(), base.revision)
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to read changed modules: {error}"),
            })?
        else {
            return Ok(None);
        };

        Ok(Some(ComponentGraphBase {
            version: base.version,
            graph,
            changed_modules: changed,
        }))
    }

    /// Build the component partition for one profile.
    pub(crate) fn provide_component_graph(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifacts = self.artifact_reader(context.revision());
        let started = self.repository.host().clock().now();
        let modules = self
            .repository
            .module_ids(context.revision())
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to enumerate profile modules: {error}"),
            })?;
        if let Some(started) = started {
            context.emit_span("modules", started);
        }
        context.emit_counter("modules", modules.len() as u64);

        // compare changed import rows against the predecessor graph
        let base = self.component_graph_base(context)?;
        let imports = match self.module_imports(&artifacts, profile, context, &modules, base)? {
            ComponentGraphInput::Unchanged(graph) => {
                return Ok(ArtifactPayload::ComponentGraph(graph));
            }
            ComponentGraphInput::Changed(imports) => imports,
        };

        // partition modules into strongly connected components
        let started = self.repository.host().clock().now();
        let sccs = strongly_connected_components(&imports);
        let component_count = sccs.len() as u64;
        if let Some(started) = started {
            context.emit_span("scc", started);
        }
        context.emit_counter("components", component_count);

        // assign component ids from each strongly connected component
        let mut members = IndexMap::<ComponentId, Vec<ModuleId>>::new();
        let mut component_of = IndexMap::<ModuleId, ComponentId>::new();
        for scc in sccs {
            let component = ComponentId::from_modules(profile, scc.iter().copied());
            for module in &scc {
                component_of.insert(*module, component);
            }
            members.insert(component, scc);
        }

        // derive the condensation: edges crossing component boundaries
        let started = self.repository.host().clock().now();
        let mut dependencies = IndexMap::<ComponentId, Vec<ComponentId>>::new();
        let mut edge_count = 0u64;
        for (module, edges) in &imports {
            let component =
                component_of
                    .get(module)
                    .copied()
                    .ok_or_else(|| CompilerError::Internal {
                        message: format!("module {module:?} is absent from component ownership"),
                    })?;
            let dependents = dependencies.entry(component).or_default();
            for edge in edges.iter() {
                let Some(target) = component_of.get(edge).copied() else {
                    continue;
                };
                if target != component && !dependents.contains(&target) {
                    dependents.push(target);
                    edge_count += 1;
                }
            }
        }
        if let Some(started) = started {
            context.emit_span("condensation", started);
        }
        context.emit_counter("edges", edge_count);

        Ok(ArtifactPayload::ComponentGraph(Arc::new(ComponentGraph {
            profile,
            imports,
            component_of,
            members,
            dependencies,
        })))
    }

    /// Return changed profile import rows, or the base graph when it still matches.
    fn module_imports(
        &self,
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        context: &dyn ProviderContext,
        modules: &[ModuleId],
        base: Option<ComponentGraphBase>,
    ) -> CompilerResult<ComponentGraphInput> {
        let started = self.repository.host().clock().now();
        let mut edge_count = 0u64;

        // build all rows when no predecessor graph is available
        let Some(base) = base else {
            let mut imports = IndexMap::with_capacity(modules.len());

            for module in modules.iter().copied() {
                let edges = self.module_edges(artifacts, profile, module)?;
                edge_count += edges.len() as u64;
                imports.insert(module, edges);
            }

            if let Some(started) = started {
                context.emit_span("edges", started);
            }
            context.emit_counter("edges", edge_count);

            return Ok(ComponentGraphInput::Changed(imports));
        };

        let changed = base.changed_modules.len();
        let reused = modules.len().saturating_sub(changed);

        context.emit_counter("changed_modules", changed as u64);
        context.emit_counter("reused_modules", reused as u64);

        let mut changed_rows = IndexMap::with_capacity(base.changed_modules.len());
        let mut is_changed = false;

        // compare only rows whose resolved imports changed
        for module in base.changed_modules.iter().copied() {
            let edges = self.module_edges(artifacts, profile, module)?;
            let base_edges = base.imports(module)?;

            edge_count += edges.len() as u64;
            is_changed |= base_edges.as_ref() != edges.as_ref();
            changed_rows.insert(module, edges);
        }

        if let Some(started) = started {
            context.emit_span("edges", started);
        }
        context.emit_counter("edges", edge_count);

        // return the predecessor graph when changed rows still match
        if !is_changed {
            return Ok(ComponentGraphInput::Unchanged(base.graph));
        }

        // assemble a full graph only when partitioning must run
        let mut imports = IndexMap::with_capacity(modules.len());
        for module in modules.iter().copied() {
            if let Some(edges) = changed_rows.get(&module) {
                imports.insert(module, edges.clone());
            } else {
                let edges = base.imports(module)?;
                imports.insert(module, edges.clone());
            }
        }

        Ok(ComponentGraphInput::Changed(imports))
    }

    /// Return the modules one module depends on, deduplicated in order.
    fn module_edges(
        &self,
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        module: ModuleId,
    ) -> CompilerResult<Arc<[ModuleId]>> {
        let resolved = artifacts
            .dir_resolved(module, profile)
            .map_err(CompilerError::from)?;

        // collect resolved dependencies without self imports
        let mut edges = IndexSet::new();
        for target in resolved.imports.modules() {
            if target != module {
                edges.insert(target);
            }
        }

        let edges = edges.into_iter().collect::<Vec<_>>();

        Ok(Arc::from(edges))
    }
}

/// Return the strongly connected components of one module import graph.
fn strongly_connected_components(
    edges: &IndexMap<ModuleId, Arc<[ModuleId]>>,
) -> Vec<Vec<ModuleId>> {
    // assign dense ids in artifact order
    let modules = edges.keys().copied().collect::<Vec<_>>();
    let module_index = modules
        .iter()
        .enumerate()
        .map(|(index, module)| (*module, index as u32))
        .collect::<IndexMap<_, _>>();

    // count in-graph imports per module
    let mut edge_offsets = vec![0u32; modules.len() + 1];
    for (source, module) in modules.iter().copied().enumerate() {
        let count = edges
            .get(&module)
            .map(Arc::as_ref)
            .unwrap_or_default()
            .iter()
            .filter(|target| module_index.contains_key(*target))
            .count();
        edge_offsets[source + 1] = count as u32;
    }

    // prefix sum counts into CSR offsets
    for source in 0..modules.len() {
        edge_offsets[source + 1] += edge_offsets[source];
    }

    // copy import targets in source import order
    let mut edge_targets = Vec::with_capacity(edge_offsets[modules.len()] as usize);
    for module in &modules {
        let imports = edges.get(module).map(Arc::as_ref).unwrap_or_default();
        for target in imports {
            if let Some(target_index) = module_index.get(target).copied() {
                edge_targets.push(target_index);
            }
        }
    }

    // partition dense module graph
    let graph = DenseGraph::new(&edge_offsets, &edge_targets);
    let partition = graph.strongly_connected_components();

    // rebuild sorted module components from dense ids
    let mut components = vec![Vec::new(); partition.component_count() as usize];
    for (index, module) in modules.into_iter().enumerate() {
        let component = partition.component(index) as usize;
        components[component].push(module);
    }
    for component in &mut components {
        component.sort_unstable();
    }

    components
}
