use destack_mir as mir;

use crate::program::{
    Branch, Check, Instruction, Jump, Opcode, Return, Switch, TableSwitch, Throw, Trap,
    Unreachable, Yield,
};
use crate::{Error, Result};

use super::lower::BlockLowerer;
use super::opcode::{select_branch_opcode, select_switch_opcode, select_switch_table_opcode};
use super::pool::Pool;

impl<'a> BlockLowerer<'a> {
    /// Convert a MIR terminator to lowered interpreter form.
    pub(super) fn lower_terminator(
        &self,
        term: &mir::Terminator,
        pool: &mut Pool<'_>,
    ) -> Result<Instruction> {
        Ok(match term {
            mir::Terminator::Error => {
                return Err(Error::MissingRepresentation {
                    context: "terminator".to_string(),
                });
            }
            mir::Terminator::Return { value } => Instruction::new(
                Opcode::Return,
                Return {
                    value: (*value)
                        .map(|value| {
                            value.value().ok_or_else(|| Error::MissingRepresentation {
                                context: "return value".to_string(),
                            })
                        })
                        .transpose()?,
                },
            ),

            mir::Terminator::Jump { target } => {
                let target_block =
                    (target.block)
                        .block()
                        .ok_or_else(|| Error::MissingRepresentation {
                            context: "jump target".to_string(),
                        })?;
                let arguments = target
                    .arguments
                    .iter()
                    .map(|argument| {
                        (*argument)
                            .value()
                            .ok_or_else(|| Error::MissingRepresentation {
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
                let moves = pool.edge_move_plan(target_parameters, &arguments)?;

                Instruction::new(
                    Opcode::Jump,
                    Jump {
                        target: target_index as u32,
                        moves,
                    },
                )
            }

            mir::Terminator::Branch {
                condition,
                then_target,
                else_target,
            } => {
                let condition =
                    (*condition)
                        .value()
                        .ok_or_else(|| Error::MissingRepresentation {
                            context: "branch condition".to_string(),
                        })?;
                let then_target_block =
                    (then_target.block)
                        .block()
                        .ok_or_else(|| Error::MissingRepresentation {
                            context: "branch then target".to_string(),
                        })?;
                let else_target_block =
                    (else_target.block)
                        .block()
                        .ok_or_else(|| Error::MissingRepresentation {
                            context: "branch else target".to_string(),
                        })?;
                let then_arguments = then_target
                    .arguments
                    .iter()
                    .map(|argument| {
                        (*argument)
                            .value()
                            .ok_or_else(|| Error::MissingRepresentation {
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
                            .ok_or_else(|| Error::MissingRepresentation {
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
                let then_moves = pool.edge_move_plan(then_parameters, &then_arguments)?;
                let else_moves = pool.edge_move_plan(else_parameters, &else_arguments)?;

                Instruction::new(
                    select_branch_opcode(self.value_layout_map(), condition),
                    Branch {
                        condition,
                        then_target: then_index as u32,
                        then_moves,
                        else_target: else_index as u32,
                        else_moves,
                    },
                )
            }

            mir::Terminator::Check {
                constraint,
                success,
                failure,
            } => {
                let success_block =
                    (success.block)
                        .block()
                        .ok_or_else(|| Error::MissingRepresentation {
                            context: "check success target".to_string(),
                        })?;
                let failure_block =
                    (failure.block)
                        .block()
                        .ok_or_else(|| Error::MissingRepresentation {
                            context: "check failure target".to_string(),
                        })?;
                let success_arguments = success
                    .arguments
                    .iter()
                    .map(|argument| {
                        (*argument)
                            .value()
                            .ok_or_else(|| Error::MissingRepresentation {
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
                            .ok_or_else(|| Error::MissingRepresentation {
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
                let success_moves = pool.edge_move_plan(success_parameters, &success_arguments)?;
                let failure_moves = pool.edge_move_plan(failure_parameters, &failure_arguments)?;

                Instruction::new(
                    Opcode::Check,
                    Check {
                        constraint: pool.check(constraint.clone()),
                        then_target: success_index as u32,
                        then_moves: success_moves,
                        else_target: failure_index as u32,
                        else_moves: failure_moves,
                    },
                )
            }

            mir::Terminator::Switch {
                value,
                cases,
                default,
            } => {
                let value = (*value)
                    .value()
                    .ok_or_else(|| Error::MissingRepresentation {
                        context: "switch value".to_string(),
                    })?;
                let default_block =
                    (default.block)
                        .block()
                        .ok_or_else(|| Error::MissingRepresentation {
                            context: "switch default target".to_string(),
                        })?;
                let default_arguments = default
                    .arguments
                    .iter()
                    .map(|argument| {
                        (*argument)
                            .value()
                            .ok_or_else(|| Error::MissingRepresentation {
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
                let default_moves = pool.edge_move_plan(default_parameters, &default_arguments)?;

                let is_word = self
                    .value_type_for_value(value)
                    .ok()
                    .and_then(|ty| self.layout_for_type(ty).ok())
                    .is_some_and(|layout| layout.is_word());
                if is_word
                    && let Some(table) = pool.switch_table_range(
                        &self.block_index_by_id,
                        &self.block_parameter,
                        cases,
                        default_index as u32,
                        default_moves,
                    )?
                {
                    Instruction::new(
                        select_switch_table_opcode(self.value_layout_map(), value),
                        TableSwitch {
                            value,
                            table,
                            default_target: default_index as u32,
                            default_moves,
                        },
                    )
                } else {
                    let cases = pool.switch_case_range(
                        &self.block_index_by_id,
                        &self.block_parameter,
                        cases,
                    )?;
                    Instruction::new(
                        select_switch_opcode(self.value_layout_map(), value),
                        Switch {
                            value,
                            cases,
                            default_target: default_index as u32,
                            default_moves,
                        },
                    )
                }
            }

            mir::Terminator::Trap { kind, payload } => Instruction::new(
                Opcode::Trap,
                Trap {
                    kind: *kind,
                    payload: (*payload)
                        .map(|value| {
                            value.value().ok_or_else(|| Error::MissingRepresentation {
                                context: "trap payload".to_string(),
                            })
                        })
                        .transpose()?,
                },
            ),

            mir::Terminator::Unreachable => Instruction::new(Opcode::Unreachable, Unreachable),

            mir::Terminator::Yield { value, .. } => {
                let value = (*value)
                    .value()
                    .ok_or_else(|| Error::MissingRepresentation {
                        context: "yield value".to_string(),
                    })?;
                let frame_state = self
                    .yield_frame_states
                    .get(&self.block_id())
                    .copied()
                    .ok_or_else(|| Error::InvariantViolation {
                        context: format!(
                            "missing yield frame state for block: {:?}",
                            self.block_id()
                        ),
                    })?;

                Instruction::new(
                    Opcode::Yield,
                    Yield {
                        value,
                        source: value,
                        frame_state,
                    },
                )
            }

            mir::Terminator::Throw { value } => Instruction::new(
                Opcode::Throw,
                Throw {
                    value: (*value)
                        .value()
                        .ok_or_else(|| Error::MissingRepresentation {
                            context: "throw value".to_string(),
                        })?,
                },
            ),

            mir::Terminator::Invoke { .. }
            | mir::Terminator::InvokeIndirect { .. }
            | mir::Terminator::InvokeVirtual { .. }
            | mir::Terminator::InvokeInterface { .. }
            | mir::Terminator::TailCall { .. }
            | mir::Terminator::TailCallIndirect { .. }
            | mir::Terminator::TailCallVirtual { .. }
            | mir::Terminator::TailCallInterface { .. } => {
                self.lower_call_terminator(term, pool)?
            }
        })
    }
}
