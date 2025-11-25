use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{LocalNodeId, LocalScopeId, Module, NodeTree, Property, SymbolTable, TypeTable};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a property to a DIR property.
    pub(super) fn bind_property(
        &self,
        module: &Module,
        scope_id: LocalScopeId,
        property_id: ast::LocalNodeId<ast::Property>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
    ) -> LocalNodeId<Property> {
        let property = module.get(property_id);
        let property = match property {
            ast::Property::Field {
                modifiers,
                key,
                value,
                default,
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.bind_binding_modifier(module, modifiers));
                let key = key.map(|key| self.bind_key(module, scope_id, key, tree, symbols, types));
                let value = value.map(|value| {
                    self.bind_expression(module, scope_id, value, tree, symbols, types)
                });
                let default = default.map(|default| {
                    self.bind_expression(module, scope_id, default, tree, symbols, types)
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
                let key = key.map(|key| self.bind_key(module, scope_id, key, tree, symbols, types));
                let signature =
                    self.bind_function_signature(module, scope_id, signature, tree, symbols, types);
                let body = body
                    .map(|body| self.bind_expression(module, scope_id, body, tree, symbols, types));
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
                let value = self.bind_expression(module, scope_id, *value, tree, symbols, types);
                Property::Spread { modifiers, value }
            }
        };
        tree.insert_from_source(property, property_id, scope_id)
    }
}
