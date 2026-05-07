use destack_artifact::Ast;
use destack_ast::{self as ast, StringId};
use destack_dir::{
    DeclarationForm, LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalScopeMark, Member,
    ModuleBinding, Mutability, NodeType, Property, ScopeKind, StaticKey, SymbolBinding, SymbolKind,
    SymbolSpace, SymbolTable, Tree, Type, TypeTable, UnevaluatedType, Visibility,
};
use destack_workspace::Module;

use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Return whether an AST declaration is ambient.
    pub(super) fn bind_ambientness(&self, is_ambient: bool) -> bool {
        is_ambient
    }

    /// Inject private visibility for private keys when no explicit visibility exists.
    fn bind_member_visibility(
        &self,
        visibility: Option<ast::Visibility>,
        key: Option<&ast::Key>,
    ) -> Option<Visibility> {
        let visibility = visibility.map(|visibility| self.bind_visibility(visibility));

        if visibility.is_some() || !matches!(key, Some(ast::Key::Private(_))) {
            return visibility;
        }

        Some(Visibility::Private)
    }

    /// Bind an AST mutability into a DIR mutability.
    fn bind_member_mutability(&self, mutability: Option<ast::Mutability>) -> Option<Mutability> {
        mutability.map(|mutability| self.bind_mutability(mutability))
    }

    /// Insert a declared type entry for one node when present.
    fn bind_declared_type_for_node(
        &self,
        module: &Module,
        node_id: LocalNodeIdAny,
        declared_type: Option<LocalNodeId<destack_dir::TypeExpression>>,
        types: &mut TypeTable,
    ) {
        let Some(declared_type) = declared_type else {
            return;
        };

        let declared_type_id = types.insert_type_from(
            Type::Unevaluated(UnevaluatedType {
                expression: declared_type,
            }),
            declared_type,
        );
        types.set_declared_type(node_id.into_global(module.id), declared_type_id);
    }

    /// Bind a property to a DIR property.
    #[allow(clippy::too_many_arguments)]
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
        tree: &mut Tree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Property> {
        let ast_property = ast.tree.get(ast_property_id);
        let property_id =
            tree.reserve_from_source(NodeType::Property, ast_property_id.id, scope, parent_id);

        match ast_property {
            ast::Property::Field {
                key,
                value,
                is_shorthand,
            } => {
                let key = self.bind_key(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    *key,
                    Some(property_id.into()),
                    tree,
                    symbols,
                    types,
                );
                let value = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    *value,
                    Some(property_id.into()),
                    tree,
                    symbols,
                    types,
                    SymbolSpace::Value,
                );
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);
                let property_id = tree.insert(
                    property_id,
                    Property::Field {
                        key,
                        value,
                        is_shorthand: *is_shorthand,
                        symbol: symbol_id,
                    },
                );
                symbols.get_symbol_mut(symbol_id).declare(property_id);
                property_id
            }
            ast::Property::Method {
                key: Some(key),
                signature,
                body: Some(body),
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
                let key = self.bind_key(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    *key,
                    Some(property_id.into()),
                    tree,
                    symbols,
                    types,
                );

                // bind the implicit this local for method bodies
                let this_name = StringId::for_text("this");
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
                    Some(property_id.into()),
                    tree,
                    symbols,
                    types,
                );
                let body = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    (method_scope_id, symbols.get_scope_mark(method_scope_id)),
                    *body,
                    Some(property_id.into()),
                    tree,
                    symbols,
                    types,
                    SymbolSpace::Value,
                );
                let property_id = tree.insert(
                    property_id,
                    Property::Method {
                        key: Some(key),
                        signature,
                        body: Some(body),
                        symbol: symbol_id,
                    },
                );
                symbols.get_symbol_mut(symbol_id).declare(property_id);
                property_id
            }
            ast::Property::Method { .. } => {
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);
                let property_id = tree.insert(property_id, Property::Error { symbol: symbol_id });
                symbols.get_symbol_mut(symbol_id).declare(property_id);
                property_id
            }
            ast::Property::Spread { value } => {
                let value = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    *value,
                    Some(property_id.into()),
                    tree,
                    symbols,
                    types,
                    SymbolSpace::Value,
                );
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);
                let property_id = tree.insert(
                    property_id,
                    Property::Spread {
                        value,
                        symbol: symbol_id,
                    },
                );
                symbols.get_symbol_mut(symbol_id).declare(property_id);
                property_id
            }
            ast::Property::Error => {
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);
                let property_id = tree.insert(property_id, Property::Error { symbol: symbol_id });
                symbols.get_symbol_mut(symbol_id).declare(property_id);
                property_id
            }
        }
    }

    /// Bind a member to a DIR member.
    #[allow(clippy::too_many_arguments)]
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
        tree: &mut Tree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Member> {
        let ast_member = ast.tree.get(ast_member_id);
        let member_id =
            tree.reserve_from_source(NodeType::Member, ast_member_id.id, scope, parent_id);

        match ast_member {
            ast::Member::AssociatedType {
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
            } => {
                let name = *name;

                let generic_parameters = generic_parameters
                    .iter()
                    .map(|parameter| {
                        self.bind_generic_parameter(
                            module,
                            ast,
                            namespace_scope,
                            global_augmentation_scope,
                            module_bindings,
                            scope,
                            *parameter,
                            Some(member_id.into()),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();

                let scope = (scope.0, symbols.get_scope_mark(scope.0));
                let where_clauses = where_clauses
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
                            Some(member_id.into()),
                            tree,
                            symbols,
                            types,
                        )
                    })
                    .collect();

                let constraint = constraint.map(|constraint| {
                    self.bind_type_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        constraint,
                        Some(member_id.into()),
                        tree,
                        symbols,
                        types,
                        SymbolSpace::Type,
                    )
                });
                let value = value.map(|value| {
                    self.bind_type_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        value,
                        Some(member_id.into()),
                        tree,
                        symbols,
                        types,
                        SymbolSpace::Type,
                    )
                });

                let (symbol_id, _) = symbols.insert_symbol(
                    SymbolKind::Item,
                    DeclarationForm::TypeAlias,
                    SymbolSpace::Type,
                    SymbolBinding::Runtime,
                    Some(StaticKey::Name(name)),
                    scope,
                    None,
                );
                let member_id = tree.insert(
                    member_id,
                    Member::AssociatedType {
                        name,
                        generic_parameters,
                        where_clauses,
                        constraint,
                        value,
                        visibility: visibility.map(|visibility| self.bind_visibility(visibility)),
                        is_ambient: self.bind_ambientness(*is_ambient),
                        is_abstract: *is_abstract,
                        is_override: *is_override,
                        is_static: *is_static,
                        symbol: symbol_id,
                    },
                );
                symbols.get_symbol_mut(symbol_id).declare(member_id);
                self.bind_declared_type_for_node(module, member_id.into_any(), constraint, types);
                member_id
            }
            ast::Member::AssociatedConst {
                name,
                declared_type,
                value,
                visibility,
                is_ambient,
                is_static,
            } => {
                let name = *name;
                let declared_type = declared_type.map(|declared_type| {
                    self.bind_type_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        declared_type,
                        Some(member_id.into()),
                        tree,
                        symbols,
                        types,
                        SymbolSpace::Type,
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
                        Some(member_id.into()),
                        tree,
                        symbols,
                        types,
                        SymbolSpace::Type,
                    )
                });

                let (symbol_id, _) = symbols.insert_symbol(
                    SymbolKind::Item,
                    DeclarationForm::Void,
                    SymbolSpace::Value,
                    SymbolBinding::Runtime,
                    Some(StaticKey::Name(name)),
                    scope,
                    None,
                );
                let member_id = tree.insert(
                    member_id,
                    Member::AssociatedConst {
                        name,
                        declared_type,
                        value,
                        visibility: visibility.map(|visibility| self.bind_visibility(visibility)),
                        is_ambient: self.bind_ambientness(*is_ambient),
                        is_static: *is_static,
                        symbol: symbol_id,
                    },
                );
                symbols.get_symbol_mut(symbol_id).declare(member_id);
                self.bind_declared_type_for_node(
                    module,
                    member_id.into_any(),
                    declared_type,
                    types,
                );
                member_id
            }
            ast::Member::Field {
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
                is_definite,
                is_accessor,
                is_comptime,
            } => {
                let visibility = self.bind_member_visibility(*visibility, Some(key));
                let key = self.bind_key(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    *key,
                    Some(member_id.into()),
                    tree,
                    symbols,
                    types,
                );
                let declared_type = declared_type.map(|declared_type| {
                    self.bind_type_expression(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        declared_type,
                        Some(member_id.into()),
                        tree,
                        symbols,
                        types,
                        SymbolSpace::Type,
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
                        Some(member_id.into()),
                        tree,
                        symbols,
                        types,
                        SymbolSpace::Value,
                    )
                });

                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);
                let member_id = tree.insert(
                    member_id,
                    Member::Field {
                        key,
                        declared_type,
                        default,
                        is_optional: *is_optional,
                        is_readonly: *is_readonly,
                        mutability: self.bind_member_mutability(*mutability),
                        visibility,
                        is_ambient: self.bind_ambientness(*is_ambient),
                        is_abstract: *is_abstract,
                        is_override: *is_override,
                        is_static: *is_static,
                        is_definite: *is_definite,
                        is_accessor: *is_accessor,
                        is_comptime: *is_comptime,
                        symbol: symbol_id,
                    },
                );
                symbols.get_symbol_mut(symbol_id).declare(member_id);
                self.bind_declared_type_for_node(
                    module,
                    member_id.into_any(),
                    declared_type,
                    types,
                );
                member_id
            }
            ast::Member::Method {
                key,
                signature,
                body,
                is_optional,
                visibility,
                is_ambient,
                is_abstract,
                is_override,
                is_static,
                is_accessor,
                is_comptime,
            } => {
                let visibility = self.bind_member_visibility(*visibility, key.as_ref());
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

                let key = key.map(|key| {
                    self.bind_key(
                        module,
                        ast,
                        namespace_scope,
                        global_augmentation_scope,
                        module_bindings,
                        scope,
                        key,
                        Some(member_id.into()),
                        tree,
                        symbols,
                        types,
                    )
                });

                // bind the implicit this local for method bodies
                let this_name = StringId::for_text("this");
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
                    Some(member_id.into()),
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
                        Some(member_id.into()),
                        tree,
                        symbols,
                        types,
                        SymbolSpace::Value,
                    )
                });

                let member_id = tree.insert(
                    member_id,
                    Member::Method {
                        key,
                        signature,
                        body,
                        is_optional: *is_optional,
                        visibility,
                        is_ambient: self.bind_ambientness(*is_ambient),
                        is_abstract: *is_abstract,
                        is_override: *is_override,
                        is_static: *is_static,
                        is_accessor: *is_accessor,
                        is_comptime: *is_comptime,
                        symbol: symbol_id,
                    },
                );
                symbols.get_symbol_mut(symbol_id).declare(member_id);
                member_id
            }
            ast::Member::Embed {
                value,
                visibility,
                is_ambient,
                is_static,
            } => {
                let value = self.bind_type_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    *value,
                    Some(member_id.into()),
                    tree,
                    symbols,
                    types,
                    SymbolSpace::Type,
                );
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);
                let member_id = tree.insert(
                    member_id,
                    Member::Embed {
                        value,
                        visibility: visibility.map(|visibility| self.bind_visibility(visibility)),
                        is_ambient: self.bind_ambientness(*is_ambient),
                        is_static: *is_static,
                        symbol: symbol_id,
                    },
                );
                symbols.get_symbol_mut(symbol_id).declare(member_id);
                member_id
            }
            ast::Member::StaticBlock { body } => {
                let body = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    *body,
                    Some(member_id.into()),
                    tree,
                    symbols,
                    types,
                    SymbolSpace::Value,
                );
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);
                let member_id = tree.insert(
                    member_id,
                    Member::StaticBlock {
                        body,
                        symbol: symbol_id,
                    },
                );
                symbols.get_symbol_mut(symbol_id).declare(member_id);
                member_id
            }
            ast::Member::ComptimeBlock { body } => {
                let body = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    *body,
                    Some(member_id.into()),
                    tree,
                    symbols,
                    types,
                    SymbolSpace::Value,
                );
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);
                let member_id = tree.insert(
                    member_id,
                    Member::ComptimeBlock {
                        body,
                        symbol: symbol_id,
                    },
                );
                symbols.get_symbol_mut(symbol_id).declare(member_id);
                member_id
            }
            ast::Member::Error => {
                let (symbol_id, _) =
                    self.bind_anonymous_item(module, ast, SymbolSpace::Value, scope, None, symbols);
                let member_id = tree.insert(member_id, Member::Error { symbol: symbol_id });
                symbols.get_symbol_mut(symbol_id).declare(member_id);
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
