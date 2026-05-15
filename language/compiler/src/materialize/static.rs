use crate::{Compiler, MaterializeError, MaterializeResult};

use destack_source::ModuleId;
use destack_workspace::ProfileId;
use {destack_dir as dir, destack_engine as engine, destack_vm as vm};

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Convert a static expression into an engine boundary value.
    pub(crate) fn static_expression_to_value(
        &self,
        value: &dir::StaticExpression,
    ) -> Option<engine::Value> {
        match value {
            dir::StaticExpression::ScalarLiteral { value } => match value {
                dir::ScalarLiteral::Boolean(value) => Some(engine::Value::bool(*value)),
                dir::ScalarLiteral::Integer(value) => Some(engine::Value::int64(*value)),
                dir::ScalarLiteral::Bigint(value) => Some(engine::Value::int64(*value)),
                dir::ScalarLiteral::Float(value) => Some(engine::Value::float64(*value)),
                dir::ScalarLiteral::Character(value) => Some(engine::Value::char(*value)),
                // #Incomplete: support more complex static values in comptime
                dir::ScalarLiteral::Null => None,
                dir::ScalarLiteral::String(_) => None,
                dir::ScalarLiteral::RegexString { .. } => None,
            },
            _ => None,
        }
    }

    /// Convert an engine boundary value into a static expression.
    pub(crate) fn value_to_static_expression(
        &self,
        _isolate: &vm::Isolate,
        _heap: &vm::Heap,
        value: &engine::Value,
    ) -> Option<dir::StaticExpression> {
        let scalar = match value {
            engine::Value::Bool(value) => dir::ScalarLiteral::Boolean(*value),
            engine::Value::Int { value, .. } => {
                dir::ScalarLiteral::Integer(i64::try_from(*value).ok()?)
            }
            engine::Value::UInt { value, .. } => {
                dir::ScalarLiteral::Integer((*value).try_into().ok()?)
            }
            engine::Value::Float32 { bits } => {
                dir::ScalarLiteral::Float(f32::from_bits(*bits) as f64)
            }
            engine::Value::Float64 { bits } => dir::ScalarLiteral::Float(f64::from_bits(*bits)),
            engine::Value::Char(value) => dir::ScalarLiteral::Character(*value),

            _ => return None, // #Incomplete: support more complex static values in comptime
        };

        Some(dir::StaticExpression::ScalarLiteral { value: scalar })
    }

    /// Require one type expression node for one local type id.
    fn insert_comptime_type_expression_for_type_id(
        &self,
        tree: &mut dir::Tree,
        types: &dir::TypeTable,
        module_id: ModuleId,
        profile_id: ProfileId,
        anchor_id: dir::LocalNodeIdAny,
        parent_id: dir::LocalNodeIdAny,
        scope: (dir::LocalScopeId, dir::LocalScopeMark),
        type_id: dir::LocalTypeId,
    ) -> MaterializeResult<dir::LocalNodeId<dir::TypeExpression>> {
        // materialize literal types directly when no reusable syntax node exists
        if let dir::Type::Literal(dir::LiteralType { value }) = types.get_type(type_id) {
            let expression_any = tree.reserve_from(
                dir::NodeType::TypeExpression,
                anchor_id,
                scope,
                Some(parent_id),
                Some(dir::ProvenanceReason::Evaluated),
            );
            let expression = dir::TypeExpression::Literal {
                value: value.clone(),
            };
            let expression_id = tree.insert(expression_any, expression);

            return Ok(expression_id);
        }

        let _ = (
            tree, types, module_id, profile_id, anchor_id, parent_id, scope, type_id,
        );

        todo!("FUGU #Incomplete: insert comptime types")
    }

    /// Convert a static expression into a DIR expression node.
    pub(crate) fn static_expression_to_expression(
        &self,
        tree: &mut dir::Tree,
        types: &dir::TypeTable,
        module_id: ModuleId,
        profile_id: ProfileId,
        anchor_id: dir::LocalNodeIdAny,
        parent_id: dir::LocalNodeIdAny,
        scope: (dir::LocalScopeId, dir::LocalScopeMark),
        value: &dir::StaticExpression,
    ) -> MaterializeResult<dir::Expression> {
        match value {
            dir::StaticExpression::ScalarLiteral { value } => Ok(dir::Expression::ScalarLiteral {
                value: value.clone(),
            }),
            dir::StaticExpression::TypeLiteral { value } => Ok(dir::Expression::TypeLiteral {
                value: value.clone(),
            }),
            dir::StaticExpression::Type { ty } => {
                let value = self.insert_comptime_type_expression_for_type_id(
                    tree, types, module_id, profile_id, anchor_id, parent_id, scope, *ty,
                )?;

                Ok(dir::Expression::Type {
                    value,
                    resolved_type: *ty,
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
                        Some(dir::ProvenanceReason::Evaluated),
                    );
                    let value_any = tree.reserve_from(
                        dir::NodeType::Expression,
                        anchor_id,
                        scope,
                        Some(argument_any),
                        Some(dir::ProvenanceReason::Evaluated),
                    );
                    let value_expression = self.static_expression_to_expression(
                        tree, types, module_id, profile_id, anchor_id, value_any, scope, element,
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
                        Some(dir::ProvenanceReason::Evaluated),
                    );
                    let value_any = tree.reserve_from(
                        dir::NodeType::Expression,
                        anchor_id,
                        scope,
                        Some(argument_any),
                        Some(dir::ProvenanceReason::Evaluated),
                    );
                    let value_expression = self.static_expression_to_expression(
                        tree, types, module_id, profile_id, anchor_id, value_any, scope, element,
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
                        Some(dir::ProvenanceReason::Evaluated),
                    );
                    let property = self.static_property_to_property(
                        tree,
                        types,
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
            _ => Err(MaterializeError::UnsupportedConstruct {
                node: anchor_id
                    .into_global(module_id)
                    .into_anchored(Some(profile_id)),
            }),
        }
    }

    /// Convert a static property into a DIR property node.
    pub(crate) fn static_property_to_property(
        &self,
        tree: &mut dir::Tree,
        types: &dir::TypeTable,
        module_id: ModuleId,
        profile_id: ProfileId,
        anchor_id: dir::LocalNodeIdAny,
        parent_id: dir::LocalNodeIdAny,
        scope: (dir::LocalScopeId, dir::LocalScopeMark),
        property: &dir::StaticProperty,
    ) -> MaterializeResult<dir::Property> {
        match property {
            dir::StaticProperty::Field { key, value, symbol } => {
                // build the field value expression
                let value_any = tree.reserve_from(
                    dir::NodeType::Expression,
                    anchor_id,
                    scope,
                    Some(parent_id),
                    Some(dir::ProvenanceReason::Evaluated),
                );
                let value_expression = self.static_expression_to_expression(
                    tree, types, module_id, profile_id, anchor_id, value_any, scope, value,
                )?;
                let value_id = tree.insert(value_any, value_expression);

                Ok(dir::Property::Field {
                    key: *key,
                    value: value_id,
                    is_shorthand: false,
                    symbol: *symbol,
                })
            }
            dir::StaticProperty::Method {
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
                    Some(dir::ProvenanceReason::Evaluated),
                );
                let body_expression = self.static_expression_to_expression(
                    tree, types, module_id, profile_id, anchor_id, body_any, scope, body,
                )?;
                let body_id = tree.insert(body_any, body_expression);

                Ok(dir::Property::Method {
                    key: *key,
                    signature: signature.clone(),
                    body: Some(body_id),
                    symbol: *symbol,
                })
            }
            dir::StaticProperty::Spread { value, symbol } => {
                // build the spread value expression
                let value_any = tree.reserve_from(
                    dir::NodeType::Expression,
                    anchor_id,
                    scope,
                    Some(parent_id),
                    Some(dir::ProvenanceReason::Evaluated),
                );
                let value_expression = self.static_expression_to_expression(
                    tree, types, module_id, profile_id, anchor_id, value_any, scope, value,
                )?;
                let value_id = tree.insert(value_any, value_expression);

                Ok(dir::Property::Spread {
                    value: value_id,
                    symbol: *symbol,
                })
            }
            dir::StaticProperty::Unevaluated { .. } => Err(MaterializeError::UnsupportedConstruct {
                node: anchor_id
                    .into_global(module_id)
                    .into_anchored(Some(profile_id)),
            }),
        }
    }
}
