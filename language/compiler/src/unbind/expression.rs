use destack_ast::{self as ast};
use destack_core::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use super::UnbindContext;
use crate::Compiler;

impl Compiler {
    /// Unbind a DIR export mode to an AST export mode.
    pub(super) fn unbind_export_mode(&self, export: dir::ExportMode) -> ast::ExportMode {
        match export {
            dir::ExportMode::Named => ast::ExportMode::Named,
            dir::ExportMode::Default => ast::ExportMode::Default,
        }
    }

    /// Unbind a DIR generic argument to an AST generic argument.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn unbind_generic_argument(
        &self,
        module: &Module,
        argument_id: dir::LocalNodeId<dir::GenericArgument>,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::NodeTree,
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

    destack_core::ensure_sufficient_stack! {
        /// Unbind a DIR expression to an AST expression.
        #[allow(clippy::too_many_arguments)]
        pub fn unbind_expression(
            &self,
            module: &Module,
            expression_id: dir::LocalNodeId<dir::Expression>,
            tree: &dir::NodeTree,
            symbols: &dir::SymbolTable,
            types: &dir::TypeTable,
            ast_tree: &mut ast::NodeTree,
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
                    let label = ast_strings.intern_from(&self.repository.strings, *label);
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

                dir::Expression::UnresolvedImport {
                    source,
                    kind,
                    target,
                    items,
                    attributes,
                    arguments,
                } => {
                    let source = self.unbind_import_source(*source);
                    let kind = self.unbind_dependency_kind(context, *kind);
                    let target = match target {
                        dir::ImportTarget::String(target) => {
                            ast::ImportTarget::String(
                                ast_strings.intern_from(&self.repository.strings, *target),
                            )
                        }
                        dir::ImportTarget::Expression { target } => {
                            let target = self.unbind_expression(
                                module,
                                *target,
                                tree,
                                symbols,
                                types,
                                ast_tree,
                                ast_strings,
                                context,
                            );
                            ast::ImportTarget::Expression { target }
                        }
                    };
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
                    let arguments = arguments.as_ref().map(|args| {
                        args.iter().map(|arg| {
                            self.unbind_argument(module, *arg, tree, symbols, types, ast_tree, ast_strings, context)
                        }).collect()
                    });
                    ast::Expression::Import {
                        source,
                        kind,
                        target,
                        items,
                        attributes,
                        arguments,
                    }
                }

                dir::Expression::Import {
                    source,
                    kind,
                    target,
                    items,
                    attributes,
                    arguments,
                    ..
                } => {
                    let source = self.unbind_import_source(*source);
                    let kind = self.unbind_dependency_kind(context, *kind);
                    let target = ast::ImportTarget::String(
                        ast_strings.intern_from(&self.repository.strings, *target),
                    );
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
                    let arguments = arguments.as_ref().map(|args| {
                        args.iter().map(|arg| {
                            self.unbind_argument(module, *arg, tree, symbols, types, ast_tree, ast_strings, context)
                        }).collect()
                    });
                    ast::Expression::Import {
                        source,
                        kind,
                        target,
                        items,
                        attributes,
                        arguments,
                    }
                }

                dir::Expression::UnresolvedReExport {
                    target,
                    kind,
                    items,
                    attributes,
                }
                | dir::Expression::ReExport {
                    target,
                    kind,
                    items,
                    attributes,
                    ..
                } => {
                    let kind = self.unbind_dependency_kind(context, *kind);
                    let target = ast_strings.intern_from(&self.repository.strings, *target);
                    let items = items.iter().map(|item| {
                        self.unbind_dependency_item(module, *item, tree, symbols, types, ast_tree, ast_strings, context)
                    }).collect();
                    let attributes = attributes.as_ref().map(|attributes| {
                        self.unbind_import_attribute_clause(attributes, ast_strings, context)
                    });
                    ast::Expression::Export {
                        kind,
                        target: Some(target),
                        items,
                        attributes,
                    }
                }

                dir::Expression::Export {
                    kind,
                    items,
                    attributes,
                } => {
                    let kind = self.unbind_dependency_kind(context, *kind);
                    let items = items.iter().map(|item| {
                        self.unbind_dependency_item(module, *item, tree, symbols, types, ast_tree, ast_strings, context)
                    }).collect();
                    let attributes = attributes.as_ref().map(|attributes| {
                        self.unbind_import_attribute_clause(attributes, ast_strings, context)
                    });
                    ast::Expression::Export {
                        kind,
                        target: None,
                        items,
                        attributes,
                    }
                }
                dir::Expression::ExportNamespace { name } => {
                    let name = ast_strings.intern_from(&self.repository.strings, *name);
                    ast::Expression::ExportNamespace { name }
                }

                dir::Expression::Let {
                    export,
                    ambient,
                    mutability,
                    declarators,
                } => {
                    // DIR does not preserve the original `let` vs `var` spelling
                    let ast_mutability = self.unbind_mutability(context, *mutability);
                    let kind = match mutability {
                        dir::Mutability::Immutable => ast::LetKind::Const,
                        dir::Mutability::Mutable => ast::LetKind::Let,
                    };
                    let export = export.map(|export| self.unbind_export_mode(export));
                    let ambient = self.unbind_ambientness(*ambient, context);
                    let declarators = declarators.iter().map(|decl| {
                        self.unbind_declarator(module, *decl, tree, symbols, types, ast_tree, ast_strings, context)
                    }).collect();
                    ast::Expression::Let {
                        kind,
                        export,
                        ambient,
                        mutability: ast_mutability,
                        declarators,
                    }
                }
                dir::Expression::Using {
                    asynchrony,
                    export,
                    ambient,
                    declarators,
                } => {
                    let asynchrony = self.unbind_asynchrony(context, *asynchrony);
                    let export = export.map(|export| self.unbind_export_mode(export));
                    let ambient = self.unbind_ambientness(*ambient, context);
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
                        ambient,
                        declarators,
                    }
                }

                dir::Expression::As {
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

                dir::Expression::OwnershipCast {
                    operator: _,
                    source: _,
                    value,
                } => {
                    // ownership casts are internal, preserve the underlying expression
                    return self.unbind_expression(
                        module,
                        *value,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    );
                }

                dir::Expression::Unary { operator, right } => {
                    let operator = self.unbind_unary_operator(context, *operator);
                    let right = self.unbind_expression(module, *right, tree, symbols, types, ast_tree, ast_strings, context);
                    ast::Expression::Unary { operator, right }
                }

                dir::Expression::ValueOf { mutability, variance, right } => {
                    let mutability = mutability.map(|m| self.unbind_mutability(context, m));
                    let variance = variance.map(|v| self.unbind_variance_bound(context, v));
                    let right = self.unbind_expression(module, *right, tree, symbols, types, ast_tree, ast_strings, context);
                    ast::Expression::ValueOf { mutability, variance, right }
                }

                dir::Expression::ReferenceOf { mutability, variance, right } => {
                    let mutability = mutability.map(|m| self.unbind_mutability(context, m));
                    let variance = variance.map(|v| self.unbind_variance_bound(context, v));
                    let right = self.unbind_expression(module, *right, tree, symbols, types, ast_tree, ast_strings, context);
                    ast::Expression::ReferenceOf { mutability, variance, right }
                }
                dir::Expression::PointerOf { mutability, right } => {
                    let mutability = mutability.map(|m| self.unbind_mutability(context, m));
                    let right = self.unbind_expression(module, *right, tree, symbols, types, ast_tree, ast_strings, context);
                    ast::Expression::PointerOf { mutability, right }
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
                    let left = self.unbind_expression(module, *left, tree, symbols, types, ast_tree, ast_strings, context);
                    let right = self.unbind_expression(module, *right, tree, symbols, types, ast_tree, ast_strings, context);
                    ast::Expression::Assign { left, operator: ast::AssignOperator::Assign, right }
                }

                dir::Expression::AssignBinary { left, operator, right } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, types, ast_tree, ast_strings, context);
                    let operator = self.unbind_assign_operator(context, *operator);
                    let right = self.unbind_expression(module, *right, tree, symbols, types, ast_tree, ast_strings, context);
                    ast::Expression::Assign { left, operator, right }
                }

                dir::Expression::Member {
                    left,
                    name,
                    generic_arguments,
                } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, types, ast_tree, ast_strings, context);
                    let name = name.map(|name| ast_strings.intern_from(&self.repository.strings, name));
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
                    ast::Expression::Member { left, name, generic_arguments }
                }
                dir::Expression::PrivateMember {
                    left,
                    name,
                    generic_arguments,
                } => {
                    let left = self.unbind_expression(module, *left, tree, symbols, types, ast_tree, ast_strings, context);
                    let name = name.map(|name| ast_strings.intern_from(&self.repository.strings, name));
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
                    ast::Expression::PrivateMember { left, name, generic_arguments }
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

                dir::Expression::Delete { value } => {
                    let value = self.unbind_expression(module, *value, tree, symbols, types, ast_tree, ast_strings, context);
                    ast::Expression::Delete { value }
                }

                dir::Expression::UnresolvedPath {
                    path,
                    generic_arguments,
                    space_order: _,
                }
                | dir::Expression::LocalReference { path, generic_arguments, .. }
                | dir::Expression::ModuleReference { path, generic_arguments, .. }
                | dir::Expression::GlobalReference { path, generic_arguments, .. } => {
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
                    let name = ast_strings.intern_from(&self.repository.strings, *name);
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

                dir::Expression::Type {
                    value,
                    resolved_type: _,
                } => {
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

                dir::Expression::TaggedTemplateExpression { tag, value } => {
                    let tag = self.unbind_expression(module, *tag, tree, symbols, types, ast_tree, ast_strings, context);
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
                    ast::Expression::TaggedTemplateExpression { tag, value }
                }

                dir::Expression::ArrayExpression { elements } => {
                    let elements = elements.iter().map(|el| {
                        self.unbind_argument(module, *el, tree, symbols, types, ast_tree, ast_strings, context)
                    }).collect();
                    ast::Expression::ArrayExpression { elements }
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

                dir::Expression::TreeExpression { left, arguments, elements } => {
                    let left = left.map(|l| self.unbind_expression(module, l, tree, symbols, types, ast_tree, ast_strings, context));
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
                    ast::Expression::TreeExpression { left, arguments, elements }
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

                dir::Expression::If { kind, condition, then_expression, else_expression } => {
                    let kind = self.unbind_if_kind(context, *kind);
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
                                dir::LetKind::Var => ast::LetKind::Var,
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
                    ast::Expression::If { kind, condition, then_expression, else_expression }
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
                            ast::Expression::While { kind: ast::WhileKind::While, condition, body }
                        }
                        dir::LoopKind::PostTest => {
                            let condition = condition.map(|c| {
                                self.unbind_expression(module, c, tree, symbols, types, ast_tree, ast_strings, context)
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
                        dir::ForEachBinding::Pattern {
                            pattern,
                            declaration_kind,
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
                            let declaration_kind = declaration_kind
                                .map(|kind| self.unbind_for_each_declaration_kind(context, kind));
                            ast::ForEachBinding::Pattern {
                                pattern,
                                declaration_kind,
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
                        kind,
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

                dir::Expression::Match { kind, value, cases, .. } => {
                    let kind = match kind {
                        dir::MatchKind::Match => ast::MatchKind::Match,
                        dir::MatchKind::Switch => ast::MatchKind::Switch,
                    };
                    let value = self.unbind_expression(module, *value, tree, symbols, types, ast_tree, ast_strings, context);
                    let cases = cases.iter().map(|case| {
                        self.unbind_match_case(
                            module,
                            *case,
                            kind,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    }).collect();
                    ast::Expression::Match { kind, value, cases }
                }

                dir::Expression::UnresolvedBreak { target, value } => {
                    let label = Some(ast_strings.intern_from(&self.repository.strings, *target));
                    let value = value.map(|v| self.unbind_expression(module, v, tree, symbols, types, ast_tree, ast_strings, context));
                    ast::Expression::Break { label, value }
                }

                dir::Expression::Break { target, value, .. } => {
                    let label = target.map(|t| ast_strings.intern_from(&self.repository.strings, t));
                    let value = value.map(|v| self.unbind_expression(module, v, tree, symbols, types, ast_tree, ast_strings, context));
                    ast::Expression::Break { label, value }
                }

                dir::Expression::UnresolvedContinue { target } => {
                    let label = Some(ast_strings.intern_from(&self.repository.strings, *target));
                    ast::Expression::Continue { label }
                }

                dir::Expression::Continue { target, .. } => {
                    let label = target.map(|t| ast_strings.intern_from(&self.repository.strings, t));
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
        _symbols: &dir::SymbolTable,
        ast_strings: &mut StringPool,
        _context: &mut UnbindContext,
    ) -> ast::TypePredicateSubject {
        match subject {
            dir::TypePredicateSubject::This => ast::TypePredicateSubject::This,
            dir::TypePredicateSubject::Identifier(name) => {
                let name = ast_strings.intern_from(&self.repository.strings, name);
                ast::TypePredicateSubject::Identifier(name)
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

    /// Unbind for each declaration kind to AST for each declaration kind.
    #[inline]
    fn unbind_for_each_declaration_kind(
        &self,
        _context: &mut UnbindContext,
        kind: dir::ForEachDeclarationKind,
    ) -> ast::ForEachDeclarationKind {
        match kind {
            dir::ForEachDeclarationKind::Var => ast::ForEachDeclarationKind::Var,
            dir::ForEachDeclarationKind::Let => ast::ForEachDeclarationKind::Let,
            dir::ForEachDeclarationKind::Const => ast::ForEachDeclarationKind::Const,
        }
    }

    /// Convert a DIR import source into an AST import source.
    fn unbind_import_source(&self, source: dir::ImportSource) -> ast::ImportSource {
        match source {
            dir::ImportSource::ImportStatement => ast::ImportSource::ImportStatement,
            dir::ImportSource::ReferencePathDirective => ast::ImportSource::ReferencePathDirective,
            dir::ImportSource::ReferenceTypesDirective => {
                ast::ImportSource::ReferenceTypesDirective
            }
            dir::ImportSource::ReferenceLibDirective => ast::ImportSource::ReferenceLibDirective,
            dir::ImportSource::ReferenceNoDefaultLibDirective => {
                ast::ImportSource::ReferenceNoDefaultLibDirective
            }
            dir::ImportSource::ImportEquals => ast::ImportSource::ImportEquals,
            dir::ImportSource::ImportCall => ast::ImportSource::ImportCall,
            dir::ImportSource::RequireCall
            | dir::ImportSource::ExportStatement
            | dir::ImportSource::ValueExpression => ast::ImportSource::ImportStatement,
        }
    }
}
