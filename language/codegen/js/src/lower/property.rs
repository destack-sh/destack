use crate::{
    BindingAnchor, BindingKind, BindingModifier, BindingOperator, Expression, LocalNodeId, Property,
};
use destack_dir::{self as dir, NodeTree, SymbolTable, TypeTable};
use destack_workspace::Module;

use crate::{TranspileResult, TranspileResultExt, Transpiler, TranspilerUnit};

impl Transpiler {
    /// Lower a binding modifier from DIR into JS AST.
    pub fn lower_binding_modifier(
        &self,
        _module: &Module,
        modifier: dir::BindingModifier,
        _unit: &mut TranspilerUnit,
    ) -> TranspileResult<BindingModifier> {
        let kind = modifier.kind.map(|kind| match kind {
            dir::BindingKind::Must => BindingKind::Must,
            dir::BindingKind::Maybe => BindingKind::Maybe,
        });
        let anchor = modifier.anchor.map(|anchor| match anchor {
            dir::BindingAnchor::Static => BindingAnchor::Static,
            dir::BindingAnchor::Instance => BindingAnchor::Instance,
        });
        let mutability = modifier
            .mutability
            .map(|mutability| self.lower_mutability(mutability));
        let visibility = modifier
            .visibility
            .map(|visibility| self.lower_visibility(visibility));
        let operator = modifier.operator.map(|operator| match operator {
            dir::BindingOperator::AsConst => BindingOperator::AsConst,
        });
        Ok(BindingModifier {
            kind,
            anchor,
            mutability,
            visibility,
            operator,
        })
    }

    /// Lower a property from DIR into JS AST.
    pub fn lower_property(
        &self,
        module: &Module,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        property_id: dir::LocalNodeId<dir::Property>,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<LocalNodeId<Property>> {
        let property = tree.get(property_id);
        let property = match property {
            dir::Property::Field {
                modifiers,
                key,
                value,
                default,
                symbol: _,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifier(module, modifiers, unit))
                    .transpose()?;
                let key = key
                    .as_ref()
                    .map(|key| self.lower_key(module, tree, symbols, types, *key, unit))
                    .transpose()?;
                let value = value
                    .as_ref()
                    .map(|value| {
                        self.lower_expression(module, tree, symbols, types, *value, unit)
                            .expect_node::<Expression>(value.into_global_any(module.id), unit)
                    })
                    .transpose()?;
                let default = default
                    .as_ref()
                    .map(|default| {
                        self.lower_expression(module, tree, symbols, types, *default, unit)
                            .expect_node::<Expression>(default.into_global_any(module.id), unit)
                    })
                    .transpose()?;
                Property::Field {
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
                symbol: _,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifier(module, modifiers, unit))
                    .transpose()?;
                let key = key
                    .as_ref()
                    .map(|key| self.lower_key(module, tree, symbols, types, *key, unit))
                    .transpose()?;
                let signature =
                    self.lower_function_signature(module, tree, symbols, types, signature, unit)?;
                let body = body
                    .as_ref()
                    .map(|body_id| {
                        self.lower_expression(module, tree, symbols, types, *body_id, unit)
                            .expect_node::<Expression>(body_id.into_global_any(module.id), unit)
                    })
                    .transpose()?;
                Property::Method {
                    modifiers,
                    key,
                    signature,
                    body,
                }
            }
            dir::Property::Spread {
                modifiers,
                value,
                symbol: _,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifier(module, modifiers, unit))
                    .transpose()?;
                let value = self
                    .lower_expression(module, tree, symbols, types, *value, unit)
                    .expect_node::<Expression>(value.into_global_any(module.id), unit)?;
                Property::Spread { modifiers, value }
            }
        };
        let property_id = unit
            .ast
            .insert_from_source(property, module.id, property_id);
        Ok(property_id)
    }
}
