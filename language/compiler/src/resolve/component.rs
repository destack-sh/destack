use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, ComponentGraph, ModuleIndex,
};
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

/// Iterative Tarjan state for strongly connected component discovery.
/// Keeping deep import chains off the call stack avoids recursion limits.
struct TarjanState<'a> {
    /// The module import edge map.
    edges: &'a IndexMap<ModuleId, Vec<ModuleId>>,
    /// The next preorder index.
    index: u32,
    /// The preorder index per reached module.
    indices: IndexMap<ModuleId, u32>,
    /// The lowest reachable index per reached module.
    low_links: IndexMap<ModuleId, u32>,
    /// Whether each reached module is still on the open stack.
    on_stack: IndexMap<ModuleId, bool>,
    /// The open Tarjan stack.
    stack: Vec<ModuleId>,
    /// The discovered components, each sorted.
    components: Vec<Vec<ModuleId>>,
}

/// Return the strongly connected components of one module import graph.
/// Each component's members sort ascending, with components emitted in
/// reverse topological order.
fn strongly_connected_components(edges: &IndexMap<ModuleId, Vec<ModuleId>>) -> Vec<Vec<ModuleId>> {
    let mut tarjan = TarjanState {
        edges,
        index: 0,
        indices: IndexMap::new(),
        low_links: IndexMap::new(),
        on_stack: IndexMap::new(),
        stack: Vec::new(),
        components: Vec::new(),
    };

    // walk every module with an explicit frame stack
    for root in edges.keys().copied() {
        if tarjan.indices.contains_key(&root) {
            continue;
        }

        // one frame per open module: the module and its next edge offset
        let mut frames: Vec<(ModuleId, usize)> = vec![(root, 0)];
        while let Some((module, edge)) = frames.last().copied() {
            if edge == 0 {
                tarjan.indices.insert(module, tarjan.index);
                tarjan.low_links.insert(module, tarjan.index);
                tarjan.index += 1;
                tarjan.stack.push(module);
                tarjan.on_stack.insert(module, true);
            }

            // descend into the next unvisited in-graph dependency
            let targets = tarjan
                .edges
                .get(&module)
                .map(|targets| targets.as_slice())
                .unwrap_or(&[]);
            let mut descended = false;
            let mut offset = edge;
            while let Some(target) = targets.get(offset).copied() {
                offset += 1;
                if !tarjan.edges.contains_key(&target) {
                    continue;
                }
                if let Some(target_index) = tarjan.indices.get(&target) {
                    // back edges into the open stack lower this link
                    if tarjan.on_stack.get(&target).copied().unwrap_or(false) {
                        let low = (*tarjan.low_links.get(&module).unwrap()).min(*target_index);
                        tarjan.low_links.insert(module, low);
                    }
                    continue;
                }

                frames.last_mut().unwrap().1 = offset;
                frames.push((target, 0));
                descended = true;
                break;
            }
            if descended {
                continue;
            }
            frames.last_mut().unwrap().1 = offset;

            // close the frame, popping a finished component at its root
            let low = *tarjan.low_links.get(&module).unwrap();
            if low == *tarjan.indices.get(&module).unwrap() {
                let mut component = Vec::new();
                while let Some(member) = tarjan.stack.pop() {
                    tarjan.on_stack.insert(member, false);
                    component.push(member);
                    if member == module {
                        break;
                    }
                }
                component.sort_unstable();
                tarjan.components.push(component);
            }
            frames.pop();

            // propagate the closed link into the parent frame
            if let Some((parent, _)) = frames.last().copied() {
                let parent_low = (*tarjan.low_links.get(&parent).unwrap()).min(low);
                tarjan.low_links.insert(parent, parent_low);
            }
        }
    }

    tarjan.components
}
