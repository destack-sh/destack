use std::collections::HashMap;

use destack_mir as mir;
use smallvec::SmallVec;

use crate::memory::Value;

use super::handlers;
use super::types::{InstData, SwitchCase, ThreadedBlock, ThreadedFunction, ThreadedInst};

/// Convert a MIR function to threaded form for fast execution.
pub fn thread_function(
    tree: &mir::NodeTree,
    func_id: mir::LocalNodeId<mir::Function>,
) -> Option<ThreadedFunction> {
    let func = tree.get(func_id);

    // imported functions can't be threaded
    if func.is_import() {
        return None;
    }

    let entry_block = func.entry?;

    // build block index map: mir block id -> index in threaded function
    let mut block_index_map: HashMap<mir::LocalNodeId<mir::Block>, usize> = HashMap::new();
    let mut mir_blocks: Vec<mir::LocalNodeId<mir::Block>> = Vec::new();

    // collect all reachable blocks via BFS
    let mut queue = vec![entry_block];
    let mut visited = std::collections::HashSet::new();

    while let Some(block_id) = queue.pop() {
        if visited.contains(&block_id) {
            continue;
        }
        visited.insert(block_id);

        let idx = mir_blocks.len();
        block_index_map.insert(block_id, idx);
        mir_blocks.push(block_id);

        // add successor blocks
        let block = tree.get(block_id);
        match &block.terminator {
            mir::Terminator::Jump { target, .. } => {
                queue.push(*target);
            }
            mir::Terminator::Branch { then_target, else_target, .. } => {
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

    // convert each block
    let mut threaded_blocks = Vec::with_capacity(mir_blocks.len());

    for mir_block_id in &mir_blocks {
        let block = tree.get(*mir_block_id);
        let threaded = thread_block(tree, block, &block_index_map);
        threaded_blocks.push(threaded);
    }

    // extract function parameters
    let parameters: SmallVec<[mir::Value; 4]> =
        func.parameters.iter().map(|p| p.value).collect();

    Some(ThreadedFunction {
        parameters,
        entry: block_index_map[&entry_block],
        blocks: threaded_blocks,
        mir_function: func_id,
    })
}

/// Convert a MIR block to threaded form.
fn thread_block(
    tree: &mir::NodeTree,
    block: &mir::Block,
    block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
) -> ThreadedBlock {
    let mut instructions = Vec::with_capacity(block.instructions.len() + 1);

    // convert regular instructions
    for &inst_id in &block.instructions {
        let inst = tree.get(inst_id);
        let threaded = thread_instruction(tree, inst);
        instructions.push(threaded);
    }

    // convert terminator as final instruction
    let terminator = thread_terminator(&block.terminator, block_index_map);
    instructions.push(terminator);

    // extract block parameters
    let parameters: SmallVec<[mir::Value; 4]> =
        block.parameters.iter().map(|p| p.value).collect();

    ThreadedBlock {
        parameters,
        instructions,
    }
}

/// Convert a MIR instruction to threaded form.
fn thread_instruction(tree: &mir::NodeTree, inst: &mir::Instruction) -> ThreadedInst {
    match inst {
        mir::Instruction::Const { destination, value } => ThreadedInst {
            handler: handlers::handle_const,
            data: InstData::Const {
                dest: *destination,
                value: Value::from(value),
            },
        },

        mir::Instruction::Binary { destination, operator, left, right } => ThreadedInst {
            handler: handlers::handle_binary,
            data: InstData::Binary {
                dest: *destination,
                op: *operator,
                left: *left,
                right: *right,
            },
        },

        mir::Instruction::Unary { destination, operator, argument } => ThreadedInst {
            handler: handlers::handle_unary,
            data: InstData::Unary {
                dest: *destination,
                op: *operator,
                arg: *argument,
            },
        },

        mir::Instruction::Cast { destination, operator, argument, to_type } => ThreadedInst {
            handler: handlers::handle_cast,
            data: InstData::Cast {
                dest: *destination,
                op: *operator,
                arg: *argument,
                to_type: *to_type,
            },
        },

        mir::Instruction::Call { destination, function, arguments } => {
            let args: SmallVec<[mir::Value; 4]> =
                tree.get_arguments(*arguments).iter().copied().collect();
            ThreadedInst {
                handler: handlers::handle_call,
                data: InstData::Call {
                    dest: *destination,
                    function: *function,
                    arguments: args,
                },
            }
        }

        mir::Instruction::CallIndirect { destination, callee, arguments } => {
            let args: SmallVec<[mir::Value; 4]> =
                tree.get_arguments(*arguments).iter().copied().collect();
            ThreadedInst {
                handler: handlers::handle_call_indirect,
                data: InstData::CallIndirect {
                    dest: *destination,
                    callee: *callee,
                    arguments: args,
                },
            }
        }

        mir::Instruction::LocalGet { destination, local } => ThreadedInst {
            handler: handlers::handle_local_get,
            data: InstData::LocalGet {
                dest: *destination,
                local: *local,
            },
        },

        mir::Instruction::LocalSet { local, value } => ThreadedInst {
            handler: handlers::handle_local_set,
            data: InstData::LocalSet {
                local: *local,
                value: *value,
            },
        },

        mir::Instruction::GlobalAddr { destination, global } => ThreadedInst {
            handler: handlers::handle_global_addr,
            data: InstData::GlobalAddr {
                dest: *destination,
                global: *global,
            },
        },

        mir::Instruction::GlobalConst { destination, global } => ThreadedInst {
            handler: handlers::handle_global_const,
            data: InstData::GlobalConst {
                dest: *destination,
                global: *global,
            },
        },

        mir::Instruction::Load { destination, pointer } => ThreadedInst {
            handler: handlers::handle_load,
            data: InstData::Load {
                dest: *destination,
                pointer: *pointer,
            },
        },

        mir::Instruction::Store { pointer, value } => ThreadedInst {
            handler: handlers::handle_store,
            data: InstData::Store {
                pointer: *pointer,
                value: *value,
            },
        },

        mir::Instruction::FieldGet { destination, aggregate, index } => ThreadedInst {
            handler: handlers::handle_field_get,
            data: InstData::FieldGet {
                dest: *destination,
                aggregate: *aggregate,
                index: *index,
            },
        },

        mir::Instruction::FieldSet { destination, aggregate, index, value } => ThreadedInst {
            handler: handlers::handle_field_set,
            data: InstData::FieldSet {
                dest: *destination,
                aggregate: *aggregate,
                index: *index,
                value: *value,
            },
        },

        mir::Instruction::ElementGet { destination, array, index } => ThreadedInst {
            handler: handlers::handle_element_get,
            data: InstData::ElementGet {
                dest: *destination,
                array: *array,
                index: *index,
            },
        },

        mir::Instruction::ElementSet { destination, array, index, value } => ThreadedInst {
            handler: handlers::handle_element_set,
            data: InstData::ElementSet {
                dest: *destination,
                array: *array,
                index: *index,
                value: *value,
            },
        },

        mir::Instruction::ManagedAlloc { destination, .. } => ThreadedInst {
            handler: handlers::handle_managed_alloc,
            data: InstData::ManagedAlloc { dest: *destination },
        },

        mir::Instruction::ManagedAllocArray { destination, length, .. } => ThreadedInst {
            handler: handlers::handle_managed_alloc_array,
            data: InstData::ManagedAllocArray {
                dest: *destination,
                length: *length,
            },
        },

        mir::Instruction::RawAlloc { destination, .. } => ThreadedInst {
            handler: handlers::handle_raw_alloc,
            data: InstData::RawAlloc { dest: *destination },
        },

        mir::Instruction::RawFree { pointer } => ThreadedInst {
            handler: handlers::handle_raw_free,
            data: InstData::RawFree { pointer: *pointer },
        },

        mir::Instruction::StackAlloc { destination, .. } => ThreadedInst {
            handler: handlers::handle_stack_alloc,
            data: InstData::StackAlloc { dest: *destination },
        },

        mir::Instruction::Intrinsic { destination, intrinsic, arguments, ordering } => {
            let args: SmallVec<[mir::Value; 4]> =
                tree.get_arguments(*arguments).iter().copied().collect();
            ThreadedInst {
                handler: handlers::handle_intrinsic,
                data: InstData::Intrinsic {
                    dest: *destination,
                    intrinsic: *intrinsic,
                    arguments: args,
                    ordering: *ordering,
                },
            }
        }
    }
}

/// Convert a MIR terminator to threaded form.
fn thread_terminator(
    term: &mir::Terminator,
    block_index_map: &HashMap<mir::LocalNodeId<mir::Block>, usize>,
) -> ThreadedInst {
    match term {
        mir::Terminator::Return { value } => ThreadedInst {
            handler: handlers::handle_return,
            data: InstData::Return { value: *value },
        },

        mir::Terminator::Jump { target, arguments } => ThreadedInst {
            handler: handlers::handle_jump,
            data: InstData::Jump {
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
        } => ThreadedInst {
            handler: handlers::handle_branch,
            data: InstData::Branch {
                condition: *condition,
                then_target: block_index_map[then_target],
                then_arguments: then_arguments.iter().copied().collect(),
                else_target: block_index_map[else_target],
                else_arguments: else_arguments.iter().copied().collect(),
            },
        },

        mir::Terminator::Switch { value, cases, default, default_arguments } => {
            let threaded_cases: Vec<SwitchCase> = cases
                .iter()
                .map(|c| SwitchCase {
                    value: c.value,
                    target: block_index_map[&c.target],
                    arguments: c.arguments.iter().copied().collect(),
                })
                .collect();

            ThreadedInst {
                handler: handlers::handle_switch,
                data: InstData::Switch {
                    value: *value,
                    cases: threaded_cases,
                    default_target: block_index_map[default],
                    default_arguments: default_arguments.iter().copied().collect(),
                },
            }
        }

        mir::Terminator::Unreachable => ThreadedInst {
            handler: handlers::handle_unreachable,
            data: InstData::Unreachable,
        },

        mir::Terminator::Yield { .. } => {
            // NOTE #Incomplete: yield not yet supported in threaded interpreter
            ThreadedInst {
                handler: handlers::handle_unreachable,
                data: InstData::Unreachable,
            }
        }
    }
}
