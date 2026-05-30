use std::sync::Arc;

use destack_mir as mir;

use crate::common::mir::{
    Analysis, AnalysisId, FunctionAnalyses, FunctionAnalysis, MemoryLocation, TypeContext, TypeKey,
    ValueTypeMap,
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
    tree: Arc<mir::Tree>,
}

impl AliasAnalysis {
    /// Build combined AA for a function with explicit options.
    pub fn build(
        function: &mir::Function,
        tree: &mir::Tree,
        strict_borrow_mode: bool,
        value_types: &ValueTypeMap,
        type_context: TypeContext,
    ) -> Self {
        Self {
            basic: BasicAA::build(
                function,
                tree,
                strict_borrow_mode,
                value_types,
                type_context,
            ),
            tbaa: TypeBasedAA::new(),
            globals: GlobalsAA::build(function, tree),
            scoped: ScopedNoAliasAA::build(
                function,
                tree,
                strict_borrow_mode,
                value_types,
                type_context,
            ),
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
        self.get_mod_ref_info_with_metadata(instruction_id, loc)
    }

    /// Get mod ref info for an instruction relative to a memory location.
    pub fn get_mod_ref_info_with_metadata(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        loc: &MemoryLocation,
    ) -> ModRefInfo {
        // start with basic aa's assessment
        let mut result = self
            .basic
            .get_mod_ref_info_with_metadata(instruction_id, loc, &self.tree);

        // if basic aa says no mod ref, we're done
        if result.is_no_mod_ref() {
            return result;
        }

        // allow global analysis to refine call effects
        let inst = self.tree.get(instruction_id);
        if matches!(
            inst,
            mir::Instruction::Call { .. }
                | mir::Instruction::CallVirtual { .. }
                | mir::Instruction::CallDynamic { .. }
                | mir::Instruction::CallIndirect { .. }
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
    const ID: AnalysisId = AnalysisId("alias");
    const DEPENDENCIES: &'static [AnalysisId] = &[];
}

impl FunctionAnalysis for AliasAnalysis {
    fn compute(
        function: &mir::Function,
        tree: &mir::Tree,
        analyses: &FunctionAnalyses<'_>,
    ) -> Self {
        let value_types = ValueTypeMap::new(function, tree);
        Self::build(
            function,
            tree,
            analyses.options().strict_borrow_mode,
            &value_types,
            analyses.type_context(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    #[test]
    fn test_combined_basic_and_tbaa() {
        let program = TestProgram::new(
            r#"
type Point {
    int32;
    int32;
}
function test(): void {
b0:
    v0: ref<Point, managed> = new.zeroed Point
    v1: ref<Point, managed> = new.zeroed Point
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let aa = analyses.get::<AliasAnalysis>();

        // different allocations don't alias
        assert!(aa.pointers_no_alias(mir::Value::new(0), mir::Value::new(1)));
    }

    #[test]
    fn test_mod_ref_for_load() {
        let program = TestProgram::new(
            r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v2: int32 = 42int32
    store v0, v2
    v3: int32 = load v1
    return v3
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let aa = analyses.get::<AliasAnalysis>();

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
            r#"
function test(): void {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v2: int32 = 42int32
    store v0, v2
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let aa = analyses.get::<AliasAnalysis>();

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
            r#"
global g1: int32 = 0int32
global g2: int32 = 0int32
function test(): void {
b0:
    v0: ref<int32, raw, space(static)> = global.address g1
    v1: ref<int32, raw, space(static)> = global.address g2
    v2: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let aa = analyses.get::<AliasAnalysis>();

        // different globals don't alias
        assert!(aa.pointers_no_alias(mir::Value::new(0), mir::Value::new(1)));

        // global doesn't alias stack allocation
        assert!(aa.pointers_no_alias(mir::Value::new(0), mir::Value::new(2)));
    }

    #[test]
    fn test_tbaa_int_vs_float() {
        let program = TestProgram::new(
            r#"
function test(): void {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: ref<float64, raw, space(frame)> = frame.alloc.zeroed float64
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let aa = analyses.get::<AliasAnalysis>();

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

    #[test]
    fn test_call_metadata_no_memory_mod_ref() {
        let mut program = TestProgram::new(
            r#"
external function external(ref<int32, raw>): void
function test(v0: ref<int32, raw>): void {
b0(v0: ref<int32, raw>):
    call external(v0): (ref<int32, raw>) -> void
    return
}"#,
        );

        let function_id = program.entry_function_id();
        let param_value = {
            let function = program.tree.get(function_id);
            function.parameters[0]
                .value
                .value()
                .expect("parameter value should be concrete")
        };
        let (call_inst, _callee) = program.first_call_in_entry(function_id);

        let call = mir::CallSite::Instruction(call_inst);
        program.tree.metadata.functions.call_mut(call).memory = mir::MemoryEffect::none();

        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);
        let aa = analyses.get::<AliasAnalysis>();
        let loc = MemoryLocation::from_ptr(param_value);

        assert!(aa.get_mod_ref_info(call_inst, &loc).is_no_mod_ref());
    }
}
