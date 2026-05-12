use destack_ast::{self as ast};
use destack_core::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use super::UnbindContext;
use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Unbind whether a DIR declaration is ambient.
    pub(super) fn unbind_ambientness(
        &self,
        is_ambient: bool,
        _context: &mut UnbindContext,
    ) -> bool {
        is_ambient
    }

    /// Unbind a DIR method abstraction into an AST method abstraction.
    fn unbind_method_abstraction(
        &self,
        abstraction: dir::MethodAbstraction,
    ) -> ast::MethodAbstraction {
        match abstraction {
            dir::MethodAbstraction::Concrete => ast::MethodAbstraction::Concrete,
            dir::MethodAbstraction::Virtual => ast::MethodAbstraction::Virtual,
            dir::MethodAbstraction::Abstract => ast::MethodAbstraction::Abstract,
        }
    }

    /// Unbind a DIR property to an AST property.
    pub(super) fn unbind_property(
        &self,
        module: &Module,
        property_id: dir::LocalNodeId<dir::Property>,
        tree: &dir::Tree,
        symbols: &dir::BindingTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::Tree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::Property> {
        let property = tree.get(property_id);
        let span = self.unbind_span(module, property_id.into());

        let ast_property = match property {
            dir::Property::Field {
                key,
                value,
                is_shorthand,
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

                ast::Property::Field {
                    key,
                    value,
                    is_shorthand: *is_shorthand,
                }
            }
            dir::Property::Method {
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

                ast::Property::Method {
                    key,
                    signature,
                    body,
                }
            }
            dir::Property::Spread { value, .. } => {
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

                ast::Property::Spread { value }
            }
            dir::Property::Error { .. } => ast::Property::Error,
        };

        let ast_property_id = ast_tree.insert(ast_property, span);
        context.map(property_id.into_any(), ast_property_id.into_any());
        ast_property_id
    }

    /// Unbind a DIR member to an AST member.
    pub(super) fn unbind_member(
        &self,
        module: &Module,
        member_id: dir::LocalNodeId<dir::Member>,
        tree: &dir::Tree,
        symbols: &dir::BindingTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::Tree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::Member> {
        let member = tree.get(member_id);
        let span = self.unbind_span(module, member_id.into());

        let ast_member = match member {
            dir::Member::AssociatedType {
                name,
                generic_parameters,
                where_clauses,
                constraint,
                value,
                visibility,
                is_ambient,
                is_abstract,
                is_override,
                is_static,
                ..
            } => {
                let name = *name;
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

                ast::Member::AssociatedType {
                    name,
                    generic_parameters,
                    where_clauses,
                    constraint,
                    value,
                    visibility: visibility
                        .map(|visibility| self.unbind_visibility(visibility, context)),
                    is_ambient: self.unbind_ambientness(*is_ambient, context),
                    is_abstract: *is_abstract,
                    is_override: *is_override,
                    is_static: *is_static,
                }
            }
            dir::Member::AssociatedConst {
                name,
                declared_type,
                value,
                visibility,
                is_ambient,
                is_static,
                ..
            } => {
                let name = *name;
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

                ast::Member::AssociatedConst {
                    name,
                    declared_type,
                    value,
                    visibility: visibility
                        .map(|visibility| self.unbind_visibility(visibility, context)),
                    is_ambient: self.unbind_ambientness(*is_ambient, context),
                    is_static: *is_static,
                }
            }
            dir::Member::Field {
                key,
                declared_type,
                default,
                is_optional,
                is_readonly,
                mutability,
                visibility,
                is_ambient,
                is_abstract,
                is_override,
                is_static,
                is_accessor,
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
                let default = default.map(|default| {
                    self.unbind_expression(
                        module,
                        default,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });

                ast::Member::Field {
                    key,
                    declared_type,
                    default,
                    is_optional: *is_optional,
                    is_readonly: *is_readonly,
                    mutability: mutability
                        .map(|mutability| self.unbind_mutability(context, mutability)),
                    visibility: visibility
                        .map(|visibility| self.unbind_visibility(visibility, context)),
                    is_ambient: self.unbind_ambientness(*is_ambient, context),
                    is_abstract: *is_abstract,
                    is_override: *is_override,
                    is_static: *is_static,
                    is_accessor: *is_accessor,
                }
            }
            dir::Member::Method {
                key,
                signature,
                abstraction,
                body,
                visibility,
                is_optional,
                is_ambient,
                is_override,
                is_static,
                is_accessor,
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

                ast::Member::Method {
                    key,
                    signature,
                    abstraction: self.unbind_method_abstraction(*abstraction),
                    body,
                    visibility: visibility
                        .map(|visibility| self.unbind_visibility(visibility, context)),
                    is_ambient: self.unbind_ambientness(*is_ambient, context),
                    is_optional: *is_optional,
                    is_override: *is_override,
                    is_static: *is_static,
                    is_accessor: *is_accessor,
                }
            }
            dir::Member::StaticBlock { body, .. } => {
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

                ast::Member::StaticBlock { body }
            }
            dir::Member::ComptimeBlock { body, .. } => {
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

                ast::Member::ComptimeBlock { body }
            }
            dir::Member::Error { .. } => ast::Member::Error,
        };

        let ast_member_id = ast_tree.insert(ast_member, span);
        context.map(member_id.into_any(), ast_member_id.into_any());
        ast_member_id
    }
}
