use std::collections::HashMap;

use super::resolution::MemberResolution;
use crate::{
    AnalyzeError, AnalyzeResult, Assignability, Compiler, Constraint, InferContext, InferTable,
    OperatorLanguageItemExt,
};
use destack_builtin::LanguageItem;
use destack_dir::{
    BinaryOperator, Expression, GlobalTypeId, LocalInstanceId, LocalNodeId, LocalTypeId, NodeTree,
    ScalarLiteral, StaticKey, SymbolTable, Type, TypeField, TypeLiteral, TypeTable, UnaryOperator,
};
use destack_workspace::Module;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Infer a unary operator expression.
    pub(super) fn infer_unary_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        operator: &UnaryOperator,
        right_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        let right_ty_id =
            self.infer_expression(module, right_id, tree, symbols, types, infer, ctx)?;
        let right_ty = types.get_type(right_ty_id).clone();

        let operator_item = operator.language_item();

        // use builtin rules when appropriate
        if self.should_use_builtin_unary_operator(operator, &right_ty, types) {
            let ty = self.infer_unary_operation(operator, &right_ty);
            self.record_builtin_resolution(
                expression_id.into_global_any(module.id),
                Some(right_ty_id),
                types,
            );
            return Ok(types.insert_type_from(ty, expression_id));
        }

        // fall back to builtin inference when no operator interface exists
        let Some(operator_item) = operator_item else {
            let ty = self.infer_unary_operation(operator, &right_ty);
            return Ok(types.insert_type_from(ty, expression_id));
        };

        let operator_key = self.operator_member_key(operator_item);
        let mut member_instance_id = None;

        // inherit static arguments and substitutions from the receiver
        let inherited = self.resolve_inherited_static_arguments(
            module,
            right_id.into_any(),
            &right_ty,
            tree,
            symbols,
            types,
        )?;

        // require explicit operator interface implementation
        if !self.is_interface_implemented(&right_ty, operator_item, types) {
            self.error(AnalyzeError::NoOverload {
                node: expression_id.into_global_any(module.id),
                receiver_ty: right_ty_id.into_global(module.id),
            });
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        }

        // resolve member dispatch for the receiver type
        let member_resolution =
            self.resolve_member_resolution(module, &right_ty, &operator_key, tree, symbols, types);
        let member_symbol = match &member_resolution {
            MemberResolution::Static { symbol } => Some(*symbol),
            _ => None,
        };

        // infer the operator member type
        let mut member_type_visited = Vec::new();
        let member_ty_id = self.infer_member_of_type(
            module,
            &right_ty,
            &operator_key,
            types,
            &mut member_type_visited,
        );
        let has_member = member_ty_id.is_some();
        let resolved_return_ty_id = if let Some(member_ty_id) = member_ty_id {
            let member_ty_id = if !inherited.substitutions.is_empty() {
                let mut cache = HashMap::new();
                self.substitute_static_parameters_in_type(
                    member_ty_id,
                    &inherited.substitutions,
                    types,
                    &mut cache,
                )
            } else {
                member_ty_id
            };

            // resolve operator signature and register instance when needed
            match types.get_type(member_ty_id).clone() {
                Type::Function {
                    static_parameters,
                    dynamic_parameters,
                    return_type,
                    ..
                } => {
                    let resolved = self.resolve_function_type_for_call(
                        module,
                        expression_id.into_any(),
                        member_symbol,
                        None,
                        &static_parameters,
                        &dynamic_parameters,
                        return_type,
                        tree,
                        symbols,
                        types,
                        infer,
                    )?;

                    // unary operators expect no dynamic parameters
                    if !resolved.dynamic_parameters.is_empty() {
                        self.error(AnalyzeError::NoOverload {
                            node: expression_id.into_global_any(module.id),
                            receiver_ty: right_ty_id.into_global(module.id),
                        });
                    }

                    if let Some(member_symbol) = member_symbol {
                        let mut instance_arguments = inherited.arguments.clone();
                        instance_arguments.extend(resolved.static_arguments);

                        if !instance_arguments.is_empty() {
                            let instance_id = self.register_instance_for_node(
                                expression_id.into_global_any(module.id),
                                member_symbol,
                                instance_arguments,
                                types,
                            );
                            member_instance_id = Some(instance_id);
                        }
                    }

                    resolved.return_type.unwrap_or_else(|| {
                        types.insert_type_from(
                            Type::TypeLiteral {
                                value: TypeLiteral::Void,
                            },
                            expression_id,
                        )
                    })
                }
                _ => {
                    self.error(AnalyzeError::MissingType {
                        node: expression_id.into_global_any(module.id),
                    });
                    types.insert_type_from(
                        Type::TypeLiteral {
                            value: TypeLiteral::Unknown,
                        },
                        expression_id,
                    )
                }
            }
        } else {
            self.error(AnalyzeError::NoOverload {
                node: expression_id.into_global_any(module.id),
                receiver_ty: right_ty_id.into_global(module.id),
            });
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            types.insert_type_from(ty, expression_id)
        };

        // record member resolution
        self.record_member_resolution(
            expression_id.into_global_any(module.id),
            Some(right_ty_id),
            &member_resolution,
            member_instance_id,
            has_member,
            types,
        );

        Ok(resolved_return_ty_id)
    }

    /// Infer a binary operator expression.
    pub(super) fn infer_binary_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        operator: &BinaryOperator,
        left_id: LocalNodeId<Expression>,
        right_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        let left_ty_id =
            self.infer_expression(module, left_id, tree, symbols, types, infer, ctx)?;
        let right_ty_id =
            self.infer_expression(module, right_id, tree, symbols, types, infer, ctx)?;
        let left_ty = types.get_type(left_ty_id).clone();
        let right_ty = types.get_type(right_ty_id).clone();

        // guard strict equality against struct types
        if matches!(
            operator,
            BinaryOperator::EqualStrict | BinaryOperator::NotEqualStrict
        ) {
            let struct_ty_id = if self.is_definitely_struct_type(&left_ty) {
                Some(left_ty_id)
            } else if self.is_definitely_struct_type(&right_ty) {
                Some(right_ty_id)
            } else {
                None
            };

            if let Some(struct_ty_id) = struct_ty_id {
                self.error(AnalyzeError::InvalidStrictEquality {
                    node: expression_id.into_global_any(module.id),
                    ty: GlobalTypeId {
                        module_id: module.id,
                        local_id: struct_ty_id,
                    },
                });
            }
        }

        // coalesce is handled separately
        if matches!(operator, BinaryOperator::Coalesce) {
            return self.infer_coalesce_expression(
                module,
                expression_id,
                left_id,
                right_id,
                left_ty_id,
                right_ty_id,
                &left_ty,
                &right_ty,
                tree,
                symbols,
                types,
                infer,
                ctx,
            );
        }

        let operator_item = operator.language_item();

        // use builtin rules when appropriate
        if self.should_use_builtin_binary_operator(operator, &left_ty, &right_ty, types) {
            let ty = self.infer_binary_operation(operator, &left_ty, &right_ty, types);
            self.record_builtin_resolution(
                expression_id.into_global_any(module.id),
                Some(left_ty_id),
                types,
            );
            return Ok(types.insert_type_from(ty, expression_id));
        }

        let Some(operator_item) = operator_item else {
            let ty = self.infer_binary_operation(operator, &left_ty, &right_ty, types);
            return Ok(types.insert_type_from(ty, expression_id));
        };

        let operator_key = self.operator_member_key(operator_item);
        let mut member_instance_id = None;

        // inherit static arguments and substitutions from the receiver
        let inherited = self.resolve_inherited_static_arguments(
            module,
            left_id.into_any(),
            &left_ty,
            tree,
            symbols,
            types,
        )?;

        // require explicit operator interface implementation
        if !self.is_interface_implemented(&left_ty, operator_item, types) {
            self.error(AnalyzeError::NoOverload {
                node: expression_id.into_global_any(module.id),
                receiver_ty: left_ty_id.into_global(module.id),
            });
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        }

        // resolve member dispatch for the receiver type
        let member_resolution =
            self.resolve_member_resolution(module, &left_ty, &operator_key, tree, symbols, types);
        let member_symbol = match &member_resolution {
            MemberResolution::Static { symbol } => Some(*symbol),
            _ => None,
        };

        // infer the operator member type
        let mut member_type_visited = Vec::new();
        let member_ty_id = self.infer_member_of_type(
            module,
            &left_ty,
            &operator_key,
            types,
            &mut member_type_visited,
        );
        let has_member = member_ty_id.is_some();
        let resolved_return_ty_id = if let Some(member_ty_id) = member_ty_id {
            let member_ty_id = if !inherited.substitutions.is_empty() {
                let mut cache = HashMap::new();
                self.substitute_static_parameters_in_type(
                    member_ty_id,
                    &inherited.substitutions,
                    types,
                    &mut cache,
                )
            } else {
                member_ty_id
            };

            // resolve operator signature and register instance when needed
            match types.get_type(member_ty_id).clone() {
                Type::Function {
                    static_parameters,
                    dynamic_parameters,
                    return_type,
                    ..
                } => {
                    let resolved = self.resolve_function_type_for_call(
                        module,
                        expression_id.into_any(),
                        member_symbol,
                        None,
                        &static_parameters,
                        &dynamic_parameters,
                        return_type,
                        tree,
                        symbols,
                        types,
                        infer,
                    )?;

                    // binary operators expect one dynamic parameter
                    let parameter_ty_id = resolved.dynamic_parameters.first().copied();
                    if resolved.dynamic_parameters.len() != 1 {
                        self.error(AnalyzeError::NoOverload {
                            node: expression_id.into_global_any(module.id),
                            receiver_ty: left_ty_id.into_global(module.id),
                        });
                    }

                    if let Some(parameter_ty_id) = parameter_ty_id {
                        infer.push_constraint(Constraint::Subtype {
                            sub: right_ty_id,
                            sup: parameter_ty_id,
                            variance: None,
                        });

                        if !self.is_infer_var_type(parameter_ty_id, types)
                            && !self.is_infer_var_type(right_ty_id, types)
                            && self.check_is_type_assignable(parameter_ty_id, right_ty_id, types)
                                == Assignability::NotAssignable
                        {
                            return Err(AnalyzeError::UnassignableType {
                                node: expression_id.into_global_any(module.id),
                                expected_ty: GlobalTypeId {
                                    module_id: module.id,
                                    local_id: parameter_ty_id,
                                },
                                actual_ty: GlobalTypeId {
                                    module_id: module.id,
                                    local_id: right_ty_id,
                                },
                            });
                        }
                    }

                    if let Some(member_symbol) = member_symbol {
                        let mut instance_arguments = inherited.arguments.clone();
                        instance_arguments.extend(resolved.static_arguments);

                        if !instance_arguments.is_empty() {
                            let instance_id = self.register_instance_for_node(
                                expression_id.into_global_any(module.id),
                                member_symbol,
                                instance_arguments,
                                types,
                            );
                            member_instance_id = Some(instance_id);
                        }
                    }

                    resolved.return_type.unwrap_or_else(|| {
                        types.insert_type_from(
                            Type::TypeLiteral {
                                value: TypeLiteral::Void,
                            },
                            expression_id,
                        )
                    })
                }
                _ => {
                    self.error(AnalyzeError::MissingType {
                        node: expression_id.into_global_any(module.id),
                    });
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    types.insert_type_from(ty, expression_id)
                }
            }
        } else {
            self.error(AnalyzeError::NoOverload {
                node: expression_id.into_global_any(module.id),
                receiver_ty: left_ty_id.into_global(module.id),
            });
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            types.insert_type_from(ty, expression_id)
        };

        // record member resolution
        self.record_member_resolution(
            expression_id.into_global_any(module.id),
            Some(left_ty_id),
            &member_resolution,
            member_instance_id,
            has_member,
            types,
        );

        Ok(resolved_return_ty_id)
    }

    /// Infer an assignment expression.
    pub(super) fn infer_assign_expression(
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

        let left_ty_id =
            self.infer_expression(module, left_id, tree, symbols, types, infer, ctx)?;

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

        // add the subtype constraint
        infer.push_constraint(Constraint::Subtype {
            sub: right_ty_id,
            sup: left_ty_id,
            variance: None,
        });

        // check assignability when types are resolved
        if !self.is_infer_var_type(left_ty_id, types)
            && !self.is_infer_var_type(right_ty_id, types)
            && self.check_is_type_assignable(left_ty_id, right_ty_id, types)
                == Assignability::NotAssignable
        {
            return Err(AnalyzeError::UnassignableType {
                node: expression_id.into_global_any(module.id),
                expected_ty: GlobalTypeId {
                    module_id: module.id,
                    local_id: left_ty_id,
                },
                actual_ty: GlobalTypeId {
                    module_id: module.id,
                    local_id: right_ty_id,
                },
            });
        }

        let ty = Type::TypeLiteral {
            value: TypeLiteral::Void,
        };
        Ok(types.insert_type_from(ty, expression_id))
    }

    /// Infer a compound assignment expression.
    pub(super) fn infer_assign_binary_expression(
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
        let _left_ty_id =
            self.infer_expression(module, left_id, tree, symbols, types, infer, ctx)?;
        let _right_ty_id =
            self.infer_expression(module, right_id, tree, symbols, types, infer, ctx)?;

        let ty = Type::TypeLiteral {
            value: TypeLiteral::Void,
        };
        Ok(types.insert_type_from(ty, expression_id))
    }

    /// Infer a coalesce expression.
    pub(super) fn infer_coalesce_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        _right_id: LocalNodeId<Expression>,
        left_ty_id: LocalTypeId,
        right_ty_id: LocalTypeId,
        left_ty: &Type,
        _right_ty: &Type,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        _ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // use Try semantics when the receiver implements Try
        if self.is_interface_implemented(left_ty, LanguageItem::Try, types) {
            let branch = self.resolve_try_branch_member(
                module,
                expression_id,
                left_id,
                left_ty_id,
                left_ty,
                tree,
                symbols,
                types,
                infer,
            )?;

            if !branch.has_member {
                self.error(AnalyzeError::NoOverload {
                    node: expression_id.into_global_any(module.id),
                    receiver_ty: left_ty_id.into_global(module.id),
                });
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                return Ok(types.insert_type_from(ty, expression_id));
            }

            let Some(value_ty_id) = branch.value_type_id else {
                self.error(AnalyzeError::MissingType {
                    node: expression_id.into_global_any(module.id),
                });
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                return Ok(types.insert_type_from(ty, expression_id));
            };

            self.record_try_branch_resolution(module, expression_id, left_ty_id, &branch, types);

            let union_ty_id = self.union_type_ids(value_ty_id, right_ty_id, types);
            return Ok(union_ty_id);
        }

        // use nullish semantics for unions with null or undefined
        let (non_nullish_ty_id, has_nullish) = self.strip_nullish_from_union(left_ty_id, types);
        let result_ty_id = if has_nullish {
            if let Some(non_nullish_ty_id) = non_nullish_ty_id {
                self.union_type_ids(non_nullish_ty_id, right_ty_id, types)
            } else {
                right_ty_id
            }
        } else {
            left_ty_id
        };

        self.record_builtin_resolution(
            expression_id.into_global_any(module.id),
            Some(left_ty_id),
            types,
        );

        Ok(result_ty_id)
    }

    /// Infer an index access expression.
    pub(super) fn infer_index_access_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        receiver_id: LocalNodeId<Expression>,
        index_id: Option<LocalNodeId<Expression>>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        let receiver_ty_id =
            self.infer_expression(module, receiver_id, tree, symbols, types, infer, ctx)?;
        let receiver_ty = types.get_type(receiver_ty_id).clone();

        let index_ty_id = if let Some(index_id) = index_id {
            Some(self.infer_expression(module, index_id, tree, symbols, types, infer, ctx)?)
        } else {
            None
        };

        let builtin_ty_id = self.infer_builtin_index_access(&receiver_ty, index_ty_id, types);
        if let Some(builtin_ty_id) = builtin_ty_id {
            self.record_builtin_resolution(
                expression_id.into_global_any(module.id),
                Some(receiver_ty_id),
                types,
            );
            return Ok(builtin_ty_id);
        }

        if !self.is_interface_implemented(&receiver_ty, LanguageItem::Index, types) {
            self.error(AnalyzeError::NonIndexable {
                node: expression_id.into_global_any(module.id),
            });
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        }

        let mut member_instance_id = None;

        // inherit static arguments and substitutions from the receiver
        let inherited = self.resolve_inherited_static_arguments(
            module,
            receiver_id.into_any(),
            &receiver_ty,
            tree,
            symbols,
            types,
        )?;

        // resolve member dispatch for the receiver type
        let member_key = self.operator_member_key(LanguageItem::Index);
        let member_resolution =
            self.resolve_member_resolution(module, &receiver_ty, &member_key, tree, symbols, types);
        let member_symbol = match &member_resolution {
            MemberResolution::Static { symbol } => Some(*symbol),
            _ => None,
        };

        // infer the index member type
        let mut member_type_visited = Vec::new();
        let member_ty_id = self.infer_member_of_type(
            module,
            &receiver_ty,
            &member_key,
            types,
            &mut member_type_visited,
        );
        let has_member = member_ty_id.is_some();
        let value_ty_id = if let Some(member_ty_id) = member_ty_id {
            let member_ty_id = if !inherited.substitutions.is_empty() {
                let mut cache = HashMap::new();
                self.substitute_static_parameters_in_type(
                    member_ty_id,
                    &inherited.substitutions,
                    types,
                    &mut cache,
                )
            } else {
                member_ty_id
            };

            // resolve index signature and register instance when needed
            match types.get_type(member_ty_id).clone() {
                Type::Function {
                    static_parameters,
                    dynamic_parameters,
                    return_type,
                    ..
                } => {
                    let resolved = self.resolve_function_type_for_call(
                        module,
                        expression_id.into_any(),
                        member_symbol,
                        None,
                        &static_parameters,
                        &dynamic_parameters,
                        return_type,
                        tree,
                        symbols,
                        types,
                        infer,
                    )?;

                    // index access expects one dynamic parameter
                    let parameter_ty_id = resolved.dynamic_parameters.first().copied();
                    if resolved.dynamic_parameters.len() != 1 {
                        self.error(AnalyzeError::NoOverload {
                            node: expression_id.into_global_any(module.id),
                            receiver_ty: receiver_ty_id.into_global(module.id),
                        });
                    }

                    if let (Some(parameter_ty_id), Some(index_ty_id)) =
                        (parameter_ty_id, index_ty_id)
                    {
                        infer.push_constraint(Constraint::Subtype {
                            sub: index_ty_id,
                            sup: parameter_ty_id,
                            variance: None,
                        });

                        if !self.is_infer_var_type(parameter_ty_id, types)
                            && !self.is_infer_var_type(index_ty_id, types)
                            && self.check_is_type_assignable(parameter_ty_id, index_ty_id, types)
                                == Assignability::NotAssignable
                        {
                            return Err(AnalyzeError::UnassignableType {
                                node: expression_id.into_global_any(module.id),
                                expected_ty: GlobalTypeId {
                                    module_id: module.id,
                                    local_id: parameter_ty_id,
                                },
                                actual_ty: GlobalTypeId {
                                    module_id: module.id,
                                    local_id: index_ty_id,
                                },
                            });
                        }
                    }

                    if let Some(member_symbol) = member_symbol {
                        let mut instance_arguments = inherited.arguments.clone();
                        instance_arguments.extend(resolved.static_arguments);

                        if !instance_arguments.is_empty() {
                            let instance_id = self.register_instance_for_node(
                                expression_id.into_global_any(module.id),
                                member_symbol,
                                instance_arguments,
                                types,
                            );
                            member_instance_id = Some(instance_id);
                        }
                    }

                    resolved.return_type.unwrap_or_else(|| {
                        types.insert_type_from(
                            Type::TypeLiteral {
                                value: TypeLiteral::Unknown,
                            },
                            expression_id,
                        )
                    })
                }
                _ => {
                    self.error(AnalyzeError::MissingType {
                        node: expression_id.into_global_any(module.id),
                    });
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    types.insert_type_from(ty, expression_id)
                }
            }
        } else {
            self.error(AnalyzeError::NoOverload {
                node: expression_id.into_global_any(module.id),
                receiver_ty: receiver_ty_id.into_global(module.id),
            });
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            types.insert_type_from(ty, expression_id)
        };

        // record member resolution
        self.record_member_resolution(
            expression_id.into_global_any(module.id),
            Some(receiver_ty_id),
            &member_resolution,
            member_instance_id,
            has_member,
            types,
        );

        Ok(value_ty_id)
    }

    /// Infer an index assignment expression.
    pub(super) fn infer_index_assignment_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        index_expression_id: LocalNodeId<Expression>,
        value_expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        let Expression::Index {
            left: receiver_id,
            right: index_id,
        } = tree.get(index_expression_id)
        else {
            unreachable!("index assignment expects an index expression");
        };

        let receiver_ty_id =
            self.infer_expression(module, *receiver_id, tree, symbols, types, infer, ctx)?;
        let receiver_ty = types.get_type(receiver_ty_id).clone();

        let index_ty_id = if let Some(index_id) = index_id {
            Some(self.infer_expression(module, *index_id, tree, symbols, types, infer, ctx)?)
        } else {
            None
        };

        let builtin_value_ty_id = self.infer_builtin_index_access(&receiver_ty, index_ty_id, types);
        if let Some(builtin_value_ty_id) = builtin_value_ty_id {
            let mut value_ctx = ctx.fork().with_expected_type(Some(builtin_value_ty_id));
            let value_ty_id = self.infer_expression(
                module,
                value_expression_id,
                tree,
                symbols,
                types,
                infer,
                &mut value_ctx,
            )?;

            infer.push_constraint(Constraint::Subtype {
                sub: value_ty_id,
                sup: builtin_value_ty_id,
                variance: None,
            });

            if !self.is_infer_var_type(builtin_value_ty_id, types)
                && !self.is_infer_var_type(value_ty_id, types)
                && self.check_is_type_assignable(builtin_value_ty_id, value_ty_id, types)
                    == Assignability::NotAssignable
            {
                return Err(AnalyzeError::UnassignableType {
                    node: expression_id.into_global_any(module.id),
                    expected_ty: GlobalTypeId {
                        module_id: module.id,
                        local_id: builtin_value_ty_id,
                    },
                    actual_ty: GlobalTypeId {
                        module_id: module.id,
                        local_id: value_ty_id,
                    },
                });
            }

            self.record_builtin_resolution(
                expression_id.into_global_any(module.id),
                Some(receiver_ty_id),
                types,
            );

            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        }

        if !self.is_interface_implemented(&receiver_ty, LanguageItem::IndexSet, types) {
            self.error(AnalyzeError::NonIndexable {
                node: expression_id.into_global_any(module.id),
            });
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        }

        let mut member_instance_id = None;

        // inherit static arguments and substitutions from the receiver
        let inherited = self.resolve_inherited_static_arguments(
            module,
            receiver_id.into_any(),
            &receiver_ty,
            tree,
            symbols,
            types,
        )?;

        // resolve member dispatch for the receiver type
        let member_key = self.operator_member_key(LanguageItem::IndexSet);
        let member_resolution =
            self.resolve_member_resolution(module, &receiver_ty, &member_key, tree, symbols, types);
        let member_symbol = match &member_resolution {
            MemberResolution::Static { symbol } => Some(*symbol),
            _ => None,
        };

        // infer the index set member type
        let mut member_type_visited = Vec::new();
        let member_ty_id = self.infer_member_of_type(
            module,
            &receiver_ty,
            &member_key,
            types,
            &mut member_type_visited,
        );
        let has_member = member_ty_id.is_some();
        if let Some(member_ty_id) = member_ty_id {
            let member_ty_id = if !inherited.substitutions.is_empty() {
                let mut cache = HashMap::new();
                self.substitute_static_parameters_in_type(
                    member_ty_id,
                    &inherited.substitutions,
                    types,
                    &mut cache,
                )
            } else {
                member_ty_id
            };

            // resolve index set signature and register instance when needed
            match types.get_type(member_ty_id).clone() {
                Type::Function {
                    static_parameters,
                    dynamic_parameters,
                    return_type,
                    ..
                } => {
                    let resolved = self.resolve_function_type_for_call(
                        module,
                        expression_id.into_any(),
                        member_symbol,
                        None,
                        &static_parameters,
                        &dynamic_parameters,
                        return_type,
                        tree,
                        symbols,
                        types,
                        infer,
                    )?;

                    // index set expects two dynamic parameters
                    let key_param_ty_id = resolved.dynamic_parameters.first().copied();
                    let value_param_ty_id = resolved.dynamic_parameters.get(1).copied();
                    if resolved.dynamic_parameters.len() != 2 {
                        self.error(AnalyzeError::NoOverload {
                            node: expression_id.into_global_any(module.id),
                            receiver_ty: receiver_ty_id.into_global(module.id),
                        });
                    }

                    if let (Some(key_param_ty_id), Some(index_ty_id)) =
                        (key_param_ty_id, index_ty_id)
                    {
                        infer.push_constraint(Constraint::Subtype {
                            sub: index_ty_id,
                            sup: key_param_ty_id,
                            variance: None,
                        });
                    }

                    let mut value_ctx = ctx.fork().with_expected_type(value_param_ty_id);
                    let value_ty_id = self.infer_expression(
                        module,
                        value_expression_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        &mut value_ctx,
                    )?;

                    if let Some(value_param_ty_id) = value_param_ty_id {
                        infer.push_constraint(Constraint::Subtype {
                            sub: value_ty_id,
                            sup: value_param_ty_id,
                            variance: None,
                        });

                        if !self.is_infer_var_type(value_param_ty_id, types)
                            && !self.is_infer_var_type(value_ty_id, types)
                            && self.check_is_type_assignable(value_param_ty_id, value_ty_id, types)
                                == Assignability::NotAssignable
                        {
                            return Err(AnalyzeError::UnassignableType {
                                node: expression_id.into_global_any(module.id),
                                expected_ty: GlobalTypeId {
                                    module_id: module.id,
                                    local_id: value_param_ty_id,
                                },
                                actual_ty: GlobalTypeId {
                                    module_id: module.id,
                                    local_id: value_ty_id,
                                },
                            });
                        }
                    }

                    if let Some(member_symbol) = member_symbol {
                        let mut instance_arguments = inherited.arguments.clone();
                        instance_arguments.extend(resolved.static_arguments);

                        if !instance_arguments.is_empty() {
                            let instance_id = self.register_instance_for_node(
                                expression_id.into_global_any(module.id),
                                member_symbol,
                                instance_arguments,
                                types,
                            );
                            member_instance_id = Some(instance_id);
                        }
                    }
                }
                _ => {
                    self.error(AnalyzeError::MissingType {
                        node: expression_id.into_global_any(module.id),
                    });
                }
            }
        } else {
            self.error(AnalyzeError::NoOverload {
                node: expression_id.into_global_any(module.id),
                receiver_ty: receiver_ty_id.into_global(module.id),
            });
        }

        // record member resolution
        self.record_member_resolution(
            expression_id.into_global_any(module.id),
            Some(receiver_ty_id),
            &member_resolution,
            member_instance_id,
            has_member,
            types,
        );

        let ty = Type::TypeLiteral {
            value: TypeLiteral::Void,
        };
        Ok(types.insert_type_from(ty, expression_id))
    }

    /// Infer a try unwrap expression.
    pub(super) fn infer_try_unwrap_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        let left_ty_id =
            self.infer_expression(module, left_id, tree, symbols, types, infer, ctx)?;
        let left_ty = types.get_type(left_ty_id).clone();

        if !self.is_interface_implemented(&left_ty, LanguageItem::Try, types) {
            self.error(AnalyzeError::NoOverload {
                node: expression_id.into_global_any(module.id),
                receiver_ty: left_ty_id.into_global(module.id),
            });
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        }

        let branch = self.resolve_try_branch_member(
            module,
            expression_id,
            left_id,
            left_ty_id,
            &left_ty,
            tree,
            symbols,
            types,
            infer,
        )?;

        if !branch.has_member {
            self.error(AnalyzeError::NoOverload {
                node: expression_id.into_global_any(module.id),
                receiver_ty: left_ty_id.into_global(module.id),
            });
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        }

        let Some(value_ty_id) = branch.value_type_id else {
            self.error(AnalyzeError::MissingType {
                node: expression_id.into_global_any(module.id),
            });
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        };

        self.record_try_branch_resolution(module, expression_id, left_ty_id, &branch, types);

        Ok(value_ty_id)
    }

    /// Resolve the Try branch member for a receiver type.
    fn resolve_try_branch_member(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        receiver_expression_id: LocalNodeId<Expression>,
        receiver_ty_id: LocalTypeId,
        receiver_ty: &Type,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) -> AnalyzeResult<TryBranchResolution> {
        let mut member_instance_id = None;

        // inherit static arguments and substitutions from the receiver
        let inherited = self.resolve_inherited_static_arguments(
            module,
            receiver_expression_id.into_any(),
            receiver_ty,
            tree,
            symbols,
            types,
        )?;

        // resolve member dispatch for the receiver type
        let member_key = self.try_branch_member_key();
        let member_resolution =
            self.resolve_member_resolution(module, receiver_ty, &member_key, tree, symbols, types);
        let member_symbol = match &member_resolution {
            MemberResolution::Static { symbol } => Some(*symbol),
            _ => None,
        };

        // infer the branch member type
        let mut member_type_visited = Vec::new();
        let member_ty_id = self.infer_member_of_type(
            module,
            receiver_ty,
            &member_key,
            types,
            &mut member_type_visited,
        );
        let has_member = member_ty_id.is_some();
        let value_type_id = if let Some(member_ty_id) = member_ty_id {
            let member_ty_id = if !inherited.substitutions.is_empty() {
                let mut cache = HashMap::new();
                self.substitute_static_parameters_in_type(
                    member_ty_id,
                    &inherited.substitutions,
                    types,
                    &mut cache,
                )
            } else {
                member_ty_id
            };

            // resolve branch signature and register instance when needed
            match types.get_type(member_ty_id).clone() {
                Type::Function {
                    static_parameters,
                    dynamic_parameters,
                    return_type,
                    ..
                } => {
                    let resolved = self.resolve_function_type_for_call(
                        module,
                        expression_id.into_any(),
                        member_symbol,
                        None,
                        &static_parameters,
                        &dynamic_parameters,
                        return_type,
                        tree,
                        symbols,
                        types,
                        infer,
                    )?;

                    // branch expects no dynamic parameters
                    if !resolved.dynamic_parameters.is_empty() {
                        self.error(AnalyzeError::NoOverload {
                            node: expression_id.into_global_any(module.id),
                            receiver_ty: receiver_ty_id.into_global(module.id),
                        });
                    }

                    if let Some(member_symbol) = member_symbol {
                        let mut instance_arguments = inherited.arguments.clone();
                        instance_arguments.extend(resolved.static_arguments);

                        if !instance_arguments.is_empty() {
                            let instance_id = self.register_instance_for_node(
                                expression_id.into_global_any(module.id),
                                member_symbol,
                                instance_arguments,
                                types,
                            );
                            member_instance_id = Some(instance_id);
                        }
                    }

                    if let Some(return_type_id) = resolved.return_type {
                        self.try_extract_branch_value_type(return_type_id, types)
                    } else {
                        None
                    }
                }
                _ => {
                    self.error(AnalyzeError::MissingType {
                        node: expression_id.into_global_any(module.id),
                    });
                    None
                }
            }
        } else {
            None
        };

        Ok(TryBranchResolution {
            value_type_id,
            member_resolution,
            member_instance_id,
            has_member,
        })
    }

    /// Record resolution for a Try branch lookup.
    fn record_try_branch_resolution(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        receiver_ty_id: LocalTypeId,
        branch: &TryBranchResolution,
        types: &mut TypeTable,
    ) {
        self.record_member_resolution(
            expression_id.into_global_any(module.id),
            Some(receiver_ty_id),
            &branch.member_resolution,
            branch.member_instance_id,
            branch.has_member,
            types,
        );
    }

    /// Infer builtin index access for arrays and tuples.
    fn infer_builtin_index_access(
        &self,
        receiver_ty: &Type,
        index_ty_id: Option<LocalTypeId>,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        match receiver_ty {
            Type::Array { element } => Some(element.unwrap_or_else(|| {
                types.insert_type(Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                })
            })),
            Type::Tuple { elements } => {
                if elements.is_empty() {
                    return None;
                }

                if let Some(index_ty_id) = index_ty_id
                    && let Type::TypeLiteral {
                        value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(index)),
                    } = types.get_type(index_ty_id)
                    && *index >= 0
                {
                    let index = *index as usize;
                    if index < elements.len() {
                        return Some(elements[index]);
                    }
                }

                Some(self.union_type_ids_from_list(elements.clone(), types))
            }
            Type::Object { fields } => {
                let index_ty_id = index_ty_id?;

                let Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(name_id)),
                } = types.get_type(index_ty_id)
                else {
                    return None;
                };

                let key = StaticKey::Name(*name_id);
                fields
                    .iter()
                    .find(|field| field.key == key)
                    .map(|field| field.ty)
            }
            _ => None,
        }
    }

    /// Extract the ok value type from a Try branch return type.
    fn try_extract_branch_value_type(
        &self,
        return_ty_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        let return_ty = types.get_type(return_ty_id).clone();
        self.try_extract_branch_value_type_from_type(&return_ty, types)
    }

    /// Extract the ok value type from a Try branch return type.
    fn try_extract_branch_value_type_from_type(
        &self,
        ty: &Type,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        match ty {
            Type::Union { elements } => {
                let mut value_types = Vec::new();

                for element_id in elements {
                    let element_ty = types.get_type(*element_id).clone();
                    if let Some(value_ty_id) =
                        self.try_extract_branch_value_type_from_type(&element_ty, types)
                    {
                        value_types.push(value_ty_id);
                    }
                }

                if value_types.is_empty() {
                    None
                } else {
                    Some(self.union_type_ids_from_list(value_types, types))
                }
            }
            Type::Object { fields } => {
                self.try_extract_branch_value_type_from_object(fields, types)
            }
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                let result_symbol = self.expect_language_item(LanguageItem::Result);
                if *symbol != result_symbol {
                    return None;
                }
                let arguments = static_arguments.as_ref()?;
                let first_argument = arguments.first()?;
                Some(self.convert_static_argument_to_type_id(first_argument, types))
            }
            _ => None,
        }
    }

    /// Extract the ok value type from a Try branch object type.
    fn try_extract_branch_value_type_from_object(
        &self,
        fields: &[TypeField],
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        let kind_key = self.program.strings.intern("kind");
        let value_key = self.program.strings.intern("value");
        let ok_value = self.program.strings.intern("ok");

        let mut has_ok_kind = false;
        let mut value_ty_id = None;

        for field in fields {
            if let StaticKey::Name(name_id) = field.key {
                if name_id == kind_key {
                    let field_ty = types.get_type(field.ty);
                    if let Type::TypeLiteral {
                        value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(kind_id)),
                    } = field_ty
                        && *kind_id == ok_value
                    {
                        has_ok_kind = true;
                    }
                }

                if name_id == value_key {
                    value_ty_id = Some(field.ty);
                }
            }
        }

        if has_ok_kind { value_ty_id } else { None }
    }

    /// Build the member key for Try.branch.
    fn try_branch_member_key(&self) -> StaticKey {
        let name_id = self.program.strings.intern("branch");
        StaticKey::Name(name_id)
    }

    /// Build the member key for a language item operator interface.
    fn operator_member_key(&self, operator_item: LanguageItem) -> StaticKey {
        let export_name = operator_item.export_name();
        let mut chars = export_name.chars();
        let first_char = chars.next().unwrap_or_default();

        let mut member_name = String::new();
        member_name.push(first_char.to_ascii_lowercase());
        member_name.push_str(chars.as_str());

        let name_id = self.program.strings.intern(&member_name);
        StaticKey::Name(name_id)
    }

    /// Check whether to use builtin unary operator rules for a type.
    fn should_use_builtin_unary_operator(
        &self,
        operator: &UnaryOperator,
        right_ty: &Type,
        types: &TypeTable,
    ) -> bool {
        if operator.language_item().is_none() {
            return true;
        }

        if self.is_unresolved_operator_type(right_ty, types) {
            return true;
        }

        match operator {
            UnaryOperator::Negate | UnaryOperator::WrappingNegate | UnaryOperator::Plus => {
                self.is_numeric_like_type(right_ty, types)
            }
            UnaryOperator::ElementwiseNot => self.is_numeric_like_type(right_ty, types),
            UnaryOperator::Dereference => false,
            UnaryOperator::PostIncrement
            | UnaryOperator::PostDecrement
            | UnaryOperator::PreIncrement
            | UnaryOperator::PreDecrement
            | UnaryOperator::Not
            | UnaryOperator::Spread => true,
        }
    }

    /// Check whether to use builtin binary operator rules for a type pair.
    fn should_use_builtin_binary_operator(
        &self,
        operator: &BinaryOperator,
        left_ty: &Type,
        right_ty: &Type,
        types: &TypeTable,
    ) -> bool {
        if operator.language_item().is_none() {
            return true;
        }

        if self.is_unresolved_operator_type(left_ty, types)
            || self.is_unresolved_operator_type(right_ty, types)
        {
            return true;
        }

        match operator {
            BinaryOperator::Add => {
                self.is_string_like_type(left_ty, types)
                    || self.is_string_like_type(right_ty, types)
                    || (self.is_numeric_like_type(left_ty, types)
                        && self.is_numeric_like_type(right_ty, types))
            }
            BinaryOperator::Subtract
            | BinaryOperator::WrappingSubtract
            | BinaryOperator::SaturatingSubtract
            | BinaryOperator::Multiply
            | BinaryOperator::WrappingMultiply
            | BinaryOperator::SaturatingMultiply
            | BinaryOperator::Divide
            | BinaryOperator::Remainder
            | BinaryOperator::Exponent
            | BinaryOperator::WrappingExponent
            | BinaryOperator::SaturatingExponent
            | BinaryOperator::WrappingAdd
            | BinaryOperator::SaturatingAdd
            | BinaryOperator::ShiftLeft
            | BinaryOperator::SaturatingShiftLeft
            | BinaryOperator::ShiftRight
            | BinaryOperator::UnsignedShiftRight
            | BinaryOperator::ElementwiseAnd
            | BinaryOperator::ElementwiseXor
            | BinaryOperator::ElementwiseOr => {
                self.is_numeric_like_type(left_ty, types)
                    && self.is_numeric_like_type(right_ty, types)
            }
            BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::EqualStrict
            | BinaryOperator::NotEqualStrict
            | BinaryOperator::LessThan
            | BinaryOperator::LessThanOrEqual
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterThanOrEqual => {
                self.is_primitive_literal_type(left_ty, types)
                    && self.is_primitive_literal_type(right_ty, types)
            }
            BinaryOperator::And
            | BinaryOperator::Or
            | BinaryOperator::Coalesce
            | BinaryOperator::In
            | BinaryOperator::InstanceOf => true,
        }
    }
}

/// Resolved information for a Try branch lookup.
#[derive(Debug)]
struct TryBranchResolution {
    /// The extracted ok value type.
    value_type_id: Option<LocalTypeId>,
    /// The member resolution for the branch lookup.
    member_resolution: MemberResolution,
    /// The instance used by the branch member.
    member_instance_id: Option<LocalInstanceId>,
    /// Whether the branch member exists on the receiver type.
    has_member: bool,
}
