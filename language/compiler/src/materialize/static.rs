use crate::{Compiler, MaterializeError, MaterializeResult};

use destack_source::ModuleId;
use destack_workspace::ProfileId;
use {destack_dir as dir, destack_engine as engine, destack_vm as vm};

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Convert a static term into an engine boundary value.
    pub(crate) fn static_term_to_value(&self, value: &dir::StaticTerm) -> Option<engine::Value> {
        match value {
            dir::StaticTerm::ScalarLiteral { value } => match value {
                dir::ScalarLiteral::Boolean(value) => Some(engine::Value::bool(*value)),
                dir::ScalarLiteral::Integer(value) => Some(engine::Value::int64(*value)),
                dir::ScalarLiteral::Bigint(value) => Some(engine::Value::int64(*value)),
                dir::ScalarLiteral::Float(value) => Some(engine::Value::float64(*value)),
                dir::ScalarLiteral::Character(value) => Some(engine::Value::char(*value)),
                dir::ScalarLiteral::Null => None,
                dir::ScalarLiteral::String(_) => None,
                dir::ScalarLiteral::RegexString { .. } => None,
            },
            _ => None,
        }
    }

    /// Convert an engine boundary value into a static term.
    pub(crate) fn value_to_static_term(
        &self,
        _isolate: &vm::Isolate,
        _heap: &vm::Heap,
        value: &engine::Value,
    ) -> Option<dir::StaticTerm> {
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

            // #Incomplete: support more complex static values in comptime
            _ => return None,
        };

        Some(dir::StaticTerm::ScalarLiteral { value: scalar })
    }

    /// Require one type expression node for one local type id.
    fn insert_comptime_type_expression_for_type_id(
        &self,
        tree: &mut dir::Tree,
        types: &dir::TypeTable<'_>,
        module_id: ModuleId,
        profile_id: ProfileId,
        anchor_id: dir::LocalNodeIdAny,
        parent_id: dir::LocalNodeIdAny,
        scope: (dir::LocalScopeId, dir::LocalScopeMark),
        type_id: dir::LocalTypeId,
    ) -> MaterializeResult<dir::LocalNodeId<dir::TypeExpression>> {
        let expression = match types.get_type(type_id) {
            dir::Type::Never => dir::TypeExpression::Literal {
                value: dir::TypeLiteral::Never,
            },
            dir::Type::Any => dir::TypeExpression::Literal {
                value: dir::TypeLiteral::Any,
            },
            dir::Type::Unknown => dir::TypeExpression::Literal {
                value: dir::TypeLiteral::Unknown,
            },
            dir::Type::Void => dir::TypeExpression::Literal {
                value: dir::TypeLiteral::Void,
            },
            dir::Type::Null => dir::TypeExpression::Literal {
                value: dir::TypeLiteral::Null,
            },
            dir::Type::Undefined => dir::TypeExpression::Literal {
                value: dir::TypeLiteral::Undefined,
            },
            dir::Type::Object => dir::TypeExpression::Literal {
                value: dir::TypeLiteral::Object,
            },
            dir::Type::Primitive(primitive) => dir::TypeExpression::Literal {
                value: (*primitive).into(),
            },
            dir::Type::Literal(value) => dir::TypeExpression::ScalarLiteral {
                value: value.clone(),
            },
            dir::Type::Operation(dir::TypeOperation::BuiltinTypeFunction(function)) => {
                dir::TypeExpression::Literal {
                    value: dir::TypeLiteral::BuiltinTypeFunction(*function),
                }
            }
            _ => {
                return Err(MaterializeError::UnsupportedConstruct {
                    anchor: anchor_id
                        .into_global(module_id)
                        .into_anchored(Some(profile_id)),
                });
            }
        };

        let expression_any =
            tree.reserve_from(dir::NodeType::TypeExpression, anchor_id, scope, Some(parent_id));
        let expression_id = tree.insert(expression_any, expression);

        Ok(expression_id)
    }

    /// Convert a static term into a DIR expression node.
    pub(crate) fn static_term_to_expression(
        &self,
        tree: &mut dir::Tree,
        types: &dir::TypeTable<'_>,
        module_id: ModuleId,
        profile_id: ProfileId,
        anchor_id: dir::LocalNodeIdAny,
        parent_id: dir::LocalNodeIdAny,
        scope: (dir::LocalScopeId, dir::LocalScopeMark),
        value: &dir::StaticTerm,
    ) -> MaterializeResult<dir::Expression> {
        match value {
            dir::StaticTerm::ScalarLiteral { value } => Ok(dir::Expression::ScalarLiteral {
                value: value.clone(),
            }),
            dir::StaticTerm::TypeLiteral { value } => Ok(dir::Expression::TypeLiteral {
                value: value.clone(),
            }),
            dir::StaticTerm::Type { ty } => {
                let value = self.insert_comptime_type_expression_for_type_id(
                    tree, types, module_id, profile_id, anchor_id, parent_id, scope, *ty,
                )?;

                Ok(dir::Expression::Type {
                    value,
                    resolved_type: *ty,
                })
            }
            dir::StaticTerm::Array { elements } => {
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
                    let value_expression = self.static_term_to_expression(
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
            dir::StaticTerm::FixedArray { value, length } => {
                // build the repeated value expression
                let value_any =
                    tree.reserve_from(dir::NodeType::Expression, anchor_id, scope, Some(parent_id));
                let value_expression = self.static_term_to_expression(
                    tree, types, module_id, profile_id, anchor_id, value_any, scope, value,
                )?;
                let value = tree.insert(value_any, value_expression);

                // build the fixed length expression
                let length_any =
                    tree.reserve_from(dir::NodeType::Expression, anchor_id, scope, Some(parent_id));
                let length_expression = self.static_term_to_expression(
                    tree, types, module_id, profile_id, anchor_id, length_any, scope, length,
                )?;
                let length = tree.insert(length_any, length_expression);

                Ok(dir::Expression::FixedArrayExpression { value, length })
            }
            dir::StaticTerm::Tuple { elements } => {
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
                    let value_expression = self.static_term_to_expression(
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
            dir::StaticTerm::Object { properties } => {
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
            dir::StaticTerm::Struct { ty, properties } => {
                let type_expression = self.insert_comptime_type_expression_for_type_id(
                    tree, types, module_id, profile_id, anchor_id, parent_id, scope, *ty,
                )?;

                // build properties for the struct expression
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

                Ok(dir::Expression::StructExpression {
                    ty: type_expression,
                    properties: property_ids,
                })
            }
            dir::StaticTerm::Declaration { .. } => Err(MaterializeError::UnsupportedConstruct {
                anchor: anchor_id
                    .into_global(module_id)
                    .into_anchored(Some(profile_id)),
            }),
            dir::StaticTerm::Symbol { .. }
            | dir::StaticTerm::Access { .. }
            | dir::StaticTerm::Space { .. }
            | dir::StaticTerm::Place { .. }
            | dir::StaticTerm::Lifetime { .. } => Err(MaterializeError::UnsupportedConstruct {
                anchor: anchor_id
                    .into_global(module_id)
                    .into_anchored(Some(profile_id)),
            }),
        }
    }

    /// Convert a static property into a DIR property node.
    pub(crate) fn static_property_to_property(
        &self,
        tree: &mut dir::Tree,
        types: &dir::TypeTable<'_>,
        module_id: ModuleId,
        profile_id: ProfileId,
        anchor_id: dir::LocalNodeIdAny,
        parent_id: dir::LocalNodeIdAny,
        scope: (dir::LocalScopeId, dir::LocalScopeMark),
        property: &dir::StaticProperty,
    ) -> MaterializeResult<dir::Property> {
        match property {
            dir::StaticProperty::Field { key, value } => {
                let key =
                    self.static_property_key_to_key(module_id, profile_id, anchor_id, *key)?;

                // build the field value expression
                let value_any = tree.reserve_from(
                    dir::NodeType::Expression,
                    anchor_id,
                    scope,
                    Some(parent_id),
                );
                let value_expression = self.static_term_to_expression(
                    tree, types, module_id, profile_id, anchor_id, value_any, scope, value,
                )?;
                let value_id = tree.insert(value_any, value_expression);

                Ok(dir::Property::Field {
                    key,
                    value: value_id,
                    is_shorthand: false,
                })
            }
            dir::StaticProperty::Method {
                key,
                signature,
                body,
            } => {
                let key = key
                    .map(|key| {
                        self.static_property_key_to_key(module_id, profile_id, anchor_id, key)
                    })
                    .transpose()?;

                // build the method body expression
                let body_any = tree.reserve_from(
                    dir::NodeType::Expression,
                    anchor_id,
                    scope,
                    Some(parent_id),
                );
                let body_expression = self.static_term_to_expression(
                    tree, types, module_id, profile_id, anchor_id, body_any, scope, body,
                )?;
                let body_id = tree.insert(body_any, body_expression);

                Ok(dir::Property::Method {
                    key,
                    signature: signature.clone(),
                    body: Some(body_id),
                })
            }
            dir::StaticProperty::Spread { value } => {
                // build the spread value expression
                let value_any = tree.reserve_from(
                    dir::NodeType::Expression,
                    anchor_id,
                    scope,
                    Some(parent_id),
                );
                let value_expression = self.static_term_to_expression(
                    tree, types, module_id, profile_id, anchor_id, value_any, scope, value,
                )?;
                let value_id = tree.insert(value_any, value_expression);

                Ok(dir::Property::Spread { value: value_id })
            }
        }
    }

    /// Convert a static property key into a DIR property key.
    fn static_property_key_to_key(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        anchor_id: dir::LocalNodeIdAny,
        key: dir::StaticKey,
    ) -> MaterializeResult<dir::Key> {
        match key {
            dir::StaticKey::Name(name) => Ok(dir::Key::Name(dir::Name::String(name))),
            dir::StaticKey::Number(name) => Ok(dir::Key::Name(dir::Name::Number(name))),
            dir::StaticKey::Symbol(_) => Err(MaterializeError::UnsupportedConstruct {
                anchor: anchor_id
                    .into_global(module_id)
                    .into_anchored(Some(profile_id)),
            }),
        }
    }
}
