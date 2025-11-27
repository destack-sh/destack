use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{
    LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalScopeMark, Module, NodeTree, NodeType,
    Property, SymbolTable, TypeTable,
};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a property to a DIR property.
    pub(super) fn bind_property(
        &self,
        module: &Module,
        scope: (LocalScopeId, LocalScopeMark),
        ast_property_id: ast::LocalNodeId<ast::Property>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Property> {
        let ast_property = module.ast.get(ast_property_id);
        let property_id =
            tree.reserve_from_source(NodeType::Property, ast_property_id, scope, parent_id);
        let property = match ast_property {
            ast::Property::Field {
                modifiers,
                key,
                value,
                default,
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, modifiers));
                let key = key.map(|key| {
                    self.bind_key(module, scope, key, Some(property_id), tree, symbols, types)
                });
                let value = value.map(|value| {
                    self.bind_expression(
                        module,
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
                        scope,
                        default,
                        Some(property_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                Property::Field {
                    modifiers,
                    key,
                    value,
                    default,
                }
            }
            ast::Property::Method {
                modifiers,
                key,
                signature,
                body,
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, modifiers));
                let key = key.map(|key| {
                    self.bind_key(module, scope, key, Some(property_id), tree, symbols, types)
                });
                let signature = self.bind_function_signature(
                    module,
                    scope,
                    signature,
                    Some(property_id),
                    tree,
                    symbols,
                    types,
                );
                let body = body.map(|body| {
                    self.bind_expression(
                        module,
                        scope,
                        body,
                        Some(property_id),
                        tree,
                        symbols,
                        types,
                    )
                });
                Property::Method {
                    modifiers,
                    key,
                    signature,
                    body,
                }
            }
            ast::Property::Spread { modifiers, value } => {
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, modifiers));
                let value = self.bind_expression(
                    module,
                    scope,
                    *value,
                    Some(property_id),
                    tree,
                    symbols,
                    types,
                );
                Property::Spread { modifiers, value }
            }
        };
        tree.insert(property_id, property)
    }
}
