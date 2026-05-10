use destack_ast::{self as ast};
use destack_core::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use super::UnbindContext;
use crate::Compiler;

impl Compiler {
    /// Unbind a DIR export kind to an AST export kind.
    pub(super) fn unbind_export_kind(&self, export: dir::ExportKind) -> ast::ExportKind {
        match export {
            dir::ExportKind::Named => ast::ExportKind::Named,
            dir::ExportKind::Default => ast::ExportKind::Default,
        }
    }

    /// Unbind a DIR generic argument to an AST generic argument.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn unbind_generic_argument(
        &self,
        module: &Module,
        argument_id: dir::LocalNodeId<dir::GenericArgument>,
        tree: &dir::Tree,
        symbols: &dir::BindingTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::Tree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::GenericArgument> {
        let argument = tree.get(argument_id);
        let span = self.unbind_span(module, argument_id.into());

        let ast_argument = match argument {
            dir::GenericArgument::Type { value } => {
                let value = self.unbind_type_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );

                ast::GenericArgument::Type { value }
            }
            dir::GenericArgument::Value { value } => {
                let value = self.unbind_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );

                ast::GenericArgument::Value { value }
            }
            dir::GenericArgument::Error => ast::GenericArgument::Error,
        };

        let ast_argument_id = ast_tree.insert(ast_argument, span);
        context.map(argument_id.into_any(), ast_argument_id.into_any());
        ast_argument_id
    }

    /// Unbind one DIR assignment pattern to an AST assignment pattern.
    #[allow(clippy::too_many_arguments)]
    fn unbind_assign_pattern(
        &self,
        module: &Module,
        assign_pattern_id: dir::LocalNodeId<dir::AssignPattern>,
        tree: &dir::Tree,
        symbols: &dir::BindingTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::Tree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::AssignPattern> {
        let assign_pattern = tree.get(assign_pattern_id);
        let span = self.unbind_span(module, assign_pattern_id.into());

        let ast_assign_pattern = match assign_pattern {
            dir::AssignPattern::Expression { value } => {
                let value = self.unbind_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );

                ast::AssignPattern::Expression { value }
            }
            dir::AssignPattern::Assign { pattern, value } => {
                let pattern = self.unbind_assign_pattern(
                    module,
                    *pattern,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let value = self.unbind_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );

                ast::AssignPattern::Assign { pattern, value }
            }
            dir::AssignPattern::Sequence { fields } => {
                let fields = fields
                    .iter()
                    .map(|field_id| {
                        self.unbind_assign_pattern_field(
                            module,
                            *field_id,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();

                ast::AssignPattern::Sequence { fields }
            }
            dir::AssignPattern::Object { fields } => {
                let fields = fields
                    .iter()
                    .map(|field_id| {
                        self.unbind_assign_pattern_field(
                            module,
                            *field_id,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();

                ast::AssignPattern::Object { fields }
            }
        };

        let ast_assign_pattern_id = ast_tree.insert(ast_assign_pattern, span);
        context.map(
            assign_pattern_id.into_any(),
            ast_assign_pattern_id.into_any(),
        );
        ast_assign_pattern_id
    }

    /// Unbind one DIR assignment pattern field to an AST assignment pattern field.
    #[allow(clippy::too_many_arguments)]
    fn unbind_assign_pattern_field(
        &self,
        module: &Module,
        assign_pattern_field_id: dir::LocalNodeId<dir::AssignPatternField>,
        tree: &dir::Tree,
        symbols: &dir::BindingTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::Tree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::AssignPatternField> {
        let assign_pattern_field = tree.get(assign_pattern_field_id);
        let span = self.unbind_span(module, assign_pattern_field_id.into());

        let ast_assign_pattern_field = match assign_pattern_field {
            dir::AssignPatternField::Named {
                name,
                is_shorthand,
                pattern,
            } => {
                let name = self.unbind_name(ast_strings, *name);
                let pattern = pattern.map(|pattern_id| {
                    self.unbind_assign_pattern(
                        module,
                        pattern_id,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });

                ast::AssignPatternField::Named {
                    name,
                    is_shorthand: *is_shorthand,
                    pattern,
                }
            }
            dir::AssignPatternField::Computed { key, pattern } => {
                let key = self.unbind_expression(
                    module,
                    *key,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let pattern = self.unbind_assign_pattern(
                    module,
                    *pattern,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );

                ast::AssignPatternField::Computed { key, pattern }
            }
            dir::AssignPatternField::Positional { pattern } => {
                let pattern = self.unbind_assign_pattern(
                    module,
                    *pattern,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );

                ast::AssignPatternField::Positional { pattern }
            }
            dir::AssignPatternField::Spread { pattern } => {
                let pattern = pattern.map(|pattern_id| {
                    self.unbind_assign_pattern(
                        module,
                        pattern_id,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });

                ast::AssignPatternField::Spread { pattern }
            }
            dir::AssignPatternField::Elision => ast::AssignPatternField::Elision,
        };

        let ast_assign_pattern_field_id = ast_tree.insert(ast_assign_pattern_field, span);
        context.map(
            assign_pattern_field_id.into_any(),
            ast_assign_pattern_field_id.into_any(),
        );
        ast_assign_pattern_field_id
    }

    /// Unbind one simple expression target into an AST assignment pattern.
    #[allow(clippy::too_many_arguments)]
    fn unbind_expression_assign_pattern(
        &self,
        module: &Module,
        expression_id: dir::LocalNodeId<dir::Expression>,
        tree: &dir::Tree,
        symbols: &dir::BindingTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::Tree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::AssignPattern> {
        let value = self.unbind_expression(
            module,
            expression_id,
            tree,
            symbols,
            types,
            ast_tree,
            ast_strings,
            context,
        );
        let span = self.unbind_span(module, expression_id.into());

        ast_tree.insert(ast::AssignPattern::Expression { value }, span)
    }

    destack_core::ensure_sufficient_stack! {
        /// Unbind a DIR expression to an AST expression.
        #[allow(clippy::too_many_arguments)]
        pub fn unbind_expression(
            &self,
            module: &Module,
            expression_id: dir::LocalNodeId<dir::Expression>,
            tree: &dir::Tree,
            symbols: &dir::BindingTable,
            types: &dir::TypeTable,
            ast_tree: &mut ast::Tree,
            ast_strings: &mut StringPool,
            context: &mut UnbindContext,
        ) -> ast::LocalNodeId<ast::Expression> {
            let expression = tree.get(expression_id);
            let span = self.unbind_span(module, expression_id.into());

            let ast_expression = match expression {
                dir::Expression::Declaration(declaration) => {
                    let declaration = self.unbind_declaration(
                        module,
                        *declaration,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );
                    ast::Expression::Declaration(declaration)
                }

                dir::Expression::Block(block) => {
                    let block = self.unbind_block(
                        module,
                        *block,
                        ast::BlockContext::Expression,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );
                    ast::Expression::Block(block)
                }

                dir::Expression::Labelled { label, body, .. } => {
                    let label = *label;
                    let body = self.unbind_expression(
                        module,
                        *body,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );
                    ast::Expression::Labelled { label, body }
                }

                dir::Expression::Import {
                    space,
                    target,
                    items,
                    attributes,
                } => {
                    let space = self.unbind_dependency_space(context, *space);
                    let target = *target;
                    let items = items.as_ref().map(|items| {
                        items
                            .iter()
                            .map(|item| {
                                self.unbind_dependency_item(
                                    module,
                                    *item,
                                    tree,
                                    symbols,
                                    types,
                                    ast_tree,
                                    ast_strings,
                                    context,
                                )
                            })
                            .collect()
                    });
                    let attributes = attributes.as_ref().map(|attributes| {
                        self.unbind_import_attribute_clause(attributes, ast_strings, context)
                    });
                    ast::Expression::Import {
                        space,
                        target,
                        items,
                        attributes,
                    }
                }

                dir::Expression::ReExport {
                    target,
                    space,
                    items,
                    attributes,
                    ..
                } => {
                    let space = self.unbind_dependency_space(context, *space);
                    let target = *target;
                    let items = items.iter().map(|item| {
                        self.unbind_dependency_item(module, *item, tree, symbols, types, ast_tree, ast_strings, context)
                    }).collect();
                    let attributes = attributes.as_ref().map(|attributes| {
                        self.unbind_import_attribute_clause(attributes, ast_strings, context)
                    });
                    ast::Expression::Export {
                        space,
                        target: Some(target),
                        items,
                        attributes,
                    }
                }

                dir::Expression::Export {
                    space,
                    items,
                    attributes,
                } => {
                    let space = self.unbind_dependency_space(context, *space);
                    let items = items.iter().map(|item| {
                        self.unbind_dependency_item(module, *item, tree, symbols, types, ast_tree, ast_strings, context)
                    }).collect();
                    let attributes = attributes.as_ref().map(|attributes| {
                        self.unbind_import_attribute_clause(attributes, ast_strings, context)
                    });
                    ast::Expression::Export {
                        space,
                        target: None,
                        items,
                        attributes,
                    }
                }
                dir::Expression::Let {
                    export,
                    is_ambient,
                    is_shared,
                    mutability,
                    declarators,
                } => {
                    // reconstruct the keyword from semantic mutability
                    let ast_mutability = self.unbind_mutability(context, *mutability);
                    let kind = match mutability {
                        dir::Mutability::Immutable => ast::LetKind::Const,
                        dir::Mutability::Mutable | dir::Mutability::Exclusive => ast::LetKind::Let,
                    };
                    let export = export.map(|export| self.unbind_export_kind(export));
                    let is_ambient = self.unbind_ambientness(*is_ambient, context);
                    let declarators = declarators.iter().map(|decl| {
                        self.unbind_declarator(module, *decl, tree, symbols, types, ast_tree, ast_strings, context)
                    }).collect();
                    ast::Expression::Let {
                        kind,
                        export,
                        is_ambient,
                        is_shared: *is_shared,
                        mutability: ast_mutability,
                        declarators,
                    }
                }
                dir::Expression::LetElse {
                    kind,
                    mutability,
                    declarator,
                    else_branch,
                } => {
                    let kind = match kind {
                        dir::LetKind::Let => ast::LetKind::Let,
                        dir::LetKind::Const => ast::LetKind::Const,
                    };
                    let mutability = self.unbind_mutability(context, *mutability);
                    let declarator = self.unbind_declarator(
                        module,
                        *declarator,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );
                    let else_branch = self.unbind_expression(
                        module,
                        *else_branch,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );

                    ast::Expression::LetElse {
                        kind,
                        mutability,
                        declarator,
                        else_branch,
                    }
                }
                dir::Expression::Using {
                    asynchrony,
                    export,
                    is_ambient,
                    declarators,
                } => {
                    let asynchrony = self.unbind_asynchrony(context, *asynchrony);
                    let export = export.map(|export| self.unbind_export_kind(export));
                    let is_ambient = self.unbind_ambientness(*is_ambient, context);
                    let declarators = declarators
                        .iter()
                        .map(|decl| {
                            self.unbind_declarator(
                                module,
                                *decl,
                                tree,
                                symbols,
                                types,
                                ast_tree,
                                ast_strings,
                                context,
                            )
                        })
                        .collect();
                    ast::Expression::Using {
                        asynchrony,
                        export,
                        is_ambient,
                        declarators,
                    }
                }

                dir::Expression::As {
                    operator: _,
                    source: _,
                    expression,
                    target_type,
                } => {
                    let expression = self.unbind_expression(
                        module,
                        *expression,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );
                    let type_annotation = self.unbind_type_expression(
                        module,
                        *target_type,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );

                    ast::Expression::As {
                        expression,
                        target_type: type_annotation,
                    }
                }

                dir::Expression::Satisfies {
                    expression,
                    target_type,
                } => {
                    let expression = self.unbind_expression(
                        module,
                        *expression,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );
                    let type_annotation = self.unbind_type_expression(
                        module,
                        *target_type,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );

                    ast::Expression::Satisfies {
                        expression,
                        target_type: type_annotation,
                    }
                }

                dir::Expression::Unary { operator, right } => {
                    let operator = self.unbind_unary_operator(context, *operator);
                    let right = self.unbind_expression(module, *right, tree, symbols, types, ast_tree, ast_strings, context);
                    ast::Expression::Unary { operator, right }
                }

                dir::Expression::MoveOf { mutability, variance, right } => {
                    let mutability = mutability.map(|m| self.unbind_mutability(context, m));
                    let variance = variance.map(|v| self.unbind_variance_bound(context, v));
                    let right = self.unbind_expression(module, *right, tree, symbols, types, ast_tree, ast_strings, context);
                    ast::Expression::MoveOf { mutability, variance, right }
                }

                dir::Expression::BorrowOf { mutability, variance, right } => {
                    let mutability = mutability.map(|m| self.unbind_mutability(context, m));
                    let variance = variance.map(|v| self.unbind_variance_bound(context, v));
                    let right = self.unbind_expression(module, *right, tree, symbols, types, ast_tree, ast_strings, context);
                    ast::Expression::BorrowOf { mutability, variance, right }
                }
                dir::Expression::Is { value, target_type } => {
                    let value = self.unbind_expression(module, *value, tree, symbols, types, ast_tree, ast_strings, context);
                    let target_type = self.unbind_type_expression(
                        module,
                        *target_type,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );

                    ast::Expression::Is { value, target_type }
                }
                dir::Expression::InstanceOf { value, target } => {
                    let value = self.unbind_expression(module, *value, tree, symbols, types, ast_tree, ast_strings, context);
                    let target = self.unbind_expression(module, *target, tree, symbols, types, ast_tree, ast_strings, context);

                    ast::Expression::InstanceOf { value, target }
                }

                dir::Expression::Binary { left, operator, right } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, types, ast_tree, ast_strings, context);
                    let operator = self.unbind_binary_operator(context, *operator);
                    let right = self.unbind_expression(module, *right, tree, symbols, types, ast_tree, ast_strings, context);
                    ast::Expression::Binary { left, operator, right }
                }

                dir::Expression::Assign { left, right } => {
                    let left = self.unbind_assign_pattern(
                        module,
                        *left,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );
                    let right = self.unbind_expression(module, *right, tree, symbols, types, ast_tree, ast_strings, context);
                    ast::Expression::Assign { left, operator: ast::AssignOperator::Assign, right }
                }

                dir::Expression::AssignBinary { left, operator, right } => {
                    let left = self.unbind_expression_assign_pattern(
                        module,
                        *left,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );
                    let operator = self.unbind_assign_operator(context, *operator);
                    let right = self.unbind_expression(module, *right, tree, symbols, types, ast_tree, ast_strings, context);
                    ast::Expression::Assign { left, operator, right }
                }

                dir::Expression::Member { left, name } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, types, ast_tree, ast_strings, context);
                    let name = name.map(|name| name);

                    ast::Expression::Member { left, name }
                }
                dir::Expression::PrivateMember { left, name } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, types, ast_tree, ast_strings, context);
                    let name = name.map(|name| name);

                    ast::Expression::PrivateMember { left, name }
                }

                dir::Expression::Call {
                    left,
                    generic_arguments,
                    arguments,
                } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, types, ast_tree, ast_strings, context);
                    let generic_arguments = generic_arguments.iter().map(|argument| {
                        self.unbind_generic_argument(
                            module,
                            *argument,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    }).collect();
                    let arguments = arguments.iter().map(|arg| {
                        self.unbind_argument(module, *arg, tree, symbols, types, ast_tree, ast_strings, context)
                    }).collect();
                    ast::Expression::Call {
                        position: ast::PostfixPosition::Direct,
                        left,
                        generic_arguments,
                        arguments,
                    }
                }

                dir::Expression::Index { left, right } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, types, ast_tree, ast_strings, context);
                    let index = right.map(|r| self.unbind_expression(module, r, tree, symbols, types, ast_tree, ast_strings, context));
                    ast::Expression::Index {
                        position: ast::PostfixPosition::Direct,
                        left,
                        index,
                    }
                }

                dir::Expression::Instantiation { left, generic_arguments } => {
                    let left =
                        self.unbind_expression(module, *left, tree, symbols, types, ast_tree, ast_strings, context);
                    let generic_arguments = generic_arguments
                        .iter()
                        .map(|argument| {
                            self.unbind_generic_argument(
                                module,
                                *argument,
                                tree,
                                symbols,
                                types,
                                ast_tree,
                                ast_strings,
                                context,
                            )
                        })
                        .collect();
                    ast::Expression::Instantiation {
                        left,
                        generic_arguments,
                    }
                }

                dir::Expression::Maybe { left } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, types, ast_tree, ast_strings, context);
                    ast::Expression::Maybe {
                        position: ast::PostfixPosition::Direct,
                        left,
                    }
                }

                dir::Expression::Must { left } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, types, ast_tree, ast_strings, context);
                    ast::Expression::Must {
                        position: ast::PostfixPosition::Direct,
                        left,
                    }
                }

                dir::Expression::New {
                    left,
                    generic_arguments,
                    arguments,
                } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, types, ast_tree, ast_strings, context);
                    let generic_arguments = generic_arguments.iter().map(|argument| {
                        self.unbind_generic_argument(
                            module,
                            *argument,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    }).collect();
                    let arguments = arguments.iter().map(|arg| {
                        self.unbind_argument(module, *arg, tree, symbols, types, ast_tree, ast_strings, context)
                    }).collect();
                    ast::Expression::New { left, generic_arguments, arguments }
                }

                dir::Expression::Path {
                    path,
                    generic_arguments,
                    ..
                } => {
                    let path = self.unbind_path(path, ast_strings, context);
                    let generic_arguments = generic_arguments.iter().map(|argument| {
                        self.unbind_generic_argument(
                            module,
                            *argument,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    }).collect::<Vec<_>>();

                    if path.segments.len() == 1 && generic_arguments.is_empty() {
                        ast::Expression::Identifier {
                            name: path.segments[0],
                        }
                    } else {
                        ast::Expression::QualifiedReference { path, generic_arguments }
                    }
                }

                dir::Expression::PrivateIdentifier { name } => {
                    let name = *name;
                    ast::Expression::PrivateIdentifier { name }
                }

                dir::Expression::ImportMeta => ast::Expression::ImportMeta,
                dir::Expression::NewTarget => ast::Expression::NewTarget,
                dir::Expression::This => {
                    ast::Expression::This
                }
                dir::Expression::Super => {
                    ast::Expression::Super
                }

                dir::Expression::ScalarLiteral { value } => {
                    let value = self.unbind_scalar_literal(value, ast_strings, context);
                    ast::Expression::ScalarLiteral(value)
                }

                dir::Expression::TypeLiteral { value } => {
                    let value = self.unbind_type_literal(value, context);
                    let value = ast_tree.insert(ast::TypeExpression::Literal { value }, span);
                    ast::Expression::Type { value }
                }

                dir::Expression::Type { value } => {
                    let value = self.unbind_type_expression(
                        module,
                        *value,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );
                    ast::Expression::Type { value }
                }

                dir::Expression::TemplateExpression { value } => {
                    let value = self.unbind_template_literal(
                        module,
                        value,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );
                    ast::Expression::TemplateExpression { value }
                }

                dir::Expression::TaggedTemplateExpression {
                    tag,
                    generic_arguments,
                    value,
                } => {
                    let tag = self.unbind_expression(module, *tag, tree, symbols, types, ast_tree, ast_strings, context);
                    let generic_arguments = generic_arguments
                        .iter()
                        .map(|argument| {
                            self.unbind_generic_argument(
                                module,
                                *argument,
                                tree,
                                symbols,
                                types,
                                ast_tree,
                                ast_strings,
                                context,
                            )
                        })
                        .collect();
                    let value = self.unbind_template_literal(
                        module,
                        value,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );

                    ast::Expression::TaggedTemplateExpression {
                        tag,
                        generic_arguments,
                        value,
                    }
                }

                dir::Expression::ArrayExpression { elements } => {
                    let elements = elements.iter().map(|el| {
                        self.unbind_argument(module, *el, tree, symbols, types, ast_tree, ast_strings, context)
                    }).collect();
                    ast::Expression::ArrayExpression { elements }
                }

                dir::Expression::FixedArrayExpression { value, length } => {
                    let value = self.unbind_expression(module, *value, tree, symbols, types, ast_tree, ast_strings, context);
                    let length = self.unbind_expression(module, *length, tree, symbols, types, ast_tree, ast_strings, context);
                    ast::Expression::FixedArrayExpression { value, length }
                }

                dir::Expression::TupleExpression { elements } => {
                    let elements = elements.iter().map(|el| {
                        self.unbind_argument(module, *el, tree, symbols, types, ast_tree, ast_strings, context)
                    }).collect();
                    ast::Expression::TupleExpression { elements }
                }

                dir::Expression::SequenceExpression { expressions } => {
                    let expressions = expressions.iter().map(|expr| {
                        self.unbind_expression(module, *expr, tree, symbols, types, ast_tree, ast_strings, context)
                    }).collect();
                    ast::Expression::SequenceExpression { expressions }
                }

                dir::Expression::ObjectExpression { ty, properties } => {
                    let ty = ty.map(|ty| {
                        self.unbind_type_expression(
                            module,
                            ty,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    });
                    let properties = properties.iter().map(|prop| {
                        self.unbind_property(module, *prop, tree, symbols, types, ast_tree, ast_strings, context)
                    }).collect();
                    ast::Expression::ObjectExpression { ty, properties }
                }

                dir::Expression::TreeExpression {
                    left,
                    generic_arguments,
                    arguments,
                    elements,
                } => {
                    let left = left.map(|l| self.unbind_expression(module, l, tree, symbols, types, ast_tree, ast_strings, context));
                    let generic_arguments = generic_arguments.iter().map(|argument| {
                        self.unbind_generic_argument(
                            module,
                            *argument,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    }).collect();
                    let arguments = arguments.as_ref().map(|args| {
                        args.iter().map(|arg| {
                            self.unbind_argument(module, *arg, tree, symbols, types, ast_tree, ast_strings, context)
                        }).collect()
                    });
                    let elements = elements.as_ref().map(|els| {
                        els.iter().map(|el| {
                            self.unbind_argument(module, *el, tree, symbols, types, ast_tree, ast_strings, context)
                        }).collect()
                    });

                    ast::Expression::TreeExpression {
                        left,
                        generic_arguments,
                        arguments,
                        elements,
                    }
                }

                dir::Expression::TaggedScalarExpression { ty, value } => {
                    // emit as call: ty(value)
                    let ty = self.unbind_type_expression(
                        module,
                        *ty,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );
                    let value = self.unbind_expression(module, *value, tree, symbols, types, ast_tree, ast_strings, context);
                    let value_arg = ast_tree.insert(
                        ast::Argument::Positional {
                            value,
                        },
                        span,
                    );
                    ast::Expression::Call {
                        position: ast::PostfixPosition::Direct,
                        left: ast_tree.insert(ast::Expression::Type { value: ty }, span),
                        generic_arguments: Vec::new(),
                        arguments: vec![value_arg],
                    }
                }

                dir::Expression::TaggedTupleExpression { ty, elements } => {
                    // emit as call: ty(elements...)
                    let ty = self.unbind_type_expression(
                        module,
                        *ty,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );
                    let elements = elements.iter().map(|el| {
                        self.unbind_argument(module, *el, tree, symbols, types, ast_tree, ast_strings, context)
                    }).collect();
                    ast::Expression::Call {
                        position: ast::PostfixPosition::Direct,
                        left: ast_tree.insert(ast::Expression::Type { value: ty }, span),
                        generic_arguments: Vec::new(),
                        arguments: elements,
                    }
                }

                dir::Expression::TaggedObjectExpression { ty, properties } => {
                    let ty = self.unbind_type_expression(
                        module,
                        *ty,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );
                    let properties = properties.iter().map(|prop| {
                        self.unbind_property(module, *prop, tree, symbols, types, ast_tree, ast_strings, context)
                    }).collect();
                    ast::Expression::ObjectExpression { ty: Some(ty), properties }
                }

                dir::Expression::Parenthesized { expression } => {
                    let expression = self.unbind_expression(module, *expression, tree, symbols, types, ast_tree, ast_strings, context);
                    ast::Expression::Parenthesized { expression }
                }

                dir::Expression::If { form, condition, then_expression, else_expression } => {
                    let form = self.unbind_if_form(context, *form);
                    let condition = match condition {
                        dir::IfCondition::Expression { condition } => {
                            let condition = self.unbind_expression(
                                module,
                                *condition,
                                tree,
                                symbols,
                                types,
                                ast_tree,
                                ast_strings,
                                context,
                            );
                            ast::IfCondition::Expression { condition }
                        }
                        dir::IfCondition::Let {
                            kind,
                            mutability,
                            declarator,
                        } => {
                            let ast_mutability = self.unbind_mutability(context, *mutability);
                            let kind = match kind {
                                dir::LetKind::Let => ast::LetKind::Let,
                                dir::LetKind::Const => ast::LetKind::Const,
                            };
                            let declarator = self.unbind_declarator(
                                module,
                                *declarator,
                                tree,
                                symbols,
                                types,
                                ast_tree,
                                ast_strings,
                                context,
                            );
                            ast::IfCondition::Let {
                                kind,
                                mutability: ast_mutability,
                                declarator,
                            }
                        }
                    };
                    let then_expression = self.unbind_expression(
                        module,
                        *then_expression,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );
                    let else_expression = else_expression.map(|e| {
                        self.unbind_expression(module, e, tree, symbols, types, ast_tree, ast_strings, context)
                    });
                    ast::Expression::If { form, condition, then_expression, else_expression }
                }

                dir::Expression::Loop { kind, condition, body, .. } => {
                    let body = self.unbind_block(
                        module,
                        *body,
                        ast::BlockContext::Statement,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );
                    match kind {
                        dir::LoopKind::NoTest => ast::Expression::Loop { body },
                        dir::LoopKind::PreTest => {
                            let condition = condition.map(|c| {
                                self.unbind_expression(module, c, tree, symbols, types, ast_tree, ast_strings, context)
                            }).unwrap_or_else(|| {
                                ast_tree.insert(ast::Expression::ScalarLiteral(ast::ScalarLiteral::Boolean(true)), span)
                            });
                            ast::Expression::While { form: ast::WhileForm::While, condition, body }
                        }
                        dir::LoopKind::PostTest => {
                            let condition = condition.map(|c| {
                                self.unbind_expression(module, c, tree, symbols, types, ast_tree, ast_strings, context)
                            }).unwrap_or_else(|| {
                                ast_tree.insert(ast::Expression::ScalarLiteral(ast::ScalarLiteral::Boolean(true)), span)
                            });
                            ast::Expression::While { form: ast::WhileForm::DoWhile, condition, body }
                        }
                    }
                }

                dir::Expression::ForEach {
                    asynchrony,
                    operator,
                    binding,
                    iterator,
                    body,
                    ..
                } => {
                    let asynchrony = self.unbind_asynchrony(context, *asynchrony);
                    let operator = self.unbind_for_each_operator(context, *operator);
                    let binding = match binding {
                        dir::ForEachBinding::Pattern {
                            pattern,
                            keyword,
                        } => {
                            let pattern = self.unbind_pattern(
                                module,
                                *pattern,
                                tree,
                                symbols,
                                types,
                                ast_tree,
                                ast_strings,
                                context,
                            );
                            let keyword = keyword
                                .map(|keyword| self.unbind_binding_keyword(context, keyword));
                            ast::ForEachBinding::Pattern {
                                pattern,
                                keyword,
                            }
                        }
                        dir::ForEachBinding::Using { asynchrony, pattern } => {
                            let pattern = self.unbind_pattern(
                                module,
                                *pattern,
                                tree,
                                symbols,
                                types,
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
                    let iterator = self.unbind_expression(module, *iterator, tree, symbols, types, ast_tree, ast_strings, context);
                    let body = self.unbind_block(
                        module,
                        *body,
                        ast::BlockContext::Statement,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );
                    ast::Expression::ForEach {
                        asynchrony,
                        operator,
                        binding,
                        iterator,
                        body,
                    }
                }

                dir::Expression::For { initialization, condition, increment, body, .. } => {
                    let initialization = initialization.map(|i| {
                        self.unbind_expression(module, i, tree, symbols, types, ast_tree, ast_strings, context)
                    });
                    let condition = condition.map(|c| {
                        self.unbind_expression(module, c, tree, symbols, types, ast_tree, ast_strings, context)
                    });
                    let increment = increment.map(|i| {
                        self.unbind_expression(module, i, tree, symbols, types, ast_tree, ast_strings, context)
                    });
                    let body = self.unbind_block(
                        module,
                        *body,
                        ast::BlockContext::Statement,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );
                    ast::Expression::For { initialization, condition, increment, body }
                }

                dir::Expression::Try { try_expression, catch_pattern, catch_ty, catch_expression, finally_expression, .. } => {
                    let try_expression = self.unbind_expression(module, *try_expression, tree, symbols, types, ast_tree, ast_strings, context);
                    let catch_pattern = catch_pattern.map(|p| {
                        self.unbind_pattern(module, p, tree, symbols, types, ast_tree, ast_strings, context)
                    });
                    let catch_ty = catch_ty.map(|t| {
                        self.unbind_type_expression(module, t, tree, symbols, types, ast_tree, ast_strings, context)
                    });
                    let catch_expression = catch_expression.map(|e| {
                        self.unbind_expression(module, e, tree, symbols, types, ast_tree, ast_strings, context)
                    });
                    let finally_expression = finally_expression.map(|e| {
                        self.unbind_expression(module, e, tree, symbols, types, ast_tree, ast_strings, context)
                    });
                    ast::Expression::Try { try_expression, catch_pattern, catch_ty, catch_expression, finally_expression }
                }

                dir::Expression::Match { form, value, cases, .. } => {
                    let form = match form {
                        dir::MatchForm::Match => ast::MatchForm::Match,
                        dir::MatchForm::Switch => ast::MatchForm::Switch,
                    };
                    let value = self.unbind_expression(module, *value, tree, symbols, types, ast_tree, ast_strings, context);
                    let cases = cases.iter().map(|case| {
                        self.unbind_match_case(
                            module,
                            *case,
                            form,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    }).collect();
                    ast::Expression::Match { form, value, cases }
                }

                dir::Expression::Break { target, value, .. } => {
                    let label = target.map(|t| t);
                    let value = value.map(|v| self.unbind_expression(module, v, tree, symbols, types, ast_tree, ast_strings, context));
                    ast::Expression::Break { label, value }
                }

                dir::Expression::Continue { target, .. } => {
                    let label = target.map(|t| t);
                    ast::Expression::Continue { label }
                }

                dir::Expression::Throw { value } => {
                    let value = self.unbind_expression(module, *value, tree, symbols, types, ast_tree, ast_strings, context);
                    ast::Expression::Throw { value }
                }

                dir::Expression::Await { expression } => {
                    let expression = self.unbind_expression(module, *expression, tree, symbols, types, ast_tree, ast_strings, context);
                    ast::Expression::Await { expression }
                }

                dir::Expression::AwaitMaybe { expression } => {
                    let expression = self.unbind_expression(module, *expression, tree, symbols, types, ast_tree, ast_strings, context);
                    ast::Expression::AwaitMaybe { expression }
                }

                dir::Expression::Comptime { body } => {
                    let body = self.unbind_expression(module, *body, tree, symbols, types, ast_tree, ast_strings, context);
                    ast::Expression::Comptime { body }
                }

                dir::Expression::Yield { cardinality, value } => {
                    let cardinality = self.unbind_yield_cardinality(context, *cardinality);
                    let value = value.map(|v| self.unbind_expression(module, v, tree, symbols, types, ast_tree, ast_strings, context));
                    ast::Expression::Yield { cardinality, value }
                }

                dir::Expression::Return { value } => {
                    let value = value.map(|v| self.unbind_expression(module, v, tree, symbols, types, ast_tree, ast_strings, context));
                    ast::Expression::Return { value }
                }

                dir::Expression::Debugger => ast::Expression::Debugger,
                dir::Expression::Missing => ast::Expression::Missing,
                dir::Expression::Stub => ast::Expression::Stub,
                dir::Expression::Error => ast::Expression::Error,
            };

            let ast_expression_id = ast_tree.insert(ast_expression, span);
            context.map(expression_id.into_any(), ast_expression_id.into_any());

            ast_expression_id
        }
    }

    /// Unbind a DIR type predicate subject to an AST type predicate subject.
    pub(super) fn unbind_type_predicate_subject(
        &self,
        subject: dir::TypePredicateSubject,
        _module: &Module,
        _symbols: &dir::BindingTable,
        _ast_strings: &mut StringPool,
        _context: &mut UnbindContext,
    ) -> ast::TypePredicateSubject {
        match subject {
            dir::TypePredicateSubject::This => ast::TypePredicateSubject::This,
            dir::TypePredicateSubject::Identifier(name) => {
                let name = name;
                ast::TypePredicateSubject::Identifier(name)
            }
        }
    }

    /// Unbind if form to AST if form.
    #[inline]
    fn unbind_if_form(&self, _context: &mut UnbindContext, form: dir::IfForm) -> ast::IfForm {
        match form {
            dir::IfForm::If => ast::IfForm::If,
            dir::IfForm::Ternary => ast::IfForm::Ternary,
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

    /// Unbind for each operator to AST for each operator.
    #[inline]
    fn unbind_for_each_operator(
        &self,
        _context: &mut UnbindContext,
        operator: dir::ForEachOperator,
    ) -> ast::ForEachOperator {
        match operator {
            dir::ForEachOperator::In => ast::ForEachOperator::In,
            dir::ForEachOperator::Of => ast::ForEachOperator::Of,
        }
    }

    /// Unbind a for each binding keyword to AST binding keyword.
    #[inline]
    fn unbind_binding_keyword(
        &self,
        _context: &mut UnbindContext,
        keyword: dir::BindingKeyword,
    ) -> ast::BindingKeyword {
        match keyword {
            dir::BindingKeyword::Let => ast::BindingKeyword::Let,
            dir::BindingKeyword::Const => ast::BindingKeyword::Const,
        }
    }
}
