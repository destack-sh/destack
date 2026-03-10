use destack_ast::{self as ast};
use destack_core::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use super::UnbindContext;
use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Unbind a DIR binding modifier to an AST binding modifier.
    pub(super) fn unbind_binding_modifier(
        &self,
        _context: &mut UnbindContext,
        modifiers: &dir::BindingModifier,
    ) -> ast::BindingModifier {
        let kind = modifiers.kind.map(|kind| match kind {
            dir::BindingKind::Must => ast::BindingKind::Must,
            dir::BindingKind::Maybe => ast::BindingKind::Maybe,
        });
        let declaration = modifiers.declaration.map(|kind| match kind {
            dir::DeclarationKind::Declaration => ast::DeclarationKind::Declaration,
            dir::DeclarationKind::Definition => ast::DeclarationKind::Definition,
        });
        let abstraction = modifiers.abstraction.map(|abstraction| match abstraction {
            dir::AbstractionModifier::Abstract => ast::AbstractionModifier::Abstract,
            dir::AbstractionModifier::Override => ast::AbstractionModifier::Override,
            dir::AbstractionModifier::AbstractOverride => {
                ast::AbstractionModifier::AbstractOverride
            }
        });
        let variance = modifiers.variance.map(|variance| match variance {
            dir::VarianceModifier::In => ast::VarianceModifier::In,
            dir::VarianceModifier::Out => ast::VarianceModifier::Out,
            dir::VarianceModifier::InOut => ast::VarianceModifier::InOut,
        });
        let anchor = modifiers.anchor.map(|anchor| match anchor {
            dir::BindingAnchor::Static => ast::BindingAnchor::Static,
            dir::BindingAnchor::Instance => ast::BindingAnchor::Instance,
        });
        let mutability = modifiers.mutability.map(|mutability| match mutability {
            dir::Mutability::Immutable => ast::Mutability::Immutable,
            dir::Mutability::Mutable => ast::Mutability::Mutable,
        });
        let visibility = modifiers.visibility.map(|visibility| match visibility {
            dir::Visibility::Public => ast::Visibility::Public,
            dir::Visibility::Protected => ast::Visibility::Protected,
            dir::Visibility::Private => ast::Visibility::Private,
        });
        let operator = modifiers.operator.map(|operator| match operator {
            dir::BindingOperator::AsConst => ast::BindingOperator::AsConst,
        });
        let accessor = modifiers.accessor.map(|accessor| match accessor {
            dir::AccessorKind::Accessor => ast::AccessorKind::Accessor,
        });
        let timing = modifiers.timing.map(|timing| match timing {
            dir::Timing::Comptime => ast::Timing::Comptime,
        });
        ast::BindingModifier {
            kind,
            declaration,
            abstraction,
            variance,
            anchor,
            mutability,
            visibility,
            operator,
            accessor,
            timing,
        }
    }

    /// Unbind a DIR parameter to an AST parameter.
    pub(super) fn unbind_parameter(
        &self,
        module: &Module,
        parameter_id: dir::LocalNodeId<dir::Parameter>,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::Parameter> {
        let parameter = tree.get(parameter_id);
        let span = self.unbind_span(module, parameter_id.into());
        let ast_parameter = match parameter {
            dir::Parameter::Named {
                modifiers,
                name,
                default,
                ..
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.unbind_binding_modifier(context, &modifiers));
                let name = ast_strings.intern_from(&self.program.strings, *name);
                let default = default.map(|default| {
                    self.unbind_expression(
                        module,
                        default,
                        tree,
                        symbols,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });
                ast::Parameter::Named {
                    modifiers,
                    name,
                    ty: None,
                    default,
                }
            }
            dir::Parameter::Pattern {
                modifiers,
                pattern,
                default,
                ..
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.unbind_binding_modifier(context, &modifiers));
                let pattern = self.unbind_pattern(
                    module,
                    *pattern,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let ty = None;
                let default = default.map(|default| {
                    self.unbind_expression(
                        module,
                        default,
                        tree,
                        symbols,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });
                ast::Parameter::Pattern {
                    modifiers,
                    pattern,
                    ty,
                    default,
                }
            }
            dir::Parameter::VariadicNamed {
                modifiers, name, ..
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.unbind_binding_modifier(context, &modifiers));
                let name = ast_strings.intern_from(&self.program.strings, *name);
                let ty = None;
                ast::Parameter::VariadicNamed {
                    modifiers,
                    name,
                    ty,
                }
            }
            dir::Parameter::VariadicPattern {
                modifiers, pattern, ..
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.unbind_binding_modifier(context, &modifiers));
                let pattern = self.unbind_pattern(
                    module,
                    *pattern,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let ty = None;
                ast::Parameter::VariadicPattern {
                    modifiers,
                    pattern,
                    ty,
                }
            }
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
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::Argument> {
        let argument = tree.get(argument_id);
        let span = self.unbind_span(module, argument_id.into());
        let ast_argument = match argument {
            dir::Argument::Named {
                modifiers,
                name,
                value,
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.unbind_binding_modifier(context, &modifiers));
                let name =
                    ast::Name::Identifier(ast_strings.intern_from(&self.program.strings, *name));
                let value = self.unbind_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::Argument::Named {
                    modifiers,
                    name,
                    value,
                }
            }
            dir::Argument::Positional { modifiers, value } => {
                let modifiers =
                    modifiers.map(|modifiers| self.unbind_binding_modifier(context, &modifiers));
                let value = self.unbind_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::Argument::Positional { modifiers, value }
            }
            dir::Argument::Spread {
                modifiers,
                label,
                value,
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.unbind_binding_modifier(context, &modifiers));
                let label =
                    label.map(|label| ast_strings.intern_from(&self.program.strings, label));
                let value = self.unbind_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::Argument::Spread {
                    modifiers,
                    label,
                    value,
                }
            }
            dir::Argument::Labeled {
                modifiers,
                label,
                value,
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.unbind_binding_modifier(context, &modifiers));
                let label = ast_strings.intern_from(&self.program.strings, *label);
                let value = self.unbind_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::Argument::Labeled {
                    modifiers,
                    label,
                    value,
                }
            }
        };
        let ast_argument_id = ast_tree.insert(ast_argument, span);
        context.map(argument_id.into_any(), ast_argument_id.into_any());
        ast_argument_id
    }
}
