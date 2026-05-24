use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CheckComponentState, Solution, StaticTerm, TypeTerm, VariableId, VariableKind, VariableOrigin,
};

use super::queue::Progress;

impl CheckComponentState<'_> {
    /// Solve one type variable and return the queue progress.
    pub(super) fn solve_type_variable(
        &mut self,
        variable: VariableId,
        term: TypeTerm,
    ) -> CompilerResult<Progress> {
        let value = Solution::Type(term);
        let changed = self
            .module_mut(variable.module)?
            .solve_variable(variable, value);

        Ok(Progress::from_change(variable, changed))
    }

    /// Solve one static variable and return the queue progress.
    pub(super) fn solve_static_variable(
        &mut self,
        variable: VariableId,
        term: StaticTerm,
    ) -> CompilerResult<Progress> {
        let value = Solution::Static(term);
        let changed = self
            .module_mut(variable.module)?
            .solve_variable(variable, value);

        Ok(Progress::from_change(variable, changed))
    }

    /// Push one solved type variable for a reduced term.
    pub(super) fn push_solved_type_variable(
        &mut self,
        module: ModuleId,
        term: TypeTerm,
    ) -> CompilerResult<VariableId> {
        let module = self.module_mut(module)?;
        let variable = module.push_variable(VariableKind::Type, VariableOrigin::generated());
        let value = Solution::Type(term);

        module.solve_variable(variable, value);

        Ok(variable)
    }

    /// Push one solved static variable for a reduced DIR term.
    pub(super) fn push_solved_static_value_variable(
        &mut self,
        module: ModuleId,
        term: dir::StaticTerm,
    ) -> CompilerResult<VariableId> {
        self.push_solved_static_term_variable(module, StaticTerm::Literal(term))
    }

    /// Push one solved static variable for a reduced check term.
    pub(super) fn push_solved_static_term_variable(
        &mut self,
        module: ModuleId,
        term: StaticTerm,
    ) -> CompilerResult<VariableId> {
        let module = self.module_mut(module)?;
        let variable = module.push_variable(VariableKind::Static, VariableOrigin::generated());
        let value = Solution::Static(term);

        module.solve_variable(variable, value);

        Ok(variable)
    }

    /// Solve variables whose lower bounds determine a concrete solution.
    pub(super) fn solve_bound_variables(&mut self) -> CompilerResult<Progress> {
        let variables = self
            .modules
            .values()
            .flat_map(|check_module| {
                check_module
                    .work
                    .variables
                    .all
                    .iter()
                    .map(|variable| variable.id)
            })
            .collect::<Vec<_>>();
        let mut progress = Progress::Unchanged;

        // solve variables in stable module and allocation order
        for variable in variables {
            progress = progress.merge(self.solve_bound_variable(variable)?);
        }

        Ok(progress)
    }

    /// Solve one variable from its collected bounds.
    fn solve_bound_variable(&mut self, variable: VariableId) -> CompilerResult<Progress> {
        if self.variable_solution(variable)?.is_some() {
            return Ok(Progress::Unchanged);
        }
        let kind = self.module(variable.module)?.variable(variable).kind;

        match kind {
            VariableKind::Type => self.solve_bound_type_variable(variable),
            VariableKind::Static => self.solve_bound_static_variable(variable),
        }
    }

    /// Solve one type variable from lower bounds.
    fn solve_bound_type_variable(&mut self, variable: VariableId) -> CompilerResult<Progress> {
        let lower_bounds = self
            .module(variable.module)?
            .variable(variable)
            .lower_bounds
            .clone();
        if lower_bounds.is_empty() {
            return Ok(Progress::Unchanged);
        }
        let mut candidates = Vec::with_capacity(lower_bounds.len());

        // collect solved lower bound candidates
        for lower_bound in lower_bounds {
            let Some(term) = self.solved_type_term(lower_bound)? else {
                return Ok(Progress::Unchanged);
            };
            let term = Self::widen_inferred_type(term);

            candidates.push(term);
        }
        let term = self.reduce_best_common_terms(variable.module, candidates)?;

        self.solve_type_variable(variable, term)
    }

    /// Solve one static variable from lower bounds.
    fn solve_bound_static_variable(&mut self, variable: VariableId) -> CompilerResult<Progress> {
        let lower_bounds = self
            .module(variable.module)?
            .variable(variable)
            .lower_bounds
            .clone();
        if lower_bounds.is_empty() {
            return Ok(Progress::Unchanged);
        }
        let mut terms = Vec::with_capacity(lower_bounds.len());

        // collect solved lower bound terms
        for lower_bound in lower_bounds {
            let Some(term) = self.solved_static_term(lower_bound)? else {
                return Ok(Progress::Unchanged);
            };

            terms.push(term);
        }
        let Some(first) = terms.first().cloned() else {
            return Ok(Progress::Unchanged);
        };

        // only equality compatible static bounds determine a value
        for term in terms.iter().skip(1) {
            if term != &first {
                return Ok(Progress::Unchanged);
            }
        }

        self.solve_static_variable(variable, first)
    }

    /// Return a solved variable value.
    pub(in crate::check) fn variable_solution(
        &self,
        variable: VariableId,
    ) -> CompilerResult<Option<Solution>> {
        let value = self
            .module(variable.module)?
            .variable_solution(variable)
            .cloned();

        Ok(value)
    }

    /// Return a solved type term.
    pub(in crate::check) fn solved_type_term(
        &self,
        variable: VariableId,
    ) -> CompilerResult<Option<TypeTerm>> {
        let value = self.variable_solution(variable)?;
        let value = match value {
            Some(Solution::Type(TypeTerm::Variable(source))) if source != variable => {
                self.solved_type_term(source)?
            }
            Some(Solution::Type(term)) => Some(term),
            _ => None,
        };

        Ok(value)
    }

    /// Return a solved static term.
    pub(in crate::check) fn solved_static_term(
        &self,
        variable: VariableId,
    ) -> CompilerResult<Option<StaticTerm>> {
        let value = self.variable_solution(variable)?;
        let value = match value {
            Some(Solution::Static(StaticTerm::Variable(source))) if source != variable => {
                self.solved_static_term(source)?
            }
            Some(Solution::Static(StaticTerm::Expression(source))) => {
                if let Some(term) = self.static_expression_term(source.clone())? {
                    Some(StaticTerm::Literal(term))
                } else {
                    Some(StaticTerm::Expression(source))
                }
            }
            Some(Solution::Static(term)) => Some(term),
            _ => None,
        };

        Ok(value)
    }

    /// Return one locally concrete static expression term.
    pub(super) fn static_expression_term(
        &self,
        expression: dir::GlobalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let expression_node = self
            .module(expression.module_id)?
            .input
            .parsed
            .tree
            .get(expression.local_id);
        let term = match expression_node {
            dir::Expression::ScalarLiteral(value) => Some(dir::StaticTerm::ScalarLiteral {
                value: value.clone(),
            }),
            _ => None,
        };

        Ok(term)
    }
}
