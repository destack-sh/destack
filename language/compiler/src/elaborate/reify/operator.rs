use destack_dir as dir;
use destack_dir::LanguageItem;
use dir::{
    Argument, BinaryOperator, DispatchResolution, DispatchTarget, Expression, LocalNodeId,
    LocalTypeId, NodeType, Resolution, UnaryOperator,
};

use crate::elaborate::ElaborateState;
use crate::{Compiler, ElaborateError, ElaborateResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Reify an operator into resolved method calls ("deload").
    /// - Builtin: keep as primitive operator
    /// - Static: emit direct method call `a.add(b)`
    /// - Dynamic: emit type dispatch match expression
    pub(super) fn reify_operator_expression(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> ElaborateResult<()> {
        // load the operator resolution for this node
        let Some(resolution) = state
            .types
            .resolution(expression_id.into_global_any(state.module_id))
            .cloned()
        else {
            return Ok(());
        };
        let Resolution::Dispatch(DispatchResolution::Static {
            receiver,
            target: candidate,
        }) = resolution
        else {
            return Ok(());
        };

        // only rewrite operator expressions
        let expression = state.tree.get(expression_id).clone();
        match expression {
            Expression::Binary {
                left,
                operator,
                right,
            } => self.reify_static_binary_operator(
                state,
                expression_id,
                left,
                operator,
                right,
                receiver,
                candidate,
            )?,
            Expression::Unary { operator, right } => {
                self.reify_static_unary_operator(state, expression_id, operator, right, candidate)?
            }
            _ => {}
        }

        Ok(())
    }

    /// Reify a static binary operator into a method call shape.
    fn reify_static_binary_operator(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        operator: BinaryOperator,
        right: LocalNodeId<Expression>,
        receiver: Option<LocalTypeId>,
        candidate: DispatchTarget,
    ) -> ElaborateResult<()> {
        // rewrite `a != b` into `!a.equal(b)` with the static call nested inside
        if operator == BinaryOperator::NotEqual {
            let call_id = self.build_static_operator_call(
                state,
                expression_id,
                left,
                &[right],
                LanguageItem::Equal,
                receiver,
                candidate.clone(),
            );
            let receiver = state
                .types
                .get_declared_or_inferred_type_id(call_id.into_global_any(state.module_id))
                .or(receiver);

            state.tree.replace(
                expression_id,
                Expression::Unary {
                    operator: UnaryOperator::Not,
                    right: call_id,
                },
            );
            self.set_builtin_resolution_for_node(state, expression_id, receiver);
            return Ok(());
        }

        // rewrite `< <= > >=` via `compare` and `Ordering`
        if let Some((comparison_operator, ordering_member_name)) =
            comparison_rewrite_for_operator(operator)
        {
            let compare_call_id = self.build_static_operator_call(
                state,
                expression_id,
                left,
                &[right],
                LanguageItem::Compare,
                receiver,
                candidate.clone(),
            );

            let compare_type_id = candidate
                .signature
                .as_ref()
                .and_then(|signature| signature.return_type)
                .ok_or(ElaborateError::UnsupportedConstruct {
                    anchor: state.module_id.into(),
                })?;

            let ordering_reference_id = self.build_ordering_reference(
                state,
                expression_id,
                compare_type_id,
                ordering_member_name,
            )?;

            state.tree.replace(
                expression_id,
                Expression::Binary {
                    left: compare_call_id,
                    operator: comparison_operator,
                    right: ordering_reference_id,
                },
            );
            self.set_builtin_resolution_for_node(state, expression_id, Some(compare_type_id));
            return Ok(());
        }

        // rewrite normal overloadable operators into direct method calls
        let Some(operator_symbol) = operator.language_item() else {
            return Ok(());
        };
        let call = self.operator_call_expression(
            state,
            expression_id,
            left,
            &[right],
            operator_symbol,
            candidate.symbol,
        );
        state.tree.replace(expression_id, call);

        Ok(())
    }

    /// Reify a static unary operator into a method call shape.
    fn reify_static_unary_operator(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        operator: UnaryOperator,
        right: LocalNodeId<Expression>,
        candidate: DispatchTarget,
    ) -> ElaborateResult<()> {
        let Some(operator_symbol) = operator.language_item() else {
            return Ok(());
        };

        let call = self.operator_call_expression(
            state,
            expression_id,
            right,
            &[],
            operator_symbol,
            candidate.symbol,
        );
        state.tree.replace(expression_id, call);

        Ok(())
    }

    /// Build a static operator call node and attach static resolution.
    fn build_static_operator_call(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        receiver: LocalNodeId<Expression>,
        arguments: &[LocalNodeId<Expression>],
        operator_symbol: LanguageItem,
        resolution_receiver: Option<LocalTypeId>,
        candidate: DispatchTarget,
    ) -> LocalNodeId<Expression> {
        // create the call expression in the current scope
        let call_id = state.tree.reserve_from(
            NodeType::Expression,
            origin_id.into_any(),
            state.tree.get_scope(origin_id),
            None,
            Some(dir::ProvenanceReason::Reified),
        );
        let call = self.operator_call_expression(
            state,
            origin_id,
            receiver,
            arguments,
            operator_symbol,
            candidate.symbol,
        );
        let call_id = state.tree.insert_as_owner(call_id, call);

        // copy the inferred return type when available
        if let Some(return_type) = candidate
            .signature
            .as_ref()
            .and_then(|signature| signature.return_type)
        {
            state
                .types
                .set_inferred_type(call_id.into_global_any(state.module_id), return_type);
        }

        // attach static resolution for lower call emission
        state.types.set_resolution(
            call_id.into_global_any(state.module_id),
            Resolution::Dispatch(DispatchResolution::Static {
                receiver: resolution_receiver,
                target: candidate,
            }),
        );

        call_id
    }

    /// Build an operator call expression of the form `receiver.member(args...)`.
    fn operator_call_expression(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        receiver: LocalNodeId<Expression>,
        arguments: &[LocalNodeId<Expression>],
        operator_symbol: LanguageItem,
        target_symbol: dir::GlobalSymbolId,
    ) -> Expression {
        // create the member expression
        let member_name = self.operator_member_name(operator_symbol);
        let member_id = state.tree.reserve_from(
            NodeType::Expression,
            origin_id.into_any(),
            state.tree.get_scope(origin_id),
            None,
            Some(dir::ProvenanceReason::Reified),
        );
        let member_id = state.tree.insert_as_owner(
            member_id,
            Expression::Member {
                left: receiver,
                name: Some(member_name),
            },
        );

        // annotate member type when available
        if let Some(member_type_id) = state.types.get_value_type_id(target_symbol) {
            state
                .types
                .set_inferred_type(member_id.into_global_any(state.module_id), member_type_id);
        }

        // create call arguments
        let mut lowered_arguments = Vec::with_capacity(arguments.len());
        for argument in arguments {
            let argument_id = state.tree.reserve_from(
                NodeType::Argument,
                origin_id.into_any(),
                state.tree.get_scope(origin_id),
                None,
                Some(dir::ProvenanceReason::Reified),
            );
            let argument_id = state
                .tree
                .insert_as_owner(argument_id, Argument::Positional { value: *argument });
            lowered_arguments.push(argument_id);
        }

        Expression::Call {
            left: member_id,
            generic_arguments: Vec::new(),
            arguments: lowered_arguments,
        }
    }

    /// Build a reference to an `Ordering` enum member.
    fn build_ordering_reference(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        _ordering_type_id: LocalTypeId,
        _member_name: &str,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        let _ = origin_id;

        Err(ElaborateError::UnsupportedConstruct {
            anchor: state.module_id.into(),
        })
    }

    /// Replace the node resolution with a builtin resolution.
    fn set_builtin_resolution_for_node(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        receiver: Option<LocalTypeId>,
    ) {
        state.types.set_resolution(
            expression_id.into_global_any(state.module_id),
            Resolution::Dispatch(DispatchResolution::Builtin { receiver }),
        );
    }

    /// Return the operator member name for a language symbol.
    fn operator_member_name(&self, operator_symbol: LanguageItem) -> dir::StringId {
        // match analyze operator key lowering: `Add` -> `add`
        let export_name = operator_symbol.export_name();
        let mut chars = export_name.chars();
        let first_char = chars.next().unwrap_or_default();

        let mut member_name = String::new();
        member_name.push(first_char.to_ascii_lowercase());
        member_name.push_str(chars.as_str());

        dir::StringId::for_text(&member_name)
    }
}

/// Return comparison rewrite details for overloaded compare operators.
fn comparison_rewrite_for_operator(
    operator: BinaryOperator,
) -> Option<(BinaryOperator, &'static str)> {
    match operator {
        BinaryOperator::LessThan => Some((BinaryOperator::Equal, "Less")),
        BinaryOperator::LessThanOrEqual => Some((BinaryOperator::NotEqual, "Greater")),
        BinaryOperator::GreaterThan => Some((BinaryOperator::Equal, "Greater")),
        BinaryOperator::GreaterThanOrEqual => Some((BinaryOperator::NotEqual, "Less")),
        _ => None,
    }
}
