use dyst_dir::{self as dir, Module};
use dyst_javascript_ast::{Argument, NodeId, Property};

use crate::{TranspileError, TranspileResult, Transpiler, TranspilerUnit};

impl<'a> Transpiler<'a> {
    /// Transpile a binding modifier from DIR into JS AST.
    pub fn transpile_binding_modifier(
        &self,
        module: &'a Module,
        modifier: dir::BindingModifier,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<BindingModifier> {
        let kind = modifier.kind.map(|kind| self.transpile_binding_kind(kind));
        let scope = modifier
            .scope
            .map(|scope| self.transpile_binding_scope(scope));
        let mutability = modifier
            .mutability
            .map(|mutability| self.transpile_mutability(mutability));
        let visibility = modifier
            .visibility
            .map(|visibility| self.transpile_visibility(visibility));
        let operator = modifier
            .operator
            .map(|operator| self.transpile_binding_operator(operator));
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
                    .map(|value| self.transpile_expression(module, *value, unit))
                    .transpose()?;
                let default = default
                    .as_ref()
                    .map(|default| self.transpile_expression(module, *default, unit))
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
                    .map(|body| self.transpile_expression(module, *body, unit))
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
                    .map(|modifiers| self.transpile_binding_modifier(module, modifiers, unit))?;
                let value = self.transpile_expression(module, value, unit)?;
                Property::Spread { modifiers, value }
            }
        };
        let property_id = unit.ast.insert_from_dir(property, module.id, property_id);
        Ok(property_id)
    }
}
