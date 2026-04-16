use destack_ast::{self as ast};
use destack_core::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use super::UnbindContext;
use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Unbind a DIR tuple element to an AST tuple element.
    fn unbind_tuple_element(
        &self,
        module: &Module,
        element_id: dir::LocalNodeId<dir::TupleElement>,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::TupleElement> {
        let element = tree.get(element_id);
        let span = self.unbind_span(module, element_id.into());

        let ast_element = match element {
            dir::TupleElement::Element {
                label,
                value,
                is_optional,
                is_readonly,
            } => {
                let label =
                    label.map(|label| ast_strings.intern_from(&self.repository.strings, label));
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

                ast::TupleElement::Element {
                    label,
                    value,
                    is_optional: *is_optional,
                    is_readonly: *is_readonly,
                }
            }
            dir::TupleElement::Spread { label, value } => {
                let label =
                    label.map(|label| ast_strings.intern_from(&self.repository.strings, label));
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

                ast::TupleElement::Spread { label, value }
            }
            dir::TupleElement::Error => ast::TupleElement::Error,
        };

        let ast_element_id = ast_tree.insert(ast_element, span);
        context.map(element_id.into_any(), ast_element_id.into_any());

        ast_element_id
    }

    /// Unbind a DIR type member to an AST type member.
    pub(super) fn unbind_type_member(
        &self,
        module: &Module,
        member_id: dir::LocalNodeId<dir::TypeMember>,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::TypeMember> {
        let member = tree.get(member_id);
        let span = self.unbind_span(module, member_id.into());

        let ast_member = match member {
            dir::TypeMember::Field {
                is_optional,
                is_readonly,
                key,
                declared_type,
                ..
            } => {
                let key = self.unbind_key(
                    module,
                    key,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let declared_type = self.unbind_type_expression(
                    module,
                    *declared_type,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );

                ast::TypeMember::Field {
                    is_optional: *is_optional,
                    is_readonly: *is_readonly,
                    key,
                    declared_type,
                }
            }
            dir::TypeMember::Method {
                is_optional,
                key,
                signature,
                body,
                ..
            } => {
                let key = key.map(|key| {
                    self.unbind_key(
                        module,
                        &key,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });
                let signature = self.unbind_function_signature(
                    module,
                    signature,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let body = body.map(|body| {
                    self.unbind_expression(
                        module,
                        body,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });

                ast::TypeMember::Method {
                    is_optional: *is_optional,
                    key,
                    signature,
                    body,
                }
            }
            dir::TypeMember::IndexSignature {
                is_optional,
                is_readonly,
                name,
                key_type,
                value_type,
                ..
            } => {
                let name = ast_strings.intern_from(&self.repository.strings, *name);
                let key_type = self.unbind_type_expression(
                    module,
                    *key_type,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let value_type = self.unbind_type_expression(
                    module,
                    *value_type,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );

                ast::TypeMember::IndexSignature {
                    is_optional: *is_optional,
                    is_readonly: *is_readonly,
                    name,
                    key_type,
                    value_type,
                }
            }
            dir::TypeMember::Embed { value, .. } => {
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

                ast::TypeMember::Embed { value }
            }
            dir::TypeMember::AssociatedType {
                name,
                generic_parameters,
                where_clauses,
                constraint,
                value,
                ..
            } => {
                let name = ast_strings.intern_from(&self.repository.strings, *name);
                let generic_parameters = generic_parameters
                    .iter()
                    .map(|parameter| {
                        self.unbind_generic_parameter(
                            module,
                            *parameter,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                let where_clauses = where_clauses
                    .iter()
                    .map(|where_clause| {
                        self.unbind_where_clause(
                            module,
                            *where_clause,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                let constraint = constraint.map(|constraint| {
                    self.unbind_type_expression(
                        module,
                        constraint,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });
                let value = value.map(|value| {
                    self.unbind_type_expression(
                        module,
                        value,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });

                ast::TypeMember::AssociatedType {
                    name,
                    generic_parameters,
                    where_clauses,
                    constraint,
                    value,
                }
            }
            dir::TypeMember::AssociatedConst {
                name,
                declared_type,
                value,
                ..
            } => {
                let name = ast_strings.intern_from(&self.repository.strings, *name);
                let declared_type = declared_type.map(|declared_type| {
                    self.unbind_type_expression(
                        module,
                        declared_type,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });
                let value = value.map(|value| {
                    self.unbind_expression(
                        module,
                        value,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });

                ast::TypeMember::AssociatedConst {
                    name,
                    declared_type,
                    value,
                }
            }
            dir::TypeMember::Error { .. } => ast::TypeMember::Error,
        };

        let ast_member_id = ast_tree.insert(ast_member, span);
        context.map(member_id.into_any(), ast_member_id.into_any());

        ast_member_id
    }

    /// Unbind a DIR type expression to an AST type expression.
    pub(super) fn unbind_type_expression(
        &self,
        module: &Module,
        expression_id: dir::LocalNodeId<dir::TypeExpression>,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::TypeExpression> {
        let expression = tree.get(expression_id);
        let span = self.unbind_span(module, expression_id.into());

        let ast_type_expression = match expression {
            dir::TypeExpression::Parenthesized { expression } => {
                let expression = self.unbind_type_expression(
                    module,
                    *expression,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );

                ast::TypeExpression::Parenthesized { expression }
            }
            dir::TypeExpression::ScalarLiteral { value } => {
                let value = self.unbind_scalar_literal(value, ast_strings, context);
                ast::TypeExpression::ScalarLiteral { value }
            }
            dir::TypeExpression::Literal { value } => {
                let value = self.unbind_type_literal(value, context);
                ast::TypeExpression::Literal { value }
            }
            dir::TypeExpression::Intrinsic => ast::TypeExpression::Intrinsic,
            dir::TypeExpression::Tuple { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| {
                        self.unbind_tuple_element(
                            module,
                            *element,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();

                ast::TypeExpression::Tuple { elements }
            }
            dir::TypeExpression::Array { element } => {
                let element = self.unbind_type_expression(
                    module,
                    *element,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );

                ast::TypeExpression::Array { element }
            }
            dir::TypeExpression::Object { members } => {
                let members = members
                    .iter()
                    .map(|member| {
                        self.unbind_type_member(
                            module,
                            *member,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();

                ast::TypeExpression::Object { members }
            }
            dir::TypeExpression::Declaration { declaration } => {
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

                ast::TypeExpression::Declaration { declaration }
            }
            dir::TypeExpression::Reference {
                path,
                generic_arguments,
                space_order: _,
            }
            | dir::TypeExpression::LocalReference {
                path,
                generic_arguments,
                target_symbol: _,
            }
            | dir::TypeExpression::ModuleReference {
                path,
                generic_arguments,
                target_symbol: _,
            }
            | dir::TypeExpression::GlobalReference {
                path,
                generic_arguments,
                target_symbol: _,
            } => {
                let path = self.unbind_path(path, ast_strings, context);
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

                ast::TypeExpression::Reference {
                    path,
                    generic_arguments,
                }
            }
            dir::TypeExpression::Member {
                left,
                name,
                generic_arguments,
            } => {
                let left = self.unbind_type_expression(
                    module,
                    *left,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let name = ast_strings.intern_from(&self.repository.strings, *name);
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

                ast::TypeExpression::Member {
                    left,
                    name,
                    generic_arguments,
                }
            }
            dir::TypeExpression::Const => ast::TypeExpression::Const,
            dir::TypeExpression::This => ast::TypeExpression::This,
            dir::TypeExpression::Import {
                target,
                arguments,
                qualifier,
                generic_arguments,
            } => {
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
                let arguments = arguments
                    .iter()
                    .map(|argument| {
                        self.unbind_argument(
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
                let qualifier = qualifier
                    .as_ref()
                    .map(|qualifier| self.unbind_path(qualifier, ast_strings, context));
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

                ast::TypeExpression::Import {
                    target,
                    arguments,
                    qualifier,
                    generic_arguments,
                }
            }
            dir::TypeExpression::Readonly { target_type } => {
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

                ast::TypeExpression::Readonly { target_type }
            }
            dir::TypeExpression::KeyOf { target_type } => {
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

                ast::TypeExpression::KeyOf { target_type }
            }
            dir::TypeExpression::TypeOfValue { value } => {
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

                ast::TypeExpression::TypeOfValue { value }
            }
            dir::TypeExpression::Must { target_type } => {
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

                ast::TypeExpression::Must { target_type }
            }
            dir::TypeExpression::AsComptime { target_type } => {
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

                ast::TypeExpression::AsComptime { target_type }
            }
            dir::TypeExpression::Not { target_type } => {
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

                ast::TypeExpression::Not { target_type }
            }
            dir::TypeExpression::ValueOf {
                mutability,
                variance,
                target_type,
            } => {
                let mutability =
                    mutability.map(|mutability| self.unbind_mutability(context, mutability));
                let variance =
                    variance.map(|variance| self.unbind_variance_bound(context, variance));
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

                ast::TypeExpression::ValueOf {
                    mutability,
                    variance,
                    target_type,
                }
            }
            dir::TypeExpression::ReferenceOf {
                mutability,
                variance,
                target_type,
            } => {
                let mutability =
                    mutability.map(|mutability| self.unbind_mutability(context, mutability));
                let variance =
                    variance.map(|variance| self.unbind_variance_bound(context, variance));
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

                ast::TypeExpression::ReferenceOf {
                    mutability,
                    variance,
                    target_type,
                }
            }
            dir::TypeExpression::PointerOf {
                mutability,
                target_type,
            } => {
                let mutability =
                    mutability.map(|mutability| self.unbind_mutability(context, mutability));
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

                ast::TypeExpression::PointerOf {
                    mutability,
                    target_type,
                }
            }
            dir::TypeExpression::Union { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| {
                        self.unbind_type_expression(
                            module,
                            *element,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();

                ast::TypeExpression::Union { elements }
            }
            dir::TypeExpression::Intersection { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| {
                        self.unbind_type_expression(
                            module,
                            *element,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();

                ast::TypeExpression::Intersection { elements }
            }
            dir::TypeExpression::Conditional {
                left,
                extends_type,
                then_type,
                else_type,
            } => {
                let left = self.unbind_type_expression(
                    module,
                    *left,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let extends_type = self.unbind_type_expression(
                    module,
                    *extends_type,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let then_type = self.unbind_type_expression(
                    module,
                    *then_type,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let else_type = self.unbind_type_expression(
                    module,
                    *else_type,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );

                ast::TypeExpression::Conditional {
                    left,
                    extends_type,
                    then_type,
                    else_type,
                }
            }
            dir::TypeExpression::Mapped {
                parameter,
                readonly,
                optional,
                value,
            } => {
                let name = ast_strings.intern_from(&self.repository.strings, parameter.name);
                let source_type = self.unbind_type_expression(
                    module,
                    parameter.source_type,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let key_remap = parameter.key_remap.map(|key_remap| {
                    self.unbind_type_expression(
                        module,
                        key_remap,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });
                let parameter = ast::TypeMappedParameter {
                    name,
                    source_type,
                    key_remap,
                };
                let readonly = self.unbind_type_modifier(*readonly);
                let optional = self.unbind_type_modifier(*optional);
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

                ast::TypeExpression::Mapped {
                    parameter,
                    readonly,
                    optional,
                    value,
                }
            }
            dir::TypeExpression::Index { left, index } => {
                let left = self.unbind_type_expression(
                    module,
                    *left,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let index = self.unbind_type_expression(
                    module,
                    *index,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );

                ast::TypeExpression::Index { left, index }
            }
            dir::TypeExpression::TemplateLiteral { strings, spans } => {
                let strings = strings
                    .iter()
                    .map(|string| ast_strings.intern_from(&self.repository.strings, *string))
                    .collect();
                let spans = spans
                    .iter()
                    .map(|span| {
                        self.unbind_type_expression(
                            module,
                            *span,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();

                ast::TypeExpression::TemplateLiteral { strings, spans }
            }
            dir::TypeExpression::Infer { name, constraint } => {
                let name = ast_strings.intern_from(&self.repository.strings, *name);
                let constraint = constraint.map(|constraint| {
                    self.unbind_type_expression(
                        module,
                        constraint,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });

                ast::TypeExpression::Infer { name, constraint }
            }
            dir::TypeExpression::Predicate {
                asserts,
                subject,
                target,
            } => {
                let subject = self.unbind_type_predicate_subject(
                    *subject,
                    module,
                    symbols,
                    ast_strings,
                    context,
                );
                let target = target.map(|target| {
                    self.unbind_type_expression(
                        module,
                        target,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });

                ast::TypeExpression::Predicate {
                    asserts: *asserts,
                    subject,
                    target,
                }
            }
            dir::TypeExpression::Missing => ast::TypeExpression::Missing,
            dir::TypeExpression::Error => ast::TypeExpression::Error,
        };

        let ast_expression_id = ast_tree.insert(ast_type_expression, span);
        context.map(expression_id.into_any(), ast_expression_id.into_any());

        ast_expression_id
    }

    /// Unbind a DIR type modifier to an AST type modifier.
    #[inline]
    fn unbind_type_modifier(&self, modifier: dir::TypeModifier) -> ast::TypeModifier {
        match modifier {
            dir::TypeModifier::Present => ast::TypeModifier::Present,
            dir::TypeModifier::Add => ast::TypeModifier::Add,
            dir::TypeModifier::Remove => ast::TypeModifier::Remove,
            dir::TypeModifier::None => ast::TypeModifier::None,
        }
    }

    /// Unbind a DIR mutability to an AST mutability.
    #[inline]
    pub(super) fn unbind_mutability(
        &self,
        _context: &mut UnbindContext,
        mutability: dir::Mutability,
    ) -> ast::Mutability {
        match mutability {
            dir::Mutability::Immutable => ast::Mutability::Immutable,
            dir::Mutability::Mutable => ast::Mutability::Mutable,
        }
    }

    /// Unbind a DIR variance bound to an AST variance bound.
    #[inline]
    pub(super) fn unbind_variance_bound(
        &self,
        _context: &mut UnbindContext,
        variance: dir::VarianceBound,
    ) -> ast::VarianceBound {
        match variance {
            dir::VarianceBound::Implements => ast::VarianceBound::Implements,
            dir::VarianceBound::Extends => ast::VarianceBound::Extends,
            dir::VarianceBound::Super => ast::VarianceBound::Super,
        }
    }

    /// Unbind a DIR asynchrony to an AST asynchrony.
    #[inline]
    pub(super) fn unbind_asynchrony(
        &self,
        _context: &mut UnbindContext,
        asynchrony: dir::Asynchrony,
    ) -> ast::Asynchrony {
        match asynchrony {
            dir::Asynchrony::Sync => ast::Asynchrony::Sync,
            dir::Asynchrony::Async => ast::Asynchrony::Async,
        }
    }
}
