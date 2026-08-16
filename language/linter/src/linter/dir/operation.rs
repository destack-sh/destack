use destack_dir as dir;
use destack_repository::ProviderError;

use super::{DirModule, IntegerStep};

/// One call whose callee is a member access.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MemberCall<'a> {
    /// The member expression used as the callee.
    pub(crate) callee: dir::LocalNodeId<dir::Expression>,
    /// The receiver expression left of the member access.
    pub(crate) receiver: dir::LocalNodeId<dir::Expression>,
    /// The authored generic arguments.
    pub(crate) generic_arguments: &'a [dir::LocalNodeId<dir::GenericArgument>],
    /// The authored call arguments.
    pub(crate) arguments: &'a [dir::LocalNodeId<dir::Argument>],
    /// Whether the call itself is optional.
    pub(crate) is_optional: bool,
    /// Whether the selected member access is optional.
    pub(crate) is_member_optional: bool,
}

/// One authored assignment to a direct place.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PlaceAssignment {
    /// The written place expression.
    pub(crate) target: dir::LocalNodeId<dir::Expression>,
    /// The authored assignment operator.
    pub(crate) operator: dir::AssignOperator,
    /// The assigned value expression.
    pub(crate) value: dir::LocalNodeId<dir::Expression>,
}

impl DirModule<'_> {
    /// Return one call whose callee is a member access.
    pub(crate) fn member_call(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<MemberCall<'_>> {
        // select the call expression
        let dir::Expression::Call {
            left,
            generic_arguments,
            arguments,
            is_optional,
            ..
        } = self.view().get(expression)
        else {
            return None;
        };

        // require a member access as its callee
        let dir::Expression::Member {
            left: receiver,
            is_optional: is_member_optional,
            ..
        } = self.view().get(*left)
        else {
            return None;
        };

        Some(MemberCall {
            callee: *left,
            receiver: *receiver,
            generic_arguments,
            arguments,
            is_optional: *is_optional,
            is_member_optional: *is_member_optional,
        })
    }

    /// Return one authored assignment to a direct place.
    pub(crate) fn place_assignment(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<PlaceAssignment> {
        // select an assignment expression
        let view = self.view();
        let dir::Expression::Assign {
            left,
            operator,
            right,
        } = view.get(expression)
        else {
            return None;
        };

        // require one direct place target
        let dir::AssignPattern::Place { expression: target } = view.get(*left) else {
            return None;
        };

        Some(PlaceAssignment {
            target: *target,
            operator: *operator,
            value: *right,
        })
    }

    /// Select an exact integral unit update and its target.
    pub(crate) fn integer_update(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<IntegerStep>, ProviderError> {
        let view = self.view();
        let step = match view.get(expression) {
            // recognize unary updates
            dir::Expression::Unary { operator, right } => match operator {
                dir::UnaryOperator::PostIncrement | dir::UnaryOperator::PreIncrement => {
                    IntegerStep::Increment(*right)
                }
                dir::UnaryOperator::PostDecrement | dir::UnaryOperator::PreDecrement => {
                    IntegerStep::Decrement(*right)
                }
                _ => return Ok(None),
            },

            // recognize direct assignment updates
            dir::Expression::Assign { .. } => {
                let Some(assignment) = self.place_assignment(expression) else {
                    return Ok(None);
                };

                // compose expanded assignments from their assigned integer step
                if assignment.operator == dir::AssignOperator::Assign {
                    let Some(step) = self.integer_step(assignment.value)? else {
                        return Ok(None);
                    };
                    let repeated = match step {
                        IntegerStep::Increment(repeated) | IntegerStep::Decrement(repeated) => {
                            repeated
                        }
                    };
                    if !self.is_same_computation(assignment.target, repeated)? {
                        return Ok(None);
                    }

                    let step = match step {
                        IntegerStep::Increment(_) => IntegerStep::Increment(assignment.target),
                        IntegerStep::Decrement(_) => IntegerStep::Decrement(assignment.target),
                    };

                    return Ok(Some(step));
                }

                // classify compound assignments with exact signed unit values
                match (
                    assignment.operator,
                    self.integral_constant(assignment.value)?,
                ) {
                    (dir::AssignOperator::AddAssign, Some(1))
                    | (dir::AssignOperator::SubtractAssign, Some(-1)) => {
                        IntegerStep::Increment(assignment.target)
                    }
                    (dir::AssignOperator::AddAssign, Some(-1))
                    | (dir::AssignOperator::SubtractAssign, Some(1)) => {
                        IntegerStep::Decrement(assignment.target)
                    }
                    _ => return Ok(None),
                }
            }
            _ => return Ok(None),
        };

        // require checked integral behavior for unary and compound updates
        let Some(operands) = self.builtin_operands(expression.into_any())? else {
            return Ok(None);
        };
        if !operands.iter().all(dir::BuiltinOperand::is_integral) {
            return Ok(None);
        }

        Ok(Some(step))
    }

    /// Iterate expressions with a checked call resolution.
    pub fn call_expressions(
        &self,
    ) -> impl Iterator<Item = Result<dir::LocalNodeId<dir::Expression>, ProviderError>> + '_ {
        self.decisions
            .decision_entries()
            .filter(|(_, decision)| matches!(decision, dir::Decision::Call(_)))
            .map(|(node, _)| {
                if node.module_id != self.id {
                    return Err(ProviderError::internal(format!(
                        "call resolution {node:?} belongs to another module"
                    )));
                }

                node.local_id
                    .try_into_typed::<dir::Expression>()
                    .map_err(ProviderError::internal)
            })
    }

    /// Iterate expressions with a checked operator resolution.
    pub fn operator_expressions(
        &self,
    ) -> impl Iterator<Item = Result<dir::LocalNodeId<dir::Expression>, ProviderError>> + '_ {
        self.decisions
            .decision_entries()
            .filter(|(_, decision)| matches!(decision, dir::Decision::Operator(_)))
            .filter(|(node, _)| node.local_id.ty == dir::NodeType::Expression)
            .map(|(node, _)| {
                if node.module_id != self.id {
                    return Err(ProviderError::internal(format!(
                        "operator resolution {node:?} belongs to another module"
                    )));
                }

                node.local_id
                    .try_into_typed::<dir::Expression>()
                    .map_err(ProviderError::internal)
            })
    }

    /// Return the operator decision selected for one checked node.
    pub fn operator_decision(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Result<Option<&dir::OperatorDecision>, ProviderError> {
        // read the required checked operator decision
        let global = node.into_global(self.id);
        let Some(resolution) = self.decisions.operator_decision(global) else {
            if self.node_type(node)?.is_error() {
                return Ok(None);
            }

            return Err(ProviderError::internal(format!(
                "checked operator node {} in module {:?} has no operator resolution",
                node.id, self.id
            )));
        };

        Ok(Some(resolution))
    }

    /// Return one checked compiler-defined unary operation.
    pub fn builtin_unary(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<(dir::UnaryOperator, &dir::BuiltinOperand)>, ProviderError> {
        if !matches!(self.view().get(expression), dir::Expression::Unary { .. }) {
            return Ok(None);
        }

        let resolution = self.operator_decision(expression.into_any())?;
        let operation = resolution.and_then(dir::OperatorDecision::builtin_unary);

        Ok(operation)
    }

    /// Return one checked compiler-defined binary operation.
    pub fn builtin_binary(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<(dir::BinaryOperator, &[dir::BuiltinOperand; 2])>, ProviderError> {
        if !matches!(self.view().get(expression), dir::Expression::Binary { .. }) {
            return Ok(None);
        }

        let resolution = self.operator_decision(expression.into_any())?;
        let operation = resolution.and_then(dir::OperatorDecision::builtin_binary);

        Ok(operation)
    }

    /// Return one operand selected for a checked builtin operator application.
    pub fn builtin_operand(
        &self,
        application: dir::LocalNodeIdAny,
        source: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<&dir::BuiltinOperand>, ProviderError> {
        let source = source.into_global(self.id);
        let Some(resolution) = self.operator_decision(application)? else {
            return Ok(None);
        };
        if resolution.builtin_operands().is_none() {
            return Ok(None);
        }
        let operand = resolution.builtin_operand(source).ok_or_else(|| {
            ProviderError::internal(format!(
                "checked builtin operator node {} in module {:?} has no operand {source:?}",
                application.id, self.id
            ))
        })?;

        Ok(Some(operand))
    }

    /// Return the operands selected for one checked builtin operator application.
    pub fn builtin_operands(
        &self,
        application: dir::LocalNodeIdAny,
    ) -> Result<Option<&[dir::BuiltinOperand]>, ProviderError> {
        let operands = self
            .operator_decision(application)?
            .and_then(dir::OperatorDecision::builtin_operands);

        Ok(operands)
    }

    /// Return the call decision selected for one checked expression.
    pub(super) fn call_decision(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<&dir::CallDecision>, ProviderError> {
        // read the required checked call decision
        let global = node.into_global_any(self.id);
        let Some(resolution) = self.decisions.call_decision(global) else {
            if self.node_type(node.into_any())?.is_error() {
                return Ok(None);
            }

            return Err(ProviderError::internal(format!(
                "checked call expression {} in module {:?} has no call resolution",
                node.id, self.id
            )));
        };

        Ok(Some(resolution))
    }

    /// Return the member decision selected for one checked expression.
    pub(super) fn member_decision(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<&dir::MemberDecision>, ProviderError> {
        // read the required checked member decision
        let global = node.into_global_any(self.id);
        let Some(resolution) = self.decisions.member_decision(global) else {
            if self.node_type(node.into_any())?.is_error() {
                return Ok(None);
            }

            return Err(ProviderError::internal(format!(
                "checked member expression {} in module {:?} has no member resolution",
                node.id, self.id
            )));
        };

        Ok(Some(resolution))
    }

    /// Return the subscript decision selected for one checked expression.
    pub fn subscript_decision(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<&dir::SubscriptDecision>, ProviderError> {
        // read the required checked subscript decision
        let global = node.into_global_any(self.id);
        let Some(resolution) = self.decisions.subscript_decision(global) else {
            if self.node_type(node.into_any())?.is_error() {
                return Ok(None);
            }

            return Err(ProviderError::internal(format!(
                "checked subscript expression {} in module {:?} has no subscript resolution",
                node.id, self.id
            )));
        };

        Ok(Some(resolution))
    }
}
