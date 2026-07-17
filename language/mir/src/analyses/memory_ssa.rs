use std::collections::{HashMap, HashSet, VecDeque};

use crate as mir;
use smallvec::SmallVec;

use crate::{
    AliasAnalysis, Analysis, AnalysisId, ControlFlowGraph, DominatorTree, FunctionAnalysis,
    FunctionAnalysisCache, MemoryRegion, NodeTable, ReferenceLocation, StorageRoot, TargetLayout,
    TypeKey, ValueDefinitions, ValueTypes, collect_reachable_blocks, compute_dominance_frontiers,
};

/// Identifier for a memory access in MemorySSA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryAccessId(u32);

impl MemoryAccessId {
    /// Create an access id from an index.
    fn from_index(index: usize) -> Self {
        Self(index as u32)
    }

    /// Return the vector index for this access id.
    fn index(self) -> usize {
        self.0 as usize
    }
}

/// Source operation for one MemorySSA access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryAccessSource {
    /// Access produced by a MIR instruction.
    Instruction(mir::LocalNodeId<mir::Instruction>),
    /// Access produced by a block terminator.
    Terminator(mir::LocalNodeId<mir::Block>),
}

impl MemoryAccessSource {
    /// Return the instruction source when this access has one.
    pub fn instruction(self) -> Option<mir::LocalNodeId<mir::Instruction>> {
        match self {
            Self::Instruction(instruction) => Some(instruction),
            Self::Terminator(_) => None,
        }
    }

    /// Return the terminator block when this access has one.
    pub fn terminator(self) -> Option<mir::LocalNodeId<mir::Block>> {
        match self {
            Self::Instruction(_) => None,
            Self::Terminator(block) => Some(block),
        }
    }
}

/// Memory access effects for an instruction.
#[derive(Debug, Clone)]
pub struct MemoryAccessEffect {
    /// Whether the instruction reads memory.
    pub reads: bool,
    /// Whether the instruction writes memory.
    pub writes: bool,
    /// Whether the instruction is volatile.
    pub is_volatile: bool,
    /// Whether the instruction acts as a memory barrier.
    pub is_barrier: bool,
    /// The memory region being accessed.
    pub region: MemoryRegion,
}

/// Query information for clobbering access lookups.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct MemoryAccessQuery {
    /// The memory region being accessed.
    region: MemoryRegion,
}

impl MemoryAccessQuery {
    /// Create a query from a full access effect.
    fn from_effect(effect: &MemoryAccessEffect) -> Self {
        Self {
            region: effect.region.clone(),
        }
    }

    /// Create a query from a region with no alias tables.
    fn from_region(region: &MemoryRegion) -> Self {
        Self {
            region: region.clone(),
        }
    }
}

impl MemoryAccessEffect {
    /// Return whether this effect may clobber one reference location.
    pub fn clobbers_location(&self, location: &ReferenceLocation, alias: &AliasAnalysis) -> bool {
        if self.is_barrier {
            return true;
        }

        self.writes && self.may_touch_location(location, alias)
    }

    /// Return whether this effect may touch one reference location.
    pub fn may_touch_location(&self, location: &ReferenceLocation, alias: &AliasAnalysis) -> bool {
        // reject disjoint memory spaces
        if !self.region.spaces().may_alias(location.spaces()) {
            return false;
        }

        // local accesses can touch local address references
        if let MemoryRegion::Local(local) = self.region {
            let storage = StorageRoot::LocalSlot(local);

            return alias.may_touch_root(location, &storage);
        }

        // reference accesses use the alias relation
        if let MemoryRegion::Reference {
            access: effect_access,
            ..
        } = &self.region
        {
            return alias.alias(effect_access, location).may_alias();
        }

        // imprecise accesses touch every compatible location
        true
    }

    /// Return whether this effect is trackable by memory optimizations.
    pub fn is_trackable(&self) -> bool {
        !self.is_volatile && !self.is_barrier && !matches!(self.region, MemoryRegion::Any { .. })
    }

    /// Return whether this effect describes the same region as another effect.
    pub fn matches_region(&self, alias: &AliasAnalysis, other: &MemoryAccessEffect) -> bool {
        // reject disjoint memory spaces
        if !self.region.spaces().may_alias(other.region.spaces()) {
            return false;
        }

        // compare concrete locations
        match (&self.region, &other.region) {
            (MemoryRegion::Local(local), MemoryRegion::Local(other_local)) => local == other_local,
            (
                MemoryRegion::Reference { access, .. },
                MemoryRegion::Reference {
                    access: other_access,
                    ..
                },
            ) => {
                if !access.is_compatible_with(other_access) {
                    return false;
                }

                alias.alias(access, other_access).is_must_alias()
            }
            _ => false,
        }
    }

    /// Return whether this effect may alias another effect.
    pub fn may_alias(&self, alias: &AliasAnalysis, other: &MemoryAccessEffect) -> bool {
        // reject disjoint memory spaces
        if !self.region.spaces().may_alias(other.region.spaces()) {
            return false;
        }

        // compare concrete locations
        match (&self.region, &other.region) {
            (MemoryRegion::Any { .. }, _) | (_, MemoryRegion::Any { .. }) => true,
            (MemoryRegion::Local(local), MemoryRegion::Local(other)) => local == other,
            (
                MemoryRegion::Reference { access, .. },
                MemoryRegion::Reference {
                    access: other_access,
                    ..
                },
            ) => {
                if !access.is_compatible_with(other_access) {
                    return false;
                }

                alias.alias(access, other_access).may_alias()
            }
            _ => false,
        }
    }

    /// Create an access effect for reads.
    fn read(region: MemoryRegion, is_volatile: bool) -> Self {
        Self {
            reads: true,
            writes: false,
            is_volatile,
            is_barrier: false,
            region,
        }
    }

    /// Create an access effect for writes.
    fn write(region: MemoryRegion, is_volatile: bool) -> Self {
        Self {
            reads: false,
            writes: true,
            is_volatile,
            is_barrier: false,
            region,
        }
    }

    /// Create an access effect for read/write.
    fn read_write(region: MemoryRegion, is_volatile: bool) -> Self {
        Self {
            reads: true,
            writes: true,
            is_volatile,
            is_barrier: false,
            region,
        }
    }

    /// Create a barrier access effect.
    fn barrier() -> Self {
        Self {
            reads: false,
            writes: true,
            is_volatile: false,
            is_barrier: true,
            region: MemoryRegion::Any {
                spaces: mir::StorageSet::ANY,
            },
        }
    }
}

/// MemorySSA access node.
#[derive(Debug, Clone)]
pub enum MemoryNode {
    /// Pseudo access that dominates all memory operations.
    LiveOnEntry,
    /// Phi node merging memory states.
    Phi(MemoryPhi),
    /// Memory definition (writes memory).
    Def(MemoryDef),
    /// Memory use (reads memory).
    Use(MemoryUse),
}

impl MemoryNode {
    /// Return the memory effect payload for this access if available.
    pub fn effect(&self) -> Option<&MemoryAccessEffect> {
        match self {
            Self::Def(def) => Some(&def.effect),
            Self::Use(use_access) => Some(&use_access.effect),
            Self::LiveOnEntry | Self::Phi(_) => None,
        }
    }

    /// Return the instruction id for this access if available.
    pub fn instruction(&self) -> Option<mir::LocalNodeId<mir::Instruction>> {
        // map instruction sourced accesses
        self.source().and_then(MemoryAccessSource::instruction)
    }

    /// Return the source operation for this access if available.
    pub fn source(&self) -> Option<MemoryAccessSource> {
        match self {
            MemoryNode::Def(def) => Some(def.source),
            MemoryNode::Use(use_access) => Some(use_access.source),
            _ => None,
        }
    }
}

/// Memory phi node.
#[derive(Debug, Clone)]
pub struct MemoryPhi {
    /// Block that owns this phi.
    pub block: mir::LocalNodeId<mir::Block>,
    /// Incoming values keyed by predecessor block.
    pub incoming: Vec<(mir::LocalNodeId<mir::Block>, MemoryAccessId)>,
}

/// Memory definition access.
#[derive(Debug, Clone)]
pub struct MemoryDef {
    /// Operation that defines memory.
    pub source: MemoryAccessSource,
    /// Immediate defining access in MemorySSA.
    pub defining_access: Option<MemoryAccessId>,
    /// Memory effects for this operation.
    pub effect: MemoryAccessEffect,
}

impl MemoryDef {
    /// Return whether this definition clobbers a query.
    fn clobbers_query(&self, query: &MemoryAccessQuery, alias: &AliasAnalysis) -> bool {
        // treat barriers as clobbering all memory
        if self.effect.is_barrier {
            return true;
        }

        // ignore non writing accesses
        if !self.effect.writes {
            return false;
        }

        // disambiguate by region sets
        if !self.effect.region.spaces().may_alias(query.region.spaces()) {
            return false;
        }

        // check for local memory
        if let MemoryRegion::Local(local) = &query.region {
            if let MemoryRegion::Local(def_local) = &self.effect.region {
                return def_local == local;
            }

            return false;
        }

        // handle unknown memory spaces
        if matches!(query.region, MemoryRegion::Any { .. }) {
            return self.effect.writes;
        }

        // resolve reference based queries
        let Some(reference_location) = query.region.reference_location() else {
            return self.effect.writes;
        };

        // ignore local defs for reference queries
        if let MemoryRegion::Local(_) = self.effect.region {
            return false;
        }

        // compare reference locations when available
        if let MemoryRegion::Reference {
            access: def_access, ..
        } = &self.effect.region
        {
            return alias.alias(def_access, reference_location).may_alias();
        }

        // imprecise defs clobber every compatible reference location
        matches!(self.effect.region, MemoryRegion::Any { .. })
    }

    /// Return whether this definition clobbers a memory region.
    fn clobbers_region(&self, region: &MemoryRegion, alias: &AliasAnalysis) -> bool {
        // treat barriers as clobbering all memory
        if self.effect.is_barrier {
            return true;
        }

        // ignore non writing accesses
        if !self.effect.writes {
            return false;
        }

        // disambiguate by region sets
        if !self.effect.region.spaces().may_alias(region.spaces()) {
            return false;
        }

        // check for local memory
        if let MemoryRegion::Local(local) = region {
            if let MemoryRegion::Local(def_local) = &self.effect.region {
                return def_local == local;
            }

            return false;
        }

        // handle unknown memory spaces
        if matches!(region, MemoryRegion::Any { .. }) {
            return self.effect.writes;
        }

        // check reference based aliasing
        let Some(reference_location) = region.reference_location() else {
            return self.effect.writes;
        };

        if let MemoryRegion::Local(_) = self.effect.region {
            return false;
        }

        // compare reference locations when available
        if let MemoryRegion::Reference {
            access: def_access, ..
        } = &self.effect.region
        {
            return alias.alias(def_access, reference_location).may_alias();
        }

        // imprecise defs clobber every compatible reference location
        matches!(self.effect.region, MemoryRegion::Any { .. })
    }

    /// Return the source instruction when this def has one.
    pub fn instruction(&self) -> Option<mir::LocalNodeId<mir::Instruction>> {
        self.source.instruction()
    }
}

/// Memory use access.
#[derive(Debug, Clone)]
pub struct MemoryUse {
    /// Operation that reads memory.
    pub source: MemoryAccessSource,
    /// Immediate defining access in MemorySSA.
    pub defining_access: Option<MemoryAccessId>,
    /// Memory effects for this operation.
    pub effect: MemoryAccessEffect,
}

impl MemoryUse {
    /// Return the source instruction when this use has one.
    pub fn instruction(&self) -> Option<mir::LocalNodeId<mir::Instruction>> {
        self.source.instruction()
    }
}

/// MemorySSA analysis for a function.
#[derive(Debug)]
pub struct MemorySSA {
    /// All memory accesses indexed by id.
    accesses: Vec<MemoryNode>,
    /// Memory phi nodes indexed by block id.
    block_phis: NodeTable<mir::Block, Option<MemoryAccessId>>,
    /// Memory accesses indexed by instruction id.
    instruction_access: NodeTable<mir::Instruction, Vec<MemoryAccessId>>,
    /// Memory accesses indexed by terminator block id.
    terminator_access: NodeTable<mir::Block, Vec<MemoryAccessId>>,
    /// Memory accesses indexed by block id.
    block_accesses: NodeTable<mir::Block, Vec<MemoryAccessId>>,
    /// Live on entry access id.
    live_on_entry: MemoryAccessId,
}

impl MemorySSA {
    /// Build MemorySSA for a function.
    fn build(
        function: &mir::Function,
        tree: &mir::Tree,
        cfg: &ControlFlowGraph,
        domtree: &DominatorTree,
        definitions: &ValueDefinitions,
        value_types: &ValueTypes,
        memory_table: &mir::MemoryTable,
        effect_table: &mir::EffectTable,
        target_layout: TargetLayout,
    ) -> Self {
        // handle imported functions
        let entry = match function.entry() {
            Some(entry) => entry,
            None => {
                let live_on_entry = MemoryAccessId::from_index(0);
                return Self {
                    accesses: vec![MemoryNode::LiveOnEntry],
                    block_phis: NodeTable::new(),
                    instruction_access: NodeTable::new(),
                    terminator_access: NodeTable::new(),
                    block_accesses: NodeTable::new(),
                    live_on_entry,
                };
            }
        };

        // collect memory accesses and definition blocks
        let mut access_collector = MemoryAccessCollector::new(
            function,
            tree,
            definitions,
            value_types,
            memory_table,
            effect_table,
            target_layout,
        );
        let collected = access_collector.collect(entry);

        // compute dominance frontier for memory defs
        let dominance_frontier =
            compute_dominance_frontiers(&collected.reachable_blocks, cfg, domtree);

        // insert memory phis for join points
        let phi_blocks = compute_phi_blocks(
            &collected.def_blocks,
            &dominance_frontier,
            &collected.reachable,
        );

        // create memory access table with live on entry
        let mut accesses = Vec::new();
        accesses.push(MemoryNode::LiveOnEntry);
        let live_on_entry = MemoryAccessId::from_index(0);

        // create block phis
        let mut block_phis = NodeTable::from_nodes(function.blocks(), || None);
        for block in &phi_blocks {
            let phi_id = MemoryAccessId::from_index(accesses.len());
            accesses.push(MemoryNode::Phi(MemoryPhi {
                block: *block,
                incoming: Vec::new(),
            }));
            *block_phis.get_mut(*block) = Some(phi_id);
        }

        // create instruction memory accesses
        let instructions = function
            .blocks()
            .iter()
            .flat_map(|block| tree.get(*block).instructions.iter().copied())
            .collect::<Vec<_>>();
        let mut instruction_access = NodeTable::from_nodes(&instructions, Vec::new);
        let mut terminator_access = NodeTable::from_nodes(function.blocks(), Vec::new);
        let mut block_accesses = NodeTable::from_nodes(function.blocks(), Vec::new);

        for &block in &collected.reachable_blocks {
            let access_list = collected.block_accesses.get(block);
            if access_list.is_empty() {
                continue;
            };
            let mut block_list = Vec::new();

            for access in access_list {
                let access_id = MemoryAccessId::from_index(accesses.len());
                match access {
                    CollectedAccess::Use { source, effect } => {
                        accesses.push(MemoryNode::Use(MemoryUse {
                            source: *source,
                            defining_access: None,
                            effect: effect.clone(),
                        }));
                        match source {
                            MemoryAccessSource::Instruction(instruction) => {
                                instruction_access.get_mut(*instruction).push(access_id);
                            }
                            MemoryAccessSource::Terminator(block) => {
                                terminator_access.get_mut(*block).push(access_id);
                            }
                        }
                    }
                    CollectedAccess::Def { source, effect } => {
                        accesses.push(MemoryNode::Def(MemoryDef {
                            source: *source,
                            defining_access: None,
                            effect: effect.clone(),
                        }));
                        match source {
                            MemoryAccessSource::Instruction(instruction) => {
                                instruction_access.get_mut(*instruction).push(access_id);
                            }
                            MemoryAccessSource::Terminator(block) => {
                                terminator_access.get_mut(*block).push(access_id);
                            }
                        }
                    }
                }
                block_list.push(access_id);
            }

            *block_accesses.get_mut(block) = block_list;
        }

        // assemble memory ssa state
        let mut ssa = Self {
            accesses,
            block_phis,
            instruction_access,
            terminator_access,
            block_accesses,
            live_on_entry,
        };

        // rename memory accesses
        let mut renamer = MemoryRenamer::new(tree, domtree, entry, &collected.reachable_blocks);
        renamer.rename(&mut ssa);

        // return the finalized ssa
        ssa
    }

    /// Return the live on entry access id.
    pub fn live_on_entry(&self) -> MemoryAccessId {
        self.live_on_entry
    }

    /// Return the access node for an id.
    pub fn access(&self, id: MemoryAccessId) -> &MemoryNode {
        &self.accesses[id.index()]
    }

    /// Return the first memory access for an instruction if present.
    pub fn instruction_access(
        &self,
        instruction: mir::LocalNodeId<mir::Instruction>,
    ) -> Option<MemoryAccessId> {
        self.instruction_accesses(instruction)
            .and_then(|accesses| accesses.first().copied())
    }

    /// Return all memory accesses for an instruction if present.
    pub fn instruction_accesses(
        &self,
        instruction: mir::LocalNodeId<mir::Instruction>,
    ) -> Option<&[MemoryAccessId]> {
        let accesses = self.instruction_access.get(instruction);
        (!accesses.is_empty()).then_some(accesses.as_slice())
    }

    /// Iterate memory effects for one instruction.
    pub fn instruction_effects(
        &self,
        instruction: mir::LocalNodeId<mir::Instruction>,
    ) -> impl Iterator<Item = &MemoryAccessEffect> {
        self.instruction_accesses(instruction)
            .into_iter()
            .flatten()
            .filter_map(|access| self.access(*access).effect())
    }

    /// Return the first memory use access for an instruction.
    pub fn first_use_access(
        &self,
        instruction: mir::LocalNodeId<mir::Instruction>,
    ) -> Option<MemoryAccessId> {
        let accesses = self.instruction_accesses(instruction)?;
        accesses
            .iter()
            .copied()
            .find(|access_id| matches!(self.access(*access_id), MemoryNode::Use(_)))
    }

    /// Return all memory accesses for a block terminator if present.
    pub fn terminator_accesses(
        &self,
        block: mir::LocalNodeId<mir::Block>,
    ) -> Option<&[MemoryAccessId]> {
        let accesses = self.terminator_access.get(block);
        (!accesses.is_empty()).then_some(accesses.as_slice())
    }

    /// Return the memory phi for a block if present.
    pub fn block_phi(&self, block: mir::LocalNodeId<mir::Block>) -> Option<MemoryAccessId> {
        *self.block_phis.get(block)
    }

    /// Return the immediate defining access for a use or def.
    pub fn defining_access(&self, id: MemoryAccessId) -> Option<MemoryAccessId> {
        // read the defining access for uses and defs
        match self.access(id) {
            MemoryNode::Def(def) => def.defining_access,
            MemoryNode::Use(use_access) => use_access.defining_access,
            _ => None,
        }
    }

    /// Compute the clobbering access for a memory use.
    pub fn clobbering_use(
        &self,
        use_access: MemoryAccessId,
        alias: &AliasAnalysis,
    ) -> MemoryAccessId {
        // read the memory use location
        let MemoryNode::Use(use_access_data) = self.access(use_access) else {
            panic!("expected memory use access");
        };

        // resolve the defining access
        let defining_access = use_access_data
            .defining_access
            .expect("memory use missing defining access");

        // build the query for this use
        let query = MemoryAccessQuery::from_effect(&use_access_data.effect);

        // compute the clobbering access
        let mut cache = HashMap::new();
        let mut visiting = HashSet::new();
        self.clobbering_access(defining_access, &query, alias, &mut cache, &mut visiting)
    }

    /// Compute the clobbering access for a memory def.
    pub fn clobbering_def(
        &self,
        def_access: MemoryAccessId,
        alias: &AliasAnalysis,
    ) -> MemoryAccessId {
        // read the memory def location
        let MemoryNode::Def(def_access_data) = self.access(def_access) else {
            panic!("expected memory def access");
        };

        // resolve the defining access
        let defining_access = def_access_data
            .defining_access
            .unwrap_or(self.live_on_entry);

        // build the query for this def
        let query = MemoryAccessQuery::from_effect(&def_access_data.effect);

        // compute the clobbering access
        let mut cache = HashMap::new();
        let mut visiting = HashSet::new();
        self.clobbering_access(defining_access, &query, alias, &mut cache, &mut visiting)
    }

    /// Compute the clobbering access for a read at the given region.
    pub fn clobbering_read(
        &self,
        access_id: MemoryAccessId,
        region: &MemoryRegion,
        alias: &AliasAnalysis,
    ) -> MemoryAccessId {
        // resolve the defining access for this read
        let defining_access = self
            .defining_access(access_id)
            .unwrap_or(self.live_on_entry);

        // build the query for this read
        let query = MemoryAccessQuery::from_region(region);

        // compute the clobbering access
        let mut cache = HashMap::new();
        let mut visiting = HashSet::new();
        self.clobbering_access(defining_access, &query, alias, &mut cache, &mut visiting)
    }

    /// Check if a def access clobbers the given region.
    pub fn def_clobbers_region(
        &self,
        def_access: MemoryAccessId,
        region: &MemoryRegion,
        alias: &AliasAnalysis,
    ) -> bool {
        // only defs can clobber spaces
        let MemoryNode::Def(def_access) = self.access(def_access) else {
            return false;
        };

        def_access.clobbers_region(region, alias)
    }

    /// Check if a def access clobbers another access.
    pub fn def_clobbers_access(
        &self,
        def_access: MemoryAccessId,
        target_access: MemoryAccessId,
        alias: &AliasAnalysis,
    ) -> bool {
        // only defs can clobber accesses
        let MemoryNode::Def(def_access) = self.access(def_access) else {
            return false;
        };

        // fetch the target access effect
        let target_effect = match self.access(target_access) {
            MemoryNode::Use(use_access) => &use_access.effect,
            MemoryNode::Def(def_access) => &def_access.effect,
            _ => return false,
        };

        // build a query for the target effect
        let query = MemoryAccessQuery::from_effect(target_effect);

        def_access.clobbers_query(&query, alias)
    }

    /// Compute the clobbering access for a memory location.
    fn clobbering_access(
        &self,
        access_id: MemoryAccessId,
        query: &MemoryAccessQuery,
        alias: &AliasAnalysis,
        cache: &mut HashMap<(MemoryAccessId, MemoryAccessQuery), MemoryAccessId>,
        visiting: &mut HashSet<MemoryAccessId>,
    ) -> MemoryAccessId {
        // consult the cache
        if let Some(cached) = cache.get(&(access_id, query.clone())) {
            return *cached;
        }

        // break cycles in phi recursion
        if visiting.contains(&access_id) {
            return access_id;
        }

        visiting.insert(access_id);

        // resolve clobber based on access kind
        let result = match self.access(access_id) {
            MemoryNode::LiveOnEntry => access_id,

            MemoryNode::Use(use_access) => {
                let defining_access = use_access
                    .defining_access
                    .expect("memory use missing defining access");
                self.clobbering_access(defining_access, query, alias, cache, visiting)
            }

            MemoryNode::Def(def_access) => {
                if def_access.clobbers_query(query, alias) {
                    access_id
                } else {
                    let defining_access = def_access
                        .defining_access
                        .expect("memory def missing defining access");
                    self.clobbering_access(defining_access, query, alias, cache, visiting)
                }
            }

            MemoryNode::Phi(phi) => {
                let mut incoming_clobber: Option<MemoryAccessId> = None;

                for (_, incoming) in &phi.incoming {
                    let clobber = self.clobbering_access(*incoming, query, alias, cache, visiting);
                    match incoming_clobber {
                        Some(existing) if existing != clobber => {
                            incoming_clobber = Some(access_id);
                            break;
                        }
                        Some(_) => {}
                        None => incoming_clobber = Some(clobber),
                    }
                }

                incoming_clobber.unwrap_or(access_id)
            }
        };

        // store and return
        visiting.remove(&access_id);
        cache.insert((access_id, query.clone()), result);
        result
    }
}

impl Analysis for MemorySSA {
    const ID: AnalysisId = AnalysisId("memory-ssa");
}

impl FunctionAnalysis for MemorySSA {
    fn compute(
        function: &mir::Function,
        tree: &mir::Tree,
        analyses: &FunctionAnalysisCache,
    ) -> Self {
        // read dependencies
        let cfg = analyses.get::<ControlFlowGraph>(function, tree);
        let domtree = analyses.get::<DominatorTree>(function, tree);
        let definitions = analyses.get::<ValueDefinitions>(function, tree);
        let value_types = analyses.get::<ValueTypes>(function, tree);

        Self::build(
            function,
            tree,
            &cfg,
            &domtree,
            &definitions,
            &value_types,
            analyses.memory(),
            analyses.effects(),
            analyses.target_layout(),
        )
    }
}

/// Collected memory access before SSA renaming.
#[derive(Debug, Clone)]
enum CollectedAccess {
    /// Memory use (read).
    Use {
        source: MemoryAccessSource,
        effect: MemoryAccessEffect,
    },
    /// Memory def (write).
    Def {
        source: MemoryAccessSource,
        effect: MemoryAccessEffect,
    },
}

/// Memory access collection results.
struct MemoryAccessCollection {
    /// Memory accesses indexed by block id.
    block_accesses: NodeTable<mir::Block, Vec<CollectedAccess>>,
    /// Blocks that contain memory definitions.
    def_blocks: HashSet<mir::LocalNodeId<mir::Block>>,
    /// Blocks reachable from entry.
    reachable_blocks: Vec<mir::LocalNodeId<mir::Block>>,
    /// Reachable block set.
    reachable: HashSet<mir::LocalNodeId<mir::Block>>,
}

/// Collector for memory accesses.
struct MemoryAccessCollector<'a> {
    /// MIR function under analysis.
    function: &'a mir::Function,
    /// MIR tree.
    tree: &'a mir::Tree,
    /// Value type lookup for reference resolution.
    value_types: &'a ValueTypes,
    /// Value definitions for reference provenance.
    definitions: ValueDefinitions,
    /// Explicit memory access table.
    memory_table: &'a mir::MemoryTable,
    /// Explicit effect table.
    effect_table: &'a mir::EffectTable,
    /// Type key cache.
    type_keys: HashMap<mir::LocalNodeId<mir::Type>, TypeKey>,
    /// Type context for layout sensitive operations.
    target_layout: TargetLayout,
}

impl<'a> MemoryAccessCollector<'a> {
    /// Create a new collector.
    fn new(
        function: &'a mir::Function,
        tree: &'a mir::Tree,
        definitions: &ValueDefinitions,
        value_types: &'a ValueTypes,
        memory_table: &'a mir::MemoryTable,
        effect_table: &'a mir::EffectTable,
        target_layout: TargetLayout,
    ) -> Self {
        // build collector state
        Self {
            function,
            tree,
            value_types,
            definitions: definitions.clone(),
            memory_table,
            effect_table,
            type_keys: HashMap::new(),
            target_layout,
        }
    }

    /// Collect memory accesses for all reachable blocks.
    fn collect(&mut self, entry: mir::LocalNodeId<mir::Block>) -> MemoryAccessCollection {
        // collect reachable blocks
        let reachable_blocks = collect_reachable_blocks(self.function, self.tree, entry);
        let reachable: HashSet<_> = reachable_blocks.iter().copied().collect();

        // collect memory accesses per block
        let mut block_accesses = NodeTable::from_nodes(self.function.blocks(), Vec::new);
        let mut def_blocks = HashSet::new();

        // scan reachable blocks
        for &block_id in &reachable_blocks {
            let block = self.tree.get(block_id);
            let mut accesses = Vec::new();

            // scan instructions for memory effects
            for &instruction_id in &block.instructions {
                let instruction = self.tree.get(instruction_id);
                let effects = self.instruction_effects(instruction_id, instruction);
                if effects.is_empty() {
                    continue;
                }

                for effect in effects {
                    if effect.writes || effect.is_barrier {
                        accesses.push(CollectedAccess::Def {
                            source: MemoryAccessSource::Instruction(instruction_id),
                            effect,
                        });
                        def_blocks.insert(block_id);
                    } else if effect.reads {
                        accesses.push(CollectedAccess::Use {
                            source: MemoryAccessSource::Instruction(instruction_id),
                            effect,
                        });
                    }
                }
            }

            // scan the terminator for call and allocation effects
            let terminator = self.tree.get(block.terminator);
            let effects = self.terminator_effects(block_id, terminator);
            for effect in effects {
                if effect.writes || effect.is_barrier {
                    accesses.push(CollectedAccess::Def {
                        source: MemoryAccessSource::Terminator(block_id),
                        effect,
                    });
                    def_blocks.insert(block_id);
                } else if effect.reads {
                    accesses.push(CollectedAccess::Use {
                        source: MemoryAccessSource::Terminator(block_id),
                        effect,
                    });
                }
            }

            // store collected accesses when present
            *block_accesses.get_mut(block_id) = accesses;
        }

        MemoryAccessCollection {
            block_accesses,
            def_blocks,
            reachable_blocks,
            reachable,
        }
    }

    /// Determine memory effects for an instruction.
    fn instruction_effects(
        &mut self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
    ) -> SmallVec<[MemoryAccessEffect; 2]> {
        // use explicit tables when present
        if let Some(effects) = self.table_effects(instruction_id) {
            return effects;
        }

        // classify instruction memory effects
        match instruction {
            mir::Instruction::Error => {
                panic!("invalid MIR instruction reached optimizer");
            }

            // pure instructions
            mir::Instruction::Const { .. }
            | mir::Instruction::Binary { .. }
            | mir::Instruction::Unary { .. }
            | mir::Instruction::Cast { .. }
            | mir::Instruction::Select { .. }
            | mir::Instruction::GlobalAddr { .. }
            | mir::Instruction::FunctionAddr { .. }
            | mir::Instruction::FunctionBind { .. }
            | mir::Instruction::FunctionEnvironment { .. }
            | mir::Instruction::FunctionPointer { .. }
            | mir::Instruction::FunctionEnvironmentCurrent { .. }
            | mir::Instruction::LocalAddr { .. }
            | mir::Instruction::Aggregate { .. }
            | mir::Instruction::FieldGet { .. }
            | mir::Instruction::FieldSet { .. }
            | mir::Instruction::ElementGet { .. }
            | mir::Instruction::ElementSet { .. }
            | mir::Instruction::FieldAddr { .. }
            | mir::Instruction::ElementAddr { .. }
            | mir::Instruction::VariantNew { .. }
            | mir::Instruction::VariantTag { .. }
            | mir::Instruction::VariantPayload { .. }
            | mir::Instruction::SliceView { .. }
            | mir::Instruction::SliceLength { .. }
            | mir::Instruction::DynamicBind { .. }
            | mir::Instruction::DynamicPayload { .. }
            | mir::Instruction::DynamicType { .. }
            | mir::Instruction::VectorSplat { .. }
            | mir::Instruction::VectorExtract { .. }
            | mir::Instruction::VectorInsert { .. }
            | mir::Instruction::VectorShuffle { .. }
            | mir::Instruction::VectorSelect { .. }
            | mir::Instruction::VectorReduce { .. }
            | mir::Instruction::VectorCompare { .. }
            | mir::Instruction::VectorConvert { .. }
            | mir::Instruction::TensorSplat { .. }
            | mir::Instruction::TensorExtract { .. }
            | mir::Instruction::TensorReshape { .. }
            | mir::Instruction::TensorBroadcast { .. }
            | mir::Instruction::TensorTranspose { .. }
            | mir::Instruction::TensorCast { .. }
            | mir::Instruction::TensorView { .. }
            | mir::Instruction::TensorSlice { .. }
            | mir::Instruction::TensorPad { .. }
            | mir::Instruction::TensorConcat { .. }
            | mir::Instruction::TensorReduce { .. }
            | mir::Instruction::TensorIndexReduce { .. }
            | mir::Instruction::TensorDot { .. }
            | mir::Instruction::TensorConvolution { .. }
            | mir::Instruction::TensorGather { .. }
            | mir::Instruction::TensorScatter { .. }
            | mir::Instruction::TensorCompare { .. }
            | mir::Instruction::TensorSelect { .. }
            | mir::Instruction::TensorConvert { .. }
            | mir::Instruction::Assume { .. }
            | mir::Instruction::ProfileIncrement { .. }
            | mir::Instruction::ProfileSample { .. }
            | mir::Instruction::Breakpoint => SmallVec::new(),
            mir::Instruction::TensorLoad { view, .. } => {
                let view = *view;

                let access_type = self.reference_location_type(view);
                let reference_kind = self.reference_kind(view);
                let reference_space = self.reference_space(view);
                let mut effect = MemoryAccessEffect::read(
                    MemoryRegion::from_reference(
                        view,
                        access_type,
                        reference_kind,
                        reference_space,
                        self.target_layout.pointer_bits(),
                    ),
                    false,
                );
                self.apply_reference_region(&mut effect, view);
                Self::single_effect(effect)
            }
            mir::Instruction::TensorStore { view, .. }
            | mir::Instruction::TensorFill { view, .. } => {
                let view = *view;

                let access_type = self.reference_location_type(view);
                let reference_kind = self.reference_kind(view);
                let reference_space = self.reference_space(view);
                let mut effect = MemoryAccessEffect::write(
                    MemoryRegion::from_reference(
                        view,
                        access_type,
                        reference_kind,
                        reference_space,
                        self.target_layout.pointer_bits(),
                    ),
                    false,
                );
                self.apply_reference_region(&mut effect, view);
                Self::single_effect(effect)
            }
            mir::Instruction::TensorCopy { target, source } => {
                let target = *target;
                let source = *source;

                let mut effects = SmallVec::new();
                let target_access = self.reference_location_type(target);
                let target_kind = self.reference_kind(target);
                let target_space = self.reference_space(target);
                let mut target_effect = MemoryAccessEffect::write(
                    MemoryRegion::from_reference(
                        target,
                        target_access,
                        target_kind,
                        target_space,
                        self.target_layout.pointer_bits(),
                    ),
                    false,
                );
                self.apply_reference_region(&mut target_effect, target);
                effects.push(target_effect);

                let source_access = self.reference_location_type(source);
                let source_kind = self.reference_kind(source);
                let source_space = self.reference_space(source);
                let mut source_effect = MemoryAccessEffect::read(
                    MemoryRegion::from_reference(
                        source,
                        source_access,
                        source_kind,
                        source_space,
                        self.target_layout.pointer_bits(),
                    ),
                    false,
                );
                self.apply_reference_region(&mut source_effect, source);
                effects.push(source_effect);

                effects
            }
            mir::Instruction::Load { pointer, .. } => {
                let pointer = *pointer;

                let access_type = self.reference_location_type(pointer);
                let reference_kind = self.reference_kind(pointer);
                let reference_space = self.reference_space(pointer);
                let mut effect = MemoryAccessEffect::read(
                    MemoryRegion::from_reference(
                        pointer,
                        access_type,
                        reference_kind,
                        reference_space,
                        self.target_layout.pointer_bits(),
                    ),
                    false,
                );
                self.apply_reference_region(&mut effect, pointer);
                Self::single_effect(effect)
            }
            mir::Instruction::Store { pointer, .. } => {
                let pointer = *pointer;

                let access_type = self.reference_location_type(pointer);
                let reference_kind = self.reference_kind(pointer);
                let reference_space = self.reference_space(pointer);
                let mut effect = MemoryAccessEffect::write(
                    MemoryRegion::from_reference(
                        pointer,
                        access_type,
                        reference_kind,
                        reference_space,
                        self.target_layout.pointer_bits(),
                    ),
                    false,
                );
                self.apply_reference_region(&mut effect, pointer);
                Self::single_effect(effect)
            }
            mir::Instruction::AtomicLoad { pointer, .. } => {
                let pointer = *pointer;

                let access_type = self.reference_location_type(pointer);
                let reference_kind = self.reference_kind(pointer);
                let reference_space = self.reference_space(pointer);
                let mut effect = MemoryAccessEffect::read(
                    MemoryRegion::from_reference(
                        pointer,
                        access_type,
                        reference_kind,
                        reference_space,
                        self.target_layout.pointer_bits(),
                    ),
                    true,
                );
                self.apply_reference_region(&mut effect, pointer);
                Self::single_effect(effect)
            }
            mir::Instruction::AtomicStore { pointer, .. } => {
                let pointer = *pointer;

                let access_type = self.reference_location_type(pointer);
                let reference_kind = self.reference_kind(pointer);
                let reference_space = self.reference_space(pointer);
                let mut effect = MemoryAccessEffect::write(
                    MemoryRegion::from_reference(
                        pointer,
                        access_type,
                        reference_kind,
                        reference_space,
                        self.target_layout.pointer_bits(),
                    ),
                    true,
                );
                self.apply_reference_region(&mut effect, pointer);
                Self::single_effect(effect)
            }
            mir::Instruction::AtomicCompareExchange { pointer, .. }
            | mir::Instruction::AtomicRmw { pointer, .. } => {
                let pointer = *pointer;

                let access_type = self.reference_location_type(pointer);
                let reference_kind = self.reference_kind(pointer);
                let reference_space = self.reference_space(pointer);
                let mut effect = MemoryAccessEffect::read_write(
                    MemoryRegion::from_reference(
                        pointer,
                        access_type,
                        reference_kind,
                        reference_space,
                        self.target_layout.pointer_bits(),
                    ),
                    true,
                );
                self.apply_reference_region(&mut effect, pointer);
                Self::single_effect(effect)
            }
            mir::Instruction::AtomicFence { .. } => {
                Self::single_effect(MemoryAccessEffect::barrier())
            }
            mir::Instruction::BarrierWrite { .. } => {
                Self::single_effect(MemoryAccessEffect::read_write(
                    MemoryRegion::any_spaces(mir::StorageSet::ANY),
                    false,
                ))
            }
            mir::Instruction::LocalGet { local, .. } => {
                let mut effect = MemoryAccessEffect::read(MemoryRegion::Local(*local), false);
                self.apply_local_region(&mut effect);
                Self::single_effect(effect)
            }
            mir::Instruction::LocalSet { local, .. } => {
                let mut effect = MemoryAccessEffect::write(MemoryRegion::Local(*local), false);
                self.apply_local_region(&mut effect);
                Self::single_effect(effect)
            }
            mir::Instruction::Call { .. } => self.call_effects(instruction_id, instruction),
            mir::Instruction::Free { .. }
            | mir::Instruction::Drop { .. }
            | mir::Instruction::Pin { .. }
            | mir::Instruction::Unpin { .. }
            | mir::Instruction::NewZeroed { .. }
            | mir::Instruction::NewUninit { .. }
            | mir::Instruction::NewComplete { .. }
            | mir::Instruction::NewSliceZeroed { .. }
            | mir::Instruction::NewSliceUninit { .. }
            | mir::Instruction::FrameAllocZeroed { .. }
            | mir::Instruction::FrameAllocUninit { .. } => {
                Self::single_effect(MemoryAccessEffect::read_write(
                    MemoryRegion::any_spaces(mir::StorageSet::ANY),
                    false,
                ))
            }
            mir::Instruction::Intrinsic {
                intrinsic,
                arguments,
                ..
            } => self.intrinsic_effects(*intrinsic, *arguments),
        }
    }

    /// Determine memory effects for a block terminator.
    fn terminator_effects(
        &mut self,
        block_id: mir::LocalNodeId<mir::Block>,
        terminator: &mir::Terminator,
    ) -> SmallVec<[MemoryAccessEffect; 2]> {
        match terminator {
            mir::Terminator::Invoke { .. } | mir::Terminator::TailCall { .. } => self
                .callsite_effects(
                    mir::CallSite::Terminator(block_id),
                    terminator.call_direct_target(),
                ),

            mir::Terminator::NewZeroedTry { .. }
            | mir::Terminator::NewUninitTry { .. }
            | mir::Terminator::NewSliceZeroedTry { .. }
            | mir::Terminator::NewSliceUninitTry { .. } => {
                Self::single_effect(MemoryAccessEffect::read_write(
                    MemoryRegion::any_spaces(mir::StorageSet::ANY),
                    false,
                ))
            }

            mir::Terminator::Error => {
                panic!("invalid MIR terminator reached MemorySSA");
            }

            mir::Terminator::Return { .. }
            | mir::Terminator::Jump { .. }
            | mir::Terminator::Branch { .. }
            | mir::Terminator::Check { .. }
            | mir::Terminator::Switch { .. }
            | mir::Terminator::Yield { .. }
            | mir::Terminator::Panic { .. }
            | mir::Terminator::UnwindResume
            | mir::Terminator::Trap { .. }
            | mir::Terminator::Unreachable => SmallVec::new(),
        }
    }

    /// Convert explicit memory tables into access effects.
    fn table_effects(
        &mut self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
    ) -> Option<SmallVec<[MemoryAccessEffect; 2]>> {
        // read tables when present
        let accesses = self.memory_table.memory_accesses(instruction_id)?;

        // build effect list from tables
        let mut effects = SmallVec::new();
        for access in accesses {
            effects.push(self.effect_from_entry(access));
        }

        Some(effects)
    }

    /// Map one memory access to a MemorySSA effect.
    fn effect_from_entry(&mut self, access: &mir::MemoryAccess) -> MemoryAccessEffect {
        // resolve the target region
        let region = match access.target {
            mir::MemoryTarget::Reference(pointer) => {
                let access_type = self.reference_location_type(pointer);
                let reference_kind = self.reference_kind(pointer);
                let reference_space = self.reference_space(pointer);
                MemoryRegion::from_reference_with_size(
                    pointer,
                    access_type,
                    reference_kind,
                    reference_space,
                    access.byte_len,
                    self.target_layout.pointer_bits(),
                )
            }
            mir::MemoryTarget::Local(local) => MemoryRegion::Local(local),
            mir::MemoryTarget::Global(global) => {
                MemoryRegion::any_spaces(self.tree.get(global).space.space_set())
            }
        };

        // map the access operation to an effect
        let is_volatile = access.requires_exact_position();
        let mut effect = match access.operation {
            mir::MemoryOperation::Read => MemoryAccessEffect::read(region, is_volatile),
            mir::MemoryOperation::Write => MemoryAccessEffect::write(region, is_volatile),
            mir::MemoryOperation::ReadWrite => MemoryAccessEffect::read_write(region, is_volatile),
        };

        // apply target storage
        let spaces = self.entry_space_set(access);
        effect.region.set_spaces(spaces);
        effect
    }

    /// Apply reference region tables to an effect.
    fn apply_reference_region(&mut self, effect: &mut MemoryAccessEffect, reference: mir::Value) {
        let spaces = self.reference_space_set(reference);
        effect.region.set_spaces(spaces);
    }

    /// Apply local region tables to an effect.
    fn apply_local_region(&mut self, effect: &mut MemoryAccessEffect) {
        effect.region.set_spaces(mir::StorageSet::FRAME);
    }

    /// Resolve storage from an access target.
    fn entry_space_set(&mut self, access: &mir::MemoryAccess) -> mir::StorageSet {
        match access.target {
            mir::MemoryTarget::Local(_) => mir::StorageSet::FRAME,
            mir::MemoryTarget::Global(global) => self.tree.get(global).space.space_set(),
            mir::MemoryTarget::Reference(pointer) => self.reference_space_set(pointer),
        }
    }

    /// Resolve the region set for a reference value.
    fn reference_space_set(&mut self, reference: mir::Value) -> mir::StorageSet {
        let Some(space) = self.value_types.reference_space(reference, self.tree) else {
            return mir::StorageSet::ANY;
        };

        space.space_set()
    }

    /// Determine memory effects for a call instruction using tables.
    fn call_effects(
        &mut self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
    ) -> SmallVec<[MemoryAccessEffect; 2]> {
        self.callsite_effects(
            mir::CallSite::Instruction(instruction_id),
            instruction.call_direct_target(),
        )
    }

    /// Determine memory effects for a callsite using tables.
    fn callsite_effects(
        &self,
        callsite: mir::CallSite,
        direct_target: Option<mir::LocalNodeId<mir::Function>>,
    ) -> SmallVec<[MemoryAccessEffect; 2]> {
        // use callsite or callee tables for memory effects
        let call_entries = self.effect_table.call(callsite);
        let mut memory_effects = call_entries
            .map(|tables| tables.memory.clone())
            .or_else(|| self.callee_memory_effects(direct_target));

        // treat missing tables as fully unknown
        let Some(effects) = memory_effects.take() else {
            return Self::single_effect(MemoryAccessEffect::read_write(
                MemoryRegion::any_spaces(mir::StorageSet::ANY),
                false,
            ));
        };

        // skip calls with no memory effects
        if !effects.reads() && !effects.writes() {
            return SmallVec::new();
        }

        // empty storage does not touch program memory
        if effects.storage().is_empty() {
            return SmallVec::new();
        }

        self.effects_from_call_effect(&effects)
    }

    /// Convert call memory effects to MemorySSA effects.
    fn effects_from_call_effect(
        &self,
        effects: &mir::MemoryEffect,
    ) -> SmallVec<[MemoryAccessEffect; 2]> {
        let mut accesses = SmallVec::new();

        // keep shared read/write spaces precise when possible
        if effects.read == effects.write {
            accesses.push(MemoryAccessEffect::read_write(
                MemoryRegion::any_spaces(effects.read),
                false,
            ));
        } else {
            if !effects.read.is_empty() {
                accesses.push(MemoryAccessEffect::read(
                    MemoryRegion::any_spaces(effects.read),
                    false,
                ));
            }

            if !effects.write.is_empty() {
                accesses.push(MemoryAccessEffect::write(
                    MemoryRegion::any_spaces(effects.write),
                    false,
                ));
            }
        }

        accesses
    }

    /// Read memory effects from a direct callee when available.
    fn callee_memory_effects(
        &self,
        function: Option<mir::LocalNodeId<mir::Function>>,
    ) -> Option<mir::MemoryEffect> {
        // only direct calls have callee tables
        let function = function?;
        self.effect_table
            .function(function)
            .map(|tables| tables.memory.clone())
    }

    /// Determine memory effects for an intrinsic.
    fn intrinsic_effects(
        &mut self,
        intrinsic: mir::Intrinsic,
        arguments: mir::ValueSlice,
    ) -> SmallVec<[MemoryAccessEffect; 2]> {
        // load intrinsic arguments
        let args = self.tree.get_values(arguments);

        // classify intrinsic memory effects
        match intrinsic {
            // bit manipulation
            mir::Intrinsic::LeadingZeroCount
            | mir::Intrinsic::TrailingZeroCount
            | mir::Intrinsic::PopulationCount
            | mir::Intrinsic::ByteSwap
            | mir::Intrinsic::BitReverse
            | mir::Intrinsic::RotateLeft
            | mir::Intrinsic::RotateRight => SmallVec::new(),

            // overflowing arithmetic
            mir::Intrinsic::AddOverflow
            | mir::Intrinsic::SubOverflow
            | mir::Intrinsic::MulOverflow => SmallVec::new(),

            // unchecked arithmetic
            mir::Intrinsic::AddUnchecked
            | mir::Intrinsic::SubUnchecked
            | mir::Intrinsic::MulUnchecked
            | mir::Intrinsic::DivUnchecked
            | mir::Intrinsic::RemUnchecked
            | mir::Intrinsic::ShlUnchecked
            | mir::Intrinsic::ShrUnchecked => SmallVec::new(),

            // saturating arithmetic
            mir::Intrinsic::SatAdd | mir::Intrinsic::SatSub => SmallVec::new(),

            // memory operations
            mir::Intrinsic::Memcpy | mir::Intrinsic::Memmove => {
                // collect the memory operands
                let mut effects = SmallVec::new();
                let dst = args.first();
                let src = args.get(1);
                let size = args.get(2).and_then(|len| self.constant_u64(*len));

                // emit read and write effects when operands are present
                match (dst, src) {
                    (Some(dst), Some(src)) => {
                        let dst_type = self.reference_location_type(*dst);
                        let src_type = self.reference_location_type(*src);
                        let dst_kind = self.reference_kind(*dst);
                        let src_kind = self.reference_kind(*src);
                        let dst_space = self.reference_space(*dst);
                        let src_space = self.reference_space(*src);
                        let mut read_effect = MemoryAccessEffect::read(
                            MemoryRegion::from_reference_with_size(
                                *src,
                                src_type,
                                src_kind,
                                src_space,
                                size,
                                self.target_layout.pointer_bits(),
                            ),
                            false,
                        );
                        self.apply_reference_region(&mut read_effect, *src);
                        effects.push(read_effect);

                        let mut write_effect = MemoryAccessEffect::write(
                            MemoryRegion::from_reference_with_size(
                                *dst,
                                dst_type,
                                dst_kind,
                                dst_space,
                                size,
                                self.target_layout.pointer_bits(),
                            ),
                            false,
                        );
                        self.apply_reference_region(&mut write_effect, *dst);
                        effects.push(write_effect);
                    }
                    _ => effects.push(MemoryAccessEffect::read_write(
                        MemoryRegion::any_spaces(mir::StorageSet::ANY),
                        false,
                    )),
                }

                // return the effects
                effects
            }
            mir::Intrinsic::Memset => {
                // collect the memory operands
                let mut effects = SmallVec::new();
                let dst = args.first();
                let size = args.get(2).and_then(|len| self.constant_u64(*len));

                // emit write effects when operands are present
                match dst {
                    Some(dst) => {
                        let dst_type = self.reference_location_type(*dst);
                        let dst_kind = self.reference_kind(*dst);
                        let dst_space = self.reference_space(*dst);
                        let mut effect = MemoryAccessEffect::write(
                            MemoryRegion::from_reference_with_size(
                                *dst,
                                dst_type,
                                dst_kind,
                                dst_space,
                                size,
                                self.target_layout.pointer_bits(),
                            ),
                            false,
                        );
                        self.apply_reference_region(&mut effect, *dst);
                        effects.push(effect);
                    }
                    None => effects.push(MemoryAccessEffect::write(
                        MemoryRegion::any_spaces(mir::StorageSet::ANY),
                        false,
                    )),
                }

                // return the effects
                effects
            }
            mir::Intrinsic::Memcmp => {
                // collect the memory operands
                let mut effects = SmallVec::new();
                let left = args.first();
                let right = args.get(1);
                let size = args.get(2).and_then(|len| self.constant_u64(*len));

                // emit read effects when operands are present
                match (left, right) {
                    (Some(left), Some(right)) => {
                        let left_type = self.reference_location_type(*left);
                        let right_type = self.reference_location_type(*right);
                        let left_kind = self.reference_kind(*left);
                        let right_kind = self.reference_kind(*right);
                        let left_space = self.reference_space(*left);
                        let right_space = self.reference_space(*right);
                        let mut left_effect = MemoryAccessEffect::read(
                            MemoryRegion::from_reference_with_size(
                                *left,
                                left_type,
                                left_kind,
                                left_space,
                                size,
                                self.target_layout.pointer_bits(),
                            ),
                            false,
                        );
                        self.apply_reference_region(&mut left_effect, *left);
                        effects.push(left_effect);

                        let mut right_effect = MemoryAccessEffect::read(
                            MemoryRegion::from_reference_with_size(
                                *right,
                                right_type,
                                right_kind,
                                right_space,
                                size,
                                self.target_layout.pointer_bits(),
                            ),
                            false,
                        );
                        self.apply_reference_region(&mut right_effect, *right);
                        effects.push(right_effect);
                    }
                    _ => effects.push(MemoryAccessEffect::read(
                        MemoryRegion::any_spaces(mir::StorageSet::ANY),
                        false,
                    )),
                }

                // return the effects
                effects
            }
            mir::Intrinsic::PrefetchRead | mir::Intrinsic::PrefetchWrite => {
                // collect the memory operands
                let mut effects = SmallVec::new();
                let pointer = args.first().copied();

                // emit read effects when operands are present
                match pointer {
                    Some(pointer) => {
                        let access_type = self.reference_location_type(pointer);
                        let reference_kind = self.reference_kind(pointer);
                        let reference_space = self.reference_space(pointer);
                        let mut effect = MemoryAccessEffect::read(
                            MemoryRegion::from_reference(
                                pointer,
                                access_type,
                                reference_kind,
                                reference_space,
                                self.target_layout.pointer_bits(),
                            ),
                            false,
                        );
                        self.apply_reference_region(&mut effect, pointer);
                        effects.push(effect);
                    }
                    None => effects.push(MemoryAccessEffect::read(
                        MemoryRegion::any_spaces(mir::StorageSet::ANY),
                        false,
                    )),
                }

                // return the effects
                effects
            }

            // type punning and raw reference ops
            mir::Intrinsic::Transmute
            | mir::Intrinsic::SpaceCast
            | mir::Intrinsic::PointerOffsetFrom
            | mir::Intrinsic::RawEq => SmallVec::new(),

            // tensor operations

            // float math
            mir::Intrinsic::Sqrt
            | mir::Intrinsic::Abs
            | mir::Intrinsic::Fma
            | mir::Intrinsic::CopySign
            | mir::Intrinsic::Min
            | mir::Intrinsic::Max
            | mir::Intrinsic::Sin
            | mir::Intrinsic::Cos
            | mir::Intrinsic::Tan
            | mir::Intrinsic::Asin
            | mir::Intrinsic::Acos
            | mir::Intrinsic::Atan
            | mir::Intrinsic::Atan2
            | mir::Intrinsic::Exp
            | mir::Intrinsic::Exp2
            | mir::Intrinsic::Log
            | mir::Intrinsic::Log2
            | mir::Intrinsic::Log10
            | mir::Intrinsic::Pow
            | mir::Intrinsic::Floor
            | mir::Intrinsic::Ceil
            | mir::Intrinsic::Trunc
            | mir::Intrinsic::Round => SmallVec::new(),

            // compiler hints
            mir::Intrinsic::Expect | mir::Intrinsic::BlackBox => SmallVec::new(),
        }
    }

    /// Resolve a constant byte size from a value when possible.
    fn constant_u64(&self, value: mir::Value) -> Option<u64> {
        // look up the defining instruction
        let instruction_id = self.definitions.instruction(value)?;
        let instruction = self.tree.get(instruction_id);

        // extract integer constants only
        let mir::Instruction::Const { value, .. } = instruction else {
            return None;
        };

        match value {
            mir::Constant::UInt { value, .. } => u64::try_from(*value).ok(),
            mir::Constant::Int { value, .. } => {
                if *value >= 0 {
                    u64::try_from(*value).ok()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Wrap a single access effect in a small vector.
    fn single_effect(effect: MemoryAccessEffect) -> SmallVec<[MemoryAccessEffect; 2]> {
        let mut effects = SmallVec::new();
        effects.push(effect);
        effects
    }

    /// Resolve the access type for a reference value.
    fn reference_location_type(&mut self, reference: mir::Value) -> Option<TypeKey> {
        // reuse cached type keys
        let pointee_type = self
            .value_types
            .reference_referent_type(reference, self.tree)?;

        Some(self.type_key(pointee_type))
    }

    /// Resolve the reference kind for a reference value.
    fn reference_kind(&self, reference: mir::Value) -> Option<mir::ReferenceKind> {
        self.value_types.reference_kind(reference, self.tree)
    }

    /// Resolve the TS++ space for a reference value.
    fn reference_space(&self, reference: mir::Value) -> Option<mir::Space> {
        self.value_types.reference_space(reference, self.tree)
    }

    /// Get or compute a type key.
    fn type_key(&mut self, ty_id: mir::LocalNodeId<mir::Type>) -> TypeKey {
        // reuse cached key when available
        if let Some(existing) = self.type_keys.get(&ty_id) {
            return existing.clone();
        }

        // build and cache the new key
        let key = TypeKey::from_type(ty_id, self.tree);
        self.type_keys.insert(ty_id, key.clone());
        key
    }
}

/// MemorySSA renamer for def use chains.
struct MemoryRenamer<'a> {
    /// MIR tree.
    tree: &'a mir::Tree,
    /// Entry block id.
    entry: mir::LocalNodeId<mir::Block>,
    /// Reachable block set.
    reachable: HashSet<mir::LocalNodeId<mir::Block>>,
    /// Dominator tree children.
    children: NodeTable<mir::Block, Vec<mir::LocalNodeId<mir::Block>>>,
}

impl<'a> MemoryRenamer<'a> {
    /// Create a new renamer.
    fn new(
        tree: &'a mir::Tree,
        domtree: &'a DominatorTree,
        entry: mir::LocalNodeId<mir::Block>,
        reachable_blocks: &[mir::LocalNodeId<mir::Block>],
    ) -> Self {
        // build reachable set
        let reachable: HashSet<_> = reachable_blocks.iter().copied().collect();

        // build dominator tree children map
        let mut children = NodeTable::from_nodes(reachable_blocks, Vec::new);
        for &block in reachable_blocks {
            if let Some(idom) = domtree.immediate_dominator(block) {
                children.get_mut(idom).push(block);
            }
        }

        Self {
            tree,
            entry,
            reachable,
            children,
        }
    }

    /// Rename memory accesses to build SSA form.
    fn rename(&mut self, ssa: &mut MemorySSA) {
        // initialize stack with live on entry
        let mut stack = Vec::new();
        stack.push(ssa.live_on_entry);

        // walk dominator tree
        self.rename_block(ssa, self.entry, &mut stack);
    }

    /// Rename a block and its dominator children.
    fn rename_block(
        &self,
        ssa: &mut MemorySSA,
        block: mir::LocalNodeId<mir::Block>,
        stack: &mut Vec<MemoryAccessId>,
    ) {
        // skip unreachable blocks
        if !self.reachable.contains(&block) {
            return;
        }

        // push block phi if present
        let mut pushed = 0usize;
        if let Some(phi_id) = *ssa.block_phis.get(block) {
            stack.push(phi_id);
            pushed += 1;
        }

        // process block accesses
        for access_id in ssa.block_accesses.get(block).clone() {
            let current = *stack.last().expect("missing memory definition");

            match ssa.accesses.get_mut(access_id.index()) {
                Some(MemoryNode::Use(use_access)) => {
                    use_access.defining_access = Some(current);
                }
                Some(MemoryNode::Def(def_access)) => {
                    def_access.defining_access = Some(current);
                    stack.push(access_id);
                    pushed += 1;
                }
                _ => {}
            }
        }

        // wire phi incoming edges for successors
        let block_data = self.tree.get(block);
        let terminator = self.tree.get(block_data.terminator);
        for successor in terminator.successors(self.tree) {
            if let Some(phi_id) = *ssa.block_phis.get(successor)
                && let Some(MemoryNode::Phi(phi)) = ssa.accesses.get_mut(phi_id.index())
            {
                let incoming = *stack.last().expect("missing memory definition");
                phi.incoming.push((block, incoming));
            }
        }

        // rename children
        for &child in self.children.get(block) {
            self.rename_block(ssa, child, stack);
        }

        // pop access stack for this block
        for _ in 0..pushed {
            stack.pop();
        }
    }
}

/// Compute phi blocks for memory SSA.
fn compute_phi_blocks(
    def_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
    dominance_frontier: &HashMap<
        mir::LocalNodeId<mir::Block>,
        HashSet<mir::LocalNodeId<mir::Block>>,
    >,
    reachable: &HashSet<mir::LocalNodeId<mir::Block>>,
) -> Vec<mir::LocalNodeId<mir::Block>> {
    // worklist seeded with def blocks
    let mut worklist: VecDeque<mir::LocalNodeId<mir::Block>> = def_blocks.iter().copied().collect();
    let mut in_worklist: HashSet<mir::LocalNodeId<mir::Block>> =
        def_blocks.iter().copied().collect();
    let mut phi_blocks = HashSet::new();

    while let Some(block) = worklist.pop_front() {
        in_worklist.remove(&block);

        let Some(frontier) = dominance_frontier.get(&block) else {
            continue;
        };

        for &df_block in frontier {
            if !reachable.contains(&df_block) {
                continue;
            }

            if phi_blocks.insert(df_block)
                && !def_blocks.contains(&df_block)
                && !in_worklist.contains(&df_block)
            {
                worklist.push_back(df_block);
                in_worklist.insert(df_block);
            }
        }
    }

    let mut result: Vec<_> = phi_blocks.into_iter().collect();
    result.sort();
    result
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::analyses::tests::TestProgram;

    /// Extract the effect payload for a memory access.
    fn access_effect(memory_ssa: &MemorySSA, access_id: MemoryAccessId) -> MemoryAccessEffect {
        // unwrap use or def payloads
        match memory_ssa.access(access_id) {
            MemoryNode::Use(use_access) => use_access.effect.clone(),
            MemoryNode::Def(def_access) => def_access.effect.clone(),
            MemoryNode::Phi(_) | MemoryNode::LiveOnEntry => {
                panic!("expected effectful access")
            }
        }
    }

    /// Extract the reference value from a location when available.
    fn reference_from_region(location: &MemoryRegion) -> Option<mir::Value> {
        // unwrap reference backed spaces
        match location {
            MemoryRegion::Reference { access, .. } => Some(access.reference),
            _ => None,
        }
    }

    /// Extract the byte size from a location when available.
    fn size_from_region(location: &MemoryRegion) -> Option<u64> {
        // unwrap reference backed spaces
        match location {
            MemoryRegion::Reference { access, .. } => access.size,
            _ => None,
        }
    }

    /// Collect the memory accesses for an instruction.
    fn instruction_accesses(
        memory_ssa: &MemorySSA,
        instruction: mir::LocalNodeId<mir::Instruction>,
    ) -> Vec<MemoryAccessId> {
        // clone the access list when present
        memory_ssa
            .instruction_accesses(instruction)
            .map(|accesses| accesses.to_vec())
            .unwrap_or_default()
    }

    /// Collect the memory accesses for a terminator.
    fn terminator_accesses(
        memory_ssa: &MemorySSA,
        block: mir::LocalNodeId<mir::Block>,
    ) -> Vec<MemoryAccessId> {
        // clone the access list when present
        memory_ssa
            .terminator_accesses(block)
            .map(|accesses| accesses.to_vec())
            .unwrap_or_default()
    }

    /// MemorySSA links uses to the latest defining access in a straight line.
    #[test]
    fn test_memory_ssa_linear_def_use() {
        let test = TestProgram::new(
            r#"
function test(v0: ref<int32, raw, mutable>): int32 {
entry(v0: ref<int32, raw, mutable>):
    v1: int32 = 1
    store v0, v1
    v2: int32 = load v0
    return v2
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analysis_cache();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let memory_ssa = memory_ssa.as_ref();

        // find memory accesses
        let block = test.tree.get(function.block(0));
        let store_id = block.instructions[1];
        let load_id = block.instructions[2];

        let store_access = memory_ssa
            .instruction_access(store_id)
            .expect("missing store access");
        let load_access = memory_ssa
            .instruction_access(load_id)
            .expect("missing load access");

        // load should depend on store
        assert_eq!(memory_ssa.defining_access(load_access), Some(store_access));

        // store should depend on live on entry
        assert_eq!(
            memory_ssa.defining_access(store_access),
            Some(memory_ssa.live_on_entry())
        );
    }

    /// MemorySSA inserts phis at join points with multiple incoming defs.
    #[test]
    fn test_memory_ssa_phi_at_join() {
        let test = TestProgram::new(
            r#"
function test(v0: ref<int32, raw, mutable>, v1: boolean): int32 {
entry(v0: ref<int32, raw, mutable>, v1: boolean):
    branch v1, b1, b2

b1:
    v2: int32 = 1
    store v0, v2
    jump b3

b2:
    v3: int32 = 2
    store v0, v3
    jump b3

b3:
    v4: int32 = load v0
    return v4
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analysis_cache();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let memory_ssa = memory_ssa.as_ref();

        // fetch join block phi
        let join_block = function.block(3);
        let phi_id = memory_ssa
            .block_phi(join_block)
            .expect("missing memory phi at join");

        // load should depend on the phi
        let load_inst = test.tree.get(join_block).instructions[0];
        let load_access = memory_ssa
            .instruction_access(load_inst)
            .expect("missing load access");
        assert_eq!(memory_ssa.defining_access(load_access), Some(phi_id));

        // phi should have incoming for both predecessors
        let MemoryNode::Phi(phi) = memory_ssa.access(phi_id) else {
            panic!("expected memory phi");
        };

        assert_eq!(phi.incoming.len(), 2);
    }

    /// MemorySSA uses alias analysis to skip non aliasing defs.
    #[test]
    fn test_memory_ssa_clobber_skips_exclusive_def() {
        let test = TestProgram::new(
            r#"
function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v2: int32 = 1
    store v0, v2
    v3: int32 = 2
    store v1, v3
    v4: int32 = load v0
    return v4
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analysis_cache();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let alias = analyses.get::<AliasAnalysis>(function, &test.tree);

        // locate accesses
        let block = test.tree.get(function.block(0));
        let store_v0 = block.instructions[3];
        let load_v0 = block.instructions[6];

        let store_access = memory_ssa
            .instruction_access(store_v0)
            .expect("missing store access");
        let load_access = memory_ssa
            .instruction_access(load_v0)
            .expect("missing load access");

        // clobbering access should be the store to v0
        let clobber = memory_ssa.clobbering_use(load_access, &alias);
        assert_eq!(clobber, store_access);
    }

    /// MemorySSA respects explicit tables over instruction semantics.
    #[test]
    fn test_memory_ssa_entries_overrides_instruction() {
        // input test
        let mut test = TestProgram::new(
            r#"
function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v2: int32 = 1
    store v0, v2
    v3: int32 = 2
    store v1, v3
    v4: int32 = load v0
    return v4
}
"#,
        );

        // locate store and load instructions
        let function_id = test.first_function_id();
        let instructions = test.entry_instructions(function_id);
        let store_v1 = instructions[5];
        let load_v0 = instructions[6];

        // attach tables that retargets the load to v1
        test.insert_reference_location(
            load_v0,
            mir::MemoryOperation::Read,
            mir::Value::new(1),
            Some(4),
        );

        // build analyses
        let function = test.tree.get(function_id);
        let analyses = test.function_analysis_cache();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let alias = analyses.get::<AliasAnalysis>(function, &test.tree);

        // locate memory accesses
        let load_access = memory_ssa
            .instruction_access(load_v0)
            .expect("missing load access");
        let store_access = memory_ssa
            .instruction_access(store_v1)
            .expect("missing store access");

        // clobber should follow the tables target
        let clobber = memory_ssa.clobbering_use(load_access, &alias);
        assert_eq!(clobber, store_access);
    }

    /// Local accesses are tracked independently of reference memory.
    #[test]
    fn test_memory_ssa_local_access() {
        let test = TestProgram::new(
            r#"
function test(): int32 {
    local l0: int32

entry:
    v0: int32 = 7
    local.set l0, v0
    v1: int32 = local.get l0
    return v1
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analysis_cache();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);

        // locate local access
        let block = test.tree.get(function.block(0));
        let store_inst = block.instructions[1];
        let load_inst = block.instructions[2];

        let store_access = memory_ssa
            .instruction_access(store_inst)
            .expect("missing local store access");
        let load_access = memory_ssa
            .instruction_access(load_inst)
            .expect("missing local load access");

        // local load should see the local set
        assert_eq!(memory_ssa.defining_access(load_access), Some(store_access));
    }

    /// Local effects touch references built from local addresses.
    #[test]
    fn test_memory_ssa_local_effect_clobbers_local_address() {
        let test = TestProgram::new(
            r#"
function test(): int32 {
    local l0: int32

entry:
    v0: int32 = 7
    local.set l0, v0
    v1: ref<int32, borrowed, mutable, space(frame)> = local.address l0
    v2: int32 = load v1
    return v2
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analysis_cache();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let alias = analyses.get::<AliasAnalysis>(function, &test.tree);

        // locate the local write and local-address load
        let instructions = test.entry_instructions(function_id);
        let local_set = instructions[1];
        let location = ReferenceLocation::from_reference(mir::Value::new(1));

        // local write must clobber the equivalent local-address reference
        let is_clobbered = memory_ssa
            .instruction_effects(local_set)
            .any(|effect| effect.clobbers_location(&location, &alias));
        assert!(is_clobbered);
    }

    /// Free effects are modeled as read and write effects over any memory.
    #[test]
    fn test_memory_ssa_free_effect_any() {
        let test = TestProgram::new(
            r#"
function test(v0: ref<int32, unique, mutable>): int32 {
entry(v0: ref<int32, unique, mutable>):
    free v0
    v1: int32 = 0
    return v1
}
"#,
        );

        let function_id = test.first_function_id();
        let instructions = test.entry_instructions(function_id);
        let free_inst = instructions[0];

        let function = test.tree.get(function_id);
        let analyses = test.function_analysis_cache();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let memory_ssa = memory_ssa.as_ref();

        let free_access = memory_ssa
            .instruction_access(free_inst)
            .expect("missing free access");
        let effect = access_effect(memory_ssa, free_access);

        assert!(effect.reads);
        assert!(effect.writes);
        assert!(matches!(effect.region, MemoryRegion::Any { .. }));
    }

    /// Allocation effects are modeled as read and write effects over any memory.
    #[test]
    fn test_memory_ssa_alloc_effect_any() {
        let test = TestProgram::new(
            r#"
type Point {
    int32;
}

function test(): ref<Point, managed, mutable> {
entry:
    v0: ref<Point, managed, mutable> = new.zeroed Point
    return v0
}
"#,
        );

        let function_id = test.first_function_id();
        let instructions = test.entry_instructions(function_id);
        let alloc_inst = instructions[0];

        let function = test.tree.get(function_id);
        let analyses = test.function_analysis_cache();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let memory_ssa = memory_ssa.as_ref();

        let alloc_access = memory_ssa
            .instruction_access(alloc_inst)
            .expect("missing alloc access");
        let effect = access_effect(memory_ssa, alloc_access);

        assert!(effect.reads);
        assert!(effect.writes);
        assert!(matches!(effect.region, MemoryRegion::Any { .. }));
    }

    /// Memcpy produces a read followed by a write access for its operands.
    #[test]
    fn test_memory_ssa_memcpy_read_write_effects() {
        let test = TestProgram::new(
            r#"
function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v2: int64 = 4
    intrinsic.memory.raw.copyBytes(v0, v1, v2)
    v3: int32 = load v0
    return v3
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analysis_cache();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let memory_ssa = memory_ssa.as_ref();

        let memcpy_inst = test.first_intrinsic_in_entry(function_id, mir::Intrinsic::Memcpy);
        let accesses = instruction_accesses(memory_ssa, memcpy_inst);

        // ensure we recorded a read and a write
        assert_eq!(accesses.len(), 2);

        // extract effects in order
        let read_effect = access_effect(memory_ssa, accesses[0]);
        let write_effect = access_effect(memory_ssa, accesses[1]);

        assert!(read_effect.reads);
        assert!(!read_effect.writes);
        assert!(write_effect.writes);
        assert!(!write_effect.reads);

        let read_reference =
            reference_from_region(&read_effect.region).expect("missing read reference");
        let write_reference =
            reference_from_region(&write_effect.region).expect("missing write reference");

        let frame_allocs = test.frame_alloc_destinations_in_entry(function_id);
        let dest_value = *frame_allocs.first().expect("missing stack allocation");
        let src_value = *frame_allocs.get(1).expect("missing stack allocation");

        assert_eq!(read_reference, src_value);
        assert_eq!(write_reference, dest_value);

        let read_size = size_from_region(&read_effect.region).expect("missing read size");
        let write_size = size_from_region(&write_effect.region).expect("missing write size");

        assert_eq!(read_size, 4);
        assert_eq!(write_size, 4);
    }

    /// Memcmp produces two read accesses for its operands.
    #[test]
    fn test_memory_ssa_memcmp_read_effects() {
        let test = TestProgram::new(
            r#"
function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v2: int64 = 4
    v3: int32 = intrinsic.memory.raw.compareBytes(v0, v1, v2)
    return v3
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analysis_cache();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let memory_ssa = memory_ssa.as_ref();

        let memcmp_inst = test.first_intrinsic_in_entry(function_id, mir::Intrinsic::Memcmp);
        let accesses = instruction_accesses(memory_ssa, memcmp_inst);

        // ensure we recorded two reads
        assert_eq!(accesses.len(), 2);

        for access_id in accesses {
            let effect = access_effect(memory_ssa, access_id);
            assert!(effect.reads);
            assert!(!effect.writes);
        }
    }

    /// Volatile accesses are marked as volatile effects.
    #[test]
    fn test_memory_ssa_volatile_marks_effects() {
        let mut test = TestProgram::new(
            r#"
function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = load v0
    store v0, v1
    return v1
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);

        // locate volatile instructions
        let block = test.tree.get(function.block(0));
        let volatile_load = block.instructions[1];
        let volatile_store = block.instructions[2];

        // attach volatile memory access entries
        test.insert_reference_location_with_options(
            volatile_load,
            mir::MemoryOperation::Read,
            mir::Value::new(0),
            Some(4),
            true,
            None,
        );
        test.insert_reference_location_with_options(
            volatile_store,
            mir::MemoryOperation::Write,
            mir::Value::new(0),
            Some(4),
            true,
            None,
        );

        let function = test.tree.get(function_id);
        let analyses = test.function_analysis_cache();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let memory_ssa = memory_ssa.as_ref();

        // collect volatile effects
        let load_access = memory_ssa
            .instruction_access(volatile_load)
            .expect("missing volatile load access");
        let store_access = memory_ssa
            .instruction_access(volatile_store)
            .expect("missing volatile store access");

        let load_effect = access_effect(memory_ssa, load_access);
        let store_effect = access_effect(memory_ssa, store_access);

        assert!(load_effect.is_volatile);
        assert!(store_effect.is_volatile);
    }

    /// Atomic accesses are treated as volatile effects.
    #[test]
    fn test_memory_ssa_atomic_marks_effects() {
        let test = TestProgram::new(
            r#"
function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = atomic.load v0, acquire, scope(device)
    atomic.store v0, v1, release, scope(device)
    return v1
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analysis_cache();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let memory_ssa = memory_ssa.as_ref();

        let block = test.tree.get(function.block(0));
        let atomic_load = block.instructions[1];
        let atomic_store = block.instructions[2];

        let load_access = memory_ssa
            .instruction_access(atomic_load)
            .expect("missing atomic load access");
        let store_access = memory_ssa
            .instruction_access(atomic_store)
            .expect("missing atomic store access");

        let load_effect = access_effect(memory_ssa, load_access);
        let store_effect = access_effect(memory_ssa, store_access);

        assert!(load_effect.is_volatile);
        assert!(store_effect.is_volatile);
    }

    /// Atomic fence produces a barrier access.
    #[test]
    fn test_memory_ssa_atomic_fence_barrier() {
        let test = TestProgram::new(
            r#"
function test(): int32 {
entry:
    atomic.fence sequentiallyConsistent, scope(device), storage(device)
    v0: int32 = 0
    return v0
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analysis_cache();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let memory_ssa = memory_ssa.as_ref();

        // locate fence instruction
        let block = test.tree.get(function.block(0));
        let fence_inst = block.instructions[0];
        let fence_access = memory_ssa
            .instruction_access(fence_inst)
            .expect("missing fence access");

        let fence_effect = access_effect(memory_ssa, fence_access);
        assert!(fence_effect.is_barrier);
        assert!(matches!(fence_effect.region, MemoryRegion::Any { .. }));
    }

    /// Calls without precise tables are modeled as read and write effects over any memory.
    #[test]
    fn test_memory_ssa_call_is_any_def() {
        let test = TestProgram::new(
            r#"
external function imported(ref<int32, raw, mutable>): void

function test(v0: ref<int32, raw, mutable>): int32 {
entry(v0: ref<int32, raw, mutable>):
    call imported(v0)
    v1: int32 = 0
    return v1
}
"#,
        );

        // select the defined function
        let function_id = test
            .tree
            .iter_nodes::<mir::Function>()
            .find(|(_, function)| function.entry().is_some())
            .expect("missing function")
            .0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analysis_cache();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let memory_ssa = memory_ssa.as_ref();

        // locate call instruction
        let block = test.tree.get(function.block(0));
        let call_inst = block.instructions[0];
        let call_access = memory_ssa
            .instruction_access(call_inst)
            .expect("missing call access");

        let call_effect = access_effect(memory_ssa, call_access);
        assert!(call_effect.reads);
        assert!(call_effect.writes);
        assert!(matches!(call_effect.region, MemoryRegion::Any { .. }));
    }

    /// Continuation loads see memory effects from invokes.
    #[test]
    fn test_memory_ssa_invoke_clobbers_continuation() {
        let test = TestProgram::new(
            r#"
external function imported(ref<int32, raw, mutable>): void

function test(v0: ref<int32, raw, mutable>): int32 {
entry(v0: ref<int32, raw, mutable>):
    invoke imported(v0) => b1 | b2

b1:
    v1: int32 = load v0
    return v1

b2:
    unwind.resume
}
"#,
        );

        let function_id = test.entry_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analysis_cache();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let memory_ssa = memory_ssa.as_ref();

        // locate the invoke and continuation load
        let entry = function.block(0);
        let continuation = function.block(1);
        let load = test.tree.get(continuation).instructions[0];

        let call_accesses = terminator_accesses(memory_ssa, entry);
        let load_access = memory_ssa
            .instruction_access(load)
            .expect("missing load access");

        // require the invoke to define the continuation memory state
        assert_eq!(call_accesses.len(), 1);
        assert_eq!(
            memory_ssa.defining_access(load_access),
            Some(call_accesses[0])
        );
    }

    /// Call tables no memory suppresses memory accesses.
    #[test]
    fn test_memory_ssa_skips_no_memory_call() {
        let mut test = TestProgram::new(
            r#"
external function imported(ref<int32, raw, mutable>): void

function test(v0: ref<int32, raw, mutable>): int32 {
entry(v0: ref<int32, raw, mutable>):
    call imported(v0)
    v1: int32 = 0
    return v1
}
"#,
        );

        let function_id = test.entry_function_id();
        let (call_inst, _callee) = test.first_call_in_entry(function_id);
        let callsite = mir::CallSite::Instruction(call_inst);
        test.effects.call_mut(callsite).memory = mir::MemoryEffect::none();

        let function = test.tree.get(function_id);
        let analyses = test.function_analysis_cache();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);

        assert!(
            memory_ssa.instruction_accesses(call_inst).is_none(),
            "no memory calls should not create memory accesses"
        );
    }

    /// Memory access entries overrides default instruction effects.
    #[test]
    fn test_memory_ssa_access_entries_override() {
        // build the test test
        let mut test = TestProgram::new(
            r#"
external function imported(ref<int32, raw, mutable>, ref<int32, raw, mutable>): void

function test(v0: ref<int32, raw, mutable>, v1: ref<int32, raw, mutable>): int32 {
entry(v0: ref<int32, raw, mutable>, v1: ref<int32, raw, mutable>):
    call imported(v0, v1)
    v2: int32 = 0
    return v2
}
"#,
        );

        // collect parameter values and the call instruction
        let function_id = test.entry_function_id();
        let param_values = {
            let function = test.tree.get(function_id);
            function
                .parameters
                .iter()
                .map(|param| param.value)
                .collect::<Vec<_>>()
        };
        let (call_inst, _callee) = test.first_call_in_entry(function_id);

        // build explicit access entries
        let read_access = mir::MemoryAccess::plain(
            mir::MemoryOperation::Read,
            mir::MemoryTarget::Reference(param_values[0]),
            Some(4),
            None,
        );
        let write_access = mir::MemoryAccess::plain(
            mir::MemoryOperation::Write,
            mir::MemoryTarget::Reference(param_values[1]),
            Some(4),
            None,
        );

        // attach memory access entries to the call
        test.insert_memory_accesses(call_inst, vec![read_access, write_access]);

        // build analyses
        let function = test.tree.get(function_id);
        let analyses = test.function_analysis_cache();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let memory_ssa = memory_ssa.as_ref();

        // locate access effects for the call
        let accesses = instruction_accesses(memory_ssa, call_inst);
        assert_eq!(accesses.len(), 2);

        // verify read and write effects
        let read_effect = access_effect(memory_ssa, accesses[0]);
        let write_effect = access_effect(memory_ssa, accesses[1]);

        assert!(read_effect.reads);
        assert!(!read_effect.writes);
        assert!(write_effect.writes);
        assert!(!write_effect.reads);

        // verify references and sizes
        let read_reference =
            reference_from_region(&read_effect.region).expect("missing read reference");
        let write_reference =
            reference_from_region(&write_effect.region).expect("missing write reference");
        assert_eq!(read_reference, function.parameters[0].value);
        assert_eq!(write_reference, function.parameters[1].value);

        let read_size = size_from_region(&read_effect.region).expect("missing read size");
        let write_size = size_from_region(&write_effect.region).expect("missing write size");
        assert_eq!(read_size, 4);
        assert_eq!(write_size, 4);
    }

    /// Loop headers get memory phis when defs flow around the backedge.
    #[test]
    fn test_memory_ssa_loop_phi_in_header() {
        let test = TestProgram::new(
            r#"
function test(v0: ref<int32, raw, mutable>, v1: int32): int32 {
entry(v0: ref<int32, raw, mutable>, v1: int32):
    v2: int32 = 0
    store v0, v2
    jump b1(v2)

b1(v3: int32):
    v4: boolean = int.lt.s v3, v1
    branch v4, b2, b3

b2:
    v5: int32 = 1
    store v0, v5
    v6: int32 = int.add v3, v5
    jump b1(v6)

b3:
    v7: int32 = load v0
    return v7
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analysis_cache();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let memory_ssa = memory_ssa.as_ref();

        // find phi for loop header
        let header_block = function.block(1);
        let phi_id = memory_ssa
            .block_phi(header_block)
            .expect("missing loop header phi");

        let MemoryNode::Phi(phi) = memory_ssa.access(phi_id) else {
            panic!("expected memory phi");
        };

        let incoming_blocks: HashSet<_> = phi.incoming.iter().map(|(block, _)| *block).collect();
        let body_block = function.block(2);

        assert!(incoming_blocks.contains(&function.block(0)));
        assert!(incoming_blocks.contains(&body_block));
    }

    /// Unreachable blocks do not contribute memory accesses.
    #[test]
    fn test_memory_ssa_ignores_unreachable_blocks() {
        let test = TestProgram::new(
            r#"
function test(): int32 {
entry:
    v0: int32 = 0
    return v0

b1:
    v1: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v2: int32 = 1
    store v1, v2
    return v2
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analysis_cache();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let memory_ssa = memory_ssa.as_ref();

        // locate store in unreachable block
        let unreachable_block = function.block(1);
        let block = test.tree.get(unreachable_block);
        let store_inst = block.instructions[2];

        assert!(
            memory_ssa.instruction_accesses(store_inst).is_none(),
            "unreachable store should not be tracked"
        );
    }
}
