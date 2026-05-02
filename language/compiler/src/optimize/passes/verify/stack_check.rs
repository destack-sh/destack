use std::collections::HashMap;

use crate::declare_pass;
use destack_mir as mir;
use mir::{Instruction, Value, ValueReference};

use crate::OptimizeError;
use crate::optimize::{
    AnalysisPreservation, CallTargetAnalysis, ControlFlowGraph, FunctionPass, Lattice,
    LifetimeAnalysis, LivenessAnalysis, PipelineContext, ResolvedLifetime, forward_dataflow,
};

declare_pass! {
    /// Verify stack safety.
    ///
    /// Tracks pointers to frame-local memory and detects escapes:
    /// - Return escape: returning a pointer to stack memory
    /// - Store escape: storing a stack pointer to a heap/global location
    /// - Suspend escape: yielding a stack pointer or suspending with one still live
    ///
    /// Uses forward dataflow analysis to correctly handle stack pointers that flow
    /// through control flow joins and block parameters.
    ///
    /// Frame-local pointers originate from `stack.alloc` and `local.address`, and propagate through
    /// `field.address`, `element.address`, `field.set`, `element.set`, `cast`, and function
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
    fn get(&self, value: impl Into<ValueReference>) -> StackPointerState {
        let Some(value) = value.into().value() else {
            return StackPointerState::NonStack;
        };

        self.0
            .get(&value)
            .copied()
            .unwrap_or(StackPointerState::NonStack)
    }

    /// Mark a value as definitely pointing to stack memory.
    fn mark_stack(&mut self, value: impl Into<ValueReference>) {
        let Some(value) = value.into().value() else {
            return;
        };

        self.0.insert(value, StackPointerState::Stack);
    }

    /// Propagate state from source to destination if source is tracked.
    fn propagate(
        &mut self,
        source: impl Into<ValueReference>,
        destination: impl Into<ValueReference>,
    ) {
        let Some(destination) = destination.into().value() else {
            return;
        };

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

            // local.address yields a pointer to stack storage
            Instruction::LocalAddr { destination, .. } => {
                self.mark_stack(*destination);
            }

            // field.address of a stack pointer is also a stack pointer
            Instruction::FieldAddr {
                destination,
                aggregate,
                ..
            } => {
                self.propagate(*aggregate, *destination);
            }

            // element.address of a stack pointer is also a stack pointer
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
        instruction_id: mir::LocalNodeId<Instruction>,
        instruction: &Instruction,
        tree: &mir::Tree,
        lifetime_analysis: &LifetimeAnalysis,
        call_targets: &CallTargetAnalysis,
    ) {
        match instruction {
            Instruction::Call {
                destination: Some(destination),
                function,
                call,
                ..
            } => {
                let Some(function) = function.function() else {
                    return;
                };

                let arguments = tree.get_arguments(call.arguments);
                let lifetime = lifetime_analysis.get(function);
                self.apply_lifetime_result(*destination, lifetime, arguments);
            }
            Instruction::CallVirtual {
                destination: Some(destination),
                receiver,
                call,
                ..
            }
            | Instruction::CallInterface {
                destination: Some(destination),
                receiver,
                call,
                ..
            } => {
                let mut args = Vec::with_capacity(tree.get_arguments(call.arguments).len() + 1);
                args.push(*receiver);
                args.extend_from_slice(tree.get_arguments(call.arguments));

                if let Some(targets) = call_targets.targets_for_instruction(instruction_id) {
                    self.apply_call_targets(*destination, &args, targets, lifetime_analysis);
                } else {
                    self.apply_signature_lifetime(*destination, call.signature, tree, &args, None);
                }
            }
            Instruction::CallIndirect {
                destination: Some(destination),
                call,
                callee,
                ..
            } => {
                let args = tree.get_arguments(call.arguments);
                if let Some(targets) = call_targets.targets_for_instruction(instruction_id) {
                    self.apply_call_targets(*destination, args, targets, lifetime_analysis);
                } else {
                    self.apply_signature_lifetime(
                        *destination,
                        call.signature,
                        tree,
                        args,
                        Some(*callee),
                    );
                }
            }
            // calls without destination: nothing to track
            _ => {}
        }
    }

    /// Apply resolved call targets to propagate stack pointer state.
    fn apply_call_targets(
        &mut self,
        destination: ValueReference,
        arguments: &[ValueReference],
        targets: &[mir::LocalNodeId<mir::Function>],
        lifetime_analysis: &LifetimeAnalysis,
    ) {
        let mut param_indices = Vec::new();
        for target in targets {
            let lifetime = lifetime_analysis.get(*target);
            if let ResolvedLifetime::Parameters(params) = lifetime {
                for &param in params {
                    if !param_indices.contains(&param) {
                        param_indices.push(param);
                    }
                }
            }
        }

        if !param_indices.is_empty() {
            self.propagate_from_param_indices(destination, arguments, &param_indices, None);
        }
    }

    /// Apply resolved lifetime information to a call destination.
    fn apply_lifetime_result(
        &mut self,
        destination: ValueReference,
        lifetime: &ResolvedLifetime,
        arguments: &[ValueReference],
    ) {
        match lifetime {
            // no borrowed references in return: destination is not a stack pointer
            ResolvedLifetime::None => {}

            // static lifetime: return borrows from global/static, not arguments
            ResolvedLifetime::Static => {}

            // return borrows from specific parameters
            ResolvedLifetime::Parameters(param_indices) => {
                self.propagate_from_param_indices(destination, arguments, param_indices, None);
            }
        }
    }

    /// Apply signature-based lifetime inference when call targets are unknown.
    fn apply_signature_lifetime(
        &mut self,
        destination: ValueReference,
        signature: mir::TypeReference,
        tree: &mir::Tree,
        arguments: &[ValueReference],
        env: Option<ValueReference>,
    ) {
        let lifetime = LifetimeAnalysis::resolve_signature(signature, tree);
        match lifetime {
            ResolvedLifetime::Parameters(param_indices) => {
                self.propagate_from_param_indices(destination, arguments, &param_indices, env);
            }
            ResolvedLifetime::Static | ResolvedLifetime::None => {
                // env can carry hidden borrows for indirect calls
                if let Some(env) = env
                    && self.get(env).is_maybe_stack()
                {
                    self.propagate(env, destination);
                }
            }
        }
    }

    /// Propagate stack pointer state from borrowed parameter indices.
    fn propagate_from_param_indices(
        &mut self,
        destination: ValueReference,
        arguments: &[ValueReference],
        param_indices: &[u32],
        env: Option<ValueReference>,
    ) {
        for &parameter_index in param_indices {
            if let Some(&argument) = arguments.get(parameter_index as usize)
                && self.get(argument).is_maybe_stack()
            {
                self.propagate(argument, destination);
                return;
            }
        }

        if let Some(environment) = env
            && self.get(environment).is_maybe_stack()
        {
            self.propagate(environment, destination);
        }
    }

    /// Apply terminator effects on stack pointer state.
    ///
    /// This handles propagation of stack pointers through block parameters.
    fn apply_terminator_effects(&mut self, terminator: &mir::Terminator, tree: &mir::Tree) {
        // for jump/branch, propagate stack state to block parameter values
        // the dataflow framework handles this via its worklist algorithm
        // we just need to make sure the state correctly reflects what flows out
        match terminator {
            mir::Terminator::Jump { target } => {
                self.propagate_block_target(target, tree);
            }
            mir::Terminator::Branch {
                then_target,
                else_target,
                ..
            } => {
                self.propagate_block_target(then_target, tree);
                self.propagate_block_target(else_target, tree);
            }
            mir::Terminator::Check {
                success, failure, ..
            } => {
                self.propagate_block_target(success, tree);
                self.propagate_block_target(failure, tree);
            }
            _ => {}
        }
    }

    /// Propagate stack state across one block edge.
    fn propagate_block_target(&mut self, target: &mir::BlockTarget, tree: &mir::Tree) {
        let Some(block) = target.block.block() else {
            return;
        };

        let block = tree.get(block);
        for (&argument, parameter) in target.arguments.iter().zip(&block.parameters) {
            self.propagate(argument, parameter.value);
        }
    }

    /// Check an instruction for stack pointer escapes.
    fn check_instruction_escapes(
        &self,
        instruction_id: mir::LocalNodeId<Instruction>,
        instruction: &Instruction,
        tree: &mir::Tree,
        context: &PipelineContext<'_>,
    ) {
        match instruction {
            // store stack pointer to non-stack location = escape
            Instruction::Store { pointer, value } => {
                if self.get(*value).is_maybe_stack() && !self.get(*pointer).is_maybe_stack() {
                    context.emit_error(OptimizeError::LocalReferenceEscapes {
                        anchor: context.anchor(tree, instruction_id.into_any()),
                    });
                }
            }

            // calls are handled via apply_call_effects using lifetime analysis
            // indirect calls are conservative (can't analyze lifetime)
            Instruction::Call { .. }
            | Instruction::CallVirtual { .. }
            | Instruction::CallInterface { .. }
            | Instruction::CallIndirect { .. } => {}

            // field.set with stack pointer into non-stack aggregate = escape
            Instruction::FieldSet {
                aggregate, value, ..
            } => {
                if self.get(*value).is_maybe_stack() && !self.get(*aggregate).is_maybe_stack() {
                    context.emit_error(OptimizeError::LocalReferenceEscapes {
                        anchor: context.anchor(tree, instruction_id.into_any()),
                    });
                }
            }

            // element.set with stack pointer into non-stack array = escape
            Instruction::ElementSet { array, value, .. } => {
                if self.get(*value).is_maybe_stack() && !self.get(*array).is_maybe_stack() {
                    context.emit_error(OptimizeError::LocalReferenceEscapes {
                        anchor: context.anchor(tree, instruction_id.into_any()),
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
        liveness: &LivenessAnalysis,
        tree: &mir::Tree,
        context: &PipelineContext<'_>,
    ) {
        match terminator {
            // returning a stack pointer = escape
            mir::Terminator::Return { value: Some(value) } => {
                if self.get(*value).is_maybe_stack() {
                    context.emit_error(OptimizeError::ReturnReferenceToLocal {
                        anchor: context.anchor(tree, block_id.into_any()),
                    });
                }
            }

            // passing stack pointers to block params is fine (same function, stack frame alive)
            mir::Terminator::Jump { .. }
            | mir::Terminator::Branch { .. }
            | mir::Terminator::Check { .. } => {}

            // yielding or resuming with live frame-local pointers is invalid
            mir::Terminator::Yield { value, .. } => {
                let yields_frame_local_pointer = self.get(*value).is_maybe_stack();
                let suspends_with_live_frame_local_pointer = liveness
                    .live_out(block_id)
                    .iter()
                    .copied()
                    .any(|value| self.get(value).is_maybe_stack());

                if yields_frame_local_pointer || suspends_with_live_frame_local_pointer {
                    context.emit_error(OptimizeError::LocalReferenceEscapes {
                        anchor: context.anchor(tree, block_id.into_any()),
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
#[allow(clippy::too_many_arguments)]
fn run_stack_check(
    function: &mir::Function,
    tree: &mir::Tree,
    cfg: &ControlFlowGraph,
    lifetime_analysis: &LifetimeAnalysis,
    call_targets: &CallTargetAnalysis,
    liveness: &LivenessAnalysis,
    context: &PipelineContext<'_>,
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
                state.apply_call_effects(
                    instruction_id,
                    instruction,
                    tree,
                    lifetime_analysis,
                    call_targets,
                );
            }

            // propagate to successors via jump arguments
            state.apply_terminator_effects(tree.get(block.terminator), tree);

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
            current_state.check_instruction_escapes(instruction_id, instruction, tree, context);
            current_state.apply_instruction_effects(instruction);
            current_state.apply_call_effects(
                instruction_id,
                instruction,
                tree,
                lifetime_analysis,
                call_targets,
            );
        }

        // check terminator
        let terminator = tree.get(block.terminator);
        current_state.check_terminator_escapes(block_id, terminator, liveness, tree, context);
    }
}

impl FunctionPass for StackCheck {
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // get function-level analyses
        let (cfg, liveness) = {
            let analyses = ctx.function_analyses(function, tree);
            (
                analyses.get::<ControlFlowGraph>().clone(),
                analyses.get::<LivenessAnalysis>().clone(),
            )
        };

        // get module-level lifetime analysis
        let module_analyses = ctx.module_analyses(tree);
        let lifetime_analysis = module_analyses.get::<LifetimeAnalysis>().clone();
        let call_targets = module_analyses.get::<CallTargetAnalysis>().clone();

        run_stack_check(
            function,
            tree,
            &cfg,
            &lifetime_analysis,
            &call_targets,
            &liveness,
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
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 42int32
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_no_errors();
        test.assert_unchanged(input);
    }

    /// Stack allocation used locally passes verification.
    #[test]
    fn test_verify_local_stack_use() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: int32 = load v0
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_no_errors();
        test.assert_unchanged(input);
    }

    /// Returning a stack pointer directly is detected.
    #[test]
    fn test_detect_return_stack_pointer() {
        let input = r#"
function test(): ref<int32, raw> {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_error(|e| matches!(e, OptimizeError::ReturnReferenceToLocal { .. }));
    }

    /// Returning a local.address pointer is detected.
    #[test]
    fn test_detect_return_local_addr() {
        let input = r#"
function test(): ref<int32, borrowed> {
    local local0: int32, owned
b0:
    v0: ref<int32, borrowed, space(frame)> = local.address local0
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_error(|e| matches!(e, OptimizeError::ReturnReferenceToLocal { .. }));
    }

    /// Returning field address of stack allocation is detected.
    #[test]
    fn test_detect_return_stack_field_addr() {
        let input = r#"
function test(): ref<int32, raw> {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, borrowed> = field.address v0, 0
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_error(|e| matches!(e, OptimizeError::ReturnReferenceToLocal { .. }));
    }

    /// Returning element address of stack allocation is detected.
    #[test]
    fn test_detect_return_stack_element_addr() {
        let input = r#"
function test(): ref<int32, raw> {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 0int32
    v2: ref<int32, borrowed> = element.address v0, v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_error(|e| matches!(e, OptimizeError::ReturnReferenceToLocal { .. }));
    }

    /// Stack pointer propagates through indirect calls with borrowed returns.
    #[test]
    fn test_detect_stack_through_call_indirect() {
        let input = r#"
function test(v0: fn(ref<int32, borrowed>) -> ref<int32, borrowed>): ref<int32, borrowed>  {
b0(v0: fn(ref<int32, borrowed>) -> ref<int32, borrowed>) -> v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: ref<int32, borrowed> = call.indirect v0(v1): (ref<int32, borrowed>) -> ref<int32, borrowed>
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_error(|e| matches!(e, OptimizeError::ReturnReferenceToLocal { .. }));
    }

    /// Loading from stack pointer and returning value is valid.
    #[test]
    fn test_verify_load_from_stack() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: int32 = load v0
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_no_errors();
    }

    /// Storing stack pointer to heap location is detected.
    #[test]
    fn test_detect_store_stack_to_heap() {
        let input = r#"
function test(v0: ref<ref<int32, raw>, raw>): void {
b0(v0: ref<ref<int32, raw>, raw>):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    store v0, v1
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_error(|e| matches!(e, OptimizeError::LocalReferenceEscapes { .. }));
    }

    /// Storing stack pointer to another stack location is valid.
    #[test]
    fn test_verify_store_stack_to_stack() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<ref<int32, raw, space(stack)>, raw, space(stack)> = stack.alloc ref<int32, raw, space(stack)>
    v2: int32 = 42int32
    store v0, v2
    store v1, v0
    v3: ref<int32, raw> = load v1
    v4: int32 = load v3
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_no_errors();
    }

    /// Cast of stack pointer still tracks as stack pointer.
    #[test]
    fn test_detect_cast_stack_pointer_return() {
        let input = r#"
function test(): ref<int8, raw> {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int8, raw> = cast.bit v0 -> ref<int8, raw>
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_error(|e| matches!(e, OptimizeError::ReturnReferenceToLocal { .. }));
    }

    /// Heap pointer can be returned.
    #[test]
    fn test_verify_return_heap_pointer() {
        let input = r#"
function test(): ref<int32, raw> {
b0:
    v0: ref<int32, raw> = raw.alloc int32
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_no_errors();
    }

    /// Function parameter pointer can be returned.
    #[test]
    fn test_verify_return_param_pointer() {
        let input = r#"
function test(v0: ref<int32, raw>): ref<int32, raw> {
b0(v0: ref<int32, raw>):
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_no_errors();
    }

    /// field.set with stack pointer into non-stack aggregate is detected.
    #[test]
    fn test_detect_field_set_stack_escape() {
        let input = r#"
function test(v0: ref<ref<int32, raw>, raw>): void {
b0(v0: ref<ref<int32, raw>, raw>):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: ref<ref<int32, raw>, raw> = field.set v0, 0, v1
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_error(|e| matches!(e, OptimizeError::LocalReferenceEscapes { .. }));
    }

    /// element.set with stack pointer into non-stack array is detected.
    #[test]
    fn test_detect_element_set_stack_escape() {
        let input = r#"
function test(v0: ref<ref<int32, raw>, raw>): void {
b0(v0: ref<ref<int32, raw>, raw>):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 0int32
    v3: ref<ref<int32, raw>, raw> = element.set v0, v2, v1
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_error(|e| matches!(e, OptimizeError::LocalReferenceEscapes { .. }));
    }

    /// Nested field addresses of stack are tracked.
    #[test]
    fn test_detect_nested_field_addr_stack() {
        let input = r#"
function test(): ref<int32, raw> {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, borrowed> = field.address v0, 0
    v2: ref<int32, borrowed> = field.address v1, 0
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_error(|e| matches!(e, OptimizeError::ReturnReferenceToLocal { .. }));
    }

    /// Control flow with stack pointer used locally is valid.
    #[test]
    fn test_verify_control_flow_stack_local() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    branch v0, b1, b2
b1:
    v2: int32 = 1int32
    store v1, v2
    jump b3
b2:
    v3: int32 = 2int32
    store v1, v3
    jump b3
b3:
    v4: int32 = load v1
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_no_errors();
    }

    /// Multiple stack allocations used locally are valid.
    #[test]
    fn test_verify_multiple_stack_allocs() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 10int32
    v3: int32 = 20int32
    store v0, v2
    store v1, v3
    v4: int32 = load v0
    v5: int32 = load v1
    v6: int32 = int.add v4, v5
    return v6
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_no_errors();
    }

    /// Void return with stack allocation used locally is valid.
    #[test]
    fn test_verify_void_return_with_stack() {
        let input = r#"
function test(): void {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_no_errors();
    }

    /// Yielding while one stack allocation stays live is detected.
    #[test]
    fn test_detect_stack_allocation_live_across_yield() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 1int32
    yield v1, b1
b1(v2: int32):
    store v0, v2
    v3: int32 = load v0
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_error(|e| matches!(e, OptimizeError::LocalReferenceEscapes { .. }));
    }

    /// Yielding after a dead stack allocation is valid.
    #[test]
    fn test_verify_dead_stack_allocation_before_yield() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 1int32
    store v0, v1
    yield v1, b1
b1(v2: int32):
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_no_errors();
    }

    /// Yielding while one local address stays live is detected.
    #[test]
    fn test_detect_local_pointer_live_across_yield() {
        let input = r#"
function test(): int32 {
    local local0: int32, owned
b0:
    v0: int32 = 1int32
    local.set local0, v0
    v1: ref<int32, borrowed, space(frame)> = local.address local0
    yield v0, b1
b1(v2: int32):
    v3: int32 = load v1
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_error(|e| matches!(e, OptimizeError::LocalReferenceEscapes { .. }));
    }

    /// Stack pointer passed through block parameter is tracked.
    #[test]
    fn test_detect_stack_pointer_through_block_param() {
        let input = r#"
function test(): ref<int32, raw> {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    jump b1(v0)
b1(v1: ref<int32, raw>):
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_error(|e| matches!(e, OptimizeError::ReturnReferenceToLocal { .. }));
    }

    /// Stack pointer through multiple jump hops is tracked.
    #[test]
    fn test_detect_stack_escape_through_multiple_jumps() {
        let input = r#"
function test(): ref<int32, raw> {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    jump b1(v0)
b1(v1: ref<int32, raw>):
    jump b2(v1)
b2(v2: ref<int32, raw>):
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_error(|e| matches!(e, OptimizeError::ReturnReferenceToLocal { .. }));
    }

    /// Stack pointer used locally through block params is valid.
    #[test]
    fn test_verify_stack_through_block_param_local_use() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    jump b1(v0)
b1(v2: ref<int32, raw>):
    v3: int32 = load v2
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_no_errors();
    }

    /// Loop with stack pointer used locally is valid.
    #[test]
    fn test_verify_loop_with_stack_local() {
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 0int32
    store v1, v2
    jump b1(v0)
b1(v3: int32):
    v4: int32 = 0int32
    v5: boolean = int.gt.s v3, v4
    branch v5, b2, b3
b2:
    v6: int32 = load v1
    v7: int32 = int.add v6, v3
    store v1, v7
    v8: int32 = 1int32
    v9: int32 = int.sub v3, v8
    jump b1(v9)
b3:
    v10: int32 = load v1
    return v10
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        test.assert_no_errors();
    }

    /// Diamond control flow: stack on one branch, heap on other = maybe stack at merge.
    #[test]
    fn test_detect_maybe_stack_escape_diamond() {
        let input = r#"
function test(v0: boolean): ref<int32, raw> {
b0(v0: boolean):
    branch v0, b1, b2
b1:
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    jump b3(v1)
b2:
    v2: ref<int32, raw> = raw.alloc int32
    jump b3(v2)
b3(v3: ref<int32, raw>):
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        // v3 might be stack (from block1) so returning it is an error
        test.assert_error(|e| matches!(e, OptimizeError::ReturnReferenceToLocal { .. }));
    }

    /// Diamond control flow: stack pointers on both branches used locally is valid.
    #[test]
    fn test_verify_stack_both_branches_local_use() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: ref<int32, raw, space(stack)> = stack.alloc int32
    v3: int32 = 1int32
    v4: int32 = 2int32
    store v1, v3
    store v2, v4
    branch v0, b1, b2
b1:
    jump b3(v1)
b2:
    jump b3(v2)
b3(v5: ref<int32, raw>):
    v6: int32 = load v5
    return v6
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);
        // loading and returning the *value* is fine, only returning the pointer is bad
        test.assert_no_errors();
    }

    /// Stack pointer escapes through identity function call.
    ///
    /// When a function returns a borrowed reference that borrows from an argument,
    /// and that argument is a stack pointer, the return value is also a stack pointer.
    #[test]
    fn test_detect_stack_escape_through_call() {
        let input = r#"
function identity(v0: ref<int32, borrowed>): ref<int32, borrowed> {
b0(v0: ref<int32, borrowed>):
    return v0
}
function test(): ref<int32, borrowed> {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: ref<int32, borrowed> = call identity(v0): (ref<int32, borrowed>) -> ref<int32, borrowed>
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StackCheck);

        // v2 = call identity(v0) where v0 is stack pointer
        // @identity returns borrowed ref from param 0, so v2 is stack pointer
        // returning v2 is a stack escape
        test.assert_error(|e| matches!(e, OptimizeError::ReturnReferenceToLocal { .. }));
    }

    /// Static lifetime prevents stack pointer propagation through call.
    ///
    /// When a function has static lifetime, its return doesn't borrow from
    /// arguments, so stack pointer status doesn't propagate.
    #[test]
    fn test_static_lifetime_no_stack_propagation() {
        let input = r#"
function getStatic(v0: ref<int32, borrowed>): ref<int32, borrowed> {
b0(v0: ref<int32, borrowed>):
    return v0
}
function test(): ref<int32, borrowed> {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: ref<int32, borrowed> = call getStatic(v0): (ref<int32, borrowed>) -> ref<int32, borrowed>
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.set_function_lifetime("getStatic", mir::BorrowRegion::Static);
        test.run_pass(&StackCheck);

        // with static lifetime, v2 doesn't inherit stack pointer status from v0
        // so returning v2 is allowed (from stack-check's perspective)
        // (this may still be incorrect at runtime, but that's a different issue)
        test.assert_no_errors();
    }

    /// Stack pointer propagates through call with explicit param lifetime.
    ///
    /// When a function has explicit lifetime annotation specifying param 0,
    /// stack pointer status from arg 0 propagates to the return value.
    #[test]
    fn test_explicit_param_lifetime_stack_propagation() {
        let input = r#"
function pick(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>): ref<int32, borrowed> {
b0(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>):
    return v0
}
function test(): ref<int32, borrowed> {
b0:
    v0: ref<int32, managed> = new int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 42int32
    store v0, v2
    store v1, v2
    v3: ref<int32, borrowed> = call pick(v0, v1): (ref<int32, borrowed>, ref<int32, borrowed>) -> ref<int32, borrowed>
    return v3
}"#;

        let mut test = TestProgram::new(input);

        // explicit lifetime: return borrows from param 0 only
        test.set_function_lifetime("pick", mir::BorrowRegion::param(0));
        test.run_pass(&StackCheck);

        // v3 borrows from v0 (param 0) which is managed, not stack
        // v1 (param 1) is stack but not borrowed from, so v3 is not stack
        test.assert_no_errors();
    }

    /// Stack pointer propagates through call when borrowing from stack arg.
    ///
    /// When explicit lifetime borrows from a param that is a stack pointer,
    /// the return is also a stack pointer.
    #[test]
    fn test_explicit_second_param_lifetime_stack_propagation() {
        let input = r#"
function pick(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>): ref<int32, borrowed> {
b0(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>):
    return v1
}
function test(): ref<int32, borrowed> {
b0:
    v0: ref<int32, managed> = new int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 42int32
    store v0, v2
    store v1, v2
    v3: ref<int32, borrowed> = call pick(v0, v1): (ref<int32, borrowed>, ref<int32, borrowed>) -> ref<int32, borrowed>
    return v3
}"#;

        let mut test = TestProgram::new(input);

        // explicit lifetime: return borrows from param 1 only
        test.set_function_lifetime("pick", mir::BorrowRegion::param(1));
        test.run_pass(&StackCheck);

        // v3 borrows from v1 (param 1) which is stack
        // returning v3 is a stack escape
        test.assert_error(|e| matches!(e, OptimizeError::ReturnReferenceToLocal { .. }));
    }
}
