use dyst_ast::{self as ast};
use dyst_dir::{
    DependencyItem, Expression, ForEachKind, IfKind, LoopKind, MatchSource, Module, NodeId,
    NodeTree, Path, PathBase, ScopeId, ScopeKind, SymbolKey, SymbolSpace, YieldCardinality,
};

use crate::Compiler;

impl<'a> Compiler<'a> {
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
        scope_id: ScopeId,
        expression_id: ast::NodeId<ast::Expression>,
        tree: &mut NodeTree,
    ) -> NodeId<Expression> {
        let expression = module.get(expression_id);
        let expression = match expression {
            ast::Expression::Block(block_id) => {
                let block_id = self.bind_block(module, scope_id, *block_id, tree);
                Expression::Block { block: block_id }
            }
            ast::Expression::Definition(definition_id) => {
                let definition_id = self.bind_definition(module, scope_id, *definition_id, tree);
                Expression::Definition {
                    definition: definition_id,
                }
            }
            ast::Expression::Statement(expression_id) => {
                let expression_id = self.bind_expression(module, scope_id, *expression_id, tree);
                Expression::Statement {
                    statement: expression_id,
                }
            }

            ast::Expression::With { clauses, body } => {
                let (symbol_id, scope_id) = tree.create_symbol_with_scope(
                    SymbolSpace::Value,
                    None,
                    ScopeKind::Block,
                    scope_id,
                );
                let clauses = clauses
                    .iter()
                    .map(|clause| self.bind_with_clause(module, scope_id, *clause, tree))
                    .collect();
                let body = body.map(|body| self.bind_block(module, scope_id, body, tree));
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
                alias,
                items,
                arguments,
            } => {
                let target = self.session.strings.intern_from(&module.strings, *target);
                // items
                let mut items: Vec<_> = items
                    .as_ref()
                    .map(|items| {
                        items
                            .iter()
                            .map(|item| {
                                self.bind_dependency_item(module, scope_id, *kind, *item, tree)
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                // default item
                if let Some(alias) = alias {
                    let alias = self.session.strings.intern_from(&module.strings, *alias);
                    let symbol_id = tree.create_symbol(
                        SymbolSpace::Value,
                        Some(SymbolKey::Name(alias)),
                        scope_id,
                    );
                    let item = DependencyItem::UnresolvedDefault {
                        kind: self.bind_dependency_kind(*kind),
                        alias,
                        symbol: symbol_id,
                    };
                    items.push(tree.insert_from_source_as_symbol(
                        item,
                        module.id,
                        expression_id,
                        symbol_id,
                    ));
                }
                // arguments
                let arguments = arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| self.bind_argument(module, scope_id, *argument, tree))
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
                mode,
                kind,
                target,
                alias,
                items,
                value,
            } => {
                let mode = self.bind_export_type(*mode);
                let target =
                    target.map(|target| self.session.strings.intern_from(&module.strings, target));
                // re-export from import
                if let Some(target) = target {
                    // items
                    let mut items: Vec<_> = items
                        .as_ref()
                        .map(|items| {
                            items
                                .iter()
                                .map(|item| {
                                    self.bind_dependency_item(module, scope_id, *kind, *item, tree)
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    // default item
                    if let Some(alias) = alias {
                        let alias = self.session.strings.intern_from(&module.strings, *alias);
                        let symbol_id = tree.create_symbol(
                            SymbolSpace::Value,
                            Some(SymbolKey::Name(alias)),
                            scope_id,
                        );
                        let item = DependencyItem::UnresolvedDefault {
                            kind: self.bind_dependency_kind(*kind),
                            alias,
                            symbol: symbol_id,
                        };
                        items.push(tree.insert_from_source_as_symbol(
                            item,
                            module.id,
                            expression_id,
                            symbol_id,
                        ));
                    }
                    let kind = self.bind_dependency_kind(*kind);
                    // re-export
                    Expression::UnresolvedReExport {
                        mode,
                        target,
                        kind,
                        items,
                    }
                }
                // export from module
                else {
                    // export = value
                    let items = {
                        if let Some(value_id) = value {
                            let value = self.bind_expression(module, scope_id, *value_id, tree);
                            let item = DependencyItem::Value { value };
                            let item_id = tree.insert_from_source(item, module.id, *value_id);
                            vec![item_id]
                        }
                        // export items
                        else {
                            items
                                .as_ref()
                                .map(|items| {
                                    items
                                        .iter()
                                        .map(|item| {
                                            self.bind_dependency_item(
                                                module, scope_id, *kind, *item, tree,
                                            )
                                        })
                                        .collect()
                                })
                                .unwrap_or_default()
                        }
                    };
                    let kind = self.bind_dependency_kind(*kind);
                    Expression::Export { mode, kind, items }
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
                let pattern = self.bind_pattern(module, scope_id, *pattern, tree);
                let ty = ty.map(|ty| self.bind_expression_to_type(module, scope_id, ty, tree));
                let value = value.map(|value| self.bind_expression(module, scope_id, value, tree));
                let symbol_id = tree.create_symbol(SymbolSpace::Value, None, scope_id);
                Expression::Let {
                    mutability,
                    pattern,
                    ty,
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
                let name = self.session.strings.intern_from(
                    &module.strings,
                    descriptor.name.expect("LetType must have a name").string(),
                );
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let static_parameters = static_parameters.as_ref().map(|params| {
                    params
                        .iter()
                        .map(|param| self.bind_parameter(module, scope_id, *param, tree))
                        .collect()
                });
                let value = self.bind_expression(module, scope_id, *value, tree);
                let symbol_id = tree.create_symbol(SymbolSpace::Value, None, scope_id);
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
                let right = self.bind_expression(module, scope_id, *right, tree);
                let operator = self.bind_unary_operator(*operator);
                Expression::UnresolvedUnary { operator, right }
            }

            ast::Expression::TypeUnary { operator, right } => {
                let right = self.bind_expression(module, scope_id, *right, tree);
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
                let right = self.bind_expression(module, scope_id, *right, tree);
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
                let right = self.bind_expression(module, scope_id, *right, tree);
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
                let left = self.bind_expression(module, scope_id, *left, tree);
                let right = self.bind_expression(module, scope_id, *right, tree);
                let operator = self.bind_binary_operator(*operator);
                Expression::UnresolvedBinary {
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
                let left = self.bind_expression(module, scope_id, *left, tree);
                let right = self.bind_expression(module, scope_id, *right, tree);
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
                let left = self.bind_expression(module, scope_id, *left, tree);
                let right = self.bind_expression(module, scope_id, *right, tree);
                let operator = self.bind_assign_operator(*operator);
                if let Some(operator) = operator {
                    Expression::UnresolvedAssignBinary {
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
                let left = self.bind_expression(module, scope_id, *left, tree);
                let name = self.session.strings.intern_from(&module.strings, *name);
                let static_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| self.bind_argument(module, scope_id, *argument, tree))
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
                let left = self.bind_expression(module, scope_id, *left, tree);
                let static_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| self.bind_argument(module, scope_id, *argument, tree))
                        .collect()
                });
                let dynamic_arguments = dynamic_arguments
                    .iter()
                    .map(|argument| self.bind_argument(module, scope_id, *argument, tree))
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
                let left = self.bind_expression(module, scope_id, *left, tree);
                let static_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| self.bind_argument(module, scope_id, *argument, tree))
                        .collect()
                });
                let dynamic_arguments = dynamic_arguments
                    .iter()
                    .map(|argument| self.bind_argument(module, scope_id, *argument, tree))
                    .collect();
                Expression::New {
                    left,
                    static_arguments,
                    dynamic_arguments,
                }
            }
            ast::Expression::Delete { value } => {
                let value = self.bind_expression(module, scope_id, *value, tree);
                Expression::Delete { value }
            }
            ast::Expression::Index {
                position: _,
                left,
                index,
            } => {
                let left = self.bind_expression(module, scope_id, *left, tree);
                let index = index.map(|index| self.bind_expression(module, scope_id, index, tree));
                Expression::Index { left, right: index }
            }
            ast::Expression::Maybe { position: _, left } => {
                let left = self.bind_expression(module, scope_id, *left, tree);
                Expression::Maybe { left }
            }
            ast::Expression::Must { position: _, left } => {
                let left = self.bind_expression(module, scope_id, *left, tree);
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
                        .map(|argument| self.bind_argument(module, scope_id, *argument, tree))
                        .collect()
                });
                Expression::UnresolvedPath {
                    path,
                    static_arguments,
                }
            }
            ast::Expression::ScalarLiteral(value) => {
                let value = self.bind_scalar_literal(module, value);
                Expression::ScalarLiteral { value }
            }
            ast::Expression::TemplateLiteral { value } => {
                let value = self.bind_template_literal(module, scope_id, value, tree);
                Expression::TemplateLiteral { value }
            }
            ast::Expression::TaggedTemplateLiteral { tag, value } => {
                let tag = self.bind_expression(module, scope_id, *tag, tree);
                let value = self.bind_template_literal(module, scope_id, value, tree);
                Expression::TaggedTemplateLiteral { tag, value }
            }
            ast::Expression::TypeLiteral(value) => {
                if *value == ast::TypeLiteral::Self_ {
                    Expression::UnresolvedPath {
                        path: Path::Base {
                            base: PathBase::SelfType,
                        },
                        static_arguments: None,
                    }
                } else {
                    let value = self.bind_type_literal(value);
                    Expression::TypeLiteral { value }
                }
            }
            ast::Expression::RangeLiteral {
                start,
                end,
                is_inclusive,
            } => {
                let start = self.bind_expression(module, scope_id, *start, tree);
                let end = self.bind_expression(module, scope_id, *end, tree);
                Expression::RangeLiteral {
                    start,
                    end,
                    is_inclusive: *is_inclusive,
                }
            }
            ast::Expression::StructLiteral { ty, properties } => {
                let ty = ty.map(|ty| self.bind_expression_to_type(module, scope_id, ty, tree));
                let properties = properties
                    .iter()
                    .map(|property| self.bind_property(module, scope_id, *property, tree))
                    .collect();
                Expression::StructLiteral { ty, properties }
            }
            ast::Expression::TupleLiteral { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| self.bind_argument(module, scope_id, *element, tree))
                    .collect();
                Expression::TupleLiteral { ty: None, elements }
            }
            ast::Expression::ArrayLiteral { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| self.bind_argument(module, scope_id, *element, tree))
                    .collect();
                Expression::ArrayLiteral { elements }
            }
            ast::Expression::TreeLiteral {
                left,
                arguments,
                elements,
            } => {
                let left = left.map(|left| self.bind_expression(module, scope_id, left, tree));
                let arguments = arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| self.bind_argument(module, scope_id, *argument, tree))
                        .collect()
                });
                let elements = elements.as_ref().map(|elements| {
                    elements
                        .iter()
                        .map(|element| self.bind_argument(module, scope_id, *element, tree))
                        .collect()
                });
                Expression::TreeLiteral {
                    left,
                    arguments,
                    elements,
                }
            }
            ast::Expression::Parenthesized { expression } => {
                let expression = self.bind_expression(module, scope_id, *expression, tree);
                Expression::Parenthesized { expression }
            }

            ast::Expression::If {
                kind,
                condition,
                then_expression,
                else_expression,
            } => {
                let kind = self.bind_if_kind(*kind);
                let condition = self.bind_expression(module, scope_id, *condition, tree);
                let then_expression =
                    self.bind_expression(module, scope_id, *then_expression, tree);
                let else_expression = else_expression.map(|else_expression| {
                    self.bind_expression(module, scope_id, else_expression, tree)
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
                let condition = self.bind_expression(module, scope_id, *condition, tree);
                let (symbol_id, scope_id) = tree.create_symbol_with_scope(
                    SymbolSpace::Value,
                    None,
                    ScopeKind::Block,
                    scope_id,
                );
                let body = self.bind_block(module, scope_id, *body, tree);
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
                let pattern = self.bind_pattern(module, scope_id, *pattern, tree);
                let iterator = self.bind_expression(module, scope_id, *iterator, tree);
                let (symbol_id, scope_id) = tree.create_symbol_with_scope(
                    SymbolSpace::Value,
                    None,
                    ScopeKind::Block,
                    scope_id,
                );
                let body = self.bind_block(module, scope_id, *body, tree);
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
                let (symbol_id, scope_id) = tree.create_symbol_with_scope(
                    SymbolSpace::Value,
                    None,
                    ScopeKind::Block,
                    scope_id,
                );
                let initialization = initialization.map(|initialization| {
                    self.bind_expression(module, scope_id, initialization, tree)
                });
                let condition = condition
                    .map(|condition| self.bind_expression(module, scope_id, condition, tree));
                let increment = increment
                    .map(|increment| self.bind_expression(module, scope_id, increment, tree));
                let body = self.bind_block(module, scope_id, *body, tree);
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
                let (symbol_id, scope_id) = tree.create_symbol_with_scope(
                    SymbolSpace::Value,
                    None,
                    ScopeKind::Block,
                    scope_id,
                );
                let body = self.bind_block(module, scope_id, *body, tree);
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
                let (symbol_id, scope_id) = tree.create_symbol_with_scope(
                    SymbolSpace::Value,
                    None,
                    ScopeKind::Block,
                    scope_id,
                );
                let try_expression = self.bind_expression(module, scope_id, *try_expression, tree);
                let catch_pattern = catch_pattern
                    .map(|catch_pattern| self.bind_pattern(module, scope_id, catch_pattern, tree));
                let catch_expression = catch_expression.map(|catch_expression| {
                    self.bind_expression(module, scope_id, catch_expression, tree)
                });
                let finally_expression = finally_expression.map(|finally_expression| {
                    self.bind_expression(module, scope_id, finally_expression, tree)
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
                let value = self.bind_expression(module, scope_id, *value, tree);
                let (symbol_id, scope_id) = tree.create_symbol_with_scope(
                    SymbolSpace::Value,
                    None,
                    ScopeKind::Block,
                    scope_id,
                );
                let cases = cases
                    .iter()
                    .map(|case| self.bind_match_case(module, scope_id, *case, tree))
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
                    label.map(|label| self.session.strings.intern_from(&module.strings, label));
                let value = value.map(|value| self.bind_expression(module, scope_id, value, tree));
                Expression::UnresolvedBreak {
                    target: label,
                    value,
                }
            }
            ast::Expression::Continue { label } => {
                let label =
                    label.map(|label| self.session.strings.intern_from(&module.strings, label));
                Expression::UnresolvedContinue { target: label }
            }
            ast::Expression::Return { value } => {
                let value = value.map(|value| self.bind_expression(module, scope_id, value, tree));
                Expression::Return { value }
            }
            ast::Expression::Defer { expression } => {
                let expression = self.bind_expression(module, scope_id, *expression, tree);
                Expression::Defer { expression }
            }
            ast::Expression::Await { expression } => {
                let expression = self.bind_expression(module, scope_id, *expression, tree);
                Expression::Await { expression }
            }
            ast::Expression::Yield { cardinality, value } => {
                let cardinality = match *cardinality {
                    ast::YieldCardinality::Generator => YieldCardinality::Generator,
                    ast::YieldCardinality::Scalar => YieldCardinality::Scalar,
                };
                let value = self.bind_expression(module, scope_id, *value, tree);
                Expression::Yield { cardinality, value }
            }
            ast::Expression::Throw { value } => {
                let value = value.map(|value| self.bind_expression(module, scope_id, value, tree));
                Expression::Throw { value }
            }

            ast::Expression::Error => Expression::Error,
        };

        // expression
        if let Some(symbol_id) = expression.symbol() {
            tree.insert_from_source_as_symbol(expression, module.id, expression_id, symbol_id)
        } else {
            tree.insert_from_source(expression, module.id, expression_id)
        }
    }
}
