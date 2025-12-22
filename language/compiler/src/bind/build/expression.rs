use destack_ast::{self as ast};
use destack_dir::{
    DeclarationKind, Declarator, DependencyMode, DependencySource, Expression, ForEachKind, IfKind,
    LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalScopeMark, LoopKind, MatchSource, NodeTree,
    NodeType, ScopeKind, StaticKey, SymbolBinding, SymbolKind, SymbolSpace, SymbolTable,
    SymbolType, TypeTable, YieldCardinality,
};
use destack_workspace::{Module, ModuleAst};

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

    destack_base::ensure_sufficient_stack! {
        /// Bind an expression to a DIR expression.
        pub(super) fn bind_expression(
            &self,
            module: &Module,
            ast: &ModuleAst,
            scope: (LocalScopeId, LocalScopeMark),
            ast_expression_id: ast::LocalNodeId<ast::Expression>,
            parent_id: Option<LocalNodeIdAny>,
            tree: &mut NodeTree,
            symbols: &mut SymbolTable,
            types: &mut TypeTable,
        ) -> LocalNodeId<Expression> {
            let ast_expression = ast.tree.get(ast_expression_id);
        let expression_id =
            tree.reserve_from_source(NodeType::Expression, ast_expression_id.id, scope, parent_id);
        let expression = match ast_expression {
            ast::Expression::Block(block_id) => {
                let block_id = self.bind_block(
                    module,
                    ast,
                    scope,
                    *block_id,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                Expression::Block { block: block_id }
            }
            ast::Expression::Declaration(declaration_id) => {
                // declarations can bind to entire containing scope
                let scope = (scope.0, LocalScopeMark::end());
                let declaration_id = self.bind_declaration(
                    module,
                    ast,
                    scope,
                    *declaration_id,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                Expression::Declaration {
                    declaration: declaration_id,
                }
            }
            ast::Expression::Statement(inner_expression_id) => {
                let inner_expression_id = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *inner_expression_id,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                Expression::Statement {
                    statement: inner_expression_id,
                }
            }

            ast::Expression::Labelled { label, body } => {
                let label = self
                    .program
                    .strings
                    .intern_from(&ast.strings, *label);
                let (symbol_id, _) = symbols.insert_symbol(
                    SymbolKind::Local,
                    SymbolType::Void,
                    SymbolSpace::Label,
                    SymbolBinding::Runtime,
                    Some(StaticKey::Name(label)),
                    scope,
                    None,
                );
                let body_id = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *body,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                Expression::Labelled {
                    label,
                    body: body_id,
                    symbol: symbol_id,
                }
            }

            ast::Expression::Import {
                kind,
                target,
                items,
                arguments,
            } => {
                // Import equals (`import A = B.C`) is lowered to Let in the parser.
                let target = self
                    .program
                    .strings
                    .intern_from(&ast.strings, *target);
                // items
                let items: Vec<_> = items
                    .iter()
                    .map(|item| {
                        self.bind_dependency_item(
                            module,
                            ast,
                            scope,
                            DependencySource::ImportStatement,
                            *kind,
                            Some(target),
                            *item,
                            Some(expression_id),
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
                            self.bind_argument(
                                module,
                                ast,
                                scope,
                                *argument,
                                Some(expression_id),
                                tree,
                                symbols,
                                types,
                            )
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
                        .intern_from(&ast.strings, target)
                });
                // re-export from import
                if let Some(target) = target {
                    // items
                    let items: Vec<_> = items
                        .iter()
                        .map(|item| {
                            self.bind_dependency_item(
                                module,
                                ast,
                                scope,
                                DependencySource::ExportStatement,
                                *kind,
                                Some(target),
                                *item,
                                Some(expression_id),
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
                                    ast,
                                    scope,
                                    DependencySource::ValueExpression,
                                    *kind,
                                    None,
                                    *item,
                                    Some(expression_id),
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
                descriptor,
                mutability,
                declarators: ast_declarators,
                ..
            } => {
                let symbol_kind = if descriptor.export.is_some() {
                    SymbolKind::Item
                } else {
                    SymbolKind::Local
                };
                let (descriptor, _) = self.bind_declaration_descriptor(
                    module,
                    ast,
                    scope,
                    descriptor,
                    symbol_kind,
                    SymbolType::Void,
                    symbols,
                );
                let symbol_id = descriptor.symbol;
                let binding = match descriptor.kind {
                    DeclarationKind::Declaration => SymbolBinding::Ambient,
                    DeclarationKind::Definition => SymbolBinding::Runtime,
                };
                let mutability = self.bind_mutability(*mutability);
                let declarators: Vec<LocalNodeId<Declarator>> = ast_declarators
                    .iter()
                    .map(|ast_decl_id| {
                        self.bind_declarator(
                            module,
                            ast,
                            scope,
                            descriptor.export,
                            binding,
                            *ast_decl_id,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();

                let expression = Expression::Let {
                    descriptor,
                    mutability,
                    declarators,
                };
                let expression_id = tree.insert(expression_id, expression);
                symbols
                    .get_symbol_mut(symbol_id)
                    .declare_primary(expression_id);
                return expression_id;
            }
            ast::Expression::Unary { operator, right } => {
                let right = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *right,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                let operator = self.bind_unary_operator(*operator);
                Expression::Unary { operator, right }
            }

            ast::Expression::TypeUnary { operator, right } => {
                let right = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *right,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
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
                let right = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *right,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
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
                let right = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *right,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
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
                let left = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *left,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                let right = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *right,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
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
                let left = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *left,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                let right = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *right,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
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
                let left = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *left,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                let right = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *right,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
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
                let left = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *left,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                let name = self.program.strings.intern_from(&ast.strings, *name);
                let static_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.bind_argument(
                                module,
                                ast,
                                scope,
                                *argument,
                                Some(expression_id),
                                tree,
                                symbols,
                                types,
                            )
                        })
                        .collect()
                });
                Expression::Member {
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
                let left = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *left,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                let static_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.bind_argument(
                                module,
                                ast,
                                scope,
                                *argument,
                                Some(expression_id),
                                tree,
                                symbols,
                                types,
                            )
                        })
                        .collect()
                });
                let dynamic_arguments = dynamic_arguments
                    .iter()
                    .map(|argument| {
                        self.bind_argument(
                            module,
                            ast,
                            scope,
                            *argument,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                        )
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
                let left = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *left,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                let static_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.bind_argument(
                                module,
                                ast,
                                scope,
                                *argument,
                                Some(expression_id),
                                tree,
                                symbols,
                                types,
                            )
                        })
                        .collect()
                });
                let dynamic_arguments = dynamic_arguments
                    .iter()
                    .map(|argument| {
                        self.bind_argument(
                            module,
                            ast,
                            scope,
                            *argument,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                Expression::New {
                    left,
                    static_arguments,
                    dynamic_arguments,
                }
            }
            ast::Expression::Delete { value } => {
                let value = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *value,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                Expression::Delete { value }
            }
            ast::Expression::Index {
                position: _,
                left,
                index,
            } => {
                let left = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *left,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                let index = index.map(|index| {
                    self.bind_expression(
                        module,
                        ast,
                        scope,
                        index,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                Expression::Index { left, right: index }
            }
            ast::Expression::Maybe { position: _, left } => {
                let left = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *left,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                Expression::Maybe { left }
            }
            ast::Expression::Must { position: _, left } => {
                let left = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *left,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                Expression::Must { left }
            }

            ast::Expression::Path {
                path,
                static_arguments,
            } => {
                let path = self.bind_path(module, ast, path);
                let static_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.bind_argument(
                                module,
                                ast,
                                scope,
                                *argument,
                                Some(expression_id),
                                tree,
                                symbols,
                                types,
                            )
                        })
                        .collect()
                });
                Expression::UnresolvedPath {
                    path,
                    static_arguments,
                }
            }
            ast::Expression::ScalarLiteral(value) => {
                let value = self.bind_scalar_literal(module, ast, value);
                Expression::ScalarLiteral { value }
            }
            ast::Expression::TemplateExpression { value } => {
                let value = self.bind_template_literal(
                    module,
                    ast,
                    scope,
                    value,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                Expression::TemplateExpression { value }
            }
            ast::Expression::TaggedTemplateExpression { tag, value } => {
                let tag = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *tag,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                let value = self.bind_template_literal(
                    module,
                    ast,
                    scope,
                    value,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                Expression::TaggedTemplateExpression { tag, value }
            }
            ast::Expression::TypeLiteral(value) => {
                let value = self.bind_type_literal(value);
                Expression::TypeLiteral { value }
            }
            ast::Expression::RangeExpression {
                start,
                end,
                is_inclusive,
            } => {
                let start = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *start,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                let end = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *end,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                Expression::RangeExpression {
                    start,
                    end,
                    is_inclusive: *is_inclusive,
                }
            }
            ast::Expression::ObjectExpression { ty, properties } => {
                let properties = properties
                    .iter()
                    .map(|property| {
                        self.bind_property(
                            module,
                            ast,
                            scope,
                            *property,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                if let Some(ty_id) = ty {
                    let ty = self.bind_expression(
                        module,
                        ast,
                        scope,
                        *ty_id,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                    );
                    Expression::TaggedObjectExpression { ty, properties }
                } else {
                    Expression::ObjectExpression { properties }
                }
            }
            ast::Expression::TupleExpression { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| {
                        self.bind_argument(
                            module,
                            ast,
                            scope,
                            *element,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                Expression::TupleExpression { elements }
            }
            ast::Expression::SequenceExpression { expressions } => {
                let expressions = expressions
                    .iter()
                    .map(|expr_id| {
                        self.bind_expression(
                            module,
                            ast,
                            scope,
                            *expr_id,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                Expression::SequenceExpression { expressions }
            }
            ast::Expression::ArrayExpression { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| {
                        self.bind_argument(
                            module,
                            ast,
                            scope,
                            *element,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();
                Expression::ArrayExpression { elements }
            }
            ast::Expression::TreeExpression {
                left,
                arguments,
                elements,
            } => {
                let left = left.map(|left| {
                    self.bind_expression(
                        module,
                        ast,
                        scope,
                        left,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                let arguments = arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.bind_argument(
                                module,
                                ast,
                                scope,
                                *argument,
                                Some(expression_id),
                                tree,
                                symbols,
                                types,
                            )
                        })
                        .collect()
                });
                let elements = elements.as_ref().map(|elements| {
                    elements
                        .iter()
                        .map(|element| {
                            self.bind_argument(
                                module,
                                ast,
                                scope,
                                *element,
                                Some(expression_id),
                                tree,
                                symbols,
                                types,
                            )
                        })
                        .collect()
                });
                Expression::TreeExpression {
                    left,
                    arguments,
                    elements,
                }
            }
            ast::Expression::Parenthesized { expression } => {
                let expression = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *expression,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                Expression::Parenthesized { expression }
            }

            ast::Expression::If {
                kind,
                condition,
                then_expression,
                else_expression,
            } => {
                let kind = self.bind_if_kind(*kind);
                let condition = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *condition,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                let then_expression = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *then_expression,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                let else_expression = else_expression.map(|else_expression| {
                    self.bind_expression(
                        module,
                        ast,
                        scope,
                        else_expression,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                    )
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
                let condition = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *condition,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                let (symbol_id, scope_id) = self.bind_anonymous_item_with_scope(
                    module,
                    ast,
                    ScopeKind::Block,
                    scope,
                    None,
                    symbols,
                );
                let body = self.bind_block(
                    module,
                    ast,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *body,
                    Some(expression_id),
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
                let iterator = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *iterator,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                let (symbol_id, scope_id) = self.bind_anonymous_item_with_scope(
                    module,
                    ast,
                    ScopeKind::Block,
                    scope,
                    None,
                    symbols,
                );
                let pattern = self.bind_pattern(
                    module,
                    ast,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    None,
                    SymbolBinding::Runtime,
                    *pattern,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                let body = self.bind_block(
                    module,
                    ast,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *body,
                    Some(expression_id),
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
                let (symbol_id, scope_id) = self.bind_anonymous_item_with_scope(
                    module,
                    ast,
                    ScopeKind::Block,
                    scope,
                    None,
                    symbols,
                );
                let initialization = initialization.map(|initialization| {
                    self.bind_expression(
                        module,
                        ast,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        initialization,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                let condition = condition.map(|condition| {
                    self.bind_expression(
                        module,
                        ast,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        condition,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                let increment = increment.map(|increment| {
                    self.bind_expression(
                        module,
                        ast,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        increment,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                let body = self.bind_block(
                    module,
                    ast,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *body,
                    Some(expression_id),
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
                let (symbol_id, scope_id) = self.bind_anonymous_item_with_scope(
                    module,
                    ast,
                    ScopeKind::Block,
                    scope,
                    None,
                    symbols,
                );
                let body = self.bind_block(
                    module,
                    ast,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *body,
                    Some(expression_id),
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
                let (symbol_id, scope_id) = self.bind_anonymous_item_with_scope(
                    module,
                    ast,
                    ScopeKind::Block,
                    scope,
                    None,
                    symbols,
                );
                let try_expression = self.bind_expression(
                    module,
                    ast,
                    (scope_id, symbols.get_scope_mark(scope_id)),
                    *try_expression,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                let catch_pattern = catch_pattern.map(|catch_pattern| {
                    self.bind_pattern(
                        module,
                        ast,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        None,
                        SymbolBinding::Runtime,
                        catch_pattern,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                let catch_expression = catch_expression.map(|catch_expression| {
                    self.bind_expression(
                        module,
                        ast,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        catch_expression,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                let finally_expression = finally_expression.map(|finally_expression| {
                    self.bind_expression(
                        module,
                        ast,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        finally_expression,
                        Some(expression_id),
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
                let value = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *value,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                let (symbol_id, scope_id) = self.bind_anonymous_item_with_scope(
                    module,
                    ast,
                    ScopeKind::Block,
                    scope,
                    None,
                    symbols,
                );
                let cases = cases
                    .iter()
                    .map(|case| {
                        self.bind_match_case(
                            module,
                            ast,
                            (scope_id, symbols.get_scope_mark(scope_id)),
                            *case,
                            Some(expression_id),
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
                    label.map(|label| self.program.strings.intern_from(&ast.strings, label));
                let value = value.map(|value| {
                    self.bind_expression(
                        module,
                        ast,
                        scope,
                        value,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                if let Some(label) = label {
                    Expression::UnresolvedBreak {
                        target: label,
                        value,
                    }
                } else {
                    Expression::Break {
                        target: None,
                        target_symbol: None,
                        value,
                    }
                }
            }
            ast::Expression::Continue { label } => {
                let label =
                    label.map(|label| self.program.strings.intern_from(&ast.strings, label));
                if let Some(label) = label {
                    Expression::UnresolvedContinue { target: label }
                } else {
                    Expression::Continue {
                        target: None,
                        target_symbol: None,
                    }
                }
            }
            ast::Expression::Return { value } => {
                let value = value.map(|value| {
                    self.bind_expression(
                        module,
                        ast,
                        scope,
                        value,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                Expression::Return { value }
            }
            ast::Expression::Await { expression } => {
                let expression = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *expression,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                Expression::Await { expression }
            }
            ast::Expression::AwaitMaybe { expression } => {
                let expression = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *expression,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                Expression::AwaitMaybe { expression }
            }
            ast::Expression::Comptime { body } => {
                let body = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *body,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                Expression::Comptime { body }
            }
            ast::Expression::Yield { cardinality, value } => {
                let cardinality = match *cardinality {
                    ast::YieldCardinality::Generator => YieldCardinality::Generator,
                    ast::YieldCardinality::Scalar => YieldCardinality::Scalar,
                };
                let value = value.map(|value| {
                    self.bind_expression(
                        module,
                        ast,
                        scope,
                        value,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                Expression::Yield { cardinality, value }
            }
            ast::Expression::Throw { value } => {
                let value = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *value,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                );
                Expression::Throw { value }
            }

            ast::Expression::Debugger => Expression::Debugger,
            ast::Expression::Stub => Expression::Stub,
            ast::Expression::Error => Expression::Error,
        };

        // expression
        if let Some(symbol_id) = expression.symbol() {
            let expression_id = tree.insert(expression_id, expression);
            symbols
                .get_symbol_mut(symbol_id)
                .declare_primary(expression_id);
            expression_id
        } else {
            tree.insert(expression_id, expression)
        }
        }
    }

    /// Bind a declarator to a DIR declarator.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn bind_declarator(
        &self,
        module: &Module,
        ast: &ModuleAst,
        scope: (LocalScopeId, LocalScopeMark),
        export: Option<DependencyMode>,
        binding: SymbolBinding,
        ast_declarator_id: ast::LocalNodeId<ast::Declarator>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Declarator> {
        let ast_declarator = ast.tree.get(ast_declarator_id);
        let declarator_id =
            tree.reserve_from_source(NodeType::Declarator, ast_declarator_id.id, scope, parent_id);
        let ast::Declarator { pattern, ty, value } = ast_declarator;

        let pattern = self.bind_pattern(
            module,
            ast,
            scope,
            export,
            binding,
            *pattern,
            Some(declarator_id),
            tree,
            symbols,
            types,
        );
        let bound_ty = ty.map(|ty_id| {
            self.bind_expression(
                module,
                ast,
                scope,
                ty_id,
                Some(declarator_id),
                tree,
                symbols,
                types,
            )
        });
        let value = value.map(|v| {
            self.bind_expression(
                module,
                ast,
                scope,
                v,
                Some(declarator_id),
                tree,
                symbols,
                types,
            )
        });
        // set declared type on declarator if type annotation is present
        if let Some(ty_id) = ty {
            let declared_ty = self.bind_expression_to_type(
                module,
                ast,
                scope,
                *ty_id,
                Some(declarator_id),
                tree,
                symbols,
                types,
            );
            types.set_declared_type(declarator_id.into_global(module.id), declared_ty);
        }
        let declarator = Declarator {
            pattern,
            ty: bound_ty,
            value,
        };
        tree.insert(declarator_id, declarator)
    }
}
