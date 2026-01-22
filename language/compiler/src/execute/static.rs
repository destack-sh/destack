use crate::{Compiler, ExecuteError, ExecuteResult};

use destack_source::ModuleId;
use destack_workspace::ProfileId;
use {destack_dir as dir, destack_vm as vm};

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Convert a static expression into a VM value.
    pub(crate) fn static_expression_to_value(
        &self,
        value: &dir::StaticExpression,
    ) -> Option<vm::Value> {
        match value {
            dir::StaticExpression::ScalarLiteral { value } => match value {
                dir::ScalarLiteral::Boolean(value) => Some(vm::Value::bool(*value)),
                dir::ScalarLiteral::Integer(value) => Some(vm::Value::int64(*value)),
                dir::ScalarLiteral::Bigint(value) => Some(vm::Value::int64(*value)),
                dir::ScalarLiteral::Float(value) => Some(vm::Value::float64(*value)),
                dir::ScalarLiteral::Character(value) => Some(vm::Value::char(*value)),
                // #Incomplete: support more complex static values in comptime
                dir::ScalarLiteral::String(_) => None,
                dir::ScalarLiteral::RegexString { .. } => None,
            },
            _ => None,
        }
    }

    /// Convert a VM value into a static expression.
    pub(crate) fn value_to_static_expression(
        &self,
        isolate: &vm::Isolate,
        value: &vm::Value,
    ) -> Option<dir::StaticExpression> {
        use vm::ValueTag;

        let scalar = match value.tag() {
            ValueTag::Bool => dir::ScalarLiteral::Boolean(value.as_bool()?),
            ValueTag::Int => dir::ScalarLiteral::Integer(value.as_int()?),
            ValueTag::UInt => dir::ScalarLiteral::Integer(value.as_uint()?.try_into().ok()?),
            ValueTag::Float32 => dir::ScalarLiteral::Float(value.as_float32()? as f64),
            ValueTag::Float64 => dir::ScalarLiteral::Float(value.as_float64()?),
            ValueTag::Char => dir::ScalarLiteral::Character(value.as_char()?),
            ValueTag::String => {
                // resolve the heap-backed UTF8 payload
                let literal = isolate.string_value(*value).ok()?;
                let literal_id = self.program.strings.intern(&literal);
                return Some(dir::StaticExpression::ScalarLiteral {
                    value: dir::ScalarLiteral::String(literal_id),
                });
            }
            _ => return None, // #Incomplete: support more complex static values in comptime
        };

        Some(dir::StaticExpression::ScalarLiteral { value: scalar })
    }

    /// Convert a static expression into a DIR expression node.
    pub(crate) fn static_expression_to_expression(
        &self,
        tree: &mut dir::NodeTree,
        module_id: ModuleId,
        profile_id: ProfileId,
        anchor_id: dir::LocalNodeIdAny,
        parent_id: dir::LocalNodeIdAny,
        scope: (dir::LocalScopeId, dir::LocalScopeMark),
        value: &dir::StaticExpression,
    ) -> ExecuteResult<dir::Expression> {
        match value {
            dir::StaticExpression::ScalarLiteral { value } => Ok(dir::Expression::ScalarLiteral {
                value: value.clone(),
            }),
            dir::StaticExpression::TypeLiteral { value } => Ok(dir::Expression::TypeLiteral {
                value: value.clone(),
            }),
            dir::StaticExpression::Type { ty } => Ok(dir::Expression::Type { value: *ty }),
            dir::StaticExpression::RangeExpression {
                start,
                end,
                is_inclusive,
            } => {
                // build expressions for range bounds
                let start_any =
                    tree.reserve_from(dir::NodeType::Expression, anchor_id, scope, Some(parent_id));
                let start_expression = self.static_expression_to_expression(
                    tree, module_id, profile_id, anchor_id, start_any, scope, start,
                )?;
                let start_id = tree.insert(start_any, start_expression);

                let end_any =
                    tree.reserve_from(dir::NodeType::Expression, anchor_id, scope, Some(parent_id));
                let end_expression = self.static_expression_to_expression(
                    tree, module_id, profile_id, anchor_id, end_any, scope, end,
                )?;
                let end_id = tree.insert(end_any, end_expression);

                Ok(dir::Expression::RangeExpression {
                    start: start_id,
                    end: end_id,
                    is_inclusive: *is_inclusive,
                })
            }
            dir::StaticExpression::ArrayExpression { elements } => {
                // build positional arguments for array elements
                let mut argument_ids = Vec::with_capacity(elements.len());
                for element in elements {
                    let argument_any = tree.reserve_from(
                        dir::NodeType::Argument,
                        anchor_id,
                        scope,
                        Some(parent_id),
                    );
                    let value_any = tree.reserve_from(
                        dir::NodeType::Expression,
                        anchor_id,
                        scope,
                        Some(argument_any),
                    );
                    let value_expression = self.static_expression_to_expression(
                        tree, module_id, profile_id, anchor_id, value_any, scope, element,
                    )?;
                    let value_id = tree.insert(value_any, value_expression);
                    let argument = dir::Argument::Positional {
                        modifiers: None,
                        value: value_id,
                    };
                    let argument_id = tree.insert(argument_any, argument);
                    argument_ids.push(argument_id);
                }

                Ok(dir::Expression::ArrayExpression {
                    elements: argument_ids,
                })
            }
            dir::StaticExpression::TupleExpression { elements } => {
                // build positional arguments for tuple elements
                let mut argument_ids = Vec::with_capacity(elements.len());
                for element in elements {
                    let argument_any = tree.reserve_from(
                        dir::NodeType::Argument,
                        anchor_id,
                        scope,
                        Some(parent_id),
                    );
                    let value_any = tree.reserve_from(
                        dir::NodeType::Expression,
                        anchor_id,
                        scope,
                        Some(argument_any),
                    );
                    let value_expression = self.static_expression_to_expression(
                        tree, module_id, profile_id, anchor_id, value_any, scope, element,
                    )?;
                    let value_id = tree.insert(value_any, value_expression);
                    let argument = dir::Argument::Positional {
                        modifiers: None,
                        value: value_id,
                    };
                    let argument_id = tree.insert(argument_any, argument);
                    argument_ids.push(argument_id);
                }

                Ok(dir::Expression::TupleExpression {
                    elements: argument_ids,
                })
            }
            dir::StaticExpression::ObjectExpression { properties } => {
                // build properties for the object expression
                let mut property_ids = Vec::with_capacity(properties.len());
                for property in properties {
                    let property_any = tree.reserve_from(
                        dir::NodeType::Property,
                        anchor_id,
                        scope,
                        Some(parent_id),
                    );
                    let property = self.static_property_to_property(
                        tree,
                        module_id,
                        profile_id,
                        anchor_id,
                        property_any,
                        scope,
                        property,
                    )?;
                    let property_id = tree.insert(property_any, property);
                    property_ids.push(property_id);
                }

                Ok(dir::Expression::ObjectExpression {
                    properties: property_ids,
                })
            }
            _ => Err(ExecuteError::UnsupportedConstruct {
                node: anchor_id
                    .into_global(module_id)
                    .into_anchored(Some(profile_id)),
            }),
        }
    }

    /// Convert a static property into a DIR property node.
    pub(crate) fn static_property_to_property(
        &self,
        tree: &mut dir::NodeTree,
        module_id: ModuleId,
        profile_id: ProfileId,
        anchor_id: dir::LocalNodeIdAny,
        parent_id: dir::LocalNodeIdAny,
        scope: (dir::LocalScopeId, dir::LocalScopeMark),
        property: &dir::StaticProperty,
    ) -> ExecuteResult<dir::Property> {
        match property {
            dir::StaticProperty::Field {
                modifiers,
                key,
                value,
                default,
                symbol,
            } => {
                // build the field value expression
                let value_any =
                    tree.reserve_from(dir::NodeType::Expression, anchor_id, scope, Some(parent_id));
                let value_expression = self.static_expression_to_expression(
                    tree, module_id, profile_id, anchor_id, value_any, scope, value,
                )?;
                let value_id = tree.insert(value_any, value_expression);

                // build the default expression when provided
                let default_id = match default {
                    Some(default) => {
                        let default_any = tree.reserve_from(
                            dir::NodeType::Expression,
                            anchor_id,
                            scope,
                            Some(parent_id),
                        );
                        let default_expression = self.static_expression_to_expression(
                            tree,
                            module_id,
                            profile_id,
                            anchor_id,
                            default_any,
                            scope,
                            default,
                        )?;
                        Some(tree.insert(default_any, default_expression))
                    }
                    None => None,
                };

                Ok(dir::Property::Field {
                    modifiers: *modifiers,
                    key: *key,
                    value: Some(value_id),
                    default: default_id,
                    symbol: *symbol,
                })
            }
            dir::StaticProperty::Method {
                modifiers,
                key,
                signature,
                body,
                symbol,
            } => {
                // build the method body expression
                let body_any =
                    tree.reserve_from(dir::NodeType::Expression, anchor_id, scope, Some(parent_id));
                let body_expression = self.static_expression_to_expression(
                    tree, module_id, profile_id, anchor_id, body_any, scope, body,
                )?;
                let body_id = tree.insert(body_any, body_expression);

                Ok(dir::Property::Method {
                    modifiers: *modifiers,
                    key: *key,
                    signature: signature.clone(),
                    body: Some(body_id),
                    symbol: *symbol,
                })
            }
            dir::StaticProperty::Unevaluated { .. } => Err(ExecuteError::UnsupportedConstruct {
                node: anchor_id
                    .into_global(module_id)
                    .into_anchored(Some(profile_id)),
            }),
        }
    }
}
