use indexmap::IndexSet;

use crate::check::{
    CheckState, Decision, Solution, StaticOperand, StaticRelation, StaticTerm, TypeOperand,
    TypeRelation, TypeTerm, VariableId, VariableKind,
};
use crate::{CompilerError, CompilerResult};

use super::Progress;

impl CheckState<'_> {
    /// Insert one solver solution and report conflicting solutions loudly.
    fn insert_solution(
        &mut self,
        variable: VariableId,
        solution: Solution,
    ) -> CompilerResult<Progress> {
        let Some(existing) = self.solutions.variable.get(&variable) else {
            self.solutions.variable.insert(variable, solution);

            return Ok(Progress::changed(variable));
        };
        if existing == &solution {
            return Ok(Progress::Unchanged);
        }

        Err(CompilerError::Internal {
            message: format!(
                "check variable {variable:?} already has solution {existing:?}, got {solution:?}"
            ),
        })
    }

    /// Solve one type variable and return solver progress.
    pub(in crate::check) fn solve_type_variable(
        &mut self,
        variable: VariableId,
        term: TypeTerm,
    ) -> CompilerResult<Progress> {
        if self.type_solution_matches(variable, &term)? {
            return Ok(Progress::Unchanged);
        }

        let term = self.terms.push(term);
        let value = Solution::Type(term);
        self.insert_solution(variable, value)
    }

    /// Solve one static variable and return solver progress.
    pub(in crate::check) fn solve_static_variable(
        &mut self,
        variable: VariableId,
        term: StaticTerm,
    ) -> CompilerResult<Progress> {
        if self.static_solution_matches(variable, &term)? {
            return Ok(Progress::Unchanged);
        }

        let term = self.terms.push(term);
        let value = Solution::Static(term);
        self.insert_solution(variable, value)
    }

    /// Return whether one type variable already has one solved type.
    fn type_solution_matches(&self, variable: VariableId, term: &TypeTerm) -> CompilerResult<bool> {
        let Some(solution) = self.solved_type_term(variable)? else {
            return Ok(false);
        };
        let decision = self.decide_type_term_relation(TypeRelation::Equal, &solution, term)?;

        Ok(decision == Decision::Yes)
    }

    /// Return whether one static variable already has one solved value.
    fn static_solution_matches(
        &self,
        variable: VariableId,
        term: &StaticTerm,
    ) -> CompilerResult<bool> {
        let Some(solution) = self.solved_static_term(variable)? else {
            return Ok(false);
        };
        let decision = self.decide_static_term_relation(StaticRelation::Equal, &solution, term)?;

        Ok(decision == Decision::Yes)
    }

    /// Solve one variable from its collected bounds.
    pub(in crate::check) fn solve_bound_variable(
        &mut self,
        variable: VariableId,
    ) -> CompilerResult<Progress> {
        // keep definitions as the source of truth
        if self.variables.defined.contains(&variable) {
            return Ok(Progress::Unchanged);
        }
        if self.variable_solution(variable).is_some() {
            return Ok(Progress::Unchanged);
        }
        let kind = self.variable(variable).kind;

        match kind {
            VariableKind::Type => self.solve_bound_type_variable(variable),
            VariableKind::Static => self.solve_bound_static_variable(variable),
        }
    }

    /// Solve one type variable from lower bounds.
    fn solve_bound_type_variable(&mut self, variable: VariableId) -> CompilerResult<Progress> {
        let lower_bounds = self.lower_type_bounds(variable);
        if lower_bounds.is_empty() {
            return Ok(Progress::Unchanged);
        }
        let mut candidates = Vec::with_capacity(lower_bounds.len());

        // collect solved lower bound candidates
        for lower_bound in lower_bounds.iter().copied() {
            let term = match lower_bound {
                TypeOperand::Variable(bound) if bound == variable => continue,
                TypeOperand::Variable(variable) => {
                    let Some(term) = self.solved_type_term(variable)? else {
                        return Ok(Progress::Unchanged);
                    };

                    term
                }
                TypeOperand::Term(term) => self.terms.get(term).clone(),
            };

            candidates.push(term);
        }
        let term = self.reduce_best_common_terms(variable.module, candidates)?;

        self.solve_type_variable(variable, term)
    }

    /// Solve one static variable from lower bounds.
    fn solve_bound_static_variable(&mut self, variable: VariableId) -> CompilerResult<Progress> {
        let lower_bounds = self.lower_static_bounds(variable);
        let upper_bounds = self.upper_static_bounds(variable);
        if lower_bounds.is_empty() && upper_bounds.is_empty() {
            return Ok(Progress::Unchanged);
        }
        let mut lower_terms = Vec::with_capacity(lower_bounds.len());

        // collect solved lower bound terms
        for lower_bound in lower_bounds.iter().copied() {
            let term = match lower_bound {
                StaticOperand::Variable(bound) if bound == variable => continue,
                StaticOperand::Variable(variable) => {
                    let Some(term) = self.solved_static_term(variable)? else {
                        return Ok(Progress::Unchanged);
                    };

                    term
                }
                StaticOperand::Term(term) => self.terms.get(term).clone(),
            };

            lower_terms.push(term);
        }
        let mut upper_terms = Vec::with_capacity(upper_bounds.len());

        // collect solved upper bound terms
        for upper_bound in upper_bounds.iter().copied() {
            let term = match upper_bound {
                StaticOperand::Variable(bound) if bound == variable => continue,
                StaticOperand::Variable(variable) => {
                    let Some(term) = self.solved_static_term(variable)? else {
                        return Ok(Progress::Unchanged);
                    };

                    term
                }
                StaticOperand::Term(term) => self.terms.get(term).clone(),
            };

            upper_terms.push(term);
        }
        if let Some(term) =
            self.reduce_static_bounds(&lower_bounds, &upper_bounds, &lower_terms, &upper_terms)
        {
            return self.solve_static_variable(variable, term);
        }

        Ok(Progress::Unchanged)
    }

    /// Return a solved variable value.
    pub(in crate::check) fn variable_solution(&self, variable: VariableId) -> Option<Solution> {
        self.solutions.variable.get(&variable).cloned()
    }

    /// Return a solved type term.
    pub(in crate::check) fn solved_type_term(
        &self,
        variable: VariableId,
    ) -> CompilerResult<Option<TypeTerm>> {
        let mut seen = IndexSet::new();

        self.solved_type_term_inner(variable, &mut seen)
    }

    /// Return a solved type operand.
    pub(in crate::check) fn solved_type_operand(
        &self,
        operand: TypeOperand,
    ) -> CompilerResult<Option<TypeTerm>> {
        match operand {
            TypeOperand::Variable(variable) => self.solved_type_term(variable),
            TypeOperand::Term(term) => Ok(Some(self.terms.get(term).clone())),
        }
    }

    /// Return a solved type term, stopping at variable cycles.
    fn solved_type_term_inner(
        &self,
        variable: VariableId,
        seen: &mut IndexSet<VariableId>,
    ) -> CompilerResult<Option<TypeTerm>> {
        if !seen.insert(variable) {
            return Ok(None);
        }
        let value = match self.variable_solution(variable) {
            Some(Solution::Type(term)) => {
                let term = self.terms.get(term);
                if let TypeTerm::Variable(source) = term
                    && *source != variable
                {
                    self.solved_type_term_inner(*source, seen)?
                } else {
                    Some(term.clone())
                }
            }
            _ => None,
        };

        Ok(value)
    }

    /// Return a solved static term.
    pub(in crate::check) fn solved_static_term(
        &self,
        variable: VariableId,
    ) -> CompilerResult<Option<StaticTerm>> {
        let mut seen = IndexSet::new();

        self.solved_static_term_inner(variable, &mut seen)
    }

    /// Return a solved static term, stopping at variable cycles.
    fn solved_static_term_inner(
        &self,
        variable: VariableId,
        seen: &mut IndexSet<VariableId>,
    ) -> CompilerResult<Option<StaticTerm>> {
        if !seen.insert(variable) {
            return Ok(None);
        }
        let value = match self.variable_solution(variable) {
            Some(Solution::Static(term)) => {
                let term = self.terms.get(term);
                if let StaticTerm::Variable(source) = term
                    && *source != variable
                {
                    self.solved_static_term_inner(*source, seen)?
                } else {
                    Some(term.clone())
                }
            }
            _ => None,
        };

        Ok(value)
    }
}
