use dyst_dir::{self as dir, Module};
use dyst_javascript_ast::{
    BindingKind, BindingModifier, BindingOperator, BindingScope, Expression, NodeId, Property,
};

use crate::{TranspileResult, TranspileResultExt, Transpiler, TranspilerUnit};

impl<'a> Transpiler<'a> {
    /// Transpile a binding modifier from DIR into JS AST.
    pub fn transpile_binding_modifier(
        &self,
        _module: &'a Module,
        modifier: dir::BindingModifier,
        _unit: &mut TranspilerUnit,
    ) -> TranspileResult<BindingModifier> {
        let kind = modifier.kind.map(|kind| match kind {
            dir::BindingKind::Must => BindingKind::Must,
            dir::BindingKind::Maybe => BindingKind::Maybe,
        });
        let scope = modifier.scope.map(|scope| match scope {
            dir::BindingScope::Static => BindingScope::Static,
            dir::BindingScope::Instance => BindingScope::Instance,
        });
        let mutability = modifier
            .mutability
            .map(|mutability| self.transpile_mutability(mutability));
        let visibility = modifier
            .visibility
            .map(|visibility| self.transpile_visibility(visibility));
        let operator = modifier.operator.map(|operator| match operator {
            dir::BindingOperator::AsConst => BindingOperator::AsConst,
        });
        Ok(BindingModifier {
            kind,
            scope,
            mutability,
            visibility,
            operator,
        })
    }

    /// Transpile a property from DIR into JS AST.
    pub fn transpile_property(
        &self,
        module: &'a Module,
        property_id: dir::NodeId<dir::Property>,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<NodeId<Property>> {
        let property = self.session.tree.get(property_id);
        let property = match property.as_ref() {
            dir::Property::Field {
                modifiers,
                key,
                value,
                default,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.transpile_binding_modifier(module, modifiers, unit))
                    .transpose()?;
                let key = key
                    .as_ref()
                    .map(|key| self.transpile_key(module, *key, unit))
                    .transpose()?;
                let value = value
                    .as_ref()
                    .map(|value| {
                        self.transpile_expression(module, *value, unit)
                            .expect_node::<Expression>(value.into_any(), unit)
                    })
                    .transpose()?;
                let default = default
                    .as_ref()
                    .map(|default| {
                        self.transpile_expression(module, *default, unit)
                            .expect_node::<Expression>(default.into_any(), unit)
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
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.transpile_binding_modifier(module, modifiers, unit))
                    .transpose()?;
                let key = key
                    .as_ref()
                    .map(|key| self.transpile_key(module, *key, unit))
                    .transpose()?;
                let signature = self.transpile_function_signature(module, signature, unit)?;
                let body = body
                    .as_ref()
                    .map(|body_id| {
                        self.transpile_expression(module, *body_id, unit)
                            .expect_node::<Expression>(body_id.into_any(), unit)
                    })
                    .transpose()?;
                Property::Method {
                    modifiers,
                    key,
                    signature,
                    body,
                }
            }
            dir::Property::Spread { modifiers, value } => {
                let modifiers = modifiers
                    .map(|modifiers| self.transpile_binding_modifier(module, modifiers, unit))
                    .transpose()?;
                let value = self
                    .transpile_expression(module, *value, unit)
                    .expect_node::<Expression>(value.into_any(), unit)?;
                Property::Spread { modifiers, value }
            }
        };
        let property_id = unit.ast.insert_from_source(property, module.id, property_id);
        Ok(property_id)
    }
}
