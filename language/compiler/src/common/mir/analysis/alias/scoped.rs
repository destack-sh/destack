use destack_mir as mir;

use crate::common::mir::{
    MemoryLocation, PointerBase, PointerDecomposer, TypeContext, ValueTypeMap,
};

use super::common::FunctionAA;
use super::result::AliasResult;

/// Scoped noalias analysis for Destack's ownership system.
///
/// Currently handles:
/// - `&mut T` in strict borrow mode: exclusive mutable borrow, noalias
///
/// NOTE #Incomplete: could also handle:
/// - `^T` ownership transfer: caller loses all references
/// - `@noManaged` functions: cannot access heap storage
/// - `@noHeap` functions: cannot access any heap
///
/// This analysis is only effective when strict borrow mode is enabled.
#[derive(Debug)]
pub(crate) struct ScopedNoAliasAA {
    /// Parameters with noalias semantics (exclusive borrows in strict mode).
    noalias_params: Vec<bool>,
    /// Whether strict borrow mode is enabled.
    strict_borrow_mode: bool,
    /// Common function information (constants, definitions, parameters).
    function: FunctionAA,
    /// Type context for layout sensitive operations.
    type_context: TypeContext,
    /// Value type map for pointer decomposition.
    value_types: ValueTypeMap,
}

impl ScopedNoAliasAA {
    /// Build ScopedNoAliasAA for a function.
    pub(super) fn build(
        function: &mir::Function,
        tree: &mir::Tree,
        strict_borrow_mode: bool,
        value_types: &ValueTypeMap,
        type_context: TypeContext,
    ) -> Self {
        let info = FunctionAA::collect(function, tree);

        // determine which parameters have noalias semantics
        let noalias_params = function
            .parameters
            .iter()
            .map(|parameter| Self::is_noalias_parameter(parameter, tree, strict_borrow_mode))
            .collect();

        Self {
            noalias_params,
            strict_borrow_mode,
            function: info,
            type_context,
            value_types: value_types.clone(),
        }
    }

    /// Check if a parameter has noalias semantics.
    fn is_noalias_parameter(parameter: &mir::Parameter, tree: &mir::Tree, strict: bool) -> bool {
        if !strict {
            return false;
        }

        // writable borrowed parameters are noalias in strict mode
        let Some(parameter) = parameter.typed_value() else {
            return false;
        };

        let ty = tree.get(parameter.ty);
        ty.is_writable_borrowed_reference()
    }

    /// Query if two memory locations may alias.
    pub(super) fn alias(
        &self,
        loc_a: &MemoryLocation,
        loc_b: &MemoryLocation,
        tree: &mir::Tree,
    ) -> AliasResult {
        if !self.strict_borrow_mode {
            return AliasResult::MayAlias;
        }

        // decompose pointers to find their bases
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

        // check noalias parameter rules
        match (&ptr_a.base, &ptr_b.base) {
            // two different noalias parameters cannot alias
            (
                PointerBase::Parameter { index: index_a, .. },
                PointerBase::Parameter { index: index_b, .. },
            ) if index_a != index_b => {
                let a_noalias = self
                    .noalias_params
                    .get(*index_a as usize)
                    .copied()
                    .unwrap_or(false);
                let b_noalias = self
                    .noalias_params
                    .get(*index_b as usize)
                    .copied()
                    .unwrap_or(false);

                if a_noalias && b_noalias {
                    return AliasResult::NoAlias;
                }

                // one noalias param doesn't alias non-noalias params
                // (the noalias one is exclusive, the other might alias it, but we're safe)
                if a_noalias || b_noalias {
                    return AliasResult::NoAlias;
                }
            }

            // noalias param doesn't alias local allocations
            (PointerBase::Parameter { index, .. }, base)
            | (base, PointerBase::Parameter { index, .. })
                if base.is_local_alloc() =>
            {
                let is_noalias = self
                    .noalias_params
                    .get(*index as usize)
                    .copied()
                    .unwrap_or(false);
                if is_noalias {
                    return AliasResult::NoAlias;
                }
            }

            _ => {}
        }

        AliasResult::MayAlias
    }

    /// Check if strict borrow mode is enabled.
    #[allow(dead_code)]
    pub(super) fn is_strict_mode(&self) -> bool {
        self.strict_borrow_mode
    }

    /// Check if a parameter index has noalias semantics.
    #[allow(dead_code)]
    pub(super) fn is_parameter_noalias(&self, index: usize) -> bool {
        self.noalias_params.get(index).copied().unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::mir::{TypeContext, ValueTypeMap};
    use crate::optimize::common::tests::TestProgram;

    /// Build a ScopedNoAliasAA instance for a test function.
    fn build_scoped_aa(
        function: &mir::Function,
        program: &TestProgram,
        strict_borrow_mode: bool,
    ) -> ScopedNoAliasAA {
        let value_types = ValueTypeMap::new(function, &program.tree);
        ScopedNoAliasAA::build(
            function,
            &program.tree,
            strict_borrow_mode,
            &value_types,
            TypeContext::default(),
        )
    }

    #[test]
    fn test_non_strict_mode_may_alias() {
        let program = TestProgram::new(
            r#"
function test(v0: ref<int32, borrowed, readonly>, v1: ref<int32, borrowed, readonly>): void {
b0(v0: ref<int32, borrowed, readonly>, v1: ref<int32, borrowed, readonly>):
    v2: int32 = 1int32
    store v0, v2
    store v1, v2
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);

        // non-strict mode: &mut doesn't guarantee noalias
        let aa = build_scoped_aa(function, &program, false);

        let loc0 = MemoryLocation::from_ptr(mir::Value::new(0));
        let loc1 = MemoryLocation::from_ptr(mir::Value::new(1));

        assert_eq!(aa.alias(&loc0, &loc1, &program.tree), AliasResult::MayAlias);
    }

    #[test]
    fn test_strict_mode_mut_borrows_no_alias() {
        let program = TestProgram::new(
            r#"
function test(v0: ref<int32, borrowed>, v1: ref<int32, borrowed, readonly>): void {
b0(v0: ref<int32, borrowed>, v1: ref<int32, borrowed, readonly>):
    v2: int32 = 1int32
    store v0, v2
    store v1, v2
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);

        // strict mode: &mut T parameters are noalias
        let aa = build_scoped_aa(function, &program, true);

        let loc0 = MemoryLocation::from_ptr(mir::Value::new(0));
        let loc1 = MemoryLocation::from_ptr(mir::Value::new(1));

        assert_eq!(aa.alias(&loc0, &loc1, &program.tree), AliasResult::NoAlias);
    }

    #[test]
    fn test_mut_borrow_vs_local_no_alias() {
        let program = TestProgram::new(
            r#"
function test(v0: ref<int32, borrowed>): void {
b0(v0: ref<int32, borrowed>):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 1int32
    store v0, v2
    store v1, v2
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);

        // strict mode: &mut param doesn't alias local allocations
        let aa = build_scoped_aa(function, &program, true);

        let loc0 = MemoryLocation::from_ptr(mir::Value::new(0));
        let loc1 = MemoryLocation::from_ptr(mir::Value::new(1));

        assert_eq!(aa.alias(&loc0, &loc1, &program.tree), AliasResult::NoAlias);
    }

    #[test]
    fn test_immutable_borrow_may_alias() {
        // even in strict mode, immutable borrows may alias each other
        let program = TestProgram::new(
            r#"
function test(v0: ref<int32, borrowed, readonly>, v1: ref<int32, borrowed, readonly>): void {
b0(v0: ref<int32, borrowed, readonly>, v1: ref<int32, borrowed, readonly>):
    v2: int32 = load v0
    v3: int32 = load v1
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);

        // strict mode, but immutable borrows can alias
        let aa = build_scoped_aa(function, &program, true);

        let loc0 = MemoryLocation::from_ptr(mir::Value::new(0));
        let loc1 = MemoryLocation::from_ptr(mir::Value::new(1));

        // immutable refs are NOT noalias
        assert_eq!(aa.alias(&loc0, &loc1, &program.tree), AliasResult::MayAlias);
    }

    #[test]
    fn test_mut_borrow_vs_immutable_borrow_no_alias() {
        // in strict mode borrow doesn't alias immutable borrow
        let program = TestProgram::new(
            r#"
function test(v0: ref<int32, borrowed>, v1: ref<int32, borrowed, readonly>): void {
b0(v0: ref<int32, borrowed>, v1: ref<int32, borrowed, readonly>):
    v2: int32 = 1int32
    store v0, v2
    v3: int32 = load v1
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);

        // strict mode: &mut is noalias, so it doesn't alias &
        let aa = build_scoped_aa(function, &program, true);

        let loc0 = MemoryLocation::from_ptr(mir::Value::new(0));
        let loc1 = MemoryLocation::from_ptr(mir::Value::new(1));

        // mut borrow is noalias, so doesn't alias other param
        assert_eq!(aa.alias(&loc0, &loc1, &program.tree), AliasResult::NoAlias);
    }

    #[test]
    fn test_derived_pointer_from_noalias_param() {
        // field.address from noalias param should still not alias other params
        let program = TestProgram::new(
            r#"
type Point {
    int32;
    int32;
}
function test(v0: ref<Point, borrowed>, v1: ref<int32, borrowed>): void {
b0(v0: ref<Point, borrowed>, v1: ref<int32, borrowed>):
    v2: ref<int32, borrowed> = field.address v0, 0
    v3: int32 = 1int32
    store v2, v3
    store v1, v3
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);

        let aa = build_scoped_aa(function, &program, true);

        // v2 is derived from noalias param v0, v1 is different noalias param
        let loc2 = MemoryLocation::from_ptr(mir::Value::new(2));
        let loc1 = MemoryLocation::from_ptr(mir::Value::new(1));

        // derived from different noalias params should not alias
        assert_eq!(aa.alias(&loc2, &loc1, &program.tree), AliasResult::NoAlias);
    }

    #[test]
    fn test_same_mut_borrow_may_alias_self() {
        // same noalias param accessed twice should may-alias (itself)
        let program = TestProgram::new(
            r#"
function test(v0: ref<int32, borrowed>): void {
b0(v0: ref<int32, borrowed>):
    v1: int32 = 1int32
    store v0, v1
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);

        let aa = build_scoped_aa(function, &program, true);

        let loc0_a = MemoryLocation::from_ptr(mir::Value::new(0));
        let loc0_b = MemoryLocation::from_ptr(mir::Value::new(0));

        // same pointer, same param index, should alias
        assert!(aa.alias(&loc0_a, &loc0_b, &program.tree).may_alias());
    }
}
