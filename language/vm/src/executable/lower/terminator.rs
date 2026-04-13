use std::collections::HashMap;

use destack_mir as mir;

use destack_heap::Value;

use crate::executable::{Instruction, InstructionData, InstructionOperation, pack_optional_value};
use crate::{Error, Result};

use super::kind::{managed_pointee_type_for_value, managed_pointee_type_for_value_kind};
use super::lower::BlockLowerer;
use super::operation::{
    select_branch_operation, select_compare_branch_const_operation,
    select_compare_branch_operation, select_switch_operation, select_switch_table_operation,
    swap_compare_operator,
};
use super::pool::Pool;
use super::tree::ValueDecomposition;

impl<'a> BlockLowerer<'a> {
    /// Try to fuse compare + branch into a single instruction.
    pub(super) fn try_fuse_compare_branch(
        &self,
        block: &mir::Block,
        instructions: &mut Vec<Instruction>,
        pool: &mut Pool,
    ) -> Option<Instruction> {
        let terminator = self.tree.get(block.terminator);
        let mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
        } = terminator
        else {
            return None;
        };
        let condition = condition.value()?;
        let then_target_block = then_target.block.block()?;
        let else_target_block = else_target.block.block()?;
        let then_arguments = then_target
            .arguments
            .iter()
            .map(|argument| argument.value())
            .collect::<Option<Vec<_>>>()?;
        let else_arguments = else_target
            .arguments
            .iter()
            .map(|argument| argument.value())
            .collect::<Option<Vec<_>>>()?;

        if self.value_use_count(condition) != Some(1) {
            return None;
        }

        let last_inst_id = block.instructions.last()?;
        let last_inst = self.tree.get(*last_inst_id);

        let mir::Instruction::Binary {
            destination,
            operator,
            left,
            right,
        } = last_inst
        else {
            return None;
        };

        if !operator.is_comparison() {
            return None;
        }

        let destination = destination.value()?;
        let left = left.value()?;
        let right = right.value()?;
        if destination != condition {
            return None;
        }

        let mut const_value = None;
        let mut left_value = left;
        let mut operator = *operator;
        let mut pop_const = false;
        if let Some(prev_inst_id) = block
            .instructions
            .get(block.instructions.len().saturating_sub(2))
        {
            let prev_inst = self.tree.get(*prev_inst_id);
            if let mir::Instruction::Const { destination, value } = prev_inst {
                let destination = destination.value()?;
                let uses = self.value_use_count(destination);
                if uses == Some(1) {
                    if destination == right {
                        const_value = Some(Value::from(value));
                        pop_const = true;
                    } else if destination == left {
                        const_value = Some(Value::from(value));
                        left_value = right;
                        operator = swap_compare_operator(operator);
                        pop_const = true;
                    }
                }
            }
        }

        instructions.pop();
        if pop_const {
            instructions.pop();
        }

        let then_index = self.block_index_by_id[&then_target_block];
        let else_index = self.block_index_by_id[&else_target_block];
        let then_parameters = self
            .block_parameter
            .get(then_index)
            .map(|params| params.as_slice())
            .unwrap_or_default();
        let else_parameters = self
            .block_parameter
            .get(else_index)
            .map(|params| params.as_slice())
            .unwrap_or_default();
        let then_copies = pool.copy_range(then_parameters, &then_arguments);
        let else_copies = pool.copy_range(else_parameters, &else_arguments);

        if let Some(right_const) = const_value {
            let operation = select_compare_branch_const_operation(operator);
            return Some(Instruction {
                operation,
                data: InstructionData::CompareAndBranchConst {
                    left: left_value,
                    right_const,
                    operator,
                    then_target: then_index as u32,
                    then_copies,
                    else_target: else_index as u32,
                    else_copies,
                },
            });
        }

        let operation = select_compare_branch_operation(operator);
        Some(Instruction {
            operation,
            data: InstructionData::CompareAndBranch {
                left,
                right,
                operator,
                then_target: then_index as u32,
                then_copies,
                else_target: else_index as u32,
                else_copies,
            },
        })
    }

    /// Convert a MIR terminator to lowered interpreter form.
    pub(super) fn lower_terminator(
        &self,
        term: &mir::Terminator,
        decomposition_by_value: &HashMap<mir::Value, ValueDecomposition>,
        pool: &mut Pool,
    ) -> Result<Instruction> {
        Ok(match term {
            mir::Terminator::Error => {
                return Err(Error::ConcreteMirRequired {
                    context: "terminator".to_string(),
                });
            }
            mir::Terminator::Return { value } => Instruction {
                operation: InstructionOperation::Return,
                data: InstructionData::Return {
                    value: pack_optional_value(
                        (*value)
                            .map(|value| {
                                value.value().ok_or_else(|| Error::ConcreteMirRequired {
                                    context: "return value".to_string(),
                                })
                            })
                            .transpose()?,
                    ),
                },
            },

            mir::Terminator::Jump { target } => {
                let target_block =
                    (target.block)
                        .block()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "jump target".to_string(),
                        })?;
                let arguments = target
                    .arguments
                    .iter()
                    .map(|argument| {
                        (*argument)
                            .value()
                            .ok_or_else(|| Error::ConcreteMirRequired {
                                context: "jump argument".to_string(),
                            })
                    })
                    .collect::<Result<Vec<_>>>()?;
                let target_index = self.block_index_by_id[&target_block];
                let target_parameters = self
                    .block_parameter
                    .get(target_index)
                    .map(|params| params.as_slice())
                    .unwrap_or_default();
                let value_tree_by_param = self.block_parameter_map.tree_by_block.get(&target_block);
                let copies = pool.edge_copy_plan(
                    target_parameters,
                    &arguments,
                    value_tree_by_param,
                    decomposition_by_value,
                )?;

                Instruction {
                    operation: InstructionOperation::Jump,
                    data: InstructionData::Jump {
                        target: target_index as u32,
                        copies,
                    },
                }
            }

            mir::Terminator::Branch {
                condition,
                then_target,
                else_target,
            } => {
                let condition = (*condition)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "branch condition".to_string(),
                    })?;
                let then_target_block =
                    (then_target.block)
                        .block()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "branch then target".to_string(),
                        })?;
                let else_target_block =
                    (else_target.block)
                        .block()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "branch else target".to_string(),
                        })?;
                let then_arguments = then_target
                    .arguments
                    .iter()
                    .map(|argument| {
                        (*argument)
                            .value()
                            .ok_or_else(|| Error::ConcreteMirRequired {
                                context: "branch then argument".to_string(),
                            })
                    })
                    .collect::<Result<Vec<_>>>()?;
                let else_arguments = else_target
                    .arguments
                    .iter()
                    .map(|argument| {
                        (*argument)
                            .value()
                            .ok_or_else(|| Error::ConcreteMirRequired {
                                context: "branch else argument".to_string(),
                            })
                    })
                    .collect::<Result<Vec<_>>>()?;
                let then_index = self.block_index_by_id[&then_target_block];
                let else_index = self.block_index_by_id[&else_target_block];
                let then_parameters = self
                    .block_parameter
                    .get(then_index)
                    .map(|params| params.as_slice())
                    .unwrap_or_default();
                let else_parameters = self
                    .block_parameter
                    .get(else_index)
                    .map(|params| params.as_slice())
                    .unwrap_or_default();
                let then_value_tree = self
                    .block_parameter_map
                    .tree_by_block
                    .get(&then_target_block);
                let else_value_tree = self
                    .block_parameter_map
                    .tree_by_block
                    .get(&else_target_block);
                let then_copies = pool.edge_copy_plan(
                    then_parameters,
                    &then_arguments,
                    then_value_tree,
                    decomposition_by_value,
                )?;
                let else_copies = pool.edge_copy_plan(
                    else_parameters,
                    &else_arguments,
                    else_value_tree,
                    decomposition_by_value,
                )?;

                Instruction {
                    operation: select_branch_operation(self.value_kind_map(), condition),
                    data: InstructionData::Branch {
                        condition,
                        then_target: then_index as u32,
                        then_copies,
                        else_target: else_index as u32,
                        else_copies,
                    },
                }
            }

            mir::Terminator::Check {
                constraint,
                success,
                failure,
            } => {
                let success_block =
                    (success.block)
                        .block()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "check success target".to_string(),
                        })?;
                let failure_block =
                    (failure.block)
                        .block()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "check failure target".to_string(),
                        })?;
                let success_arguments = success
                    .arguments
                    .iter()
                    .map(|argument| {
                        (*argument)
                            .value()
                            .ok_or_else(|| Error::ConcreteMirRequired {
                                context: "check success argument".to_string(),
                            })
                    })
                    .collect::<Result<Vec<_>>>()?;
                let failure_arguments = failure
                    .arguments
                    .iter()
                    .map(|argument| {
                        (*argument)
                            .value()
                            .ok_or_else(|| Error::ConcreteMirRequired {
                                context: "check failure argument".to_string(),
                            })
                    })
                    .collect::<Result<Vec<_>>>()?;
                let success_index = self.block_index_by_id[&success_block];
                let failure_index = self.block_index_by_id[&failure_block];
                let success_parameters = self
                    .block_parameter
                    .get(success_index)
                    .map(|params| params.as_slice())
                    .unwrap_or_default();
                let failure_parameters = self
                    .block_parameter
                    .get(failure_index)
                    .map(|params| params.as_slice())
                    .unwrap_or_default();
                let success_value_tree = self.block_parameter_map.tree_by_block.get(&success_block);
                let failure_value_tree = self.block_parameter_map.tree_by_block.get(&failure_block);
                let success_copies = pool.edge_copy_plan(
                    success_parameters,
                    &success_arguments,
                    success_value_tree,
                    decomposition_by_value,
                )?;
                let failure_copies = pool.edge_copy_plan(
                    failure_parameters,
                    &failure_arguments,
                    failure_value_tree,
                    decomposition_by_value,
                )?;

                Instruction {
                    operation: InstructionOperation::Check,
                    data: InstructionData::Check {
                        constraint: constraint.clone(),
                        then_target: success_index as u32,
                        then_copies: success_copies,
                        else_target: failure_index as u32,
                        else_copies: failure_copies,
                    },
                }
            }

            mir::Terminator::Switch {
                value,
                cases,
                default,
            } => {
                let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "switch value".to_string(),
                })?;
                let default_block =
                    (default.block)
                        .block()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "switch default target".to_string(),
                        })?;
                let default_arguments = default
                    .arguments
                    .iter()
                    .map(|argument| {
                        (*argument)
                            .value()
                            .ok_or_else(|| Error::ConcreteMirRequired {
                                context: "switch default argument".to_string(),
                            })
                    })
                    .collect::<Result<Vec<_>>>()?;
                let default_index = self.block_index_by_id[&default_block];
                let default_parameters = self
                    .block_parameter
                    .get(default_index)
                    .map(|params| params.as_slice())
                    .unwrap_or_default();
                let default_value_tree = self.block_parameter_map.tree_by_block.get(&default_block);
                let default_copies = pool.edge_copy_plan(
                    default_parameters,
                    &default_arguments,
                    default_value_tree,
                    decomposition_by_value,
                )?;

                if let Some((min_value, table_range)) = pool.switch_table_range(
                    &self.block_index_by_id,
                    &self.block_parameter,
                    cases,
                    default_index as u32,
                    default_copies,
                    self.block_parameter_map,
                    decomposition_by_value,
                )? {
                    Instruction {
                        operation: select_switch_table_operation(self.value_kind_map(), value),
                        data: InstructionData::SwitchTable {
                            value,
                            min: min_value,
                            table: table_range,
                            default_target: default_index as u32,
                            default_copies,
                        },
                    }
                } else {
                    let cases = pool.switch_case_range(
                        &self.block_index_by_id,
                        &self.block_parameter,
                        cases,
                        self.block_parameter_map,
                        decomposition_by_value,
                    )?;
                    Instruction {
                        operation: select_switch_operation(self.value_kind_map(), value),
                        data: InstructionData::Switch {
                            value,
                            cases,
                            default_target: default_index as u32,
                            default_copies,
                        },
                    }
                }
            }

            mir::Terminator::Trap { kind, payload } => Instruction {
                operation: InstructionOperation::Trap,
                data: InstructionData::Trap {
                    kind: *kind,
                    payload: pack_optional_value(
                        (*payload)
                            .map(|value| {
                                value.value().ok_or_else(|| Error::ConcreteMirRequired {
                                    context: "trap payload".to_string(),
                                })
                            })
                            .transpose()?,
                    ),
                },
            },

            mir::Terminator::Unreachable => Instruction {
                operation: InstructionOperation::Unreachable,
                data: InstructionData::Unreachable,
            },

            mir::Terminator::Yield { value, .. } => {
                let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "yield value".to_string(),
                })?;
                let resume_point = self
                    .yield_resume_points
                    .get(&self.block_id())
                    .copied()
                    .ok_or_else(|| Error::InvariantViolation {
                        context: format!(
                            "missing yield resume point for block: {:?}",
                            self.block_id()
                        ),
                    })?;

                Instruction {
                    operation: InstructionOperation::Yield,
                    data: InstructionData::Yield {
                        value,
                        resume_point,
                    },
                }
            }

            mir::Terminator::Throw { value } => Instruction {
                operation: InstructionOperation::Throw,
                data: InstructionData::Throw {
                    value: pack_optional_value(Some((*value).value().ok_or_else(|| {
                        Error::ConcreteMirRequired {
                            context: "throw value".to_string(),
                        }
                    })?)),
                },
            },

            mir::Terminator::Invoke { function, call, .. } => {
                let function =
                    (*function)
                        .function()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "invoke callee".to_string(),
                        })?;
                let args = pool.argument_reference_range(&call.arguments, "invoke argument")?;
                let &(normal_resume_point, unwind_resume_point) = self
                    .exceptional_call_resume_points
                    .get(&self.block_id())
                    .ok_or_else(|| Error::InvariantViolation {
                        context: format!(
                            "missing exceptional call resume points for block: {:?}",
                            self.block_id()
                        ),
                    })?;

                Instruction {
                    operation: InstructionOperation::CallBranch,
                    data: InstructionData::CallBranch {
                        function: function.id,
                        target: self.call_target(function)?,
                        arguments: args,
                        normal_resume_point,
                        unwind_resume_point,
                    },
                }
            }

            mir::Terminator::InvokeIndirect { callee, call, .. } => {
                let callee = (*callee)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "invoke indirect callee".to_string(),
                    })?;
                let arguments =
                    pool.argument_reference_range(&call.arguments, "invoke indirect argument")?;
                let &(normal_resume_point, unwind_resume_point) = self
                    .exceptional_call_resume_points
                    .get(&self.block_id())
                    .ok_or_else(|| Error::InvariantViolation {
                        context: format!(
                            "missing exceptional call resume points for block: {:?}",
                            self.block_id()
                        ),
                    })?;

                Instruction {
                    operation: InstructionOperation::CallIndirectBranch,
                    data: InstructionData::CallIndirectBranch {
                        callee,
                        arguments,
                        normal_resume_point,
                        unwind_resume_point,
                    },
                }
            }

            mir::Terminator::InvokeVirtual {
                receiver,
                slot_id,
                call,
                ..
            } => {
                let receiver = (*receiver)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "invoke virtual receiver".to_string(),
                    })?;
                let arguments =
                    pool.argument_reference_range(&call.arguments, "invoke virtual argument")?;
                let &(normal_resume_point, unwind_resume_point) = self
                    .exceptional_call_resume_points
                    .get(&self.block_id())
                    .ok_or_else(|| Error::InvariantViolation {
                        context: format!(
                            "missing exceptional call resume points for block: {:?}",
                            self.block_id()
                        ),
                    })?;

                Instruction {
                    operation: InstructionOperation::CallVirtualBranch,
                    data: InstructionData::CallVirtualBranch {
                        receiver,
                        managed_pointee: managed_pointee_type_for_value(
                            self.tree,
                            self.value_type(),
                            receiver,
                        ),
                        slot_id: slot_id.0,
                        arguments,
                        normal_resume_point,
                        unwind_resume_point,
                    },
                }
            }

            mir::Terminator::InvokeInterface {
                receiver,
                slot_id,
                call,
                ..
            } => {
                let receiver = (*receiver)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "invoke interface receiver".to_string(),
                    })?;
                let arguments =
                    pool.argument_reference_range(&call.arguments, "invoke interface argument")?;
                let &(normal_resume_point, unwind_resume_point) = self
                    .exceptional_call_resume_points
                    .get(&self.block_id())
                    .ok_or_else(|| Error::InvariantViolation {
                        context: format!(
                            "missing exceptional call resume points for block: {:?}",
                            self.block_id()
                        ),
                    })?;

                Instruction {
                    operation: InstructionOperation::CallInterfaceBranch,
                    data: InstructionData::CallInterfaceBranch {
                        receiver,
                        managed_pointee: managed_pointee_type_for_value(
                            self.tree,
                            self.value_type(),
                            receiver,
                        ),
                        slot_id: slot_id.0,
                        arguments,
                        normal_resume_point,
                        unwind_resume_point,
                    },
                }
            }

            mir::Terminator::TailCall { function, call, .. } => {
                let function =
                    (*function)
                        .function()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "tail call callee".to_string(),
                        })?;
                if function == self.function_id {
                    let args =
                        pool.argument_reference_range(&call.arguments, "tail call argument")?;
                    Instruction {
                        operation: InstructionOperation::TailCallSelf,
                        data: InstructionData::TailCallSelf {
                            entry: self.entry_block,
                            arguments: args,
                        },
                    }
                } else {
                    let callee = self.tree.get(function);
                    let arguments = call
                        .arguments
                        .iter()
                        .map(|argument| {
                            (*argument)
                                .value()
                                .ok_or_else(|| Error::ConcreteMirRequired {
                                    context: "tail call argument".to_string(),
                                })
                        })
                        .collect::<Result<Vec<_>>>()?;
                    let copies = pool.parameter_copy_range(&callee.parameters, &arguments)?;
                    let target = self.call_target(function)?;

                    Instruction {
                        operation: InstructionOperation::TailCall,
                        data: InstructionData::TailCall {
                            function: function.id,
                            target,
                            copies,
                        },
                    }
                }
            }

            mir::Terminator::TailCallIndirect { callee, call, .. } => {
                let callee = (*callee)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "tail indirect callee".to_string(),
                    })?;
                let args =
                    pool.argument_reference_range(&call.arguments, "tail indirect argument")?;
                Instruction {
                    operation: InstructionOperation::TailCallIndirect,
                    data: InstructionData::TailCallIndirect {
                        callee,
                        arguments: args,
                    },
                }
            }

            mir::Terminator::TailCallVirtual {
                receiver,
                slot_id,
                call,
                ..
            } => {
                let receiver = (*receiver)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "tail virtual receiver".to_string(),
                    })?;
                let args =
                    pool.argument_reference_range(&call.arguments, "tail virtual argument")?;
                Instruction {
                    operation: InstructionOperation::TailCallVirtual,
                    data: InstructionData::TailCallVirtual {
                        receiver,
                        managed_pointee: managed_pointee_type_for_value_kind(
                            self.value_kind_map(),
                            receiver,
                        ),
                        slot_id: slot_id.0,
                        arguments: args,
                    },
                }
            }

            mir::Terminator::TailCallInterface {
                receiver,
                slot_id,
                call,
                ..
            } => {
                let receiver = (*receiver)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "tail interface receiver".to_string(),
                    })?;
                let args =
                    pool.argument_reference_range(&call.arguments, "tail interface argument")?;
                Instruction {
                    operation: InstructionOperation::TailCallInterface,
                    data: InstructionData::TailCallInterface {
                        receiver,
                        managed_pointee: managed_pointee_type_for_value_kind(
                            self.value_kind_map(),
                            receiver,
                        ),
                        slot_id: slot_id.0,
                        arguments: args,
                    },
                }
            }
        })
    }
}
