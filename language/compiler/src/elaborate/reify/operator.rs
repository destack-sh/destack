use destack_builtin::LanguageSymbol;
use destack_dir as dir;
use dir::{
    Argument, BinaryOperator, Expression, LocalNodeId, LocalTypeId, NodeType, Path, Resolution,
    ResolutionCandidate, StaticKey, UnaryOperator,
};

use crate::elaborate::common::ElaborateState;
use crate::{Compiler, ElaborateError, ElaborateResult, OperatorLanguageSymbolExt};

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
        let Some(resolution_id) = state
            .types
            .get_resolution_for_node(expression_id.into_global_any(state.ctx.module_id))
        else {
            return Ok(());
        };
        let resolution = state.types.get_resolution(resolution_id).clone();
        let Resolution::Static {
            receiver,
            candidate,
        } = resolution
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
        candidate: ResolutionCandidate,
    ) -> ElaborateResult<()> {
        // rewrite `a != b` into `!a.equal(b)` with the static call nested inside
        if operator == BinaryOperator::NotEqual {
            let call_id = self.build_static_operator_call(
                state,
                expression_id,
                left,
                &[right],
                LanguageSymbol::Equal,
                receiver,
                candidate.clone(),
            );
            let receiver = state
                .types
                .get_declared_or_inferred_type_id(call_id.into_global_any(state.ctx.module_id))
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
                LanguageSymbol::Compare,
                receiver,
                candidate.clone(),
            );

            let compare_type_id = candidate
                .resolved_signature
                .as_ref()
                .and_then(|signature| signature.return_type)
                .ok_or(ElaborateError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(state.ctx.module_id)
                        .into_anchored(Some(state.ctx.profile)),
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
        let Some(operator_symbol) = operator.language_symbol() else {
            return Ok(());
        };
        let call = self.operator_call_expression(
            state,
            expression_id,
            left,
            &[right],
            operator_symbol,
            candidate.target_symbol,
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
        candidate: ResolutionCandidate,
    ) -> ElaborateResult<()> {
        let Some(operator_symbol) = operator.language_symbol() else {
            return Ok(());
        };

        let call = self.operator_call_expression(
            state,
            expression_id,
            right,
            &[],
            operator_symbol,
            candidate.target_symbol,
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
        operator_symbol: LanguageSymbol,
        resolution_receiver: Option<LocalTypeId>,
        candidate: ResolutionCandidate,
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
            candidate.target_symbol,
        );
        let call_id = state.tree.insert_as_owner(call_id, call);

        // copy the inferred return type when available
        if let Some(return_type) = candidate
            .resolved_signature
            .as_ref()
            .and_then(|signature| signature.return_type)
        {
            state
                .types
                .set_inferred_type(call_id.into_global_any(state.ctx.module_id), return_type);
        }

        // attach static resolution for lower call emission
        let resolution_id = state.types.insert_resolution(Resolution::Static {
            receiver: resolution_receiver,
            candidate,
        });
        state
            .types
            .set_resolution_for_node(call_id.into_global_any(state.ctx.module_id), resolution_id);

        call_id
    }

    /// Build an operator call expression of the form `receiver.member(args...)`.
    fn operator_call_expression(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        receiver: LocalNodeId<Expression>,
        arguments: &[LocalNodeId<Expression>],
        operator_symbol: LanguageSymbol,
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
                generic_arguments: Vec::new(),
            },
        );

        // annotate member type when available
        if let Some(member_type_id) = state.types.get_value_type_id(target_symbol) {
            state.types.set_inferred_type(
                member_id.into_global_any(state.ctx.module_id),
                member_type_id,
            );
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
        ordering_type_id: LocalTypeId,
        member_name: &str,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // resolve `Ordering.<member>` symbol
        let ordering_symbol = self.language_symbol(state.ctx.profile, LanguageSymbol::Ordering);
        let member_key = StaticKey::Name(self.repository.strings.intern(member_name));
        let node = origin_id
            .into_global_any(state.ctx.module_id)
            .into_anchored(Some(state.ctx.profile));
        let ordering_member_symbol = self
            .resolve_static_member_symbol(
                state.ctx.compiler_context.revision(),
                state.ctx.module,
                state.ctx.profile,
                origin_id,
                ordering_symbol,
                member_key,
                state.tree,
                state.symbols,
            )
            .map_err(|error| self.elaborate_error_from_resolve(error, node))?;

        // build the reference expression
        let ordering_name = self
            .repository
            .strings
            .intern(LanguageSymbol::Ordering.export_name());
        let member_name = self.repository.strings.intern(member_name);
        let reference_id = state.tree.reserve_from(
            NodeType::Expression,
            origin_id.into_any(),
            state.tree.get_scope(origin_id),
            None,
            Some(dir::ProvenanceReason::Reified),
        );
        let reference_id = state.tree.insert_as_owner(
            reference_id,
            Expression::ModuleReference {
                path: Path::from(&[ordering_name, member_name][..]),
                generic_arguments: vec![],
                target_symbol: ordering_member_symbol,
            },
        );

        // annotate the reference with the ordering type
        let reference_type_id = state
            .types
            .get_value_type_id(ordering_member_symbol)
            .unwrap_or(ordering_type_id);
        state.types.set_inferred_type(
            reference_id.into_global_any(state.ctx.module_id),
            reference_type_id,
        );

        Ok(reference_id)
    }

    /// Replace the node resolution with a builtin resolution.
    fn set_builtin_resolution_for_node(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        receiver: Option<LocalTypeId>,
    ) {
        let resolution_id = state
            .types
            .insert_resolution(Resolution::Builtin { receiver });
        state.types.set_resolution_for_node(
            expression_id.into_global_any(state.ctx.module_id),
            resolution_id,
        );
    }

    /// Return the operator member name for a language symbol.
    fn operator_member_name(&self, operator_symbol: LanguageSymbol) -> dir::StringId {
        // match analyze operator key lowering: `Add` -> `add`
        let export_name = operator_symbol.export_name();
        let mut chars = export_name.chars();
        let first_char = chars.next().unwrap_or_default();

        let mut member_name = String::new();
        member_name.push(first_char.to_ascii_lowercase());
        member_name.push_str(chars.as_str());

        self.repository.strings.intern(&member_name)
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
