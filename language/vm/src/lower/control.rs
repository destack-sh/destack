use destack_mir as mir;

use crate::program::{Instruction, Op};
use crate::{Error, Result};

use super::lower::BlockLowerer;
use super::op::{select_switch_op, select_switch_table_op};
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
            mir::Terminator::Return { value } => {
                let Some(value) = value else {
                    return Ok(Instruction::new(Op::ReturnVoid, 0, 0, 0, 0));
                };
                let value = value.value().ok_or_else(|| Error::MissingRepresentation {
                    context: "return value".to_string(),
                })?;

                Instruction::new(Op::Return, value.id(), 0, 0, 0)
            }

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

                Instruction::new(Op::Jump, target_index as u32, moves.start, moves.len, 0)
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
                let then_edge = pool.edge(then_index as u32, then_moves);
                let else_edge = pool.edge(else_index as u32, else_moves);

                Instruction::new(Op::BranchBool, condition.id(), then_edge.0, else_edge.0, 0)
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
                let constraint = pool.check(constraint.clone());
                let success_edge = pool.edge(success_index as u32, success_moves);
                let failure_edge = pool.edge(failure_index as u32, failure_moves);

                Instruction::new(Op::Check, constraint.0, success_edge.0, failure_edge.0, 0)
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
                let default_edge = pool.edge(default_index as u32, default_moves);

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
                        select_switch_table_op(self.value_layout_map(), value),
                        value.id(),
                        table.0,
                        default_edge.0,
                        0,
                    )
                } else {
                    let cases = pool.switch_case_range(
                        &self.block_index_by_id,
                        &self.block_parameter,
                        cases,
                    )?;
                    Instruction::new(
                        select_switch_op(self.value_layout_map(), value),
                        value.id(),
                        cases.0,
                        default_edge.0,
                        0,
                    )
                }
            }

            mir::Terminator::Trap { kind, payload } => match kind {
                mir::TrapKind::Abort => Instruction::new(Op::Abort, 0, 0, 0, 0),
                mir::TrapKind::Panic => {
                    let payload = payload.and_then(|payload| payload.value()).ok_or_else(|| {
                        Error::MissingRepresentation {
                            context: "trap panic payload".to_string(),
                        }
                    })?;

                    Instruction::new(Op::Panic, payload.id(), 0, 0, 0)
                }
            },

            mir::Terminator::Unreachable => Instruction::new(Op::Unreachable, 0, 0, 0, 0),

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

                Instruction::new(Op::Yield, value.id(), value.id(), frame_state.0, 0)
            }

            mir::Terminator::Throw { value } => {
                let value = (*value)
                    .value()
                    .ok_or_else(|| Error::MissingRepresentation {
                        context: "throw value".to_string(),
                    })?;

                Instruction::new(Op::Throw, value.id(), 0, 0, 0)
            }

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
