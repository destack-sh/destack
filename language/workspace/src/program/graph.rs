use destack_source::{ModuleId, ModuleVersion};
use indexmap::map::Entry;
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

use crate::ProfileId;

/// Unique key for a module graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ModuleGraphKey {
    /// The profile id for the graph.
    pub profile_id: ProfileId,
}

impl ModuleGraphKey {
    /// Create a new module graph key.
    pub fn new(profile_id: ProfileId) -> Self {
        Self { profile_id }
    }
}

/// Module dependency graph for a profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleGraph {
    /// The profile id for this graph.
    pub profile_id: ProfileId,
    /// Module dependencies keyed by module id.
    pub dependencies: IndexMap<ModuleId, IndexSet<ModuleId>>,
    /// Reverse dependencies keyed by module id.
    pub dependents: IndexMap<ModuleId, IndexSet<ModuleId>>,
    /// Versions of modules included in the graph.
    pub module_versions: IndexMap<ModuleId, ModuleVersion>,
}

impl ModuleGraph {
    /// Create an empty module graph for a profile.
    pub fn new(profile_id: ProfileId) -> Self {
        Self {
            profile_id,
            dependencies: IndexMap::new(),
            dependents: IndexMap::new(),
            module_versions: IndexMap::new(),
        }
    }

    /// Update the dependencies for a module.
    pub fn update_module(
        &mut self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        dependencies: Vec<ModuleId>,
    ) {
        // normalize and sort dependency list
        let mut sorted = dependencies;
        sorted.sort_unstable();
        sorted.dedup();

        // snapshot previous dependencies for reverse updates
        let previous = self.dependencies.get(&module_id).cloned();

        // update dependency set
        let mut next_set = IndexSet::new();
        for dep in sorted {
            next_set.insert(dep);
        }
        self.dependencies.insert(module_id, next_set.clone());

        // track module version
        self.module_versions.insert(module_id, module_version);

        // remove stale reverse edges
        if let Some(previous) = previous {
            for dep in previous {
                if next_set.contains(&dep) {
                    continue;
                }
                if let Some(entry) = self.dependents.get_mut(&dep) {
                    entry.shift_remove(&module_id);
                    if entry.is_empty() {
                        self.dependents.shift_remove(&dep);
                    }
                }
            }
        }

        // add new reverse edges
        for dep in next_set {
            match self.dependents.entry(dep) {
                Entry::Occupied(mut entry) => {
                    entry.get_mut().insert(module_id);
                }
                Entry::Vacant(entry) => {
                    let mut set = IndexSet::new();
                    set.insert(module_id);
                    entry.insert(set);
                }
            }
        }
    }

    /// Get the dependency list for a module.
    pub fn dependencies_for(&self, module_id: ModuleId) -> Vec<ModuleId> {
        self.dependencies
            .get(&module_id)
            .map(|deps| deps.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Get the dependent list for a module.
    pub fn dependents_for(&self, module_id: ModuleId) -> Vec<ModuleId> {
        self.dependents
            .get(&module_id)
            .map(|deps| deps.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Remove a module from the graph.
    pub fn remove_module(&mut self, module_id: ModuleId) {
        // remove forward dependencies and clean reverse edges
        if let Some(dependencies) = self.dependencies.shift_remove(&module_id) {
            for dependency in dependencies {
                if let Some(entry) = self.dependents.get_mut(&dependency) {
                    entry.shift_remove(&module_id);
                    if entry.is_empty() {
                        self.dependents.shift_remove(&dependency);
                    }
                }
            }
        }

        // remove reverse dependencies and clean forward edges
        if let Some(dependents) = self.dependents.shift_remove(&module_id) {
            for dependent in dependents {
                if let Some(entry) = self.dependencies.get_mut(&dependent) {
                    entry.shift_remove(&module_id);
                    if entry.is_empty() {
                        self.dependencies.shift_remove(&dependent);
                    }
                }
            }
        }

        // drop module version tracking
        self.module_versions.shift_remove(&module_id);
    }
}
