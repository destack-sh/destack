use std::collections::HashSet;

use destack_mir as mir;
use mir::Instruction;

/// Check if an instruction has side effects and cannot be removed even if unused.
///
/// Instructions with side effects must be preserved regardless of whether their
/// result is used. This includes stores, calls, allocations, and drops.
pub fn instruction_has_side_effects(instruction: &Instruction) -> bool {
    match instruction {
        // pure computations, no side effects
        Instruction::Const { .. }
        | Instruction::Binary { .. }
        | Instruction::Unary { .. }
        | Instruction::Cast { .. }
        | Instruction::FieldGet { .. }
        | Instruction::FieldAddr { .. }
        | Instruction::ElementGet { .. }
        | Instruction::ElementAddr { .. }
        | Instruction::GlobalConst { .. }
        | Instruction::GlobalAddr { .. } => false,

        // memory reads are pure (assuming no volatile)
        Instruction::LocalGet { .. } | Instruction::Load { .. } => false,

        // memory writes have side effects
        Instruction::LocalSet { .. } | Instruction::Store { .. } => true,

        // aggregate updates create new values, but FieldSet/ElementSet don't have
        // side effects if the result is unused (they produce new values, not mutate)
        Instruction::FieldSet { .. } | Instruction::ElementSet { .. } => false,

        // drops have side effects (run destructors)
        Instruction::Drop { .. } => true,

        // calls may have side effects
        Instruction::Call { .. } | Instruction::CallIndirect { .. } => true,

        // allocations have side effects (memory allocation)
        Instruction::ManagedAlloc { .. }
        | Instruction::ManagedAllocArray { .. }
        | Instruction::RawAlloc { .. }
        | Instruction::StackAlloc { .. } => true,

        // deallocation has side effects
        Instruction::RawFree { .. } => true,

        // intrinsics may have side effects (conservative)
        Instruction::Intrinsic { .. } => true,
    }
}

/// Collect all values that are used by instructions or terminators in a function.
///
/// This is useful for dead code elimination and other analyses that need to know
/// which values are live.
pub fn instruction_collect_used_values(
    function: &mir::Function,
    tree: &mir::NodeTree,
) -> HashSet<mir::Value> {
    let mut used = HashSet::new();

    // add function parameters as implicitly used (they're inputs)
    for param in &function.parameters {
        used.insert(param.value);
    }

    for &block_id in &function.blocks {
        let block = tree.get(block_id);

        // collect uses from instructions
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);

            // add inline uses
            for value in instruction.uses() {
                used.insert(value);
            }

            // add externalized argument uses (for Call, CallIndirect, Intrinsic)
            if let Some(args_slice) = instruction.argument_slice() {
                for &arg in tree.get_arguments(args_slice) {
                    used.insert(arg);
                }
            }
        }

        // collect uses from terminator
        for value in block.terminator.uses() {
            used.insert(value);
        }
    }

    used
}
