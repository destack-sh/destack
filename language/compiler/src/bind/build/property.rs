use crate::Compiler;
use destack_ast as ast;
use destack_dir::{
    LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalScopeMark, Member, NodeTree, NodeType,
    Property, ScopeKind, StaticKey, SymbolSpace, SymbolTable, TypeTable,
};
use destack_workspace::{Module, ModuleAst};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a property to a DIR property.
    pub(super) fn bind_property(
        &self,
        module: &Module,
        ast: &ModuleAst,
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
                        scope,
                        value,
                        Some(property_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                let default = default.map(|default| {
                    self.bind_expression(
                        module,
                        ast,
                        scope,
                        default,
                        Some(property_id),
                        tree,
                        symbols,
                        types,
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
                        scope,
                        key,
                        Some(property_id),
                        tree,
                        symbols,
                        types,
                    )
                });
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
                        (method_scope_id, symbols.get_scope_mark(method_scope_id)),
                        body,
                        Some(property_id),
                        tree,
                        symbols,
                        types,
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
                    scope,
                    *value,
                    Some(property_id),
                    tree,
                    symbols,
                    types,
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
        }
    }

    /// Bind a member to a DIR member.
    pub(super) fn bind_member(
        &self,
        module: &Module,
        ast: &ModuleAst,
        scope: (LocalScopeId, LocalScopeMark),
        ast_member_id: ast::LocalNodeId<ast::Member>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Member> {
        let ast_member = ast.tree.get(ast_member_id);
        match ast_member {
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
                        scope,
                        key,
                        Some(member_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                let value = value.map(|value| {
                    self.bind_expression(
                        module,
                        ast,
                        scope,
                        value,
                        Some(member_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                let default = default.map(|default| {
                    self.bind_expression(
                        module,
                        ast,
                        scope,
                        default,
                        Some(member_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);
                tree.insert(
                    member_id,
                    Member::Field {
                        modifiers,
                        key,
                        value,
                        default,
                        symbol: symbol_id,
                    },
                )
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
                        scope,
                        key,
                        Some(member_id),
                        tree,
                        symbols,
                        types,
                    )
                });
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
                        (method_scope_id, symbols.get_scope_mark(method_scope_id)),
                        body,
                        Some(member_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                tree.insert(
                    member_id,
                    Member::Method {
                        modifiers,
                        key,
                        signature,
                        body,
                        symbol: symbol_id,
                    },
                )
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
                    scope,
                    *value,
                    Some(member_id),
                    tree,
                    symbols,
                    types,
                );
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);
                tree.insert(
                    member_id,
                    Member::Embed {
                        modifiers,
                        value,
                        symbol: symbol_id,
                    },
                )
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
                    scope,
                    *body,
                    Some(member_id),
                    tree,
                    symbols,
                    types,
                );
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);
                tree.insert(
                    member_id,
                    Member::StaticBlock {
                        modifiers,
                        body,
                        symbol: symbol_id,
                    },
                )
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
