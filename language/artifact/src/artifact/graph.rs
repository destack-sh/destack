use std::sync::Arc;

use destack_serde::Reflect;
use destack_source::{ModuleId, ProfileId};
use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

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
}

/// One exported extension of a target declared in its own package.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct Implementation {
    /// The implemented interface.
    pub interface: GlobalSymbolId,
    /// The implementing extension symbol.
    pub symbol: GlobalSymbolId,
    /// The implementing target root, absent for blankets.
    pub root: Option<GlobalSymbolId>,
}

/// Import graph over the modules of one profile.
#[derive(Debug, Clone, Hash, Serialize, Deserialize, Reflect)]
pub struct ModuleGraph {
    /// The profile this graph belongs to.
    profile: ProfileId,
    /// Modules sorted by stable id.
    modules: Arc<[ModuleId]>,
    /// Import edges over the module universe.
    edges: ModuleEdges,
    /// All interface implementations declared across the graph's modules, sorted by interface.
    implementations: Arc<[Implementation]>,
}

impl ModuleGraph {
    /// Build one module graph from complete import edges.
    pub fn from_edges(
        profile: ProfileId,
        edges: IndexMap<ModuleId, Arc<[ModuleId]>>,
        mut implementations: Vec<Implementation>,
    ) -> Result<Self, ModuleId> {
        let mut modules = edges.keys().copied().collect::<Vec<_>>();
        modules.sort_unstable();
        let modules = Arc::<[ModuleId]>::from(modules);
        let edges = ModuleEdges::from_edges(&modules, &edges)?;
        implementations.sort_unstable();
        implementations.dedup();

        Ok(Self {
            profile,
            modules,
            edges,
            implementations: Arc::from(implementations),
        })
    }

    /// Derive a module graph after changing edges and removing modules.
    pub fn derive(
        &self,
        updated_edges: IndexMap<ModuleId, Arc<[ModuleId]>>,
        mut removed_modules: Vec<ModuleId>,
        implementations: Vec<Implementation>,
    ) -> Result<Self, ModuleId> {
        // index removals for filtering sources and targets
        removed_modules.sort_unstable();
        removed_modules.dedup();

        // merge retained edges with the updates
        let mut edges = IndexMap::new();
        for (index, module) in self.modules.iter().enumerate() {
            if removed_modules.binary_search(module).is_ok() || updated_edges.contains_key(module) {
                continue;
            }
            let targets = self
                .edges
                .targets(index)
                .iter()
                .map(|target| self.modules[*target as usize])
                .filter(|target| removed_modules.binary_search(target).is_err())
                .collect::<Arc<[ModuleId]>>();
            edges.insert(*module, targets);
        }
        for (module, targets) in updated_edges {
            // removal wins over an edge update for the same module
            if removed_modules.binary_search(&module).is_ok() {
                continue;
            }
            let targets = targets
                .iter()
                .copied()
                .filter(|target| removed_modules.binary_search(target).is_err())
                .collect::<Arc<[ModuleId]>>();
            edges.insert(module, targets);
        }

        Self::from_edges(self.profile, edges, implementations)
    }

    /// Return the profile this graph belongs to.
    pub fn profile(&self) -> ProfileId {
        self.profile
    }

    /// Return the sorted module universe.
    pub fn modules(&self) -> &[ModuleId] {
        &self.modules
    }

    /// Return whether one module is part of this graph.
    pub fn contains(&self, module: ModuleId) -> bool {
        self.module_index(module).is_some()
    }

    /// Return outgoing import edges for one module.
    pub fn edges(&self, module: ModuleId) -> Option<Arc<[ModuleId]>> {
        let index = self.module_index(module)?;
        let targets = self
            .edges
            .targets(index)
            .iter()
            .map(|target| self.modules[*target as usize])
            .collect();

        Some(targets)
    }

    /// Return whether one module's import edges equal an external edge list.
    pub fn edges_equal(&self, module: ModuleId, edges: &[ModuleId]) -> bool {
        let Some(index) = self.module_index(module) else {
            return false;
        };
        let targets = self.edges.targets(index);
        if targets.len() != edges.len() {
            return false;
        }

        targets
            .iter()
            .zip(edges)
            .all(|(target, edge)| self.modules[*target as usize] == *edge)
    }

    /// Return sorted modules reachable from the given roots over import edges.
    pub fn reachable(&self, roots: &[ModuleId]) -> Vec<ModuleId> {
        let mut visited = vec![false; self.modules.len()];
        let mut pending = roots
            .iter()
            .filter_map(|root| self.module_index(*root))
            .collect::<Vec<_>>();

        // walk import edges breadth first
        let mut reachable = Vec::new();
        while let Some(index) = pending.pop() {
            if visited[index] {
                continue;
            }
            visited[index] = true;
            reachable.push(self.modules[index]);
            pending.extend(
                self.edges
                    .targets(index)
                    .iter()
                    .map(|target| *target as usize),
            );
        }
        reachable.sort_unstable();

        reachable
    }

    /// Return all interface implementations declared across the graph's modules.
    pub fn implementations(&self) -> &[Implementation] {
        &self.implementations
    }

    /// Return the implementations of one interface.
    pub fn interface_implementations(&self, interface: GlobalSymbolId) -> &[Implementation] {
        let start = self
            .implementations
            .partition_point(|implementation| implementation.interface < interface);
        let end = start
            + self.implementations[start..]
                .partition_point(|implementation| implementation.interface == interface);

        &self.implementations[start..end]
    }

    /// Iterate the distinct interfaces with implementations in the graph.
    pub fn implemented_interfaces(&self) -> impl Iterator<Item = GlobalSymbolId> + '_ {
        self.implementations
            .chunk_by(|left, right| left.interface == right.interface)
            .map(|run| run[0].interface)
    }

    /// Return the stable fingerprint of one projected module graph value.
    pub(crate) fn fingerprint_projection(
        &self,
        projection: ArtifactProjectionKey,
    ) -> Option<ArtifactProjectionFingerprint> {
        match projection {
            ArtifactProjectionKey::ModuleGraphModules => {
                Some(ArtifactProjectionFingerprint::new(self.modules()))
            }
            ArtifactProjectionKey::ModuleGraphEdges(module) => {
                let edges = self.edges(module)?;

                Some(ArtifactProjectionFingerprint::new(edges.as_ref()))
            }
            ArtifactProjectionKey::ModuleGraphImplementations(interface) => Some(
                ArtifactProjectionFingerprint::new(self.interface_implementations(interface)),
            ),
            ArtifactProjectionKey::DirResolvedComponentRelations
            | ArtifactProjectionKey::Payload => None,
        }
    }

    /// Return the dense index of one module.
    fn module_index(&self, module: ModuleId) -> Option<usize> {
        self.modules.binary_search(&module).ok()
    }
}

/// Index one dense module universe by module id.
fn module_index_map(modules: &[ModuleId]) -> FxHashMap<ModuleId, u32> {
    modules
        .iter()
        .enumerate()
        .map(|(index, module)| (*module, index as u32))
        .collect()
}
