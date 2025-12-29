use destack_ast::{self as ast};
use destack_base::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;
use smallvec::smallvec;

use super::UnbindContext;
use crate::Compiler;

impl Compiler {
    destack_base::ensure_sufficient_stack! {
        /// Unbind a DIR expression to an AST expression.
        pub fn unbind_expression(
            &self,
            module: &Module,
            expression_id: dir::LocalNodeId<dir::Expression>,
            tree: &dir::NodeTree,
            symbols: &dir::SymbolTable,
            ast_tree: &mut ast::NodeTree,
            ast_strings: &mut StringPool,
            context: &mut UnbindContext,
        ) -> ast::LocalNodeId<ast::Expression> {
            let expression = tree.get(expression_id);
            let span = self.unbind_span(module, expression_id.into());

            let ast_expression = match expression {
                dir::Expression::Declaration { declaration } => {
                    let declaration = self.unbind_declaration(module, *declaration, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::Declaration(declaration)
                }

                dir::Expression::Block { block } => {
                    let block = self.unbind_block(module, *block, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::Block(block)
                }

                dir::Expression::Statement { statement } => {
                    let statement = self.unbind_expression(module, *statement, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::Statement(statement)
                }

                dir::Expression::Labelled { label, body, .. } => {
                    let label = ast_strings.intern_from(&self.program.strings, *label);
                    let body = self.unbind_expression(module, *body, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::Labelled { label, body }
                }

                dir::Expression::UnresolvedImport { kind, target, items, arguments }
                | dir::Expression::Import { kind, target, items, arguments, .. } => {
                    let kind = self.unbind_dependency_kind(context, *kind);
                    let target = ast_strings.intern_from(&self.program.strings, *target);
                    let items = items.iter().map(|item| {
                        self.unbind_dependency_item(module, *item, tree, symbols, ast_tree, ast_strings, context)
                    }).collect();
                    let arguments = arguments.as_ref().map(|args| {
                        args.iter().map(|arg| {
                            self.unbind_argument(module, *arg, tree, symbols, ast_tree, ast_strings, context)
                        }).collect()
                    });
                    ast::Expression::Import { kind, target, items, arguments }
                }

                dir::Expression::UnresolvedReExport { target, kind, items }
                | dir::Expression::ReExport { target, kind, items, .. } => {
                    let kind = self.unbind_dependency_kind(context, *kind);
                    let target = ast_strings.intern_from(&self.program.strings, *target);
                    let items = items.iter().map(|item| {
                        self.unbind_dependency_item(module, *item, tree, symbols, ast_tree, ast_strings, context)
                    }).collect();
                    ast::Expression::Export { kind, target: Some(target), items }
                }

                dir::Expression::Export { kind, items } => {
                    let kind = self.unbind_dependency_kind(context, *kind);
                    let items = items.iter().map(|item| {
                        self.unbind_dependency_item(module, *item, tree, symbols, ast_tree, ast_strings, context)
                    }).collect();
                    ast::Expression::Export { kind, target: None, items }
                }

                dir::Expression::Let { descriptor, mutability, declarators } => {
                    let descriptor = self.unbind_declaration_descriptor(descriptor, ast_strings, context);
                    let ast_mutability = self.unbind_mutability(context, *mutability);
                    // Derive LetKind from mutability (DIR doesn't preserve original keyword)
                    let kind = match mutability {
                        dir::Mutability::Immutable => ast::LetKind::Const,
                        dir::Mutability::Mutable => ast::LetKind::Let,
                    };
                    let declarators = declarators.iter().map(|decl| {
                        self.unbind_declarator(module, *decl, tree, symbols, ast_tree, ast_strings, context)
                    }).collect();
                    ast::Expression::Let { kind, descriptor, mutability: ast_mutability, declarators }
                }
                dir::Expression::Using {
                    asynchrony,
                    descriptor,
                    declarators,
                } => {
                    let asynchrony = self.unbind_asynchrony(context, *asynchrony);
                    let descriptor = self.unbind_declaration_descriptor(descriptor, ast_strings, context);
                    let declarators = declarators
                        .iter()
                        .map(|decl| {
                        self.unbind_declarator(
                            module,
                            *decl,
                            tree,
                            symbols,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                        })
                        .collect();
                    ast::Expression::Using {
                        asynchrony,
                        descriptor,
                        declarators,
                    }
                }

                dir::Expression::TypeUnary { operator, right } => {
                    let operator = self.unbind_type_unary_operator(context, *operator);
                    let right = self.unbind_expression(module, *right, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::TypeUnary { operator, right }
                }

                dir::Expression::TypeBinary { left, operator, right } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, ast_tree, ast_strings, context);
                    let operator = self.unbind_type_binary_operator(context, *operator);
                    let right = self.unbind_expression(module, *right, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::TypeBinary { left, operator, right }
                }

                dir::Expression::TypeConditional {
                    left,
                    right,
                    then_type,
                    else_type,
                } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, ast_tree, ast_strings, context);
                    let right = self.unbind_expression(module, *right, tree, symbols, ast_tree, ast_strings, context);
                    let then_type = self.unbind_expression(module, *then_type, tree, symbols, ast_tree, ast_strings, context);
                    let else_type = self.unbind_expression(module, *else_type, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::TypeConditional {
                        left,
                        right,
                        then_type,
                        else_type,
                    }
                }

                dir::Expression::TypeMapped {
                    parameter,
                    modifiers,
                    value,
                } => {
                    let name = ast_strings.intern_from(&self.program.strings, parameter.name);
                    let constraint = self.unbind_expression(
                        module,
                        parameter.constraint,
                        tree,
                        symbols,
                        ast_tree,
                        ast_strings,
                        context,
                    );
                    let key_remap = parameter.key_remap.map(|key_remap| {
                        self.unbind_expression(
                            module,
                            key_remap,
                            tree,
                            symbols,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    });
                    let parameter = ast::TypeMappedParameter {
                        name,
                        constraint,
                        key_remap,
                    };
                    let modifiers = self.unbind_type_mapped_modifiers(context, *modifiers);
                    let value = self.unbind_expression(module, *value, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::TypeMapped {
                        parameter,
                        modifiers,
                        value,
                    }
                }

                dir::Expression::TypeIndex { left, index } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, ast_tree, ast_strings, context);
                    let index = self.unbind_expression(module, *index, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::TypeIndex { left, index }
                }

                dir::Expression::TypeTemplateLiteral { strings, spans } => {
                    let strings = strings
                        .iter()
                        .map(|string| ast_strings.intern_from(&self.program.strings, *string))
                        .collect();
                    let spans = spans
                        .iter()
                        .map(|span| {
                            self.unbind_expression(module, *span, tree, symbols, ast_tree, ast_strings, context)
                        })
                        .collect();
                    ast::Expression::TypeTemplateLiteral { strings, spans }
                }

                dir::Expression::TypeImport { target, qualifier } => {
                    let target = ast_strings.intern_from(&self.program.strings, *target);
                    let qualifier = qualifier.as_ref().map(|qualifier| {
                        self.unbind_path(qualifier, ast_strings, context)
                    });
                    ast::Expression::TypeImport { target, qualifier }
                }

                dir::Expression::TypeInfer { name, constraint } => {
                    let name = ast_strings.intern_from(&self.program.strings, *name);
                    let constraint = constraint.map(|constraint| {
                        self.unbind_expression(
                            module,
                            constraint,
                            tree,
                            symbols,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    });
                    ast::Expression::TypeInfer { name, constraint }
                }

                dir::Expression::TypePredicate {
                    asserts,
                    subject,
                    target,
                } => {
                    let subject =
                        self.unbind_type_predicate_subject(*subject, module, symbols, ast_strings, context);
                    let target = target.map(|target| {
                        self.unbind_expression(
                            module,
                            target,
                            tree,
                            symbols,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    });
                    ast::Expression::TypePredicate {
                        asserts: *asserts,
                        subject,
                        target,
                    }
                }

                dir::Expression::Unary { operator, right } => {
                    let operator = self.unbind_unary_operator(context, *operator);
                    let right = self.unbind_expression(module, *right, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::Unary { operator, right }
                }

                dir::Expression::ValueOf { mutability, variance, right } => {
                    let mutability = mutability.map(|m| self.unbind_mutability(context, m));
                    let variance = variance.map(|v| self.unbind_variance_bound(context, v));
                    let right = self.unbind_expression(module, *right, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::ValueOf { mutability, variance, right }
                }

                dir::Expression::ReferenceOf { mutability, variance, right } => {
                    let mutability = mutability.map(|m| self.unbind_mutability(context, m));
                    let variance = variance.map(|v| self.unbind_variance_bound(context, v));
                    let right = self.unbind_expression(module, *right, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::ReferenceOf { mutability, variance, right }
                }

                dir::Expression::Binary { left, operator, right } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, ast_tree, ast_strings, context);
                    let operator = self.unbind_binary_operator(context, *operator);
                    let right = self.unbind_expression(module, *right, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::Binary { left, operator, right }
                }

                dir::Expression::Assign { left, right } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, ast_tree, ast_strings, context);
                    let right = self.unbind_expression(module, *right, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::Assign { left, operator: ast::AssignOperator::Assign, right }
                }

                dir::Expression::AssignBinary { left, operator, right } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, ast_tree, ast_strings, context);
                    let operator = self.unbind_assign_operator(context, *operator);
                    let right = self.unbind_expression(module, *right, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::Assign { left, operator, right }
                }

                dir::Expression::Member { left, name, static_arguments } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, ast_tree, ast_strings, context);
                    let name = ast_strings.intern_from(&self.program.strings, *name);
                    let static_arguments = static_arguments.as_ref().map(|args| {
                        args.iter().map(|arg| {
                            self.unbind_argument(module, *arg, tree, symbols, ast_tree, ast_strings, context)
                        }).collect()
                    });
                    ast::Expression::Member { left, name, static_arguments }
                }

                dir::Expression::Call { left, static_arguments, dynamic_arguments } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, ast_tree, ast_strings, context);
                    let static_arguments = static_arguments.as_ref().map(|args| {
                        args.iter().map(|arg| {
                            self.unbind_argument(module, *arg, tree, symbols, ast_tree, ast_strings, context)
                        }).collect()
                    });
                    let dynamic_arguments = dynamic_arguments.iter().map(|arg| {
                        self.unbind_argument(module, *arg, tree, symbols, ast_tree, ast_strings, context)
                    }).collect();
                    ast::Expression::Call {
                        position: ast::PostfixPosition::Direct,
                        left,
                        static_arguments,
                        dynamic_arguments,
                    }
                }

                dir::Expression::Index { left, right } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, ast_tree, ast_strings, context);
                    let index = right.map(|r| self.unbind_expression(module, r, tree, symbols, ast_tree, ast_strings, context));
                    ast::Expression::Index {
                        position: ast::PostfixPosition::Direct,
                        left,
                        index,
                    }
                }

                dir::Expression::Maybe { left } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::Maybe {
                        position: ast::PostfixPosition::Direct,
                        left,
                    }
                }

                dir::Expression::Must { left } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::Must {
                        position: ast::PostfixPosition::Direct,
                        left,
                    }
                }

                dir::Expression::New { left, static_arguments, dynamic_arguments } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, ast_tree, ast_strings, context);
                    let static_arguments = static_arguments.as_ref().map(|args| {
                        args.iter().map(|arg| {
                            self.unbind_argument(module, *arg, tree, symbols, ast_tree, ast_strings, context)
                        }).collect()
                    });
                    let dynamic_arguments = dynamic_arguments.iter().map(|arg| {
                        self.unbind_argument(module, *arg, tree, symbols, ast_tree, ast_strings, context)
                    }).collect();
                    ast::Expression::New { left, static_arguments, dynamic_arguments }
                }

                dir::Expression::Delete { value } => {
                    let value = self.unbind_expression(module, *value, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::Delete { value }
                }

                dir::Expression::UnresolvedPath { path, static_arguments }
                | dir::Expression::LocalReference { path, static_arguments, .. }
                | dir::Expression::ModuleReference { path, static_arguments, .. }
                | dir::Expression::GlobalReference { path, static_arguments, .. } => {
                    let path = self.unbind_path(path, ast_strings, context);
                    let static_arguments = static_arguments.as_ref().map(|args| {
                        args.iter().map(|arg| {
                            self.unbind_argument(module, *arg, tree, symbols, ast_tree, ast_strings, context)
                        }).collect()
                    });
                    ast::Expression::Path { path, static_arguments }
                }

                dir::Expression::ImportMeta => {
                    let import_id = ast_strings.intern("import");
                    let meta_id = ast_strings.intern("meta");
                    let path = ast::Path {
                        segments: smallvec![import_id, meta_id],
                    };
                    ast::Expression::Path {
                        path,
                        static_arguments: None,
                    }
                }
                dir::Expression::This => {
                    let this_id = ast_strings.intern("this");
                    let path = ast::Path {
                        segments: smallvec![this_id],
                    };
                    ast::Expression::Path {
                        path,
                        static_arguments: None,
                    }
                }

                dir::Expression::ScalarLiteral { value } => {
                    let value = self.unbind_scalar_literal(value, ast_strings, context);
                    ast::Expression::ScalarLiteral(value)
                }

                dir::Expression::TypeLiteral { value } => {
                    let value = self.unbind_type_literal(value, context);
                    ast::Expression::TypeLiteral(value)
                }

                dir::Expression::Type { .. } => {
                    // #Incomplete: unbind dir::Expression::Type?
                    ast::Expression::Error
                }

                dir::Expression::TemplateExpression { value } => {
                    let value = self.unbind_template_literal(module,value, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::TemplateExpression { value }
                }

                dir::Expression::TaggedTemplateExpression { tag, value } => {
                    let tag = self.unbind_expression(module, *tag, tree, symbols, ast_tree, ast_strings, context);
                    let value = self.unbind_template_literal(module,value, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::TaggedTemplateExpression { tag, value }
                }

                dir::Expression::RangeExpression { start, end, is_inclusive } => {
                    let start = self.unbind_expression(module, *start, tree, symbols, ast_tree, ast_strings, context);
                    let end = self.unbind_expression(module, *end, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::RangeExpression { start, end, is_inclusive: *is_inclusive }
                }

                dir::Expression::ArrayExpression { elements } => {
                    let elements = elements.iter().map(|el| {
                        self.unbind_argument(module, *el, tree, symbols, ast_tree, ast_strings, context)
                    }).collect();
                    ast::Expression::ArrayExpression { elements }
                }

                dir::Expression::TupleExpression { elements } => {
                    let elements = elements.iter().map(|el| {
                        self.unbind_argument(module, *el, tree, symbols, ast_tree, ast_strings, context)
                    }).collect();
                    ast::Expression::TupleExpression { elements }
                }

                dir::Expression::SequenceExpression { expressions } => {
                    let expressions = expressions.iter().map(|expr| {
                        self.unbind_expression(module, *expr, tree, symbols, ast_tree, ast_strings, context)
                    }).collect();
                    ast::Expression::SequenceExpression { expressions }
                }

                dir::Expression::ObjectExpression { properties } => {
                    let properties = properties.iter().map(|prop| {
                        self.unbind_property(module, *prop, tree, symbols, ast_tree, ast_strings, context)
                    }).collect();
                    ast::Expression::ObjectExpression { ty: None, properties }
                }

                dir::Expression::TreeExpression { left, arguments, elements } => {
                    let left = left.map(|l| self.unbind_expression(module, l, tree, symbols, ast_tree, ast_strings, context));
                    let arguments = arguments.as_ref().map(|args| {
                        args.iter().map(|arg| {
                            self.unbind_argument(module, *arg, tree, symbols, ast_tree, ast_strings, context)
                        }).collect()
                    });
                    let elements = elements.as_ref().map(|els| {
                        els.iter().map(|el| {
                            self.unbind_argument(module, *el, tree, symbols, ast_tree, ast_strings, context)
                        }).collect()
                    });
                    ast::Expression::TreeExpression { left, arguments, elements }
                }

                dir::Expression::TaggedScalarExpression { ty, value } => {
                    // emit as call: ty(value)
                    let ty = self.unbind_expression(module, *ty, tree, symbols, ast_tree, ast_strings, context);
                    let value = self.unbind_expression(module, *value, tree, symbols, ast_tree, ast_strings, context);
                    let value_arg = ast_tree.insert(
                        ast::Argument::Positional {
                            modifiers: None,
                            value,
                        },
                        span,
                    );
                    ast::Expression::Call {
                        position: ast::PostfixPosition::Direct,
                        left: ty,
                        static_arguments: None,
                        dynamic_arguments: vec![value_arg],
                    }
                }

                dir::Expression::TaggedTupleExpression { ty, elements } => {
                    // emit as call: ty(elements...)
                    let ty = self.unbind_expression(module, *ty, tree, symbols, ast_tree, ast_strings, context);
                    let elements = elements.iter().map(|el| {
                        self.unbind_argument(module, *el, tree, symbols, ast_tree, ast_strings, context)
                    }).collect();
                    ast::Expression::Call {
                        position: ast::PostfixPosition::Direct,
                        left: ty,
                        static_arguments: None,
                        dynamic_arguments: elements,
                    }
                }

                dir::Expression::TaggedObjectExpression { ty, properties } => {
                    let ty = self.unbind_expression(module, *ty, tree, symbols, ast_tree, ast_strings, context);
                    let properties = properties.iter().map(|prop| {
                        self.unbind_property(module, *prop, tree, symbols, ast_tree, ast_strings, context)
                    }).collect();
                    ast::Expression::ObjectExpression { ty: Some(ty), properties }
                }

                dir::Expression::Parenthesized { expression } => {
                    let expression = self.unbind_expression(module, *expression, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::Parenthesized { expression }
                }

                dir::Expression::If { kind, condition, then_expression, else_expression } => {
                    let kind = self.unbind_if_kind(context, *kind);
                    let condition = self.unbind_expression(module, *condition, tree, symbols, ast_tree, ast_strings, context);
                    let then_expression = self.unbind_expression(module, *then_expression, tree, symbols, ast_tree, ast_strings, context);
                    let else_expression = else_expression.map(|e| {
                        self.unbind_expression(module, e, tree, symbols, ast_tree, ast_strings, context)
                    });
                    ast::Expression::If { kind, condition, then_expression, else_expression }
                }

                dir::Expression::Loop { kind, condition, body, .. } => {
                    let body = self.unbind_block(module, *body, tree, symbols, ast_tree, ast_strings, context);
                    match kind {
                        dir::LoopKind::NoTest => ast::Expression::Loop { body },
                        dir::LoopKind::PreTest => {
                            let condition = condition.map(|c| {
                                self.unbind_expression(module, c, tree, symbols, ast_tree, ast_strings, context)
                            }).unwrap_or_else(|| {
                                ast_tree.insert(ast::Expression::ScalarLiteral(ast::ScalarLiteral::Boolean(true)), span)
                            });
                            ast::Expression::While { kind: ast::WhileKind::While, condition, body }
                        }
                        dir::LoopKind::PostTest => {
                            let condition = condition.map(|c| {
                                self.unbind_expression(module, c, tree, symbols, ast_tree, ast_strings, context)
                            }).unwrap_or_else(|| {
                                ast_tree.insert(ast::Expression::ScalarLiteral(ast::ScalarLiteral::Boolean(true)), span)
                            });
                            ast::Expression::While { kind: ast::WhileKind::DoWhile, condition, body }
                        }
                    }
                }

                dir::Expression::ForEach {
                    asynchrony,
                    kind,
                    binding,
                    iterator,
                    body,
                    ..
                } => {
                    let asynchrony = self.unbind_asynchrony(context, *asynchrony);
                    let kind = self.unbind_for_each_kind(context, *kind);
                    let binding = match binding {
                        dir::ForEachBinding::Pattern { pattern } => {
                            let pattern = self.unbind_pattern(
                                module,
                                *pattern,
                                tree,
                                symbols,
                                ast_tree,
                                ast_strings,
                                context,
                            );
                            ast::ForEachBinding::Pattern { pattern }
                        }
                        dir::ForEachBinding::Using { asynchrony, pattern } => {
                            let pattern = self.unbind_pattern(
                                module,
                                *pattern,
                                tree,
                                symbols,
                                ast_tree,
                                ast_strings,
                                context,
                            );
                            ast::ForEachBinding::Using {
                                asynchrony: self.unbind_asynchrony(context, *asynchrony),
                                pattern,
                            }
                        }
                    };
                    let iterator = self.unbind_expression(module, *iterator, tree, symbols, ast_tree, ast_strings, context);
                    let body = self.unbind_block(module, *body, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::ForEach {
                        asynchrony,
                        kind,
                        binding,
                        iterator,
                        body,
                    }
                }

                dir::Expression::For { initialization, condition, increment, body, .. } => {
                    let initialization = initialization.map(|i| {
                        self.unbind_expression(module, i, tree, symbols, ast_tree, ast_strings, context)
                    });
                    let condition = condition.map(|c| {
                        self.unbind_expression(module, c, tree, symbols, ast_tree, ast_strings, context)
                    });
                    let increment = increment.map(|i| {
                        self.unbind_expression(module, i, tree, symbols, ast_tree, ast_strings, context)
                    });
                    let body = self.unbind_block(module, *body, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::For { initialization, condition, increment, body }
                }

                dir::Expression::Try { try_expression, catch_pattern, catch_expression, finally_expression, .. } => {
                    let try_expression = self.unbind_expression(module, *try_expression, tree, symbols, ast_tree, ast_strings, context);
                    let catch_pattern = catch_pattern.map(|p| {
                        self.unbind_pattern(module, p, tree, symbols, ast_tree, ast_strings, context)
                    });
                    let catch_expression = catch_expression.map(|e| {
                        self.unbind_expression(module, e, tree, symbols, ast_tree, ast_strings, context)
                    });
                    let finally_expression = finally_expression.map(|e| {
                        self.unbind_expression(module, e, tree, symbols, ast_tree, ast_strings, context)
                    });
                    ast::Expression::Try { try_expression, catch_pattern, catch_expression, finally_expression }
                }

                dir::Expression::Match { value, cases, source, .. } => {
                    let kind = match source {
                        dir::MatchSource::Match => ast::MatchKind::Match,
                        _ => ast::MatchKind::Switch,
                    };
                    let value = self.unbind_expression(module, *value, tree, symbols, ast_tree, ast_strings, context);
                    let cases = cases.iter().map(|case| {
                        self.unbind_match_case(module, *case, tree, symbols, ast_tree, ast_strings, context)
                    }).collect();
                    ast::Expression::Match { kind, value, cases }
                }

                dir::Expression::UnresolvedBreak { target, value } => {
                    let label = Some(ast_strings.intern_from(&self.program.strings, *target));
                    let value = value.map(|v| self.unbind_expression(module, v, tree, symbols, ast_tree, ast_strings, context));
                    ast::Expression::Break { label, value }
                }

                dir::Expression::Break { target, value, .. } => {
                    let label = target.map(|t| ast_strings.intern_from(&self.program.strings, t));
                    let value = value.map(|v| self.unbind_expression(module, v, tree, symbols, ast_tree, ast_strings, context));
                    ast::Expression::Break { label, value }
                }

                dir::Expression::UnresolvedContinue { target } => {
                    let label = Some(ast_strings.intern_from(&self.program.strings, *target));
                    ast::Expression::Continue { label }
                }

                dir::Expression::Continue { target, .. } => {
                    let label = target.map(|t| ast_strings.intern_from(&self.program.strings, t));
                    ast::Expression::Continue { label }
                }

                dir::Expression::Throw { value } => {
                    let value = self.unbind_expression(module, *value, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::Throw { value }
                }

                dir::Expression::Await { expression } => {
                    let expression = self.unbind_expression(module, *expression, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::Await { expression }
                }

                dir::Expression::AwaitMaybe { expression } => {
                    let expression = self.unbind_expression(module, *expression, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::AwaitMaybe { expression }
                }

                dir::Expression::Comptime { body } => {
                    let body = self.unbind_expression(module, *body, tree, symbols, ast_tree, ast_strings, context);
                    ast::Expression::Comptime { body }
                }

                dir::Expression::Yield { cardinality, value } => {
                    let cardinality = self.unbind_yield_cardinality(context, *cardinality);
                    let value = value.map(|v| self.unbind_expression(module, v, tree, symbols, ast_tree, ast_strings, context));
                    ast::Expression::Yield { cardinality, value }
                }

                dir::Expression::Return { value } => {
                    let value = value.map(|v| self.unbind_expression(module, v, tree, symbols, ast_tree, ast_strings, context));
                    ast::Expression::Return { value }
                }

                dir::Expression::Debugger => ast::Expression::Debugger,
                dir::Expression::Stub => ast::Expression::Stub,
                dir::Expression::Error => ast::Expression::Error,
            };

            let ast_expression_id = ast_tree.insert(ast_expression, span);
            context.map(expression_id.into_any(), ast_expression_id.into_any());

            ast_expression_id
        }
    }

    /// Unbind a DIR type predicate subject to an AST type predicate subject.
    fn unbind_type_predicate_subject(
        &self,
        subject: dir::TypePredicateSubject,
        module: &Module,
        symbols: &dir::SymbolTable,
        ast_strings: &mut StringPool,
        _context: &mut UnbindContext,
    ) -> ast::TypePredicateSubject {
        match subject {
            dir::TypePredicateSubject::This => ast::TypePredicateSubject::This,
            dir::TypePredicateSubject::Unresolved(name) => {
                let name = ast_strings.intern_from(&self.program.strings, name);
                ast::TypePredicateSubject::Identifier(name)
            }
            dir::TypePredicateSubject::Symbol(symbol_id) => {
                let name_id = if symbol_id.module_id == module.id {
                    symbols.get_symbol(symbol_id.into_local()).name()
                } else {
                    self.program
                        .modules
                        .get(symbol_id.module_id)
                        .read()
                        .dir_base_maybe()
                        .and_then(|dir| {
                            let symbols = dir.symbols.read();
                            symbols.get_symbol(symbol_id.into_local()).name()
                        })
                };
                let name_id = name_id
                    .map(|name| ast_strings.intern_from(&self.program.strings, name))
                    .unwrap_or_else(|| ast_strings.intern("_"));
                ast::TypePredicateSubject::Identifier(name_id)
            }
        }
    }

    /// Unbind if kind to AST if kind.
    #[inline]
    fn unbind_if_kind(&self, _context: &mut UnbindContext, kind: dir::IfKind) -> ast::IfKind {
        match kind {
            dir::IfKind::If => ast::IfKind::If,
            dir::IfKind::Ternary => ast::IfKind::Ternary,
        }
    }

    /// Unbind yield cardinality to AST yield cardinality.
    #[inline]
    fn unbind_yield_cardinality(
        &self,
        _context: &mut UnbindContext,
        cardinality: dir::YieldCardinality,
    ) -> ast::YieldCardinality {
        match cardinality {
            dir::YieldCardinality::Generator => ast::YieldCardinality::Generator,
            dir::YieldCardinality::Scalar => ast::YieldCardinality::Scalar,
        }
    }

    /// Unbind for each kind to AST for each kind.
    #[inline]
    fn unbind_for_each_kind(
        &self,
        _context: &mut UnbindContext,
        kind: dir::ForEachKind,
    ) -> ast::ForEachKind {
        match kind {
            dir::ForEachKind::In => ast::ForEachKind::In,
            dir::ForEachKind::Of => ast::ForEachKind::Of,
        }
    }
}
