use std::sync::Arc;

use destack_serde::Reflect;
use destack_source::{ModuleId, PackageId, ProfileId};
use indexmap::IndexMap;
use rustc_hash::{FxHashMap, FxHashSet};
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
    /// Index the own modules' edges over the own and foreign universe, foreign modules as leaves.
    fn from_edges(
        modules: &[ModuleId],
        foreign: &[ModuleId],
        edges: &IndexMap<ModuleId, Arc<[ModuleId]>>,
    ) -> Result<Self, ModuleId> {
        let universe = modules.iter().chain(foreign).copied().collect::<Vec<_>>();
        let module_index = module_index_map(&universe);
        let mut offsets = Vec::with_capacity(universe.len() + 1);
        let mut targets = Vec::new();

        // write each own module's outgoing edges as dense target indexes
        offsets.push(0);
        for module in modules {
            let edges = edges.get(module).ok_or(*module)?;
            for target in edges.iter() {
                let target = module_index.get(target).copied().ok_or(*target)?;
                targets.push(target);
            }
            offsets.push(targets.len() as u32);
        }

        // close each foreign leaf without edges
        for _ in foreign {
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

/// Import graph over the modules of one package under one profile.
#[derive(Debug, Clone, Hash, Serialize, Deserialize, Reflect)]
pub struct ModuleGraph {
    /// The package this graph belongs to.
    package: PackageId,
    /// The profile this graph belongs to.
    profile: ProfileId,
    /// The package's own modules sorted by stable id.
    modules: Arc<[ModuleId]>,
    /// The imported modules of other packages sorted by stable id, leaves of this graph.
    foreign: Arc<[ModuleId]>,
    /// Import edges over the own modules followed by the foreign ones.
    edges: ModuleEdges,
    /// The interface implementations the package's own modules declare, sorted by interface.
    implementations: Arc<[Implementation]>,
}

impl ModuleGraph {
    /// Build one module graph from the complete import edges of a package's modules.
    pub fn from_edges(
        package: PackageId,
        profile: ProfileId,
        edges: IndexMap<ModuleId, Arc<[ModuleId]>>,
        mut implementations: Vec<Implementation>,
    ) -> Result<Self, ModuleId> {
        let mut modules = edges.keys().copied().collect::<Vec<_>>();
        modules.sort_unstable();
        let mut foreign = edges
            .values()
            .flat_map(|targets| targets.iter().copied())
            .filter(|target| modules.binary_search(target).is_err())
            .collect::<Vec<_>>();
        foreign.sort_unstable();
        foreign.dedup();
        let edges = ModuleEdges::from_edges(&modules, &foreign, &edges)?;
        implementations.sort_unstable();
        implementations.dedup();

        Ok(Self {
            package,
            profile,
            modules: Arc::from(modules),
            foreign: Arc::from(foreign),
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
                .map(|target| self.universe_module(*target as usize))
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

        Self::from_edges(self.package, self.profile, edges, implementations)
    }

    /// Return the package this graph belongs to.
    pub fn package(&self) -> PackageId {
        self.package
    }

    /// Return the profile this graph belongs to.
    pub fn profile(&self) -> ProfileId {
        self.profile
    }

    /// Return the package's own modules, sorted.
    pub fn modules(&self) -> &[ModuleId] {
        &self.modules
    }

    /// Return whether one module is an own module of this graph.
    pub fn contains(&self, module: ModuleId) -> bool {
        self.modules.binary_search(&module).is_ok()
    }

    /// Iterate the import targets of one own module, none for a module this graph does not hold.
    pub fn edge_targets(&self, module: ModuleId) -> Option<impl Iterator<Item = ModuleId> + '_> {
        let index = self.modules.binary_search(&module).ok()?;
        let targets = self.edges.targets(index).iter();

        Some(targets.map(|target| self.universe_module(*target as usize)))
    }

    /// Return outgoing import edges for one own module.
    pub fn edges(&self, module: ModuleId) -> Option<Arc<[ModuleId]>> {
        let index = self.modules.binary_search(&module).ok()?;
        let targets = self
            .edges
            .targets(index)
            .iter()
            .map(|target| self.universe_module(*target as usize))
            .collect();

        Some(targets)
    }

    /// Return whether one own module's import edges equal an external edge list.
    pub fn edges_equal(&self, module: ModuleId, edges: &[ModuleId]) -> bool {
        let Ok(index) = self.modules.binary_search(&module) else {
            return false;
        };
        let targets = self.edges.targets(index);
        if targets.len() != edges.len() {
            return false;
        }

        targets
            .iter()
            .zip(edges)
            .all(|(target, edge)| self.universe_module(*target as usize) == *edge)
    }

    /// Return sorted modules reachable from the given roots over the graphs of every package they cross.
    ///
    /// The error names the first module its package graph does not hold.
    pub fn reachable_across(
        graphs: &[Arc<ModuleGraph>],
        roots: &[ModuleId],
    ) -> Result<Vec<ModuleId>, ModuleId> {
        let by_package = graphs
            .iter()
            .map(|graph| (graph.package, graph))
            .collect::<FxHashMap<_, _>>();
        let mut visited = FxHashSet::default();
        let mut pending = roots.to_vec();
        let mut reachable = Vec::new();

        // walk import edges through the owning graph of each module
        while let Some(module) = pending.pop() {
            if !visited.insert(module) {
                continue;
            }
            reachable.push(module);
            let targets = by_package
                .get(&module.package_id)
                .and_then(|graph| graph.edge_targets(module))
                .ok_or(module)?;
            pending.extend(targets);
        }
        reachable.sort_unstable();

        Ok(reachable)
    }

    /// Return the module standing at one dense index of the own and foreign universe.
    fn universe_module(&self, index: usize) -> ModuleId {
        match index.checked_sub(self.modules.len()) {
            Some(foreign) => self.foreign[foreign],
            None => self.modules[index],
        }
    }

    /// Return all interface implementations declared across the package's own modules.
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
            | ArtifactProjectionKey::ProgramAnalysisFunctionEffects(_)
            | ArtifactProjectionKey::Payload => None,
        }
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
