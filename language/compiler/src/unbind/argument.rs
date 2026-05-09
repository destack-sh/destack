use destack_ast::{self as ast};
use destack_core::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use super::UnbindContext;
use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Unbind a DIR visibility to an AST visibility.
    #[inline]
    pub(super) fn unbind_visibility(
        &self,
        visibility: dir::Visibility,
        _context: &mut UnbindContext,
    ) -> ast::Visibility {
        match visibility {
            dir::Visibility::Public => ast::Visibility::Public,
            dir::Visibility::Protected => ast::Visibility::Protected,
            dir::Visibility::Private => ast::Visibility::Private,
        }
    }

    /// Unbind a DIR parameter to an AST parameter.
    pub(super) fn unbind_parameter(
        &self,
        module: &Module,
        parameter_id: dir::LocalNodeId<dir::Parameter>,
        tree: &dir::Tree,
        symbols: &dir::BindingTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::Tree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::Parameter> {
        let parameter = tree.get(parameter_id);
        let span = self.unbind_span(module, parameter_id.into());

        let ast_parameter = match parameter {
            dir::Parameter::Named {
                name,
                visibility,
                is_readonly,
                is_optional,
                declared_type,
                default,
                ..
            } => {
                let name = *name;
                let visibility =
                    visibility.map(|visibility| self.unbind_visibility(visibility, context));
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
                ast::Parameter::Named {
                    name,
                    visibility,
                    is_readonly: *is_readonly,
                    is_optional: *is_optional,
                    declared_type,
                    default,
                }
            }
            dir::Parameter::Pattern {
                pattern,
                is_optional,
                declared_type,
                default,
                ..
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
                ast::Parameter::Pattern {
                    pattern,
                    is_optional: *is_optional,
                    declared_type,
                    default,
                }
            }
            dir::Parameter::VariadicNamed {
                name,
                visibility,
                is_readonly,
                declared_type,
                ..
            } => {
                let name = *name;
                let visibility =
                    visibility.map(|visibility| self.unbind_visibility(visibility, context));
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
                ast::Parameter::VariadicNamed {
                    name,
                    visibility,
                    is_readonly: *is_readonly,
                    declared_type,
                }
            }
            dir::Parameter::VariadicPattern {
                pattern,
                declared_type,
                ..
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
                ast::Parameter::VariadicPattern {
                    pattern,
                    declared_type,
                }
            }
            dir::Parameter::Error { .. } => ast::Parameter::Error,
        };

        let ast_parameter_id = ast_tree.insert(ast_parameter, span);
        context.map(parameter_id.into_any(), ast_parameter_id.into_any());
        ast_parameter_id
    }

    /// Unbind a DIR argument to an AST argument.
    pub(super) fn unbind_argument(
        &self,
        module: &Module,
        argument_id: dir::LocalNodeId<dir::Argument>,
        tree: &dir::Tree,
        symbols: &dir::BindingTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::Tree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::Argument> {
        let argument = tree.get(argument_id);
        let span = self.unbind_span(module, argument_id.into());

        let ast_argument = match argument {
            dir::Argument::Named { name, value } => {
                let name = self.unbind_name(ast_strings, *name);
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
                ast::Argument::Named { name, value }
            }
            dir::Argument::Labeled { label, value } => {
                let label = *label;
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
                ast::Argument::Labeled { label, value }
            }
            dir::Argument::Positional { value } => {
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
                ast::Argument::Positional { value }
            }
            dir::Argument::Spread { label, value } => {
                let label = label.map(|label| label);
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
                ast::Argument::Spread { label, value }
            }
            dir::Argument::Error { .. } => ast::Argument::Error,
        };

        let ast_argument_id = ast_tree.insert(ast_argument, span);
        context.map(argument_id.into_any(), ast_argument_id.into_any());
        ast_argument_id
    }
}
