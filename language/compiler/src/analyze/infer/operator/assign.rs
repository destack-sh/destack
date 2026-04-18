use super::*;
use crate::analyze::StaticMemberSymbolKind;
use destack_dir::{Name, Resolution};

/// Shared assignment target metadata used by assignment inference paths.
#[derive(Clone, Copy, Debug)]
struct AssignTargetBinding {
    /// The normalized assignment target expression id.
    target_id: LocalNodeId<Expression>,
    /// The optional symbol resolved from the normalized target.
    target_symbol: Option<GlobalSymbolId>,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(crate) fn infer_assign_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        right_id: LocalNodeId<Expression>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_EXPRESSION_OPERATOR);

        // route index assignment to index set resolution
        if let Expression::Index { left: _, right: _ } = ctx.tree.get(left_id) {
            return self.infer_index_assignment_expression(
                ctx,
                expression_id,
                left_id,
                right_id,
                state,
            );
        }

        let AssignTargetBinding {
            target_id,
            target_symbol,
        } = self.check_assignment_left_target(&mut ctx.reborrow(), left_id, state)?;

        let left_ty_id = self.infer_expression(&mut ctx.reborrow(), left_id, state)?;

        // reject assignments to associated projections in value space
        if self.assignment_target_is_associated_projection(&ctx.reborrow(), left_id)? {
            self.error(AnalyzeError::InvalidAssignmentTarget {
                node: left_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });

            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            return Ok(ctx.types.insert_type_from(ty, expression_id));
        }

        // use the left type as the expected type for the right expression
        let mut right_ctx = state.fork().with_expected_type(Some(left_ty_id));
        let right_ty_id = self.infer_expression(&mut ctx.reborrow(), right_id, &mut right_ctx)?;

        // enforce explicit ownership when implicit managed values are disabled
        self.check_no_implicit_managed_value(
            ctx.module_type_view(),
            right_id,
            left_ty_id,
            right_ty_id,
            ctx.tree,
            &state.options,
        );

        // add the subtype constraint
        ctx.infer.push_constraint(Constraint::Subtype {
            sub_type: right_ty_id,
            super_type: left_ty_id,
            variance: None,
        });

        // enforce assignment relation after convergence when needed
        self.enforce_assignability_or_defer_diagnostic(
            &mut ctx.reborrow(),
            expression_id.into_any(),
            left_ty_id,
            right_ty_id,
            UnassignableRelationFailureMode::PropagateError,
        )?;

        // narrow desugared nullish assignments to non nullish targets
        if let Expression::Binary {
            left: coalesce_left,
            operator: BinaryOperator::Coalesce,
            right: _,
        } = ctx.tree.get(right_id)
            && let Some(target_symbol) = target_symbol
        {
            let coalesce_left = self.unwrap_parenthesized_expression(*coalesce_left, ctx.tree);
            if coalesce_left == target_id {
                let canonical_symbol = self.canonical_symbol_id(
                    ctx.module_symbol_view(),
                    target_symbol,
                    CanonicalSymbolMode::FollowAliases,
                );
                let (non_nullish, has_nullish) =
                    self.strip_nullish_from_union(left_ty_id, ctx.types);
                if has_nullish {
                    let narrowed_ty_id = non_nullish.unwrap_or(right_ty_id);
                    state.narrow(canonical_symbol, narrowed_ty_id);
                }
            }
        }

        let ty = Type::TypeLiteral {
            value: TypeLiteral::Void,
        };
        Ok(ctx.types.insert_type_from(ty, expression_id))
    }

    /// Infer a compound assignment expression.
    pub(crate) fn infer_assign_binary_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        operator: &AssignOperator,
        left_id: LocalNodeId<Expression>,
        right_id: LocalNodeId<Expression>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // route index assignment to index set resolution
        if let Expression::Index { left: _, right: _ } = ctx.tree.get(left_id) {
            return self.infer_index_assignment_expression(
                ctx,
                expression_id,
                left_id,
                right_id,
                state,
            );
        }

        let AssignTargetBinding {
            target_id: _,
            target_symbol,
        } = self.check_assignment_left_target(&mut ctx.reborrow(), left_id, state)?;

        // infer left and right types
        let left_ty_id = self.infer_expression(&mut ctx.reborrow(), left_id, state)?;

        // reject assignments to associated projections in value space
        if self.assignment_target_is_associated_projection(&ctx.reborrow(), left_id)? {
            self.error(AnalyzeError::InvalidAssignmentTarget {
                node: left_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });

            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            return Ok(ctx.types.insert_type_from(ty, expression_id));
        }

        let mut right_ctx = state.fork().with_expected_type(Some(left_ty_id));
        let right_ty_id = self.infer_expression(&mut ctx.reborrow(), right_id, &mut right_ctx)?;

        // enforce logical assignment semantics
        let is_logical_assignment = matches!(
            operator,
            AssignOperator::AndAssign | AssignOperator::OrAssign | AssignOperator::CoalesceAssign
        );
        if is_logical_assignment {
            // enforce explicit ownership when implicit managed values are disabled
            self.check_no_implicit_managed_value(
                ctx.module_type_view(),
                right_id,
                left_ty_id,
                right_ty_id,
                ctx.tree,
                &state.options,
            );

            // add the subtype constraint
            ctx.infer.push_constraint(Constraint::Subtype {
                sub_type: right_ty_id,
                super_type: left_ty_id,
                variance: None,
            });

            // enforce assignment relation after convergence when needed
            self.enforce_assignability_or_defer_diagnostic(
                &mut ctx.reborrow(),
                expression_id.into_any(),
                left_ty_id,
                right_ty_id,
                UnassignableRelationFailureMode::PropagateError,
            )?;

            // narrow nullish assignments to non nullish targets
            if matches!(operator, AssignOperator::CoalesceAssign)
                && let Some(target_symbol) = target_symbol
            {
                let canonical_symbol = self.canonical_symbol_id(
                    ctx.module_symbol_view(),
                    target_symbol,
                    CanonicalSymbolMode::FollowAliases,
                );
                let (non_nullish, has_nullish) =
                    self.strip_nullish_from_union(left_ty_id, ctx.types);
                if has_nullish {
                    let narrowed_ty_id = non_nullish.unwrap_or(right_ty_id);
                    state.narrow(canonical_symbol, narrowed_ty_id);
                }
            }
        }

        let ty = Type::TypeLiteral {
            value: TypeLiteral::Void,
        };
        Ok(ctx.types.insert_type_from(ty, expression_id))
    }

    /// Resolve mutability for a binding symbol when it can be derived locally.
    pub(super) fn binding_mutability_for_symbol(
        &self,
        module: &Module,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
    ) -> Option<Mutability> {
        if symbol.module_id != module.id {
            return Some(Mutability::Immutable);
        }
        symbols.get_symbol(symbol.local_id).binding_mutability
    }

    /// Validate assignment target mutability and readonly restrictions.
    fn check_assignment_left_target(
        &self,
        ctx: &mut InferContext<'_>,
        left_id: LocalNodeId<Expression>,
        state: &mut InferState,
    ) -> AnalyzeResult<AssignTargetBinding> {
        // reject assignments to immutable bindings
        let target_id = self.unwrap_parenthesized_expression(left_id, ctx.tree);
        let target_symbol = self.reference_symbol_for_expression(ctx.tree_symbol_view(), target_id);
        if let Some(target_symbol) = target_symbol
            && matches!(
                self.binding_mutability_for_symbol(ctx.module, target_symbol, ctx.symbols),
                Some(Mutability::Immutable)
            )
        {
            self.error(AnalyzeError::ImmutableBindingAssignment {
                node: target_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }

        // reject writes through immutable references and readonly members
        if let Expression::Member {
            left: receiver_id,
            name,
        } = ctx.tree.get(left_id)
        {
            let receiver_ty_id = self.infer_member_assignment_receiver_type(
                &mut ctx.reborrow(),
                *receiver_id,
                state,
            )?;
            if self.type_is_immutable_reference(receiver_ty_id, ctx.types) {
                self.error(AnalyzeError::ImmutableReferenceAssignment {
                    node: receiver_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            }

            // reject writes to readonly members when the key is known
            if let Some(name) = *name {
                let member_key = self.static_key_from_key(
                    ctx.compiler_context.revision(),
                    ctx.profile,
                    ctx.tree,
                    ctx.symbols,
                    ctx.types,
                    Key::Name(Name::Identifier(name)),
                );
                if let Some(member_key) = member_key {
                    let is_field_readonly = self
                        .field_modifiers_for_key(
                            &mut ctx.type_context_reborrow(),
                            receiver_ty_id,
                            &member_key,
                        )
                        .is_some_and(|(_, is_readonly)| is_readonly);
                    let is_parameter_property_readonly = if let Some(receiver_symbol) =
                        self.receiver_symbol_for_visibility(receiver_ty_id, ctx.types)
                    {
                        self.parameter_property_member_context_for_key(
                            &mut ctx.reborrow(),
                            receiver_symbol,
                            &member_key,
                        )?
                        .is_some_and(|context| context.is_readonly)
                    } else {
                        false
                    };
                    if is_field_readonly || is_parameter_property_readonly {
                        self.error(AnalyzeError::ReadonlyProperty {
                            node: left_id
                                .into_global_any(ctx.module.id)
                                .into_anchored(Some(ctx.profile)),
                            member_key,
                        });
                    }
                }
            }
        } else if let Expression::Unary { operator, right } = ctx.tree.get(left_id)
            && matches!(operator, UnaryOperator::Dereference)
        {
            let right_ty_id = self.infer_expression(&mut ctx.reborrow(), *right, state)?;
            if self.type_is_immutable_reference(right_ty_id, ctx.types) {
                self.error(AnalyzeError::ImmutableReferenceAssignment {
                    node: right
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            }
        }

        Ok(AssignTargetBinding {
            target_id,
            target_symbol,
        })
    }

    /// Infer a member assignment receiver in value or projection mode.
    fn infer_member_assignment_receiver_type(
        &self,
        ctx: &mut InferContext<'_>,
        receiver_id: LocalNodeId<Expression>,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        let is_projection_receiver =
            self.is_projection_receiver_expression(&mut ctx.reborrow(), receiver_id);
        if is_projection_receiver {
            if let Some(type_id) = ctx
                .types
                .get_declared_or_inferred_type_id(receiver_id.into_global_any(ctx.module.id))
            {
                return Ok(type_id);
            }
        }

        self.infer_expression(&mut ctx.reborrow(), receiver_id, state)
    }

    /// Return true when an assignment target resolves to an associated projection.
    fn assignment_target_is_associated_projection(
        &self,
        ctx: &InferContext<'_>,
        target_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<bool> {
        let target_id = self.unwrap_parenthesized_expression(target_id, ctx.tree);
        if !matches!(ctx.tree.get(target_id), Expression::Member { .. }) {
            return Ok(false);
        }

        let node_id = target_id.into_global_any(ctx.module.id);
        let Some(resolution) = self.query_resolution_for_node_infer(node_id, ctx.infer, ctx.types)
        else {
            return Ok(false);
        };
        let target_symbol = match resolution {
            Resolution::Static { candidate, .. } => candidate.target_symbol,
            _ => return Ok(false),
        };

        let kind =
            self.query_static_member_symbol_kind_for_symbol(ctx.tree_symbol_view(), target_symbol)?;
        Ok(matches!(
            kind,
            Some(
                StaticMemberSymbolKind::AssociatedType
                    | StaticMemberSymbolKind::AssociatedComptimeConst
            )
        ))
    }

    /// Check whether a type is an immutable reference or pointer.
    pub(super) fn type_is_immutable_reference(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        match types.get_type(ty_id) {
            Type::ReferenceOf { mutability, .. } | Type::PointerOf { mutability, .. } => {
                *mutability != Some(Mutability::Mutable)
            }
            Type::Value { value } => self.type_is_immutable_reference(*value, types),
            _ => false,
        }
    }
}
