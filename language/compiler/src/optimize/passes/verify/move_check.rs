use destack_compiler_macros::declare_pass;
use destack_mir as mir;
use destack_source::ModuleId;
use destack_workspace::TargetId;
use mir::{Instruction, Value};

use crate::OptimizeError;
use crate::optimize::{
    AnalysisPreservation, DiagnosticEmitter, FunctionPass, MoveLocation, OwnershipAnalysis,
    OwnershipMap, PipelineContext,
};

declare_pass! {
    /// Verify move semantics for owned values.
    ///
    /// Consumes `OwnershipAnalysis` and emits errors for:
    /// - Using a value that was definitely moved (`UseAfterMove`)
    /// - Using a value that may have been moved on some paths (`MaybeUseAfterMove`)
    #[pass(id = "move-check")]
    pub MoveCheck,
    "Verify move semantics"
}

/// Context for move checking.
struct MoveCheckContext<'a> {
    /// The MIR node tree.
    tree: &'a mir::NodeTree,
    /// The ownership analysis results.
    ownership: &'a OwnershipAnalysis,
    /// The module being checked.
    module_id: ModuleId,
    /// The target being checked.
    target_id: TargetId,
}

impl<'a> MoveCheckContext<'a> {
    /// Create a new move check context.
    fn new(
        tree: &'a mir::NodeTree,
        ownership: &'a OwnershipAnalysis,
        module_id: ModuleId,
        target_id: TargetId,
    ) -> Self {
        Self {
            tree,
            ownership,
            module_id,
            target_id,
        }
    }

    /// Create an anchored node ID from a move location for diagnostics.
    fn anchor(&self, location: &MoveLocation) -> mir::AnchoredGlobalNodeId {
        match location {
            MoveLocation::Instruction(id) => id
                .into_any()
                .into_anchored(self.module_id, self.target_id.clone()),
            MoveLocation::Terminator(id) => id
                .into_any()
                .into_anchored(self.module_id, self.target_id.clone()),
        }
    }

    /// Create an anchored node ID for an instruction.
    fn anchor_instruction(
        &self,
        instruction_id: mir::LocalNodeId<Instruction>,
    ) -> mir::AnchoredGlobalNodeId {
        instruction_id
            .into_any()
            .into_anchored(self.module_id, self.target_id.clone())
    }

    /// Create an anchored node ID for a block.
    fn anchor_block(&self, block_id: mir::LocalNodeId<mir::Block>) -> mir::AnchoredGlobalNodeId {
        block_id
            .into_any()
            .into_anchored(self.module_id, self.target_id.clone())
    }

    /// Check if using a value is valid at the current state.
    fn check_use(
        &self,
        state: &OwnershipMap,
        value: Value,
        at_instruction: Option<mir::LocalNodeId<Instruction>>,
        at_block: mir::LocalNodeId<mir::Block>,
        context: &impl DiagnosticEmitter,
    ) {
        if let Some(ownership) = state.get(value)
            && ownership.is_moved()
        {
            let use_anchor = if let Some(instruction_id) = at_instruction {
                self.anchor_instruction(instruction_id)
            } else {
                self.anchor_block(at_block)
            };

            let move_anchor = self.anchor(ownership.move_location().unwrap());

            if ownership.is_maybe_moved() {
                context.emit_error(OptimizeError::MaybeUseAfterMove {
                    node: use_anchor,
                    moved_at: move_anchor,
                });
            } else {
                context.emit_error(OptimizeError::UseAfterMove {
                    node: use_anchor,
                    moved_at: move_anchor,
                });
            }
        }
    }

    /// Run the move check on a function.
    fn check(&self, function: &mir::Function, context: &impl DiagnosticEmitter) {
        if function.entry.is_none() {
            return;
        }

        // for each block, check uses against the entry state
        for &block_id in &function.blocks {
            let Some(entry_state) = self.ownership.state_at_entry(block_id) else {
                continue;
            };

            let block = self.tree.get(block_id);

            // track state as we process instructions
            let mut current_state = entry_state.clone();

            // check each instruction's uses
            for &instruction_id in &block.instructions {
                let inst = self.tree.get(instruction_id);

                // check uses in this instruction
                self.check_instruction_uses(
                    &current_state,
                    instruction_id,
                    inst,
                    block_id,
                    context,
                );

                // update state for moves made by this instruction
                self.apply_instruction_effects(&mut current_state, instruction_id, inst);
            }

            // check terminator uses
            self.check_terminator_uses(&current_state, block_id, &block.terminator, context);
        }
    }

    /// Check uses in an instruction.
    fn check_instruction_uses(
        &self,
        state: &OwnershipMap,
        instruction_id: mir::LocalNodeId<Instruction>,
        inst: &Instruction,
        block_id: mir::LocalNodeId<mir::Block>,
        context: &impl DiagnosticEmitter,
    ) {
        // get all values used by this instruction
        for used in inst.uses() {
            self.check_use(state, used, Some(instruction_id), block_id, context);
        }

        // check arguments stored externally
        if let Some(arg_slice) = inst.argument_slice() {
            for &arg in self.tree.get_arguments(arg_slice) {
                self.check_use(state, arg, Some(instruction_id), block_id, context);
            }
        }
    }

    /// Apply the effects of an instruction on ownership state.
    ///
    /// Delegates to OwnershipAnalysis to ensure consistency.
    fn apply_instruction_effects(
        &self,
        state: &mut OwnershipMap,
        instruction_id: mir::LocalNodeId<Instruction>,
        inst: &Instruction,
    ) {
        self.ownership
            .apply_instruction_effects(state, instruction_id, inst, self.tree);
    }

    /// Check uses in a terminator.
    fn check_terminator_uses(
        &self,
        state: &OwnershipMap,
        block_id: mir::LocalNodeId<mir::Block>,
        terminator: &mir::Terminator,
        context: &impl DiagnosticEmitter,
    ) {
        match terminator {
            mir::Terminator::Return { value } => {
                if let Some(v) = value {
                    self.check_use(state, *v, None, block_id, context);
                }
            }
            mir::Terminator::Branch { condition, .. } => {
                self.check_use(state, *condition, None, block_id, context);
            }
            mir::Terminator::Check {
                condition,
                constraint,
                success,
                failure,
            } => {
                self.check_use(state, *condition, None, block_id, context);
                for &value in constraint.uses().iter() {
                    self.check_use(state, value, None, block_id, context);
                }
                for &arg in success.arguments.iter().chain(failure.arguments.iter()) {
                    self.check_use(state, arg, None, block_id, context);
                }
            }
            mir::Terminator::Switch { value, .. } => {
                self.check_use(state, *value, None, block_id, context);
            }
            mir::Terminator::Jump { arguments, .. } => {
                for &arg in arguments {
                    self.check_use(state, arg, None, block_id, context);
                }
            }
            mir::Terminator::Yield {
                value,
                resume_arguments,
                ..
            } => {
                self.check_use(state, *value, None, block_id, context);
                for &arg in resume_arguments {
                    self.check_use(state, arg, None, block_id, context);
                }
            }
            mir::Terminator::Unreachable => {}
            mir::Terminator::TailCall { arguments, .. } => {
                for &arg in arguments {
                    self.check_use(state, arg, None, block_id, context);
                }
            }
            mir::Terminator::TailCallVirtual {
                receiver,
                arguments,
                ..
            }
            | mir::Terminator::TailCallInterface {
                receiver,
                arguments,
                ..
            } => {
                self.check_use(state, *receiver, None, block_id, context);
                for &arg in arguments {
                    self.check_use(state, arg, None, block_id, context);
                }
            }
            mir::Terminator::TailCallIndirect {
                callee, arguments, ..
            } => {
                self.check_use(state, *callee, None, block_id, context);
                for &arg in arguments {
                    self.check_use(state, arg, None, block_id, context);
                }
            }
        }
    }
}

impl FunctionPass for MoveCheck {
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // get ownership analysis
        let ownership = {
            let analyses = ctx.function_analyses(function, tree);
            analyses.get::<OwnershipAnalysis>().clone()
        };

        let module_id = ctx.module_id();
        let target_id = ctx.target_id().clone();

        let checker = MoveCheckContext::new(tree, &ownership, module_id, target_id);
        checker.check(function, ctx);

        AnalysisPreservation::all()
    }

    fn name(&self) -> &'static str {
        "MoveCheck"
    }

    fn id(&self) -> &'static str {
        "move-check"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OptimizeError;
    use crate::optimize::common::tests::TestProgram;

    /// Simple function with no moves passes verification.
    #[test]
    fn test_verify_no_moves() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_no_errors();
    }

    /// Binary operation using same value twice is valid (copy semantics).
    #[test]
    fn test_verify_use_twice_primitive() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 10i32
    v1 = iadd v0, v0
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_no_errors();
    }

    /// Value used after being dropped is detected.
    #[test]
    fn test_detect_use_after_drop() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    raw.drop v0
    v1 = iadd v0, v0
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_error(|e| matches!(e, OptimizeError::UseAfterMove { .. }));
    }

    /// Copy types can be used after store.
    #[test]
    fn test_verify_use_after_store_copy_type() {
        let input = r#"function @test(v0: ref<raw i32>) -> i32 {
block0(v0: ref<raw i32>):
    v1 = iconst 42i32
    store v0, v1
    v2 = iadd v1, v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_no_errors();
    }

    /// Copy types can be used after local.set.
    #[test]
    fn test_verify_use_after_local_set_copy_type() {
        let input = r#"function @test() -> i32 {
local0: i32
block0:
    v0 = iconst 42i32
    local.set local0, v0
    v1 = iadd v0, v0
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_no_errors();
    }

    /// Copy types can be used after call.
    #[test]
    fn test_verify_use_after_call_copy_type() {
        let input = r#"extern function @consume(i32) -> void

function @test() -> i32 {
block0:
    v0 = iconst 42i32
    call @consume(v0) -> fn(i32) -> void
    v1 = iadd v0, v0
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_no_errors();
    }

    /// Value used in branch after drop is detected.
    #[test]
    fn test_detect_use_after_move_in_branch() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst true
    raw.drop v0
    branch v0, block1, block2
block1:
    v1 = iconst 1i32
    return v1
block2:
    v2 = iconst 2i32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_error(|e| matches!(e, OptimizeError::UseAfterMove { .. }));
    }

    /// Block parameters receive fresh ownership.
    #[test]
    fn test_verify_block_parameters_fresh() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 10i32
    jump block1(v0, v1)
block1(v2: i32, v3: i32):
    v4 = iadd v2, v3
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_no_errors();
    }

    /// Moved value passed to jump is detected.
    #[test]
    fn test_detect_use_after_move_jump_arg() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    raw.drop v0
    jump block1(v0)
block1(v1: i32):
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_error(|e| matches!(e, OptimizeError::UseAfterMove { .. }));
    }

    /// Return value used after move is detected.
    #[test]
    fn test_detect_use_after_move_return() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    raw.drop v0
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_error(|e| matches!(e, OptimizeError::UseAfterMove { .. }));
    }

    /// Diamond control flow: value moved on one path, used after merge.
    #[test]
    fn test_detect_maybe_moved_diamond() {
        let input = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    branch v0, block1, block2
block1:
    raw.drop v1
    jump block3
block2:
    jump block3
block3:
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        // v1 is moved on block1 path but not block2, so it's "maybe moved" at block3
        test.assert_error(|e| matches!(e, OptimizeError::MaybeUseAfterMove { .. }));
    }

    /// Diamond control flow: value moved on both paths is definitely moved.
    #[test]
    fn test_detect_definitely_moved_diamond() {
        let input = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    branch v0, block1, block2
block1:
    raw.drop v1
    jump block3
block2:
    raw.drop v1
    jump block3
block3:
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        // v1 is definitely moved on all paths
        test.assert_error(|e| matches!(e, OptimizeError::UseAfterMove { .. }));
    }

    /// Diamond control flow: value NOT moved on either path is OK.
    #[test]
    fn test_verify_not_moved_diamond() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 42i32
    branch v0, block1, block2
block1:
    jump block3
block2:
    jump block3
block3:
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_no_errors();
    }

    /// Loop with value used in body is OK if not moved.
    #[test]
    fn test_verify_loop_no_move() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 0i32
    jump block1(v1)
block1(v2: i32):
    v3 = iconst 1i32
    v4 = iadd v2, v3
    branch v0, block1(v4), block2
block2:
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_no_errors();
    }

    /// Raw pointers have copy semantics.
    #[test]
    fn test_verify_raw_pointer_copy() {
        let input = r#"function @test(v0: ref<raw ref<raw i32>>) -> void {
block0(v0: ref<raw ref<raw i32>>):
    v1 = raw.alloc i32 -> ref<raw i32>
    store v0, v1
    v2 = iconst 42i32
    store v1, v2
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_no_errors();
    }

    /// Pointer used after raw.free is detected.
    #[test]
    fn test_detect_use_after_raw_free() {
        let input = r#"function @test() -> void {
block0:
    v0 = raw.alloc i32 -> ref<raw i32>
    raw.free v0
    v1 = iconst 42i32
    store v0, v1
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_error(|e| matches!(e, OptimizeError::UseAfterMove { .. }));
    }

    /// Cast doesn't move its argument.
    #[test]
    fn test_verify_cast_no_move() {
        let input = r#"function @test() -> i64 {
block0:
    v0 = iconst 42i32
    v1 = sextend v0 -> i64
    v2 = iadd v0, v0
    v3 = sextend v2 -> i64
    v4 = iadd v1, v3
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_no_errors();
    }

    /// Unary operation doesn't move its argument.
    #[test]
    fn test_verify_unary_no_move() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    v1 = ineg v0
    v2 = iadd v0, v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_no_errors();
    }

    /// Multiple sequential operations work correctly.
    #[test]
    fn test_verify_sequential_operations() {
        let input = r#"function @test() -> i32 {
local0: i32
local1: i32
block0:
    v0 = iconst 42i32
    v1 = iconst 10i32
    local.set local0, v0
    local.set local1, v1
    v2 = local.get local0
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_no_errors();
    }

    /// Owned reference used after store is detected.
    #[test]
    fn test_detect_owned_ref_use_after_store() {
        let input = r#"function @test(v0: ref<raw ref<owned i32>>) -> void {
block0(v0: ref<raw ref<owned i32>>):
    v1 = managed.alloc i32 -> ref<managed i32>
    store v0, v1
    raw.drop v1
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_error(|e| matches!(e, OptimizeError::UseAfterMove { .. }));
    }

    /// Owned reference can be used before store.
    #[test]
    fn test_verify_owned_ref_use_before_store() {
        let input = r#"function @test(v0: ref<raw ref<owned i32>>) -> void {
block0(v0: ref<raw ref<owned i32>>):
    v1 = managed.alloc i32 -> ref<managed i32>
    v2 = iconst 42i32
    store v1, v2
    store v0, v1
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_no_errors();
    }

    /// Managed reference used after store is detected.
    #[test]
    fn test_detect_managed_ref_use_after_store() {
        let input = r#"function @test(v0: ref<raw ref<managed i32>>) -> void {
block0(v0: ref<raw ref<managed i32>>):
    v1 = managed.alloc i32 -> ref<managed i32>
    store v0, v1
    v2 = iconst 42i32
    store v1, v2
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_error(|e| matches!(e, OptimizeError::UseAfterMove { .. }));
    }

    /// Owned reference used after local.set is detected.
    #[test]
    fn test_detect_owned_ref_use_after_local_set() {
        let input = r#"function @test() -> void {
local0: ref<owned i32>
block0:
    v0 = managed.alloc i32 -> ref<managed i32>
    local.set local0, v0
    raw.drop v0
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_error(|e| matches!(e, OptimizeError::UseAfterMove { .. }));
    }

    /// Owned reference used after call is detected.
    #[test]
    fn test_detect_owned_ref_use_after_call() {
        let input = r#"extern function @consume(ref<owned i32>) -> void

function @test() -> void {
block0:
    v0 = managed.alloc i32 -> ref<managed i32>
    call @consume(v0) -> fn(ref<owned i32>) -> void
    raw.drop v0
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_error(|e| matches!(e, OptimizeError::UseAfterMove { .. }));
    }

    /// Borrowed reference can be used after store (copy semantics).
    #[test]
    fn test_verify_borrowed_ref_copy_after_store() {
        let input = r#"function @test(v0: ref<raw ref<borrowed i32>>, v1: ref<borrowed i32>) -> void {
block0(v0: ref<raw ref<borrowed i32>>, v1: ref<borrowed i32>):
    store v0, v1
    v2 = iconst 42i32
    store v1, v2
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_no_errors();
    }

    // Tests for aggregate construction

    /// Tuple construction moves owned field.
    #[test]
    fn test_detect_tuple_moves_owned_field() {
        let input = r#"function @test() -> (ref<owned i32>,) {
block0:
    v0 = managed.alloc i32 -> ref<managed i32>
    v1 = tuple (ref<owned i32>,) (v0)
    raw.drop v0
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_error(|e| matches!(e, OptimizeError::UseAfterMove { .. }));
    }

    /// Tuple construction with copy type doesn't move.
    #[test]
    fn test_verify_tuple_copy_field() {
        let input = r#"function @test() -> (i32,) {
block0:
    v0 = iconst 42i32
    v1 = tuple (i32,) (v0)
    v2 = iadd v0, v0
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_no_errors();
    }

    /// Struct construction moves owned field.
    #[test]
    fn test_detect_struct_moves_owned_field() {
        let input = r#"function @test() -> { ref<owned i32> } {
block0:
    v0 = managed.alloc i32 -> ref<managed i32>
    v1 = struct { ref<owned i32> } (v0)
    raw.drop v0
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_error(|e| matches!(e, OptimizeError::UseAfterMove { .. }));
    }

    /// Array construction moves owned elements.
    #[test]
    fn test_detect_array_moves_owned_elements() {
        let input = r#"function @test() -> [ref<owned i32>; 1] {
block0:
    v0 = managed.alloc i32 -> ref<managed i32>
    v1 = array [ref<owned i32>; 1] (v0)
    raw.drop v0
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_error(|e| matches!(e, OptimizeError::UseAfterMove { .. }));
    }

    // Tests for field.set/element.set

    /// field.set moves owned value.
    #[test]
    fn test_detect_field_set_moves_owned() {
        let input = r#"function @test(v0: { ref<owned i32> }) -> { ref<owned i32> } {
block0(v0: { ref<owned i32> }):
    v1 = managed.alloc i32 -> ref<managed i32>
    v2 = field.set v0, 0, v1
    raw.drop v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_error(|e| matches!(e, OptimizeError::UseAfterMove { .. }));
    }

    /// element.set moves owned value.
    #[test]
    fn test_detect_element_set_moves_owned() {
        let input = r#"function @test(v0: [ref<owned i32>; 2]) -> [ref<owned i32>; 2] {
block0(v0: [ref<owned i32>; 2]):
    v1 = managed.alloc i32 -> ref<managed i32>
    v2 = iconst 0u64
    v3 = element.set v0, v2, v1
    raw.drop v1
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_error(|e| matches!(e, OptimizeError::UseAfterMove { .. }));
    }

    /// Loop that moves value on back edge is detected.
    #[test]
    fn test_detect_loop_moves_value() {
        let input = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    v1 = managed.alloc i32 -> ref<managed i32>
    jump block1(v1)
block1(v2: ref<owned i32>):
    raw.drop v2
    branch v0, block1(v2), block2
block2:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_error(|e| matches!(e, OptimizeError::UseAfterMove { .. }));
    }

    /// Loop with fresh value each iteration is OK.
    #[test]
    fn test_verify_loop_fresh_value() {
        let input = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    jump block1
block1:
    v1 = managed.alloc i32 -> ref<managed i32>
    raw.drop v1
    branch v0, block1, block2
block2:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_no_errors();
    }

    // Tests for terminators

    /// CallIndirect moves owned arguments.
    #[test]
    fn test_detect_call_indirect_moves_owned() {
        let input = r#"function @test(v0: fn(ref<owned i32>) -> void) -> void {
block0(v0: fn(ref<owned i32>) -> void):
    v1 = managed.alloc i32 -> ref<managed i32>
    call.indirect v0(v1) -> fn(ref<owned i32>) -> void
    raw.drop v1
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_error(|e| matches!(e, OptimizeError::UseAfterMove { .. }));
    }

    /// Switch uses value correctly.
    #[test]
    fn test_detect_switch_use_after_move() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    raw.drop v0
    switch v0, block3, 0 => block1, 1 => block2
block1:
    v1 = iconst 1i32
    return v1
block2:
    v2 = iconst 2i32
    return v2
block3:
    v3 = iconst 0i32
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_error(|e| matches!(e, OptimizeError::UseAfterMove { .. }));
    }

    /// Jump with moved owned argument is detected.
    #[test]
    fn test_detect_jump_moved_owned_arg() {
        let input = r#"function @test() -> void {
block0:
    v0 = managed.alloc i32 -> ref<managed i32>
    raw.drop v0
    jump block1(v0)
block1(v1: ref<owned i32>):
    raw.drop v1
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_error(|e| matches!(e, OptimizeError::UseAfterMove { .. }));
    }

    /// Return with moved owned value is detected.
    #[test]
    fn test_detect_return_moved_owned() {
        let input = r#"function @test() -> ref<owned i32> {
block0:
    v0 = managed.alloc i32 -> ref<managed i32>
    raw.drop v0
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);
        test.assert_error(|e| matches!(e, OptimizeError::UseAfterMove { .. }));
    }

    /// Multiple use-after-move errors are all reported.
    #[test]
    fn test_multiple_errors_reported() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    raw.drop v0
    v1 = iadd v0, v0
    v2 = iadd v0, v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&MoveCheck);

        // count use-after-move errors
        let use_after_move_count = test
            .errors()
            .iter()
            .filter(|e| matches!(e, OptimizeError::UseAfterMove { .. }))
            .count();

        // v0 used in iadd on v1 line (both operands), and v0 used again in iadd on v2 line
        // the checker reports one error per use, so we expect 3 errors (v0 appears 3 times after drop)
        assert_eq!(
            use_after_move_count, 3,
            "expected 3 UseAfterMove errors: v0 used twice in v1=iadd, once in v2=iadd"
        );
    }
}
