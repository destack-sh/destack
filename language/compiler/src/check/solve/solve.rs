use smallvec::SmallVec;

use crate::check::{
    Answer, CheckEvent, CheckState, Condition, Constraint, ConstraintId, Task, TraceConstraintKind,
    TraceOperand, TracePatternRelation, VariableId, VariableKind,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Solve collected component constraints to a fixed point.
    pub(in crate::check) fn solve(&mut self) -> CompilerResult<()> {
        if self.inference.is_probing() {
            return Err(CompilerError::Internal {
                message: "solver cannot drain work while inference probe is active".into(),
            });
        }

        self.record_event(CheckEvent::SolveStart {
            tasks: self.inference.task_count(),
            variables: self.inference.variable_count(),
        });

        let mut steps = 0;
        loop {
            let Some(task) = self.inference.pop_task()? else {
                break;
            };

            let answer = match task {
                Task::Constraint(constraint) => self.solve_constraint(constraint),
                Task::Variable(variable) => self.solve_variable(variable),
                Task::ArgumentDefault(variable) => self.solve_generic_argument_default(variable),
            }?;

            match answer {
                Answer::Ready(()) => self.inference.finish_task(task)?,
                Answer::Pending(blockers) => self.inference.block_task(task, blockers)?,
            }

            self.record_event(CheckEvent::SolveStep { step: steps });
            steps += 1;
        }

        self.record_event(CheckEvent::SolveFinish {
            iterations: steps,
            variables: self.inference.variable_count(),
        });

        Ok(())
    }

    /// Solve one active constraint once.
    fn solve_constraint(&mut self, id: ConstraintId) -> CompilerResult<Answer<()>> {
        if self.inference.is_constraint_complete(id) {
            return Ok(Answer::Ready(()));
        }

        let origin = self.inference.constraint_by_id(id).origin();
        let condition_decision = match self.inference.constraint_by_id(id).condition() {
            Condition::Always => Answer::Ready(true),
            Condition::Never => Answer::Ready(false),
            Condition::When { conditions } => {
                let predicates = conditions.iter().copied().collect::<SmallVec<[_; 2]>>();

                self.decide_condition_predicates(&predicates)?
            }
        };
        if let Answer::Pending(blockers) = condition_decision {
            return Ok(Answer::Pending(blockers));
        }
        if condition_decision == Answer::Ready(false) {
            self.inference.complete_constraint(id);

            return Ok(Answer::Ready(()));
        }

        let answer = if let Constraint::Type {
            relation,
            left,
            right,
            origin,
            condition: _,
            coercion: _,
        } = self.inference.constraint_by_id(id)
        {
            let relation = *relation;
            let left = *left;
            let right = *right;
            let origin = *origin;

            self.record_event(CheckEvent::ConstraintSolve {
                constraint: id,
                kind: TraceConstraintKind::Type(relation),
                origin,
                left: Some(TraceOperand::Type(left)),
                right: TraceOperand::Type(right),
            });

            self.solve_type_relation(origin, relation, left, right)?
        } else if let Constraint::Static {
            relation,
            left,
            right,
            origin,
            condition: _,
        } = self.inference.constraint_by_id(id)
        {
            let relation = *relation;
            let left = *left;
            let right = *right;
            let origin = *origin;

            self.record_event(CheckEvent::ConstraintSolve {
                constraint: id,
                kind: TraceConstraintKind::Static(relation),
                origin,
                left: Some(TraceOperand::Static(left)),
                right: TraceOperand::Static(right),
            });

            self.solve_static_relation(origin, relation, left, right)?
        } else if let Constraint::Pattern {
            relation,
            value,
            origin,
            condition: _,
        } = self.inference.constraint_by_id(id)
        {
            let relation = *relation;
            let value = *value;
            let origin = *origin;
            let kind = TracePatternRelation::from(&relation);

            self.record_event(CheckEvent::ConstraintSolve {
                constraint: id,
                kind: TraceConstraintKind::Pattern(kind),
                origin,
                left: None,
                right: TraceOperand::Type(value),
            });

            self.solve_pattern_relation(origin, relation, value)?
        } else {
            unreachable!("constraint variant must be type, static, or pattern")
        };

        match answer {
            Answer::Ready(()) => {
                self.inference.complete_constraint(id);

                Ok(Answer::Ready(()))
            }
            Answer::Pending(blockers) if blockers.is_empty() => {
                let constraint = self.inference.constraint_by_id(id);
                let constraint = self.dump_in_module(origin.module(), constraint);

                Err(CompilerError::Internal {
                    message: format!(
                        "check constraint {id:?} is pending without dependencies: {constraint}"
                    ),
                })
            }
            Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
        }
    }

    /// Solve one variable from its collected bounds.
    fn solve_variable(&mut self, variable: VariableId) -> CompilerResult<Answer<()>> {
        self.record_event(CheckEvent::VariableSolve { variable });

        // skip solved variables
        if self.variable_solution(variable).is_some() {
            return Ok(Answer::Ready(()));
        }

        // dispatch by variable domain
        let kind = self.variable(variable).kind;
        match kind {
            VariableKind::Type => self.solve_type_variable_from_bounds(variable),
            VariableKind::Static => self.solve_static_variable_from_bounds(variable),
        }
    }
}
