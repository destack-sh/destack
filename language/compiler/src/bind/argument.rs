use crate::Compiler;
use destack_ast as ast;
use destack_dir::{
    Argument, BindingAnchor, BindingKind, BindingModifier, BindingOperator, LocalNodeId,
    LocalNodeIdAny, LocalScopeId, LocalScopeMark, Mutability, NodeTree, NodeType,
    Parameter, StaticKey, SymbolSpace, SymbolTable, TypeTable, Visibility,
};
use destack_workspace::Module;

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
        ast_parameter_id: ast::LocalNodeId<ast::Parameter>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Parameter> {
        let ast_parameter = module.ast.get(ast_parameter_id);
        let parameter_id =
            tree.reserve_from_source(NodeType::Parameter, ast_parameter_id, scope, parent_id);
        match ast_parameter {
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
                    self.bind_expression(
                        module,
                        scope,
                        default,
                        Some(parameter_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                let (symbol_id, _) = self.bind_named_item(
                    module,
                    SymbolSpace::Value,
                    StaticKey::Name(name),
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
                let parameter_id = tree.insert(parameter_id, parameter);
                let symbol = symbols.get_symbol_mut(symbol_id);
                symbol.primary_declaration = Some(parameter_id.into_global_any(module.id));
                if let Some(ty) = ty {
                    let ty = self.bind_expression_to_type(
                        module,
                        scope,
                        *ty,
                        Some(parameter_id.into()),
                        tree,
                        symbols,
                        types,
                    );
                    types.set_declared_type(parameter_id.into_global_any(module.id), ty);
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
                let pattern = self.bind_pattern(
                    module,
                    scope,
                    None,
                    *pattern,
                    Some(parameter_id),
                    tree,
                    symbols,
                    types,
                );
                let default = default.map(|default| {
                    self.bind_expression(
                        module,
                        scope,
                        default,
                        Some(parameter_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, SymbolSpace::Value, scope, None, symbols);
                let parameter = Parameter::Pattern {
                    modifiers,
                    pattern,
                    default,
                    symbol: symbol_id,
                };
                let parameter_id = tree.insert(parameter_id, parameter);
                let symbol = symbols.get_symbol_mut(symbol_id);
                symbol.primary_declaration = Some(parameter_id.into_global_any(module.id));
                if let Some(ty) = ty {
                    let ty = self.bind_expression_to_type(
                        module,
                        scope,
                        *ty,
                        Some(parameter_id.into()),
                        tree,
                        symbols,
                        types,
                    );
                    types.set_declared_type(parameter_id.into_global_any(module.id), ty);
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
                    StaticKey::Name(name),
                    scope,
                    None,
                    symbols,
                );
                let parameter = Parameter::Variadic {
                    modifiers,
                    name,
                    symbol: symbol_id,
                };
                let parameter_id = tree.insert(parameter_id, parameter);
                let symbol = symbols.get_symbol_mut(symbol_id);
                symbol.primary_declaration = Some(parameter_id.into_global_any(module.id));
                if let Some(ty) = ty {
                    let ty = self.bind_expression_to_type(
                        module,
                        scope,
                        *ty,
                        Some(parameter_id.into()),
                        tree,
                        symbols,
                        types,
                    );
                    types.set_declared_type(parameter_id.into_global_any(module.id), ty);
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
        ast_argument_id: ast::LocalNodeId<ast::Argument>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Argument> {
        let ast_argument = module.ast.get(ast_argument_id);
        let argument_id =
            tree.reserve_from_source(NodeType::Argument, ast_argument_id, scope, parent_id);
        match ast_argument {
            ast::Argument::Named { name, value } => {
                let name = self
                    .program
                    .strings
                    .intern_from(&module.ast_strings, name.string());
                let value = self.bind_expression(
                    module,
                    scope,
                    *value,
                    Some(argument_id),
                    tree,
                    symbols,
                    types,
                );
                tree.insert(argument_id, Argument::Named { name, value })
            }
            ast::Argument::Positional { value } => {
                let value = self.bind_expression(
                    module,
                    scope,
                    *value,
                    Some(argument_id),
                    tree,
                    symbols,
                    types,
                );
                tree.insert(argument_id, Argument::Positional { value })
            }
            ast::Argument::Spread { value } => {
                let value = self.bind_expression(
                    module,
                    scope,
                    *value,
                    Some(argument_id),
                    tree,
                    symbols,
                    types,
                );
                tree.insert(argument_id, Argument::Spread { value })
            }
        }
    }
}
