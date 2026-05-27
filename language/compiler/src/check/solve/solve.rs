use crate::CompilerResult;
use crate::check::{
    CheckState, Constraint, Decision, Definition, PatternRelation, StaticTerm, TypeOperationTerm,
    TypeTerm, VariableId,
};

use super::Progress;
use super::queue::{SolverItem, SolverQueue};

impl CheckState<'_> {
    /// Solve collected component constraints to a fixed point.
    pub(in crate::check) fn solve(&mut self) -> CompilerResult<()> {
        self.variables.close();

        let items = self.collect_solve_items();
        let mut queue = SolverQueue::from(items, self);

        loop {
            // step queued work until no variable changes
            while let Some(item) = queue.next() {
                let progress = self.step_solve_item(&item)?;

                queue.wake(progress);
            }
            let progress = self.solve_bound_variables()?;
            if progress.is_unchanged() {
                break;
            }

            queue.wake_all();
        }

        Ok(())
    }

    /// Collect all component solver items in stable module order.
    fn collect_solve_items(&self) -> Vec<SolverItem> {
        let mut items = Vec::new();

        let variables = &self.variables;

        items.extend(
            variables
                .definitions
                .iter()
                .cloned()
                .map(SolverItem::Definition),
        );
        items.extend(
            variables
                .constraints
                .iter()
                .cloned()
                .map(SolverItem::Constraint),
        );

        items
    }

    /// Collect all component definitions in stable module order.
    pub(in crate::check) fn collect_definitions(&self) -> Vec<Definition> {
        self.variables.definitions.clone()
    }

    /// Collect all component constraints in stable module order.
    pub(in crate::check) fn collect_constraints(&self) -> Vec<Constraint> {
        self.variables.constraints.clone()
    }

    /// Step one solver item once.
    fn step_solve_item(&mut self, item: &SolverItem) -> CompilerResult<Progress> {
        match item {
            SolverItem::Definition(definition) => self.step_definition(definition),
            SolverItem::Constraint(constraint) => self.step_constraint(constraint),
        }
    }

    /// Step one definition once.
    fn step_definition(&mut self, definition: &Definition) -> CompilerResult<Progress> {
        match definition {
            Definition::Type {
                result,
                term,
                condition,
                ..
            } => match self.decide_static_condition(condition)? {
                Decision::No => Ok(Progress::Unchanged),
                Decision::Yes | Decision::Undecidable => {
                    let term = self.terms.get(*term).clone();
                    self.step_type_definition(*result, &term)
                }
            },
            Definition::Static {
                result,
                term,
                condition,
                ..
            } => match self.decide_static_condition(condition)? {
                Decision::No => Ok(Progress::Unchanged),
                Decision::Yes | Decision::Undecidable => {
                    let term = self.terms.get(*term).clone();
                    self.step_static_definition(*result, &term)
                }
            },
        }
    }

    /// Step one constraint once.
    fn step_constraint(&mut self, constraint: &Constraint) -> CompilerResult<Progress> {
        match constraint {
            Constraint::Type {
                relation,
                left,
                right,
                condition,
                ..
            } => match self.decide_static_condition(condition)? {
                Decision::Yes => self.solve_type_relation(*relation, *left, *right),
                Decision::No | Decision::Undecidable => Ok(Progress::Unchanged),
            },
            Constraint::Static {
                relation,
                left,
                right,
                condition,
                ..
            } => match self.decide_static_condition(condition)? {
                Decision::Yes => self.solve_static_relation(*relation, *left, *right),
                Decision::No | Decision::Undecidable => Ok(Progress::Unchanged),
            },
            Constraint::Pattern {
                relation,
                value,
                condition,
                ..
            } => match self.decide_static_condition(condition)? {
                Decision::Yes => match relation {
                    PatternRelation::Match(pattern) => self.expect_pattern_term(*value, *pattern),
                    PatternRelation::Assign(pattern) => {
                        self.expect_assign_pattern_term(*value, *pattern)
                    }
                },
                Decision::No | Decision::Undecidable => Ok(Progress::Unchanged),
            },
        }
    }

    /// Step one type definition.
    fn step_type_definition(
        &mut self,
        result: VariableId,
        term: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let reduction = self.reduce_type_term(result.module, term)?;
        let forward = match reduction.value {
            Some(term) => self.solve_type_variable(result, term)?,
            None if self.type_term_is_durable(term) => {
                self.solve_type_variable(result, term.clone())?
            }
            None => Progress::Unchanged,
        };
        let backward = self.expect_type_term(result, term)?;

        Ok(reduction.progress.merge(forward).merge(backward))
    }

    /// Return whether an unreduced type term can be committed as type structure.
    fn type_term_is_durable(&self, term: &TypeTerm) -> bool {
        let TypeTerm::Operation(operation) = term else {
            return false;
        };

        matches!(
            self.terms.get(*operation),
            TypeOperationTerm::Conditional { .. }
                | TypeOperationTerm::Index { .. }
                | TypeOperationTerm::TemplateLiteral { .. }
                | TypeOperationTerm::Infer { .. }
                | TypeOperationTerm::KeyOf { .. }
                | TypeOperationTerm::Mapped { .. }
        )
    }

    /// Step one static definition.
    fn step_static_definition(
        &mut self,
        result: VariableId,
        term: &StaticTerm,
    ) -> CompilerResult<Progress> {
        if let Some(term) = self.reduce_static_term(result.module, term)? {
            return self.solve_static_variable(result, term);
        }

        match term {
            // preserve parametric static source for generic substitution
            StaticTerm::Expression(_) | StaticTerm::Variable(_) => {
                self.solve_static_variable(result, term.clone())
            }
            _ => Ok(Progress::Unchanged),
        }
    }
}
