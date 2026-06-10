use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{NarrowPredicate, TypeLiteralTerm, TypeOperand, TypeTerm, WalkState};

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
                self.narrow_by_key_membership(*left, *right, branch)?;
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
        let source = self.node_type_operand(value)?;

        match branch {
            ConditionBranch::True => {
                self.narrow_flow_path_by(path, source, NarrowPredicate::Is(target));
            }
            ConditionBranch::False => {
                self.narrow_flow_path_by(path, source, NarrowPredicate::IsNot(target));
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
    ) -> CompilerResult<()> {
        if branch == ConditionBranch::False {
            return Ok(());
        }
        let Some(path) = self.flow_path(value) else {
            return Ok(());
        };
        let Some(key) = self.tree.get(key).static_key() else {
            return Ok(());
        };
        let source = self.expression_type_operand(value)?;
        self.narrow_flow_path_by(path, source, NarrowPredicate::HasKey(key));

        Ok(())
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
        let source = self.node_type_operand(value)?;

        match branch {
            ConditionBranch::True => {
                self.narrow_flow_path_by(path, source, NarrowPredicate::Is(target));
            }
            ConditionBranch::False => {
                self.narrow_flow_path_by(path, source, NarrowPredicate::IsNot(target));
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

        let source = self.node_type_operand(value)?;
        let predicate = match branch {
            ConditionBranch::True => NarrowPredicate::Is(target),
            ConditionBranch::False => NarrowPredicate::IsNot(target),
        };
        self.narrow_flow_path_by(path, source, predicate);
        self.narrow_parent_by_member_predicate(value, predicate)?;

        Ok(())
    }

    /// Narrow a base flow path from a member predicate.
    ///
    /// Example:
    /// ```ds
    /// if (state.kind == "pending") { state.reactions }
    /// ```
    fn narrow_parent_by_member_predicate(
        &mut self,
        value: dir::LocalNodeId<dir::Expression>,
        predicate: NarrowPredicate,
    ) -> CompilerResult<()> {
        // require an existing member flow path
        let Some(path) = self.flow_path(value) else {
            return Ok(());
        };
        let Some((base_path, key)) = path.split_last() else {
            return Ok(());
        };
        let Some(base) = self.member_base_expression(value) else {
            return Ok(());
        };

        // narrow base with a structural member predicate
        let source = self.expression_type_operand(base)?;
        self.narrow_base_flow_path_by_member(base_path, source, key, predicate);

        Ok(())
    }

    /// Return the base expression for one member path expression.
    fn member_base_expression(
        &self,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        match self.tree.get(value) {
            // value.member
            dir::Expression::Member { left, .. }
            // value.#member
            | dir::Expression::PrivateMember { left, .. }
            // value[index]
            | dir::Expression::Index { left, .. } => Some(*left),
            // not a member path
            _ => None,
        }
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
        let literal = match self.tree.get(id) {
            // literal
            dir::Expression::ScalarLiteral(value) => value,
            // not a literal equality target
            _ => return None,
        };
        let term = self.equality_literal_target(literal);

        let term = self.check.inference.push_term(term);

        Some(term.into())
    }

    /// Return the target type used by one literal equality test.
    fn equality_literal_target(&mut self, literal: &dir::ScalarLiteral) -> TypeTerm {
        match literal {
            // nullish equality uses canonical nullish types
            dir::ScalarLiteral::Null => TypeTerm::Literal(TypeLiteralTerm::Null),
            dir::ScalarLiteral::Undefined => TypeTerm::Literal(TypeLiteralTerm::Undefined),
            // other scalar equality keeps the literal exact
            literal => TypeTerm::Literal(TypeLiteralTerm::Scalar(literal.clone())),
        }
    }
}
