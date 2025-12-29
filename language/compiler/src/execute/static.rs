use crate::{Compiler, ExecuteError, ExecuteResult};

use destack_source::ModuleId;
use {destack_dir as dir, destack_machine as machine};

#[allow(dead_code)]
impl Compiler {
    /// Convert a static expression into a machine value.
    pub(crate) fn static_expression_to_value(
        &self,
        value: &dir::StaticExpression,
    ) -> Option<machine::Value> {
        match value {
            dir::StaticExpression::ScalarLiteral { value } => match value {
                dir::ScalarLiteral::Boolean(value) => Some(machine::Value::Bool(*value)),
                dir::ScalarLiteral::Integer(value) => Some(machine::Value::Int {
                    value: *value,
                    width: 64,
                }),
                dir::ScalarLiteral::Bigint(value) => Some(machine::Value::Int {
                    value: *value,
                    width: 64,
                }),
                dir::ScalarLiteral::Float(value) => Some(machine::Value::Float64(*value)),
                dir::ScalarLiteral::Character(value) => Some(machine::Value::Char(*value)),
                dir::ScalarLiteral::String(value) => {
                    let value = self.program.strings.get(*value).to_string();
                    Some(machine::Value::String(value.into_boxed_str()))
                }
                dir::ScalarLiteral::RegexString { .. } => None,
            },
            _ => None,
        }
    }

    /// Convert a machine value into a static expression.
    pub(crate) fn value_to_static_expression(
        &self,
        value: &machine::Value,
    ) -> Option<dir::StaticExpression> {
        let scalar = match value {
            machine::Value::Bool(value) => dir::ScalarLiteral::Boolean(*value),
            machine::Value::Int { value, .. } => dir::ScalarLiteral::Integer(*value),
            machine::Value::UInt { value, .. } => {
                dir::ScalarLiteral::Integer((*value).try_into().ok()?)
            }
            machine::Value::Float32(value) => dir::ScalarLiteral::Float(*value as f64),
            machine::Value::Float64(value) => dir::ScalarLiteral::Float(*value),
            machine::Value::String(value) => {
                let id = self.program.strings.intern(value.as_ref());
                dir::ScalarLiteral::String(id)
            }
            machine::Value::Char(value) => dir::ScalarLiteral::Character(*value),
            _ => return None,
        };

        Some(dir::StaticExpression::ScalarLiteral { value: scalar })
    }

    /// Convert a static expression into a DIR expression node.
    pub(crate) fn static_expression_to_expression(
        &self,
        tree: &mut dir::NodeTree,
        module_id: ModuleId,
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
                // build range endpoints as child expressions
                let start_any = tree.reserve_from(
                    dir::NodeType::Expression,
                    anchor_id,
                    scope,
                    Some(parent_id),
                );
                let start_expression = self.static_expression_to_expression(
                    tree,
                    module_id,
                    anchor_id,
                    start_any,
                    scope,
                    start,
                )?;
                let start_id = tree.insert(start_any, start_expression);

                let end_any = tree.reserve_from(
                    dir::NodeType::Expression,
                    anchor_id,
                    scope,
                    Some(parent_id),
                );
                let end_expression = self.static_expression_to_expression(
                    tree,
                    module_id,
                    anchor_id,
                    end_any,
                    scope,
                    end,
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
                        tree,
                        module_id,
                        anchor_id,
                        value_any,
                        scope,
                        element,
                    )?;
                    let value_id = tree.insert(value_any, value_expression);
                    let argument = dir::Argument::Positional { value: value_id };
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
                        tree,
                        module_id,
                        anchor_id,
                        value_any,
                        scope,
                        element,
                    )?;
                    let value_id = tree.insert(value_any, value_expression);
                    let argument = dir::Argument::Positional { value: value_id };
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
                node: anchor_id.into_global(module_id),
            }),
        }
    }

    /// Convert a static property into a DIR property node.
    pub(crate) fn static_property_to_property(
        &self,
        tree: &mut dir::NodeTree,
        module_id: ModuleId,
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
                let value_any = tree.reserve_from(
                    dir::NodeType::Expression,
                    anchor_id,
                    scope,
                    Some(parent_id),
                );
                let value_expression = self.static_expression_to_expression(
                    tree,
                    module_id,
                    anchor_id,
                    value_any,
                    scope,
                    value,
                )?;
                let value_id = tree.insert(value_any, value_expression);

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
                let body_any = tree.reserve_from(
                    dir::NodeType::Expression,
                    anchor_id,
                    scope,
                    Some(parent_id),
                );
                let body_expression = self.static_expression_to_expression(
                    tree,
                    module_id,
                    anchor_id,
                    body_any,
                    scope,
                    body,
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
                node: anchor_id.into_global(module_id),
            }),
        }
    }
}
