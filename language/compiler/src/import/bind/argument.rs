use crate::Compiler;
use destack_ast as ast;
use destack_dir::{
    AccessorKind, Argument, BindingAnchor, BindingKind, BindingModifier, BindingOperator,
    LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalScopeMark, Mutability, NodeTree, NodeType,
    Parameter, StaticKey, SymbolBinding, SymbolSpace, SymbolSpaceOrder, SymbolTable, Timing,
    TypeTable, Visibility,
};
use destack_workspace::{Module, ModuleAst};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a binding modifiers into a DIR binding modifiers.
    pub(super) fn bind_binding_modifier(
        &self,
        _module: &Module,
        _ast: &ModuleAst,
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
        let accessor = modifiers.accessor.map(|accessor| match accessor {
            ast::AccessorKind::Accessor => AccessorKind::Accessor,
        });
        let timing = modifiers.timing.map(|timing| match timing {
            ast::Timing::Comptime => Timing::Comptime,
        });
        BindingModifier {
            kind,
            anchor,
            mutability,
            visibility,
            operator,
            accessor,
            timing,
        }
    }

    /// Bind a parameter into a DIR parameter.
    pub(super) fn bind_parameter(
        &self,
        module: &Module,
        ast: &ModuleAst,
        scope: (LocalScopeId, LocalScopeMark),
        symbol_space: SymbolSpace,
        ast_parameter_id: ast::LocalNodeId<ast::Parameter>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Parameter> {
        let ast_parameter = ast.tree.get(ast_parameter_id);
        let parameter_id =
            tree.reserve_from_source(NodeType::Parameter, ast_parameter_id.id, scope, parent_id);

        // prefer type space for defaults on type parameters
        let default_space_order = match symbol_space {
            SymbolSpace::Type => SymbolSpaceOrder::TypeThenValue,
            SymbolSpace::Value | SymbolSpace::TypeValue | SymbolSpace::Label => {
                SymbolSpaceOrder::ValueThenType
            }
        };

        let constraint_scope = (scope.0, LocalScopeMark::end());
        let default_mutability = self.default_binding_mutability(module);
        match ast_parameter {
            ast::Parameter::Named {
                modifiers,
                name,
                ty,
                default,
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, ast, modifiers));
                let binding_mutability = modifiers
                    .and_then(|modifiers| modifiers.mutability)
                    .unwrap_or(default_mutability);
                let name = self.program.strings.intern_from(&ast.strings, *name);
                let default = default.map(|default| {
                    self.bind_expression(
                        module,
                        ast,
                        scope,
                        default,
                        Some(parameter_id),
                        tree,
                        symbols,
                        types,
                        default_space_order,
                    )
                });
                let (symbol_id, _) = self.bind_named_item(
                    module,
                    ast,
                    symbol_space,
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
                self.apply_binding_mutability(symbols, symbol_id, binding_mutability);
                if let Some(ty) = ty {
                    let ty = self.bind_expression_to_type(
                        module,
                        ast,
                        constraint_scope,
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
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, ast, modifiers));
                let binding_mutability = modifiers
                    .and_then(|modifiers| modifiers.mutability)
                    .unwrap_or(default_mutability);
                let pattern = self.bind_pattern(
                    module,
                    ast,
                    scope,
                    None,
                    SymbolBinding::Runtime,
                    Some(binding_mutability),
                    *pattern,
                    Some(parameter_id),
                    tree,
                    symbols,
                    types,
                );
                let default = default.map(|default| {
                    self.bind_expression(
                        module,
                        ast,
                        scope,
                        default,
                        Some(parameter_id),
                        tree,
                        symbols,
                        types,
                        default_space_order,
                    )
                });
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, symbol_space, scope, None, symbols);
                let parameter = Parameter::Pattern {
                    modifiers,
                    pattern,
                    default,
                    symbol: symbol_id,
                };
                let parameter_id = tree.insert(parameter_id, parameter);
                let symbol = symbols.get_symbol_mut(symbol_id);
                symbol.primary_declaration = Some(parameter_id.into_global_any(module.id));
                self.apply_binding_mutability(symbols, symbol_id, binding_mutability);
                if let Some(ty) = ty {
                    let ty = self.bind_expression_to_type(
                        module,
                        ast,
                        constraint_scope,
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
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, ast, modifiers));
                let binding_mutability = modifiers
                    .and_then(|modifiers| modifiers.mutability)
                    .unwrap_or(default_mutability);
                let name = self.program.strings.intern_from(&ast.strings, *name);
                let (symbol_id, _) = self.bind_named_item(
                    module,
                    ast,
                    symbol_space,
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
                self.apply_binding_mutability(symbols, symbol_id, binding_mutability);
                if let Some(ty) = ty {
                    let ty = self.bind_expression_to_type(
                        module,
                        ast,
                        constraint_scope,
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
        ast: &ModuleAst,
        scope: (LocalScopeId, LocalScopeMark),
        ast_argument_id: ast::LocalNodeId<ast::Argument>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
        space_order: SymbolSpaceOrder,
    ) -> LocalNodeId<Argument> {
        let ast_argument = ast.tree.get(ast_argument_id);
        let argument_id =
            tree.reserve_from_source(NodeType::Argument, ast_argument_id.id, scope, parent_id);
        match ast_argument {
            ast::Argument::Named {
                modifiers,
                name,
                value,
                ..
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, ast, modifiers));
                let name = self
                    .program
                    .strings
                    .intern_from(&ast.strings, name.string());
                let value = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *value,
                    Some(argument_id),
                    tree,
                    symbols,
                    types,
                    space_order,
                );
                tree.insert(
                    argument_id,
                    Argument::Named {
                        modifiers,
                        name,
                        value,
                    },
                )
            }
            ast::Argument::Positional { modifiers, value } => {
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, ast, modifiers));
                let value = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *value,
                    Some(argument_id),
                    tree,
                    symbols,
                    types,
                    space_order,
                );
                tree.insert(argument_id, Argument::Positional { modifiers, value })
            }
            ast::Argument::Spread {
                modifiers,
                label,
                value,
                ..
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, ast, modifiers));
                let label =
                    label.map(|label| self.program.strings.intern_from(&ast.strings, label));
                let value = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *value,
                    Some(argument_id),
                    tree,
                    symbols,
                    types,
                    space_order,
                );
                tree.insert(
                    argument_id,
                    Argument::Spread {
                        modifiers,
                        label,
                        value,
                    },
                )
            }
            ast::Argument::Labeled {
                modifiers,
                label,
                value,
                ..
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, ast, modifiers));
                let label = self.program.strings.intern_from(&ast.strings, *label);
                let value = self.bind_expression(
                    module,
                    ast,
                    scope,
                    *value,
                    Some(argument_id),
                    tree,
                    symbols,
                    types,
                    space_order,
                );
                tree.insert(
                    argument_id,
                    Argument::Labeled {
                        modifiers,
                        label,
                        value,
                    },
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;
    use crate::{assert_node, assert_path};
    use destack_dir::{Declaration, Expression, Parameter, SymbolSpaceOrder};

    /// Check static parameter defaults use type space order.
    #[test]
    fn test_bind_static_parameter_default_space_order() {
        // setup module
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
type Wrapper<T = Foo> = T;
"#,
        );

        // bind and compile
        test.bind_module(module_id);
        test.compile();
        test.check_clean();

        // load bound tree
        let module = test.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir_base();
        let tree = dir.tree.read();

        // select the root expression
        let root_id = test.expect_root_expression(module_id);

        // assert default path space order
        assert_node!(
            tree,
            root_id,
            Expression::Declaration { declaration } => {
                assert_node!(
                    tree,
                    *declaration,
                    Declaration::Type {
                        static_parameters: Some(static_parameters),
                        ..
                    } => {
                        let parameter_id = static_parameters.first().copied().expect("expected parameter");
                        assert_node!(
                            tree,
                            parameter_id,
                            Parameter::Named { default: Some(default), .. } => {
                                assert_node!(
                                    tree,
                                    *default,
                                    Expression::UnresolvedPath {
                                        path,
                                        static_arguments: _,
                                        space_order,
                                    } => {
                                        assert_path!(test.program, path, "Foo");
                                        assert_eq!(*space_order, SymbolSpaceOrder::TypeThenValue);
                                    }
                                );
                            }
                        );
                    }
                );
            }
        );
    }
}
