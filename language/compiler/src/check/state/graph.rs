use destack_repository::ProviderContext;
use destack_source::{ModuleId, ProfileId};
use indexmap::{IndexMap, IndexSet};

use crate::{Compiler, CompilerError, CompilerResult};

/// Resolved module dependency graph for component discovery.
pub(in crate::check) struct CheckComponentGraph {
    /// Forward dependency edges keyed by source module.
    edges: IndexMap<ModuleId, Vec<ModuleId>>,
}

impl CheckComponentGraph {
    /// Create a resolved dependency graph.
    fn new(edges: IndexMap<ModuleId, Vec<ModuleId>>) -> Self {
        Self { edges }
    }

    /// Return the strongly connected component containing one module.
    pub(in crate::check) fn component(&self, module: ModuleId) -> Vec<ModuleId> {
        let reverse_edges = self.reverse_edges();
        let reachable_from_module = self.reachable(module, &self.edges);
        let reachable_to_module = self.reachable(module, &reverse_edges);
        let mut modules = Vec::new();

        // keep modules mutually reachable with the entry module
        for module in self.edges.keys().copied() {
            if reachable_from_module.contains(&module) && reachable_to_module.contains(&module) {
                modules.push(module);
            }
        }

        modules.sort_unstable();
        modules
    }

    /// Return outgoing dependencies from one component.
    pub(in crate::check) fn dependencies(&self, component: &[ModuleId]) -> Vec<ModuleId> {
        let component = component.iter().copied().collect::<IndexSet<_>>();
        let mut dependencies = IndexSet::new();

        // collect unique edges leaving the component
        for module in &component {
            let Some(module_dependencies) = self.edges.get(module) else {
                continue;
            };

            for dependency in module_dependencies {
                if !component.contains(dependency) {
                    dependencies.insert(*dependency);
                }
            }
        }

        let mut dependencies = dependencies.into_iter().collect::<Vec<_>>();
        dependencies.sort_unstable();
        dependencies
    }

    /// Return reverse dependency edges for loaded modules.
    fn reverse_edges(&self) -> IndexMap<ModuleId, Vec<ModuleId>> {
        let mut reverse_edges = IndexMap::new();

        // include every loaded module in the reverse graph
        for module in self.edges.keys().copied() {
            reverse_edges.entry(module).or_insert_with(Vec::new);
        }

        // reverse edges that point to loaded modules
        for (source, targets) in &self.edges {
            for target in targets {
                if self.edges.contains_key(target) {
                    reverse_edges
                        .entry(*target)
                        .or_insert_with(Vec::new)
                        .push(*source);
                }
            }
        }

        reverse_edges
    }

    /// Return modules reachable from one module through the provided edges.
    fn reachable(
        &self,
        module: ModuleId,
        edges: &IndexMap<ModuleId, Vec<ModuleId>>,
    ) -> IndexSet<ModuleId> {
        let mut reachable = IndexSet::new();
        let mut pending = vec![module];

        // walk graph without recursion
        while let Some(module) = pending.pop() {
            if !reachable.insert(module) {
                continue;
            }
            let Some(targets) = edges.get(&module) else {
                continue;
            };

            for target in targets.iter().rev() {
                if !reachable.contains(target) {
                    pending.push(*target);
                }
            }
        }

        reachable
    }
}

impl Compiler {
    /// Load the resolved dependency graph reachable from one module.
    pub(in crate::check) fn collect_check_component_graph(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<CheckComponentGraph> {
        let edges = self.collect_check_component_closure(module, profile, context)?;

        Ok(CheckComponentGraph::new(edges))
    }

    /// Load resolved dependency edges reachable from one module.
    fn collect_check_component_closure(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<IndexMap<ModuleId, Vec<ModuleId>>> {
        let artifacts = self.artifact_reader(context);
        let mut graph = IndexMap::new();
        let mut pending = vec![module];

        // load each resolved module once
        while let Some(module) = pending.pop() {
            if graph.contains_key(&module) {
                continue;
            }

            let resolved = artifacts
                .dir_resolved(module, profile)
                .map_err(CompilerError::from)?;
            let dependencies = resolved.imports.dependencies.clone();

            // schedule dependencies before committing this node
            for dependency in dependencies.iter().rev() {
                if !graph.contains_key(dependency) {
                    pending.push(*dependency);
                }
            }

            graph.insert(module, dependencies);
        }

        Ok(graph)
    }
}
