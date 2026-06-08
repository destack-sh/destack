use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{ShapeMember, TypeLiteralTerm, TypeOperand, TypeTerm, WalkState};

/// The condition branch being entered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum ConditionBranch {
    /// The condition is known true.
    True,
    /// The condition is known false.
    False,
}

impl ConditionBranch {
    /// Return the opposite control branch.
    pub(in crate::check) fn opposite(self) -> Self {
        match self {
            // true branch
            Self::True => Self::False,
            // false branch
            Self::False => Self::True,
        }
    }
}

impl WalkState<'_, '_> {
    /// Narrow flow from one condition.
    ///
    /// Example:
    /// ```ds
    /// if value is T { value }
    /// ```
    pub(in crate::check) fn narrow_condition(
        &mut self,
        condition: &dir::IfCondition,
        branch: ConditionBranch,
    ) -> CompilerResult<()> {
        match condition {
            // if condition
            dir::IfCondition::Expression { condition } => {
                self.narrow_expression(*condition, branch)?;
            }
            // if let pattern = value
            dir::IfCondition::Let { declarator, .. } if branch == ConditionBranch::True => {
                self.mark_declarator_assigned(self.tree.get(*declarator));
                self.narrow_declarator_pattern_success(*declarator)?;
            }
            // if let pattern = value
            dir::IfCondition::Let { .. } => {}
        }

        Ok(())
    }

    /// Narrow flow from one expression condition.
    ///
    /// Example:
    /// ```ds
    /// if value !== undefined { value }
    /// ```
    pub(in crate::check) fn narrow_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        branch: ConditionBranch,
    ) -> CompilerResult<()> {
        match self.tree.get(id) {
            // (value)
            dir::Expression::Parenthesized { expression } => {
                self.narrow_expression(*expression, branch)?;
            }
            // !value
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Not,
                right,
            } => {
                self.narrow_expression(*right, branch.opposite())?;
            }
            // left && right
            dir::Expression::Binary {
                left,
                operator: dir::BinaryOperator::And,
                right,
            } if branch == ConditionBranch::True => {
                self.narrow_expression(*left, branch)?;
                self.narrow_expression(*right, branch)?;
            }
            // left || right
            dir::Expression::Binary {
                left,
                operator: dir::BinaryOperator::Or,
                right,
            } if branch == ConditionBranch::False => {
                self.narrow_expression(*left, branch)?;
                self.narrow_expression(*right, branch)?;
            }
            // left === right
            dir::Expression::Binary {
                left,
                operator,
                right,
            } if operator.is_equality() => {
                let branch = if operator.is_negative_equality() {
                    branch.opposite()
                } else {
                    branch
                };

                self.narrow_by_equality(*left, *right, branch)?;
                self.narrow_by_equality(*right, *left, branch)?;
            }
            // key in value
            dir::Expression::Binary {
                left,
                operator: dir::BinaryOperator::In,
                right,
            } => {
                self.narrow_by_key_membership(*left, *right, branch);
            }
            // value is T
            dir::Expression::Is { value, target_type } => {
                self.narrow_by_is(*value, *target_type, branch)?;
            }
            // value instanceof Target
            dir::Expression::InstanceOf { value, target } => {
                self.narrow_by_instance(*value, *target, branch)?;
            }
            // expressions without flow effects
            _ => {}
        }

        Ok(())
    }

    /// Narrow flow from one `is` expression.
    ///
    /// Example:
    /// ```ds
    /// value is T
    /// ```
    fn narrow_by_is(
        &mut self,
        value: dir::LocalNodeId<dir::Expression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
        branch: ConditionBranch,
    ) -> CompilerResult<()> {
        let Some(path) = self.flow_path(value) else {
            return Ok(());
        };
        let target = self.node_type_operand(target_type)?;

        match branch {
            ConditionBranch::True => {
                self.narrow_flow_path(path, target);
            }
            ConditionBranch::False => {
                let original = self.node_type_operand(value)?;

                self.narrow_flow_path_excluding(path, original, target);
            }
        }

        Ok(())
    }

    /// Narrow flow from one `"key" in value` expression.
    ///
    /// Example:
    /// ```ds
    /// "name" in value
    /// ```
    fn narrow_by_key_membership(
        &mut self,
        key: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
        branch: ConditionBranch,
    ) {
        if branch == ConditionBranch::False {
            return;
        }
        let Some(path) = self.flow_path(value) else {
            return;
        };
        let Some(key) = self.tree.get(key).static_key() else {
            return;
        };
        let ty = self
            .check
            .inference
            .push_term(TypeTerm::Literal(TypeLiteralTerm::Unknown));
        let member = ShapeMember::Field {
            key,
            ty: ty.into(),
            is_optional: false,
            is_readonly: false,
        };
        let target = self.check.push_shape_type(vec![member].into());
        let target = self.check.inference.push_term(target);

        self.narrow_flow_path(path, target);
    }

    /// Narrow flow from one `value instanceof Target` expression.
    ///
    /// Example:
    /// ```ds
    /// value instanceof Target
    /// ```
    fn narrow_by_instance(
        &mut self,
        value: dir::LocalNodeId<dir::Expression>,
        target: dir::LocalNodeId<dir::Expression>,
        branch: ConditionBranch,
    ) -> CompilerResult<()> {
        let Some(path) = self.flow_path(value) else {
            return Ok(());
        };
        let target = self.node_type_operand(target)?;

        match branch {
            ConditionBranch::True => {
                self.narrow_flow_path(path, target);
            }
            ConditionBranch::False => {
                let original = self.node_type_operand(value)?;

                self.narrow_flow_path_excluding(path, original, target);
            }
        }

        Ok(())
    }

    /// Narrow flow from one equality expression.
    ///
    /// Example:
    /// ```ds
    /// value === undefined
    /// ```
    fn narrow_by_equality(
        &mut self,
        value: dir::LocalNodeId<dir::Expression>,
        target: dir::LocalNodeId<dir::Expression>,
        branch: ConditionBranch,
    ) -> CompilerResult<()> {
        let Some(path) = self.flow_path(value) else {
            return Ok(());
        };
        let Some(target) = self.equality_target_type(target) else {
            return Ok(());
        };

        match branch {
            ConditionBranch::True => {
                self.narrow_flow_path(path, target);
            }
            ConditionBranch::False => {
                let original = self.node_type_operand(value)?;

                self.narrow_flow_path_excluding(path, original, target);
            }
        }

        Ok(())
    }

    /// Return the literal type used by one equality test.
    ///
    /// Example:
    /// ```ds
    /// value === "ready"
    /// ```
    fn equality_target_type(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<TypeOperand> {
        let term = match self.tree.get(id) {
            // literal
            dir::Expression::ScalarLiteral(value) => {
                TypeTerm::Literal(TypeLiteralTerm::Scalar(value.clone()))
            }
            // not a literal equality target
            _ => return None,
        };

        let term = self.check.inference.push_term(term);

        Some(term.into())
    }
}
