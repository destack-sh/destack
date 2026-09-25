use std::cmp::Ordering;

use tspp_dir as dir;

use crate::{Binding, Bindings, MatchError, ModuleContext, Predicate, ProgramContext};

/// One predicate expression evaluated against structural bindings.
#[derive(Debug)]
pub(crate) struct Evaluator<'a> {
    /// The predicate being evaluated.
    predicate: &'a Predicate,
    /// The structural bindings.
    bindings: &'a Bindings,
    /// The candidate root anchoring lexical predicate references.
    candidate: dir::LocalNodeIdAny,
    /// The candidate module.
    module: &'a ModuleContext,
    /// The checked program.
    program: &'a ProgramContext,
}

impl<'a> Evaluator<'a> {
    /// Create one predicate evaluator.
    pub(crate) fn new(
        predicate: &'a Predicate,
        bindings: &'a Bindings,
        candidate: dir::LocalNodeIdAny,
        module: &'a ModuleContext,
        program: &'a ProgramContext,
    ) -> Self {
        Self {
            predicate,
            bindings,
            candidate,
            module,
            program,
        }
    }

    /// Evaluate the predicate root.
    pub(crate) fn evaluate(&self) -> Result<bool, MatchError> {
        self.evaluate_condition(self.predicate.root())
    }

    /// Evaluate one boolean predicate expression.
    fn evaluate_condition(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, MatchError> {
        if let Some(variable) = self.predicate.uses().get(expression) {
            return Ok(self.bindings.get(variable).is_some());
        }

        match self.predicate.tree().get(expression) {
            dir::Expression::Literal(dir::Literal::Boolean(value)) => Ok(*value),
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Not,
                right,
            } => Ok(!self.evaluate_condition(*right)?),
            dir::Expression::Binary {
                left,
                operator: dir::BinaryOperator::And,
                right,
            } => {
                if !self.evaluate_condition(*left)? {
                    return Ok(false);
                }

                self.evaluate_condition(*right)
            }
            dir::Expression::Binary {
                left,
                operator: dir::BinaryOperator::Or,
                right,
            } => {
                if self.evaluate_condition(*left)? {
                    return Ok(true);
                }

                self.evaluate_condition(*right)
            }
            dir::Expression::Binary {
                left,
                operator,
                right,
            } if operator.is_equality() => self.evaluate_equality(*left, *operator, *right),
            dir::Expression::Binary {
                left,
                operator:
                    operator @ (dir::BinaryOperator::LessThan
                    | dir::BinaryOperator::LessThanOrEqual
                    | dir::BinaryOperator::GreaterThan
                    | dir::BinaryOperator::GreaterThanOrEqual),
                right,
            } => self.evaluate_order(*left, *operator, *right),
            dir::Expression::Satisfies {
                expression,
                target_type,
            } => self.evaluate_assignability(*expression, *target_type),
            _ => Err(MatchError::UnsupportedPredicateExpression),
        }
    }

    /// Evaluate one equality predicate.
    fn evaluate_equality(
        &self,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, MatchError> {
        let left_scalar = self.scalar(left)?;
        let right_scalar = self.scalar(right)?;
        let is_equal = if let (Some(left), Some(right)) = (left_scalar, right_scalar) {
            left == right
        } else {
            let left = self.symbols(left)?;
            let right = self.symbols(right)?;

            !left.is_empty() && left == right
        };
        let is_negative = matches!(
            operator,
            dir::BinaryOperator::NotEqual | dir::BinaryOperator::NotEqualStrict
        );

        Ok(if is_negative { !is_equal } else { is_equal })
    }

    /// Evaluate one ordered scalar predicate.
    fn evaluate_order(
        &self,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, MatchError> {
        let Some(left) = self.scalar(left)? else {
            return Ok(false);
        };
        let Some(right) = self.scalar(right)? else {
            return Ok(false);
        };
        let Some(ordering) = Self::scalar_order(left, right) else {
            return Ok(false);
        };
        let result = match operator {
            dir::BinaryOperator::LessThan => ordering == Ordering::Less,
            dir::BinaryOperator::LessThanOrEqual => ordering != Ordering::Greater,
            dir::BinaryOperator::GreaterThan => ordering == Ordering::Greater,
            dir::BinaryOperator::GreaterThanOrEqual => ordering != Ordering::Less,
            _ => return Err(MatchError::UnsupportedPredicateExpression),
        };

        Ok(result)
    }

    /// Evaluate one closed checked-type relation.
    fn evaluate_assignability(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        target: dir::LocalNodeId<dir::TypeExpression>,
    ) -> Result<bool, MatchError> {
        let Some(variable) = self.predicate.uses().get(expression) else {
            return Err(MatchError::InvalidPredicateBinding);
        };
        let Some(Binding::Node(node)) = self.bindings.get(variable) else {
            return Err(MatchError::InvalidPredicateBinding);
        };
        let Some(source) = self.module.node_type_id(*node) else {
            return Ok(false);
        };
        let result = self.module.is_assignable(
            self.candidate,
            source,
            self.predicate.tree(),
            target,
            self.program,
        )?;

        Ok(result)
    }

    /// Resolve one expression as a target symbol set.
    fn symbols(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Vec<dir::GlobalSymbolId>, MatchError> {
        let symbols = if let Some(variable) = self.predicate.uses().get(expression) {
            let Some(Binding::Node(node)) = self.bindings.get(variable) else {
                return Err(MatchError::InvalidPredicateBinding);
            };

            self.module.symbol_targets(*node)
        } else {
            return self
                .module
                .resolve_reference(
                    self.candidate,
                    self.predicate.tree(),
                    expression,
                    self.program,
                )
                .map_err(MatchError::from);
        };

        self.program
            .symbol_targets(&symbols)
            .map_err(MatchError::from)
    }

    /// Resolve one expression as a checked static scalar.
    fn scalar(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::Literal>, MatchError> {
        if let Some(variable) = self.predicate.uses().get(expression) {
            let Some(binding) = self.bindings.get(variable) else {
                return Err(MatchError::InvalidPredicateBinding);
            };

            return match binding {
                Binding::Node(node) => self
                    .module
                    .static_scalar(*node, self.program)
                    .map_err(MatchError::from),
                Binding::Name { name, .. } => Ok(Some(dir::Literal::String(*name))),
                Binding::Nodes(_) => Err(MatchError::InvalidPredicateBinding),
            };
        }

        match self.predicate.tree().get(expression) {
            dir::Expression::Literal(value) => Ok(Some(*value)),
            _ => {
                let symbols = self.module.resolve_reference(
                    self.candidate,
                    self.predicate.tree(),
                    expression,
                    self.program,
                )?;
                let [symbol] = symbols.as_slice() else {
                    return Ok(None);
                };

                self.program
                    .static_scalar(*symbol)
                    .map_err(MatchError::from)
            }
        }
    }

    /// Compare two scalar literals from the same ordered domain.
    fn scalar_order(left: dir::Literal, right: dir::Literal) -> Option<Ordering> {
        match (left, right) {
            (dir::Literal::Integer(left), dir::Literal::Integer(right))
            | (dir::Literal::Bigint(left), dir::Literal::Bigint(right)) => Some(left.cmp(&right)),
            (dir::Literal::Float(left), dir::Literal::Float(right)) => left.partial_cmp(&right),
            (dir::Literal::Character(left), dir::Literal::Character(right)) => {
                Some(left.cmp(&right))
            }
            _ => None,
        }
    }
}
