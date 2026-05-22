use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CheckComponentState, CheckFailure, Constraint, ConstraintKind, LayoutTerm, Relation,
    ResolutionTerm, ResolutionValue, StaticRelation, StaticTerm, Step, Term, TypeRelation,
    TypeTerm, VariableId, VariableValue,
};

use super::queue::WorkQueue;
use super::relation::RelationResult;

impl CheckComponentState<'_> {
    /// Solve collected component inference work into checked DIR tables.
    pub(in crate::check) fn solve(&mut self) -> CompilerResult<()> {
        let mut queue = WorkQueue::new(self.constraints());

        // drain constraints unlocked by changed variables
        while let Some(constraint) = queue.pop() {
            let step = self.step_constraint(&constraint)?;

            self.apply_step(step, &mut queue)?;
        }

        Ok(())
    }

    /// Return all component constraints in stable module order.
    fn constraints(&self) -> Vec<Constraint> {
        let mut constraints = Vec::new();

        // preserve module order and then local walk order
        for check_module in self.modules.values() {
            constraints.extend(check_module.constraints().iter().cloned());
        }

        constraints
    }

    /// Step one constraint once.
    fn step_constraint(&mut self, constraint: &Constraint) -> CompilerResult<Step> {
        if !constraint.guard.is_always() {
            return Ok(Step::Pending);
        }

        match &constraint.kind {
            ConstraintKind::Bind { variable, term } => self.step_bind(*variable, term),
            ConstraintKind::Relate(relation) => self.step_relation(relation),
            ConstraintKind::Construct(_) | ConstraintKind::Resolve(_) => Ok(Step::Pending),
        }
    }

    /// Step one binding constraint.
    fn step_bind(&mut self, variable: VariableId, term: &Term) -> CompilerResult<Step> {
        let Some(value) = self.term_value(variable, term)? else {
            return Ok(Step::Pending);
        };

        let step = self
            .module_mut(variable.module)?
            .bind_variable(variable, value);

        Ok(step)
    }

    /// Step one relation constraint.
    fn step_relation(&mut self, relation: &Relation) -> CompilerResult<Step> {
        match relation {
            Relation::Type {
                relation: TypeRelation::Equal,
                left,
                right,
            } => self.step_type_equal(*left, *right),
            Relation::Type {
                relation,
                left,
                right,
            } => self.step_type_relation(*relation, *left, *right),
            Relation::Static {
                relation: StaticRelation::Equal,
                left,
                right,
            } => self.step_static_equal(*left, *right),
        }
    }

    /// Step one type equality relation.
    fn step_type_equal(&mut self, left: VariableId, right: VariableId) -> CompilerResult<Step> {
        let left_value = self.type_value(left)?;
        let right_value = self.type_value(right)?;

        // propagate a solved side into the unsolved side
        match (left_value, right_value) {
            (Some(left_type), None) => {
                let value = VariableValue::Type(left_type);

                Ok(self.module_mut(right.module)?.bind_variable(right, value))
            }
            (None, Some(right_type)) => {
                let value = VariableValue::Type(right_type);

                Ok(self.module_mut(left.module)?.bind_variable(left, value))
            }
            (Some(_), Some(_)) => {
                let result = self.check_type_relation(TypeRelation::Equal, left, right)?;
                let step = match result {
                    RelationResult::Holds | RelationResult::Waits => Step::Pending,
                    RelationResult::Fails => Step::Failed(CheckFailure::TypeRelation {
                        relation: TypeRelation::Equal,
                        left,
                        right,
                    }),
                };

                Ok(step)
            }
            _ => Ok(Step::Pending),
        }
    }

    /// Step one non-equality type relation.
    fn step_type_relation(
        &mut self,
        relation: TypeRelation,
        left: VariableId,
        right: VariableId,
    ) -> CompilerResult<Step> {
        let result = self.check_type_relation(relation, left, right)?;
        let step = match result {
            RelationResult::Holds | RelationResult::Waits => Step::Pending,
            RelationResult::Fails => Step::Failed(CheckFailure::TypeRelation {
                relation,
                left,
                right,
            }),
        };

        Ok(step)
    }

    /// Step one static equality relation.
    fn step_static_equal(&mut self, left: VariableId, right: VariableId) -> CompilerResult<Step> {
        let left_value = self.static_value(left)?;
        let right_value = self.static_value(right)?;

        // propagate a solved side into the unsolved side
        match (left_value, right_value) {
            (Some(left_static), None) => {
                let value = VariableValue::Static(left_static);

                Ok(self.module_mut(right.module)?.bind_variable(right, value))
            }
            (None, Some(right_static)) => {
                let value = VariableValue::Static(right_static);

                Ok(self.module_mut(left.module)?.bind_variable(left, value))
            }
            (Some(left_static), Some(right_static)) if left_static != right_static => {
                Ok(Step::Failed(CheckFailure::VariableConflict {
                    variable: left,
                }))
            }
            _ => Ok(Step::Pending),
        }
    }

    /// Return a solved variable value.
    pub(in crate::check) fn variable_value(
        &self,
        variable: VariableId,
    ) -> CompilerResult<Option<VariableValue>> {
        let value = self
            .module(variable.module)?
            .variable_value(variable)
            .cloned();

        Ok(value)
    }

    /// Return a solved type value.
    pub(in crate::check) fn type_value(
        &self,
        variable: VariableId,
    ) -> CompilerResult<Option<dir::LocalTypeId>> {
        let value = self.variable_value(variable)?;
        let value = match value {
            Some(VariableValue::Type(type_id)) => Some(type_id),
            _ => None,
        };

        Ok(value)
    }

    /// Return a solved static value.
    fn static_value(&self, variable: VariableId) -> CompilerResult<Option<dir::LocalStaticId>> {
        let value = self.variable_value(variable)?;
        let value = match value {
            Some(VariableValue::Static(static_id)) => Some(static_id),
            _ => None,
        };

        Ok(value)
    }

    /// Return the solved value for one term.
    fn term_value(
        &mut self,
        variable: VariableId,
        term: &Term,
    ) -> CompilerResult<Option<VariableValue>> {
        match term {
            Term::Type(term) => self.type_term_value(variable, term),
            Term::Static(term) => self.static_term_value(variable, term),
            Term::Layout(term) => {
                let value = match term {
                    LayoutTerm::Layout(layout) => Some(VariableValue::Layout(*layout)),
                    LayoutTerm::Variable(source) => self.variable_value(*source)?,
                };

                Ok(value)
            }
            Term::Resolution(term) => Ok(Some(VariableValue::Resolution(match term {
                ResolutionTerm::Name(resolution) => ResolutionValue::Name(resolution.clone()),
                ResolutionTerm::Call(resolution) => ResolutionValue::Call(resolution.clone()),
                ResolutionTerm::Member(resolution) => ResolutionValue::Member(resolution.clone()),
            }))),
        }
    }

    /// Return the solved value for one type term.
    fn type_term_value(
        &mut self,
        variable: VariableId,
        term: &TypeTerm,
    ) -> CompilerResult<Option<VariableValue>> {
        match term {
            TypeTerm::Type(ty) => {
                let source = self.module(variable.module)?.variable_source_node(variable);
                let type_id = self
                    .module_mut(variable.module)?
                    .intern_type(ty.clone(), source);

                Ok(Some(VariableValue::Type(type_id)))
            }
            TypeTerm::Variable(source) => {
                let value = self.type_value(*source)?.map(VariableValue::Type);

                Ok(value)
            }
            TypeTerm::TypeExpression(node) => {
                let value = self
                    .evaluate_type_expression(variable, node.clone())?
                    .map(VariableValue::Type);

                Ok(value)
            }
        }
    }

    /// Return the solved value for one static term.
    fn static_term_value(
        &mut self,
        _variable: VariableId,
        term: &StaticTerm,
    ) -> CompilerResult<Option<VariableValue>> {
        match term {
            StaticTerm::Value(value) => {
                let static_id = self
                    .module_mut(_variable.module)?
                    .intern_static(value.clone());

                Ok(Some(VariableValue::Static(static_id)))
            }
            StaticTerm::Variable(source) => {
                let value = self.static_value(*source)?.map(VariableValue::Static);

                Ok(value)
            }
            StaticTerm::Expression(_) => Ok(None),
        }
    }

    /// Apply one solver step.
    fn apply_step(&mut self, step: Step, queue: &mut WorkQueue) -> CompilerResult<()> {
        match step {
            Step::Pending => {}
            Step::Applied(variables) => {
                for variable in variables {
                    queue.enqueue_dependents(variable);
                }
            }
            Step::Failed(failure) => {
                let module = self.failure_module(&failure);

                self.module_mut(module)?.push_failure(failure);
            }
        }

        Ok(())
    }

    /// Return the module that should receive one failure.
    fn failure_module(&self, failure: &CheckFailure) -> ModuleId {
        match failure {
            CheckFailure::VariableConflict { variable }
            | CheckFailure::TypeRelation { left: variable, .. } => variable.module,
            CheckFailure::StaticEvaluation { node } | CheckFailure::TypeEvaluation { node } => {
                node.module_id
            }
        }
    }
}
