use std::collections::{HashMap, HashSet, VecDeque};

use crate as mir;
use smallvec::SmallVec;

use crate::{
    AliasAnalysis, Analysis, AnalysisId, ControlFlowGraph, DominatorTree, FunctionAnalyses,
    FunctionAnalysis, MemoryLocation, NodeTable, TargetLayout, TypeKey, ValueDefinitions,
    ValueTypeMap, collect_reachable_blocks, compute_dominance_frontiers,
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

/// Memory target touched by one effect.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MemoryEffectTarget {
    /// Reference based memory access.
    Reference {
        /// The reference access payload.
        location: MemoryLocation,
        /// The memory spaces the reference may touch.
        spaces: mir::SpaceSet,
    },
    /// Local slot access.
    Local(mir::LocalNodeId<mir::Local>),
    /// Imprecise access to a set of memory spaces.
    Any {
        /// The memory spaces the effect may touch.
        spaces: mir::SpaceSet,
    },
}

impl MemoryEffectTarget {
    /// Create an imprecise target covering the given spaces.
    fn any(spaces: mir::SpaceSet) -> Self {
        Self::Any { spaces }
    }

    /// Create a reference location with optional access type and inferred size.
    fn from_reference(
        reference: mir::Value,
        access_type: Option<TypeKey>,
        reference_kind: Option<mir::ReferenceKind>,
        reference_space: Option<mir::Space>,
        pointer_width_bits: u16,
    ) -> Self {
        Self::from_reference_with_size(
            reference,
            access_type,
            reference_kind,
            reference_space,
            None,
            pointer_width_bits,
        )
    }

    /// Create a reference location with an explicit size override.
    fn from_reference_with_size(
        reference: mir::Value,
        access_type: Option<TypeKey>,
        reference_kind: Option<mir::ReferenceKind>,
        reference_space: Option<mir::Space>,
        size: Option<u64>,
        pointer_width_bits: u16,
    ) -> Self {
        let inferred_size = size.or_else(|| {
            access_type
                .as_ref()
                .and_then(|access_type| access_type.byte_size(pointer_width_bits))
        });
        MemoryEffectTarget::Reference {
            location: MemoryLocation::new(
                reference,
                inferred_size,
                access_type,
                reference_kind,
                reference_space,
            ),
            spaces: mir::SpaceSet::ANY,
        }
    }

    /// Return a reference location if available.
    fn as_reference(&self) -> Option<&MemoryLocation> {
        // unwrap reference backed spaces
        match self {
            MemoryEffectTarget::Reference { location, .. } => Some(location),
            _ => None,
        }
    }

    /// Return the memory spaces covered by this target.
    pub fn spaces(&self) -> mir::SpaceSet {
        match self {
            MemoryEffectTarget::Reference { spaces, .. } => *spaces,
            MemoryEffectTarget::Local(_) => mir::SpaceSet::FRAME,
            MemoryEffectTarget::Any { spaces } => *spaces,
        }
    }

    /// Set spaces on the target.
    fn set_spaces(&mut self, new_spaces: mir::SpaceSet) {
        match self {
            MemoryEffectTarget::Reference { spaces, .. } | MemoryEffectTarget::Any { spaces } => {
                *spaces = new_spaces;
            }
            MemoryEffectTarget::Local(_) => {}
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
    /// The memory location being accessed.
    pub location: MemoryEffectTarget,
}

/// Query information for clobbering access lookups.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct MemoryAccessQuery {
    /// The memory location being accessed.
    location: MemoryEffectTarget,
}

impl MemoryAccessQuery {
    /// Create a query from a full access effect.
    fn from_effect(effect: &MemoryAccessEffect) -> Self {
        Self {
            location: effect.location.clone(),
        }
    }

    /// Create a query from a location with no alias metadata.
    fn from_location(location: &MemoryEffectTarget) -> Self {
        Self {
            location: location.clone(),
        }
    }
}

impl MemoryAccessEffect {
    /// Return whether this effect is trackable by memory optimizations.
    pub fn is_trackable(&self) -> bool {
        !self.is_volatile
            && !self.is_barrier
            && !matches!(self.location, MemoryEffectTarget::Any { .. })
    }

    /// Return whether this effect describes the same location as another effect.
    pub fn matches_location(&self, alias: &AliasAnalysis, other: &MemoryAccessEffect) -> bool {
        // reject disjoint memory spaces
        if !self.location.spaces().may_alias(other.location.spaces()) {
            return false;
        }

        // compare concrete locations
        match (&self.location, &other.location) {
            (MemoryEffectTarget::Local(local), MemoryEffectTarget::Local(other_local)) => {
                local == other_local
            }
            (
                MemoryEffectTarget::Reference { location, .. },
                MemoryEffectTarget::Reference {
                    location: other_location,
                    ..
                },
            ) => {
                if !location.is_compatible_with(other_location) {
                    return false;
                }

                alias.alias(location, other_location).is_must_alias()
            }
            _ => false,
        }
    }

    /// Return whether this effect may alias another effect.
    pub fn may_alias(&self, alias: &AliasAnalysis, other: &MemoryAccessEffect) -> bool {
        // reject disjoint memory spaces
        if !self.location.spaces().may_alias(other.location.spaces()) {
            return false;
        }

        // compare concrete locations
        match (&self.location, &other.location) {
            (MemoryEffectTarget::Any { .. }, _) | (_, MemoryEffectTarget::Any { .. }) => true,
            (MemoryEffectTarget::Local(local), MemoryEffectTarget::Local(other)) => local == other,
            (
                MemoryEffectTarget::Reference { location, .. },
                MemoryEffectTarget::Reference {
                    location: other_location,
                    ..
                },
            ) => {
                if !location.is_compatible_with(other_location) {
                    return false;
                }

                alias.alias(location, other_location).may_alias()
            }
            _ => false,
        }
    }

    /// Create an access effect for reads.
    fn read(location: MemoryEffectTarget, is_volatile: bool) -> Self {
        Self {
            reads: true,
            writes: false,
            is_volatile,
            is_barrier: false,
            location,
        }
    }

    /// Create an access effect for writes.
    fn write(location: MemoryEffectTarget, is_volatile: bool) -> Self {
        Self {
            reads: false,
            writes: true,
            is_volatile,
            is_barrier: false,
            location,
        }
    }

    /// Create an access effect for read/write.
    fn read_write(location: MemoryEffectTarget, is_volatile: bool) -> Self {
        Self {
            reads: true,
            writes: true,
            is_volatile,
            is_barrier: false,
            location,
        }
    }

    /// Create a barrier access effect.
    fn barrier() -> Self {
        Self {
            reads: false,
            writes: true,
            is_volatile: false,
            is_barrier: true,
            location: MemoryEffectTarget::Any {
                spaces: mir::SpaceSet::ANY,
            },
        }
    }
}

/// MemorySSA access node.
#[derive(Debug, Clone)]
pub enum MemoryAccess {
    /// Pseudo access that dominates all memory operations.
    LiveOnEntry,
    /// Phi node merging memory states.
    Phi(MemoryPhi),
    /// Memory definition (writes memory).
    Def(MemoryDef),
    /// Memory use (reads memory).
    Use(MemoryUse),
}

impl MemoryAccess {
    /// Return the instruction id for this access if available.
    pub fn instruction(&self) -> Option<mir::LocalNodeId<mir::Instruction>> {
        // map instruction sourced accesses
        self.source().and_then(MemoryAccessSource::instruction)
    }

    /// Return the source operation for this access if available.
    pub fn source(&self) -> Option<MemoryAccessSource> {
        match self {
            MemoryAccess::Def(def) => Some(def.source),
            MemoryAccess::Use(use_access) => Some(use_access.source),
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

        // disambiguate by location sets
        if !self
            .effect
            .location
            .spaces()
            .may_alias(query.location.spaces())
        {
            return false;
        }

        // check for local memory
        if let MemoryEffectTarget::Local(local) = &query.location {
            if let MemoryEffectTarget::Local(def_local) = &self.effect.location {
                return def_local == local;
            }

            return false;
        }

        // handle unknown memory spaces
        if matches!(query.location, MemoryEffectTarget::Any { .. }) {
            return self.effect.writes;
        }

        // resolve reference based queries
        let Some(reference_location) = query.location.as_reference() else {
            return self.effect.writes;
        };

        // ignore local defs for reference queries
        if let MemoryEffectTarget::Local(_) = self.effect.location {
            return false;
        }

        // compare reference locations when available
        if let MemoryEffectTarget::Reference {
            location: def_location,
            ..
        } = &self.effect.location
        {
            return alias.alias(def_location, reference_location).may_alias();
        }

        // imprecise defs clobber every compatible reference location
        matches!(self.effect.location, MemoryEffectTarget::Any { .. })
    }

    /// Return whether this definition clobbers a memory target.
    fn clobbers_location(&self, location: &MemoryEffectTarget, alias: &AliasAnalysis) -> bool {
        // treat barriers as clobbering all memory
        if self.effect.is_barrier {
            return true;
        }

        // ignore non writing accesses
        if !self.effect.writes {
            return false;
        }

        // disambiguate by location sets
        if !self.effect.location.spaces().may_alias(location.spaces()) {
            return false;
        }

        // check for local memory
        if let MemoryEffectTarget::Local(local) = location {
            if let MemoryEffectTarget::Local(def_local) = &self.effect.location {
                return def_local == local;
            }

            return false;
        }

        // handle unknown memory spaces
        if matches!(location, MemoryEffectTarget::Any { .. }) {
            return self.effect.writes;
        }

        // check reference based aliasing
        let Some(reference_location) = location.as_reference() else {
            return self.effect.writes;
        };

        if let MemoryEffectTarget::Local(_) = self.effect.location {
            return false;
        }

        // compare reference locations when available
        if let MemoryEffectTarget::Reference {
            location: def_location,
            ..
        } = &self.effect.location
        {
            return alias.alias(def_location, reference_location).may_alias();
        }

        // imprecise defs clobber every compatible reference location
        matches!(self.effect.location, MemoryEffectTarget::Any { .. })
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
    accesses: Vec<MemoryAccess>,
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
        target_layout: TargetLayout,
    ) -> Self {
        // handle imported functions
        let entry = match function.entry {
            Some(entry) => entry,
            None => {
                let live_on_entry = MemoryAccessId::from_index(0);
                return Self {
                    accesses: vec![MemoryAccess::LiveOnEntry],
                    block_phis: NodeTable::new(),
                    instruction_access: NodeTable::new(),
                    terminator_access: NodeTable::new(),
                    block_accesses: NodeTable::new(),
                    live_on_entry,
                };
            }
        };

        // collect memory accesses and definition blocks
        let mut access_collector =
            MemoryAccessCollector::new(function, tree, definitions, target_layout);
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
        accesses.push(MemoryAccess::LiveOnEntry);
        let live_on_entry = MemoryAccessId::from_index(0);

        // create block phis
        let mut block_phis = NodeTable::from_nodes(&function.blocks, || None);
        for block in &phi_blocks {
            let phi_id = MemoryAccessId::from_index(accesses.len());
            accesses.push(MemoryAccess::Phi(MemoryPhi {
                block: *block,
                incoming: Vec::new(),
            }));
            *block_phis.get_mut(*block) = Some(phi_id);
        }

        // create instruction memory accesses
        let instructions = function
            .blocks
            .iter()
            .flat_map(|block| tree.get(*block).instructions.iter().copied())
            .collect::<Vec<_>>();
        let mut instruction_access = NodeTable::from_nodes(&instructions, Vec::new);
        let mut terminator_access = NodeTable::from_nodes(&function.blocks, Vec::new);
        let mut block_accesses = NodeTable::from_nodes(&function.blocks, Vec::new);

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
                        accesses.push(MemoryAccess::Use(MemoryUse {
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
                        accesses.push(MemoryAccess::Def(MemoryDef {
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
    pub fn access(&self, id: MemoryAccessId) -> &MemoryAccess {
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

    /// Return the first memory use access for an instruction.
    pub fn first_use_access(
        &self,
        instruction: mir::LocalNodeId<mir::Instruction>,
    ) -> Option<MemoryAccessId> {
        let accesses = self.instruction_accesses(instruction)?;
        accesses
            .iter()
            .copied()
            .find(|access_id| matches!(self.access(*access_id), MemoryAccess::Use(_)))
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
            MemoryAccess::Def(def) => def.defining_access,
            MemoryAccess::Use(use_access) => use_access.defining_access,
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
        let MemoryAccess::Use(use_access_data) = self.access(use_access) else {
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
        let MemoryAccess::Def(def_access_data) = self.access(def_access) else {
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

    /// Compute the clobbering access for a read at the given location.
    pub fn clobbering_read(
        &self,
        access_id: MemoryAccessId,
        location: &MemoryEffectTarget,
        alias: &AliasAnalysis,
    ) -> MemoryAccessId {
        // resolve the defining access for this read
        let defining_access = self
            .defining_access(access_id)
            .unwrap_or(self.live_on_entry);

        // build the query for this read
        let query = MemoryAccessQuery::from_location(location);

        // compute the clobbering access
        let mut cache = HashMap::new();
        let mut visiting = HashSet::new();
        self.clobbering_access(defining_access, &query, alias, &mut cache, &mut visiting)
    }

    /// Check if a def access clobbers the given location.
    pub fn def_clobbers_location(
        &self,
        def_access: MemoryAccessId,
        location: &MemoryEffectTarget,
        alias: &AliasAnalysis,
    ) -> bool {
        // only defs can clobber spaces
        let MemoryAccess::Def(def_access) = self.access(def_access) else {
            return false;
        };

        def_access.clobbers_location(location, alias)
    }

    /// Check if a def access clobbers another access.
    pub fn def_clobbers_access(
        &self,
        def_access: MemoryAccessId,
        target_access: MemoryAccessId,
        alias: &AliasAnalysis,
    ) -> bool {
        // only defs can clobber accesses
        let MemoryAccess::Def(def_access) = self.access(def_access) else {
            return false;
        };

        // fetch the target access effect
        let target_effect = match self.access(target_access) {
            MemoryAccess::Use(use_access) => &use_access.effect,
            MemoryAccess::Def(def_access) => &def_access.effect,
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
            MemoryAccess::LiveOnEntry => access_id,

            MemoryAccess::Use(use_access) => {
                let defining_access = use_access
                    .defining_access
                    .expect("memory use missing defining access");
                self.clobbering_access(defining_access, query, alias, cache, visiting)
            }

            MemoryAccess::Def(def_access) => {
                if def_access.clobbers_query(query, alias) {
                    access_id
                } else {
                    let defining_access = def_access
                        .defining_access
                        .expect("memory def missing defining access");
                    self.clobbering_access(defining_access, query, alias, cache, visiting)
                }
            }

            MemoryAccess::Phi(phi) => {
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
    fn compute(function: &mir::Function, tree: &mir::Tree, analyses: &FunctionAnalyses) -> Self {
        // read dependencies
        let cfg = analyses.get::<ControlFlowGraph>(function, tree);
        let domtree = analyses.get::<DominatorTree>(function, tree);
        let definitions = analyses.get::<ValueDefinitions>(function, tree);

        Self::build(
            function,
            tree,
            &cfg,
            &domtree,
            &definitions,
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
    value_types: ValueTypeMap,
    /// Value definitions for reference provenance.
    definitions: ValueDefinitions,
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
        target_layout: TargetLayout,
    ) -> Self {
        // build value type lookup
        let value_types = ValueTypeMap::new(function, tree);

        // build collector state
        Self {
            function,
            tree,
            value_types,
            definitions: definitions.clone(),
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
        let mut block_accesses = NodeTable::from_nodes(&self.function.blocks, Vec::new);
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
        // use explicit metadata when present
        if let Some(effects) = self.metadata_effects(instruction_id) {
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
            | mir::Instruction::Struct { .. }
            | mir::Instruction::Tuple { .. }
            | mir::Instruction::Array { .. }
            | mir::Instruction::FieldGet { .. }
            | mir::Instruction::FieldSet { .. }
            | mir::Instruction::FieldAddr { .. }
            | mir::Instruction::ElementAddr { .. }
            | mir::Instruction::SliceView { .. }
            | mir::Instruction::SliceLength { .. }
            | mir::Instruction::DynamicPayload { .. }
            | mir::Instruction::DynamicType { .. }
            | mir::Instruction::VariantTag { .. }
            | mir::Instruction::VariantPayload { .. }
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
            | mir::Instruction::ProfileValue { .. } => SmallVec::new(),
            mir::Instruction::TensorLoad { view, .. } => {
                let view = *view;

                let access_type = self.reference_access_type(view);
                let reference_kind = self.reference_kind(view);
                let reference_space = self.reference_space(view);
                let mut effect = MemoryAccessEffect::read(
                    MemoryEffectTarget::from_reference(
                        view,
                        access_type,
                        reference_kind,
                        reference_space,
                        self.target_layout.pointer_width_bits,
                    ),
                    false,
                );
                self.apply_reference_location(&mut effect, view);
                Self::single_effect(effect)
            }
            mir::Instruction::TensorStore { view, .. }
            | mir::Instruction::TensorFill { view, .. } => {
                let view = *view;

                let access_type = self.reference_access_type(view);
                let reference_kind = self.reference_kind(view);
                let reference_space = self.reference_space(view);
                let mut effect = MemoryAccessEffect::write(
                    MemoryEffectTarget::from_reference(
                        view,
                        access_type,
                        reference_kind,
                        reference_space,
                        self.target_layout.pointer_width_bits,
                    ),
                    false,
                );
                self.apply_reference_location(&mut effect, view);
                Self::single_effect(effect)
            }
            mir::Instruction::TensorCopy { target, source } => {
                let target = *target;
                let source = *source;

                let mut effects = SmallVec::new();
                let target_access = self.reference_access_type(target);
                let target_kind = self.reference_kind(target);
                let target_space = self.reference_space(target);
                let mut target_effect = MemoryAccessEffect::write(
                    MemoryEffectTarget::from_reference(
                        target,
                        target_access,
                        target_kind,
                        target_space,
                        self.target_layout.pointer_width_bits,
                    ),
                    false,
                );
                self.apply_reference_location(&mut target_effect, target);
                effects.push(target_effect);

                let source_access = self.reference_access_type(source);
                let source_kind = self.reference_kind(source);
                let source_space = self.reference_space(source);
                let mut source_effect = MemoryAccessEffect::read(
                    MemoryEffectTarget::from_reference(
                        source,
                        source_access,
                        source_kind,
                        source_space,
                        self.target_layout.pointer_width_bits,
                    ),
                    false,
                );
                self.apply_reference_location(&mut source_effect, source);
                effects.push(source_effect);

                effects
            }
            mir::Instruction::Load { pointer, .. } => {
                let pointer = *pointer;

                let access_type = self.reference_access_type(pointer);
                let reference_kind = self.reference_kind(pointer);
                let reference_space = self.reference_space(pointer);
                let mut effect = MemoryAccessEffect::read(
                    MemoryEffectTarget::from_reference(
                        pointer,
                        access_type,
                        reference_kind,
                        reference_space,
                        self.target_layout.pointer_width_bits,
                    ),
                    false,
                );
                self.apply_reference_location(&mut effect, pointer);
                Self::single_effect(effect)
            }
            mir::Instruction::Store { pointer, .. } => {
                let pointer = *pointer;

                let access_type = self.reference_access_type(pointer);
                let reference_kind = self.reference_kind(pointer);
                let reference_space = self.reference_space(pointer);
                let mut effect = MemoryAccessEffect::write(
                    MemoryEffectTarget::from_reference(
                        pointer,
                        access_type,
                        reference_kind,
                        reference_space,
                        self.target_layout.pointer_width_bits,
                    ),
                    false,
                );
                self.apply_reference_location(&mut effect, pointer);
                Self::single_effect(effect)
            }
            mir::Instruction::AtomicLoad { pointer, .. } => {
                let pointer = *pointer;

                let access_type = self.reference_access_type(pointer);
                let reference_kind = self.reference_kind(pointer);
                let reference_space = self.reference_space(pointer);
                let mut effect = MemoryAccessEffect::read(
                    MemoryEffectTarget::from_reference(
                        pointer,
                        access_type,
                        reference_kind,
                        reference_space,
                        self.target_layout.pointer_width_bits,
                    ),
                    true,
                );
                self.apply_reference_location(&mut effect, pointer);
                Self::single_effect(effect)
            }
            mir::Instruction::AtomicStore { pointer, .. } => {
                let pointer = *pointer;

                let access_type = self.reference_access_type(pointer);
                let reference_kind = self.reference_kind(pointer);
                let reference_space = self.reference_space(pointer);
                let mut effect = MemoryAccessEffect::write(
                    MemoryEffectTarget::from_reference(
                        pointer,
                        access_type,
                        reference_kind,
                        reference_space,
                        self.target_layout.pointer_width_bits,
                    ),
                    true,
                );
                self.apply_reference_location(&mut effect, pointer);
                Self::single_effect(effect)
            }
            mir::Instruction::AtomicCompareExchange { pointer, .. }
            | mir::Instruction::AtomicRmw { pointer, .. } => {
                let pointer = *pointer;

                let access_type = self.reference_access_type(pointer);
                let reference_kind = self.reference_kind(pointer);
                let reference_space = self.reference_space(pointer);
                let mut effect = MemoryAccessEffect::read_write(
                    MemoryEffectTarget::from_reference(
                        pointer,
                        access_type,
                        reference_kind,
                        reference_space,
                        self.target_layout.pointer_width_bits,
                    ),
                    true,
                );
                self.apply_reference_location(&mut effect, pointer);
                Self::single_effect(effect)
            }
            mir::Instruction::AtomicFence { .. } => {
                Self::single_effect(MemoryAccessEffect::barrier())
            }
            mir::Instruction::BarrierWrite { .. } => Self::single_effect(
                MemoryAccessEffect::read_write(MemoryEffectTarget::any(mir::SpaceSet::ANY), false),
            ),
            mir::Instruction::LocalGet { local, .. } => {
                let mut effect = MemoryAccessEffect::read(MemoryEffectTarget::Local(*local), false);
                self.apply_local_location(&mut effect);
                Self::single_effect(effect)
            }
            mir::Instruction::LocalSet { local, .. } => {
                let mut effect =
                    MemoryAccessEffect::write(MemoryEffectTarget::Local(*local), false);
                self.apply_local_location(&mut effect);
                Self::single_effect(effect)
            }
            mir::Instruction::Call { .. }
            | mir::Instruction::CallVirtual { .. }
            | mir::Instruction::CallDynamic { .. } => {
                self.call_effects(instruction_id, instruction)
            }
            mir::Instruction::CallIndirect { .. } => self.call_effects(instruction_id, instruction),
            mir::Instruction::Free { .. }
            | mir::Instruction::Pin { .. }
            | mir::Instruction::Unpin { .. }
            | mir::Instruction::NewZeroed { .. }
            | mir::Instruction::NewUninit { .. }
            | mir::Instruction::NewComplete { .. }
            | mir::Instruction::NewSliceZeroed { .. }
            | mir::Instruction::NewSliceUninit { .. }
            | mir::Instruction::FrameAllocZeroed { .. }
            | mir::Instruction::FrameAllocUninit { .. } => Self::single_effect(
                MemoryAccessEffect::read_write(MemoryEffectTarget::any(mir::SpaceSet::ANY), false),
            ),
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
            mir::Terminator::Call { .. }
            | mir::Terminator::CallIndirect { .. }
            | mir::Terminator::CallVirtual { .. }
            | mir::Terminator::CallDynamic { .. }
            | mir::Terminator::TailCall { .. }
            | mir::Terminator::TailCallIndirect { .. }
            | mir::Terminator::TailCallVirtual { .. }
            | mir::Terminator::TailCallDynamic { .. } => self.callsite_effects(
                mir::CallSite::Terminator(block_id),
                terminator.call_direct_target(),
            ),

            mir::Terminator::NewZeroedTry { .. }
            | mir::Terminator::NewUninitTry { .. }
            | mir::Terminator::NewSliceZeroedTry { .. }
            | mir::Terminator::NewSliceUninitTry { .. } => Self::single_effect(
                MemoryAccessEffect::read_write(MemoryEffectTarget::any(mir::SpaceSet::ANY), false),
            ),

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

    /// Convert explicit memory metadata into access effects.
    fn metadata_effects(
        &mut self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
    ) -> Option<SmallVec<[MemoryAccessEffect; 2]>> {
        // read metadata when present
        let accesses = self.tree.metadata.memory.memory_accesses(instruction_id)?;

        // build effect list from metadata
        let mut effects = SmallVec::new();
        for access in accesses {
            effects.push(self.effect_from_metadata(access));
        }

        Some(effects)
    }

    /// Map a memory access metadata entry to a MemorySSA effect.
    fn effect_from_metadata(&mut self, access: &mir::MemoryAccessMetadata) -> MemoryAccessEffect {
        // resolve the target location
        let location = match access.target {
            mir::MemoryAccessTarget::Reference(pointer) => {
                let access_type = self.reference_access_type(pointer);
                let reference_kind = self.reference_kind(pointer);
                let reference_space = self.reference_space(pointer);
                MemoryEffectTarget::from_reference_with_size(
                    pointer,
                    access_type,
                    reference_kind,
                    reference_space,
                    access.size,
                    self.target_layout.pointer_width_bits,
                )
            }
            mir::MemoryAccessTarget::Local(local) => MemoryEffectTarget::Local(local),
            mir::MemoryAccessTarget::Global(global) => {
                MemoryEffectTarget::any(self.tree.get(global).space.space_set())
            }
            mir::MemoryAccessTarget::Unknown => MemoryEffectTarget::any(mir::SpaceSet::ANY),
        };

        // map the access kind to an effect
        let mut effect = match access.kind {
            mir::MemoryAccessKind::Read => MemoryAccessEffect::read(location, access.is_volatile),
            mir::MemoryAccessKind::Write => MemoryAccessEffect::write(location, access.is_volatile),
            mir::MemoryAccessKind::ReadWrite | mir::MemoryAccessKind::ReadModifyWrite => {
                MemoryAccessEffect::read_write(location, access.is_volatile)
            }
            mir::MemoryAccessKind::PrefetchRead | mir::MemoryAccessKind::PrefetchWrite => {
                MemoryAccessEffect::read(location, access.is_volatile)
            }
            mir::MemoryAccessKind::Fence => {
                let mut effect = MemoryAccessEffect::barrier();
                effect.is_volatile = access.is_volatile;
                effect
            }
        };

        // ordered accesses act like volatile for optimization purposes
        if access.ordering.is_some() {
            effect.is_volatile = true;
        }

        // flags and scopes imply atomic behavior
        if access.flags.is_some() || access.scope.is_some() || access.memory_scope.is_some() {
            effect.is_volatile = true;
        }

        // availability and visibility flags block reordering
        if let Some(flags) = access.flags
            && (flags.makes_available || flags.makes_visible)
        {
            effect.is_barrier = true;
        }

        // apply location metadata
        let spaces = self.metadata_space_set(access, &effect.location);
        effect.location.set_spaces(spaces);
        effect
    }

    /// Apply reference location metadata to an effect.
    fn apply_reference_location(&mut self, effect: &mut MemoryAccessEffect, reference: mir::Value) {
        let spaces = self.reference_space_set(reference);
        effect.location.set_spaces(spaces);
    }

    /// Apply local location metadata to an effect.
    fn apply_local_location(&mut self, effect: &mut MemoryAccessEffect) {
        effect.location.set_spaces(mir::SpaceSet::FRAME);
    }

    /// Resolve location metadata from an access description.
    fn metadata_space_set(
        &mut self,
        access: &mir::MemoryAccessMetadata,
        location: &MemoryEffectTarget,
    ) -> mir::SpaceSet {
        if let Some(space) = access.space.clone() {
            return space.space_set();
        }

        match access.target {
            mir::MemoryAccessTarget::Local(_) => mir::SpaceSet::FRAME,
            mir::MemoryAccessTarget::Global(global) => self.tree.get(global).space.space_set(),
            mir::MemoryAccessTarget::Reference(pointer) => self.reference_space_set(pointer),
            mir::MemoryAccessTarget::Unknown => location.spaces(),
        }
    }

    /// Resolve the location set for a reference value.
    fn reference_space_set(&mut self, reference: mir::Value) -> mir::SpaceSet {
        let Some(space) = self.value_types.reference_space(reference, self.tree) else {
            return mir::SpaceSet::ANY;
        };

        space.space_set()
    }

    /// Determine memory effects for a call instruction using metadata.
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

    /// Determine memory effects for a callsite using metadata.
    fn callsite_effects(
        &self,
        callsite: mir::CallSite,
        direct_target: Option<mir::LocalNodeId<mir::Function>>,
    ) -> SmallVec<[MemoryAccessEffect; 2]> {
        // use callsite or callee metadata for memory effects
        let call_metadata = self.tree.metadata.functions.call(callsite);
        let mut memory_effects = call_metadata
            .map(|metadata| metadata.memory.clone())
            .or_else(|| self.callee_memory_effects(direct_target));

        // fall back to conservative unknown when missing
        let Some(effects) = memory_effects.take() else {
            return Self::single_effect(MemoryAccessEffect::read_write(
                MemoryEffectTarget::any(mir::SpaceSet::ANY),
                false,
            ));
        };

        // skip calls with no memory effects
        if !effects.reads && !effects.writes {
            return SmallVec::new();
        }

        // empty spaces do not touch program memory
        if effects.spaces.is_empty() {
            return SmallVec::new();
        }

        // fall back to a single summarized access
        Self::single_effect(self.effect_from_call_effect(&effects))
    }

    /// Convert call memory effects to a MemorySSA effect.
    fn effect_from_call_effect(&self, effects: &mir::MemoryEffect) -> MemoryAccessEffect {
        // translate call summary into a generic access effect
        match (effects.reads, effects.writes) {
            (true, true) => {
                MemoryAccessEffect::read_write(MemoryEffectTarget::any(effects.spaces), false)
            }
            (true, false) => {
                MemoryAccessEffect::read(MemoryEffectTarget::any(effects.spaces), false)
            }
            (false, true) => {
                MemoryAccessEffect::write(MemoryEffectTarget::any(effects.spaces), false)
            }
            (false, false) => {
                MemoryAccessEffect::read_write(MemoryEffectTarget::any(effects.spaces), false)
            }
        }
    }

    /// Read memory effects from a direct callee when available.
    fn callee_memory_effects(
        &self,
        function: Option<mir::LocalNodeId<mir::Function>>,
    ) -> Option<mir::MemoryEffect> {
        // only direct calls have callee metadata
        let function = function?;
        self.tree
            .metadata
            .functions
            .function(function)
            .map(|metadata| metadata.memory.clone())
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
            // reflection
            mir::Intrinsic::TypeOf | mir::Intrinsic::SizeOf | mir::Intrinsic::AlignOf => {
                SmallVec::new()
            }

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
                        let dst_type = self.reference_access_type(*dst);
                        let src_type = self.reference_access_type(*src);
                        let dst_kind = self.reference_kind(*dst);
                        let src_kind = self.reference_kind(*src);
                        let dst_space = self.reference_space(*dst);
                        let src_space = self.reference_space(*src);
                        let mut read_effect = MemoryAccessEffect::read(
                            MemoryEffectTarget::from_reference_with_size(
                                *src,
                                src_type,
                                src_kind,
                                src_space,
                                size,
                                self.target_layout.pointer_width_bits,
                            ),
                            false,
                        );
                        self.apply_reference_location(&mut read_effect, *src);
                        effects.push(read_effect);

                        let mut write_effect = MemoryAccessEffect::write(
                            MemoryEffectTarget::from_reference_with_size(
                                *dst,
                                dst_type,
                                dst_kind,
                                dst_space,
                                size,
                                self.target_layout.pointer_width_bits,
                            ),
                            false,
                        );
                        self.apply_reference_location(&mut write_effect, *dst);
                        effects.push(write_effect);
                    }
                    _ => effects.push(MemoryAccessEffect::read_write(
                        MemoryEffectTarget::any(mir::SpaceSet::ANY),
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
                        let dst_type = self.reference_access_type(*dst);
                        let dst_kind = self.reference_kind(*dst);
                        let dst_space = self.reference_space(*dst);
                        let mut effect = MemoryAccessEffect::write(
                            MemoryEffectTarget::from_reference_with_size(
                                *dst,
                                dst_type,
                                dst_kind,
                                dst_space,
                                size,
                                self.target_layout.pointer_width_bits,
                            ),
                            false,
                        );
                        self.apply_reference_location(&mut effect, *dst);
                        effects.push(effect);
                    }
                    None => effects.push(MemoryAccessEffect::write(
                        MemoryEffectTarget::any(mir::SpaceSet::ANY),
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
                        let left_type = self.reference_access_type(*left);
                        let right_type = self.reference_access_type(*right);
                        let left_kind = self.reference_kind(*left);
                        let right_kind = self.reference_kind(*right);
                        let left_space = self.reference_space(*left);
                        let right_space = self.reference_space(*right);
                        let mut left_effect = MemoryAccessEffect::read(
                            MemoryEffectTarget::from_reference_with_size(
                                *left,
                                left_type,
                                left_kind,
                                left_space,
                                size,
                                self.target_layout.pointer_width_bits,
                            ),
                            false,
                        );
                        self.apply_reference_location(&mut left_effect, *left);
                        effects.push(left_effect);

                        let mut right_effect = MemoryAccessEffect::read(
                            MemoryEffectTarget::from_reference_with_size(
                                *right,
                                right_type,
                                right_kind,
                                right_space,
                                size,
                                self.target_layout.pointer_width_bits,
                            ),
                            false,
                        );
                        self.apply_reference_location(&mut right_effect, *right);
                        effects.push(right_effect);
                    }
                    _ => effects.push(MemoryAccessEffect::read(
                        MemoryEffectTarget::any(mir::SpaceSet::ANY),
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
                        let access_type = self.reference_access_type(pointer);
                        let reference_kind = self.reference_kind(pointer);
                        let reference_space = self.reference_space(pointer);
                        let mut effect = MemoryAccessEffect::read(
                            MemoryEffectTarget::from_reference(
                                pointer,
                                access_type,
                                reference_kind,
                                reference_space,
                                self.target_layout.pointer_width_bits,
                            ),
                            false,
                        );
                        self.apply_reference_location(&mut effect, pointer);
                        effects.push(effect);
                    }
                    None => effects.push(MemoryAccessEffect::read(
                        MemoryEffectTarget::any(mir::SpaceSet::ANY),
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

            // control flow and debugging
            mir::Intrinsic::Breakpoint
            | mir::Intrinsic::ReturnAddress
            | mir::Intrinsic::FrameAddress
            | mir::Intrinsic::Expect
            | mir::Intrinsic::BlackBox => SmallVec::new(),
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
    fn reference_access_type(&mut self, reference: mir::Value) -> Option<TypeKey> {
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
                Some(MemoryAccess::Use(use_access)) => {
                    use_access.defining_access = Some(current);
                }
                Some(MemoryAccess::Def(def_access)) => {
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
                && let Some(MemoryAccess::Phi(phi)) = ssa.accesses.get_mut(phi_id.index())
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
            MemoryAccess::Use(use_access) => use_access.effect.clone(),
            MemoryAccess::Def(def_access) => def_access.effect.clone(),
            MemoryAccess::Phi(_) | MemoryAccess::LiveOnEntry => {
                panic!("expected effectful access")
            }
        }
    }

    /// Extract the reference value from a location when available.
    fn reference_from_location(location: &MemoryEffectTarget) -> Option<mir::Value> {
        // unwrap reference backed spaces
        match location {
            MemoryEffectTarget::Reference { location, .. } => Some(location.reference),
            _ => None,
        }
    }

    /// Extract the byte size from a location when available.
    fn size_from_location(location: &MemoryEffectTarget) -> Option<u64> {
        // unwrap reference backed spaces
        match location {
            MemoryEffectTarget::Reference { location, .. } => location.size,
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
        let analyses = test.function_analyses();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let memory_ssa = memory_ssa.as_ref();

        // find memory accesses
        let block = test.tree.get(function.blocks[0]);
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
        let analyses = test.function_analyses();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let memory_ssa = memory_ssa.as_ref();

        // fetch join block phi
        let join_block = function.blocks[3];
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
        let MemoryAccess::Phi(phi) = memory_ssa.access(phi_id) else {
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
        let analyses = test.function_analyses();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let alias = analyses.get::<AliasAnalysis>(function, &test.tree);

        // locate accesses
        let block = test.tree.get(function.blocks[0]);
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

    /// Memory metadata marks scoped effects as volatile barriers.
    #[test]
    fn test_memory_ssa_metadata_marks_effects() {
        let mut test = TestProgram::new(
            r#"
function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = load v0
    return v1
}
"#,
        );

        let function_id = test.first_function_id();
        let instructions = test.entry_instructions(function_id);
        let load_inst = instructions[1];

        let flags = mir::MemoryFlags::with_flags(mir::SpaceSet::ANY, true, false);
        let access = mir::MemoryAccessMetadata {
            kind: mir::MemoryAccessKind::Read,
            target: mir::MemoryAccessTarget::Reference(mir::Value::new(0)),
            size: Some(4),
            alignment: None,
            is_volatile: false,
            is_load_invariant: false,
            ordering: None,
            scope: Some(mir::SyncScope::Device),
            memory_scope: Some(mir::MemoryScope::Device),
            flags: Some(flags),
            space: Some(mir::Space::Frame),
        };
        test.insert_memory_accesses(load_inst, vec![access]);

        let function = test.tree.get(function_id);
        let analyses = test.function_analyses();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);

        let access_id = memory_ssa
            .instruction_access(load_inst)
            .expect("missing load access");
        let effect = access_effect(memory_ssa.as_ref(), access_id);
        assert!(effect.is_volatile);
        assert!(effect.is_barrier);
    }

    /// Backing spaces can disambiguate memory accesses.
    #[test]
    fn test_memory_ssa_space_disambiguate() {
        let mut test = TestProgram::new(
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
        let instructions = test.entry_instructions(function_id);
        let store_inst = instructions[1];
        let load_inst = instructions[2];

        let store_access = mir::MemoryAccessMetadata {
            kind: mir::MemoryAccessKind::Write,
            target: mir::MemoryAccessTarget::Reference(mir::Value::new(0)),
            size: Some(4),
            alignment: None,
            is_volatile: false,
            is_load_invariant: false,
            ordering: None,
            scope: None,
            memory_scope: None,
            flags: None,
            space: Some(mir::Space::Frame),
        };
        let load_access = mir::MemoryAccessMetadata {
            kind: mir::MemoryAccessKind::Read,
            target: mir::MemoryAccessTarget::Reference(mir::Value::new(0)),
            size: Some(4),
            alignment: None,
            is_volatile: false,
            is_load_invariant: false,
            ordering: None,
            scope: None,
            memory_scope: None,
            flags: None,
            space: Some(mir::Space::Static),
        };

        test.insert_memory_accesses(store_inst, vec![store_access]);
        test.insert_memory_accesses(load_inst, vec![load_access]);

        let function = test.tree.get(function_id);
        let analyses = test.function_analyses();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let alias = analyses.get::<AliasAnalysis>(function, &test.tree);

        let load_access = memory_ssa
            .instruction_access(load_inst)
            .expect("missing load access");

        let clobber = memory_ssa.clobbering_use(load_access, &alias);
        assert_eq!(clobber, memory_ssa.live_on_entry());
    }

    /// MemorySSA respects explicit metadata over instruction semantics.
    #[test]
    fn test_memory_ssa_metadata_overrides_instruction() {
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

        // attach metadata that retargets the load to v1
        test.insert_reference_access(
            load_v0,
            mir::MemoryAccessKind::Read,
            mir::Value::new(1),
            Some(4),
        );

        // build analyses
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let alias = analyses.get::<AliasAnalysis>(function, &test.tree);

        // locate memory accesses
        let load_access = memory_ssa
            .instruction_access(load_v0)
            .expect("missing load access");
        let store_access = memory_ssa
            .instruction_access(store_v1)
            .expect("missing store access");

        // clobber should follow the metadata target
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
        let analyses = test.function_analyses();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);

        // locate local access
        let block = test.tree.get(function.blocks[0]);
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
        let analyses = test.function_analyses();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let memory_ssa = memory_ssa.as_ref();

        let free_access = memory_ssa
            .instruction_access(free_inst)
            .expect("missing free access");
        let effect = access_effect(memory_ssa, free_access);

        assert!(effect.reads);
        assert!(effect.writes);
        assert!(matches!(effect.location, MemoryEffectTarget::Any { .. }));
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
        let analyses = test.function_analyses();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let memory_ssa = memory_ssa.as_ref();

        let alloc_access = memory_ssa
            .instruction_access(alloc_inst)
            .expect("missing alloc access");
        let effect = access_effect(memory_ssa, alloc_access);

        assert!(effect.reads);
        assert!(effect.writes);
        assert!(matches!(effect.location, MemoryEffectTarget::Any { .. }));
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
        let analyses = test.function_analyses();
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
            reference_from_location(&read_effect.location).expect("missing read reference");
        let write_reference =
            reference_from_location(&write_effect.location).expect("missing write reference");

        let frame_allocs = test.frame_alloc_destinations_in_entry(function_id);
        let dest_value = *frame_allocs.first().expect("missing stack allocation");
        let src_value = *frame_allocs.get(1).expect("missing stack allocation");

        assert_eq!(read_reference, src_value);
        assert_eq!(write_reference, dest_value);

        let read_size = size_from_location(&read_effect.location).expect("missing read size");
        let write_size = size_from_location(&write_effect.location).expect("missing write size");

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
        let analyses = test.function_analyses();
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
        let block = test.tree.get(function.blocks[0]);
        let volatile_load = block.instructions[1];
        let volatile_store = block.instructions[2];

        // attach volatile memory access metadata
        test.insert_reference_access_with_options(
            volatile_load,
            mir::MemoryAccessKind::Read,
            mir::Value::new(0),
            Some(4),
            true,
            None,
        );
        test.insert_reference_access_with_options(
            volatile_store,
            mir::MemoryAccessKind::Write,
            mir::Value::new(0),
            Some(4),
            true,
            None,
        );

        let function = test.tree.get(function_id);
        let analyses = test.function_analyses();
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
        let analyses = test.function_analyses();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let memory_ssa = memory_ssa.as_ref();

        let block = test.tree.get(function.blocks[0]);
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
    atomic.fence sequentiallyConsistent, scope(device), memory(device)
    v0: int32 = 0
    return v0
}
"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let memory_ssa = memory_ssa.as_ref();

        // locate fence instruction
        let block = test.tree.get(function.blocks[0]);
        let fence_inst = block.instructions[0];
        let fence_access = memory_ssa
            .instruction_access(fence_inst)
            .expect("missing fence access");

        let fence_effect = access_effect(memory_ssa, fence_access);
        assert!(fence_effect.is_barrier);
        assert!(matches!(
            fence_effect.location,
            MemoryEffectTarget::Any { .. }
        ));
    }

    /// Calls without precise metadata are modeled as read and write effects over any memory.
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
            .find(|(_, function)| function.entry.is_some())
            .expect("missing function")
            .0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let memory_ssa = memory_ssa.as_ref();

        // locate call instruction
        let block = test.tree.get(function.blocks[0]);
        let call_inst = block.instructions[0];
        let call_access = memory_ssa
            .instruction_access(call_inst)
            .expect("missing call access");

        let call_effect = access_effect(memory_ssa, call_access);
        assert!(call_effect.reads);
        assert!(call_effect.writes);
        assert!(matches!(
            call_effect.location,
            MemoryEffectTarget::Any { .. }
        ));
    }

    /// Continuation loads see memory effects from call terminators.
    #[test]
    fn test_memory_ssa_call_terminator_clobbers_continuation() {
        let test = TestProgram::new(
            r#"
external function imported(ref<int32, raw, mutable>): void

function test(v0: ref<int32, raw, mutable>): int32 {
entry(v0: ref<int32, raw, mutable>):
    call imported(v0) => b1

b1:
    v1: int32 = load v0
    return v1
}
"#,
        );

        let function_id = test.entry_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let memory_ssa = memory_ssa.as_ref();

        // locate the call terminator and continuation load
        let entry = function.blocks[0];
        let continuation = function.blocks[1];
        let load = test.tree.get(continuation).instructions[0];

        let call_accesses = terminator_accesses(memory_ssa, entry);
        let load_access = memory_ssa
            .instruction_access(load)
            .expect("missing load access");

        // require the call terminator to define the continuation memory state
        assert_eq!(call_accesses.len(), 1);
        assert_eq!(
            memory_ssa.defining_access(load_access),
            Some(call_accesses[0])
        );
    }

    /// Call metadata no memory suppresses memory accesses.
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
        test.tree.metadata.functions.call_mut(callsite).memory = mir::MemoryEffect::none();

        let function = test.tree.get(function_id);
        let analyses = test.function_analyses();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);

        assert!(
            memory_ssa.instruction_accesses(call_inst).is_none(),
            "no memory calls should not create memory accesses"
        );
    }

    /// Memory access metadata overrides default instruction effects.
    #[test]
    fn test_memory_ssa_access_metadata_override() {
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

        // build explicit access metadata
        let read_access = mir::MemoryAccessMetadata {
            kind: mir::MemoryAccessKind::Read,
            target: mir::MemoryAccessTarget::Reference(param_values[0]),
            size: Some(4),
            alignment: None,
            is_volatile: false,
            is_load_invariant: false,
            ordering: None,
            scope: None,
            memory_scope: None,
            flags: None,
            space: None,
        };
        let write_access = mir::MemoryAccessMetadata {
            kind: mir::MemoryAccessKind::Write,
            target: mir::MemoryAccessTarget::Reference(param_values[1]),
            size: Some(4),
            alignment: None,
            is_volatile: false,
            is_load_invariant: false,
            ordering: None,
            scope: None,
            memory_scope: None,
            flags: None,
            space: None,
        };

        // attach memory access metadata to the call
        test.insert_memory_accesses(call_inst, vec![read_access, write_access]);

        // build analyses
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses();
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
            reference_from_location(&read_effect.location).expect("missing read reference");
        let write_reference =
            reference_from_location(&write_effect.location).expect("missing write reference");
        assert_eq!(read_reference, function.parameters[0].value);
        assert_eq!(write_reference, function.parameters[1].value);

        let read_size = size_from_location(&read_effect.location).expect("missing read size");
        let write_size = size_from_location(&write_effect.location).expect("missing write size");
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
        let analyses = test.function_analyses();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let memory_ssa = memory_ssa.as_ref();

        // find phi for loop header
        let header_block = function.blocks[1];
        let phi_id = memory_ssa
            .block_phi(header_block)
            .expect("missing loop header phi");

        let MemoryAccess::Phi(phi) = memory_ssa.access(phi_id) else {
            panic!("expected memory phi");
        };

        let incoming_blocks: HashSet<_> = phi.incoming.iter().map(|(block, _)| *block).collect();
        let body_block = function.blocks[2];

        assert!(incoming_blocks.contains(&function.blocks[0]));
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
        let analyses = test.function_analyses();
        let memory_ssa = analyses.get::<MemorySSA>(function, &test.tree);
        let memory_ssa = memory_ssa.as_ref();

        // locate store in unreachable block
        let unreachable_block = function.blocks[1];
        let block = test.tree.get(unreachable_block);
        let store_inst = block.instructions[2];

        assert!(
            memory_ssa.instruction_accesses(store_inst).is_none(),
            "unreachable store should not be tracked"
        );
    }
}
