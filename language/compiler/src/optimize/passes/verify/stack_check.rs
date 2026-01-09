use std::collections::HashMap;

use destack_compiler_macros::declare_pass;
use destack_mir as mir;
use destack_source::ModuleId;
use destack_workspace::TargetId;
use mir::{Instruction, Value};

use crate::OptimizeError;
use crate::optimize::{
    AnalysisPreservation, ControlFlowGraph, FunctionPass, Lattice, OptimizationContext, Pass,
    PassMetadata, forward_dataflow,
};

declare_pass! {
    /// Verify stack safety.
    ///
    /// Tracks pointers to stack-allocated memory and detects escapes:
    /// - Return escape: returning a pointer to stack memory
    /// - Store escape: storing a stack pointer to a heap/global location
    /// - Yield escape: yielding a stack pointer from a coroutine
    ///
    /// Uses forward dataflow analysis to correctly handle stack pointers that flow
    /// through control flow joins and block parameters.
    ///
    /// Stack pointers originate from `stack.alloc` and propagate through
    /// `field.addr`, `element.addr`, `field.set`, `element.set`, and `cast`.
    /// Storing a stack pointer to another stack location is allowed.
    #[pass(id = "stack-check")]
    pub StackCheck,
    "Verify stack safety"
}

impl Pass for StackCheck {
    fn metadata(&self) -> &'static PassMetadata {
        StackCheck::metadata()
    }
}

/// Stack pointer state for a single value.
///
/// Forms a lattice:
/// ```text
///     NonStack (top - definitely not stack)
///        |
///    MaybeStack (might be stack on some paths)
///        |
///      Stack (bottom - definitely stack)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StackPointerState {
    /// Definitely not pointing to stack memory.
    NonStack,
    /// May or may not point to stack memory (unknown on some paths).
    MaybeStack,
    /// Definitely points to stack memory.
    Stack,
}

impl StackPointerState {
    /// Compute the meet of two stack pointer states.
    ///
    /// Conservative: if either could be stack, result reflects that.
    fn meet(self, other: Self) -> Self {
        match (self, other) {
            (Self::NonStack, Self::NonStack) => Self::NonStack,
            (Self::Stack, Self::Stack) => Self::Stack,
            // different states on different paths = maybe
            _ => Self::MaybeStack,
        }
    }

    /// Check if this state indicates a possible stack pointer.
    fn is_maybe_stack(self) -> bool {
        matches!(self, Self::Stack | Self::MaybeStack)
    }
}

/// Stack pointer state for all values at a program point.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct StackPointerMap(HashMap<Value, StackPointerState>);

impl StackPointerMap {
    /// Create an empty map.
    fn new() -> Self {
        Self(HashMap::new())
    }

    /// Get the state for a value (NonStack if not tracked).
    fn get(&self, value: Value) -> StackPointerState {
        self.0
            .get(&value)
            .copied()
            .unwrap_or(StackPointerState::NonStack)
    }

    /// Mark a value as definitely pointing to stack memory.
    fn mark_stack(&mut self, value: Value) {
        self.0.insert(value, StackPointerState::Stack);
    }

    /// Propagate state from source to destination if source is tracked.
    fn propagate(&mut self, source: Value, destination: Value) {
        let state = self.get(source);
        if state != StackPointerState::NonStack {
            self.0.insert(destination, state);
        }
    }

    /// Apply the effects of an instruction on stack pointer state.
    fn apply_instruction_effects(&mut self, instruction: &Instruction) {
        match instruction {
            // stack.alloc creates a stack pointer
            Instruction::StackAlloc { destination, .. } => {
                self.mark_stack(*destination);
            }

            // field.addr of a stack pointer is also a stack pointer
            Instruction::FieldAddr {
                destination,
                aggregate,
                ..
            } => {
                self.propagate(*aggregate, *destination);
            }

            // element.addr of a stack pointer is also a stack pointer
            Instruction::ElementAddr {
                destination, array, ..
            } => {
                self.propagate(*array, *destination);
            }

            // field.set propagates stack-ness of the aggregate
            Instruction::FieldSet {
                destination,
                aggregate,
                ..
            } => {
                self.propagate(*aggregate, *destination);
            }

            // element.set propagates stack-ness of the array
            Instruction::ElementSet {
                destination, array, ..
            } => {
                self.propagate(*array, *destination);
            }

            // cast of stack pointer is still a stack pointer
            Instruction::Cast {
                destination,
                argument,
                ..
            } => {
                self.propagate(*argument, *destination);
            }

            _ => {}
        }
    }

    /// Apply terminator effects on stack pointer state.
    ///
    /// This handles propagation of stack pointers through block parameters.
    fn apply_terminator_effects(&mut self, terminator: &mir::Terminator, tree: &mir::NodeTree) {
        // for jump/branch, propagate stack state to block parameter values
        // the dataflow framework handles this via its worklist algorithm
        // we just need to make sure the state correctly reflects what flows out
        match terminator {
            mir::Terminator::Jump {
                target, arguments, ..
            } => {
                let target_block = tree.get(*target);
                for (arg, param) in arguments.iter().zip(&target_block.parameters) {
                    if self.get(*arg).is_maybe_stack() {
                        // parameter will receive stack pointer - handled by dataflow merge
                        // but we mark in our state so it propagates correctly
                        self.propagate(*arg, param.value);
                    }
                }
            }
            mir::Terminator::Branch {
                then_target,
                then_arguments,
                else_target,
                else_arguments,
                ..
            } => {
                // propagate to then branch params
                let then_block = tree.get(*then_target);
                for (arg, param) in then_arguments.iter().zip(&then_block.parameters) {
                    self.propagate(*arg, param.value);
                }

                // propagate to else branch params
                let else_block = tree.get(*else_target);
                for (arg, param) in else_arguments.iter().zip(&else_block.parameters) {
                    self.propagate(*arg, param.value);
                }
            }
            _ => {}
        }
    }

    /// Check an instruction for stack pointer escapes.
    fn check_instruction_escapes(
        &self,
        instruction_id: mir::LocalNodeId<Instruction>,
        instruction: &Instruction,
        module_id: &ModuleId,
        target_id: &TargetId,
        context: &OptimizationContext<'_>,
    ) {
        match instruction {
            // store stack pointer to non-stack location = escape
            Instruction::Store { pointer, value } => {
                if self.get(*value).is_maybe_stack() && !self.get(*pointer).is_maybe_stack() {
                    context.emit_error(OptimizeError::LocalReferenceEscapes {
                        node: instruction_id
                            .into_any()
                            .into_anchored(*module_id, target_id.clone()),
                    });
                }
            }

            // NOTE #Incomplete: call with stack pointer arg is allowed here; interprocedural escape
            // analysis would require function signature annotations (e.g., @noescape)
            Instruction::Call { .. } | Instruction::CallIndirect { .. } => {}

            // field.set with stack pointer into non-stack aggregate = escape
            Instruction::FieldSet {
                aggregate, value, ..
            } => {
                if self.get(*value).is_maybe_stack() && !self.get(*aggregate).is_maybe_stack() {
                    context.emit_error(OptimizeError::LocalReferenceEscapes {
                        node: instruction_id
                            .into_any()
                            .into_anchored(*module_id, target_id.clone()),
                    });
                }
            }

            // element.set with stack pointer into non-stack array = escape
            Instruction::ElementSet { array, value, .. } => {
                if self.get(*value).is_maybe_stack() && !self.get(*array).is_maybe_stack() {
                    context.emit_error(OptimizeError::LocalReferenceEscapes {
                        node: instruction_id
                            .into_any()
                            .into_anchored(*module_id, target_id.clone()),
                    });
                }
            }

            _ => {}
        }
    }

    /// Check a terminator for stack pointer escapes.
    fn check_terminator_escapes(
        &self,
        block_id: mir::LocalNodeId<mir::Block>,
        terminator: &mir::Terminator,
        module_id: &ModuleId,
        target_id: &TargetId,
        context: &OptimizationContext<'_>,
    ) {
        match terminator {
            // returning a stack pointer = escape
            mir::Terminator::Return { value: Some(v) } => {
                if self.get(*v).is_maybe_stack() {
                    context.emit_error(OptimizeError::ReturnReferenceToLocal {
                        node: block_id
                            .into_any()
                            .into_anchored(*module_id, target_id.clone()),
                    });
                }
            }

            // passing stack pointers to block params is fine (same function, stack frame alive)
            mir::Terminator::Jump { .. } | mir::Terminator::Branch { .. } => {}

            // yielding a stack pointer = escape (coroutine could be resumed after stack frame gone)
            mir::Terminator::Yield { value, .. } => {
                if self.get(*value).is_maybe_stack() {
                    context.emit_error(OptimizeError::LocalReferenceEscapes {
                        node: block_id
                            .into_any()
                            .into_anchored(*module_id, target_id.clone()),
                    });
                }
            }

            _ => {}
        }
    }
}

impl Lattice for StackPointerMap {
    fn meet(&self, other: &Self) -> Self {
        let mut result = self.0.clone();

        for (&value, &state_b) in &other.0 {
            result
                .entry(value)
                .and_modify(|state_a| {
                    *state_a = state_a.meet(state_b);
                })
                .or_insert(state_b);
        }

        StackPointerMap(result)
    }
}

impl FunctionPass for StackCheck {
    fn run_on_function(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        context: &OptimizationContext<'_>,
    ) -> AnalysisPreservation {
        if function.entry.is_none() {
            return AnalysisPreservation::all();
        }

        let cfg = context
            .analyses
            .get::<ControlFlowGraph>(function, tree, context);

        // run forward dataflow to compute stack pointer states
        let result = forward_dataflow(
            function,
            tree,
            &cfg,
            StackPointerMap::new(),
            |block_id, mut state, tree| {
                let block = tree.get(block_id);

                // process each instruction
                for &instruction_id in &block.instructions {
                    let instruction = tree.get(instruction_id);
                    state.apply_instruction_effects(instruction);
                }

                // propagate to successors via jump arguments
                state.apply_terminator_effects(&block.terminator, tree);

                state
            },
        );

        // check for escapes using computed states
        let module_id = context.module_id();
        let target_id = context.target_id().clone();

        for &block_id in &function.blocks {
            let Some(entry_state) = result.entry(block_id) else {
                continue;
            };

            let block = tree.get(block_id);

            // track state as we process instructions
            let mut current_state = entry_state.clone();

            // check each instruction
            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                current_state.check_instruction_escapes(
                    instruction_id,
                    instruction,
                    &module_id,
                    &target_id,
                    context,
                );
                current_state.apply_instruction_effects(instruction);
            }

            // check terminator
            current_state.check_terminator_escapes(
                block_id,
                &block.terminator,
                &module_id,
                &target_id,
                context,
            );
        }

        AnalysisPreservation::all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OptimizeError;
    use crate::optimize::common::tests::TestProgram;

    /// Function without stack allocations passes verification.
    #[test]
    fn test_verify_no_stack_allocs() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&StackCheck);
        program.assert_no_errors();
        program.assert_unchanged(input);
    }

    /// Stack allocation used locally passes verification.
    #[test]
    fn test_verify_local_stack_use() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    v2 = load v0
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&StackCheck);
        program.assert_no_errors();
        program.assert_unchanged(input);
    }

    /// Returning a stack pointer directly is detected.
    #[test]
    fn test_detect_return_stack_pointer() {
        let input = r#"function @test() -> ref<raw i32> {
block0:
    v0 = stack.alloc i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&StackCheck);
        program.assert_error(|e| matches!(e, OptimizeError::ReturnReferenceToLocal { .. }));
    }

    /// Returning field address of stack allocation is detected.
    #[test]
    fn test_detect_return_stack_field_addr() {
        let input = r#"function @test() -> ref<raw i32> {
block0:
    v0 = stack.alloc i32
    v1 = field.addr v0, 0
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&StackCheck);
        program.assert_error(|e| matches!(e, OptimizeError::ReturnReferenceToLocal { .. }));
    }

    /// Returning element address of stack allocation is detected.
    #[test]
    fn test_detect_return_stack_element_addr() {
        let input = r#"function @test() -> ref<raw i32> {
block0:
    v0 = stack.alloc i32
    v1 = iconst 0i32
    v2 = element.addr v0, v1
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&StackCheck);
        program.assert_error(|e| matches!(e, OptimizeError::ReturnReferenceToLocal { .. }));
    }

    /// Loading from stack pointer and returning value is valid.
    #[test]
    fn test_verify_load_from_stack() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    v2 = load v0
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&StackCheck);
        program.assert_no_errors();
    }

    /// Storing stack pointer to heap location is detected.
    #[test]
    fn test_detect_store_stack_to_heap() {
        let input = r#"function @test(v0: ref<raw ref<raw i32>>) -> void {
block0(v0: ref<raw ref<raw i32>>):
    v1 = stack.alloc i32
    store v0, v1
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&StackCheck);
        program.assert_error(|e| matches!(e, OptimizeError::LocalReferenceEscapes { .. }));
    }

    /// Storing stack pointer to another stack location is valid.
    #[test]
    fn test_verify_store_stack_to_stack() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = stack.alloc ref<raw i32>
    v2 = iconst 42i32
    store v0, v2
    store v1, v0
    v3 = load v1
    v4 = load v3
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&StackCheck);
        program.assert_no_errors();
    }

    /// Cast of stack pointer still tracks as stack pointer.
    #[test]
    fn test_detect_cast_stack_pointer_return() {
        let input = r#"function @test() -> ref<raw i8> {
block0:
    v0 = stack.alloc i32
    v1 = bitcast v0 -> ref<raw i8>
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&StackCheck);
        program.assert_error(|e| matches!(e, OptimizeError::ReturnReferenceToLocal { .. }));
    }

    /// Heap pointer can be returned.
    #[test]
    fn test_verify_return_heap_pointer() {
        let input = r#"function @test() -> ref<raw i32> {
block0:
    v0 = raw.alloc i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&StackCheck);
        program.assert_no_errors();
    }

    /// Function parameter pointer can be returned.
    #[test]
    fn test_verify_return_param_pointer() {
        let input = r#"function @test(v0: ref<raw i32>) -> ref<raw i32> {
block0(v0: ref<raw i32>):
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&StackCheck);
        program.assert_no_errors();
    }

    /// field.set with stack pointer into non-stack aggregate is detected.
    #[test]
    fn test_detect_field_set_stack_escape() {
        let input = r#"function @test(v0: ref<raw ref<raw i32>>) -> void {
block0(v0: ref<raw ref<raw i32>>):
    v1 = stack.alloc i32
    v2 = field.set v0, 0, v1
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&StackCheck);
        program.assert_error(|e| matches!(e, OptimizeError::LocalReferenceEscapes { .. }));
    }

    /// element.set with stack pointer into non-stack array is detected.
    #[test]
    fn test_detect_element_set_stack_escape() {
        let input = r#"function @test(v0: ref<raw ref<raw i32>>) -> void {
block0(v0: ref<raw ref<raw i32>>):
    v1 = stack.alloc i32
    v2 = iconst 0i32
    v3 = element.set v0, v2, v1
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&StackCheck);
        program.assert_error(|e| matches!(e, OptimizeError::LocalReferenceEscapes { .. }));
    }

    /// Nested field addresses of stack are tracked.
    #[test]
    fn test_detect_nested_field_addr_stack() {
        let input = r#"function @test() -> ref<raw i32> {
block0:
    v0 = stack.alloc i32
    v1 = field.addr v0, 0
    v2 = field.addr v1, 0
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&StackCheck);
        program.assert_error(|e| matches!(e, OptimizeError::ReturnReferenceToLocal { .. }));
    }

    /// Control flow with stack pointer used locally is valid.
    #[test]
    fn test_verify_control_flow_stack_local() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = stack.alloc i32
    branch v0, block1, block2
block1:
    v2 = iconst 1i32
    store v1, v2
    jump block3
block2:
    v3 = iconst 2i32
    store v1, v3
    jump block3
block3:
    v4 = load v1
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&StackCheck);
        program.assert_no_errors();
    }

    /// Multiple stack allocations used locally are valid.
    #[test]
    fn test_verify_multiple_stack_allocs() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = stack.alloc i32
    v2 = iconst 10i32
    v3 = iconst 20i32
    store v0, v2
    store v1, v3
    v4 = load v0
    v5 = load v1
    v6 = iadd v4, v5
    return v6
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&StackCheck);
        program.assert_no_errors();
    }

    /// Void return with stack allocation used locally is valid.
    #[test]
    fn test_verify_void_return_with_stack() {
        let input = r#"function @test() -> void {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&StackCheck);
        program.assert_no_errors();
    }

    /// Stack pointer passed through block parameter is tracked.
    #[test]
    fn test_detect_stack_pointer_through_block_param() {
        let input = r#"function @test() -> ref<raw i32> {
block0:
    v0 = stack.alloc i32
    jump block1(v0)
block1(v1: ref<raw i32>):
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&StackCheck);
        program.assert_error(|e| matches!(e, OptimizeError::ReturnReferenceToLocal { .. }));
    }

    /// Stack pointer through multiple jump hops is tracked.
    #[test]
    fn test_detect_stack_escape_through_multiple_jumps() {
        let input = r#"function @test() -> ref<raw i32> {
block0:
    v0 = stack.alloc i32
    jump block1(v0)
block1(v1: ref<raw i32>):
    jump block2(v1)
block2(v2: ref<raw i32>):
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&StackCheck);
        program.assert_error(|e| matches!(e, OptimizeError::ReturnReferenceToLocal { .. }));
    }

    /// Stack pointer used locally through block params is valid.
    #[test]
    fn test_verify_stack_through_block_param_local_use() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    jump block1(v0)
block1(v1: ref<raw i32>):
    v2 = load v1
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&StackCheck);
        program.assert_no_errors();
    }

    /// Loop with stack pointer used locally is valid.
    #[test]
    fn test_verify_loop_with_stack_local() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = stack.alloc i32
    v2 = iconst 0i32
    store v1, v2
    jump block1(v0)
block1(v3: i32):
    v4 = iconst 0i32
    v5 = icmp_sgt v3, v4
    branch v5, block2, block3
block2:
    v6 = load v1
    v7 = iadd v6, v3
    store v1, v7
    v8 = iconst 1i32
    v9 = isub v3, v8
    jump block1(v9)
block3:
    v10 = load v1
    return v10
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&StackCheck);
        program.assert_no_errors();
    }

    /// Diamond control flow: stack on one branch, heap on other = maybe stack at merge.
    #[test]
    fn test_detect_maybe_stack_escape_diamond() {
        let input = r#"function @test(v0: bool) -> ref<raw i32> {
block0(v0: bool):
    branch v0, block1, block2
block1:
    v1 = stack.alloc i32
    jump block3(v1)
block2:
    v2 = raw.alloc i32
    jump block3(v2)
block3(v3: ref<raw i32>):
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&StackCheck);
        // v3 might be stack (from block1) so returning it is an error
        program.assert_error(|e| matches!(e, OptimizeError::ReturnReferenceToLocal { .. }));
    }

    /// Diamond control flow: stack pointers on both branches used locally is valid.
    #[test]
    fn test_verify_stack_both_branches_local_use() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = stack.alloc i32
    v2 = stack.alloc i32
    v3 = iconst 1i32
    v4 = iconst 2i32
    store v1, v3
    store v2, v4
    branch v0, block1, block2
block1:
    jump block3(v1)
block2:
    jump block3(v2)
block3(v5: ref<raw i32>):
    v6 = load v5
    return v6
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&StackCheck);
        // loading and returning the *value* is fine, only returning the pointer is bad
        program.assert_no_errors();
    }

    // NOTE: yield escape test omitted because MIR text parser doesn't support yield terminators.
    // The implementation in check_terminator_escapes correctly handles Yield.
}
