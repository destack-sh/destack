use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{Module, NodeId, NodeTree, Property, ScopeId};

impl<'a> Compiler<'a> {
    /// Lower a property to a DIR property.
    pub(super) fn lower_property(
        &self,
        module: &Module,
        scope_id: ScopeId,
        property_id: ast::NodeId<ast::Property>,
        tree: &mut NodeTree,
    ) -> NodeId<Property> {
        let property = module.get(property_id);
        let property = match property {
            ast::Property::Field {
                modifiers,
                key,
                value,
                default,
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.lower_binding_modifier(module, modifiers));
                let key = key.map(|key| self.lower_key(module, scope_id, key, tree));
                let value = value.map(|value| self.lower_expression(module, scope_id, value, tree));
                let default =
                    default.map(|default| self.lower_expression(module, scope_id, default, tree));
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
                    modifiers.map(|modifiers| self.lower_binding_modifier(module, modifiers));
                let key = key.map(|key| self.lower_key(module, scope_id, key, tree));
                let signature = self.lower_function_signature(module, scope_id, signature, tree);
                let body = body.map(|body| self.lower_expression(module, scope_id, body, tree));
                Property::Method {
                    modifiers,
                    key,
                    signature,
                    body,
                }
            }
            ast::Property::Spread { modifiers, value } => {
                let modifiers =
                    modifiers.map(|modifiers| self.lower_binding_modifier(module, modifiers));
                let value = self.lower_expression(module, scope_id, *value, tree);
                Property::Spread { modifiers, value }
            }
        };
        tree.insert_from_source(property, module.id, property_id)
    }
}
