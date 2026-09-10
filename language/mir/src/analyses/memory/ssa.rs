use std::collections::VecDeque;

use crate as mir;
use destack_core::{FxIndexMap, FxIndexSet};

use crate::{
    AliasTable, Analysis, ControlTable, DominatorTable, MemoryAccessEffect, MemoryEffects,
    MemoryRegion, Mutation, NodeTable,
};

/// Memory versions for one function.
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
    /// Incoming memory keyed by predecessor block; none denotes initial function entry.
    pub incoming: Vec<(Option<mir::LocalNodeId<mir::Block>>, MemoryAccessId)>,
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
    fn clobbers_query(&self, query: &MemoryAccessQuery, alias: &AliasTable) -> bool {
        // treat barriers as clobbering all memory
        if self.effect.is_barrier {
            return true;
        }

        // ignore non-writing accesses
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

        // resolve addressed queries
        let Some(location) = query.region.location() else {
            return self.effect.writes;
        };

        // ignore local defs for addressed queries
        if let MemoryRegion::Local(_) = self.effect.region {
            return false;
        }

        // compare memory locations when available
        if let MemoryRegion::Address {
            location: definition,
            ..
        } = &self.effect.region
        {
            return alias.alias(definition, location).may_alias();
        }

        // treat imprecise definitions as clobbering compatible locations
        matches!(self.effect.region, MemoryRegion::Any { .. })
    }

    /// Return whether this definition clobbers a memory region.
    fn clobbers_region(&self, region: &MemoryRegion, alias: &AliasTable) -> bool {
        // treat barriers as clobbering all memory
        if self.effect.is_barrier {
            return true;
        }

        // ignore non-writing accesses
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

        // check address-based aliasing
        let Some(location) = region.location() else {
            return self.effect.writes;
        };

        if let MemoryRegion::Local(_) = self.effect.region {
            return false;
        }

        // compare memory locations when available
        if let MemoryRegion::Address {
            location: definition,
            ..
        } = &self.effect.region
        {
            return alias.alias(definition, location).may_alias();
        }

        // treat imprecise definitions as clobbering compatible locations
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

impl MemorySSA {
    /// Build MemorySSA for a function.
    pub fn analyse(
        function: &mir::Function,
        cfg: &ControlTable,
        dominator: &DominatorTable,
        effects: &MemoryEffects,
        tree: &mir::Tree,
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
        let collected = MemoryAccessCollection::collect(function, effects, cfg, tree);

        // compute dominance frontier for memory defs
        let dominance_frontier = dominator.frontiers();

        // insert memory phis for join points
        let phi_blocks = Self::phi_blocks(
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
            // merge initial memory when a backedge returns to the entry
            let incoming = if *block == entry {
                vec![(None, live_on_entry)]
            } else {
                Vec::new()
            };

            // index the new phi by its block
            let phi_id = MemoryAccessId::from_index(accesses.len());
            accesses.push(MemoryNode::Phi(MemoryPhi {
                block: *block,
                incoming,
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

        // assemble the memory table
        let mut ssa = Self {
            accesses,
            block_phis,
            instruction_access,
            terminator_access,
            block_accesses,
            live_on_entry,
        };

        // rename memory accesses
        let mut renamer = MemoryRenamer::new(tree, dominator, entry, &collected.reachable_blocks);
        renamer.rename(&mut ssa);

        // return the memory table
        ssa
    }

    /// Compute the blocks that require memory phi nodes.
    fn phi_blocks(
        definitions: &FxIndexSet<mir::LocalNodeId<mir::Block>>,
        frontiers: &FxIndexMap<
            mir::LocalNodeId<mir::Block>,
            FxIndexSet<mir::LocalNodeId<mir::Block>>,
        >,
        reachable: &FxIndexSet<mir::LocalNodeId<mir::Block>>,
    ) -> Vec<mir::LocalNodeId<mir::Block>> {
        let mut worklist: VecDeque<_> = definitions.iter().copied().collect();
        let mut queued: FxIndexSet<_> = definitions.iter().copied().collect();
        let mut blocks = FxIndexSet::default();

        // close definition blocks over their iterated dominance frontier
        while let Some(block) = worklist.pop_front() {
            queued.swap_remove(&block);

            let Some(frontier) = frontiers.get(&block) else {
                continue;
            };

            for &candidate in frontier {
                if !reachable.contains(&candidate) {
                    continue;
                }

                if blocks.insert(candidate)
                    && !definitions.contains(&candidate)
                    && queued.insert(candidate)
                {
                    worklist.push_back(candidate);
                }
            }
        }

        let mut blocks = blocks.into_iter().collect::<Vec<_>>();
        blocks.sort();

        blocks
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

    /// Iterate memory effects for one block terminator.
    pub fn terminator_effects(
        &self,
        block: mir::LocalNodeId<mir::Block>,
    ) -> impl Iterator<Item = &MemoryAccessEffect> {
        self.terminator_accesses(block)
            .into_iter()
            .flatten()
            .filter_map(|access| self.access(*access).effect())
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
    pub fn clobbering_use(&self, use_access: MemoryAccessId, alias: &AliasTable) -> MemoryAccessId {
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
        let mut cache = FxIndexMap::default();
        let mut visiting = FxIndexSet::default();
        self.clobbering_access(defining_access, &query, alias, &mut cache, &mut visiting)
    }

    /// Compute the clobbering access for a memory def.
    pub fn clobbering_def(&self, def_access: MemoryAccessId, alias: &AliasTable) -> MemoryAccessId {
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
        let mut cache = FxIndexMap::default();
        let mut visiting = FxIndexSet::default();
        self.clobbering_access(defining_access, &query, alias, &mut cache, &mut visiting)
    }

    /// Compute the clobbering access for a read at the given region.
    pub fn clobbering_read(
        &self,
        access_id: MemoryAccessId,
        region: &MemoryRegion,
        alias: &AliasTable,
    ) -> MemoryAccessId {
        // resolve the defining access for this read
        let defining_access = self
            .defining_access(access_id)
            .unwrap_or(self.live_on_entry);

        // build the query for this read
        let query = MemoryAccessQuery::from_region(region);

        // compute the clobbering access
        let mut cache = FxIndexMap::default();
        let mut visiting = FxIndexSet::default();
        self.clobbering_access(defining_access, &query, alias, &mut cache, &mut visiting)
    }

    /// Return whether a definition clobbers one region.
    pub fn def_clobbers_region(
        &self,
        def_access: MemoryAccessId,
        region: &MemoryRegion,
        alias: &AliasTable,
    ) -> bool {
        // only defs can clobber spaces
        let MemoryNode::Def(def_access) = self.access(def_access) else {
            return false;
        };

        def_access.clobbers_region(region, alias)
    }

    /// Return whether a definition clobbers another access.
    pub fn def_clobbers_access(
        &self,
        def_access: MemoryAccessId,
        target_access: MemoryAccessId,
        alias: &AliasTable,
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
        alias: &AliasTable,
        cache: &mut FxIndexMap<(MemoryAccessId, MemoryAccessQuery), MemoryAccessId>,
        visiting: &mut FxIndexSet<MemoryAccessId>,
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
        visiting.swap_remove(&access_id);
        cache.insert((access_id, query.clone()), result);
        result
    }
}

impl Analysis for MemorySSA {
    const INVALIDATED_BY: Mutation =
        MemoryEffects::INVALIDATED_BY.union(DominatorTable::INVALIDATED_BY);
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
    def_blocks: FxIndexSet<mir::LocalNodeId<mir::Block>>,
    /// Blocks reachable from entry.
    reachable_blocks: Vec<mir::LocalNodeId<mir::Block>>,
    /// Reachable block set.
    reachable: FxIndexSet<mir::LocalNodeId<mir::Block>>,
}

impl MemoryAccessCollection {
    /// Collect memory accesses for all reachable blocks.
    fn collect(
        function: &mir::Function,
        effects: &MemoryEffects,
        control: &ControlTable,
        tree: &mir::Tree,
    ) -> Self {
        // reuse the cached reachable blocks
        let reachable_blocks: Vec<_> = control.reachable_blocks().collect();
        let reachable: FxIndexSet<_> = reachable_blocks.iter().copied().collect();

        // collect memory accesses per block
        let mut block_accesses = NodeTable::from_nodes(function.blocks(), Vec::new);
        let mut def_blocks = FxIndexSet::default();

        // scan reachable blocks
        for &block_id in &reachable_blocks {
            let block = tree.get(block_id);
            let mut accesses = Vec::new();

            // scan instructions for memory effects
            for &instruction_id in &block.instructions {
                for effect in effects.instruction_effects(instruction_id).cloned() {
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
            for effect in effects.terminator_effects(block_id).cloned() {
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

        Self {
            block_accesses,
            def_blocks,
            reachable_blocks,
            reachable,
        }
    }
}

/// MemorySSA renamer for def use chains.
struct MemoryRenamer<'a> {
    /// MIR tree.
    tree: &'a mir::Tree,
    /// Entry block id.
    entry: mir::LocalNodeId<mir::Block>,
    /// Reachable block set.
    reachable: FxIndexSet<mir::LocalNodeId<mir::Block>>,
    /// Dominator tree children.
    children: NodeTable<mir::Block, Vec<mir::LocalNodeId<mir::Block>>>,
}

impl<'a> MemoryRenamer<'a> {
    /// Create a new renamer.
    fn new(
        tree: &'a mir::Tree,
        dominator: &'a DominatorTable,
        entry: mir::LocalNodeId<mir::Block>,
        reachable_blocks: &[mir::LocalNodeId<mir::Block>],
    ) -> Self {
        // build reachable set
        let reachable: FxIndexSet<_> = reachable_blocks.iter().copied().collect();

        // build dominator tree children map
        let mut children = NodeTable::from_nodes(reachable_blocks, Vec::new);
        for &block in reachable_blocks {
            if let Some(idom) = dominator.immediate_dominator(block) {
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
        // initialize the stack with live definitions
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
        let node = self.tree.get(block);
        let terminator = self.tree.get(node.terminator);
        for successor in terminator.successors(self.tree) {
            if let Some(phi_id) = *ssa.block_phis.get(successor)
                && let Some(MemoryNode::Phi(phi)) = ssa.accesses.get_mut(phi_id.index())
            {
                let incoming = *stack.last().expect("missing memory definition");
                phi.incoming.push((Some(block), incoming));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestProgram;
    use crate::{MemoryAddress, MemoryLocation};

    /// Extract the effect payload for a memory access.
    fn access_effect(memory: &MemorySSA, access_id: MemoryAccessId) -> MemoryAccessEffect {
        // unwrap use or def payloads
        match memory.access(access_id) {
            MemoryNode::Use(use_access) => use_access.effect.clone(),
            MemoryNode::Def(def_access) => def_access.effect.clone(),
            MemoryNode::Phi(_) | MemoryNode::LiveOnEntry => {
                panic!("expected effectful access")
            }
        }
    }

    /// Extract the address value from a location when available.
    fn address_from_region(location: &MemoryRegion) -> Option<mir::Value> {
        // unwrap address-backed regions
        match location {
            MemoryRegion::Address { location, .. } => Some(location.address.value()),
            _ => None,
        }
    }

    /// Extract the byte size from a location when available.
    fn size_from_region(location: &MemoryRegion) -> Option<u64> {
        // unwrap address-backed regions
        match location {
            MemoryRegion::Address { location, .. } => location.size,
            _ => None,
        }
    }

    /// Collect the memory accesses for an instruction.
    fn instruction_accesses(
        memory: &MemorySSA,
        instruction: mir::LocalNodeId<mir::Instruction>,
    ) -> Vec<MemoryAccessId> {
        // clone the access list when present
        memory
            .instruction_accesses(instruction)
            .map(|accesses| accesses.to_vec())
            .unwrap_or_default()
    }

    /// Collect the memory accesses for a terminator.
    fn terminator_accesses(
        memory: &MemorySSA,
        block: mir::LocalNodeId<mir::Block>,
    ) -> Vec<MemoryAccessId> {
        // clone the access list when present
        memory
            .terminator_accesses(block)
            .map(|accesses| accesses.to_vec())
            .unwrap_or_default()
    }

    /// MemorySSA links uses to the latest defining access in a straight line.
    #[test]
    fn test_memory_linear_def_use() {
        let test = TestProgram::new(
            r#"
function test<'a>(v0: ref<int32, borrowed, 'a, mutable, local>): int32 {
entry(v0: ref<int32, borrowed, 'a, mutable, local>):
    v1: int32 = 1
    store v0, v1
    v2: int32 = load v0
    return v2
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let memory = analyses.memory(function, &test.tree, &test.accesses, &test.effects);
        let memory = memory.as_ref();

        // find memory accesses
        let block = test.tree.get(function.block(0));
        let store_id = block.instructions[1];
        let load_id = block.instructions[2];

        let store_access = memory
            .instruction_access(store_id)
            .expect("missing store access");
        let load_access = memory
            .instruction_access(load_id)
            .expect("missing load access");

        // load should depend on store
        assert_eq!(memory.defining_access(load_access), Some(store_access));

        // store should depend on live on entry
        assert_eq!(
            memory.defining_access(store_access),
            Some(memory.live_on_entry())
        );
    }

    /// MemorySSA inserts phis at join points with multiple incoming defs.
    #[test]
    fn test_memory_phi_at_join() {
        let test = TestProgram::new(
            r#"
function test<'a>(v0: ref<int32, borrowed, 'a, mutable, local>, v1: boolean): int32 {
entry(v0: ref<int32, borrowed, 'a, mutable, local>, v1: boolean):
    branch v1 => b1 | b2

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
        let mut analyses = test.function_analyses();
        let memory = analyses.memory(function, &test.tree, &test.accesses, &test.effects);
        let memory = memory.as_ref();

        // fetch join block phi
        let join_block = function.block(3);
        let phi_id = memory
            .block_phi(join_block)
            .expect("missing memory phi at join");

        // load should depend on the phi
        let load_inst = test.tree.get(join_block).instructions[0];
        let load_access = memory
            .instruction_access(load_inst)
            .expect("missing load access");
        assert_eq!(memory.defining_access(load_access), Some(phi_id));

        // phi should have incoming for both predecessors
        let MemoryNode::Phi(phi) = memory.access(phi_id) else {
            panic!("expected memory phi");
        };

        assert_eq!(phi.incoming.len(), 2);
    }

    /// MemorySSA uses alias analysis to skip non aliasing defs.
    #[test]
    fn test_memory_clobber_skips_disjoint_def() {
        let test = TestProgram::new(
            r#"
function test(): int32 {
    local l0: int32
    local l1: int32

entry:
    v0: ref<int32, borrowed, 'frame, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, 'frame, mutable, frame> = local.address l1
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
        let mut analyses = test.function_analyses();
        let memory = analyses.memory(function, &test.tree, &test.accesses, &test.effects);
        let alias = analyses.alias(function, &test.tree);

        // locate accesses
        let block = test.tree.get(function.block(0));
        let store_v0 = block.instructions[3];
        let load_v0 = block.instructions[6];

        let store_access = memory
            .instruction_access(store_v0)
            .expect("missing store access");
        let load_access = memory
            .instruction_access(load_v0)
            .expect("missing load access");

        // clobbering access should be the store to v0
        let clobber = memory.clobbering_use(load_access, &alias);
        assert_eq!(clobber, store_access);
    }

    /// MemorySSA prefers explicit access records.
    #[test]
    fn test_memory_entries_overrides_instruction() {
        // input test
        let mut test = TestProgram::new(
            r#"
function test(): int32 {
    local l0: int32
    local l1: int32

entry:
    v0: ref<int32, borrowed, 'frame, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, 'frame, mutable, frame> = local.address l1
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

        // attach an access entry that retargets the load to v1
        test.insert_address_location(
            load_v0,
            mir::MemoryOperation::Read,
            mir::Value::new(1),
            Some(4),
        );

        // build analyses
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let memory = analyses.memory(function, &test.tree, &test.accesses, &test.effects);
        let alias = analyses.alias(function, &test.tree);

        // locate memory accesses
        let load_access = memory
            .instruction_access(load_v0)
            .expect("missing load access");
        let store_access = memory
            .instruction_access(store_v1)
            .expect("missing store access");

        // clobber should follow the entry's target
        let clobber = memory.clobbering_use(load_access, &alias);
        assert_eq!(clobber, store_access);
    }

    /// Local accesses are tracked independently of address memory.
    #[test]
    fn test_memory_local_access() {
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
        let mut analyses = test.function_analyses();
        let memory = analyses.memory(function, &test.tree, &test.accesses, &test.effects);

        // locate local access
        let block = test.tree.get(function.block(0));
        let store_inst = block.instructions[1];
        let load_inst = block.instructions[2];

        let store_access = memory
            .instruction_access(store_inst)
            .expect("missing local store access");
        let load_access = memory
            .instruction_access(load_inst)
            .expect("missing local load access");

        // local load should see the local set
        assert_eq!(memory.defining_access(load_access), Some(store_access));
    }

    /// Local effects touch addresses built from local addresses.
    #[test]
    fn test_memory_local_effect_clobbers_local_address() {
        let test = TestProgram::new(
            r#"
function test(): int32 {
    local l0: int32

entry:
    v0: int32 = 7
    local.set l0, v0
    v1: ref<int32, borrowed, 'frame, mutable, frame> = local.address l0
    v2: int32 = load v1
    return v2
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let memory = analyses.memory(function, &test.tree, &test.accesses, &test.effects);
        let alias = analyses.alias(function, &test.tree);

        // locate the local write and local-address load
        let instructions = test.entry_instructions(function_id);
        let local_set = instructions[1];
        let location = MemoryLocation::from_address(mir::Value::new(1));

        // local write must clobber the equivalent local-address address
        let is_clobbered = memory
            .instruction_effects(local_set)
            .any(|effect| effect.clobbers_location(&location, &alias));
        assert!(is_clobbered);
    }

    /// Free effects are modeled as read and write effects over any memory.
    #[test]
    fn test_memory_free_effect_any() {
        let test = TestProgram::new(
            r#"
function test(v0: ref<int32, unique, mutable, local>): int32 {
entry(v0: ref<int32, unique, mutable, local>):
    release v0
    v1: int32 = 0
    return v1
}
"#,
        );

        let function_id = test.first_function_id();
        let instructions = test.entry_instructions(function_id);
        let free_inst = instructions[0];

        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let memory = analyses.memory(function, &test.tree, &test.accesses, &test.effects);
        let memory = memory.as_ref();

        let free_access = memory
            .instruction_access(free_inst)
            .expect("missing free access");
        let effect = access_effect(memory, free_access);

        assert!(effect.reads);
        assert!(effect.writes);
        assert!(matches!(effect.region, MemoryRegion::Any { .. }));
    }

    /// Allocation effects are modeled as read and write effects over any memory.
    #[test]
    fn test_memory_alloc_effect_any() {
        let test = TestProgram::new(
            r#"
type Point {
    int32;
}

function test(): ref<Point, managed, mutable, local> {
entry:
    v0: ref<Point, managed, mutable, local> = new.zeroed Point
    return v0
}
"#,
        );

        let function_id = test.first_function_id();
        let instructions = test.entry_instructions(function_id);
        let alloc_inst = instructions[0];

        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let memory = analyses.memory(function, &test.tree, &test.accesses, &test.effects);
        let memory = memory.as_ref();

        let alloc_access = memory
            .instruction_access(alloc_inst)
            .expect("missing alloc access");
        let effect = access_effect(memory, alloc_access);

        assert!(effect.reads);
        assert!(effect.writes);
        assert!(matches!(effect.region, MemoryRegion::Any { .. }));
    }

    /// Memcpy produces a read followed by a write access for its operands.
    #[test]
    fn test_memory_memcpy_read_write_effects() {
        let test = TestProgram::new(
            r#"
function test<'a, 'b>(v0: ref<int32, borrowed, 'a, mutable, local>, v1: ref<int32, borrowed, 'b, mutable, local>): int32 {
entry(v0: ref<int32, borrowed, 'a, mutable, local>, v1: ref<int32, borrowed, 'b, mutable, local>):
    v2: int64 = 4
    intrinsic.memory.raw.copyBytes(v0, v1, v2)
    v3: int32 = load v0
    return v3
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let memory = analyses.memory(function, &test.tree, &test.accesses, &test.effects);
        let memory = memory.as_ref();

        let memcpy_inst = test.first_intrinsic_in_entry(function_id, mir::Intrinsic::Memcpy);
        let accesses = instruction_accesses(memory, memcpy_inst);

        // require one recorded read and one recorded write
        assert_eq!(accesses.len(), 2);

        // extract effects in order
        let read_effect = access_effect(memory, accesses[0]);
        let write_effect = access_effect(memory, accesses[1]);

        assert!(read_effect.reads);
        assert!(!read_effect.writes);
        assert!(write_effect.writes);
        assert!(!write_effect.reads);

        let read_address = address_from_region(&read_effect.region).expect("missing read address");
        let write_address =
            address_from_region(&write_effect.region).expect("missing write address");

        assert_eq!(read_address, mir::Value::new(1));
        assert_eq!(write_address, mir::Value::new(0));

        let read_size = size_from_region(&read_effect.region).expect("missing read size");
        let write_size = size_from_region(&write_effect.region).expect("missing write size");

        assert_eq!(read_size, 4);
        assert_eq!(write_size, 4);
    }

    /// Memcmp produces two read accesses for its operands.
    #[test]
    fn test_memory_memcmp_read_effects() {
        let test = TestProgram::new(
            r#"
function test<'a, 'b>(v0: ref<int32, borrowed, 'a, mutable, local>, v1: ref<int32, borrowed, 'b, mutable, local>): int32 {
entry(v0: ref<int32, borrowed, 'a, mutable, local>, v1: ref<int32, borrowed, 'b, mutable, local>):
    v2: int64 = 4
    v3: int32 = intrinsic.memory.raw.compareBytes(v0, v1, v2)
    return v3
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let memory = analyses.memory(function, &test.tree, &test.accesses, &test.effects);
        let memory = memory.as_ref();

        let memcmp_inst = test.first_intrinsic_in_entry(function_id, mir::Intrinsic::Memcmp);
        let accesses = instruction_accesses(memory, memcmp_inst);

        // require two recorded reads
        assert_eq!(accesses.len(), 2);

        for access_id in accesses {
            let effect = access_effect(memory, access_id);
            assert!(effect.reads);
            assert!(!effect.writes);
        }
    }

    /// Volatile accesses are marked as volatile effects.
    #[test]
    fn test_memory_volatile_marks_effects() {
        let mut test = TestProgram::new(
            r#"
function test<'a>(v0: ref<int32, borrowed, 'a, mutable, local>): int32 {
entry(v0: ref<int32, borrowed, 'a, mutable, local>):
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
        let volatile_load = block.instructions[0];
        let volatile_store = block.instructions[1];

        // attach volatile memory access entries
        test.insert_address_location_with_options(
            volatile_load,
            mir::MemoryOperation::Read,
            mir::Value::new(0),
            Some(4),
            true,
            None,
        );
        test.insert_address_location_with_options(
            volatile_store,
            mir::MemoryOperation::Write,
            mir::Value::new(0),
            Some(4),
            true,
            None,
        );

        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let memory = analyses.memory(function, &test.tree, &test.accesses, &test.effects);
        let memory = memory.as_ref();

        // collect volatile effects
        let load_access = memory
            .instruction_access(volatile_load)
            .expect("missing volatile load access");
        let store_access = memory
            .instruction_access(volatile_store)
            .expect("missing volatile store access");

        let load_effect = access_effect(memory, load_access);
        let store_effect = access_effect(memory, store_access);

        assert!(load_effect.is_volatile);
        assert!(store_effect.is_volatile);
    }

    /// Atomic accesses are treated as volatile effects.
    #[test]
    fn test_memory_atomic_marks_effects() {
        let test = TestProgram::new(
            r#"
function test<'a>(v0: ref<atomic<int32>, borrowed, 'a, mutable, local>): int32 {
entry(v0: ref<atomic<int32>, borrowed, 'a, mutable, local>):
    v1: int32 = atomic.load v0, acquire, scope(device)
    atomic.store v0, v1, release, scope(device)
    return v1
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let memory = analyses.memory(function, &test.tree, &test.accesses, &test.effects);
        let memory = memory.as_ref();

        let block = test.tree.get(function.block(0));
        let atomic_load = block.instructions[0];
        let atomic_store = block.instructions[1];

        let load_access = memory
            .instruction_access(atomic_load)
            .expect("missing atomic load access");
        let store_access = memory
            .instruction_access(atomic_store)
            .expect("missing atomic store access");

        let load_effect = access_effect(memory, load_access);
        let store_effect = access_effect(memory, store_access);

        assert!(load_effect.is_volatile);
        assert!(store_effect.is_volatile);
    }

    /// Atomic fence produces a barrier access.
    #[test]
    fn test_memory_atomic_fence_barrier() {
        let test = TestProgram::new(
            r#"
function test(): int32 {
entry:
    atomic.fence sequentiallyConsistent, scope(device), storage(shared)
    v0: int32 = 0
    return v0
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let memory = analyses.memory(function, &test.tree, &test.accesses, &test.effects);
        let memory = memory.as_ref();

        // locate fence instruction
        let block = test.tree.get(function.block(0));
        let fence_inst = block.instructions[0];
        let fence_access = memory
            .instruction_access(fence_inst)
            .expect("missing fence access");

        let fence_effect = access_effect(memory, fence_access);
        assert!(fence_effect.is_barrier);
        assert!(matches!(fence_effect.region, MemoryRegion::Any { .. }));
    }

    /// Calls without precise tables are modeled as read and write effects over any memory.
    #[test]
    fn test_memory_call_is_any_def() {
        let test = TestProgram::new(
            r#"
external function imported<'a>(ref<int32, borrowed, 'a, mutable, local>): void

function test<'a>(v0: ref<int32, borrowed, 'a, mutable, local>): int32 {
entry(v0: ref<int32, borrowed, 'a, mutable, local>):
    call imported(v0): <'a>(ref<int32, borrowed, 'a, mutable, local>) => void
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
        let mut analyses = test.function_analyses();
        let memory = analyses.memory(function, &test.tree, &test.accesses, &test.effects);
        let memory = memory.as_ref();

        // locate call instruction
        let block = test.tree.get(function.block(0));
        let call_inst = block.instructions[0];
        let call_access = memory
            .instruction_access(call_inst)
            .expect("missing call access");

        let call_effect = access_effect(memory, call_access);
        assert!(call_effect.reads);
        assert!(call_effect.writes);
        assert!(matches!(call_effect.region, MemoryRegion::Any { .. }));
    }

    /// Continuation loads see memory effects from invokes.
    #[test]
    fn test_memory_invoke_clobbers_continuation() {
        let test = TestProgram::new(
            r#"
external function imported<'a>(ref<int32, borrowed, 'a, mutable, local>): void

function test<'a>(v0: ref<int32, borrowed, 'a, mutable, local>): int32 {
entry(v0: ref<int32, borrowed, 'a, mutable, local>):
    invoke imported(v0): <'a>(ref<int32, borrowed, 'a, mutable, local>) => void => b1 | b2

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
        let mut analyses = test.function_analyses();
        let memory = analyses.memory(function, &test.tree, &test.accesses, &test.effects);
        let memory = memory.as_ref();

        // locate the invoke and continuation load
        let entry = function.block(0);
        let continuation = function.block(1);
        let load = test.tree.get(continuation).instructions[0];

        let call_accesses = terminator_accesses(memory, entry);
        let load_access = memory
            .instruction_access(load)
            .expect("missing load access");

        // require the invoke to define the continuation memory state
        assert_eq!(call_accesses.len(), 1);
        assert_eq!(memory.defining_access(load_access), Some(call_accesses[0]));
    }

    /// A call declaring no memory effects suppresses memory accesses.
    #[test]
    fn test_memory_skips_no_memory_call() {
        let mut test = TestProgram::new(
            r#"
external function imported<'a>(ref<int32, borrowed, 'a, mutable, local>): void

function test<'a>(v0: ref<int32, borrowed, 'a, mutable, local>): int32 {
entry(v0: ref<int32, borrowed, 'a, mutable, local>):
    call imported(v0): <'a>(ref<int32, borrowed, 'a, mutable, local>) => void
    v1: int32 = 0
    return v1
}
"#,
        );

        let function_id = test.entry_function_id();
        let (call_inst, _callee) = test.first_call_in_entry(function_id);
        let callsite = mir::Point::Instruction(call_inst);
        test.effects.upsert_call(callsite).memory = mir::MemoryEffect::none();

        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let memory = analyses.memory(function, &test.tree, &test.accesses, &test.effects);

        assert!(
            memory.instruction_accesses(call_inst).is_none(),
            "no memory calls should not create memory accesses"
        );
    }

    /// Memory access entries override default instruction effects.
    #[test]
    fn test_memory_access_entries_override() {
        // build the test program
        let mut test = TestProgram::new(
            r#"
external function imported<'a, 'b>(ref<int32, borrowed, 'a, mutable, local>, ref<int32, borrowed, 'b, mutable, local>): void

function test<'a, 'b>(v0: ref<int32, borrowed, 'a, mutable, local>, v1: ref<int32, borrowed, 'b, mutable, local>): int32 {
entry(v0: ref<int32, borrowed, 'a, mutable, local>, v1: ref<int32, borrowed, 'b, mutable, local>):
    call imported(v0, v1): <'a, 'b>(ref<int32, borrowed, 'a, mutable, local>, ref<int32, borrowed, 'b, mutable, local>) => void
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
            mir::MemoryTarget::Address(param_values[0]),
            Some(4),
            None,
        );
        let write_access = mir::MemoryAccess::plain(
            mir::MemoryOperation::Write,
            mir::MemoryTarget::Address(param_values[1]),
            Some(4),
            None,
        );

        // attach memory access entries to the call
        test.insert_accesses(call_inst, vec![read_access, write_access]);

        // build analyses
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let memory = analyses.memory(function, &test.tree, &test.accesses, &test.effects);
        let memory = memory.as_ref();

        // locate access effects for the call
        let accesses = instruction_accesses(memory, call_inst);
        assert_eq!(accesses.len(), 2);

        // verify read and write effects
        let read_effect = access_effect(memory, accesses[0]);
        let write_effect = access_effect(memory, accesses[1]);

        assert!(read_effect.reads);
        assert!(!read_effect.writes);
        assert!(write_effect.writes);
        assert!(!write_effect.reads);

        // verify addresses and sizes
        let read_address = address_from_region(&read_effect.region).expect("missing read address");
        let write_address =
            address_from_region(&write_effect.region).expect("missing write address");
        assert_eq!(read_address, function.parameters[0].value);
        assert_eq!(write_address, function.parameters[1].value);

        let read_size = size_from_region(&read_effect.region).expect("missing read size");
        let write_size = size_from_region(&write_effect.region).expect("missing write size");
        assert_eq!(read_size, 4);
        assert_eq!(write_size, 4);
    }

    /// Dynamic field reads retain their dispatch projections and exact widths.
    #[test]
    fn test_memory_tracks_dynamic_fields() {
        let test = TestProgram::new(
            r#"
type Writer {
    first: int32;
    second: int32;
}

function test(v0: dynamic<Writer, managed, readonly, local>): int32 {
entry(v0: dynamic<Writer, managed, readonly, local>):
    v1: int32 = dynamic.read v0, 0
    v2: int32 = dynamic.read v0, 1
    v3: int32 = add v1, v2
    return v3
}
"#,
        );

        let function_id = test.entry_function_id();
        let function = test.tree.get(function_id);
        let block = test.tree.get(function.block(0));
        let mut analyses = test.function_analyses();
        let memory = analyses.memory(function, &test.tree, &test.accesses, &test.effects);

        // resolve both field reads
        let first = memory
            .instruction_access(block.instructions[0])
            .map(|access| access_effect(&memory, access))
            .expect("missing first dynamic read");
        let second = memory
            .instruction_access(block.instructions[1])
            .map(|access| access_effect(&memory, access))
            .expect("missing second dynamic read");

        // retain the payload root while distinguishing the dispatch slots
        let MemoryRegion::Address {
            location: first_location,
            ..
        } = first.region
        else {
            panic!("dynamic read should address its payload");
        };
        let MemoryRegion::Address {
            location: second_location,
            ..
        } = second.region
        else {
            panic!("dynamic read should address its payload");
        };
        assert_eq!(
            first_location.address,
            MemoryAddress::Dynamic {
                value: function.parameters[0].value,
                slot: mir::DispatchSlot(0),
            }
        );
        assert_eq!(
            second_location.address,
            MemoryAddress::Dynamic {
                value: function.parameters[0].value,
                slot: mir::DispatchSlot(1),
            }
        );
        assert_eq!(first_location.size, Some(4));
        assert_eq!(second_location.size, Some(4));
    }

    /// Loop headers get memory phis when defs flow around the backedge.
    #[test]
    fn test_memory_loop_phi_in_header() {
        let test = TestProgram::new(
            r#"
function test<'a>(v0: ref<int32, borrowed, 'a, mutable, local>, v1: int32): int32 {
entry(v0: ref<int32, borrowed, 'a, mutable, local>, v1: int32):
    v2: int32 = 0
    store v0, v2
    jump b1(v2)

b1(v3: int32):
    v4: boolean = lt v3, v1
    branch v4 => b2 | b3

b2:
    v5: int32 = 1
    store v0, v5
    v6: int32 = add v3, v5
    jump b1(v6)

b3:
    v7: int32 = load v0
    return v7
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let memory = analyses.memory(function, &test.tree, &test.accesses, &test.effects);
        let memory = memory.as_ref();

        // find phi for loop header
        let header_block = function.block(1);
        let phi_id = memory
            .block_phi(header_block)
            .expect("missing loop header phi");

        let MemoryNode::Phi(phi) = memory.access(phi_id) else {
            panic!("expected memory phi");
        };

        let incoming_blocks: FxIndexSet<_> = phi.incoming.iter().map(|(block, _)| *block).collect();
        let body_block = function.block(2);

        assert!(incoming_blocks.contains(&Some(function.block(0))));
        assert!(incoming_blocks.contains(&Some(body_block)));
    }

    /// Memory analysis skips the accesses unreachable blocks write.
    #[test]
    fn test_memory_ignores_unreachable_blocks() {
        let test = TestProgram::new(
            r#"
function test(): int32 {
    local l0: int32

entry:
    v0: int32 = 0
    return v0

b1:
    v1: ref<int32, borrowed, 'frame, mutable, frame> = local.address l0
    v2: int32 = 1
    store v1, v2
    return v2
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let memory = analyses.memory(function, &test.tree, &test.accesses, &test.effects);
        let memory = memory.as_ref();

        // locate store in unreachable block
        let unreachable_block = function.block(1);
        let block = test.tree.get(unreachable_block);
        let store_inst = block.instructions[2];

        assert!(
            memory.instruction_accesses(store_inst).is_none(),
            "unreachable store should not be tracked"
        );
    }

    /// Merge initial memory with backedge stores when the entry is a loop header.
    #[test]
    fn test_merge_memory_at_entry_backedge() {
        let test = TestProgram::new(
            r#"
function test<'a>(v0: ref<int32, borrowed, 'a, mutable, local>, v1: int32, v2: boolean): int32 {
entry(v0: ref<int32, borrowed, 'a, mutable, local>, v1: int32, v2: boolean):
    v3: int32 = load v0
    store v0, v1
    branch v2 => entry(v0, v1, v2) | exit

exit:
    return v3
}
"#,
        );
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let entry = function.block(0);
        let instructions = &test.tree.get(entry).instructions;
        let mut analyses = test.function_analyses();
        let memory = analyses.memory(function, &test.tree, &test.accesses, &test.effects);
        let alias = analyses.alias(function, &test.tree);
        let load = memory
            .instruction_access(instructions[0])
            .expect("load access");
        let store = memory
            .instruction_access(instructions[1])
            .expect("store access");
        let phi = memory.block_phi(entry).expect("entry merge");

        // distinguish the first invocation from subsequent loop iterations
        let MemoryNode::Phi(merge) = memory.access(phi) else {
            panic!("expected entry phi")
        };
        assert_eq!(
            merge.incoming,
            vec![(None, memory.live_on_entry()), (Some(entry), store)]
        );
        assert_eq!(memory.defining_access(load), Some(phi));
        assert_eq!(memory.defining_access(store), Some(phi));
        assert_eq!(memory.clobbering_use(load, &alias), phi);
    }
}
