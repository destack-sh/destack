use crate::Compiler;
use destack_ast as ast;
use destack_dir::{
    BindingModifier, DynamicKey, LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalScopeMark, Member,
    ModuleBinding, NodeTree, NodeType, Property, ScopeKind, StaticKey, SymbolBinding, SymbolKind,
    SymbolSpace, SymbolSpaceOrder, SymbolTable, SymbolType, TypeTable, Visibility,
};
use destack_workspace::{Ast, Module};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Apply private visibility to members with private keys.
    pub(super) fn apply_private_member_visibility(
        &self,
        modifiers: Option<BindingModifier>,
        key: Option<DynamicKey>,
    ) -> Option<BindingModifier> {
        // keep modifiers unchanged when the key is not private
        if !matches!(key, Some(DynamicKey::Private(_))) {
            return modifiers;
        }

        // ensure we have a modifier to update
        let mut modifiers = modifiers.unwrap_or(BindingModifier {
            kind: None,
            declaration: None,
            abstraction: None,
            variance: None,
            anchor: None,
            mutability: None,
            visibility: None,
            operator: None,
            accessor: None,
            timing: None,
        });

        // inject private visibility when not already specified
        if modifiers.visibility.is_none() {
            modifiers.visibility = Some(Visibility::Private);
        }

        Some(modifiers)
    }

    /// Bind a property to a DIR property.
    pub(super) fn bind_property(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_augmentation_scope: LocalScopeId,
        module_bindings: &mut Vec<ModuleBinding>,
        scope: (LocalScopeId, LocalScopeMark),
        ast_property_id: ast::LocalNodeId<ast::Property>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Property> {
        let ast_property = ast.tree.get(ast_property_id);
        match ast_property {
            ast::Property::Field {
                modifiers,
                key,
                value,
                default,
            } => {
                let property_id = tree.reserve_from_source(
                    NodeType::Property,
                    ast_property_id.id,
                    scope,
                    parent_id,
                );
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, ast, modifiers));
                let key = key.map(|key| {
                    self.bind_key(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        key,
                        Some(property_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                let value = value.map(|value| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        value,
                        Some(property_id),
                        tree,
                        symbols,
                        types,
                        SymbolSpaceOrder::ValueThenType,
                    )
                });
                let default = default.map(|default| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        default,
                        Some(property_id),
                        tree,
                        symbols,
                        types,
                        SymbolSpaceOrder::ValueThenType,
                    )
                });
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);
                let property = Property::Field {
                    modifiers,
                    key,
                    value,
                    default,
                    symbol: symbol_id,
                };
                tree.insert(property_id, property)
            }
            ast::Property::Method {
                modifiers,
                key,
                signature,
                body,
            } => {
                let (symbol_id, method_scope_id) = self.bind_anonymous_item_with_scope(
                    module,
                    ast,
                    ScopeKind::Namespace,
                    scope,
                    None,
                    symbols,
                );
                let property_id = tree.reserve_from_source(
                    NodeType::Property,
                    ast_property_id.id,
                    (method_scope_id, LocalScopeMark::end()),
                    parent_id,
                );
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, ast, modifiers));
                let key = key.map(|key| {
                    self.bind_key(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        key,
                        Some(property_id),
                        tree,
                        symbols,
                        types,
                    )
                });

                // bind the implicit this local for method bodies
                let this_name = self.program.strings.intern("this");
                let method_scope = (method_scope_id, symbols.get_scope_mark(method_scope_id));
                self.bind_named_local(
                    module,
                    ast,
                    SymbolSpace::Value,
                    StaticKey::Name(this_name),
                    method_scope,
                    symbols,
                );

                let method_scope = (method_scope_id, symbols.get_scope_mark(method_scope_id));
                let signature = self.bind_function_signature(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    method_scope,
                    signature,
                    Some(property_id),
                    tree,
                    symbols,
                    types,
                );
                let body = body.map(|body| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        (method_scope_id, symbols.get_scope_mark(method_scope_id)),
                        body,
                        Some(property_id),
                        tree,
                        symbols,
                        types,
                        SymbolSpaceOrder::ValueThenType,
                    )
                });
                let property = Property::Method {
                    modifiers,
                    key,
                    signature,
                    body,
                    symbol: symbol_id,
                };
                tree.insert(property_id, property)
            }
            ast::Property::Spread {
                modifiers, value, ..
            } => {
                let property_id = tree.reserve_from_source(
                    NodeType::Property,
                    ast_property_id.id,
                    scope,
                    parent_id,
                );
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, ast, modifiers));
                let value = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    *value,
                    Some(property_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpaceOrder::ValueThenType,
                );
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);
                let property = Property::Spread {
                    modifiers,
                    value,
                    symbol: symbol_id,
                };
                tree.insert(property_id, property)
            }
            ast::Property::Error => {
                let property_id = tree.reserve_from_source(
                    NodeType::Property,
                    ast_property_id.id,
                    scope,
                    parent_id,
                );
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);
                let property = Property::Error { symbol: symbol_id };
                let property_id = tree.insert(property_id, property);
                symbols
                    .get_symbol_mut(symbol_id)
                    .declare_primary(property_id);
                property_id
            }
        }
    }

    /// Bind a member to a DIR member.
    pub(super) fn bind_member(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_augmentation_scope: LocalScopeId,
        module_bindings: &mut Vec<ModuleBinding>,
        scope: (LocalScopeId, LocalScopeMark),
        ast_member_id: ast::LocalNodeId<ast::Member>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Member> {
        let ast_member = ast.tree.get(ast_member_id);
        match ast_member {
            ast::Member::Type {
                modifiers,
                name,
                static_parameters,
                where_clauses,
                ty,
                value,
            } => {
                let member_id =
                    tree.reserve_from_source(NodeType::Member, ast_member_id.id, scope, parent_id);
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, ast, modifiers));
                let name = self.program.strings.intern_from(&ast.strings, *name);

                // bind associated type static parameters
                let static_parameters = static_parameters.as_ref().map(|parameters| {
                    parameters
                        .iter()
                        .map(|parameter| {
                            self.bind_parameter(
                                module,
                                ast,
                                namespace_scope,
                                global_augmentation_scope,
                                module_bindings,
                                scope,
                                SymbolSpace::Type,
                                *parameter,
                                Some(member_id),
                                tree,
                                symbols,
                                types,
                            )
                        })
                        .collect()
                });

                // refresh scope so type member parameters are visible
                let scope = (scope.0, symbols.get_scope_mark(scope.0));

                // bind associated type where clauses in parameter scope
                let where_clauses = where_clauses.as_ref().map(|where_clauses| {
                    where_clauses
                        .iter()
                        .map(|where_clause| {
                            self.bind_where_clause(
                                module,
                                ast,
                                namespace_scope,
                                global_augmentation_scope,
                                module_bindings,
                                scope,
                                *where_clause,
                                Some(member_id),
                                tree,
                                symbols,
                                types,
                            )
                        })
                        .collect()
                });

                let ty = ty.map(|ty| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        ty,
                        Some(member_id),
                        tree,
                        symbols,
                        types,
                        SymbolSpaceOrder::TypeThenValue,
                    )
                });
                let value = value.map(|value| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        value,
                        Some(member_id),
                        tree,
                        symbols,
                        types,
                        SymbolSpaceOrder::TypeThenValue,
                    )
                });

                // type members are named type aliases in type space
                let (symbol_id, _) = symbols.insert_symbol(
                    SymbolKind::Item,
                    SymbolType::TypeAlias,
                    SymbolSpace::Type,
                    SymbolBinding::Runtime,
                    Some(StaticKey::Name(name)),
                    scope,
                    None,
                );
                let member_id = tree.insert(
                    member_id,
                    Member::Type {
                        modifiers,
                        name,
                        static_parameters,
                        where_clauses,
                        ty,
                        value,
                        symbol: symbol_id,
                    },
                );

                // bind symbol to member node
                symbols.get_symbol_mut(symbol_id).declare_primary(member_id);

                member_id
            }
            ast::Member::ComptimeConst {
                modifiers,
                name,
                ty,
                value,
            } => {
                let member_id =
                    tree.reserve_from_source(NodeType::Member, ast_member_id.id, scope, parent_id);
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, ast, modifiers));
                let name = self.program.strings.intern_from(&ast.strings, *name);
                let ty = ty.map(|ty| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        ty,
                        Some(member_id),
                        tree,
                        symbols,
                        types,
                        SymbolSpaceOrder::TypeThenValue,
                    )
                });
                let value = value.map(|value| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        value,
                        Some(member_id),
                        tree,
                        symbols,
                        types,
                        SymbolSpaceOrder::TypeThenValue,
                    )
                });

                // associated comptime constants live in owner value space
                let (symbol_id, _) = symbols.insert_symbol(
                    SymbolKind::Item,
                    SymbolType::Void,
                    SymbolSpace::Value,
                    SymbolBinding::Runtime,
                    Some(StaticKey::Name(name)),
                    scope,
                    None,
                );
                let member_id = tree.insert(
                    member_id,
                    Member::ComptimeConst {
                        modifiers,
                        name,
                        ty,
                        value,
                        symbol: symbol_id,
                    },
                );

                // bind symbol to member node
                symbols.get_symbol_mut(symbol_id).declare_primary(member_id);

                member_id
            }
            ast::Member::Field {
                modifiers,
                key,
                value,
                default,
                ..
            } => {
                let member_id =
                    tree.reserve_from_source(NodeType::Member, ast_member_id.id, scope, parent_id);
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, ast, modifiers));
                let key = key.map(|key| {
                    self.bind_key(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        key,
                        Some(member_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                let modifiers = self.apply_private_member_visibility(modifiers, key);
                let value = value.map(|value| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        value,
                        Some(member_id),
                        tree,
                        symbols,
                        types,
                        SymbolSpaceOrder::TypeThenValue,
                    )
                });
                let default = default.map(|default| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        default,
                        Some(member_id),
                        tree,
                        symbols,
                        types,
                        SymbolSpaceOrder::ValueThenType,
                    )
                });
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);
                let member_id = tree.insert(
                    member_id,
                    Member::Field {
                        modifiers,
                        key,
                        value,
                        default,
                        symbol: symbol_id,
                    },
                );

                // bind symbol to member node
                symbols.get_symbol_mut(symbol_id).declare_primary(member_id);

                member_id
            }
            ast::Member::Method {
                modifiers,
                key,
                signature,
                body,
                ..
            } => {
                let (symbol_id, method_scope_id) = self.bind_anonymous_item_with_scope(
                    module,
                    ast,
                    ScopeKind::Namespace,
                    scope,
                    None,
                    symbols,
                );
                let member_id = tree.reserve_from_source(
                    NodeType::Member,
                    ast_member_id.id,
                    (method_scope_id, LocalScopeMark::end()),
                    parent_id,
                );
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, ast, modifiers));
                let key = key.map(|key| {
                    self.bind_key(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        key,
                        Some(member_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                let modifiers = self.apply_private_member_visibility(modifiers, key);

                // bind the implicit this local for method bodies
                let this_name = self.program.strings.intern("this");
                let method_scope = (method_scope_id, symbols.get_scope_mark(method_scope_id));
                self.bind_named_local(
                    module,
                    ast,
                    SymbolSpace::Value,
                    StaticKey::Name(this_name),
                    method_scope,
                    symbols,
                );

                let method_scope = (method_scope_id, symbols.get_scope_mark(method_scope_id));
                let signature = self.bind_function_signature(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    method_scope,
                    signature,
                    Some(member_id),
                    tree,
                    symbols,
                    types,
                );
                let body = body.map(|body| {
                    self.bind_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        (method_scope_id, symbols.get_scope_mark(method_scope_id)),
                        body,
                        Some(member_id),
                        tree,
                        symbols,
                        types,
                        SymbolSpaceOrder::ValueThenType,
                    )
                });
                let member_id = tree.insert(
                    member_id,
                    Member::Method {
                        modifiers,
                        key,
                        signature,
                        body,
                        symbol: symbol_id,
                    },
                );

                // bind symbol to member node
                symbols.get_symbol_mut(symbol_id).declare_primary(member_id);

                member_id
            }
            ast::Member::Embed {
                modifiers, value, ..
            } => {
                let member_id =
                    tree.reserve_from_source(NodeType::Member, ast_member_id.id, scope, parent_id);
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, ast, modifiers));
                let value = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    *value,
                    Some(member_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpaceOrder::TypeThenValue,
                );
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);
                let member_id = tree.insert(
                    member_id,
                    Member::Embed {
                        modifiers,
                        value,
                        symbol: symbol_id,
                    },
                );

                // bind symbol to member node
                symbols.get_symbol_mut(symbol_id).declare_primary(member_id);

                member_id
            }
            ast::Member::StaticBlock { modifiers, body } => {
                let member_id =
                    tree.reserve_from_source(NodeType::Member, ast_member_id.id, scope, parent_id);
                // modifiers are validated in the analyze validate pass
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, ast, modifiers));
                let body = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    *body,
                    Some(member_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpaceOrder::ValueThenType,
                );
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);
                let member_id = tree.insert(
                    member_id,
                    Member::StaticBlock {
                        modifiers,
                        body,
                        symbol: symbol_id,
                    },
                );

                // bind symbol to member node
                symbols.get_symbol_mut(symbol_id).declare_primary(member_id);

                member_id
            }
            ast::Member::ComptimeBlock { modifiers, body } => {
                let member_id =
                    tree.reserve_from_source(NodeType::Member, ast_member_id.id, scope, parent_id);
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, ast, modifiers));
                let body = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    *body,
                    Some(member_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpaceOrder::ValueThenType,
                );
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);
                let member_id = tree.insert(
                    member_id,
                    Member::ComptimeBlock {
                        modifiers,
                        body,
                        symbol: symbol_id,
                    },
                );

                // bind symbol to member node
                symbols.get_symbol_mut(symbol_id).declare_primary(member_id);

                member_id
            }
            ast::Member::Error => {
                let member_id =
                    tree.reserve_from_source(NodeType::Member, ast_member_id.id, scope, parent_id);
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);
                let member = Member::Error { symbol: symbol_id };
                let member_id = tree.insert(member_id, member);
                symbols.get_symbol_mut(symbol_id).declare_primary(member_id);
                member_id
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;

    #[test]
    fn test_bind_property_method_parameter_scopes() {
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.d.ds",
            r#"
declare var Factory: {
    new(type: string): any;
    new(type: number): any;
};
"#,
        );
        test.bind_module(module_id);
        test.compile();
        test.check_clean();
    }
}
