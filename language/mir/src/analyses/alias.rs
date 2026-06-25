use std::sync::Arc;

use crate as mir;

use crate::{
    Analysis, AnalysisId, FunctionAnalyses, FunctionAnalysis, MemoryLocation, MemoryTarget,
    MemoryTargetBuilder, Storage, TargetLayout, ValueDefinitions, ValueTypeMap,
};

/// Alias analysis for one MIR function.
#[derive(Debug)]
pub struct AliasAnalysis {
    /// Value definitions used by target resolution.
    definitions: ValueDefinitions,
    /// Function parameters used by target resolution.
    parameters: Vec<mir::FunctionParameter>,
    /// Value type lookup for reference element sizing.
    value_types: ValueTypeMap,
    /// Target layout for layout sensitive operations.
    target_layout: TargetLayout,
    /// The MIR tree for queries.
    tree: Arc<mir::Tree>,
}

impl AliasAnalysis {
    /// Build alias analysis for one function.
    pub fn build(
        function: &mir::Function,
        definitions: &ValueDefinitions,
        value_types: ValueTypeMap,
        target_layout: TargetLayout,
        tree: &mir::Tree,
    ) -> Self {
        Self {
            definitions: definitions.clone(),
            parameters: function.parameters.clone(),
            value_types,
            target_layout,
            tree: Arc::new(tree.clone()),
        }
    }

    /// Query whether two memory locations alias.
    pub fn alias(&self, left: &MemoryLocation, right: &MemoryLocation) -> AliasResult {
        if left.reference == right.reference {
            return left.alias_same_reference(right);
        }

        let left_target = self.target(left.reference);
        let right_target = self.target(right.reference);

        match (&left_target, &right_target) {
            (MemoryTarget::Place(left_place), MemoryTarget::Place(right_place)) => {
                if left_place.storage.is_disjoint_from(&right_place.storage) {
                    return AliasResult::NoAlias;
                }

                if left_place
                    .storage
                    .exclusive_parameters_are_disjoint(&right_place.storage)
                {
                    return AliasResult::NoAlias;
                }

                if left_place.storage == right_place.storage {
                    return left_place.alias_with(left, right_place, right);
                }
            }
            _ => {
                let left_spaces = left_target.spaces(&self.tree);
                let right_spaces = right_target.spaces(&self.tree);

                if left_spaces.is_disjoint(right_spaces) {
                    return AliasResult::NoAlias;
                }
            }
        }

        AliasResult::MayAlias
    }

    /// Return memory behavior for an instruction relative to one location.
    pub fn memory_effect(
        &self,
        instruction: mir::LocalNodeId<mir::Instruction>,
        location: &MemoryLocation,
    ) -> MemoryEffectKind {
        if let Some(accesses) = self.tree.metadata.memory.memory_accesses(instruction) {
            return self.memory_effect_from_metadata(accesses, location);
        }

        let instruction = self.tree.get(instruction);

        match instruction {
            mir::Instruction::Error => {
                panic!("invalid MIR instruction reached alias analysis");
            }
            mir::Instruction::Load { pointer, .. }
            | mir::Instruction::TensorLoad { view: pointer, .. }
            | mir::Instruction::AtomicLoad { pointer, .. } => {
                self.reference_memory_effect(*pointer, location, MemoryEffectKind::READ)
            }
            mir::Instruction::Store { pointer, .. }
            | mir::Instruction::TensorStore { view: pointer, .. }
            | mir::Instruction::TensorFill { view: pointer, .. }
            | mir::Instruction::AtomicStore { pointer, .. }
            | mir::Instruction::Free { value: pointer } => {
                self.reference_memory_effect(*pointer, location, MemoryEffectKind::WRITE)
            }
            mir::Instruction::TensorCopy { target, source } => {
                let target =
                    self.reference_memory_effect(*target, location, MemoryEffectKind::WRITE);
                let source =
                    self.reference_memory_effect(*source, location, MemoryEffectKind::READ);

                target.union(source)
            }
            mir::Instruction::AtomicCompareExchange { pointer, .. }
            | mir::Instruction::AtomicRmw { pointer, .. } => {
                self.reference_memory_effect(*pointer, location, MemoryEffectKind::READ_WRITE)
            }
            mir::Instruction::AtomicFence { .. } | mir::Instruction::BarrierWrite { .. } => {
                MemoryEffectKind::READ_WRITE
            }
            mir::Instruction::LocalGet { local, .. } => self.storage_memory_effect(
                Storage::LocalSlot(*local),
                location,
                MemoryEffectKind::READ,
            ),
            mir::Instruction::LocalSet { local, .. } => self.storage_memory_effect(
                Storage::LocalSlot(*local),
                location,
                MemoryEffectKind::WRITE,
            ),
            mir::Instruction::Call { .. }
            | mir::Instruction::CallVirtual { .. }
            | mir::Instruction::CallDynamic { .. }
            | mir::Instruction::CallIndirect { .. } => {
                self.call_memory_effect(instruction, location)
            }
            mir::Instruction::Intrinsic { intrinsic, .. } => {
                MemoryEffectKind::from_intrinsic(intrinsic)
            }
            mir::Instruction::NewZeroed { .. }
            | mir::Instruction::NewUninit { .. }
            | mir::Instruction::NewSliceZeroed { .. }
            | mir::Instruction::NewSliceUninit { .. }
            | mir::Instruction::FrameAllocZeroed { .. }
            | mir::Instruction::FrameAllocUninit { .. }
            | mir::Instruction::NewComplete { .. } => MemoryEffectKind::NONE,
            _ => MemoryEffectKind::NONE,
        }
    }

    /// Return memory behavior for one storage-root operation.
    fn storage_memory_effect(
        &self,
        storage: Storage,
        location: &MemoryLocation,
        effect: MemoryEffectKind,
    ) -> MemoryEffectKind {
        let location_target = self.target(location.reference);

        if location_target.may_touch_storage(&storage, &self.tree) {
            effect
        } else {
            MemoryEffectKind::NONE
        }
    }

    /// Return true when two reference values may alias.
    pub fn references_may_alias(&self, left: mir::Value, right: mir::Value) -> bool {
        let left = MemoryLocation::from_reference(left);
        let right = MemoryLocation::from_reference(right);

        self.alias(&left, &right).may_alias()
    }

    /// Return true when two reference values definitely do not alias.
    pub fn references_no_alias(&self, left: mir::Value, right: mir::Value) -> bool {
        let left = MemoryLocation::from_reference(left);
        let right = MemoryLocation::from_reference(right);

        self.alias(&left, &right).is_no_alias()
    }

    /// Return true when an instruction may write to one location.
    pub fn may_clobber(
        &self,
        instruction: mir::LocalNodeId<mir::Instruction>,
        location: &MemoryLocation,
    ) -> bool {
        self.memory_effect(instruction, location).writes()
    }

    /// Return true when an instruction may read one location.
    pub fn may_read(
        &self,
        instruction: mir::LocalNodeId<mir::Instruction>,
        location: &MemoryLocation,
    ) -> bool {
        self.memory_effect(instruction, location).reads()
    }

    /// Resolve one reference value into a memory target.
    fn target(&self, reference: mir::Value) -> MemoryTarget {
        let mut builder = MemoryTargetBuilder::new(
            &self.definitions,
            &self.tree,
            &self.parameters,
            &self.value_types,
            self.target_layout,
        );

        builder.target(reference)
    }

    /// Return the memory spaces a reference may access.
    fn space_set(&self, reference: mir::Value) -> mir::SpaceSet {
        self.target(reference).spaces(&self.tree)
    }

    /// Return memory behavior for one reference-backed operation.
    fn reference_memory_effect(
        &self,
        reference: mir::Value,
        location: &MemoryLocation,
        effect: MemoryEffectKind,
    ) -> MemoryEffectKind {
        let access = MemoryLocation::from_reference(reference);

        if self.alias(&access, location).is_no_alias() {
            MemoryEffectKind::NONE
        } else {
            effect
        }
    }

    /// Compute memory behavior for memory metadata entries.
    fn memory_effect_from_metadata(
        &self,
        accesses: &[mir::MemoryAccessMetadata],
        location: &MemoryLocation,
    ) -> MemoryEffectKind {
        let mut result = MemoryEffectKind::NONE;

        for access in accesses {
            if !self.metadata_access_may_alias(access, location) {
                continue;
            }

            let access_effect = match access.kind {
                mir::MemoryAccessKind::Read | mir::MemoryAccessKind::PrefetchRead => {
                    MemoryEffectKind::READ
                }
                mir::MemoryAccessKind::Write => MemoryEffectKind::WRITE,
                mir::MemoryAccessKind::ReadWrite | mir::MemoryAccessKind::ReadModifyWrite => {
                    MemoryEffectKind::READ_WRITE
                }
                mir::MemoryAccessKind::PrefetchWrite => MemoryEffectKind::READ,
                mir::MemoryAccessKind::Fence => MemoryEffectKind::READ_WRITE,
            };

            result = result.union(access_effect);
        }

        result
    }

    /// Return whether metadata access may touch one location.
    fn metadata_access_may_alias(
        &self,
        access: &mir::MemoryAccessMetadata,
        location: &MemoryLocation,
    ) -> bool {
        if !self.metadata_spaces_overlap(access, location) {
            return false;
        }

        match access.target {
            mir::MemoryAccessTarget::Reference(pointer) => {
                let access = MemoryLocation::new(pointer, access.size, None, None, None);

                self.alias(&access, location).may_alias()
            }
            mir::MemoryAccessTarget::Local(local) => {
                let location_target = self.target(location.reference);
                let access_storage = Storage::LocalSlot(local);

                location_target.may_touch_storage(&access_storage, &self.tree)
            }
            mir::MemoryAccessTarget::Global(global) => {
                let location_target = self.target(location.reference);
                let access_storage = Storage::Static(global);

                location_target.may_touch_storage(&access_storage, &self.tree)
            }
            mir::MemoryAccessTarget::Unknown => true,
        }
    }

    /// Return whether metadata access spaces overlap one location.
    fn metadata_spaces_overlap(
        &self,
        access: &mir::MemoryAccessMetadata,
        location: &MemoryLocation,
    ) -> bool {
        let location_space = self.space_set(location.reference);

        let access_space = match access.space.clone() {
            Some(space) => space.space_set(),
            None => match access.target {
                mir::MemoryAccessTarget::Reference(pointer) => self.space_set(pointer),
                mir::MemoryAccessTarget::Local(_) => mir::SpaceSet::FRAME,
                mir::MemoryAccessTarget::Global(global) => {
                    let global = self.tree.get(global);

                    global.space.space_set()
                }
                mir::MemoryAccessTarget::Unknown => mir::SpaceSet::ANY,
            },
        };

        !access_space.is_disjoint(location_space)
    }

    /// Return memory behavior for a call instruction.
    fn call_memory_effect(
        &self,
        instruction: &mir::Instruction,
        location: &MemoryLocation,
    ) -> MemoryEffectKind {
        let Some(effects) = instruction
            .call_direct_target()
            .and_then(|function| self.tree.metadata.functions.function(function))
            .map(|metadata| metadata.memory.clone())
        else {
            return MemoryEffectKind::READ_WRITE;
        };

        if !effects.reads && !effects.writes {
            return MemoryEffectKind::NONE;
        }

        if effects.spaces.is_empty() {
            return MemoryEffectKind::NONE;
        }

        let space_set = self.space_set(location.reference);
        if !effects.spaces.contains(space_set) {
            return MemoryEffectKind::NONE;
        }

        MemoryEffectKind::from_flags(effects.reads, effects.writes)
    }
}

impl Analysis for AliasAnalysis {
    const ID: AnalysisId = AnalysisId("alias");
    const INVALIDATED_BY: mir::Mutation = mir::Mutation::VALUE.union(mir::Mutation::MEMORY);
}

impl FunctionAnalysis for AliasAnalysis {
    fn compute(function: &mir::Function, tree: &mir::Tree, analyses: &FunctionAnalyses) -> Self {
        let definitions = analyses.get::<ValueDefinitions>(function, tree);
        let value_types = ValueTypeMap::new(function, tree);

        Self::build(
            function,
            &definitions,
            value_types,
            analyses.target_layout(),
            tree,
        )
    }
}

/// Result of an alias query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AliasResult {
    /// The locations definitely refer to the same memory.
    MustAlias,
    /// The locations partially overlap.
    PartialAlias,
    /// The locations might refer to the same memory.
    MayAlias,
    /// The locations definitely do not overlap.
    NoAlias,
}

impl AliasResult {
    /// Check whether this proves non-aliasing.
    pub fn is_no_alias(self) -> bool {
        matches!(self, AliasResult::NoAlias)
    }

    /// Check whether the locations may alias.
    pub fn may_alias(self) -> bool {
        !matches!(self, AliasResult::NoAlias)
    }

    /// Check whether this proves identical memory.
    pub fn is_must_alias(self) -> bool {
        matches!(self, AliasResult::MustAlias)
    }
}

/// Read and write behavior for memory operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryEffectKind(u8);

impl MemoryEffectKind {
    /// No memory access.
    pub const NONE: Self = Self(0);
    /// Reads memory only.
    pub const READ: Self = Self(1);
    /// Writes memory only.
    pub const WRITE: Self = Self(2);
    /// Reads and writes memory.
    pub const READ_WRITE: Self = Self(3);

    /// Check whether this includes a read.
    pub fn reads(self) -> bool {
        self.0 & 1 != 0
    }

    /// Check whether this includes a write.
    pub fn writes(self) -> bool {
        self.0 & 2 != 0
    }

    /// Union with another memory behavior.
    pub fn union(self, other: MemoryEffectKind) -> MemoryEffectKind {
        Self(self.0 | other.0)
    }

    /// Build memory behavior from read and write bits.
    pub fn from_flags(reads: bool, writes: bool) -> Self {
        let mut value = 0;
        if reads {
            value |= 1;
        }
        if writes {
            value |= 2;
        }

        Self(value)
    }

    /// Return memory behavior for one intrinsic.
    pub fn from_intrinsic(intrinsic: &mir::Intrinsic) -> Self {
        match intrinsic {
            mir::Intrinsic::Memcpy | mir::Intrinsic::Memmove | mir::Intrinsic::Memset => {
                Self::READ_WRITE
            }
            _ => Self::NONE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_alias_results() {
        assert!(AliasResult::NoAlias.is_no_alias());
        assert!(!AliasResult::MayAlias.is_no_alias());

        assert!(AliasResult::MustAlias.is_must_alias());
        assert!(!AliasResult::PartialAlias.is_must_alias());

        assert!(AliasResult::MayAlias.may_alias());
        assert!(AliasResult::MustAlias.may_alias());
        assert!(!AliasResult::NoAlias.may_alias());
    }

    #[test]
    fn test_check_memory_effect_results() {
        assert!(!MemoryEffectKind::NONE.reads());
        assert!(!MemoryEffectKind::NONE.writes());

        assert!(MemoryEffectKind::READ.reads());
        assert!(!MemoryEffectKind::READ.writes());

        assert!(!MemoryEffectKind::WRITE.reads());
        assert!(MemoryEffectKind::WRITE.writes());

        assert!(MemoryEffectKind::READ_WRITE.reads());
        assert!(MemoryEffectKind::READ_WRITE.writes());
    }

    #[test]
    fn test_union_memory_effect_results() {
        assert_eq!(
            MemoryEffectKind::READ.union(MemoryEffectKind::WRITE),
            MemoryEffectKind::READ_WRITE
        );
        assert_eq!(
            MemoryEffectKind::NONE.union(MemoryEffectKind::READ),
            MemoryEffectKind::READ
        );
        assert_eq!(
            MemoryEffectKind::READ_WRITE.union(MemoryEffectKind::READ_WRITE),
            MemoryEffectKind::READ_WRITE
        );
    }

    #[test]
    fn test_build_memory_effect_results() {
        assert_eq!(
            MemoryEffectKind::from_flags(false, false),
            MemoryEffectKind::NONE
        );
        assert_eq!(
            MemoryEffectKind::from_flags(true, false),
            MemoryEffectKind::READ
        );
        assert_eq!(
            MemoryEffectKind::from_flags(false, true),
            MemoryEffectKind::WRITE
        );
        assert_eq!(
            MemoryEffectKind::from_flags(true, true),
            MemoryEffectKind::READ_WRITE
        );
    }
}
