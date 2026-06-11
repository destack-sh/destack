use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, BoundSide, CheckEvent, CheckState, Dependency, GenericParameterId, Origin,
    StaticOperand, StaticSolution, TraceOperand, TypeOperand, TypeOperationTerm, TypeSolution,
    TypeTerm,
};

/// Component-valid id for one check variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(in crate::check) struct VariableId {
    /// The module that produced the variable.
    pub(in crate::check) module: ModuleId,
    /// The variable index inside the checked component.
    pub(in crate::check) index: u32,
}

impl VariableId {
    /// Create one variable id.
    pub(in crate::check) fn new(module: ModuleId, index: u32) -> Self {
        Self { module, index }
    }
}

/// One solver variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct Variable {
    /// The variable id.
    pub(in crate::check) id: VariableId,
    /// The variable kind.
    pub(in crate::check) kind: VariableKind,
    /// The source that produced the variable.
    pub(in crate::check) source: Origin,
}

impl Variable {
    /// Create one unsolved variable.
    pub(in crate::check) fn new(id: VariableId, kind: VariableKind, source: Origin) -> Self {
        Self { id, kind, source }
    }
}

/// The value space of one check variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum VariableKind {
    /// Type variable.
    Type,
    /// Static value variable.
    Static,
}

/// Default value for one omitted generic argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum GenericArgumentDefault {
    /// Type default.
    Type {
        /// The generic parameter declaring the default.
        parameter: GenericParameterId,
        /// The default value.
        value: TypeOperand,
    },
    /// Static default.
    Static {
        /// The generic parameter declaring the default.
        parameter: GenericParameterId,
        /// The default value.
        value: StaticOperand,
    },
}

impl GenericArgumentDefault {
    /// Create one type argument default.
    pub(in crate::check) fn r#type(parameter: GenericParameterId, value: TypeOperand) -> Self {
        Self::Type { parameter, value }
    }

    /// Create one static argument default.
    pub(in crate::check) fn r#static(parameter: GenericParameterId, value: StaticOperand) -> Self {
        Self::Static { parameter, value }
    }

    /// Return the generic parameter declaring this default.
    pub(in crate::check) fn parameter(self) -> GenericParameterId {
        match self {
            Self::Type { parameter, .. } | Self::Static { parameter, .. } => parameter,
        }
    }
}

impl CheckState<'_> {
    /// Solve one type variable from its current bounds.
    pub(in crate::check) fn solve_type_variable(
        &mut self,
        variable: VariableId,
        term: TypeTerm,
    ) -> CompilerResult<Answer<()>> {
        let solution = TypeSolution::Term(self.inference.push_term(term));

        self.set_variable_solution(variable, solution.into())?;

        Ok(Answer::Ready(()))
    }

    /// Solve one type variable from one operand.
    fn solve_type_variable_operand(
        &mut self,
        variable: VariableId,
        operand: TypeOperand,
    ) -> CompilerResult<Answer<()>> {
        let solution = match operand {
            TypeOperand::Variable(alias) => {
                return Ok(Answer::pending([Dependency::Variable(alias)]));
            }
            TypeOperand::Term(term) => TypeSolution::Term(term),
            TypeOperand::Type(ty) => TypeSolution::Type(ty),
        };

        self.set_variable_solution(variable, solution.into())?;

        Ok(Answer::Ready(()))
    }

    /// Solve one static variable from its current bounds.
    pub(in crate::check) fn solve_static_variable(
        &mut self,
        variable: VariableId,
        operand: StaticOperand,
    ) -> CompilerResult<Answer<()>> {
        let solution = match operand {
            StaticOperand::Variable(alias) => {
                return Ok(Answer::pending([Dependency::Variable(alias)]));
            }
            StaticOperand::Term(term) => StaticSolution::Term(term),
            StaticOperand::Static(value) => StaticSolution::Static(value),
        };

        self.set_variable_solution(variable, solution.into())?;

        Ok(Answer::Ready(()))
    }

    /// Solve one variable from its generic argument default.
    pub(in crate::check) fn solve_generic_argument_default(
        &mut self,
        variable: VariableId,
    ) -> CompilerResult<Answer<()>> {
        let Some(default) = self.inference.generic_argument_default(variable) else {
            return Ok(Answer::Ready(()));
        };

        let answer = match default {
            GenericArgumentDefault::Type { value, .. } => {
                self.solve_type_argument_default(variable, value)?
            }
            GenericArgumentDefault::Static { value, .. } => {
                self.solve_static_argument_default(variable, value)?
            }
        };

        Ok(answer)
    }

    /// Solve one type variable from its default.
    fn solve_type_argument_default(
        &mut self,
        variable: VariableId,
        default: TypeOperand,
    ) -> CompilerResult<Answer<()>> {
        if self.variable_solution(variable).is_some()
            || self.inference.has_lower_type_bounds(variable)
        {
            return Ok(Answer::Ready(()));
        }

        let origin = self.variable(variable).source;
        let default = match self.reduce_type_bound(origin, variable, default)? {
            Answer::Ready(default) => default,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let Ok(solution) = TypeSolution::try_from(default) else {
            return Ok(Answer::Ready(()));
        };

        self.set_variable_solution(variable, solution.into())?;

        Ok(Answer::Ready(()))
    }

    /// Solve one static variable from its default.
    fn solve_static_argument_default(
        &mut self,
        variable: VariableId,
        default: StaticOperand,
    ) -> CompilerResult<Answer<()>> {
        if self.variable_solution(variable).is_some()
            || self.inference.has_lower_static_bounds(variable)
        {
            return Ok(Answer::Ready(()));
        }

        let default = match default {
            StaticOperand::Term(_) | StaticOperand::Variable(_) => {
                let origin = self.variable(variable).source;
                match self.reduce_static_bound(origin, variable, default)? {
                    Answer::Ready(default) => default,
                    Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                }
            }
            StaticOperand::Static(_) => default,
        };
        let Ok(solution) = StaticSolution::try_from(default) else {
            return Ok(Answer::Ready(()));
        };

        self.set_variable_solution(variable, solution.into())?;

        Ok(Answer::Ready(()))
    }

    /// Solve one type variable from lower bounds.
    pub(in crate::check) fn solve_type_variable_from_bounds(
        &mut self,
        variable: VariableId,
    ) -> CompilerResult<Answer<()>> {
        let lower_bounds = self.inference.lower_type_bounds(variable);
        let upper_bounds = self.inference.upper_type_bounds(variable);

        // solve contextual variables from upper bounds
        if lower_bounds.is_empty() {
            return self.solve_type_variable_from_upper_bounds(variable, &upper_bounds);
        }

        // collect produced operands
        let origin = self.variable(variable).source;
        let mut operands = SmallVec::<[TypeOperand; 4]>::new();
        let mut blockers = SmallVec::<[Dependency; 2]>::new();

        for bound in lower_bounds {
            let operand = match self.reduce_type_bound(origin, variable, bound)? {
                Answer::Ready(operand) => operand,
                Answer::Pending(dependencies) => {
                    self.record_event(CheckEvent::BoundPending {
                        variable,
                        kind: VariableKind::Type,
                        side: BoundSide::Lower,
                        value: TraceOperand::Type(bound),
                    });

                    blockers.extend(dependencies);

                    continue;
                }
            };
            if matches!(operand, TypeOperand::Variable(bound) if bound == variable) {
                continue;
            }

            operands.push(operand);
        }

        // wait for all produced bounds
        if !blockers.is_empty() {
            return Ok(Answer::pending(blockers));
        }
        if operands.is_empty() {
            return self.solve_type_variable_from_upper_bounds(variable, &upper_bounds);
        }
        let term = match operands.as_slice() {
            [operand] => {
                return self.solve_type_variable_operand(variable, *operand);
            }
            _ => {
                let operation = self.inference.push_term(TypeOperationTerm::BestCommon {
                    elements: operands.into_vec(),
                });

                TypeTerm::Operation(operation)
            }
        };

        self.solve_type_variable(variable, term)
    }

    /// Solve one type variable from contextual upper bounds.
    fn solve_type_variable_from_upper_bounds(
        &mut self,
        variable: VariableId,
        upper_bounds: &[TypeOperand],
    ) -> CompilerResult<Answer<()>> {
        if upper_bounds.is_empty() {
            return Ok(Answer::Ready(()));
        }

        let origin = self.variable(variable).source;
        let mut operands = SmallVec::<[TypeOperand; 4]>::new();
        let mut blockers = SmallVec::<[Dependency; 2]>::new();

        for bound in upper_bounds.iter().copied() {
            let operand = match self.reduce_type_bound(origin, variable, bound)? {
                Answer::Ready(operand) => operand,
                Answer::Pending(dependencies) => {
                    self.record_event(CheckEvent::BoundPending {
                        variable,
                        kind: VariableKind::Type,
                        side: BoundSide::Upper,
                        value: TraceOperand::Type(bound),
                    });

                    blockers.extend(dependencies);

                    continue;
                }
            };
            if matches!(operand, TypeOperand::Variable(bound) if bound == variable) {
                continue;
            }

            operands.push(operand);
        }

        // wait for all contextual bounds before choosing a contextual solution
        if !blockers.is_empty() {
            return Ok(Answer::pending(blockers));
        }

        let term = match operands.as_slice() {
            [] => return Ok(Answer::Ready(())),
            [operand] if matches!(operand, TypeOperand::Variable(_)) => {
                return self.solve_type_variable_operand(variable, *operand);
            }
            [operand] => {
                return self.solve_type_variable_operand(variable, *operand);
            }
            _ => TypeTerm::Intersection {
                elements: operands.into_vec(),
            },
        };

        self.solve_type_variable(variable, term)
    }

    /// Reduce one type bound.
    pub(in crate::check) fn reduce_type_bound(
        &mut self,
        origin: Origin,
        variable: VariableId,
        bound: TypeOperand,
    ) -> CompilerResult<Answer<TypeOperand>> {
        // resolve the bound operand
        let operand = match bound {
            TypeOperand::Variable(bound) if bound == variable => {
                return Ok(Answer::pending([Dependency::Variable(bound)]));
            }
            TypeOperand::Variable(bound) => {
                let Some(operand) = self.solved_type_operand(bound) else {
                    return Ok(Answer::pending([Dependency::Variable(bound)]));
                };

                operand
            }
            TypeOperand::Term(_) | TypeOperand::Type(_) => bound,
        };

        // reduce the bound operand
        let operand = match self.reduce_type_operand(origin, operand)? {
            Answer::Ready(operand) => operand,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        if matches!(operand, TypeOperand::Variable(bound) if bound == variable) {
            return Ok(Answer::Ready(operand));
        }

        Ok(Answer::Ready(operand))
    }

    /// Solve one static variable from its bounds.
    pub(in crate::check) fn solve_static_variable_from_bounds(
        &mut self,
        variable: VariableId,
    ) -> CompilerResult<Answer<()>> {
        let lower_bounds = self.inference.lower_static_bounds(variable);
        let upper_bounds = self.inference.upper_static_bounds(variable);

        // wait for useful bounds
        if lower_bounds.is_empty() && upper_bounds.is_empty() {
            return Ok(Answer::Ready(()));
        }

        // collect lower bound terms
        let origin = self.variable(variable).source;
        let mut ready_lower_bounds = SmallVec::<[StaticOperand; 4]>::new();
        let mut lower_operands = SmallVec::<[StaticOperand; 4]>::new();
        let mut blockers = SmallVec::<[Dependency; 2]>::new();

        for bound in lower_bounds {
            if matches!(bound, StaticOperand::Variable(bound) if bound == variable) {
                continue;
            }

            let operand = match self.reduce_static_bound(origin, variable, bound)? {
                Answer::Ready(operand) => operand,
                Answer::Pending(dependencies) => {
                    self.record_event(CheckEvent::BoundPending {
                        variable,
                        kind: VariableKind::Static,
                        side: BoundSide::Lower,
                        value: TraceOperand::Static(bound),
                    });

                    blockers.extend(dependencies);

                    continue;
                }
            };

            ready_lower_bounds.push(bound);
            lower_operands.push(operand);
        }

        // collect upper bound terms
        let mut ready_upper_bounds = SmallVec::<[StaticOperand; 4]>::new();
        let mut upper_operands = SmallVec::<[StaticOperand; 4]>::new();
        for bound in upper_bounds {
            if matches!(bound, StaticOperand::Variable(bound) if bound == variable) {
                continue;
            }

            let operand = match self.reduce_static_bound(origin, variable, bound)? {
                Answer::Ready(operand) => operand,
                Answer::Pending(dependencies) => {
                    self.record_event(CheckEvent::BoundPending {
                        variable,
                        kind: VariableKind::Static,
                        side: BoundSide::Upper,
                        value: TraceOperand::Static(bound),
                    });

                    blockers.extend(dependencies);

                    continue;
                }
            };

            ready_upper_bounds.push(bound);
            upper_operands.push(operand);
        }

        // wait for blocked reductions
        if !blockers.is_empty() {
            return Ok(Answer::pending(blockers));
        }

        // solve from the static domain
        let Some(operand) = self.static_bound_solution(
            &ready_lower_bounds,
            &ready_upper_bounds,
            &lower_operands,
            &upper_operands,
        ) else {
            return Ok(Answer::Ready(()));
        };

        self.solve_static_variable(variable, operand)
    }

    /// Reduce one static bound.
    fn reduce_static_bound(
        &mut self,
        origin: Origin,
        variable: VariableId,
        bound: StaticOperand,
    ) -> CompilerResult<Answer<StaticOperand>> {
        let operand = match bound {
            StaticOperand::Variable(bound) if bound == variable => {
                return Ok(Answer::pending([Dependency::Variable(bound)]));
            }
            StaticOperand::Variable(bound) => {
                let Some(operand) = self.resolved_static_variable(bound) else {
                    return Ok(Answer::pending([Dependency::Variable(bound)]));
                };

                operand
            }
            StaticOperand::Term(_) | StaticOperand::Static(_) => bound,
        };

        self.reduce_static_operand(origin, operand)
    }
}
