use destack_mir as mir;

use crate::Word;

use crate::program::{Instruction, Opcode, Operands, pack_optional_value};
use crate::{Error, Result};

use super::access::{interface_table_field_for_receiver, virtual_table_field_for_receiver};
use super::lower::BlockLowerer;
use super::opcode::{
    select_branch_opcode, select_compare_branch_const_opcode, select_compare_branch_opcode,
    select_switch_opcode, select_switch_table_opcode, swap_compare_operator,
};
use super::pool::Pool;
use super::repr::{
    heap_pointee_type_for_value, heap_pointee_type_for_value_repr, pointer_class_for_value,
};

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
                        const_value = Some(Word::from(value));
                        pop_const = true;
                    } else if destination == left {
                        const_value = Some(Word::from(value));
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
        let then_moves = pool.move_range(then_parameters, &then_arguments);
        let else_moves = pool.move_range(else_parameters, &else_arguments);

        if let Some(right_const) = const_value {
            let opcode = select_compare_branch_const_opcode(operator);
            return Some(Instruction {
                opcode,
                operands: Operands::CompareAndBranchConst {
                    left: left_value,
                    right_const,
                    operator,
                    then_target: then_index as u32,
                    then_moves,
                    else_target: else_index as u32,
                    else_moves,
                },
            });
        }

        let opcode = select_compare_branch_opcode(operator);
        Some(Instruction {
            opcode,
            operands: Operands::CompareAndBranch {
                left,
                right,
                operator,
                then_target: then_index as u32,
                then_moves,
                else_target: else_index as u32,
                else_moves,
            },
        })
    }

    /// Convert a MIR terminator to lowered interpreter form.
    pub(super) fn lower_terminator(
        &self,
        term: &mir::Terminator,
        pool: &mut Pool,
    ) -> Result<Instruction> {
        Ok(match term {
            mir::Terminator::Error => {
                return Err(Error::ConcreteMirRequired {
                    context: "terminator".to_string(),
                });
            }
            mir::Terminator::Return { value } => Instruction {
                opcode: Opcode::Return,
                operands: Operands::Return {
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
                let moves = pool.edge_move_plan(target_parameters, &arguments);

                Instruction {
                    opcode: Opcode::Jump,
                    operands: Operands::Jump {
                        target: target_index as u32,
                        moves,
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
                let then_moves = pool.edge_move_plan(then_parameters, &then_arguments);
                let else_moves = pool.edge_move_plan(else_parameters, &else_arguments);

                Instruction {
                    opcode: select_branch_opcode(self.value_repr_map(), condition),
                    operands: Operands::Branch {
                        condition,
                        then_target: then_index as u32,
                        then_moves,
                        else_target: else_index as u32,
                        else_moves,
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
                let success_moves = pool.edge_move_plan(success_parameters, &success_arguments);
                let failure_moves = pool.edge_move_plan(failure_parameters, &failure_arguments);

                Instruction {
                    opcode: Opcode::Check,
                    operands: Operands::Check {
                        constraint: constraint.clone(),
                        then_target: success_index as u32,
                        then_moves: success_moves,
                        else_target: failure_index as u32,
                        else_moves: failure_moves,
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
                let default_moves = pool.edge_move_plan(default_parameters, &default_arguments);

                if let Some((min_value, table_range)) = pool.switch_table_range(
                    &self.block_index_by_id,
                    &self.block_parameter,
                    cases,
                    default_index as u32,
                    default_moves,
                )? {
                    Instruction {
                        opcode: select_switch_table_opcode(self.value_repr_map(), value),
                        operands: Operands::SwitchTable {
                            value,
                            min: min_value,
                            table: table_range,
                            default_target: default_index as u32,
                            default_moves,
                        },
                    }
                } else {
                    let cases = pool.switch_case_range(
                        &self.block_index_by_id,
                        &self.block_parameter,
                        cases,
                    )?;
                    Instruction {
                        opcode: select_switch_opcode(self.value_repr_map(), value),
                        operands: Operands::Switch {
                            value,
                            cases,
                            default_target: default_index as u32,
                            default_moves,
                        },
                    }
                }
            }

            mir::Terminator::Trap { kind, payload } => Instruction {
                opcode: Opcode::Trap,
                operands: Operands::Trap {
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
                opcode: Opcode::Unreachable,
                operands: Operands::Unreachable,
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
                    opcode: Opcode::Yield,
                    operands: Operands::Yield {
                        value,
                        source: value,
                        resume_point,
                    },
                }
            }

            mir::Terminator::Throw { value } => Instruction {
                opcode: Opcode::Throw,
                operands: Operands::Throw {
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
                    opcode: Opcode::CallBranch,
                    operands: Operands::CallBranch {
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
                    opcode: Opcode::CallIndirectBranch,
                    operands: Operands::CallIndirectBranch {
                        callee,
                        arguments,
                        normal_resume_point,
                        unwind_resume_point,
                    },
                }
            }

            mir::Terminator::InvokeVirtual {
                receiver,
                slot_id: method,
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
                    opcode: Opcode::CallVirtualBranch,
                    operands: Operands::CallVirtualBranch {
                        receiver,
                        table_field: virtual_table_field_for_receiver(
                            self.layouts(),
                            heap_pointee_type_for_value(self.tree, self.value_type(), receiver),
                            pointer_class_for_value(self.value_repr_map(), receiver),
                        ),
                        method_index: method.0,
                        arguments,
                        normal_resume_point,
                        unwind_resume_point,
                    },
                }
            }

            mir::Terminator::InvokeInterface {
                receiver,
                slot_id: method,
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
                    opcode: Opcode::CallInterfaceBranch,
                    operands: Operands::CallInterfaceBranch {
                        receiver,
                        table_field: interface_table_field_for_receiver(
                            self.layouts(),
                            heap_pointee_type_for_value(self.tree, self.value_type(), receiver),
                            pointer_class_for_value(self.value_repr_map(), receiver),
                        ),
                        method_index: method.0,
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
                        opcode: Opcode::TailCallSelf,
                        operands: Operands::TailCallSelf {
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
                    let moves = pool.parameter_move_range(&callee.parameters, &arguments)?;
                    let target = self.call_target(function)?;

                    Instruction {
                        opcode: Opcode::TailCall,
                        operands: Operands::TailCall {
                            function: function.id,
                            target,
                            moves,
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
                    opcode: Opcode::TailCallIndirect,
                    operands: Operands::TailCallIndirect {
                        callee,
                        arguments: args,
                    },
                }
            }

            mir::Terminator::TailCallVirtual {
                receiver,
                slot_id: method,
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
                    opcode: Opcode::TailCallVirtual,
                    operands: Operands::TailCallVirtual {
                        receiver,
                        table_field: virtual_table_field_for_receiver(
                            self.layouts(),
                            heap_pointee_type_for_value_repr(self.value_repr_map(), receiver),
                            pointer_class_for_value(self.value_repr_map(), receiver),
                        ),
                        method_index: method.0,
                        arguments: args,
                    },
                }
            }

            mir::Terminator::TailCallInterface {
                receiver,
                slot_id: method,
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
                    opcode: Opcode::TailCallInterface,
                    operands: Operands::TailCallInterface {
                        receiver,
                        table_field: interface_table_field_for_receiver(
                            self.layouts(),
                            heap_pointee_type_for_value_repr(self.value_repr_map(), receiver),
                            pointer_class_for_value(self.value_repr_map(), receiver),
                        ),
                        method_index: method.0,
                        arguments: args,
                    },
                }
            }
        })
    }
}
