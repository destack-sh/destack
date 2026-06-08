use std::ops::ControlFlow;

use crate::CompilerResult;
use crate::check::{
    CheckState, Origin, Solution, StaticOperand, StaticSolution, StaticTerm, TypeOperand,
    TypeSolution, TypeTerm, VariableId,
};

/// Contribution from one bound.
enum BoundReduction<T> {
    /// The bound contributes no value.
    Empty,
    /// The bound is waiting on more solver input.
    Pending,
    /// The bound contributes one value.
    Value(T),
}

impl<T> BoundReduction<T> {
    /// Add this reduction to one bound collection.
    fn collect_into(self, values: &mut Vec<T>) -> ControlFlow<()> {
        match self {
            Self::Empty => ControlFlow::Continue(()),
            Self::Pending => ControlFlow::Break(()),
            Self::Value(value) => {
                values.push(value);

                ControlFlow::Continue(())
            }
        }
    }
}

impl CheckState<'_> {
    /// Set one complete variable solution.
    fn solve_complete_variable(
        &mut self,
        variable: VariableId,
        solution: Solution,
    ) -> CompilerResult<()> {
        // wait for complete candidate solutions
        if !self.solution_is_complete(solution) {
            return Ok(());
        }

        self.set_variable_solution(variable, solution)?;

        Ok(())
    }

    /// Solve one type variable from its current bounds.
    pub(in crate::check) fn solve_type_variable(
        &mut self,
        variable: VariableId,
        term: TypeTerm,
    ) -> CompilerResult<()> {
        // wait for nested operands to solve
        if !term.referenced_variables(self).is_empty() {
            return Ok(());
        }

        let solution = TypeSolution::Term(self.inference.push_term(term));

        self.solve_complete_variable(variable, solution.into())
    }

    /// Solve one type variable from an omitted generic default.
    pub(in crate::check) fn solve_default_type_variable(
        &mut self,
        variable: VariableId,
        default: TypeOperand,
    ) -> CompilerResult<()> {
        // keep real lower bounds stronger than defaults
        if self.inference.has_lower_type_bounds(variable)
            || self.variable_solution(variable).is_some()
        {
            return Ok(());
        }

        let default = match default {
            // wait for another variable to solve first
            TypeOperand::Variable(source) if source != variable => {
                let Some(default) = self.variable_type_solution_operand(source) else {
                    return Ok(());
                };

                default
            }
            TypeOperand::Variable(_) => return Ok(()),
            TypeOperand::Term(_) | TypeOperand::Type(_) => default,
        };
        let solution = match default {
            TypeOperand::Term(term) => TypeSolution::Term(term),
            TypeOperand::Type(ty) => TypeSolution::Type(ty),
            TypeOperand::Variable(_) => return Ok(()),
        };

        self.solve_complete_variable(variable, solution.into())
    }

    /// Solve one static variable from its current bounds.
    pub(in crate::check) fn solve_static_variable(
        &mut self,
        variable: VariableId,
        term: StaticTerm,
    ) -> CompilerResult<()> {
        // wait for nested operands to solve
        if !term.referenced_variables(self).is_empty() {
            return Ok(());
        }

        let solution = StaticSolution::Term(self.inference.push_term(term));

        self.solve_complete_variable(variable, solution.into())
    }

    /// Solve one static variable from an omitted generic default.
    pub(in crate::check) fn solve_default_static_variable(
        &mut self,
        variable: VariableId,
        default: StaticOperand,
    ) -> CompilerResult<()> {
        // keep real lower bounds stronger than defaults
        if self.inference.has_lower_static_bounds(variable)
            || self.variable_solution(variable).is_some()
        {
            return Ok(());
        }

        let default = match default {
            // wait for another variable to solve first
            StaticOperand::Variable(source) if source != variable => {
                let Some(default) = self.variable_static_solution_operand(source) else {
                    return Ok(());
                };

                default
            }
            StaticOperand::Variable(_) => return Ok(()),
            StaticOperand::Term(_) | StaticOperand::Static(_) => default,
        };
        let solution = match default {
            StaticOperand::Term(term) => StaticSolution::Term(term),
            StaticOperand::Static(value) => StaticSolution::Static(value),
            StaticOperand::Variable(_) => return Ok(()),
        };

        self.solve_complete_variable(variable, solution.into())
    }

    /// Solve one type variable from lower bounds.
    pub(in crate::check) fn solve_type_variable_from_bounds(
        &mut self,
        variable: VariableId,
    ) -> CompilerResult<()> {
        let lower_bounds = self.inference.lower_type_bounds(variable);

        // wait for useful lower bounds
        if lower_bounds.is_empty() {
            return Ok(());
        }

        // collect concrete lower bound operands
        let origin = self.variable(variable).source;
        let mut operands = Vec::with_capacity(lower_bounds.len());
        for bound in lower_bounds {
            let reduction = self.reduce_type_bound(origin, variable, bound)?;
            if reduction.collect_into(&mut operands).is_break() {
                return Ok(());
            }
        }

        // solve from the best common concrete operand
        if operands.is_empty() {
            return Ok(());
        }
        let mut terms = Vec::with_capacity(operands.len());
        for operand in operands {
            let Some(term) = self.type_operand_term(operand)? else {
                return Ok(());
            };

            terms.push(term);
        }
        let term = self.reduce_best_common_terms(variable.module, terms)?;

        self.solve_type_variable(variable, term)
    }

    /// Reduce one lower type bound.
    fn reduce_type_bound(
        &mut self,
        origin: Origin,
        variable: VariableId,
        bound: TypeOperand,
    ) -> CompilerResult<BoundReduction<TypeOperand>> {
        // resolve the bound operand
        let operand = match bound {
            TypeOperand::Variable(bound) if bound == variable => {
                return Ok(BoundReduction::Empty);
            }
            TypeOperand::Variable(bound) => {
                let Some(operand) = self.variable_type_solution_operand(bound) else {
                    return Ok(BoundReduction::Pending);
                };

                operand
            }
            TypeOperand::Term(_) | TypeOperand::Type(_) => bound,
        };

        // push upper bounds into the lower bound operand
        self.expect_type_operand(origin, variable, operand)?;

        // reduce the bound operand
        let Some(operand) = self.reduce_type_operand(origin, operand)? else {
            return Ok(BoundReduction::Pending);
        };

        Ok(BoundReduction::Value(operand))
    }

    /// Solve one static variable from its bounds.
    pub(in crate::check) fn solve_static_variable_from_bounds(
        &mut self,
        variable: VariableId,
    ) -> CompilerResult<()> {
        let lower_bounds = self.inference.lower_static_bounds(variable);
        let upper_bounds = self.inference.upper_static_bounds(variable);

        // wait for useful bounds
        if lower_bounds.is_empty() && upper_bounds.is_empty() {
            return Ok(());
        }

        // collect lower bound terms
        let origin = self.variable(variable).source;
        let mut lower_terms = Vec::with_capacity(lower_bounds.len());
        for bound in lower_bounds.iter().copied() {
            let reduction = self.reduce_static_bound(origin, variable, bound)?;
            if reduction.collect_into(&mut lower_terms).is_break() {
                return Ok(());
            }
        }

        // collect upper bound terms
        let mut upper_terms = Vec::with_capacity(upper_bounds.len());
        for bound in upper_bounds.iter().copied() {
            let reduction = self.reduce_static_bound(origin, variable, bound)?;
            if reduction.collect_into(&mut upper_terms).is_break() {
                return Ok(());
            }
        }

        // solve from the static domain
        let Some(term) =
            self.reduce_static_bounds(&lower_bounds, &upper_bounds, &lower_terms, &upper_terms)
        else {
            return Ok(());
        };

        self.solve_static_variable(variable, term)
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
                    return Ok(BoundReduction::Pending);
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
            return Ok(BoundReduction::Pending);
        };

        Ok(BoundReduction::Value(term))
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
