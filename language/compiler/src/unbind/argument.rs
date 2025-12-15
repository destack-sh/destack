use destack_ast::{self as ast};
use destack_base::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use crate::Compiler;

impl Compiler {
    /// Unbind a DIR binding modifier to an AST binding modifier.
    pub(super) fn unbind_binding_modifier(
        &self,
        modifiers: &dir::BindingModifier,
    ) -> ast::BindingModifier {
        let kind = modifiers.kind.map(|kind| match kind {
            dir::BindingKind::Must => ast::BindingKind::Must,
            dir::BindingKind::Maybe => ast::BindingKind::Maybe,
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
        ast::BindingModifier {
            kind,
            anchor,
            mutability,
            visibility,
            operator,
            accessor,
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
                let modifiers = modifiers.map(|modifiers| self.unbind_binding_modifier(&modifiers));
                let name = ast_strings.intern_from(&self.program.strings, *name);
                let default = default.map(|default| {
                    self.unbind_expression(module, default, tree, symbols, ast_tree, ast_strings)
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
                let modifiers = modifiers.map(|modifiers| self.unbind_binding_modifier(&modifiers));
                let pattern =
                    self.unbind_pattern(module, *pattern, tree, symbols, ast_tree, ast_strings);
                let ty = None;
                let default = default.map(|default| {
                    self.unbind_expression(module, default, tree, symbols, ast_tree, ast_strings)
                });
                ast::Parameter::Pattern {
                    modifiers,
                    pattern,
                    ty,
                    default,
                }
            }
            dir::Parameter::Variadic {
                modifiers, name, ..
            } => {
                let modifiers = modifiers.map(|modifiers| self.unbind_binding_modifier(&modifiers));
                let name = ast_strings.intern_from(&self.program.strings, *name);
                let ty = None;
                ast::Parameter::Variadic {
                    modifiers,
                    name,
                    ty,
                }
            }
        };
        ast_tree.insert(ast_parameter, span)
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
    ) -> ast::LocalNodeId<ast::Argument> {
        let argument = tree.get(argument_id);
        let span = self.unbind_span(module, argument_id.into());
        let ast_argument = match argument {
            dir::Argument::Named { name, value } => {
                let name =
                    ast::Name::Identifier(ast_strings.intern_from(&self.program.strings, *name));
                let value =
                    self.unbind_expression(module, *value, tree, symbols, ast_tree, ast_strings);
                ast::Argument::Named { name, value }
            }
            dir::Argument::Positional { value } => {
                let value =
                    self.unbind_expression(module, *value, tree, symbols, ast_tree, ast_strings);
                ast::Argument::Positional { value }
            }
            dir::Argument::Spread { value } => {
                let value =
                    self.unbind_expression(module, *value, tree, symbols, ast_tree, ast_strings);
                ast::Argument::Spread { value }
            }
            dir::Argument::Labeled { label, value } => {
                let label = ast_strings.intern_from(&self.program.strings, *label);
                let value =
                    self.unbind_expression(module, *value, tree, symbols, ast_tree, ast_strings);
                ast::Argument::Labeled { label, value }
            }
        };
        ast_tree.insert(ast_argument, span)
    }
}
