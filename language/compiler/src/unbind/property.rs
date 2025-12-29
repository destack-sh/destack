use destack_ast::{self as ast};
use destack_base::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use super::UnbindContext;
use crate::Compiler;

impl Compiler {
    /// Unbind a DIR property to an AST property.
    pub(super) fn unbind_property(
        &self,
        module: &Module,
        property_id: dir::LocalNodeId<dir::Property>,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::Property> {
        let property = tree.get(property_id);
        let span = self.unbind_span(module, property_id.into());
        let ast_property = match property {
            dir::Property::Field {
                modifiers,
                key,
                value,
                default,
                ..
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.unbind_binding_modifier(context, &modifiers));
                let key = key.as_ref().map(|key| {
                    self.unbind_key(module, key, tree, symbols, ast_tree, ast_strings, context)
                });
                let value = value.map(|value| {
                    self.unbind_expression(
                        module,
                        value,
                        tree,
                        symbols,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });
                let default = default.map(|default| {
                    self.unbind_expression(
                        module,
                        default,
                        tree,
                        symbols,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });
                ast::Property::Field {
                    modifiers,
                    key,
                    value,
                    default,
                }
            }
            dir::Property::Method {
                modifiers,
                key,
                signature,
                body,
                ..
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.unbind_binding_modifier(context, &modifiers));
                let key = key.as_ref().map(|key| {
                    self.unbind_key(module, key, tree, symbols, ast_tree, ast_strings, context)
                });
                let signature = self.unbind_function_signature(
                    module,
                    signature,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let body = body.map(|body| {
                    self.unbind_expression(
                        module,
                        body,
                        tree,
                        symbols,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });
                ast::Property::Method {
                    modifiers,
                    key,
                    signature,
                    body,
                }
            }
            dir::Property::Spread {
                modifiers, value, ..
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.unbind_binding_modifier(context, &modifiers));
                let value = self.unbind_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::Property::Spread { modifiers, value }
            }
        };
        let ast_property_id = ast_tree.insert(ast_property, span);
        context.map(property_id.into_any(), ast_property_id.into_any());
        ast_property_id
    }

    /// Unbind a DIR member to an AST member.
    pub(super) fn unbind_member(
        &self,
        module: &Module,
        member_id: dir::LocalNodeId<dir::Member>,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::Member> {
        let member = tree.get(member_id);
        let span = self.unbind_span(module, member_id.into());
        let ast_member = match member {
            dir::Member::Field {
                modifiers,
                key,
                value,
                default,
                ..
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.unbind_binding_modifier(context, &modifiers));
                let key = key.as_ref().map(|key| {
                    self.unbind_key(module, key, tree, symbols, ast_tree, ast_strings, context)
                });
                let value = value.map(|value| {
                    self.unbind_expression(
                        module,
                        value,
                        tree,
                        symbols,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });
                let default = default.map(|default| {
                    self.unbind_expression(
                        module,
                        default,
                        tree,
                        symbols,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });
                ast::Member::Field {
                    modifiers,
                    key,
                    value,
                    default,
                }
            }
            dir::Member::Method {
                modifiers,
                key,
                signature,
                body,
                ..
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.unbind_binding_modifier(context, &modifiers));
                let key = key.as_ref().map(|key| {
                    self.unbind_key(module, key, tree, symbols, ast_tree, ast_strings, context)
                });
                let signature = self.unbind_function_signature(
                    module,
                    signature,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let body = body.map(|body| {
                    self.unbind_expression(
                        module,
                        body,
                        tree,
                        symbols,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });
                ast::Member::Method {
                    modifiers,
                    key,
                    signature,
                    body,
                }
            }
            dir::Member::Embed {
                modifiers, value, ..
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.unbind_binding_modifier(context, &modifiers));
                let value = self.unbind_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::Member::Embed { modifiers, value }
            }
            dir::Member::StaticBlock {
                modifiers, body, ..
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.unbind_binding_modifier(context, &modifiers));
                let body = self.unbind_expression(
                    module,
                    *body,
                    tree,
                    symbols,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::Member::StaticBlock { modifiers, body }
            }
        };
        let ast_member_id = ast_tree.insert(ast_member, span);
        context.map(member_id.into_any(), ast_member_id.into_any());
        ast_member_id
    }
}
