use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{FlowPredicate, PathPredicate, WalkState};

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
        condition: &dir::Condition,
        branch: ConditionBranch,
    ) -> CompilerResult<()> {
        // false may stop at any operand
        if branch == ConditionBranch::False && condition.operands.len() != 1 {
            return Ok(());
        }

        // true means every operand succeeded
        for operand in &condition.operands {
            match operand {
                // boolean condition
                dir::ConditionOperand::Expression { condition } => {
                    self.narrow_expression(*condition, branch)?;
                }
                // pattern binding condition
                dir::ConditionOperand::Binding { declarator, .. }
                    if branch == ConditionBranch::True =>
                {
                    self.narrow_let_condition(*declarator)?;
                }
                // failed binding condition
                dir::ConditionOperand::Binding { declarator, .. } => {
                    self.narrow_declarator_pattern(*declarator, false)?;
                }
            }
        }

        Ok(())
    }

    /// Narrow flow from a successful let condition.
    pub(in crate::check) fn narrow_let_condition(
        &mut self,
        declarator: dir::LocalNodeId<dir::Declarator>,
    ) -> CompilerResult<()> {
        self.mark_declarator_assigned(self.tree.get(declarator), false);
        self.narrow_declarator_match(declarator)
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
                operator: dir::BinaryOperator::In,
                right,
                ..
            } => {
                self.narrow_by_guard(id, *right, branch)?;
            }
            // value is T
            dir::Expression::Is { value, .. } => {
                self.narrow_by_guard(id, *value, branch)?;
            }
            // value instanceof Target
            dir::Expression::InstanceOf { value, .. } => {
                self.narrow_by_guard(id, *value, branch)?;
            }
            // expressions without flow effects
            _ => {}
        }

        Ok(())
    }

    /// Narrow flow from one guard expression.
    ///
    /// Example:
    /// ```ds
    /// value is T
    /// ```
    fn narrow_by_guard(
        &mut self,
        guard: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
        branch: ConditionBranch,
    ) -> CompilerResult<()> {
        let Some(path) = self.flow_path(value) else {
            return Ok(());
        };
        let predicate = match branch {
            ConditionBranch::True => FlowPredicate::Guard {
                guard: guard.into_global(self.module),
                is_positive: true,
            },
            ConditionBranch::False => FlowPredicate::Guard {
                guard: guard.into_global(self.module),
                is_positive: false,
            },
        };

        self.apply_flow_predicate(path, predicate);

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
        let Some(target) = self.equality_target_type(target)? else {
            return Ok(());
        };

        let predicate = match branch {
            ConditionBranch::True => PathPredicate::Is(target),
            ConditionBranch::False => PathPredicate::IsNot(target),
        };
        self.apply_path_predicate(path, predicate)?;
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
        predicate: PathPredicate,
    ) -> CompilerResult<()> {
        // require an existing member flow path
        let Some(path) = self.flow_path(value) else {
            return Ok(());
        };
        let Some((base_path, key)) = path.split_last() else {
            return Ok(());
        };
        if self.member_base_expression(value).is_none() {
            return Ok(());
        }

        // narrow base with a structural member predicate
        self.apply_member_path_predicate(base_path, key, predicate)
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

        Ok(Some(self.intern_type(ty)?))
    }
}
