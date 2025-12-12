use crate::{AnalyzeError, AnalyzeResult, Assignability, Compiler, InferContext};
use destack_dir::{
    Argument, Block, Declaration, Declarator, DependencyItem, DynamicKey, EnumField, Expression,
    Extension, ExtensionKind, FunctionSignature, Generics, GlobalSymbolId, GlobalTypeId, Heritage,
    Lineage, LocalNodeId, LocalNodeIdAny, LocalSymbolId, LocalTypeId, MatchCase, MatchSource,
    Member, NodeTree, Parameter, Pattern, PatternField, PrimitiveType, Property, StaticKey,
    SymbolTable, Type, TypeField, TypeKind, TypeLiteral, TypeTable, WhereClause,
};
use destack_workspace::Module;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Infer the type of an expression.
    pub(super) fn infer_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        if let Some(ty_id) = types.get_inferred_type_id(expression_id.into_global_any(module.id)) {
            return Ok(ty_id);
        }

        let expression = tree.get(expression_id);
        let ty_id: LocalTypeId = match expression {
            // declaration -> analyze the declaration
            Expression::Declaration { declaration } => {
                self.infer_declaration(module, *declaration, tree, symbols, types, ctx)?;
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // block -> analyze the block
            Expression::Block { block } => {
                self.infer_block(module, *block, tree, symbols, types, ctx)?
            }

            // statement -> analyze the statement
            Expression::Statement { statement } => {
                self.infer_expression(module, *statement, tree, symbols, types, ctx)?;
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
                self.infer_expression(module, *body_id, tree, symbols, types, ctx)?;
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
                    self.infer_dependency_item(module, *item_id, tree, symbols, types, ctx)?;
                }
                if let Some(arguments) = arguments {
                    for argument_id in arguments {
                        self.infer_argument(module, *argument_id, None, tree, symbols, types, ctx)?;
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
                    self.infer_dependency_item(module, *item_id, tree, symbols, types, ctx)?;
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
                    self.infer_expression(module, *right, tree, symbols, types, ctx)?;
                let ty = self.infer_type_unary_operation(operator, right_ty_id, types);
                types.insert_type_from(ty, expression_id)
            }
            Expression::TypeBinary {
                left,
                operator,
                right,
            } => {
                let left_ty_id = self.infer_expression(module, *left, tree, symbols, types, ctx)?;
                let right_ty_id =
                    self.infer_expression(module, *right, tree, symbols, types, ctx)?;
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
                    self.infer_expression(module, *right, tree, symbols, types, ctx)?;
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
                    self.infer_expression(module, *right, tree, symbols, types, ctx)?;
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
                    self.infer_expression(module, *right, tree, symbols, types, ctx)?;
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
                let left_ty_id = self.infer_expression(module, *left, tree, symbols, types, ctx)?;
                let right_ty_id =
                    self.infer_expression(module, *right, tree, symbols, types, ctx)?;

                let ty = self.infer_binary_operation(
                    operator,
                    types.get_type(left_ty_id),
                    types.get_type(right_ty_id),
                );
                types.insert_type_from(ty, expression_id)
            }

            // assignment operations -> void
            Expression::Assign { left, right } => {
                let left_ty_id = self.infer_expression(module, *left, tree, symbols, types, ctx)?;
                let right_ty_id =
                    self.infer_expression(module, *right, tree, symbols, types, ctx)?;

                // type check: right must be assignable to left
                if self.check_is_type_assignable(left_ty_id, right_ty_id, types)
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
                    self.infer_expression(module, *left, tree, symbols, types, ctx)?;
                let _right_ty_id =
                    self.infer_expression(module, *right, tree, symbols, types, ctx)?;
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // delete operation -> void
            Expression::Delete { value } => {
                let _value_ty_id =
                    self.infer_expression(module, *value, tree, symbols, types, ctx)?;
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
                let ty = Type::TypeLiteral {
                    value: self.infer_scalar_literal(value),
                };
                types.insert_type_from(ty, expression_id)
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
                for element_id in elements {
                    self.infer_argument(module, *element_id, None, tree, symbols, types, ctx)?;
                }
                let element_tys: Vec<LocalTypeId> = elements
                    .iter()
                    .map(|element_id| {
                        let element = tree.get(*element_id);
                        let element_id = element.value();
                        self.infer_expression(module, element_id, tree, symbols, types, ctx)
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
                for expr_id in expressions {
                    last_ty =
                        Some(self.infer_expression(module, *expr_id, tree, symbols, types, ctx)?);
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
            } => self.infer_expression(module, *inner_id, tree, symbols, types, ctx)?,

            // object expression -> object type
            Expression::ObjectExpression { properties } => {
                let mut fields = Vec::new();
                for property_id in properties {
                    if let Some(field) =
                        self.infer_property(module, *property_id, tree, symbols, types, ctx)?
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
                    self.infer_expression(module, *left, tree, symbols, types, ctx)?;

                // analyze arguments
                let mut argument_ty_ids = Vec::with_capacity(dynamic_arguments.len());
                for arg in dynamic_arguments {
                    self.infer_argument(module, *arg, None, tree, symbols, types, ctx)?;
                    let arg_expr = tree.get(*arg);
                    let arg_value_id = arg_expr.value();
                    let argument_ty_id =
                        self.infer_expression(module, arg_value_id, tree, symbols, types, ctx)?;
                    argument_ty_ids.push(argument_ty_id);
                }

                // check callee type and get return type
                let callee_ty = types.get_type(callee_ty_id);
                match callee_ty {
                    Type::Function {
                        dynamic_parameters,
                        return_type,
                        ..
                    } => {
                        // type check arguments against parameters
                        let param_types = dynamic_parameters.clone();
                        let return_type = *return_type;
                        for (i, (argument_ty_id, param_ty_id)) in
                            argument_ty_ids.iter().zip(param_types.iter()).enumerate()
                        {
                            if self.check_is_type_assignable(*param_ty_id, *argument_ty_id, types)
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
                let left_ty_id = self.infer_expression(module, *left, tree, symbols, types, ctx)?;
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
                    self.infer_expression(module, *left, tree, symbols, types, ctx)?;
                if let Some(right) = right {
                    self.infer_expression(module, *right, tree, symbols, types, ctx)?;
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
                    self.infer_expression(module, *left, tree, symbols, types, ctx)?;
                for arg in dynamic_arguments {
                    self.infer_argument(module, *arg, None, tree, symbols, types, ctx)?;
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
                self.infer_expression(module, *condition, tree, symbols, types, ctx)?;
                let then_ty_id =
                    self.infer_expression(module, *then_expression, tree, symbols, types, ctx)?;
                if let Some(else_expr) = else_expression {
                    let _else_ty_id =
                        self.infer_expression(module, *else_expr, tree, symbols, types, ctx)?;
                    // #Incomplete: compute union or common type of then/else branches
                }
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
                    self.infer_expression(module, *cond, tree, symbols, types, ctx)?;
                }
                let mut ctx = ctx.fork().in_loop();
                self.infer_block(module, *body, tree, symbols, types, &mut ctx)?;
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
                    self.infer_expression(module, *iterator, tree, symbols, types, ctx)?;
                self.infer_pattern(
                    module,
                    *pattern,
                    Some(iterator_ty_id),
                    tree,
                    symbols,
                    types,
                    ctx,
                )?;
                let mut ctx = ctx.fork().in_loop();
                self.infer_block(module, *body, tree, symbols, types, &mut ctx)?;
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
                if let Some(init) = initialization {
                    self.infer_expression(module, *init, tree, symbols, types, ctx)?;
                }
                if let Some(cond) = condition {
                    self.infer_expression(module, *cond, tree, symbols, types, ctx)?;
                }
                if let Some(incr) = increment {
                    self.infer_expression(module, *incr, tree, symbols, types, ctx)?;
                }
                let mut ctx = ctx.fork().in_loop();
                self.infer_block(module, *body, tree, symbols, types, &mut ctx)?;
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
                    self.infer_expression(module, *value, tree, symbols, types, ctx)?;
                let mut ctx = if *source == MatchSource::Match {
                    ctx.fork().in_switch()
                } else {
                    ctx.fork()
                };
                let mut result_ty_id = None;
                for case_id in cases {
                    let case = tree.get(*case_id);
                    let (pattern, guard, body_expr) = match case {
                        MatchCase::Expression {
                            pattern,
                            body,
                            guard,
                            scope: _,
                        } => (*pattern, *guard, Some(*body)),
                        MatchCase::Block {
                            pattern,
                            body,
                            guard,
                            scope: _,
                        } => {
                            self.infer_block(module, *body, tree, symbols, types, &mut ctx)?;
                            (*pattern, *guard, None)
                        }
                    };
                    self.infer_pattern(
                        module,
                        pattern,
                        Some(value_ty_id),
                        tree,
                        symbols,
                        types,
                        &mut ctx,
                    )?;
                    if let Some(guard_expr) = guard {
                        self.infer_expression(module, guard_expr, tree, symbols, types, &mut ctx)?;
                    }
                    if let Some(expr) = body_expr {
                        let case_ty_id =
                            self.infer_expression(module, expr, tree, symbols, types, &mut ctx)?;
                        if result_ty_id.is_none() {
                            result_ty_id = Some(case_ty_id);
                        }
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
                    self.infer_expression(module, *try_expression, tree, symbols, types, ctx)?;
                if let Some(catch_pat) = catch_pattern {
                    self.infer_pattern(module, *catch_pat, None, tree, symbols, types, ctx)?;
                }
                if let Some(catch_expr) = catch_expression {
                    self.infer_expression(module, *catch_expr, tree, symbols, types, ctx)?;
                }
                if let Some(finally_expr) = finally_expression {
                    self.infer_expression(module, *finally_expr, tree, symbols, types, ctx)?;
                }
                try_ty_id
            }

            // return -> never (control flow)
            Expression::Return { value } => {
                if let Some(val) = value {
                    self.infer_expression(module, *val, tree, symbols, types, ctx)?;
                }
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Never,
                };
                types.insert_type_from(ty, expression_id)
            }

            // break/continue -> never (control flow)
            Expression::Break { target: _, value }
            | Expression::UnresolvedBreak { target: _, value } => {
                if let Some(val) = value {
                    self.infer_expression(module, *val, tree, symbols, types, ctx)?;
                }
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Never,
                };
                types.insert_type_from(ty, expression_id)
            }
            Expression::Continue { target: _ } | Expression::UnresolvedContinue { target: _ } => {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Never,
                };
                types.insert_type_from(ty, expression_id)
            }

            // throw -> never (control flow)
            Expression::Throw { value } => {
                if let Some(val) = value {
                    self.infer_expression(module, *val, tree, symbols, types, ctx)?;
                }
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Never,
                };
                types.insert_type_from(ty, expression_id)
            }

            // await -> unwrapped promise type
            Expression::Await { expression } => {
                let _inner_ty_id =
                    self.infer_expression(module, *expression, tree, symbols, types, ctx)?;
                // #Incomplete: unwrap Promise type
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // yield -> yielded type
            Expression::Yield {
                cardinality: _,
                value,
            } => {
                if let Some(value_id) = value {
                    self.infer_expression(module, *value_id, tree, symbols, types, ctx)?;
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
                    self.infer_expression(module, *left, tree, symbols, types, ctx)?;
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
                let _tag_ty_id = self.infer_expression(module, *tag, tree, symbols, types, ctx)?;
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
                self.infer_expression(module, *start, tree, symbols, types, ctx)?;
                self.infer_expression(module, *end, tree, symbols, types, ctx)?;
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // tagged expressions for newtype construction
            Expression::TaggedScalarExpression { ty, value } => {
                let ty_id = self.infer_expression(module, *ty, tree, symbols, types, ctx)?;
                self.infer_expression(module, *value, tree, symbols, types, ctx)?;
                ty_id
            }
            Expression::TaggedTupleExpression { ty, elements } => {
                let ty_id = self.infer_expression(module, *ty, tree, symbols, types, ctx)?;
                for elem in elements {
                    self.infer_argument(module, *elem, None, tree, symbols, types, ctx)?;
                }
                ty_id
            }
            Expression::TaggedObjectExpression { ty, properties } => {
                let ty_id = self.infer_expression(module, *ty, tree, symbols, types, ctx)?;
                for prop_id in properties {
                    self.infer_property(module, *prop_id, tree, symbols, types, ctx)?;
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
                    self.infer_expression(module, *left, tree, symbols, types, ctx)?;
                }
                if let Some(args) = arguments {
                    for arg in args {
                        self.infer_argument(module, *arg, None, tree, symbols, types, ctx)?;
                    }
                }
                if let Some(elems) = elements {
                    for elem in elems {
                        self.infer_argument(module, *elem, None, tree, symbols, types, ctx)?;
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

    /// Analyze a block.
    fn infer_block(
        &self,
        module: &Module,
        block_id: LocalNodeId<Block>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        if let Some(ty_id) = types.get_inferred_type_id(block_id.into_global_any(module.id)) {
            return Ok(ty_id);
        }

        let block = tree.get(block_id);
        for expression_id in &block.expressions {
            self.infer_expression(module, *expression_id, tree, symbols, types, ctx)?;
        }

        // type is last expression type
        let ty_id = if let Some(last_expression_id) = block.expressions.last() {
            self.infer_expression(module, *last_expression_id, tree, symbols, types, ctx)?
        } else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            types.insert_type_from(ty, block_id)
        };

        types.set_inferred_type(block_id.into_global_any(module.id), ty_id);

        Ok(ty_id)
    }

    /// Analyze a declaration.
    fn infer_declaration(
        &self,
        module: &Module,
        declaration_id: LocalNodeId<Declaration>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
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
                self.infer_generics(module, generics, tree, symbols, types, ctx)?;
                for expression_id in expressions {
                    self.infer_expression(module, *expression_id, tree, symbols, types, ctx)?;
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
                        self.infer_parameter(module, *parameter_id, tree, symbols, types, ctx)?;
                    }
                }

                // type instance type -> type value
                let instance_ty_id =
                    self.infer_expression(module, *value, tree, symbols, types, ctx)?;
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
                self.infer_generics(module, generics, tree, symbols, types, ctx)?;
                self.infer_heritage(
                    module,
                    heritage,
                    declaration_id.into_any(),
                    Some(descriptor.symbol),
                    tree,
                    symbols,
                    types,
                    ctx,
                )?;

                // type fields
                let mut fields = Vec::new();
                for member_id in members {
                    if let Some(field) =
                        self.infer_member(module, *member_id, tree, symbols, types, ctx)?
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
                self.infer_generics(module, generics, tree, symbols, types, ctx)?;
                self.infer_heritage(
                    module,
                    heritage,
                    declaration_id.into_any(),
                    Some(descriptor.symbol),
                    tree,
                    symbols,
                    types,
                    ctx,
                )?;

                // type fields
                let mut fields = Vec::new();
                for member_id in members {
                    if let Some(field) =
                        self.infer_member(module, *member_id, tree, symbols, types, ctx)?
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
                generics,
                heritage,
                scope: _,
                fields,
                members,
            } => {
                // walk
                self.infer_generics(module, generics, tree, symbols, types, ctx)?;
                self.infer_heritage(
                    module,
                    heritage,
                    declaration_id.into_any(),
                    Some(descriptor.symbol),
                    tree,
                    symbols,
                    types,
                    ctx,
                )?;
                for field_id in fields {
                    self.infer_enum_field(module, *field_id, tree, symbols, types, ctx)?;
                }
                for member_id in members {
                    self.infer_member(module, *member_id, tree, symbols, types, ctx)?;
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
                self.infer_generics(module, generics, tree, symbols, types, ctx)?;
                self.infer_expression(module, *target_type, tree, symbols, types, ctx)?;
                self.infer_heritage(
                    module,
                    heritage,
                    declaration_id.into_any(),
                    Some(descriptor.symbol), // (put lineage on extension symbol itself)
                    tree,
                    symbols,
                    types,
                    ctx,
                )?;

                // collect type fields from members
                let mut fields = Vec::new();
                for member_id in members {
                    if let Some(field) =
                        self.infer_member(module, *member_id, tree, symbols, types, ctx)?
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
                        ExtensionKind::Named
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
                generics,
                heritage,
                scope: _,
                members,
            } => {
                // walk
                self.infer_generics(module, generics, tree, symbols, types, ctx)?;
                self.infer_heritage(
                    module,
                    heritage,
                    declaration_id.into_any(),
                    Some(descriptor.symbol),
                    tree,
                    symbols,
                    types,
                    ctx,
                )?;

                // collect type fields from members
                let mut fields = Vec::new();
                for member_id in members {
                    if let Some(field) =
                        self.infer_member(module, *member_id, tree, symbols, types, ctx)?
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
                    signature,
                    tree,
                    symbols,
                    types,
                    ctx,
                )?;
                // register the function type as the value_type for the function's symbol
                types.set_value_type(descriptor.symbol.into_global(module.id), fn_ty_id);

                if let Some(body) = body {
                    let mut ctx = ctx.reset();
                    self.infer_expression(module, *body, tree, symbols, types, &mut ctx)?;
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
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
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

                // infer the value type
                let value_ty_id = if let Some(value) = value {
                    self.infer_expression(module, *value, tree, symbols, types, ctx)?
                } else {
                    // no value, return unknown type
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    types.insert_type(ty)
                };

                // analyze default if present
                if let Some(default) = default {
                    self.infer_expression(module, *default, tree, symbols, types, ctx)?;
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
            Property::Method { body, .. } => {
                // #Incomplete: infer method type
                if let Some(body) = body {
                    let mut ctx = ctx.reset();
                    self.infer_expression(module, *body, tree, symbols, types, &mut ctx)?;
                }
                Ok(None)
            }
            Property::Spread { value, .. } => {
                // #Incomplete: expand spread type into object type
                self.infer_expression(module, *value, tree, symbols, types, ctx)?;
                Ok(None)
            }
        }
    }

    /// Analyze a member and return its TypeField if it has a static key.
    fn infer_member(
        &self,
        module: &Module,
        member_id: LocalNodeId<Member>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
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
                    self.infer_expression(module, *value, tree, symbols, types, ctx)?
                } else {
                    // no value, return unknown type
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    types.insert_type(ty)
                };

                // analyze default if present
                if let Some(default) = default {
                    self.infer_expression(module, *default, tree, symbols, types, ctx)?;
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
                    signature,
                    tree,
                    symbols,
                    types,
                    ctx,
                )?;

                // body
                if let Some(body) = body {
                    let mut ctx = ctx.reset();
                    self.infer_expression(module, *body, tree, symbols, types, &mut ctx)?;
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
                self.infer_expression(module, *value, tree, symbols, types, ctx)?;
                Ok(None)
            }
            Member::StaticBlock { body, .. } => {
                self.infer_expression(module, *body, tree, symbols, types, ctx)?;
                Ok(None)
            }
        }
    }

    /// Analyze generics.
    fn infer_generics(
        &self,
        module: &Module,
        generics: &Generics,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        if let Some(static_parameters) = &generics.static_parameters {
            for parameter_id in static_parameters {
                self.infer_parameter(module, *parameter_id, tree, symbols, types, ctx)?;
            }
        }
        if let Some(clauses) = &generics.where_clauses {
            for clause_id in clauses {
                self.infer_where_clause(module, *clause_id, tree, symbols, types, ctx)?;
            }
        }
        Ok(())
    }

    /// Analyze heritage.
    fn infer_heritage(
        &self,
        module: &Module,
        heritage: &Heritage,
        node_id: LocalNodeIdAny,
        symbol: Option<LocalSymbolId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        // analyze and extract extends symbols
        let mut extends_symbols = Vec::new();
        if let Some(extend_types) = &heritage.extends_types {
            for expression_id in extend_types {
                self.infer_expression(module, *expression_id, tree, symbols, types, ctx)?;
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
                self.infer_expression(module, *expression_id, tree, symbols, types, ctx)?;
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
                self.infer_expression(module, *expression_id, tree, symbols, types, ctx)?;
                let expression = tree.get(*expression_id);
                if let Some(target_symbol) = expression.target_symbol() {
                    embedded_symbols.push(target_symbol);
                }
            }
        }

        // build and store lineage if we have a declaring symbol
        if let Some(symbol) = symbol {
            // check lineage
            if extends_symbols.len() > 1 {
                self.error(AnalyzeError::InvalidLineage {
                    node: node_id.into_global(module.id),
                    extends_symbols: extends_symbols.clone(),
                    implements_symbols: implements_symbols.clone(),
                    embedded_symbols: embedded_symbols.clone(),
                });
            }

            // remember lineage
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
        signature: &FunctionSignature,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // walk generics
        if let Some(generics) = &signature.generics {
            self.infer_generics(module, generics, tree, symbols, types, ctx)?;
        }

        // collect parameter types
        let mut dynamic_param_types = Vec::with_capacity(signature.dynamic_parameters.len());
        for parameter_id in &signature.dynamic_parameters {
            self.infer_parameter(module, *parameter_id, tree, symbols, types, ctx)?;
            // get the declared type for this parameter
            let param_ty_id = types
                .get_declared_type_id(parameter_id.into_global_any(module.id))
                .unwrap_or_else(|| {
                    // no declared type: infer unknown
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    types.insert_type_from(ty, *parameter_id)
                });
            dynamic_param_types.push(param_ty_id);
        }

        // get return type
        let return_type = if let Some(return_type_expr_id) = signature.return_type {
            let return_ty_id =
                self.infer_expression(module, return_type_expr_id, tree, symbols, types, ctx)?;
            // unwrap Value type if present
            let return_ty = types.get_type(return_ty_id);
            match return_ty {
                Type::Value { value } => Some(*value),
                _ => Some(return_ty_id),
            }
        } else {
            None
        };

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

    /// Analyze a parameter.
    fn infer_parameter(
        &self,
        module: &Module,
        parameter_id: LocalNodeId<Parameter>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let parameter = tree.get(parameter_id);
        match parameter {
            Parameter::Named {
                modifiers: _,
                name: _,
                default,
                symbol: _,
            } => {
                if let Some(default) = default {
                    self.infer_expression(module, *default, tree, symbols, types, ctx)?;
                }
            }
            Parameter::Pattern {
                modifiers: _,
                pattern,
                default,
                symbol: _,
            } => {
                // infer type from default if present
                let default_ty_id = if let Some(default) = default {
                    Some(self.infer_expression(module, *default, tree, symbols, types, ctx)?)
                } else {
                    None
                };
                self.infer_pattern(module, *pattern, default_ty_id, tree, symbols, types, ctx)?;
            }
            Parameter::Variadic {
                modifiers: _,
                name: _,
                symbol: _,
            } => {
                // nothing to do
            }
        }
        Ok(())
    }

    /// Infer an argument.
    fn infer_argument(
        &self,
        module: &Module,
        argument_id: LocalNodeId<Argument>,
        _binding_ty_id: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let argument = tree.get(argument_id);
        match argument {
            Argument::Positional { value } => {
                self.infer_expression(module, *value, tree, symbols, types, ctx)?;
            }
            Argument::Named { name: _, value } => {
                self.infer_expression(module, *value, tree, symbols, types, ctx)?;
            }
            Argument::Labeled { label: _, value } => {
                self.infer_expression(module, *value, tree, symbols, types, ctx)?;
            }
            Argument::Spread { value } => {
                self.infer_expression(module, *value, tree, symbols, types, ctx)?;
            }
            Argument::Dynamic { key, value } => {
                self.infer_expression(module, *key, tree, symbols, types, ctx)?;
                self.infer_expression(module, *value, tree, symbols, types, ctx)?;
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
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let clause = tree.get(clause_id);
        match clause {
            WhereClause::Assertion { left: _, right } => {
                self.infer_expression(module, *right, tree, symbols, types, ctx)?;
            }
            WhereClause::Guard { guard } => {
                self.infer_expression(module, *guard, tree, symbols, types, ctx)?;
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
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let field = tree.get(field_id);
        if let Some(value) = field.value {
            self.infer_expression(module, value, tree, symbols, types, ctx)?;
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
                    ctx,
                )?;
            }
            Pattern::ReferenceOf {
                mutability: _,
                right,
            } => {
                self.infer_pattern(module, *right, binding_ty_id, tree, symbols, types, ctx)?;
            }
            Pattern::ValueOf {
                mutability: _,
                right,
            } => {
                self.infer_pattern(module, *right, binding_ty_id, tree, symbols, types, ctx)?;
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
                        ctx,
                    )?;
                }
            }
            Pattern::Expression { value } => {
                self.infer_expression(module, *value, tree, symbols, types, ctx)?;
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
                    ctx,
                )?;
            }
            Pattern::TaggedTuple { ty, fields } => {
                let ty_id = self.infer_expression(module, *ty, tree, symbols, types, ctx)?;
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
                        ctx,
                    )?;
                }
            }
            Pattern::TaggedObject { ty, fields } => {
                let ty_id = self.infer_expression(module, *ty, tree, symbols, types, ctx)?;
                for field_id in fields {
                    self.infer_pattern_field(
                        module,
                        *field_id,
                        Some(ty_id),
                        tree,
                        symbols,
                        types,
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
                        ctx,
                    )?;
                }
            }
        }

        Ok(())
    }

    /// Analyze a sequence of pattern fields (with spread syntax support).
    fn infer_pattern_sequence(
        &self,
        module: &Module,
        fields: &Vec<LocalNodeId<PatternField>>,
        binding_ty_id: Option<LocalTypeId>,
        to_rest_type: impl Fn(Vec<LocalTypeId>) -> Type,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
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
            self.infer_pattern_field(module, *field_id, field_ty, tree, symbols, types, ctx)?;
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
                    self.infer_expression(module, *default, tree, symbols, types, ctx)?;
                }
            }
            PatternField::Positional { pattern } => {
                self.infer_pattern(module, *pattern, binding_ty_id, tree, symbols, types, ctx)?;
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
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let declarator = tree.get(declarator_id);
        let Declarator { pattern, ty, value } = declarator;

        // infer type from value or annotation
        // declared type is now on the declarator node, not the let expression
        let declared_ty_id =
            types.get_declared_type_id(declarator_id.into_global(module.id).into());
        let inferred_ty_id = if let Some(value) = value {
            Some(self.infer_expression(module, *value, tree, symbols, types, ctx)?)
        } else {
            None
        };

        // type check: if both declared and inferred, check assignability
        if let (Some(declared), Some(inferred)) = (declared_ty_id, inferred_ty_id)
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

        // analyze the type expression if present
        if let Some(ty_id) = ty {
            self.infer_expression(module, *ty_id, tree, symbols, types, ctx)?;
        }

        let binding_ty_id = declared_ty_id.or(inferred_ty_id);
        self.infer_pattern(module, *pattern, binding_ty_id, tree, symbols, types, ctx)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use destack_dir::{Expression, PrimitiveType, ScalarLiteral, Type, TypeLiteral};

    use crate::{TestProgram, assert_type};

    #[test]
    fn test_analyze_number_literal() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module("test.ds", "42");
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let tree = module.dir.tree.read();
        let types = module.dir.types.read();

        let expression_id = module.dir.roots[0];
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
        let tree = module.dir.tree.read();
        let types = module.dir.types.read();

        let expression_id = module.dir.roots[0];
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
        let tree = module.dir.tree.read();
        let types = module.dir.types.read();

        let expression_id = module.dir.roots[0];
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
        let tree = module.dir.tree.read();
        let types = module.dir.types.read();

        let expression_id = module.dir.roots[0];
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
        let tree = module.dir.tree.read();
        let types = module.dir.types.read();

        let expression_id = module.dir.roots[0];
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
        let types = module.dir.types.read();

        let let_expr_id = module.dir.roots[0];
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
        let tree = module.dir.tree.read();
        let types = module.dir.types.read();

        let expression_id = module.dir.roots[0];
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
        let types = module.dir.types.read();

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
        let types = module.dir.types.read();

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
        let types = module.dir.types.read();

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
        let types = module.dir.types.read();

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
        let types = module.dir.types.read();

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

    /// Verify circular imports work during analysis.
    // nocheckin #Broken: fix this, make TypeTable access/computations granular (no &mut TypeTable)
    #[test]
    #[ignore]
    fn test_analyze_circular_type_dependency() {
        let test = TestProgram::memory_sequential();
        test.add_file(
            "a.ds",
            r#"
import { B } from "./b.ds";

export struct A { value: B }

export function helperA(): A { throw "not implemented" }
"#,
        );
        test.add_file(
            "b.ds",
            r#"
import { A } from "./a.ds";

export struct B { value: A }

export function helperB(): B { throw "not implemented" }
"#,
        );
        let module_id = test.add_module(
            "main.ds",
            r#"
import { A, helperA } from "./a.ds";
import { B, helperB } from "./b.ds";

declare const a: A = helperA();
declare const b: B = helperB();
"#,
        );
        test.analyze_module(module_id);
        test.compile_dump_clean();

        let a_symbol_id = test.resolve_to_symbol("a.ds", "A").unwrap();
        let b_symbol_id = test.resolve_to_symbol("b.ds", "B").unwrap();

        // struct A { value: B } -> typeOf(A.value) should point to canonical B
        let a_module = test.program.modules.get(a_symbol_id.module_id);
        let a_module = a_module.read();
        let a_types = a_module.dir.types.read();
        let a_ty_id = a_types.get_value_type_id(a_symbol_id).unwrap();
        assert_type!(a_types, a_ty_id, Type::Object { fields } => {
            assert_eq!(fields.len(), 1);
            assert_type!(a_types, fields[0].ty, Type::Reference { symbol, static_arguments: _ } => {
                assert_eq!(*symbol, b_symbol_id);
            });
        });

        // struct B { value: A } -> typeOf(B.value) should point to canonical A
        let b_module = test.program.modules.get(b_symbol_id.module_id);
        let b_module = b_module.read();
        let b_types = b_module.dir.types.read();
        let b_ty_id = b_types.get_value_type_id(b_symbol_id).unwrap();
        assert_type!(b_types, b_ty_id, Type::Object { fields } => {
            assert_eq!(fields.len(), 1);
            assert_type!(b_types, fields[0].ty, Type::Reference { symbol, static_arguments: _ } => {
                assert_eq!(*symbol, a_symbol_id);
            });
        });
    }
}
