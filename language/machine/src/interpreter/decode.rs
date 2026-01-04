use std::collections::HashMap;

use destack_mir as mir;
use smallvec::SmallVec;

use crate::memory::Value;

use super::bytecode;
use super::threaded::{
    SwitchCase, ThreadedBlock, ThreadedFunction, ThreadedInstruction, ThreadedInstructionData,
};

/// Convert a MIR function to threaded form for fast execution.
pub(super) fn thread_function(
    tree: &mir::NodeTree,
    func_id: mir::LocalNodeId<mir::Function>,
) -> Option<ThreadedFunction> {
    // load function
    let func = tree.get(func_id);

    // imported functions can't be threaded
    if func.is_import() {
        return None;
    }

    // read entry block
    let entry_block = func.entry?;

    // prepare block index mapping
    let mut block_index_map: HashMap<mir::LocalNodeId<mir::Block>, usize> = HashMap::new();
    let mut mir_blocks: Vec<mir::LocalNodeId<mir::Block>> = Vec::new();

    // seed traversal queue
    let mut queue = vec![entry_block];
    let mut visited = std::collections::HashSet::new();

    while let Some(block_id) = queue.pop() {
        // skip visited blocks
        if visited.contains(&block_id) {
            continue;
        }

        // record block index
        visited.insert(block_id);
        let idx = mir_blocks.len();
        block_index_map.insert(block_id, idx);
        mir_blocks.push(block_id);

        // enqueue successor blocks
        let block = tree.get(block_id);
        match &block.terminator {
            mir::Terminator::Jump { target, .. } => {
                queue.push(*target);
            }
            mir::Terminator::Branch {
                then_target,
                else_target,
                ..
            } => {
                queue.push(*then_target);
                queue.push(*else_target);
            }
            mir::Terminator::Switch { cases, default, .. } => {
                for case in cases {
                    queue.push(case.target);
                }
                queue.push(*default);
            }
            mir::Terminator::Return { .. }
            | mir::Terminator::Unreachable
            | mir::Terminator::Yield { .. } => {}
        }
    }

    // allocate threaded blocks
    let mut threaded_blocks = Vec::with_capacity(mir_blocks.len());

    // thread each mir block
    for mir_block_id in &mir_blocks {
        let block = tree.get(*mir_block_id);
        let threaded = thread_block(tree, *mir_block_id, block, &block_index_map);
        threaded_blocks.push(threaded);
    }

    // gather function parameters
    let parameters: SmallVec<[mir::Value; 4]> = func.parameters.iter().map(|p| p.value).collect();

    // compute storage sizes
    let value_count = compute_value_count(&parameters, &threaded_blocks);
    let local_count = func.locals.len();

    // assemble threaded function
    Some(ThreadedFunction {
        parameters,
        entry: block_index_map[&entry_block],
        blocks: threaded_blocks,
        value_count,
        local_count,
    })
}

/// Convert a MIR block to threaded form.
fn thread_block(
    tree: &mir::NodeTree,
    mir_block: mir::LocalNodeId<mir::Block>,
    block: &mir::Block,
    block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
) -> ThreadedBlock {
    // preallocate instruction list
    let mut instructions = Vec::with_capacity(block.instructions.len() + 1);

    // convert regular instructions
    for &inst_id in &block.instructions {
        let inst = tree.get(inst_id);
        let threaded = thread_instruction(tree, inst);
        instructions.push(threaded);
    }

    // append threaded terminator
    let terminator = thread_terminator(&block.terminator, block_index_map);
    instructions.push(terminator);

    // gather block parameters
    let parameters: SmallVec<[mir::Value; 4]> = block.parameters.iter().map(|p| p.value).collect();

    // assemble block
    ThreadedBlock {
        mir_block,
        parameters,
        instructions,
    }
}

/// Convert a MIR instruction to threaded form.
fn thread_instruction(tree: &mir::NodeTree, inst: &mir::Instruction) -> ThreadedInstruction {
    // map instruction opcode to threaded form
    match inst {
        mir::Instruction::Const { destination, value } => ThreadedInstruction {
            handler: bytecode::handle_const,
            data: ThreadedInstructionData::Const {
                dest: *destination,
                value: Value::from(value),
            },
        },

        mir::Instruction::Binary {
            destination,
            operator,
            left,
            right,
        } => ThreadedInstruction {
            handler: bytecode::handle_binary,
            data: ThreadedInstructionData::Binary {
                dest: *destination,
                op: *operator,
                left: *left,
                right: *right,
            },
        },

        mir::Instruction::Unary {
            destination,
            operator,
            argument,
        } => ThreadedInstruction {
            handler: bytecode::handle_unary,
            data: ThreadedInstructionData::Unary {
                dest: *destination,
                op: *operator,
                arg: *argument,
            },
        },

        mir::Instruction::Cast {
            destination,
            operator,
            argument,
            to_type,
        } => ThreadedInstruction {
            handler: bytecode::handle_cast,
            data: ThreadedInstructionData::Cast {
                dest: *destination,
                op: *operator,
                arg: *argument,
                to_type: *to_type,
            },
        },

        mir::Instruction::Call {
            destination,
            function,
            arguments,
        } => {
            let args: SmallVec<[mir::Value; 4]> =
                tree.get_arguments(*arguments).iter().copied().collect();
            ThreadedInstruction {
                handler: bytecode::handle_call,
                data: ThreadedInstructionData::Call {
                    dest: *destination,
                    function: *function,
                    arguments: args,
                },
            }
        }

        mir::Instruction::CallIndirect {
            destination,
            callee,
            arguments,
        } => {
            let args: SmallVec<[mir::Value; 4]> =
                tree.get_arguments(*arguments).iter().copied().collect();
            ThreadedInstruction {
                handler: bytecode::handle_call_indirect,
                data: ThreadedInstructionData::CallIndirect {
                    dest: *destination,
                    callee: *callee,
                    arguments: args,
                },
            }
        }

        mir::Instruction::LocalGet { destination, local } => ThreadedInstruction {
            handler: bytecode::handle_local_get,
            data: ThreadedInstructionData::LocalGet {
                dest: *destination,
                local: *local,
            },
        },

        mir::Instruction::LocalSet { local, value } => ThreadedInstruction {
            handler: bytecode::handle_local_set,
            data: ThreadedInstructionData::LocalSet {
                local: *local,
                value: *value,
            },
        },

        mir::Instruction::GlobalAddr {
            destination,
            global,
        } => ThreadedInstruction {
            handler: bytecode::handle_global_addr,
            data: ThreadedInstructionData::GlobalAddr {
                dest: *destination,
                global: *global,
            },
        },

        mir::Instruction::GlobalConst {
            destination,
            global,
        } => ThreadedInstruction {
            handler: bytecode::handle_global_const,
            data: ThreadedInstructionData::GlobalConst {
                dest: *destination,
                global: *global,
            },
        },

        mir::Instruction::Load {
            destination,
            pointer,
        } => ThreadedInstruction {
            handler: bytecode::handle_load,
            data: ThreadedInstructionData::Load {
                dest: *destination,
                pointer: *pointer,
            },
        },

        mir::Instruction::Store { pointer, value } => ThreadedInstruction {
            handler: bytecode::handle_store,
            data: ThreadedInstructionData::Store {
                pointer: *pointer,
                value: *value,
            },
        },

        mir::Instruction::FieldGet {
            destination,
            aggregate,
            index,
        } => ThreadedInstruction {
            handler: bytecode::handle_field_get,
            data: ThreadedInstructionData::FieldGet {
                dest: *destination,
                aggregate: *aggregate,
                index: *index,
            },
        },

        mir::Instruction::FieldSet {
            destination,
            aggregate,
            index,
            value,
        } => ThreadedInstruction {
            handler: bytecode::handle_field_set,
            data: ThreadedInstructionData::FieldSet {
                dest: *destination,
                aggregate: *aggregate,
                index: *index,
                value: *value,
            },
        },

        mir::Instruction::ElementGet {
            destination,
            array,
            index,
        } => ThreadedInstruction {
            handler: bytecode::handle_element_get,
            data: ThreadedInstructionData::ElementGet {
                dest: *destination,
                array: *array,
                index: *index,
            },
        },

        mir::Instruction::ElementSet {
            destination,
            array,
            index,
            value,
        } => ThreadedInstruction {
            handler: bytecode::handle_element_set,
            data: ThreadedInstructionData::ElementSet {
                dest: *destination,
                array: *array,
                index: *index,
                value: *value,
            },
        },

        mir::Instruction::ManagedAlloc { destination, .. } => ThreadedInstruction {
            handler: bytecode::handle_managed_alloc,
            data: ThreadedInstructionData::ManagedAlloc { dest: *destination },
        },

        mir::Instruction::ManagedAllocArray {
            destination,
            length,
            ..
        } => ThreadedInstruction {
            handler: bytecode::handle_managed_alloc_array,
            data: ThreadedInstructionData::ManagedAllocArray {
                dest: *destination,
                length: *length,
            },
        },

        mir::Instruction::RawAlloc { destination, .. } => ThreadedInstruction {
            handler: bytecode::handle_raw_alloc,
            data: ThreadedInstructionData::RawAlloc { dest: *destination },
        },

        mir::Instruction::RawFree { pointer } => ThreadedInstruction {
            handler: bytecode::handle_raw_free,
            data: ThreadedInstructionData::RawFree { pointer: *pointer },
        },

        mir::Instruction::StackAlloc { destination, .. } => ThreadedInstruction {
            handler: bytecode::handle_stack_alloc,
            data: ThreadedInstructionData::StackAlloc { dest: *destination },
        },

        mir::Instruction::Intrinsic {
            destination,
            intrinsic,
            arguments,
            ordering,
        } => {
            let args: SmallVec<[mir::Value; 4]> =
                tree.get_arguments(*arguments).iter().copied().collect();
            ThreadedInstruction {
                handler: bytecode::handle_intrinsic,
                data: ThreadedInstructionData::Intrinsic {
                    dest: *destination,
                    intrinsic: *intrinsic,
                    arguments: args,
                    ordering: *ordering,
                },
            }
        }
    }
}

/// Compute the number of SSA values required by a threaded function.
fn compute_value_count(parameters: &[mir::Value], blocks: &[ThreadedBlock]) -> usize {
    // start with no max value id
    let mut max_value: Option<u32> = None;

    // scan function parameters
    for value in parameters {
        update_max_value(&mut max_value, *value);
    }

    // scan block parameters and instructions
    for block in blocks {
        for value in &block.parameters {
            update_max_value(&mut max_value, *value);
        }
        for inst in &block.instructions {
            update_max_from_instruction(&mut max_value, &inst.data);
        }
    }

    max_value.map(|id| id as usize + 1).unwrap_or(0)
}

/// Update the tracked maximum SSA value id.
fn update_max_value(max_value: &mut Option<u32>, value: mir::Value) {
    // grab the raw value id
    let id = value.0;

    // update max tracking
    match max_value {
        Some(current) => {
            if id > *current {
                *current = id;
            }
        }
        None => {
            *max_value = Some(id);
        }
    }
}

/// Update the tracked maximum SSA value id based on instruction operands.
fn update_max_from_instruction(max_value: &mut Option<u32>, data: &ThreadedInstructionData) {
    // scan operands for SSA values
    match data {
        ThreadedInstructionData::Const { dest, .. } => update_max_value(max_value, *dest),
        ThreadedInstructionData::Binary {
            dest, left, right, ..
        } => {
            update_max_value(max_value, *dest);
            update_max_value(max_value, *left);
            update_max_value(max_value, *right);
        }
        ThreadedInstructionData::Unary { dest, arg, .. } => {
            update_max_value(max_value, *dest);
            update_max_value(max_value, *arg);
        }
        ThreadedInstructionData::Cast { dest, arg, .. } => {
            update_max_value(max_value, *dest);
            update_max_value(max_value, *arg);
        }
        ThreadedInstructionData::Call {
            dest, arguments, ..
        } => {
            if let Some(dest) = dest {
                update_max_value(max_value, *dest);
            }
            for arg in arguments {
                update_max_value(max_value, *arg);
            }
        }
        ThreadedInstructionData::CallIndirect {
            dest,
            callee,
            arguments,
        } => {
            if let Some(dest) = dest {
                update_max_value(max_value, *dest);
            }
            update_max_value(max_value, *callee);
            for arg in arguments {
                update_max_value(max_value, *arg);
            }
        }
        ThreadedInstructionData::LocalGet { dest, .. } => {
            update_max_value(max_value, *dest);
        }
        ThreadedInstructionData::LocalSet { value, .. } => {
            update_max_value(max_value, *value);
        }
        ThreadedInstructionData::GlobalAddr { dest, .. } => {
            update_max_value(max_value, *dest);
        }
        ThreadedInstructionData::GlobalConst { dest, .. } => {
            update_max_value(max_value, *dest);
        }
        ThreadedInstructionData::Load { dest, pointer } => {
            update_max_value(max_value, *dest);
            update_max_value(max_value, *pointer);
        }
        ThreadedInstructionData::Store { pointer, value } => {
            update_max_value(max_value, *pointer);
            update_max_value(max_value, *value);
        }
        ThreadedInstructionData::FieldGet {
            dest, aggregate, ..
        } => {
            update_max_value(max_value, *dest);
            update_max_value(max_value, *aggregate);
        }
        ThreadedInstructionData::FieldSet {
            dest,
            aggregate,
            value,
            ..
        } => {
            update_max_value(max_value, *dest);
            update_max_value(max_value, *aggregate);
            update_max_value(max_value, *value);
        }
        ThreadedInstructionData::ElementGet { dest, array, index } => {
            update_max_value(max_value, *dest);
            update_max_value(max_value, *array);
            update_max_value(max_value, *index);
        }
        ThreadedInstructionData::ElementSet {
            dest,
            array,
            index,
            value,
        } => {
            update_max_value(max_value, *dest);
            update_max_value(max_value, *array);
            update_max_value(max_value, *index);
            update_max_value(max_value, *value);
        }
        ThreadedInstructionData::ManagedAlloc { dest } => update_max_value(max_value, *dest),
        ThreadedInstructionData::ManagedAllocArray { dest, length } => {
            update_max_value(max_value, *dest);
            update_max_value(max_value, *length);
        }
        ThreadedInstructionData::RawAlloc { dest } => update_max_value(max_value, *dest),
        ThreadedInstructionData::RawFree { pointer } => update_max_value(max_value, *pointer),
        ThreadedInstructionData::StackAlloc { dest } => update_max_value(max_value, *dest),
        ThreadedInstructionData::Intrinsic {
            dest, arguments, ..
        } => {
            if let Some(dest) = dest {
                update_max_value(max_value, *dest);
            }
            for arg in arguments {
                update_max_value(max_value, *arg);
            }
        }
        ThreadedInstructionData::Return { value } => {
            if let Some(value) = value {
                update_max_value(max_value, *value);
            }
        }
        ThreadedInstructionData::Jump { arguments, .. } => {
            for arg in arguments {
                update_max_value(max_value, *arg);
            }
        }
        ThreadedInstructionData::Branch {
            condition,
            then_arguments,
            else_arguments,
            ..
        } => {
            update_max_value(max_value, *condition);
            for arg in then_arguments {
                update_max_value(max_value, *arg);
            }
            for arg in else_arguments {
                update_max_value(max_value, *arg);
            }
        }
        ThreadedInstructionData::Switch {
            value,
            cases,
            default_arguments,
            ..
        } => {
            update_max_value(max_value, *value);
            for case in cases {
                for arg in &case.arguments {
                    update_max_value(max_value, *arg);
                }
            }
            for arg in default_arguments {
                update_max_value(max_value, *arg);
            }
        }
        ThreadedInstructionData::Unreachable | ThreadedInstructionData::Unsupported { .. } => {}
    }
}

/// Convert a MIR terminator to threaded form.
fn thread_terminator(
    term: &mir::Terminator,
    block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
) -> ThreadedInstruction {
    // map terminator opcode to threaded form
    match term {
        mir::Terminator::Return { value } => ThreadedInstruction {
            handler: bytecode::handle_return,
            data: ThreadedInstructionData::Return { value: *value },
        },

        mir::Terminator::Jump { target, arguments } => ThreadedInstruction {
            handler: bytecode::handle_jump,
            data: ThreadedInstructionData::Jump {
                target: block_index_map[target],
                arguments: arguments.iter().copied().collect(),
            },
        },

        mir::Terminator::Branch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
        } => ThreadedInstruction {
            handler: bytecode::handle_branch,
            data: ThreadedInstructionData::Branch {
                condition: *condition,
                then_target: block_index_map[then_target],
                then_arguments: then_arguments.iter().copied().collect(),
                else_target: block_index_map[else_target],
                else_arguments: else_arguments.iter().copied().collect(),
            },
        },

        mir::Terminator::Switch {
            value,
            cases,
            default,
            default_arguments,
        } => {
            let threaded_cases: Vec<SwitchCase> = cases
                .iter()
                .map(|c| SwitchCase {
                    value: c.value,
                    target: block_index_map[&c.target],
                    arguments: c.arguments.iter().copied().collect(),
                })
                .collect();

            ThreadedInstruction {
                handler: bytecode::handle_switch,
                data: ThreadedInstructionData::Switch {
                    value: *value,
                    cases: threaded_cases,
                    default_target: block_index_map[default],
                    default_arguments: default_arguments.iter().copied().collect(),
                },
            }
        }

        mir::Terminator::Unreachable => ThreadedInstruction {
            handler: bytecode::handle_unreachable,
            data: ThreadedInstructionData::Unreachable,
        },

        mir::Terminator::Yield { .. } => ThreadedInstruction {
            handler: bytecode::handle_unsupported,
            data: ThreadedInstructionData::Unsupported { name: "yield" },
        },
    }
}
