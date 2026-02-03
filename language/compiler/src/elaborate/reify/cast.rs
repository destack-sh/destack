use destack_dir::{
    Argument, BinaryOperator, Block, CastOperator, CastSource, Declarator, DynamicKey,
    EnumBackingType, Expression, GlobalSymbolId, IfCondition, IfKind, LocalNodeId, LocalTypeId,
    MatchCase, NodeTree, NodeType, Path, Resolution, ResolutionCandidate, ResolvedSignature,
    ScalarLiteral, StaticArgument, StaticExpression, StaticKey, SymbolSpace, SymbolTable,
    SymbolType, Type, TypeBinaryOperator, TypeElement, TypeLiteral, TypeTable, UnaryOperator,
    WellKnownSymbol,
};
use destack_source::ModuleId;
use destack_workspace::{ImplicitCollectionConversionPolicy, Module, ProfileId};

use super::r#type::{
    are_types_semantically_equal, common_numeric_type_id_for_binary, is_any_type, is_integer_type,
    is_nullable_union, is_object_type, is_pointer_type, is_scalar_literal_type, is_string_type,
    is_union_type, is_unknown_type, numeric_cast_operator,
};
use crate::{Compiler, ElaborateError, ElaborateResult, ElaborateWarning};

/// The resolved record-like target data for reification.
struct RecordLikeTargetInfo {
    /// The key type for the record-like conversion.
    key_type_id: LocalTypeId,
    /// The target map symbol that provides `from`.
    map_symbol: GlobalSymbolId,
    /// The resolved map type reference.
    map_type_id: LocalTypeId,
    /// The entry tuple type for `Map<K, V>`.
    entry_tuple_type_id: LocalTypeId,
    /// The array type for map entries.
    entries_array_type_id: LocalTypeId,
    /// Static arguments for `Map<K, V>`.
    map_static_arguments: Vec<StaticArgument>,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Reify an explicit cast expression into a cast node.
    pub(super) fn reify_explicit_cast_expression(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        target_type: LocalNodeId<Expression>,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        module: &Module,
    ) -> ElaborateResult<()> {
        // read the source and target type ids
        let value_type_id = types
            .get_declared_or_inferred_type_id(value.into_global_any(module_id))
            .ok_or(ElaborateError::UnsupportedConstruct {
                node: value
                    .into_global_any(module_id)
                    .into_anchored(Some(profile)),
            })?;
        let target_type_id = types
            .get_declared_or_inferred_type_id(expression_id.into_global_any(module_id))
            .ok_or(ElaborateError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(module_id)
                    .into_anchored(Some(profile)),
            })?;

        // reify record-like and sized array literals into collection construction
        if let Some(reified) = self.reify_record_like_object_literal(
            module,
            profile,
            expression_id,
            value,
            target_type_id,
            tree,
            symbols,
            types,
        )? {
            let source_id = reified.into_global_any(module_id);
            let target_id = expression_id.into_global_any(module_id);
            types.copy_node_analysis(source_id, target_id);
            let expression = tree.get(reified).clone();
            tree.replace(expression_id, expression);
            return Ok(());
        }
        if let Some(reified) = self.reify_array_sized_value(
            module,
            profile,
            expression_id,
            value,
            target_type_id,
            tree,
            symbols,
            types,
        )? {
            let source_id = reified.into_global_any(module_id);
            let target_id = expression_id.into_global_any(module_id);
            types.copy_node_analysis(source_id, target_id);
            let expression = tree.get(reified).clone();
            tree.replace(expression_id, expression);
            return Ok(());
        }

        // classify the cast
        let operator = self.cast_operator_for_types(
            module_id,
            profile,
            symbols,
            types,
            value_type_id,
            target_type_id,
            module,
        );
        // replace the expression with a cast node
        tree.replace(
            expression_id,
            Expression::Cast {
                operator,
                source: CastSource::Explicit,
                value,
                target_type,
            },
        );

        Ok(())
    }

    /// Reify implicit casts for reference expressions when the node type narrows.
    pub(super) fn reify_implicit_casts_in_reference(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        module: &Module,
    ) -> ElaborateResult<()> {
        // skip references used by guard operators
        if self.reference_is_guard_operand(expression_id, tree) {
            return Ok(());
        }

        // skip references that resolve in type-only space
        let symbol_space = if target_symbol.module_id == module.id {
            let symbol_entry = symbols.get_symbol(target_symbol.local_id);
            symbol_entry.space
        } else {
            let remote_module = self.program.modules.get(target_symbol.module_id);
            let remote_module = remote_module.read();
            let dir = remote_module.dir(profile);
            let remote_symbols = dir.symbols.read();
            let symbol_entry = remote_symbols.get_symbol(target_symbol.local_id);
            symbol_entry.space
        };
        if !matches!(symbol_space, SymbolSpace::Value | SymbolSpace::TypeValue) {
            return Ok(());
        }

        // skip references marked as type expressions
        if let Some(inferred_type_id) =
            types.get_inferred_type_id(expression_id.into_global_any(module_id))
            && matches!(types.get_type(inferred_type_id), Type::Value { .. })
        {
            return Ok(());
        }

        // require a declared or inferred node type
        let Some(target_type_id) =
            types.get_declared_or_inferred_type_id(expression_id.into_global_any(module_id))
        else {
            return Ok(());
        };

        // require a source value type from the referenced symbol
        let Some(source_type_id) = types.get_value_type_id(target_symbol) else {
            return Ok(());
        };

        // unwrap type aliases and values
        let target_type_id = self.unwrap_value_type_id(types, target_type_id);
        let source_type_id = self.unwrap_value_type_id(types, source_type_id);

        // skip when the types are semantically identical
        if are_types_semantically_equal(
            types.get_type(source_type_id),
            types.get_type(target_type_id),
            types,
        ) {
            return Ok(());
        }

        // classify the cast for this narrowing
        let operator = self.cast_operator_for_types(
            module_id,
            profile,
            symbols,
            types,
            source_type_id,
            target_type_id,
            module,
        );
        if operator == CastOperator::Identity {
            return Ok(());
        }

        // move the reference into a new value node
        let expression = tree.get(expression_id).clone();
        let scope = tree.get_scope(expression_id);
        let value_expression_id =
            tree.reserve_from(NodeType::Expression, expression_id.into_any(), scope, None);
        let value_expression_id = tree.insert(value_expression_id, expression);
        types.set_inferred_type(
            value_expression_id.into_global_any(module_id),
            source_type_id,
        );

        // build the target type expression
        let target_expression_id =
            tree.reserve_from(NodeType::Expression, expression_id.into_any(), scope, None);
        let target_expression = match types.get_type(target_type_id) {
            Type::TypeLiteral { value } => Expression::TypeLiteral {
                value: value.clone(),
            },
            _ => Expression::Type {
                value: target_type_id,
            },
        };
        let target_expression_id = tree.insert(target_expression_id, target_expression);

        // set the inferred type for the target type expression
        let target_type_value = Type::Value {
            value: target_type_id,
        };
        let target_type_value_id = types.insert_type_from(target_type_value, target_expression_id);
        types.set_inferred_type(
            target_expression_id.into_global_any(module_id),
            target_type_value_id,
        );

        // replace the reference with the cast expression
        let cast_expression = Expression::Cast {
            operator,
            source: CastSource::Implicit,
            value: value_expression_id,
            target_type: target_expression_id,
        };
        if self.options.elaborate_parenthesize_casts {
            let cast_expression_id =
                tree.reserve_from(NodeType::Expression, expression_id.into_any(), scope, None);
            let cast_expression_id = tree.insert(cast_expression_id, cast_expression);
            types.set_inferred_type(
                cast_expression_id.into_global_any(module_id),
                target_type_id,
            );
            tree.replace(
                expression_id,
                Expression::Parenthesized {
                    expression: cast_expression_id,
                },
            );
        } else {
            tree.replace(expression_id, cast_expression);
        }
        types.set_inferred_type(expression_id.into_global_any(module_id), target_type_id);

        Ok(())
    }

    /// Return true when a reference is used by a guard operator.
    fn reference_is_guard_operand(
        &self,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
    ) -> bool {
        let Some(parent_id) = tree.get_parent(expression_id.id) else {
            return false;
        };
        let Ok(parent_id) = parent_id.try_into_typed::<Expression>() else {
            return false;
        };

        let parent = tree.get(parent_id);
        match parent {
            Expression::TypeBinary {
                left,
                operator,
                right,
            } => {
                matches!(
                    operator,
                    TypeBinaryOperator::Is
                        | TypeBinaryOperator::InstanceOf
                        | TypeBinaryOperator::Cast
                ) && (*left == expression_id || *right == expression_id)
            }
            Expression::Binary {
                left,
                operator,
                right,
            } => match operator {
                BinaryOperator::InstanceOf => *left == expression_id || *right == expression_id,
                BinaryOperator::In => *right == expression_id,
                _ => false,
            },
            Expression::Unary { operator, right } => {
                matches!(operator, UnaryOperator::Typeof) && *right == expression_id
            }
            _ => false,
        }
    }

    /// Reify implicit casts in let and using bindings.
    pub(super) fn reify_implicit_casts_in_binding(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        declarators: &[LocalNodeId<Declarator>],
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        module: &Module,
    ) -> ElaborateResult<()> {
        // visit each declarator
        for declarator_id in declarators {
            // skip declarators without a value
            let declarator = tree.get(*declarator_id).clone();
            let Some(value_id) = declarator.value else {
                continue;
            };
            let Some(target_type_id) =
                types.get_declared_type_id(declarator_id.into_global_any(module_id))
            else {
                continue;
            };

            // wrap the value with a cast when needed
            let cast_value_id = self.wrap_value_with_cast(
                module_id,
                profile,
                value_id,
                value_id,
                target_type_id,
                tree,
                symbols,
                types,
                module,
            )?;

            // update the declarator when the value changes
            if cast_value_id != value_id {
                let updated = Declarator {
                    value: Some(cast_value_id),
                    ..declarator
                };
                tree.replace(*declarator_id, updated);
            }
        }

        Ok(())
    }

    /// Reify implicit casts in an assignment expression.
    pub(super) fn reify_implicit_casts_in_assignment(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        module: &Module,
    ) -> ElaborateResult<()> {
        // read the target type from the left hand side
        let target_type_id = self
            .value_type_id_for_expression(module_id, left, tree, symbols, types)
            .ok_or(ElaborateError::UnsupportedConstruct {
                node: left.into_global_any(module_id).into_anchored(Some(profile)),
            })?;

        // wrap the right hand side when needed
        let cast_right_id = self.wrap_value_with_cast(
            module_id,
            profile,
            expression_id,
            right,
            target_type_id,
            tree,
            symbols,
            types,
            module,
        )?;

        // replace the assignment when the value changes
        if cast_right_id != right {
            tree.replace(
                expression_id,
                Expression::Assign {
                    left,
                    right: cast_right_id,
                },
            );
        }

        Ok(())
    }

    /// Reify implicit casts in a return expression.
    pub(super) fn reify_implicit_casts_in_return(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        value: Option<LocalNodeId<Expression>>,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        module: &Module,
    ) -> ElaborateResult<()> {
        // skip returns without values
        let Some(value_id) = value else {
            return Ok(());
        };

        // read the declared return type
        let Some(target_type_id) =
            self.enclosing_return_type(module_id, expression_id, tree, types)
        else {
            return Ok(());
        };

        // wrap the return value when needed
        let cast_value_id = self.wrap_value_with_cast(
            module_id,
            profile,
            expression_id,
            value_id,
            target_type_id,
            tree,
            symbols,
            types,
            module,
        )?;

        // replace the return when the value changes
        if cast_value_id != value_id {
            tree.replace(
                expression_id,
                Expression::Return {
                    value: Some(cast_value_id),
                },
            );
        }

        Ok(())
    }

    /// Reify implicit casts in call arguments.
    pub(super) fn reify_implicit_casts_in_call(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        dynamic_arguments: &[LocalNodeId<Argument>],
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        module: &Module,
    ) -> ElaborateResult<()> {
        // resolve expected types for arguments
        let Some(expected_argument_types) = self.expected_argument_types_for_call(
            module_id,
            expression_id,
            dynamic_arguments,
            tree,
            types,
        )?
        else {
            return Ok(());
        };

        // visit each argument and insert casts as needed
        for (index, argument_id) in dynamic_arguments.iter().enumerate() {
            let Some(expected_type_id) = expected_argument_types.get(index).copied().flatten()
            else {
                continue;
            };

            let argument = tree.get(*argument_id).clone();
            let value_id = argument.value();

            let cast_value_id = self.wrap_value_with_cast(
                module_id,
                profile,
                expression_id,
                value_id,
                expected_type_id,
                tree,
                symbols,
                types,
                module,
            )?;

            if cast_value_id == value_id {
                continue;
            }

            let updated = match argument {
                Argument::Named {
                    modifiers, name, ..
                } => Argument::Named {
                    modifiers,
                    name,
                    value: cast_value_id,
                },
                Argument::Labeled {
                    modifiers, label, ..
                } => Argument::Labeled {
                    modifiers,
                    label,
                    value: cast_value_id,
                },
                Argument::Positional { modifiers, .. } => Argument::Positional {
                    modifiers,
                    value: cast_value_id,
                },
                Argument::Spread {
                    modifiers, label, ..
                } => Argument::Spread {
                    modifiers,
                    label,
                    value: cast_value_id,
                },
            };
            tree.replace(*argument_id, updated);
        }

        Ok(())
    }

    /// Collapse redundant nested casts to the same target type.
    pub(super) fn normalize_redundant_casts(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        module: &Module,
    ) -> ElaborateResult<()> {
        // walk all expressions to find nested casts
        for expression_id in tree.iter_node_ids_of_type::<Expression>() {
            // skip inactive expressions
            if !self.is_node_active(tree, symbols, expression_id.into_any()) {
                continue;
            }

            let Expression::Cast {
                value,
                target_type,
                operator,
                source,
            } = tree.get(expression_id).clone()
            else {
                continue;
            };

            // require the cast value to be another cast
            let Expression::Cast {
                value: inner_value,
                target_type: inner_target_type,
                ..
            } = tree.get(value).clone()
            else {
                continue;
            };

            // resolve target type ids for both casts
            let Some(outer_target_type_id) =
                self.type_id_for_type_expression(module_id, target_type, tree, types)
            else {
                continue;
            };
            let Some(inner_target_type_id) =
                self.type_id_for_type_expression(module_id, inner_target_type, tree, types)
            else {
                continue;
            };

            // keep nested casts when targets differ
            if !are_types_semantically_equal(
                types.get_type(outer_target_type_id),
                types.get_type(inner_target_type_id),
                types,
            ) {
                let options = self.analyze_context_options_for_module(module_id);
                let to_outer = self.is_type_assignable(
                    module,
                    profile,
                    symbols,
                    outer_target_type_id,
                    inner_target_type_id,
                    types,
                    &options,
                );
                let to_inner = self.is_type_assignable(
                    module,
                    profile,
                    symbols,
                    inner_target_type_id,
                    outer_target_type_id,
                    types,
                    &options,
                );
                if !(to_outer.is_assignable() && to_inner.is_assignable()) {
                    continue;
                }
            }

            // replace with a single cast to the shared target
            tree.replace(
                expression_id,
                Expression::Cast {
                    operator,
                    source,
                    value: inner_value,
                    target_type: inner_target_type,
                },
            );
            types.set_inferred_type(
                expression_id.into_global_any(module_id),
                outer_target_type_id,
            );
        }

        Ok(())
    }

    /// Reify implicit casts in ternary expressions.
    pub(super) fn reify_implicit_casts_in_ternary(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        condition: LocalNodeId<Expression>,
        then_expression: LocalNodeId<Expression>,
        else_expression: Option<LocalNodeId<Expression>>,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        module: &Module,
    ) -> ElaborateResult<()> {
        // use the expression type as the target for both branches
        let Some(target_type_id) =
            types.get_declared_or_inferred_type_id(expression_id.into_global_any(module_id))
        else {
            return Ok(());
        };

        // cast the then branch when needed
        let cast_then_id = self.wrap_value_with_cast(
            module_id,
            profile,
            expression_id,
            then_expression,
            target_type_id,
            tree,
            symbols,
            types,
            module,
        )?;

        // cast the else branch when present
        let cast_else_id = if let Some(else_expression) = else_expression {
            let cast_else_id = self.wrap_value_with_cast(
                module_id,
                profile,
                expression_id,
                else_expression,
                target_type_id,
                tree,
                symbols,
                types,
                module,
            )?;
            Some(cast_else_id)
        } else {
            None
        };

        // update the ternary expression when any branch changes
        if cast_then_id != then_expression || cast_else_id != else_expression {
            tree.replace(
                expression_id,
                Expression::If {
                    kind: IfKind::Ternary,
                    condition: IfCondition::Expression { condition },
                    then_expression: cast_then_id,
                    else_expression: cast_else_id,
                },
            );
        }

        Ok(())
    }

    /// Reify implicit casts in match case bodies.
    pub(super) fn reify_implicit_casts_in_match(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        cases: &[LocalNodeId<MatchCase>],
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        module: &Module,
    ) -> ElaborateResult<()> {
        // use the match expression type as the target
        let Some(target_type_id) =
            types.get_declared_or_inferred_type_id(expression_id.into_global_any(module_id))
        else {
            return Ok(());
        };

        // visit each case and cast the produced value
        for case_id in cases {
            let case = tree.get(*case_id).clone();
            match case {
                MatchCase::Expression {
                    selector,
                    body,
                    scope,
                } => {
                    let cast_body_id = self.wrap_value_with_cast(
                        module_id,
                        profile,
                        expression_id,
                        body,
                        target_type_id,
                        tree,
                        symbols,
                        types,
                        module,
                    )?;

                    if cast_body_id != body {
                        tree.replace(
                            *case_id,
                            MatchCase::Expression {
                                selector,
                                body: cast_body_id,
                                scope,
                            },
                        );
                    }
                }
                MatchCase::Block {
                    selector: _,
                    body,
                    scope: _,
                } => {
                    let block = tree.get(body).clone();
                    let Some(last_expression_id) = block.expressions.last().copied() else {
                        continue;
                    };

                    let cast_last_id = self.wrap_value_with_cast(
                        module_id,
                        profile,
                        expression_id,
                        last_expression_id,
                        target_type_id,
                        tree,
                        symbols,
                        types,
                        module,
                    )?;

                    if cast_last_id != last_expression_id {
                        let mut expressions = block.expressions;
                        let Some(last_expression) = expressions.last_mut() else {
                            continue;
                        };
                        *last_expression = cast_last_id;
                        tree.replace(
                            body,
                            Block {
                                scope: block.scope,
                                expressions,
                            },
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Reify implicit casts in builtin binary expressions.
    pub(super) fn reify_implicit_casts_in_binary(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        operator: BinaryOperator,
        right: LocalNodeId<Expression>,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        module: &Module,
    ) -> ElaborateResult<()> {
        // skip binary expressions without a resolution
        if types
            .get_resolution_for_node(expression_id.into_global_any(module_id))
            .is_none()
        {
            return Ok(());
        }

        // require numeric operators
        if !self.is_numeric_binary_operator(operator) {
            return Ok(());
        }

        // read operand type ids
        let left_type_id = self
            .value_type_id_for_expression(module_id, left, tree, symbols, types)
            .ok_or(ElaborateError::UnsupportedConstruct {
                node: left.into_global_any(module_id).into_anchored(Some(profile)),
            })?;
        let right_type_id = self
            .value_type_id_for_expression(module_id, right, tree, symbols, types)
            .ok_or(ElaborateError::UnsupportedConstruct {
                node: right
                    .into_global_any(module_id)
                    .into_anchored(Some(profile)),
            })?;

        // compute the common numeric type for both operands
        let Some(target_type_id) =
            common_numeric_type_id_for_binary(left_type_id, right_type_id, expression_id, types)
        else {
            return Ok(());
        };

        // align the binary expression type with the chosen numeric type
        types.set_inferred_type(expression_id.into_global_any(module_id), target_type_id);

        // wrap both operands when needed
        let cast_left_id = self.wrap_value_with_cast_for_numeric_binary(
            module_id,
            profile,
            expression_id,
            left,
            target_type_id,
            tree,
            symbols,
            types,
            module,
        )?;
        let cast_right_id = self.wrap_value_with_cast_for_numeric_binary(
            module_id,
            profile,
            expression_id,
            right,
            target_type_id,
            tree,
            symbols,
            types,
            module,
        )?;

        // update the binary expression when either operand changes
        if cast_left_id != left || cast_right_id != right {
            tree.replace(
                expression_id,
                Expression::Binary {
                    left: cast_left_id,
                    operator,
                    right: cast_right_id,
                },
            );
        }

        Ok(())
    }

    /// Wrap a value in a cast when the target type differs.
    fn wrap_value_with_cast(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        origin_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        target_type_id: LocalTypeId,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        module: &Module,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        self.wrap_value_with_cast_internal(
            module_id,
            profile,
            origin_id,
            value_id,
            target_type_id,
            tree,
            symbols,
            types,
            module,
            true,
        )
    }

    /// Wrap a value in a cast for numeric binary alignment.
    fn wrap_value_with_cast_for_numeric_binary(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        origin_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        target_type_id: LocalTypeId,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        module: &Module,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        self.wrap_value_with_cast_internal(
            module_id,
            profile,
            origin_id,
            value_id,
            target_type_id,
            tree,
            symbols,
            types,
            module,
            false,
        )
    }

    /// Wrap a value in a cast with numeric literal elision.
    fn wrap_value_with_cast_internal(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        origin_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        target_type_id: LocalTypeId,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        module: &Module,
        allow_assignable_skip: bool,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // read the source type id
        let value_type_id = self
            .value_type_id_for_expression(module_id, value_id, tree, symbols, types)
            .ok_or(ElaborateError::UnsupportedConstruct {
                node: value_id
                    .into_global_any(module_id)
                    .into_anchored(Some(profile)),
            })?;

        // reify record-like and sized array literals into collection construction
        if let Some(reified) = self.reify_record_like_object_literal(
            module,
            profile,
            origin_id,
            value_id,
            target_type_id,
            tree,
            symbols,
            types,
        )? {
            return Ok(reified);
        }
        if let Some(reified) = self.reify_array_sized_value(
            module,
            profile,
            origin_id,
            value_id,
            target_type_id,
            tree,
            symbols,
            types,
        )? {
            return Ok(reified);
        }

        // resolve unevaluated target types for cast classification
        let target_type_id = self
            .ensure_type_evaluated(module, profile, target_type_id, tree, symbols, types)
            .map_err(|_| ElaborateError::UnsupportedConstruct {
                node: types
                    .get_type_source(target_type_id)
                    .into_global(module_id)
                    .into_anchored(Some(profile)),
            })?;

        // check casts that change representation despite matching type ids
        let value_type = types.get_type(value_type_id).clone();
        let target_type = types.get_type(target_type_id).clone();
        let value_is_concrete = self.is_concrete_resolution(module_id, value_id, types)
            || self.is_concrete_new_expression(tree, value_id)
            || self.is_tagged_expression(tree, value_id);
        let value_is_nullish_literal = matches!(
            value_type,
            Type::TypeLiteral {
                value: TypeLiteral::Null | TypeLiteral::Undefined,
            }
        );
        let types_match = are_types_semantically_equal(&value_type, &target_type, types);
        let source_is_interface = self.is_interface_reference_type(types, value_type_id);
        let target_is_interface = self.is_interface_reference_type(types, target_type_id);

        // figure out if we need a representation change cast
        let requires_interface_upcast =
            target_is_interface && (!source_is_interface || value_is_concrete || !types_match);
        let requires_union_upcast =
            is_union_type(&target_type) && (value_is_concrete || value_is_nullish_literal);
        let requires_nullable_upcast = is_nullable_union(&target_type, types)
            && (value_is_concrete || value_is_nullish_literal);
        let requires_representation_cast =
            requires_interface_upcast || requires_union_upcast || requires_nullable_upcast;
        if types_match && !requires_representation_cast {
            return Ok(value_id);
        }

        // skip when the types are mutually assignable
        let options = self.analyze_context_options_for_module(module_id);
        let to_target = self.is_type_assignable(
            module,
            profile,
            symbols,
            target_type_id,
            value_type_id,
            types,
            &options,
        );
        let to_source = self.is_type_assignable(
            module,
            profile,
            symbols,
            value_type_id,
            target_type_id,
            types,
            &options,
        );
        if allow_assignable_skip
            && !requires_representation_cast
            && to_target.is_assignable()
            && to_source.is_assignable()
        {
            return Ok(value_id);
        }

        // skip when the value is already cast to an equivalent type
        if let Expression::Cast { target_type, .. } = tree.get(value_id)
            && let Some(existing_target_type_id) =
                self.type_id_for_type_expression(module_id, *target_type, tree, types)
        {
            let to_target = self.is_type_assignable(
                module,
                profile,
                symbols,
                target_type_id,
                existing_target_type_id,
                types,
                &options,
            );
            let to_source = self.is_type_assignable(
                module,
                profile,
                symbols,
                existing_target_type_id,
                target_type_id,
                types,
                &options,
            );
            if to_target.is_assignable() && to_source.is_assignable() {
                return Ok(value_id);
            }
        }

        // classify the cast
        let mut operator = self.cast_operator_for_types(
            module_id,
            profile,
            symbols,
            types,
            value_type_id,
            target_type_id,
            module,
        );
        if requires_interface_upcast && operator == CastOperator::Identity {
            operator = CastOperator::InstanceUpcast;
        }
        if requires_union_upcast && operator == CastOperator::Identity {
            operator = CastOperator::UnionUpcast;
        }
        if requires_nullable_upcast && operator == CastOperator::Identity {
            operator = CastOperator::NullableUpcast;
        }

        // skip redundant union and nullable upcasts
        if matches!(
            operator,
            CastOperator::UnionUpcast | CastOperator::NullableUpcast
        ) {
            let options = self.analyze_context_options_for_module(module_id);
            let to_target = self.is_type_assignable(
                module,
                profile,
                symbols,
                target_type_id,
                value_type_id,
                types,
                &options,
            );
            let to_source = self.is_type_assignable(
                module,
                profile,
                symbols,
                value_type_id,
                target_type_id,
                types,
                &options,
            );
            if to_target.is_assignable() && to_source.is_assignable() {
                return Ok(value_id);
            }
        }

        // skip numeric casts for scalar literals
        let value_type = types.get_type(value_type_id);
        if allow_assignable_skip
            && is_scalar_literal_type(value_type)
            && self.is_numeric_cast_operator(operator)
        {
            // align literal types with the selected numeric target
            types.set_inferred_type(value_id.into_global_any(module_id), target_type_id);
            return Ok(value_id);
        }
        if operator == CastOperator::Identity {
            // align equivalent types when numeric binaries need a unified representation
            if !allow_assignable_skip && value_type_id != target_type_id {
                types.set_inferred_type(value_id.into_global_any(module_id), target_type_id);
            }
            return Ok(value_id);
        }

        // build the target type expression
        let scope = tree.get_scope(origin_id);
        let target_expression_id =
            tree.reserve_from(NodeType::Expression, origin_id.into_any(), scope, None);
        let target_expression = match types.get_type(target_type_id) {
            Type::TypeLiteral { value } => Expression::TypeLiteral {
                value: value.clone(),
            },
            _ => Expression::Type {
                value: target_type_id,
            },
        };
        let target_expression_id = tree.insert(target_expression_id, target_expression);

        // set the inferred type for the target type expression
        let target_type_value = Type::Value {
            value: target_type_id,
        };
        let target_type_value_id = types.insert_type_from(target_type_value, target_expression_id);
        types.set_inferred_type(
            target_expression_id.into_global_any(module_id),
            target_type_value_id,
        );

        // insert the cast expression
        let cast_expression_id =
            tree.reserve_from(NodeType::Expression, origin_id.into_any(), scope, None);
        let cast_expression_id = tree.insert(
            cast_expression_id,
            Expression::Cast {
                operator,
                source: CastSource::Implicit,
                value: value_id,
                target_type: target_expression_id,
            },
        );
        types.set_inferred_type(
            cast_expression_id.into_global_any(module_id),
            target_type_id,
        );
        if self.options.elaborate_parenthesize_casts {
            let parenthesized_id =
                tree.reserve_from(NodeType::Expression, origin_id.into_any(), scope, None);
            let parenthesized_id = tree.insert(
                parenthesized_id,
                Expression::Parenthesized {
                    expression: cast_expression_id,
                },
            );
            types.set_inferred_type(parenthesized_id.into_global_any(module_id), target_type_id);
            return Ok(parenthesized_id);
        }

        Ok(cast_expression_id)
    }

    /// Resolve the value type id for an expression node.
    fn value_type_id_for_expression(
        &self,
        module_id: ModuleId,
        value_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        _symbols: &SymbolTable,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        // prefer declared or inferred types on the node
        if let Some(type_id) =
            types.get_declared_or_inferred_type_id(value_id.into_global_any(module_id))
        {
            return Some(self.unwrap_value_type_id(types, type_id));
        }

        // read the expression node
        let expression = tree.get(value_id);

        // prefer symbol value types when referencing a binding
        let symbol = match expression {
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => Some(*target_symbol),
            _ => None,
        };

        // return the symbol value type when available
        if let Some(symbol) = symbol
            && let Some(type_id) = types.get_value_type_id(symbol)
        {
            return Some(self.unwrap_value_type_id(types, type_id));
        }

        None
    }

    /// Resolve the type id encoded in a type expression.
    fn type_id_for_type_expression(
        &self,
        module_id: ModuleId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        // read the type expression node
        let expression = tree.get(expression_id);

        // use the explicit type id when available
        if let Expression::Type { value } = expression {
            return Some(*value);
        }

        // fall back to the inferred type value
        let type_id =
            types.get_declared_or_inferred_type_id(expression_id.into_global_any(module_id))?;
        match types.get_type(type_id) {
            Type::Value { value } => Some(*value),
            _ => Some(type_id),
        }
    }

    /// Classify the cast operator for two types.
    fn cast_operator_for_types(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        source_id: LocalTypeId,
        target_id: LocalTypeId,
        module: &Module,
    ) -> CastOperator {
        // fast path for identical types
        if source_id == target_id {
            return CastOperator::Identity;
        }

        // read the source and target types
        let source = types.get_type(source_id).clone();
        let target = types.get_type(target_id).clone();

        // semantic equality check (handles Foo→Foo, any→any, int32[]→int32[], etc.)
        if are_types_semantically_equal(&source, &target, types) {
            return CastOperator::Identity;
        }

        // handle any casts
        if is_any_type(&target) {
            return CastOperator::AnyUpcast;
        }
        if is_any_type(&source) {
            return CastOperator::AnyDowncast;
        }

        // handle unknown casts
        if is_unknown_type(&target) {
            return CastOperator::UnknownUpcast;
        }
        if is_unknown_type(&source) {
            return CastOperator::UnknownDowncast;
        }

        // handle object casts
        if is_object_type(&target) {
            return CastOperator::ObjectUpcast;
        }
        if is_object_type(&source) {
            return CastOperator::ObjectDowncast;
        }

        // handle numeric casts first
        if let Some(operator) = numeric_cast_operator(&source, &target) {
            return operator;
        }

        // handle pointer casts
        if is_pointer_type(&source) && is_integer_type(&target) {
            return CastOperator::PointerToInt;
        }
        if is_integer_type(&source) && is_pointer_type(&target) {
            return CastOperator::IntToPointer;
        }
        if is_pointer_type(&source) && is_pointer_type(&target) {
            return CastOperator::PointerCast;
        }

        // handle sized array to slice casts
        if let (
            Type::ArraySized { element, .. },
            Type::Array {
                element: target_element,
                ..
            },
        ) = (&source, &target)
        {
            let matches_element = target_element
                .map(|target_element| target_element == *element)
                .unwrap_or(true);
            if matches_element {
                return CastOperator::ArraySizedToSlice;
            }
        }

        // handle enum casts
        if let Some(operator) = self.enum_cast_operator(&source, &target, types) {
            return operator;
        }

        // handle nullable casts
        if is_nullable_union(&target, types) {
            return CastOperator::NullableUpcast;
        }
        if is_nullable_union(&source, types) {
            return CastOperator::NullableDowncast;
        }

        // handle union casts
        if is_union_type(&target) {
            return CastOperator::UnionUpcast;
        }
        if is_union_type(&source) {
            return CastOperator::UnionDowncast;
        }

        // fall back to assignability based instance casts
        let options = self.analyze_context_options_for_module(module_id);
        let assignable = self.is_type_assignable(
            module, profile, symbols, target_id, source_id, types, &options,
        );
        if assignable.is_assignable() {
            CastOperator::InstanceUpcast
        } else {
            CastOperator::InstanceDowncast
        }
    }

    /// Check whether implicit collection reification is enabled for this profile.
    fn collection_reify_is_enabled(
        &self,
        module: &Module,
        profile: ProfileId,
        origin_id: LocalNodeId<Expression>,
    ) -> ElaborateResult<bool> {
        // read the conversion policy from dsconfig
        let policy = self
            .program
            .with_dsconfig_options(module, |ds| ds.compiler.implicit_collection_conversions)
            .unwrap_or(ImplicitCollectionConversionPolicy::Allow);

        // skip reify for non-native outputs
        if !self.program.profile(profile).key.output.is_native() {
            return Ok(false);
        }

        // honor the configured policy for native outputs
        match policy {
            ImplicitCollectionConversionPolicy::Allow => Ok(true),
            ImplicitCollectionConversionPolicy::Warn => {
                self.warning(ElaborateWarning::ImplicitCollectionConversion {
                    node: origin_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile)),
                });
                Ok(true)
            }
            ImplicitCollectionConversionPolicy::Deny => {
                Err(ElaborateError::ImplicitCollectionConversion {
                    node: origin_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile)),
                })
            }
        }
    }

    /// Reify record like object literals into map construction.
    fn reify_record_like_object_literal(
        &self,
        module: &Module,
        profile: ProfileId,
        origin_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        target_type_id: LocalTypeId,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        // only reify record like literals when enabled for the profile
        if !self.collection_reify_is_enabled(module, profile, origin_id)? {
            return Ok(None);
        }

        // require an object literal value
        let Expression::ObjectExpression { properties } = tree.get(value_id).clone() else {
            return Ok(None);
        };

        // resolve the record key/value types
        let Some(record_like) =
            self.record_like_map_target(module, profile, origin_id, target_type_id, types)?
        else {
            return Ok(None);
        };

        // resolve Map.from symbol
        let map_from_symbol = self.map_from_symbol(
            module,
            profile,
            origin_id,
            record_like.map_symbol,
            tree,
            symbols,
        )?;

        // register a concrete instance for Map.from<K, V>
        let map_from_instance_id = types
            .find_instance(map_from_symbol, &record_like.map_static_arguments)
            .unwrap_or_else(|| {
                let instance = destack_dir::Instance::new(
                    map_from_symbol,
                    record_like.map_static_arguments.clone(),
                );
                types.insert_instance(instance)
            });

        // build tuple entries for each property
        let mut entry_arguments = Vec::with_capacity(properties.len());
        for property_id in properties {
            let property = tree.get(property_id);
            let (key, value) = match property {
                destack_dir::Property::Field { key, value, .. } => {
                    let key = key.ok_or(ElaborateError::UnsupportedConstruct {
                        node: property_id
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                    })?;
                    let value = value.ok_or(ElaborateError::UnsupportedConstruct {
                        node: property_id
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                    })?;
                    (key, value)
                }
                destack_dir::Property::Method { .. } | destack_dir::Property::Spread { .. } => {
                    return Err(ElaborateError::UnsupportedConstruct {
                        node: property_id
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                    });
                }
            };

            // resolve the key expression
            let key_expression_id = self.record_key_expression(
                module,
                profile,
                origin_id,
                key,
                record_like.key_type_id,
                tree,
                types,
            )?;

            // build a tuple expression for the entry
            let entry_tuple_id = self.record_entry_tuple_expression(
                module,
                profile,
                origin_id,
                key_expression_id,
                value,
                record_like.entry_tuple_type_id,
                tree,
                types,
            )?;

            // wrap tuple entries as array arguments
            let entry_argument_id = tree.reserve_from(
                NodeType::Argument,
                origin_id.into_any(),
                tree.get_scope(origin_id),
                None,
            );
            let entry_argument_id = tree.insert(
                entry_argument_id,
                Argument::Positional {
                    modifiers: None,
                    value: entry_tuple_id,
                },
            );
            entry_arguments.push(entry_argument_id);
        }

        // build the entries array expression
        let entries_array_id = tree.reserve_from(
            NodeType::Expression,
            origin_id.into_any(),
            tree.get_scope(origin_id),
            None,
        );
        let entries_array_id = tree.insert(
            entries_array_id,
            Expression::ArrayExpression {
                elements: entry_arguments,
            },
        );
        types.set_inferred_type(
            entries_array_id.into_global_any(module.id),
            record_like.entries_array_type_id,
        );

        // build a module reference to Map.from
        let map_name = self
            .program
            .strings
            .intern(WellKnownSymbol::Map.export_name());
        let from_name = self.program.strings.intern("from");
        let left_id = tree.reserve_from(
            NodeType::Expression,
            origin_id.into_any(),
            tree.get_scope(origin_id),
            None,
        );
        let left_id = tree.insert(
            left_id,
            Expression::ModuleReference {
                path: Path::from(&[map_name, from_name][..]),
                static_arguments: None,
                target_symbol: map_from_symbol,
            },
        );

        // build the Map.from call expression
        let call_id = tree.reserve_from(
            NodeType::Expression,
            origin_id.into_any(),
            tree.get_scope(origin_id),
            None,
        );
        let entries_argument_id = self.argument_for_value(origin_id, entries_array_id, tree);
        let call_id = tree.insert(
            call_id,
            Expression::Call {
                left: left_id,
                static_arguments: None,
                dynamic_arguments: vec![entries_argument_id],
            },
        );
        types.set_inferred_type(call_id.into_global_any(module.id), record_like.map_type_id);

        // attach a static resolution for the generated call
        let resolution_id = types.insert_resolution(Resolution::Static {
            receiver: None,
            candidate: ResolutionCandidate {
                key: None,
                target_symbol: map_from_symbol,
                instance: Some(map_from_instance_id),
                resolved_signature: Some(ResolvedSignature {
                    dynamic_parameters: vec![record_like.entries_array_type_id],
                    return_type: Some(record_like.map_type_id),
                    static_arguments: record_like.map_static_arguments,
                }),
            },
        });
        types.set_resolution_for_node(call_id.into_global_any(module.id), resolution_id);

        Ok(Some(call_id))
    }

    /// Reify sized arrays into dynamic arrays with Array.fromSized.
    fn reify_array_sized_value(
        &self,
        module: &Module,
        profile: ProfileId,
        origin_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        target_type_id: LocalTypeId,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> ElaborateResult<Option<LocalNodeId<Expression>>> {
        // only reify array casts when enabled for the profile
        if !self.collection_reify_is_enabled(module, profile, origin_id)? {
            return Ok(None);
        }

        // resolve source and target types
        let value_type_id = self
            .value_type_id_for_expression(module.id, value_id, tree, symbols, types)
            .ok_or(ElaborateError::UnsupportedConstruct {
                node: value_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
            })?;
        let value_type_id = self.unwrap_value_type_id(types, value_type_id);
        let target_type_id = self.unwrap_value_type_id(types, target_type_id);
        let value_type = types.get_type(value_type_id).clone();
        let target_type = types.get_type(target_type_id).clone();

        // resolve the dynamic array target type
        let (element_type_id, target_element_type_id) = match (value_type, target_type) {
            (
                Type::ArraySized {
                    element: value_element,
                    ..
                },
                Type::Array {
                    element: target_element,
                    ..
                },
            ) => (value_element, target_element),
            (
                Type::ArraySized {
                    element: value_element,
                    ..
                },
                Type::Reference {
                    symbol,
                    static_arguments,
                },
            ) => {
                let Some(well_known) = self.well_known_array_kind(profile, symbol) else {
                    return Ok(None);
                };
                let Some(Type::Array { element, .. }) = self.normalize_well_known_type_reference(
                    module,
                    symbols,
                    profile,
                    origin_id.into_any(),
                    symbol,
                    well_known,
                    static_arguments.as_deref(),
                    types,
                ) else {
                    return Ok(None);
                };
                (value_element, element)
            }
            _ => return Ok(None),
        };

        // ensure element compatibility when the target is explicit
        if let Some(target_element_type_id) = target_element_type_id {
            let options = self.analyze_context_options_for_module(module.id);
            let to_target = self.is_type_assignable(
                module,
                profile,
                symbols,
                target_element_type_id,
                element_type_id,
                types,
                &options,
            );
            let to_source = self.is_type_assignable(
                module,
                profile,
                symbols,
                element_type_id,
                target_element_type_id,
                types,
                &options,
            );
            if !to_target.is_assignable() || !to_source.is_assignable() {
                return Ok(None);
            }
        }

        // resolve Array.fromSized symbol
        let array_symbol = self
            .get_well_known_type_symbol(profile, WellKnownSymbol::Array)
            .ok_or(ElaborateError::UnsupportedConstruct {
                node: origin_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
            })?;
        let from_sized_symbol = self.array_from_sized_symbol(
            module,
            profile,
            origin_id,
            array_symbol,
            tree,
            symbols,
        )?;

        // register a concrete instance for Array.fromSized<T>
        let array_static_arguments = vec![StaticArgument::value(StaticExpression::Type {
            ty: element_type_id,
        })];
        let from_sized_instance_id = types
            .find_instance(from_sized_symbol, &array_static_arguments)
            .unwrap_or_else(|| {
                let instance =
                    destack_dir::Instance::new(from_sized_symbol, array_static_arguments.clone());
                types.insert_instance(instance)
            });

        // build a module reference to Array.fromSized
        let array_name = self
            .program
            .strings
            .intern(WellKnownSymbol::Array.export_name());
        let from_name = self.program.strings.intern("fromSized");
        let left_id = tree.reserve_from(
            NodeType::Expression,
            origin_id.into_any(),
            tree.get_scope(origin_id),
            None,
        );
        let left_id = tree.insert(
            left_id,
            Expression::ModuleReference {
                path: Path::from(&[array_name, from_name][..]),
                static_arguments: None,
                target_symbol: from_sized_symbol,
            },
        );

        // build the Array.fromSized call expression
        let call_id = tree.reserve_from(
            NodeType::Expression,
            origin_id.into_any(),
            tree.get_scope(origin_id),
            None,
        );
        let value_argument_id = self.argument_for_value(origin_id, value_id, tree);
        let call_id = tree.insert(
            call_id,
            Expression::Call {
                left: left_id,
                static_arguments: None,
                dynamic_arguments: vec![value_argument_id],
            },
        );
        types.set_inferred_type(call_id.into_global_any(module.id), target_type_id);

        // attach a static resolution for the generated call
        let resolution_id = types.insert_resolution(Resolution::Static {
            receiver: None,
            candidate: ResolutionCandidate {
                key: None,
                target_symbol: from_sized_symbol,
                instance: Some(from_sized_instance_id),
                resolved_signature: Some(ResolvedSignature {
                    dynamic_parameters: vec![value_type_id],
                    return_type: Some(target_type_id),
                    static_arguments: array_static_arguments,
                }),
            },
        });
        types.set_resolution_for_node(call_id.into_global_any(module.id), resolution_id);

        Ok(Some(call_id))
    }

    /// Resolve record like target information for map reification.
    fn record_like_map_target(
        &self,
        module: &Module,
        profile: ProfileId,
        origin_id: LocalNodeId<Expression>,
        target_type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> ElaborateResult<Option<RecordLikeTargetInfo>> {
        // unwrap value wrapper types
        let target_type_id = self.unwrap_value_type_id(types, target_type_id);

        // resolve Map symbol for record like lowering
        let Some(map_symbol) = self.get_well_known_type_symbol(profile, WellKnownSymbol::Map)
        else {
            return Ok(None);
        };
        let record_symbol = self.get_well_known_type_symbol(profile, WellKnownSymbol::Record);

        // walk aliases until we reach a record like target
        let mut current_type_id = target_type_id;
        let mut visited = Vec::new();
        loop {
            // avoid alias resolution cycles
            if visited.contains(&current_type_id) {
                return Ok(None);
            }
            visited.push(current_type_id);

            // read the current target type
            let target_type = types.get_type(current_type_id).clone();

            // use direct object index signatures as map targets
            if let Type::Object {
                index_signatures, ..
            } = &target_type
            {
                let Some(signature) = index_signatures.first() else {
                    return Ok(None);
                };
                let info = self.record_like_target_for_types(
                    origin_id,
                    map_symbol,
                    signature.key_type,
                    signature.value_type,
                    types,
                )?;
                return Ok(Some(info));
            }

            // resolve record-like references to Map<K, V>
            if let Type::Reference {
                symbol,
                static_arguments,
            } = &target_type
            {
                // only accept Map and Record symbols
                let is_map_symbol = *symbol == map_symbol
                    || record_symbol.is_some_and(|record_symbol| record_symbol == *symbol);
                if !is_map_symbol {
                    // follow alias targets when present
                    if let Some(alias_target_id) = types.get_alias_target_type_id(*symbol) {
                        current_type_id = alias_target_id;
                        continue;
                    }

                    return Ok(None);
                }

                // resolve key and value type ids from static arguments
                let Some(static_arguments) = static_arguments.as_deref() else {
                    return Ok(None);
                };
                if static_arguments.len() != 2 {
                    return Ok(None);
                }

                let key_type_id = self.static_argument_type_id(
                    module,
                    profile,
                    origin_id,
                    &static_arguments[0],
                    types,
                )?;
                let value_type_id = self.static_argument_type_id(
                    module,
                    profile,
                    origin_id,
                    &static_arguments[1],
                    types,
                )?;

                let info = self.record_like_target_for_types(
                    origin_id,
                    map_symbol,
                    key_type_id,
                    value_type_id,
                    types,
                )?;
                return Ok(Some(info));
            }

            return Ok(None);
        }
    }

    /// Build record like info for key/value types.
    fn record_like_target_for_types(
        &self,
        origin_id: LocalNodeId<Expression>,
        map_symbol: GlobalSymbolId,
        key_type_id: LocalTypeId,
        value_type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> ElaborateResult<RecordLikeTargetInfo> {
        // build the tuple entry type
        let entry_tuple_type = Type::Tuple {
            elements: vec![
                TypeElement::new(key_type_id),
                TypeElement::new(value_type_id),
            ],
            is_readonly: false,
        };
        let entry_tuple_type_id = types.insert_type_from(entry_tuple_type, origin_id);

        // build the entries array type
        let entries_array_type = Type::Array {
            element: Some(entry_tuple_type_id),
            is_readonly: false,
        };
        let entries_array_type_id = types.insert_type_from(entries_array_type, origin_id);

        // build the Map<K, V> reference type
        let map_static_arguments = vec![
            StaticArgument::value(StaticExpression::Type { ty: key_type_id }),
            StaticArgument::value(StaticExpression::Type { ty: value_type_id }),
        ];
        let map_type_id = types.insert_type_from(
            Type::Reference {
                symbol: map_symbol,
                static_arguments: Some(map_static_arguments.clone()),
            },
            origin_id,
        );

        Ok(RecordLikeTargetInfo {
            key_type_id,
            map_symbol,
            map_type_id,
            entry_tuple_type_id,
            entries_array_type_id,
            map_static_arguments,
        })
    }

    /// Resolve a single static argument to a type id.
    fn static_argument_type_id(
        &self,
        module: &Module,
        profile: ProfileId,
        origin_id: LocalNodeId<Expression>,
        argument: &StaticArgument,
        types: &mut TypeTable,
    ) -> ElaborateResult<LocalTypeId> {
        // require an evaluated static argument
        let StaticArgument::Evaluated { value, .. } = argument else {
            return Err(ElaborateError::UnsupportedConstruct {
                node: origin_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
            });
        };

        // resolve type expressions to concrete ids
        match value {
            StaticExpression::Type { ty } => Ok(*ty),
            StaticExpression::TypeLiteral { value } => {
                let literal_type_id = types.insert_type_from(
                    Type::TypeLiteral {
                        value: value.clone(),
                    },
                    origin_id,
                );
                Ok(literal_type_id)
            }
            _ => Err(ElaborateError::UnsupportedConstruct {
                node: origin_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
            }),
        }
    }

    /// Build a key expression for a record like object property.
    fn record_key_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        origin_id: LocalNodeId<Expression>,
        key: DynamicKey,
        key_type_id: LocalTypeId,
        tree: &mut NodeTree,
        types: &mut TypeTable,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // require computed keys for non-string index signatures
        let key_type = types.get_type(key_type_id);
        let key_is_string = is_string_type(key_type)
            || matches!(
                key_type,
                Type::Reference { symbol, .. }
                    if self.is_well_known_symbol(profile, *symbol, WellKnownSymbol::String)
            );
        let is_static_key = matches!(key, DynamicKey::Name(_) | DynamicKey::Number(_));
        if is_static_key && !key_is_string {
            return Err(ElaborateError::UnsupportedConstruct {
                node: origin_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
            });
        }

        // use direct expressions for computed keys
        let key_expression_id = match key {
            DynamicKey::Expression(expression) => expression,
            DynamicKey::NamedExpression { key, .. } => key,
            DynamicKey::Name(name) | DynamicKey::Number(name) => {
                // create a string literal for static keys
                let key_expression_id = tree.reserve_from(
                    NodeType::Expression,
                    origin_id.into_any(),
                    tree.get_scope(origin_id),
                    None,
                );
                let key_expression_id = tree.insert(
                    key_expression_id,
                    Expression::ScalarLiteral {
                        value: ScalarLiteral::String(name),
                    },
                );

                // assign the key type for literal values
                types.set_inferred_type(key_expression_id.into_global_any(module.id), key_type_id);

                key_expression_id
            }
            DynamicKey::Private(_) => {
                return Err(ElaborateError::UnsupportedConstruct {
                    node: origin_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile)),
                });
            }
        };

        Ok(key_expression_id)
    }

    /// Build a tuple expression for a record entry.
    fn record_entry_tuple_expression(
        &self,
        module: &Module,
        _profile: ProfileId,
        origin_id: LocalNodeId<Expression>,
        key_expression_id: LocalNodeId<Expression>,
        value_expression_id: LocalNodeId<Expression>,
        tuple_type_id: LocalTypeId,
        tree: &mut NodeTree,
        types: &mut TypeTable,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // allocate tuple argument nodes
        let key_argument_id = tree.reserve_from(
            NodeType::Argument,
            origin_id.into_any(),
            tree.get_scope(origin_id),
            None,
        );
        let key_argument_id = tree.insert(
            key_argument_id,
            Argument::Positional {
                modifiers: None,
                value: key_expression_id,
            },
        );
        let value_argument_id = tree.reserve_from(
            NodeType::Argument,
            origin_id.into_any(),
            tree.get_scope(origin_id),
            None,
        );
        let value_argument_id = tree.insert(
            value_argument_id,
            Argument::Positional {
                modifiers: None,
                value: value_expression_id,
            },
        );

        // build the tuple expression
        let tuple_expression_id = tree.reserve_from(
            NodeType::Expression,
            origin_id.into_any(),
            tree.get_scope(origin_id),
            None,
        );
        let tuple_expression_id = tree.insert(
            tuple_expression_id,
            Expression::TupleExpression {
                elements: vec![key_argument_id, value_argument_id],
            },
        );
        types.set_inferred_type(
            tuple_expression_id.into_global_any(module.id),
            tuple_type_id,
        );

        Ok(tuple_expression_id)
    }

    /// Resolve the Map.from symbol for record like reification.
    fn map_from_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        origin_id: LocalNodeId<Expression>,
        map_symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> ElaborateResult<GlobalSymbolId> {
        // locate the member key
        let name = self.program.strings.intern("from");
        let member_key = StaticKey::Name(name);

        self.resolve_static_member_symbol(
            module,
            profile,
            origin_id,
            map_symbol,
            member_key,
            tree,
            symbols,
        )
        .map_err(|_| ElaborateError::UnsupportedConstruct {
            node: origin_id
                .into_global_any(module.id)
                .into_anchored(Some(profile)),
        })
    }

    /// Resolve the Array.fromSized symbol for sized array reification.
    fn array_from_sized_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        origin_id: LocalNodeId<Expression>,
        array_symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> ElaborateResult<GlobalSymbolId> {
        // locate the member key
        let name = self.program.strings.intern("fromSized");
        let member_key = StaticKey::Name(name);

        self.resolve_static_member_symbol(
            module,
            profile,
            origin_id,
            array_symbol,
            member_key,
            tree,
            symbols,
        )
        .map_err(|_| ElaborateError::UnsupportedConstruct {
            node: origin_id
                .into_global_any(module.id)
                .into_anchored(Some(profile)),
        })
    }

    /// Build a positional argument for a value expression.
    fn argument_for_value(
        &self,
        origin_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        tree: &mut NodeTree,
    ) -> LocalNodeId<Argument> {
        let argument_id = tree.reserve_from(
            NodeType::Argument,
            origin_id.into_any(),
            tree.get_scope(origin_id),
            None,
        );
        tree.insert(
            argument_id,
            Argument::Positional {
                modifiers: None,
                value: value_id,
            },
        )
    }

    /// Check whether a cast operator is numeric.
    fn is_numeric_cast_operator(&self, operator: CastOperator) -> bool {
        // match numeric cast operators
        matches!(
            operator,
            CastOperator::IntWiden
                | CastOperator::IntNarrow
                | CastOperator::IntSignChange
                | CastOperator::FloatWiden
                | CastOperator::FloatNarrow
                | CastOperator::IntToFloat
                | CastOperator::FloatToInt
        )
    }

    /// Classify enum casts between enum and primitive types.
    fn enum_cast_operator(
        &self,
        source: &Type,
        target: &Type,
        types: &TypeTable,
    ) -> Option<CastOperator> {
        // shared enum backing lookup
        let backing_for_type = |ty: &Type| -> Option<EnumBackingType> {
            // only enum references have a backing type
            let Type::Reference { symbol, .. } = ty else {
                return None;
            };

            types.get_enum_backing_type(*symbol)
        };

        // enum to primitive casts
        if let Some(backing) = backing_for_type(source) {
            match backing {
                EnumBackingType::Int(_) if is_integer_type(target) => {
                    return Some(CastOperator::EnumToInt);
                }
                EnumBackingType::String if is_string_type(target) => {
                    return Some(CastOperator::EnumToString);
                }
                _ => {}
            }
        }

        // primitive to enum casts
        if let Some(backing) = backing_for_type(target) {
            match backing {
                EnumBackingType::Int(_) if is_integer_type(source) => {
                    return Some(CastOperator::IntToEnum);
                }
                EnumBackingType::String if is_string_type(source) => {
                    return Some(CastOperator::StringToEnum);
                }
                _ => {}
            }
        }

        None
    }

    /// Strip value wrapper types to reach the underlying type id.
    fn unwrap_value_type_id(&self, types: &TypeTable, type_id: LocalTypeId) -> LocalTypeId {
        // peel value wrapper types
        let mut current = type_id;
        loop {
            match types.get_type(current) {
                Type::Value { value } => {
                    current = *value;
                }
                _ => return current,
            }
        }
    }

    /// Return true when a type id points at an interface reference type.
    fn is_interface_reference_type(&self, types: &TypeTable, type_id: LocalTypeId) -> bool {
        // resolve the underlying type id
        let type_id = self.unwrap_value_type_id(types, type_id);

        // check for interface reference types
        matches!(
            types.get_type(type_id),
            Type::Reference { symbol, .. } if symbol.ty() == SymbolType::Interface
        )
    }

    /// Return true when a new expression targets a nominal class or struct.
    fn is_concrete_new_expression(
        &self,
        tree: &NodeTree,
        value_id: LocalNodeId<Expression>,
    ) -> bool {
        // require a new expression
        let Expression::New { left, .. } = tree.get(value_id) else {
            return false;
        };

        // accept nominal constructor targets
        match tree.get(*left) {
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => {
                matches!(target_symbol.ty(), SymbolType::Class | SymbolType::Struct)
            }
            _ => false,
        }
    }

    /// Return true when a value expression is a tagged constructor literal.
    fn is_tagged_expression(&self, tree: &NodeTree, value_id: LocalNodeId<Expression>) -> bool {
        // check tagged constructor expressions
        matches!(
            tree.get(value_id),
            Expression::TaggedScalarExpression { .. }
                | Expression::TaggedTupleExpression { .. }
                | Expression::TaggedObjectExpression { .. }
        )
    }

    /// Return true when a value expression resolves to a concrete nominal symbol.
    fn is_concrete_resolution(
        &self,
        module_id: ModuleId,
        value_id: LocalNodeId<Expression>,
        types: &TypeTable,
    ) -> bool {
        // resolve the node resolution
        let node_id = value_id.into_global_any(module_id);
        let Some(resolution_id) = types.get_resolution_for_node(node_id) else {
            return false;
        };
        let resolution = types.get_resolution(resolution_id);

        // accept concrete static resolutions only
        match resolution {
            Resolution::Static { candidate, .. } => {
                self.is_concrete_resolution_candidate(candidate, types)
            }
            Resolution::Dynamic { .. }
            | Resolution::Unresolved { .. }
            | Resolution::Builtin { .. } => false,
        }
    }

    /// Return true when a resolution candidate targets a concrete nominal symbol.
    fn is_concrete_resolution_candidate(
        &self,
        candidate: &ResolutionCandidate,
        types: &TypeTable,
    ) -> bool {
        // accept direct nominal symbols
        if matches!(
            candidate.target_symbol.ty(),
            SymbolType::Class | SymbolType::Struct
        ) {
            return true;
        }

        // fall back to the resolved return type
        let Some(resolved_signature) = candidate.resolved_signature.as_ref() else {
            return false;
        };
        let Some(return_type) = resolved_signature.return_type else {
            return false;
        };

        // require a nominal return type
        let return_type = self.unwrap_value_type_id(types, return_type);
        matches!(
            types.get_type(return_type),
            Type::Reference { symbol, .. }
                if matches!(symbol.ty(), SymbolType::Class | SymbolType::Struct)
        )
    }

    /// Check whether a binary operator is numeric.
    fn is_numeric_binary_operator(&self, operator: BinaryOperator) -> bool {
        // match numeric binary operators
        matches!(
            operator,
            BinaryOperator::Multiply
                | BinaryOperator::WrappingMultiply
                | BinaryOperator::SaturatingMultiply
                | BinaryOperator::Exponent
                | BinaryOperator::WrappingExponent
                | BinaryOperator::SaturatingExponent
                | BinaryOperator::Divide
                | BinaryOperator::Remainder
                | BinaryOperator::Add
                | BinaryOperator::WrappingAdd
                | BinaryOperator::SaturatingAdd
                | BinaryOperator::Subtract
                | BinaryOperator::WrappingSubtract
                | BinaryOperator::SaturatingSubtract
                | BinaryOperator::ShiftLeft
                | BinaryOperator::SaturatingShiftLeft
                | BinaryOperator::ShiftRight
                | BinaryOperator::UnsignedShiftRight
                | BinaryOperator::ElementwiseAnd
                | BinaryOperator::ElementwiseXor
                | BinaryOperator::ElementwiseOr
                | BinaryOperator::Equal
                | BinaryOperator::NotEqual
                | BinaryOperator::EqualStrict
                | BinaryOperator::NotEqualStrict
                | BinaryOperator::LessThan
                | BinaryOperator::LessThanOrEqual
                | BinaryOperator::GreaterThan
                | BinaryOperator::GreaterThanOrEqual
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;
    use destack_workspace::OutputFormat;

    #[test]
    fn test_reify_implicit_cast_in_binding() {
        // binding casts are inserted for mismatched types
        let test = TestProgram::memory_sequential_with_prelude_and_libs()
            .with_profile_output(OutputFormat::Native)
            .with_profile_libs(&["native"]);
        let module_id = test.add_module(
            "test.ds",
            r#"
function intValue(): int32 {
    return 1;
}

function test(): float {
    let value: float = intValue();
    return value;
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function intValue(): int32 {
    return 1;
}

function test(): float64 {
    let value = (intValue() as float64);
    return value;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_in_assignment() {
        // assignment casts are inserted for mismatched types
        let test = TestProgram::memory_sequential_with_prelude_and_libs()
            .with_profile_output(OutputFormat::Native)
            .with_profile_libs(&["native"]);
        let module_id = test.add_module(
            "test.ds",
            r#"
function intValue(): int32 {
    return 1;
}

function test(): float {
    let value: float = intValue();
    value = intValue();
    return value;
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function intValue(): int32 {
    return 1;
}

function test(): float64 {
    let value = (intValue() as float64);
    value = (intValue() as float64);
    return value;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_in_return() {
        // return casts are inserted for declared return types
        let test = TestProgram::memory_sequential_with_prelude_and_libs()
            .with_profile_output(OutputFormat::Native)
            .with_profile_libs(&["native"]);
        let module_id = test.add_module(
            "test.ds",
            r#"
function intValue(): int32 {
    return 1;
}

function test(): float {
    return intValue();
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function intValue(): int32 {
    return 1;
}

function test(): float64 {
    return (intValue() as float64);
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_in_call_argument() {
        // call arguments are cast to parameter types
        let test = TestProgram::memory_sequential_with_prelude_and_libs()
            .with_profile_output(OutputFormat::Native)
            .with_profile_libs(&["native"]);
        let module_id = test.add_module(
            "test.ds",
            r#"
function intValue(): int32 {
    return 1;
}

function takeFloat(value: float): float {
    return value;
}

function test(): float {
    return takeFloat(intValue());
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function intValue(): int32 {
    return 1;
}

function takeFloat(value): float64 {
    return value;
}

function test(): float64 {
    return takeFloat((intValue() as float64));
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_in_ternary() {
        // ternary branches cast to the expression type
        let test = TestProgram::memory_sequential_with_prelude_and_libs()
            .with_profile_output(OutputFormat::Native)
            .with_profile_libs(&["native"]);
        let module_id = test.add_module(
            "test.ds",
            r#"
function floatValue(): float {
    return 1;
}

function intValue(): int32 {
    return 1;
}

function test(condition: boolean): float {
    return condition ? floatValue() : intValue();
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function floatValue(): float64 {
    return 1;
}

function intValue(): int32 {
    return 1;
}

function test(condition): float64 {
    return condition ? floatValue() : (intValue() as float64);
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_in_match_expression() {
        // match case expressions cast to the match expression type
        let test = TestProgram::memory_sequential_with_prelude_and_libs()
            .with_profile_output(OutputFormat::Native)
            .with_profile_libs(&["native"]);
        let module_id = test.add_module(
            "test.ds",
            r#"
function intValue(): int32 {
    return 1;
}

function test(condition: boolean): float {
    let value: float = match (condition) {
        true => intValue()
        false => intValue()
    };
    return value;
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function intValue(): int32 {
    return 1;
}

function test(condition): float64 {
    let value;
    if (condition == true) {
        value = (intValue() as float64);
    } else {
        value = (intValue() as float64);
    }
    return value;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_in_match_block() {
        // match case blocks cast their trailing expressions
        let test = TestProgram::memory_sequential_with_prelude_and_libs()
            .with_profile_output(OutputFormat::Native)
            .with_profile_libs(&["native"]);
        let module_id = test.add_module(
            "test.ds",
            r#"
function intValue(): int32 {
    return 1;
}

function test(condition: boolean): float {
    let value: float = match (condition) {
        true => {
            let value = intValue();
            value
        }
        false => {
            let value = intValue();
            value
        }
    };
    return value;
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function intValue(): int32 {
    return 1;
}

function test(condition): float64 {
    let value;
    if (condition == true) {
        let value = intValue();
        value = (value as float64);
    } else {
        let value = intValue();
        value = (value as float64);
    }
    return value;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_in_binary_comparison() {
        // comparison expressions cast numeric literals for alignment
        let test = TestProgram::memory_sequential_with_prelude_and_libs()
            .with_profile_output(OutputFormat::Native)
            .with_profile_libs(&["native"]);
        let module_id = test.add_module(
            "test.ds",
            r#"
function test(value: float): boolean {
    return value < 2;
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function test(value): boolean {
    return value < (2 as float64);
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_in_binary_arithmetic() {
        // arithmetic expressions do not cast numeric literals
        let test = TestProgram::memory_sequential_with_prelude_and_libs()
            .with_profile_output(OutputFormat::Native)
            .with_profile_libs(&["native"]);
        let module_id = test.add_module(
            "test.ds",
            r#"
function test(value: float): float {
    return value - 1;
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function test(value): float64 {
    return value - 1;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_in_binary_left_literal() {
        // numeric literals do not cast to the non literal side
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function test(value: float): float {
    return 2 + value;
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function test(value): float64 {
    return 2 + value;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_union_upcast() {
        // union upcasts are inserted for union bindings
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function intValue(): int32 {
    return 1;
}

function test(): int32 | float64 {
    let value: int32 | float64 = intValue();
    return value;
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function intValue(): int32 {
    return 1;
}

function test(): int32 | float64 {
    let value = (intValue() as int32 | float64);
    return value;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_in_reference_narrowing() {
        // narrowed references insert implicit downcasts
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
struct Foo {
    x: int32;
}

struct Bar {
    y: int32;
}

function test(value: Foo | Bar): int32 {
    if (value is Foo) {
        return value.x;
    }
    return 0;
}
"#,
        );

        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
struct Foo {
    x: int32,
}

struct Bar {
    y: int32,
}

function test(value): int32 {
    if (value is Foo) {
        return (value as Foo).x;
    }
    return 0;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_in_guard_after_narrowing() {
        // narrowed references inside guard expressions insert casts
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
struct Foo {
    x: int32;
}

struct Bar {
    y: int32;
}

function test(value: Foo | Bar): int32 {
    if (value is Foo && value.x > 0) {
        return value.x;
    }
    return 0;
}
"#,
        );

        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
struct Foo {
    x: int32,
}

struct Bar {
    y: int32,
}

function test(value): int32 {
    if (value is Foo && (value as Foo).x > (0 as int32)) {
        return (value as Foo).x;
    }
    return 0;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_in_guard_multiple_checks() {
        // narrowed references inside compound guards insert casts consistently
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
struct Foo {
    x: int32;
}

struct Bar {
    y: int32;
}

function test(value: Foo | Bar): int32 {
    if (value is Foo && value.x > 0 && value.x < 10) {
        return value.x;
    }
    return 0;
}
"#,
        );

        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
struct Foo {
    x: int32,
}

struct Bar {
    y: int32,
}

function test(value): int32 {
    if (value is Foo && (value as Foo).x > (0 as int32) && (value as Foo).x < (10 as int32)) {
        return (value as Foo).x;
    }
    return 0;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_in_while_guard() {
        // while guards insert casts for narrowed references
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
struct Foo {
    x: int32;
}

struct Bar {
    y: int32;
}

function test(value: Foo | Bar): int32 {
    while (value is Foo && value.x > 0) {
        break;
    }
    return 0;
}
"#,
        );

        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
struct Foo {
    x: int32,
}

struct Bar {
    y: int32,
}

function test(value): int32 {
    while (value is Foo && (value as Foo).x > (0 as int32)) {
        break;
    }
    return 0;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_skips_is_operand() {
        // guard operands keep their original references
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
struct Foo {
    x: int32;
}

struct Bar {
    y: int32;
}

function isFoo(value: Foo | Bar): boolean {
    return value is Foo;
}
"#,
        );

        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
struct Foo {
    x: int32,
}

struct Bar {
    y: int32,
}

function isFoo(value): boolean {
    return value is Foo;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_in_typeof_guard() {
        // typeof guards narrow references without casting the guard operand
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function test(value: (() => int32) | int32): int32 {
    if (typeof value == "function") {
        return value();
    }
    return value;
}
"#,
        );

        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function test(value): int32 {
    if (typeof value == "function") {
        return (value as () => int32)();
    }
    return (value as int32);
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_in_instanceof_guard() {
        // instanceof guards narrow references without casting the guard operand
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
class Foo {
    x: int32 = 0;
}

class Bar {
    y: int32 = 0;
}

function test(value: Foo | Bar): int32 {
    if (value instanceof Foo) {
        return value.x;
    }
    return 0;
}
"#,
        );

        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
class Foo {
    x: int32 = 0,
}

class Bar {
    y: int32 = 0,
}

function test(value): int32 {
    if (value instanceof Foo) {
        return (value as Foo).x;
    }
    return 0;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_in_in_guard() {
        // in guards narrow references without casting the guard operand
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
type WithX = { x: int32 };
type WithY = { y: int32 };

function test(value: WithX | WithY): int32 {
    if ("x" in value) {
        return value.x;
    }
    return 0;
}
"#,
        );

        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
type WithX = { x: int32 };

type WithY = { y: int32 };

function test(value): int32 {
    if ('x' in value) {
        return (value as { x: int32 }).x;
    }
    return 0;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_in_match_guard() {
        // match guards narrow case bodies without casting the guard operand
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
struct Foo {
    x: int32;
}

struct Bar {
    y: int32;
}

function test(value: Foo | Bar): int32 {
    match (value) {
        _ if value is Foo => value.x
        _ => 0
    }
}
"#,
        );

        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
struct Foo {
    x: int32,
}

struct Bar {
    y: int32,
}

function test(value): int32 {
    if (value is Foo) return (value as Foo).x; else {
        return 0;
    }
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_skips_cast_operand() {
        // explicit casts do not add extra implicit casts
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
struct Foo {
    x: int32;
}

struct Bar {
    y: int32;
}

function test(value: Foo | Bar): Foo {
    return value as Foo;
}
"#,
        );

        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
struct Foo {
    x: int32,
}

struct Bar {
    y: int32,
}

function test(value): Foo {
    return value as Foo;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_skips_tagged_type_reference() {
        // tagged constructors keep their type reference intact
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
struct Counter {
    value: int32;
}

function make(value: int32): Counter {
    return Counter { value };
}
"#,
        );

        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
struct Counter {
    value: int32,
}

function make(value): Counter {
    return Counter { value };
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_interface_upcast() {
        // interface upcasts are inserted for contextual constructor values
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
interface Greeter {
    greet(): int32;
}

class GreeterImpl implements Greeter {
    value: int32;

    constructor(value: int32) {
        this.value = value;
        return;
    }

    greet(): int32 { 
        return this.value;
    }
}

function test(): Greeter {
    let value: Greeter = new GreeterImpl(1);
    return value;
}
"#,
        );

        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
interface Greeter {
    greet(): int32
}

class GreeterImpl implements Greeter {
    value: int32,
    constructor(value) {
        this.value = value;
        return;
    }
    greet(): int32 {
        return this.value;
    }
}

function test(): Greeter {
    let value = (new GreeterImpl(1) as Greeter);
    return value;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_interface_to_interface() {
        // interface to interface casts are reified
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
interface Greeter {
    greet(): int32;
}

interface Speaker {
    greet(): int32;
}

function test(value: Greeter): Speaker {
    let assigned: Speaker = value;
    return assigned;
}
"#,
        );

        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
interface Greeter {
    greet(): int32
}

interface Speaker {
    greet(): int32
}

function test(value): Speaker {
    let assigned = (value as Speaker);
    return assigned;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_nullable_upcast() {
        // nullable upcasts are inserted for nullable bindings
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function intValue(): int32 {
    return 1;
}

function test(): int32 | null {
    let value: int32 | null = intValue();
    return value;
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function intValue(): int32 {
    return 1;
}

function test(): int32 | null {
    let value = (intValue() as int32 | null);
    return value;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_nullable_null_literal() {
        // nullable upcasts are inserted for null literals
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function test(): int32 | null {
    let value: int32 | null = null;
    return value;
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function test(): int32 | null {
    let value = (null as int32 | null);
    return value;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_undefined_literal_upcast() {
        // undefined upcasts are inserted for undefined literals
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function test(): int32 | undefined {
    let value: int32 | undefined = undefined;
    return value;
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function test(): int32 | undefined {
    let value = (undefined as int32 | undefined);
    return value;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_nullable_return_literal() {
        // nullable upcasts are inserted for return literals
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function test(): int32 | null {
    return null;
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function test(): int32 | null {
    return (null as int32 | null);
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_undefined_upcast() {
        // undefined upcasts are inserted for undefined unions
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function intValue(): int32 {
    return 1;
}

function test(): int32 | undefined {
    let value: int32 | undefined = intValue();
    return value;
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function intValue(): int32 {
    return 1;
}

function test(): int32 | undefined {
    let value = (intValue() as int32 | undefined);
    return value;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_nullable_undefined_call() {
        // nullable and undefined upcasts are inserted for call arguments
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function accept(value: int32 | null | undefined): int32 | null | undefined {
    return value;
}

function intValue(): int32 {
    return 1;
}

function test(): int32 | null | undefined {
    return accept(intValue());
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function accept(value): int32 | null | undefined {
    return value;
}

function intValue(): int32 {
    return 1;
}

function test(): int32 | null | undefined {
    return accept((intValue() as int32 | null | undefined));
}
"#,
        );
    }

    #[test]
    fn test_reify_explicit_cast_expression() {
        // explicit casts stay explicit
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function intValue(): int32 {
    return 1;
}

function test(): float {
    return intValue() as float;
}
"#,
        );

        // run elaborate
        test.elaborate_module(module_id);
        test.compile_check_clean();

        // assert elaborated
        test.assert_elaborated(
            module_id,
            r#"
function intValue(): int32 {
    return 1;
}

function test(): float64 {
    return intValue() as float64;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_in_using_binding() {
        // using bindings cast initializers when needed
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function intValue(): int32 {
    return 1;
}

function test(): void {
    using value: float = intValue();
}
"#,
        );

        // run elaborate
        test.elaborate_module(module_id);
        test.compile_check_clean();

        // assert elaborated
        test.assert_elaborated(
            module_id,
            r#"
function intValue(): int32 {
    return 1;
}

function test(): void {
    using value = (intValue() as float64);
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_skip_same_type() {
        // matching types do not insert casts
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function intValue(): int32 {
    return 1;
}

function test(): int32 {
    let value: int32 = intValue();
    return value;
}
"#,
        );

        // run elaborate
        test.elaborate_module(module_id);
        test.compile_check_clean();

        // assert elaborated
        test.assert_elaborated(
            module_id,
            r#"
function intValue(): int32 {
    return 1;
}

function test(): int32 {
    let value = intValue();
    return value;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_skip_numeric_literal() {
        // scalar literal numeric casts are omitted
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function test(): float {
    let value: float = 1;
    return value;
}
"#,
        );

        // run elaborate
        test.elaborate_module(module_id);
        test.compile_check_clean();

        // assert elaborated
        test.assert_elaborated(
            module_id,
            r#"
function test(): float64 {
    let value = 1;
    return value;
}
"#,
        );
    }

    #[test]
    fn test_reify_implicit_cast_object_upcast() {
        // non-primitives are implicitly upcast to object
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function getArray(): int32[] {
    return [1, 2, 3];
}

function test(): object {
    let value: object = getArray();
    return value;
}
"#,
        );

        // run elaborate
        test.elaborate_module(module_id);
        test.compile_check_clean();

        // assert elaborated: array literal is typed, getArray() → object needs upcast
        test.assert_elaborated(
            module_id,
            r#"
function getArray(): int32[] {
    return [1, 2, 3];
}

function test(): object {
    let value = (getArray() as object);
    return value;
}
"#,
        );
    }

    #[test]
    fn test_reify_explicit_cast_object_downcast() {
        // object is explicitly downcast to specific types
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
interface Foo { x: int32 }

function getObject(): object {
    return { x: 1 };
}

function test(): Foo {
    return getObject() as Foo;
}
"#,
        );

        // run elaborate
        test.elaborate_module(module_id);
        test.compile_check_clean();

        // assert elaborated: object literal → object upcast, explicit Foo downcast preserved
        test.assert_elaborated(
            module_id,
            r#"
interface Foo {
    x: int32,
}

function getObject(): object {
    return ({ x: 1 } as object);
}

function test(): Foo {
    return getObject() as Foo;
}
"#,
        );
    }

    #[test]
    fn test_reify_record_like_object_literal_in_binding() {
        // record like bindings reify object literals into map construction
        let test = TestProgram::memory_sequential_with_prelude_and_libs()
            .with_profile_output(OutputFormat::Native)
            .with_profile_libs(&["native"]);
        let module_id = test.add_module(
            "test.ds",
            r#"
function build(): Record<string, int32> {
    let record: Record<string, int32> = { alpha: 1, beta: 2 };
    return record;
}
"#,
        );

        // run elaborate
        test.elaborate_module(module_id);
        test.compile_check_clean();

        // assert elaborated: record literal reifies to Map.from
        test.assert_elaborated(
            module_id,
            r#"
function build(): Record<string, int32> {
    let record = Map.from([("alpha", 1,), ("beta", 2,)]);
    return record;
}
"#,
        );
    }

    #[test]
    fn test_reify_record_like_object_literal_in_assignment() {
        // record like assignments reify object literals into map construction
        let test = TestProgram::memory_sequential_with_prelude_and_libs()
            .with_profile_output(OutputFormat::Native)
            .with_profile_libs(&["native"]);
        let module_id = test.add_module(
            "test.ds",
            r#"
function build(): Record<string, int32> {
    let record: Record<string, int32> = { alpha: 1 };
    record = { beta: 2 };
    return record;
}
"#,
        );

        // run elaborate
        test.elaborate_module(module_id);
        test.compile_check_clean();

        // assert elaborated: record literals reify to Map.from
        test.assert_elaborated(
            module_id,
            r#"
function build(): Record<string, int32> {
    let record = Map.from([("alpha", 1,)]);
    record = Map.from([("beta", 2,)]);
    return record;
}
"#,
        );
    }

    #[test]
    fn test_reify_record_like_object_literal_in_return() {
        // record like returns reify object literals into map construction
        let test = TestProgram::memory_sequential_with_prelude_and_libs()
            .with_profile_output(OutputFormat::Native)
            .with_profile_libs(&["native"]);
        let module_id = test.add_module(
            "test.ds",
            r#"
function build(): Record<string, int32> {
    return { beta: 2 };
}
"#,
        );

        // run elaborate
        test.elaborate_module(module_id);
        test.compile_check_clean();

        // assert elaborated: record literal reifies to Map.from
        test.assert_elaborated(
            module_id,
            r#"
function build(): Record<string, int32> {
    return Map.from([("beta", 2,)]);
}
"#,
        );
    }

    #[test]
    fn test_reify_record_like_object_literal_in_call_argument() {
        // record like arguments reify object literals into map construction
        let test = TestProgram::memory_sequential_with_prelude_and_libs()
            .with_profile_output(OutputFormat::Native)
            .with_profile_libs(&["native"]);
        let module_id = test.add_module(
            "test.ds",
            r#"
function take(record: Record<string, int32>): void {
    return;
}

function test(): void {
    take({ beta: 2 });
}
"#,
        );

        // run elaborate
        test.elaborate_module(module_id);
        test.compile_check_clean();

        // assert elaborated: record literal reifies to Map.from
        test.assert_elaborated(
            module_id,
            r#"
function take(record): void {
    return;
}

function test(): void {
    take(Map.from([("beta", 2,)]));
}
"#,
        );
    }

    #[test]
    fn test_reify_record_like_object_literal_explicit_cast() {
        // record like explicit casts reify object literals into map construction
        let test = TestProgram::memory_sequential_with_prelude_and_libs()
            .with_profile_output(OutputFormat::Native)
            .with_profile_libs(&["native"]);
        let module_id = test.add_module(
            "test.ds",
            r#"
function build(): Record<string, int32> {
    return { beta: 2 } as Record<string, int32>;
}
"#,
        );

        // run elaborate
        test.elaborate_module(module_id);
        test.compile_check_clean();

        // assert elaborated: record literal reifies to Map.from
        test.assert_elaborated(
            module_id,
            r#"
function build(): Record<string, int32> {
    return Map.from([("beta", 2,)]);
}
"#,
        );
    }

    #[test]
    fn test_reify_record_like_object_literal_skips_plain_assignment() {
        // non-record assignments keep object literals
        let test = TestProgram::memory_sequential_with_prelude_and_libs()
            .with_profile_output(OutputFormat::Native)
            .with_profile_libs(&["native"]);
        let module_id = test.add_module(
            "test.ds",
            r#"
function buildPlain(): { beta: int32 } {
    let plain = { beta: 2 } as { beta: int32 };
    plain = { beta: 4 } as { beta: int32 };
    return plain;
}
"#,
        );

        // run elaborate
        test.elaborate_module(module_id);
        test.compile_check_clean();

        // assert elaborated: object literals remain
        test.assert_elaborated(
            module_id,
            r#"
function buildPlain(): { beta: int32 } {
    let plain = { beta: 2 } as { beta: int32 };
    plain = { beta: 4 } as { beta: int32 };
    return plain;
}
"#,
        );
    }

    #[test]
    fn test_reify_array_sized_value_in_binding() {
        // sized arrays reify into dynamic arrays in bindings
        let test = TestProgram::memory_sequential_with_prelude_and_libs()
            .with_profile_output(OutputFormat::Native)
            .with_profile_libs(&["native"]);
        let module_id = test.add_module(
            "test.ds",
            r#"
function build(values: int32[3]): int32[] {
    let dynamic: int32[] = values;
    return dynamic;
}
"#,
        );

        // run elaborate
        test.elaborate_module(module_id);
        test.compile_check_clean();

        // assert elaborated: sized array reifies to Array.fromSized
        test.assert_elaborated(
            module_id,
            r#"
function build(values): int32[] {
    let dynamic = Array.fromSized(values);
    return dynamic;
}
"#,
        );
    }

    #[test]
    fn test_reify_array_sized_value_in_return() {
        // sized arrays reify into dynamic arrays in returns
        let test = TestProgram::memory_sequential_with_prelude_and_libs()
            .with_profile_output(OutputFormat::Native)
            .with_profile_libs(&["native"]);
        let module_id = test.add_module(
            "test.ds",
            r#"
function build(values: int32[3]): int32[] {
    return values;
}
"#,
        );

        // run elaborate
        test.elaborate_module(module_id);
        test.compile_check_clean();

        // assert elaborated: sized array reifies to Array.fromSized
        test.assert_elaborated(
            module_id,
            r#"
function build(values): int32[] {
    return Array.fromSized(values);
}
"#,
        );
    }

    #[test]
    fn test_reify_array_sized_value_in_call_argument() {
        // sized arrays reify into dynamic arrays in call arguments
        let test = TestProgram::memory_sequential_with_prelude_and_libs()
            .with_profile_output(OutputFormat::Native)
            .with_profile_libs(&["native"]);
        let module_id = test.add_module(
            "test.ds",
            r#"
function take(values: int32[]): void {
    return;
}

function test(values: int32[3]): void {
    take(values);
}
"#,
        );

        // run elaborate
        test.elaborate_module(module_id);
        test.compile_check_clean();

        // assert elaborated: sized array reifies to Array.fromSized
        test.assert_elaborated(
            module_id,
            r#"
function take(values): void {
    return;
}

function test(values): void {
    take(Array.fromSized(values));
}
"#,
        );
    }
}
