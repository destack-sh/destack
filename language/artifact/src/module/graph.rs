use destack_core::StringId;
use destack_source::{ModuleId, ModuleVersion, ProfileId};
use indexmap::map::Entry;
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

use crate::{ModuleEdge, ModuleEdgeRelation, ModuleKind};

/// The metadata tracked for one module node in the graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleMetadata {
    /// The semantic kind of this module.
    pub kind: ModuleKind,
    /// The version snapshot captured for this module.
    pub version: ModuleVersion,
    /// The outgoing dependency edges from this module.
    pub dependencies: IndexSet<ModuleEdge>,
}

/// Module dependency graph for a profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleGraph {
    /// The profile id for this graph.
    pub profile_id: ProfileId,
    /// Module metadata keyed by module id.
    pub modules: IndexMap<ModuleId, ModuleMetadata>,
    /// Reverse dependency edges keyed by target module id.
    pub dependents: IndexMap<ModuleId, IndexSet<ModuleEdge>>,
}

impl ModuleGraph {
    /// Create an empty module graph for a profile.
    pub fn new(profile_id: ProfileId) -> Self {
        Self {
            profile_id,
            modules: IndexMap::new(),
            dependents: IndexMap::new(),
        }
    }

    /// Return whether another graph represents the same dependency snapshot.
    pub fn matches_snapshot(&self, other: &Self) -> bool {
        self == other
    }

    /// Update one module and its outgoing dependency edges.
    pub fn update_module(
        &mut self,
        module_id: ModuleId,
        module_kind: ModuleKind,
        module_version: ModuleVersion,
        dependencies: Vec<ModuleEdge>,
    ) {
        // normalize and sort dependency list
        let mut sorted = dependencies;
        sorted.sort_unstable();
        sorted.dedup();

        // snapshot previous dependencies for reverse updates
        let previous = self
            .modules
            .get(&module_id)
            .map(|metadata| metadata.dependencies.clone());

        // update dependency set
        let mut next_set = IndexSet::new();
        for dependency in sorted {
            next_set.insert(dependency);
        }
        self.modules.insert(
            module_id,
            ModuleMetadata {
                kind: module_kind,
                version: module_version,
                dependencies: next_set.clone(),
            },
        );

        // remove stale reverse edges
        if let Some(previous) = previous {
            for dependency in previous {
                if next_set.contains(&dependency) {
                    continue;
                }
                if let Some(entry) = self.dependents.get_mut(&dependency.target) {
                    entry.shift_remove(&dependency.reverse_for(module_id));
                    if entry.is_empty() {
                        self.dependents.shift_remove(&dependency.target);
                    }
                }
            }
        }

        // add new reverse edges
        for dependency in next_set {
            let reverse_edge = dependency.reverse_for(module_id);
            match self.dependents.entry(dependency.target) {
                Entry::Occupied(mut entry) => {
                    entry.get_mut().insert(reverse_edge);
                }
                Entry::Vacant(entry) => {
                    let mut set = IndexSet::new();
                    set.insert(reverse_edge);
                    entry.insert(set);
                }
            }
        }
    }

    /// Update one module with plain import like dependencies.
    pub fn update_module_dependencies(
        &mut self,
        module_id: ModuleId,
        module_kind: ModuleKind,
        module_version: ModuleVersion,
        dependencies: Vec<ModuleId>,
    ) {
        let edges = dependencies
            .into_iter()
            .map(|module_id| ModuleEdge::new(module_id, ModuleEdgeRelation::Import))
            .collect();

        self.update_module(module_id, module_kind, module_version, edges);
    }

    /// Return the registered kind for one module when available.
    pub fn module_kind_for(&self, module_id: ModuleId) -> Option<ModuleKind> {
        self.modules.get(&module_id).map(|metadata| metadata.kind)
    }

    /// Return the registered version for one module when available.
    pub fn module_version_for(&self, module_id: ModuleId) -> Option<ModuleVersion> {
        self.modules
            .get(&module_id)
            .map(|metadata| metadata.version)
    }

    /// Return the dependency edges for one module.
    pub fn dependency_edges_for(&self, module_id: ModuleId) -> Vec<ModuleEdge> {
        self.modules
            .get(&module_id)
            .map(|metadata| metadata.dependencies.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Return the dependency edges for one module filtered by relation.
    pub fn dependency_edges_with_relation(
        &self,
        module_id: ModuleId,
        relation: ModuleEdgeRelation,
    ) -> Vec<ModuleEdge> {
        self.dependency_edges_for(module_id)
            .into_iter()
            .filter(|edge| edge.relation == relation)
            .collect()
    }

    /// Return one exact authored dependency edge when it exists.
    pub fn dependency_edge_for_specifier(
        &self,
        module_id: ModuleId,
        relation: ModuleEdgeRelation,
        specifier: StringId,
    ) -> Option<ModuleEdge> {
        self.dependency_edges_for(module_id)
            .into_iter()
            .find(|edge| edge.relation == relation && edge.specifier == Some(specifier))
    }

    /// Return the dependency edges recorded for one exact source site.
    pub fn dependency_edges_for_site(&self, module_id: ModuleId, site: u32) -> Vec<ModuleEdge> {
        self.dependency_edges_for(module_id)
            .into_iter()
            .filter(|edge| edge.site == Some(site))
            .collect()
    }

    /// Return one exact authored dependency edge for one source site when it exists.
    pub fn dependency_edge_for_site_specifier(
        &self,
        module_id: ModuleId,
        relation: ModuleEdgeRelation,
        site: u32,
        specifier: StringId,
    ) -> Option<ModuleEdge> {
        self.dependency_edges_for_site(module_id, site)
            .into_iter()
            .find(|edge| edge.relation == relation && edge.specifier == Some(specifier))
    }

    /// Return one exact authored dependency target when it exists.
    pub fn dependency_target_for_specifier(
        &self,
        module_id: ModuleId,
        relation: ModuleEdgeRelation,
        specifier: StringId,
    ) -> Option<ModuleId> {
        self.dependency_edge_for_specifier(module_id, relation, specifier)
            .map(|edge| edge.target)
    }

    /// Return the dependency list for one module.
    pub fn dependencies_for(&self, module_id: ModuleId) -> Vec<ModuleId> {
        let mut dependencies = self
            .dependency_edges_for(module_id)
            .into_iter()
            .map(|dependency| dependency.target)
            .collect::<Vec<_>>();
        dependencies.sort_unstable();
        dependencies.dedup();
        dependencies
    }

    /// Return the dependent edges for one module.
    pub fn dependent_edges_for(&self, module_id: ModuleId) -> Vec<ModuleEdge> {
        self.dependents
            .get(&module_id)
            .map(|dependencies| dependencies.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Return the dependent list for one module.
    pub fn dependents_for(&self, module_id: ModuleId) -> Vec<ModuleId> {
        let mut dependents = self
            .dependent_edges_for(module_id)
            .into_iter()
            .map(|dependency| dependency.target)
            .collect::<Vec<_>>();
        dependents.sort_unstable();
        dependents.dedup();
        dependents
    }

    /// Remove a module from the graph.
    pub fn remove_module(&mut self, module_id: ModuleId) {
        // remove forward dependencies and clean reverse edges
        if let Some(metadata) = self.modules.shift_remove(&module_id) {
            for dependency in metadata.dependencies {
                if let Some(entry) = self.dependents.get_mut(&dependency.target) {
                    entry.shift_remove(&dependency.reverse_for(module_id));
                    if entry.is_empty() {
                        self.dependents.shift_remove(&dependency.target);
                    }
                }
            }
        }

        // remove reverse dependencies and clean forward edges
        if let Some(dependents) = self.dependents.shift_remove(&module_id) {
            for dependent in dependents {
                if let Some(metadata) = self.modules.get_mut(&dependent.target) {
                    metadata
                        .dependencies
                        .shift_remove(&dependent.reverse_for(module_id));
                }
            }
        }
    }
}
