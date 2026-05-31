use crate::check::{
    CheckState, Origin, Solution, StaticOperand, StaticTerm, TypeOperand, TypeTerm, VariableId,
    VariableKind,
};
use crate::{CompilerError, CompilerResult};

use super::Progress;

/// Reduced value for one bound.
enum BoundReduction<T> {
    /// The bound contributes no term.
    Empty,
    /// The bound is waiting on more solver progress.
    Pending,
    /// The bound contributes one concrete term.
    Term(T),
}

impl CheckState<'_> {
    /// Insert one solver solution and report conflicting solutions loudly.
    fn solve_variable(
        &mut self,
        variable: VariableId,
        solution: Solution,
    ) -> CompilerResult<Progress> {
        // reject conflicting writes
        if let Some(existing) = self.inference.variable_solution(variable) {
            let origin = self.variable(variable).source;
            Err(CompilerError::Internal {
                message: format!(
                    "check variable {variable:?} from {origin:?} already has solution {existing:?}, got {solution:?}"
                ),
            })
        } else {
            // store the first solution
            self.inference.insert_variable_solution(variable, solution);
            Ok(Progress::changed(variable))
        }
    }

    /// Solve one type variable and return solver progress.
    pub(in crate::check) fn solve_type_variable(
        &mut self,
        variable: VariableId,
        term: TypeTerm,
    ) -> CompilerResult<Progress> {
        // ignore self aliases
        if matches!(term, TypeTerm::Variable(source) if source == variable) {
            return Ok(Progress::Unchanged);
        }

        // preserve aliases as operands
        let operand = match term {
            TypeTerm::Variable(variable) => TypeOperand::Variable(variable),
            term => self.push_term(term).into(),
        };

        self.solve_variable(variable, Solution::Type(operand))
    }

    /// Solve one type variable from a fallback default.
    pub(in crate::check) fn solve_default_type_variable(
        &mut self,
        variable: VariableId,
        default: TypeOperand,
    ) -> CompilerResult<Progress> {
        // keep real lower bounds stronger than defaults
        if !self.lower_type_bounds(variable).is_empty()
            || self.variable_solution(variable).is_some()
        {
            return Ok(Progress::Unchanged);
        }

        // ignore self aliases
        if matches!(default, TypeOperand::Variable(source) if source == variable) {
            return Ok(Progress::Unchanged);
        }

        self.solve_variable(variable, Solution::Type(default))
    }

    /// Solve one static variable and return solver progress.
    pub(in crate::check) fn solve_static_variable(
        &mut self,
        variable: VariableId,
        term: StaticTerm,
    ) -> CompilerResult<Progress> {
        // ignore self aliases
        if matches!(term, StaticTerm::Variable(source) if source == variable) {
            return Ok(Progress::Unchanged);
        }

        // preserve aliases as operands
        let operand = match term {
            StaticTerm::Variable(variable) => StaticOperand::Variable(variable),
            term => self.push_term(term).into(),
        };

        self.solve_variable(variable, Solution::Static(operand))
    }

    /// Solve one static variable from a fallback default.
    pub(in crate::check) fn solve_default_static_variable(
        &mut self,
        variable: VariableId,
        default: StaticOperand,
    ) -> CompilerResult<Progress> {
        // keep real lower bounds stronger than defaults
        if !self.lower_static_bounds(variable).is_empty()
            || self.variable_solution(variable).is_some()
        {
            return Ok(Progress::Unchanged);
        }

        // ignore self aliases
        if matches!(default, StaticOperand::Variable(source) if source == variable) {
            return Ok(Progress::Unchanged);
        }

        self.solve_variable(variable, Solution::Static(default))
    }

    /// Solve one variable from its collected bounds.
    pub(in crate::check) fn solve_bound_variable(
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
            VariableKind::Type => self.solve_bound_type_variable(variable),
            VariableKind::Static => self.solve_bound_static_variable(variable),
        }
    }

    /// Solve one type variable from lower bounds.
    fn solve_bound_type_variable(&mut self, variable: VariableId) -> CompilerResult<Progress> {
        let lower_bounds = self.lower_type_bounds(variable).to_vec();

        // wait for useful lower bounds
        if lower_bounds.is_empty() {
            return Ok(Progress::Unchanged);
        }

        // preserve a single direct alias without forcing a concrete term
        if let [TypeOperand::Variable(source)] = lower_bounds.as_slice() {
            return self.solve_type_variable(variable, TypeTerm::Variable(*source));
        }

        // collect concrete lower bound terms
        let origin = self.variable(variable).source;
        let mut terms = Vec::with_capacity(lower_bounds.len());
        for bound in lower_bounds {
            match self.reduce_type_bound(origin, variable, bound)? {
                BoundReduction::Empty => {}
                BoundReduction::Pending => return Ok(Progress::Unchanged),
                BoundReduction::Term(term) => terms.push(term),
            }
        }

        // solve from the best common concrete term
        if terms.is_empty() {
            return Ok(Progress::Unchanged);
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
    ) -> CompilerResult<BoundReduction<TypeTerm>> {
        // resolve the bound operand
        let term = match bound {
            TypeOperand::Variable(bound) if bound == variable => return Ok(BoundReduction::Empty),
            TypeOperand::Variable(bound) => {
                let Some(term) = self.type_solution(bound)? else {
                    return Ok(BoundReduction::Pending);
                };

                term
            }
            TypeOperand::Term(term) => self.term(term).clone(),
        };

        // reduce the bound term
        self.reduce_type_bound_term(origin, variable, term)
    }

    /// Reduce one solved lower type bound.
    fn reduce_type_bound_term(
        &mut self,
        origin: Origin,
        variable: VariableId,
        term: TypeTerm,
    ) -> CompilerResult<BoundReduction<TypeTerm>> {
        // reject unresolved aliases
        let term = match term {
            TypeTerm::Variable(source) if source == variable => return Ok(BoundReduction::Empty),
            TypeTerm::Variable(_) => return Ok(BoundReduction::Pending),
            term => term,
        };

        // reduce computed type terms
        let Some(term) = self.reduce_type_operand_term(origin, term)? else {
            return Ok(BoundReduction::Pending);
        };

        Ok(BoundReduction::Term(term))
    }

    /// Solve one static variable from its bounds.
    fn solve_bound_static_variable(&mut self, variable: VariableId) -> CompilerResult<Progress> {
        let lower_bounds = self.lower_static_bounds(variable).to_vec();
        let upper_bounds = self.upper_static_bounds(variable).to_vec();

        // wait for useful bounds
        if lower_bounds.is_empty() && upper_bounds.is_empty() {
            return Ok(Progress::Unchanged);
        }

        // collect lower bound terms
        let origin = self.variable(variable).source;
        let mut lower_terms = Vec::with_capacity(lower_bounds.len());
        for bound in lower_bounds.iter().copied() {
            match self.reduce_static_bound(origin, variable, bound)? {
                BoundReduction::Empty => {}
                BoundReduction::Pending => return Ok(Progress::Unchanged),
                BoundReduction::Term(term) => lower_terms.push(term),
            }
        }

        // collect upper bound terms
        let mut upper_terms = Vec::with_capacity(upper_bounds.len());
        for bound in upper_bounds.iter().copied() {
            match self.reduce_static_bound(origin, variable, bound)? {
                BoundReduction::Empty => {}
                BoundReduction::Pending => return Ok(Progress::Unchanged),
                BoundReduction::Term(term) => upper_terms.push(term),
            }
        }

        // solve from the static domain
        let Some(term) =
            self.reduce_static_bounds(&lower_bounds, &upper_bounds, &lower_terms, &upper_terms)
        else {
            return Ok(Progress::Unchanged);
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
            StaticOperand::Term(term) => self.term(term).clone(),
        };

        // reduce the bound term
        self.reduce_static_bound_term(origin, variable, term)
    }

    /// Reduce one solved static bound.
    fn reduce_static_bound_term(
        &mut self,
        origin: Origin,
        variable: VariableId,
        term: StaticTerm,
    ) -> CompilerResult<BoundReduction<StaticTerm>> {
        // preserve static aliases
        let term = match term {
            StaticTerm::Variable(source) if source == variable => return Ok(BoundReduction::Empty),
            StaticTerm::Variable(_) => return Ok(BoundReduction::Term(term)),
            term => term,
        };

        // reduce computed static terms
        let Some(term) = self.reduce_static_term(origin, &term)? else {
            return Ok(BoundReduction::Pending);
        };

        Ok(BoundReduction::Term(term))
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
        let term = match self.variable_solution(variable) {
            Some(Solution::Type(TypeOperand::Variable(variable))) => TypeTerm::Variable(variable),
            Some(Solution::Type(TypeOperand::Term(term))) => self.term(term).clone(),
            Some(Solution::Static(_)) | None => return Ok(None),
        };

        Ok(Some(term))
    }

    /// Return the static solution for one variable.
    pub(in crate::check) fn static_solution(
        &self,
        variable: VariableId,
    ) -> CompilerResult<Option<StaticTerm>> {
        let term = match self.variable_solution(variable) {
            Some(Solution::Static(StaticOperand::Variable(variable))) => {
                StaticTerm::Variable(variable)
            }
            Some(Solution::Static(StaticOperand::Term(term))) => self.term(term).clone(),
            Some(Solution::Type(_)) | None => return Ok(None),
        };

        Ok(Some(term))
    }
}
