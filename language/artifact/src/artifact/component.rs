use destack_core::{DenseGraph, SccPartition};
use indexmap::IndexMap;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use destack_serde::Reflect;
use destack_source::{ComponentId, ModuleId, ProfileId};

use destack_dir::GlobalSymbolId;

use crate::{ArtifactProjectionFingerprint, ArtifactProjectionKey};

/// Dense per-module edge targets in compressed row storage.
#[derive(Debug, Clone, Hash, Serialize, Deserialize, Reflect)]
pub struct ModuleEdges {
    /// Per-module target start offsets into `targets`.
    offsets: Arc<[u32]>,
    /// Edge targets as dense module indexes.
    targets: Arc<[u32]>,
}

/// Reference and inference component partitions for one profile.
#[derive(Debug, Clone, Hash, Serialize, Deserialize, Reflect)]
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
    /// External inference components each component depends on.
    inference_dependencies: Arc<[Arc<[ComponentId]>]>,
    /// All inherent extensions declared across the graph's modules.
    extensions: Arc<[InherentExtension]>,
    /// The extension components and their transitive dependencies, sorted.
    extension_components: Arc<[ComponentId]>,
}

/// External reference components loaded with one component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalReferenceComponents {
    /// Components loaded through ordinary references.
    pub references: Vec<ComponentId>,
    /// Components loaded through inherent extensions.
    pub extensions: Vec<ComponentId>,
}

impl ExternalReferenceComponents {
    /// Iterate ordinary references followed by inherent extension components.
    pub fn components(&self) -> impl Iterator<Item = ComponentId> + '_ {
        self.references.iter().chain(&self.extensions).copied()
    }
}

/// One exported extension of a target declared in its own package.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct InherentExtension {
    /// The extension symbol.
    pub symbol: GlobalSymbolId,
    /// The extended target root.
    pub target: GlobalSymbolId,
}

impl ModuleEdges {
    /// Index complete module edges over one dense module universe.
    fn from_edges(
        modules: &[ModuleId],
        edges: &IndexMap<ModuleId, Arc<[ModuleId]>>,
    ) -> Result<Self, ModuleId> {
        let module_index = module_index_map(modules);
        let mut offsets = Vec::with_capacity(modules.len() + 1);
        let mut targets = Vec::new();

        // write each module's outgoing edges as dense target indexes
        offsets.push(0);
        for module in modules {
            let edges = edges.get(module).ok_or(*module)?;
            for target in edges.iter() {
                let target = module_index.get(target).copied().ok_or(*target)?;
                targets.push(target);
            }
            offsets.push(targets.len() as u32);
        }

        Ok(Self {
            offsets: Arc::from(offsets),
            targets: Arc::from(targets),
        })
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
    ) -> Result<Self, ModuleId> {
        let mut modules = reference_edges.keys().copied().collect::<Vec<_>>();
        modules.sort_unstable();
        let modules = Arc::<[ModuleId]>::from(modules);
        let reference = ModuleEdges::from_edges(&modules, &reference_edges)?;
        let inference = ModuleEdges::from_edges(&modules, &inference_edges)?;
        let mut graph = Self::build(profile, modules, reference, inference)?;
        graph.set_inherent_extensions(extensions)?;

        Ok(graph)
    }

    /// Derive a component graph after changing edges and removing modules.
    pub fn derive(
        &self,
        updated_edges: IndexMap<ModuleId, Arc<[ModuleId]>>,
        removed_modules: Vec<ModuleId>,
        inference_edges: IndexMap<ModuleId, Arc<[ModuleId]>>,
        extensions: Vec<InherentExtension>,
    ) -> Result<Self, ModuleId> {
        // keep the reference partition when no reference edges changed
        let mut graph = if updated_edges.is_empty() && removed_modules.is_empty() {
            let inference = ModuleEdges::from_edges(&self.modules, &inference_edges)?;

            self.with_inference_edges(inference)?
        }
        // keep the membership when changed edges provably preserve it
        else if let Some(changed_components) =
            self.changed_dependency_components(&updated_edges, &removed_modules)
        {
            let (modules, reference) = self.derived_reference(&updated_edges, &removed_modules)?;
            let inference = ModuleEdges::from_edges(&modules, &inference_edges)?;

            self.with_reference_edges(modules, reference, &changed_components)?
                .with_inference_edges(inference)?
        }
        // repartition from the derived module universe
        else {
            let (modules, reference) = self.derived_reference(&updated_edges, &removed_modules)?;
            let inference = ModuleEdges::from_edges(&modules, &inference_edges)?;

            Self::build(self.profile, modules, reference, inference)?
        };
        graph.set_inherent_extensions(extensions)?;

        Ok(graph)
    }

    /// Return outgoing reference edges for one module.
    pub fn reference_edges(&self, module: ModuleId) -> Option<Arc<[ModuleId]>> {
        self.module_targets(&self.reference_edges, module)
    }

    /// Return outgoing inference edges for one module.
    pub fn inference_edges(&self, module: ModuleId) -> Option<Arc<[ModuleId]>> {
        self.module_targets(&self.inference_edges, module)
    }

    /// Return whether one module's reference edges equal an external edge list.
    pub fn reference_edges_equal(&self, module: ModuleId, edges: &[ModuleId]) -> bool {
        self.module_edges_equal(&self.reference_edges, module, edges)
    }

    /// Return the reference component containing one module.
    pub fn reference_component(&self, module: ModuleId) -> Option<ComponentId> {
        let index = self.module_index(module)?;

        Some(self.reference_components[self.reference_component_indexes[index] as usize])
    }

    /// Return the member modules of one reference component.
    pub fn reference_members(&self, component: ComponentId) -> Option<&[ModuleId]> {
        let index = self.component_index(component)?;

        Some(&self.reference_modules[self.reference_member_range(index)])
    }

    /// Return whether one module's inference edges equal an external edge list.
    pub fn inference_edges_equal(&self, module: ModuleId, edges: &[ModuleId]) -> bool {
        self.module_edges_equal(&self.inference_edges, module, edges)
    }

    /// Return all modules in stable id order.
    pub fn modules(&self) -> &[ModuleId] {
        &self.modules
    }

    /// Return all reference components in stable id order.
    pub fn reference_components(&self) -> &[ComponentId] {
        &self.reference_components
    }

    /// Return all inference components in stable id order.
    pub fn inference_components(&self) -> &[ComponentId] {
        &self.inference_components
    }

    /// Return the dense index of one module.
    fn module_index(&self, module: ModuleId) -> Option<usize> {
        self.modules.binary_search(&module).ok()
    }

    /// Return one module's edge targets from one edge column.
    fn module_targets(&self, edges: &ModuleEdges, module: ModuleId) -> Option<Arc<[ModuleId]>> {
        let index = self.module_index(module)?;
        let targets = edges
            .targets(index)
            .iter()
            .map(|target| self.modules[*target as usize])
            .collect::<Vec<_>>();

        Some(Arc::from(targets))
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
    pub fn inference_members(&self, component: ComponentId) -> Option<&[ModuleId]> {
        let index = self.inference_components.binary_search(&component).ok()?;
        let range =
            self.inference_offsets[index] as usize..self.inference_offsets[index + 1] as usize;

        Some(&self.inference_modules[range])
    }

    /// Return the entry module of one inference component.
    pub fn inference_entry(&self, component: ComponentId) -> Option<ModuleId> {
        self.inference_members(component)?.first().copied()
    }

    /// Return the upstream inference components one component depends on.
    pub fn inference_dependencies(&self, inference: ComponentId) -> Option<&[ComponentId]> {
        let index = self.inference_components.binary_search(&inference).ok()?;

        Some(&self.inference_dependencies[index])
    }

    /// Return the entry module of one reference component.
    pub fn reference_entry(&self, component: ComponentId) -> Option<ModuleId> {
        let index = self.component_index(component)?;

        self.reference_modules[self.reference_member_range(index)]
            .first()
            .copied()
    }

    /// Return the external reference components one component depends on.
    pub fn reference_dependencies(&self, component: ComponentId) -> Option<&[ComponentId]> {
        let index = self.component_index(component)?;

        Some(&self.reference_dependencies[index])
    }

    /// Return every transitive dependency in stable breadth-first order.
    fn transitive_reference_dependencies(
        &self,
        component: ComponentId,
    ) -> Option<Vec<ComponentId>> {
        let mut dependencies = Vec::new();
        let mut seen = FxHashSet::default();

        // seed the walk with direct dependencies in graph order
        for dependency in self.reference_dependencies(component)? {
            if seen.insert(*dependency) {
                dependencies.push(*dependency);
            }
        }

        // extend the ordered worklist through the condensation graph
        let mut index = 0;
        while index < dependencies.len() {
            let current = dependencies[index];
            index += 1;

            let current_dependencies = self.reference_dependencies(current)?;
            for dependency in current_dependencies {
                if seen.insert(*dependency) {
                    dependencies.push(*dependency);
                }
            }
        }

        Some(dependencies)
    }

    /// Attach inherent extensions and close their external source components.
    fn set_inherent_extensions(
        &mut self,
        extensions: Vec<InherentExtension>,
    ) -> Result<(), ModuleId> {
        self.extensions = Arc::from(extensions);

        // require every extension endpoint in the module universe
        for extension in self.extensions.iter() {
            self.reference_component(extension.symbol.module_id)
                .ok_or(extension.symbol.module_id)?;
            self.reference_component(extension.target.module_id)
                .ok_or(extension.target.module_id)?;
        }

        // close over every cross-component extension source
        let mut closure = FxHashSet::default();
        let mut sources = FxHashSet::default();
        for extension in self.cross_component_extensions() {
            let source = self
                .reference_component(extension.symbol.module_id)
                .ok_or(extension.symbol.module_id)?;
            if !sources.insert(source) {
                continue;
            }
            closure.insert(source);
            let dependencies = self
                .transitive_reference_dependencies(source)
                .ok_or(extension.symbol.module_id)?;
            closure.extend(dependencies);
        }
        let mut closure = closure.into_iter().collect::<Vec<_>>();
        closure.sort_unstable();

        self.extension_components = Arc::from(closure);

        Ok(())
    }

    /// Return the cross-component inherent extensions.
    pub fn cross_component_extensions(&self) -> impl Iterator<Item = &InherentExtension> {
        self.extensions.iter().filter(|extension| {
            let source = self.reference_component(extension.symbol.module_id);
            let target = self.reference_component(extension.target.module_id);

            source != target
        })
    }

    /// Return all inherent extensions declared across the graph's modules.
    pub fn extensions(&self) -> &[InherentExtension] {
        &self.extensions
    }

    /// Return whether one component builds into the inherent extensions.
    pub fn is_extension_component(&self, component: ComponentId) -> bool {
        self.extension_components.binary_search(&component).is_ok()
    }

    /// Return the stable fingerprint of one projected component graph value.
    pub(crate) fn fingerprint_projection(
        &self,
        projection: ArtifactProjectionKey,
    ) -> Option<ArtifactProjectionFingerprint> {
        match projection {
            ArtifactProjectionKey::ReferenceComponent(module) => self
                .reference_component(module)
                .map(|component| ArtifactProjectionFingerprint::new(&component)),
            ArtifactProjectionKey::ReferenceMembers(component) => {
                let members = self.reference_members(component)?;

                Some(ArtifactProjectionFingerprint::new(members))
            }
            ArtifactProjectionKey::ReferenceDependencies(component) => {
                let dependencies = self.reference_dependencies(component)?;

                Some(ArtifactProjectionFingerprint::new(dependencies))
            }
            ArtifactProjectionKey::InferenceComponent(module) => self
                .inference_component(module)
                .map(|component| ArtifactProjectionFingerprint::new(&component)),
            ArtifactProjectionKey::InferenceComponents => Some(ArtifactProjectionFingerprint::new(
                self.inference_components(),
            )),
            ArtifactProjectionKey::InferenceMembers(component) => {
                let members = self.inference_members(component)?;

                Some(ArtifactProjectionFingerprint::new(members))
            }
            ArtifactProjectionKey::InferenceDependencies(component) => self
                .inference_dependencies(component)
                .map(ArtifactProjectionFingerprint::new),
            ArtifactProjectionKey::InherentExtensions => {
                let extensions = self
                    .cross_component_extensions()
                    .map(|extension| {
                        (
                            *extension,
                            self.reference_component(extension.symbol.module_id),
                            self.reference_component(extension.target.module_id),
                        )
                    })
                    .collect::<Vec<_>>();

                Some(ArtifactProjectionFingerprint::new(&(
                    extensions,
                    self.extension_components.as_ref(),
                )))
            }
            ArtifactProjectionKey::DirDeclaredModule(_)
            | ArtifactProjectionKey::DirInferenceExports
            | ArtifactProjectionKey::DirComponentEdges
            | ArtifactProjectionKey::DirCheckedModule(_) => None,
        }
    }

    /// Build one component graph from a dense module graph.
    fn build(
        profile: ProfileId,
        modules: Arc<[ModuleId]>,
        reference_edges: ModuleEdges,
        inference_edges: ModuleEdges,
    ) -> Result<Self, ModuleId> {
        let partition = reference_edges.strongly_connected_components();
        let membership = ComponentMembership::from_partition(&partition, modules.len());
        let components = ComponentPartition::from_membership(profile, &modules, &membership);

        let dependencies = components.dependencies(&reference_edges);
        let inference = inference_partition(profile, &modules, &inference_edges);
        require_partition_refinement(
            &modules,
            &components.module_components,
            &inference.module_components,
        )?;
        let inference_dependencies = inference.dependencies(&inference_edges);

        Ok(Self {
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
            inference_dependencies: Arc::from(inference_dependencies.targets),
            extensions: Arc::from([]),
            extension_components: Arc::from([]),
        })
    }

    /// Rebuild the inference partition over new inference edges.
    fn with_inference_edges(&self, inference_edges: ModuleEdges) -> Result<Self, ModuleId> {
        let inference = inference_partition(self.profile, &self.modules, &inference_edges);
        require_partition_refinement(
            &self.modules,
            &self.reference_component_indexes,
            &inference.module_components,
        )?;
        let dependencies = inference.dependencies(&inference_edges);
        let mut graph = self.clone();
        graph.inference_edges = inference_edges;
        graph.module_inference_components = Arc::from(inference.component_of);
        graph.inference_components = Arc::from(inference.ids);
        graph.inference_offsets = Arc::from(inference.member_offsets);
        graph.inference_modules = Arc::from(inference.member_modules);
        graph.inference_dependencies = Arc::from(dependencies.targets);

        Ok(graph)
    }

    /// Rebuild changed dependencies while keeping the previous component membership.
    fn with_reference_edges(
        &self,
        modules: Arc<[ModuleId]>,
        reference_edges: ModuleEdges,
        changed_components: &[u32],
    ) -> Result<Self, ModuleId> {
        let mut dependencies = self
            .reference_dependencies
            .iter()
            .cloned()
            .collect::<Vec<_>>();

        // recompute only components whose outgoing module edges changed
        for component in changed_components {
            let component_dependencies =
                self.component_dependencies(&modules, &reference_edges, *component)?;
            dependencies[*component as usize] = Arc::from(component_dependencies);
        }

        Ok(Self {
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
            inference_dependencies: self.inference_dependencies.clone(),
            extensions: self.extensions.clone(),
            extension_components: self.extension_components.clone(),
        })
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
    ) -> Result<(Arc<[ModuleId]>, ModuleEdges), ModuleId> {
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
                None => self
                    .module_targets(&self.reference_edges, *module)
                    .ok_or(*module)?,
            };
            edges.insert(*module, targets);
        }
        let modules = Arc::<[ModuleId]>::from(modules);
        let reference = ModuleEdges::from_edges(&modules, &edges)?;

        Ok((modules, reference))
    }

    /// Return direct dependencies for one component through the reference edges.
    fn component_dependencies(
        &self,
        modules: &[ModuleId],
        reference_edges: &ModuleEdges,
        component: u32,
    ) -> Result<Vec<ComponentId>, ModuleId> {
        let component = component as usize;
        let mut dependencies = Vec::<u32>::new();
        let member_range = self.reference_offsets[component] as usize
            ..self.reference_offsets[component + 1] as usize;

        // collect external component dependencies through member module edges
        for module in &self.reference_modules[member_range] {
            let module = modules.binary_search(module).map_err(|_| *module)?;
            for target in reference_edges.targets(module) {
                let target = self.reference_component_indexes[*target as usize] as usize;
                if target != component {
                    dependencies.push(target as u32);
                }
            }
        }

        dependencies.sort_unstable();
        dependencies.dedup();

        let dependencies = dependencies
            .into_iter()
            .map(|component| self.reference_components[component as usize])
            .collect();

        Ok(dependencies)
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
        let mut start = 0;
        for size in sizes {
            let end = start + size;

            member_offsets.push(end);
            cursor.push(start);
            start = end;
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
fn inference_partition(
    profile: ProfileId,
    modules: &[ModuleId],
    inference_edges: &ModuleEdges,
) -> ComponentPartition {
    let partition = inference_edges.strongly_connected_components();
    let membership = ComponentMembership::from_partition(&partition, modules.len());

    ComponentPartition::from_membership(profile, modules, &membership)
}

/// Require every inference component to refine one reference component.
fn require_partition_refinement(
    modules: &[ModuleId],
    reference_components: &[u32],
    inference_components: &[u32],
) -> Result<(), ModuleId> {
    let mut owners = vec![None; modules.len()];

    // require every inference component to have one reference component owner
    for (module, inference) in inference_components.iter().copied().enumerate() {
        let reference = reference_components[module];
        let owner = &mut owners[inference as usize];
        match owner {
            Some(owner) if *owner != reference => return Err(modules[module]),
            Some(_) => {}
            None => *owner = Some(reference),
        }
    }

    Ok(())
}

/// Stable component partition ordered by component id.
struct ComponentPartition {
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

impl ComponentPartition {
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

    fn component_graph(
        reference_edges: IndexMap<ModuleId, Arc<[ModuleId]>>,
        inference_edges: IndexMap<ModuleId, Arc<[ModuleId]>>,
    ) -> ComponentGraph {
        ComponentGraph::from_edges(profile(), reference_edges, inference_edges, Vec::new())
            .expect("component graph should build")
    }

    #[test]
    fn test_derive_updates_changed_edges() {
        let first = module(1);
        let second = module(2);
        let third = module(3);
        let edges = graph_edges(&[(first, &[second]), (second, &[]), (third, &[])]);
        let graph = component_graph(edges.clone(), no_coupling(&edges));

        let derived = graph
            .derive(
                graph_edges(&[(second, &[third])]),
                Vec::new(),
                no_coupling(&edges),
                Vec::new(),
            )
            .expect("component graph should derive");

        assert_eq!(
            derived.reference_edges(first).as_deref(),
            Some([second].as_slice())
        );
        assert_eq!(
            derived.reference_edges(second).as_deref(),
            Some([third].as_slice())
        );
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
        let graph = component_graph(edges.clone(), no_coupling(&edges));
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
            Some(vec![second, third, fourth])
        );
    }

    #[test]
    fn test_derive_keeps_partition_after_dependency_edge_addition() {
        let first = module(1);
        let second = module(2);
        let third = module(3);
        let edges = graph_edges(&[(first, &[second]), (second, &[]), (third, &[])]);
        let graph = component_graph(edges.clone(), no_coupling(&edges));
        let first_component = graph.reference_component(first);
        let second_component = graph.reference_component(second);
        let third_component = graph.reference_component(third);

        let derived = graph
            .derive(
                graph_edges(&[(first, &[second, third])]),
                Vec::new(),
                no_coupling(&edges),
                Vec::new(),
            )
            .expect("component graph should derive");
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
            Some(dependencies.as_slice())
        );
    }

    #[test]
    fn test_derive_repartitions_after_cycle_edge_addition() {
        let first = module(1);
        let second = module(2);
        let edges = graph_edges(&[(first, &[second]), (second, &[])]);
        let graph = component_graph(edges.clone(), no_coupling(&edges));

        let derived = graph
            .derive(
                graph_edges(&[(second, &[first])]),
                Vec::new(),
                no_coupling(&edges),
                Vec::new(),
            )
            .expect("component graph should derive");

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
            Some([first, second].as_slice())
        );
    }

    #[test]
    fn test_derive_removes_deleted_module_from_edges() {
        let first = module(1);
        let second = module(2);
        let third = module(3);
        let edges = graph_edges(&[(first, &[second, third]), (second, &[]), (third, &[])]);
        let graph = component_graph(edges.clone(), no_coupling(&edges));

        let derived = graph
            .derive(
                IndexMap::new(),
                vec![second],
                no_coupling(&edges),
                Vec::new(),
            )
            .expect("component graph should derive");

        assert_eq!(
            derived.reference_edges(first).as_deref(),
            Some([third].as_slice())
        );
        assert_eq!(derived.reference_edges(second), None);
    }

    #[test]
    fn test_derive_splits_component_after_edge_removal() {
        let first = module(1);
        let second = module(2);
        let edges = graph_edges(&[(first, &[second]), (second, &[first])]);
        let graph = component_graph(edges.clone(), no_coupling(&edges));

        let derived = graph
            .derive(
                graph_edges(&[(second, &[])]),
                Vec::new(),
                no_coupling(&edges),
                Vec::new(),
            )
            .expect("component graph should derive");

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
            Some(
                [derived
                    .reference_component(second)
                    .expect("second component should exist")]
                .as_slice()
            )
        );
    }

    #[test]
    fn test_refine_reference_components_into_inference_components() {
        let first = module(1);
        let second = module(2);

        // a reference cycle without a coupling cycle splits into two inference components
        let edges = graph_edges(&[(first, &[second]), (second, &[first])]);
        let coupling = graph_edges(&[(first, &[second]), (second, &[])]);
        let graph = component_graph(edges, coupling);

        let component = graph
            .reference_component(first)
            .expect("component should exist");
        assert_eq!(graph.reference_component(second), Some(component));
        assert_ne!(
            graph.inference_component(first),
            graph.inference_component(second)
        );
        let first_unit = graph.inference_component(first).expect("unit should exist");
        let second_unit = graph
            .inference_component(second)
            .expect("unit should exist");
        assert_eq!(graph.inference_entry(first_unit), Some(first));
        assert_eq!(graph.inference_entry(second_unit), Some(second));
        assert_eq!(
            graph.inference_dependencies(first_unit),
            Some([second_unit].as_slice())
        );
    }

    #[test]
    fn test_allow_inference_dependencies_across_reference_components() {
        let first = module(1);
        let second = module(2);
        let references = graph_edges(&[(first, &[second]), (second, &[])]);
        let inference = graph_edges(&[(first, &[second]), (second, &[])]);
        let graph = component_graph(references, inference);

        let first_component = graph
            .inference_component(first)
            .expect("first inference component should exist");
        let second_component = graph
            .inference_component(second)
            .expect("second inference component should exist");

        assert_eq!(
            graph.inference_dependencies(first_component),
            Some([second_component].as_slice())
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
        let graph = component_graph(edges, coupling);

        let unit = graph
            .inference_component(first)
            .expect("settle unit should exist");
        assert_eq!(graph.inference_component(second), Some(unit));
        assert_eq!(
            graph.inference_members(unit),
            Some([first, second].as_slice())
        );
        assert_eq!(graph.inference_entry(unit), Some(first));
        assert_ne!(graph.inference_component(third), Some(unit));
    }

    #[test]
    fn test_derive_repartitions_inference_components_from_coupling_edges() {
        let first = module(1);
        let second = module(2);

        // coupling the cycle joins the inference components without touching references
        let edges = graph_edges(&[(first, &[second]), (second, &[first])]);
        let graph = component_graph(edges.clone(), no_coupling(&edges));
        assert_ne!(
            graph.inference_component(first),
            graph.inference_component(second)
        );

        let derived = graph
            .derive(IndexMap::new(), Vec::new(), edges, Vec::new())
            .expect("component graph should derive");

        assert_eq!(
            derived.reference_component(first),
            graph.reference_component(first)
        );
        assert_eq!(
            derived.inference_component(first),
            derived.inference_component(second)
        );
    }
}
