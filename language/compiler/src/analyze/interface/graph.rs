use crate::{Compiler, RequirementError};
use destack_artifact::ModuleGraph;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Revision};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;
use std::sync::Arc;

/// Canonical interface component index for one graph snapshot.
#[derive(Debug, Default)]
pub(crate) struct InterfaceComponentGraphIndex {
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
    pub(super) fn component_id_for_module(&self, module_id: ModuleId) -> Option<usize> {
        self.module_to_component.get(&module_id).copied()
    }

    /// Return all modules in the module's component.
    pub(crate) fn component_modules_for_module(&self, module_id: ModuleId) -> Option<&[ModuleId]> {
        let component_id = self.component_id_for_module(module_id)?;
        self.component_modules.get(component_id).map(Vec::as_slice)
    }

    /// Return the canonical component anchor for one module.
    fn component_anchor_for_module(&self, module_id: ModuleId) -> Option<ModuleId> {
        let component_id = self.component_id_for_module(module_id)?;
        self.component_anchors.get(component_id).copied()
    }

    /// Return dependency anchors for the module's component.
    pub(crate) fn component_dependency_anchors_for_module(
        &self,
        module_id: ModuleId,
    ) -> Option<&[ModuleId]> {
        let component_id = self.component_id_for_module(module_id)?;
        self.component_dependency_anchors
            .get(component_id)
            .map(Vec::as_slice)
    }
}

impl Compiler {
    /// Ensure resolved dependency edges exist for one module set's transitive closure.
    pub(crate) fn require_resolved_dependency_closure(
        &self,
        revision: Revision,
        modules: impl IntoIterator<Item = ModuleId>,
        profile: ProfileId,
    ) -> Result<(), RequirementError> {
        let mut pending = modules.into_iter().collect::<VecDeque<_>>();
        let mut visited = FxHashSet::default();

        while let Some(pending_module_id) = pending.pop_front() {
            if !visited.insert(pending_module_id) {
                continue;
            }

            self.require_dir_resolved(revision, pending_module_id, profile)?;

            if let Some(graph) = self.module_graph(profile) {
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
        let Some(index) = self.interface_component_graph_index(profile) else {
            return module_id;
        };

        index
            .component_anchor_for_module(module_id)
            .unwrap_or(module_id)
    }

    /// Return the canonical interface component index for the current graph snapshot.
    pub(super) fn interface_component_graph_index(
        &self,
        profile: ProfileId,
    ) -> Option<Arc<InterfaceComponentGraphIndex>> {
        let graph = self.module_graph(profile)?;

        // reuse the cached index while the current graph snapshot is unchanged
        if let Some(entry) = self.index.interface_component_graph_indices.get(&profile)
            && (Arc::ptr_eq(&entry.value().0, &graph)
                || entry.value().0.matches_snapshot(graph.as_ref()))
        {
            return Some(entry.value().1.clone());
        }

        // collect all modules that participate in this graph snapshot
        let modules = self.interface_graph_module_domain(&graph);
        if modules.is_empty() {
            return Some(Arc::new(InterfaceComponentGraphIndex::default()));
        }

        // compute strongly connected components once for this module domain
        let components = self.interface_graph_scc(&graph, &modules);
        if components.is_empty() {
            return Some(Arc::new(InterfaceComponentGraphIndex::default()));
        }

        // schedule components in deterministic topological order
        let component_order = self.interface_graph_component_order(&graph, &components);
        let mut ordered_components = Vec::with_capacity(component_order.len());
        for component_id in component_order {
            if let Some(component_modules) = components.get(component_id) {
                ordered_components.push(component_modules.clone());
            }
        }

        // index modules by component id and canonical anchor
        let mut module_to_component = FxHashMap::default();
        let mut component_anchors = Vec::with_capacity(ordered_components.len());
        for (component_id, component_modules) in ordered_components.iter().enumerate() {
            let Some(component_anchor) = component_modules.first().copied() else {
                continue;
            };
            component_anchors.push(component_anchor);
            for component_module_id in component_modules.iter().copied() {
                module_to_component.insert(component_module_id, component_id);
            }
        }

        // collect dependency anchors for each component
        let mut component_dependency_anchors = Vec::with_capacity(ordered_components.len());
        for (component_id, component_modules) in ordered_components.iter().enumerate() {
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

        let index = Arc::new(InterfaceComponentGraphIndex {
            module_to_component,
            component_modules: ordered_components,
            component_anchors,
            component_dependency_anchors,
        });
        self.index
            .interface_component_graph_indices
            .insert(profile, (graph, index.clone()));

        Some(index)
    }

    /// Collect one deterministic module domain from the graph snapshot.
    fn interface_graph_module_domain(&self, graph: &ModuleGraph) -> Vec<ModuleId> {
        let mut modules = FxHashSet::default();
        for module_id in graph.modules.keys().copied() {
            modules.insert(module_id);
        }
        for (module_id, metadata) in &graph.modules {
            modules.insert(*module_id);
            for dependency in metadata.dependencies.iter().copied() {
                modules.insert(dependency.target);
            }
        }
        for (module_id, dependents) in &graph.dependents {
            modules.insert(*module_id);
            for dependent in dependents.iter().copied() {
                modules.insert(dependent.target);
            }
        }

        let mut modules: Vec<_> = modules.into_iter().collect();
        modules.sort_unstable();
        modules
    }

    /// Compute strongly connected components for one graph domain.
    fn interface_graph_scc(&self, graph: &ModuleGraph, modules: &[ModuleId]) -> Vec<Vec<ModuleId>> {
        let module_domain: FxHashSet<_> = modules.iter().copied().collect();

        // first pass: collect finish order on dependency edges
        let mut visited = FxHashSet::default();
        let mut finish_order = Vec::with_capacity(modules.len());
        for module_id in modules.iter().copied() {
            if visited.contains(&module_id) {
                continue;
            }

            let mut pending = vec![(module_id, false)];
            while let Some((current_module_id, expanded)) = pending.pop() {
                if expanded {
                    finish_order.push(current_module_id);
                    continue;
                }
                if !visited.insert(current_module_id) {
                    continue;
                }

                pending.push((current_module_id, true));

                let mut dependencies: Vec<_> = graph
                    .dependencies_for(current_module_id)
                    .into_iter()
                    .filter(|dependency_module_id| module_domain.contains(dependency_module_id))
                    .collect();
                dependencies.sort_unstable_by(|left, right| right.cmp(left));
                for dependency_module_id in dependencies {
                    if !visited.contains(&dependency_module_id) {
                        pending.push((dependency_module_id, false));
                    }
                }
            }
        }

        // second pass: collect components on reverse edges
        let mut assigned = FxHashSet::default();
        let mut components = Vec::new();
        for module_id in finish_order.into_iter().rev() {
            if !assigned.insert(module_id) {
                continue;
            }

            let mut component = Vec::new();
            let mut pending = vec![module_id];
            while let Some(current_module_id) = pending.pop() {
                component.push(current_module_id);

                let mut dependents: Vec<_> = graph
                    .dependents_for(current_module_id)
                    .into_iter()
                    .filter(|dependent_module_id| module_domain.contains(dependent_module_id))
                    .collect();
                dependents.sort_unstable_by(|left, right| right.cmp(left));
                for dependent_module_id in dependents {
                    if assigned.insert(dependent_module_id) {
                        pending.push(dependent_module_id);
                    }
                }
            }

            component.sort_unstable();
            components.push(component);
        }

        components
    }

    /// Build one deterministic topological component order.
    fn interface_graph_component_order(
        &self,
        graph: &ModuleGraph,
        components: &[Vec<ModuleId>],
    ) -> Vec<usize> {
        let mut module_to_component = FxHashMap::default();
        let mut component_anchors = Vec::with_capacity(components.len());
        for (component_id, component_modules) in components.iter().enumerate() {
            let Some(component_anchor) = component_modules.first().copied() else {
                continue;
            };
            component_anchors.push(component_anchor);
            for component_module_id in component_modules.iter().copied() {
                module_to_component.insert(component_module_id, component_id);
            }
        }

        // dependency edges are: dependency component -> dependent component
        let mut component_edges: Vec<FxHashSet<usize>> =
            vec![FxHashSet::default(); components.len()];
        let mut indegree = vec![0usize; components.len()];
        for (component_id, component_modules) in components.iter().enumerate() {
            for component_module_id in component_modules.iter().copied() {
                for dependency_module_id in graph.dependencies_for(component_module_id) {
                    let Some(dependency_component_id) =
                        module_to_component.get(&dependency_module_id).copied()
                    else {
                        continue;
                    };
                    if dependency_component_id == component_id {
                        continue;
                    }
                    if component_edges[dependency_component_id].insert(component_id) {
                        indegree[component_id] += 1;
                    }
                }
            }
        }

        // stable kahn schedule with anchor-first tie-breaking
        let mut ready: Vec<_> = indegree
            .iter()
            .enumerate()
            .filter_map(|(component_id, degree)| {
                if *degree == 0 {
                    Some(component_id)
                } else {
                    None
                }
            })
            .collect();
        let mut order = Vec::with_capacity(components.len());
        while !ready.is_empty() {
            let mut next_index = 0usize;
            for index in 1..ready.len() {
                let candidate_component_id = ready[index];
                let current_component_id = ready[next_index];
                if component_anchors[candidate_component_id]
                    < component_anchors[current_component_id]
                {
                    next_index = index;
                }
            }
            let component_id = ready.swap_remove(next_index);
            order.push(component_id);

            let mut dependents: Vec<_> = component_edges[component_id].iter().copied().collect();
            dependents.sort_unstable_by_key(|dependent_id| component_anchors[*dependent_id]);
            for dependent_component_id in dependents {
                assert!(
                    indegree[dependent_component_id] > 0,
                    "interface component edge invariant violated: component {dependent_component_id} underflowed indegree"
                );
                indegree[dependent_component_id] -= 1;
                if indegree[dependent_component_id] == 0 {
                    ready.push(dependent_component_id);
                }
            }
        }

        // component condensation must be a DAG, so kahn must schedule all components
        assert_eq!(
            order.len(),
            components.len(),
            "interface component graph order must include every component"
        );

        order
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use destack_artifact::ArtifactKey;

    use crate::TestProgram;

    /// Rebuild the cached interface component index when the module graph snapshot changes.
    #[test]
    fn test_interface_component_graph_index_rebuilds_after_graph_change() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "main.ds",
            r#"
export const value = 1;
"#,
        );

        // build and cache the initial interface graph index
        test.resolve_module(module_id);
        test.compile_check_clean();

        let profile = test.default_profile_id(module_id);
        let initial_index = test
            .compiler
            .interface_component_graph_index(profile)
            .unwrap_or_else(|| panic!("expected interface component graph index"));

        // publish a distinct module graph snapshot for the same profile
        let mut graph = test
            .compiler
            .module_graph(profile)
            .unwrap_or_else(|| panic!("expected module graph for test profile"))
            .as_ref()
            .clone();
        let graph_version = test.compiler.artifact_version_for_revision(
            test.program.current_revision(),
            &ArtifactKey::module_graph(profile),
        );
        let module_version = graph
            .module_version_for(module_id)
            .unwrap_or_else(|| panic!("expected module graph version for {module_id:?}"));
        graph.update_module_dependencies(
            module_id,
            destack_artifact::ModuleKind::Code,
            module_version,
            vec![module_id],
        );
        test.compiler
            .artifacts
            .publish_module_graph(graph_version, graph);

        let rebuilt_index = test
            .compiler
            .interface_component_graph_index(profile)
            .unwrap_or_else(|| panic!("expected rebuilt interface component graph index"));

        // a changed graph snapshot must not reuse the old cached index
        assert!(!Arc::ptr_eq(&initial_index, &rebuilt_index));
    }
}
