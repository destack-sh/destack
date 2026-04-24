use std::collections::{HashMap, HashSet, VecDeque};

use destack_mir as mir;
use smallvec::SmallVec;

use crate::optimize::common::{
    MemoryLocation, TypeKey, ValueTypeMap, address_spaces_may_alias, alias_scopes_may_alias,
    build_value_definition_map, collect_reachable_blocks, compute_dominance_frontiers,
    resolve_pointer_address_space, resolve_pointer_kind, resolve_pointer_pointee_type,
    space_sets_may_alias, type_alias_tags_may_alias,
};
use crate::optimize::{
    Analysis, AnalysisId, ControlFlowGraph, DominatorTree, FunctionAnalyses, FunctionAnalysis,
    TypeContext,
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

/// Location being accessed by a memory operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MemoryAccessLocation {
    /// Pointer based memory location.
    Pointer(MemoryLocation),
    /// Local slot access.
    Local(mir::LocalNodeId<mir::Local>),
    /// Unknown memory location.
    Unknown,
}

impl MemoryAccessLocation {
    /// Create a pointer location with optional access type and inferred size.
    fn from_pointer(
        ptr: mir::Value,
        access_type: Option<TypeKey>,
        pointer_kind: Option<mir::ReferenceKind>,
        pointer_address_space: Option<mir::AddressSpace>,
        pointer_width_bits: u16,
    ) -> Self {
        Self::from_pointer_with_size(
            ptr,
            access_type,
            pointer_kind,
            pointer_address_space,
            None,
            pointer_width_bits,
        )
    }

    /// Create a pointer location with an explicit size override.
    fn from_pointer_with_size(
        ptr: mir::Value,
        access_type: Option<TypeKey>,
        pointer_kind: Option<mir::ReferenceKind>,
        pointer_address_space: Option<mir::AddressSpace>,
        size: Option<u64>,
        pointer_width_bits: u16,
    ) -> Self {
        let inferred_size = size.or_else(|| {
            access_type
                .as_ref()
                .and_then(|access_type| access_type.byte_size(pointer_width_bits))
        });
        MemoryAccessLocation::Pointer(MemoryLocation::new(
            ptr,
            inferred_size,
            access_type,
            pointer_kind,
            pointer_address_space,
        ))
    }

    /// Return a pointer location if available.
    fn as_pointer(&self) -> Option<&MemoryLocation> {
        // unwrap pointer backed spaces
        match self {
            MemoryAccessLocation::Pointer(location) => Some(location),
            _ => None,
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
    pub location: MemoryAccessLocation,
    /// The effect space set associated with this access.
    pub space_set: mir::MemorySpaceSet,
    /// The address spaces associated with this access.
    pub address_spaces: Option<mir::AddressSpaceSet>,
    /// Alias scopes applied to this access.
    pub alias_scopes: Vec<mir::MemoryAliasScopeId>,
    /// No alias scopes applied to this access.
    pub noalias_scopes: Vec<mir::MemoryAliasScopeId>,
    /// Optional type-alias tag for this access.
    pub type_alias_tag: Option<mir::TypeAliasTagId>,
}

/// Query information for clobbering access lookups.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct MemoryAccessQuery {
    /// The memory location being accessed.
    location: MemoryAccessLocation,
    /// The effect space set associated with this query.
    space_set: mir::MemorySpaceSet,
    /// The address spaces associated with this query.
    address_spaces: Option<mir::AddressSpaceSet>,
    /// Alias scopes applied to this access.
    alias_scopes: Vec<mir::MemoryAliasScopeId>,
    /// No alias scopes applied to this access.
    noalias_scopes: Vec<mir::MemoryAliasScopeId>,
    /// Optional type-alias tag for the access.
    type_alias_tag: Option<mir::TypeAliasTagId>,
}

impl MemoryAccessQuery {
    /// Create a query from a full access effect.
    fn from_effect(effect: &MemoryAccessEffect) -> Self {
        // canonicalize alias scopes for cache stability
        let alias_scopes = canonicalize_alias_scopes(effect.alias_scopes.clone());
        let noalias_scopes = canonicalize_alias_scopes(effect.noalias_scopes.clone());

        Self {
            location: effect.location.clone(),
            space_set: effect.space_set,
            address_spaces: effect.address_spaces.clone(),
            alias_scopes,
            noalias_scopes,
            type_alias_tag: effect.type_alias_tag,
        }
    }

    /// Create a query from a location with no alias metadata.
    fn from_location(location: &MemoryAccessLocation) -> Self {
        Self {
            location: location.clone(),
            space_set: space_set_for_location(location),
            address_spaces: None,
            alias_scopes: Vec::new(),
            noalias_scopes: Vec::new(),
            type_alias_tag: None,
        }
    }
}

/// Determine the location set for a memory location.
fn space_set_for_location(location: &MemoryAccessLocation) -> mir::MemorySpaceSet {
    match location {
        MemoryAccessLocation::Local(_) => mir::MemorySpaceSet::STACK,
        _ => mir::MemorySpaceSet::ANY,
    }
}

/// Canonicalize alias scope lists for stable comparisons.
fn canonicalize_alias_scopes(
    mut scopes: Vec<mir::MemoryAliasScopeId>,
) -> Vec<mir::MemoryAliasScopeId> {
    // sort scopes by id and drop duplicates
    scopes.sort_by_key(|scope| scope.index());
    scopes.dedup();
    scopes
}

impl MemoryAccessEffect {
    /// Create an access effect for reads.
    fn read(location: MemoryAccessLocation, is_volatile: bool) -> Self {
        Self {
            reads: true,
            writes: false,
            is_volatile,
            is_barrier: false,
            location,
            space_set: mir::MemorySpaceSet::ANY,
            address_spaces: None,
            alias_scopes: Vec::new(),
            noalias_scopes: Vec::new(),
            type_alias_tag: None,
        }
    }

    /// Create an access effect for writes.
    fn write(location: MemoryAccessLocation, is_volatile: bool) -> Self {
        Self {
            reads: false,
            writes: true,
            is_volatile,
            is_barrier: false,
            location,
            space_set: mir::MemorySpaceSet::ANY,
            address_spaces: None,
            alias_scopes: Vec::new(),
            noalias_scopes: Vec::new(),
            type_alias_tag: None,
        }
    }

    /// Create an access effect for read/write.
    fn read_write(location: MemoryAccessLocation, is_volatile: bool) -> Self {
        Self {
            reads: true,
            writes: true,
            is_volatile,
            is_barrier: false,
            location,
            space_set: mir::MemorySpaceSet::ANY,
            address_spaces: None,
            alias_scopes: Vec::new(),
            noalias_scopes: Vec::new(),
            type_alias_tag: None,
        }
    }

    /// Create a barrier access effect.
    fn barrier() -> Self {
        Self {
            reads: false,
            writes: true,
            is_volatile: false,
            is_barrier: true,
            location: MemoryAccessLocation::Unknown,
            space_set: mir::MemorySpaceSet::ANY,
            address_spaces: None,
            alias_scopes: Vec::new(),
            noalias_scopes: Vec::new(),
            type_alias_tag: None,
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
        // map access to its instruction
        match self {
            MemoryAccess::Def(def) => Some(def.instruction),
            MemoryAccess::Use(use_access) => Some(use_access.instruction),
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
    /// Instruction that defines memory.
    pub instruction: mir::LocalNodeId<mir::Instruction>,
    /// Immediate defining access in MemorySSA.
    pub defining_access: Option<MemoryAccessId>,
    /// Memory effects for this instruction.
    pub effect: MemoryAccessEffect,
}

/// Memory use access.
#[derive(Debug, Clone)]
pub struct MemoryUse {
    /// Instruction that reads memory.
    pub instruction: mir::LocalNodeId<mir::Instruction>,
    /// Immediate defining access in MemorySSA.
    pub defining_access: Option<MemoryAccessId>,
    /// Memory effects for this instruction.
    pub effect: MemoryAccessEffect,
}

/// MemorySSA analysis for a function.
#[derive(Debug)]
pub struct MemorySSA {
    /// All memory accesses indexed by id.
    accesses: Vec<MemoryAccess>,
    /// Memory phi nodes per block.
    block_phis: HashMap<mir::LocalNodeId<mir::Block>, MemoryAccessId>,
    /// Instruction to memory accesses mapping.
    instruction_access: HashMap<mir::LocalNodeId<mir::Instruction>, Vec<MemoryAccessId>>,
    /// Memory accesses per block.
    block_accesses: HashMap<mir::LocalNodeId<mir::Block>, Vec<MemoryAccessId>>,
    /// Live on entry access id.
    live_on_entry: MemoryAccessId,
}

impl MemorySSA {
    /// Build MemorySSA for a function.
    fn build(
        function: &mir::Function,
        tree: &mir::NodeTree,
        cfg: &ControlFlowGraph,
        domtree: &DominatorTree,
        type_context: TypeContext,
    ) -> Self {
        // handle imported functions
        let entry = match function.entry {
            Some(entry) => entry,
            None => {
                let live_on_entry = MemoryAccessId::from_index(0);
                return Self {
                    accesses: vec![MemoryAccess::LiveOnEntry],
                    block_phis: HashMap::new(),
                    instruction_access: HashMap::new(),
                    block_accesses: HashMap::new(),
                    live_on_entry,
                };
            }
        };

        // collect memory accesses and definition blocks
        let mut access_collector = MemoryAccessCollector::new(function, tree, type_context);
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
        let mut block_phis = HashMap::new();
        for block in &phi_blocks {
            let phi_id = MemoryAccessId::from_index(accesses.len());
            accesses.push(MemoryAccess::Phi(MemoryPhi {
                block: *block,
                incoming: Vec::new(),
            }));
            block_phis.insert(*block, phi_id);
        }

        // create instruction memory accesses
        let mut instruction_access: HashMap<_, Vec<MemoryAccessId>> = HashMap::new();
        let mut block_accesses = HashMap::new();
        for (&block_id, access_list) in &collected.block_accesses {
            let mut block_list = Vec::new();

            for access in access_list {
                let access_id = MemoryAccessId::from_index(accesses.len());
                match access {
                    CollectedAccess::Use {
                        instruction,
                        effect,
                    } => {
                        accesses.push(MemoryAccess::Use(MemoryUse {
                            instruction: *instruction,
                            defining_access: None,
                            effect: effect.clone(),
                        }));
                        instruction_access
                            .entry(*instruction)
                            .or_default()
                            .push(access_id);
                    }
                    CollectedAccess::Def {
                        instruction,
                        effect,
                    } => {
                        accesses.push(MemoryAccess::Def(MemoryDef {
                            instruction: *instruction,
                            defining_access: None,
                            effect: effect.clone(),
                        }));
                        instruction_access
                            .entry(*instruction)
                            .or_default()
                            .push(access_id);
                    }
                }
                block_list.push(access_id);
            }

            block_accesses.insert(block_id, block_list);
        }

        // assemble memory ssa state
        let mut ssa = Self {
            accesses,
            block_phis,
            instruction_access,
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
    pub fn access_for_instruction(
        &self,
        instruction: mir::LocalNodeId<mir::Instruction>,
    ) -> Option<MemoryAccessId> {
        self.accesses_for_instruction(instruction)
            .and_then(|accesses| accesses.first().copied())
    }

    /// Return all memory accesses for an instruction if present.
    pub fn accesses_for_instruction(
        &self,
        instruction: mir::LocalNodeId<mir::Instruction>,
    ) -> Option<&[MemoryAccessId]> {
        self.instruction_access.get(&instruction).map(Vec::as_slice)
    }

    /// Return the first memory use access for an instruction.
    pub fn first_use_access(
        &self,
        instruction: mir::LocalNodeId<mir::Instruction>,
    ) -> Option<MemoryAccessId> {
        let accesses = self.accesses_for_instruction(instruction)?;
        accesses
            .iter()
            .copied()
            .find(|access_id| matches!(self.access(*access_id), MemoryAccess::Use(_)))
    }

    /// Return the memory phi for a block if present.
    pub fn phi_for_block(&self, block: mir::LocalNodeId<mir::Block>) -> Option<MemoryAccessId> {
        self.block_phis.get(&block).copied()
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
    pub fn clobbering_access_for_use(
        &self,
        use_access: MemoryAccessId,
        alias: &crate::optimize::analyses::AliasAnalysis,
        tree: &mir::NodeTree,
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
        self.clobbering_access(
            defining_access,
            &query,
            alias,
            tree,
            &mut cache,
            &mut visiting,
        )
    }

    /// Compute the clobbering access for a memory def.
    pub fn clobbering_access_for_def(
        &self,
        def_access: MemoryAccessId,
        alias: &crate::optimize::analyses::AliasAnalysis,
        tree: &mir::NodeTree,
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
        self.clobbering_access(
            defining_access,
            &query,
            alias,
            tree,
            &mut cache,
            &mut visiting,
        )
    }

    /// Compute the clobbering access for a read at the given location.
    pub fn clobbering_access_for_read(
        &self,
        access_id: MemoryAccessId,
        location: &MemoryAccessLocation,
        alias: &crate::optimize::analyses::AliasAnalysis,
        tree: &mir::NodeTree,
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
        self.clobbering_access(
            defining_access,
            &query,
            alias,
            tree,
            &mut cache,
            &mut visiting,
        )
    }

    /// Check if a def access clobbers the given location.
    pub fn def_clobbers_location(
        &self,
        def_access: MemoryAccessId,
        location: &MemoryAccessLocation,
        alias: &crate::optimize::analyses::AliasAnalysis,
    ) -> bool {
        // only defs can clobber spaces
        let MemoryAccess::Def(def_access) = self.access(def_access) else {
            return false;
        };

        access_clobbers_location(def_access, location, alias)
    }

    /// Check if a def access clobbers another access.
    pub fn def_clobbers_access(
        &self,
        def_access: MemoryAccessId,
        target_access: MemoryAccessId,
        alias: &crate::optimize::analyses::AliasAnalysis,
        tree: &mir::NodeTree,
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

        access_clobbers_query(def_access, &query, alias, tree)
    }

    /// Compute the clobbering access for a memory location.
    fn clobbering_access(
        &self,
        access_id: MemoryAccessId,
        query: &MemoryAccessQuery,
        alias: &crate::optimize::analyses::AliasAnalysis,
        tree: &mir::NodeTree,
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
                self.clobbering_access(defining_access, query, alias, tree, cache, visiting)
            }

            MemoryAccess::Def(def_access) => {
                if access_clobbers_query(def_access, query, alias, tree) {
                    access_id
                } else {
                    let defining_access = def_access
                        .defining_access
                        .expect("memory def missing defining access");
                    self.clobbering_access(defining_access, query, alias, tree, cache, visiting)
                }
            }

            MemoryAccess::Phi(phi) => {
                let mut incoming_clobber: Option<MemoryAccessId> = None;

                for (_, incoming) in &phi.incoming {
                    let clobber =
                        self.clobbering_access(*incoming, query, alias, tree, cache, visiting);
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
    const DEPENDENCIES: &'static [AnalysisId] = &[DominatorTree::ID, ControlFlowGraph::ID];
}

impl FunctionAnalysis for MemorySSA {
    fn compute(
        function: &mir::Function,
        tree: &mir::NodeTree,
        analyses: &FunctionAnalyses<'_>,
    ) -> Self {
        // read dependencies
        let cfg = analyses.get::<ControlFlowGraph>();
        let domtree = analyses.get::<DominatorTree>();

        Self::build(function, tree, &cfg, &domtree, analyses.type_context())
    }
}

/// Collected memory access before SSA renaming.
#[derive(Debug, Clone)]
enum CollectedAccess {
    /// Memory use (read).
    Use {
        instruction: mir::LocalNodeId<mir::Instruction>,
        effect: MemoryAccessEffect,
    },
    /// Memory def (write).
    Def {
        instruction: mir::LocalNodeId<mir::Instruction>,
        effect: MemoryAccessEffect,
    },
}

/// Memory access collection results.
struct MemoryAccessCollection {
    /// Memory accesses per block.
    block_accesses: HashMap<mir::LocalNodeId<mir::Block>, Vec<CollectedAccess>>,
    /// Blocks that contain memory definitions.
    def_blocks: HashSet<mir::LocalNodeId<mir::Block>>,
    /// Blocks reachable from entry.
    reachable_blocks: Vec<mir::LocalNodeId<mir::Block>>,
    /// Reachable block set.
    reachable: HashSet<mir::LocalNodeId<mir::Block>>,
}

/// Helper to collect memory accesses.
struct MemoryAccessCollector<'a> {
    /// MIR function under analysis.
    function: &'a mir::Function,
    /// MIR node tree.
    tree: &'a mir::NodeTree,
    /// Value type lookup for pointer resolution.
    value_types: ValueTypeMap,
    /// Map from value to defining instruction.
    definitions: HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    /// Type key cache.
    type_keys: HashMap<mir::LocalNodeId<mir::Type>, TypeKey>,
    /// Type context for layout sensitive operations.
    type_context: TypeContext,
}

impl<'a> MemoryAccessCollector<'a> {
    /// Create a new collector.
    fn new(
        function: &'a mir::Function,
        tree: &'a mir::NodeTree,
        type_context: TypeContext,
    ) -> Self {
        // collect value definitions for pointer resolution
        let definitions = build_value_definition_map(function, tree);

        // build value type lookup
        let value_types = ValueTypeMap::new(function, tree);

        // build collector state
        Self {
            function,
            tree,
            value_types,
            definitions,
            type_keys: HashMap::new(),
            type_context,
        }
    }

    /// Collect memory accesses for all reachable blocks.
    fn collect(&mut self, entry: mir::LocalNodeId<mir::Block>) -> MemoryAccessCollection {
        // collect reachable blocks
        let reachable_blocks = collect_reachable_blocks(self.function, self.tree, entry);
        let reachable: HashSet<_> = reachable_blocks.iter().copied().collect();

        // collect memory accesses per block
        let mut block_accesses = HashMap::new();
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
                            instruction: instruction_id,
                            effect,
                        });
                        def_blocks.insert(block_id);
                    } else if effect.reads {
                        accesses.push(CollectedAccess::Use {
                            instruction: instruction_id,
                            effect,
                        });
                    }
                }
            }

            // store collected accesses when present
            if !accesses.is_empty() {
                block_accesses.insert(block_id, accesses);
            }
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
                panic!("recovered MIR instruction reached optimizer");
            }

            // pure instructions
            mir::Instruction::Const { .. }
            | mir::Instruction::Binary { .. }
            | mir::Instruction::Unary { .. }
            | mir::Instruction::Cast { .. }
            | mir::Instruction::Select { .. }
            | mir::Instruction::GlobalAddr { .. }
            | mir::Instruction::FunctionAddr { .. }
            | mir::Instruction::CallableBind { .. }
            | mir::Instruction::CallableEnvironment { .. }
            | mir::Instruction::LocalAddr { .. }
            | mir::Instruction::FieldGet { .. }
            | mir::Instruction::FieldAddr { .. }
            | mir::Instruction::FieldSet { .. }
            | mir::Instruction::ElementGet { .. }
            | mir::Instruction::ElementAddr { .. }
            | mir::Instruction::ElementSet { .. }
            | mir::Instruction::Struct { .. }
            | mir::Instruction::Tuple { .. }
            | mir::Instruction::Array { .. }
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
            | mir::Instruction::TensorDot { .. }
            | mir::Instruction::TensorConvolution { .. }
            | mir::Instruction::TensorGather { .. }
            | mir::Instruction::TensorScatter { .. }
            | mir::Instruction::TensorCompare { .. }
            | mir::Instruction::TensorSelect { .. }
            | mir::Instruction::TensorConvert { .. }
            | mir::Instruction::Assume { .. } => SmallVec::new(),
            mir::Instruction::TensorLoad { view, .. } => {
                let Some(view) = view.value() else {
                    return Self::single_effect(MemoryAccessEffect::read(
                        MemoryAccessLocation::Unknown,
                        false,
                    ));
                };

                let access_type = self.pointer_access_type(view);
                let pointer_kind = self.pointer_kind(view);
                let pointer_address_space = self.pointer_address_space(view);
                let mut effect = MemoryAccessEffect::read(
                    MemoryAccessLocation::from_pointer(
                        view,
                        access_type,
                        pointer_kind,
                        pointer_address_space,
                        self.type_context.pointer_width_bits,
                    ),
                    false,
                );
                self.apply_pointer_location(&mut effect, view);
                Self::single_effect(effect)
            }
            mir::Instruction::TensorStore { view, .. }
            | mir::Instruction::TensorFill { view, .. } => {
                let Some(view) = view.value() else {
                    return Self::single_effect(MemoryAccessEffect::write(
                        MemoryAccessLocation::Unknown,
                        false,
                    ));
                };

                let access_type = self.pointer_access_type(view);
                let pointer_kind = self.pointer_kind(view);
                let pointer_address_space = self.pointer_address_space(view);
                let mut effect = MemoryAccessEffect::write(
                    MemoryAccessLocation::from_pointer(
                        view,
                        access_type,
                        pointer_kind,
                        pointer_address_space,
                        self.type_context.pointer_width_bits,
                    ),
                    false,
                );
                self.apply_pointer_location(&mut effect, view);
                Self::single_effect(effect)
            }
            mir::Instruction::TensorCopy { target, source } => {
                let (Some(target), Some(source)) = (target.value(), source.value()) else {
                    return Self::single_effect(MemoryAccessEffect::read_write(
                        MemoryAccessLocation::Unknown,
                        false,
                    ));
                };

                let mut effects = SmallVec::new();
                let target_access = self.pointer_access_type(target);
                let target_kind = self.pointer_kind(target);
                let target_space = self.pointer_address_space(target);
                let mut target_effect = MemoryAccessEffect::write(
                    MemoryAccessLocation::from_pointer(
                        target,
                        target_access,
                        target_kind,
                        target_space,
                        self.type_context.pointer_width_bits,
                    ),
                    false,
                );
                self.apply_pointer_location(&mut target_effect, target);
                effects.push(target_effect);

                let source_access = self.pointer_access_type(source);
                let source_kind = self.pointer_kind(source);
                let source_space = self.pointer_address_space(source);
                let mut source_effect = MemoryAccessEffect::read(
                    MemoryAccessLocation::from_pointer(
                        source,
                        source_access,
                        source_kind,
                        source_space,
                        self.type_context.pointer_width_bits,
                    ),
                    false,
                );
                self.apply_pointer_location(&mut source_effect, source);
                effects.push(source_effect);

                effects
            }
            mir::Instruction::Load { pointer, .. } => {
                let Some(pointer) = pointer.value() else {
                    return Self::single_effect(MemoryAccessEffect::read(
                        MemoryAccessLocation::Unknown,
                        false,
                    ));
                };

                let access_type = self.pointer_access_type(pointer);
                let pointer_kind = self.pointer_kind(pointer);
                let pointer_space = self.pointer_address_space(pointer);
                let mut effect = MemoryAccessEffect::read(
                    MemoryAccessLocation::from_pointer(
                        pointer,
                        access_type,
                        pointer_kind,
                        pointer_space,
                        self.type_context.pointer_width_bits,
                    ),
                    false,
                );
                self.apply_pointer_location(&mut effect, pointer);
                Self::single_effect(effect)
            }
            mir::Instruction::Store { pointer, .. } => {
                let Some(pointer) = pointer.value() else {
                    return Self::single_effect(MemoryAccessEffect::write(
                        MemoryAccessLocation::Unknown,
                        false,
                    ));
                };

                let access_type = self.pointer_access_type(pointer);
                let pointer_kind = self.pointer_kind(pointer);
                let pointer_space = self.pointer_address_space(pointer);
                let mut effect = MemoryAccessEffect::write(
                    MemoryAccessLocation::from_pointer(
                        pointer,
                        access_type,
                        pointer_kind,
                        pointer_space,
                        self.type_context.pointer_width_bits,
                    ),
                    false,
                );
                self.apply_pointer_location(&mut effect, pointer);
                Self::single_effect(effect)
            }
            mir::Instruction::AtomicLoad { pointer, .. } => {
                let Some(pointer) = pointer.value() else {
                    return Self::single_effect(MemoryAccessEffect::read(
                        MemoryAccessLocation::Unknown,
                        true,
                    ));
                };

                let access_type = self.pointer_access_type(pointer);
                let pointer_kind = self.pointer_kind(pointer);
                let pointer_space = self.pointer_address_space(pointer);
                let mut effect = MemoryAccessEffect::read(
                    MemoryAccessLocation::from_pointer(
                        pointer,
                        access_type,
                        pointer_kind,
                        pointer_space,
                        self.type_context.pointer_width_bits,
                    ),
                    true,
                );
                self.apply_pointer_location(&mut effect, pointer);
                Self::single_effect(effect)
            }
            mir::Instruction::AtomicStore { pointer, .. } => {
                let Some(pointer) = pointer.value() else {
                    return Self::single_effect(MemoryAccessEffect::write(
                        MemoryAccessLocation::Unknown,
                        true,
                    ));
                };

                let access_type = self.pointer_access_type(pointer);
                let pointer_kind = self.pointer_kind(pointer);
                let pointer_space = self.pointer_address_space(pointer);
                let mut effect = MemoryAccessEffect::write(
                    MemoryAccessLocation::from_pointer(
                        pointer,
                        access_type,
                        pointer_kind,
                        pointer_space,
                        self.type_context.pointer_width_bits,
                    ),
                    true,
                );
                self.apply_pointer_location(&mut effect, pointer);
                Self::single_effect(effect)
            }
            mir::Instruction::AtomicCompareExchange { pointer, .. }
            | mir::Instruction::AtomicRmw { pointer, .. } => {
                let Some(pointer) = pointer.value() else {
                    return Self::single_effect(MemoryAccessEffect::read_write(
                        MemoryAccessLocation::Unknown,
                        true,
                    ));
                };

                let access_type = self.pointer_access_type(pointer);
                let pointer_kind = self.pointer_kind(pointer);
                let pointer_space = self.pointer_address_space(pointer);
                let mut effect = MemoryAccessEffect::read_write(
                    MemoryAccessLocation::from_pointer(
                        pointer,
                        access_type,
                        pointer_kind,
                        pointer_space,
                        self.type_context.pointer_width_bits,
                    ),
                    true,
                );
                self.apply_pointer_location(&mut effect, pointer);
                Self::single_effect(effect)
            }
            mir::Instruction::AtomicFence { .. } | mir::Instruction::Barrier { .. } => {
                Self::single_effect(MemoryAccessEffect::barrier())
            }
            mir::Instruction::LocalGet { local, .. } => {
                let Some(local) = local.local() else {
                    return Self::single_effect(MemoryAccessEffect::read(
                        MemoryAccessLocation::Unknown,
                        false,
                    ));
                };

                let mut effect =
                    MemoryAccessEffect::read(MemoryAccessLocation::Local(local), false);
                self.apply_local_location(&mut effect);
                Self::single_effect(effect)
            }
            mir::Instruction::LocalSet { local, .. } => {
                let Some(local) = local.local() else {
                    return Self::single_effect(MemoryAccessEffect::write(
                        MemoryAccessLocation::Unknown,
                        false,
                    ));
                };

                let mut effect =
                    MemoryAccessEffect::write(MemoryAccessLocation::Local(local), false);
                self.apply_local_location(&mut effect);
                Self::single_effect(effect)
            }
            mir::Instruction::Call { call, .. }
            | mir::Instruction::CallVirtual { call, .. }
            | mir::Instruction::CallInterface { call, .. } => {
                self.call_effects(instruction, call.arguments, None)
            }
            mir::Instruction::CallIndirect { call, .. } => {
                self.call_effects(instruction, call.arguments, None)
            }
            mir::Instruction::RawFree { .. }
            | mir::Instruction::Dispose { .. }
            | mir::Instruction::AsyncDispose { .. }
            | mir::Instruction::Pin { .. }
            | mir::Instruction::Unpin { .. }
            | mir::Instruction::Drop { .. }
            | mir::Instruction::New { .. }
            | mir::Instruction::NewSlice { .. }
            | mir::Instruction::RawAlloc { .. }
            | mir::Instruction::StackAlloc { .. } => Self::single_effect(
                MemoryAccessEffect::read_write(MemoryAccessLocation::Unknown, false),
            ),
            mir::Instruction::Intrinsic {
                intrinsic,
                arguments,
                ..
            } => self.intrinsic_effects(*intrinsic, *arguments),
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
            mir::MemoryAccessTarget::Pointer(pointer) => {
                let access_type = self.pointer_access_type(pointer);
                let pointer_kind = self.pointer_kind(pointer);
                let pointer_space = self.pointer_address_space(pointer);
                MemoryAccessLocation::from_pointer_with_size(
                    pointer,
                    access_type,
                    pointer_kind,
                    pointer_space,
                    access.size,
                    self.type_context.pointer_width_bits,
                )
            }
            mir::MemoryAccessTarget::Local(local) => MemoryAccessLocation::Local(local),
            mir::MemoryAccessTarget::Global(_) | mir::MemoryAccessTarget::Unknown => {
                MemoryAccessLocation::Unknown
            }
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

        // semantics and scopes imply atomic behavior, treat as volatile
        if access.semantics.is_some() || access.scope.is_some() || access.memory_scope.is_some() {
            effect.is_volatile = true;
        }

        // make-available/visible semantics act as barriers for optimization
        if let Some(semantics) = access.semantics
            && (semantics.is_make_available || semantics.is_make_visible)
        {
            effect.is_barrier = true;
        }

        // apply location metadata
        let (space_set, address_spaces) = self.space_set_for_metadata(access, &effect.location);
        effect.space_set = space_set;
        effect.address_spaces = address_spaces;

        effect.alias_scopes = canonicalize_alias_scopes(access.alias_scopes.clone());
        effect.noalias_scopes = canonicalize_alias_scopes(access.noalias_scopes.clone());
        effect.type_alias_tag = access.type_alias_tag;
        effect
    }

    /// Apply pointer location metadata to an effect.
    fn apply_pointer_location(&mut self, effect: &mut MemoryAccessEffect, pointer: mir::Value) {
        let (space_set, address_spaces) = self.space_set_for_pointer(pointer);
        effect.space_set = space_set;
        effect.address_spaces = address_spaces;
    }

    /// Apply local location metadata to an effect.
    fn apply_local_location(&mut self, effect: &mut MemoryAccessEffect) {
        effect.space_set = mir::MemorySpaceSet::STACK;
        effect.address_spaces = self.address_space_set(mir::AddressSpace::Stack);
    }

    /// Resolve location metadata from an access description.
    fn space_set_for_metadata(
        &mut self,
        access: &mir::MemoryAccessMetadata,
        location: &MemoryAccessLocation,
    ) -> (mir::MemorySpaceSet, Option<mir::AddressSpaceSet>) {
        if let Some(address_space) = access.address_space.clone() {
            let space_set = self.space_set_for_address_space(address_space.clone());
            return (space_set, self.address_space_set(address_space));
        }

        match access.target {
            mir::MemoryAccessTarget::Local(_) => (
                mir::MemorySpaceSet::STACK,
                self.address_space_set(mir::AddressSpace::Stack),
            ),
            mir::MemoryAccessTarget::Global(global) => (
                mir::MemorySpaceSet::STATIC,
                self.address_space_set(self.tree.get(global).space.clone()),
            ),
            mir::MemoryAccessTarget::Pointer(pointer) => self.space_set_for_pointer(pointer),
            mir::MemoryAccessTarget::Unknown => (space_set_for_location(location), None),
        }
    }

    /// Resolve the location set for a pointer value.
    fn space_set_for_pointer(
        &mut self,
        pointer: mir::Value,
    ) -> (mir::MemorySpaceSet, Option<mir::AddressSpaceSet>) {
        let Some(address_space) =
            resolve_pointer_address_space(pointer, self.tree, &self.value_types)
        else {
            return (mir::MemorySpaceSet::ANY, None);
        };

        let space_set = self.space_set_for_address_space(address_space.clone());
        let address_spaces = self.address_space_set(address_space);
        (space_set, address_spaces)
    }

    /// Map an address space to a location set.
    fn space_set_for_address_space(&self, address_space: mir::AddressSpace) -> mir::MemorySpaceSet {
        match address_space {
            mir::AddressSpace::Stack => mir::MemorySpaceSet::STACK,
            mir::AddressSpace::Frame => mir::MemorySpaceSet::STACK,
            mir::AddressSpace::Static => mir::MemorySpaceSet::STATIC,
            mir::AddressSpace::Shared => mir::MemorySpaceSet::SHARED,
            mir::AddressSpace::Local => mir::MemorySpaceSet::LOCAL,
            mir::AddressSpace::Named(_) => mir::MemorySpaceSet::ANY,
        }
    }

    /// Build an address space set when the space is explicit.
    fn address_space_set(&self, address_space: mir::AddressSpace) -> Option<mir::AddressSpaceSet> {
        Some(mir::AddressSpaceSet::new(vec![address_space]))
    }

    /// Merge pointer and call location sets conservatively.
    fn merge_space_sets(
        &self,
        pointer_set: mir::MemorySpaceSet,
        call_set: mir::MemorySpaceSet,
    ) -> mir::MemorySpaceSet {
        let intersection = pointer_set.intersection(call_set);
        if intersection.is_empty() {
            mir::MemorySpaceSet::ANY
        } else {
            intersection
        }
    }

    /// Merge pointer and call address space sets conservatively.
    fn merge_address_spaces(
        &self,
        pointer_spaces: Option<mir::AddressSpaceSet>,
        call_spaces: Option<&mir::AddressSpaceSet>,
    ) -> Option<mir::AddressSpaceSet> {
        match (pointer_spaces, call_spaces) {
            (Some(pointer_spaces), Some(call_spaces)) => {
                if pointer_spaces.is_disjoint(call_spaces) {
                    None
                } else {
                    let spaces = pointer_spaces
                        .spaces
                        .iter()
                        .cloned()
                        .filter(|space| call_spaces.contains(space.clone()))
                        .collect();
                    Some(mir::AddressSpaceSet::new(spaces))
                }
            }
            (Some(pointer_spaces), None) => Some(pointer_spaces),
            (None, Some(call_spaces)) => Some(call_spaces.clone()),
            (None, None) => None,
        }
    }

    /// Determine memory effects for a call instruction using metadata.
    fn call_effects(
        &mut self,
        instruction: &mir::Instruction,
        arguments: mir::ArgumentSlice,
        env: Option<mir::Value>,
    ) -> SmallVec<[MemoryAccessEffect; 2]> {
        // read callsite effects when present
        // use callsite or callee metadata for memory effects
        let mut memory_effects = instruction
            .call_memory_effect()
            .cloned()
            .or_else(|| self.callee_memory_effects(instruction));

        // fall back to conservative unknown when missing
        let Some(effects) = memory_effects.take() else {
            return Self::single_effect(MemoryAccessEffect::read_write(
                MemoryAccessLocation::Unknown,
                false,
            ));
        };

        // skip calls with no memory effects
        if !effects.reads && !effects.writes {
            return SmallVec::new();
        }

        // inaccessible only effects do not touch visible memory
        if effects.inaccessible_mem_only {
            return SmallVec::new();
        }

        // handle argmemonly calls by modeling argument accesses directly
        if effects.argmemonly {
            // load call arguments
            let mut args = self.tree.get_arguments(arguments).to_vec();
            if let Some(env) = env {
                args.push(env.into());
            }
            if args.is_empty() {
                return SmallVec::new();
            }

            // resolve argument types
            let arg_types = self.call_argument_types(instruction);
            let mut arg_types = if let Some(arg_types) = arg_types {
                arg_types
            } else {
                return Self::single_effect(self.effect_from_call_effect(&effects));
            };
            if let Some(env) = env {
                let Some(env_type) = self.value_types.value_type(env) else {
                    return Self::single_effect(self.effect_from_call_effect(&effects));
                };

                arg_types.push(env_type);
            }

            // collect access effects per argument
            let mut arg_effects = SmallVec::new();
            for (index, &arg_value) in args.iter().enumerate() {
                let Some(arg_value) = arg_value.value() else {
                    continue;
                };

                // read the argument type
                let arg_type = match arg_types.get(index) {
                    Some(ty) => *ty,
                    None => continue,
                };

                // skip non reference arguments
                if !matches!(
                    self.tree.get(arg_type),
                    mir::Type::Reference { .. } | mir::Type::TensorView { .. }
                ) {
                    continue;
                }

                // read argument metadata
                let arg_attribute = instruction
                    .call_argument_attributes()
                    .and_then(|argument_attributes| argument_attributes.get(index))
                    .cloned()
                    .unwrap_or_default();

                // clamp access to the call effects
                let access = self.clamp_argument_access(arg_attribute.access, &effects);
                if access == mir::ArgumentAccess::None {
                    continue;
                }

                // build the access location
                let size = arg_attribute
                    .attributes
                    .dereferenceable_bytes
                    .or(arg_attribute.attributes.dereferenceable_or_null_bytes);
                let access_type = self.pointer_access_type(arg_value);
                let pointer_kind = self.pointer_kind(arg_value);
                let pointer_space = self.pointer_address_space(arg_value);
                let location = MemoryAccessLocation::from_pointer_with_size(
                    arg_value,
                    access_type,
                    pointer_kind,
                    pointer_space,
                    size,
                    self.type_context.pointer_width_bits,
                );

                // convert access mode to a memory effect
                let mut effect = match access {
                    mir::ArgumentAccess::Read => MemoryAccessEffect::read(location, false),
                    mir::ArgumentAccess::Write => MemoryAccessEffect::write(location, false),
                    mir::ArgumentAccess::ReadWrite => {
                        MemoryAccessEffect::read_write(location, false)
                    }
                    mir::ArgumentAccess::None => continue,
                };

                // apply location metadata for argument accesses
                let (pointer_space_set, pointer_spaces) = self.space_set_for_pointer(arg_value);
                effect.space_set = self.merge_space_sets(pointer_space_set, effects.spaces);
                effect.address_spaces =
                    self.merge_address_spaces(pointer_spaces, effects.address_spaces.as_ref());

                effect.alias_scopes = canonicalize_alias_scopes(arg_attribute.alias_scopes.clone());
                effect.noalias_scopes =
                    canonicalize_alias_scopes(arg_attribute.noalias_scopes.clone());
                effect.type_alias_tag = arg_attribute.type_alias_tag;

                // record the access effect
                arg_effects.push(effect);
            }

            // return the recorded argument effects
            return arg_effects;
        }

        // fall back to a single summarized access
        Self::single_effect(self.effect_from_call_effect(&effects))
    }

    /// Convert call memory effects to a MemorySSA effect.
    fn effect_from_call_effect(&self, effects: &mir::MemoryEffect) -> MemoryAccessEffect {
        // translate call summary into a generic access effect
        let mut effect = match (effects.reads, effects.writes) {
            (true, true) => MemoryAccessEffect::read_write(MemoryAccessLocation::Unknown, false),
            (true, false) => MemoryAccessEffect::read(MemoryAccessLocation::Unknown, false),
            (false, true) => MemoryAccessEffect::write(MemoryAccessLocation::Unknown, false),
            (false, false) => MemoryAccessEffect::read_write(MemoryAccessLocation::Unknown, false),
        };

        effect.space_set = effects.spaces;
        effect.address_spaces = effects.address_spaces.clone();
        effect
    }

    /// Clamp argument access based on the call wide effects.
    fn clamp_argument_access(
        &self,
        access: mir::ArgumentAccess,
        effects: &mir::MemoryEffect,
    ) -> mir::ArgumentAccess {
        // drop access when the call does not touch memory
        if !effects.reads && !effects.writes {
            return mir::ArgumentAccess::None;
        }

        // compute the read and write mask for this argument
        let reads = effects.reads
            && matches!(
                access,
                mir::ArgumentAccess::Read | mir::ArgumentAccess::ReadWrite
            );
        let writes = effects.writes
            && matches!(
                access,
                mir::ArgumentAccess::Write | mir::ArgumentAccess::ReadWrite
            );

        // map the mask back to an access mode
        match (reads, writes) {
            (true, true) => mir::ArgumentAccess::ReadWrite,
            (true, false) => mir::ArgumentAccess::Read,
            (false, true) => mir::ArgumentAccess::Write,
            (false, false) => mir::ArgumentAccess::None,
        }
    }

    /// Resolve the call argument types from metadata or direct signatures.
    fn call_argument_types(
        &self,
        instruction: &mir::Instruction,
    ) -> Option<Vec<mir::LocalNodeId<mir::Type>>> {
        // prefer the signature from the instruction
        if let Some(signature) = instruction.call_signature() {
            let signature = self.tree.get(signature.ty()?);
            if let mir::Type::FunctionSignature { parameters, .. } = signature {
                return parameters
                    .iter()
                    .map(|parameter| parameter.ty())
                    .collect::<Option<Vec<_>>>();
            }
        }

        // fall back to direct call signatures when available
        if let mir::Instruction::Call { function, .. } = instruction {
            let callee = self.tree.get(function.function()?);
            let parameters = callee
                .parameters
                .iter()
                .map(|param| param.ty.ty())
                .collect::<Option<Vec<_>>>()?;
            return Some(parameters);
        }

        None
    }

    /// Read memory effects from a direct callee when available.
    fn callee_memory_effects(&self, instruction: &mir::Instruction) -> Option<mir::MemoryEffect> {
        // only direct calls have callee metadata
        let function = instruction.call_declared_target()?;
        let callee = self.tree.get(function.function()?);
        Some(callee.memory_effect.clone())
    }

    /// Determine memory effects for an intrinsic.
    fn intrinsic_effects(
        &mut self,
        intrinsic: mir::Intrinsic,
        arguments: mir::ArgumentSlice,
    ) -> SmallVec<[MemoryAccessEffect; 2]> {
        // load intrinsic arguments
        let args = self.tree.get_arguments(arguments);

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

            // checked arithmetic
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
                let dst = args.first().and_then(|value| value.value());
                let src = args.get(1).and_then(|value| value.value());
                let size = args
                    .get(2)
                    .and_then(|len| len.value())
                    .and_then(|len| self.constant_u64(len));

                // emit read and write effects when operands are present
                match (dst, src) {
                    (Some(dst), Some(src)) => {
                        let dst_type = self.pointer_access_type(dst);
                        let src_type = self.pointer_access_type(src);
                        let dst_kind = self.pointer_kind(dst);
                        let src_kind = self.pointer_kind(src);
                        let dst_space = self.pointer_address_space(dst);
                        let src_space = self.pointer_address_space(src);
                        let mut read_effect = MemoryAccessEffect::read(
                            MemoryAccessLocation::from_pointer_with_size(
                                src,
                                src_type,
                                src_kind,
                                src_space,
                                size,
                                self.type_context.pointer_width_bits,
                            ),
                            false,
                        );
                        self.apply_pointer_location(&mut read_effect, src);
                        effects.push(read_effect);

                        let mut write_effect = MemoryAccessEffect::write(
                            MemoryAccessLocation::from_pointer_with_size(
                                dst,
                                dst_type,
                                dst_kind,
                                dst_space,
                                size,
                                self.type_context.pointer_width_bits,
                            ),
                            false,
                        );
                        self.apply_pointer_location(&mut write_effect, dst);
                        effects.push(write_effect);
                    }
                    _ => effects.push(MemoryAccessEffect::read_write(
                        MemoryAccessLocation::Unknown,
                        false,
                    )),
                }

                // return the effects
                effects
            }
            mir::Intrinsic::Memset => {
                // collect the memory operands
                let mut effects = SmallVec::new();
                let dst = args.first().and_then(|value| value.value());
                let size = args
                    .get(2)
                    .and_then(|len| len.value())
                    .and_then(|len| self.constant_u64(len));

                // emit write effects when operands are present
                match dst {
                    Some(dst) => {
                        let dst_type = self.pointer_access_type(dst);
                        let dst_kind = self.pointer_kind(dst);
                        let dst_space = self.pointer_address_space(dst);
                        let mut effect = MemoryAccessEffect::write(
                            MemoryAccessLocation::from_pointer_with_size(
                                dst,
                                dst_type,
                                dst_kind,
                                dst_space,
                                size,
                                self.type_context.pointer_width_bits,
                            ),
                            false,
                        );
                        self.apply_pointer_location(&mut effect, dst);
                        effects.push(effect);
                    }
                    None => effects.push(MemoryAccessEffect::write(
                        MemoryAccessLocation::Unknown,
                        false,
                    )),
                }

                // return the effects
                effects
            }
            mir::Intrinsic::Memcmp => {
                // collect the memory operands
                let mut effects = SmallVec::new();
                let left = args.first().and_then(|value| value.value());
                let right = args.get(1).and_then(|value| value.value());
                let size = args
                    .get(2)
                    .and_then(|len| len.value())
                    .and_then(|len| self.constant_u64(len));

                // emit read effects when operands are present
                match (left, right) {
                    (Some(left), Some(right)) => {
                        let left_type = self.pointer_access_type(left);
                        let right_type = self.pointer_access_type(right);
                        let left_kind = self.pointer_kind(left);
                        let right_kind = self.pointer_kind(right);
                        let left_space = self.pointer_address_space(left);
                        let right_space = self.pointer_address_space(right);
                        let mut left_effect = MemoryAccessEffect::read(
                            MemoryAccessLocation::from_pointer_with_size(
                                left,
                                left_type,
                                left_kind,
                                left_space,
                                size,
                                self.type_context.pointer_width_bits,
                            ),
                            false,
                        );
                        self.apply_pointer_location(&mut left_effect, left);
                        effects.push(left_effect);

                        let mut right_effect = MemoryAccessEffect::read(
                            MemoryAccessLocation::from_pointer_with_size(
                                right,
                                right_type,
                                right_kind,
                                right_space,
                                size,
                                self.type_context.pointer_width_bits,
                            ),
                            false,
                        );
                        self.apply_pointer_location(&mut right_effect, right);
                        effects.push(right_effect);
                    }
                    _ => effects.push(MemoryAccessEffect::read(
                        MemoryAccessLocation::Unknown,
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
                        let Some(pointer) = pointer.value() else {
                            effects.push(MemoryAccessEffect::read(
                                MemoryAccessLocation::Unknown,
                                false,
                            ));
                            return effects;
                        };

                        let access_type = self.pointer_access_type(pointer);
                        let pointer_kind = self.pointer_kind(pointer);
                        let pointer_space = self.pointer_address_space(pointer);
                        let mut effect = MemoryAccessEffect::read(
                            MemoryAccessLocation::from_pointer(
                                pointer,
                                access_type,
                                pointer_kind,
                                pointer_space,
                                self.type_context.pointer_width_bits,
                            ),
                            false,
                        );
                        self.apply_pointer_location(&mut effect, pointer);
                        effects.push(effect);
                    }
                    None => effects.push(MemoryAccessEffect::read(
                        MemoryAccessLocation::Unknown,
                        false,
                    )),
                }

                // return the effects
                effects
            }

            // type punning and pointer ops
            mir::Intrinsic::Transmute
            | mir::Intrinsic::AddressSpaceCast
            | mir::Intrinsic::PointerOffsetFrom
            | mir::Intrinsic::RawEq => SmallVec::new(),

            // garbage collection
            mir::Intrinsic::WriteBarrier => Self::single_effect(MemoryAccessEffect::read_write(
                MemoryAccessLocation::Unknown,
                false,
            )),

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
        let instruction_id = self.definitions.get(&value)?;
        let instruction = self.tree.get(*instruction_id);

        // extract integer constants only
        let mir::Instruction::Const { value, .. } = instruction else {
            return None;
        };

        match value {
            mir::Constant::UInt { value, .. } => Some(*value),
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

    /// Resolve the access type for a pointer value.
    fn pointer_access_type(&mut self, pointer: mir::Value) -> Option<TypeKey> {
        // reuse cached type keys
        let pointee_type = resolve_pointer_pointee_type(pointer, self.tree, &self.value_types)?;

        Some(self.type_key(pointee_type))
    }

    /// Resolve the reference kind for a pointer value.
    fn pointer_kind(&self, pointer: mir::Value) -> Option<mir::ReferenceKind> {
        resolve_pointer_kind(pointer, self.tree, &self.value_types)
    }

    /// Resolve the address space for a pointer value.
    fn pointer_address_space(&self, pointer: mir::Value) -> Option<mir::AddressSpace> {
        resolve_pointer_address_space(pointer, self.tree, &self.value_types)
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
    /// MIR node tree.
    tree: &'a mir::NodeTree,
    /// Entry block id.
    entry: mir::LocalNodeId<mir::Block>,
    /// Reachable block set.
    reachable: HashSet<mir::LocalNodeId<mir::Block>>,
    /// Dominator tree children.
    children: HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>>,
}

impl<'a> MemoryRenamer<'a> {
    /// Create a new renamer.
    fn new(
        tree: &'a mir::NodeTree,
        domtree: &'a DominatorTree,
        entry: mir::LocalNodeId<mir::Block>,
        reachable_blocks: &[mir::LocalNodeId<mir::Block>],
    ) -> Self {
        // build reachable set
        let reachable: HashSet<_> = reachable_blocks.iter().copied().collect();

        // build dominator tree children map
        let mut children = HashMap::new();
        for &block in reachable_blocks {
            children.insert(block, Vec::new());
        }
        for &block in reachable_blocks {
            if let Some(idom) = domtree.immediate_dominator(block)
                && let Some(list) = children.get_mut(&idom)
            {
                list.push(block);
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
        if let Some(phi_id) = ssa.block_phis.get(&block).copied() {
            stack.push(phi_id);
            pushed += 1;
        }

        // process block accesses
        if let Some(accesses) = ssa.block_accesses.get(&block).cloned() {
            for access_id in accesses {
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
        }

        // wire phi incoming edges for successors
        let block_data = self.tree.get(block);
        let terminator = self.tree.get(block_data.terminator);
        for successor in terminator.successors() {
            let Some(successor) = successor.block() else {
                continue;
            };

            if let Some(phi_id) = ssa.block_phis.get(&successor).copied()
                && let Some(MemoryAccess::Phi(phi)) = ssa.accesses.get_mut(phi_id.index())
            {
                let incoming = *stack.last().expect("missing memory definition");
                phi.incoming.push((block, incoming));
            }
        }

        // rename children
        if let Some(children) = self.children.get(&block) {
            for &child in children {
                self.rename_block(ssa, child, stack);
            }
        }

        // pop access stack for this block
        for _ in 0..pushed {
            stack.pop();
        }
    }
}

/// Determine whether a def access clobbers a query.
fn access_clobbers_query(
    def_access: &MemoryDef,
    query: &MemoryAccessQuery,
    alias: &crate::optimize::analyses::AliasAnalysis,
    tree: &mir::NodeTree,
) -> bool {
    // treat barriers as clobbering all memory
    if def_access.effect.is_barrier {
        return true;
    }

    // ignore non writing accesses
    if !def_access.effect.writes {
        return false;
    }

    // disambiguate by location sets
    if !space_sets_may_alias(def_access.effect.space_set, query.space_set) {
        return false;
    }

    // disambiguate by address spaces
    if !address_spaces_may_alias(&def_access.effect.address_spaces, &query.address_spaces) {
        return false;
    }

    // disambiguate by alias scopes and tbaa
    if !effects_may_alias(&def_access.effect, query, tree) {
        return false;
    }

    // check for local memory
    if let MemoryAccessLocation::Local(local) = &query.location {
        if let MemoryAccessLocation::Local(def_local) = &def_access.effect.location {
            return def_local == local;
        }

        return false;
    }

    // handle unknown memory spaces
    if matches!(query.location, MemoryAccessLocation::Unknown) {
        return def_access.effect.writes;
    }

    // resolve pointer based queries
    let Some(pointer_location) = query.location.as_pointer() else {
        return def_access.effect.writes;
    };

    // ignore local defs for pointer queries
    if let MemoryAccessLocation::Local(_) = def_access.effect.location {
        return false;
    }

    // compare pointer locations when available
    if let MemoryAccessLocation::Pointer(def_location) = &def_access.effect.location {
        return alias.alias(def_location, pointer_location).may_alias();
    }

    // fall back to mod ref for unknown spaces
    let mod_ref = alias.get_mod_ref_info_with_metadata(
        def_access.instruction,
        pointer_location,
        &query.alias_scopes,
        &query.noalias_scopes,
        query.type_alias_tag,
    );
    mod_ref.is_mod()
}

/// Check whether two access effects may alias.
fn effects_may_alias(
    def_effect: &MemoryAccessEffect,
    query: &MemoryAccessQuery,
    tree: &mir::NodeTree,
) -> bool {
    // check location sets
    if !space_sets_may_alias(def_effect.space_set, query.space_set) {
        return false;
    }

    // check address spaces
    if !address_spaces_may_alias(&def_effect.address_spaces, &query.address_spaces) {
        return false;
    }

    // check scoped noalias metadata
    if !alias_scopes_may_alias(
        &def_effect.alias_scopes,
        &def_effect.noalias_scopes,
        &query.alias_scopes,
        &query.noalias_scopes,
    ) {
        return false;
    }

    // check type-alias disambiguation
    if !type_alias_tags_may_alias(
        &tree.metadata.memory.type_alias,
        def_effect.type_alias_tag,
        query.type_alias_tag,
    ) {
        return false;
    }

    true
}

/// Determine whether a def access clobbers a location.
fn access_clobbers_location(
    def_access: &MemoryDef,
    location: &MemoryAccessLocation,
    alias: &crate::optimize::analyses::AliasAnalysis,
) -> bool {
    // treat barriers as clobbering all memory
    if def_access.effect.is_barrier {
        return true;
    }

    // ignore non writing accesses
    if !def_access.effect.writes {
        return false;
    }

    // disambiguate by location sets
    let query_space_set = space_set_for_location(location);
    if !space_sets_may_alias(def_access.effect.space_set, query_space_set) {
        return false;
    }

    // check for local memory
    if let MemoryAccessLocation::Local(local) = location {
        if let MemoryAccessLocation::Local(def_local) = &def_access.effect.location {
            return def_local == local;
        }

        return false;
    }

    // handle unknown memory spaces
    if matches!(location, MemoryAccessLocation::Unknown) {
        return def_access.effect.writes;
    }

    // check pointer based aliasing
    let Some(pointer_location) = location.as_pointer() else {
        return def_access.effect.writes;
    };

    if let MemoryAccessLocation::Local(_) = def_access.effect.location {
        return false;
    }

    let mod_ref = alias.get_mod_ref_info_with_metadata(
        def_access.instruction,
        pointer_location,
        &[],
        &[],
        None,
    );
    mod_ref.is_mod()
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
    use crate::optimize::analyses::AliasAnalysis;
    use crate::optimize::common::tests::TestProgram;

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

    /// Extract the pointer value from a location when available.
    fn pointer_from_location(location: &MemoryAccessLocation) -> Option<mir::Value> {
        // unwrap pointer backed spaces
        match location {
            MemoryAccessLocation::Pointer(location) => Some(location.ptr),
            _ => None,
        }
    }

    /// Extract the byte size from a location when available.
    fn size_from_location(location: &MemoryAccessLocation) -> Option<u64> {
        // unwrap pointer backed spaces
        match location {
            MemoryAccessLocation::Pointer(location) => location.size,
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
            .accesses_for_instruction(instruction)
            .map(|accesses| accesses.to_vec())
            .unwrap_or_default()
    }

    /// MemorySSA links uses to the latest defining access in a straight line.
    #[test]
    fn test_memory_ssa_linear_def_use() {
        let test = TestProgram::new(
            r#"
function test(v0: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>):
    v1: int32 = 1int32
    store v0, v1
    v2: int32 = load v0
    return v2
}"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let memory_ssa = memory_ssa.as_ref();

        // find memory accesses
        let block = test.tree.get(function.blocks[0]);
        let store_id = block.instructions[1];
        let load_id = block.instructions[2];

        let store_access = memory_ssa
            .access_for_instruction(store_id)
            .expect("missing store access");
        let load_access = memory_ssa
            .access_for_instruction(load_id)
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
function test(v0: ref<int32, raw>, v1: boolean): int32 {
b0(v0: ref<int32, raw>, v1: boolean):
    branch v1, b1, b2
b1:
    v2: int32 = 1int32
    store v0, v2
    jump b3
b2:
    v3: int32 = 2int32
    store v0, v3
    jump b3
b3:
    v4: int32 = load v0
    return v4
}"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let memory_ssa = memory_ssa.as_ref();

        // fetch join block phi
        let join_block = function.blocks[3];
        let phi_id = memory_ssa
            .phi_for_block(join_block)
            .expect("missing memory phi at join");

        // load should depend on the phi
        let load_inst = test.tree.get(join_block).instructions[0];
        let load_access = memory_ssa
            .access_for_instruction(load_inst)
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
    fn test_memory_ssa_clobber_skips_noalias_def() {
        let test = TestProgram::new(
            r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 1int32
    store v0, v2
    v3: int32 = 2int32
    store v1, v3
    v4: int32 = load v0
    return v4
}"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let alias = analyses.get::<AliasAnalysis>();

        // locate accesses
        let block = test.tree.get(function.blocks[0]);
        let store_v0 = block.instructions[3];
        let load_v0 = block.instructions[6];

        let store_access = memory_ssa
            .access_for_instruction(store_v0)
            .expect("missing store access");
        let load_access = memory_ssa
            .access_for_instruction(load_v0)
            .expect("missing load access");

        // clobbering access should be the store to v0
        let clobber = memory_ssa.clobbering_access_for_use(load_access, &alias, &test.tree);
        assert_eq!(clobber, store_access);
    }

    /// Memory semantics metadata marks effects as volatile and barrier when needed.
    #[test]
    fn test_memory_ssa_semantics_marks_effects() {
        let mut test = TestProgram::new(
            r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = load v0
    return v1
}"#,
        );

        let function_id = test.first_function_id();
        let instructions = test.entry_instructions(function_id);
        let load_inst = instructions[1];

        let semantics =
            mir::MemorySemantics::with_flags(mir::MemorySpaceSet::ANY, true, true, false);
        let access = mir::MemoryAccessMetadata {
            kind: mir::MemoryAccessKind::Read,
            target: mir::MemoryAccessTarget::Pointer(mir::Value::new(0)),
            size: Some(4),
            alignment: None,
            is_volatile: false,
            is_load_invariant: false,
            ordering: None,
            scope: Some(mir::AtomicScope::Device),
            memory_scope: Some(mir::MemoryScope::Device),
            semantics: Some(semantics),
            address_space: Some(mir::AddressSpace::Stack),
            alias_scopes: Vec::new(),
            noalias_scopes: Vec::new(),
            type_alias_tag: None,
        };
        test.insert_memory_accesses(load_inst, vec![access]);

        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();

        let access_id = memory_ssa
            .access_for_instruction(load_inst)
            .expect("missing load access");
        let effect = access_effect(memory_ssa.as_ref(), access_id);
        assert!(effect.is_volatile);
        assert!(effect.is_barrier);
    }

    /// MemorySSA uses alias scopes to ignore disjoint accesses.
    #[test]
    fn test_memory_ssa_clobber_skips_alias_scope() {
        let mut test = TestProgram::new(
            r#"
function test(v0: ref<int32, raw>, v1: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>, v1: ref<int32, raw>):
    v2: int32 = 1int32
    store v0, v2
    v3: int32 = 2int32
    store v1, v3
    v4: int32 = load v0
    return v4
}"#,
        );

        // create a scope for the disjoint access
        let scope = test.create_alias_scope();

        // locate store and load instructions
        let function_id = test.first_function_id();
        let instructions = test.entry_instructions(function_id);
        let store_v0 = instructions[1];
        let store_v1 = instructions[3];
        let load_v0 = instructions[4];

        // attach alias scope metadata to the store on v1
        test.insert_pointer_access(
            store_v1,
            mir::MemoryAccessKind::Write,
            mir::Value::new(1),
            Some(4),
            vec![scope],
            Vec::new(),
            None,
        );

        // attach noalias metadata to the load of v0
        test.insert_pointer_access(
            load_v0,
            mir::MemoryAccessKind::Read,
            mir::Value::new(0),
            Some(4),
            Vec::new(),
            vec![scope],
            None,
        );

        // build analyses
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let alias = analyses.get::<AliasAnalysis>();

        // locate memory accesses
        let load_access = memory_ssa
            .access_for_instruction(load_v0)
            .expect("missing load access");
        let store_access = memory_ssa
            .access_for_instruction(store_v0)
            .expect("missing store access");

        // clobber should skip the scoped store on v1
        let clobber = memory_ssa.clobbering_access_for_use(load_access, &alias, &test.tree);
        assert_eq!(clobber, store_access);
    }

    /// MemorySSA uses TBAA tags to ignore disjoint types.
    #[test]
    fn test_memory_ssa_clobber_skips_tbaa() {
        let mut test = TestProgram::new(
            r#"
function test(v0: ref<int32, raw>, v1: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>, v1: ref<int32, raw>):
    v2: int32 = 1int32
    store v0, v2
    v3: int32 = 2int32
    store v1, v3
    v4: int32 = load v0
    return v4
}"#,
        );

        // create disjoint tbaa tags
        let root = test.create_type_alias_node(None, false);
        let int_node = test.create_type_alias_node(Some(root), false);
        let float_node = test.create_type_alias_node(Some(root), false);
        let int_tag = test.create_type_alias_tag(root, int_node, 0, 4, false);
        let float_tag = test.create_type_alias_tag(root, float_node, 0, 4, false);

        // locate store and load instructions
        let function_id = test.first_function_id();
        let instructions = test.entry_instructions(function_id);
        let store_v0 = instructions[1];
        let store_v1 = instructions[3];
        let load_v0 = instructions[4];

        // tag store and load with disjoint tbaa metadata
        test.insert_pointer_access(
            store_v1,
            mir::MemoryAccessKind::Write,
            mir::Value::new(1),
            Some(4),
            Vec::new(),
            Vec::new(),
            Some(float_tag),
        );

        test.insert_pointer_access(
            load_v0,
            mir::MemoryAccessKind::Read,
            mir::Value::new(0),
            Some(4),
            Vec::new(),
            Vec::new(),
            Some(int_tag),
        );

        // build analyses
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let alias = analyses.get::<AliasAnalysis>();

        // locate memory accesses
        let load_access = memory_ssa
            .access_for_instruction(load_v0)
            .expect("missing load access");
        let store_access = memory_ssa
            .access_for_instruction(store_v0)
            .expect("missing store access");

        // clobber should skip the tbaa disjoint store
        let clobber = memory_ssa.clobbering_access_for_use(load_access, &alias, &test.tree);
        assert_eq!(clobber, store_access);
    }

    /// MemorySSA applies noalias metadata in either direction.
    #[test]
    fn test_memory_ssa_clobber_alias_scope_symmetry() {
        // input test
        let mut test = TestProgram::new(
            r#"
function test(v0: ref<int32, raw>, v1: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>, v1: ref<int32, raw>):
    v2: int32 = 1int32
    store v0, v2
    v3: int32 = 2int32
    store v1, v3
    v4: int32 = load v0
    return v4
}"#,
        );

        // create a scope for the disjoint access
        let scope = test.create_alias_scope();

        // locate store and load instructions
        let function_id = test.first_function_id();
        let instructions = test.entry_instructions(function_id);
        let store_v0 = instructions[1];
        let store_v1 = instructions[3];
        let load_v0 = instructions[4];

        // attach noalias metadata to the store on v1
        test.insert_pointer_access(
            store_v1,
            mir::MemoryAccessKind::Write,
            mir::Value::new(1),
            Some(4),
            Vec::new(),
            vec![scope],
            None,
        );

        // attach alias scope metadata to the load of v0
        test.insert_pointer_access(
            load_v0,
            mir::MemoryAccessKind::Read,
            mir::Value::new(0),
            Some(4),
            vec![scope],
            Vec::new(),
            None,
        );

        // build analyses
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let alias = analyses.get::<AliasAnalysis>();

        // locate memory accesses
        let load_access = memory_ssa
            .access_for_instruction(load_v0)
            .expect("missing load access");
        let store_access = memory_ssa
            .access_for_instruction(store_v0)
            .expect("missing store access");

        // clobber should skip the scoped store on v1
        let clobber = memory_ssa.clobbering_access_for_use(load_access, &alias, &test.tree);
        assert_eq!(clobber, store_access);
    }

    /// MemorySSA respects disjoint TBAA offsets.
    #[test]
    fn test_memory_ssa_clobber_tbaa_disjoint_offsets() {
        // input test
        let mut test = TestProgram::new(
            r#"
function test(v0: ref<int32, raw>, v1: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>, v1: ref<int32, raw>):
    v2: int32 = 1int32
    store v0, v2
    v3: int32 = 2int32
    store v1, v3
    v4: int32 = load v0
    return v4
}"#,
        );

        // create disjoint tbaa tags with the same base and access
        let root = test.create_type_alias_node(None, false);
        let access = test.create_type_alias_node(Some(root), false);
        let tag_a = test.create_type_alias_tag(root, access, 0, 4, false);
        let tag_b = test.create_type_alias_tag(root, access, 8, 4, false);

        // locate store and load instructions
        let function_id = test.first_function_id();
        let instructions = test.entry_instructions(function_id);
        let store_v0 = instructions[1];
        let store_v1 = instructions[3];
        let load_v0 = instructions[4];

        // tag store and load with disjoint tbaa metadata
        test.insert_pointer_access(
            store_v1,
            mir::MemoryAccessKind::Write,
            mir::Value::new(1),
            Some(4),
            Vec::new(),
            Vec::new(),
            Some(tag_b),
        );

        test.insert_pointer_access(
            load_v0,
            mir::MemoryAccessKind::Read,
            mir::Value::new(0),
            Some(4),
            Vec::new(),
            Vec::new(),
            Some(tag_a),
        );

        // build analyses
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let alias = analyses.get::<AliasAnalysis>();

        // locate memory accesses
        let load_access = memory_ssa
            .access_for_instruction(load_v0)
            .expect("missing load access");
        let store_access = memory_ssa
            .access_for_instruction(store_v0)
            .expect("missing store access");

        // clobber should skip the disjoint tbaa store
        let clobber = memory_ssa.clobbering_access_for_use(load_access, &alias, &test.tree);
        assert_eq!(clobber, store_access);
    }

    /// MemorySSA does not disambiguate overlapping TBAA offsets.
    #[test]
    fn test_memory_ssa_clobber_tbaa_overlap_offsets() {
        // input test
        let mut test = TestProgram::new(
            r#"
function test(v0: ref<int32, raw>, v1: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>, v1: ref<int32, raw>):
    v2: int32 = 1int32
    store v0, v2
    v3: int32 = 2int32
    store v1, v3
    v4: int32 = load v0
    return v4
}"#,
        );

        // create overlapping tbaa tags with the same base and access
        let root = test.create_type_alias_node(None, false);
        let access = test.create_type_alias_node(Some(root), false);
        let tag_a = test.create_type_alias_tag(root, access, 0, 8, false);
        let tag_b = test.create_type_alias_tag(root, access, 4, 8, false);

        // locate store and load instructions
        let function_id = test.first_function_id();
        let instructions = test.entry_instructions(function_id);
        let store_v1 = instructions[3];
        let load_v0 = instructions[4];

        // tag store and load with overlapping tbaa metadata
        test.insert_pointer_access(
            store_v1,
            mir::MemoryAccessKind::Write,
            mir::Value::new(1),
            Some(8),
            Vec::new(),
            Vec::new(),
            Some(tag_b),
        );

        test.insert_pointer_access(
            load_v0,
            mir::MemoryAccessKind::Read,
            mir::Value::new(0),
            Some(8),
            Vec::new(),
            Vec::new(),
            Some(tag_a),
        );

        // build analyses
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let alias = analyses.get::<AliasAnalysis>();

        // locate memory accesses
        let load_access = memory_ssa
            .access_for_instruction(load_v0)
            .expect("missing load access");
        let store_access = memory_ssa
            .access_for_instruction(store_v1)
            .expect("missing store access");

        // clobber should see the overlapping tbaa store
        let clobber = memory_ssa.clobbering_access_for_use(load_access, &alias, &test.tree);
        assert_eq!(clobber, store_access);
    }

    /// Alias scopes can disambiguate memory accesses.
    #[test]
    fn test_memory_ssa_alias_scopes_disambiguate() {
        let mut test = TestProgram::new(
            r#"
function test(v0: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>):
    v1: int32 = 1int32
    store v0, v1
    v2: int32 = load v0
    return v2
}"#,
        );

        let function_id = test.first_function_id();
        let instructions = test.entry_instructions(function_id);
        let store_inst = instructions[1];
        let load_inst = instructions[2];

        let scope = test.create_alias_scope();

        test.insert_pointer_access(
            store_inst,
            mir::MemoryAccessKind::Write,
            mir::Value::new(0),
            Some(4),
            vec![scope],
            Vec::new(),
            None,
        );

        test.insert_pointer_access(
            load_inst,
            mir::MemoryAccessKind::Read,
            mir::Value::new(0),
            Some(4),
            Vec::new(),
            vec![scope],
            None,
        );

        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let alias = analyses.get::<AliasAnalysis>();

        let load_access = memory_ssa
            .access_for_instruction(load_inst)
            .expect("missing load access");

        let clobber = memory_ssa.clobbering_access_for_use(load_access, &alias, &test.tree);
        assert_eq!(clobber, memory_ssa.live_on_entry());
    }

    /// Address spaces can disambiguate memory accesses.
    #[test]
    fn test_memory_ssa_address_space_disambiguate() {
        let mut test = TestProgram::new(
            r#"
function test(v0: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>):
    v1: int32 = 1int32
    store v0, v1
    v2: int32 = load v0
    return v2
}"#,
        );

        let function_id = test.first_function_id();
        let instructions = test.entry_instructions(function_id);
        let store_inst = instructions[1];
        let load_inst = instructions[2];

        let store_access = mir::MemoryAccessMetadata {
            kind: mir::MemoryAccessKind::Write,
            target: mir::MemoryAccessTarget::Pointer(mir::Value::new(0)),
            size: Some(4),
            alignment: None,
            is_volatile: false,
            is_load_invariant: false,
            ordering: None,
            scope: None,
            memory_scope: None,
            semantics: None,
            address_space: Some(mir::AddressSpace::Stack),
            alias_scopes: Vec::new(),
            noalias_scopes: Vec::new(),
            type_alias_tag: None,
        };
        let load_access = mir::MemoryAccessMetadata {
            kind: mir::MemoryAccessKind::Read,
            target: mir::MemoryAccessTarget::Pointer(mir::Value::new(0)),
            size: Some(4),
            alignment: None,
            is_volatile: false,
            is_load_invariant: false,
            ordering: None,
            scope: None,
            memory_scope: None,
            semantics: None,
            address_space: Some(mir::AddressSpace::Static),
            alias_scopes: Vec::new(),
            noalias_scopes: Vec::new(),
            type_alias_tag: None,
        };

        test.insert_memory_accesses(store_inst, vec![store_access]);
        test.insert_memory_accesses(load_inst, vec![load_access]);

        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let alias = analyses.get::<AliasAnalysis>();

        let load_access = memory_ssa
            .access_for_instruction(load_inst)
            .expect("missing load access");

        let clobber = memory_ssa.clobbering_access_for_use(load_access, &alias, &test.tree);
        assert_eq!(clobber, memory_ssa.live_on_entry());
    }

    /// MemorySSA respects explicit metadata over instruction semantics.
    #[test]
    fn test_memory_ssa_metadata_overrides_instruction() {
        // input test
        let mut test = TestProgram::new(
            r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 1int32
    store v0, v2
    v3: int32 = 2int32
    store v1, v3
    v4: int32 = load v0
    return v4
}"#,
        );

        // locate store and load instructions
        let function_id = test.first_function_id();
        let instructions = test.entry_instructions(function_id);
        let store_v1 = instructions[5];
        let load_v0 = instructions[6];

        // attach metadata that retargets the load to v1
        test.insert_pointer_access(
            load_v0,
            mir::MemoryAccessKind::Read,
            mir::Value::new(1),
            Some(4),
            Vec::new(),
            Vec::new(),
            None,
        );

        // build analyses
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let alias = analyses.get::<AliasAnalysis>();

        // locate memory accesses
        let load_access = memory_ssa
            .access_for_instruction(load_v0)
            .expect("missing load access");
        let store_access = memory_ssa
            .access_for_instruction(store_v1)
            .expect("missing store access");

        // clobber should follow the metadata target
        let clobber = memory_ssa.clobbering_access_for_use(load_access, &alias, &test.tree);
        assert_eq!(clobber, store_access);
    }

    /// Local accesses are tracked independently of pointer memory.
    #[test]
    fn test_memory_ssa_local_access() {
        let test = TestProgram::new(
            r#"
function test(): int32 {
    local local0: int32, owned
b0:
    v0: int32 = 7int32
    local.set local0, v0
    v1: int32 = local.get local0
    return v1
}"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();

        // locate local access
        let block = test.tree.get(function.blocks[0]);
        let store_inst = block.instructions[1];
        let load_inst = block.instructions[2];

        let store_access = memory_ssa
            .access_for_instruction(store_inst)
            .expect("missing local store access");
        let load_access = memory_ssa
            .access_for_instruction(load_inst)
            .expect("missing local load access");

        // local load should see the local set
        assert_eq!(memory_ssa.defining_access(load_access), Some(store_access));
    }

    /// Drop effects are modeled as unknown read and write accesses.
    #[test]
    fn test_memory_ssa_drop_effect_unknown() {
        let test = TestProgram::new(
            r#"
function test(v0: int32): int32 {
b0(v0: int32):
    drop v0
    v1: int32 = 0int32
    return v1
}"#,
        );

        let function_id = test.first_function_id();
        let instructions = test.entry_instructions(function_id);
        let drop_inst = instructions[0];

        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let memory_ssa = memory_ssa.as_ref();

        let drop_access = memory_ssa
            .access_for_instruction(drop_inst)
            .expect("missing drop access");
        let effect = access_effect(memory_ssa, drop_access);

        assert!(effect.reads);
        assert!(effect.writes);
        assert!(matches!(effect.location, MemoryAccessLocation::Unknown));
    }

    /// Allocation effects are modeled as unknown read and write accesses.
    #[test]
    fn test_memory_ssa_alloc_effect_unknown() {
        let test = TestProgram::new(
            r#"
type Point {
    int32;
}
function test(): ref<Point, managed> {
b0:
    v0: ref<Point, managed> = new Point
    return v0
}"#,
        );

        let function_id = test.first_function_id();
        let instructions = test.entry_instructions(function_id);
        let alloc_inst = instructions[0];

        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let memory_ssa = memory_ssa.as_ref();

        let alloc_access = memory_ssa
            .access_for_instruction(alloc_inst)
            .expect("missing alloc access");
        let effect = access_effect(memory_ssa, alloc_access);

        assert!(effect.reads);
        assert!(effect.writes);
        assert!(matches!(effect.location, MemoryAccessLocation::Unknown));
    }

    /// Memcpy produces a read followed by a write access for its operands.
    #[test]
    fn test_memory_ssa_memcpy_read_write_effects() {
        let test = TestProgram::new(
            r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int64 = 4int64
    intrinsic.memcpy(v0, v1, v2)
    v3: int32 = load v0
    return v3
}"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
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

        let read_ptr = pointer_from_location(&read_effect.location).expect("missing read pointer");
        let write_ptr =
            pointer_from_location(&write_effect.location).expect("missing write pointer");

        let stack_allocs = test.stack_alloc_destinations_in_entry(function_id);
        let dest_value = *stack_allocs.first().expect("missing stack allocation");
        let src_value = *stack_allocs.get(1).expect("missing stack allocation");

        assert_eq!(read_ptr, src_value);
        assert_eq!(write_ptr, dest_value);

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
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int64 = 4int64
    v3: int32 = intrinsic.memcmp(v0, v1, v2)
    return v3
}"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
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
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = load v0
    store v0, v1
    return v1
}"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);

        // locate volatile instructions
        let block = test.tree.get(function.blocks[0]);
        let volatile_load = block.instructions[1];
        let volatile_store = block.instructions[2];

        // attach volatile memory access metadata
        test.insert_pointer_access_with_options(
            volatile_load,
            mir::MemoryAccessKind::Read,
            mir::Value::new(0),
            Some(4),
            Vec::new(),
            Vec::new(),
            None,
            true,
            None,
        );
        test.insert_pointer_access_with_options(
            volatile_store,
            mir::MemoryAccessKind::Write,
            mir::Value::new(0),
            Some(4),
            Vec::new(),
            Vec::new(),
            None,
            true,
            None,
        );

        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let memory_ssa = memory_ssa.as_ref();

        // collect volatile effects
        let load_access = memory_ssa
            .access_for_instruction(volatile_load)
            .expect("missing volatile load access");
        let store_access = memory_ssa
            .access_for_instruction(volatile_store)
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
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = atomic.load v0, acquire, device, device, any
    atomic.store v0, v1, release, device, device, any
    return v1
}"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let memory_ssa = memory_ssa.as_ref();

        let block = test.tree.get(function.blocks[0]);
        let atomic_load = block.instructions[1];
        let atomic_store = block.instructions[2];

        let load_access = memory_ssa
            .access_for_instruction(atomic_load)
            .expect("missing atomic load access");
        let store_access = memory_ssa
            .access_for_instruction(atomic_store)
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
b0:
    atomic.fence seq_cst, device, device, any
    v0: int32 = 0int32
    return v0
}"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let memory_ssa = memory_ssa.as_ref();

        // locate fence instruction
        let block = test.tree.get(function.blocks[0]);
        let fence_inst = block.instructions[0];
        let fence_access = memory_ssa
            .access_for_instruction(fence_inst)
            .expect("missing fence access");

        let fence_effect = access_effect(memory_ssa, fence_access);
        assert!(fence_effect.is_barrier);
        assert!(matches!(
            fence_effect.location,
            MemoryAccessLocation::Unknown
        ));
    }

    /// Calls are modeled as unknown read write effects.
    #[test]
    fn test_memory_ssa_call_is_unknown_def() {
        let test = TestProgram::new(
            r#"
extern function external(ref<int32, raw>): void
function test(v0: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>):
    call external(v0): (ref<int32, raw>) -> void
    v1: int32 = 0int32
    return v1
}"#,
        );

        // select the defined function
        let function_id = test
            .tree
            .iter_nodes::<mir::Function>()
            .find(|(_, function)| function.entry.is_some())
            .expect("missing function")
            .0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let memory_ssa = memory_ssa.as_ref();

        // locate call instruction
        let block = test.tree.get(function.blocks[0]);
        let call_inst = block.instructions[0];
        let call_access = memory_ssa
            .access_for_instruction(call_inst)
            .expect("missing call access");

        let call_effect = access_effect(memory_ssa, call_access);
        assert!(call_effect.reads);
        assert!(call_effect.writes);
        assert!(matches!(
            call_effect.location,
            MemoryAccessLocation::Unknown
        ));
    }

    /// Call metadata readnone suppresses memory accesses.
    #[test]
    fn test_memory_ssa_call_readnone_metadata() {
        let mut test = TestProgram::new(
            r#"
extern function external(ref<int32, raw>): void
function test(v0: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>):
    call external(v0): (ref<int32, raw>) -> void
    v1: int32 = 0int32
    return v1
}"#,
        );

        let function_id = test.entry_function_id();
        let (call_inst, _callee) = test.first_call_in_entry(function_id);
        let instruction = test.tree.get_mut(call_inst);
        let Some(memory_effect) = instruction.call_memory_effect_mut() else {
            panic!("expected call instruction");
        };
        *memory_effect = Some(mir::MemoryEffect::none());

        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();

        assert!(
            memory_ssa.accesses_for_instruction(call_inst).is_none(),
            "readnone calls should not create memory accesses"
        );
    }

    /// Call metadata argmemonly reads are modeled as pointer uses.
    #[test]
    fn test_memory_ssa_call_argmemonly_reads() {
        let mut test = TestProgram::new(
            r#"
extern function external(ref<int32, raw>, int32): void
function test(v0: ref<int32, raw>, v1: int32): int32 {
b0(v0: ref<int32, raw>, v1: int32):
    call external(v0, v1): (ref<int32, raw>, int32) -> void
    v2: int32 = 0int32
    return v2
}"#,
        );

        let function_id = test.entry_function_id();
        let param_value = {
            let function = test.tree.get(function_id);
            function.parameters[0]
                .value
                .value()
                .expect("parameter value should be concrete")
        };
        let (call_inst, _callee) = test.first_call_in_entry(function_id);

        let arg0 = mir::ArgumentAttribute {
            access: mir::ArgumentAccess::Read,
            ..Default::default()
        };
        let arg1 = mir::ArgumentAttribute::default();

        let instruction = test.tree.get_mut(call_inst);
        let mir::Instruction::Call { call, .. } = instruction else {
            panic!("expected call instruction");
        };

        call.memory_effect =
            Some(mir::MemoryEffect::read_only(mir::MemorySpaceSet::NONE).with_argmemonly());
        call.argument_attributes = vec![arg0, arg1];

        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let memory_ssa = memory_ssa.as_ref();

        let accesses = instruction_accesses(memory_ssa, call_inst);
        assert_eq!(accesses.len(), 1);

        let effect = access_effect(memory_ssa, accesses[0]);
        assert!(effect.reads);
        assert!(!effect.writes);

        let read_ptr = pointer_from_location(&effect.location).expect("missing read pointer");
        assert_eq!(read_ptr, param_value);
    }

    /// Memory access metadata overrides default instruction effects.
    #[test]
    fn test_memory_ssa_access_metadata_override() {
        // build the test test
        let mut test = TestProgram::new(
            r#"
extern function external(ref<int32, raw>, ref<int32, raw>): void
function test(v0: ref<int32, raw>, v1: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>, v1: ref<int32, raw>):
    call external(v0, v1): (ref<int32, raw>, ref<int32, raw>) -> void
    v2: int32 = 0int32
    return v2
}"#,
        );

        // collect parameter values and the call instruction
        let function_id = test.entry_function_id();
        let param_values = {
            let function = test.tree.get(function_id);
            function
                .parameters
                .iter()
                .map(|param| {
                    param
                        .value
                        .value()
                        .expect("parameter value should be concrete")
                })
                .collect::<Vec<_>>()
        };
        let (call_inst, _callee) = test.first_call_in_entry(function_id);

        // build explicit access metadata
        let read_access = mir::MemoryAccessMetadata {
            kind: mir::MemoryAccessKind::Read,
            target: mir::MemoryAccessTarget::Pointer(param_values[0]),
            size: Some(4),
            alignment: None,
            is_volatile: false,
            is_load_invariant: false,
            ordering: None,
            scope: None,
            memory_scope: None,
            semantics: None,
            address_space: None,
            alias_scopes: Vec::new(),
            noalias_scopes: Vec::new(),
            type_alias_tag: None,
        };
        let write_access = mir::MemoryAccessMetadata {
            kind: mir::MemoryAccessKind::Write,
            target: mir::MemoryAccessTarget::Pointer(param_values[1]),
            size: Some(4),
            alignment: None,
            is_volatile: false,
            is_load_invariant: false,
            ordering: None,
            scope: None,
            memory_scope: None,
            semantics: None,
            address_space: None,
            alias_scopes: Vec::new(),
            noalias_scopes: Vec::new(),
            type_alias_tag: None,
        };

        // attach memory access metadata to the call
        test.insert_memory_accesses(call_inst, vec![read_access, write_access]);

        // build analyses
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
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

        // verify pointers and sizes
        let read_ptr = pointer_from_location(&read_effect.location).expect("missing read pointer");
        let write_ptr =
            pointer_from_location(&write_effect.location).expect("missing write pointer");
        assert_eq!(
            read_ptr,
            function.parameters[0]
                .value
                .value()
                .expect("first parameter value should be concrete")
        );
        assert_eq!(
            write_ptr,
            function.parameters[1]
                .value
                .value()
                .expect("second parameter value should be concrete")
        );

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
function test(v0: ref<int32, raw>, v1: int32): int32 {
b0(v0: ref<int32, raw>, v1: int32):
    v2: int32 = 0int32
    store v0, v2
    jump b1(v2)
b1(v3: int32):
    v4: boolean = int.lt.s v3, v1
    branch v4, b2, b3
b2:
    v5: int32 = 1int32
    store v0, v5
    v6: int32 = int.add v3, v5
    jump b1(v6)
b3:
    v7: int32 = load v0
    return v7
}"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let memory_ssa = memory_ssa.as_ref();

        // find phi for loop header
        let header_block = function.blocks[1];
        let phi_id = memory_ssa
            .phi_for_block(header_block)
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
b0:
    v0: int32 = 0int32
    return v0
b1:
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 1int32
    store v1, v2
    return v2
}"#,
        );

        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let memory_ssa = memory_ssa.as_ref();

        // locate store in unreachable block
        let unreachable_block = function.blocks[1];
        let block = test.tree.get(unreachable_block);
        let store_inst = block.instructions[2];

        assert!(
            memory_ssa.accesses_for_instruction(store_inst).is_none(),
            "unreachable store should not be tracked"
        );
    }
}
