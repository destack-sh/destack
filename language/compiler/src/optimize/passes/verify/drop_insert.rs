use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;
use mir::{Instruction, Value};

use crate::optimize::common::ValueTypeMap;
use crate::optimize::{
    AnalysisPreservation, FunctionPass, LivenessAnalysis, OwnershipAnalysis, OwnershipMap,
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
    /// - Function and block parameters with owning handle types
    /// - Explicit ownership values (owned refs and aggregates containing them)
    /// - Instructions with explicit types (struct, tuple, array, cast)
    /// - Local variable loads (local.get)
    ///
    /// Modifies MIR and invalidates all analyses.
    #[pass(id = "drop-insert")]
    pub DropInsert,
    "Insert drop instructions at last-use points"
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
    let value_types = ValueTypeMap::new(function, tree);
    let droppable = find_droppable_values_with_ownership(function, tree, ownership, &value_types);
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
            if let Some(dest) = instruction.destination().and_then(|value| value.value())
                && droppable.contains(&dest)
                && !liveness.is_live_out(block_id, dest)
            {
                // value dies in this block, find where
                let death_idx =
                    find_death_point(block_id, idx, dest, &instructions, tree, liveness);

                if let Some(after_idx) = death_idx {
                    if last_use_moves_value(dest, after_idx, &instructions, tree, ownership) {
                        continue;
                    }
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
                    if last_use_moves_value(value, after_idx, &instructions, tree, ownership) {
                        continue;
                    }
                    drops_to_insert.push(DropInsertionPoint::AfterInstruction {
                        block: block_id,
                        instruction_index: after_idx,
                        value,
                    });
                }
            }
        }

        // check for values live-out that need drops at block exit (e.g., before return)
        let terminator = tree.get(block.terminator);
        if let mir::Terminator::Return { value: ret_val } = terminator {
            // check live-in values
            for &value in liveness.live_in(block_id) {
                // check if value is used in return; if not, drop before return
                if droppable.contains(&value) && ret_val != &Some(value.into()) {
                    drops_to_insert.push(DropInsertionPoint::BeforeTerminator {
                        block: block_id,
                        value,
                    });
                }
            }

            // also check block parameters (not live-in but need drops)
            for param in &block.parameters {
                let Some(param_value) = param.value.value() else {
                    continue;
                };

                if droppable.contains(&param_value)
                    && ret_val != &Some(param_value.into())
                    && !liveness.is_live_out(block_id, param_value)
                {
                    // parameter dies in this block, find where or drop before return
                    let death_idx =
                        find_death_point(block_id, 0, param_value, &instructions, tree, liveness);

                    if let Some(after_idx) = death_idx {
                        if last_use_moves_value(
                            param_value,
                            after_idx,
                            &instructions,
                            tree,
                            ownership,
                        ) {
                            continue;
                        }
                        drops_to_insert.push(DropInsertionPoint::AfterInstruction {
                            block: block_id,
                            instruction_index: after_idx,
                            value: param_value,
                        });
                    } else {
                        // used only in terminator or no uses: drop before terminator
                        drops_to_insert.push(DropInsertionPoint::BeforeTerminator {
                            block: block_id,
                            value: param_value,
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

/// Find all values in the function that need Drop instructions using ownership analysis.
///
/// Uses the value type table for accurate value type tracking, which handles
/// field.set/element.set without inference.
fn find_droppable_values_with_ownership(
    function: &mir::Function,
    tree: &mir::NodeTree,
    ownership: &OwnershipAnalysis,
    value_types: &ValueTypeMap,
) -> HashSet<Value> {
    let mut droppable = HashSet::new();

    // check function parameters
    for param in &function.parameters {
        // drop only values that require explicit cleanup
        let Some(param_value) = param.value.value() else {
            continue;
        };

        if value_needs_drop(param_value, ownership, value_types, tree) {
            droppable.insert(param_value);
        }
    }

    // check all instructions
    for &block_id in &function.blocks {
        let block = tree.get(block_id);

        // block parameters
        for param in &block.parameters {
            let Some(param_value) = param.value.value() else {
                continue;
            };

            if value_needs_drop(param_value, ownership, value_types, tree) {
                droppable.insert(param_value);
            }
        }

        // instructions
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);

            if let Some(dest) = instruction.destination().and_then(|value| value.value())
                && value_needs_drop(dest, ownership, value_types, tree)
            {
                droppable.insert(dest);
            }
        }
    }

    droppable
}

/// Return true when a value requires explicit drop insertion.
fn value_needs_drop(
    value: Value,
    ownership: &OwnershipAnalysis,
    value_types: &ValueTypeMap,
    tree: &mir::NodeTree,
) -> bool {
    // managed allocations are GC owned
    if ownership.is_managed_allocated(value) {
        return false;
    }

    // resolve the value type from ownership
    let type_id = value_types.require_value_type(value);

    // drop rules are based on the value type
    matches!(
        tree.get(type_id),
        mir::Type::Reference {
            kind: mir::ReferenceKind::Owned,
            ..
        } | mir::Type::TensorReference {
            kind: mir::ReferenceKind::Owned,
            ..
        } | mir::Type::Struct {
            copyability: mir::Copyability::Linear,
            ..
        } | mir::Type::Newtype {
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
        if instruction.uses().contains(&value.into()) {
            last_use_idx = Some(idx);
        }

        // check externalized arguments
        if let Some(arg_slice) = instruction.argument_slice()
            && tree.get_arguments(arg_slice).contains(&value.into())
        {
            last_use_idx = Some(idx);
        }
    }

    // if value is used in terminator, we can't drop in this block
    let block = tree.get(block_id);
    let terminator = tree.get(block.terminator);
    if crate::optimize::terminator_uses(terminator, value) {
        return None;
    }

    last_use_idx
}

/// Return true when the last use is a move that transfers ownership.
fn last_use_moves_value(
    value: Value,
    instruction_index: usize,
    instructions: &[mir::LocalNodeId<Instruction>],
    tree: &mir::NodeTree,
    ownership: &OwnershipAnalysis,
) -> bool {
    let instruction_id = match instructions.get(instruction_index) {
        Some(id) => *id,
        None => return false,
    };
    let instruction = tree.get(instruction_id);

    let mut state = OwnershipMap::new();
    state.mark_owned(value);
    ownership.apply_instruction_effects(&mut state, instruction_id, instruction, tree);

    state.is_moved(value)
}

/// Emit one ownership-end marker for a value.
fn emit_drop_sequence(
    tree: &mut mir::NodeTree,
    value: Value,
    instructions: &mut Vec<mir::LocalNodeId<Instruction>>,
) {
    // insert one semantic drop and let later stages choose storage behavior
    let drop_instruction = Instruction::Drop {
        value: value.into(),
    };
    let drop_id = tree.insert(drop_instruction);
    instructions.push(drop_id);
}

/// Insert Drop instructions at the specified points.
fn insert_drops(
    _function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    drops: &[DropInsertionPoint],
    _ownership: &OwnershipAnalysis,
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
                    emit_drop_sequence(tree, value, &mut new_instructions);
                }
            }
        }

        // insert drops before terminator
        for value in drops_before_terminator {
            emit_drop_sequence(tree, value, &mut new_instructions);
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

    /// Function with no droppable values is unchanged.
    #[test]
    fn test_verify_no_drops_needed() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 42int32
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DropInsert);
        test.assert_no_errors();
        test.assert_unchanged(input);
    }

    /// Primitive types don't need drops.
    #[test]
    fn test_verify_primitives_no_drop() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 1int32
    v1: int32 = 2int32
    v2: int32 = int.add v0, v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DropInsert);
        test.assert_no_errors();
        test.assert_unchanged(input);
    }

    /// Boolean comparison doesn't need drops.
    #[test]
    fn test_verify_booleans_no_drop() {
        let input = r#"
function test(): boolean {
b0:
    v0: int32 = 1int32
    v1: int32 = 2int32
    v2: boolean = int.eq v0, v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DropInsert);
        test.assert_no_errors();
        test.assert_unchanged(input);
    }

    /// Float values don't need drops.
    #[test]
    fn test_verify_floats_no_drop() {
        let input = r#"
function test(): float64 {
b0:
    v0: float64 = 1float64
    v1: float64 = 2float64
    v2: float64 = float.add v0, v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DropInsert);
        test.assert_no_errors();
    }

    /// Void function is unchanged.
    #[test]
    fn test_verify_void_function() {
        let input = r#"
function test(): void {
b0:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DropInsert);
        test.assert_no_errors();
        test.assert_unchanged(input);
    }

    /// Stack allocation with primitive doesn't need drop.
    #[test]
    fn test_verify_stack_alloc_primitive() {
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
        test.run_pass(&DropInsert);
        test.assert_no_errors();
        test.assert_unchanged(input);
    }

    /// Managed allocations are not dropped explicitly.
    #[test]
    fn test_verify_managed_alloc_no_drop() {
        let input = r#"
type Node {
    int32;
}
function test(): int32 {
b0:
    v0: ref<Node, managed> = managed.alloc Node
    v1: int32 = 1int32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DropInsert);
        test.assert_no_errors();
        test.assert_unchanged(input);
    }

    /// Multiple arithmetic operations don't need drops.
    #[test]
    fn test_verify_arithmetic_chain() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 1int32
    v1: int32 = 2int32
    v2: int32 = 3int32
    v3: int32 = int.add v0, v1
    v4: int32 = int.mul v3, v2
    v5: int32 = int.sub v4, v0
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DropInsert);
        test.assert_no_errors();
        test.assert_unchanged(input);
    }

    /// Control flow with primitives doesn't need drops.
    #[test]
    fn test_verify_control_flow_primitives() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    branch v0, b1, b2
b1:
    v1: int32 = 1int32
    return v1
b2:
    v2: int32 = 2int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DropInsert);
        test.assert_no_errors();
        test.assert_unchanged(input);
    }

    /// Loop with primitives doesn't need drops.
    #[test]
    fn test_verify_loop_primitives() {
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    jump b1(v1)
b1(v2: int32):
    v3: boolean = int.lt.s v2, v0
    branch v3, b2, b3
b2:
    v4: int32 = 1int32
    v5: int32 = int.add v2, v4
    jump b1(v5)
b3:
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DropInsert);
        test.assert_no_errors();
        test.assert_unchanged(input);
    }

    /// Cast operations don't need drops.
    #[test]
    fn test_verify_cast_no_drop() {
        let input = r#"
function test(): int64 {
b0:
    v0: int32 = 42int32
    v1: int64 = cast.extend.s v0 -> int64
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DropInsert);
        test.assert_no_errors();
        test.assert_unchanged(input);
    }

    /// Load/store with primitives don't need drops.
    #[test]
    fn test_verify_load_store_primitives() {
        let input = r#"
function test(v0: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>):
    v1: int32 = load v0
    v2: int32 = 10int32
    v3: int32 = int.add v1, v2
    store v0, v3
    v4: int32 = load v0
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DropInsert);
        test.assert_no_errors();
        test.assert_unchanged(input);
    }

    /// Function calls with primitive args don't need drops.
    #[test]
    fn test_verify_call_primitives() {
        let input = r#"
extern function add(int32, int32): int32
function test(): int32 {
b0:
    v0: int32 = 1int32
    v1: int32 = 2int32
    v2: int32 = call add(v0, v1): (int32, int32) -> int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DropInsert);
        test.assert_no_errors();
    }

    /// Local variables with primitives don't need drops.
    #[test]
    fn test_verify_locals_primitives() {
        let input = r#"
function test(): int32 {
    local local0: int32, owned
b0:
    v0: int32 = 42int32
    local.set local0, v0
    v1: int32 = local.get local0
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DropInsert);
        test.assert_no_errors();
    }

    /// Unreachable terminator doesn't need special handling.
    #[test]
    fn test_verify_unreachable() {
        let input = r#"
function test(): int32 {
b0:
    unreachable
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DropInsert);
        test.assert_no_errors();
        test.assert_unchanged(input);
    }

    /// Switch with primitives doesn't need drops.
    #[test]
    fn test_verify_switch_primitives() {
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    switch v0, b3, 0 => b1, 1 => b2
b1:
    v1: int32 = 10int32
    return v1
b2:
    v2: int32 = 20int32
    return v2
b3:
    v3: int32 = 30int32
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DropInsert);
        test.assert_no_errors();
        test.assert_unchanged(input);
    }

    /// Managed allocation is not dropped explicitly.
    #[test]
    fn test_managed_alloc_no_drop() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, managed> = managed.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: int32 = load v0
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DropInsert);
        test.assert_no_errors();
        test.assert_unchanged(input);
    }

    /// Managed allocation returned is not dropped.
    #[test]
    fn test_managed_alloc_returned_no_drop() {
        let input = r#"
function test(): ref<int32, managed> {
b0:
    v0: ref<int32, managed> = managed.alloc int32
    v1: int32 = 42int32
    store v0, v1
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DropInsert);
        test.assert_no_errors();
        test.assert_unchanged(input);
    }

    /// Owned parameter gets drop inserted before return.
    #[test]
    fn test_insert_drop_for_owned_param() {
        let input = r#"
function test(v0: ref<int32, owned>): int32 {
b0(v0: ref<int32, owned>):
    v1: int32 = load v0
    return v1
}"#;

        let expected = r#"
function test(v0: ref<int32, owned>): int32 {
b0(v0: ref<int32, owned>):
    v1: int32 = load v0
    drop v0
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DropInsert);
        test.assert_no_errors();
        test.assert_output(expected);
    }

    /// Move into call should not get a drop inserted.
    #[test]
    fn test_no_drop_after_move_into_call() {
        let input = r#"
extern function consume(ref<int32, owned>): void
function test(v0: ref<int32, owned>): void {
b0(v0: ref<int32, owned>):
    call consume(v0): (ref<int32, owned>) -> void
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DropInsert);
        test.assert_no_errors();
        test.assert_unchanged(input);
    }

    /// Move into store should not get a drop inserted.
    #[test]
    fn test_no_drop_after_move_into_store() {
        let input = r#"
function test(v0: ref<ref<int32, owned>, raw>, v1: ref<int32, owned>): void {
b0(v0: ref<ref<int32, owned>, raw>, v1: ref<int32, owned>):
    store v0, v1
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DropInsert);
        test.assert_no_errors();
        test.assert_unchanged(input);
    }

    /// Raw allocation does not get automatic drop.
    #[test]
    fn test_raw_alloc_no_automatic_drop() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw> = raw.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: int32 = load v0
    raw.free v0
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DropInsert);
        test.assert_no_errors();
        test.assert_unchanged(input);
    }
}
