use crate::{
    AnalyzeError, AnalyzeResult, Assignability, Compiler, Constraint, InferContext, InferTable,
};
use destack_dir::{
    Argument, BindingKind, Block, Declaration, DynamicKey, Expression, FunctionKind,
    GlobalSymbolId, LocalNodeId, LocalTypeId, MatchCase, MatchSelector, MatchSource, Mutability,
    NodeTree, Pattern, PatternField, PrimitiveType, Property, StaticKey, SymbolTable, Type,
    TypeElement, TypeField, TypeLiteral, TypeTable,
};
use destack_workspace::{Module, ProfileId};

/// Object literal field metadata for excess property checks.
#[derive(Debug, Clone)]
pub(super) struct ObjectLiteralField {
    /// The field of the object literal.
    field: TypeField,
    /// The corresponding property of the object literal.
    property_id: LocalNodeId<Property>,
}

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
                    &ctx.options,
                );
                types.insert_type_from(ty, expression_id)
            }
            Expression::TypeConditional { .. }
            | Expression::TypeMapped { .. }
            | Expression::TypeIndex { .. }
            | Expression::TypeTemplateLiteral { .. }
            | Expression::TypeImport { .. }
            | Expression::TypeInfer { .. }
            | Expression::TypePredicate { .. } => {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // unary operations: compound type
            Expression::Unary { operator, right } => self.infer_unary_expression(
                module,
                expression_id,
                operator,
                *right,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?,

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
            Expression::Binary {
                left,
                operator,
                right,
            } => self.infer_binary_expression(
                module,
                expression_id,
                operator,
                *left,
                *right,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?,

            // assignment operations: void
            Expression::Assign { left, right } => self.infer_assign_expression(
                module,
                expression_id,
                *left,
                *right,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?,
            Expression::AssignBinary { left, right, .. } => self.infer_assign_binary_expression(
                module,
                expression_id,
                *left,
                *right,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?,

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
            } => self.infer_reference_expression(
                module,
                expression_id,
                *target_symbol,
                static_arguments.as_deref(),
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?,

            // import meta: statically known type
            Expression::ImportMeta => {
                if module.module_type.is_script() {
                    self.error(AnalyzeError::InvalidImportMeta {
                        node: expression_id.into_global_any(module.id),
                    });
                }
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // this: reference to the current instance item
            Expression::This => {
                let this_symbol = self.find_this_symbol(module, expression_id, tree, symbols);
                if let Some(this_symbol) = this_symbol
                    && let Some(ty_id) = types.get_value_type_id(this_symbol)
                {
                    ty_id
                } else if module.module_type.is_module() {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Undefined,
                    };
                    types.insert_type_from(ty, expression_id)
                } else {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    types.insert_type_from(ty, expression_id)
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
                let element_tys: Vec<TypeElement> = elements
                    .iter()
                    .map(|element_id| {
                        let element = tree.get(*element_id);
                        let element_id = element.value();
                        let ty = self.infer_expression(
                            module,
                            element_id,
                            tree,
                            symbols,
                            types,
                            infer,
                            ctx,
                        )?;
                        Ok(TypeElement::new(ty))
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
                let mut literal_fields = Vec::new();
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
                        literal_fields.push(field);
                    }
                }
                self.check_excess_object_literal_properties(
                    module,
                    ctx.expected_type,
                    &literal_fields,
                    types,
                );

                let fields = literal_fields
                    .iter()
                    .map(|field| field.field.clone())
                    .collect();
                let ty = Type::Object {
                    fields,
                    call_signatures: Vec::new(),
                    construct_signatures: Vec::new(),
                    index_signatures: Vec::new(),
                };
                types.insert_type_from(ty, expression_id)
            }

            // call: return type of callee
            Expression::Call {
                left,
                static_arguments,
                dynamic_arguments,
            } => self.infer_call_expression(
                module,
                expression_id,
                *left,
                static_arguments.as_deref(),
                dynamic_arguments,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?,

            Expression::Member {
                left,
                name,
                static_arguments,
            } => self.infer_member_expression(
                module,
                expression_id,
                *left,
                *name,
                static_arguments.as_deref(),
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?,

            // index: element type
            Expression::Index { left, right } => {
                self.infer_index_access_expression(
                    module,
                    expression_id,
                    *left,
                    *right,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?
            }

            // new: instance type
            Expression::New {
                left,
                static_arguments: _,
                dynamic_arguments,
            } => self.infer_new_expression(
                module,
                expression_id,
                *left,
                dynamic_arguments,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?,

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
                            sub_type: value_ty_id,
                            super_type: return_ty_id,
                            variance: None,
                        });
                    }
                } else if let Some(return_ty_id) = ctx.return_type {
                    let void_ty_id = types.insert_type(Type::TypeLiteral {
                        value: TypeLiteral::Void,
                    });
                    infer.push_constraint(Constraint::Subtype {
                        sub_type: void_ty_id,
                        super_type: return_ty_id,
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

            // await? should be desugared in Bind
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

            // maybe unwrap: try operator
            Expression::Maybe { left } => self.infer_try_unwrap_expression(
                module,
                expression_id,
                *left,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?,

            // must unwrap: non null assertion
            Expression::Must { left } => {
                let left_ty_id =
                    self.infer_expression(module, *left, tree, symbols, types, infer, ctx)?;
                let (non_nullish_ty_id, has_nullish) =
                    self.strip_nullish_from_union(left_ty_id, types);
                if has_nullish {
                    non_nullish_ty_id.unwrap_or_else(|| {
                        types.insert_type_from(
                            Type::TypeLiteral {
                                value: TypeLiteral::Unknown,
                            },
                            expression_id,
                        )
                    })
                } else {
                    left_ty_id
                }
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
                // NOTE #Incomplete: tagged template should return type from tag function
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
                for property_id in properties {
                    self.infer_property(
                        module,
                        *property_id,
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

    /// Find the nearest `this` symbol visible to the expression.
    /// NOTE #Architecture: should find_this_symbol be resolved during Bind? (instead of Analyze/infer)?
    fn find_this_symbol(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        let this_name = self.program.strings.intern("this");
        let mut scope = symbols.get_scope(expression_id, tree);
        loop {
            if let Some(symbol_id) = scope.1.find_up_to(StaticKey::Name(this_name), scope.2) {
                return Some(symbol_id.into_global(module.id));
            }
            let (parent_scope_id, parent_mark) = scope.1.parent?;
            scope = (
                parent_scope_id,
                symbols.get_scope_by_id(parent_scope_id),
                parent_mark,
            );
        }
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
    ) -> AnalyzeResult<Option<ObjectLiteralField>> {
        let options = ctx.options;
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
                    DynamicKey::Number(name) => Some(StaticKey::Number(name)),
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
                    .is_some_and(|m| matches!(m.kind, Some(BindingKind::Maybe)));

                // is readonly
                let is_readonly = modifiers
                    .as_ref()
                    .is_some_and(|m| matches!(m.mutability, Some(Mutability::Immutable)));

                // static key
                if let Some(key) = static_key {
                    Ok(Some(ObjectLiteralField {
                        field: TypeField {
                            key,
                            ty: value_ty_id,
                            is_optional,
                            is_readonly,
                        },
                        property_id,
                    }))
                } else {
                    Ok(None)
                }
            }
            Property::Method {
                modifiers,
                key,
                signature,
                body,
                symbol,
                ..
            } => {
                let expected_method_ty_id = key
                    .and_then(|key| match key {
                        DynamicKey::Name(name) => Some(StaticKey::Name(name)),
                        DynamicKey::Number(name) => Some(StaticKey::Number(name)),
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
                            sub_type: body_ty_id,
                            super_type: return_ty_id,
                            variance: None,
                        });

                        if !self.is_infer_var_type(return_ty_id, types)
                            && !self.is_infer_var_type(body_ty_id, types)
                            && self.check_is_type_assignable(
                                return_ty_id,
                                body_ty_id,
                                types,
                                &options,
                            ) == Assignability::NotAssignable
                        {
                            self.error(AnalyzeError::UnassignableType {
                                node: body.into_global_any(module.id),
                                expected_ty: return_ty_id.into_global(module.id),
                                actual_ty: body_ty_id.into_global(module.id),
                            });
                        }
                    }
                }
                let static_key = key.and_then(|key| match key {
                    DynamicKey::Name(name) => Some(StaticKey::Name(name)),
                    DynamicKey::Number(name) => Some(StaticKey::Number(name)),
                    DynamicKey::Expression(_) | DynamicKey::NamedExpression { .. } => None,
                });
                let is_optional = modifiers
                    .as_ref()
                    .is_some_and(|m| matches!(m.kind, Some(destack_dir::BindingKind::Maybe)));
                let is_readonly = modifiers.as_ref().is_some_and(|m| {
                    matches!(m.mutability, Some(destack_dir::Mutability::Immutable))
                });
                if let Some(key) = static_key {
                    Ok(Some(ObjectLiteralField {
                        field: TypeField {
                            key,
                            ty: method_ty_id,
                            is_optional,
                            is_readonly,
                        },
                        property_id,
                    }))
                } else {
                    Ok(None)
                }
            }
            Property::Spread { value, .. } => {
                // #Incomplete: expand spread type into object type #TypeNormalization
                self.infer_expression(module, *value, tree, symbols, types, infer, ctx)?;
                Ok(None)
            }
        }
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
            Pattern::Must(pattern_id) => {
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
                        elements: rest_types.into_iter().map(TypeElement::new).collect(),
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
                        elements: rest_types.into_iter().map(TypeElement::new).collect(),
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
        let binding_ty_fields: Vec<LocalTypeId> = binding_ty_id
            .and_then(|ty_id| match types.get_type(ty_id) {
                Type::Tuple { elements } => {
                    Some(elements.iter().map(|element| element.ty).collect())
                }
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

    /// Get the target symbol for a reference expression.
    pub(super) fn reference_symbol_for_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        match tree.get(expression_id) {
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => {
                Some(self.canonical_symbol_id(module, symbols, profile, *target_symbol))
            }
            _ => None,
        }
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

    /// Infer a reference expression (local, module, or global).
    pub(super) fn infer_reference_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
        static_arguments: Option<&[LocalNodeId<Argument>]>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // resolve the canonical symbol for imported references
        let canonical_symbol =
            self.canonical_symbol_id(module, symbols, ctx.profile, target_symbol);

        // pick the base type for the symbol
        let base_ty_id = if let Some(narrowed_ty_id) = ctx.get_narrowed(canonical_symbol) {
            narrowed_ty_id
        } else if let Some(value_ty_id) = types.get_value_type_id(canonical_symbol) {
            value_ty_id
        } else if canonical_symbol.module_id != module.id {
            self.resolve_remote_symbol_value_type(
                module,
                ctx.profile,
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

        // handle static arguments for generic instantiation
        let Some(static_argument_ids) = static_arguments else {
            return Ok(base_ty_id);
        };

        let Type::Function {
            asynchrony,
            cardinality,
            static_parameters,
            this_parameter,
            dynamic_parameters,
            return_type,
        } = types.get_type(base_ty_id).clone()
        else {
            self.error(AnalyzeError::MissingType {
                node: expression_id.into_global_any(module.id),
            });
            return Ok(base_ty_id);
        };

        let resolved = self.resolve_function_signature(
            module,
            expression_id.into_any(),
            Some(canonical_symbol),
            Some(static_argument_ids),
            &static_parameters,
            &dynamic_parameters,
            return_type,
            ctx.profile,
            &ctx.options,
            tree,
            symbols,
            types,
            infer,
        )?;

        let instantiated_fn = Type::Function {
            asynchrony,
            cardinality,
            static_parameters: Vec::new(),
            this_parameter,
            dynamic_parameters: resolved.dynamic_parameters,
            return_type: resolved.return_type,
        };
        let instantiated_ty_id = types.insert_type_from(instantiated_fn, expression_id);

        if !resolved.static_arguments.is_empty() {
            self.register_instance_for_node(
                expression_id.into_global_any(module.id),
                canonical_symbol,
                resolved.static_arguments,
                types,
            );
        }

        Ok(instantiated_ty_id)
    }

    /// Check excess properties on an object literal against a contextual type.
    fn check_excess_object_literal_properties(
        &self,
        module: &Module,
        expected_ty_id: Option<LocalTypeId>,
        fields: &[ObjectLiteralField],
        types: &TypeTable,
    ) {
        // collect candidates for excess property checks
        let Some(expected_ty_id) = self.expected_value_type_id(expected_ty_id, types) else {
            return;
        };
        let mut candidates: Vec<LocalTypeId> = Vec::new();
        self.collect_object_literal_candidates(expected_ty_id, types, &mut candidates);
        if candidates.is_empty() {
            return;
        }

        // check if any candidate matches the fields
        for candidate in candidates.iter().copied() {
            if self.object_literal_matches_target(fields, candidate, types) {
                return;
            }
        }

        // if no candidate matches, report the first excess property
        let Some(candidate) = candidates.first().copied() else {
            return;
        };
        let excess_fields = self.object_literal_excess_properties(fields, candidate, types);
        if excess_fields.is_empty() {
            return;
        }

        for (property_id, member_key) in excess_fields {
            self.error(AnalyzeError::ExcessProperty {
                node: property_id.into_global_any(module.id),
                expected_ty: candidate.into_global(module.id),
                member_key,
            });
        }
    }

    /// Collect object-like candidates for excess property checks.
    fn collect_object_literal_candidates(
        &self,
        expected_ty_id: LocalTypeId,
        types: &TypeTable,
        candidates: &mut Vec<LocalTypeId>,
    ) {
        match types.get_type(expected_ty_id) {
            Type::Object { .. } => candidates.push(expected_ty_id),
            Type::Reference { symbol, .. } => {
                if let Some(instance_ty_id) = types.get_instance_type_id(*symbol) {
                    candidates.push(instance_ty_id);
                }
            }
            Type::Union { elements } => {
                for element in elements {
                    self.collect_object_literal_candidates(*element, types, candidates);
                }
            }
            _ => {}
        }
    }

    /// Check if an object literal matches a target object type.
    fn object_literal_matches_target(
        &self,
        fields: &[ObjectLiteralField],
        target_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        let Type::Object {
            fields: target_fields,
            index_signatures,
            ..
        } = types.get_type(target_ty_id)
        else {
            return false;
        };
        if !index_signatures.is_empty() {
            return true;
        }

        for field in fields {
            let matches = target_fields
                .iter()
                .any(|target_field| target_field.key.matches(&field.field.key));
            if !matches {
                return false;
            }
        }

        true
    }

    /// Find the first excess property key for a target object type.
    fn object_literal_excess_properties(
        &self,
        fields: &[ObjectLiteralField],
        target_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Vec<(LocalNodeId<Property>, StaticKey)> {
        let Type::Object {
            fields: target_fields,
            index_signatures,
            ..
        } = types.get_type(target_ty_id)
        else {
            return Vec::new();
        };
        if !index_signatures.is_empty() {
            return Vec::new();
        }

        let mut excess_fields = Vec::new();
        for field in fields {
            let matches = target_fields
                .iter()
                .any(|target_field| target_field.key.matches(&field.field.key));
            if !matches {
                excess_fields.push((field.property_id, field.field.key));
            }
        }

        excess_fields
    }
}
