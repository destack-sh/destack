use std::collections::{HashMap, HashSet};

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

/// Substitute values in an instruction according to the given map.
///
/// Creates a new instruction with value references replaced according to the substitution map.
/// Values not in the map are left unchanged.
pub fn instruction_substitute_uses(
    instruction: &mir::Instruction,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> mir::Instruction {
    if substitutions.is_empty() {
        return instruction.clone();
    }

    let substitute = |v: &mir::Value| -> mir::Value { *substitutions.get(v).unwrap_or(v) };

    match instruction {
        mir::Instruction::Binary {
            destination,
            operator,
            left,
            right,
        } => mir::Instruction::Binary {
            destination: *destination,
            operator: *operator,
            left: substitute(left),
            right: substitute(right),
        },
        mir::Instruction::Unary {
            destination,
            operator,
            argument,
        } => mir::Instruction::Unary {
            destination: *destination,
            operator: *operator,
            argument: substitute(argument),
        },
        mir::Instruction::Cast {
            destination,
            operator,
            argument,
            to_type,
        } => mir::Instruction::Cast {
            destination: *destination,
            operator: *operator,
            argument: substitute(argument),
            to_type: *to_type,
        },
        mir::Instruction::Load {
            destination,
            pointer,
        } => mir::Instruction::Load {
            destination: *destination,
            pointer: substitute(pointer),
        },
        mir::Instruction::Store { pointer, value } => mir::Instruction::Store {
            pointer: substitute(pointer),
            value: substitute(value),
        },
        mir::Instruction::Drop { value } => mir::Instruction::Drop {
            value: substitute(value),
        },
        mir::Instruction::FieldGet {
            destination,
            aggregate,
            index,
        } => mir::Instruction::FieldGet {
            destination: *destination,
            aggregate: substitute(aggregate),
            index: *index,
        },
        mir::Instruction::FieldAddr {
            destination,
            aggregate,
            index,
        } => mir::Instruction::FieldAddr {
            destination: *destination,
            aggregate: substitute(aggregate),
            index: *index,
        },
        mir::Instruction::FieldSet {
            destination,
            aggregate,
            index,
            value,
        } => mir::Instruction::FieldSet {
            destination: *destination,
            aggregate: substitute(aggregate),
            index: *index,
            value: substitute(value),
        },
        mir::Instruction::ElementGet {
            destination,
            array,
            index,
        } => mir::Instruction::ElementGet {
            destination: *destination,
            array: substitute(array),
            index: substitute(index),
        },
        mir::Instruction::ElementAddr {
            destination,
            array,
            index,
        } => mir::Instruction::ElementAddr {
            destination: *destination,
            array: substitute(array),
            index: substitute(index),
        },
        mir::Instruction::ElementSet {
            destination,
            array,
            index,
            value,
        } => mir::Instruction::ElementSet {
            destination: *destination,
            array: substitute(array),
            index: substitute(index),
            value: substitute(value),
        },
        mir::Instruction::LocalSet { local, value } => mir::Instruction::LocalSet {
            local: *local,
            value: substitute(value),
        },
        mir::Instruction::CallIndirect {
            destination,
            callee,
            arguments,
        } => mir::Instruction::CallIndirect {
            destination: *destination,
            callee: substitute(callee),
            arguments: *arguments,
        },
        mir::Instruction::ManagedAllocArray {
            destination,
            element,
            length,
        } => mir::Instruction::ManagedAllocArray {
            destination: *destination,
            element: *element,
            length: substitute(length),
        },
        mir::Instruction::RawFree { pointer } => mir::Instruction::RawFree {
            pointer: substitute(pointer),
        },
        // instructions without value operands or with externalized arguments
        mir::Instruction::Const { .. }
        | mir::Instruction::LocalGet { .. }
        | mir::Instruction::GlobalAddr { .. }
        | mir::Instruction::GlobalConst { .. }
        | mir::Instruction::Call { .. }
        | mir::Instruction::ManagedAlloc { .. }
        | mir::Instruction::RawAlloc { .. }
        | mir::Instruction::StackAlloc { .. }
        | mir::Instruction::Intrinsic { .. } => instruction.clone(),
    }
}
