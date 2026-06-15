use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, ComponentGraph, ModuleIndex,
};
use destack_core::DenseGraph;
use destack_repository::{ArtifactReader, ProfileId, ProviderContext};
use destack_source::{ComponentId, ModuleId};
use indexmap::{IndexMap, IndexSet};
use std::sync::Arc;

use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Collect inputs for the module import edge index of one profile.
    pub(crate) fn collect_module_index(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let modules = self
            .repository
            .module_ids(context.revision())
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to enumerate profile modules: {error}"),
            })?;

        let mut dependencies = ArtifactDependencySet::default();
        for module in modules {
            dependencies.require(ArtifactKey::dir_resolved(module, profile));
        }

        Ok(dependencies)
    }

    /// Build the module import edge index for one profile.
    pub(crate) fn provide_module_index(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifacts = self.artifact_reader(context.revision());
        let modules = self
            .repository
            .module_ids(context.revision())
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to enumerate profile modules: {error}"),
            })?;

        // record every module's resolved import edges
        let mut imports = IndexMap::<ModuleId, Vec<ModuleId>>::new();
        for module in modules {
            let edges = self.module_edges(&artifacts, profile, module)?;
            imports.insert(module, edges);
        }

        Ok(ArtifactPayload::ModuleIndex(Arc::new(ModuleIndex {
            profile,
            imports,
        })))
    }

    /// Return the modules one module depends on, deduplicated in order.
    fn module_edges(
        &self,
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        module: ModuleId,
    ) -> CompilerResult<Vec<ModuleId>> {
        let resolved = artifacts
            .dir_resolved(module, profile)
            .map_err(CompilerError::from)?;

        // collect resolved module dependencies, dropping self imports
        let mut edges = IndexSet::new();
        for target in resolved.imports.modules() {
            if target != module {
                edges.insert(target);
            }
        }

        Ok(edges.into_iter().collect())
    }

    /// Collect inputs for the component partition of one profile.
    pub(crate) fn collect_component_graph(
        &self,
        profile: ProfileId,
        _context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::module_index(profile));

        Ok(dependencies)
    }

    /// Build the component partition for one profile.
    pub(crate) fn provide_component_graph(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifacts = self.artifact_reader(context.revision());
        let index = artifacts
            .module_index(profile)
            .map_err(CompilerError::from)?;

        // partition modules into strongly connected components
        let sccs = strongly_connected_components(&index.imports);
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
        let mut dependencies = IndexMap::<ComponentId, Vec<ComponentId>>::new();
        for (module, edges) in &index.imports {
            let Some(component) = component_of.get(module).copied() else {
                continue;
            };
            let dependents = dependencies.entry(component).or_default();
            for edge in edges {
                let Some(target) = component_of.get(edge).copied() else {
                    continue;
                };
                if target != component && !dependents.contains(&target) {
                    dependents.push(target);
                }
            }
        }

        Ok(ArtifactPayload::ComponentGraph(Arc::new(ComponentGraph {
            profile,
            component_of,
            members,
            dependencies,
        })))
    }
}

/// Return the strongly connected components of one module import graph.
fn strongly_connected_components(edges: &IndexMap<ModuleId, Vec<ModuleId>>) -> Vec<Vec<ModuleId>> {
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
            .map(Vec::as_slice)
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
        let imports = edges.get(module).map(Vec::as_slice).unwrap_or_default();
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
