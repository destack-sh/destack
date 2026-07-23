use destack_core::{DenseGraph, SccPartition};
use indexmap::IndexMap;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use destack_serde::Reflect;
use destack_source::{ComponentId, ModuleId, ProfileId};

use destack_dir::GlobalSymbolId;

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
    /// Module edges that couple checking through inference.
    coupling_graph: ModuleGraph,
    /// Components sorted by stable id.
    components: Arc<[ComponentId]>,
    /// Per-component member start offsets into `member_modules`.
    member_offsets: Arc<[u32]>,
    /// Component members as module ids.
    member_modules: Arc<[ModuleId]>,
    /// Per-module dense component index.
    module_components: Arc<[u32]>,
    /// Per-component topological rank in the condensation graph.
    component_ranks: Arc<[u32]>,
    /// External components each component depends on.
    dependencies: Arc<[Arc<[ComponentId]>]>,
    /// Each module's inference component.
    inference_of: Arc<[ComponentId]>,
    /// Inference components sorted by stable id.
    inference_ids: Arc<[ComponentId]>,
    /// Per-component member start offsets into `inference_member_modules`.
    inference_member_offsets: Arc<[u32]>,
    /// Inference component members as module ids.
    inference_member_modules: Arc<[ModuleId]>,
    /// Cross-component inherent extensions resolved across the graph's modules.
    inherent: Arc<[InherentExtension]>,
    /// The extension components and their transitive dependencies, sorted.
    inherent_closure: Arc<[ComponentId]>,
}

/// One exported extension of a target declared in its own package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct InherentExtension {
    /// The extension symbol.
    pub symbol: GlobalSymbolId,
    /// The module declaring the extended target root.
    pub target: ModuleId,
}

impl ModuleGraph {
    /// Build one module graph from complete program module edges.
    fn from_edges(profile: ProfileId, edges: IndexMap<ModuleId, Arc<[ModuleId]>>) -> Self {
        // build the shared dense module universe
        let mut modules = edges.keys().copied().collect::<Vec<_>>();
        modules.sort_unstable();
        let graph_edges = dense_module_edges(&modules, &edges);

        Self {
            profile,
            modules: Arc::from(modules),
            edges: graph_edges,
        }
    }

    /// Derive a module graph after changing outgoing edges and removing modules.
    fn derive(
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

    /// Index complete module edges over this graph's module universe.
    fn index_edges(&self, edges: &IndexMap<ModuleId, Arc<[ModuleId]>>) -> Arc<[Arc<[u32]>]> {
        dense_module_edges(&self.modules, edges)
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
        // a module outside the graph cannot match
        let Some(index) = self.module_index(module) else {
            return false;
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
                    targets.push(module_index[target]);
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
    /// Build one component graph from complete module and coupling edges.
    pub fn from_edges(
        profile: ProfileId,
        edges: IndexMap<ModuleId, Arc<[ModuleId]>>,
        coupling_edges: IndexMap<ModuleId, Arc<[ModuleId]>>,
        extensions: Vec<InherentExtension>,
    ) -> Self {
        let module_graph = ModuleGraph::from_edges(profile, edges);
        let coupling_graph = ModuleGraph::from_edges(profile, coupling_edges);
        let mut graph = Self::build(module_graph, coupling_graph);
        graph.set_inherent_extensions(extensions);

        graph
    }

    /// Derive a component graph after changing edges and removing modules.
    pub fn derive(
        &self,
        updated_edges: IndexMap<ModuleId, Arc<[ModuleId]>>,
        removed_modules: Vec<ModuleId>,
        coupling_edges: IndexMap<ModuleId, Arc<[ModuleId]>>,
        extensions: Vec<InherentExtension>,
    ) -> Self {
        let coupling_graph = ModuleGraph::from_edges(self.module_graph.profile, coupling_edges);

        // keep the reference partition when no reference edges changed
        let mut graph = if updated_edges.is_empty() && removed_modules.is_empty() {
            self.with_coupling_graph(coupling_graph)
        }
        // keep the membership when changed edges provably preserve it
        else if let Some(changed_components) =
            self.changed_dependency_components(&updated_edges, &removed_modules)
        {
            let module_graph = self.module_graph.derive(updated_edges, removed_modules);

            self.with_module_graph(module_graph, &changed_components)
                .with_coupling_graph(coupling_graph)
        }
        // repartition from the derived module graph
        else {
            let module_graph = self.module_graph.derive(updated_edges, removed_modules);

            Self::build(module_graph, coupling_graph)
        };
        graph.set_inherent_extensions(extensions);

        graph
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

        Some(self.components[self.module_components[index] as usize])
    }

    /// Return the member modules of one component.
    pub fn members(&self, component: ComponentId) -> &[ModuleId] {
        let Some(index) = self.component_index(component) else {
            return &[];
        };

        &self.member_modules[self.member_range(index)]
    }

    /// Return whether one module's coupling edges equal an external edge list.
    pub fn coupling_edges_equal(&self, module: ModuleId, edges: &[ModuleId]) -> bool {
        // a module outside the graph cannot match
        let Some(index) = self.module_graph.module_index(module) else {
            return false;
        };

        if self.coupling_edges[index].len() != edges.len() {
            return false;
        }

        self.coupling_edges[index]
            .iter()
            .zip(edges)
            .all(|(target, edge)| self.module_graph.modules[*target as usize] == *edge)
    }

    /// Return the inference component containing one module.
    pub fn inference_component(&self, module: ModuleId) -> Option<ComponentId> {
        let index = self.module_graph.module_index(module)?;

        Some(self.inference_of[index])
    }

    /// Return the member modules of one inference component.
    pub fn inference_members(&self, unit: ComponentId) -> &[ModuleId] {
        let Ok(index) = self.inference_ids.binary_search(&unit) else {
            return &[];
        };
        let range = self.inference_member_offsets[index] as usize
            ..self.inference_member_offsets[index + 1] as usize;

        &self.inference_member_modules[range]
    }

    /// Return the entry module of one inference component.
    pub fn inference_entry(&self, unit: ComponentId) -> Option<ModuleId> {
        self.inference_members(unit).first().copied()
    }

    /// Return the inference components of one reference component in member order.
    pub fn inference_components(&self, component: ComponentId) -> Vec<ComponentId> {
        let mut units = Vec::new();

        for module in self.members(component) {
            let Some(unit) = self.inference_component(*module) else {
                continue;
            };
            if !units.contains(&unit) {
                units.push(unit);
            }
        }

        units
    }

    /// Return the upstream inference components one unit couples to in its reference component.
    pub fn inference_dependencies(&self, unit: ComponentId) -> Vec<ComponentId> {
        let Some(component) = self
            .inference_entry(unit)
            .and_then(|entry| self.component(entry))
        else {
            return Vec::new();
        };
        let mut upstream = Vec::new();

        // follow coupling edges into sibling units inside the same reference component
        for module in self.inference_members(unit) {
            for target in self.coupling_graph.edges(*module).iter() {
                let Some(target_unit) = self.inference_component(*target) else {
                    continue;
                };
                if target_unit == unit || self.component(*target) != Some(component) {
                    continue;
                }
                if !upstream.contains(&target_unit) {
                    upstream.push(target_unit);
                }
            }
        }

        upstream.sort_unstable();

        upstream
    }

    /// Return the reference component and inference entry containing one module.
    pub fn inference_component_entry(&self, module: ModuleId) -> Option<(ComponentId, ModuleId)> {
        let component = self.component(module)?;
        let unit = self.inference_component(module)?;
        let entry = self.inference_entry(unit)?;

        Some((component, entry))
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

    /// Return every transitive dependency in stable breadth-first order.
    pub fn transitive_dependencies(&self, component: ComponentId) -> Vec<ComponentId> {
        let mut dependencies = Vec::new();
        let mut seen = FxHashSet::default();

        // seed the walk with direct dependencies in graph order
        for dependency in self.dependencies(component) {
            if seen.insert(*dependency) {
                dependencies.push(*dependency);
            }
        }

        // extend the ordered worklist through the condensation graph
        let mut index = 0;
        while index < dependencies.len() {
            let current = dependencies[index];
            index += 1;

            for dependency in self.dependencies(current) {
                if seen.insert(*dependency) {
                    dependencies.push(*dependency);
                }
            }
        }

        dependencies
    }

    /// Return whether one extension loads across components.
    ///
    /// Same-component extensions load with their target through plain
    /// dependencies, so only cross-component rows stay on the graph.
    fn is_cross_component(&self, extension: &InherentExtension) -> bool {
        self.component(extension.symbol.module_id) != self.component(extension.target)
    }

    /// Attach the cross-component inherent extensions and close their components.
    fn set_inherent_extensions(&mut self, mut extensions: Vec<InherentExtension>) {
        extensions.retain(|extension| self.is_cross_component(extension));

        // close over the components the extensions build from
        let mut closure = FxHashSet::default();
        let mut sources = FxHashSet::default();
        for extension in &extensions {
            let Some(component) = self.component(extension.symbol.module_id) else {
                continue;
            };
            if !sources.insert(component) {
                continue;
            }
            closure.insert(component);
            closure.extend(self.transitive_dependencies(component));
        }
        let mut closure = closure.into_iter().collect::<Vec<_>>();
        closure.sort_unstable();

        self.inherent = Arc::from(extensions);
        self.inherent_closure = Arc::from(closure);
    }

    /// Return whether one unfiltered extension list matches the attached rows.
    pub fn inherent_extensions_equal(&self, extensions: &[InherentExtension]) -> bool {
        let retained = extensions
            .iter()
            .filter(|extension| self.is_cross_component(extension));

        retained.eq(self.inherent.iter())
    }

    /// Return the cross-component inherent extensions.
    pub fn inherent_extensions(&self) -> &[InherentExtension] {
        &self.inherent
    }

    /// Return whether one component builds into the inherent extensions.
    pub fn inherent_closure_contains(&self, component: ComponentId) -> bool {
        self.inherent_closure.binary_search(&component).is_ok()
    }

    /// Return the stable fingerprint of one projected component graph value.
    pub fn projection_fingerprint(
        &self,
        projection: ComponentGraphProjection,
    ) -> ArtifactProjectionFingerprint {
        match projection {
            ComponentGraphProjection::Component(module) => {
                ArtifactProjectionFingerprint::new(&self.component(module))
            }
            ComponentGraphProjection::InferenceEntry(module) => {
                ArtifactProjectionFingerprint::new(&self.inference_component_entry(module))
            }
            ComponentGraphProjection::Members(component) => {
                ArtifactProjectionFingerprint::new(&self.members(component))
            }
            ComponentGraphProjection::Dependencies(component) => {
                ArtifactProjectionFingerprint::new(&self.dependencies(component))
            }
            ComponentGraphProjection::InferenceMembers(unit) => {
                ArtifactProjectionFingerprint::new(&self.inference_members(unit))
            }
            ComponentGraphProjection::InferenceDependencies(unit) => {
                ArtifactProjectionFingerprint::new(&self.inference_dependencies(unit))
            }
            ComponentGraphProjection::InherentExtensions => {
                ArtifactProjectionFingerprint::new(&self.inherent.as_ref())
            }
        }
    }

    /// Build one component graph from a dense module graph.
    fn build(module_graph: ModuleGraph, coupling_edges: Arc<[Arc<[u32]>]>) -> Self {
        let partition = strongly_connected_components(&module_graph.edges);
        let membership =
            ComponentMembership::from_partition(&partition, module_graph.modules.len());
        let components = ComponentIndex::from_membership(
            module_graph.profile,
            &module_graph.modules,
            &membership,
        );

        let dependencies = components.dependencies(&module_graph);
        let settle = inference_index(module_graph.profile, &module_graph.modules, &coupling_edges);

        Self {
            module_graph,
            coupling_graph,
            components: Arc::from(components.ids),
            member_offsets: Arc::from(components.member_offsets),
            member_modules: Arc::from(components.member_modules),
            module_components: Arc::from(components.module_components),
            component_ranks: Arc::from(dependencies.ranks),
            dependencies: Arc::from(dependencies.targets),
            inference_of: Arc::from(settle.component_of),
            inference_ids: Arc::from(settle.ids),
            inference_member_offsets: Arc::from(settle.member_offsets),
            inference_member_modules: Arc::from(settle.member_modules),
            inherent: Arc::from([]),
            inherent_closure: Arc::from([]),
        }
    }

    /// Rebuild the inference partition over one new coupling graph.
    fn with_coupling_edges(&self, coupling_edges: Arc<[Arc<[u32]>]>) -> Self {
        let settle = inference_index(
            self.module_graph.profile,
            &self.module_graph.modules,
            &coupling_edges,
        );
        let mut graph = self.clone();
        graph.coupling_edges = coupling_edges;
        graph.inference_of = Arc::from(settle.component_of);
        graph.inference_ids = Arc::from(settle.ids);
        graph.inference_member_offsets = Arc::from(settle.member_offsets);
        graph.inference_member_modules = Arc::from(settle.member_modules);

        graph
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
            coupling_graph: self.coupling_graph.clone(),
            components: self.components.clone(),
            member_offsets: self.member_offsets.clone(),
            member_modules: self.member_modules.clone(),
            module_components: self.module_components.clone(),
            component_ranks: self.component_ranks.clone(),
            dependencies: Arc::from(dependencies),
            inference_of: self.inference_of.clone(),
            inference_ids: self.inference_ids.clone(),
            inference_member_offsets: self.inference_member_offsets.clone(),
            inference_member_modules: self.inference_member_modules.clone(),
            inherent: self.inherent.clone(),
            inherent_closure: self.inherent_closure.clone(),
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
        for module in &self.member_modules[member_range] {
            let module = module_graph
                .module_index(*module)
                .expect("component member is in the module graph");
            for target in module_graph.edge_targets(module) {
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

/// Return the strongly connected components of one dense edge column.
fn strongly_connected_components(edges: &[Arc<[u32]>]) -> SccPartition {
    let mut edge_offsets = Vec::with_capacity(edges.len() + 1);
    let edge_count = edges.iter().map(|edges| edges.len()).sum();
    let mut edge_targets = Vec::with_capacity(edge_count);

    // materialize transient CSR storage for Tarjan
    edge_offsets.push(0);
    for edges in edges {
        edge_targets.extend(edges.iter().copied());
        edge_offsets.push(edge_targets.len() as u32);
    }

    let graph = DenseGraph::new(&edge_offsets, &edge_targets);

    graph.strongly_connected_components()
}

/// Partition dense coupling edges into inference components.
fn inference_index(
    profile: ProfileId,
    modules: &[ModuleId],
    coupling_edges: &[Arc<[u32]>],
) -> ComponentIndex {
    let partition = strongly_connected_components(coupling_edges);
    let membership = ComponentMembership::from_partition(&partition, modules.len());

    ComponentIndex::from_membership(profile, modules, &membership)
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

/// Write complete module edges as dense target indexes.
fn dense_module_edges(
    modules: &[ModuleId],
    edges: &IndexMap<ModuleId, Arc<[ModuleId]>>,
) -> Arc<[Arc<[u32]>]> {
    let module_index = module_index_map(modules);
    let mut dense_edges = Vec::with_capacity(modules.len());

    // transcribe every source and target directly
    for module in modules {
        let targets = edges[module]
            .iter()
            .map(|target| module_index[target])
            .collect::<Vec<_>>();
        dense_edges.push(Arc::from(targets));
    }

    Arc::from(dense_edges)
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

    fn no_coupling(
        edges: &IndexMap<ModuleId, Arc<[ModuleId]>>,
    ) -> IndexMap<ModuleId, Arc<[ModuleId]>> {
        edges
            .keys()
            .map(|module| (*module, Arc::from([])))
            .collect()
    }

    #[test]
    fn test_derive_updates_changed_edges() {
        let first = module(1);
        let second = module(2);
        let third = module(3);
        let edges = graph_edges(&[(first, &[second]), (second, &[]), (third, &[])]);
        let graph = ComponentGraph::from_edges(profile(), edges.clone(), no_coupling(&edges));

        let derived = graph.derive(
            graph_edges(&[(second, &[third])]),
            Vec::new(),
            no_coupling(&edges),
        );

        assert_eq!(derived.edges(first).as_ref(), &[second]);
        assert_eq!(derived.edges(second).as_ref(), &[third]);
    }

    #[test]
    fn test_return_transitive_component_dependencies() {
        let first = module(1);
        let second = module(2);
        let third = module(3);
        let fourth = module(4);
        let edges = graph_edges(&[
            (first, &[second, third]),
            (second, &[fourth]),
            (third, &[fourth]),
            (fourth, &[]),
        ]);
        let graph = ComponentGraph::from_edges(profile(), edges.clone(), no_coupling(&edges));
        let first = graph
            .component(first)
            .expect("first component should exist");
        let second = graph
            .component(second)
            .expect("second component should exist");
        let third = graph
            .component(third)
            .expect("third component should exist");
        let fourth = graph
            .component(fourth)
            .expect("fourth component should exist");

        assert_eq!(
            graph.transitive_dependencies(first),
            vec![second, third, fourth]
        );
    }

    #[test]
    fn test_derive_keeps_partition_after_dependency_edge_addition() {
        let first = module(1);
        let second = module(2);
        let third = module(3);
        let edges = graph_edges(&[(first, &[second]), (second, &[]), (third, &[])]);
        let graph = ComponentGraph::from_edges(profile(), edges.clone(), no_coupling(&edges));
        let first_component = graph.component(first);
        let second_component = graph.component(second);
        let third_component = graph.component(third);

        let derived = graph.derive(
            graph_edges(&[(first, &[second, third])]),
            Vec::new(),
            no_coupling(&edges),
        );
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
        let edges = graph_edges(&[(first, &[second]), (second, &[])]);
        let graph = ComponentGraph::from_edges(profile(), edges.clone(), no_coupling(&edges));

        let derived = graph.derive(
            graph_edges(&[(second, &[first])]),
            Vec::new(),
            no_coupling(&edges),
        );

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
        let edges = graph_edges(&[(first, &[second, third]), (second, &[]), (third, &[])]);
        let graph = ComponentGraph::from_edges(profile(), edges.clone(), no_coupling(&edges));
        let coupling = graph_edges(&[(first, &[]), (third, &[])]);

        let derived = graph.derive(IndexMap::new(), vec![second], coupling);

        assert_eq!(derived.edges(first).as_ref(), &[third]);
        assert!(derived.edges(second).is_empty());
    }

    #[test]
    fn test_derive_splits_component_after_edge_removal() {
        let first = module(1);
        let second = module(2);
        let edges = graph_edges(&[(first, &[second]), (second, &[first])]);
        let graph = ComponentGraph::from_edges(profile(), edges.clone(), no_coupling(&edges));

        let derived = graph.derive(
            graph_edges(&[(second, &[])]),
            Vec::new(),
            no_coupling(&edges),
        );

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
    fn test_refine_reference_components_into_inference_components() {
        let first = module(1);
        let second = module(2);

        // a reference cycle without a coupling cycle splits into two inference components
        let edges = graph_edges(&[(first, &[second]), (second, &[first])]);
        let coupling = graph_edges(&[(first, &[second]), (second, &[])]);
        let graph = ComponentGraph::from_edges(profile(), edges, coupling);

        let component = graph.component(first).expect("component should exist");
        assert_eq!(graph.component(second), Some(component));
        assert_ne!(
            graph.inference_component(first),
            graph.inference_component(second)
        );
        assert_eq!(
            graph.inference_component_entry(first),
            Some((component, first))
        );
        assert_eq!(
            graph.inference_component_entry(second),
            Some((component, second))
        );
    }

    #[test]
    fn test_inference_components_join_coupling_cycles() {
        let first = module(1);
        let second = module(2);
        let third = module(3);

        // the coupled pair infers together while the third module stands alone
        let edges = graph_edges(&[(first, &[second]), (second, &[first, third]), (third, &[])]);
        let coupling = graph_edges(&[(first, &[second]), (second, &[first]), (third, &[])]);
        let graph = ComponentGraph::from_edges(profile(), edges, coupling);

        let unit = graph
            .inference_component(first)
            .expect("settle unit should exist");
        assert_eq!(graph.inference_component(second), Some(unit));
        assert_eq!(graph.inference_members(unit), &[first, second]);
        assert_eq!(graph.inference_entry(unit), Some(first));
        assert_ne!(graph.inference_component(third), Some(unit));
    }

    #[test]
    fn test_derive_repartitions_inference_components_from_coupling_edges() {
        let first = module(1);
        let second = module(2);

        // coupling the cycle joins the inference components without touching references
        let edges = graph_edges(&[(first, &[second]), (second, &[first])]);
        let graph = ComponentGraph::from_edges(profile(), edges.clone(), no_coupling(&edges));
        assert_ne!(
            graph.inference_component(first),
            graph.inference_component(second)
        );

        let derived = graph.derive(IndexMap::new(), Vec::new(), edges);

        assert_eq!(derived.component(first), graph.component(first));
        assert_eq!(
            derived.inference_component(first),
            derived.inference_component(second)
        );
    }

    #[test]
    #[ignore = "prints large component graph timings"]
    fn test_component_graph_perf_large_sparse_edit() {
        let module_count = 100_000u32;
        let edges = large_dag_edges(module_count);
        let coupling = no_coupling(&edges);
        let graph = timed("full build", || {
            ComponentGraph::from_edges(profile(), edges.clone(), coupling.clone())
        });

        let changed = graph_edges(&[(module(100), &[module(101), module(107), module(50_000)])]);
        let derived = timed("dependency edit", || {
            graph.derive(changed, Vec::new(), coupling.clone())
        });

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
