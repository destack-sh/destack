use std::collections::HashMap;

use crate::{
    AnalyzeError, AnalyzeResult, Assignability, Compiler, Constraint, InferContext, InferTable,
};
use destack_dir::{
    Argument, Block, Declaration, DynamicKey, Expression, FunctionKind, GlobalSymbolId,
    GlobalTypeId, LocalNodeId, LocalTypeId, MatchCase, MatchSelector, MatchSource, NodeTree,
    Pattern, PatternField, PrimitiveType, Property, StaticKey, SymbolTable, Type, TypeField,
    TypeLiteral, TypeTable,
};
use destack_workspace::Module;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    destack_base::ensure_sufficient_stack! {
        /// Infer the type of an expression.
        pub(super) fn infer_expression(
            &self,
            module: &Module,
            expression_id: LocalNodeId<Expression>,
            tree: &NodeTree,
            symbols: &SymbolTable,
            types: &mut TypeTable,
            infer: &mut InferTable,
            ctx: &mut InferContext,
        ) -> AnalyzeResult<LocalTypeId> {
            if let Some(ty_id) =
                types.get_inferred_type_id(expression_id.into_global_any(module.id))
            {
                return Ok(ty_id);
            }

        let expression = tree.get(expression_id);
        let ty_id: LocalTypeId = match expression {
            // declaration: analyze the declaration
            Expression::Declaration { declaration } => {
                self.infer_declaration(module, *declaration, tree, symbols, types, infer, ctx)?;
                let declaration = tree.get(*declaration);

                // lambda declarations evaluate to function values
                if let Declaration::Function {
                    descriptor,
                    signature,
                    ..
                } = declaration
                {
                    if matches!(signature.kind, FunctionKind::Lambda) {
                        if let Some(value_ty_id) =
                            types.get_value_type_id(descriptor.symbol.into_global(module.id))
                        {
                            value_ty_id
                        } else {
                            let ty = Type::TypeLiteral {
                                value: TypeLiteral::Void,
                            };
                            types.insert_type_from(ty, expression_id)
                        }
                    } else {
                        let ty = Type::TypeLiteral {
                            value: TypeLiteral::Void,
                        };
                        types.insert_type_from(ty, expression_id)
                    }
                } else {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Void,
                    };
                    types.insert_type_from(ty, expression_id)
                }
            }

            // block: analyze the block
            Expression::Block { block } => {
                self.infer_block(module, *block, tree, symbols, types, infer, ctx)?
            }

            // statement: analyze the statement
            Expression::Statement { statement } => {
                self.infer_expression(module, *statement, tree, symbols, types, infer, ctx)?;
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // labelled statement: analyze the body with label in context
            Expression::Labelled {
                label: _,
                body: body_id,
                symbol: _,
            } => {
                self.infer_expression(module, *body_id, tree, symbols, types, infer, ctx)?;
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // import / exports
            Expression::Import {
                kind: _,
                target: _,
                target_module: _,
                items,
                arguments,
            }
            | Expression::UnresolvedImport {
                kind: _,
                target: _,
                items,
                arguments,
            } => {
                for item_id in items {
                    self.infer_dependency_item(module, *item_id, tree, symbols, types, infer, ctx)?;
                }
                if let Some(arguments) = arguments {
                    for argument_id in arguments {
                        self.infer_argument(module, *argument_id, None, tree, symbols, types, infer, ctx)?;
                    }
                }
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }
            Expression::ReExport {
                target: _,
                target_module: _,
                kind: _,
                items,
            }
            | Expression::Export { kind: _, items }
            | Expression::UnresolvedReExport {
                target: _,
                kind: _,
                items,
            } => {
                for item_id in items {
                    self.infer_dependency_item(module, *item_id, tree, symbols, types, infer, ctx)?;
                }
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // let
            Expression::Let {
                descriptor: _,
                mutability: _,
                declarators,
            } => {
                for decl_id in declarators {
                    self.infer_declarator(
                        module,
                        *decl_id,
                        expression_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // type operations
            Expression::TypeUnary { operator, right } => {
                let right_ty_id =
                    self.infer_expression(module, *right, tree, symbols, types, infer, ctx)?;
                let ty = self.infer_type_unary_operation(operator, right_ty_id, types);
                types.insert_type_from(ty, expression_id)
            }
            Expression::TypeBinary {
                left,
                operator,
                right,
            } => {
                let left_ty_id = match operator {
                    destack_dir::TypeBinaryOperator::Extends
                    | destack_dir::TypeBinaryOperator::Implements => self
                        .try_evaluate_expression_to_type(module, *left, tree, symbols, types)?,
                    _ => self.infer_expression(module, *left, tree, symbols, types, infer, ctx)?,
                };
                let mut right_ty_id =
                    self.try_evaluate_expression_to_type(module, *right, tree, symbols, types)?;

                if matches!(types.get_type(right_ty_id), Type::Unevaluated { .. }) {
                    right_ty_id =
                        self.infer_expression(module, *right, tree, symbols, types, infer, ctx)?;
                }
                let ty = self.infer_type_binary_operation(
                    module,
                    expression_id,
                    operator,
                    left_ty_id,
                    right_ty_id,
                    types,
                );
                types.insert_type_from(ty, expression_id)
            }

            // unary operations: compound type
            // NOTE #Incomplete: resolve unary operator overloads
            Expression::Unary { operator, right } => {
                let right_ty_id =
                    self.infer_expression(module, *right, tree, symbols, types, infer, ctx)?;
                let ty = self.infer_unary_operation(operator, types.get_type(right_ty_id));
                types.insert_type_from(ty, expression_id)
            }

            // value of operation: value of type
            Expression::ValueOf {
                mutability,
                variance,
                right,
            } => {
                let right_ty_id =
                    self.infer_expression(module, *right, tree, symbols, types, infer, ctx)?;
                let ty = self.infer_value_of_operation(
                    *mutability,
                    *variance,
                    types.get_type(right_ty_id),
                );
                types.insert_type_from(ty, expression_id)
            }

            // reference of operation: reference of type
            Expression::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                let right_ty_id =
                    self.infer_expression(module, *right, tree, symbols, types, infer, ctx)?;
                let ty = self.infer_reference_of_operation(
                    *mutability,
                    *variance,
                    types.get_type(right_ty_id),
                );
                types.insert_type_from(ty, expression_id)
            }

            // binary operations: compound type
            // NOTE #Incomplete: resolve binary operator overloads
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                let left_ty_id = self.infer_expression(module, *left, tree, symbols, types, infer, ctx)?;
                let right_ty_id =
                    self.infer_expression(module, *right, tree, symbols, types, infer, ctx)?;

                let ty = self.infer_binary_operation(
                    operator,
                    types.get_type(left_ty_id),
                    types.get_type(right_ty_id),
                );
                types.insert_type_from(ty, expression_id)
            }

            // assignment operations: void
            Expression::Assign { left, right } => {
                let left_ty_id = self.infer_expression(module, *left, tree, symbols, types, infer, ctx)?;

                // use the left type as the expected type for the right expression
                let mut right_ctx = ctx.fork().with_expected_type(Some(left_ty_id));
                let right_ty_id = self.infer_expression(
                    module,
                    *right,
                    tree,
                    symbols,
                    types,
                    infer,
                    &mut right_ctx,
                )?;

                // type check: right must be assignable to left
                infer.push_constraint(Constraint::Subtype {
                    sub: right_ty_id,
                    sup: left_ty_id,
                    variance: None,
                });
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
                types.insert_type_from(ty, expression_id)
            }
            Expression::AssignBinary {
                left,
                operator: _,
                right,
            } => {
                let _left_ty_id =
                    self.infer_expression(module, *left, tree, symbols, types, infer, ctx)?;
                let _right_ty_id =
                    self.infer_expression(module, *right, tree, symbols, types, infer, ctx)?;
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // delete operation: void
            Expression::Delete { value } => {
                let _value_ty_id =
                    self.infer_expression(module, *value, tree, symbols, types, infer, ctx)?;
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // references: look up symbol type
            Expression::UnresolvedPath {
                path: _,
                static_arguments: _,
            } => {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // reference: symbol type
            Expression::LocalReference {
                path: _,
                target_symbol,
                static_arguments,
            }
            | Expression::ModuleReference {
                path: _,
                target_symbol,
                static_arguments,
            }
            | Expression::GlobalReference {
                path: _,
                target_symbol,
                static_arguments,
            } => {
                // resolve the canonical symbol for imported references
                let canonical_symbol =
                    self.canonical_symbol_id(module, symbols, *target_symbol);

                // pick the base type for the symbol
                let base_ty_id = if let Some(narrowed_ty_id) = ctx.get_narrowed(canonical_symbol) {
                    narrowed_ty_id
                } else if let Some(value_ty_id) = types.get_value_type_id(canonical_symbol) {
                    value_ty_id
                } else if canonical_symbol.module_id != module.id {
                    self.resolve_remote_symbol_value_type(
                        module,
                        expression_id,
                        canonical_symbol,
                        types,
                    )?
                } else {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    types.insert_type_from(ty, expression_id)
                };

                if let Some(static_argument_ids) = static_arguments.as_deref() {
                    match types.get_type(base_ty_id).clone() {
                        Type::Function {
                            asynchrony,
                            cardinality,
                            static_parameters,
                            dynamic_parameters,
                            return_type,
                        } => {
                            let resolved = self.resolve_function_static_arguments(
                                module,
                                expression_id.into_any(),
                                Some(canonical_symbol),
                                Some(static_argument_ids),
                                &static_parameters,
                                &dynamic_parameters,
                                return_type,
                                tree,
                                symbols,
                                types,
                                infer,
                            )?;

                            if let Some(resolved) = resolved {
                                let instantiated_fn = Type::Function {
                                    asynchrony,
                                    cardinality,
                                    static_parameters: Vec::new(),
                                    dynamic_parameters: resolved.dynamic_parameters,
                                    return_type: resolved.return_type,
                                };
                                let instantiated_ty_id =
                                    types.insert_type_from(instantiated_fn, expression_id);

                                if !resolved.static_arguments.is_empty() {
                                    self.register_instance_for_node(
                                        expression_id.into_global_any(module.id),
                                        canonical_symbol,
                                        resolved.static_arguments,
                                        types,
                                    );
                                }

                                instantiated_ty_id
                            } else {
                                base_ty_id
                            }
                        }
                        _ => {
                            self.error(AnalyzeError::MissingType {
                                node: expression_id.into_global_any(module.id),
                            });
                            base_ty_id
                        }
                    }
                } else {
                    base_ty_id
                }
            }

            // scalar literal: derive type from value
            Expression::ScalarLiteral { value } => {
                // apply contextual typing when a matching expected type is available
                if let Some(expected_ty_id) =
                    self.expected_type_for_scalar_literal(value, ctx.expected_type, types)
                {
                    expected_ty_id
                } else {
                    let ty = Type::TypeLiteral {
                        value: self.infer_scalar_literal(value),
                    };
                    types.insert_type_from(ty, expression_id)
                }
            }

            // type literal: use the given type literal?
            // NOTE #Suspicious: using the type literal type itself as its type is strange
            Expression::TypeLiteral { value } => {
                let ty = Type::TypeLiteral {
                    value: value.clone(),
                };
                types.insert_type_from(ty, expression_id)
            }

            // type as a value: type
            Expression::Type { value } => {
                let ty = Type::Value { value: *value };
                types.insert_type_from(ty, expression_id)
            }

            // array or tuple expression: precise tuple type for each element
            Expression::ArrayExpression { elements } | Expression::TupleExpression { elements } => {
                // infer element types using any contextual type
                let expected_element_types =
                    self.expected_element_types(ctx.expected_type, elements.len(), types);

                // infer element types
                for (index, element_id) in elements.iter().enumerate() {
                    let expected_element_ty_id =
                        expected_element_types.get(index).copied().flatten();
                    self.infer_argument(
                        module,
                        *element_id,
                        expected_element_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }

                // collect element types
                let element_tys: Vec<LocalTypeId> = elements
                    .iter()
                    .map(|element_id| {
                        let element = tree.get(*element_id);
                        let element_id = element.value();
                        self.infer_expression(module, element_id, tree, symbols, types, infer, ctx)
                    })
                    .collect::<Result<Vec<_>, AnalyzeError>>()?;
                let ty = Type::Tuple {
                    elements: element_tys,
                };
                types.insert_type_from(ty, expression_id)
            }

            // sequence expression (comma operator): type of last expression
            Expression::SequenceExpression { expressions } => {
                let mut last_ty = None;
                for (index, expr_id) in expressions.iter().enumerate() {
                    let is_last = index + 1 == expressions.len();
                    if is_last {
                        let mut expr_ctx =
                            ctx.fork().with_expected_type(ctx.expected_type);
                        last_ty = Some(self.infer_expression(
                            module,
                            *expr_id,
                            tree,
                            symbols,
                            types,
                            infer,
                            &mut expr_ctx,
                        )?);
                    } else {
                        let mut expr_ctx = ctx.fork().with_expected_type(None);
                        last_ty = Some(self.infer_expression(
                            module,
                            *expr_id,
                            tree,
                            symbols,
                            types,
                            infer,
                            &mut expr_ctx,
                        )?);
                    }
                }
                // return the type of the last expression, or void if empty (shouldn't be empty?)
                last_ty.unwrap_or_else(|| {
                    types.insert_type_from(
                        Type::TypeLiteral {
                            value: TypeLiteral::Void,
                        },
                        expression_id,
                    )
                })
            }

            // parenthesized: same type as inner
            Expression::Parenthesized {
                expression: inner_id,
            } => self.infer_expression(module, *inner_id, tree, symbols, types, infer, ctx)?,

            // object expression: object type
            Expression::ObjectExpression { properties } => {
                // apply contextual object type when available
                let expected_object_ty_id =
                    self.expected_object_type_id(ctx.expected_type, types);

                let mut fields = Vec::new();
                for property_id in properties {
                    if let Some(field) = self.infer_property(
                        module,
                        *property_id,
                        expected_object_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?
                    {
                        fields.push(field);
                    }
                }
                let ty = Type::Object { fields };
                types.insert_type_from(ty, expression_id)
            }

            // call: return type of callee
            Expression::Call {
                left: call_left_id,
                static_arguments: call_static_arguments,
                dynamic_arguments,
            } => {
                let call_left_id = *call_left_id;
                let callee_ty_id =
                    self.infer_expression(module, call_left_id, tree, symbols, types, infer, ctx)?;

                // track static arguments that come from member expressions
                let call_has_static_arguments = call_static_arguments
                    .as_ref()
                    .is_some_and(|arguments| !arguments.is_empty());
                let mut member_instance_arguments = None;
                let unwrapped_call_left_id =
                    self.unwrap_parenthesized_expression(call_left_id, tree);

                // resolve inherited static arguments and the callee symbol
                let mut inherited_static_arguments = Vec::new();
                let callee_symbol = match tree.get(unwrapped_call_left_id) {
                    Expression::Member {
                        left: receiver_id,
                        name,
                        static_arguments: member_static_arguments,
                        ..
                    } => {
                        let receiver_ty_id =
                            if let Some(receiver_ty_id) = types.get_inferred_type_id(
                                receiver_id.into_global_any(module.id),
                            ) {
                                receiver_ty_id
                            } else {
                                self.infer_expression(
                                    module,
                                    *receiver_id,
                                    tree,
                                    symbols,
                                    types,
                                    infer,
                                    ctx,
                                )?
                            };
                        let receiver_ty = types.get_type(receiver_ty_id).clone();

                        if let Type::Reference {
                            symbol,
                            static_arguments,
                        } = &receiver_ty
                            && let Some(resolved) = self
                                .resolve_type_reference_static_arguments(
                                    module,
                                    receiver_id.into_any(),
                                    *symbol,
                                    static_arguments.as_deref(),
                                    tree,
                                    symbols,
                                    types,
                                )?
                        {
                            inherited_static_arguments = resolved;
                        }

                        // resolve the member symbol on the receiver type
                        let member_key = StaticKey::Name(*name);
                        let mut member_symbol_visited = Vec::new();
                        let member_symbol = self.resolve_member_symbol_for_type(
                            module,
                            &receiver_ty,
                            &member_key,
                            tree,
                            symbols,
                            types,
                            &mut member_symbol_visited,
                        );

                        // prefer member instance arguments when static arguments live on the member expression
                        let member_has_static_arguments = member_static_arguments
                            .as_ref()
                            .is_some_and(|arguments| !arguments.is_empty());

                        // report duplicate static arguments on the member and call
                        if call_has_static_arguments && member_has_static_arguments {
                            self.error(AnalyzeError::ConflictingStaticArguments {
                                node: expression_id.into_global_any(module.id),
                            });
                        }

                        if !call_has_static_arguments && member_has_static_arguments {
                            member_instance_arguments = self.member_instance_arguments_for_call(
                                module,
                                unwrapped_call_left_id,
                                member_symbol,
                                types,
                            );
                        }

                        member_symbol
                    }
                    _ => {
                        self.reference_symbol_for_expression(
                            module,
                            unwrapped_call_left_id,
                            tree,
                            symbols,
                        )
                    }
                };

                // resolve the callee signature and static arguments
                match types.get_type(callee_ty_id).clone() {
                    Type::Function {
                        asynchrony: _,
                        cardinality: _,
                        static_parameters,
                        dynamic_parameters,
                        return_type,
                    } => {
                        let resolved = self.resolve_function_static_arguments(
                            module,
                            expression_id.into_any(),
                            callee_symbol,
                            call_static_arguments.as_deref(),
                            &static_parameters,
                            &dynamic_parameters,
                            return_type,
                            tree,
                            symbols,
                            types,
                            infer,
                        )?;

                        let (resolved_dynamic_parameters, resolved_return_type, resolved_static_arguments) =
                            if let Some(resolved) = resolved {
                                (
                                    resolved.dynamic_parameters,
                                    resolved.return_type,
                                    resolved.static_arguments,
                                )
                            } else {
                                (dynamic_parameters, return_type, Vec::new())
                            };

                        // analyze arguments with contextual parameter types
                        let mut argument_ty_ids = Vec::with_capacity(dynamic_arguments.len());
                        for (index, argument_id) in dynamic_arguments.iter().enumerate() {
                            let expected_arg_ty_id =
                                resolved_dynamic_parameters.get(index).copied();
                            self.infer_argument(
                                module,
                                *argument_id,
                                expected_arg_ty_id,
                                tree,
                                symbols,
                                types,
                                infer,
                                ctx,
                            )?;

                            let argument = tree.get(*argument_id);
                            let argument_value_id = argument.value();
                            let argument_ty_id = self.infer_expression(
                                module,
                                argument_value_id,
                                tree,
                                symbols,
                                types,
                                infer,
                                ctx,
                            )?;
                            argument_ty_ids.push(argument_ty_id);
                        }

                        // add constraints between arguments and parameters
                        for (argument_ty_id, param_ty_id) in
                            argument_ty_ids.iter().zip(resolved_dynamic_parameters.iter())
                        {
                            infer.push_constraint(Constraint::Subtype {
                                sub: *argument_ty_id,
                                sup: *param_ty_id,
                                variance: None,
                            });
                        }

                        // check argument assignability against parameters
                        for (index, (argument_ty_id, param_ty_id)) in argument_ty_ids
                            .iter()
                            .zip(resolved_dynamic_parameters.iter())
                            .enumerate()
                        {
                            if !self.is_infer_var_type(*param_ty_id, types)
                                && !self.is_infer_var_type(*argument_ty_id, types)
                                && self.check_is_type_assignable(
                                    *param_ty_id,
                                    *argument_ty_id,
                                    types,
                                ) == Assignability::NotAssignable
                            {
                                let argument_node = dynamic_arguments
                                    .get(index)
                                    .map(|id| id.into_global_any(module.id))
                                    .unwrap_or_else(|| expression_id.into_global_any(module.id));
                                return Err(AnalyzeError::UnassignableType {
                                    node: argument_node,
                                    expected_ty: GlobalTypeId {
                                        module_id: module.id,
                                        local_id: *param_ty_id,
                                    },
                                    actual_ty: GlobalTypeId {
                                        module_id: module.id,
                                        local_id: *argument_ty_id,
                                    },
                                });
                            }
                        }

                        // register the instance if the call is to a symbol
                        if let Some(callee_symbol) = callee_symbol {
                            let instance_arguments = member_instance_arguments
                                .unwrap_or_else(|| {
                                    let mut arguments = inherited_static_arguments;
                                    arguments.extend(resolved_static_arguments);
                                    arguments
                                });

                            if !instance_arguments.is_empty() {
                                self.register_instance_for_node(
                                    expression_id.into_global_any(module.id),
                                    callee_symbol,
                                    instance_arguments,
                                    types,
                                );
                            }
                        }

                        resolved_return_type.unwrap_or_else(|| {
                            let ty = Type::TypeLiteral {
                                value: TypeLiteral::Void,
                            };
                            types.insert_type_from(ty, expression_id)
                        })
                    }
                    _ => {
                        for argument_id in dynamic_arguments {
                            self.infer_argument(
                                module,
                                *argument_id,
                                None,
                                tree,
                                symbols,
                                types,
                                infer,
                                ctx,
                            )?;
                        }

                        let ty = Type::TypeLiteral {
                            value: TypeLiteral::Unknown,
                        };
                        types.insert_type_from(ty, expression_id)
                    }
                }
            }

            // member access: member type
            Expression::Member {
                left,
                name,
                static_arguments,
            } => {
                let left_ty_id =
                    self.infer_expression(module, *left, tree, symbols, types, infer, ctx)?;
                let left_ty = types.get_type(left_ty_id).clone();
                let (inherited_static_arguments, inherited_substitutions) =
                    if let Type::Reference {
                        symbol,
                        static_arguments,
                    } = &left_ty
                    {
                        if let Some(resolved) = self.resolve_type_reference_static_arguments(
                            module,
                            left.into_any(),
                            *symbol,
                            static_arguments.as_deref(),
                            tree,
                            symbols,
                            types,
                        )? {
                            let substitutions = self.build_type_parameter_substitutions_for_symbol(
                                module,
                                *symbol,
                                &resolved,
                                tree,
                                symbols,
                                types,
                            );
                            (resolved, substitutions)
                        } else {
                            (Vec::new(), HashMap::new())
                        }
                    } else {
                        (Vec::new(), HashMap::new())
                    };

                // look up member type on the left type
                let member_key = StaticKey::Name(*name);

                let mut member_symbol_visited = Vec::new();
                let member_symbol = self.resolve_member_symbol_for_type(
                    module,
                    &left_ty,
                    &member_key,
                    tree,
                    symbols,
                    types,
                    &mut member_symbol_visited,
                );

                let mut member_type_visited = Vec::new();
                if let Some(member_ty_id) = self.infer_member_of_type(
                    module,
                    &left_ty,
                    &member_key,
                    types,
                    &mut member_type_visited,
                )
                {
                    let member_ty_id = if !inherited_substitutions.is_empty() {
                        let mut cache = HashMap::new();
                        self.substitute_static_parameters_in_type(
                            member_ty_id,
                            &inherited_substitutions,
                            types,
                            &mut cache,
                        )
                    } else {
                        member_ty_id
                    };

                    if let Some(static_argument_ids) = static_arguments.as_deref() {
                        match types.get_type(member_ty_id).clone() {
                            Type::Function {
                                asynchrony,
                                cardinality,
                                static_parameters,
                                dynamic_parameters,
                                return_type,
                            } => {
                                let resolved = self.resolve_function_static_arguments(
                                    module,
                                    expression_id.into_any(),
                                    member_symbol,
                                    Some(static_argument_ids),
                                    &static_parameters,
                                    &dynamic_parameters,
                                    return_type,
                                    tree,
                                    symbols,
                                    types,
                                    infer,
                                )?;

                                if let Some(resolved) = resolved {
                                    let resolved_static_arguments = resolved.static_arguments;
                                    let (resolved_dynamic_parameters, resolved_return_type) =
                                        if inherited_substitutions.is_empty() {
                                            (
                                                resolved.dynamic_parameters,
                                                resolved.return_type,
                                            )
                                        } else {
                                            let mut cache = HashMap::new();
                                            let dynamic_parameters = resolved
                                                .dynamic_parameters
                                                .iter()
                                                .map(|parameter| {
                                                    self.substitute_static_parameters_in_type(
                                                        *parameter,
                                                        &inherited_substitutions,
                                                        types,
                                                        &mut cache,
                                                    )
                                                })
                                                .collect::<Vec<_>>();
                                            let return_type = resolved.return_type.map(|return_type| {
                                                self.substitute_static_parameters_in_type(
                                                    return_type,
                                                    &inherited_substitutions,
                                                    types,
                                                    &mut cache,
                                                )
                                            });
                                            (dynamic_parameters, return_type)
                                        };

                                    let instantiated_fn = Type::Function {
                                        asynchrony,
                                        cardinality,
                                        static_parameters: Vec::new(),
                                        dynamic_parameters: resolved_dynamic_parameters,
                                        return_type: resolved_return_type,
                                    };
                                    if let Some(member_symbol) = member_symbol {
                                        let mut instance_arguments =
                                            inherited_static_arguments.clone();
                                        instance_arguments.extend(resolved_static_arguments);
                                        if !instance_arguments.is_empty() {
                                            self.register_instance_for_node(
                                                expression_id.into_global_any(module.id),
                                                member_symbol,
                                                instance_arguments,
                                                types,
                                            );
                                        }
                                    }

                                    types.insert_type_from(instantiated_fn, expression_id)
                                } else {
                                    member_ty_id
                                }
                            }
                            _ => {
                                self.error(AnalyzeError::MissingType {
                                    node: expression_id.into_global_any(module.id),
                                });
                                member_ty_id
                            }
                        }
                    } else {
                        member_ty_id
                    }
                } else {
                    // member not found, report error and continue with unknown type
                    self.error(AnalyzeError::MissingMember {
                        node: expression_id.into_global_any(module.id),
                        receiver_ty: left_ty_id.into_global(module.id),
                        member_key,
                    });
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    types.insert_type_from(ty, expression_id)
                }
            }

            // index: element type
            Expression::Index { left, right } => {
                let _left_ty_id =
                    self.infer_expression(module, *left, tree, symbols, types, infer, ctx)?;
                if let Some(right) = right {
                    self.infer_expression(module, *right, tree, symbols, types, infer, ctx)?;
                }
                // NOTE #Incomplete: resolve element type from array/tuple type
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // new: instance type
            Expression::New {
                left,
                static_arguments: _,
                dynamic_arguments,
            } => {
                let _left_ty_id =
                    self.infer_expression(module, *left, tree, symbols, types, infer, ctx)?;
                for arg in dynamic_arguments {
                    self.infer_argument(module, *arg, None, tree, symbols, types, infer, ctx)?;
                }
                // NOTE #Incomplete: resolve instance type from constructor
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // if expression: union of branches or common type
            Expression::If {
                kind: _,
                condition,
                then_expression,
                else_expression,
            } => {
                self.infer_expression(module, *condition, tree, symbols, types, infer, ctx)?;

                // then
                let mut then_ctx = ctx.fork().with_expected_type(ctx.expected_type);
                let then_ty_id = self.infer_expression(
                    module,
                    *then_expression,
                    tree,
                    symbols,
                    types,
                    infer,
                    &mut then_ctx,
                )?;

                // else
                if let Some(else_expr) = else_expression {
                    let mut else_ctx = ctx.fork().with_expected_type(ctx.expected_type);
                    let _else_ty_id = self.infer_expression(
                        module,
                        *else_expr,
                        tree,
                        symbols,
                        types,
                        infer,
                        &mut else_ctx,
                    )?;
                }

                // NOTE #Incomplete: compute union or common type of then/else branches
                then_ty_id
            }

            // loop: loop body type or never
            Expression::Loop {
                kind: _,
                condition,
                body,
                scope: _,
                symbol: _,
            } => {
                if let Some(cond) = condition {
                    self.infer_expression(module, *cond, tree, symbols, types, infer, ctx)?;
                }
                let mut ctx = ctx.fork().in_loop(expression_id.into_any());
                self.infer_block(module, *body, tree, symbols, types, infer, &mut ctx)?;
                // NOTE #Incomplete: loop return type depends on break value
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // for each: void
            Expression::ForEach {
                asynchrony: _,
                kind: _,
                pattern,
                iterator,
                body,
                scope: _,
                symbol: _,
            } => {
                let iterator_ty_id =
                    self.infer_expression(module, *iterator, tree, symbols, types, infer, ctx)?;
                self.infer_pattern(
                    module,
                    *pattern,
                    Some(iterator_ty_id),
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
                let mut ctx = ctx.fork().in_loop(expression_id.into_any());
                self.infer_block(module, *body, tree, symbols, types, infer, &mut ctx)?;
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // for: void
            Expression::For {
                initialization,
                condition,
                increment,
                body,
                scope: _,
                symbol: _,
            } => {
                if let Some(initialization) = initialization {
                    self.infer_expression(module, *initialization, tree, symbols, types, infer, ctx)?;
                }
                if let Some(condition) = condition {
                    self.infer_expression(module, *condition, tree, symbols, types, infer, ctx)?;
                }
                if let Some(increment) = increment {
                    self.infer_expression(module, *increment, tree, symbols, types, infer, ctx)?;
                }
                let mut ctx = ctx.fork().in_loop(expression_id.into_any());
                self.infer_block(module, *body, tree, symbols, types, infer, &mut ctx)?;
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // match: union of case result types
            Expression::Match {
                value,
                cases,
                source,
                scope: _,
                symbol: _,
            } => {
                let value_ty_id =
                    self.infer_expression(module, *value, tree, symbols, types, infer, ctx)?;
                let mut ctx = if *source == MatchSource::Match {
                    ctx.fork().in_match(expression_id.into_any())
                } else {
                    ctx.fork()
                };
                let mut result_ty_id = None;
                for case_id in cases {
                    let case = tree.get(*case_id);
                    let (selector, body_expr, block_body) = match case {
                        MatchCase::Expression {
                            selector,
                            body,
                            scope: _,
                        } => (selector, Some(*body), None),
                        MatchCase::Block {
                            selector,
                            body,
                            scope: _,
                        } => (selector, None, Some(*body)),
                    };
                    // infer pattern and guard from selector
                    if let MatchSelector::Pattern { pattern, guard } = selector {
                        self.infer_pattern(
                            module,
                            *pattern,
                            Some(value_ty_id),
                            tree,
                            symbols,
                            types,
                            infer,
                            &mut ctx,
                        )?;
                        if let Some(guard_expr) = guard {
                            self.infer_expression(
                                module,
                                *guard_expr,
                                tree,
                                symbols,
                                types,
                                infer,
                                &mut ctx,
                            )?;
                        }
                    }

                    // apply contextual typing to the case body
                    let mut case_ctx = ctx.fork().with_expected_type(ctx.expected_type);

                    // default selector has no pattern or guard to infer
                    if let Some(expr) = body_expr {
                        let case_ty_id = self.infer_expression(
                            module,
                            expr,
                            tree,
                            symbols,
                            types,
                            infer,
                            &mut case_ctx,
                        )?;
                        if result_ty_id.is_none() {
                            result_ty_id = Some(case_ty_id);
                        }
                    }

                    // body
                    if let Some(body) = block_body {
                        self.infer_block(
                            module,
                            body,
                            tree,
                            symbols,
                            types,
                            infer,
                            &mut case_ctx,
                        )?;
                    }

                    // NOTE #Incomplete: compute union of case types
                }
                result_ty_id.unwrap_or_else(|| {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Never,
                    };
                    types.insert_type_from(ty, expression_id)
                })
            }

            // try: result type
            Expression::Try {
                try_expression,
                catch_pattern,
                catch_expression,
                finally_expression,
                scope: _,
                symbol: _,
            } => {
                let try_ty_id =
                    self.infer_expression(module, *try_expression, tree, symbols, types, infer, ctx)?;
                if let Some(catch_pat) = catch_pattern {
                    self.infer_pattern(module, *catch_pat, None, tree, symbols, types, infer, ctx)?;
                }
                if let Some(catch_expr) = catch_expression {
                    self.infer_expression(module, *catch_expr, tree, symbols, types, infer, ctx)?;
                }
                if let Some(finally_expr) = finally_expression {
                    self.infer_expression(module, *finally_expr, tree, symbols, types, infer, ctx)?;
                }
                try_ty_id
            }

            // return: never (control flow)
            Expression::Return { value } => {
                // validate return position
                if !ctx.can_return() {
                    self.error(AnalyzeError::InvalidReturn {
                        node: expression_id.into_global_any(module.id),
                    });
                }

                // constrain return value to the function return type
                if let Some(val) = value {
                    let mut return_ctx = ctx.fork().with_expected_type(ctx.return_type);
                    let value_ty_id = self.infer_expression(
                        module,
                        *val,
                        tree,
                        symbols,
                        types,
                        infer,
                        &mut return_ctx,
                    )?;
                    if let Some(return_ty_id) = ctx.return_type {
                        infer.push_constraint(Constraint::Subtype {
                            sub: value_ty_id,
                            sup: return_ty_id,
                            variance: None,
                        });
                    }
                } else if let Some(return_ty_id) = ctx.return_type {
                    let void_ty_id = types.insert_type(Type::TypeLiteral {
                        value: TypeLiteral::Void,
                    });
                    infer.push_constraint(Constraint::Subtype {
                        sub: void_ty_id,
                        sup: return_ty_id,
                        variance: None,
                    });
                }

                // return expressions always end control flow
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Never,
                };
                types.insert_type_from(ty, expression_id)
            }

            // break: never
            Expression::Break { target, target_symbol: _, value } => {
                if !ctx.can_break() {
                    self.error(AnalyzeError::InvalidBreak {
                        node: expression_id.into_global_any(module.id),
                        label: *target,
                    });
                }
                if let Some(val) = value {
                    self.infer_expression(module, *val, tree, symbols, types, infer, ctx)?;
                }
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Never,
                };
                types.insert_type_from(ty, expression_id)
            }
            Expression::UnresolvedBreak { target, value } => {
                if !ctx.can_break() {
                    self.error(AnalyzeError::InvalidBreak {
                        node: expression_id.into_global_any(module.id),
                        label: Some(*target),
                    });
                }
                if let Some(val) = value {
                    self.infer_expression(module, *val, tree, symbols, types, infer, ctx)?;
                }
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Never,
                };
                types.insert_type_from(ty, expression_id)
            }

            // continue: never
            Expression::Continue { target, target_symbol: _ } => {
                if !ctx.can_continue() {
                    self.error(AnalyzeError::InvalidContinue {
                        node: expression_id.into_global_any(module.id),
                        label: *target,
                    });
                }
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Never,
                };
                types.insert_type_from(ty, expression_id)
            }
            Expression::UnresolvedContinue { target } => {
                if !ctx.can_continue() {
                    self.error(AnalyzeError::InvalidContinue {
                        node: expression_id.into_global_any(module.id),
                        label: Some(*target),
                    });
                }
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Never,
                };
                types.insert_type_from(ty, expression_id)
            }

            // throw: never (control flow)
            Expression::Throw { value } => {
                self.infer_expression(module, *value, tree, symbols, types, infer, ctx)?;
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Never,
                };
                types.insert_type_from(ty, expression_id)
            }

            // await: unwrapped promise type
            Expression::Await { expression } => {
                if !ctx.can_await() {
                    self.error(AnalyzeError::InvalidAwait {
                        node: expression_id.into_global_any(module.id),
                    });
                }
                let _inner_ty_id =
                    self.infer_expression(module, *expression, tree, symbols, types, infer, ctx)?;
                // NOTE #Incomplete: unwrap Promise type
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // await? should be desugared to Maybe { Await } before analysis
            Expression::AwaitMaybe { .. } => {
                unreachable!("AwaitMaybe should be desugared before analysis")
            }

            // comptime: type of body (evaluated at compile time)
            Expression::Comptime { body } => {
                // NOTE #Incomplete: validate that body can be evaluated at comptime
                self.infer_expression(module, *body, tree, symbols, types, infer, ctx)?
            }

            // yield: yielded type
            Expression::Yield {
                cardinality: _,
                value,
            } => {
                if !ctx.can_yield() {
                    self.error(AnalyzeError::InvalidYield {
                        node: expression_id.into_global_any(module.id),
                    });
                }
                if let Some(value_id) = value {
                    self.infer_expression(module, *value_id, tree, symbols, types, infer, ctx)?;
                }
                // NOTE #Incomplete: yield type depends on generator context
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // maybe or must unwrap: inner type or error
            Expression::Maybe { left } | Expression::Must { left } => {
                let _inner_ty_id =
                    self.infer_expression(module, *left, tree, symbols, types, infer, ctx)?;
                // NOTE #Incomplete: unwrap optional type
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // template expressions: string
            Expression::TemplateExpression { value: _ } => {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::String),
                };
                types.insert_type_from(ty, expression_id)
            }
            Expression::TaggedTemplateExpression { tag, value: _ } => {
                let _tag_ty_id = self.infer_expression(module, *tag, tree, symbols, types, infer, ctx)?;
                // NOTE #Incomplete: tagged template return type from tag function
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // range expression: analyze bounds, type is Iterable<T>
            // NOTE #Incomplete: range should satisfy Iterable<T> where T is the element type
            Expression::RangeExpression {
                start,
                end,
                is_inclusive: _,
            } => {
                self.infer_expression(module, *start, tree, symbols, types, infer, ctx)?;
                self.infer_expression(module, *end, tree, symbols, types, infer, ctx)?;
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // tagged expressions for newtype construction
            Expression::TaggedScalarExpression { ty, value } => {
                let ty_id = self.infer_expression(module, *ty, tree, symbols, types, infer, ctx)?;
                self.infer_expression(module, *value, tree, symbols, types, infer, ctx)?;
                ty_id
            }
            Expression::TaggedTupleExpression { ty, elements } => {
                let ty_id = self.infer_expression(module, *ty, tree, symbols, types, infer, ctx)?;
                let expected_element_types =
                    self.expected_element_types(Some(ty_id), elements.len(), types);

                for (index, elem) in elements.iter().enumerate() {
                    let expected_element_ty_id =
                        expected_element_types.get(index).copied().flatten();
                    self.infer_argument(
                        module,
                        *elem,
                        expected_element_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
                ty_id
            }
            Expression::TaggedObjectExpression { ty, properties } => {
                let ty_id = self.infer_expression(module, *ty, tree, symbols, types, infer, ctx)?;
                let expected_object_ty_id = self.expected_object_type_id(Some(ty_id), types);

                for prop_id in properties {
                    self.infer_property(
                        module,
                        *prop_id,
                        expected_object_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
                ty_id
            }

            // tree expression (JSX like)
            Expression::TreeExpression {
                left,
                arguments,
                elements,
            } => {
                if let Some(left) = left {
                    self.infer_expression(module, *left, tree, symbols, types, infer, ctx)?;
                }
                if let Some(args) = arguments {
                    for arg in args {
                        self.infer_argument(module, *arg, None, tree, symbols, types, infer, ctx)?;
                    }
                }
                if let Some(elems) = elements {
                    for elem in elems {
                        self.infer_argument(module, *elem, None, tree, symbols, types, infer, ctx)?;
                    }
                }
                // NOTE #Incomplete: JSX element type (see Elaborate/reify)
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // error expression: error type
            Expression::Error => {
                let ty = Type::Error;
                types.insert_type_from(ty, expression_id)
            }

            // debugger: void
            Expression::Debugger => {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // stub: nothing to do
            Expression::Stub => {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }
        };

        types.set_inferred_type(expression_id.into_global_any(module.id), ty_id);

        Ok(ty_id)
        }
    }

    /// Infer a block.
    pub(super) fn infer_block(
        &self,
        module: &Module,
        block_id: LocalNodeId<Block>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        if let Some(ty_id) = types.get_inferred_type_id(block_id.into_global_any(module.id)) {
            return Ok(ty_id);
        }

        let block = tree.get(block_id);
        // infer all but the last expression without contextual typing
        let last_index = block.expressions.len().saturating_sub(1);
        for (index, expression_id) in block.expressions.iter().enumerate() {
            if index == last_index {
                continue;
            }
            let mut expr_ctx = ctx.fork().with_expected_type(None);
            self.infer_expression(
                module,
                *expression_id,
                tree,
                symbols,
                types,
                infer,
                &mut expr_ctx,
            )?;
        }

        // infer the last expression with contextual typing
        let ty_id = if let Some(last_expression_id) = block.expressions.last() {
            let mut last_ctx = ctx.fork().with_expected_type(ctx.expected_type);
            self.infer_expression(
                module,
                *last_expression_id,
                tree,
                symbols,
                types,
                infer,
                &mut last_ctx,
            )?
        } else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            types.insert_type_from(ty, block_id)
        };

        types.set_inferred_type(block_id.into_global_any(module.id), ty_id);

        Ok(ty_id)
    }

    /// Infer a declaration.
    /// Infer a property and return its TypeField if it has a static key.
    pub(super) fn infer_property(
        &self,
        module: &Module,
        property_id: LocalNodeId<Property>,
        expected_object_ty_id: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<Option<TypeField>> {
        let property = tree.get(property_id);
        match property {
            Property::Field {
                modifiers,
                key,
                value,
                default,
                symbol: _,
            } => {
                // extract the static key from the dynamic key
                let static_key = key.and_then(|k| match k {
                    DynamicKey::Name(name) => Some(StaticKey::Name(name)),
                    // dynamic keys can't be used for static type inference
                    DynamicKey::Expression(_) | DynamicKey::NamedExpression { .. } => None,
                });

                // derive an expected field type from the contextual object type
                let expected_field_ty_id = static_key
                    .as_ref()
                    .and_then(|key| self.expected_field_type_id(expected_object_ty_id, key, types));

                // infer the value type
                let value_ty_id = if let Some(value) = value {
                    let mut value_ctx = ctx.fork().with_expected_type(expected_field_ty_id);
                    self.infer_expression(
                        module,
                        *value,
                        tree,
                        symbols,
                        types,
                        infer,
                        &mut value_ctx,
                    )?
                } else {
                    // no value, return unknown type
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    types.insert_type(ty)
                };

                // infer default with the same expected type
                if let Some(default) = default {
                    let mut default_ctx = ctx.fork().with_expected_type(expected_field_ty_id);
                    self.infer_expression(
                        module,
                        *default,
                        tree,
                        symbols,
                        types,
                        infer,
                        &mut default_ctx,
                    )?;
                }

                // is optional
                let is_optional = modifiers
                    .as_ref()
                    .is_some_and(|m| matches!(m.kind, Some(destack_dir::BindingKind::Maybe)));

                // is readonly
                let is_readonly = modifiers.as_ref().is_some_and(|m| {
                    matches!(m.mutability, Some(destack_dir::Mutability::Immutable))
                });

                // static key
                if let Some(key) = static_key {
                    Ok(Some(TypeField {
                        key,
                        ty: value_ty_id,
                        is_optional,
                        is_readonly,
                    }))
                } else {
                    Ok(None)
                }
            }
            Property::Method {
                key,
                signature,
                body,
                symbol,
                ..
            } => {
                // NOTE #Incomplete: infer method type
                let expected_method_ty_id = key
                    .and_then(|key| match key {
                        DynamicKey::Name(name) => Some(StaticKey::Name(name)),
                        DynamicKey::Expression(_) | DynamicKey::NamedExpression { .. } => None,
                    })
                    .and_then(|key| {
                        self.expected_field_type_id(expected_object_ty_id, &key, types)
                    });

                // infer the method signature with contextual typing
                let method_ty_id = self.infer_signature(
                    module,
                    property_id.into_any(),
                    symbol.into_global(module.id),
                    signature,
                    expected_method_ty_id,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;

                // body
                if let Some(body) = body {
                    let return_type = self.function_return_type(method_ty_id, types);
                    let ctx = ctx
                        .reset()
                        .in_function_with_signature(property_id.into_any(), signature);
                    let mut ctx = ctx
                        .with_return_type(return_type)
                        .with_expected_type(return_type);

                    // infer the method body with implicit return typing
                    let body_ty_id = self
                        .infer_expression(module, *body, tree, symbols, types, infer, &mut ctx)?;

                    // constrain implicit return types against the declared return type
                    if let Some(return_ty_id) = return_type
                        && self.has_implicit_return(*body, tree)
                    {
                        infer.push_constraint(Constraint::Subtype {
                            sub: body_ty_id,
                            sup: return_ty_id,
                            variance: None,
                        });

                        if !self.is_infer_var_type(return_ty_id, types)
                            && !self.is_infer_var_type(body_ty_id, types)
                            && self.check_is_type_assignable(return_ty_id, body_ty_id, types)
                                == Assignability::NotAssignable
                        {
                            self.error(AnalyzeError::UnassignableType {
                                node: body.into_global_any(module.id),
                                expected_ty: GlobalTypeId {
                                    module_id: module.id,
                                    local_id: return_ty_id,
                                },
                                actual_ty: GlobalTypeId {
                                    module_id: module.id,
                                    local_id: body_ty_id,
                                },
                            });
                        }
                    }
                }
                Ok(None)
            }
            Property::Spread { value, .. } => {
                // NOTE #Incomplete: expand spread type into object type
                self.infer_expression(module, *value, tree, symbols, types, infer, ctx)?;
                Ok(None)
            }
        }
    }

    /// Infer a member and return its TypeField if it has a static key.
    pub(super) fn infer_argument(
        &self,
        module: &Module,
        argument_id: LocalNodeId<Argument>,
        expected_ty_id: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let argument = tree.get(argument_id);
        // apply the expected type to the argument value
        let mut argument_ctx = ctx.fork().with_expected_type(expected_ty_id);

        match argument {
            Argument::Positional { value } => {
                self.infer_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    types,
                    infer,
                    &mut argument_ctx,
                )?;
            }
            Argument::Named { name: _, value } => {
                self.infer_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    types,
                    infer,
                    &mut argument_ctx,
                )?;
            }
            Argument::Labeled { label: _, value } => {
                self.infer_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    types,
                    infer,
                    &mut argument_ctx,
                )?;
            }
            Argument::Spread { value } => {
                self.infer_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    types,
                    infer,
                    &mut argument_ctx,
                )?;
            }
        }
        Ok(())
    }

    /// Infer a dependency item.
    pub(super) fn infer_pattern(
        &self,
        module: &Module,
        pattern_id: LocalNodeId<Pattern>,
        binding_ty_id: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let pattern = tree.get(pattern_id);
        match pattern {
            Pattern::Wildcard => {
                // nothing to do
            }
            Pattern::Maybe(pattern_id) => {
                self.infer_pattern(
                    module,
                    *pattern_id,
                    binding_ty_id,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
            }
            Pattern::ReferenceOf {
                mutability: _,
                right,
            } => {
                self.infer_pattern(
                    module,
                    *right,
                    binding_ty_id,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
            }
            Pattern::ValueOf {
                mutability: _,
                right,
            } => {
                self.infer_pattern(
                    module,
                    *right,
                    binding_ty_id,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
            }
            Pattern::Binding {
                mutability: _,
                name: _,
                symbol,
                pattern,
            } => {
                if let Some(ty_id) = binding_ty_id {
                    types.set_value_type(symbol.into_global(module.id), ty_id);
                }
                if let Some(pattern_id) = pattern {
                    self.infer_pattern(
                        module,
                        *pattern_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
            }
            Pattern::Expression { value } => {
                self.infer_expression(module, *value, tree, symbols, types, infer, ctx)?;
            }
            Pattern::Range {
                start,
                end,
                is_inclusive: _,
            } => {
                if let Some(start_pattern_id) = start {
                    self.infer_pattern(
                        module,
                        *start_pattern_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
                if let Some(end_pattern_id) = end {
                    self.infer_pattern(
                        module,
                        *end_pattern_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
            }
            Pattern::Tuple { fields } => {
                self.infer_pattern_sequence(
                    module,
                    fields,
                    binding_ty_id,
                    |rest_types| Type::Tuple {
                        elements: rest_types,
                    },
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
            }
            Pattern::TaggedTuple { ty, fields } => {
                let ty_id = self.infer_expression(module, *ty, tree, symbols, types, infer, ctx)?;
                self.infer_pattern_sequence(
                    module,
                    fields,
                    Some(ty_id),
                    |rest_types| Type::Tuple {
                        elements: rest_types,
                    },
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
            }
            Pattern::Array { fields } => {
                self.infer_pattern_sequence(
                    module,
                    fields,
                    binding_ty_id,
                    |rest_types| Type::Array {
                        element: rest_types.first().cloned(),
                    },
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
            }
            Pattern::Object { fields } => {
                for field_id in fields {
                    self.infer_pattern_field(
                        module,
                        *field_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
            }
            Pattern::TaggedObject { ty, fields } => {
                let ty_id = self.infer_expression(module, *ty, tree, symbols, types, infer, ctx)?;
                for field_id in fields {
                    self.infer_pattern_field(
                        module,
                        *field_id,
                        Some(ty_id),
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
            }
            Pattern::Union { patterns } => {
                for pattern_id in patterns {
                    self.infer_pattern(
                        module,
                        *pattern_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
            }
        }

        Ok(())
    }

    /// Infer a sequence of pattern fields (with spread syntax support).
    pub(super) fn infer_pattern_sequence(
        &self,
        module: &Module,
        fields: &Vec<LocalNodeId<PatternField>>,
        binding_ty_id: Option<LocalTypeId>,
        to_rest_type: impl Fn(Vec<LocalTypeId>) -> Type,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let binding_ty_fields = binding_ty_id
            .and_then(|ty_id| match types.get_type(ty_id) {
                Type::Tuple { elements } => Some(elements.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let spread_len = binding_ty_fields.len().saturating_sub(fields.len() - 1);
        let mut ty_idx = 0;
        for field_id in fields {
            let field = tree.get(*field_id);
            let field_ty = match field {
                PatternField::Named { .. }
                | PatternField::Alias { .. }
                | PatternField::Positional { .. } => {
                    let ty = binding_ty_fields.get(ty_idx).cloned();
                    ty_idx += 1;
                    ty
                }
                PatternField::Spread { .. } => {
                    let rest_types = binding_ty_fields
                        .get(ty_idx..ty_idx + spread_len)
                        .map(|s| s.to_vec())
                        .unwrap_or_default();
                    ty_idx += spread_len;
                    let rest_ty = to_rest_type(rest_types);
                    Some(types.insert_type_from(rest_ty, *field_id))
                }
                PatternField::Elision => {
                    // elision skips a type position
                    ty_idx += 1;
                    None
                }
            };
            self.infer_pattern_field(
                module, *field_id, field_ty, tree, symbols, types, infer, ctx,
            )?;
        }
        Ok(())
    }

    /// Infer a pattern field and propagate type to bound symbol.
    pub(super) fn infer_pattern_field(
        &self,
        module: &Module,
        field_id: LocalNodeId<PatternField>,
        binding_ty_id: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let field = tree.get(field_id);
        match field {
            PatternField::Named {
                mutability: _,
                name: _,
                default: _,
                symbol,
                pattern,
            } => {
                if let Some(ty_id) = binding_ty_id {
                    types.set_value_type(symbol.into_global(module.id), ty_id);
                }
                if let Some(pattern_id) = pattern {
                    self.infer_pattern(
                        module,
                        *pattern_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
            }
            PatternField::Alias {
                mutability: _,
                name: _,
                alias: _,
                default,
                symbol,
            } => {
                if let Some(ty_id) = binding_ty_id {
                    types.set_value_type(symbol.into_global(module.id), ty_id);
                }
                if let Some(default) = default {
                    self.infer_expression(module, *default, tree, symbols, types, infer, ctx)?;
                }
            }
            PatternField::Positional { pattern } => {
                self.infer_pattern(
                    module,
                    *pattern,
                    binding_ty_id,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
            }
            PatternField::Spread {
                mutability: _,
                name: _,
                symbol,
            } => {
                if let Some(ty_id) = binding_ty_id {
                    types.set_value_type(symbol.into_global(module.id), ty_id);
                }
            }
            PatternField::Elision => {
                // elision doesn't bind anything
            }
        }
        Ok(())
    }

    /// Extract the return type from a function type.
    pub(super) fn function_return_type(
        &self,
        fn_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        match types.get_type(fn_ty_id) {
            Type::Function { return_type, .. } => *return_type,
            _ => None,
        }
    }

    /// Resolve the canonical symbol for a reference.
    pub(super) fn canonical_symbol_id(
        &self,
        module: &Module,
        symbols: &SymbolTable,
        symbol: GlobalSymbolId,
    ) -> GlobalSymbolId {
        if symbol.module_id == module.id {
            let symbol_entry = symbols.get_symbol(symbol.local_id);
            symbol_entry.canonical_symbol.unwrap_or(symbol)
        } else {
            symbol
        }
    }

    /// Get the target symbol for a reference expression.
    pub(super) fn reference_symbol_for_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        match tree.get(expression_id) {
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => {
                Some(self.canonical_symbol_id(module, symbols, *target_symbol))
            }
            _ => None,
        }
    }

    /// Resolve the member symbol for a type and member key.
    pub(super) fn resolve_member_symbol_for_type(
        &self,
        module: &Module,
        receiver_ty: &Type,
        member_key: &StaticKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        visited: &mut Vec<GlobalSymbolId>,
    ) -> Option<GlobalSymbolId> {
        match receiver_ty {
            Type::Reference { symbol, .. } => self.resolve_member_symbol_for_symbol(
                module, *symbol, member_key, tree, symbols, types, visited,
            ),
            _ => None,
        }
    }

    /// Resolve the member symbol for a nominal type symbol.
    pub(super) fn resolve_member_symbol_for_symbol(
        &self,
        module: &Module,
        symbol: GlobalSymbolId,
        member_key: &StaticKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        visited: &mut Vec<GlobalSymbolId>,
    ) -> Option<GlobalSymbolId> {
        if visited.contains(&symbol) {
            return None;
        }
        visited.push(symbol);

        if let Some(member_symbol) =
            self.find_member_symbol_in_declaration(module, symbol, member_key, tree, symbols)
        {
            return Some(member_symbol);
        }

        if let Some(lineage) = types.get_lineage_for_symbol(symbol).cloned() {
            if let Some(extends) = lineage.extends
                && let Some(member_symbol) = self.resolve_member_symbol_for_symbol(
                    module, extends, member_key, tree, symbols, types, visited,
                )
            {
                return Some(member_symbol);
            }

            for implements in &lineage.implements {
                if let Some(member_symbol) = self.resolve_member_symbol_for_symbol(
                    module,
                    *implements,
                    member_key,
                    tree,
                    symbols,
                    types,
                    visited,
                ) {
                    return Some(member_symbol);
                }
            }

            for embedded in &lineage.embedded {
                if let Some(member_symbol) = self.resolve_member_symbol_for_symbol(
                    module, *embedded, member_key, tree, symbols, types, visited,
                ) {
                    return Some(member_symbol);
                }
            }
        }

        let extension_ids = types.get_extensions_for_target(symbol)?.clone();
        for extension_id in extension_ids {
            let extension = types.get_extension(extension_id);
            if !self.is_extension_visible(module, extension) {
                continue;
            }

            if let Some(member_symbol) = self.find_member_symbol_in_declaration(
                module,
                extension.symbol,
                member_key,
                tree,
                symbols,
            ) {
                return Some(member_symbol);
            }
        }

        None
    }

    /// Find a member symbol inside a declaration for a key.
    pub(super) fn find_member_symbol_in_declaration(
        &self,
        module: &Module,
        symbol: GlobalSymbolId,
        member_key: &StaticKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        if symbol.module_id != module.id {
            return None;
        }

        let symbol_entry = symbols.get_symbol(symbol.local_id);
        let primary_declaration = symbol_entry.primary_declaration?;
        let declaration_id = primary_declaration
            .try_into_local_typed::<Declaration>()
            .ok()?;
        let declaration = tree.get(declaration_id);

        let members = match declaration {
            Declaration::Struct { members, .. }
            | Declaration::Class { members, .. }
            | Declaration::Enum { members, .. }
            | Declaration::Interface { members, .. }
            | Declaration::Extension { members, .. } => members,
            _ => return None,
        };

        for member_id in members {
            let member = tree.get(*member_id);
            let static_key = member.key().and_then(|key| match key {
                DynamicKey::Name(name) => Some(StaticKey::Name(*name)),
                DynamicKey::Expression(_) | DynamicKey::NamedExpression { .. } => None,
            });

            if let Some(static_key) = static_key
                && &static_key == member_key
            {
                return Some(member.symbol().into_global(module.id));
            }
        }

        None
    }

    /// Peel nested parenthesized expressions to the underlying expression.
    pub(super) fn unwrap_parenthesized_expression(
        &self,
        mut expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
    ) -> LocalNodeId<Expression> {
        loop {
            let Expression::Parenthesized { expression } = tree.get(expression_id) else {
                break;
            };

            expression_id = *expression;
        }

        expression_id
    }

    /// Check whether an expression participates in implicit return typing.
    pub(super) fn has_implicit_return(
        &self,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
    ) -> bool {
        match tree.get(expression_id) {
            Expression::Statement { .. }
            | Expression::Return { .. }
            | Expression::Break { .. }
            | Expression::Continue { .. } => false,
            Expression::Block { block } => {
                let block = tree.get(*block);

                let Some(last_expression_id) = block.expressions.last() else {
                    return false;
                };

                !matches!(
                    tree.get(*last_expression_id),
                    Expression::Statement { .. }
                        | Expression::Return { .. }
                        | Expression::Break { .. }
                        | Expression::Continue { .. }
                )
            }
            _ => true,
        }
    }
}
