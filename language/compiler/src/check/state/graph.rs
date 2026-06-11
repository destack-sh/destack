use destack_source::{ComponentId, ModuleId, ProfileId};
use indexmap::{IndexMap, IndexSet};

use crate::{CompilerError, CompilerResult};

/// Resolved module dependency graph for component discovery.
pub(in crate::check) struct CheckComponentGraph {
    /// Forward dependency edges keyed by source module.
    edges: IndexMap<ModuleId, Vec<ModuleId>>,
}

/// Artifact coordinates for one checked component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct CheckComponentArtifact {
    /// The checked component entry module.
    pub entry: ModuleId,
    /// The checked component id.
    pub component: ComponentId,
}

impl CheckComponentGraph {
    /// Create a resolved dependency graph.
    pub(in crate::check) fn new(edges: IndexMap<ModuleId, Vec<ModuleId>>) -> Self {
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

    /// Return external modules reached from one component.
    pub(in crate::check) fn external_modules(&self, component: &[ModuleId]) -> Vec<ModuleId> {
        let component = component.iter().copied().collect::<IndexSet<_>>();
        let mut external_modules = IndexSet::new();

        // collect unique edges leaving the component
        for module in &component {
            let Some(module_dependencies) = self.edges.get(module) else {
                continue;
            };

            for external_module in module_dependencies {
                if !component.contains(external_module) {
                    external_modules.insert(*external_module);
                }
            }
        }

        let mut external_modules = external_modules.into_iter().collect::<Vec<_>>();
        external_modules.sort_unstable();
        external_modules
    }

    /// Return outgoing component artifacts from one component.
    pub(in crate::check) fn external_components(
        &self,
        profile: ProfileId,
        component: &[ModuleId],
    ) -> CompilerResult<IndexMap<ModuleId, CheckComponentArtifact>> {
        let external_modules = self.external_modules(component);
        let mut components = IndexMap::new();

        // map every outgoing module edge to its owning component
        for external_module in external_modules {
            components.insert(
                external_module,
                self.component_artifact(profile, external_module)?,
            );
        }

        Ok(components)
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

    /// Return checked component coordinates for one module in this graph.
    pub(in crate::check) fn component_artifact(
        &self,
        profile: ProfileId,
        module: ModuleId,
    ) -> CompilerResult<CheckComponentArtifact> {
        let component_modules = self.component(module);
        let entry = component_modules
            .first()
            .copied()
            .ok_or_else(|| CompilerError::Internal {
                message: format!("checked external module {module:?} has no component"),
            })?;
        let component = ComponentId::from_modules(profile, component_modules.iter().copied());

        Ok(CheckComponentArtifact { entry, component })
    }
}
