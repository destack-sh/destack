use std::collections::HashMap;

use destack_compiler_macros::declare_pass;
use destack_mir as mir;
use destack_source::ModuleId;
use destack_workspace::TargetId;
use mir::{Instruction, Value};

use crate::OptimizeError;
use crate::optimize::{
    AnalysisPreservation, ControlFlowGraph, DiagnosticEmitter, FunctionPass,
    Lattice, LifetimeAnalysis, PipelineContext, ResolvedLifetime, forward_dataflow,
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
    /// `field.addr`, `element.addr`, `field.set`, `element.set`, `cast`, and function
    /// calls (via lifetime analysis). Storing a stack pointer to another stack
    /// location is allowed.
    #[pass(id = "stack-check")]
    pub StackCheck,
    "Verify stack safety"
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

    /// Apply call instruction effects based on lifetime analysis.
    ///
    /// If a call returns a borrowed reference that borrows from arguments
    /// that are stack pointers, the return value is also a stack pointer.
    fn apply_call_effects(
        &mut self,
        instruction: &Instruction,
        tree: &mir::NodeTree,
        lifetime_analysis: &LifetimeAnalysis,
    ) {
        let (destination, callee_id, arguments) = match instruction {
            Instruction::Call {
                destination: Some(dest),
                function,
                arguments,
            } => (*dest, *function, tree.get_arguments(*arguments)),
            // indirect calls: conservative, can't analyze lifetime
            // calls without destination: nothing to track
            _ => return,
        };

        // get lifetime for the callee
        let lifetime = lifetime_analysis.get(callee_id);
        match lifetime {
            // no borrowed references in return: destination is not a stack pointer
            ResolvedLifetime::None => {}

            // static lifetime: return borrows from global/static, not arguments
            ResolvedLifetime::Static => {}

            // return borrows from specific parameters
            ResolvedLifetime::Parameters(param_indices) => {
                for &param_idx in param_indices {
                    if let Some(&arg) = arguments.get(param_idx as usize) {
                        // if argument is a stack pointer, return is too
                        if self.get(arg).is_maybe_stack() {
                            self.0.insert(destination, self.get(arg));
                            return; // once we find one stack arg, we're done
                        }
                    }
                }
            }
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
            mir::Terminator::Check {
                success, failure, ..
            } => {
                let success_block = tree.get(success.target);
                for (arg, param) in success.arguments.iter().zip(&success_block.parameters) {
                    self.propagate(*arg, param.value);
                }

                let failure_block = tree.get(failure.target);
                for (arg, param) in failure.arguments.iter().zip(&failure_block.parameters) {
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
        context: &impl DiagnosticEmitter,
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

            // calls are handled via apply_call_effects using lifetime analysis
            // indirect calls are conservative (can't analyze lifetime)
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
        context: &impl DiagnosticEmitter,
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
            mir::Terminator::Jump { .. }
            | mir::Terminator::Branch { .. }
            | mir::Terminator::Check { .. } => {}

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

/// Run stack check on a function.
fn run_stack_check(
    function: &mir::Function,
    tree: &mir::NodeTree,
    cfg: &ControlFlowGraph,
    lifetime_analysis: &LifetimeAnalysis,
    module_id: ModuleId,
    target_id: TargetId,
    context: &impl DiagnosticEmitter,
) {
    if function.entry.is_none() {
        return;
    }

    // run forward dataflow to compute stack pointer states
    let result = forward_dataflow(
        function,
        tree,
        cfg,
        StackPointerMap::new(),
        |block_id, mut state, tree| {
            let block = tree.get(block_id);

            // process each instruction
            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                state.apply_instruction_effects(instruction);
                state.apply_call_effects(instruction, tree, lifetime_analysis);
            }

            // propagate to successors via jump arguments
            state.apply_terminator_effects(&block.terminator, tree);

            state
        },
    );

    // check for escapes using computed states
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
            current_state.apply_call_effects(instruction, tree, lifetime_analysis);
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
}

impl FunctionPass for StackCheck {
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // get function-level analyses
        let cfg = {
            let analyses = ctx.function_analyses(function, tree);
            analyses.get::<ControlFlowGraph>().clone()
        };

        // get module-level lifetime analysis
        let lifetime_analysis = ctx.module_analyses(tree).get::<LifetimeAnalysis>().clone();

        run_stack_check(
            function,
            tree,
            &cfg,
            &lifetime_analysis,
            ctx.module_id(),
            ctx.target_id().clone(),
            ctx,
        );

        AnalysisPreservation::all()
    }

    fn name(&self) -> &'static str {
        "StackCheck"
    }

    fn id(&self) -> &'static str {
        "stack-check"
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
block1(v2: ref<raw i32>):
    v3 = load v2
    return v3
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

    /// Set the return lifetime for a function by name.
    fn set_function_lifetime(program: &mut TestProgram, name: &str, lifetime: mir::Lifetime) {
        let fn_id = program
            .tree
            .iter_nodes::<mir::Function>()
            .find(|(_, f)| program.get_string(f.name) == name)
            .map(|(id, _)| id)
            .unwrap_or_else(|| panic!("function '{name}' not found"));
        let function = program.tree.get_mut(fn_id);
        function.return_lifetime = lifetime;
    }

    /// Stack pointer escapes through identity function call.
    ///
    /// When a function returns a borrowed reference that borrows from an argument,
    /// and that argument is a stack pointer, the return value is also a stack pointer.
    #[test]
    fn test_detect_stack_escape_through_call() {
        let input = r#"function @identity(v0: ref<borrowed i32>) -> ref<borrowed i32> {
block0(v0: ref<borrowed i32>):
    return v0
}

function @test() -> ref<borrowed i32> {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    v2 = call @identity(v0)
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&StackCheck);

        // v2 = call @identity(v0) where v0 is stack pointer
        // @identity returns borrowed ref from param 0, so v2 is stack pointer
        // returning v2 is a stack escape
        program.assert_error(|e| matches!(e, OptimizeError::ReturnReferenceToLocal { .. }));
    }

    /// Static lifetime prevents stack pointer propagation through call.
    ///
    /// When a function has static lifetime, its return doesn't borrow from
    /// arguments, so stack pointer status doesn't propagate.
    #[test]
    fn test_static_lifetime_no_stack_propagation() {
        let input = r#"function @getStatic(v0: ref<borrowed i32>) -> ref<borrowed i32> {
block0(v0: ref<borrowed i32>):
    return v0
}

function @test() -> ref<borrowed i32> {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    v2 = call @getStatic(v0)
    return v2
}"#;

        let mut program = TestProgram::new(input);
        set_function_lifetime(&mut program, "getStatic", mir::Lifetime::Static);
        program.run_pass(&StackCheck);

        // with static lifetime, v2 doesn't inherit stack pointer status from v0
        // so returning v2 is allowed (from stack-check's perspective)
        // (this may still be incorrect at runtime, but that's a different issue)
        program.assert_no_errors();
    }

    /// Stack pointer propagates through call with explicit param lifetime.
    ///
    /// When a function has explicit lifetime annotation specifying param 0,
    /// stack pointer status from arg 0 propagates to the return value.
    #[test]
    fn test_explicit_param_lifetime_stack_propagation() {
        let input = r#"function @pick(v0: ref<borrowed i32>, v1: ref<borrowed i32>) -> ref<borrowed i32> {
block0(v0: ref<borrowed i32>, v1: ref<borrowed i32>):
    return v0
}

function @test() -> ref<borrowed i32> {
block0:
    v0 = managed.alloc i32
    v1 = stack.alloc i32
    v2 = iconst 42i32
    store v0, v2
    store v1, v2
    v3 = call @pick(v0, v1)
    return v3
}"#;

        let mut program = TestProgram::new(input);

        // explicit lifetime: return borrows from param 0 only
        set_function_lifetime(&mut program, "pick", mir::Lifetime::param(0));
        program.run_pass(&StackCheck);

        // v3 borrows from v0 (param 0) which is managed, not stack
        // v1 (param 1) is stack but not borrowed from, so v3 is not stack
        program.assert_no_errors();
    }

    /// Stack pointer propagates through call when borrowing from stack arg.
    ///
    /// When explicit lifetime borrows from a param that is a stack pointer,
    /// the return is also a stack pointer.
    #[test]
    fn test_explicit_second_param_lifetime_stack_propagation() {
        let input = r#"function @pick(v0: ref<borrowed i32>, v1: ref<borrowed i32>) -> ref<borrowed i32> {
block0(v0: ref<borrowed i32>, v1: ref<borrowed i32>):
    return v1
}

function @test() -> ref<borrowed i32> {
block0:
    v0 = managed.alloc i32
    v1 = stack.alloc i32
    v2 = iconst 42i32
    store v0, v2
    store v1, v2
    v3 = call @pick(v0, v1)
    return v3
}"#;

        let mut program = TestProgram::new(input);

        // explicit lifetime: return borrows from param 1 only
        set_function_lifetime(&mut program, "pick", mir::Lifetime::param(1));
        program.run_pass(&StackCheck);

        // v3 borrows from v1 (param 1) which is stack
        // returning v3 is a stack escape
        program.assert_error(|e| matches!(e, OptimizeError::ReturnReferenceToLocal { .. }));
    }
}
