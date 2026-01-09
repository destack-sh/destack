use destack_mir as mir;

use crate::optimize::common::{
    DecomposedPointer, MemoryLocation, PointerBase, PointerDecomposer, RangeRelation,
    range_relation,
};

use super::common::FunctionAA;
use super::result::{AliasResult, ModRefInfo, ParameterAttributes};

/// Basic alias analysis using pointer provenance and offset tracking.
///
/// This is the primary alias analysis that handles:
/// - Pointer decomposition into base + offset
/// - NoAlias from different identified bases (allocations, globals)
/// - NoAlias from non-overlapping offsets within same base
/// - NoAlias from noalias function parameters
/// - MustAlias for identical pointers
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct BasicAA {
    /// Common function information (constants, definitions, parameters).
    function: FunctionAA,
    /// Parameter attributes (noalias, nocapture, etc.).
    param_attrs: Vec<ParameterAttributes>,
    /// Whether strict borrow mode is enabled.
    strict_borrow_mode: bool,
}

impl BasicAA {
    /// Build BasicAA for a function.
    pub(super) fn build(
        function: &mir::Function,
        tree: &mir::NodeTree,
        strict_borrow_mode: bool,
    ) -> Self {
        let info = FunctionAA::collect(function, tree);

        // NOTE #Incomplete: infer parameter attributes from function signature/annotations
        let param_attrs = function
            .parameters
            .iter()
            .map(|_| ParameterAttributes::NONE)
            .collect();

        Self {
            function: info,
            param_attrs,
            strict_borrow_mode,
        }
    }

    /// Query if two memory locations alias.
    pub(super) fn alias(
        &self,
        loc_a: &MemoryLocation,
        loc_b: &MemoryLocation,
        tree: &mir::NodeTree,
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

        // rule 3: same base, check offset-based aliasing
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
            (PointerBase::ManagedAlloc(a), PointerBase::ManagedAlloc(b)) => a != b,
            (PointerBase::RawAlloc(a), PointerBase::RawAlloc(b)) => a != b,

            // different globals
            (PointerBase::Global(a), PointerBase::Global(b)) => a != b,

            // different allocation types never alias
            (PointerBase::StackAlloc(_), PointerBase::ManagedAlloc(_))
            | (PointerBase::ManagedAlloc(_), PointerBase::StackAlloc(_))
            | (PointerBase::StackAlloc(_), PointerBase::RawAlloc(_))
            | (PointerBase::RawAlloc(_), PointerBase::StackAlloc(_))
            | (PointerBase::ManagedAlloc(_), PointerBase::RawAlloc(_))
            | (PointerBase::RawAlloc(_), PointerBase::ManagedAlloc(_)) => true,

            // globals vs local allocations
            (PointerBase::Global(_), PointerBase::StackAlloc(_))
            | (PointerBase::StackAlloc(_), PointerBase::Global(_))
            | (PointerBase::Global(_), PointerBase::ManagedAlloc(_))
            | (PointerBase::ManagedAlloc(_), PointerBase::Global(_))
            | (PointerBase::Global(_), PointerBase::RawAlloc(_))
            | (PointerBase::RawAlloc(_), PointerBase::Global(_)) => true,

            _ => false,
        }
    }

    /// Check noalias parameter aliasing rules.
    fn noalias_param_check(&self, base_a: &PointerBase, base_b: &PointerBase) -> bool {
        let a_noalias = base_a.is_noalias_param();
        let b_noalias = base_b.is_noalias_param();

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
        if self.var_offsets_disjoint(ptr_a, ptr_b) {
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
    fn var_offsets_disjoint(&self, ptr_a: &DecomposedPointer, ptr_b: &DecomposedPointer) -> bool {
        // if same single variable index with same scale, check constant offset difference
        if ptr_a.var_offsets.len() == 1
            && ptr_b.var_offsets.len() == 1
            && ptr_a.var_offsets[0].index == ptr_b.var_offsets[0].index
            && ptr_a.var_offsets[0].scale == ptr_b.var_offsets[0].scale
        {
            // same index, different constant offsets
            let scale = ptr_a.var_offsets[0].scale as i64;
            let diff = (ptr_a.const_offset - ptr_b.const_offset).abs();
            if diff > 0 && diff < scale {
                // within same element but different constant offset
                // this means they're at different byte positions
                return true;
            }
        }

        // check if indices are provably different constants
        if ptr_a.var_offsets.len() == 1 && ptr_b.var_offsets.len() == 1 {
            let idx_a = ptr_a.var_offsets[0].index;
            let idx_b = ptr_b.var_offsets[0].index;
            if let (Some(&c_a), Some(&c_b)) = (
                self.function.constants.get(&idx_a),
                self.function.constants.get(&idx_b),
            ) && c_a != c_b
            {
                return true;
            }
        }

        false
    }

    /// Get mod/ref info for an instruction relative to a memory location.
    pub(super) fn get_mod_ref_info(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        loc: &MemoryLocation,
        tree: &mir::NodeTree,
    ) -> ModRefInfo {
        let inst = tree.get(instruction_id);

        match inst {
            mir::Instruction::Load { pointer, .. } => {
                let load_loc = MemoryLocation::from_ptr(*pointer);
                if self.alias(&load_loc, loc, tree) == AliasResult::NoAlias {
                    ModRefInfo::NO_MOD_REF
                } else {
                    ModRefInfo::REF
                }
            }

            mir::Instruction::Store { pointer, .. } => {
                let store_loc = MemoryLocation::from_ptr(*pointer);
                if self.alias(&store_loc, loc, tree) == AliasResult::NoAlias {
                    ModRefInfo::NO_MOD_REF
                } else {
                    ModRefInfo::MOD
                }
            }

            mir::Instruction::Call { .. } | mir::Instruction::CallIndirect { .. } => {
                self.get_call_mod_ref(inst, loc, tree)
            }

            // intrinsics that access memory
            mir::Instruction::Intrinsic { intrinsic, .. } => {
                self.get_intrinsic_mod_ref(intrinsic, loc)
            }

            // allocations don't alias existing locations
            mir::Instruction::ManagedAlloc { .. }
            | mir::Instruction::ManagedAllocArray { .. }
            | mir::Instruction::RawAlloc { .. }
            | mir::Instruction::StackAlloc { .. } => ModRefInfo::NO_MOD_REF,

            // deallocation: only affects the freed memory
            mir::Instruction::RawFree { pointer } => {
                let free_loc = MemoryLocation::from_ptr(*pointer);
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

    /// Get mod/ref for a call instruction.
    fn get_call_mod_ref(
        &self,
        _inst: &mir::Instruction,
        _loc: &MemoryLocation,
        _tree: &mir::NodeTree,
    ) -> ModRefInfo {
        // NOTE #Incomplete: check function attributes (readonly, argmemonly, etc.)
        // NOTE #Incomplete: check if loc is derived from any call argument
        ModRefInfo::MOD_REF
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

            // atomics
            Intrinsic::AtomicLoad => ModRefInfo::REF,
            Intrinsic::AtomicStore => ModRefInfo::MOD,
            Intrinsic::AtomicFetchAdd
            | Intrinsic::AtomicFetchSub
            | Intrinsic::AtomicFetchAnd
            | Intrinsic::AtomicFetchOr
            | Intrinsic::AtomicFetchXor
            | Intrinsic::AtomicFetchMin
            | Intrinsic::AtomicFetchMax => ModRefInfo::MOD_REF,

            // fence is a barrier but doesn't access specific memory
            Intrinsic::AtomicFence => ModRefInfo::NO_MOD_REF,

            // GC barriers
            Intrinsic::GcWriteBarrier => ModRefInfo::MOD,
            Intrinsic::GcReadBarrier => ModRefInfo::REF,

            // volatile memory access
            Intrinsic::VolatileLoad => ModRefInfo::REF,
            Intrinsic::VolatileStore => ModRefInfo::MOD,

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
    pub(super) fn is_arg_derived(&self, value: mir::Value, tree: &mir::NodeTree) -> bool {
        let mut decomposer = PointerDecomposer::new(
            &self.function.constants,
            &self.function.definitions,
            tree,
            &self.function.parameters,
            self.strict_borrow_mode,
        );

        let decomp = decomposer.decompose(value);
        matches!(decomp.base, PointerBase::Parameter { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    #[test]
    fn test_different_allocations_no_alias() {
        let program = TestProgram::new(
            r#"type @Point = { i32, i32 }
function @test() -> void {
block0:
    v0 = managed.alloc @Point
    v1 = managed.alloc @Point
    v2 = iconst 1i32
    store v0, v2
    store v1, v2
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = BasicAA::build(function, &program.tree, false);

        let loc0 = MemoryLocation::from_ptr(mir::Value::new(0));
        let loc1 = MemoryLocation::from_ptr(mir::Value::new(1));

        assert_eq!(aa.alias(&loc0, &loc1, &program.tree), AliasResult::NoAlias);
    }

    #[test]
    fn test_same_pointer_must_alias() {
        let program = TestProgram::new(
            r#"type @Point = { i32, i32 }
function @test() -> void {
block0:
    v0 = managed.alloc @Point
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = BasicAA::build(function, &program.tree, false);

        let loc = MemoryLocation::with_size(mir::Value::new(0), 8);

        assert_eq!(aa.alias(&loc, &loc, &program.tree), AliasResult::MustAlias);
    }

    #[test]
    fn test_different_fields_no_alias() {
        let program = TestProgram::new(
            r#"type @Point = { i32, i32 }
function @test() -> void {
block0:
    v0 = managed.alloc @Point
    v1 = field.addr v0, 0
    v2 = field.addr v0, 1
    v3 = iconst 1i32
    store v1, v3
    store v2, v3
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = BasicAA::build(function, &program.tree, false);

        let loc1 = MemoryLocation::from_ptr(mir::Value::new(1));
        let loc2 = MemoryLocation::from_ptr(mir::Value::new(2));

        assert_eq!(aa.alias(&loc1, &loc2, &program.tree), AliasResult::NoAlias);
    }

    #[test]
    fn test_stack_vs_heap_no_alias() {
        let program = TestProgram::new(
            r#"function @test() -> void {
block0:
    v0 = stack.alloc i32
    v1 = managed.alloc i32
    v2 = iconst 1i32
    store v0, v2
    store v1, v2
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = BasicAA::build(function, &program.tree, false);

        let loc0 = MemoryLocation::from_ptr(mir::Value::new(0));
        let loc1 = MemoryLocation::from_ptr(mir::Value::new(1));

        assert_eq!(aa.alias(&loc0, &loc1, &program.tree), AliasResult::NoAlias);
    }

    #[test]
    fn test_global_vs_local_no_alias() {
        let program = TestProgram::new(
            r#"global @g: i32 = 0i32
function @test() -> void {
block0:
    v0 = global.addr @g
    v1 = stack.alloc i32
    v2 = iconst 1i32
    store v0, v2
    store v1, v2
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = BasicAA::build(function, &program.tree, false);

        let loc0 = MemoryLocation::from_ptr(mir::Value::new(0));
        let loc1 = MemoryLocation::from_ptr(mir::Value::new(1));

        assert_eq!(aa.alias(&loc0, &loc1, &program.tree), AliasResult::NoAlias);
    }

    #[test]
    fn test_different_elements_no_alias() {
        let program = TestProgram::new(
            r#"type @Arr = [i32; 10]
function @test() -> void {
block0:
    v0 = stack.alloc @Arr
    v1 = iconst 0i64
    v2 = iconst 1i64
    v3 = element.addr v0, v1
    v4 = element.addr v0, v2
    v5 = iconst 42i32
    store v3, v5
    store v4, v5
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = BasicAA::build(function, &program.tree, false);

        let loc3 = MemoryLocation::from_ptr(mir::Value::new(3));
        let loc4 = MemoryLocation::from_ptr(mir::Value::new(4));

        assert_eq!(aa.alias(&loc3, &loc4, &program.tree), AliasResult::NoAlias);
    }

    #[test]
    fn test_parameters_may_alias() {
        let program = TestProgram::new(
            r#"function @test(v0: ref<raw i32>, v1: ref<raw i32>) -> void {
block0(v0: ref<raw i32>, v1: ref<raw i32>):
    v2 = iconst 1i32
    store v0, v2
    store v1, v2
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = BasicAA::build(function, &program.tree, false);

        let loc0 = MemoryLocation::from_ptr(mir::Value::new(0));
        let loc1 = MemoryLocation::from_ptr(mir::Value::new(1));

        // without noalias, params may alias
        assert_eq!(aa.alias(&loc0, &loc1, &program.tree), AliasResult::MayAlias);
    }

    #[test]
    fn test_same_pointer_different_sizes_partial_alias() {
        // same pointer but different access sizes should be PartialAlias
        let program = TestProgram::new(
            r#"function @test() -> void {
block0:
    v0 = stack.alloc i64
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = BasicAA::build(function, &program.tree, false);

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
            r#"function @test() -> void {
block0:
    v0 = stack.alloc i64
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = BasicAA::build(function, &program.tree, false);

        // same pointer, one unknown size
        let loc_known = MemoryLocation::with_size(mir::Value::new(0), 4);
        let loc_unknown = MemoryLocation::from_ptr(mir::Value::new(0));

        assert_eq!(
            aa.alias(&loc_known, &loc_unknown, &program.tree),
            AliasResult::PartialAlias
        );
    }

    #[test]
    fn test_raw_vs_managed_alloc_no_alias() {
        // different allocation types never alias
        let program = TestProgram::new(
            r#"function @test() -> void {
block0:
    v0 = raw.alloc i64
    v1 = managed.alloc i64
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = BasicAA::build(function, &program.tree, false);

        let loc0 = MemoryLocation::from_ptr(mir::Value::new(0));
        let loc1 = MemoryLocation::from_ptr(mir::Value::new(1));

        assert_eq!(aa.alias(&loc0, &loc1, &program.tree), AliasResult::NoAlias);
    }

    #[test]
    fn test_cast_preserves_provenance() {
        // cast should preserve pointer provenance
        let program = TestProgram::new(
            r#"function @test() -> void {
block0:
    v0 = stack.alloc i32
    v1 = stack.alloc i32
    v2 = bitcast v0 -> ref<raw i8>
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = BasicAA::build(function, &program.tree, false);

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
            r#"type @Arr = [i32; 10]
function @test(v0: i64) -> void {
block0(v0: i64):
    v1 = stack.alloc @Arr
    v2 = element.addr v1, v0
    v3 = element.addr v1, v0
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = BasicAA::build(function, &program.tree, false);

        let loc2 = MemoryLocation::from_ptr(mir::Value::new(2));
        let loc3 = MemoryLocation::from_ptr(mir::Value::new(3));

        // same index variable means they alias
        assert!(aa.alias(&loc2, &loc3, &program.tree).may_alias());
    }

    #[test]
    fn test_nested_field_access() {
        // nested struct field access
        let program = TestProgram::new(
            r#"type @Inner = { i32, i32 }
type @Outer = { @Inner, @Inner }
function @test() -> void {
block0:
    v0 = stack.alloc @Outer
    v1 = field.addr v0, 0
    v2 = field.addr v0, 1
    v3 = field.addr v1, 0
    v4 = field.addr v2, 0
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let aa = BasicAA::build(function, &program.tree, false);

        // v3 is outer.0.0, v4 is outer.1.0 - different top-level fields
        let loc3 = MemoryLocation::from_ptr(mir::Value::new(3));
        let loc4 = MemoryLocation::from_ptr(mir::Value::new(4));

        assert_eq!(aa.alias(&loc3, &loc4, &program.tree), AliasResult::NoAlias);
    }
}
