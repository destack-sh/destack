use destack_ast::{self as ast};
use destack_base::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

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
        ) -> ast::LocalNodeId<ast::Expression> {
            let expression = tree.get(expression_id);
            let span = self.unbind_span(module, expression_id.into());

            let ast_expression = match expression {
                dir::Expression::Declaration { declaration } => {
                    let declaration = self.unbind_declaration(module, *declaration, tree, symbols, ast_tree, ast_strings);
                    ast::Expression::Declaration(declaration)
                }

                dir::Expression::Block { block } => {
                    let block = self.unbind_block(module, *block, tree, symbols, ast_tree, ast_strings);
                    ast::Expression::Block(block)
                }

                dir::Expression::Statement { statement } => {
                    let statement = self.unbind_expression(module, *statement, tree, symbols, ast_tree, ast_strings);
                    ast::Expression::Statement(statement)
                }

                dir::Expression::Labelled { label, body, .. } => {
                    let label = ast_strings.intern_from(&self.program.strings, *label);
                    let body = self.unbind_expression(module, *body, tree, symbols, ast_tree, ast_strings);
                    ast::Expression::Labelled { label, body }
                }

                dir::Expression::UnresolvedImport { kind, target, items, arguments }
                | dir::Expression::Import { kind, target, items, arguments, .. } => {
                    let kind = self.unbind_dependency_kind(*kind);
                    let target = ast_strings.intern_from(&self.program.strings, *target);
                    let items = items.iter().map(|item| {
                        self.unbind_dependency_item(module, *item, tree, symbols, ast_tree, ast_strings)
                    }).collect();
                    let arguments = arguments.as_ref().map(|args| {
                        args.iter().map(|arg| {
                            self.unbind_argument(module, *arg, tree, symbols, ast_tree, ast_strings)
                        }).collect()
                    });
                    ast::Expression::Import { kind, target, items, arguments }
                }

                dir::Expression::UnresolvedReExport { target, kind, items }
                | dir::Expression::ReExport { target, kind, items, .. } => {
                    let kind = self.unbind_dependency_kind(*kind);
                    let target = ast_strings.intern_from(&self.program.strings, *target);
                    let items = items.iter().map(|item| {
                        self.unbind_dependency_item(module, *item, tree, symbols, ast_tree, ast_strings)
                    }).collect();
                    ast::Expression::Export { kind, target: Some(target), items }
                }

                dir::Expression::Export { kind, items } => {
                    let kind = self.unbind_dependency_kind(*kind);
                    let items = items.iter().map(|item| {
                        self.unbind_dependency_item(module, *item, tree, symbols, ast_tree, ast_strings)
                    }).collect();
                    ast::Expression::Export { kind, target: None, items }
                }

                dir::Expression::Let { descriptor, mutability, declarators } => {
                    let descriptor = self.unbind_declaration_descriptor(descriptor, ast_strings);
                    let mutability = self.unbind_mutability(*mutability);
                    let declarators = declarators.iter().map(|decl| {
                        self.unbind_declarator(module, *decl, tree, symbols, ast_tree, ast_strings)
                    }).collect();
                    ast::Expression::Let { descriptor, mutability, declarators }
                }

                dir::Expression::TypeUnary { operator, right } => {
                    let operator = self.unbind_type_unary_operator(*operator);
                    let right = self.unbind_expression(module, *right, tree, symbols, ast_tree, ast_strings);
                    ast::Expression::TypeUnary { operator, right }
                }

                dir::Expression::TypeBinary { left, operator, right } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, ast_tree, ast_strings);
                    let operator = self.unbind_type_binary_operator(*operator);
                    let right = self.unbind_expression(module, *right, tree, symbols, ast_tree, ast_strings);
                    ast::Expression::TypeBinary { left, operator, right }
                }

                dir::Expression::Unary { operator, right } => {
                    let operator = self.unbind_unary_operator(*operator);
                    let right = self.unbind_expression(module, *right, tree, symbols, ast_tree, ast_strings);
                    ast::Expression::Unary { operator, right }
                }

                dir::Expression::ValueOf { mutability, variance, right } => {
                    let mutability = mutability.map(|m| self.unbind_mutability(m));
                    let variance = variance.map(|v| self.unbind_variance_bound(v));
                    let right = self.unbind_expression(module, *right, tree, symbols, ast_tree, ast_strings);
                    ast::Expression::ValueOf { mutability, variance, right }
                }

                dir::Expression::ReferenceOf { mutability, variance, right } => {
                    let mutability = mutability.map(|m| self.unbind_mutability(m));
                    let variance = variance.map(|v| self.unbind_variance_bound(v));
                    let right = self.unbind_expression(module, *right, tree, symbols, ast_tree, ast_strings);
                    ast::Expression::ReferenceOf { mutability, variance, right }
                }

                dir::Expression::Binary { left, operator, right } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, ast_tree, ast_strings);
                    let operator = self.unbind_binary_operator(*operator);
                    let right = self.unbind_expression(module, *right, tree, symbols, ast_tree, ast_strings);
                    ast::Expression::Binary { left, operator, right }
                }

                dir::Expression::Assign { left, right } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, ast_tree, ast_strings);
                    let right = self.unbind_expression(module, *right, tree, symbols, ast_tree, ast_strings);
                    ast::Expression::Assign { left, operator: ast::AssignOperator::Assign, right }
                }

                dir::Expression::AssignBinary { left, operator, right } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, ast_tree, ast_strings);
                    let operator = self.unbind_assign_operator(*operator);
                    let right = self.unbind_expression(module, *right, tree, symbols, ast_tree, ast_strings);
                    ast::Expression::Assign { left, operator, right }
                }

                dir::Expression::Member { left, name, static_arguments } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, ast_tree, ast_strings);
                    let name = ast_strings.intern_from(&self.program.strings, *name);
                    let static_arguments = static_arguments.as_ref().map(|args| {
                        args.iter().map(|arg| {
                            self.unbind_argument(module, *arg, tree, symbols, ast_tree, ast_strings)
                        }).collect()
                    });
                    ast::Expression::Member { left, name, static_arguments }
                }

                dir::Expression::Call { left, static_arguments, dynamic_arguments } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, ast_tree, ast_strings);
                    let static_arguments = static_arguments.as_ref().map(|args| {
                        args.iter().map(|arg| {
                            self.unbind_argument(module, *arg, tree, symbols, ast_tree, ast_strings)
                        }).collect()
                    });
                    let dynamic_arguments = dynamic_arguments.iter().map(|arg| {
                        self.unbind_argument(module, *arg, tree, symbols, ast_tree, ast_strings)
                    }).collect();
                    ast::Expression::Call {
                        position: ast::PostfixPosition::Direct,
                        left,
                        static_arguments,
                        dynamic_arguments,
                    }
                }

                dir::Expression::Index { left, right } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, ast_tree, ast_strings);
                    let index = right.map(|r| self.unbind_expression(module, r, tree, symbols, ast_tree, ast_strings));
                    ast::Expression::Index {
                        position: ast::PostfixPosition::Direct,
                        left,
                        index,
                    }
                }

                dir::Expression::Maybe { left } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, ast_tree, ast_strings);
                    ast::Expression::Maybe {
                        position: ast::PostfixPosition::Direct,
                        left,
                    }
                }

                dir::Expression::Must { left } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, ast_tree, ast_strings);
                    ast::Expression::Must {
                        position: ast::PostfixPosition::Direct,
                        left,
                    }
                }

                dir::Expression::New { left, static_arguments, dynamic_arguments } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, ast_tree, ast_strings);
                    let static_arguments = static_arguments.as_ref().map(|args| {
                        args.iter().map(|arg| {
                            self.unbind_argument(module, *arg, tree, symbols, ast_tree, ast_strings)
                        }).collect()
                    });
                    let dynamic_arguments = dynamic_arguments.iter().map(|arg| {
                        self.unbind_argument(module, *arg, tree, symbols, ast_tree, ast_strings)
                    }).collect();
                    ast::Expression::New { left, static_arguments, dynamic_arguments }
                }

                dir::Expression::Delete { value } => {
                    let value = self.unbind_expression(module, *value, tree, symbols, ast_tree, ast_strings);
                    ast::Expression::Delete { value }
                }

                dir::Expression::UnresolvedPath { path, static_arguments }
                | dir::Expression::LocalReference { path, static_arguments, .. }
                | dir::Expression::ModuleReference { path, static_arguments, .. }
                | dir::Expression::GlobalReference { path, static_arguments, .. } => {
                    let path = self.unbind_path(path, ast_strings);
                    let static_arguments = static_arguments.as_ref().map(|args| {
                        args.iter().map(|arg| {
                            self.unbind_argument(module, *arg, tree, symbols, ast_tree, ast_strings)
                        }).collect()
                    });
                    ast::Expression::Path { path, static_arguments }
                }

                dir::Expression::ScalarLiteral { value } => {
                    let value = self.unbind_scalar_literal(value, ast_strings);
                    ast::Expression::ScalarLiteral(value)
                }

                dir::Expression::TypeLiteral { value } => {
                    let value = self.unbind_type_literal(value);
                    ast::Expression::TypeLiteral(value)
                }

                dir::Expression::Type { .. } => {
                    // #Incomplete: unbind dir::Expression::Type?
                    ast::Expression::Error
                }

                dir::Expression::TemplateExpression { value } => {
                    let value = self.unbind_template_literal(module,value, tree, symbols, ast_tree, ast_strings);
                    ast::Expression::TemplateExpression { value }
                }

                dir::Expression::TaggedTemplateExpression { tag, value } => {
                    let tag = self.unbind_expression(module, *tag, tree, symbols, ast_tree, ast_strings);
                    let value = self.unbind_template_literal(module,value, tree, symbols, ast_tree, ast_strings);
                    ast::Expression::TaggedTemplateExpression { tag, value }
                }

                dir::Expression::RangeExpression { start, end, is_inclusive } => {
                    let start = self.unbind_expression(module, *start, tree, symbols, ast_tree, ast_strings);
                    let end = self.unbind_expression(module, *end, tree, symbols, ast_tree, ast_strings);
                    ast::Expression::RangeExpression { start, end, is_inclusive: *is_inclusive }
                }

                dir::Expression::ArrayExpression { elements } => {
                    let elements = elements.iter().map(|el| {
                        self.unbind_argument(module, *el, tree, symbols, ast_tree, ast_strings)
                    }).collect();
                    ast::Expression::ArrayExpression { elements }
                }

                dir::Expression::TupleExpression { elements } => {
                    let elements = elements.iter().map(|el| {
                        self.unbind_argument(module, *el, tree, symbols, ast_tree, ast_strings)
                    }).collect();
                    ast::Expression::TupleExpression { elements }
                }

                dir::Expression::SequenceExpression { expressions } => {
                    let expressions = expressions.iter().map(|expr| {
                        self.unbind_expression(module, *expr, tree, symbols, ast_tree, ast_strings)
                    }).collect();
                    ast::Expression::SequenceExpression { expressions }
                }

                dir::Expression::ObjectExpression { properties } => {
                    let properties = properties.iter().map(|prop| {
                        self.unbind_property(module, *prop, tree, symbols, ast_tree, ast_strings)
                    }).collect();
                    ast::Expression::ObjectExpression { ty: None, properties }
                }

                dir::Expression::TreeExpression { left, arguments, elements } => {
                    let left = left.map(|l| self.unbind_expression(module, l, tree, symbols, ast_tree, ast_strings));
                    let arguments = arguments.as_ref().map(|args| {
                        args.iter().map(|arg| {
                            self.unbind_argument(module, *arg, tree, symbols, ast_tree, ast_strings)
                        }).collect()
                    });
                    let elements = elements.as_ref().map(|els| {
                        els.iter().map(|el| {
                            self.unbind_argument(module, *el, tree, symbols, ast_tree, ast_strings)
                        }).collect()
                    });
                    ast::Expression::TreeExpression { left, arguments, elements }
                }

                dir::Expression::TaggedScalarExpression { ty, value } => {
                    // emit as call: ty(value)
                    let ty = self.unbind_expression(module, *ty, tree, symbols, ast_tree, ast_strings);
                    let value = self.unbind_expression(module, *value, tree, symbols, ast_tree, ast_strings);
                    let value_arg = ast_tree.insert(ast::Argument::Positional { value }, span);
                    ast::Expression::Call {
                        position: ast::PostfixPosition::Direct,
                        left: ty,
                        static_arguments: None,
                        dynamic_arguments: vec![value_arg],
                    }
                }

                dir::Expression::TaggedTupleExpression { ty, elements } => {
                    // emit as call: ty(elements...)
                    let ty = self.unbind_expression(module, *ty, tree, symbols, ast_tree, ast_strings);
                    let elements = elements.iter().map(|el| {
                        self.unbind_argument(module, *el, tree, symbols, ast_tree, ast_strings)
                    }).collect();
                    ast::Expression::Call {
                        position: ast::PostfixPosition::Direct,
                        left: ty,
                        static_arguments: None,
                        dynamic_arguments: elements,
                    }
                }

                dir::Expression::TaggedObjectExpression { ty, properties } => {
                    let ty = self.unbind_expression(module, *ty, tree, symbols, ast_tree, ast_strings);
                    let properties = properties.iter().map(|prop| {
                        self.unbind_property(module, *prop, tree, symbols, ast_tree, ast_strings)
                    }).collect();
                    ast::Expression::ObjectExpression { ty: Some(ty), properties }
                }

                dir::Expression::Parenthesized { expression } => {
                    let expression = self.unbind_expression(module, *expression, tree, symbols, ast_tree, ast_strings);
                    ast::Expression::Parenthesized { expression }
                }

                dir::Expression::If { kind, condition, then_expression, else_expression } => {
                    let kind = self.unbind_if_kind(*kind);
                    let condition = self.unbind_expression(module, *condition, tree, symbols, ast_tree, ast_strings);
                    let then_expression = self.unbind_expression(module, *then_expression, tree, symbols, ast_tree, ast_strings);
                    let else_expression = else_expression.map(|e| {
                        self.unbind_expression(module, e, tree, symbols, ast_tree, ast_strings)
                    });
                    ast::Expression::If { kind, condition, then_expression, else_expression }
                }

                dir::Expression::Loop { kind, condition, body, .. } => {
                    let body = self.unbind_block(module, *body, tree, symbols, ast_tree, ast_strings);
                    match kind {
                        dir::LoopKind::NoTest => ast::Expression::Loop { body },
                        dir::LoopKind::PreTest => {
                            let condition = condition.map(|c| {
                                self.unbind_expression(module, c, tree, symbols, ast_tree, ast_strings)
                            }).unwrap_or_else(|| {
                                ast_tree.insert(ast::Expression::ScalarLiteral(ast::ScalarLiteral::Boolean(true)), span)
                            });
                            ast::Expression::While { kind: ast::WhileKind::While, condition, body }
                        }
                        dir::LoopKind::PostTest => {
                            let condition = condition.map(|c| {
                                self.unbind_expression(module, c, tree, symbols, ast_tree, ast_strings)
                            }).unwrap_or_else(|| {
                                ast_tree.insert(ast::Expression::ScalarLiteral(ast::ScalarLiteral::Boolean(true)), span)
                            });
                            ast::Expression::While { kind: ast::WhileKind::DoWhile, condition, body }
                        }
                    }
                }

                dir::Expression::ForEach { asynchrony, kind, pattern, iterator, body, .. } => {
                    let asynchrony = self.unbind_asynchrony(*asynchrony);
                    let kind = self.unbind_for_each_kind(*kind);
                    let pattern = self.unbind_pattern(module, *pattern, tree, symbols, ast_tree, ast_strings);
                    let iterator = self.unbind_expression(module, *iterator, tree, symbols, ast_tree, ast_strings);
                    let body = self.unbind_block(module, *body, tree, symbols, ast_tree, ast_strings);
                    ast::Expression::ForEach { asynchrony, kind, pattern, iterator, body }
                }

                dir::Expression::For { initialization, condition, increment, body, .. } => {
                    let initialization = initialization.map(|i| {
                        self.unbind_expression(module, i, tree, symbols, ast_tree, ast_strings)
                    });
                    let condition = condition.map(|c| {
                        self.unbind_expression(module, c, tree, symbols, ast_tree, ast_strings)
                    });
                    let increment = increment.map(|i| {
                        self.unbind_expression(module, i, tree, symbols, ast_tree, ast_strings)
                    });
                    let body = self.unbind_block(module, *body, tree, symbols, ast_tree, ast_strings);
                    ast::Expression::For { initialization, condition, increment, body }
                }

                dir::Expression::Try { try_expression, catch_pattern, catch_expression, finally_expression, .. } => {
                    let try_expression = self.unbind_expression(module, *try_expression, tree, symbols, ast_tree, ast_strings);
                    let catch_pattern = catch_pattern.map(|p| {
                        self.unbind_pattern(module, p, tree, symbols, ast_tree, ast_strings)
                    });
                    let catch_expression = catch_expression.map(|e| {
                        self.unbind_expression(module, e, tree, symbols, ast_tree, ast_strings)
                    });
                    let finally_expression = finally_expression.map(|e| {
                        self.unbind_expression(module, e, tree, symbols, ast_tree, ast_strings)
                    });
                    ast::Expression::Try { try_expression, catch_pattern, catch_expression, finally_expression }
                }

                dir::Expression::Match { value, cases, source, .. } => {
                    let kind = match source {
                        dir::MatchSource::Match => ast::MatchKind::Match,
                        _ => ast::MatchKind::Switch,
                    };
                    let value = self.unbind_expression(module, *value, tree, symbols, ast_tree, ast_strings);
                    let cases = cases.iter().map(|case| {
                        self.unbind_match_case(module, *case, tree, symbols, ast_tree, ast_strings)
                    }).collect();
                    ast::Expression::Match { kind, value, cases }
                }

                dir::Expression::UnresolvedBreak { target, value } => {
                    let label = Some(ast_strings.intern_from(&self.program.strings, *target));
                    let value = value.map(|v| self.unbind_expression(module, v, tree, symbols, ast_tree, ast_strings));
                    ast::Expression::Break { label, value }
                }

                dir::Expression::Break { target, value, .. } => {
                    let label = target.map(|t| ast_strings.intern_from(&self.program.strings, t));
                    let value = value.map(|v| self.unbind_expression(module, v, tree, symbols, ast_tree, ast_strings));
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
                    let value = self.unbind_expression(module, *value, tree, symbols, ast_tree, ast_strings);
                    ast::Expression::Throw { value }
                }

                dir::Expression::Await { expression } => {
                    let expression = self.unbind_expression(module, *expression, tree, symbols, ast_tree, ast_strings);
                    ast::Expression::Await { expression }
                }

                dir::Expression::Yield { cardinality, value } => {
                    let cardinality = self.unbind_yield_cardinality(*cardinality);
                    let value = value.map(|v| self.unbind_expression(module, v, tree, symbols, ast_tree, ast_strings));
                    ast::Expression::Yield { cardinality, value }
                }

                dir::Expression::Return { value } => {
                    let value = value.map(|v| self.unbind_expression(module, v, tree, symbols, ast_tree, ast_strings));
                    ast::Expression::Return { value }
                }

                dir::Expression::Debugger => ast::Expression::Debugger,
                dir::Expression::Stub => ast::Expression::Stub,
                dir::Expression::Error => ast::Expression::Error,
            };

            ast_tree.insert(ast_expression, span)
        }
    }

    /// Unbind if kind to AST if kind.
    #[inline]
    fn unbind_if_kind(&self, kind: dir::IfKind) -> ast::IfKind {
        match kind {
            dir::IfKind::If => ast::IfKind::If,
            dir::IfKind::Ternary => ast::IfKind::Ternary,
        }
    }

    /// Unbind yield cardinality to AST yield cardinality.
    #[inline]
    fn unbind_yield_cardinality(
        &self,
        cardinality: dir::YieldCardinality,
    ) -> ast::YieldCardinality {
        match cardinality {
            dir::YieldCardinality::Generator => ast::YieldCardinality::Generator,
            dir::YieldCardinality::Scalar => ast::YieldCardinality::Scalar,
        }
    }

    /// Unbind for each kind to AST for each kind.
    #[inline]
    fn unbind_for_each_kind(&self, kind: dir::ForEachKind) -> ast::ForEachKind {
        match kind {
            dir::ForEachKind::In => ast::ForEachKind::In,
            dir::ForEachKind::Of => ast::ForEachKind::Of,
        }
    }
}
