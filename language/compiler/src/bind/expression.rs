use dyst_ast::{self as ast};
use dyst_dir::{
    DependencySource, Expression, ForEachKind, IfKind, LocalNodeId, LocalScopeId, LocalScopeMark,
    LoopKind, MatchSource, Module, NodeTree, ScopeKind, SymbolKey, SymbolSpace, SymbolTable,
    TypeTable, YieldCardinality,
};

use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind if kind into a DIR if kind.
    #[inline]
    pub(super) fn bind_if_kind(&self, kind: ast::IfKind) -> IfKind {
        match kind {
            ast::IfKind::If => IfKind::If,
            ast::IfKind::Ternary => IfKind::Ternary,
        }
    }

    /// Bind an expression to a DIR expression.
    pub(super) fn bind_expression(
        &self,
        module: &Module,
        scope: (LocalScopeId, LocalScopeMark),
        expression_id: ast::LocalNodeId<ast::Expression>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Expression> {
        let expression = module.get(expression_id);
        let expression = match expression {
            ast::Expression::Block(block_id) => {
                let block_id = self.bind_block(module, scope, *block_id, tree, symbols, types);
                Expression::Block { block: block_id }
            }
            ast::Expression::Declaration(declaration_id) => {
                let declaration_id =
                    self.bind_declaration(module, scope, *declaration_id, tree, symbols, types);
                Expression::Declaration {
                    declaration: declaration_id,
                }
            }
            ast::Expression::Statement(expression_id) => {
                let expression_id =
                    self.bind_expression(module, scope, *expression_id, tree, symbols, types);
                Expression::Statement {
                    statement: expression_id,
                }
            }

            ast::Expression::With { clauses, body } => {
                let (symbol_id, scope_id) =
                    self.bind_anonymous_item_with_scope(module, ScopeKind::Block, scope, symbols);
                let clauses = clauses
                    .iter()
                    .map(|clause| {
                        self.bind_with_clause(
                            module,
                            (scope_id, symbols.get_scope_mark(scope_id)),
                            *clause,
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                let body = body.map(|body| {
                    self.bind_block(
                        module,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        body,
                        tree,
                        symbols,
                        types,
                    )
                });
                Expression::With {
                    clauses,
                    body,
                    scope: scope_id,
                    symbol: symbol_id,
                }
            }
            ast::Expression::Import {
                kind,
                target,
                items,
                arguments,
            } => {
                let target = self
                    .program
                    .strings
                    .intern_from(&module.ast_strings, *target);
                // items
                let items: Vec<_> = items
                    .iter()
                    .map(|item| {
                        self.bind_dependency_item(
                            module,
                            scope,
                            DependencySource::ImportStatement,
                            *kind,
                            Some(target),
                            *item,
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                // arguments
                let arguments = arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.bind_argument(module, scope, *argument, tree, symbols, types)
                        })
                        .collect()
                });
                let kind = self.bind_dependency_kind(*kind);
                // import
                Expression::UnresolvedImport {
                    kind,
                    target,
                    items,
                    arguments,
                }
            }
            ast::Expression::Export {
                kind,
                target,
                items,
            } => {
                let target = target.map(|target| {
                    self.program
                        .strings
                        .intern_from(&module.ast_strings, target)
                });
                // re-export from import
                if let Some(target) = target {
                    // items
                    let items: Vec<_> = items
                        .iter()
                        .map(|item| {
                            self.bind_dependency_item(
                                module,
                                scope,
                                DependencySource::ReExportStatement,
                                *kind,
                                Some(target),
                                *item,
                                tree,
                                symbols,
                                types,
                            )
                        })
                        .collect();
                    let kind = self.bind_dependency_kind(*kind);
                    // re-export
                    Expression::UnresolvedReExport {
                        target,
                        kind,
                        items,
                    }
                }
                // export from module
                else {
                    // export = value
                    let items = {
                        // export items
                        items
                            .iter()
                            .map(|item| {
                                self.bind_dependency_item(
                                    module,
                                    scope,
                                    DependencySource::ValueExpression,
                                    *kind,
                                    None,
                                    *item,
                                    tree,
                                    symbols,
                                    types,
                                )
                            })
                            .collect()
                    };
                    let kind = self.bind_dependency_kind(*kind);
                    Expression::Export { kind, items }
                }
            }
            ast::Expression::Let {
                descriptor: _,
                mutability,
                pattern,
                ty,
                value,
            } => {
                let mutability = self.bind_mutability(*mutability);
                let pattern = self.bind_pattern(module, scope, *pattern, tree, symbols, types);
                let value = value
                    .map(|value| self.bind_expression(module, scope, value, tree, symbols, types));
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, SymbolSpace::Value, scope, symbols);
                if let Some(ty) = ty {
                    let ty = self.bind_expression_to_type(module, scope, *ty, tree, symbols, types);
                    let expression = Expression::Let {
                        mutability,
                        pattern,
                        value,
                        symbol: symbol_id,
                    };
                    let expression_id = tree.insert_from_source(expression, expression_id, scope);
                    symbols
                        .get_symbol_mut(symbol_id)
                        .declare_primary(expression_id);
                    types.declare_type(expression_id.into_global_any(module.id), ty);
                    return expression_id;
                }
                Expression::Let {
                    mutability,
                    pattern,
                    value,
                    symbol: symbol_id,
                }
            }
            ast::Expression::LetType {
                kind,
                descriptor,
                mutability,
                static_parameters,
                value,
            } => {
                let kind = self.bind_type_kind(*kind);
                let name = self.program.strings.intern_from(
                    &module.ast_strings,
                    descriptor.name.expect("LetType must have a name").string(),
                );
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let static_parameters = static_parameters.as_ref().map(|params| {
                    params
                        .iter()
                        .map(|param| {
                            self.bind_parameter(module, scope, *param, tree, symbols, types)
                        })
                        .collect()
                });
                let value = self.bind_expression(module, scope, *value, tree, symbols, types);
                let (symbol_id, _) = self.bind_named_item(
                    module,
                    SymbolSpace::Value,
                    SymbolKey::Name(name),
                    scope,
                    symbols,
                );
                Expression::LetType {
                    kind,
                    mutability,
                    name,
                    static_parameters,
                    value,
                    symbol: symbol_id,
                }
            }

            ast::Expression::Unary { operator, right } => {
                let right = self.bind_expression(module, scope, *right, tree, symbols, types);
                let operator = self.bind_unary_operator(*operator);
                Expression::Unary { operator, right }
            }

            ast::Expression::TypeUnary { operator, right } => {
                let right = self.bind_expression(module, scope, *right, tree, symbols, types);
                let operator = self.bind_type_unary_operator(*operator);
                Expression::TypeUnary { operator, right }
            }

            ast::Expression::ValueOf {
                mutability,
                variance,
                right,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let variance = variance.map(|variance| self.bind_variance_bound(variance));
                let right = self.bind_expression(module, scope, *right, tree, symbols, types);
                Expression::ValueOf {
                    mutability,
                    variance,
                    right,
                }
            }
            ast::Expression::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let variance = variance.map(|variance| self.bind_variance_bound(variance));
                let right = self.bind_expression(module, scope, *right, tree, symbols, types);
                Expression::ReferenceOf {
                    mutability,
                    variance,
                    right,
                }
            }
            ast::Expression::Binary {
                left,
                operator,
                right,
            } => {
                let left = self.bind_expression(module, scope, *left, tree, symbols, types);
                let right = self.bind_expression(module, scope, *right, tree, symbols, types);
                let operator = self.bind_binary_operator(*operator);
                Expression::Binary {
                    left,
                    operator,
                    right,
                }
            }
            ast::Expression::TypeBinary {
                left,
                operator,
                right,
            } => {
                let left = self.bind_expression(module, scope, *left, tree, symbols, types);
                let right = self.bind_expression(module, scope, *right, tree, symbols, types);
                let operator = self.bind_type_binary_operator(*operator);
                Expression::TypeBinary {
                    left,
                    operator,
                    right,
                }
            }
            ast::Expression::Assign {
                left,
                operator,
                right,
            } => {
                let left = self.bind_expression(module, scope, *left, tree, symbols, types);
                let right = self.bind_expression(module, scope, *right, tree, symbols, types);
                let operator = self.bind_assign_operator(*operator);
                if let Some(operator) = operator {
                    Expression::AssignBinary {
                        left,
                        operator,
                        right,
                    }
                } else {
                    Expression::Assign { left, right }
                }
            }

            ast::Expression::Member {
                left,
                name,
                static_arguments,
            } => {
                let left = self.bind_expression(module, scope, *left, tree, symbols, types);
                let name = self.program.strings.intern_from(&module.ast_strings, *name);
                let static_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.bind_argument(module, scope, *argument, tree, symbols, types)
                        })
                        .collect()
                });
                Expression::UnresolvedMember {
                    left,
                    name,
                    static_arguments,
                }
            }
            ast::Expression::Call {
                position: _,
                left,
                static_arguments,
                dynamic_arguments,
            } => {
                let left = self.bind_expression(module, scope, *left, tree, symbols, types);
                let static_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.bind_argument(module, scope, *argument, tree, symbols, types)
                        })
                        .collect()
                });
                let dynamic_arguments = dynamic_arguments
                    .iter()
                    .map(|argument| {
                        self.bind_argument(module, scope, *argument, tree, symbols, types)
                    })
                    .collect();
                Expression::Call {
                    left,
                    static_arguments,
                    dynamic_arguments,
                }
            }
            ast::Expression::New {
                left,
                static_arguments,
                dynamic_arguments,
            } => {
                let left = self.bind_expression(module, scope, *left, tree, symbols, types);
                let static_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.bind_argument(module, scope, *argument, tree, symbols, types)
                        })
                        .collect()
                });
                let dynamic_arguments = dynamic_arguments
                    .iter()
                    .map(|argument| {
                        self.bind_argument(module, scope, *argument, tree, symbols, types)
                    })
                    .collect();
                Expression::New {
                    left,
                    static_arguments,
                    dynamic_arguments,
                }
            }
            ast::Expression::Delete { value } => {
                let value = self.bind_expression(module, scope, *value, tree, symbols, types);
                Expression::Delete { value }
            }
            ast::Expression::Index {
                position: _,
                left,
                index,
            } => {
                let left = self.bind_expression(module, scope, *left, tree, symbols, types);
                let index = index
                    .map(|index| self.bind_expression(module, scope, index, tree, symbols, types));
                Expression::Index { left, right: index }
            }
            ast::Expression::Maybe { position: _, left } => {
                let left = self.bind_expression(module, scope, *left, tree, symbols, types);
                Expression::Maybe { left }
            }
            ast::Expression::Must { position: _, left } => {
                let left = self.bind_expression(module, scope, *left, tree, symbols, types);
                Expression::Must { left }
            }

            ast::Expression::Path {
                path,
                static_arguments,
            } => {
                let path = self.bind_path(module, path);
                let static_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.bind_argument(module, scope, *argument, tree, symbols, types)
                        })
                        .collect()
                });
                Expression::UnresolvedAbsolutePath {
                    path,
                    static_arguments,
                }
            }
            ast::Expression::ScalarLiteral(value) => {
                let value = self.bind_scalar_literal(module, value);
                Expression::ScalarLiteral { value }
            }
            ast::Expression::TemplateLiteral { value } => {
                let value = self.bind_template_literal(module, scope, value, tree, symbols, types);
                Expression::TemplateLiteral { value }
            }
            ast::Expression::TaggedTemplateLiteral { tag, value } => {
                let tag = self.bind_expression(module, scope, *tag, tree, symbols, types);
                let value = self.bind_template_literal(module, scope, value, tree, symbols, types);
                Expression::TaggedTemplateLiteral { tag, value }
            }
            ast::Expression::TypeLiteral(value) => {
                let value = self.bind_type_literal(value);
                Expression::TypeLiteral { value }
            }
            ast::Expression::RangeLiteral {
                start,
                end,
                is_inclusive,
            } => {
                let start = self.bind_expression(module, scope, *start, tree, symbols, types);
                let end = self.bind_expression(module, scope, *end, tree, symbols, types);
                Expression::RangeLiteral {
                    start,
                    end,
                    is_inclusive: *is_inclusive,
                }
            }
            ast::Expression::StructLiteral { ty, properties } => {
                let ty = ty.map(|ty| self.bind_expression(module, scope, ty, tree, symbols, types));
                let properties = properties
                    .iter()
                    .map(|property| {
                        self.bind_property(module, scope, *property, tree, symbols, types)
                    })
                    .collect();
                Expression::StructLiteral { ty, properties }
            }
            ast::Expression::TupleLiteral { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| {
                        self.bind_argument(module, scope, *element, tree, symbols, types)
                    })
                    .collect();
                Expression::TupleLiteral { ty: None, elements }
            }
            ast::Expression::ArrayLiteral { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| {
                        self.bind_argument(module, scope, *element, tree, symbols, types)
                    })
                    .collect();
                Expression::ArrayLiteral { elements }
            }
            ast::Expression::TreeLiteral {
                left,
                arguments,
                elements,
            } => {
                let left = left
                    .map(|left| self.bind_expression(module, scope, left, tree, symbols, types));
                let arguments = arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.bind_argument(module, scope, *argument, tree, symbols, types)
                        })
                        .collect()
                });
                let elements = elements.as_ref().map(|elements| {
                    elements
                        .iter()
                        .map(|element| {
                            self.bind_argument(module, scope, *element, tree, symbols, types)
                        })
                        .collect()
                });
                Expression::TreeLiteral {
                    left,
                    arguments,
                    elements,
                }
            }
            ast::Expression::Parenthesized { expression } => {
                let expression =
                    self.bind_expression(module, scope, *expression, tree, symbols, types);
                Expression::Parenthesized { expression }
            }

            ast::Expression::If {
                kind,
                condition,
                then_expression,
                else_expression,
            } => {
                let kind = self.bind_if_kind(*kind);
                let condition =
                    self.bind_expression(module, scope, *condition, tree, symbols, types);
                let then_expression =
                    self.bind_expression(module, scope, *then_expression, tree, symbols, types);
                let else_expression = else_expression.map(|else_expression| {
                    self.bind_expression(module, scope, else_expression, tree, symbols, types)
                });
                Expression::If {
                    kind,
                    condition,
                    then_expression,
                    else_expression,
                }
            }
            ast::Expression::While {
                kind,
                condition,
                body,
            } => {
                let kind = match *kind {
                    ast::WhileKind::While => LoopKind::PreTest,
                    ast::WhileKind::DoWhile => LoopKind::PostTest,
                };
                let condition =
                    self.bind_expression(module, scope, *condition, tree, symbols, types);
                let (symbol_id, scope_id) =
                    self.bind_anonymous_item_with_scope(module, ScopeKind::Block, scope, symbols);
                let body = self.bind_block(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *body,
                    tree,
                    symbols,
                    types,
                );
                Expression::Loop {
                    kind,
                    condition: Some(condition),
                    body,
                    scope: scope_id,
                    symbol: symbol_id,
                }
            }
            ast::Expression::ForEach {
                asynchrony,
                kind,
                pattern,
                iterator,
                body,
            } => {
                let asynchrony = self.bind_asynchrony(*asynchrony);
                let kind = match *kind {
                    ast::ForEachKind::In => ForEachKind::In,
                    ast::ForEachKind::Of => ForEachKind::Of,
                };
                let pattern = self.bind_pattern(module, scope, *pattern, tree, symbols, types);
                let iterator = self.bind_expression(module, scope, *iterator, tree, symbols, types);
                let (symbol_id, scope_id) =
                    self.bind_anonymous_item_with_scope(module, ScopeKind::Block, scope, symbols);
                let body = self.bind_block(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *body,
                    tree,
                    symbols,
                    types,
                );
                Expression::ForEach {
                    asynchrony,
                    kind,
                    pattern,
                    iterator,
                    body,
                    scope: scope_id,
                    symbol: symbol_id,
                }
            }
            ast::Expression::For {
                initialization,
                condition,
                increment,
                body,
            } => {
                let (symbol_id, scope_id) =
                    self.bind_anonymous_item_with_scope(module, ScopeKind::Block, scope, symbols);
                let initialization = initialization.map(|initialization| {
                    self.bind_expression(
                        module,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        initialization,
                        tree,
                        symbols,
                        types,
                    )
                });
                let condition = condition.map(|condition| {
                    self.bind_expression(
                        module,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        condition,
                        tree,
                        symbols,
                        types,
                    )
                });
                let increment = increment.map(|increment| {
                    self.bind_expression(
                        module,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        increment,
                        tree,
                        symbols,
                        types,
                    )
                });
                let body = self.bind_block(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *body,
                    tree,
                    symbols,
                    types,
                );
                Expression::For {
                    initialization,
                    condition,
                    increment,
                    body,
                    scope: scope_id,
                    symbol: symbol_id,
                }
            }
            ast::Expression::Loop { body } => {
                let (symbol_id, scope_id) =
                    self.bind_anonymous_item_with_scope(module, ScopeKind::Block, scope, symbols);
                let body = self.bind_block(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *body,
                    tree,
                    symbols,
                    types,
                );
                Expression::Loop {
                    kind: LoopKind::NoTest,
                    condition: None,
                    body,
                    scope: scope_id,
                    symbol: symbol_id,
                }
            }
            ast::Expression::Try {
                try_expression,
                catch_pattern,
                catch_expression,
                finally_expression,
            } => {
                let (symbol_id, scope_id) =
                    self.bind_anonymous_item_with_scope(module, ScopeKind::Block, scope, symbols);
                let try_expression = self.bind_expression(
                    module,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *try_expression,
                    tree,
                    symbols,
                    types,
                );
                let catch_pattern = catch_pattern.map(|catch_pattern| {
                    self.bind_pattern(
                        module,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        catch_pattern,
                        tree,
                        symbols,
                        types,
                    )
                });
                let catch_expression = catch_expression.map(|catch_expression| {
                    self.bind_expression(
                        module,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        catch_expression,
                        tree,
                        symbols,
                        types,
                    )
                });
                let finally_expression = finally_expression.map(|finally_expression| {
                    self.bind_expression(
                        module,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        finally_expression,
                        tree,
                        symbols,
                        types,
                    )
                });
                Expression::Try {
                    try_expression,
                    catch_pattern,
                    catch_expression,
                    finally_expression,
                    scope: scope_id,
                    symbol: symbol_id,
                }
            }
            ast::Expression::Match {
                kind: _,
                value,
                cases,
            } => {
                let value = self.bind_expression(module, scope, *value, tree, symbols, types);
                let (symbol_id, scope_id) =
                    self.bind_anonymous_item_with_scope(module, ScopeKind::Block, scope, symbols);
                let cases = cases
                    .iter()
                    .map(|case| {
                        self.bind_match_case(
                            module,
                            (scope_id, symbols.get_scope_mark(scope_id)),
                            *case,
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                Expression::Match {
                    value,
                    cases,
                    source: MatchSource::Match,
                    scope: scope_id,
                    symbol: symbol_id,
                }
            }

            ast::Expression::Break { label, value } => {
                let label =
                    label.map(|label| self.program.strings.intern_from(&module.ast_strings, label));
                let value = value
                    .map(|value| self.bind_expression(module, scope, value, tree, symbols, types));
                Expression::UnresolvedBreak {
                    target: label,
                    value,
                }
            }
            ast::Expression::Continue { label } => {
                let label =
                    label.map(|label| self.program.strings.intern_from(&module.ast_strings, label));
                Expression::UnresolvedContinue { target: label }
            }
            ast::Expression::Return { value } => {
                let value = value
                    .map(|value| self.bind_expression(module, scope, value, tree, symbols, types));
                Expression::Return { value }
            }
            ast::Expression::Defer { expression } => {
                let expression =
                    self.bind_expression(module, scope, *expression, tree, symbols, types);
                Expression::Defer { expression }
            }
            ast::Expression::Await { expression } => {
                let expression =
                    self.bind_expression(module, scope, *expression, tree, symbols, types);
                Expression::Await { expression }
            }
            ast::Expression::Yield { cardinality, value } => {
                let cardinality = match *cardinality {
                    ast::YieldCardinality::Generator => YieldCardinality::Generator,
                    ast::YieldCardinality::Scalar => YieldCardinality::Scalar,
                };
                let value = self.bind_expression(module, scope, *value, tree, symbols, types);
                Expression::Yield { cardinality, value }
            }
            ast::Expression::Throw { value } => {
                let value = value
                    .map(|value| self.bind_expression(module, scope, value, tree, symbols, types));
                Expression::Throw { value }
            }

            ast::Expression::Error => Expression::Error,
        };

        // expression
        if let Some(symbol_id) = expression.symbol() {
            let expression_id = tree.insert_from_source(expression, expression_id, scope);
            symbols
                .get_symbol_mut(symbol_id)
                .declare_primary(expression_id);
            expression_id
        } else {
            tree.insert_from_source(expression, expression_id, scope)
        }
    }
}
