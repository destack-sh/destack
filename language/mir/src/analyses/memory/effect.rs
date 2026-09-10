use std::ops::Range;

use smallvec::SmallVec;

use crate as mir;
use crate::{
    AliasTable, Analysis, ConstantTable, MemoryAddress, MemoryLocation, MemoryRegion, Mutation,
    NodeTable, StorageRoot, TargetLayout, collect_reachable_blocks,
};

/// Classified memory effects for the operations in one function.
#[derive(Debug)]
pub struct MemoryEffects {
    /// Memory effects in execution order within each reachable block.
    effects: Vec<MemoryAccessEffect>,
    /// Effect ranges indexed by instruction.
    instructions: NodeTable<mir::Instruction, Range<u32>>,
    /// Effect ranges indexed by block terminator.
    terminators: NodeTable<mir::Block, Range<u32>>,
}

impl MemoryEffects {
    /// Classify the operations in each reachable block.
    pub fn analyse(
        function: &mir::Function,
        constants: &ConstantTable,
        accesses: &mir::AccessTable,
        effects: &mir::EffectTable,
        target_layout: TargetLayout,
        tree: &mir::Tree,
    ) -> Self {
        // allocate entries for every operation in the function
        let instructions = function
            .blocks()
            .iter()
            .flat_map(|block| tree.get(*block).instructions.iter().copied())
            .collect::<Vec<_>>();
        let mut result = Self {
            effects: Vec::new(),
            instructions: NodeTable::from_nodes(&instructions, || 0..0),
            terminators: NodeTable::from_nodes(function.blocks(), || 0..0),
        };

        // preserve empty results for declarations without bodies
        let Some(entry) = function.entry() else {
            return result;
        };

        // classify reachable operations using their explicit and derived effects
        let mut builder =
            MemoryEffectBuilder::new(function, tree, constants, accesses, effects, target_layout);
        for block_id in collect_reachable_blocks(function, tree, entry) {
            let block = tree.get(block_id);

            // retain each instruction's observable memory accesses
            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
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
            let terminator = tree.get(block.terminator);
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

impl Analysis for MemoryEffects {
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
    /// Whether the instruction is volatile.
    pub is_volatile: bool,
    /// Whether the instruction acts as a memory barrier.
    pub is_barrier: bool,
    /// The memory region being accessed.
    pub region: MemoryRegion,
}

impl MemoryAccessEffect {
    /// Return whether this effect may clobber one memory location.
    pub fn clobbers_location(&self, location: &MemoryLocation, alias: &AliasTable) -> bool {
        if self.is_barrier {
            return true;
        }

        self.writes && self.may_touch_location(location, alias)
    }

    /// Return whether this effect may touch one memory location.
    pub fn may_touch_location(&self, location: &MemoryLocation, alias: &AliasTable) -> bool {
        // reject disjoint memory spaces
        if !self.region.spaces().may_alias(location.spaces()) {
            return false;
        }

        // compare local access roots
        if let MemoryRegion::Local(local) = self.region {
            let storage = StorageRoot::LocalSlot(local);

            return alias.may_touch_root(location, &storage);
        }

        // compare addressed accesses through alias analysis
        if let MemoryRegion::Address {
            location: effect_location,
            ..
        } = &self.region
        {
            return alias.alias(effect_location, location).may_alias();
        }

        // treat imprecise accesses as touching compatible locations
        true
    }

    /// Return whether this effect is trackable by memory optimizations.
    pub fn is_trackable(&self) -> bool {
        !self.is_volatile && !self.is_barrier && !matches!(self.region, MemoryRegion::Any { .. })
    }

    /// Return whether this effect describes the same region as another effect.
    pub fn matches_region(&self, alias: &AliasTable, other: &MemoryAccessEffect) -> bool {
        // reject disjoint memory spaces
        if !self.region.spaces().may_alias(other.region.spaces()) {
            return false;
        }

        // compare concrete locations
        match (&self.region, &other.region) {
            (MemoryRegion::Local(local), MemoryRegion::Local(other_local)) => local == other_local,
            (
                MemoryRegion::Address { location, .. },
                MemoryRegion::Address {
                    location: other_location,
                    ..
                },
            ) => {
                if !location.has_compatible_value(other_location) {
                    return false;
                }

                alias.alias(location, other_location).is_must_alias()
            }
            _ => false,
        }
    }

    /// Return whether this effect may alias another effect.
    pub fn may_alias(&self, alias: &AliasTable, other: &MemoryAccessEffect) -> bool {
        // reject disjoint memory spaces
        if !self.region.spaces().may_alias(other.region.spaces()) {
            return false;
        }

        // compare concrete locations
        match (&self.region, &other.region) {
            (MemoryRegion::Any { .. }, _) | (_, MemoryRegion::Any { .. }) => true,
            (MemoryRegion::Local(local), MemoryRegion::Local(other)) => local == other,
            (
                MemoryRegion::Address { location, .. },
                MemoryRegion::Address {
                    location: other_location,
                    ..
                },
            ) => alias.alias(location, other_location).may_alias(),
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

/// Collector for memory accesses.
struct MemoryEffectBuilder<'a> {
    /// MIR function under analysis.
    function: &'a mir::Function,
    /// MIR tree.
    tree: &'a mir::Tree,
    /// Constants available for memory ranges.
    constants: &'a ConstantTable,
    /// Explicit memory access table.
    accesses: &'a mir::AccessTable,
    /// Explicit effect table.
    effect_table: &'a mir::EffectTable,
    /// Type context for layout sensitive operations.
    target_layout: TargetLayout,
}

impl<'a> MemoryEffectBuilder<'a> {
    /// Create a new collector.
    fn new(
        function: &'a mir::Function,
        tree: &'a mir::Tree,
        constants: &'a ConstantTable,
        accesses: &'a mir::AccessTable,
        effect_table: &'a mir::EffectTable,
        target_layout: TargetLayout,
    ) -> Self {
        // build collector state
        Self {
            function,
            tree,
            constants,
            accesses,
            effect_table,
            target_layout,
        }
    }

    /// Return memory effects for one instruction.
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
            | mir::Instruction::FunctionEnvironmentCurrent { .. }
            | mir::Instruction::ContextGet { .. }
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
            | mir::Instruction::VariantPayloadAddr { .. }
            | mir::Instruction::SliceView { .. }
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
            mir::Instruction::Load { pointer, .. }
            | mir::Instruction::VariantTagLoad {
                variant: pointer, ..
            } => {
                let effect = self.address_effect(*pointer, mir::MemoryOperation::Read, false);

                Self::single_effect(effect)
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
                    self.target_layout.pointer_bits(),
                    self.tree,
                );
                region.set_spaces(self.address_storage_set(*dynamic));

                Self::single_effect(MemoryAccessEffect::read(region, false))
            }
            mir::Instruction::Store { pointer, .. } => {
                let effect = self.address_effect(*pointer, mir::MemoryOperation::Write, false);

                Self::single_effect(effect)
            }
            mir::Instruction::AtomicLoad { pointer, .. } => {
                let effect = self.address_effect(*pointer, mir::MemoryOperation::Read, true);

                Self::single_effect(effect)
            }
            mir::Instruction::AtomicStore { pointer, .. } => {
                let effect = self.address_effect(*pointer, mir::MemoryOperation::Write, true);

                Self::single_effect(effect)
            }
            mir::Instruction::AtomicCompareExchange { pointer, .. }
            | mir::Instruction::AtomicRmw { pointer, .. } => {
                let effect = self.address_effect(*pointer, mir::MemoryOperation::ReadWrite, true);

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
            mir::Instruction::ContextCurrent { .. }
            | mir::Instruction::ContextReplace { .. }
            | mir::Instruction::ContextBind { .. }
            | mir::Instruction::Release { .. }
            | mir::Instruction::Drop { .. }
            | mir::Instruction::NewZeroed { .. }
            | mir::Instruction::NewUninit { .. }
            | mir::Instruction::NewSliceZeroed { .. }
            | mir::Instruction::NewSliceUninit { .. }
            | mir::Instruction::Poll => Self::single_effect(MemoryAccessEffect::read_write(
                MemoryRegion::any_spaces(mir::StorageSet::ANY),
                false,
            )),
            mir::Instruction::Intrinsic {
                intrinsic,
                arguments,
                ..
            } => self.intrinsic_effects(*intrinsic, *arguments),
        }
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
            | mir::Terminator::VariantSwitch { .. }
            | mir::Terminator::Panic { .. }
            | mir::Terminator::UnwindResume
            | mir::Terminator::Abort { .. }
            | mir::Terminator::Unreachable => SmallVec::new(),
        }
    }

    /// Convert explicit memory tables into access effects.
    fn table_effects(
        &mut self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
    ) -> Option<SmallVec<[MemoryAccessEffect; 2]>> {
        // read tables when present
        let accesses = self.accesses.get(instruction_id)?;

        // build effect list from tables
        let mut effects = SmallVec::new();
        for access in accesses {
            effects.push(self.effect_from_entry(access));
        }

        Some(effects)
    }

    /// Classify one explicit memory access.
    fn effect_from_entry(&mut self, access: &mir::MemoryAccess) -> MemoryAccessEffect {
        // resolve the target region
        let region = match access.target {
            mir::MemoryTarget::Address(pointer) => {
                let value_type = self.address_value_type(pointer);
                let reference_kind = self.reference_kind(pointer);
                let reference_storage = self.reference_storage(pointer);
                MemoryRegion::from_address_with_size(
                    pointer,
                    value_type,
                    reference_kind,
                    reference_storage,
                    access.byte_len,
                    self.target_layout.pointer_bits(),
                    self.tree,
                )
            }
            mir::MemoryTarget::Local(local) => MemoryRegion::Local(local),
            mir::MemoryTarget::Global(global) => MemoryRegion::any_spaces(
                mir::Storage::global(self.tree.get(global).space).storage_set(),
            ),
        };

        // map the access operation to an effect
        let is_volatile = access.requires_exact_position();
        let mut effect = match access.operation {
            mir::MemoryOperation::Read => MemoryAccessEffect::read(region, is_volatile),
            mir::MemoryOperation::Write => MemoryAccessEffect::write(region, is_volatile),
            mir::MemoryOperation::ReadWrite => MemoryAccessEffect::read_write(region, is_volatile),
        };

        // apply target storage
        let spaces = self.entry_storage_set(access);
        effect.region.set_spaces(spaces);
        effect
    }

    /// Build one memory effect over an address-bearing value.
    fn address_effect(
        &self,
        address: mir::Value,
        operation: mir::MemoryOperation,
        is_volatile: bool,
    ) -> MemoryAccessEffect {
        let value_type = self.address_value_type(address);
        let reference_kind = self.reference_kind(address);
        let reference_storage = self.reference_storage(address);
        let region = MemoryRegion::from_address(
            address,
            value_type,
            reference_kind,
            reference_storage,
            self.target_layout.pointer_bits(),
            self.tree,
        );
        let mut effect = match operation {
            mir::MemoryOperation::Read => MemoryAccessEffect::read(region, is_volatile),
            mir::MemoryOperation::Write => MemoryAccessEffect::write(region, is_volatile),
            mir::MemoryOperation::ReadWrite => MemoryAccessEffect::read_write(region, is_volatile),
        };

        // constrain the region to the address storage
        let spaces = self.address_storage_set(address);
        effect.region.set_spaces(spaces);

        effect
    }

    /// Constrain one memory effect to its address storage.
    fn apply_address_region(&self, effect: &mut MemoryAccessEffect, address: mir::Value) {
        let spaces = self.address_storage_set(address);
        effect.region.set_spaces(spaces);
    }

    /// Apply local region tables to an effect.
    fn apply_local_region(&self, effect: &mut MemoryAccessEffect) {
        effect.region.set_spaces(mir::StorageSet::FRAME);
    }

    /// Resolve storage from an access target.
    fn entry_storage_set(&self, access: &mir::MemoryAccess) -> mir::StorageSet {
        match access.target {
            mir::MemoryTarget::Local(_) => mir::StorageSet::FRAME,
            mir::MemoryTarget::Global(global) => {
                mir::Storage::global(self.tree.get(global).space).storage_set()
            }
            mir::MemoryTarget::Address(pointer) => self.address_storage_set(pointer),
        }
    }

    /// Resolve the memory spaces for an address-bearing value.
    fn address_storage_set(&self, address: mir::Value) -> mir::StorageSet {
        let Some(storage) = self.function.reference_storage(address, self.tree) else {
            return mir::StorageSet::ANY;
        };

        storage.storage_set()
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

        // skip empty storage
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
                        let dst_type = self.address_value_type(*dst);
                        let src_type = self.address_value_type(*src);
                        let dst_kind = self.reference_kind(*dst);
                        let src_kind = self.reference_kind(*src);
                        let dst_storage = self.reference_storage(*dst);
                        let src_storage = self.reference_storage(*src);
                        let mut read_effect = MemoryAccessEffect::read(
                            MemoryRegion::from_address_with_size(
                                *src,
                                src_type,
                                src_kind,
                                src_storage,
                                size,
                                self.target_layout.pointer_bits(),
                                self.tree,
                            ),
                            false,
                        );
                        self.apply_address_region(&mut read_effect, *src);
                        effects.push(read_effect);

                        let mut write_effect = MemoryAccessEffect::write(
                            MemoryRegion::from_address_with_size(
                                *dst,
                                dst_type,
                                dst_kind,
                                dst_storage,
                                size,
                                self.target_layout.pointer_bits(),
                                self.tree,
                            ),
                            false,
                        );
                        self.apply_address_region(&mut write_effect, *dst);
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
                        let dst_type = self.address_value_type(*dst);
                        let dst_kind = self.reference_kind(*dst);
                        let dst_storage = self.reference_storage(*dst);
                        let mut effect = MemoryAccessEffect::write(
                            MemoryRegion::from_address_with_size(
                                *dst,
                                dst_type,
                                dst_kind,
                                dst_storage,
                                size,
                                self.target_layout.pointer_bits(),
                                self.tree,
                            ),
                            false,
                        );
                        self.apply_address_region(&mut effect, *dst);
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
                        let left_type = self.address_value_type(*left);
                        let right_type = self.address_value_type(*right);
                        let left_kind = self.reference_kind(*left);
                        let right_kind = self.reference_kind(*right);
                        let left_storage = self.reference_storage(*left);
                        let right_storage = self.reference_storage(*right);
                        let mut left_effect = MemoryAccessEffect::read(
                            MemoryRegion::from_address_with_size(
                                *left,
                                left_type,
                                left_kind,
                                left_storage,
                                size,
                                self.target_layout.pointer_bits(),
                                self.tree,
                            ),
                            false,
                        );
                        self.apply_address_region(&mut left_effect, *left);
                        effects.push(left_effect);

                        let mut right_effect = MemoryAccessEffect::read(
                            MemoryRegion::from_address_with_size(
                                *right,
                                right_type,
                                right_kind,
                                right_storage,
                                size,
                                self.target_layout.pointer_bits(),
                                self.tree,
                            ),
                            false,
                        );
                        self.apply_address_region(&mut right_effect, *right);
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
                        let value_type = self.address_value_type(pointer);
                        let reference_kind = self.reference_kind(pointer);
                        let reference_storage = self.reference_storage(pointer);
                        let mut effect = MemoryAccessEffect::read(
                            MemoryRegion::from_address(
                                pointer,
                                value_type,
                                reference_kind,
                                reference_storage,
                                self.target_layout.pointer_bits(),
                                self.tree,
                            ),
                            false,
                        );
                        self.apply_address_region(&mut effect, pointer);
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

            // preserve volatile pointer position
            mir::Intrinsic::VolatileLoad | mir::Intrinsic::VolatileStore => {
                let mut effects = SmallVec::new();
                let pointer = args.first().copied();
                let is_load = intrinsic == mir::Intrinsic::VolatileLoad;

                // emit a volatile effect on the accessed location
                match pointer {
                    Some(pointer) => {
                        let value_type = self.address_value_type(pointer);
                        let reference_kind = self.reference_kind(pointer);
                        let reference_storage = self.reference_storage(pointer);
                        let region = MemoryRegion::from_address(
                            pointer,
                            value_type,
                            reference_kind,
                            reference_storage,
                            self.target_layout.pointer_bits(),
                            self.tree,
                        );
                        let mut effect = match is_load {
                            true => MemoryAccessEffect::read(region, true),
                            false => MemoryAccessEffect::write(region, true),
                        };
                        self.apply_address_region(&mut effect, pointer);
                        effects.push(effect);
                    }
                    None => effects.push(MemoryAccessEffect::read_write(
                        MemoryRegion::any_spaces(mir::StorageSet::ANY),
                        true,
                    )),
                }

                effects
            }

            // representation-only intrinsics
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

            // compiler hints
            mir::Intrinsic::SpinLoop | mir::Intrinsic::Expect | mir::Intrinsic::BlackBox => {
                SmallVec::new()
            }
        }
    }

    /// Resolve a constant byte size from a value when possible.
    fn constant_u64(&self, value: mir::Value) -> Option<u64> {
        let constant = self.constants.constant(value)?;

        match constant {
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

    /// Resolve the pointee type for an address-bearing value.
    fn address_value_type(&self, address: mir::Value) -> Option<mir::TypeId> {
        self.function.pointee_type(address, self.tree)
    }

    /// Return the reference kind carried by an address, when applicable.
    fn reference_kind(&self, address: mir::Value) -> Option<mir::ReferenceKind> {
        self.function.reference_kind(address, self.tree)
    }

    /// Return the reference storage carried by an address, when applicable.
    fn reference_storage(&self, address: mir::Value) -> Option<mir::Storage> {
        self.function.reference_storage(address, self.tree)
    }
}
