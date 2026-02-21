use std::collections::VecDeque;

use crate::{AnalyzeError, AnalyzeResult, Compiler, TaskDependencyError};
use destack_source::ModuleId;
use destack_workspace::{ModuleGraph, ModuleGraphKey, ProfileId};
use rustc_hash::{FxHashMap, FxHashSet};

/// One resolved interface component execution plan.
#[derive(Debug)]
pub(super) struct InterfaceComponentPlan {
    /// All modules that belong to the strongly connected component.
    pub(super) component_modules: Vec<ModuleId>,
    /// Dependency component anchors outside the component boundary.
    pub(super) dependency_modules: Vec<ModuleId>,
}

/// Canonical interface component index for one graph snapshot.
#[derive(Debug, Default)]
struct InterfaceComponentGraphIndex {
    /// Component id for each module.
    module_to_component: FxHashMap<ModuleId, usize>,
    /// Modules in each component.
    component_modules: Vec<Vec<ModuleId>>,
    /// Canonical anchor module for each component.
    component_anchors: Vec<ModuleId>,
    /// Dependency anchors for each component.
    component_dependency_anchors: Vec<Vec<ModuleId>>,
}

impl InterfaceComponentGraphIndex {
    /// Return the component id for one module when present.
    fn component_id_for_module(&self, module_id: ModuleId) -> Option<usize> {
        self.module_to_component.get(&module_id).copied()
    }

    /// Return all modules in the module's component.
    fn component_modules_for_module(&self, module_id: ModuleId) -> Option<&[ModuleId]> {
        let component_id = self.component_id_for_module(module_id)?;
        self.component_modules.get(component_id).map(Vec::as_slice)
    }

    /// Return the canonical component anchor for one module.
    fn component_anchor_for_module(&self, module_id: ModuleId) -> Option<ModuleId> {
        let component_id = self.component_id_for_module(module_id)?;
        self.component_anchors.get(component_id).copied()
    }

    /// Return dependency anchors for the module's component.
    fn component_dependency_anchors_for_module(&self, module_id: ModuleId) -> Option<&[ModuleId]> {
        let component_id = self.component_id_for_module(module_id)?;
        self.component_dependency_anchors
            .get(component_id)
            .map(Vec::as_slice)
    }
}

impl Compiler {
    /// Ensure direct resolve edges exist for one module's forward dependency closure.
    pub(super) fn require_resolve_forward_closure_for_interface_component(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        let key = ModuleGraphKey::new(profile);
        let mut pending = VecDeque::from([module_id]);
        let mut visited = FxHashSet::default();

        while let Some(pending_module_id) = pending.pop_front() {
            if !visited.insert(pending_module_id) {
                continue;
            }

            self.require_resolve_module_canonical(pending_module_id, profile)?;

            if let Some(graph) = self.program.index.module_graphs.get(&key) {
                for dependency_module_id in graph.dependencies_for(pending_module_id) {
                    if !visited.contains(&dependency_module_id) {
                        pending.push_back(dependency_module_id);
                    }
                }
            }
        }

        Ok(())
    }

    /// Select a stable module id anchor for the interface component.
    pub(super) fn interface_component_anchor_module_id(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> ModuleId {
        let key = ModuleGraphKey::new(profile);
        let Some(graph) = self.program.index.module_graphs.get(&key) else {
            return module_id;
        };

        let index = self.interface_component_graph_index(&graph);
        index
            .component_anchor_for_module(module_id)
            .unwrap_or(module_id)
    }

    /// Resolve one strict execution plan for one interface component.
    pub(super) fn interface_component_plan(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> AnalyzeResult<InterfaceComponentPlan> {
        // require a graph snapshot for strict component ownership
        let key = ModuleGraphKey::new(profile);
        let graph = self
            .program
            .index
            .module_graphs
            .get(&key)
            .ok_or(AnalyzeError::Internal {
                message: format!("missing interface graph snapshot for profile {profile:?}"),
            })?;

        // collect strongly connected modules and component dependencies
        let index = self.interface_component_graph_index(&graph);
        let component_modules = index
            .component_modules_for_module(module_id)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| vec![module_id]);
        let dependency_modules = index
            .component_dependency_anchors_for_module(module_id)
            .map(ToOwned::to_owned)
            .unwrap_or_default();

        Ok(InterfaceComponentPlan {
            component_modules,
            dependency_modules,
        })
    }

    /// Return true when two modules belong to the same interface component.
    pub(crate) fn interface_modules_share_component(
        &self,
        profile: ProfileId,
        left_module_id: ModuleId,
        right_module_id: ModuleId,
    ) -> bool {
        let key = ModuleGraphKey::new(profile);
        let Some(graph) = self.program.index.module_graphs.get(&key) else {
            return left_module_id == right_module_id;
        };

        let index = self.interface_component_graph_index(&graph);
        let left_component_id = index.component_id_for_module(left_module_id);
        let right_component_id = index.component_id_for_module(right_module_id);
        match (left_component_id, right_component_id) {
            (Some(left_component_id), Some(right_component_id)) => {
                left_component_id == right_component_id
            }
            _ => left_module_id == right_module_id,
        }
    }

    /// Build a canonical interface component index for one graph snapshot.
    fn interface_component_graph_index(&self, graph: &ModuleGraph) -> InterfaceComponentGraphIndex {
        // collect all known graph modules
        let mut modules = FxHashSet::default();
        for module_id in graph.module_versions.keys().copied() {
            modules.insert(module_id);
        }
        for (module_id, dependencies) in &graph.dependencies {
            modules.insert(*module_id);
            for dependency_module_id in dependencies.iter().copied() {
                modules.insert(dependency_module_id);
            }
        }
        for (module_id, dependents) in &graph.dependents {
            modules.insert(*module_id);
            for dependent_module_id in dependents.iter().copied() {
                modules.insert(dependent_module_id);
            }
        }

        // partition graph modules into strongly connected components
        let mut sorted_modules: Vec<_> = modules.into_iter().collect();
        sorted_modules.sort_unstable();
        let mut unassigned: FxHashSet<_> = sorted_modules.iter().copied().collect();
        let mut components = Vec::new();
        for seed_module_id in sorted_modules.iter().copied() {
            if !unassigned.contains(&seed_module_id) {
                continue;
            }

            // intersect forward and backward reachability sets
            let reachable_forward = self.interface_graph_reachable_forward(graph, seed_module_id);
            let reachable_backward = self.interface_graph_reachable_backward(graph, seed_module_id);
            let mut component_modules: Vec<_> = reachable_forward
                .into_iter()
                .filter(|module_id| {
                    unassigned.contains(module_id) && reachable_backward.contains(module_id)
                })
                .collect();
            if !component_modules.contains(&seed_module_id) {
                component_modules.push(seed_module_id);
            }
            component_modules.sort_unstable();

            // mark component modules as assigned
            for component_module_id in component_modules.iter().copied() {
                unassigned.remove(&component_module_id);
            }

            components.push(component_modules);
        }

        // keep deterministic component order by canonical anchor
        components.sort_unstable_by_key(|modules| modules.first().copied());

        // index modules by component id and anchor
        let mut module_to_component = FxHashMap::default();
        let mut component_anchors = Vec::new();
        for (component_id, component_modules) in components.iter().enumerate() {
            let Some(component_anchor) = component_modules.first().copied() else {
                continue;
            };
            component_anchors.push(component_anchor);
            for component_module_id in component_modules.iter().copied() {
                module_to_component.insert(component_module_id, component_id);
            }
        }

        // collect dependency anchors for each component
        let mut component_dependency_anchors = Vec::with_capacity(components.len());
        for (component_id, component_modules) in components.iter().enumerate() {
            let mut dependency_component_ids = FxHashSet::default();
            for component_module_id in component_modules.iter().copied() {
                for dependency_module_id in graph.dependencies_for(component_module_id) {
                    let Some(dependency_component_id) =
                        module_to_component.get(&dependency_module_id).copied()
                    else {
                        continue;
                    };
                    if dependency_component_id != component_id {
                        dependency_component_ids.insert(dependency_component_id);
                    }
                }
            }

            let mut dependency_anchors: Vec<_> = dependency_component_ids
                .into_iter()
                .filter_map(|dependency_component_id| {
                    component_anchors.get(dependency_component_id).copied()
                })
                .collect();
            dependency_anchors.sort_unstable();
            component_dependency_anchors.push(dependency_anchors);
        }

        InterfaceComponentGraphIndex {
            module_to_component,
            component_modules: components,
            component_anchors,
            component_dependency_anchors,
        }
    }

    /// Collect modules reachable from one module along dependency edges.
    fn interface_graph_reachable_forward(
        &self,
        graph: &ModuleGraph,
        module_id: ModuleId,
    ) -> FxHashSet<ModuleId> {
        // traverse dependency edges from this module
        let mut visited = FxHashSet::default();
        let mut pending = VecDeque::new();
        pending.push_back(module_id);
        while let Some(current_module_id) = pending.pop_front() {
            if !visited.insert(current_module_id) {
                continue;
            }

            for dependency_module_id in graph.dependencies_for(current_module_id) {
                pending.push_back(dependency_module_id);
            }
        }

        visited
    }

    /// Collect modules that can reach one module along dependency edges.
    fn interface_graph_reachable_backward(
        &self,
        graph: &ModuleGraph,
        module_id: ModuleId,
    ) -> FxHashSet<ModuleId> {
        // traverse reverse dependency edges into this module
        let mut visited = FxHashSet::default();
        let mut pending = VecDeque::new();
        pending.push_back(module_id);
        while let Some(current_module_id) = pending.pop_front() {
            if !visited.insert(current_module_id) {
                continue;
            }

            for dependent_module_id in graph.dependents_for(current_module_id) {
                pending.push_back(dependent_module_id);
            }
        }

        visited
    }
}
