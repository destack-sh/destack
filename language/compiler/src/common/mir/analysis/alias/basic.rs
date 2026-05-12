use destack_mir as mir;

use crate::common::mir::{
    DecomposedPointer, MemoryLocation, PointerBase, PointerDecomposer, RangeRelation, TypeContext,
    ValueTypeMap, alias_scopes_may_alias, range_relation, type_alias_tags_may_alias,
};

use super::common::FunctionAA;
use super::result::{AliasResult, ModRefInfo, ParameterAttributes};

/// Basic alias analysis using pointer provenance and offset tracking.
///
/// This is the primary alias analysis that handles pointer decomposition into base and offset.
/// It returns NoAlias for different identified bases.
/// It returns NoAlias for non overlapping offsets within the same base.
/// It returns NoAlias for noalias function parameters.
/// It returns MustAlias for identical pointers.
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct BasicAA {
    /// Common function information (constants, definitions, parameters).
    function: FunctionAA,
    /// Parameter attributes (noalias, nocapture, etc.).
    param_attrs: Vec<ParameterAttributes>,
    /// Whether strict borrow mode is enabled.
    strict_borrow_mode: bool,
    /// Value type map for pointer decomposition.
    value_types: ValueTypeMap,
    /// Type context for layout sensitive operations.
    type_context: TypeContext,
}

impl BasicAA {
    /// Build BasicAA for a function.
    pub(super) fn build(
        function: &mir::Function,
        tree: &mir::Tree,
        strict_borrow_mode: bool,
        value_types: &ValueTypeMap,
        type_context: TypeContext,
    ) -> Self {
        let info = FunctionAA::collect(function, tree);

        // derive parameter attributes from mir metadata
        let param_attrs = function
            .parameters
            .iter()
            .enumerate()
            .map(|(index, _)| {
                function
                    .parameter_attributes
                    .get(index)
                    .map(Self::parameter_attributes)
                    .unwrap_or(ParameterAttributes::NONE)
            })
            .collect();

        Self {
            function: info,
            param_attrs,
            strict_borrow_mode,
            value_types: value_types.clone(),
            type_context,
        }
    }

    /// Query if two memory locations alias.
    pub(super) fn alias(
        &self,
        loc_a: &MemoryLocation,
        loc_b: &MemoryLocation,
        tree: &mir::Tree,
    ) -> AliasResult {
        // same pointer is must alias
        if loc_a.ptr == loc_b.ptr {
            return self.same_pointer_alias(loc_a, loc_b);
        }

        // decompose pointers
        let mut decomposer = PointerDecomposer::new(
            &self.function.constants,
            &self.function.definitions,
            tree,
            &self.function.parameters,
            self.strict_borrow_mode,
            &self.value_types,
            self.type_context,
        );

        let ptr_a = decomposer.decompose(loc_a.ptr);
        let ptr_b = decomposer.decompose(loc_b.ptr);

        // rule 1: different identified bases cannot alias
        if self.different_identified_bases(&ptr_a.base, &ptr_b.base) {
            return AliasResult::NoAlias;
        }

        // rule 2: noalias parameters don't alias other identified objects or other noalias params
        if self.noalias_param_check(&ptr_a.base, &ptr_b.base) {
            return AliasResult::NoAlias;
        }

        // rule 3: same base, check offset based aliasing
        if self.same_base(&ptr_a.base, &ptr_b.base) {
            return self.same_base_alias(&ptr_a, loc_a, &ptr_b, loc_b);
        }

        // conservative
        AliasResult::MayAlias
    }

    /// Alias result when pointers are identical.
    fn same_pointer_alias(&self, loc_a: &MemoryLocation, loc_b: &MemoryLocation) -> AliasResult {
        match (loc_a.size, loc_b.size) {
            // both sizes known and equal: definite full overlap
            (Some(s1), Some(s2)) if s1 == s2 => AliasResult::MustAlias,

            // both sizes known but different: partial overlap
            (Some(_), Some(_)) => AliasResult::PartialAlias,

            // unknown sizes: conservative, they definitely overlap but extent unknown
            _ => AliasResult::PartialAlias,
        }
    }

    /// Check if two bases are provably different identified objects.
    fn different_identified_bases(&self, base_a: &PointerBase, base_b: &PointerBase) -> bool {
        // both must be identified
        if !base_a.is_identified() || !base_b.is_identified() {
            return false;
        }

        match (base_a, base_b) {
            // different allocation instructions
            (PointerBase::StackAlloc(a), PointerBase::StackAlloc(b)) => a != b,
            (PointerBase::Local(a), PointerBase::Local(b)) => a != b,
            (PointerBase::HeapAlloc(a), PointerBase::HeapAlloc(b)) => a != b,
            (PointerBase::RawAlloc(a), PointerBase::RawAlloc(b)) => a != b,

            // different globals
            (PointerBase::Global(a), PointerBase::Global(b)) => a != b,

            // different allocation types never alias
            (PointerBase::StackAlloc(_), PointerBase::HeapAlloc(_))
            | (PointerBase::HeapAlloc(_), PointerBase::StackAlloc(_))
            | (PointerBase::StackAlloc(_), PointerBase::RawAlloc(_))
            | (PointerBase::RawAlloc(_), PointerBase::StackAlloc(_))
            | (PointerBase::HeapAlloc(_), PointerBase::RawAlloc(_))
            | (PointerBase::RawAlloc(_), PointerBase::HeapAlloc(_))
            | (PointerBase::Local(_), PointerBase::StackAlloc(_))
            | (PointerBase::StackAlloc(_), PointerBase::Local(_))
            | (PointerBase::Local(_), PointerBase::HeapAlloc(_))
            | (PointerBase::HeapAlloc(_), PointerBase::Local(_))
            | (PointerBase::Local(_), PointerBase::RawAlloc(_))
            | (PointerBase::RawAlloc(_), PointerBase::Local(_)) => true,

            // globals vs local allocations
            (PointerBase::Global(_), PointerBase::StackAlloc(_))
            | (PointerBase::StackAlloc(_), PointerBase::Global(_))
            | (PointerBase::Global(_), PointerBase::HeapAlloc(_))
            | (PointerBase::HeapAlloc(_), PointerBase::Global(_))
            | (PointerBase::Global(_), PointerBase::RawAlloc(_))
            | (PointerBase::RawAlloc(_), PointerBase::Global(_))
            | (PointerBase::Global(_), PointerBase::Local(_))
            | (PointerBase::Local(_), PointerBase::Global(_)) => true,

            _ => false,
        }
    }

    /// Check noalias parameter aliasing rules.
    fn noalias_param_check(&self, base_a: &PointerBase, base_b: &PointerBase) -> bool {
        let a_noalias = self.base_is_noalias_param(base_a);
        let b_noalias = self.base_is_noalias_param(base_b);

        // two different noalias params don't alias each other
        if a_noalias && b_noalias {
            return base_a != base_b;
        }

        // noalias param doesn't alias identified objects
        if a_noalias && base_b.is_identified() {
            return true;
        }
        if b_noalias && base_a.is_identified() {
            return true;
        }

        false
    }

    /// Check if a pointer base is a noalias parameter.
    fn base_is_noalias_param(&self, base: &PointerBase) -> bool {
        match base {
            PointerBase::Parameter { index, noalias } => {
                if *noalias {
                    return true;
                }

                self.get_param_attrs(*index as usize).is_noalias()
            }
            _ => false,
        }
    }

    /// Check if two bases are the same.
    fn same_base(&self, base_a: &PointerBase, base_b: &PointerBase) -> bool {
        base_a == base_b
    }

    /// Alias analysis when pointers share the same base.
    fn same_base_alias(
        &self,
        ptr_a: &DecomposedPointer,
        loc_a: &MemoryLocation,
        ptr_b: &DecomposedPointer,
        loc_b: &MemoryLocation,
    ) -> AliasResult {
        // if both have only constant offsets, check range overlap
        if ptr_a.is_constant_offset() && ptr_b.is_constant_offset() {
            // check field path disjointness first
            if self.field_paths_disjoint(&ptr_a.field_path, &ptr_b.field_path) {
                return AliasResult::NoAlias;
            }

            // check byte range overlap
            if let (Some(size_a), Some(size_b)) = (loc_a.size, loc_b.size) {
                let relation =
                    range_relation(ptr_a.const_offset, size_a, ptr_b.const_offset, size_b);
                return match relation {
                    RangeRelation::Disjoint => AliasResult::NoAlias,
                    RangeRelation::Equal => AliasResult::MustAlias,
                    _ => AliasResult::PartialAlias,
                };
            }
        }

        // check field path even with variable offsets
        if self.field_paths_disjoint(&ptr_a.field_path, &ptr_b.field_path) {
            return AliasResult::NoAlias;
        }

        // with variable offsets, check if they're provably different
        if self.var_offsets_disjoint(ptr_a, ptr_b, loc_a.size, loc_b.size) {
            return AliasResult::NoAlias;
        }

        AliasResult::MayAlias
    }

    /// Check if two field paths are disjoint (different fields at some level).
    fn field_paths_disjoint(&self, path_a: &[u32], path_b: &[u32]) -> bool {
        if path_a.is_empty() || path_b.is_empty() {
            return false;
        }

        // find where paths diverge
        for (a, b) in path_a.iter().zip(path_b.iter()) {
            if a != b {
                return true; // different fields at this level
            }
        }

        // one path is prefix of the other: may alias (nested access)
        false
    }

    /// Check if variable offsets are provably different.
    fn var_offsets_disjoint(
        &self,
        ptr_a: &DecomposedPointer,
        ptr_b: &DecomposedPointer,
        size_a: Option<u64>,
        size_b: Option<u64>,
    ) -> bool {
        // require a single variable offset per pointer
        if ptr_a.var_offsets.len() != 1 || ptr_b.var_offsets.len() != 1 {
            return false;
        }

        // require matching scale
        let offset_a = &ptr_a.var_offsets[0];
        let offset_b = &ptr_b.var_offsets[0];
        if offset_a.scale != offset_b.scale {
            return false;
        }

        // require constant indices
        let Some(&idx_a) = self.function.constants.get(&offset_a.index) else {
            return false;
        };
        let Some(&idx_b) = self.function.constants.get(&offset_b.index) else {
            return false;
        };

        // use explicit sizes or fall back to element scale
        let size_a = size_a.or(Some(offset_a.scale));
        let size_b = size_b.or(Some(offset_b.scale));
        let (Some(size_a), Some(size_b)) = (size_a, size_b) else {
            return false;
        };

        // compute byte offsets and compare ranges
        let offset_a = ptr_a
            .const_offset
            .saturating_add(idx_a.saturating_mul(offset_a.scale as i64));
        let offset_b = ptr_b
            .const_offset
            .saturating_add(idx_b.saturating_mul(offset_b.scale as i64));
        let relation = range_relation(offset_a, size_a, offset_b, size_b);

        matches!(relation, RangeRelation::Disjoint)
    }

    /// Get mod/ref info for an instruction relative to a memory location.
    #[cfg(test)]
    pub(super) fn get_mod_ref_info(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        loc: &MemoryLocation,
        tree: &mir::Tree,
    ) -> ModRefInfo {
        self.get_mod_ref_info_with_metadata(instruction_id, loc, &[], &[], None, tree)
    }

    /// Get mod ref info for an instruction relative to a memory location.
    pub(super) fn get_mod_ref_info_with_metadata(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        loc: &MemoryLocation,
        query_alias_scopes: &[mir::MemoryAliasScopeId],
        query_noalias_scopes: &[mir::MemoryAliasScopeId],
        query_type_alias_tag: Option<mir::TypeAliasTagId>,
        tree: &mir::Tree,
    ) -> ModRefInfo {
        // prefer explicit memory metadata when present
        if let Some(accesses) = tree.metadata.memory.memory_accesses(instruction_id) {
            return self.mod_ref_from_metadata(
                accesses,
                loc,
                query_alias_scopes,
                query_noalias_scopes,
                query_type_alias_tag,
                tree,
            );
        }

        let inst = tree.get(instruction_id);

        match inst {
            mir::Instruction::Load { pointer, .. } => {
                let Some(pointer) = pointer.value() else {
                    return ModRefInfo::NO_MOD_REF;
                };

                let load_loc = MemoryLocation::from_ptr(pointer);
                if self.alias(&load_loc, loc, tree) == AliasResult::NoAlias {
                    ModRefInfo::NO_MOD_REF
                } else {
                    ModRefInfo::REF
                }
            }

            mir::Instruction::Store { pointer, .. } => {
                let Some(pointer) = pointer.value() else {
                    return ModRefInfo::NO_MOD_REF;
                };

                let store_loc = MemoryLocation::from_ptr(pointer);
                if self.alias(&store_loc, loc, tree) == AliasResult::NoAlias {
                    ModRefInfo::NO_MOD_REF
                } else {
                    ModRefInfo::MOD
                }
            }
            mir::Instruction::AtomicLoad { pointer, .. } => {
                let Some(pointer) = pointer.value() else {
                    return ModRefInfo::NO_MOD_REF;
                };

                let load_loc = MemoryLocation::from_ptr(pointer);
                if self.alias(&load_loc, loc, tree) == AliasResult::NoAlias {
                    ModRefInfo::NO_MOD_REF
                } else {
                    ModRefInfo::REF
                }
            }
            mir::Instruction::AtomicStore { pointer, .. } => {
                let Some(pointer) = pointer.value() else {
                    return ModRefInfo::NO_MOD_REF;
                };

                let store_loc = MemoryLocation::from_ptr(pointer);
                if self.alias(&store_loc, loc, tree) == AliasResult::NoAlias {
                    ModRefInfo::NO_MOD_REF
                } else {
                    ModRefInfo::MOD
                }
            }
            mir::Instruction::AtomicCompareExchange { pointer, .. }
            | mir::Instruction::AtomicRmw { pointer, .. } => {
                let Some(pointer) = pointer.value() else {
                    return ModRefInfo::NO_MOD_REF;
                };

                let access_loc = MemoryLocation::from_ptr(pointer);
                if self.alias(&access_loc, loc, tree) == AliasResult::NoAlias {
                    ModRefInfo::NO_MOD_REF
                } else {
                    ModRefInfo::MOD_REF
                }
            }
            mir::Instruction::AtomicFence { .. } => ModRefInfo::NO_MOD_REF,
            mir::Instruction::BarrierWrite { .. } => ModRefInfo::MOD,

            mir::Instruction::Call { .. }
            | mir::Instruction::CallClass { .. }
            | mir::Instruction::CallInterface { .. }
            | mir::Instruction::CallIndirect { .. } => {
                self.get_call_mod_ref(instruction_id, inst, loc, tree)
            }

            // intrinsics that access memory
            mir::Instruction::Intrinsic { intrinsic, .. } => {
                self.get_intrinsic_mod_ref(intrinsic, loc)
            }

            // allocations don't alias existing spaces
            mir::Instruction::New { .. }
            | mir::Instruction::NewSlice { .. }
            | mir::Instruction::RawAlloc { .. }
            | mir::Instruction::StackAlloc { .. } => ModRefInfo::NO_MOD_REF,

            // deallocation only affects the freed memory
            mir::Instruction::RawFree { pointer } | mir::Instruction::Free { value: pointer } => {
                let Some(pointer) = pointer.value() else {
                    return ModRefInfo::NO_MOD_REF;
                };

                let free_loc = MemoryLocation::from_ptr(pointer);
                if self.alias(&free_loc, loc, tree) == AliasResult::NoAlias {
                    ModRefInfo::NO_MOD_REF
                } else {
                    ModRefInfo::MOD
                }
            }

            // pure instructions don't access memory
            _ => ModRefInfo::NO_MOD_REF,
        }
    }

    /// Compute mod ref info for memory metadata entries.
    fn mod_ref_from_metadata(
        &self,
        accesses: &[mir::MemoryAccessMetadata],
        loc: &MemoryLocation,
        query_alias_scopes: &[mir::MemoryAliasScopeId],
        query_noalias_scopes: &[mir::MemoryAliasScopeId],
        query_type_alias_tag: Option<mir::TypeAliasTagId>,
        tree: &mir::Tree,
    ) -> ModRefInfo {
        if accesses.is_empty() {
            return ModRefInfo::NO_MOD_REF;
        }

        let mut result = ModRefInfo::NO_MOD_REF;

        for access in accesses {
            if !self.metadata_may_alias(
                access,
                loc,
                query_alias_scopes,
                query_noalias_scopes,
                query_type_alias_tag,
                tree,
            ) {
                continue;
            }

            let access_mod_ref = match access.kind {
                mir::MemoryAccessKind::Read | mir::MemoryAccessKind::PrefetchRead => {
                    ModRefInfo::REF
                }
                mir::MemoryAccessKind::Write => ModRefInfo::MOD,
                mir::MemoryAccessKind::ReadWrite | mir::MemoryAccessKind::ReadModifyWrite => {
                    ModRefInfo::MOD_REF
                }
                mir::MemoryAccessKind::PrefetchWrite => ModRefInfo::REF,
                mir::MemoryAccessKind::Fence => ModRefInfo::MOD_REF,
            };

            result = result.union(access_mod_ref);
        }

        result
    }

    /// Check whether a memory metadata entry may alias a location.
    fn metadata_may_alias(
        &self,
        access: &mir::MemoryAccessMetadata,
        loc: &MemoryLocation,
        query_alias_scopes: &[mir::MemoryAliasScopeId],
        query_noalias_scopes: &[mir::MemoryAliasScopeId],
        query_type_alias_tag: Option<mir::TypeAliasTagId>,
        tree: &mir::Tree,
    ) -> bool {
        if !self.space_sets_overlap(access, loc, tree) {
            return false;
        }

        if !self.address_spaces_overlap(access, loc, tree) {
            return false;
        }

        if !alias_scopes_may_alias(
            &access.alias_scopes,
            &access.noalias_scopes,
            query_alias_scopes,
            query_noalias_scopes,
        ) {
            return false;
        }

        if !type_alias_tags_may_alias(
            &tree.metadata.memory.type_alias,
            access.type_alias_tag,
            query_type_alias_tag,
        ) {
            return false;
        }

        match access.target {
            mir::MemoryAccessTarget::Pointer(pointer) => {
                let access_loc = MemoryLocation::new(pointer, access.size, None, None, None);
                !self.alias(&access_loc, loc, tree).is_no_alias()
            }
            mir::MemoryAccessTarget::Local(local) => {
                let mut decomposer = PointerDecomposer::new(
                    &self.function.constants,
                    &self.function.definitions,
                    tree,
                    &self.function.parameters,
                    self.strict_borrow_mode,
                    &self.value_types,
                    self.type_context,
                );
                let ptr = decomposer.decompose(loc.ptr);
                let access_base = PointerBase::Local(local);
                !self.different_identified_bases(&access_base, &ptr.base)
            }
            mir::MemoryAccessTarget::Global(global) => {
                let mut decomposer = PointerDecomposer::new(
                    &self.function.constants,
                    &self.function.definitions,
                    tree,
                    &self.function.parameters,
                    self.strict_borrow_mode,
                    &self.value_types,
                    self.type_context,
                );
                let ptr = decomposer.decompose(loc.ptr);
                let access_base = PointerBase::Global(global);
                !self.different_identified_bases(&access_base, &ptr.base)
            }
            mir::MemoryAccessTarget::Unknown => true,
        }
    }

    /// Check location set overlap for metadata.
    fn space_sets_overlap(
        &self,
        access: &mir::MemoryAccessMetadata,
        loc: &MemoryLocation,
        tree: &mir::Tree,
    ) -> bool {
        let Some(loc_set) = self.space_set_for_location(loc, tree) else {
            return true;
        };

        let access_set = self.space_set_for_access(access, tree);
        access_set.intersects(loc_set)
    }

    /// Check address space overlap for metadata.
    fn address_spaces_overlap(
        &self,
        access: &mir::MemoryAccessMetadata,
        loc: &MemoryLocation,
        tree: &mir::Tree,
    ) -> bool {
        let Some(loc_space) = self.location_address_space(loc, tree) else {
            return true;
        };

        let Some(access_space) = self.address_space_for_access(access, tree) else {
            return true;
        };

        loc_space == access_space
    }

    /// Resolve the coarse location set for a metadata access.
    fn space_set_for_access(
        &self,
        access: &mir::MemoryAccessMetadata,
        tree: &mir::Tree,
    ) -> mir::MemorySpaceSet {
        if let Some(address_space) = access.address_space.clone() {
            return self.space_set_for_address_space(address_space);
        }

        match access.target {
            mir::MemoryAccessTarget::Pointer(pointer) => {
                let access_loc = MemoryLocation::from_ptr(pointer);
                self.space_set_for_location(&access_loc, tree)
                    .unwrap_or(mir::MemorySpaceSet::ANY)
            }
            mir::MemoryAccessTarget::Local(_) => mir::MemorySpaceSet::STACK,
            mir::MemoryAccessTarget::Global(global) => {
                self.space_set_for_address_space(tree.get(global).space.clone())
            }
            mir::MemoryAccessTarget::Unknown => mir::MemorySpaceSet::ANY,
        }
    }

    /// Resolve the address space for a metadata access.
    fn address_space_for_access(
        &self,
        access: &mir::MemoryAccessMetadata,
        tree: &mir::Tree,
    ) -> Option<mir::AddressSpace> {
        if let Some(address_space) = access.address_space.clone() {
            return Some(address_space);
        }

        match access.target {
            mir::MemoryAccessTarget::Pointer(pointer) => {
                let access_loc = MemoryLocation::from_ptr(pointer);
                self.location_address_space(&access_loc, tree)
            }
            mir::MemoryAccessTarget::Local(_) => Some(mir::AddressSpace::Stack),
            mir::MemoryAccessTarget::Global(global) => Some(tree.get(global).space.clone()),
            mir::MemoryAccessTarget::Unknown => None,
        }
    }

    /// Map address spaces to coarse location sets.
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

    /// Get mod/ref for a call instruction.
    fn get_call_mod_ref(
        &self,
        _instruction_id: mir::LocalNodeId<mir::Instruction>,
        inst: &mir::Instruction,
        loc: &MemoryLocation,
        tree: &mir::Tree,
    ) -> ModRefInfo {
        // read callsite effects when present
        // read call memory effects from metadata or callee
        let mut memory_effects = inst
            .call_memory_effect()
            .cloned()
            .or_else(|| self.callee_memory_effects(inst, tree));

        // fall back to conservative behavior without effects
        let Some(effects) = memory_effects.take() else {
            return ModRefInfo::MOD_REF;
        };

        // calls with no memory effects are pure
        if !effects.reads && !effects.writes {
            return ModRefInfo::NO_MOD_REF;
        }

        // inaccessible memory does not alias normal spaces
        if effects.inaccessible_mem_only {
            return ModRefInfo::NO_MOD_REF;
        }

        // honor address space restrictions when known
        if let Some(address_spaces) = &effects.address_spaces
            && let Some(location_space) = self.location_address_space(loc, tree)
            && !address_spaces.contains(location_space)
        {
            return ModRefInfo::NO_MOD_REF;
        }

        // argmemonly calls only touch pointer arguments
        if effects.argmemonly {
            return self.argmemonly_mod_ref(inst, loc, tree, &effects);
        }

        // honor coarse location set restrictions when possible
        if let Some(space_set) = self.space_set_for_location(loc, tree)
            && !effects.spaces.contains(space_set)
        {
            return ModRefInfo::NO_MOD_REF;
        }

        // return the summarized mod ref info
        ModRefInfo::from_flags(effects.reads, effects.writes)
    }

    /// Refine mod ref for argmemonly calls.
    fn argmemonly_mod_ref(
        &self,
        inst: &mir::Instruction,
        loc: &MemoryLocation,
        tree: &mir::Tree,
        effects: &mir::MemoryEffect,
    ) -> ModRefInfo {
        // honor coarse location sets when a real region restriction exists
        if !effects.spaces.is_empty()
            && let Some(space_set) = self.space_set_for_location(loc, tree)
            && !effects.spaces.contains(space_set)
        {
            return ModRefInfo::NO_MOD_REF;
        }

        // load call arguments
        let Some(arg_slice) = inst.argument_slice() else {
            return ModRefInfo::from_flags(effects.reads, effects.writes);
        };

        // handle empty argument lists
        let args = tree.get_arguments(arg_slice);
        if args.is_empty() {
            return ModRefInfo::NO_MOD_REF;
        }

        // resolve argument types
        let arg_types = self.call_argument_types(inst, tree);
        let Some(arg_types) = arg_types else {
            return ModRefInfo::from_flags(effects.reads, effects.writes);
        };

        // collect mod ref info based on aliased arguments
        let mut result = ModRefInfo::NO_MOD_REF;
        for (index, &arg_value) in args.iter().enumerate() {
            // read the argument type
            let arg_type = match arg_types.get(index) {
                Some(ty) => *ty,
                None => continue,
            };

            // skip non reference arguments
            match tree.get(arg_type) {
                mir::Type::Reference { .. } | mir::Type::TensorView { .. } => {}
                _ => continue,
            }

            // read argument metadata
            let arg_attribute = inst
                .call_argument_attributes()
                .and_then(|argument_attributes| argument_attributes.get(index))
                .cloned()
                .unwrap_or_default();

            // clamp the argument access to call effects
            let arg_access = self.clamp_argument_access(arg_attribute.access, effects);
            if arg_access == mir::ArgumentAccess::None {
                continue;
            }

            // build a location for the argument pointer
            let Some(arg_value) = arg_value.value() else {
                continue;
            };
            let arg_loc = if let Some(size) = arg_attribute
                .attributes
                .dereferenceable_bytes
                .or(arg_attribute.attributes.dereferenceable_or_null_bytes)
            {
                MemoryLocation::with_size(arg_value, size)
            } else {
                MemoryLocation::from_ptr(arg_value)
            };

            // skip arguments that do not alias the location
            if self.alias(&arg_loc, loc, tree).is_no_alias() {
                continue;
            }

            // merge the access into the final mod ref info
            result = result.union(self.mod_ref_from_access(arg_access));
        }

        result
    }

    /// Convert pointer attributes into basic alias parameter attributes.
    fn parameter_attributes(attrs: &mir::PointerAttribute) -> ParameterAttributes {
        let mut result = ParameterAttributes::NONE;

        // apply noalias when guaranteed
        if attrs.noalias {
            result = result.with(ParameterAttributes::noalias());
        }

        // apply nocapture when guaranteed
        if attrs.capture == mir::CaptureKind::NoCapture {
            result = result.with(ParameterAttributes::nocapture());
        }

        // apply readonly or writeonly when present
        if attrs.readonly {
            result = result.with(ParameterAttributes::readonly());
        }
        if attrs.writeonly {
            result = result.with(ParameterAttributes::writeonly());
        }

        result
    }

    /// Clamp argument access based on call memory effects.
    fn clamp_argument_access(
        &self,
        access: mir::ArgumentAccess,
        effects: &mir::MemoryEffect,
    ) -> mir::ArgumentAccess {
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

    /// Convert argument access into ModRefInfo.
    fn mod_ref_from_access(&self, access: mir::ArgumentAccess) -> ModRefInfo {
        // map access flags to mod ref info
        match access {
            mir::ArgumentAccess::None => ModRefInfo::NO_MOD_REF,
            mir::ArgumentAccess::Read => ModRefInfo::REF,
            mir::ArgumentAccess::Write => ModRefInfo::MOD,
            mir::ArgumentAccess::ReadWrite => ModRefInfo::MOD_REF,
        }
    }

    /// Resolve call argument types for direct calls and metadata signatures.
    fn call_argument_types(
        &self,
        inst: &mir::Instruction,
        tree: &mir::Tree,
    ) -> Option<Vec<mir::LocalNodeId<mir::Type>>> {
        // use the instruction signature when available
        let signature = inst.call_signature()?.ty()?;
        let signature = tree.get(signature);
        let mir::Type::FunctionSignature { parameters, .. } = signature else {
            return None;
        };

        parameters
            .iter()
            .copied()
            .map(|parameter| parameter.ty())
            .collect()
    }

    /// Resolve memory effects from a direct callee when present.
    fn callee_memory_effects(
        &self,
        inst: &mir::Instruction,
        tree: &mir::Tree,
    ) -> Option<mir::MemoryEffect> {
        // resolve the declared target when available
        let function = inst.call_declared_target()?.function()?;
        let callee = tree.get(function);
        Some(callee.memory_effect.clone())
    }

    /// Resolve a coarse memory space set for a pointer location.
    fn space_set_for_location(
        &self,
        loc: &MemoryLocation,
        tree: &mir::Tree,
    ) -> Option<mir::MemorySpaceSet> {
        // compute pointer base for space classification
        let mut decomposer = PointerDecomposer::new(
            &self.function.constants,
            &self.function.definitions,
            tree,
            &self.function.parameters,
            self.strict_borrow_mode,
            &self.value_types,
            self.type_context,
        );
        let decomposed = decomposer.decompose(loc.ptr);

        // map known bases to memory space sets
        match decomposed.base {
            PointerBase::StackAlloc(_) | PointerBase::Local(_) => Some(mir::MemorySpaceSet::STACK),
            PointerBase::HeapAlloc(_) => Some(mir::MemorySpaceSet::HEAP),
            PointerBase::RawAlloc(_) => Some(mir::MemorySpaceSet::RAW_HEAP),
            PointerBase::Global(global) => {
                Some(self.space_set_for_address_space(tree.get(global).space.clone()))
            }
            _ => None,
        }
    }

    /// Resolve an address space for a pointer location when possible.
    fn location_address_space(
        &self,
        loc: &MemoryLocation,
        tree: &mir::Tree,
    ) -> Option<mir::AddressSpace> {
        // compute pointer base for address space inference
        let mut decomposer = PointerDecomposer::new(
            &self.function.constants,
            &self.function.definitions,
            tree,
            &self.function.parameters,
            self.strict_borrow_mode,
            &self.value_types,
            self.type_context,
        );
        let decomposed = decomposer.decompose(loc.ptr);

        // map known bases to address spaces
        match decomposed.base {
            PointerBase::StackAlloc(_) | PointerBase::Local(_) => Some(mir::AddressSpace::Stack),
            PointerBase::HeapAlloc(_) | PointerBase::RawAlloc(_) => Some(mir::AddressSpace::Local),
            PointerBase::Global(global) => Some(tree.get(global).space.clone()),
            PointerBase::Parameter { index, .. } => {
                // extract address space from parameter type when possible
                let parameter = self.function.parameters.get(index as usize)?;
                let ty = parameter.ty.ty()?;

                match tree.get(ty) {
                    mir::Type::Reference { address_space, .. }
                    | mir::Type::TensorView { address_space, .. } => Some(address_space.clone()),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Get mod/ref for an intrinsic.
    fn get_intrinsic_mod_ref(
        &self,
        intrinsic: &mir::Intrinsic,
        _loc: &MemoryLocation,
    ) -> ModRefInfo {
        use mir::Intrinsic;

        match intrinsic {
            // memory operations
            Intrinsic::Memcpy | Intrinsic::Memmove | Intrinsic::Memset => ModRefInfo::MOD_REF,

            // pure intrinsics
            _ => ModRefInfo::NO_MOD_REF,
        }
    }

    /// Get parameter attributes.
    #[allow(dead_code)]
    pub(super) fn get_param_attrs(&self, param_idx: usize) -> ParameterAttributes {
        self.param_attrs
            .get(param_idx)
            .copied()
            .unwrap_or(ParameterAttributes::NONE)
    }

    /// Check if a value is derived from a function argument.
    #[allow(dead_code)]
    pub(super) fn is_arg_derived(&self, value: mir::Value, tree: &mir::Tree) -> bool {
        let mut decomposer = PointerDecomposer::new(
            &self.function.constants,
            &self.function.definitions,
            tree,
            &self.function.parameters,
            self.strict_borrow_mode,
            &self.value_types,
            self.type_context,
        );

        let decomp = decomposer.decompose(value);
        matches!(decomp.base, PointerBase::Parameter { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::mir::{TypeContext, ValueTypeMap};
    use crate::optimize::common::tests::TestProgram;

    /// Build a BasicAA instance for a test function.
    fn build_basic_aa(
        function: &mir::Function,
        program: &TestProgram,
        strict_borrow_mode: bool,
    ) -> BasicAA {
        let value_types = ValueTypeMap::new(function, &program.tree);
        BasicAA::build(
            function,
            &program.tree,
            strict_borrow_mode,
            &value_types,
            TypeContext::default(),
        )
    }

    #[test]
    fn test_different_allocations_no_alias() {
        let program = TestProgram::new(
            r#"
type Point {
    int32;
    int32;
}
function test(): void {
b0:
    v0: ref<Point, managed> = new Point
    v1: ref<Point, managed> = new Point
    v2: int32 = 1int32
    store v0, v2
    store v1, v2
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = build_basic_aa(function, &program, false);

        let loc0 = MemoryLocation::from_ptr(mir::Value::new(0));
        let loc1 = MemoryLocation::from_ptr(mir::Value::new(1));

        assert_eq!(aa.alias(&loc0, &loc1, &program.tree), AliasResult::NoAlias);
    }

    #[test]
    fn test_memory_metadata_disambiguates_intrinsic() {
        let mut program = TestProgram::new(
            r#"
function test(): void {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int8 = 0int8
    v3: int64 = 4int64
    intrinsic.memset(v1, v2, v3)
    return
}"#,
        );

        let function_id = program.entry_function_id();
        let pointers = program.stack_alloc_destinations_in_entry(function_id);
        let memset_inst = program.first_intrinsic_in_entry(function_id, mir::Intrinsic::Memset);

        assert_eq!(pointers.len(), 2);

        let pointer_target = pointers[1];
        let pointer_query = pointers[0];

        program.insert_pointer_access(
            memset_inst,
            mir::MemoryAccessKind::Write,
            pointer_target,
            Some(4),
            Vec::new(),
            Vec::new(),
            None,
        );

        let function = program.tree.get(function_id);
        let aa = build_basic_aa(function, &program, false);
        let loc = MemoryLocation::from_ptr(pointer_query);
        let mod_ref = aa.get_mod_ref_info(memset_inst, &loc, &program.tree);

        assert_eq!(mod_ref, ModRefInfo::NO_MOD_REF);
    }

    #[test]
    fn test_memory_metadata_respects_alias_scopes() {
        let mut program = TestProgram::new(
            r#"
function test(v0: ref<int32, raw>, v1: ref<int32, raw>): void {
b0(v0: ref<int32, raw>, v1: ref<int32, raw>):
    v2: int8 = 0int8
    v3: int64 = 4int64
    intrinsic.memset(v1, v2, v3)
    return
}"#,
        );

        let function_id = program.entry_function_id();
        let memset_inst = program.first_intrinsic_in_entry(function_id, mir::Intrinsic::Memset);
        let scope = program.create_alias_scope();

        program.insert_pointer_access(
            memset_inst,
            mir::MemoryAccessKind::Write,
            mir::Value::new(1),
            Some(4),
            Vec::new(),
            vec![scope],
            None,
        );

        let function = program.tree.get(function_id);
        let aa = build_basic_aa(function, &program, false);
        let loc0 = MemoryLocation::from_ptr(mir::Value::new(0));
        let loc1 = MemoryLocation::from_ptr(mir::Value::new(1));

        assert_eq!(aa.alias(&loc0, &loc1, &program.tree), AliasResult::MayAlias);

        let mod_ref = aa.get_mod_ref_info_with_metadata(
            memset_inst,
            &loc0,
            &[scope],
            &[],
            None,
            &program.tree,
        );

        assert_eq!(mod_ref, ModRefInfo::NO_MOD_REF);
    }

    #[test]
    fn test_memory_metadata_respects_tbaa_tags() {
        let mut program = TestProgram::new(
            r#"
function test(v0: ref<int32, raw>, v1: ref<int32, raw>): void {
b0(v0: ref<int32, raw>, v1: ref<int32, raw>):
    v2: int8 = 0int8
    v3: int64 = 4int64
    intrinsic.memset(v1, v2, v3)
    return
}"#,
        );

        let function_id = program.entry_function_id();
        let memset_inst = program.first_intrinsic_in_entry(function_id, mir::Intrinsic::Memset);
        let int_node = program.create_type_alias_node(None, false);
        let float_node = program.create_type_alias_node(None, false);
        let int_tag = program.create_type_alias_tag(int_node, int_node, 0, 4, false);
        let float_tag = program.create_type_alias_tag(float_node, float_node, 0, 4, false);

        program.insert_pointer_access(
            memset_inst,
            mir::MemoryAccessKind::Write,
            mir::Value::new(1),
            Some(4),
            Vec::new(),
            Vec::new(),
            Some(int_tag),
        );

        let function = program.tree.get(function_id);
        let aa = build_basic_aa(function, &program, false);
        let loc0 = MemoryLocation::from_ptr(mir::Value::new(0));
        let loc1 = MemoryLocation::from_ptr(mir::Value::new(1));

        assert_eq!(aa.alias(&loc0, &loc1, &program.tree), AliasResult::MayAlias);

        let mod_ref = aa.get_mod_ref_info_with_metadata(
            memset_inst,
            &loc0,
            &[],
            &[],
            Some(float_tag),
            &program.tree,
        );

        assert_eq!(mod_ref, ModRefInfo::NO_MOD_REF);
    }

    #[test]
    fn test_memory_metadata_local_target_aliases_stack_pointer() {
        let mut program = TestProgram::new(
            r#"
function test(): void {
    local local0: int32, owned
b0:
    v0: ref<int32, raw, space(frame)> = local.address local0
    v1: int8 = 0int8
    v2: int64 = 4int64
    intrinsic.memset(v0, v1, v2)
    return
}"#,
        );

        let function_id = program.entry_function_id();
        let local_id = *program
            .tree
            .get(function_id)
            .locals
            .first()
            .expect("missing local");
        let memset_inst = program.first_intrinsic_in_entry(function_id, mir::Intrinsic::Memset);

        let pointer_value = program
            .tree
            .get(program.entry_block_id(function_id))
            .instructions
            .iter()
            .find_map(|&instruction_id| match program.tree.get(instruction_id) {
                mir::Instruction::LocalAddr { destination, .. } => Some(*destination),
                _ => None,
            })
            .expect("missing local.address");

        program.tree.metadata.memory.insert_memory_accesses(
            memset_inst,
            vec![mir::MemoryAccessMetadata {
                kind: mir::MemoryAccessKind::Write,
                target: mir::MemoryAccessTarget::Local(local_id),
                size: Some(4),
                alignment: None,
                is_volatile: false,
                is_load_invariant: false,
                ordering: None,
                scope: None,
                memory_scope: None,
                flags: None,
                address_space: None,
                alias_scopes: Vec::new(),
                noalias_scopes: Vec::new(),
                type_alias_tag: None,
            }],
        );

        let function = program.tree.get(function_id);
        let aa = build_basic_aa(function, &program, false);
        let loc = MemoryLocation::from_ptr(
            pointer_value
                .value()
                .expect("pointer value should be concrete"),
        );
        let mod_ref = aa.get_mod_ref_info(memset_inst, &loc, &program.tree);

        assert!(mod_ref.is_mod());
    }

    #[test]
    fn test_same_pointer_must_alias() {
        let program = TestProgram::new(
            r#"
type Point {
    int32;
    int32;
}
function test(): void {
b0:
    v0: ref<Point, managed> = new Point
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = build_basic_aa(function, &program, false);

        let loc = MemoryLocation::with_size(mir::Value::new(0), 8);

        assert_eq!(aa.alias(&loc, &loc, &program.tree), AliasResult::MustAlias);
    }

    #[test]
    fn test_different_fields_no_alias() {
        let program = TestProgram::new(
            r#"
type Point {
    int32;
    int32;
}
function test(): void {
b0:
    v0: ref<Point, managed> = new Point
    v1: ref<int32, borrowed> = field.address v0, 0
    v2: ref<int32, borrowed> = field.address v0, 1
    v3: int32 = 1int32
    store v1, v3
    store v2, v3
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = build_basic_aa(function, &program, false);

        let loc1 = MemoryLocation::from_ptr(mir::Value::new(1));
        let loc2 = MemoryLocation::from_ptr(mir::Value::new(2));

        assert_eq!(aa.alias(&loc1, &loc2, &program.tree), AliasResult::NoAlias);
    }

    #[test]
    fn test_stack_vs_heap_no_alias() {
        let program = TestProgram::new(
            r#"
function test(): void {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, managed> = new int32
    v2: int32 = 1int32
    store v0, v2
    store v1, v2
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = build_basic_aa(function, &program, false);

        let loc0 = MemoryLocation::from_ptr(mir::Value::new(0));
        let loc1 = MemoryLocation::from_ptr(mir::Value::new(1));

        assert_eq!(aa.alias(&loc0, &loc1, &program.tree), AliasResult::NoAlias);
    }

    #[test]
    fn test_global_vs_local_no_alias() {
        let program = TestProgram::new(
            r#"
global g: int32 = 0int32
function test(): void {
b0:
    v0: ref<int32, raw, space(static)> = global.address g
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 1int32
    store v0, v2
    store v1, v2
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = build_basic_aa(function, &program, false);

        let loc0 = MemoryLocation::from_ptr(mir::Value::new(0));
        let loc1 = MemoryLocation::from_ptr(mir::Value::new(1));

        assert_eq!(aa.alias(&loc0, &loc1, &program.tree), AliasResult::NoAlias);
    }

    #[test]
    fn test_different_elements_no_alias() {
        let program = TestProgram::new(
            r#"
type Arr int32[10]
function test(): void {
b0:
    v0: ref<Arr, raw, space(stack)> = stack.alloc Arr
    v1: int64 = 0int64
    v2: int64 = 1int64
    v3: ref<int32, borrowed> = element.address v0, v1
    v4: ref<int32, borrowed> = element.address v0, v2
    v5: int32 = 42int32
    store v3, v5
    store v4, v5
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = build_basic_aa(function, &program, false);

        let loc3 = MemoryLocation::from_ptr(mir::Value::new(3));
        let loc4 = MemoryLocation::from_ptr(mir::Value::new(4));

        assert_eq!(aa.alias(&loc3, &loc4, &program.tree), AliasResult::NoAlias);
    }

    #[test]
    fn test_parameters_may_alias() {
        let program = TestProgram::new(
            r#"
function test(v0: ref<int32, raw>, v1: ref<int32, raw>): void {
b0(v0: ref<int32, raw>, v1: ref<int32, raw>):
    v2: int32 = 1int32
    store v0, v2
    store v1, v2
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = build_basic_aa(function, &program, false);

        let loc0 = MemoryLocation::from_ptr(mir::Value::new(0));
        let loc1 = MemoryLocation::from_ptr(mir::Value::new(1));

        // without noalias, params may alias
        assert_eq!(aa.alias(&loc0, &loc1, &program.tree), AliasResult::MayAlias);
    }

    #[test]
    fn test_noalias_parameters_no_alias() {
        let mut program = TestProgram::new(
            r#"
function test(v0: ref<int32, raw>, v1: ref<int32, raw>): void {
b0(v0: ref<int32, raw>, v1: ref<int32, raw>):
    v2: int32 = 1int32
    store v0, v2
    store v1, v2
    return
}"#,
        );

        let function_id = program.entry_function_id();
        {
            let function = program.tree.get_mut(function_id);
            function.parameter_attributes[0].noalias = true;
            function.parameter_attributes[1].noalias = true;
        }

        let function = program.tree.get(function_id);
        let aa = build_basic_aa(function, &program, false);

        let loc0 = MemoryLocation::from_ptr(mir::Value::new(0));
        let loc1 = MemoryLocation::from_ptr(mir::Value::new(1));

        assert_eq!(aa.alias(&loc0, &loc1, &program.tree), AliasResult::NoAlias);
    }

    #[test]
    fn test_same_pointer_different_sizes_partial_alias() {
        // same pointer but different access sizes should be PartialAlias
        let program = TestProgram::new(
            r#"
function test(): void {
b0:
    v0: ref<int64, raw, space(stack)> = stack.alloc int64
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = build_basic_aa(function, &program, false);

        // same pointer, different sizes
        let loc_small = MemoryLocation::with_size(mir::Value::new(0), 4);
        let loc_large = MemoryLocation::with_size(mir::Value::new(0), 8);

        assert_eq!(
            aa.alias(&loc_small, &loc_large, &program.tree),
            AliasResult::PartialAlias
        );
    }

    #[test]
    fn test_same_pointer_unknown_size_partial_alias() {
        // same pointer with unknown size should be PartialAlias (conservative)
        let program = TestProgram::new(
            r#"
function test(): void {
b0:
    v0: ref<int64, raw, space(stack)> = stack.alloc int64
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = build_basic_aa(function, &program, false);

        // same pointer, one unknown size
        let loc_known = MemoryLocation::with_size(mir::Value::new(0), 4);
        let loc_unknown = MemoryLocation::from_ptr(mir::Value::new(0));

        assert_eq!(
            aa.alias(&loc_known, &loc_unknown, &program.tree),
            AliasResult::PartialAlias
        );
    }

    #[test]
    fn test_raw_vs_heap_alloc_no_alias() {
        // different allocation types never alias
        let program = TestProgram::new(
            r#"
function test(): void {
b0:
    v0: ref<int64, raw> = raw.alloc int64
    v1: ref<int64, managed> = new int64
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = build_basic_aa(function, &program, false);

        let loc0 = MemoryLocation::from_ptr(mir::Value::new(0));
        let loc1 = MemoryLocation::from_ptr(mir::Value::new(1));

        assert_eq!(aa.alias(&loc0, &loc1, &program.tree), AliasResult::NoAlias);
    }

    #[test]
    fn test_cast_preserves_provenance() {
        // cast should preserve pointer provenance
        let program = TestProgram::new(
            r#"
function test(): void {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: ref<int8, raw> = cast.bit v0 -> ref<int8, raw>
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = build_basic_aa(function, &program, false);

        // v0 and v2 alias (v2 is just a cast of v0)
        let loc0 = MemoryLocation::from_ptr(mir::Value::new(0));
        let loc2 = MemoryLocation::from_ptr(mir::Value::new(2));

        // should be PartialAlias or MayAlias (same base, but accessed through cast)
        assert!(aa.alias(&loc0, &loc2, &program.tree).may_alias());

        // v1 and v2 should not alias (different allocations)
        let loc1 = MemoryLocation::from_ptr(mir::Value::new(1));
        assert_eq!(aa.alias(&loc1, &loc2, &program.tree), AliasResult::NoAlias);
    }

    #[test]
    fn test_same_element_same_index_may_alias() {
        // same array with same variable index should may alias
        let program = TestProgram::new(
            r#"
type Arr int32[10]
function test(v0: int64): void {
b0(v0: int64):
    v1: ref<Arr, raw, space(stack)> = stack.alloc Arr
    v2: ref<int32, borrowed> = element.address v1, v0
    v3: ref<int32, borrowed> = element.address v1, v0
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = build_basic_aa(function, &program, false);

        let loc2 = MemoryLocation::from_ptr(mir::Value::new(2));
        let loc3 = MemoryLocation::from_ptr(mir::Value::new(3));

        // same index variable means they alias
        assert!(aa.alias(&loc2, &loc3, &program.tree).may_alias());
    }

    #[test]
    fn test_nested_field_access() {
        // nested struct field access
        let program = TestProgram::new(
            r#"
type Inner {
    int32;
    int32;
}
type Outer {
    Inner;
    Inner;
}
function test(): void {
b0:
    v0: ref<Outer, raw, space(stack)> = stack.alloc Outer
    v1: ref<Inner, borrowed> = field.address v0, 0
    v2: ref<Inner, borrowed> = field.address v0, 1
    v3: ref<int32, borrowed> = field.address v1, 0
    v4: ref<int32, borrowed> = field.address v2, 0
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = build_basic_aa(function, &program, false);

        // v3 is outer.0.0, v4 is outer.1.0, different top level fields
        let loc3 = MemoryLocation::from_ptr(mir::Value::new(3));
        let loc4 = MemoryLocation::from_ptr(mir::Value::new(4));

        assert_eq!(aa.alias(&loc3, &loc4, &program.tree), AliasResult::NoAlias);
    }
}
