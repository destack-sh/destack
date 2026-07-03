use destack_core::{DenseGraph, SccPartition};
use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use destack_serde::Reflect;
use destack_source::{ComponentId, ModuleId, ProfileId};

use crate::{ArtifactProjectionFingerprint, ComponentGraphProjection};

/// Dense module dependency graph for one profile.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ModuleGraph {
    /// The profile this graph belongs to.
    pub profile: ProfileId,
    /// Modules sorted by stable id.
    modules: Arc<[ModuleId]>,
    /// Per-module edge targets as dense module indexes.
    edges: Arc<[Arc<[u32]>]>,
}

/// Strongly connected component partition of one profile's module graph.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ComponentGraph {
    /// The dense module graph being partitioned.
    module_graph: ModuleGraph,
    /// Per-module owning component.
    component_of: Arc<[ComponentId]>,
    /// Components sorted by stable id.
    components: Arc<[ComponentId]>,
    /// Per-component member start offsets into `member_modules`.
    member_offsets: Arc<[u32]>,
    /// Component members as module ids.
    member_modules: Arc<[ModuleId]>,
    /// Component members as dense module indexes.
    member_indexes: Arc<[u32]>,
    /// Per-module dense component index.
    module_components: Arc<[u32]>,
    /// Per-component topological rank in the condensation graph.
    component_ranks: Arc<[u32]>,
    /// External components each component depends on.
    dependencies: Arc<[Arc<[ComponentId]>]>,
}

impl ModuleGraph {
    /// Build one module graph from complete in-profile module edges.
    pub fn from_edges(profile: ProfileId, edges: IndexMap<ModuleId, Arc<[ModuleId]>>) -> Self {
        // build dense module ids
        let mut modules = edges.keys().copied().collect::<Vec<_>>();
        modules.sort_unstable();
        let module_index = module_index_map(&modules);

        let mut graph_edges = Vec::with_capacity(modules.len());

        // write each module's outgoing edges as dense target indexes
        for module in &modules {
            let edges = edges.get(module).map(Arc::as_ref).unwrap_or_default();
            let mut targets = Vec::with_capacity(edges.len());
            for target in edges {
                if let Some(target) = module_index.get(target).copied() {
                    targets.push(target);
                }
            }
            graph_edges.push(Arc::from(targets));
        }

        Self {
            profile,
            modules: Arc::from(modules),
            edges: Arc::from(graph_edges),
        }
    }

    /// Derive a module graph after changing outgoing edges and removing modules.
    pub fn derive(
        &self,
        updated_edges: IndexMap<ModuleId, Arc<[ModuleId]>>,
        removed_modules: Vec<ModuleId>,
    ) -> Self {
        // keep unchanged graphs by identity
        if updated_edges.is_empty() && removed_modules.is_empty() {
            return self.clone();
        }

        // canonicalize removals for cheap membership checks
        let mut removed_modules = removed_modules;
        removed_modules.sort_unstable();
        removed_modules.dedup();

        if removed_modules.is_empty()
            && let Some(graph) = self.patch_edges(&updated_edges)
        {
            return graph;
        }

        self.derive_edges(updated_edges, &removed_modules)
    }

    /// Return the strongly connected components of this graph.
    fn strongly_connected_components(&self) -> SccPartition {
        let mut edge_offsets = Vec::with_capacity(self.modules.len() + 1);
        let edge_count = self.edges.iter().map(|edges| edges.len()).sum();
        let mut edge_targets = Vec::with_capacity(edge_count);

        // materialize transient CSR storage for Tarjan
        edge_offsets.push(0);
        for edges in self.edges.iter() {
            edge_targets.extend(edges.iter().copied());
            edge_offsets.push(edge_targets.len() as u32);
        }

        let graph = DenseGraph::new(&edge_offsets, &edge_targets);

        graph.strongly_connected_components()
    }

    /// Patch outgoing edges without changing the module universe.
    fn patch_edges(&self, updated_edges: &IndexMap<ModuleId, Arc<[ModuleId]>>) -> Option<Self> {
        let mut edges = self.edges.iter().cloned().collect::<Vec<_>>();

        // replace only changed module edge slices
        for (module, targets) in updated_edges {
            let source = self.module_index(*module)?;
            let mut target_indexes = Vec::with_capacity(targets.len());

            for target in targets.iter().copied() {
                target_indexes.push(self.module_index(target)? as u32);
            }

            edges[source] = Arc::from(target_indexes);
        }

        Some(Self {
            profile: self.profile,
            modules: self.modules.clone(),
            edges: Arc::from(edges),
        })
    }

    /// Return outgoing module edges for one module.
    pub fn edges(&self, module: ModuleId) -> Arc<[ModuleId]> {
        // return an empty edge list for modules outside the graph
        let Some(index) = self.module_index(module) else {
            return Arc::from([]);
        };

        let edges = self.edges[index]
            .iter()
            .map(|target| self.modules[*target as usize])
            .collect::<Vec<_>>();

        Arc::from(edges)
    }

    /// Return whether one module's outgoing edges equal an external edge list.
    pub fn edges_equal(&self, module: ModuleId, edges: &[ModuleId]) -> bool {
        // compare missing modules as empty edge lists
        let Some(index) = self.module_index(module) else {
            return edges.is_empty();
        };

        if self.edges[index].len() != edges.len() {
            return false;
        }

        self.edges[index]
            .iter()
            .zip(edges)
            .all(|(target, edge)| self.modules[*target as usize] == *edge)
    }

    /// Iterate over every module and its outgoing edges.
    pub fn iter_edges(&self) -> impl Iterator<Item = (ModuleId, Arc<[ModuleId]>)> + '_ {
        self.modules.iter().enumerate().map(|(index, module)| {
            let edges = self.edges[index]
                .iter()
                .map(|target| self.modules[*target as usize])
                .collect::<Vec<_>>();

            (*module, Arc::from(edges))
        })
    }

    /// Return whether the graph contains one module.
    pub fn contains_module(&self, module: ModuleId) -> bool {
        self.module_index(module).is_some()
    }

    /// Derive dense edges after applying changed module edges.
    fn derive_edges(
        &self,
        updated_edges: IndexMap<ModuleId, Arc<[ModuleId]>>,
        removed_modules: &[ModuleId],
    ) -> Self {
        // derive the sorted module universe
        let mut modules = Vec::with_capacity(self.modules.len() + updated_edges.len());

        for module in self.modules.iter().copied() {
            if removed_modules.binary_search(&module).is_err()
                && !updated_edges.contains_key(&module)
            {
                modules.push(module);
            }
        }

        for module in updated_edges.keys().copied() {
            if removed_modules.binary_search(&module).is_err() {
                modules.push(module);
            }
        }
        modules.sort_unstable();
        modules.dedup();

        // build dense module ids for the derived graph
        let module_index = module_index_map(&modules);
        let mut graph_edges = Vec::with_capacity(modules.len());

        // write old or changed edges directly into dense target ids
        for module in &modules {
            let mut targets = Vec::new();
            if let Some(edges) = updated_edges.get(module) {
                for target in edges.iter() {
                    if let Some(target) = module_index.get(target).copied() {
                        targets.push(target);
                    }
                }
            } else if let Some(source) = self.module_index(*module) {
                for target in self.edges[source].iter().copied() {
                    let target = self.modules[target as usize];
                    if let Some(target) = module_index.get(&target).copied() {
                        targets.push(target);
                    }
                }
            }

            graph_edges.push(Arc::from(targets));
        }

        Self {
            profile: self.profile,
            modules: Arc::from(modules),
            edges: Arc::from(graph_edges),
        }
    }

    /// Return the dense index of one module.
    fn module_index(&self, module: ModuleId) -> Option<usize> {
        self.modules.binary_search(&module).ok()
    }

    /// Return dense outgoing edge targets for one module index.
    fn edge_targets(&self, module: usize) -> &[u32] {
        &self.edges[module]
    }
}

impl ComponentGraph {
    /// Build one component graph from complete module edges.
    pub fn from_edges(profile: ProfileId, edges: IndexMap<ModuleId, Arc<[ModuleId]>>) -> Self {
        let module_graph = ModuleGraph::from_edges(profile, edges);

        Self::from_module_graph(module_graph)
    }

    /// Build one component graph from one module graph.
    pub fn from_module_graph(module_graph: ModuleGraph) -> Self {
        Self::build(module_graph)
    }

    /// Derive a component graph after changing outgoing edges and removing modules.
    pub fn derive(
        &self,
        updated_edges: IndexMap<ModuleId, Arc<[ModuleId]>>,
        removed_modules: Vec<ModuleId>,
    ) -> Self {
        // keep unchanged graphs by identity
        if updated_edges.is_empty() && removed_modules.is_empty() {
            return self.clone();
        }

        let changed_components =
            self.changed_dependency_components(&updated_edges, &removed_modules);
        let module_graph = self.module_graph.derive(updated_edges, removed_modules);

        if let Some(changed_components) = changed_components {
            return self.with_module_graph(module_graph, &changed_components);
        }

        Self::from_module_graph(module_graph)
    }

    /// Return outgoing module edges for one module.
    pub fn edges(&self, module: ModuleId) -> Arc<[ModuleId]> {
        self.module_graph.edges(module)
    }

    /// Return whether one module's outgoing edges equal an external edge list.
    pub fn edges_equal(&self, module: ModuleId, edges: &[ModuleId]) -> bool {
        self.module_graph.edges_equal(module, edges)
    }

    /// Iterate over every module and its outgoing edges.
    pub fn iter_edges(&self) -> impl Iterator<Item = (ModuleId, Arc<[ModuleId]>)> + '_ {
        self.module_graph.iter_edges()
    }

    /// Return whether the graph contains one module.
    pub fn contains_module(&self, module: ModuleId) -> bool {
        self.module_graph.contains_module(module)
    }

    /// Return the component containing one module.
    pub fn component(&self, module: ModuleId) -> Option<ComponentId> {
        let index = self.module_graph.module_index(module)?;

        Some(self.component_of[index])
    }

    /// Return the component and entry module containing one module.
    pub fn component_entry(&self, module: ModuleId) -> Option<(ComponentId, ModuleId)> {
        let component = self.component(module)?;
        let entry = self.entry(component)?;

        Some((component, entry))
    }

    /// Return the member modules of one component.
    pub fn members(&self, component: ComponentId) -> &[ModuleId] {
        let Some(index) = self.component_index(component) else {
            return &[];
        };

        &self.member_modules[self.member_range(index)]
    }

    /// Return the entry module of one component.
    pub fn entry(&self, component: ComponentId) -> Option<ModuleId> {
        let index = self.component_index(component)?;

        self.member_modules[self.member_range(index)]
            .first()
            .copied()
    }

    /// Return the external components one component depends on.
    pub fn dependencies(&self, component: ComponentId) -> &[ComponentId] {
        let Some(index) = self.component_index(component) else {
            return &[];
        };

        &self.dependencies[index]
    }

    /// Return the stable fingerprint of one projected component graph value.
    pub fn projection_fingerprint(
        &self,
        projection: ComponentGraphProjection,
    ) -> ArtifactProjectionFingerprint {
        match projection {
            ComponentGraphProjection::ComponentOf(module) => {
                ArtifactProjectionFingerprint::new(&self.component(module))
            }
            ComponentGraphProjection::ComponentEntryOf(module) => {
                ArtifactProjectionFingerprint::new(&self.component_entry(module))
            }
            ComponentGraphProjection::Members(component) => {
                ArtifactProjectionFingerprint::new(&self.members(component))
            }
            ComponentGraphProjection::Dependencies(component) => {
                ArtifactProjectionFingerprint::new(&self.dependencies(component))
            }
        }
    }

    /// Build one component graph from a dense module graph.
    fn build(module_graph: ModuleGraph) -> Self {
        let partition = module_graph.strongly_connected_components();
        let membership =
            ComponentMembership::from_partition(&partition, module_graph.modules.len());
        let components = ComponentIndex::from_membership(
            module_graph.profile,
            &module_graph.modules,
            &membership,
        );

        let dependencies = components.dependencies(&module_graph);

        Self {
            module_graph,
            component_of: Arc::from(components.component_of),
            components: Arc::from(components.ids),
            member_offsets: Arc::from(components.member_offsets),
            member_modules: Arc::from(components.member_modules),
            member_indexes: Arc::from(components.member_indexes),
            module_components: Arc::from(components.module_components),
            component_ranks: Arc::from(dependencies.ranks),
            dependencies: Arc::from(dependencies.targets),
        }
    }

    /// Rebuild changed dependencies while keeping the previous component membership.
    fn with_module_graph(&self, module_graph: ModuleGraph, changed_components: &[u32]) -> Self {
        let mut dependencies = self.dependencies.iter().cloned().collect::<Vec<_>>();

        // recompute only components whose outgoing module edges changed
        for component in changed_components {
            dependencies[*component as usize] =
                Arc::from(self.component_dependencies(&module_graph, *component as usize));
        }

        Self {
            module_graph,
            component_of: self.component_of.clone(),
            components: self.components.clone(),
            member_offsets: self.member_offsets.clone(),
            member_modules: self.member_modules.clone(),
            member_indexes: self.member_indexes.clone(),
            module_components: self.module_components.clone(),
            component_ranks: self.component_ranks.clone(),
            dependencies: Arc::from(dependencies),
        }
    }

    /// Return dependency components when changed edges keep the old component partition valid.
    fn changed_dependency_components(
        &self,
        updated_edges: &IndexMap<ModuleId, Arc<[ModuleId]>>,
        removed_modules: &[ModuleId],
    ) -> Option<Vec<u32>> {
        // module additions or removals change the membership universe
        if !removed_modules.is_empty() {
            return None;
        }

        let mut changed_components = Vec::with_capacity(updated_edges.len());
        for (module, new_edges) in updated_edges {
            let source = self.module_graph.module_index(*module)?;
            let source_component = self.module_components[source];
            let old_edges = self.module_graph.edge_targets(source);
            changed_components.push(source_component);

            // removing an intra-component edge can split the source component
            for target in old_edges {
                let target_index = *target as usize;
                let target_module = self.module_graph.modules[target_index];
                if new_edges.contains(&target_module) {
                    continue;
                }

                let target_component = self.module_components[target_index];
                if source_component == target_component {
                    return None;
                }
            }

            // adding a dependency edge can merge components only when it closes a DAG cycle
            for target in new_edges.iter().copied() {
                let target = self.module_graph.module_index(target)?;

                let target_component = self.module_components[target];
                if source_component == target_component || old_edges.contains(&(target as u32)) {
                    continue;
                }

                let source_rank = self.component_ranks[source_component as usize];
                let target_rank = self.component_ranks[target_component as usize];
                if source_rank >= target_rank {
                    return None;
                }
            }
        }

        changed_components.sort_unstable();
        changed_components.dedup();

        Some(changed_components)
    }

    /// Return direct dependencies for one component through one module graph.
    fn component_dependencies(
        &self,
        module_graph: &ModuleGraph,
        component: usize,
    ) -> Vec<ComponentId> {
        let mut dependencies = Vec::<u32>::new();
        let member_range =
            self.member_offsets[component] as usize..self.member_offsets[component + 1] as usize;

        // collect external component dependencies through member module edges
        for module in &self.member_indexes[member_range] {
            for target in module_graph.edge_targets(*module as usize) {
                let target = self.module_components[*target as usize] as usize;
                if target != component {
                    dependencies.push(target as u32);
                }
            }
        }

        dependencies.sort_unstable();
        dependencies.dedup();

        dependencies
            .into_iter()
            .map(|component| self.components[component as usize])
            .collect()
    }

    /// Return the dense index of one component.
    fn component_index(&self, component: ComponentId) -> Option<usize> {
        self.components.binary_search(&component).ok()
    }

    /// Return the member range for one dense component.
    fn member_range(&self, component: usize) -> std::ops::Range<usize> {
        self.member_offsets[component] as usize..self.member_offsets[component + 1] as usize
    }
}

/// SCC component membership using dense module indexes.
struct ComponentMembership {
    /// Per-component member start offsets into `members`.
    member_offsets: Vec<u32>,
    /// Component members as dense module indexes.
    members: Vec<u32>,
}

impl ComponentMembership {
    /// Build SCC component membership from one partition.
    fn from_partition(partition: &SccPartition, module_count: usize) -> Self {
        let component_count = partition.component_count() as usize;
        let mut sizes = vec![0u32; component_count];

        // count members per partition component
        for module in 0..module_count {
            let component = partition.component(module) as usize;

            sizes[component] += 1;
        }

        let mut member_offsets = Vec::with_capacity(component_count + 1);
        let mut members = vec![0u32; module_count];
        let mut cursor = Vec::with_capacity(component_count);

        // allocate one dense member list per component
        member_offsets.push(0);
        for size in sizes {
            let start = member_offsets.last().copied().unwrap_or_default();
            let end = start + size;

            member_offsets.push(end);
            cursor.push(start);
        }

        // fill membership lists in dense module order
        for module in 0..module_count {
            let component = partition.component(module) as usize;
            let target = cursor[component] as usize;

            members[target] = module as u32;
            cursor[component] += 1;
        }

        Self {
            member_offsets,
            members,
        }
    }

    /// Return the number of components.
    fn len(&self) -> usize {
        self.member_offsets.len() - 1
    }

    /// Return one component's dense module indexes.
    fn members(&self, component: usize) -> &[u32] {
        let range =
            self.member_offsets[component] as usize..self.member_offsets[component + 1] as usize;

        &self.members[range]
    }

    /// Return stable component ids for each partition component.
    fn ids(&self, profile: ProfileId, modules: &[ModuleId]) -> Vec<ComponentId> {
        (0..self.len())
            .map(|component| {
                ComponentId::from_sorted_modules(
                    profile,
                    self.members(component)
                        .iter()
                        .map(|member| modules[*member as usize]),
                )
            })
            .collect()
    }
}

/// Stable component index ordered by component id.
struct ComponentIndex {
    /// Components sorted by stable id.
    ids: Vec<ComponentId>,
    /// Per-component member start offsets into `member_modules`.
    member_offsets: Vec<u32>,
    /// Component members as dense module indexes.
    member_indexes: Vec<u32>,
    /// Component members as module ids.
    member_modules: Vec<ModuleId>,
    /// Per-module stable component id.
    component_of: Vec<ComponentId>,
    /// Per-module stable dense component index.
    module_components: Vec<u32>,
}

impl ComponentIndex {
    /// Build one stable component index from SCC membership.
    fn from_membership(
        profile: ProfileId,
        modules: &[ModuleId],
        membership: &ComponentMembership,
    ) -> Self {
        let partition_ids = membership.ids(profile, modules);
        let partition_order = sorted_indexes(&partition_ids);
        let mut partition_to_component = vec![0u32; membership.len()];
        let mut ids = Vec::with_capacity(partition_ids.len());
        let mut member_offsets = Vec::with_capacity(partition_ids.len() + 1);
        let mut member_indexes = Vec::new();
        let mut member_modules = Vec::new();

        // write members in stable component order
        member_offsets.push(0);
        for partition_component in partition_order {
            partition_to_component[partition_component] = ids.len() as u32;
            ids.push(partition_ids[partition_component]);

            let members = membership.members(partition_component);
            member_indexes.extend(members.iter().copied());
            member_modules.extend(members.iter().map(|module| modules[*module as usize]));
            member_offsets.push(member_modules.len() as u32);
        }

        let mut component_of = vec![ComponentId::new(0); modules.len()];
        let mut module_components = vec![0u32; modules.len()];

        // write per-module component projections
        for (partition_component, component) in partition_to_component.iter().copied().enumerate() {
            let component_id = ids[component as usize];

            for module in membership.members(partition_component) {
                let module = *module as usize;

                component_of[module] = component_id;
                module_components[module] = component;
            }
        }

        Self {
            ids,
            member_offsets,
            member_indexes,
            member_modules,
            component_of,
            module_components,
        }
    }

    /// Return direct component dependencies through one module graph.
    fn dependencies(&self, module_graph: &ModuleGraph) -> ComponentDependencies {
        let component_count = self.ids.len();
        let mut seen = vec![u32::MAX; component_count];
        let mut dense_targets = Vec::with_capacity(component_count);
        let mut targets = Vec::with_capacity(component_count);

        // collect and deduplicate one stable component at a time
        for component in 0..component_count {
            let mut component_targets = Vec::<u32>::new();
            let member_range = self.member_offsets[component] as usize
                ..self.member_offsets[component + 1] as usize;

            for module in &self.member_indexes[member_range] {
                for target in module_graph.edge_targets(*module as usize) {
                    let target = self.module_components[*target as usize] as usize;
                    if target != component && seen[target] != component as u32 {
                        seen[target] = component as u32;
                        component_targets.push(target as u32);
                    }
                }
            }

            component_targets.sort_unstable();
            let component_ids = component_targets
                .iter()
                .map(|component| self.ids[*component as usize])
                .collect::<Vec<_>>();
            dense_targets.push(Arc::from(component_targets));
            targets.push(Arc::from(component_ids));
        }

        let ranks = component_ranks(&dense_targets);

        ComponentDependencies { targets, ranks }
    }
}

/// Direct component dependencies and topological ranks.
struct ComponentDependencies {
    /// External components per component.
    targets: Vec<Arc<[ComponentId]>>,
    /// Per-component topological rank.
    ranks: Vec<u32>,
}

/// Return dense indexes keyed by module id.
fn module_index_map(modules: &[ModuleId]) -> FxHashMap<ModuleId, u32> {
    modules
        .iter()
        .enumerate()
        .map(|(index, module)| (*module, index as u32))
        .collect()
}

/// Return indexes sorted by their corresponding values.
fn sorted_indexes<T: Ord>(values: &[T]) -> Vec<usize> {
    let mut indexes = (0..values.len()).collect::<Vec<_>>();

    indexes.sort_unstable_by_key(|index| &values[*index]);

    indexes
}

/// Return per-component topological ranks for one dense condensation graph.
fn component_ranks(dependencies: &[Arc<[u32]>]) -> Vec<u32> {
    let mut indegrees = vec![0u32; dependencies.len()];

    // count incoming dependency edges
    for targets in dependencies {
        for target in targets.iter().copied() {
            indegrees[target as usize] += 1;
        }
    }

    let mut queue = Vec::new();
    for (component, indegree) in indegrees.iter().copied().enumerate() {
        if indegree == 0 {
            queue.push(component);
        }
    }

    let mut cursor = 0;
    let mut next_rank = 0u32;
    let mut ranks = vec![0u32; dependencies.len()];

    // assign ranks in Kahn order
    while cursor < queue.len() {
        let component = queue[cursor];
        cursor += 1;

        ranks[component] = next_rank;
        next_rank += 1;

        for target in dependencies[component].iter().copied() {
            let target = target as usize;
            indegrees[target] -= 1;
            if indegrees[target] == 0 {
                queue.push(target);
            }
        }
    }

    ranks
}

#[cfg(test)]
mod tests {
    use super::*;

    use destack_source::PackageId;
    use std::time::Instant;

    fn profile() -> ProfileId {
        ProfileId::new(1)
    }

    fn module(index: u32) -> ModuleId {
        ModuleId::new(PackageId::new(1), index.into())
    }

    fn graph_edges(edges: &[(ModuleId, &[ModuleId])]) -> IndexMap<ModuleId, Arc<[ModuleId]>> {
        edges
            .iter()
            .map(|(module, edges)| (*module, Arc::from(*edges)))
            .collect()
    }

    #[test]
    fn test_derive_updates_changed_edges() {
        let first = module(1);
        let second = module(2);
        let third = module(3);
        let graph = ComponentGraph::from_edges(
            profile(),
            graph_edges(&[(first, &[second]), (second, &[]), (third, &[])]),
        );

        let derived = graph.derive(graph_edges(&[(second, &[third])]), Vec::new());

        assert_eq!(derived.edges(first).as_ref(), &[second]);
        assert_eq!(derived.edges(second).as_ref(), &[third]);
    }

    #[test]
    fn test_derive_keeps_partition_after_dependency_edge_addition() {
        let first = module(1);
        let second = module(2);
        let third = module(3);
        let graph = ComponentGraph::from_edges(
            profile(),
            graph_edges(&[(first, &[second]), (second, &[]), (third, &[])]),
        );
        let first_component = graph.component(first);
        let second_component = graph.component(second);
        let third_component = graph.component(third);

        let derived = graph.derive(graph_edges(&[(first, &[second, third])]), Vec::new());
        let mut dependencies = vec![
            second_component.expect("second component should exist"),
            third_component.expect("third component should exist"),
        ];
        dependencies.sort_unstable();

        assert_eq!(derived.component(first), first_component);
        assert_eq!(derived.component(second), second_component);
        assert_eq!(derived.component(third), third_component);
        assert_eq!(
            derived.dependencies(first_component.expect("first component should exist")),
            dependencies.as_slice()
        );
    }

    #[test]
    fn test_derive_repartitions_after_cycle_edge_addition() {
        let first = module(1);
        let second = module(2);
        let graph = ComponentGraph::from_edges(
            profile(),
            graph_edges(&[(first, &[second]), (second, &[])]),
        );

        let derived = graph.derive(graph_edges(&[(second, &[first])]), Vec::new());

        assert_eq!(derived.component(first), derived.component(second));
        assert_eq!(
            derived.members(derived.component(first).expect("component should exist")),
            &[first, second]
        );
    }

    #[test]
    fn test_derive_removes_deleted_module_from_edges() {
        let first = module(1);
        let second = module(2);
        let third = module(3);
        let graph = ComponentGraph::from_edges(
            profile(),
            graph_edges(&[(first, &[second, third]), (second, &[]), (third, &[])]),
        );

        let derived = graph.derive(IndexMap::new(), vec![second]);

        assert_eq!(derived.edges(first).as_ref(), &[third]);
        assert!(derived.edges(second).is_empty());
    }

    #[test]
    fn test_derive_splits_component_after_edge_removal() {
        let first = module(1);
        let second = module(2);
        let graph = ComponentGraph::from_edges(
            profile(),
            graph_edges(&[(first, &[second]), (second, &[first])]),
        );

        let derived = graph.derive(graph_edges(&[(second, &[])]), Vec::new());

        assert_ne!(derived.component(first), derived.component(second));
        assert_eq!(
            derived.dependencies(
                derived
                    .component(first)
                    .expect("first component should exist")
            ),
            &[derived
                .component(second)
                .expect("second component should exist")]
        );
    }

    #[test]
    #[ignore = "prints large component graph timings"]
    fn test_component_graph_perf_large_sparse_edit() {
        let module_count = 100_000u32;
        let graph = timed("full build", || {
            ComponentGraph::from_edges(profile(), large_dag_edges(module_count))
        });

        let changed = graph_edges(&[(module(100), &[module(101), module(107), module(50_000)])]);
        let derived = timed("dependency edit", || graph.derive(changed, Vec::new()));

        assert_eq!(derived.component(module(100)), graph.component(module(100)));
    }

    fn large_dag_edges(count: u32) -> IndexMap<ModuleId, Arc<[ModuleId]>> {
        let mut edges = IndexMap::with_capacity(count as usize);

        for index in 0..count {
            let current = module(index);
            let mut targets = Vec::with_capacity(2);

            if index + 1 < count {
                targets.push(module(index + 1));
            }
            if index + 7 < count {
                targets.push(module(index + 7));
            }

            edges.insert(current, Arc::from(targets));
        }

        edges
    }

    fn timed<T>(name: &str, f: impl FnOnce() -> T) -> T {
        let started = Instant::now();
        let value = f();
        let elapsed = started.elapsed();

        eprintln!("{name}: {elapsed:?}");

        value
    }
}
