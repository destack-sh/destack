use std::collections::{HashMap, HashSet, VecDeque};

use destack_mir as mir;
use smallvec::SmallVec;

use crate::optimize::common::{
    MemoryLocation, TypeKey, build_value_definition_map, collect_reachable_blocks,
    compute_dominance_frontiers, resolve_pointer_pointee_type,
};
use crate::optimize::{
    Analysis, AnalysisId, ControlFlowGraph, DominatorTree, FunctionAnalyses, FunctionAnalysis,
};

use super::OwnershipAnalysis;

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
    /// Pointer-based memory location.
    Pointer(MemoryLocation),
    /// Local slot access.
    Local(mir::LocalNodeId<mir::Local>),
    /// Unknown memory location.
    Unknown,
}

impl MemoryAccessLocation {
    /// Create a pointer location with optional access type and inferred size.
    fn from_pointer(ptr: mir::Value, access_type: Option<TypeKey>) -> Self {
        Self::from_pointer_with_size(ptr, access_type, None)
    }

    /// Create a pointer location with an explicit size override.
    fn from_pointer_with_size(
        ptr: mir::Value,
        access_type: Option<TypeKey>,
        size: Option<u64>,
    ) -> Self {
        let inferred_size = size.or_else(|| access_type.as_ref().and_then(TypeKey::byte_size));
        MemoryAccessLocation::Pointer(MemoryLocation::new(ptr, inferred_size, access_type))
    }

    /// Return a pointer location if available.
    fn as_pointer(&self) -> Option<&MemoryLocation> {
        // unwrap pointer-backed locations
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
    /// Live-on-entry access id.
    live_on_entry: MemoryAccessId,
}

impl MemorySSA {
    /// Build MemorySSA for a function.
    fn build(
        function: &mir::Function,
        tree: &mir::NodeTree,
        cfg: &ControlFlowGraph,
        domtree: &DominatorTree,
        ownership: &OwnershipAnalysis,
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
        let mut access_collector = MemoryAccessCollector::new(function, tree, ownership);
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

        // create memory access table with live-on-entry
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

    /// Return the live-on-entry access id.
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
    ) -> MemoryAccessId {
        // read the memory use location
        let MemoryAccess::Use(use_access_data) = self.access(use_access) else {
            panic!("expected memory use access");
        };

        // resolve the defining access
        let defining_access = use_access_data
            .defining_access
            .expect("memory use missing defining access");

        // compute the clobbering access
        let mut cache = HashMap::new();
        let mut visiting = HashSet::new();
        self.clobbering_access(
            defining_access,
            &use_access_data.effect.location,
            alias,
            &mut cache,
            &mut visiting,
        )
    }

    /// Compute the clobbering access for a memory location.
    fn clobbering_access(
        &self,
        access_id: MemoryAccessId,
        location: &MemoryAccessLocation,
        alias: &crate::optimize::analyses::AliasAnalysis,
        cache: &mut HashMap<(MemoryAccessId, MemoryAccessLocation), MemoryAccessId>,
        visiting: &mut HashSet<MemoryAccessId>,
    ) -> MemoryAccessId {
        // consult the cache
        if let Some(cached) = cache.get(&(access_id, location.clone())) {
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
                self.clobbering_access(defining_access, location, alias, cache, visiting)
            }

            MemoryAccess::Def(def_access) => {
                if access_clobbers_location(def_access, location, alias) {
                    access_id
                } else {
                    let defining_access = def_access
                        .defining_access
                        .expect("memory def missing defining access");
                    self.clobbering_access(defining_access, location, alias, cache, visiting)
                }
            }

            MemoryAccess::Phi(phi) => {
                let mut incoming_clobber: Option<MemoryAccessId> = None;

                for (_, incoming) in &phi.incoming {
                    let clobber =
                        self.clobbering_access(*incoming, location, alias, cache, visiting);
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
        cache.insert((access_id, location.clone()), result);
        result
    }
}

impl Analysis for MemorySSA {
    const ID: AnalysisId = AnalysisId("memory-ssa");
    const DEPENDENCIES: &'static [AnalysisId] = &[
        DominatorTree::ID,
        OwnershipAnalysis::ID,
        ControlFlowGraph::ID,
    ];
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
        let ownership = analyses.get::<OwnershipAnalysis>();

        Self::build(function, tree, &cfg, &domtree, &ownership)
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
    /// Ownership analysis for value type inference.
    ownership: &'a OwnershipAnalysis,
    /// Map from value to defining instruction.
    definitions: HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    /// Type key cache.
    type_keys: HashMap<mir::LocalNodeId<mir::Type>, TypeKey>,
}

impl<'a> MemoryAccessCollector<'a> {
    /// Create a new collector.
    fn new(
        function: &'a mir::Function,
        tree: &'a mir::NodeTree,
        ownership: &'a OwnershipAnalysis,
    ) -> Self {
        // collect value definitions for pointer resolution
        let definitions = build_value_definition_map(function, tree);

        // build collector state
        Self {
            function,
            tree,
            ownership,
            definitions,
            type_keys: HashMap::new(),
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
                let effects = self.instruction_effects(instruction);
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
        instruction: &mir::Instruction,
    ) -> SmallVec<[MemoryAccessEffect; 2]> {
        // classify instruction memory effects
        match instruction {
            // pure instructions
            mir::Instruction::Const { .. }
            | mir::Instruction::Binary { .. }
            | mir::Instruction::Unary { .. }
            | mir::Instruction::Cast { .. }
            | mir::Instruction::Select { .. }
            | mir::Instruction::GlobalAddr { .. }
            | mir::Instruction::GlobalConst { .. }
            | mir::Instruction::FieldGet { .. }
            | mir::Instruction::FieldAddr { .. }
            | mir::Instruction::FieldSet { .. }
            | mir::Instruction::ElementGet { .. }
            | mir::Instruction::ElementAddr { .. }
            | mir::Instruction::ElementSet { .. }
            | mir::Instruction::Struct { .. }
            | mir::Instruction::Tuple { .. }
            | mir::Instruction::Array { .. }
            | mir::Instruction::Assume { .. } => SmallVec::new(),
            mir::Instruction::Load { pointer, .. } => {
                let access_type = self.pointer_access_type(*pointer);
                Self::single_effect(MemoryAccessEffect::read(
                    MemoryAccessLocation::from_pointer(*pointer, access_type),
                    false,
                ))
            }
            mir::Instruction::Store { pointer, .. } => {
                let access_type = self.pointer_access_type(*pointer);
                Self::single_effect(MemoryAccessEffect::write(
                    MemoryAccessLocation::from_pointer(*pointer, access_type),
                    false,
                ))
            }
            mir::Instruction::LocalGet { local, .. } => Self::single_effect(
                MemoryAccessEffect::read(MemoryAccessLocation::Local(*local), false),
            ),
            mir::Instruction::LocalSet { local, .. } => Self::single_effect(
                MemoryAccessEffect::write(MemoryAccessLocation::Local(*local), false),
            ),
            mir::Instruction::Call { .. } | mir::Instruction::CallIndirect { .. } => {
                Self::single_effect(MemoryAccessEffect::read_write(
                    MemoryAccessLocation::Unknown,
                    false,
                ))
            }
            mir::Instruction::RawFree { pointer } => {
                let access_type = self.pointer_access_type(*pointer);
                Self::single_effect(MemoryAccessEffect::write(
                    MemoryAccessLocation::from_pointer(*pointer, access_type),
                    false,
                ))
            }
            mir::Instruction::RawDrop { value } | mir::Instruction::StackDrop { value } => {
                let access_type = self.pointer_access_type(*value);
                Self::single_effect(MemoryAccessEffect::write(
                    MemoryAccessLocation::from_pointer(*value, access_type),
                    false,
                ))
            }
            mir::Instruction::ManagedAlloc { destination, .. }
            | mir::Instruction::ManagedAllocArray { destination, .. }
            | mir::Instruction::RawAlloc { destination, .. }
            | mir::Instruction::StackAlloc { destination, .. } => {
                let access_type = self.pointer_access_type(*destination);
                Self::single_effect(MemoryAccessEffect::write(
                    MemoryAccessLocation::from_pointer(*destination, access_type),
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
            mir::Intrinsic::Clz
            | mir::Intrinsic::Ctz
            | mir::Intrinsic::Popcnt
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
                let dst = args.get(0).copied();
                let src = args.get(1).copied();
                let size = args.get(2).and_then(|len| self.constant_u64(*len));

                // emit read and write effects when operands are present
                match (dst, src) {
                    (Some(dst), Some(src)) => {
                        let dst_type = self.pointer_access_type(dst);
                        let src_type = self.pointer_access_type(src);
                        effects.push(MemoryAccessEffect::read(
                            MemoryAccessLocation::from_pointer_with_size(src, src_type, size),
                            false,
                        ));
                        effects.push(MemoryAccessEffect::write(
                            MemoryAccessLocation::from_pointer_with_size(dst, dst_type, size),
                            false,
                        ));
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
                let dst = args.get(0).copied();
                let size = args.get(2).and_then(|len| self.constant_u64(*len));

                // emit write effects when operands are present
                match dst {
                    Some(dst) => {
                        let dst_type = self.pointer_access_type(dst);
                        effects.push(MemoryAccessEffect::write(
                            MemoryAccessLocation::from_pointer_with_size(dst, dst_type, size),
                            false,
                        ));
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
                let left = args.get(0).copied();
                let right = args.get(1).copied();
                let size = args.get(2).and_then(|len| self.constant_u64(*len));

                // emit read effects when operands are present
                match (left, right) {
                    (Some(left), Some(right)) => {
                        let left_type = self.pointer_access_type(left);
                        let right_type = self.pointer_access_type(right);
                        effects.push(MemoryAccessEffect::read(
                            MemoryAccessLocation::from_pointer_with_size(left, left_type, size),
                            false,
                        ));
                        effects.push(MemoryAccessEffect::read(
                            MemoryAccessLocation::from_pointer_with_size(right, right_type, size),
                            false,
                        ));
                    }
                    _ => effects.push(MemoryAccessEffect::read(
                        MemoryAccessLocation::Unknown,
                        false,
                    )),
                }

                // return the effects
                effects
            }
            mir::Intrinsic::VolatileLoad => {
                // collect the memory operands
                let mut effects = SmallVec::new();
                let pointer = args.get(0).copied();

                // emit read effects when operands are present
                match pointer {
                    Some(pointer) => {
                        let access_type = self.pointer_access_type(pointer);
                        effects.push(MemoryAccessEffect::read(
                            MemoryAccessLocation::from_pointer(pointer, access_type),
                            true,
                        ));
                    }
                    None => effects.push(MemoryAccessEffect::read(
                        MemoryAccessLocation::Unknown,
                        true,
                    )),
                }

                // return the effects
                effects
            }
            mir::Intrinsic::VolatileStore => {
                // collect the memory operands
                let mut effects = SmallVec::new();
                let pointer = args.get(0).copied();

                // emit write effects when operands are present
                match pointer {
                    Some(pointer) => {
                        let access_type = self.pointer_access_type(pointer);
                        effects.push(MemoryAccessEffect::write(
                            MemoryAccessLocation::from_pointer(pointer, access_type),
                            true,
                        ));
                    }
                    None => effects.push(MemoryAccessEffect::write(
                        MemoryAccessLocation::Unknown,
                        true,
                    )),
                }

                // return the effects
                effects
            }
            mir::Intrinsic::PrefetchRead | mir::Intrinsic::PrefetchWrite => {
                // collect the memory operands
                let mut effects = SmallVec::new();
                let pointer = args.get(0).copied();

                // emit read effects when operands are present
                match pointer {
                    Some(pointer) => {
                        let access_type = self.pointer_access_type(pointer);
                        effects.push(MemoryAccessEffect::read(
                            MemoryAccessLocation::from_pointer(pointer, access_type),
                            false,
                        ));
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
            | mir::Intrinsic::AddrSpaceCast
            | mir::Intrinsic::PtrOffsetFrom
            | mir::Intrinsic::RawEq => SmallVec::new(),

            // garbage collection
            mir::Intrinsic::GcWriteBarrier => Self::single_effect(MemoryAccessEffect::read_write(
                MemoryAccessLocation::Unknown,
                false,
            )),

            // atomics
            mir::Intrinsic::AtomicLoad => {
                // collect the memory operands
                let mut effects = SmallVec::new();
                let pointer = args.get(0).copied();

                // emit read effects when operands are present
                match pointer {
                    Some(pointer) => {
                        let access_type = self.pointer_access_type(pointer);
                        effects.push(MemoryAccessEffect::read(
                            MemoryAccessLocation::from_pointer(pointer, access_type),
                            false,
                        ));
                    }
                    None => effects.push(MemoryAccessEffect::read(
                        MemoryAccessLocation::Unknown,
                        false,
                    )),
                }

                // return the effects
                effects
            }
            mir::Intrinsic::AtomicStore => {
                // collect the memory operands
                let mut effects = SmallVec::new();
                let pointer = args.get(0).copied();

                // emit write effects when operands are present
                match pointer {
                    Some(pointer) => {
                        let access_type = self.pointer_access_type(pointer);
                        effects.push(MemoryAccessEffect::write(
                            MemoryAccessLocation::from_pointer(pointer, access_type),
                            false,
                        ));
                    }
                    None => effects.push(MemoryAccessEffect::write(
                        MemoryAccessLocation::Unknown,
                        false,
                    )),
                }

                // return the effects
                effects
            }
            mir::Intrinsic::AtomicCas
            | mir::Intrinsic::AtomicFetchAdd
            | mir::Intrinsic::AtomicFetchSub
            | mir::Intrinsic::AtomicFetchAnd
            | mir::Intrinsic::AtomicFetchOr
            | mir::Intrinsic::AtomicFetchXor
            | mir::Intrinsic::AtomicFetchMin
            | mir::Intrinsic::AtomicFetchMax => {
                // collect the memory operands
                let mut effects = SmallVec::new();
                let pointer = args.get(0).copied();

                // emit read-write effects when operands are present
                match pointer {
                    Some(pointer) => {
                        let access_type = self.pointer_access_type(pointer);
                        effects.push(MemoryAccessEffect::read_write(
                            MemoryAccessLocation::from_pointer(pointer, access_type),
                            false,
                        ));
                    }
                    None => effects.push(MemoryAccessEffect::read_write(
                        MemoryAccessLocation::Unknown,
                        false,
                    )),
                }

                // return the effects
                effects
            }
            mir::Intrinsic::AtomicFence => Self::single_effect(MemoryAccessEffect::barrier()),

            // float math
            mir::Intrinsic::Sqrt
            | mir::Intrinsic::Abs
            | mir::Intrinsic::Fma
            | mir::Intrinsic::Copysign
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
            mir::Intrinsic::Unreachable
            | mir::Intrinsic::Breakpoint
            | mir::Intrinsic::Abort
            | mir::Intrinsic::ReturnAddress
            | mir::Intrinsic::FrameAddress
            | mir::Intrinsic::Expect
            | mir::Intrinsic::Likely
            | mir::Intrinsic::Unlikely
            | mir::Intrinsic::BlackBox => SmallVec::new(),

            // simd
            mir::Intrinsic::Shuffle
            | mir::Intrinsic::Select
            | mir::Intrinsic::Splat
            | mir::Intrinsic::ReduceAdd
            | mir::Intrinsic::ReduceMul
            | mir::Intrinsic::ReduceMin
            | mir::Intrinsic::ReduceMax
            | mir::Intrinsic::ReduceAnd
            | mir::Intrinsic::ReduceOr
            | mir::Intrinsic::ReduceXor => SmallVec::new(),
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
        let Some(pointee_type) = resolve_pointer_pointee_type(
            pointer,
            self.function,
            self.tree,
            self.ownership,
            &self.definitions,
        ) else {
            return None;
        };

        Some(self.type_key(pointee_type))
    }

    /// Get or compute a type key.
    fn type_key(&mut self, ty_id: mir::LocalNodeId<mir::Type>) -> TypeKey {
        // reuse cached key when available
        if let Some(existing) = self.type_keys.get(&ty_id) {
            return existing.clone();
        }

        // build and cache the new key
        let ty = self.tree.get(ty_id);
        let key = TypeKey::from_type(ty, self.tree);
        self.type_keys.insert(ty_id, key.clone());
        key
    }
}

/// MemorySSA renamer for def-use chains.
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
            if let Some(idom) = domtree.immediate_dominator(block) {
                if let Some(list) = children.get_mut(&idom) {
                    list.push(block);
                }
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
        // initialize stack with live-on-entry
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
        for successor in block_data.terminator.successors() {
            if let Some(phi_id) = ssa.block_phis.get(&successor).copied() {
                if let Some(MemoryAccess::Phi(phi)) = ssa.accesses.get_mut(phi_id.index()) {
                    let incoming = *stack.last().expect("missing memory definition");
                    phi.incoming.push((block, incoming));
                }
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

    // ignore non-writing accesses
    if !def_access.effect.writes {
        return false;
    }

    // check for local memory
    if let MemoryAccessLocation::Local(local) = location {
        if let MemoryAccessLocation::Local(def_local) = &def_access.effect.location {
            return def_local == local;
        }

        return false;
    }

    // handle unknown memory locations
    if matches!(location, MemoryAccessLocation::Unknown) {
        return def_access.effect.writes;
    }

    // check pointer-based aliasing
    let Some(pointer_location) = location.as_pointer() else {
        return def_access.effect.writes;
    };

    if let MemoryAccessLocation::Local(_) = def_access.effect.location {
        return false;
    }

    let mod_ref = alias.get_mod_ref_info(def_access.instruction, pointer_location);
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

            if phi_blocks.insert(df_block) {
                if !def_blocks.contains(&df_block) && !in_worklist.contains(&df_block) {
                    worklist.push_back(df_block);
                    in_worklist.insert(df_block);
                }
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
        // unwrap pointer-backed locations
        match location {
            MemoryAccessLocation::Pointer(location) => Some(location.ptr),
            _ => None,
        }
    }

    /// Extract the byte size from a location when available.
    fn size_from_location(location: &MemoryAccessLocation) -> Option<u64> {
        // unwrap pointer-backed locations
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
        let program = TestProgram::new(
            r#"function @test(v0: ref<raw mut i32>) -> i32 {
block0(v0: ref<raw mut i32>):
    v1 = iconst 1i32
    store v0, v1
    v2 = load v0
    return v2
}"#,
        );

        let function_id = program.first_function_id();
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let memory_ssa = memory_ssa.as_ref();

        // find memory accesses
        let block = program.tree.get(function.blocks[0]);
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

        // store should depend on live-on-entry
        assert_eq!(
            memory_ssa.defining_access(store_access),
            Some(memory_ssa.live_on_entry())
        );
    }

    /// MemorySSA inserts phis at join points with multiple incoming defs.
    #[test]
    fn test_memory_ssa_phi_at_join() {
        let program = TestProgram::new(
            r#"function @test(v0: ref<raw mut i32>, v1: bool) -> i32 {
block0(v0: ref<raw mut i32>, v1: bool):
    branch v1, block1, block2
block1:
    v2 = iconst 1i32
    store v0, v2
    jump block3
block2:
    v3 = iconst 2i32
    store v0, v3
    jump block3
block3:
    v4 = load v0
    return v4
}"#,
        );

        let function_id = program.first_function_id();
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let memory_ssa = memory_ssa.as_ref();

        // fetch join block phi
        let join_block = function.blocks[3];
        let phi_id = memory_ssa
            .phi_for_block(join_block)
            .expect("missing memory phi at join");

        // load should depend on the phi
        let load_inst = program.tree.get(join_block).instructions[0];
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

    /// MemorySSA uses alias analysis to skip non-aliasing defs.
    #[test]
    fn test_memory_ssa_clobber_skips_noalias_def() {
        let program = TestProgram::new(
            r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = stack.alloc i32
    v2 = iconst 1i32
    store v0, v2
    v3 = iconst 2i32
    store v1, v3
    v4 = load v0
    return v4
}"#,
        );

        let function_id = program.first_function_id();
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let alias = analyses.get::<AliasAnalysis>();

        // locate accesses
        let block = program.tree.get(function.blocks[0]);
        let store_v0 = block.instructions[3];
        let load_v0 = block.instructions[6];

        let store_access = memory_ssa
            .access_for_instruction(store_v0)
            .expect("missing store access");
        let load_access = memory_ssa
            .access_for_instruction(load_v0)
            .expect("missing load access");

        // clobbering access should be the store to v0
        let clobber = memory_ssa.clobbering_access_for_use(load_access, &alias);
        assert_eq!(clobber, store_access);
    }

    /// Local accesses are tracked independently of pointer memory.
    #[test]
    fn test_memory_ssa_local_access() {
        let program = TestProgram::new(
            r#"function @test() -> i32 {
    local0: i32
block0:
    v0 = iconst 7i32
    local.set local0, v0
    v1 = local.get local0
    return v1
}"#,
        );

        let function_id = program.first_function_id();
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();

        // locate local access
        let block = program.tree.get(function.blocks[0]);
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

    /// Memcpy produces a read followed by a write access for its operands.
    #[test]
    fn test_memory_ssa_memcpy_read_write_effects() {
        let program = TestProgram::new(
            r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = stack.alloc i32
    v2 = iconst 4i64
    intrinsic.memcpy(v0, v1, v2)
    v3 = load v0
    return v3
}"#,
        );

        let function_id = program.first_function_id();
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let memory_ssa = memory_ssa.as_ref();

        // locate memcpy instruction
        let block = program.tree.get(function.blocks[0]);
        let memcpy_inst = block.instructions[3];
        let accesses = instruction_accesses(memory_ssa, memcpy_inst);

        // ensure we recorded a read and a write
        assert_eq!(accesses.len(), 2);

        // extract effects in order
        let read_effect = access_effect(memory_ssa, accesses[0]);
        let write_effect = access_effect(memory_ssa, accesses[1]);

        // confirm read then write ordering
        assert!(read_effect.reads);
        assert!(!read_effect.writes);
        assert!(write_effect.writes);
        assert!(!write_effect.reads);

        // confirm pointer locations match operands
        let read_ptr = pointer_from_location(&read_effect.location).expect("missing read pointer");
        let write_ptr =
            pointer_from_location(&write_effect.location).expect("missing write pointer");

        let dest_value = match program.tree.get(block.instructions[0]) {
            mir::Instruction::StackAlloc { destination, .. } => *destination,
            _ => panic!("expected stack allocation"),
        };
        let src_value = match program.tree.get(block.instructions[1]) {
            mir::Instruction::StackAlloc { destination, .. } => *destination,
            _ => panic!("expected stack allocation"),
        };

        assert_eq!(read_ptr, src_value);
        assert_eq!(write_ptr, dest_value);

        // confirm constant byte sizes are attached
        let read_size = size_from_location(&read_effect.location).expect("missing read size");
        let write_size = size_from_location(&write_effect.location).expect("missing write size");

        assert_eq!(read_size, 4);
        assert_eq!(write_size, 4);
    }

    /// Memcmp produces two read accesses for its operands.
    #[test]
    fn test_memory_ssa_memcmp_read_effects() {
        let program = TestProgram::new(
            r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = stack.alloc i32
    v2 = iconst 4i64
    v3 = intrinsic.memcmp(v0, v1, v2)
    return v3
}"#,
        );

        let function_id = program.first_function_id();
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let memory_ssa = memory_ssa.as_ref();

        // locate memcmp instruction
        let block = program.tree.get(function.blocks[0]);
        let memcmp_inst = block.instructions[3];
        let accesses = instruction_accesses(memory_ssa, memcmp_inst);

        // ensure we recorded two reads
        assert_eq!(accesses.len(), 2);

        // confirm each access is read-only
        for access_id in accesses {
            let effect = access_effect(memory_ssa, access_id);
            assert!(effect.reads);
            assert!(!effect.writes);
        }
    }

    /// Volatile accesses are marked as volatile effects.
    #[test]
    fn test_memory_ssa_volatile_marks_effects() {
        let program = TestProgram::new(
            r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = intrinsic.volatile.load(v0)
    intrinsic.volatile.store(v0, v1)
    return v1
}"#,
        );

        let function_id = program.first_function_id();
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let memory_ssa = memory_ssa.as_ref();

        // locate volatile instructions
        let block = program.tree.get(function.blocks[0]);
        let volatile_load = block.instructions[1];
        let volatile_store = block.instructions[2];

        // collect volatile effects
        let load_access = memory_ssa
            .access_for_instruction(volatile_load)
            .expect("missing volatile load access");
        let store_access = memory_ssa
            .access_for_instruction(volatile_store)
            .expect("missing volatile store access");

        // confirm volatility flags
        let load_effect = access_effect(memory_ssa, load_access);
        let store_effect = access_effect(memory_ssa, store_access);

        assert!(load_effect.is_volatile);
        assert!(store_effect.is_volatile);
    }

    /// Atomic fence produces a barrier access.
    #[test]
    fn test_memory_ssa_atomic_fence_barrier() {
        let program = TestProgram::new(
            r#"function @test() -> i32 {
block0:
    intrinsic.atomic.fence(seq_cst)
    v0 = iconst 0i32
    return v0
}"#,
        );

        let function_id = program.first_function_id();
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let memory_ssa = memory_ssa.as_ref();

        // locate fence instruction
        let block = program.tree.get(function.blocks[0]);
        let fence_inst = block.instructions[0];
        let fence_access = memory_ssa
            .access_for_instruction(fence_inst)
            .expect("missing fence access");

        // confirm barrier classification
        let fence_effect = access_effect(memory_ssa, fence_access);
        assert!(fence_effect.is_barrier);
        assert!(matches!(
            fence_effect.location,
            MemoryAccessLocation::Unknown
        ));
    }

    /// Calls are modeled as unknown read-write effects.
    #[test]
    fn test_memory_ssa_call_is_unknown_def() {
        let program = TestProgram::new(
            r#"extern function @external(ref<raw i32>) -> void
function @test(v0: ref<raw i32>) -> i32 {
block0(v0: ref<raw i32>):
    call @external(v0)
    v1 = iconst 0i32
    return v1
}"#,
        );

        // select the defined function
        let function_id = program
            .tree
            .iter_nodes::<mir::Function>()
            .find(|(_, function)| function.entry.is_some())
            .expect("missing function")
            .0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let memory_ssa = memory_ssa.as_ref();

        // locate call instruction
        let block = program.tree.get(function.blocks[0]);
        let call_inst = block.instructions[0];
        let call_access = memory_ssa
            .access_for_instruction(call_inst)
            .expect("missing call access");

        // confirm unknown read-write classification
        let call_effect = access_effect(memory_ssa, call_access);
        assert!(call_effect.reads);
        assert!(call_effect.writes);
        assert!(matches!(
            call_effect.location,
            MemoryAccessLocation::Unknown
        ));
    }

    /// Loop headers get memory phis when defs flow around the backedge.
    #[test]
    fn test_memory_ssa_loop_phi_in_header() {
        let program = TestProgram::new(
            r#"function @test(v0: ref<raw mut i32>, v1: i32) -> i32 {
block0(v0: ref<raw mut i32>, v1: i32):
    v2 = iconst 0i32
    store v0, v2
    jump block1(v2)
block1(v3: i32):
    v4 = icmp_slt v3, v1
    branch v4, block2, block3
block2:
    v5 = iconst 1i32
    store v0, v5
    v6 = iadd v3, v5
    jump block1(v6)
block3:
    v7 = load v0
    return v7
}"#,
        );

        let function_id = program.first_function_id();
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
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

        // confirm incoming edges include preheader and backedge
        let incoming_blocks: HashSet<_> = phi.incoming.iter().map(|(block, _)| *block).collect();
        let body_block = function.blocks[2];

        assert!(incoming_blocks.contains(&function.blocks[0]));
        assert!(incoming_blocks.contains(&body_block));
    }

    /// Unreachable blocks do not contribute memory accesses.
    #[test]
    fn test_memory_ssa_ignores_unreachable_blocks() {
        let program = TestProgram::new(
            r#"function @test() -> i32 {
block0:
    v0 = iconst 0i32
    return v0
block1:
    v1 = stack.alloc i32
    v2 = iconst 1i32
    store v1, v2
    return v2
}"#,
        );

        let function_id = program.first_function_id();
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let memory_ssa = analyses.get::<MemorySSA>();
        let memory_ssa = memory_ssa.as_ref();

        // locate store in unreachable block
        let unreachable_block = function.blocks[1];
        let block = program.tree.get(unreachable_block);
        let store_inst = block.instructions[2];

        // confirm unreachable store is not tracked
        assert!(
            memory_ssa.accesses_for_instruction(store_inst).is_none(),
            "unreachable store should not be tracked"
        );
    }
}
