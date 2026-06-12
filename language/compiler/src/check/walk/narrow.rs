use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{NarrowPredicate, WalkState};

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
                self.narrow_declarator_match(*declarator)?;
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
                let expression = *expression;
                self.narrow_expression(expression, branch)?;
            }
            // !value
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Not,
                right,
            } => {
                let right = *right;
                self.narrow_expression(right, branch.opposite())?;
            }
            // left && right
            dir::Expression::Binary {
                left,
                operator: dir::BinaryOperator::And,
                right,
            } if branch == ConditionBranch::True => {
                let (left, right) = (*left, *right);
                self.narrow_expression(left, branch)?;
                self.narrow_expression(right, branch)?;
            }
            // left || right
            dir::Expression::Binary {
                left,
                operator: dir::BinaryOperator::Or,
                right,
            } if branch == ConditionBranch::False => {
                let (left, right) = (*left, *right);
                self.narrow_expression(left, branch)?;
                self.narrow_expression(right, branch)?;
            }
            // left === right
            dir::Expression::Binary {
                left,
                operator,
                right,
            } if operator.is_equality() => {
                let (left, right) = (*left, *right);
                let branch = if operator.is_negative_equality() {
                    branch.opposite()
                } else {
                    branch
                };
                self.narrow_by_equality(left, right, branch)?;
                self.narrow_by_equality(right, left, branch)?;
            }
            // key in value
            dir::Expression::Binary {
                left,
                operator: dir::BinaryOperator::In,
                right,
            } => {
                let (left, right) = (*left, *right);
                self.narrow_by_key_membership(left, right, branch)?;
            }
            // value is T
            dir::Expression::Is { value, target_type } => {
                let (value, target_type) = (*value, *target_type);
                self.narrow_by_is(value, target_type, branch)?;
            }
            // value instanceof Target
            dir::Expression::InstanceOf { value, target } => {
                let (value, target) = (*value, *target);
                self.narrow_by_instance(value, target, branch)?;
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
        let target = self.walk_type_expression(target_type)?;
        let source = self.node_type(value)?;
        let predicate = match branch {
            ConditionBranch::True => NarrowPredicate::Is(target),
            ConditionBranch::False => NarrowPredicate::IsNot(target),
        };

        self.narrow_flow_path_by(path, source, value.into_any(), predicate)
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
        let source = self.expression_type(value)?;

        self.narrow_flow_path_by(path, source, value.into_any(), NarrowPredicate::HasKey(key))
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
        let target = self.node_type(target)?;
        let source = self.node_type(value)?;
        let predicate = match branch {
            ConditionBranch::True => NarrowPredicate::Is(target),
            ConditionBranch::False => NarrowPredicate::IsNot(target),
        };

        self.narrow_flow_path_by(path, source, value.into_any(), predicate)
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
        let Some(target) = self.equality_target_type(target)? else {
            return Ok(());
        };

        let source = self.node_type(value)?;
        let predicate = match branch {
            ConditionBranch::True => NarrowPredicate::Is(target),
            ConditionBranch::False => NarrowPredicate::IsNot(target),
        };
        self.narrow_flow_path_by(path, source, value.into_any(), predicate)?;
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
        let source = self.expression_type(base)?;
        self.narrow_base_flow_path_by_member(base_path, source, base.into_any(), key, predicate)
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
    /// Nullish targets use the canonical nullish types.
    ///
    /// Example:
    /// ```ds
    /// value === "ready"
    /// ```
    fn equality_target_type(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let ty = match self.tree.get(id) {
            dir::Expression::ScalarLiteral(dir::ScalarLiteral::Null) => dir::Type::Null,
            dir::Expression::ScalarLiteral(dir::ScalarLiteral::Undefined) => dir::Type::Undefined,
            // other scalar equality keeps the literal exact
            dir::Expression::ScalarLiteral(value) => dir::Type::Literal(*value),
            // not a literal equality target
            _ => return Ok(None),
        };

        Ok(Some(self.push_type(ty, id.into_any())?))
    }
}
