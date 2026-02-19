use super::*;
use crate::analyze::common::StaticMemberSymbolKind;
use destack_dir::Resolution;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(crate) fn infer_assign_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        right_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_EXPRESSION_OPERATOR);

        let options = ctx.options;

        // route index assignment to index set resolution
        if let Expression::Index { left: _, right: _ } = tree.get(left_id) {
            return self.infer_index_assignment_expression(
                module,
                expression_id,
                left_id,
                right_id,
                tree,
                symbols,
                types,
                infer,
                ctx,
            );
        }

        // reject assignments to immutable bindings
        let target_id = self.unwrap_parenthesized_expression(left_id, tree);
        let target_symbol =
            self.reference_symbol_for_expression(module, target_id, ctx.profile, tree, symbols);
        if let Some(target_symbol) = target_symbol
            && matches!(
                self.binding_mutability_for_symbol(module, target_symbol, symbols),
                Some(Mutability::Immutable)
            )
        {
            self.error(AnalyzeError::ImmutableBindingAssignment {
                node: target_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }

        // reject assignments through immutable references
        if let Expression::Member {
            left: receiver_id,
            name,
            static_arguments,
        } = tree.get(left_id)
        {
            let receiver_ty_id = self.infer_member_assignment_receiver_type(
                module,
                *receiver_id,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?;
            if self.type_is_immutable_reference(receiver_ty_id, types) {
                self.error(AnalyzeError::ImmutableReferenceAssignment {
                    node: receiver_id
                        .into_global_any(module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            }

            // reject writes to readonly members when the key is known
            if static_arguments.is_none() {
                let member_key = self.static_key_from_dynamic_key(
                    ctx.profile,
                    DynamicKey::Name(*name),
                    tree,
                    symbols,
                    types,
                );
                if let Some(member_key) = member_key
                    && (self
                        .field_modifiers_for_key(
                            module,
                            ctx.profile,
                            receiver_ty_id,
                            &member_key,
                            symbols,
                            types,
                        )
                        .is_some_and(|(_, is_readonly)| is_readonly)
                        || self
                            .receiver_symbol_for_visibility(receiver_ty_id, types)
                            .and_then(|receiver_symbol| {
                                self.parameter_property_member_context_for_key(
                                    module,
                                    ctx.profile,
                                    receiver_symbol,
                                    &member_key,
                                    tree,
                                    symbols,
                                    types,
                                )
                            })
                            .is_some_and(|context| context.is_readonly))
                {
                    self.error(AnalyzeError::ReadonlyProperty {
                        node: left_id
                            .into_global_any(module.id)
                            .into_anchored(Some(ctx.profile)),
                        member_key,
                    });
                }
            }
        } else if let Expression::Unary { operator, right } = tree.get(left_id)
            && matches!(operator, UnaryOperator::Dereference)
        {
            let right_ty_id =
                self.infer_expression(module, *right, tree, symbols, types, infer, ctx)?;
            if self.type_is_immutable_reference(right_ty_id, types) {
                self.error(AnalyzeError::ImmutableReferenceAssignment {
                    node: right
                        .into_global_any(module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            }
        }

        let left_ty_id =
            self.infer_expression(module, left_id, tree, symbols, types, infer, ctx)?;

        // reject assignments to associated projections in value space
        if self.assignment_target_is_associated_projection(
            module,
            ctx.profile,
            left_id,
            tree,
            symbols,
            types,
        )? {
            self.error(AnalyzeError::InvalidAssignmentTarget {
                node: left_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
            });

            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        }

        // use the left type as the expected type for the right expression
        let mut right_ctx = ctx.fork().with_expected_type(Some(left_ty_id));
        let right_ty_id = self.infer_expression(
            module,
            right_id,
            tree,
            symbols,
            types,
            infer,
            &mut right_ctx,
        )?;

        // enforce explicit ownership when implicit managed values are disabled
        self.check_no_implicit_managed_value(
            module,
            ctx.profile,
            right_id,
            left_ty_id,
            right_ty_id,
            tree,
            types,
            &options,
        );

        // add the subtype constraint
        infer.push_constraint(Constraint::Subtype {
            sub_type: right_ty_id,
            super_type: left_ty_id,
            variance: None,
        });

        // check assignability when types are resolved
        if !self.is_type_assignable_or_deferred(
            module,
            ctx.profile,
            symbols,
            left_ty_id,
            right_ty_id,
            types,
            &options,
        ) {
            return Err(AnalyzeError::UnassignableType {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
                expected_ty: left_ty_id.into_global(module.id),
                actual_ty: right_ty_id.into_global(module.id),
            });
        }

        // narrow desugared nullish assignments to non nullish targets
        if let Expression::Binary {
            left: coalesce_left,
            operator: BinaryOperator::Coalesce,
            right: _,
        } = tree.get(right_id)
            && let Some(target_symbol) = target_symbol
        {
            let coalesce_left = self.unwrap_parenthesized_expression(*coalesce_left, tree);
            if coalesce_left == target_id {
                let canonical_symbol = self.canonical_symbol_id(
                    module,
                    symbols,
                    ctx.profile,
                    target_symbol,
                    CanonicalSymbolMode::FollowAliases,
                );
                let (non_nullish, has_nullish) = self.strip_nullish_from_union(left_ty_id, types);
                if has_nullish {
                    let narrowed_ty_id = non_nullish.unwrap_or(right_ty_id);
                    ctx.narrow(canonical_symbol, narrowed_ty_id);
                }
            }
        }

        let ty = Type::TypeLiteral {
            value: TypeLiteral::Void,
        };
        Ok(types.insert_type_from(ty, expression_id))
    }

    /// Infer a compound assignment expression.
    pub(crate) fn infer_assign_binary_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        operator: &AssignOperator,
        left_id: LocalNodeId<Expression>,
        right_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // route index assignment to index set resolution
        if let Expression::Index { left: _, right: _ } = tree.get(left_id) {
            return self.infer_index_assignment_expression(
                module,
                expression_id,
                left_id,
                right_id,
                tree,
                symbols,
                types,
                infer,
                ctx,
            );
        }

        // reject assignments to immutable bindings
        let target_id = self.unwrap_parenthesized_expression(left_id, tree);
        let target_symbol =
            self.reference_symbol_for_expression(module, target_id, ctx.profile, tree, symbols);
        if let Some(target_symbol) = target_symbol
            && matches!(
                self.binding_mutability_for_symbol(module, target_symbol, symbols),
                Some(Mutability::Immutable)
            )
        {
            self.error(AnalyzeError::ImmutableBindingAssignment {
                node: target_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }

        // reject assignments through immutable references
        if let Expression::Member {
            left: receiver_id,
            name,
            static_arguments,
        } = tree.get(left_id)
        {
            let receiver_ty_id = self.infer_member_assignment_receiver_type(
                module,
                *receiver_id,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?;
            if self.type_is_immutable_reference(receiver_ty_id, types) {
                self.error(AnalyzeError::ImmutableReferenceAssignment {
                    node: receiver_id
                        .into_global_any(module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            }

            // reject writes to readonly members when the key is known
            if static_arguments.is_none() {
                let member_key = self.static_key_from_dynamic_key(
                    ctx.profile,
                    DynamicKey::Name(*name),
                    tree,
                    symbols,
                    types,
                );
                if let Some(member_key) = member_key
                    && (self
                        .field_modifiers_for_key(
                            module,
                            ctx.profile,
                            receiver_ty_id,
                            &member_key,
                            symbols,
                            types,
                        )
                        .is_some_and(|(_, is_readonly)| is_readonly)
                        || self
                            .receiver_symbol_for_visibility(receiver_ty_id, types)
                            .and_then(|receiver_symbol| {
                                self.parameter_property_member_context_for_key(
                                    module,
                                    ctx.profile,
                                    receiver_symbol,
                                    &member_key,
                                    tree,
                                    symbols,
                                    types,
                                )
                            })
                            .is_some_and(|context| context.is_readonly))
                {
                    self.error(AnalyzeError::ReadonlyProperty {
                        node: left_id
                            .into_global_any(module.id)
                            .into_anchored(Some(ctx.profile)),
                        member_key,
                    });
                }
            }
        } else if let Expression::Unary { operator, right } = tree.get(left_id)
            && matches!(operator, UnaryOperator::Dereference)
        {
            let right_ty_id =
                self.infer_expression(module, *right, tree, symbols, types, infer, ctx)?;
            if self.type_is_immutable_reference(right_ty_id, types) {
                self.error(AnalyzeError::ImmutableReferenceAssignment {
                    node: right
                        .into_global_any(module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            }
        }

        // infer left and right types
        let left_ty_id =
            self.infer_expression(module, left_id, tree, symbols, types, infer, ctx)?;

        // reject assignments to associated projections in value space
        if self.assignment_target_is_associated_projection(
            module,
            ctx.profile,
            left_id,
            tree,
            symbols,
            types,
        )? {
            self.error(AnalyzeError::InvalidAssignmentTarget {
                node: left_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
            });

            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        }

        let mut right_ctx = ctx.fork().with_expected_type(Some(left_ty_id));
        let right_ty_id = self.infer_expression(
            module,
            right_id,
            tree,
            symbols,
            types,
            infer,
            &mut right_ctx,
        )?;

        // enforce logical assignment semantics
        let is_logical_assignment = matches!(
            operator,
            AssignOperator::AndAssign | AssignOperator::OrAssign | AssignOperator::CoalesceAssign
        );
        if is_logical_assignment {
            let options = ctx.options;

            // enforce explicit ownership when implicit managed values are disabled
            self.check_no_implicit_managed_value(
                module,
                ctx.profile,
                right_id,
                left_ty_id,
                right_ty_id,
                tree,
                types,
                &options,
            );

            // add the subtype constraint
            infer.push_constraint(Constraint::Subtype {
                sub_type: right_ty_id,
                super_type: left_ty_id,
                variance: None,
            });

            // check assignability when types are resolved
            if !self.is_type_assignable_or_deferred(
                module,
                ctx.profile,
                symbols,
                left_ty_id,
                right_ty_id,
                types,
                &options,
            ) {
                return Err(AnalyzeError::UnassignableType {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(ctx.profile)),
                    expected_ty: left_ty_id.into_global(module.id),
                    actual_ty: right_ty_id.into_global(module.id),
                });
            }

            // narrow nullish assignments to non nullish targets
            if matches!(operator, AssignOperator::CoalesceAssign)
                && let Some(target_symbol) = target_symbol
            {
                let canonical_symbol = self.canonical_symbol_id(
                    module,
                    symbols,
                    ctx.profile,
                    target_symbol,
                    CanonicalSymbolMode::FollowAliases,
                );
                let (non_nullish, has_nullish) = self.strip_nullish_from_union(left_ty_id, types);
                if has_nullish {
                    let narrowed_ty_id = non_nullish.unwrap_or(right_ty_id);
                    ctx.narrow(canonical_symbol, narrowed_ty_id);
                }
            }
        }

        let ty = Type::TypeLiteral {
            value: TypeLiteral::Void,
        };
        Ok(types.insert_type_from(ty, expression_id))
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

    /// Infer a member assignment receiver in value or projection mode.
    fn infer_member_assignment_receiver_type(
        &self,
        module: &Module,
        receiver_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        if self.query_expression_is_projection_receiver_for_infer(
            module,
            ctx.profile,
            receiver_id,
            tree,
            symbols,
            types,
        ) {
            return self.try_evaluate_expression_to_type(
                module,
                ctx.profile,
                receiver_id,
                tree,
                symbols,
                types,
                true,
                true,
            );
        }

        self.infer_expression(module, receiver_id, tree, symbols, types, infer, ctx)
    }

    /// Return true when an assignment target resolves to an associated projection.
    fn assignment_target_is_associated_projection(
        &self,
        module: &Module,
        profile: ProfileId,
        target_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> AnalyzeResult<bool> {
        let target_id = self.unwrap_parenthesized_expression(target_id, tree);
        if !matches!(tree.get(target_id), Expression::Member { .. }) {
            return Ok(false);
        }

        let node_id = target_id.into_global_any(module.id);
        let Some(resolution_id) = types.get_resolution_for_node(node_id) else {
            return Ok(false);
        };
        let resolution = types.get_resolution(resolution_id);
        let target_symbol = match resolution {
            Resolution::Static { candidate, .. } => candidate.target_symbol,
            _ => return Ok(false),
        };

        let kind = self
            .query_static_member_symbol_kind_for_symbol(
                module,
                profile,
                target_symbol,
                tree,
                symbols,
            )
            .map_err(AnalyzeError::from)?;
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
