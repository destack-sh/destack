use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{
    Argument, BindingAnchor, BindingKind, BindingModifier, BindingOperator, Expression,
    LocalNodeId, LocalScopeId, LocalScopeMark, Module, Mutability, NodeTree, Parameter, Path,
    SymbolKey, SymbolSpace, SymbolTable, TypeTable, Visibility,
};
use smallvec::smallvec;

#[allow(clippy::too_many_arguments)]
impl Compiler {
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
        let anchor = modifiers.anchor.map(|anchor| match anchor {
            ast::BindingAnchor::Static => BindingAnchor::Static,
            ast::BindingAnchor::Instance => BindingAnchor::Instance,
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
            anchor,
            mutability,
            visibility,
            operator,
        }
    }

    /// Bind a parameter into a DIR parameter.
    pub(super) fn bind_parameter(
        &self,
        module: &Module,
        scope: (LocalScopeId, LocalScopeMark),
        parameter_id: ast::LocalNodeId<ast::Parameter>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Parameter> {
        let parameter = module.ast.get(parameter_id);
        match parameter {
            ast::Parameter::Named {
                modifiers,
                name,
                ty,
                default,
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, modifiers));
                let name = self.program.strings.intern_from(&module.ast_strings, *name);
                let default = default.map(|default| {
                    self.bind_expression(module, scope, default, tree, symbols, types)
                });
                let (symbol_id, _) = self.bind_named_item(
                    module,
                    SymbolSpace::Value,
                    SymbolKey::Name(name),
                    scope,
                    None,
                    symbols,
                );
                let parameter = Parameter::Named {
                    modifiers,
                    name,
                    default,
                    symbol: symbol_id,
                };
                let parameter_id = tree.insert_from_source(parameter, parameter_id, scope);
                let symbol = symbols.get_symbol_mut(symbol_id);
                symbol.primary_declaration = Some(parameter_id.into_global_any(module.id));
                if let Some(ty) = ty {
                    let ty = self.bind_expression_to_type(module, scope, *ty, tree, symbols, types);
                    types.declare_type(parameter_id.into_global_any(module.id), ty);
                }
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
                let pattern =
                    self.bind_pattern(module, scope, None, *pattern, tree, symbols, types);
                let default = default.map(|default| {
                    self.bind_expression(module, scope, default, tree, symbols, types)
                });
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, SymbolSpace::Value, scope, None, symbols);
                let parameter = Parameter::Pattern {
                    modifiers,
                    pattern,
                    default,
                    symbol: symbol_id,
                };
                let parameter_id = tree.insert_from_source(parameter, parameter_id, scope);
                let symbol = symbols.get_symbol_mut(symbol_id);
                symbol.primary_declaration = Some(parameter_id.into_global_any(module.id));
                if let Some(ty) = ty {
                    let ty = self.bind_expression_to_type(module, scope, *ty, tree, symbols, types);
                    types.declare_type(parameter_id.into_global_any(module.id), ty);
                }
                parameter_id
            }
            ast::Parameter::Variadic {
                modifiers,
                name,
                ty,
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, modifiers));
                let name = self.program.strings.intern_from(&module.ast_strings, *name);
                let (symbol_id, _) = self.bind_named_item(
                    module,
                    SymbolSpace::Value,
                    SymbolKey::Name(name),
                    scope,
                    None,
                    symbols,
                );
                let parameter = Parameter::Variadic {
                    modifiers,
                    name,
                    symbol: symbol_id,
                };
                let parameter_id = tree.insert_from_source(parameter, parameter_id, scope);
                let symbol = symbols.get_symbol_mut(symbol_id);
                symbol.primary_declaration = Some(parameter_id.into_global_any(module.id));
                if let Some(ty) = ty {
                    let ty = self.bind_expression_to_type(module, scope, *ty, tree, symbols, types);
                    types.declare_type(parameter_id.into_global_any(module.id), ty);
                }
                parameter_id
            }
        }
    }

    /// Bind an argument into a DIR argument.
    pub(super) fn bind_argument(
        &self,
        module: &Module,
        scope: (LocalScopeId, LocalScopeMark),
        argument_id: ast::LocalNodeId<ast::Argument>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Argument> {
        let argument = module.ast.get(argument_id);
        match argument {
            ast::Argument::Named {
                modifiers,
                name,
                value,
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, modifiers));
                let name = self
                    .program
                    .strings
                    .intern_from(&module.ast_strings, name.string());
                let value = self.bind_expression(module, scope, *value, tree, symbols, types);
                tree.insert_from_source(
                    Argument::UnresolvedNamed {
                        modifiers,
                        name,
                        value,
                    },
                    argument_id,
                    scope,
                )
            }
            ast::Argument::Shorthand { modifiers, name } => {
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, modifiers));
                let name = self.program.strings.intern_from(&module.ast_strings, *name);
                let path = Path {
                    segments: smallvec![name],
                };
                let value = tree.insert_from_source(
                    Expression::UnresolvedAbsolutePath {
                        path,
                        static_arguments: None,
                    },
                    argument_id,
                    scope,
                );
                tree.insert_from_source(
                    Argument::UnresolvedNamed {
                        modifiers,
                        name,
                        value,
                    },
                    argument_id,
                    scope,
                )
            }
            ast::Argument::Positional { modifiers, value } => {
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, modifiers));
                let value = self.bind_expression(module, scope, *value, tree, symbols, types);
                tree.insert_from_source(
                    Argument::UnresolvedPositional { modifiers, value },
                    argument_id,
                    scope,
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
                    name.map(|name| self.program.strings.intern_from(&module.ast_strings, name));
                let value = self.bind_expression(module, scope, *value, tree, symbols, types);
                tree.insert_from_source(
                    Argument::UnresolvedSpread {
                        modifiers,
                        name,
                        value,
                    },
                    argument_id,
                    scope,
                )
            }
        }
    }
}
