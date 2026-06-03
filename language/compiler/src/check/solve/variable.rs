use crate::CompilerResult;
use crate::check::{
    CheckState, Origin, Solution, StaticOperand, StaticSolution, StaticTerm, TypeOperand,
    TypeSolution, TypeTerm, VariableId, VariableKind,
};

use super::Progress;

/// Reduced value for one bound.
enum BoundReduction<T> {
    /// The bound contributes no term.
    Empty,
    /// The bound is waiting on more solver progress.
    Pending(Progress),
    /// The bound contributes one concrete term.
    Term {
        /// The reduced term.
        term: T,
        /// The progress made while reducing the term.
        progress: Progress,
    },
}

impl CheckState<'_> {
    /// Set one complete variable solution.
    fn solve_complete_variable(
        &mut self,
        variable: VariableId,
        solution: Solution,
    ) -> CompilerResult<Progress> {
        // wait for complete candidate solutions
        if !self.solution_is_complete(solution) {
            return Ok(Progress::Unchanged);
        }

        self.set_variable_solution(variable, solution)?;

        Ok(Progress::changed(variable))
    }

    /// Solve one type variable and return solver progress.
    pub(in crate::check) fn solve_type_variable(
        &mut self,
        variable: VariableId,
        term: TypeTerm,
    ) -> CompilerResult<Progress> {
        // wait for nested operands to solve
        if !term.referenced_variables(self).is_empty() {
            return Ok(Progress::Unchanged);
        }

        let solution = TypeSolution::Term(self.inference.push_term(term));

        self.solve_complete_variable(variable, solution.into())
    }

    /// Solve one type variable from an omitted generic default.
    pub(in crate::check) fn solve_default_type_variable(
        &mut self,
        variable: VariableId,
        default: TypeOperand,
    ) -> CompilerResult<Progress> {
        // keep real lower bounds stronger than defaults
        if self.inference.has_lower_type_bounds(variable)
            || self.variable_solution(variable).is_some()
        {
            return Ok(Progress::Unchanged);
        }

        let default = match default {
            // wait for another variable to solve first
            TypeOperand::Variable(source) if source != variable => {
                let Some(default) = self.variable_type_solution_operand(source) else {
                    return Ok(Progress::Unchanged);
                };

                default
            }
            TypeOperand::Variable(_) => return Ok(Progress::Unchanged),
            TypeOperand::Term(_) | TypeOperand::Type(_) => default,
        };
        let solution = match default {
            TypeOperand::Term(term) => TypeSolution::Term(term),
            TypeOperand::Type(ty) => TypeSolution::Type(ty),
            TypeOperand::Variable(_) => return Ok(Progress::Unchanged),
        };

        self.solve_complete_variable(variable, solution.into())
    }

    /// Solve one static variable and return solver progress.
    pub(in crate::check) fn solve_static_variable(
        &mut self,
        variable: VariableId,
        term: StaticTerm,
    ) -> CompilerResult<Progress> {
        // wait for nested operands to solve
        if !term.referenced_variables(self).is_empty() {
            return Ok(Progress::Unchanged);
        }

        let solution = StaticSolution::Term(self.inference.push_term(term));

        self.solve_complete_variable(variable, solution.into())
    }

    /// Solve one static variable from an omitted generic default.
    pub(in crate::check) fn solve_default_static_variable(
        &mut self,
        variable: VariableId,
        default: StaticOperand,
    ) -> CompilerResult<Progress> {
        // keep real lower bounds stronger than defaults
        if self.inference.has_lower_static_bounds(variable)
            || self.variable_solution(variable).is_some()
        {
            return Ok(Progress::Unchanged);
        }

        let default = match default {
            // wait for another variable to solve first
            StaticOperand::Variable(source) if source != variable => {
                let Some(default) = self.variable_static_solution_operand(source) else {
                    return Ok(Progress::Unchanged);
                };

                default
            }
            StaticOperand::Variable(_) => return Ok(Progress::Unchanged),
            StaticOperand::Term(_) | StaticOperand::Static(_) => default,
        };
        let solution = match default {
            StaticOperand::Term(term) => StaticSolution::Term(term),
            StaticOperand::Static(value) => StaticSolution::Static(value),
            StaticOperand::Variable(_) => return Ok(Progress::Unchanged),
        };

        self.solve_complete_variable(variable, solution.into())
    }

    /// Solve one variable from its collected bounds.
    pub(in crate::check) fn solve_variable_from_bounds(
        &mut self,
        variable: VariableId,
    ) -> CompilerResult<Progress> {
        // skip solved variables
        if self.variable_solution(variable).is_some() {
            return Ok(Progress::Unchanged);
        }

        // dispatch by variable domain
        let kind = self.variable(variable).kind;
        match kind {
            VariableKind::Type => self.solve_type_variable_from_bounds(variable),
            VariableKind::Static => self.solve_static_variable_from_bounds(variable),
        }
    }

    /// Solve one type variable from lower bounds.
    fn solve_type_variable_from_bounds(
        &mut self,
        variable: VariableId,
    ) -> CompilerResult<Progress> {
        let lower_bounds = self.inference.lower_type_bounds(variable);

        // wait for useful lower bounds
        if lower_bounds.is_empty() {
            return Ok(Progress::Unchanged);
        }

        // collect concrete lower bound terms
        let origin = self.variable(variable).source;
        let mut progress = Progress::Unchanged;
        let mut terms = Vec::with_capacity(lower_bounds.len());
        for bound in lower_bounds {
            match self.reduce_type_bound(origin, variable, bound)? {
                BoundReduction::Empty => {}
                BoundReduction::Pending(pending) => return Ok(pending),
                BoundReduction::Term {
                    term,
                    progress: reduced,
                } => {
                    progress = progress.merge(reduced);
                    terms.push(term);
                }
            }
        }

        // solve from the best common concrete term
        if terms.is_empty() {
            return Ok(Progress::Unchanged);
        }
        let term = self.reduce_best_common_terms(variable.module, terms)?;

        Ok(progress.merge(self.solve_type_variable(variable, term)?))
    }

    /// Reduce one lower type bound.
    fn reduce_type_bound(
        &mut self,
        origin: Origin,
        variable: VariableId,
        bound: TypeOperand,
    ) -> CompilerResult<BoundReduction<TypeTerm>> {
        // resolve the bound operand
        let term = match bound {
            TypeOperand::Variable(bound) if bound == variable => return Ok(BoundReduction::Empty),
            TypeOperand::Variable(bound) => {
                let Some(term) = self.type_solution(bound)? else {
                    return Ok(BoundReduction::Pending(Progress::Unchanged));
                };

                term
            }
            TypeOperand::Term(term) => self.inference.term(term).clone(),
            TypeOperand::Type(ty) => TypeTerm::Type(ty),
        };

        // reduce the bound term
        self.reduce_type_bound_term(origin, term)
    }

    /// Reduce one solved lower type bound.
    fn reduce_type_bound_term(
        &mut self,
        origin: Origin,
        term: TypeTerm,
    ) -> CompilerResult<BoundReduction<TypeTerm>> {
        let reduction = self.reduce_type_term(origin, &term)?;
        let progress = reduction.progress;
        let Some(term) = reduction.value else {
            return Ok(BoundReduction::Pending(progress));
        };

        Ok(BoundReduction::Term { term, progress })
    }

    /// Solve one static variable from its bounds.
    fn solve_static_variable_from_bounds(
        &mut self,
        variable: VariableId,
    ) -> CompilerResult<Progress> {
        let lower_bounds = self.inference.lower_static_bounds(variable);
        let upper_bounds = self.inference.upper_static_bounds(variable);

        // wait for useful bounds
        if lower_bounds.is_empty() && upper_bounds.is_empty() {
            return Ok(Progress::Unchanged);
        }

        // collect lower bound terms
        let origin = self.variable(variable).source;
        let mut progress = Progress::Unchanged;
        let mut lower_terms = Vec::with_capacity(lower_bounds.len());
        for bound in lower_bounds.iter().copied() {
            match self.reduce_static_bound(origin, variable, bound)? {
                BoundReduction::Empty => {}
                BoundReduction::Pending(pending) => return Ok(pending),
                BoundReduction::Term {
                    term,
                    progress: reduced,
                } => {
                    progress = progress.merge(reduced);
                    lower_terms.push(term);
                }
            }
        }

        // collect upper bound terms
        let mut upper_terms = Vec::with_capacity(upper_bounds.len());
        for bound in upper_bounds.iter().copied() {
            match self.reduce_static_bound(origin, variable, bound)? {
                BoundReduction::Empty => {}
                BoundReduction::Pending(pending) => return Ok(pending),
                BoundReduction::Term {
                    term,
                    progress: reduced,
                } => {
                    progress = progress.merge(reduced);
                    upper_terms.push(term);
                }
            }
        }

        // solve from the static domain
        let Some(term) =
            self.reduce_static_bounds(&lower_bounds, &upper_bounds, &lower_terms, &upper_terms)
        else {
            return Ok(Progress::Unchanged);
        };

        Ok(progress.merge(self.solve_static_variable(variable, term)?))
    }

    /// Reduce one static bound.
    fn reduce_static_bound(
        &mut self,
        origin: Origin,
        variable: VariableId,
        bound: StaticOperand,
    ) -> CompilerResult<BoundReduction<StaticTerm>> {
        // resolve the bound operand
        let term = match bound {
            StaticOperand::Variable(bound) if bound == variable => {
                return Ok(BoundReduction::Empty);
            }
            StaticOperand::Variable(bound) => {
                let Some(term) = self.static_solution(bound)? else {
                    return Ok(BoundReduction::Pending(Progress::Unchanged));
                };

                term
            }
            StaticOperand::Term(term) => self.inference.term(term).clone(),
            StaticOperand::Static(value) => StaticTerm::Static(value),
        };

        // reduce the bound term
        self.reduce_static_bound_term(origin, term)
    }

    /// Reduce one solved static bound.
    fn reduce_static_bound_term(
        &mut self,
        origin: Origin,
        term: StaticTerm,
    ) -> CompilerResult<BoundReduction<StaticTerm>> {
        // reduce computed static terms
        let Some(term) = self.reduce_static_term(origin, &term)? else {
            return Ok(BoundReduction::Pending(Progress::Unchanged));
        };

        Ok(BoundReduction::Term {
            term,
            progress: Progress::Unchanged,
        })
    }

    /// Return a solved variable value.
    pub(in crate::check) fn variable_solution(&self, variable: VariableId) -> Option<Solution> {
        self.inference.variable_solution(variable)
    }

    /// Return the type solution for one variable.
    pub(in crate::check) fn type_solution(
        &self,
        variable: VariableId,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(operand) = self.variable_type_solution_operand(variable) else {
            return Ok(None);
        };
        let term = match operand {
            TypeOperand::Variable(_) => return Ok(None),
            TypeOperand::Term(term) => self.inference.term(term).clone(),
            TypeOperand::Type(ty) => TypeTerm::Type(ty),
        };

        Ok(Some(term))
    }

    /// Return the static solution for one variable.
    pub(in crate::check) fn static_solution(
        &self,
        variable: VariableId,
    ) -> CompilerResult<Option<StaticTerm>> {
        let Some(operand) = self.variable_static_solution_operand(variable) else {
            return Ok(None);
        };
        let term = match operand {
            StaticOperand::Variable(_) => return Ok(None),
            StaticOperand::Term(term) => self.inference.term(term).clone(),
            StaticOperand::Static(value) => StaticTerm::Static(value),
        };

        Ok(Some(term))
    }

    /// Return whether one candidate solution contains no inference variables.
    fn solution_is_complete(&self, solution: Solution) -> bool {
        match solution {
            Solution::Type(TypeSolution::Term(term)) => self
                .inference
                .term(term)
                .referenced_variables(self)
                .is_empty(),
            Solution::Static(StaticSolution::Term(term)) => self
                .inference
                .term(term)
                .referenced_variables(self)
                .is_empty(),
            Solution::Type(TypeSolution::Type(_)) | Solution::Static(StaticSolution::Static(_)) => {
                true
            }
        }
    }
}
