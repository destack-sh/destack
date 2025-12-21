use crate::{
    AnalyzeError, AnalyzeResult, Assignability, Compiler, Constraint, InferContext, InferOrigin,
    InferScope, InferTable,
};
use destack_dir::{
    Argument, Block, Declaration, DeclarationAbstraction, Declarator, DependencyItem, DynamicKey,
    EnumField, Expression, Extension, ExtensionKind, FunctionKind, FunctionSignature, Generics,
    GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, Heritage, Lineage, LocalNodeId, LocalNodeIdAny,
    LocalSymbolId, LocalTypeId, MatchCase, MatchSelector, MatchSource, Member, NodeTree, Parameter,
    Pattern, PatternField, PrimitiveType, Property, ScalarLiteral, StaticKey, SymbolTable, Type,
    TypeField, TypeKind, TypeLiteral, TypeTable, WhereClause,
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
            if let Some(ty_id) = types.get_inferred_type_id(expression_id.into_global_any(module.id)) {
            return Ok(ty_id);
        }

        let expression = tree.get(expression_id);
        let ty_id: LocalTypeId = match expression {
            // declaration -> analyze the declaration
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

            // block -> analyze the block
            Expression::Block { block } => {
                self.infer_block(module, *block, tree, symbols, types, infer, ctx)?
            }

            // statement -> analyze the statement
            Expression::Statement { statement } => {
                self.infer_expression(module, *statement, tree, symbols, types, infer, ctx)?;
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // labelled statement -> analyze the body with label in context
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
                let left_ty_id = self.infer_expression(module, *left, tree, symbols, types, infer, ctx)?;
                let right_ty_id =
                    self.infer_expression(module, *right, tree, symbols, types, infer, ctx)?;
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

            // unary operations -> compound type
            // #Incomplete: resolve unary operator overloads
            Expression::Unary { operator, right } => {
                let right_ty_id =
                    self.infer_expression(module, *right, tree, symbols, types, infer, ctx)?;
                let ty = self.infer_unary_operation(operator, types.get_type(right_ty_id));
                types.insert_type_from(ty, expression_id)
            }

            // value of operation -> value of type
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

            // reference of operation -> reference of type
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

            // binary operations -> compound type
            // #Incomplete: resolve binary operator overloads
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

            // assignment operations -> void
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

            // delete operation -> void
            Expression::Delete { value } => {
                let _value_ty_id =
                    self.infer_expression(module, *value, tree, symbols, types, infer, ctx)?;
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // references -> look up symbol type
            Expression::UnresolvedPath {
                path: _,
                static_arguments: _,
            } => {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // #Incomplete: instantiate references with static arguments
            // reference -> symbol type
            Expression::LocalReference {
                path: _,
                target_symbol,
                static_arguments: _,
            }
            | Expression::ModuleReference {
                path: _,
                target_symbol,
                static_arguments: _,
            }
            | Expression::GlobalReference {
                path: _,
                target_symbol,
                static_arguments: _,
            } => {
                // follow symbol chain to get canonical symbol (for imports/re-exports)
                let canonical_symbol: GlobalSymbolId = {
                    // if local symbol, check if it has a canonical_symbol (for imports)
                    if target_symbol.module_id == module.id {
                        let symbol = symbols.get_symbol(target_symbol.local_id);
                        if let Some(canonical_symbol) = symbol.canonical_symbol {
                            canonical_symbol
                        } else {
                            *target_symbol
                        }
                    } else {
                        // otherwise return as-is
                        *target_symbol
                    }
                };

                // narrow symbol type in context
                if let Some(narrowed_ty_id) = ctx.get_narrowed(canonical_symbol) {
                    narrowed_ty_id
                }
                // symbol value type (local)
                else if let Some(value_ty_id) = types.get_value_type_id(canonical_symbol) {
                    value_ty_id
                }
                // symbol value type (remote)
                else if canonical_symbol.module_id != module.id {
                    self.resolve_remote_symbol_value_type(
                        module,
                        expression_id,
                        canonical_symbol,
                        types,
                    )?
                }
                // unknown type
                else {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    types.insert_type_from(ty, expression_id)
                }
            }

            // scalar literal -> derive type from value
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

            // type literal -> use the given type literal?
            // NOTE #Suspicious: using the type literal type itself as its type is strange
            Expression::TypeLiteral { value } => {
                let ty = Type::TypeLiteral {
                    value: value.clone(),
                };
                types.insert_type_from(ty, expression_id)
            }

            // type as a value -> type
            Expression::Type { value } => {
                let ty = Type::Value { value: *value };
                types.insert_type_from(ty, expression_id)
            }

            // array / tuple expression -> precise tuple type for each element
            Expression::ArrayExpression { elements } | Expression::TupleExpression { elements } => {
                // infer element types using any contextual type
                let expected_element_types =
                    self.expected_element_types(ctx.expected_type, elements.len(), types);

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

            // sequence expression (comma operator) -> type of last expression
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

            // parenthesized -> same type as inner
            Expression::Parenthesized {
                expression: inner_id,
            } => self.infer_expression(module, *inner_id, tree, symbols, types, infer, ctx)?,

            // object expression -> object type
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

            // call -> return type of callee
            // #Incomplete: instantiate functions with static arguments
            Expression::Call {
                left,
                static_arguments: _,
                dynamic_arguments,
            } => {
                let callee_ty_id =
                    self.infer_expression(module, *left, tree, symbols, types, infer, ctx)?;

                // extract callee signature for contextual typing
                let callee_signature = match types.get_type(callee_ty_id) {
                    Type::Function {
                        dynamic_parameters,
                        return_type,
                        ..
                    } => Some((dynamic_parameters.clone(), *return_type)),
                    _ => None,
                };

                // analyze arguments with contextual parameter types
                let mut argument_ty_ids = Vec::with_capacity(dynamic_arguments.len());
                for (index, arg) in dynamic_arguments.iter().enumerate() {
                    let expected_arg_ty_id = callee_signature
                        .as_ref()
                        .and_then(|(params, _)| params.get(index).copied());
                    self.infer_argument(
                        module,
                        *arg,
                        expected_arg_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;

                    let arg_expr = tree.get(*arg);
                    let arg_value_id = arg_expr.value();
                    let argument_ty_id =
                        self.infer_expression(module, arg_value_id, tree, symbols, types, infer, ctx)?;
                    argument_ty_ids.push(argument_ty_id);
                }

                // check callee type and get return type
                match callee_signature {
                    Some((dynamic_parameters, return_type)) => {
                        // add argument constraints against parameters
                        for (argument_ty_id, param_ty_id) in
                            argument_ty_ids.iter().zip(dynamic_parameters.iter())
                        {
                            infer.push_constraint(Constraint::Subtype {
                                sub: *argument_ty_id,
                                sup: *param_ty_id,
                                variance: None,
                            });
                        }

                        // type check arguments against parameters
                        for (i, (argument_ty_id, param_ty_id)) in
                            argument_ty_ids.iter().zip(dynamic_parameters.iter()).enumerate()
                        {
                            if !self.is_infer_var_type(*param_ty_id, types)
                                && !self.is_infer_var_type(*argument_ty_id, types)
                                && self.check_is_type_assignable(*param_ty_id, *argument_ty_id, types)
                                    == Assignability::NotAssignable
                            {
                                // get the argument node for error reporting
                                let argument_node = dynamic_arguments
                                    .get(i)
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
                        return_type.unwrap_or_else(|| {
                            let ty = Type::TypeLiteral {
                                value: TypeLiteral::Void,
                            };
                            types.insert_type_from(ty, expression_id)
                        })
                    }
                    // #Incomplete: handle other callable types (objects with call signature)
                    _ => {
                        let ty = Type::TypeLiteral {
                            value: TypeLiteral::Unknown,
                        };
                        types.insert_type_from(ty, expression_id)
                    }
                }
            }

            // member access -> member type
            Expression::Member {
                left,
                name,
                static_arguments: _,
            } => {
                let left_ty_id = self.infer_expression(module, *left, tree, symbols, types, infer, ctx)?;
                let left_ty = types.get_type(left_ty_id).clone();

                // look up member type on the left type (including extensions)
                let mut visited = Vec::new();
                let member_key = StaticKey::Name(*name);
                if let Some(member_ty_id) =
                    self.infer_member_of_type(module, &left_ty, &member_key, types, &mut visited)
                {
                    member_ty_id
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

            // index -> element type
            Expression::Index { left, right } => {
                let _left_ty_id =
                    self.infer_expression(module, *left, tree, symbols, types, infer, ctx)?;
                if let Some(right) = right {
                    self.infer_expression(module, *right, tree, symbols, types, infer, ctx)?;
                }
                // #Incomplete: resolve element type from array/tuple type
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // new -> instance type
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
                // #Incomplete: resolve instance type from constructor
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // if expression -> union of branches or common type
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

                // #Incomplete: compute union or common type of then/else branches
                then_ty_id
            }

            // loop -> loop body type or never
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
                // #Incomplete: loop return type depends on break value
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // for-each -> void
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

            // for -> void
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

            // match -> union of case result types
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

                    // #Incomplete: compute union of case types
                }
                result_ty_id.unwrap_or_else(|| {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Never,
                    };
                    types.insert_type_from(ty, expression_id)
                })
            }

            // try -> result type
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

            // return -> never (control flow)
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

            // break -> never
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

            // continue -> never
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

            // throw -> never (control flow)
            Expression::Throw { value } => {
                self.infer_expression(module, *value, tree, symbols, types, infer, ctx)?;
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Never,
                };
                types.insert_type_from(ty, expression_id)
            }

            // await -> unwrapped promise type
            Expression::Await { expression } => {
                if !ctx.can_await() {
                    self.error(AnalyzeError::InvalidAwait {
                        node: expression_id.into_global_any(module.id),
                    });
                }
                let _inner_ty_id =
                    self.infer_expression(module, *expression, tree, symbols, types, infer, ctx)?;
                // #Incomplete: unwrap Promise type
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // await? should be desugared to Maybe { Await } before analysis
            Expression::AwaitMaybe { .. } => {
                unreachable!("AwaitMaybe should be desugared before analysis")
            }

            // comptime -> type of body (evaluated at compile time)
            Expression::Comptime { body } => {
                // #Incomplete: validate that body can be evaluated at comptime
                self.infer_expression(module, *body, tree, symbols, types, infer, ctx)?
            }

            // yield -> yielded type
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
                // #Incomplete: yield type depends on generator context
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // maybe/must unwrap -> inner type or error
            Expression::Maybe { left } | Expression::Must { left } => {
                let _inner_ty_id =
                    self.infer_expression(module, *left, tree, symbols, types, infer, ctx)?;
                // #Incomplete: unwrap optional type
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // template expressions -> string
            Expression::TemplateExpression { value: _ } => {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::String),
                };
                types.insert_type_from(ty, expression_id)
            }
            Expression::TaggedTemplateExpression { tag, value: _ } => {
                let _tag_ty_id = self.infer_expression(module, *tag, tree, symbols, types, infer, ctx)?;
                // #Incomplete: tagged template return type from tag function
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // range expression: analyze bounds, type is Iterable<T>
            // #Incomplete: range should satisfy Iterable<T> where T is the element type
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

            // tree expression (JSX-like)
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
                // #Incomplete: JSX element type
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // error expression -> error type
            Expression::Error => {
                let ty = Type::Error;
                types.insert_type_from(ty, expression_id)
            }

            // debugger -> void
            Expression::Debugger => {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // stub -> nothing to do
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
    fn infer_block(
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
    fn infer_declaration(
        &self,
        module: &Module,
        declaration_id: LocalNodeId<Declaration>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let declaration = tree.get(declaration_id);

        match declaration {
            // namespace
            Declaration::Namespace {
                descriptor: _,
                generics,
                scope: _,
                expressions,
            } => {
                // walk
                self.infer_generics(module, generics, tree, symbols, types, infer, ctx)?;
                for expression_id in expressions {
                    self.infer_expression(
                        module,
                        *expression_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
            }

            // type alias
            Declaration::Type {
                descriptor,
                kind,
                mutability: _,
                static_parameters,
                value,
            } => {
                // walk
                if let Some(parameters) = static_parameters {
                    for parameter_id in parameters {
                        self.infer_parameter(
                            module,
                            *parameter_id,
                            None,
                            tree,
                            symbols,
                            types,
                            infer,
                            ctx,
                        )?;
                    }
                }

                // type instance type -> type value
                let instance_ty_id =
                    self.infer_expression(module, *value, tree, symbols, types, infer, ctx)?;
                match *kind {
                    TypeKind::Structural => {
                        types.set_instance_type(
                            descriptor.symbol.into_global(module.id),
                            instance_ty_id,
                        );
                    }
                    TypeKind::Nominal => {
                        let ty = Type::Reference {
                            symbol: descriptor.symbol.into_global(module.id),
                            static_arguments: None,
                        };
                        let ty_id = types.insert_type_from(ty, declaration_id);
                        types.set_instance_type(descriptor.symbol.into_global(module.id), ty_id);
                    }
                }

                // type value type -> type metatype
                let value_ty = Type::Value {
                    value: instance_ty_id,
                };
                let value_ty_id = types.insert_type_from(value_ty, declaration_id);
                types.set_value_type(descriptor.symbol.into_global(module.id), value_ty_id);
            }

            // struct
            Declaration::Struct {
                descriptor,
                generics,
                heritage,
                scope: _,
                members,
            } => {
                // walk
                self.infer_generics(module, generics, tree, symbols, types, infer, ctx)?;
                self.infer_heritage(
                    module,
                    heritage,
                    declaration_id.into_any(),
                    Some(descriptor.symbol),
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;

                // type fields
                let mut fields = Vec::new();
                for member_id in members {
                    if let Some(field) =
                        self.infer_member(module, *member_id, tree, symbols, types, infer, ctx)?
                    {
                        fields.push(field);
                    }
                }

                // struct instance type -> object type with fields
                let instance_ty = Type::Object { fields };
                let instance_ty_id = types.insert_type_from(instance_ty, declaration_id);
                types.set_instance_type(descriptor.symbol.into_global(module.id), instance_ty_id);

                // struct nominal type -> reference type
                let nominal_ty = Type::Reference {
                    symbol: descriptor.symbol.into_global(module.id),
                    static_arguments: None,
                };
                let nominal_ty_id = types.insert_type_from(nominal_ty, declaration_id);

                // struct value type -> nominal type
                let value_ty = Type::Value {
                    value: nominal_ty_id,
                };
                let value_ty_id = types.insert_type_from(value_ty, declaration_id);
                types.set_value_type(descriptor.symbol.into_global(module.id), value_ty_id);
            }

            // class
            Declaration::Class {
                descriptor,
                generics,
                heritage,
                scope: _,
                members,
            } => {
                // walk
                self.infer_generics(module, generics, tree, symbols, types, infer, ctx)?;
                self.infer_heritage(
                    module,
                    heritage,
                    declaration_id.into_any(),
                    Some(descriptor.symbol),
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;

                // context
                let is_abstract = descriptor.abstraction == DeclarationAbstraction::Abstract;
                let mut ctx = ctx.fork().in_abstract_class_maybe(is_abstract);

                // type fields
                let mut fields = Vec::new();
                for member_id in members {
                    if let Some(field) = self
                        .infer_member(module, *member_id, tree, symbols, types, infer, &mut ctx)?
                    {
                        fields.push(field);
                    }
                }

                // class instance type -> object type with fields
                let instance_ty = Type::Object { fields };
                let instance_ty_id = types.insert_type_from(instance_ty, declaration_id);
                types.set_instance_type(descriptor.symbol.into_global(module.id), instance_ty_id);

                // class nominal type -> reference type
                let nominal_ty = Type::Reference {
                    symbol: descriptor.symbol.into_global(module.id),
                    static_arguments: None,
                };
                let nominal_ty_id = types.insert_type_from(nominal_ty, declaration_id);

                // class value type -> nominal type
                let value_ty = Type::Value {
                    value: nominal_ty_id,
                };
                let value_ty_id = types.insert_type_from(value_ty, declaration_id);
                types.set_value_type(descriptor.symbol.into_global(module.id), value_ty_id);
            }

            // enum
            Declaration::Enum {
                descriptor,
                kind: _,
                generics,
                heritage,
                scope: _,
                fields,
                members,
            } => {
                // walk
                self.infer_generics(module, generics, tree, symbols, types, infer, ctx)?;
                self.infer_heritage(
                    module,
                    heritage,
                    declaration_id.into_any(),
                    Some(descriptor.symbol),
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
                for field_id in fields {
                    self.infer_enum_field(module, *field_id, tree, symbols, types, infer, ctx)?;
                }
                for member_id in members {
                    self.infer_member(module, *member_id, tree, symbols, types, infer, ctx)?;
                }

                // enum instance type -> enum value type
                let instance_ty = Type::Reference {
                    symbol: descriptor.symbol.into_global(module.id),
                    static_arguments: None,
                };
                let instance_ty_id = types.insert_type_from(instance_ty, declaration_id);
                types.set_instance_type(descriptor.symbol.into_global(module.id), instance_ty_id);

                // enum instance type -> enum value type
                let value_ty = Type::Value {
                    value: instance_ty_id,
                };
                let value_ty_id = types.insert_type_from(value_ty, declaration_id);
                types.set_value_type(descriptor.symbol.into_global(module.id), value_ty_id);
            }

            // extension
            Declaration::Extension {
                descriptor,
                generics,
                target_type,
                target_symbol,
                heritage,
                scope: _,
                members,
            } => {
                // walk
                self.infer_generics(module, generics, tree, symbols, types, infer, ctx)?;
                self.infer_expression(module, *target_type, tree, symbols, types, infer, ctx)?;
                self.infer_heritage(
                    module,
                    heritage,
                    declaration_id.into_any(),
                    Some(descriptor.symbol), // (put lineage on extension symbol itself)
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;

                // collect type fields from members
                let mut fields = Vec::new();
                for member_id in members {
                    if let Some(field) =
                        self.infer_member(module, *member_id, tree, symbols, types, infer, ctx)?
                    {
                        fields.push(field);
                    }
                }

                // extension instance type -> object type with its methods
                let instance_ty = Type::Object { fields };
                let instance_ty_id = types.insert_type_from(instance_ty, declaration_id);
                let extension_symbol = descriptor.symbol.into_global(module.id);
                types.set_instance_type(extension_symbol, instance_ty_id);

                // register extension
                if let Some(target) = target_symbol {
                    let kind = if module.id == target.module_id {
                        ExtensionKind::Inherent
                    } else if descriptor.name.is_some() {
                        ExtensionKind::Nominal
                    } else {
                        ExtensionKind::Local
                    };
                    let lineage = types.get_lineage_id_for_symbol(extension_symbol);
                    let extension = Extension::new(extension_symbol, kind, *target, lineage);
                    types.insert_extension(extension);
                }
            }

            // interface
            Declaration::Interface {
                descriptor,
                kind: _,
                generics,
                heritage,
                scope: _,
                members,
            } => {
                // walk
                self.infer_generics(module, generics, tree, symbols, types, infer, ctx)?;
                self.infer_heritage(
                    module,
                    heritage,
                    declaration_id.into_any(),
                    Some(descriptor.symbol),
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;

                // collect type fields from members
                let mut fields = Vec::new();
                for member_id in members {
                    if let Some(field) =
                        self.infer_member(module, *member_id, tree, symbols, types, infer, ctx)?
                    {
                        fields.push(field);
                    }
                }

                // interface instance type -> object type with fields
                let instance_ty = Type::Object { fields };
                let instance_ty_id = types.insert_type_from(instance_ty, declaration_id);
                types.set_instance_type(descriptor.symbol.into_global(module.id), instance_ty_id);

                // interface nominal type -> reference type
                let nominal_ty = Type::Reference {
                    symbol: descriptor.symbol.into_global(module.id),
                    static_arguments: None,
                };
                let nominal_ty_id = types.insert_type_from(nominal_ty, declaration_id);

                // interface value type -> nominal type
                let value_ty = Type::Value {
                    value: nominal_ty_id,
                };
                let value_ty_id = types.insert_type_from(value_ty, declaration_id);
                types.set_value_type(descriptor.symbol.into_global(module.id), value_ty_id);
            }

            // function
            Declaration::Function {
                descriptor,
                signature,
                scope: _,
                body,
            } => {
                let fn_ty_id = self.infer_signature(
                    module,
                    declaration_id.into_any(),
                    descriptor.symbol.into_global(module.id),
                    signature,
                    ctx.expected_type,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
                // register the function type as the value_type for the function's symbol #Suspicious
                types.set_value_type(descriptor.symbol.into_global(module.id), fn_ty_id);

                if let Some(body) = body {
                    let return_type = self.function_return_type(fn_ty_id, types);
                    let ctx = ctx
                        .reset()
                        .in_function_with_signature(declaration_id.into_any(), signature);
                    let mut ctx = ctx.with_return_type(return_type);
                    self.infer_expression(module, *body, tree, symbols, types, infer, &mut ctx)?;
                }
            }
        }

        Ok(())
    }

    /// Infer a property and return its TypeField if it has a static key.
    fn infer_property(
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
                // #Incomplete: infer method type
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
                    let mut ctx = ctx.with_return_type(return_type);
                    self.infer_expression(module, *body, tree, symbols, types, infer, &mut ctx)?;
                }
                Ok(None)
            }
            Property::Spread { value, .. } => {
                // #Incomplete: expand spread type into object type
                self.infer_expression(module, *value, tree, symbols, types, infer, ctx)?;
                Ok(None)
            }
        }
    }

    /// Infer a member and return its TypeField if it has a static key.
    fn infer_member(
        &self,
        module: &Module,
        member_id: LocalNodeId<Member>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<Option<TypeField>> {
        let member = tree.get(member_id);
        match member {
            Member::Field {
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

                // infer the value type
                let value_ty_id = if let Some(value) = value {
                    self.infer_expression(module, *value, tree, symbols, types, infer, ctx)?
                } else {
                    // no value, return unknown type
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    types.insert_type(ty)
                };

                // analyze default if present
                if let Some(default) = default {
                    self.infer_expression(module, *default, tree, symbols, types, infer, ctx)?;
                }

                // check if the field is optional
                let is_optional = modifiers
                    .as_ref()
                    .is_some_and(|m| matches!(m.kind, Some(destack_dir::BindingKind::Maybe)));

                // check if the field is readonly
                let is_readonly = modifiers.as_ref().is_some_and(|m| {
                    matches!(m.mutability, Some(destack_dir::Mutability::Immutable))
                });

                // only return a field if we have a static key
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
            Member::Method {
                key,
                signature,
                body,
                modifiers: _,
                ..
            } => {
                // extract the static key from the dynamic key
                let static_key = key.and_then(|k| match k {
                    DynamicKey::Name(name) => Some(StaticKey::Name(name)),
                    DynamicKey::Expression(_) | DynamicKey::NamedExpression { .. } => None,
                });

                // signature
                let method_ty_id = self.infer_signature(
                    module,
                    member_id.into_any(),
                    member.symbol().into_global(module.id),
                    signature,
                    None,
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
                        .in_function_with_signature(member_id.into_any(), signature);
                    let mut ctx = ctx.with_return_type(return_type);
                    self.infer_expression(module, *body, tree, symbols, types, infer, &mut ctx)?;
                }

                // return a field if we have a static key
                if let Some(key) = static_key {
                    Ok(Some(TypeField {
                        key,
                        ty: method_ty_id,
                        is_optional: false,
                        is_readonly: true,
                    }))
                } else {
                    Ok(None)
                }
            }
            Member::Embed { value, .. } => {
                // #Incomplete: expand embedded type into member fields?
                self.infer_expression(module, *value, tree, symbols, types, infer, ctx)?;
                Ok(None)
            }
            Member::StaticBlock { body, .. } => {
                self.infer_expression(module, *body, tree, symbols, types, infer, ctx)?;
                Ok(None)
            }
        }
    }

    /// Infer generics.
    fn infer_generics(
        &self,
        module: &Module,
        generics: &Generics,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        if let Some(static_parameters) = &generics.static_parameters {
            for parameter_id in static_parameters {
                self.infer_parameter(
                    module,
                    *parameter_id,
                    None,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
            }
        }
        if let Some(clauses) = &generics.where_clauses {
            for clause_id in clauses {
                self.infer_where_clause(module, *clause_id, tree, symbols, types, infer, ctx)?;
            }
        }
        Ok(())
    }

    /// Infer heritage.
    fn infer_heritage(
        &self,
        module: &Module,
        heritage: &Heritage,
        _node_id: LocalNodeIdAny,
        symbol: Option<LocalSymbolId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        // analyze and extract extends symbols
        let mut extends_symbols = Vec::new();
        if let Some(extend_types) = &heritage.extends_types {
            for expression_id in extend_types {
                self.infer_expression(module, *expression_id, tree, symbols, types, infer, ctx)?;
                let expression = tree.get(*expression_id);
                if let Some(target_symbol) = expression.target_symbol() {
                    extends_symbols.push(target_symbol);
                }
            }
        }

        // analyze and extract implements symbols
        let mut implements_symbols = Vec::new();
        if let Some(implements_types) = &heritage.implements_types {
            for expression_id in implements_types {
                self.infer_expression(module, *expression_id, tree, symbols, types, infer, ctx)?;
                let expression = tree.get(*expression_id);
                if let Some(target_symbol) = expression.target_symbol() {
                    implements_symbols.push(target_symbol);
                }
            }
        }

        // analyze and extract embedded symbols
        let mut embedded_symbols = Vec::new();
        if let Some(embedded_types) = &heritage.embedded_types {
            for expression_id in embedded_types {
                self.infer_expression(module, *expression_id, tree, symbols, types, infer, ctx)?;
                let expression = tree.get(*expression_id);
                if let Some(target_symbol) = expression.target_symbol() {
                    embedded_symbols.push(target_symbol);
                }
            }
        }

        // build and store lineage if we have a declaring symbol
        if let Some(symbol) = symbol {
            // remember lineage (validation happens in validate phase)
            let lineage = Lineage {
                extends: extends_symbols.first().copied(),
                implements: implements_symbols,
                embedded: embedded_symbols,
            };
            if !lineage.is_empty() {
                let lineage_id = types.insert_lineage(lineage);
                types.set_lineage_for_symbol(symbol.into_global(module.id), lineage_id);
            }
        }

        Ok(())
    }

    /// Infer a function signature.
    fn infer_signature(
        &self,
        module: &Module,
        node_id: LocalNodeIdAny,
        owner_symbol: GlobalSymbolId,
        signature: &FunctionSignature,
        expected_fn_ty_id: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // walk generics
        if let Some(generics) = &signature.generics {
            self.infer_generics(module, generics, tree, symbols, types, infer, ctx)?;
        }

        // extract any contextual function signature
        let expected_signature = self.expected_function_signature(expected_fn_ty_id, types);

        // collect parameter types
        let scope = InferScope {
            owner: owner_symbol,
            function_id: Some(node_id.into_global(module.id)),
        };

        let mut dynamic_param_types = Vec::with_capacity(signature.dynamic_parameters.len());
        for (index, parameter_id) in signature.dynamic_parameters.iter().enumerate() {
            let declared_ty_id =
                types.get_declared_type_id(parameter_id.into_global_any(module.id));
            let expected_param_ty_id = expected_signature
                .as_ref()
                .and_then(|signature| signature.dynamic_parameters.get(index).copied());

            let param_symbol = tree.get(*parameter_id).symbol().into_global(module.id);
            let param_ty_id = declared_ty_id.or(expected_param_ty_id).unwrap_or_else(|| {
                self.infer_var_type_for_symbol(
                    infer,
                    types,
                    param_symbol,
                    InferOrigin::Parameter(parameter_id.into_global_any(module.id)),
                    scope,
                )
            });

            self.infer_parameter(
                module,
                *parameter_id,
                Some(param_ty_id),
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?;

            types.set_value_type(param_symbol, param_ty_id);
            dynamic_param_types.push(param_ty_id);
        }

        // get return type
        let return_type = if let Some(return_type_expr_id) = signature.return_type {
            let return_ty_id = self.infer_expression(
                module,
                return_type_expr_id,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?;
            // unwrap Value type if present
            let return_ty = types.get_type(return_ty_id);
            match return_ty {
                Type::Value { value } => Some(*value),
                _ => Some(return_ty_id),
            }
        } else if let Some(return_type) =
            expected_signature.and_then(|signature| signature.return_type)
        {
            Some(return_type)
        } else {
            Some(self.infer_var_type_for_node(
                infer,
                types,
                node_id.into_global(module.id),
                InferOrigin::Return(node_id.into_global(module.id)),
                scope,
            ))
        };

        // build the function type for this signature
        let ty = Type::Function {
            asynchrony: signature.asynchrony,
            cardinality: signature.cardinality,
            dynamic_parameters: dynamic_param_types,
            static_parameters: Vec::new(), // #Incomplete: static parameters
            return_type,
        };
        let ty_id = types.insert_type_from_any(ty, node_id);

        Ok(ty_id)
    }

    /// Infer a parameter.
    fn infer_parameter(
        &self,
        module: &Module,
        parameter_id: LocalNodeId<Parameter>,
        binding_ty_id: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let parameter = tree.get(parameter_id);
        match parameter {
            Parameter::Named {
                modifiers: _,
                name: _,
                default,
                symbol,
            } => {
                // bind parameter symbol to its type
                if let Some(binding_ty_id) = binding_ty_id {
                    types.set_value_type(symbol.into_global(module.id), binding_ty_id);
                }

                // infer default expression and constrain to parameter type
                if let Some(default) = default {
                    let default_ty_id =
                        self.infer_expression(module, *default, tree, symbols, types, infer, ctx)?;
                    if let Some(binding_ty_id) = binding_ty_id {
                        infer.push_constraint(Constraint::Subtype {
                            sub: default_ty_id,
                            sup: binding_ty_id,
                            variance: None,
                        });
                    }
                }
            }
            Parameter::Pattern {
                modifiers: _,
                pattern,
                default,
                symbol,
            } => {
                // bind parameter symbol to its type
                if let Some(binding_ty_id) = binding_ty_id {
                    types.set_value_type(symbol.into_global(module.id), binding_ty_id);
                }

                // infer default expression and pick a binding type
                let default_ty_id = if let Some(default) = default {
                    Some(self.infer_expression(module, *default, tree, symbols, types, infer, ctx)?)
                } else {
                    None
                };
                let binding_ty_id = binding_ty_id.or(default_ty_id);

                // constrain default to the binding type
                if let (Some(default_ty_id), Some(binding_ty_id)) = (default_ty_id, binding_ty_id) {
                    infer.push_constraint(Constraint::Subtype {
                        sub: default_ty_id,
                        sup: binding_ty_id,
                        variance: None,
                    });
                }

                // infer bindings within the pattern
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
            Parameter::Variadic {
                modifiers: _,
                name: _,
                symbol,
            } => {
                // bind variadic parameter symbol to its type
                if let Some(binding_ty_id) = binding_ty_id {
                    types.set_value_type(symbol.into_global(module.id), binding_ty_id);
                }
            }
        }
        Ok(())
    }

    /// Infer an argument.
    fn infer_argument(
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
    fn infer_dependency_item(
        &self,
        _module: &Module,
        item_id: LocalNodeId<DependencyItem>,
        tree: &NodeTree,
        _symbols: &SymbolTable,
        _types: &mut TypeTable,
        _infer: &mut InferTable,
        _ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let item = tree.get(item_id);
        match item {
            DependencyItem::UnresolvedRemote { .. } => {
                // nothing to do
            }
            DependencyItem::UnresolvedLocal { .. } => {
                // nothing to do
            }
            DependencyItem::Value { .. } => {
                // nothing to do
            }
            DependencyItem::Local { .. } => {
                // nothing to do
            }
            DependencyItem::Remote { .. } => {
                // nothing to do
            }
        }
        Ok(())
    }

    /// Infer where clause.
    fn infer_where_clause(
        &self,
        module: &Module,
        clause_id: LocalNodeId<WhereClause>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let clause = tree.get(clause_id);
        match clause {
            WhereClause::Assertion { left: _, right } => {
                self.infer_expression(module, *right, tree, symbols, types, infer, ctx)?;
            }
            WhereClause::Guard { guard } => {
                self.infer_expression(module, *guard, tree, symbols, types, infer, ctx)?;
            }
        }
        Ok(())
    }

    /// Infer an enum field.
    fn infer_enum_field(
        &self,
        module: &Module,
        field_id: LocalNodeId<EnumField>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let field = tree.get(field_id);
        if let Some(value) = field.value {
            self.infer_expression(module, value, tree, symbols, types, infer, ctx)?;
        }
        Ok(())
    }

    /// Infer a pattern, given an optional binding type of the pattern.
    fn infer_pattern(
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
    fn infer_pattern_sequence(
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
    fn infer_pattern_field(
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

    /// Infer a declarator.
    fn infer_declarator(
        &self,
        module: &Module,
        declarator_id: LocalNodeId<Declarator>,
        _let_expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let declarator = tree.get(declarator_id);
        let Declarator { pattern, ty, value } = declarator;

        // infer type from value or annotation
        // declared type is now on the declarator node, not the let expression
        let declared_ty_id =
            types.get_declared_type_id(declarator_id.into_global(module.id).into());
        let inferred_ty_id = if let Some(value) = value {
            // apply declared type as the expected type when available
            let mut value_ctx = if let Some(declared_ty_id) = declared_ty_id {
                ctx.fork().with_expected_type(Some(declared_ty_id))
            } else {
                ctx.fork()
            };
            Some(self.infer_expression(
                module,
                *value,
                tree,
                symbols,
                types,
                infer,
                &mut value_ctx,
            )?)
        } else {
            None
        };

        // type check: if both declared and inferred, check assignability
        if let (Some(declared), Some(inferred)) = (declared_ty_id, inferred_ty_id) {
            infer.push_constraint(Constraint::Subtype {
                sub: inferred,
                sup: declared,
                variance: None,
            });
            if !self.is_infer_var_type(declared, types)
                && !self.is_infer_var_type(inferred, types)
                && self.check_is_type_assignable(declared, inferred, types)
                    == Assignability::NotAssignable
            {
                return Err(AnalyzeError::UnassignableType {
                    node: declarator_id.into_global(module.id).into(),
                    expected_ty: GlobalTypeId {
                        module_id: module.id,
                        local_id: declared,
                    },
                    actual_ty: GlobalTypeId {
                        module_id: module.id,
                        local_id: inferred,
                    },
                });
            }
        }

        // analyze the type expression if present
        if let Some(ty_id) = ty {
            self.infer_expression(module, *ty_id, tree, symbols, types, infer, ctx)?;
        }

        // infer pattern bindings from declared or inferred type
        let binding_ty_id = declared_ty_id.or(inferred_ty_id);
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

        Ok(())
    }

    /// Derive an expected function signature from a contextual type.
    fn expected_function_signature(
        &self,
        expected_ty_id: Option<LocalTypeId>,
        types: &TypeTable,
    ) -> Option<ExpectedFunctionSignature> {
        let expected_ty_id = self.expected_value_type_id(expected_ty_id, types)?;
        match types.get_type(expected_ty_id) {
            Type::Function {
                dynamic_parameters,
                return_type,
                ..
            } => Some(ExpectedFunctionSignature {
                dynamic_parameters: dynamic_parameters.clone(),
                return_type: *return_type,
            }),
            _ => None,
        }
    }

    /// Derive an expected object type id from a contextual type.
    fn expected_object_type_id(
        &self,
        expected_ty_id: Option<LocalTypeId>,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        let expected_ty_id = self.expected_value_type_id(expected_ty_id, types)?;
        match types.get_type(expected_ty_id) {
            Type::Object { .. } => Some(expected_ty_id),
            Type::Reference { symbol, .. } => types.get_instance_type_id(*symbol),
            _ => None,
        }
    }

    /// Resolve an expected field type from a contextual object type and key.
    fn expected_field_type_id(
        &self,
        expected_object_ty_id: Option<LocalTypeId>,
        key: &StaticKey,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        let expected_object_ty_id = expected_object_ty_id?;
        match types.get_type(expected_object_ty_id) {
            Type::Object { fields } => fields
                .iter()
                .find(|field| &field.key == key)
                .map(|field| field.ty),
            _ => None,
        }
    }

    /// Resolve contextual element types for array and tuple expressions.
    fn expected_element_types(
        &self,
        expected_ty_id: Option<LocalTypeId>,
        element_count: usize,
        types: &TypeTable,
    ) -> Vec<Option<LocalTypeId>> {
        let mut expected = vec![None; element_count];
        let expected_ty_id = match self.expected_value_type_id(expected_ty_id, types) {
            Some(expected_ty_id) => expected_ty_id,
            None => return expected,
        };

        match types.get_type(expected_ty_id) {
            Type::Tuple { elements } => {
                for (index, element_ty_id) in elements.iter().enumerate().take(element_count) {
                    expected[index] = Some(*element_ty_id);
                }
            }
            Type::Array {
                element: Some(element_ty_id),
            } => {
                expected.fill(Some(*element_ty_id));
            }
            _ => {}
        }

        expected
    }

    /// Match a scalar literal against a contextual type when possible.
    fn expected_type_for_scalar_literal(
        &self,
        value: &ScalarLiteral,
        expected_ty_id: Option<LocalTypeId>,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        let expected_ty_id = self.expected_value_type_id(expected_ty_id, types)?;
        self.match_scalar_literal_expected(value, expected_ty_id, types)
    }

    /// Strip a Type::Value wrapper from a type id when present.
    fn expected_value_type_id(
        &self,
        expected_ty_id: Option<LocalTypeId>,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        let expected_ty_id = expected_ty_id?;
        let expected_ty_id = self.unwrap_value_type_id(expected_ty_id, types);
        if self.is_infer_var_type(expected_ty_id, types) {
            None
        } else {
            Some(expected_ty_id)
        }
    }

    /// Unwrap Type::Value to its underlying type.
    fn unwrap_value_type_id(&self, ty_id: LocalTypeId, types: &TypeTable) -> LocalTypeId {
        match types.get_type(ty_id) {
            Type::Value { value } => *value,
            _ => ty_id,
        }
    }

    /// Match a scalar literal against an expected type.
    fn match_scalar_literal_expected(
        &self,
        value: &ScalarLiteral,
        expected_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        match types.get_type(expected_ty_id) {
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number),
            } => match value {
                ScalarLiteral::Integer(_) | ScalarLiteral::Float(_) => Some(expected_ty_id),
                _ => None,
            },
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            } => match value {
                ScalarLiteral::String(_) => Some(expected_ty_id),
                _ => None,
            },
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            } => match value {
                ScalarLiteral::Boolean(_) => Some(expected_ty_id),
                _ => None,
            },
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(expected_literal),
            } => {
                if expected_literal == value {
                    Some(expected_ty_id)
                } else {
                    None
                }
            }
            Type::Union { elements } => {
                for element in elements {
                    if let Some(matched) =
                        self.match_scalar_literal_expected(value, *element, types)
                    {
                        return Some(matched);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Get or create an inference variable type for a symbol.
    fn infer_var_type_for_symbol(
        &self,
        infer: &mut InferTable,
        types: &mut TypeTable,
        symbol: GlobalSymbolId,
        origin: InferOrigin,
        scope: InferScope,
    ) -> LocalTypeId {
        // reuse existing inference variable when available
        if let Some(var_id) = infer.var_by_symbol_id.get(&symbol).copied()
            && let Some(ty_id) = infer.type_for_var(var_id)
            && ty_id.0 != u32::MAX
        {
            return ty_id;
        }

        // allocate and bind a new inference variable
        let var_id = infer.new_var(origin, scope);
        let ty_id = types.insert_type(Type::InferVar { id: var_id });
        infer.bind_type(var_id, ty_id);
        infer.var_by_symbol_id.insert(symbol, var_id);
        ty_id
    }

    /// Get or create an inference variable type for a node.
    fn infer_var_type_for_node(
        &self,
        infer: &mut InferTable,
        types: &mut TypeTable,
        node_id: GlobalNodeIdAny,
        origin: InferOrigin,
        scope: InferScope,
    ) -> LocalTypeId {
        // reuse existing inference variable when available
        if let Some(var_id) = infer.var_by_node_id.get(&node_id).copied()
            && let Some(ty_id) = infer.type_for_var(var_id)
            && ty_id.0 != u32::MAX
        {
            return ty_id;
        }

        // allocate and bind a new inference variable
        let var_id = infer.new_var(origin, scope);
        let ty_id = types.insert_type(Type::InferVar { id: var_id });
        infer.bind_type(var_id, ty_id);
        infer.var_by_node_id.insert(node_id, var_id);
        ty_id
    }

    /// Check whether a type id points at an inference variable.
    fn is_infer_var_type(&self, ty_id: LocalTypeId, types: &TypeTable) -> bool {
        matches!(types.get_type(ty_id), Type::InferVar { .. })
    }

    /// Extract the return type from a function type.
    fn function_return_type(
        &self,
        fn_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        match types.get_type(fn_ty_id) {
            Type::Function { return_type, .. } => *return_type,
            _ => None,
        }
    }
}

/// Contextual function signature derived from an expected type.
#[derive(Debug, Clone)]
struct ExpectedFunctionSignature {
    /// Expected dynamic parameter types.
    dynamic_parameters: Vec<LocalTypeId>,
    /// Expected return type.
    return_type: Option<LocalTypeId>,
}

#[cfg(test)]
mod tests {
    use destack_dir::{
        Declaration, Expression, LocalNodeId, NodeTree, PrimitiveType, ScalarLiteral, Type,
        TypeLiteral,
    };
    use destack_source::ModuleId;

    use crate::{TestProgram, assert_type};

    fn first_function_symbol(
        module_id: ModuleId,
        tree: &NodeTree,
        roots: &[LocalNodeId<Expression>],
    ) -> destack_dir::GlobalSymbolId {
        for root_id in roots {
            let expression = tree.get(*root_id);
            let declaration_id = match expression {
                Expression::Declaration { declaration } => Some(*declaration),
                Expression::Statement { statement } => match tree.get(*statement) {
                    Expression::Declaration { declaration } => Some(*declaration),
                    _ => None,
                },
                _ => None,
            };
            if let Some(declaration_id) = declaration_id {
                let declaration = tree.get(declaration_id);
                if let Declaration::Function { descriptor, .. } = declaration {
                    return descriptor.symbol.into_global(module_id);
                }
            }
        }
        panic!("expected function declaration");
    }

    #[test]
    fn test_analyze_number_literal() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let tree = module.dir().tree.read();
        let types = module.dir().types.read();

        let expression_id = module.dir().roots[0];
        let expression = tree.get(expression_id);
        let &Expression::Statement {
            statement: expression_id,
        } = expression
        else {
            panic!("expected statement");
        };
        let ty = types
            .get_inferred_type(expression_id.into_global_any(module.id))
            .unwrap();

        // integer literals now have literal types, not widened primitive types
        assert_eq!(
            *ty,
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(42))
            }
        );
    }

    #[test]
    fn test_analyze_string_literal() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", r#""hello""#);
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let tree = module.dir().tree.read();
        let types = module.dir().types.read();

        let expression_id = module.dir().roots[0];
        let expression = tree.get(expression_id);
        let &Expression::Statement {
            statement: expression_id,
        } = expression
        else {
            panic!("expected statement");
        };
        let ty = types
            .get_inferred_type(expression_id.into_global_any(module.id))
            .unwrap();

        // string literal has literal type (e.g., "hello" has type "hello")
        assert!(matches!(
            ty,
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_))
            }
        ));
    }

    #[test]
    fn test_analyze_boolean_literal() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "true");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let tree = module.dir().tree.read();
        let types = module.dir().types.read();

        let expression_id = module.dir().roots[0];
        let expression = tree.get(expression_id);
        let &Expression::Statement {
            statement: expression_id,
        } = expression
        else {
            panic!("expected statement");
        };
        let ty = types
            .get_inferred_type(expression_id.into_global_any(module.id))
            .unwrap();

        // boolean literal has literal type (e.g., true has type true)
        assert_eq!(
            *ty,
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(true))
            }
        );
    }

    #[test]
    fn test_analyze_binary_number_operation() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "1 + 2");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let tree = module.dir().tree.read();
        let types = module.dir().types.read();

        let expression_id = module.dir().roots[0];
        let expression = tree.get(expression_id);
        let &Expression::Statement {
            statement: expression_id,
        } = expression
        else {
            panic!("expected statement");
        };
        let ty = types
            .get_inferred_type(expression_id.into_global_any(module.id))
            .unwrap();

        // constant folding: 1 + 2 evaluates to literal type 3
        assert_eq!(
            *ty,
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(3))
            }
        );
    }

    #[test]
    fn test_analyze_binary_number_comparison() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "1 < 2");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let tree = module.dir().tree.read();
        let types = module.dir().types.read();

        let expression_id = module.dir().roots[0];
        let expression = tree.get(expression_id);
        let &Expression::Statement {
            statement: expression_id,
        } = expression
        else {
            panic!("expected statement");
        };
        let ty = types
            .get_inferred_type(expression_id.into_global_any(module.id))
            .unwrap();

        // constant folding: 1 < 2 evaluates to literal true
        assert_eq!(
            *ty,
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(true))
            }
        );
    }

    #[test]
    fn test_analyze_let_expression_infer_type() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "let x = 42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let types = module.dir().types.read();

        let let_expr_id = module.dir().roots[0];
        let x_symbol = test.resolve_to_symbol("test.ds", "x").unwrap();

        // no declared type
        assert!(
            types
                .get_declared_type(let_expr_id.into_global_any(module.id))
                .is_none()
        );

        // value_type[x] = literal 42
        let x_ty = types.get_value_type(x_symbol).unwrap();
        assert_eq!(
            *x_ty,
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(42))
            }
        );

        // instance_type[x] = undefined
        assert!(types.get_instance_type(x_symbol).is_none());
    }

    #[test]
    fn test_analyze_let_expression_declare_type() {
        // use compatible types: string annotation with string value
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", r#"let x: string = "hello""#);
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let tree = module.dir().tree.read();
        let types = module.dir().types.read();

        let expression_id = module.dir().roots[0];
        let expression = tree.get(expression_id);
        let &Expression::Statement {
            statement: expression_id,
        } = expression
        else {
            panic!("expected statement");
        };
        let let_expression = tree.get(expression_id);
        let Expression::Let { declarators, .. } = let_expression else {
            panic!("expected let expression");
        };
        let declarator_id = declarators.first().unwrap();
        let x_symbol = test.resolve_to_symbol("test.ds", "x").unwrap();

        // declared_type[declarator] = string
        let declared = types
            .get_declared_type(declarator_id.into_global(module.id).into())
            .unwrap();
        assert_eq!(
            *declared,
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String)
            }
        );

        // value_type[x] = string
        let x_ty = types.get_value_type(x_symbol).unwrap();
        assert_eq!(
            *x_ty,
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String)
            }
        );

        // instance_type[x] = undefined
        assert!(types.get_instance_type(x_symbol).is_none());
    }

    #[test]
    fn test_analyze_let_expression_infer_tuple_type_with_pattern() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
let (x, y, ...rest, z) = (123, 'abc', true, 456);
"#,
        );
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let types = module.dir().types.read();

        let x_symbol = test.resolve_to_symbol("test.ds", "x").unwrap();
        let y_symbol = test.resolve_to_symbol("test.ds", "y").unwrap();
        let rest_symbol = test.resolve_to_symbol("test.ds", "rest").unwrap();
        let z_symbol = test.resolve_to_symbol("test.ds", "z").unwrap();

        // value_type[x] = literal 123
        let x_ty_id = types.get_value_type_id(x_symbol).unwrap();
        assert_type!(
            types,
            x_ty_id,
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(123))
            }
        );

        // value_type[y] = literal 'abc'
        let y_ty_id = types.get_value_type_id(y_symbol).unwrap();
        assert_type!(
            types,
            y_ty_id,
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_))
            }
        );

        // value_type[rest] = (true,)
        let rest_ty_id = types.get_value_type_id(rest_symbol).unwrap();
        assert_type!(types, rest_ty_id, Type::Tuple { elements } => {
            assert_eq!(elements.len(), 1);
            assert_type!(types, elements[0], Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(true))
            });
        });

        // value_type[z] = literal 456
        let z_ty_id = types.get_value_type_id(z_symbol).unwrap();
        assert_type!(
            types,
            z_ty_id,
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(456))
            }
        );
    }

    #[test]
    fn test_analyze_let_expression_infer_array_tuple_type_with_pattern() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
let [x, y, ...rest, z] = [123, 'abc', true, 456]; // array used as a tuple
"#,
        );
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let types = module.dir().types.read();

        let x_symbol = test.resolve_to_symbol("test.ds", "x").unwrap();
        let y_symbol = test.resolve_to_symbol("test.ds", "y").unwrap();
        let rest_symbol = test.resolve_to_symbol("test.ds", "rest").unwrap();
        let z_symbol = test.resolve_to_symbol("test.ds", "z").unwrap();

        // value_type[x] = literal 123
        let x_ty_id = types.get_value_type_id(x_symbol).unwrap();
        assert_type!(
            types,
            x_ty_id,
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(123))
            }
        );

        // value_type[y] = literal 'abc'
        let y_ty_id = types.get_value_type_id(y_symbol).unwrap();
        assert_type!(
            types,
            y_ty_id,
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_))
            }
        );

        // value_type[rest] = true[] (array of boolean literal)
        let rest_ty_id = types.get_value_type_id(rest_symbol).unwrap();
        assert_type!(types, rest_ty_id, Type::Array { element: Some(element_id) } => {
            assert_type!(types, *element_id, Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(true))
            });
        });

        // value_type[z] = literal 456
        let z_ty_id = types.get_value_type_id(z_symbol).unwrap();
        assert_type!(
            types,
            z_ty_id,
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(456))
            }
        );
    }

    #[test]
    fn test_analyze_cross_module_type_import() {
        // import a value from another module and verify its type is correctly imported
        let test = TestProgram::memory_sequential();
        test.add_file(
            "lib.ds",
            r#"
export let value = 42;
"#,
        );
        let module_id = test.add_module(
            "main.ds",
            r#"
import { value } from "./lib.ds";
let x = value;
"#,
        );
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let types = module.dir().types.read();

        // x should have literal type 42 (imported from lib.ds)
        let x_symbol = test.resolve_to_symbol("main.ds", "x").unwrap();
        let x_ty_id = types.get_value_type_id(x_symbol).unwrap();
        assert_type!(
            types,
            x_ty_id,
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(42))
            }
        );
    }

    #[test]
    fn test_analyze_cross_module_tuple_type_import() {
        // import a tuple value from another module
        // (array literal [1, 2, 3] is inferred as tuple, not array)
        let test = TestProgram::memory_sequential();
        test.add_file(
            "lib.ds",
            r#"
export let items = [1, 2, 3];
"#,
        );
        let module_id = test.add_module(
            "main.ds",
            r#"
import { items } from "./lib.ds";
let x = items;
"#,
        );
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let types = module.dir().types.read();

        // x should have tuple type [1, 2, 3] with literal elements (imported from lib.ds)
        let x_symbol = test.resolve_to_symbol("main.ds", "x").unwrap();
        let x_ty_id = types.get_value_type_id(x_symbol).unwrap();
        assert_type!(types, x_ty_id, Type::Tuple { elements } => {
            assert_eq!(elements.len(), 3);
            for elem_id in elements {
                assert_type!(types, *elem_id, Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(_))
                });
            }
        });
    }

    #[test]
    fn test_analyze_cross_module_string_type_import() {
        // import a string value from another module
        let test = TestProgram::memory_sequential();
        test.add_file(
            "lib.ds",
            r#"
export let greeting = "hello";
"#,
        );
        let module_id = test.add_module(
            "main.ds",
            r#"
import { greeting } from "./lib.ds";
let x = greeting;
"#,
        );
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let types = module.dir().types.read();

        // x should have literal string type (imported from lib.ds)
        let x_symbol = test.resolve_to_symbol("main.ds", "x").unwrap();
        let x_ty_id = types.get_value_type_id(x_symbol).unwrap();
        assert_type!(
            types,
            x_ty_id,
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_))
            }
        );
    }

    /// Resolve member access on object literal to field type.
    #[test]
    fn test_analyze_member_access_object_field() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
let obj = { x: 42, y: "hello" };
let a = obj.x;
let b = obj.y;
"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// Resolve member access across multiple fields.
    #[test]
    fn test_analyze_member_access_multiple_fields() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
let obj = { x: 42, y: "hello", z: true };
let a = obj.x;
let b = obj.y;
let c = obj.z;
"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// Resolve chained member access on nested objects.
    #[test]
    fn test_analyze_member_access_chained() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
let obj = { inner: { value: 42 } };
let a = obj.inner.value;
"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_clean();
    }

    /// Infer an inherent extension.
    #[test]
    fn test_analyze_inherent_extension() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
struct Point { 
    x: number, 
    y: number,
}

extension for Point {
    magnitude(): number { 
        return 0; 
    }
}

extension for Point {
    distance(other: Point): number { 
        return 0; 
    }
}
"#,
        );
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let _point_id = test.resolve_to_symbol("test.ds", "Point").unwrap();
        // #Incomplete: #Extensions
    }

    /// Infer a local extension (on a foreign type).
    #[test]
    fn test_analyze_local_extension() {
        let test = TestProgram::memory_sequential();
        test.add_file(
            "point.ds",
            r#"
struct Point { 
    x: number, 
    y: number,
}
"#,
        );
        let module_id = test.add_module(
            "test.ds",
            r#"
import { Point } from "./point.ds";

// local extension on foreign type
extension for Point {
    distance(other: Point): number { return 0; }
}
"#,
        );
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let _point_id = test.resolve_to_symbol("test.ds", "Point").unwrap();
        // #Incomplete: #Extensions
    }

    /// Infer a named extension (on a foreign type, from a foreign extension).
    #[test]
    fn test_analyze_named_extension() {
        let test = TestProgram::memory_sequential();
        test.add_file(
            "point.ds",
            r#"
struct Point { 
    x: number, 
    y: number,
}
"#,
        );
        test.add_file(
            "extensions.ds",
            r#"
import { Point } from "./point.ds";

extension PointHelpers for Point {
    distance(other: Point): number { return 0; }
}
"#,
        );
        let module_id = test.add_module(
            "test.ds",
            r#"
import { PointHelpers } from "./extensions.ds";
import { Point } from "./point.ds";
"#,
        );
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let _point_id = test.resolve_to_symbol("test.ds", "Point").unwrap();
        let _point_helpers_id = test.resolve_to_symbol("test.ds", "PointHelpers").unwrap();
        // #Incomplete: #Extensions
    }

    #[test]
    fn test_analyze_infer_parameter_types_from_call_arguments() {
        // infers parameter types from call arguments
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function add(a, b) {
    return a + b
}
add(1, 2)
"#,
        );
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let tree = module.dir().tree.read();
        let types = module.dir().types.read();

        let fn_symbol = first_function_symbol(module.id, &tree, &module.dir().roots);
        let fn_ty_id = types
            .get_value_type_id(fn_symbol)
            .expect("expected function type");

        assert_type!(types, fn_ty_id, Type::Function { dynamic_parameters, return_type, .. } => {
            assert_eq!(dynamic_parameters.len(), 2);
            assert_type!(types, dynamic_parameters[0], Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(1))
            });
            assert_type!(types, dynamic_parameters[1], Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(2))
            });
            let return_type = return_type.expect("expected return type");
            assert_type!(types, return_type, Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number)
            });
        });
    }

    #[test]
    fn test_analyze_infer_parameter_type_from_default() {
        // infers parameter type from default value
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function greet(name = "hi") {
    return name
}
"#,
        );
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let tree = module.dir().tree.read();
        let types = module.dir().types.read();

        let fn_symbol = first_function_symbol(module.id, &tree, &module.dir().roots);
        let fn_ty_id = types
            .get_value_type_id(fn_symbol)
            .expect("expected function type");

        assert_type!(types, fn_ty_id, Type::Function { dynamic_parameters, return_type, .. } => {
            assert_eq!(dynamic_parameters.len(), 1);
            assert_type!(types, dynamic_parameters[0], Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_))
            });
            let return_type = return_type.expect("expected return type");
            assert_type!(types, return_type, Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_))
            });
        });
    }

    #[test]
    fn test_analyze_contextual_lambda_from_annotation() {
        // infers lambda signature from contextual function type
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
const add: (a: number, b: number) => number = (a, b) => a + b;
"#,
        );
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let tree = module.dir().tree.read();
        let types = module.dir().types.read();

        let expression_id = module.dir().roots[0];
        let expression = tree.get(expression_id);
        let &Expression::Statement {
            statement: expression_id,
        } = expression
        else {
            panic!("expected statement");
        };
        let let_expression = tree.get(expression_id);
        let Expression::Let { declarators, .. } = let_expression else {
            panic!("expected let expression");
        };
        let declarator_id = declarators.first().unwrap();
        let declarator = tree.get(*declarator_id);
        let value_id = declarator.value.expect("expected function value");
        let value_ty_id = types
            .get_inferred_type_id(value_id.into_global_any(module.id))
            .expect("expected function type");

        assert_type!(types, value_ty_id, Type::Function { dynamic_parameters, return_type, .. } => {
            assert_eq!(dynamic_parameters.len(), 2);
            assert_type!(types, dynamic_parameters[0], Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number)
            });
            assert_type!(types, dynamic_parameters[1], Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number)
            });
            let return_type = return_type.expect("expected return type");
            assert_type!(types, return_type, Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number)
            });
        });
    }

    #[test]
    fn test_analyze_contextual_lambda_from_argument() {
        // infers lambda signature from argument type
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function apply(fn: (a: number) => number): number {
    return fn(1);
}
apply((a) => a + 1);
"#,
        );
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let tree = module.dir().tree.read();
        let types = module.dir().types.read();

        let call_root_id = module.dir().roots[1];
        let call_root = tree.get(call_root_id);
        let &Expression::Statement {
            statement: call_expression_id,
        } = call_root
        else {
            panic!("expected statement");
        };
        let call_expression = tree.get(call_expression_id);
        let Expression::Call {
            dynamic_arguments, ..
        } = call_expression
        else {
            panic!("expected call expression");
        };
        let argument_id = dynamic_arguments.first().expect("expected argument");
        let argument = tree.get(*argument_id);
        let argument_value_id = argument.value();
        let argument_ty_id = types
            .get_inferred_type_id(argument_value_id.into_global_any(module.id))
            .expect("expected argument type");

        assert_type!(types, argument_ty_id, Type::Function { dynamic_parameters, return_type, .. } => {
            assert_eq!(dynamic_parameters.len(), 1);
            assert_type!(types, dynamic_parameters[0], Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number)
            });
            let return_type = return_type.expect("expected return type");
            assert_type!(types, return_type, Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number)
            });
        });
    }

    #[test]
    fn test_analyze_contextual_object_literal() {
        // infers object literal field types from contextual type
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
const point: { x: number, y: string } = { x: 1, y: "hi" };
"#,
        );
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let tree = module.dir().tree.read();
        let types = module.dir().types.read();

        let expression_id = module.dir().roots[0];
        let expression = tree.get(expression_id);
        let &Expression::Statement {
            statement: expression_id,
        } = expression
        else {
            panic!("expected statement");
        };
        let let_expression = tree.get(expression_id);
        let Expression::Let { declarators, .. } = let_expression else {
            panic!("expected let expression");
        };
        let declarator_id = declarators.first().unwrap();
        let declarator = tree.get(*declarator_id);
        let value_id = declarator.value.expect("expected object value");
        let value_ty_id = types
            .get_inferred_type_id(value_id.into_global_any(module.id))
            .expect("expected object type");

        assert_type!(types, value_ty_id, Type::Object { fields } => {
            assert_eq!(fields.len(), 2);
            let mut saw_number = false;
            let mut saw_string = false;
            for field in fields {
                match types.get_type(field.ty) {
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::Number),
                    } => saw_number = true,
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::String),
                    } => saw_string = true,
                    _ => {}
                }
            }
            assert!(saw_number);
            assert!(saw_string);
        });
    }

    #[test]
    fn test_analyze_contextual_array_literal() {
        // infers array literal element types from contextual type
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
const numbers: number[] = [1, 2];
"#,
        );
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let tree = module.dir().tree.read();
        let types = module.dir().types.read();

        let expression_id = module.dir().roots[0];
        let expression = tree.get(expression_id);
        let &Expression::Statement {
            statement: expression_id,
        } = expression
        else {
            panic!("expected statement");
        };
        let let_expression = tree.get(expression_id);
        let Expression::Let { declarators, .. } = let_expression else {
            panic!("expected let expression");
        };
        let declarator_id = declarators.first().unwrap();
        let declarator = tree.get(*declarator_id);
        let value_id = declarator.value.expect("expected array value");
        let value_ty_id = types
            .get_inferred_type_id(value_id.into_global_any(module.id))
            .expect("expected array type");

        assert_type!(types, value_ty_id, Type::Tuple { elements } => {
            assert_eq!(elements.len(), 2);
            assert_type!(types, elements[0], Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number)
            });
            assert_type!(types, elements[1], Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number)
            });
        });
    }
}
