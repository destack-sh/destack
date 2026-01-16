use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;
use mir::{Instruction, Value};

use crate::optimize::{
    AnalysisPreservation, FunctionPass, LivenessAnalysis, OwnershipAnalysis,
    PipelineContext,
};

declare_pass! {
    /// Insert Drop instructions at last-use points.
    ///
    /// Implements non-lexical lifetimes (NLL) by dropping owned values as soon as
    /// they're no longer needed, rather than at lexical scope boundaries.
    ///
    /// Algorithm:
    /// 1. Identify droppable values (owned/managed refs, aggregates containing them)
    /// 2. Use liveness analysis to find where each value becomes dead
    /// 3. Insert `drop` after the last use, or before return for still-live values
    ///
    /// Identifies droppable values from:
    /// - Function and block parameters with owned reference types
    /// - Explicit ownership values (owned refs and aggregates containing them)
    /// - Instructions with explicit types (struct, tuple, array, cast)
    /// - Local variable loads (local.get)
    ///
    /// Modifies MIR and invalidates all analyses.
    #[pass(id = "drop-insert")]
    pub DropInsert,
    "Insert drop calls at last-use points"
}

/// Run drop insertion on a function.
///
/// Returns true if any changes were made.
fn run_drop_insert(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    liveness: &LivenessAnalysis,
    ownership: &OwnershipAnalysis,
) -> bool {
    // find values that need drops using ownership analysis
    let droppable = find_droppable_values_with_ownership(function, tree, ownership);
    if droppable.is_empty() {
        return false;
    }

    // collect drop insertion points
    let mut drops_to_insert: Vec<DropInsertionPoint> = Vec::new();
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let instructions = block.instructions.clone();

        // check each instruction for values that die after it
        for (idx, &instruction_id) in instructions.iter().enumerate() {
            let instruction = tree.get(instruction_id);

            // check if any droppable value is defined here and dies before block exit
            if let Some(dest) = instruction.destination()
                && droppable.contains(&dest)
                && !liveness.is_live_out(block_id, dest)
            {
                // value dies in this block, find where
                let death_idx =
                    find_death_point(block_id, idx, dest, &instructions, tree, liveness);

                if let Some(after_idx) = death_idx {
                    drops_to_insert.push(DropInsertionPoint::AfterInstruction {
                        block: block_id,
                        instruction_index: after_idx,
                        value: dest,
                    });
                }
            }
        }

        // check for values that are live-in but not live-out (die in block)
        for &value in liveness.live_in(block_id) {
            if droppable.contains(&value) && !liveness.is_live_out(block_id, value) {
                // value dies in this block
                let death_idx = find_death_point(block_id, 0, value, &instructions, tree, liveness);

                if let Some(after_idx) = death_idx {
                    drops_to_insert.push(DropInsertionPoint::AfterInstruction {
                        block: block_id,
                        instruction_index: after_idx,
                        value,
                    });
                }
            }
        }

        // check for values live-out that need drops at block exit (e.g., before return)
        if let mir::Terminator::Return { value: ret_val } = &block.terminator {
            // check live-in values
            for &value in liveness.live_in(block_id) {
                // check if value is used in return; if not, drop before return
                if droppable.contains(&value) && ret_val != &Some(value) {
                    drops_to_insert.push(DropInsertionPoint::BeforeTerminator {
                        block: block_id,
                        value,
                    });
                }
            }

            // also check block parameters (not live-in but need drops)
            for param in &block.parameters {
                if droppable.contains(&param.value)
                    && ret_val != &Some(param.value)
                    && !liveness.is_live_out(block_id, param.value)
                {
                    // parameter dies in this block, find where or drop before return
                    let death_idx =
                        find_death_point(block_id, 0, param.value, &instructions, tree, liveness);

                    if let Some(after_idx) = death_idx {
                        drops_to_insert.push(DropInsertionPoint::AfterInstruction {
                            block: block_id,
                            instruction_index: after_idx,
                            value: param.value,
                        });
                    } else {
                        // used only in terminator or no uses: drop before terminator
                        drops_to_insert.push(DropInsertionPoint::BeforeTerminator {
                            block: block_id,
                            value: param.value,
                        });
                    }
                }
            }
        }
    }

    // deduplicate and sort drops
    drops_to_insert.sort();
    drops_to_insert.dedup();
    if drops_to_insert.is_empty() {
        return false;
    }

    // insert drops (process in reverse to maintain indices)
    insert_drops(function, tree, &drops_to_insert, ownership);

    true
}

impl FunctionPass for DropInsert {
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // get analyses
        let (liveness, ownership) = {
            let analyses = ctx.function_analyses(function, tree);
            (
                analyses.get::<LivenessAnalysis>().clone(),
                analyses.get::<OwnershipAnalysis>().clone(),
            )
        };

        let changed = run_drop_insert(function, tree, &liveness, &ownership);

        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    fn name(&self) -> &'static str {
        "DropInsert"
    }

    fn id(&self) -> &'static str {
        "drop-insert"
    }
}

/// Where to insert a Drop instruction.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum DropInsertionPoint {
    /// Insert after an instruction (before the next instruction).
    AfterInstruction {
        block: mir::LocalNodeId<mir::Block>,
        instruction_index: usize,
        value: Value,
    },
    /// Insert before the block's terminator.
    BeforeTerminator {
        block: mir::LocalNodeId<mir::Block>,
        value: Value,
    },
}

/// Find all values in the function that need Drop calls using ownership analysis.
///
/// Uses OwnershipAnalysis for accurate value type tracking, which handles cases like
/// field.set/element.set where the type needs to be inferred.
fn find_droppable_values_with_ownership(
    function: &mir::Function,
    tree: &mir::NodeTree,
    ownership: &OwnershipAnalysis,
) -> HashSet<Value> {
    let mut droppable = HashSet::new();

    // check function parameters
    for param in &function.parameters {
        // drop only values that require explicit cleanup
        if value_needs_drop(param.value, ownership, tree) {
            droppable.insert(param.value);
        }
    }

    // check all instructions
    for &block_id in &function.blocks {
        let block = tree.get(block_id);

        // block parameters
        for param in &block.parameters {
            if value_needs_drop(param.value, ownership, tree) {
                droppable.insert(param.value);
            }
        }

        // instructions
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);

            if let Some(dest) = instruction.destination()
                && value_needs_drop(dest, ownership, tree)
            {
                droppable.insert(dest);
            }
        }
    }

    droppable
}

/// Return true when a value requires explicit drop insertion.
fn value_needs_drop(value: Value, ownership: &OwnershipAnalysis, tree: &mir::NodeTree) -> bool {
    // managed allocations are GC owned
    if ownership.is_managed_allocated(value) {
        return false;
    }

    // resolve the value type from ownership
    let Some(type_id) = ownership.value_type(value) else {
        return false;
    };

    // drop rules are based on the value type
    matches!(
        tree.get(type_id),
        mir::Type::Reference {
            kind: mir::ReferenceKind::Owned,
            ..
        } | mir::Type::Struct {
            copyability: mir::Copyability::Linear,
            ..
        } | mir::Type::Tuple {
            copyability: mir::Copyability::Linear,
            ..
        } | mir::Type::Array {
            copyability: mir::Copyability::Linear,
            ..
        }
    )
}

/// Find the instruction index after which a value dies (last use).
fn find_death_point(
    block_id: mir::LocalNodeId<mir::Block>,
    start_idx: usize,
    value: Value,
    instructions: &[mir::LocalNodeId<Instruction>],
    tree: &mir::NodeTree,
    _liveness: &LivenessAnalysis,
) -> Option<usize> {
    // scan forward to find the last use
    let mut last_use_idx = None;

    for (idx, &instruction_id) in instructions.iter().enumerate().skip(start_idx) {
        let instruction = tree.get(instruction_id);

        // check if this instruction uses the value
        if instruction.uses().contains(&value) {
            last_use_idx = Some(idx);
        }

        // check externalized arguments
        if let Some(arg_slice) = instruction.argument_slice()
            && tree.get_arguments(arg_slice).contains(&value)
        {
            last_use_idx = Some(idx);
        }
    }

    // if value is used in terminator, we can't drop in this block
    let block = tree.get(block_id);
    if crate::optimize::terminator_uses(&block.terminator, value) {
        return None;
    }

    last_use_idx
}

/// Emit the drop sequence for a value.
///
/// If the value's type has a drop function in the metadata table, emits a call
/// to the drop function first. Then emits the drop instruction.
fn emit_drop_sequence(
    tree: &mut mir::NodeTree,
    ownership: &OwnershipAnalysis,
    value: Value,
    instructions: &mut Vec<mir::LocalNodeId<Instruction>>,
) {
    // check if value's type has a drop function
    if let Some(type_id) = ownership.value_type(value)
        && let Some(&drop_fn) = tree.type_table.drop_function_by_type_id.get(&type_id)
    {
        let arguments = tree.add_arguments(&[value]);
        let call = Instruction::Call {
            destination: None,
            function: drop_fn,
            arguments,
        };
        let call_id = tree.insert(call);
        instructions.push(call_id);
    }

    // emit the drop instruction based on allocation kind
    let drop_instruction = if ownership.is_stack_allocated(value) {
        Instruction::StackDrop { value }
    } else {
        Instruction::RawDrop { value }
    };
    let drop_id = tree.insert(drop_instruction);
    instructions.push(drop_id);
}

/// Insert Drop instructions at the specified points.
///
/// For each drop, if the value's type has a drop function registered in the
/// metadata table, emits a call to the drop function before the drop instruction.
fn insert_drops(
    _function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    drops: &[DropInsertionPoint],
    ownership: &OwnershipAnalysis,
) {
    // group by block for efficient insertion
    let mut by_block: HashMap<mir::LocalNodeId<mir::Block>, Vec<&DropInsertionPoint>> =
        HashMap::new();

    for drop in drops {
        let block_id = match drop {
            DropInsertionPoint::AfterInstruction { block, .. } => *block,
            DropInsertionPoint::BeforeTerminator { block, .. } => *block,
        };
        by_block.entry(block_id).or_default().push(drop);
    }

    // process each block
    for (block_id, block_drops) in by_block {
        let block = tree.get(block_id).clone();
        let mut new_instructions = Vec::with_capacity(block.instructions.len() + block_drops.len());

        // track which indices need drops after them
        let mut drops_after: HashMap<usize, Vec<Value>> = HashMap::new();
        let mut drops_before_terminator: Vec<Value> = Vec::new();

        for drop in block_drops {
            match drop {
                DropInsertionPoint::AfterInstruction {
                    instruction_index,
                    value,
                    ..
                } => {
                    drops_after
                        .entry(*instruction_index)
                        .or_default()
                        .push(*value);
                }
                DropInsertionPoint::BeforeTerminator { value, .. } => {
                    drops_before_terminator.push(*value);
                }
            }
        }

        // rebuild instruction list with drops inserted
        for (idx, &instruction_id) in block.instructions.iter().enumerate() {
            new_instructions.push(instruction_id);

            // insert drops after this instruction
            if let Some(values) = drops_after.get(&idx) {
                for &value in values {
                    emit_drop_sequence(tree, ownership, value, &mut new_instructions);
                }
            }
        }

        // insert drops before terminator
        for value in drops_before_terminator {
            emit_drop_sequence(tree, ownership, value, &mut new_instructions);
        }

        // update block
        let block_mut = tree.get_mut(block_id);
        block_mut.instructions = new_instructions;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Register a drop function for all types matching a predicate.
    ///
    /// The MIR parser creates separate type nodes for each type occurrence,
    /// so this helper registers the drop function for all matching types.
    fn register_drop_function_for<F>(
        program: &mut TestProgram,
        drop_fn: mir::LocalNodeId<mir::Function>,
        predicate: F,
    ) where
        F: Fn(&mir::Type) -> bool,
    {
        let type_ids: Vec<_> = program
            .tree
            .iter_nodes::<mir::Type>()
            .filter_map(|(id, ty)| if predicate(ty) { Some(id) } else { None })
            .collect();
        for type_id in type_ids {
            program
                .tree
                .type_table
                .drop_function_by_type_id
                .insert(type_id, drop_fn);
        }
    }

    /// Function with no droppable values is unchanged.
    #[test]
    fn test_verify_no_drops_needed() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DropInsert);
        program.assert_no_errors();
        program.assert_unchanged(input);
    }

    /// Primitive types don't need drops.
    #[test]
    fn test_verify_primitives_no_drop() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    v2 = iadd v0, v1
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DropInsert);
        program.assert_no_errors();
        program.assert_unchanged(input);
    }

    /// Boolean comparison doesn't need drops.
    #[test]
    fn test_verify_booleans_no_drop() {
        let input = r#"function @test() -> bool {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    v2 = icmp_eq v0, v1
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DropInsert);
        program.assert_no_errors();
        program.assert_unchanged(input);
    }

    /// Float values don't need drops.
    #[test]
    fn test_verify_floats_no_drop() {
        let input = r#"function @test() -> f64 {
block0:
    v0 = iconst 1.0f64
    v1 = iconst 2.0f64
    v2 = fadd v0, v1
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DropInsert);
        program.assert_no_errors();
    }

    /// Void function is unchanged.
    #[test]
    fn test_verify_void_function() {
        let input = r#"function @test() -> void {
block0:
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DropInsert);
        program.assert_no_errors();
        program.assert_unchanged(input);
    }

    /// Stack allocation with primitive doesn't need drop.
    #[test]
    fn test_verify_stack_alloc_primitive() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = iconst 42i32
    store v0, v1
    v2 = load v0 -> i32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DropInsert);
        program.assert_no_errors();
        program.assert_unchanged(input);
    }

    /// Managed allocations are not dropped explicitly.
    #[test]
    fn test_verify_managed_alloc_no_drop() {
        let input = r#"type @Node = { i32 }
function @test() -> i32 {
block0:
    v0 = managed.alloc @Node -> ref<managed @Node>
    v1 = iconst 1i32
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DropInsert);
        program.assert_no_errors();
        program.assert_unchanged(input);
    }

    /// Multiple arithmetic operations don't need drops.
    #[test]
    fn test_verify_arithmetic_chain() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    v2 = iconst 3i32
    v3 = iadd v0, v1
    v4 = imul v3, v2
    v5 = isub v4, v0
    return v5
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DropInsert);
        program.assert_no_errors();
        program.assert_unchanged(input);
    }

    /// Control flow with primitives doesn't need drops.
    #[test]
    fn test_verify_control_flow_primitives() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    branch v0, block1, block2
block1:
    v1 = iconst 1i32
    return v1
block2:
    v2 = iconst 2i32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DropInsert);
        program.assert_no_errors();
        program.assert_unchanged(input);
    }

    /// Loop with primitives doesn't need drops.
    #[test]
    fn test_verify_loop_primitives() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    jump block1(v1)
block1(v2: i32):
    v3 = icmp_slt v2, v0
    branch v3, block2, block3
block2:
    v4 = iconst 1i32
    v5 = iadd v2, v4
    jump block1(v5)
block3:
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DropInsert);
        program.assert_no_errors();
        program.assert_unchanged(input);
    }

    /// Cast operations don't need drops.
    #[test]
    fn test_verify_cast_no_drop() {
        let input = r#"function @test() -> i64 {
block0:
    v0 = iconst 42i32
    v1 = sextend v0 -> i64
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DropInsert);
        program.assert_no_errors();
        program.assert_unchanged(input);
    }

    /// Load/store with primitives don't need drops.
    #[test]
    fn test_verify_load_store_primitives() {
        let input = r#"function @test(v0: ref<raw i32>) -> i32 {
block0(v0: ref<raw i32>):
    v1 = load v0 -> i32
    v2 = iconst 10i32
    v3 = iadd v1, v2
    store v0, v3
    v4 = load v0 -> i32
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DropInsert);
        program.assert_no_errors();
        program.assert_unchanged(input);
    }

    /// Function calls with primitive args don't need drops.
    #[test]
    fn test_verify_call_primitives() {
        let input = r#"extern function @add(i32, i32) -> i32

function @test() -> i32 {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    v2 = call @add(v0, v1)
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DropInsert);
        program.assert_no_errors();
    }

    /// Local variables with primitives don't need drops.
    #[test]
    fn test_verify_locals_primitives() {
        let input = r#"function @test() -> i32 {
local0: i32
block0:
    v0 = iconst 42i32
    local.set local0, v0
    v1 = local.get local0
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DropInsert);
        program.assert_no_errors();
    }

    /// Unreachable terminator doesn't need special handling.
    #[test]
    fn test_verify_unreachable() {
        let input = r#"function @test() -> i32 {
block0:
    unreachable
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DropInsert);
        program.assert_no_errors();
        program.assert_unchanged(input);
    }

    /// Switch with primitives doesn't need drops.
    #[test]
    fn test_verify_switch_primitives() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    switch v0, block3, 0 => block1, 1 => block2
block1:
    v1 = iconst 10i32
    return v1
block2:
    v2 = iconst 20i32
    return v2
block3:
    v3 = iconst 30i32
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DropInsert);
        program.assert_no_errors();
        program.assert_unchanged(input);
    }

    /// Managed allocation is not dropped explicitly.
    #[test]
    fn test_managed_alloc_no_drop() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = managed.alloc i32 -> ref<managed i32>
    v1 = iconst 42i32
    store v0, v1
    v2 = load v0 -> i32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DropInsert);
        program.assert_no_errors();
        program.assert_unchanged(input);
    }

    /// Managed allocation returned is not dropped.
    #[test]
    fn test_managed_alloc_returned_no_drop() {
        let input = r#"function @test() -> ref<managed i32> {
block0:
    v0 = managed.alloc i32 -> ref<managed i32>
    v1 = iconst 42i32
    store v0, v1
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DropInsert);
        program.assert_no_errors();
        program.assert_unchanged(input);
    }

    /// Owned parameter gets drop inserted before return.
    #[test]
    fn test_insert_drop_for_owned_param() {
        let input = r#"function @test(v0: ref<owned i32>) -> i32 {
block0(v0: ref<owned i32>):
    v1 = load v0 -> i32
    return v1
}"#;

        let expected = r#"function @test(v0: ref<owned i32>) -> i32 {
block0(v0: ref<owned i32>):
    v1 = load v0 -> i32
    raw.drop v0
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DropInsert);
        program.assert_no_errors();
        program.assert_output(expected);
    }

    /// Raw allocation does not get automatic drop.
    #[test]
    fn test_raw_alloc_no_automatic_drop() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = raw.alloc i32 -> ref<raw i32>
    v1 = iconst 42i32
    store v0, v1
    v2 = load v0 -> i32
    raw.free v0
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DropInsert);
        program.assert_no_errors();
        program.assert_unchanged(input);
    }

    /// Type with drop function gets call emitted before raw.drop.
    #[test]
    fn test_drop_function_called_before_raw_drop() {
        let input = r#"function @my_drop(v0: ref<raw i32>) -> void {
block0(v0: ref<raw i32>):
    return
}

function @test(v0: ref<owned i32>) -> i32 {
block0(v0: ref<owned i32>):
    v1 = load v0 -> i32
    return v1
}"#;

        let mut program = TestProgram::new(input);
        let drop_fn = program
            .tree
            .iter_nodes::<mir::Function>()
            .find(|(_, f)| program.get_string(f.name) == "my_drop")
            .map(|(id, _)| id)
            .expect("drop function not found");

        register_drop_function_for(&mut program, drop_fn, |ty| {
            matches!(
                ty,
                mir::Type::Reference {
                    kind: mir::ReferenceKind::Owned,
                    ..
                }
            )
        });

        program.run_pass(&DropInsert);
        program.assert_no_errors();

        let expected = r#"function @my_drop(v0: ref<raw i32>) -> void {
block0(v0: ref<raw i32>):
    return
}
function @test(v0: ref<owned i32>) -> i32 {
block0(v0: ref<owned i32>):
    v1 = load v0 -> i32
    call @my_drop(v0)
    raw.drop v0
    return v1
}"#;
        program.assert_output(expected);
    }
}
