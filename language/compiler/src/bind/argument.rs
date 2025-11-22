use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{
    Argument, BindingKind, BindingModifier, BindingOperator, BindingScope, Expression, LocalNodeId,
    LocalScopeId, Module, Mutability, NodeTree, Parameter, Path, SymbolKey, SymbolSpace,
    SymbolTable, Visibility,
};
use dyst_source::smallvec;

impl<'a> Compiler<'a> {
    /// Bind a binding modifiers into a DIR binding modifiers.
    pub(super) fn bind_binding_modifier(
        &self,
        _module: &Module,
        modifiers: ast::BindingModifier,
    ) -> BindingModifier {
        let kind = modifiers.kind.map(|kind| match kind {
            ast::BindingKind::Must => BindingKind::Must,
            ast::BindingKind::Maybe => BindingKind::Maybe,
        });
        let scope = modifiers.scope.map(|scope| match scope {
            ast::BindingScope::Static => BindingScope::Static,
            ast::BindingScope::Instance => BindingScope::Instance,
        });
        let mutability = modifiers.mutability.map(|mutability| match mutability {
            ast::Mutability::Immutable => Mutability::Immutable,
            ast::Mutability::Mutable => Mutability::Mutable,
        });
        let visibility = modifiers.visibility.map(|visibility| match visibility {
            ast::Visibility::Public => Visibility::Public,
            ast::Visibility::Protected => Visibility::Protected,
            ast::Visibility::Private => Visibility::Private,
        });
        let operator = modifiers.operator.map(|operator| match operator {
            ast::BindingOperator::AsConst => BindingOperator::AsConst,
        });
        BindingModifier {
            kind,
            scope,
            mutability,
            visibility,
            operator,
        }
    }

    /// Bind a parameter into a DIR parameter.
    pub(super) fn bind_parameter(
        &self,
        module: &Module,
        scope_id: LocalScopeId,
        parameter_id: ast::LocalNodeId<ast::Parameter>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
    ) -> LocalNodeId<Parameter> {
        let parameter = module.get(parameter_id);
        match parameter {
            ast::Parameter::Named {
                modifiers,
                name,
                ty,
                default,
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, modifiers));
                let name = self.session.strings.intern_from(&module.ast_strings, *name);
                let ty =
                    ty.map(|ty| self.bind_expression_to_type(module, scope_id, ty, tree, symbols));
                let default = default
                    .map(|default| self.bind_expression(module, scope_id, default, tree, symbols));
                let symbol_id = symbols.insert_symbol(
                    SymbolSpace::Value,
                    Some(SymbolKey::Name(name)),
                    scope_id,
                );
                let parameter = Parameter::Named {
                    modifiers,
                    name,
                    ty,
                    default,
                    symbol: symbol_id,
                };
                let parameter_id = tree.insert_from_source(parameter, parameter_id, scope_id);
                symbols.set_primary_declaration(symbol_id, parameter_id);
                parameter_id
            }
            ast::Parameter::Pattern {
                modifiers,
                pattern,
                ty,
                default,
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, modifiers));
                let pattern = self.bind_pattern(module, scope_id, *pattern, tree, symbols);
                let ty =
                    ty.map(|ty| self.bind_expression_to_type(module, scope_id, ty, tree, symbols));
                let default = default
                    .map(|default| self.bind_expression(module, scope_id, default, tree, symbols));
                let symbol_id = symbols.insert_symbol(SymbolSpace::Value, None, scope_id);
                let parameter = Parameter::Pattern {
                    modifiers,
                    pattern,
                    ty,
                    default,
                    symbol: symbol_id,
                };
                let parameter_id = tree.insert_from_source(parameter, parameter_id, scope_id);
                symbols.set_primary_declaration(symbol_id, parameter_id);
                parameter_id
            }
            ast::Parameter::Variadic {
                modifiers,
                name,
                ty,
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, modifiers));
                let name = self.session.strings.intern_from(&module.ast_strings, *name);
                let ty =
                    ty.map(|ty| self.bind_expression_to_type(module, scope_id, ty, tree, symbols));
                let symbol_id = symbols.insert_symbol(
                    SymbolSpace::Value,
                    Some(SymbolKey::Name(name)),
                    scope_id,
                );
                let parameter = Parameter::Variadic {
                    modifiers,
                    name,
                    ty,
                    symbol: symbol_id,
                };
                let parameter_id = tree.insert_from_source(parameter, parameter_id, scope_id);
                symbols.set_primary_declaration(symbol_id, parameter_id);
                parameter_id
            }
        }
    }

    /// Bind an argument into a DIR argument.
    pub(super) fn bind_argument(
        &self,
        module: &Module,
        scope_id: LocalScopeId,
        argument_id: ast::LocalNodeId<ast::Argument>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
    ) -> LocalNodeId<Argument> {
        let argument = module.get(argument_id);
        match argument {
            ast::Argument::Named {
                modifiers,
                name,
                value,
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, modifiers));
                let name = self
                    .session
                    .strings
                    .intern_from(&module.ast_strings, name.string());
                let value = self.bind_expression(module, scope_id, *value, tree, symbols);
                tree.insert_from_source(
                    Argument::UnresolvedNamed {
                        modifiers,
                        name,
                        value,
                    },
                    argument_id,
                    scope_id,
                )
            }
            ast::Argument::Shorthand { modifiers, name } => {
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, modifiers));
                let name = self.session.strings.intern_from(&module.ast_strings, *name);
                let path = Path {
                    segments: smallvec![name],
                };
                let value = tree.insert_from_source(
                    Expression::UnresolvedAbsolutePath {
                        path,
                        static_arguments: None,
                    },
                    argument_id,
                    scope_id,
                );
                tree.insert_from_source(
                    Argument::UnresolvedNamed {
                        modifiers,
                        name,
                        value,
                    },
                    argument_id,
                    scope_id,
                )
            }
            ast::Argument::Positional { modifiers, value } => {
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, modifiers));
                let value = self.bind_expression(module, scope_id, *value, tree, symbols);
                tree.insert_from_source(
                    Argument::UnresolvedPositional { modifiers, value },
                    argument_id,
                    scope_id,
                )
            }
            ast::Argument::Spread {
                modifiers,
                name,
                value,
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, modifiers));
                let name =
                    name.map(|name| self.session.strings.intern_from(&module.ast_strings, name));
                let value = self.bind_expression(module, scope_id, *value, tree, symbols);
                tree.insert_from_source(
                    Argument::UnresolvedSpread {
                        modifiers,
                        name,
                        value,
                    },
                    argument_id,
                    scope_id,
                )
            }
        }
    }
}
