use std::sync::Arc;

use destack_mir as mir;

use crate::optimize::common::{
    Analysis, AnalysisKind, MemoryLocation, OptimizationContext, TypeKey,
};

use super::basic::BasicAA;
use super::globals::GlobalsAA;
use super::result::{AliasResult, ModRefInfo};
use super::scoped::ScopedNoAliasAA;
use super::tbaa::TypeBasedAA;

/// Combined alias analysis results.
///
/// Queries multiple sub-analyses and returns the strongest result.
/// Any sub-analysis proving NoAlias is sufficient for the overall result.
///
/// Sub-analyses included:
/// - BasicAA: pointer provenance and offset-based analysis
/// - TBAA: type-based strict aliasing
/// - GlobalsAA: global variable tracking
/// - ScopedNoAliasAA: Destack ownership/borrow semantics
#[derive(Debug)]
pub struct AliasAnalysis {
    /// Pointer provenance and offset-based analysis.
    basic: BasicAA,
    /// Type-based strict aliasing analysis.
    tbaa: TypeBasedAA,
    /// Global variable access tracking.
    globals: GlobalsAA,
    /// Destack ownership/borrow-based noalias analysis.
    scoped: ScopedNoAliasAA,
    /// The MIR tree for queries.
    tree: Arc<mir::NodeTree>,
}

impl AliasAnalysis {
    /// Build combined AA for a function.
    fn build(function: &mir::Function, tree: &mir::NodeTree, strict_borrow_mode: bool) -> Self {
        Self {
            basic: BasicAA::build(function, tree, strict_borrow_mode),
            tbaa: TypeBasedAA::default(),
            globals: GlobalsAA::build(function, tree),
            scoped: ScopedNoAliasAA::build(function, tree, strict_borrow_mode),
            tree: Arc::new(tree.clone()),
        }
    }

    /// Query if two memory locations may alias.
    ///
    /// Combines results from all sub-analyses. NoAlias from any sub-analysis
    /// is sufficient to return NoAlias overall.
    pub fn alias(&self, loc_a: &MemoryLocation, loc_b: &MemoryLocation) -> AliasResult {
        // basic pointer provenance analysis
        let basic_result = self.basic.alias(loc_a, loc_b, &self.tree);
        if basic_result == AliasResult::NoAlias {
            return AliasResult::NoAlias;
        }

        // type-based alias analysis
        let tbaa_result = self.tbaa.alias(loc_a, loc_b);
        if tbaa_result == AliasResult::NoAlias {
            return AliasResult::NoAlias;
        }

        // global variable analysis
        let globals_result = self.globals.alias(loc_a, loc_b, &self.tree);
        if globals_result == AliasResult::NoAlias {
            return AliasResult::NoAlias;
        }

        // scoped noalias analysis (ownership/borrow semantics)
        let scoped_result = self.scoped.alias(loc_a, loc_b, &self.tree);
        if scoped_result == AliasResult::NoAlias {
            return AliasResult::NoAlias;
        }

        // check for MustAlias (only if all agree)
        if basic_result == AliasResult::MustAlias
            && tbaa_result != AliasResult::NoAlias
            && globals_result != AliasResult::NoAlias
            && scoped_result != AliasResult::NoAlias
        {
            return AliasResult::MustAlias;
        }

        // check for PartialAlias
        if basic_result == AliasResult::PartialAlias {
            return AliasResult::PartialAlias;
        }

        AliasResult::MayAlias
    }

    /// Get mod/ref info for an instruction relative to a memory location.
    ///
    /// Returns what effect the instruction may have on the given location.
    pub fn get_mod_ref_info(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        loc: &MemoryLocation,
    ) -> ModRefInfo {
        // start with BasicAA's assessment
        let mut result = self.basic.get_mod_ref_info(instruction_id, loc, &self.tree);

        // if BasicAA says no mod/ref, we're done
        if result.is_no_mod_ref() {
            return result;
        }

        // global analysis can refine for global accesses
        let inst = self.tree.get(instruction_id);
        if matches!(
            inst,
            mir::Instruction::Call { .. } | mir::Instruction::CallIndirect { .. }
        ) {
            let globals_result = self.globals.get_call_mod_ref(inst, loc, &self.tree);
            result = result.intersect(globals_result);
        }

        result
    }

    /// Convenience: check if two pointer values may alias.
    pub fn pointers_may_alias(&self, ptr1: mir::Value, ptr2: mir::Value) -> bool {
        let loc1 = MemoryLocation::from_ptr(ptr1);
        let loc2 = MemoryLocation::from_ptr(ptr2);
        self.alias(&loc1, &loc2).may_alias()
    }

    /// Convenience: check if two pointer values definitely don't alias.
    pub fn pointers_no_alias(&self, ptr1: mir::Value, ptr2: mir::Value) -> bool {
        let loc1 = MemoryLocation::from_ptr(ptr1);
        let loc2 = MemoryLocation::from_ptr(ptr2);
        self.alias(&loc1, &loc2).is_no_alias()
    }

    /// Query with type information for TBAA.
    pub fn alias_with_type(
        &self,
        ptr1: mir::Value,
        ty1: TypeKey,
        ptr2: mir::Value,
        ty2: TypeKey,
    ) -> AliasResult {
        let loc1 = MemoryLocation::with_type(ptr1, ty1);
        let loc2 = MemoryLocation::with_type(ptr2, ty2);
        self.alias(&loc1, &loc2)
    }

    /// Check if an instruction may clobber a memory location.
    pub fn may_clobber(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        loc: &MemoryLocation,
    ) -> bool {
        self.get_mod_ref_info(instruction_id, loc).is_mod()
    }

    /// Check if an instruction may read a memory location.
    pub fn may_read(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        loc: &MemoryLocation,
    ) -> bool {
        self.get_mod_ref_info(instruction_id, loc).is_ref()
    }
}

impl Analysis for AliasAnalysis {
    const KIND: AnalysisKind = AnalysisKind::Alias;

    fn compute(
        function: &mir::Function,
        tree: &mir::NodeTree,
        context: &OptimizationContext<'_>,
    ) -> Arc<Self> {
        Arc::new(Self::build(
            function,
            tree,
            context.options.strict_borrow_mode,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    #[test]
    fn test_combined_basic_and_tbaa() {
        let program = TestProgram::new(
            r#"type @Point = { i32, i32 }
function @test() -> void {
block0:
    v0 = managed.alloc @Point
    v1 = managed.alloc @Point
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let context = program.context();
        let aa = context
            .analyses
            .get::<AliasAnalysis>(function, &program.tree, &context);

        // different allocations don't alias
        assert!(aa.pointers_no_alias(mir::Value::new(0), mir::Value::new(1)));
    }

    #[test]
    fn test_mod_ref_for_load() {
        let program = TestProgram::new(
            r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = stack.alloc i32
    v2 = iconst 42i32
    store v0, v2
    v3 = load v1
    return v3
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let context = program.context();
        let aa = context
            .analyses
            .get::<AliasAnalysis>(function, &program.tree, &context);

        // find the load instruction
        let block = program.tree.get(function.blocks[0]);
        let load_inst = block.instructions[4]; // v3 = load v1

        // load v1 shouldn't reference v0's location
        let loc0 = MemoryLocation::from_ptr(mir::Value::new(0));
        assert!(aa.get_mod_ref_info(load_inst, &loc0).is_no_mod_ref());
    }

    #[test]
    fn test_mod_ref_for_store() {
        let program = TestProgram::new(
            r#"function @test() -> void {
block0:
    v0 = stack.alloc i32
    v1 = stack.alloc i32
    v2 = iconst 42i32
    store v0, v2
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let context = program.context();
        let aa = context
            .analyses
            .get::<AliasAnalysis>(function, &program.tree, &context);

        // find the store instruction
        let block = program.tree.get(function.blocks[0]);
        let store_inst = block.instructions[3]; // store v0, v2

        // store to v0 shouldn't modify v1's location
        let loc1 = MemoryLocation::from_ptr(mir::Value::new(1));
        assert!(aa.get_mod_ref_info(store_inst, &loc1).is_no_mod_ref());

        // but it does modify v0's location
        let loc0 = MemoryLocation::from_ptr(mir::Value::new(0));
        assert!(aa.get_mod_ref_info(store_inst, &loc0).is_mod());
    }

    #[test]
    fn test_global_alias_analysis() {
        let program = TestProgram::new(
            r#"global @g1: i32 = 0i32
global @g2: i32 = 0i32
function @test() -> void {
block0:
    v0 = global.addr @g1
    v1 = global.addr @g2
    v2 = stack.alloc i32
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let context = program.context();
        let aa = context
            .analyses
            .get::<AliasAnalysis>(function, &program.tree, &context);

        // different globals don't alias
        assert!(aa.pointers_no_alias(mir::Value::new(0), mir::Value::new(1)));

        // global doesn't alias stack allocation
        assert!(aa.pointers_no_alias(mir::Value::new(0), mir::Value::new(2)));
    }

    #[test]
    fn test_tbaa_int_vs_float() {
        let program = TestProgram::new(
            r#"function @test() -> void {
block0:
    v0 = stack.alloc i32
    v1 = stack.alloc f64
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let context = program.context();
        let aa = context
            .analyses
            .get::<AliasAnalysis>(function, &program.tree, &context);

        // with type info, int and float don't alias
        let int_ty = TypeKey::Int {
            width: 32,
            signed: true,
        };
        let float_ty = TypeKey::Float { width: 64 };

        // basic analysis already proves these don't alias (different allocations)
        // but TBAA would also prove it if they were the same allocation
        let result = aa.alias_with_type(mir::Value::new(0), int_ty, mir::Value::new(1), float_ty);
        assert_eq!(result, AliasResult::NoAlias);
    }
}
