use tspp_dir as dir;
use tspp_repository::ProviderError;

use super::{DirModule, IntegerStep};

/// One call whose callee is a member access.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MemberCall<'a> {
    /// The complete call expression.
    pub(crate) expression: dir::LocalNodeId<dir::Expression>,
    /// The member expression used as the callee.
    pub(crate) callee: dir::LocalNodeId<dir::Expression>,
    /// The receiver expression left of the member access.
    pub(crate) receiver: dir::LocalNodeId<dir::Expression>,
    /// The authored generic arguments.
    pub(crate) generic_arguments: &'a [dir::LocalNodeId<dir::GenericArgument>],
    /// The authored call arguments.
    pub(crate) arguments: &'a [dir::LocalNodeId<dir::Argument>],
    /// Whether the call or its member access is optional.
    is_optional: bool,
}

/// One exact nullish test and the sense in which it succeeds.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NullishTest {
    /// The value compared with nullish literals.
    pub(crate) value: dir::LocalNodeId<dir::Expression>,
    /// Whether the test succeeds for non-nullish values.
    pub(crate) is_defined: bool,
}

/// One exact emptiness test over a canonical collection measurement.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EmptinessTest {
    /// The measured collection.
    pub(crate) receiver: dir::LocalNodeId<dir::Expression>,
    /// The canonical length or size member.
    pub(crate) measurement: dir::LanguageMember,
    /// Whether the test succeeds for an empty collection.
    pub(crate) is_empty: bool,
}

/// One exact comparison between a value and a nullish literal.
struct NullishComparison {
    /// The compared value.
    value: dir::LocalNodeId<dir::Expression>,
    /// Whether the comparison succeeds for non-nullish values.
    is_defined: bool,
    /// Whether the comparison distinguishes null.
    checks_null: bool,
    /// Whether the comparison distinguishes undefined.
    checks_undefined: bool,
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

impl MemberCall<'_> {
    /// Return whether the call or its member access is optional.
    pub(crate) fn is_optional(self) -> bool {
        self.is_optional
    }
}

impl DirModule<'_> {
    /// Select one exact test for whether a canonical collection is empty.
    pub(crate) fn emptiness_test(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<EmptinessTest>, ProviderError> {
        let Some((operator, [left, right])) = self.builtin_binary(expression)? else {
            return Ok(None);
        };
        let Some(swapped_operator) = operator.swapped() else {
            return Ok(None);
        };

        // normalize the collection measurement to the left operand
        for (measurement, bound, operator) in [
            (left.source.local_id, right.source.local_id, operator),
            (
                right.source.local_id,
                left.source.local_id,
                swapped_operator,
            ),
        ] {
            // require one canonical collection measurement
            let dir::Expression::Member {
                left: receiver,
                is_optional: false,
                ..
            } = self.view().get(measurement)
            else {
                continue;
            };
            let Some(language_member) = self.language_member(measurement)? else {
                continue;
            };
            if !is_collection_measurement(language_member) {
                continue;
            }

            // read the exact comparison bound
            let Some(dir::Literal::Integer(bound)) = self.scalar_constant(bound)? else {
                continue;
            };

            // map the canonical zero and one bounds to their empty state
            let is_empty = match (operator, bound) {
                (dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict, 0)
                | (dir::BinaryOperator::LessThanOrEqual, 0)
                | (dir::BinaryOperator::LessThan, 1) => true,
                (
                    dir::BinaryOperator::NotEqual
                    | dir::BinaryOperator::NotEqualStrict
                    | dir::BinaryOperator::GreaterThan,
                    0,
                )
                | (dir::BinaryOperator::GreaterThanOrEqual, 1) => false,
                _ => continue,
            };

            return Ok(Some(EmptinessTest {
                receiver: *receiver,
                measurement: language_member,
                is_empty,
            }));
        }

        Ok(None)
    }

    /// Return the call that directly receives one argument value.
    pub(crate) fn argument_call(
        &self,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        // select the direct argument containing the value
        let view = self.view();
        let argument: dir::LocalNodeId<dir::Argument> =
            view.get_parent_for(value)?.try_into_typed().ok()?;
        if view.get(argument).value() != Some(value) {
            return None;
        }

        // require the argument to belong directly to one call
        let expression: dir::LocalNodeId<dir::Expression> =
            view.get_parent_for(argument)?.try_into_typed().ok()?;
        let dir::Expression::Call { arguments, .. } = view.get(expression) else {
            return None;
        };
        if !arguments.contains(&argument) {
            return None;
        }

        Some(expression)
    }

    /// Select one exact test that distinguishes every possible nullish value.
    pub(crate) fn nullish_test(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<NullishTest>, ProviderError> {
        let Some((operator, [left, right])) = self.builtin_binary(expression)? else {
            return Ok(None);
        };

        // combine strict comparisons that cover both nullish literals
        if matches!(operator, dir::BinaryOperator::And | dir::BinaryOperator::Or) {
            let Some(first) = self.nullish_comparison(left.source.local_id)? else {
                return Ok(None);
            };
            let Some(second) = self.nullish_comparison(right.source.local_id)? else {
                return Ok(None);
            };
            let is_defined = operator == dir::BinaryOperator::And;
            if first.is_defined != is_defined
                || second.is_defined != is_defined
                || !self.is_same_computation(first.value, second.value)?
                || !(first.checks_null || second.checks_null)
                || !(first.checks_undefined || second.checks_undefined)
            {
                return Ok(None);
            }

            return Ok(Some(NullishTest {
                value: first.value,
                is_defined,
            }));
        }

        // require one comparison that covers every possible nullish value
        let Some(comparison) = self.nullish_comparison(expression)? else {
            return Ok(None);
        };
        let type_id = self.node_type_id(comparison.value.into_any())?;
        let includes_null = self
            .dir
            .type_includes(type_id, |ty| *ty == dir::Type::Null)?;
        let includes_undefined = self.dir.type_includes_undefined(type_id)?;
        if !comparison.checks_null && includes_null
            || !comparison.checks_undefined && includes_undefined
        {
            return Ok(None);
        }

        Ok(Some(NullishTest {
            value: comparison.value,
            is_defined: comparison.is_defined,
        }))
    }

    /// Select one equality comparison against null or undefined.
    fn nullish_comparison(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<NullishComparison>, ProviderError> {
        let Some((operator, [left, right])) = self.builtin_binary(expression)? else {
            return Ok(None);
        };
        if !operator.is_equality() {
            return Ok(None);
        }

        // normalize the value and nullish literal from either operand order
        let left_literal = self.scalar_constant(left.source.local_id)?;
        let right_literal = self.scalar_constant(right.source.local_id)?;
        let (value, literal) = match (left_literal, right_literal) {
            (
                Some(dir::Literal::Null | dir::Literal::Undefined),
                Some(dir::Literal::Null | dir::Literal::Undefined),
            ) => return Ok(None),
            (_, Some(literal @ (dir::Literal::Null | dir::Literal::Undefined))) => {
                (left.source.local_id, literal)
            }
            (Some(literal @ (dir::Literal::Null | dir::Literal::Undefined)), _) => {
                (right.source.local_id, literal)
            }
            _ => return Ok(None),
        };

        // loose nullish equality covers both singleton values
        let (checks_null, checks_undefined) = if operator.is_strict_equality() {
            (
                literal == dir::Literal::Null,
                literal == dir::Literal::Undefined,
            )
        } else {
            (true, true)
        };

        Ok(Some(NullishComparison {
            value,
            is_defined: operator.is_negative_equality(),
            checks_null,
            checks_undefined,
        }))
    }

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
            expression,
            callee: *left,
            receiver: *receiver,
            generic_arguments,
            arguments,
            is_optional: *is_optional || *is_member_optional,
        })
    }

    /// Return the member call that directly receives one expression.
    pub(crate) fn receiver_call(
        &self,
        receiver: dir::LocalNodeId<dir::Expression>,
    ) -> Option<MemberCall<'_>> {
        let view = self.view();

        // select the direct member access over the receiver
        let callee: dir::LocalNodeId<dir::Expression> =
            view.get_parent_for(receiver)?.try_into_typed().ok()?;
        let dir::Expression::Member { left, .. } = view.get(callee) else {
            return None;
        };
        if *left != receiver {
            return None;
        }

        // require the member access to be called directly
        let expression: dir::LocalNodeId<dir::Expression> =
            view.get_parent_for(callee)?.try_into_typed().ok()?;
        let call = self.member_call(expression)?;
        if call.callee != callee {
            return None;
        }

        Some(call)
    }

    /// Return one direct builtin negation enclosing an expression.
    pub(crate) fn direct_negation(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
        let Some(parent) = self
            .view()
            .get_parent_for(expression)
            .and_then(|parent| parent.try_into_typed::<dir::Expression>().ok())
        else {
            return Ok(None);
        };

        // require builtin boolean negation over the selected expression
        let is_negation = matches!(
            self.builtin_unary(parent)?,
            Some((dir::UnaryOperator::Not, operand)) if operand.source.local_id == expression
        );

        Ok(is_negation.then_some(parent))
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

        // require integral behavior for unary and compound updates
        let Some(operands) = self.builtin_operands(expression.into_any())? else {
            return Ok(None);
        };
        if !operands.iter().all(dir::BuiltinOperand::is_integral) {
            return Ok(None);
        }

        Ok(Some(step))
    }

    /// Return the parameters selected by one call.
    pub(crate) fn call_parameters(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<Vec<dir::FunctionParameterType>>, ProviderError> {
        // select one callable type shared by every runtime arm
        let Some(callable) = self
            .call_decision(expression)?
            .and_then(dir::CallDecision::agreed_callable_type)
        else {
            return Ok(None);
        };

        // read its concrete function parameters
        let signature = self
            .dir
            .callable_signature_type_id(callable)?
            .ok_or_else(|| {
                ProviderError::internal(format!(
                    "call expression {expression:?} selected non-callable type {callable:?}"
                ))
            })?;

        self.dir.signature_parameters(signature).map(Some)
    }

    /// Return the receiver type selected by every arm of one call.
    pub(crate) fn call_receiver_type_id(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::GlobalTypeId>, ProviderError> {
        // read the runtime alternatives
        let Some(decision) = self.call_decision(expression)? else {
            return Ok(None);
        };

        Ok(decision.agreed_receiver_type())
    }

    /// Return the declaration symbol selected by every arm of one call.
    pub(crate) fn call_symbol(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::GlobalSymbolId>, ProviderError> {
        // read the runtime alternatives
        let Some(decision) = self.call_decision(expression)? else {
            return Ok(None);
        };

        Ok(decision.agreed_target_symbol())
    }

    /// Return the declaration symbol selected by one construction.
    pub(crate) fn construct_symbol(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::GlobalSymbolId>, ProviderError> {
        let global = expression.into_global_any(self.id);
        let Some(decision) = self.decisions.construct_decision(global) else {
            if self.node_type(expression.into_any())?.is_error() {
                return Ok(None);
            }

            return Err(ProviderError::internal(format!(
                "construction {global:?} has no construct resolution"
            )));
        };

        Ok(decision.target.symbol())
    }

    /// Iterate expressions with a call resolution.
    pub fn call_expressions(
        &self,
    ) -> impl Iterator<Item = Result<dir::LocalNodeId<dir::Expression>, ProviderError>> + '_ {
        self.decisions
            .expression_entries()
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

    /// Iterate expressions with an operator resolution.
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

    /// Return the operator decision selected for one node.
    pub fn operator_decision(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Result<Option<&dir::OperatorDecision>, ProviderError> {
        // read the required operator decision
        let global = node.into_global(self.id);
        let Some(resolution) = self.decisions.operator_decision(global) else {
            if self.node_type(node)?.is_error() {
                return Ok(None);
            }

            return Err(ProviderError::internal(format!(
                "node {global:?} has no operator resolution"
            )));
        };

        Ok(Some(resolution))
    }

    /// Return one compiler-defined unary operation.
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

    /// Return one compiler-defined binary operation.
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

    /// Return one operand selected for a builtin operator application.
    pub fn builtin_operand(
        &self,
        application: dir::LocalNodeIdAny,
        source: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<&dir::BuiltinOperand>, ProviderError> {
        let global = application.into_global(self.id);
        let source = source.into_global(self.id);
        let Some(resolution) = self.operator_decision(application)? else {
            return Ok(None);
        };
        if resolution.builtin_operands().is_none() {
            return Ok(None);
        }
        let operand = resolution.builtin_operand(source).ok_or_else(|| {
            ProviderError::internal(format!("node {global:?} has no operand {source:?}"))
        })?;

        Ok(Some(operand))
    }

    /// Return the operands selected for one builtin operator application.
    pub fn builtin_operands(
        &self,
        application: dir::LocalNodeIdAny,
    ) -> Result<Option<&[dir::BuiltinOperand]>, ProviderError> {
        let operands = self
            .operator_decision(application)?
            .and_then(dir::OperatorDecision::builtin_operands);

        Ok(operands)
    }

    /// Return the call decision selected for one expression.
    pub(crate) fn call_decision(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<&dir::CallDecision>, ProviderError> {
        // read the required call decision
        let global = node.into_global_any(self.id);
        let Some(resolution) = self.decisions.call_decision(global) else {
            if self.node_type(node.into_any())?.is_error() {
                return Ok(None);
            }

            // constructor calls carry their resolution as a construct decision
            if self.decisions.construct_decision(global).is_some() {
                return Ok(None);
            }

            return Err(ProviderError::internal(format!(
                "call expression {} in module {:?} has no call resolution",
                node.id, self.id
            )));
        };
        if resolution.arms().is_empty() {
            return Err(ProviderError::internal(format!(
                "call expression {global:?} has no runtime alternatives"
            )));
        }

        Ok(Some(resolution))
    }

    /// Return the generic argument bindings shared by every selected call target.
    pub(crate) fn call_generic_bindings(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<&[dir::GenericArgumentBinding]>, ProviderError> {
        // read the runtime alternatives
        let Some(decision) = self.call_decision(node)? else {
            return Ok(None);
        };

        Ok(decision.agreed_generic_arguments())
    }

    /// Return the element type one `Iterator.cloned` call duplicates.
    pub(crate) fn cloned_element(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::GlobalTypeId>, ProviderError> {
        let Some(decision) = self.call_decision(node)? else {
            return Ok(None);
        };

        self.dir.application_argument(decision.return_type(), 1)
    }

    /// Return the member decision selected for one expression.
    pub(super) fn member_decision(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<&dir::MemberDecision>, ProviderError> {
        // read the required member decision
        let global = node.into_global_any(self.id);
        let Some(resolution) = self.decisions.member_decision(global) else {
            if self.node_type(node.into_any())?.is_error() {
                return Ok(None);
            }

            // write targets carry their member resolution inside the assignment
            if self.decisions.assignment_decision(global).is_some() {
                return Ok(None);
            }

            return Err(ProviderError::internal(format!(
                "member expression {} in module {:?} has no member resolution",
                node.id, self.id
            )));
        };

        Ok(Some(resolution))
    }

    /// Return the subscript decision selected for one expression.
    pub fn subscript_decision(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<&dir::SubscriptDecision>, ProviderError> {
        // read the required subscript decision
        let global = node.into_global_any(self.id);
        let Some(resolution) = self.decisions.subscript_decision(global) else {
            if self.node_type(node.into_any())?.is_error() {
                return Ok(None);
            }

            return Err(ProviderError::internal(format!(
                "subscript expression {} in module {:?} has no subscript resolution",
                node.id, self.id
            )));
        };

        Ok(Some(resolution))
    }
}

/// Return whether one canonical member measures a collection with `isEmpty`.
fn is_collection_measurement(member: dir::LanguageMember) -> bool {
    let owner = member.owner;
    if member != owner.member("length") && member != owner.member("size") {
        return false;
    }

    matches!(
        owner,
        dir::LanguageItem::Array
            | dir::LanguageItem::BinaryHeap
            | dir::LanguageItem::ConcurrentMap
            | dir::LanguageItem::ConcurrentSet
            | dir::LanguageItem::Deque
            | dir::LanguageItem::FixedArray
            | dir::LanguageItem::LinkedList
            | dir::LanguageItem::Map
            | dir::LanguageItem::ReadonlyArray
            | dir::LanguageItem::Set
            | dir::LanguageItem::Slice
            | dir::LanguageItem::Slab
            | dir::LanguageItem::SmallArray
            | dir::LanguageItem::SortedMap
            | dir::LanguageItem::SortedSet
            | dir::LanguageItem::String
            | dir::LanguageItem::StringBuilder
            | dir::LanguageItem::StringSlice
    )
}
