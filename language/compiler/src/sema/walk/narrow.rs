use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::{CheckState, FlowPredicate};

/// The condition branch being entered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum ConditionBranch {
    /// The condition is known true.
    True,
    /// The condition is known false.
    False,
}

impl ConditionBranch {
    /// Return the opposite control branch.
    pub(in crate::sema) fn opposite(self) -> Self {
        match self {
            // true branch
            Self::True => Self::False,
            // false branch
            Self::False => Self::True,
        }
    }

    /// Return the branch entered by one short-circuit operator's right operand.
    pub(in crate::sema) fn from_short_circuit(operator: dir::BinaryOperator) -> Option<Self> {
        match operator {
            dir::BinaryOperator::And => Some(Self::True),
            dir::BinaryOperator::Or | dir::BinaryOperator::Coalesce => Some(Self::False),
            _ => None,
        }
    }
}

impl CheckState<'_> {
    /// Narrow flow from one condition.
    ///
    /// Example:
    /// ```tspp
    /// if value is T { value }
    /// ```
    pub(in crate::sema) fn narrow_condition(
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
    pub(in crate::sema) fn narrow_let_condition(
        &mut self,
        declarator: dir::LocalNodeId<dir::Declarator>,
    ) -> CompilerResult<()> {
        let node = self.module(self.module_id).view().get(declarator).clone();
        self.assign_declarator_bindings(&node, false);
        self.narrow_declarator_match(declarator)
    }

    /// Narrow flow from one expression condition.
    ///
    /// Example:
    /// ```tspp
    /// if value !== undefined { value }
    /// ```
    pub(in crate::sema) fn narrow_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        branch: ConditionBranch,
    ) -> CompilerResult<()> {
        let node = self.module(self.module_id).view().get(id).clone();
        match node {
            // !value
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Not,
                right,
            } => {
                self.narrow_expression(right, branch.opposite())?;
            }
            // left && right
            dir::Expression::Binary {
                left,
                operator: dir::BinaryOperator::And,
                right,
            } if branch == ConditionBranch::True => {
                self.narrow_expression(left, branch)?;
                self.narrow_expression(right, branch)?;
            }
            // left || right
            dir::Expression::Binary {
                left,
                operator: dir::BinaryOperator::Or,
                right,
            } if branch == ConditionBranch::False => {
                self.narrow_expression(left, branch)?;
                self.narrow_expression(right, branch)?;
            }
            // left === right
            dir::Expression::Binary {
                left,
                operator,
                right,
            } if operator.is_equality() => {
                let is_equal = match (operator.is_negative_equality(), branch) {
                    (false, ConditionBranch::True) | (true, ConditionBranch::False) => true,
                    (false, ConditionBranch::False) | (true, ConditionBranch::True) => false,
                };
                self.narrow_by_equality(id.into_global_any(self.module_id), left, right, is_equal);
            }
            // key in value
            dir::Expression::Binary {
                operator: dir::BinaryOperator::In,
                right,
                ..
            } => {
                self.narrow_by_guard(id, right, branch);
            }
            // value is T
            dir::Expression::Is { value, .. } => {
                self.narrow_by_guard(id, value, branch);
            }
            // value instanceof Target
            dir::Expression::InstanceOf { value, .. } => {
                self.narrow_by_guard(id, value, branch);
            }
            // expressions without flow effects
            _ => {}
        }

        Ok(())
    }

    /// Narrow flow from one guard expression.
    ///
    /// Example:
    /// ```tspp
    /// value is T
    /// ```
    fn narrow_by_guard(
        &mut self,
        guard: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
        branch: ConditionBranch,
    ) {
        let Some(path) = self.lexical_access_path(value) else {
            return;
        };
        let predicate = FlowPredicate::Guard {
            guard: guard.into_global(self.module_id),
            is_positive: branch == ConditionBranch::True,
        };

        self.flow.apply_narrowing(path, predicate);
    }

    /// Exclude one previously matched pattern from a flow path.
    pub(in crate::sema) fn exclude_match_pattern(
        &mut self,
        path: dir::AccessPath,
        pattern: dir::GlobalNodeId<dir::Pattern>,
    ) {
        let predicate = FlowPredicate::Pattern {
            pattern,
            is_positive: false,
        };
        self.flow.apply_narrowing(path, predicate);
    }

    /// Narrow flow from one equality expression.
    ///
    /// Example:
    /// ```tspp
    /// value === undefined
    /// ```
    pub(in crate::sema) fn narrow_by_equality(
        &mut self,
        operation: dir::GlobalNodeIdAny,
        left: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
        is_equal: bool,
    ) {
        let predicate = FlowPredicate::Equality {
            operation,
            is_equal,
        };

        // record one directed fact for each stable operand access
        let mut paths = Vec::new();
        for operand in [left, right] {
            let Some(path) = self.lexical_access_path(operand) else {
                continue;
            };
            if !paths.contains(&path) {
                paths.push(path);
            }
        }

        for path in paths {
            self.flow.apply_narrowing(path, predicate);
        }
    }
}
