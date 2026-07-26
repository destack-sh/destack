use destack_core::{DenseGraph, SccPartition};
use indexmap::IndexMap;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use destack_serde::Reflect;
use destack_source::{ComponentId, ModuleId, ProfileId};

use destack_dir::GlobalSymbolId;

use crate::{ArtifactProjectionFingerprint, ComponentGraphProjection};

/// Dense per-module edge targets in compressed row storage.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ModuleEdges {
    /// Per-module target start offsets into `targets`.
    offsets: Arc<[u32]>,
    /// Edge targets as dense module indexes.
    targets: Arc<[u32]>,
}

/// Reference and inference component partitions for one profile.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ComponentGraph {
    /// The profile this graph belongs to.
    profile: ProfileId,
    /// Modules sorted by stable id.
    modules: Arc<[ModuleId]>,
    /// Reference edges over the module universe.
    reference_edges: ModuleEdges,
    /// Inference edges over the module universe.
    inference_edges: ModuleEdges,
    /// Reference components sorted by stable id.
    reference_components: Arc<[ComponentId]>,
    /// Per-component member start offsets into `reference_modules`.
    reference_offsets: Arc<[u32]>,
    /// Reference component members as module ids.
    reference_modules: Arc<[ModuleId]>,
    /// Per-module dense reference component index.
    reference_component_indexes: Arc<[u32]>,
    /// Per-reference-component topological rank.
    reference_ranks: Arc<[u32]>,
    /// External reference components each component depends on.
    reference_dependencies: Arc<[Arc<[ComponentId]>]>,
    /// Each module's inference component.
    module_inference_components: Arc<[ComponentId]>,
    /// Inference components sorted by stable id.
    inference_components: Arc<[ComponentId]>,
    /// Per-component member start offsets into `inference_modules`.
    inference_offsets: Arc<[u32]>,
    /// Inference component members as module ids.
    inference_modules: Arc<[ModuleId]>,
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
    /// The extended target root.
    pub target: GlobalSymbolId,
}

impl ModuleEdges {
    /// Index complete module edges over one dense module universe.
    fn from_edges(modules: &[ModuleId], edges: &IndexMap<ModuleId, Arc<[ModuleId]>>) -> Self {
        let module_index = module_index_map(modules);
        let mut offsets = Vec::with_capacity(modules.len() + 1);
        let mut targets = Vec::new();

        // write each module's outgoing edges as dense target indexes
        offsets.push(0);
        for module in modules {
            let edges = edges.get(module).map(Arc::as_ref).unwrap_or_default();
            for target in edges {
                if let Some(target) = module_index.get(target).copied() {
                    targets.push(target);
                }
            }
            offsets.push(targets.len() as u32);
        }

        Self {
            offsets: Arc::from(offsets),
            targets: Arc::from(targets),
        }
    }

    /// Return the dense edge targets of one module index.
    fn targets(&self, module: usize) -> &[u32] {
        let range = self.offsets[module] as usize..self.offsets[module + 1] as usize;

        &self.targets[range]
    }

    /// Return the strongly connected components of these edges.
    fn strongly_connected_components(&self) -> SccPartition {
        let graph = DenseGraph::new(&self.offsets, &self.targets);

        graph.strongly_connected_components()
    }
}

impl ComponentGraph {
    /// Build one component graph from complete reference and inference edges.
    pub fn from_edges(
        profile: ProfileId,
        reference_edges: IndexMap<ModuleId, Arc<[ModuleId]>>,
        inference_edges: IndexMap<ModuleId, Arc<[ModuleId]>>,
        extensions: Vec<InherentExtension>,
    ) -> Self {
        let mut modules = reference_edges.keys().copied().collect::<Vec<_>>();
        modules.sort_unstable();
        let modules = Arc::<[ModuleId]>::from(modules);
        let reference = ModuleEdges::from_edges(&modules, &reference_edges);
        let inference = ModuleEdges::from_edges(&modules, &inference_edges);
        let mut graph = Self::build(profile, modules, reference, inference);
        graph.set_inherent_extensions(extensions);

        graph
    }

    /// Derive a component graph after changing edges and removing modules.
    pub fn derive(
        &self,
        updated_edges: IndexMap<ModuleId, Arc<[ModuleId]>>,
        removed_modules: Vec<ModuleId>,
        inference_edges: IndexMap<ModuleId, Arc<[ModuleId]>>,
        extensions: Vec<InherentExtension>,
    ) -> Self {
        // keep the reference partition when no reference edges changed
        let mut graph = if updated_edges.is_empty() && removed_modules.is_empty() {
            let inference = ModuleEdges::from_edges(&self.modules, &inference_edges);

            self.with_inference_edges(inference)
        }
        // keep the membership when changed edges provably preserve it
        else if let Some(changed_components) =
            self.changed_dependency_components(&updated_edges, &removed_modules)
        {
            let (modules, reference) = self.derived_reference(&updated_edges, &removed_modules);
            let inference = ModuleEdges::from_edges(&modules, &inference_edges);

            self.with_reference_edges(modules, reference, &changed_components)
                .with_inference_edges(inference)
        }
        // repartition from the derived module universe
        else {
            let (modules, reference) = self.derived_reference(&updated_edges, &removed_modules);
            let inference = ModuleEdges::from_edges(&modules, &inference_edges);

            Self::build(self.profile, modules, reference, inference)
        };
        graph.set_inherent_extensions(extensions);

        graph
    }

    /// Return outgoing reference edges for one module.
    pub fn edges(&self, module: ModuleId) -> Arc<[ModuleId]> {
        self.module_targets(&self.reference_edges, module)
    }

    /// Return outgoing inference edges for one module.
    pub fn inference_edges(&self, module: ModuleId) -> Arc<[ModuleId]> {
        self.module_targets(&self.inference_edges, module)
    }

    /// Return whether one module's reference edges equal an external edge list.
    pub fn edges_equal(&self, module: ModuleId, edges: &[ModuleId]) -> bool {
        self.module_edges_equal(&self.reference_edges, module, edges)
    }

    /// Return the reference component containing one module.
    pub fn reference_component(&self, module: ModuleId) -> Option<ComponentId> {
        let index = self.module_index(module)?;

        Some(self.reference_components[self.reference_component_indexes[index] as usize])
    }

    /// Return the member modules of one reference component.
    pub fn reference_members(&self, component: ComponentId) -> &[ModuleId] {
        let Some(index) = self.component_index(component) else {
            return &[];
        };

        &self.reference_modules[self.reference_member_range(index)]
    }

    /// Return whether one module's inference edges equal an external edge list.
    pub fn inference_edges_equal(&self, module: ModuleId, edges: &[ModuleId]) -> bool {
        self.module_edges_equal(&self.inference_edges, module, edges)
    }

    /// Return the dense index of one module.
    fn module_index(&self, module: ModuleId) -> Option<usize> {
        self.modules.binary_search(&module).ok()
    }

    /// Return one module's edge targets from one edge column.
    fn module_targets(&self, edges: &ModuleEdges, module: ModuleId) -> Arc<[ModuleId]> {
        // return an empty edge list for modules outside the graph
        let Some(index) = self.module_index(module) else {
            return Arc::from([]);
        };
        let targets = edges
            .targets(index)
            .iter()
            .map(|target| self.modules[*target as usize])
            .collect::<Vec<_>>();

        Arc::from(targets)
    }

    /// Return whether one module's edges in one column equal an external list.
    fn module_edges_equal(
        &self,
        edges: &ModuleEdges,
        module: ModuleId,
        external: &[ModuleId],
    ) -> bool {
        // a module outside the graph cannot match
        let Some(index) = self.module_index(module) else {
            return false;
        };
        let targets = edges.targets(index);
        if targets.len() != external.len() {
            return false;
        }

        targets
            .iter()
            .zip(external)
            .all(|(target, edge)| self.modules[*target as usize] == *edge)
    }

    /// Return the inference component containing one module.
    pub fn inference_component(&self, module: ModuleId) -> Option<ComponentId> {
        let index = self.module_index(module)?;

        Some(self.module_inference_components[index])
    }

    /// Return the member modules of one inference component.
    pub fn inference_members(&self, component: ComponentId) -> &[ModuleId] {
        let Ok(index) = self.inference_components.binary_search(&component) else {
            return &[];
        };
        let range =
            self.inference_offsets[index] as usize..self.inference_offsets[index + 1] as usize;

        &self.inference_modules[range]
    }

    /// Return the entry module of one inference component.
    pub fn inference_entry(&self, component: ComponentId) -> Option<ModuleId> {
        self.inference_members(component).first().copied()
    }

    /// Return the upstream inference components one component depends on.
    pub fn inference_dependencies(&self, inference: ComponentId) -> Vec<ComponentId> {
        let Some(reference) = self
            .inference_entry(inference)
            .and_then(|entry| self.reference_component(entry))
        else {
            return Vec::new();
        };
        let mut upstream = Vec::new();

        // follow inference edges into sibling components
        for module in self.inference_members(inference) {
            for target in self.inference_edges(*module).iter() {
                let Some(target_component) = self.inference_component(*target) else {
                    continue;
                };
                if target_component == inference
                    || self.reference_component(*target) != Some(reference)
                {
                    continue;
                }
                if !upstream.contains(&target_component) {
                    upstream.push(target_component);
                }
            }
        }

        upstream.sort_unstable();

        upstream
    }

    /// Return the entry module of one reference component.
    pub fn reference_entry(&self, component: ComponentId) -> Option<ModuleId> {
        let index = self.component_index(component)?;

        self.reference_modules[self.reference_member_range(index)]
            .first()
            .copied()
    }

    /// Return the external reference components one component depends on.
    pub fn reference_dependencies(&self, component: ComponentId) -> &[ComponentId] {
        let Some(index) = self.component_index(component) else {
            return &[];
        };

        &self.reference_dependencies[index]
    }

    /// Return every transitive dependency in stable breadth-first order.
    pub fn transitive_reference_dependencies(&self, component: ComponentId) -> Vec<ComponentId> {
        let mut dependencies = Vec::new();
        let mut seen = FxHashSet::default();

        // seed the walk with direct dependencies in graph order
        for dependency in self.reference_dependencies(component) {
            if seen.insert(*dependency) {
                dependencies.push(*dependency);
            }
        }

        // extend the ordered worklist through the condensation graph
        let mut index = 0;
        while index < dependencies.len() {
            let current = dependencies[index];
            index += 1;

            for dependency in self.reference_dependencies(current) {
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
        self.reference_component(extension.symbol.module_id)
            != self.reference_component(extension.target.module_id)
    }

    /// Attach the cross-component inherent extensions and close their components.
    fn set_inherent_extensions(&mut self, mut extensions: Vec<InherentExtension>) {
        extensions.retain(|extension| self.is_cross_component(extension));

        // close over the components the extensions build from
        let mut closure = FxHashSet::default();
        let mut sources = FxHashSet::default();
        for extension in &extensions {
            let Some(component) = self.reference_component(extension.symbol.module_id) else {
                continue;
            };
            if !sources.insert(component) {
                continue;
            }
            closure.insert(component);
            closure.extend(self.transitive_reference_dependencies(component));
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
            ComponentGraphProjection::ReferenceComponent(module) => {
                ArtifactProjectionFingerprint::new(&self.reference_component(module))
            }
            ComponentGraphProjection::ReferenceMembers(component) => {
                ArtifactProjectionFingerprint::new(&self.reference_members(component))
            }
            ComponentGraphProjection::ReferenceDependencies(component) => {
                ArtifactProjectionFingerprint::new(&self.reference_dependencies(component))
            }
            ComponentGraphProjection::InferenceComponent(module) => {
                ArtifactProjectionFingerprint::new(&self.inference_component(module))
            }
            ComponentGraphProjection::InferenceMembers(component) => {
                ArtifactProjectionFingerprint::new(&self.inference_members(component))
            }
            ComponentGraphProjection::InferenceDependencies(component) => {
                ArtifactProjectionFingerprint::new(&self.inference_dependencies(component))
            }
            ComponentGraphProjection::InherentExtensions => {
                ArtifactProjectionFingerprint::new(&self.inherent.as_ref())
            }
        }
    }

    /// Build one component graph from a dense module graph.
    fn build(
        profile: ProfileId,
        modules: Arc<[ModuleId]>,
        reference_edges: ModuleEdges,
        inference_edges: ModuleEdges,
    ) -> Self {
        let partition = reference_edges.strongly_connected_components();
        let membership = ComponentMembership::from_partition(&partition, modules.len());
        let components = ComponentIndex::from_membership(profile, &modules, &membership);

        let dependencies = components.dependencies(&reference_edges);
        let inference = inference_index(profile, &modules, &inference_edges);

        Self {
            profile,
            modules,
            reference_edges,
            inference_edges,
            reference_components: Arc::from(components.ids),
            reference_offsets: Arc::from(components.member_offsets),
            reference_modules: Arc::from(components.member_modules),
            reference_component_indexes: Arc::from(components.module_components),
            reference_ranks: Arc::from(dependencies.ranks),
            reference_dependencies: Arc::from(dependencies.targets),
            module_inference_components: Arc::from(inference.component_of),
            inference_components: Arc::from(inference.ids),
            inference_offsets: Arc::from(inference.member_offsets),
            inference_modules: Arc::from(inference.member_modules),
            inherent: Arc::from([]),
            inherent_closure: Arc::from([]),
        }
    }

    /// Rebuild the inference partition over new inference edges.
    fn with_inference_edges(&self, inference_edges: ModuleEdges) -> Self {
        let inference = inference_index(self.profile, &self.modules, &inference_edges);
        let mut graph = self.clone();
        graph.inference_edges = inference_edges;
        graph.module_inference_components = Arc::from(inference.component_of);
        graph.inference_components = Arc::from(inference.ids);
        graph.inference_offsets = Arc::from(inference.member_offsets);
        graph.inference_modules = Arc::from(inference.member_modules);

        graph
    }

    /// Rebuild changed dependencies while keeping the previous component membership.
    fn with_reference_edges(
        &self,
        modules: Arc<[ModuleId]>,
        reference_edges: ModuleEdges,
        changed_components: &[u32],
    ) -> Self {
        let mut dependencies = self
            .reference_dependencies
            .iter()
            .cloned()
            .collect::<Vec<_>>();

        // recompute only components whose outgoing module edges changed
        for component in changed_components {
            dependencies[*component as usize] =
                Arc::from(self.component_dependencies(&modules, &reference_edges, *component));
        }

        Self {
            profile: self.profile,
            modules,
            reference_edges,
            inference_edges: self.inference_edges.clone(),
            reference_components: self.reference_components.clone(),
            reference_offsets: self.reference_offsets.clone(),
            reference_modules: self.reference_modules.clone(),
            reference_component_indexes: self.reference_component_indexes.clone(),
            reference_ranks: self.reference_ranks.clone(),
            reference_dependencies: Arc::from(dependencies),
            module_inference_components: self.module_inference_components.clone(),
            inference_components: self.inference_components.clone(),
            inference_offsets: self.inference_offsets.clone(),
            inference_modules: self.inference_modules.clone(),
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
            let source = self.module_index(*module)?;
            let source_component = self.reference_component_indexes[source];
            let old_edges = self.reference_edges.targets(source);
            changed_components.push(source_component);

            // removing an intra-component edge can split the source component
            for target in old_edges {
                let target_index = *target as usize;
                let target_module = self.modules[target_index];
                if new_edges.contains(&target_module) {
                    continue;
                }

                let target_component = self.reference_component_indexes[target_index];
                if source_component == target_component {
                    return None;
                }
            }

            // adding a dependency edge can merge components only when it closes a DAG cycle
            for target in new_edges.iter().copied() {
                let target = self.module_index(target)?;

                let target_component = self.reference_component_indexes[target];
                if source_component == target_component || old_edges.contains(&(target as u32)) {
                    continue;
                }

                let source_rank = self.reference_ranks[source_component as usize];
                let target_rank = self.reference_ranks[target_component as usize];
                if source_rank >= target_rank {
                    return None;
                }
            }
        }

        changed_components.sort_unstable();
        changed_components.dedup();

        Some(changed_components)
    }

    /// Derive the module universe and reference edges after changed module edges.
    fn derived_reference(
        &self,
        updated_edges: &IndexMap<ModuleId, Arc<[ModuleId]>>,
        removed_modules: &[ModuleId],
    ) -> (Arc<[ModuleId]>, ModuleEdges) {
        // canonicalize removals for cheap membership checks
        let mut removed = removed_modules.to_vec();
        removed.sort_unstable();
        removed.dedup();

        // derive the sorted module universe
        let mut modules = Vec::with_capacity(self.modules.len() + updated_edges.len());
        for module in self.modules.iter().copied() {
            if removed.binary_search(&module).is_err() {
                modules.push(module);
            }
        }
        for module in updated_edges.keys().copied() {
            if removed.binary_search(&module).is_err() {
                modules.push(module);
            }
        }
        modules.sort_unstable();
        modules.dedup();

        // keep unchanged module edges and index the changed ones
        let mut edges = IndexMap::with_capacity(modules.len());
        for module in &modules {
            let targets = match updated_edges.get(module) {
                Some(targets) => targets.clone(),
                None => self.module_targets(&self.reference_edges, *module),
            };
            edges.insert(*module, targets);
        }
        let modules = Arc::<[ModuleId]>::from(modules);
        let reference = ModuleEdges::from_edges(&modules, &edges);

        (modules, reference)
    }

    /// Return direct dependencies for one component through the reference edges.
    fn component_dependencies(
        &self,
        modules: &[ModuleId],
        reference_edges: &ModuleEdges,
        component: u32,
    ) -> Vec<ComponentId> {
        let component = component as usize;
        let mut dependencies = Vec::<u32>::new();
        let member_range = self.reference_offsets[component] as usize
            ..self.reference_offsets[component + 1] as usize;

        // collect external component dependencies through member module edges
        for module in &self.reference_modules[member_range] {
            let module = modules
                .binary_search(module)
                .expect("component member is in the module universe");
            for target in reference_edges.targets(module) {
                let target = self.reference_component_indexes[*target as usize] as usize;
                if target != component {
                    dependencies.push(target as u32);
                }
            }
        }

        dependencies.sort_unstable();
        dependencies.dedup();

        dependencies
            .into_iter()
            .map(|component| self.reference_components[component as usize])
            .collect()
    }

    /// Return the dense index of one component.
    fn component_index(&self, component: ComponentId) -> Option<usize> {
        self.reference_components.binary_search(&component).ok()
    }

    /// Return the member range for one dense component.
    fn reference_member_range(&self, component: usize) -> std::ops::Range<usize> {
        self.reference_offsets[component] as usize..self.reference_offsets[component + 1] as usize
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

/// Partition dense inference edges into inference components.
fn inference_index(
    profile: ProfileId,
    modules: &[ModuleId],
    inference_edges: &ModuleEdges,
) -> ComponentIndex {
    let partition = inference_edges.strongly_connected_components();
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
    fn dependencies(&self, edges: &ModuleEdges) -> ComponentDependencies {
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
                for target in edges.targets(*module as usize) {
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

    fn no_inference_edges(
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
        let graph = ComponentGraph::from_edges(
            profile(),
            edges.clone(),
            no_inference_edges(&edges),
            Vec::new(),
        );

        let derived = graph.derive(
            graph_edges(&[(second, &[third])]),
            Vec::new(),
            no_inference_edges(&edges),
            Vec::new(),
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
        let graph = ComponentGraph::from_edges(
            profile(),
            edges.clone(),
            no_inference_edges(&edges),
            Vec::new(),
        );
        let first = graph
            .reference_component(first)
            .expect("first component should exist");
        let second = graph
            .reference_component(second)
            .expect("second component should exist");
        let third = graph
            .reference_component(third)
            .expect("third component should exist");
        let fourth = graph
            .reference_component(fourth)
            .expect("fourth component should exist");

        assert_eq!(
            graph.transitive_reference_dependencies(first),
            vec![second, third, fourth]
        );
    }

    #[test]
    fn test_derive_keeps_partition_after_dependency_edge_addition() {
        let first = module(1);
        let second = module(2);
        let third = module(3);
        let edges = graph_edges(&[(first, &[second]), (second, &[]), (third, &[])]);
        let graph = ComponentGraph::from_edges(
            profile(),
            edges.clone(),
            no_inference_edges(&edges),
            Vec::new(),
        );
        let first_component = graph.reference_component(first);
        let second_component = graph.reference_component(second);
        let third_component = graph.reference_component(third);

        let derived = graph.derive(
            graph_edges(&[(first, &[second, third])]),
            Vec::new(),
            no_inference_edges(&edges),
            Vec::new(),
        );
        let mut dependencies = vec![
            second_component.expect("second component should exist"),
            third_component.expect("third component should exist"),
        ];
        dependencies.sort_unstable();

        assert_eq!(derived.reference_component(first), first_component);
        assert_eq!(derived.reference_component(second), second_component);
        assert_eq!(derived.reference_component(third), third_component);
        assert_eq!(
            derived.reference_dependencies(first_component.expect("first component should exist")),
            dependencies.as_slice()
        );
    }

    #[test]
    fn test_derive_repartitions_after_cycle_edge_addition() {
        let first = module(1);
        let second = module(2);
        let edges = graph_edges(&[(first, &[second]), (second, &[])]);
        let graph = ComponentGraph::from_edges(
            profile(),
            edges.clone(),
            no_inference_edges(&edges),
            Vec::new(),
        );

        let derived = graph.derive(
            graph_edges(&[(second, &[first])]),
            Vec::new(),
            no_inference_edges(&edges),
            Vec::new(),
        );

        assert_eq!(
            derived.reference_component(first),
            derived.reference_component(second)
        );
        assert_eq!(
            derived.reference_members(
                derived
                    .reference_component(first)
                    .expect("component should exist")
            ),
            &[first, second]
        );
    }

    #[test]
    fn test_derive_removes_deleted_module_from_edges() {
        let first = module(1);
        let second = module(2);
        let third = module(3);
        let edges = graph_edges(&[(first, &[second, third]), (second, &[]), (third, &[])]);
        let graph = ComponentGraph::from_edges(
            profile(),
            edges.clone(),
            no_inference_edges(&edges),
            Vec::new(),
        );

        let derived = graph.derive(
            IndexMap::new(),
            vec![second],
            no_inference_edges(&edges),
            Vec::new(),
        );

        assert_eq!(derived.edges(first).as_ref(), &[third]);
        assert!(derived.edges(second).is_empty());
    }

    #[test]
    fn test_derive_splits_component_after_edge_removal() {
        let first = module(1);
        let second = module(2);
        let edges = graph_edges(&[(first, &[second]), (second, &[first])]);
        let graph = ComponentGraph::from_edges(
            profile(),
            edges.clone(),
            no_inference_edges(&edges),
            Vec::new(),
        );

        let derived = graph.derive(
            graph_edges(&[(second, &[])]),
            Vec::new(),
            no_inference_edges(&edges),
            Vec::new(),
        );

        assert_ne!(
            derived.reference_component(first),
            derived.reference_component(second)
        );
        assert_eq!(
            derived.reference_dependencies(
                derived
                    .reference_component(first)
                    .expect("first component should exist")
            ),
            &[derived
                .reference_component(second)
                .expect("second component should exist")]
        );
    }

    #[test]
    fn test_refine_reference_components_into_inference_components() {
        let first = module(1);
        let second = module(2);

        // a reference cycle without an inference cycle splits into two components
        let edges = graph_edges(&[(first, &[second]), (second, &[first])]);
        let inference = graph_edges(&[(first, &[second]), (second, &[])]);
        let graph = ComponentGraph::from_edges(profile(), edges, inference, Vec::new());

        let component = graph
            .reference_component(first)
            .expect("component should exist");
        assert_eq!(graph.reference_component(second), Some(component));
        assert_ne!(
            graph.inference_component(first),
            graph.inference_component(second)
        );
        assert_eq!(
            graph.inference_component(first),
            Some(ComponentId::from_sorted_modules(
                profile(),
                [first].into_iter()
            ))
        );
        assert_eq!(
            graph.inference_component(second),
            Some(ComponentId::from_sorted_modules(
                profile(),
                [second].into_iter()
            ))
        );
    }

    #[test]
    fn test_join_inference_cycles() {
        let first = module(1);
        let second = module(2);
        let third = module(3);

        // the cyclic pair infers together while the third module stands alone
        let edges = graph_edges(&[(first, &[second]), (second, &[first, third]), (third, &[])]);
        let inference = graph_edges(&[(first, &[second]), (second, &[first]), (third, &[])]);
        let graph = ComponentGraph::from_edges(profile(), edges, inference, Vec::new());

        let component = graph
            .inference_component(first)
            .expect("inference component should exist");
        assert_eq!(graph.inference_component(second), Some(component));
        assert_eq!(graph.inference_members(component), &[first, second]);
        assert_eq!(graph.inference_entry(component), Some(first));
        assert_ne!(graph.inference_component(third), Some(component));
    }

    #[test]
    fn test_derive_repartitions_inference_components() {
        let first = module(1);
        let second = module(2);

        // adding an inference cycle joins components without changing references
        let edges = graph_edges(&[(first, &[second]), (second, &[first])]);
        let graph = ComponentGraph::from_edges(
            profile(),
            edges.clone(),
            no_inference_edges(&edges),
            Vec::new(),
        );
        assert_ne!(
            graph.inference_component(first),
            graph.inference_component(second)
        );

        let derived = graph.derive(IndexMap::new(), Vec::new(), edges, Vec::new());

        assert_eq!(
            derived.reference_component(first),
            graph.reference_component(first)
        );
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
        let inference = no_inference_edges(&edges);
        let graph = timed("full build", || {
            ComponentGraph::from_edges(profile(), edges.clone(), inference.clone(), Vec::new())
        });

        let changed = graph_edges(&[(module(100), &[module(101), module(107), module(50_000)])]);
        let derived = timed("dependency edit", || {
            graph.derive(changed, Vec::new(), inference.clone(), Vec::new())
        });

        assert_eq!(
            derived.reference_component(module(100)),
            graph.reference_component(module(100))
        );
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
