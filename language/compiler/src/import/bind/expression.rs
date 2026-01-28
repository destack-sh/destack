use destack_ast::{self as ast};
use destack_dir::{
    DeclarationKind, Declarator, DependencyMode, DependencySource, Expression, ForEachBinding,
    ForEachKind, IfCondition, IfKind, LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalScopeMark,
    LoopKind, MatchKind, MatchSource, Mutability, NodeTree, NodeType, ScopeKind, StaticKey,
    SymbolBinding, SymbolKind, SymbolSpace, SymbolSpaceOrder, SymbolTable, SymbolType, Type,
    TypeMappedParameterExpression, TypePredicateSubject, TypeTable, YieldCardinality,
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
        /// Bind an expression to a DIR expression with a symbol space order.
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
            space_order: SymbolSpaceOrder,
        ) -> LocalNodeId<Expression> {
            let ast_expression = ast.tree.get(ast_expression_id);

            let expression_id = tree.reserve_from_source(
                NodeType::Expression,
                ast_expression_id.id,
                scope,
                parent_id,
            );

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
                    space_order,
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
                    space_order,
                );
                Expression::Labelled {
                    label,
                    body: body_id,
                    symbol: symbol_id,
                }
            }

            ast::Expression::Import {
                source,
                kind,
                target,
                items,
                arguments,
            } => {
                let target = self
                    .program
                    .strings
                    .intern_from(&ast.strings, *target);
                let source = self.bind_dependency_source(*source);
                // items
                let items: Vec<_> = items
                    .iter()
                    .map(|item| {
                        self.bind_dependency_item(
                            module,
                            ast,
                            scope,
                            source,
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
                                SymbolSpaceOrder::ValueThenType,
                            )
                        })
                        .collect()
                });
                let kind = self.bind_dependency_kind(*kind);
                // import
                Expression::UnresolvedImport {
                    source,
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
            ast::Expression::ExportNamespace { name } => {
                let name = self
                    .program
                    .strings
                    .intern_from(&ast.strings, *name);
                Expression::ExportNamespace { name }
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
                            Some(mutability),
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
            ast::Expression::Using {
                asynchrony,
                descriptor,
                declarators: ast_declarators,
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
                let asynchrony = self.bind_asynchrony(*asynchrony);
                let mutability = Mutability::Immutable;
                let declarators: Vec<LocalNodeId<Declarator>> = ast_declarators
                    .iter()
                    .map(|ast_decl_id| {
                        self.bind_declarator(
                            module,
                            ast,
                            scope,
                            descriptor.export,
                            binding,
                            Some(mutability),
                            *ast_decl_id,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();

                let expression = Expression::Using {
                    asynchrony,
                    descriptor,
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
                    space_order,
                );
                let operator = self.bind_unary_operator(*operator);
                Expression::Unary { operator, right }
            }

            ast::Expression::TypeUnary { operator, right } => {
                let space_order = if matches!(operator, ast::TypeUnaryOperator::Typeof) {
                    SymbolSpaceOrder::ValueOnly
                } else {
                    space_order
                };
                let right = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *right,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space_order,
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
                    space_order,
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
                    space_order,
                );
                Expression::ReferenceOf {
                    mutability,
                    variance,
                    right,
                }
            }
            ast::Expression::PointerOf { mutability, right } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let right = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *right,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space_order,
                );
                Expression::PointerOf { mutability, right }
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
                    space_order,
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
                    space_order,
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
                    space_order,
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
                    space_order,
                );
                let operator = self.bind_type_binary_operator(*operator);
                Expression::TypeBinary {
                    left,
                    operator,
                    right,
                }
            }
            ast::Expression::TypeConditional {
                left,
                right,
                then_type,
                else_type,
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
                    space_order,
                );
                let type_scope_id = symbols.insert_scope(ScopeKind::Type, Some(scope), None);
                let type_scope = (type_scope_id, symbols.get_scope_mark(type_scope_id));
                let right = self.bind_expression(
                    module,
                    ast,
                    type_scope,
                    *right,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space_order,
                );
                let type_scope = (type_scope_id, symbols.get_scope_mark(type_scope_id));
                let then_type = self.bind_expression(
                    module,
                    ast,
                    type_scope,
                    *then_type,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space_order,
                );
                let else_type = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *else_type,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space_order,
                );
                Expression::TypeConditional {
                    left,
                    right,
                    then_type,
                    else_type,
                }
            }
            ast::Expression::TypeMapped {
                parameter,
                modifiers,
                value,
            } => {
                let name = self
                    .program
                    .strings
                    .intern_from(&ast.strings, parameter.name);
                let constraint = self.bind_expression(
                    module,
                    ast,
                    scope,
                    parameter.constraint,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space_order,
                );
                let parameter_scope_id = symbols.insert_scope(ScopeKind::Type, Some(scope), None);
                let parameter_scope = (
                    parameter_scope_id,
                    symbols.get_scope_mark(parameter_scope_id),
                );
                let (symbol, _) = self.bind_named_local(
                    module,
                    ast,
                    SymbolSpace::Type,
                    StaticKey::Name(name),
                    parameter_scope,
                    symbols,
                );
                let parameter_scope = (
                    parameter_scope_id,
                    symbols.get_scope_mark(parameter_scope_id),
                );
                let key_remap = parameter.key_remap.map(|key_remap| {
                    self.bind_expression(
                        module,
                        ast,
                        parameter_scope,
                        key_remap,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                        space_order,
                    )
                });
                let parameter = TypeMappedParameterExpression {
                    name,
                    symbol,
                    constraint,
                    key_remap,
                };
                let modifiers = self.bind_type_mapped_modifiers(modifiers);
                let value = self.bind_expression(
                    module,
                    ast,
                    parameter_scope,
                    *value,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space_order,
                );
                Expression::TypeMapped {
                    parameter,
                    modifiers,
                    value,
                }
            }
            ast::Expression::TypeIndex { left, index } => {
                let left = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *left,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space_order,
                );
                let index = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *index,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space_order,
                );
                Expression::TypeIndex { left, index }
            }
            ast::Expression::TypeTemplateLiteral { strings, spans } => {
                let strings = strings
                    .iter()
                    .map(|string| self.program.strings.intern_from(&ast.strings, *string))
                    .collect();
                let spans = spans
                    .iter()
                    .map(|span| {
                        self.bind_expression(
                            module,
                            ast,
                            scope,
                            *span,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                            space_order,
                        )
                    })
                    .collect();
                Expression::TypeTemplateLiteral { strings, spans }
            }
            ast::Expression::TypeImport {
                target,
                qualifier,
                static_arguments,
            } => {
                let target = self.program.strings.intern_from(&ast.strings, *target);
                let qualifier = qualifier
                    .as_ref()
                    .map(|path| self.bind_path(module, ast, path));
                let static_argument_space_order = if module.language_type.is_destack() {
                    SymbolSpaceOrder::ValueThenType
                } else {
                    SymbolSpaceOrder::TypeThenValue
                };
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
                                static_argument_space_order,
                            )
                        })
                        .collect()
                });
                Expression::TypeImport {
                    target,
                    qualifier,
                    static_arguments,
                }
            }
            ast::Expression::TypeInfer { name, constraint } => {
                let name = self.program.strings.intern_from(&ast.strings, *name);
                let constraint = constraint.map(|constraint| {
                    self.bind_expression(
                        module,
                        ast,
                        scope,
                        constraint,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                        space_order,
                    )
                });
                let infer_scope = self
                    .find_nearest_scope_of_kind(symbols, scope, ScopeKind::Type)
                    .unwrap_or(scope);
                let _ = self.bind_named_local(
                    module,
                    ast,
                    SymbolSpace::Type,
                    StaticKey::Name(name),
                    infer_scope,
                    symbols,
                );
                Expression::TypeInfer { name, constraint }
            }
            ast::Expression::TypePredicate {
                asserts,
                subject,
                target,
            } => {
                let subject = match subject {
                    ast::TypePredicateSubject::Identifier(name) => {
                        let name = self.program.strings.intern_from(&ast.strings, *name);
                        TypePredicateSubject::Unresolved(name)
                    }
                    ast::TypePredicateSubject::This => TypePredicateSubject::This,
                };
                let target = target.map(|target| {
                    self.bind_expression(
                        module,
                        ast,
                        scope,
                        target,
                        Some(expression_id),
                        tree,
                        symbols,
                        types,
                        space_order,
                    )
                });
                Expression::TypePredicate {
                    asserts: *asserts,
                    subject,
                    target,
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
                    space_order,
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
                    space_order,
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
                    space_order,
                );
                let name = self.program.strings.intern_from(&ast.strings, *name);
                let static_argument_space_order = if module.language_type.is_destack() {
                    SymbolSpaceOrder::ValueThenType
                } else {
                    SymbolSpaceOrder::TypeThenValue
                };
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
                                static_argument_space_order,
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
                    space_order,
                );
                let static_argument_space_order = if module.language_type.is_destack() {
                    SymbolSpaceOrder::ValueThenType
                } else {
                    SymbolSpaceOrder::TypeThenValue
                };
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
                                static_argument_space_order,
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
                            SymbolSpaceOrder::ValueThenType,
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
                    space_order,
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
                                SymbolSpaceOrder::TypeThenValue,
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
                            SymbolSpaceOrder::ValueThenType,
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
                    space_order,
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
                    space_order,
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
                        space_order,
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
                    space_order,
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
                    space_order,
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
                                SymbolSpaceOrder::TypeThenValue,
                            )
                        })
                        .collect()
                });
                Expression::UnresolvedPath {
                    path,
                    static_arguments,
                    space_order,
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
                    space_order,
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
                    space_order,
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
                    space_order,
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
                        space_order,
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
                            SymbolSpaceOrder::ValueThenType,
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
                            space_order,
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
                            SymbolSpaceOrder::ValueThenType,
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
                        space_order,
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
                                SymbolSpaceOrder::ValueThenType,
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
                                SymbolSpaceOrder::ValueThenType,
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
                    space_order,
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
                let (condition, then_expression, else_expression) = match condition {
                    ast::IfCondition::Expression { condition } => {
                        let condition = self.bind_expression(
                            module,
                            ast,
                            scope,
                            *condition,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                            space_order,
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
                            space_order,
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
                                space_order,
                            )
                        });
                        (
                            IfCondition::Expression { condition },
                            then_expression,
                            else_expression,
                        )
                    }
                    ast::IfCondition::Let {
                        kind: _,
                        mutability,
                        declarator,
                    } => {
                        let mutability = self.bind_mutability(*mutability);
                        let outer_scope = scope;
                        let if_scope_id =
                            symbols.insert_scope(ScopeKind::Block, Some(outer_scope), None);
                        let if_scope = (if_scope_id, symbols.get_scope_mark(if_scope_id));
                        let declarator = self.bind_declarator(
                            module,
                            ast,
                            if_scope,
                            None,
                            SymbolBinding::Runtime,
                            Some(mutability),
                            *declarator,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                        );
                        let if_scope = (if_scope_id, symbols.get_scope_mark(if_scope_id));
                        let then_expression = self.bind_expression(
                            module,
                            ast,
                            if_scope,
                            *then_expression,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                            space_order,
                        );
                        let else_expression = else_expression.map(|else_expression| {
                            let else_scope_id =
                                symbols.insert_scope(ScopeKind::Block, Some(outer_scope), None);
                            let else_scope = (else_scope_id, symbols.get_scope_mark(else_scope_id));
                            self.bind_expression(
                                module,
                                ast,
                                else_scope,
                                else_expression,
                                Some(expression_id),
                                tree,
                                symbols,
                                types,
                                space_order,
                            )
                        });
                        (
                            IfCondition::Let {
                                mutability,
                                declarator,
                            },
                            then_expression,
                            else_expression,
                        )
                    }
                };
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
                    space_order,
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
                binding,
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
                    space_order,
                );
                let (symbol_id, scope_id) = self.bind_anonymous_item_with_scope(
                    module,
                    ast,
                    ScopeKind::Block,
                    scope,
                    None,
                    symbols,
                );
                let binding = match binding {
                    ast::ForEachBinding::Pattern { pattern } => {
                        let pattern = self.bind_pattern(
                            module,
                            ast,
                            (scope_id, symbols.get_scope_mark(scope_id)),
                            None,
                            SymbolBinding::Runtime,
                            None,
                            *pattern,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                        );
                        ForEachBinding::Pattern { pattern }
                    }
                    ast::ForEachBinding::Using { asynchrony, pattern } => {
                        let pattern = self.bind_pattern(
                            module,
                            ast,
                            (scope_id, symbols.get_scope_mark(scope_id)),
                            None,
                            SymbolBinding::Runtime,
                            Some(Mutability::Immutable),
                            *pattern,
                            Some(expression_id),
                            tree,
                            symbols,
                            types,
                        );
                        ForEachBinding::Using {
                            asynchrony: self.bind_asynchrony(*asynchrony),
                            pattern,
                        }
                    }
                };
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
                    binding,
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
                        space_order,
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
                        space_order,
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
                        space_order,
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
                    space_order,
                );
                let catch_pattern = catch_pattern.map(|catch_pattern| {
                    self.bind_pattern(
                        module,
                        ast,
                        (scope_id, symbols.get_scope_mark(scope_id)),
                        None,
                        SymbolBinding::Runtime,
                        None,
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
                        space_order,
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
                        space_order,
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
            ast::Expression::Match { kind, value, cases } => {
                let value = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *value,
                    Some(expression_id),
                    tree,
                    symbols,
                    types,
                    space_order,
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
                let kind = match *kind {
                    ast::MatchKind::Match => MatchKind::Match,
                    ast::MatchKind::Switch => MatchKind::Switch,
                };
                Expression::Match {
                    kind,
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
                        space_order,
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
                        space_order,
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
                    space_order,
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
                    space_order,
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
                    space_order,
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
                        space_order,
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
                    space_order,
                );
                Expression::Throw { value }
            }

            ast::Expression::This => Expression::This,
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
        binding_mutability: Option<Mutability>,
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
            binding_mutability,
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
                SymbolSpaceOrder::TypeThenValue,
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
                SymbolSpaceOrder::ValueThenType,
            )
        });
        // set declared type on declarator if type annotation is present
        if let Some(ty_id) = bound_ty {
            let declared_ty = types.insert_type_from(Type::Unevaluated(ty_id), ty_id);
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

#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;
    use crate::{assert_node, assert_path};
    use destack_dir::{
        Declarator, Expression, StaticKey, SymbolSpace, SymbolSpaceOrder, SymbolTable,
    };

    // Test that infer type variables are visible in the then-branch of conditional types.
    #[test]
    fn test_bind_infer_type_variable_in_conditional_type() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.d.ts",
            r#"
type ThisParameterType<T> = T extends (this: infer U, ...args: never) => any ? U : unknown;
type OmitThisParameter<T> = unknown extends ThisParameterType<T> ? T : T extends (...args: infer A) => infer R ? (...args: A) => R : T;
"#,
        );
        test.bind_module(module_id);
        test.compile();
        test.check_clean();
    }

    // Test that infer in nested positions (object types, function types) works correctly.
    #[test]
    fn test_bind_infer_type_variable_nested_in_object_type() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.d.ts",
            r#"
type ExtractCallback<T> = T extends { callback: (x: infer U) => void } ? U : never;
"#,
        );
        test.bind_module(module_id);
        test.compile();
        test.check_clean();
    }

    // check declarator paths use explicit space order
    #[test]
    fn test_bind_declarator_space_order() {
        // setup module
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
let Foo: Foo = Foo;
"#,
        );

        // bind and compile
        test.bind_module(module_id);
        test.compile();
        test.check_clean();

        // load bound tree
        let module = test.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir_base();
        let tree = dir.tree.read();

        // select the root expression
        let root_id = test.expect_root_expression(module_id);

        // assert declarator space order
        assert_node!(
            tree,
            root_id,
            Expression::Let { declarators, .. } => {
                let declarator_id = declarators.first().copied().expect("expected declarator");
                assert_node!(
                    tree,
                    declarator_id,
                    Declarator {
                        ty: Some(ty_id),
                        value: Some(value_id),
                        ..
                    } => {
                        assert_node!(
                            tree,
                            *ty_id,
                            Expression::UnresolvedPath {
                                path,
                                static_arguments: _,
                                space_order,
                            } => {
                                assert_path!(test.program, path, "Foo");
                                assert_eq!(*space_order, SymbolSpaceOrder::TypeThenValue);
                            }
                        );

                        assert_node!(
                            tree,
                            *value_id,
                            Expression::UnresolvedPath {
                                path,
                                static_arguments: _,
                                space_order,
                            } => {
                                assert_path!(test.program, path, "Foo");
                                assert_eq!(*space_order, SymbolSpaceOrder::ValueThenType);
                            }
                        );
                    }
                );
            }
        );
    }

    // ensure import type bindings use type space only
    #[test]
    fn test_bind_import_type_uses_type_space() {
        let test = TestProgram::memory_sequential();
        let _dep_id = test.add_module("dep.d.ts", "export type Foo = number;");
        let main_id = test.add_module("main.d.ts", r#"import type { Foo } from "./dep";"#);

        test.bind_module(main_id);
        test.compile();
        test.check_clean();

        let module = test.program.modules.get(main_id);
        let module = module.read();
        let dir = module.dir_base();
        let symbols = dir.symbols.read();
        let name = test.program.strings.intern("Foo");
        let key = StaticKey::Name(name);
        let (type_count, value_count, type_value_count) = count_symbol_spaces(&symbols, key);

        assert_eq!(type_count, 1);
        assert_eq!(value_count, 0);
        assert_eq!(type_value_count, 0);
    }

    fn count_symbol_spaces(symbols: &SymbolTable, key: StaticKey) -> (usize, usize, usize) {
        let mut type_count = 0;
        let mut value_count = 0;
        let mut type_value_count = 0;

        for local_id in symbols.active_symbol_ids() {
            let symbol = symbols.get_symbol(local_id);
            if symbol.key != Some(key) {
                continue;
            }

            match symbol.space {
                SymbolSpace::Type => type_count += 1,
                SymbolSpace::Value => value_count += 1,
                SymbolSpace::TypeValue => type_value_count += 1,
                SymbolSpace::Label => {}
            }
        }

        (type_count, value_count, type_value_count)
    }
}
