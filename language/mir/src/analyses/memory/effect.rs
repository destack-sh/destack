use std::ops::Range;

use smallvec::{SmallVec, smallvec};

use crate as mir;
use crate::{
    AliasTable, Analysis, ConstantTable, ControlTable, LayoutError, MemoryAccessOrder,
    MemoryAddress, MemoryLocation, MemoryRegion, Mutation, NodeTable,
};

/// Classified memory effects for the operations in one function.
#[derive(Debug)]
pub struct MemoryEffectTable {
    /// Memory effects in execution order within each reachable block.
    effects: Vec<MemoryAccessEffect>,
    /// Effect ranges indexed by instruction.
    instructions: NodeTable<mir::Instruction, Range<u32>>,
    /// Effect ranges indexed by block terminator.
    terminators: NodeTable<mir::Block, Range<u32>>,
}

impl MemoryEffectTable {
    /// Classify the operations in each reachable block.
    pub fn analyse(
        function: mir::FunctionId,
        constants: &ConstantTable,
        control: &ControlTable,
        effects: &mir::EffectTable,
        tree: &mut mir::Tree,
    ) -> Self {
        // allocate entries for every operation in the function
        let instructions = tree
            .get(function)
            .blocks()
            .iter()
            .flat_map(|block| tree.get(*block).instructions.iter().copied())
            .collect::<Vec<_>>();
        let mut result = Self {
            effects: Vec::new(),
            instructions: NodeTable::from_nodes(&instructions, || 0..0),
            terminators: NodeTable::from_nodes(tree.get(function).blocks(), || 0..0),
        };

        // preserve empty results for declarations without bodies
        let Some(_) = tree.get(function).entry() else {
            return result;
        };

        // classify reachable operations using their explicit and derived effects
        let mut builder = MemoryEffectBuilder::new(function, tree, constants, effects);
        for block_id in control.reachable_blocks() {
            let block = builder.tree.get(block_id).clone();

            // retain each instruction's observable memory accesses
            for &instruction_id in &block.instructions {
                let instruction = &builder.tree.get(instruction_id).clone();
                let effects = builder.instruction_effects(instruction_id, instruction);
                let start = result.effects.len() as u32;
                result.effects.extend(
                    effects
                        .into_iter()
                        .filter(|effect| effect.reads || effect.writes || effect.is_barrier),
                );
                let end = result.effects.len() as u32;
                *result.instructions.get_mut(instruction_id) = start..end;
            }

            // retain the terminator's observable memory accesses
            let terminator = &builder.tree.get(block.terminator).clone();
            let effects = builder.terminator_effects(block_id, terminator);
            let start = result.effects.len() as u32;
            result.effects.extend(
                effects
                    .into_iter()
                    .filter(|effect| effect.reads || effect.writes || effect.is_barrier),
            );
            let end = result.effects.len() as u32;
            *result.terminators.get_mut(block_id) = start..end;
        }

        result
    }

    /// Return whether an instruction's execution or explicit accesses can have observable effects.
    pub fn has_side_effects(
        &self,
        instruction: mir::LocalNodeId<mir::Instruction>,
        tree: &mir::Tree,
    ) -> bool {
        tree.get(instruction).has_side_effects()
            || self.instruction_effects(instruction).any(|effect| {
                effect.writes
                    || effect.is_barrier
                    || !matches!(effect.order, MemoryAccessOrder::Plain)
            })
    }

    /// Iterate the memory effects of one instruction.
    pub fn instruction_effects(
        &self,
        instruction: mir::LocalNodeId<mir::Instruction>,
    ) -> impl Iterator<Item = &MemoryAccessEffect> {
        let range = self.instructions.get(instruction);

        self.effects[range.start as usize..range.end as usize].iter()
    }

    /// Iterate the memory effects of one block terminator.
    pub fn terminator_effects(
        &self,
        block: mir::LocalNodeId<mir::Block>,
    ) -> impl Iterator<Item = &MemoryAccessEffect> {
        let range = self.terminators.get(block);

        self.effects[range.start as usize..range.end as usize].iter()
    }
}

impl Analysis for MemoryEffectTable {
    const INVALIDATED_BY: Mutation = Mutation::CONTROL
        .union(Mutation::VALUE)
        .union(Mutation::MEMORY)
        .union(Mutation::EFFECT)
        .union(Mutation::LAYOUT);
}

/// Memory access effects for an instruction.
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryAccessEffect {
    /// Whether the instruction reads memory.
    pub reads: bool,
    /// Whether the instruction writes memory.
    pub writes: bool,
    /// The strongest ordering required by this access.
    pub order: MemoryAccessOrder,
    /// Whether the instruction acts as a memory barrier.
    pub is_barrier: bool,
    /// The memory region being accessed.
    pub region: MemoryRegion,
}

impl MemoryAccessEffect {
    /// Return whether this effect may clobber one memory location.
    pub fn clobbers_location(
        &self,
        location: &MemoryLocation,
        alias: &AliasTable,
        tree: &mir::Tree,
    ) -> Result<bool, LayoutError> {
        let region = MemoryRegion::Address {
            location: location.clone(),
            spaces: location.spaces(tree),
        };

        self.clobbers_region(&region, alias)
    }

    /// Return whether this effect may touch one memory location.
    pub fn may_touch_location(
        &self,
        location: &MemoryLocation,
        alias: &AliasTable,
        tree: &mir::Tree,
    ) -> Result<bool, LayoutError> {
        let region = MemoryRegion::Address {
            location: location.clone(),
            spaces: location.spaces(tree),
        };

        self.region.may_alias(&region, alias)
    }

    /// Return whether this effect is trackable by memory optimizations.
    pub fn is_trackable(&self) -> bool {
        matches!(self.order, MemoryAccessOrder::Plain)
            && !self.is_barrier
            && !matches!(self.region, MemoryRegion::Any { .. })
    }

    /// Return whether this effect describes the same region as another effect.
    pub fn matches_region(
        &self,
        alias: &AliasTable,
        other: &MemoryAccessEffect,
    ) -> Result<bool, LayoutError> {
        // reject disjoint memory spaces
        if !self.region.spaces().may_alias(other.region.spaces()) {
            return Ok(false);
        }

        // compare concrete locations
        Ok(match (&self.region, &other.region) {
            (MemoryRegion::Local(local), MemoryRegion::Local(other_local)) => local == other_local,
            (
                MemoryRegion::Address { location, .. },
                MemoryRegion::Address {
                    location: other_location,
                    ..
                },
            ) => {
                if !location.has_compatible_value(other_location, &alias.layouts)? {
                    return Ok(false);
                }

                alias.alias(location, other_location)?.is_must_alias()
            }
            _ => false,
        })
    }

    /// Return whether this effect may clobber a queried region.
    pub fn clobbers_region(
        &self,
        region: &MemoryRegion,
        alias: &AliasTable,
    ) -> Result<bool, LayoutError> {
        // preserve memory ordering independently of location disambiguation
        if self.is_barrier || !matches!(self.order, MemoryAccessOrder::Plain) {
            return Ok(true);
        }

        Ok(self.writes && self.region.may_alias(region, alias)?)
    }

    /// Return whether this effect may alias another effect.
    pub fn may_alias(
        &self,
        alias: &AliasTable,
        other: &MemoryAccessEffect,
    ) -> Result<bool, LayoutError> {
        self.region.may_alias(&other.region, alias)
    }

    /// Create a memory access with its operation and ordering.
    fn new(
        region: MemoryRegion,
        operation: mir::MemoryOperation,
        order: MemoryAccessOrder,
    ) -> Self {
        Self {
            reads: matches!(
                operation,
                mir::MemoryOperation::Read | mir::MemoryOperation::ReadWrite
            ),
            writes: matches!(
                operation,
                mir::MemoryOperation::Write | mir::MemoryOperation::ReadWrite
            ),
            order,
            is_barrier: false,
            region,
        }
    }

    /// Create a barrier access effect.
    fn barrier(access: mir::FenceAccess) -> Self {
        Self {
            reads: false,
            writes: false,
            order: MemoryAccessOrder::Atomic(mir::AtomicAccess::new(access.ordering, access.scope)),
            is_barrier: true,
            region: MemoryRegion::Any {
                spaces: access.storage,
            },
        }
    }
}

/// Collector for memory accesses.
struct MemoryEffectBuilder<'a> {
    /// MIR function under analysis.
    function: mir::FunctionId,
    /// MIR tree.
    tree: &'a mut mir::Tree,
    /// Constants available for memory ranges.
    constants: &'a ConstantTable,

    /// Explicit effect table.
    effect_table: &'a mir::EffectTable,
}

impl<'a> MemoryEffectBuilder<'a> {
    /// Create a new collector.
    fn new(
        function: mir::FunctionId,
        tree: &'a mut mir::Tree,
        constants: &'a ConstantTable,
        effect_table: &'a mir::EffectTable,
    ) -> Self {
        // build collector state
        Self {
            function,
            tree,
            constants,
            effect_table,
        }
    }

    /// Return memory effects for one instruction.
    fn instruction_effects(
        &mut self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
    ) -> SmallVec<[MemoryAccessEffect; 2]> {
        let mut effects = SmallVec::new();

        // read reference storage before following each dereference
        if let Some(place) = instruction.place() {
            let mut prefix = mir::Place::new(place.origin);
            for projection in &place.path.projections {
                if *projection == mir::Projection::Deref
                    && !(matches!(prefix.origin, mir::PlaceOrigin::Value(_))
                        && prefix.path.is_root())
                {
                    effects.push(self.place_effect(
                        &prefix,
                        mir::MemoryOperation::Read,
                        MemoryAccessOrder::Plain,
                    ));
                }
                prefix.push(projection.clone());
            }
        }

        // classify instruction memory effects
        let accesses: SmallVec<[MemoryAccessEffect; 2]> = match instruction {
            mir::Instruction::Error => {
                unreachable!("invalid MIR instruction reached memory effect analysis");
            }

            // omit instructions that access no memory
            mir::Instruction::Copy { .. }
            | mir::Instruction::Const { .. }
            | mir::Instruction::Binary { .. }
            | mir::Instruction::Unary { .. }
            | mir::Instruction::Cast { .. }
            | mir::Instruction::Select { .. }
            | mir::Instruction::Address { .. }
            | mir::Instruction::FunctionAddr { .. }
            | mir::Instruction::FunctionBind { .. }
            | mir::Instruction::FunctionEnvironment { .. }
            | mir::Instruction::FunctionEnvironmentCurrent { .. }
            | mir::Instruction::ContextGet { .. }
            | mir::Instruction::Aggregate { .. }
            | mir::Instruction::FieldGet { .. }
            | mir::Instruction::FieldSet { .. }
            | mir::Instruction::ElementGet { .. }
            | mir::Instruction::ElementSet { .. }
            | mir::Instruction::VariantNew { .. }
            | mir::Instruction::VariantTag { .. }
            | mir::Instruction::VariantPayload { .. }
            | mir::Instruction::SliceLength { .. }
            | mir::Instruction::DynamicBind { .. }
            | mir::Instruction::DynamicPayload { .. }
            | mir::Instruction::DynamicType { .. }
            | mir::Instruction::DynamicFind { .. }
            | mir::Instruction::VectorSplat { .. }
            | mir::Instruction::VectorExtract { .. }
            | mir::Instruction::VectorInsert { .. }
            | mir::Instruction::VectorShuffle { .. }
            | mir::Instruction::VectorSelect { .. }
            | mir::Instruction::VectorReduce { .. }
            | mir::Instruction::VectorCompare { .. }
            | mir::Instruction::VectorConvert { .. }
            | mir::Instruction::NewComplete { .. }
            | mir::Instruction::Assume { .. }
            | mir::Instruction::ProfileIncrement { .. }
            | mir::Instruction::ProfileSample { .. }
            | mir::Instruction::Breakpoint => SmallVec::new(),
            mir::Instruction::Load { place, copy, .. } => {
                // consuming a value invalidates its source storage
                let operation = match copy {
                    mir::Copy::Yes => mir::MemoryOperation::Read,
                    mir::Copy::No => mir::MemoryOperation::ReadWrite,
                };
                let effect = self.place_effect(place, operation, MemoryAccessOrder::Plain);

                smallvec![effect]
            }
            mir::Instruction::VariantTagLoad { place, .. } => {
                let effect =
                    self.place_effect(place, mir::MemoryOperation::Read, MemoryAccessOrder::Plain);

                smallvec![effect]
            }
            mir::Instruction::DynamicRead {
                dynamic,
                slot,
                result_type,
                ..
            } => {
                let reference_kind = self.reference_kind(*dynamic);
                let reference_storage = self.reference_storage(*dynamic);
                let mut region = MemoryRegion::from_address(
                    MemoryAddress::Dynamic {
                        value: *dynamic,
                        slot: *slot,
                    },
                    Some(*result_type),
                    reference_kind,
                    reference_storage,
                );
                region.set_spaces(self.address_storage_set(*dynamic));

                smallvec![MemoryAccessEffect::new(
                    region,
                    mir::MemoryOperation::Read,
                    MemoryAccessOrder::Plain
                )]
            }
            mir::Instruction::Store { place, .. } => {
                let effect =
                    self.place_effect(place, mir::MemoryOperation::Write, MemoryAccessOrder::Plain);

                smallvec![effect]
            }
            mir::Instruction::AtomicLoad { place, access, .. } => {
                let effect = self.place_effect(
                    place,
                    mir::MemoryOperation::Read,
                    MemoryAccessOrder::Atomic(*access),
                );

                smallvec![effect]
            }
            mir::Instruction::AtomicStore { place, access, .. } => {
                let effect = self.place_effect(
                    place,
                    mir::MemoryOperation::Write,
                    MemoryAccessOrder::Atomic(*access),
                );

                smallvec![effect]
            }
            mir::Instruction::AtomicCompareExchange { place, access, .. } => {
                let effect = self.place_effect(
                    place,
                    mir::MemoryOperation::ReadWrite,
                    MemoryAccessOrder::Atomic(access.success),
                );

                smallvec![effect]
            }
            mir::Instruction::AtomicRmw { place, access, .. } => {
                let effect = self.place_effect(
                    place,
                    mir::MemoryOperation::ReadWrite,
                    MemoryAccessOrder::Atomic(*access),
                );

                smallvec![effect]
            }
            mir::Instruction::AtomicFence { access } => {
                smallvec![MemoryAccessEffect::barrier(*access)]
            }
            mir::Instruction::BarrierWrite { .. } => {
                smallvec![MemoryAccessEffect::new(
                    MemoryRegion::any_spaces(mir::StorageSet::ANY),
                    mir::MemoryOperation::ReadWrite,
                    MemoryAccessOrder::Plain
                )]
            }
            mir::Instruction::Call { .. } => self.call_effects(instruction_id, instruction),
            mir::Instruction::ContextCurrent { .. }
            | mir::Instruction::ContextReplace { .. }
            | mir::Instruction::ContextBind { .. }
            | mir::Instruction::Release { .. }
            | mir::Instruction::Drop { .. }
            | mir::Instruction::NewZeroed { .. }
            | mir::Instruction::NewUninit { .. }
            | mir::Instruction::NewSliceZeroed { .. }
            | mir::Instruction::NewSliceUninit { .. }
            | mir::Instruction::Poll => smallvec![MemoryAccessEffect::new(
                MemoryRegion::any_spaces(mir::StorageSet::ANY),
                mir::MemoryOperation::ReadWrite,
                MemoryAccessOrder::Plain
            )],
            mir::Instruction::Intrinsic {
                intrinsic,
                arguments,
                ..
            } => self.intrinsic_effects(*intrinsic, *arguments),
        };

        effects.extend(accesses);

        effects
    }

    /// Return memory effects for one block terminator.
    fn terminator_effects(
        &mut self,
        block_id: mir::LocalNodeId<mir::Block>,
        terminator: &mir::Terminator,
    ) -> SmallVec<[MemoryAccessEffect; 2]> {
        match terminator {
            mir::Terminator::Invoke { .. } | mir::Terminator::TailCall { .. } => self
                .callsite_effects(
                    mir::Point::Terminator(block_id),
                    terminator.call_direct_target(),
                ),

            mir::Terminator::NewZeroedTry { .. }
            | mir::Terminator::NewUninitTry { .. }
            | mir::Terminator::NewSliceZeroedTry { .. }
            | mir::Terminator::NewSliceUninitTry { .. } => {
                smallvec![MemoryAccessEffect::new(
                    MemoryRegion::any_spaces(mir::StorageSet::ANY),
                    mir::MemoryOperation::ReadWrite,
                    MemoryAccessOrder::Plain
                )]
            }

            mir::Terminator::Error => {
                unreachable!("invalid MIR terminator reached memory effect analysis");
            }

            mir::Terminator::Return { .. }
            | mir::Terminator::Jump { .. }
            | mir::Terminator::Branch { .. }
            | mir::Terminator::Check { .. }
            | mir::Terminator::Switch { .. }
            | mir::Terminator::VariantSwitch { .. }
            | mir::Terminator::Panic { .. }
            | mir::Terminator::UnwindResume
            | mir::Terminator::Abort { .. }
            | mir::Terminator::Unreachable => SmallVec::new(),
        }
    }

    /// Build a memory effect over explicitly selected storage.
    fn place_effect(
        &mut self,
        place: &mir::Place,
        operation: mir::MemoryOperation,
        order: MemoryAccessOrder,
    ) -> MemoryAccessEffect {
        // retain direct locals in the local memory flow
        if let mir::PlaceOrigin::Local(local) = place.origin
            && place.path.is_root()
        {
            return MemoryAccessEffect::new(MemoryRegion::Local(local), operation, order);
        }

        let value_type = match place.ty(self.function, self.tree) {
            Some(mir::PlaceType::Value(ty)) => Some(ty),
            Some(mir::PlaceType::Sequence(_)) => None,
            None => unreachable!("memory operation has an invalid place"),
        };
        let storage = place.storage(self.function, self.tree);
        let kind = place
            .reference_type(self.function, self.tree)
            .and_then(|ty| self.tree.type_definition(ty).reference_kind());
        let mut region = MemoryRegion::from_address(place.clone(), value_type, kind, storage);
        region.set_spaces(storage.map_or(mir::StorageSet::ANY, |storage| {
            storage.storage_set(self.tree)
        }));

        MemoryAccessEffect::new(region, operation, order)
    }

    /// Build one memory effect over an address-bearing value.
    fn address_effect(
        &mut self,
        address: mir::Value,
        operation: mir::MemoryOperation,
        order: MemoryAccessOrder,
    ) -> MemoryAccessEffect {
        let value_type = self.address_value_type(address);
        let reference_kind = self.reference_kind(address);
        let reference_storage = self.reference_storage(address);
        let region =
            MemoryRegion::from_address(address, value_type, reference_kind, reference_storage);
        let mut effect = MemoryAccessEffect::new(region, operation, order);

        // constrain the region to the address storage
        let spaces = self.address_storage_set(address);
        effect.region.set_spaces(spaces);

        effect
    }

    /// Resolve the memory spaces for an address-bearing value.
    fn address_storage_set(&mut self, address: mir::Value) -> mir::StorageSet {
        let Some(storage) = self.reference_storage(address) else {
            return mir::StorageSet::ANY;
        };

        storage.storage_set(self.tree)
    }

    /// Return memory effects for one call instruction.
    fn call_effects(
        &mut self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
    ) -> SmallVec<[MemoryAccessEffect; 2]> {
        self.callsite_effects(
            mir::Point::Instruction(instruction_id),
            instruction.call_direct_target(),
        )
    }

    /// Return memory effects for one callsite.
    fn callsite_effects(
        &self,
        callsite: mir::Point,
        direct_target: Option<mir::LocalNodeId<mir::Function>>,
    ) -> SmallVec<[MemoryAccessEffect; 2]> {
        // use callsite or callee tables for memory effects
        let call_entries = self.effect_table.call(callsite);
        let mut memory_effects = call_entries
            .and_then(|tables| tables.memory.clone())
            .or_else(|| self.callee_memory_effects(direct_target));

        // treat missing tables as fully unknown
        let Some(effects) = memory_effects.take() else {
            return smallvec![MemoryAccessEffect::new(
                MemoryRegion::any_spaces(mir::StorageSet::ANY),
                mir::MemoryOperation::ReadWrite,
                MemoryAccessOrder::Plain
            )];
        };

        // skip calls with no memory effects
        if !effects.reads() && !effects.writes() {
            return SmallVec::new();
        }

        // skip empty storage
        if effects.storage().is_empty() {
            return SmallVec::new();
        }

        self.effects_from_call_effect(&effects)
    }

    /// Convert call memory effects to MemorySsaTable effects.
    fn effects_from_call_effect(
        &self,
        effects: &mir::MemoryEffect,
    ) -> SmallVec<[MemoryAccessEffect; 2]> {
        let mut accesses = SmallVec::new();

        // keep shared read/write spaces precise when possible
        if effects.read == effects.write {
            accesses.push(MemoryAccessEffect::new(
                MemoryRegion::any_spaces(effects.read),
                mir::MemoryOperation::ReadWrite,
                MemoryAccessOrder::Plain,
            ));
        } else {
            if !effects.read.is_empty() {
                accesses.push(MemoryAccessEffect::new(
                    MemoryRegion::any_spaces(effects.read),
                    mir::MemoryOperation::Read,
                    MemoryAccessOrder::Plain,
                ));
            }

            // append the spaces the call may write
            if !effects.write.is_empty() {
                accesses.push(MemoryAccessEffect::new(
                    MemoryRegion::any_spaces(effects.write),
                    mir::MemoryOperation::Write,
                    MemoryAccessOrder::Plain,
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

    /// Return memory effects for one intrinsic.
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
            | mir::Intrinsic::RotateRight
            | mir::Intrinsic::IsolateLowestOne => SmallVec::new(),

            // numeric operations
            mir::Intrinsic::Midpoint
            | mir::Intrinsic::Clamp
            | mir::Intrinsic::DivideCeil
            | mir::Intrinsic::RemainderEuclidean
            | mir::Intrinsic::IsMultipleOf
            | mir::Intrinsic::AbsDiff => SmallVec::new(),

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

            // classify byte copies and comparisons using their runtime length operand
            mir::Intrinsic::Memcpy | mir::Intrinsic::Memmove | mir::Intrinsic::Memcmp => {
                let [left, right, length] = *args else {
                    unreachable!("bulk memory operation requires three MIR arguments");
                };
                let size = self.constant_u64(length);
                let left_operation = if intrinsic == mir::Intrinsic::Memcmp {
                    mir::MemoryOperation::Read
                } else {
                    mir::MemoryOperation::Write
                };
                let left = self.byte_effect(left, left_operation, size);
                let right = self.byte_effect(right, mir::MemoryOperation::Read, size);

                // preserve operand order for comparisons and destination order for copies
                if intrinsic == mir::Intrinsic::Memcmp {
                    smallvec![left, right]
                } else {
                    smallvec![right, left]
                }
            }
            mir::Intrinsic::Memset => {
                let [destination, _, length] = args else {
                    unreachable!("memory set requires three MIR arguments");
                };
                let size = self.constant_u64(*length);
                let effect = self.byte_effect(*destination, mir::MemoryOperation::Write, size);

                smallvec![effect]
            }
            mir::Intrinsic::PrefetchRead | mir::Intrinsic::PrefetchWrite => {
                let [pointer] = args else {
                    unreachable!("memory prefetch requires one MIR argument");
                };
                let effect = self.address_effect(
                    *pointer,
                    mir::MemoryOperation::Read,
                    MemoryAccessOrder::Plain,
                );

                smallvec![effect]
            }
            mir::Intrinsic::VolatileLoad | mir::Intrinsic::VolatileStore => {
                let (pointer, operation) = match (intrinsic, args) {
                    (mir::Intrinsic::VolatileLoad, [pointer]) => {
                        (*pointer, mir::MemoryOperation::Read)
                    }
                    (mir::Intrinsic::VolatileStore, [pointer, _]) => {
                        (*pointer, mir::MemoryOperation::Write)
                    }
                    _ => unreachable!("volatile memory operation has invalid MIR arguments"),
                };
                let effect = self.address_effect(pointer, operation, MemoryAccessOrder::Volatile);

                smallvec![effect]
            }

            // omit representation intrinsics
            mir::Intrinsic::Transmute
            | mir::Intrinsic::SpaceCast
            | mir::Intrinsic::PointerByteOffsetFrom
            | mir::Intrinsic::RawEq => SmallVec::new(),

            // float math
            mir::Intrinsic::Sqrt
            | mir::Intrinsic::Cbrt
            | mir::Intrinsic::Abs
            | mir::Intrinsic::IsFinite
            | mir::Intrinsic::IsInfinite
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
            | mir::Intrinsic::Expm1
            | mir::Intrinsic::Exp2
            | mir::Intrinsic::Log
            | mir::Intrinsic::Log1p
            | mir::Intrinsic::Log2
            | mir::Intrinsic::Log10
            | mir::Intrinsic::Pow
            | mir::Intrinsic::Floor
            | mir::Intrinsic::Ceil
            | mir::Intrinsic::Trunc
            | mir::Intrinsic::Round
            | mir::Intrinsic::RoundTiesEven
            | mir::Intrinsic::RoundTiesAway => SmallVec::new(),

            // omit compiler hints
            mir::Intrinsic::SpinLoop | mir::Intrinsic::Expect | mir::Intrinsic::BlackBox => {
                SmallVec::new()
            }
        }
    }

    /// Classify an address access with an explicit or runtime byte count.
    fn byte_effect(
        &mut self,
        address: mir::Value,
        operation: mir::MemoryOperation,
        size: Option<u64>,
    ) -> MemoryAccessEffect {
        let mut effect = self.address_effect(address, operation, MemoryAccessOrder::Plain);
        let MemoryRegion::Address { location, .. } = &mut effect.region else {
            unreachable!("address effect requires an addressed region");
        };
        location.size = size.into();

        effect
    }

    /// Return a constant byte count or retain a runtime length.
    fn constant_u64(&self, value: mir::Value) -> Option<u64> {
        let constant = self.constants.constant(value)?;
        let count = match constant {
            mir::Constant::UInt { value, .. } => u64::try_from(*value),
            mir::Constant::Int { value, .. } => u64::try_from(*value),
            _ => unreachable!("MIR byte length requires an integer constant"),
        };

        Some(count.unwrap_or_else(|_| unreachable!("MIR byte length exceeds the address range")))
    }

    /// Resolve the pointee type for an address-bearing value.
    fn address_value_type(&mut self, address: mir::Value) -> Option<mir::TypeId> {
        let ty = self.tree.get(self.function).expect_value_type(address);
        let ty = mir::Substitution::resolve(ty, self.tree);

        self.tree.get(ty).pointee_type()
    }

    /// Return the reference kind carried by an address, when applicable.
    fn reference_kind(&mut self, address: mir::Value) -> Option<mir::Reference> {
        let ty = self.tree.get(self.function).expect_value_type(address);
        let ty = mir::Substitution::resolve(ty, self.tree);

        self.tree.get(ty).reference_kind()
    }

    /// Return the reference storage carried by an address, when applicable.
    fn reference_storage(&mut self, address: mir::Value) -> Option<mir::Storage> {
        let ty = self.tree.get(self.function).expect_value_type(address);
        let ty = mir::Substitution::resolve(ty, self.tree);

        self.tree.get(ty).reference_storage()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemorySize;
    use crate::analyses::tests::TestModule;

    /// Match direct and addressed accesses to the same local storage.
    #[test]
    fn test_match_direct_and_addressed_local_accesses() {
        let mut program = TestModule::new(
            r#"
function test(): int32 {
    local l0: int32
    local l1: int32
    local l2: ref<int32, borrowed, 'frame, mutable, frame>

entry:
    v0: int32 = 7
    store l0, v0
    store l1, v0
    v1: ref<int32, borrowed, 'frame, mutable, frame> = address l0
    v2: int32 = load.copy (*v1)
    store l2, v1
    v3: ref<int32, borrowed, 'frame, mutable, frame> = load.copy l2
    v4: int32 = load.copy (*l2)
    store (*l2), v0
    return v4
}
"#,
        );
        let function_id = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let alias = analyses
            .alias(function_id, program.layouts.clone(), &mut program.tree)
            .unwrap();
        let effects = analyses.memory_effect(function_id, &mut program.tree, &program.effects);
        let instructions = program.entry_instructions(function_id);
        let first = effects.instruction_effects(instructions[1]).next().unwrap();
        let second = effects.instruction_effects(instructions[2]).next().unwrap();
        let load = effects.instruction_effects(instructions[4]).next().unwrap();

        assert!(first.may_alias(&alias, load).unwrap());
        assert!(load.may_alias(&alias, first).unwrap());
        assert!(!second.may_alias(&alias, load).unwrap());
        assert!(!load.may_alias(&alias, second).unwrap());

        // read the stored reference before loading or storing its pointee
        let reference = effects.instruction_effects(instructions[6]).next().unwrap();
        let place = program.tree.get(instructions[6]).place().unwrap();
        let pointee = place.clone().with_projection(mir::Projection::Deref);
        let mut access = load.clone();
        let MemoryRegion::Address { location, .. } = &mut access.region else {
            panic!("pointee load must name an address");
        };
        location.address = MemoryAddress::Place(pointee);
        for (index, reads, writes) in [(7, true, false), (8, false, true)] {
            access.reads = reads;
            access.writes = writes;
            let actual = effects
                .instruction_effects(instructions[index])
                .collect::<Vec<_>>();

            assert_eq!(actual, [reference, &access]);
        }
    }

    /// Retain runtime memory effects when allocating and releasing an owned object.
    #[test]
    fn test_record_allocation_and_release_effects() {
        let mut program = TestModule::new(
            r#"
function test(): void {
entry:
    v0: ref<int32, unique, mutable, local> = new.zeroed int32
    release v0
    return
}
"#,
        );
        let function = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let effects = analyses.memory_effect(
            program.entry_function_id(),
            &mut program.tree,
            &program.effects,
        );
        let instructions = &program
            .tree
            .get(program.tree.get(function).block(0))
            .instructions;
        for &instruction in instructions {
            let actual = effects
                .instruction_effects(instruction)
                .cloned()
                .collect::<Vec<_>>();
            let expected = MemoryAccessEffect {
                reads: true,
                writes: true,
                order: MemoryAccessOrder::Plain,
                is_barrier: false,
                region: MemoryRegion::Any {
                    spaces: mir::StorageSet::ANY,
                },
            };

            assert_eq!(actual, [expected]);
        }
    }

    /// Read the copy source, write its destination, and read both comparison operands.
    #[test]
    fn test_read_copy_source_and_write_destination() {
        let mut program = TestModule::new(
            r#"
function test<'a>(v0: ref<int32, borrowed, 'a, mutable, local>, v1: ref<int32, borrowed, 'a, mutable, local>): int32 {
entry(v0: ref<int32, borrowed, 'a, mutable, local>, v1: ref<int32, borrowed, 'a, mutable, local>):
    v2: usize = 12
    intrinsic.memory.raw.copyBytes(v0, v1, v2)
    v3: int32 = intrinsic.memory.raw.compareBytes(v0, v1, v2)
    return v3
}
"#,
        );
        let int32 = program.tree.intern_type(mir::Type::INT32);
        let function = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let effects = analyses.memory_effect(
            program.entry_function_id(),
            &mut program.tree,
            &program.effects,
        );
        let instructions = &program
            .tree
            .get(program.tree.get(function).block(0))
            .instructions;
        for (index, accesses) in [
            (1, [(1, true, false), (0, false, true)]),
            (2, [(0, true, false), (1, true, false)]),
        ] {
            let expected = accesses.map(|(address, reads, writes)| MemoryAccessEffect {
                reads,
                writes,
                order: MemoryAccessOrder::Plain,
                is_barrier: false,
                region: MemoryRegion::Address {
                    location: MemoryLocation::new(
                        mir::Value(address),
                        MemorySize::Bytes(12),
                        Some(int32),
                        Some(mir::Reference::Borrowed),
                        Some(mir::Storage::Heap(mir::Space::Local)),
                    ),
                    spaces: mir::StorageSet::LOCAL,
                },
            });
            let actual = effects
                .instruction_effects(instructions[index])
                .cloned()
                .collect::<Vec<_>>();

            assert_eq!(actual, expected, "instruction {index}");
        }
    }

    /// Preserve volatile access addresses and leave adjacent plain loads unordered.
    #[test]
    fn test_preserve_volatile_accesses() {
        let mut test = TestModule::new(
            r#"
function test<'a>(v0: ref<int32, borrowed, 'a, mutable, local>, v1: ref<int32, borrowed, 'a, mutable, local>): int32 {
entry(v0: ref<int32, borrowed, 'a, mutable, local>, v1: ref<int32, borrowed, 'a, mutable, local>):
    v2: int32 = intrinsic.memory.ptr.readVolatile(v0)
    intrinsic.memory.ptr.writeVolatile(v1, v2)
    v3: int32 = load.copy (*v0)
    return v3
}
"#,
        );
        let int32 = test.tree.intern_type(mir::Type::INT32);

        let function_id = test.first_function_id();

        // locate volatile instructions
        let block = test.tree.get(function_id).block(0);
        let volatile_load = test.tree.get(block).instructions[0];
        let volatile_store = test.tree.get(block).instructions[1];

        let function = function_id;
        let mut analyses = test.function_analyses();
        let effects = analyses.memory_effect(function_id, &mut test.tree, &test.effects);
        let plain_load = test.tree.get(test.tree.get(function).block(0)).instructions[2];
        for (instruction, address, reads, writes, order, size) in [
            (
                volatile_load,
                0,
                true,
                false,
                MemoryAccessOrder::Volatile,
                MemorySize::Type(int32),
            ),
            (
                volatile_store,
                1,
                false,
                true,
                MemoryAccessOrder::Volatile,
                MemorySize::Type(int32),
            ),
            (
                plain_load,
                0,
                true,
                false,
                MemoryAccessOrder::Plain,
                MemorySize::Type(int32),
            ),
        ] {
            let expected = MemoryAccessEffect {
                reads,
                writes,
                order,
                is_barrier: false,
                region: MemoryRegion::Address {
                    location: MemoryLocation::new(
                        mir::Value(address),
                        size,
                        Some(int32),
                        Some(mir::Reference::Borrowed),
                        Some(mir::Storage::Heap(mir::Space::Local)),
                    ),
                    spaces: mir::StorageSet::LOCAL,
                },
            };
            let actual = effects
                .instruction_effects(instruction)
                .cloned()
                .collect::<Vec<_>>();

            assert_eq!(actual, [expected]);
        }
    }

    /// Record each atomic operation's address, extent, access mode, ordering, and scope.
    #[test]
    fn test_record_atomic_memory_effects() {
        let mut program = TestModule::new(
            r#"
function test<'a>(v0: ref<int32, borrowed, 'a, readonly, frame>, v1: ref<uint64, unique, mutable, local>, v2: ref<int32, borrowed, 'a, mutable, shared>): void {
entry(v0: ref<int32, borrowed, 'a, readonly, frame>, v1: ref<uint64, unique, mutable, local>, v2: ref<int32, borrowed, 'a, mutable, shared>):
    v3: int32 = atomic.load (*v0), acquire, scope(invocation)
    v4: uint64 = 23
    atomic.store (*v1), v4, release, scope(device)
    v5: int32 = atomic.rmw.add (*v2), v3, acquireRelease, scope(workgroup)
    return
}
"#,
        );
        let int32 = program.tree.intern_type(mir::Type::INT32);
        let uint64 = program.tree.intern_type(mir::Type::UINT64);
        let function = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let effects = analyses.memory_effect(
            program.entry_function_id(),
            &mut program.tree,
            &program.effects,
        );
        let instructions = &program
            .tree
            .get(program.tree.get(function).block(0))
            .instructions;

        for (instruction, expected) in [
            (
                0,
                MemoryAccessEffect {
                    reads: true,
                    writes: false,
                    order: MemoryAccessOrder::Atomic(mir::AtomicAccess::new(
                        mir::MemoryOrdering::Acquire,
                        mir::ExecutionScope::Invocation,
                    )),
                    is_barrier: false,
                    region: MemoryRegion::Address {
                        location: MemoryLocation::new(
                            mir::Value(0),
                            MemorySize::Type(int32),
                            Some(int32),
                            Some(mir::Reference::Borrowed),
                            Some(mir::Storage::Frame),
                        ),
                        spaces: mir::StorageSet::FRAME,
                    },
                },
            ),
            (
                2,
                MemoryAccessEffect {
                    reads: false,
                    writes: true,
                    order: MemoryAccessOrder::Atomic(mir::AtomicAccess::new(
                        mir::MemoryOrdering::Release,
                        mir::ExecutionScope::Device,
                    )),
                    is_barrier: false,
                    region: MemoryRegion::Address {
                        location: MemoryLocation::new(
                            mir::Value(1),
                            MemorySize::Type(uint64),
                            Some(uint64),
                            Some(mir::Reference::Unique),
                            Some(mir::Storage::Heap(mir::Space::Local)),
                        ),
                        spaces: mir::StorageSet::LOCAL,
                    },
                },
            ),
            (
                3,
                MemoryAccessEffect {
                    reads: true,
                    writes: true,
                    order: MemoryAccessOrder::Atomic(mir::AtomicAccess::new(
                        mir::MemoryOrdering::AcquireRelease,
                        mir::ExecutionScope::Workgroup,
                    )),
                    is_barrier: false,
                    region: MemoryRegion::Address {
                        location: MemoryLocation::new(
                            mir::Value(2),
                            MemorySize::Type(int32),
                            Some(int32),
                            Some(mir::Reference::Borrowed),
                            Some(mir::Storage::Heap(mir::Space::Shared)),
                        ),
                        spaces: mir::StorageSet::SHARED,
                    },
                },
            ),
        ] {
            let actual = effects
                .instruction_effects(instructions[instruction])
                .cloned()
                .collect::<Vec<_>>();

            assert_eq!(actual, [expected], "instruction {instruction}");
        }

        assert_eq!(
            effects
                .instruction_effects(instructions[1])
                .collect::<Vec<_>>(),
            Vec::<&MemoryAccessEffect>::new()
        );
    }

    /// Record compare-exchange as a conditional write ordered by its success ordering.
    #[test]
    fn test_record_compare_exchange_effects() {
        let mut program = TestModule::new(
            r#"
function test<'a>(v0: ref<uint32, borrowed, 'a, mutable, shared>, v1: ref<uint32, borrowed, 'a, mutable, local>): void {
entry(v0: ref<uint32, borrowed, 'a, mutable, shared>, v1: ref<uint32, borrowed, 'a, mutable, local>):
    v2: uint32 = 1
    v3: uint32 = 2
    v4: (uint32, boolean) = atomic.cas (*v0), v2, v3, acquireRelease, failure(acquire), scope(workgroup)
    v5: (uint32, boolean) = atomic.cas (*v1), v3, v2, acquire, failure(relaxed), scope(system)
    return
}
"#,
        );
        let uint32 = program.tree.intern_type(mir::Type::UINT32);
        let function = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let effects = analyses.memory_effect(
            program.entry_function_id(),
            &mut program.tree,
            &program.effects,
        );
        let instructions = &program
            .tree
            .get(program.tree.get(function).block(0))
            .instructions;

        for (instruction, address, space, ordering, scope) in [
            (
                2,
                0,
                mir::Space::Shared,
                mir::MemoryOrdering::AcquireRelease,
                mir::ExecutionScope::Workgroup,
            ),
            (
                3,
                1,
                mir::Space::Local,
                mir::MemoryOrdering::Acquire,
                mir::ExecutionScope::System,
            ),
        ] {
            let storage = mir::Storage::Heap(space);
            let expected = MemoryAccessEffect {
                reads: true,
                writes: true,
                order: MemoryAccessOrder::Atomic(mir::AtomicAccess::new(ordering, scope)),
                is_barrier: false,
                region: MemoryRegion::Address {
                    location: MemoryLocation::new(
                        mir::Value(address),
                        MemorySize::Type(uint32),
                        Some(uint32),
                        Some(mir::Reference::Borrowed),
                        Some(storage),
                    ),
                    spaces: storage.storage_set(&program.tree),
                },
            };
            let actual = effects
                .instruction_effects(instructions[instruction])
                .cloned()
                .collect::<Vec<_>>();

            assert_eq!(actual, [expected], "instruction {instruction}");
        }
    }

    /// Retain fence ordering and storage without reporting a byte read or write.
    #[test]
    fn test_record_fence_order_scope_and_storage() {
        let mut program = TestModule::new(
            r#"
function test(): int32 {
entry:
    atomic.fence sequentiallyConsistent, scope(device), storage(shared)
    v0: int32 = 0
    return v0
}
"#,
        );
        let function = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let effects = analyses.memory_effect(
            program.entry_function_id(),
            &mut program.tree,
            &program.effects,
        );
        let instructions = &program
            .tree
            .get(program.tree.get(function).block(0))
            .instructions;
        let expected = MemoryAccessEffect {
            reads: false,
            writes: false,
            order: MemoryAccessOrder::Atomic(mir::AtomicAccess::new(
                mir::MemoryOrdering::SequentiallyConsistent,
                mir::ExecutionScope::Device,
            )),
            is_barrier: true,
            region: MemoryRegion::Any {
                spaces: mir::StorageSet::SHARED,
            },
        };
        let actual = effects
            .instruction_effects(instructions[0])
            .cloned()
            .collect::<Vec<_>>();

        assert_eq!(actual, [expected]);
    }

    /// Retain unknown call effects until the callee declares no memory accesses.
    #[test]
    fn test_restrict_call_effects_to_declared_memory() {
        let mut test = TestModule::new(
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
        let mut analyses = test.function_analyses();
        let effects = analyses.memory_effect(function_id, &mut test.tree, &test.effects);
        let actual = effects
            .instruction_effects(call_inst)
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            [MemoryAccessEffect {
                reads: true,
                writes: true,
                order: MemoryAccessOrder::Plain,
                is_barrier: false,
                region: MemoryRegion::Any {
                    spaces: mir::StorageSet::ANY
                },
            }]
        );

        // apply the callee's explicit declaration of no memory effects
        test.effects.upsert_call(callsite).memory = Some(mir::MemoryEffect::none());
        let mut analyses = test.function_analyses();
        let effects = analyses.memory_effect(function_id, &mut test.tree, &test.effects);
        let actual = effects.instruction_effects(call_inst).collect::<Vec<_>>();

        assert_eq!(actual, Vec::<&MemoryAccessEffect>::new());
    }

    /// Dynamic field reads retain their dispatch projections and exact widths.
    #[test]
    fn test_retain_dynamic_field_addresses() {
        let mut test = TestModule::new(
            r#"
type Writer {
    first: int32;
    second: float64;
}

function test(v0: dynamic<Writer, managed, readonly, local>): void {
entry(v0: dynamic<Writer, managed, readonly, local>):
    v1: int32 = dynamic.read v0, 0
    v2: float64 = dynamic.read v0, 1
    return
}
"#,
        );
        let int32 = test.tree.intern_type(mir::Type::INT32);
        let float64 = test.tree.intern_type(mir::Type::FLOAT64);

        let function_id = test.entry_function_id();
        let function = function_id;
        let block = test.tree.get(function).block(0);
        let mut analyses = test.function_analyses();
        let effects = analyses.memory_effect(function_id, &mut test.tree, &test.effects);

        for (slot, ty) in [(0, int32), (1, float64)] {
            let expected = MemoryAccessEffect {
                reads: true,
                writes: false,
                order: MemoryAccessOrder::Plain,
                is_barrier: false,
                region: MemoryRegion::Address {
                    location: MemoryLocation::new(
                        MemoryAddress::Dynamic {
                            value: mir::Value(0),
                            slot: mir::DispatchSlot(slot),
                        },
                        MemorySize::Type(ty),
                        Some(ty),
                        Some(mir::Reference::Managed),
                        Some(mir::Storage::Heap(mir::Space::Local)),
                    ),
                    spaces: mir::StorageSet::LOCAL,
                },
            };
            let actual = effects
                .instruction_effects(test.tree.get(block).instructions[slot as usize])
                .cloned()
                .collect::<Vec<_>>();

            assert_eq!(actual, [expected], "field {slot}");
        }
    }

    /// Preserve runtime byte lengths instead of substituting the pointed-to type's size.
    #[test]
    fn test_preserve_runtime_copy_extent() {
        let mut program = TestModule::new(
            r#"
function test(v0: ptr<int32, mutable>, v1: ptr<int32, readonly>, v2: usize): void {
entry(v0: ptr<int32, mutable>, v1: ptr<int32, readonly>, v2: usize):
    intrinsic.memory.raw.copyBytes(v0, v1, v2)
    return
}
"#,
        );
        let function = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let effects = analyses.memory_effect(
            program.entry_function_id(),
            &mut program.tree,
            &program.effects,
        );
        let instruction = program
            .tree
            .get(program.tree.get(function).block(0))
            .instructions[0];
        let actual = effects
            .instruction_effects(instruction)
            .map(|effect| {
                let location = effect.region.location().unwrap();
                (
                    effect.reads,
                    effect.writes,
                    location.address.clone(),
                    location.size,
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            actual,
            [
                (
                    true,
                    false,
                    MemoryAddress::from(mir::Value(1)),
                    MemorySize::Dynamic
                ),
                (
                    false,
                    true,
                    MemoryAddress::from(mir::Value(0)),
                    MemorySize::Dynamic
                ),
            ]
        );
    }
}
