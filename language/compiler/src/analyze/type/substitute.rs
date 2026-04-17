use super::*;
use destack_dir::MappedTypeParameter;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(crate) fn substitute_this_type(
        &self,
        ty_id: LocalTypeId,
        this_ty_id: LocalTypeId,
        types: &mut TypeTable,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> LocalTypeId {
        if let Some(mapped) = cache.get(&ty_id).copied() {
            return mapped;
        }

        let ty = types.get_type(ty_id).clone();
        let mapped = match ty {
            Type::This => this_ty_id,
            Type::Reference {
                symbol,
                generic_arguments,
            } => {
                if let Some(generic_arguments) = generic_arguments {
                    let mut changed = false;
                    let mapped_arguments = generic_arguments
                        .iter()
                        .map(|argument| {
                            let mapped = self.substitute_this_static_argument(
                                argument, this_ty_id, types, cache,
                            );
                            if mapped != *argument {
                                changed = true;
                            }
                            mapped
                        })
                        .collect::<Vec<_>>();

                    if changed {
                        types.insert_type_from_type(
                            Type::Reference {
                                symbol,
                                generic_arguments: Some(mapped_arguments),
                            },
                            ty_id,
                        )
                    } else {
                        ty_id
                    }
                } else {
                    ty_id
                }
            }
            Type::TypeLiteral { .. }
            | Type::InferVar { .. }
            | Type::Unevaluated(_)
            | Type::Import { .. }
            | Type::Error => ty_id,
            Type::Value { value } => {
                let mapped_value = self.substitute_this_type(value, this_ty_id, types, cache);
                if mapped_value == value {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Value {
                            value: mapped_value,
                        },
                        ty_id,
                    )
                }
            }
            Type::Readonly { target_type: right } => {
                let mapped_right = self.substitute_this_type(right, this_ty_id, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Readonly {
                            target_type: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::KeyOf { target_type: right } => {
                let mapped_right = self.substitute_this_type(right, this_ty_id, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::KeyOf {
                            target_type: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::Must { target_type: right } => {
                let mapped_right = self.substitute_this_type(right, this_ty_id, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Must {
                            target_type: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::AsComptime { target_type: right } => {
                let mapped_right = self.substitute_this_type(right, this_ty_id, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::AsComptime {
                            target_type: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::Not { target_type: right } => {
                let mapped_right = self.substitute_this_type(right, this_ty_id, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Not {
                            target_type: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::In { left, right } => {
                let mapped_left = self.substitute_this_type(left, this_ty_id, types, cache);
                let mapped_right = self.substitute_this_type(right, this_ty_id, types, cache);
                if mapped_left == left && mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::In {
                            left: mapped_left,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::Extends { left, right } => {
                let mapped_left = self.substitute_this_type(left, this_ty_id, types, cache);
                let mapped_right = self.substitute_this_type(right, this_ty_id, types, cache);
                if mapped_left == left && mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Extends {
                            left: mapped_left,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::Implements { left, right } => {
                let mapped_left = self.substitute_this_type(left, this_ty_id, types, cache);
                let mapped_right = self.substitute_this_type(right, this_ty_id, types, cache);
                if mapped_left == left && mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Implements {
                            left: mapped_left,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::Conditional {
                distributive_symbol,
                left,
                right,
                then_type,
                else_type,
            } => {
                let mapped_left = self.substitute_this_type(left, this_ty_id, types, cache);
                let mapped_right = self.substitute_this_type(right, this_ty_id, types, cache);
                let mapped_then = self.substitute_this_type(then_type, this_ty_id, types, cache);
                let mapped_else = self.substitute_this_type(else_type, this_ty_id, types, cache);
                if mapped_left == left
                    && mapped_right == right
                    && mapped_then == then_type
                    && mapped_else == else_type
                {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Conditional {
                            distributive_symbol,
                            left: mapped_left,
                            right: mapped_right,
                            then_type: mapped_then,
                            else_type: mapped_else,
                        },
                        ty_id,
                    )
                }
            }
            Type::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                let mapped_constraint =
                    self.substitute_this_type(parameter.constraint, this_ty_id, types, cache);
                let mapped_key_remap = parameter.key_remap.map(|key_remap| {
                    self.substitute_this_type(key_remap, this_ty_id, types, cache)
                });
                let mapped_value = self.substitute_this_type(value, this_ty_id, types, cache);
                if mapped_constraint == parameter.constraint
                    && mapped_key_remap == parameter.key_remap
                    && mapped_value == value
                {
                    ty_id
                } else {
                    let parameter = MappedTypeParameter {
                        name: parameter.name,
                        symbol: parameter.symbol,
                        constraint: mapped_constraint,
                        key_remap: mapped_key_remap,
                    };
                    types.insert_type_from_type(
                        Type::Mapped {
                            parameter,
                            modifiers,
                            value: mapped_value,
                        },
                        ty_id,
                    )
                }
            }
            Type::Index { left, index } => {
                let mapped_left = self.substitute_this_type(left, this_ty_id, types, cache);
                let mapped_index = self.substitute_this_type(index, this_ty_id, types, cache);
                if mapped_left == left && mapped_index == index {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Index {
                            left: mapped_left,
                            index: mapped_index,
                        },
                        ty_id,
                    )
                }
            }
            Type::TemplateLiteral { strings, spans } => {
                let mut changed = false;
                let mapped_spans = spans
                    .iter()
                    .map(|span| {
                        let mapped = self.substitute_this_type(*span, this_ty_id, types, cache);
                        if mapped != *span {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                if changed {
                    types.insert_type_from_type(
                        Type::TemplateLiteral {
                            strings,
                            spans: mapped_spans,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Infer { name, constraint } => {
                let mapped_constraint = constraint.map(|constraint| {
                    self.substitute_this_type(constraint, this_ty_id, types, cache)
                });
                if mapped_constraint == constraint {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Infer {
                            name,
                            constraint: mapped_constraint,
                        },
                        ty_id,
                    )
                }
            }
            Type::Predicate {
                asserts,
                subject,
                target,
            } => {
                let mapped_target = target
                    .map(|target| self.substitute_this_type(target, this_ty_id, types, cache));
                if mapped_target == target {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Predicate {
                            asserts,
                            subject,
                            target: mapped_target,
                        },
                        ty_id,
                    )
                }
            }
            Type::ValueOf {
                mutability,
                variance,
                right,
            } => {
                let mapped_right = self.substitute_this_type(right, this_ty_id, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::ValueOf {
                            mutability,
                            variance,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                let mapped_right = self.substitute_this_type(right, this_ty_id, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::ReferenceOf {
                            mutability,
                            variance,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::PointerOf { mutability, right } => {
                let mapped_right = self.substitute_this_type(right, this_ty_id, types, cache);
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::PointerOf {
                            mutability,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::ArraySized {
                element,
                count,
                is_readonly,
            } => {
                let mapped_element = self.substitute_this_type(element, this_ty_id, types, cache);
                if mapped_element == element {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::ArraySized {
                            element: mapped_element,
                            count,
                            is_readonly,
                        },
                        ty_id,
                    )
                }
            }
            Type::Array {
                element,
                is_readonly,
            } => {
                let mapped_element = element
                    .map(|element| self.substitute_this_type(element, this_ty_id, types, cache));
                if mapped_element == element {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Array {
                            element: mapped_element,
                            is_readonly,
                        },
                        ty_id,
                    )
                }
            }
            Type::Tuple {
                elements,
                is_readonly,
            } => {
                let mut changed = false;
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        let mapped =
                            self.substitute_this_type(element.ty, this_ty_id, types, cache);
                        if mapped != element.ty {
                            changed = true;
                        }
                        let mut element = element.clone();
                        element.ty = mapped;
                        element
                    })
                    .collect::<Vec<_>>();
                if changed {
                    types.insert_type_from_type(
                        Type::Tuple {
                            elements: mapped_elements,
                            is_readonly,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                let mut changed = false;
                let mapped_fields = fields
                    .iter()
                    .map(|field| {
                        let mapped = self.substitute_this_type(field.ty, this_ty_id, types, cache);
                        if mapped != field.ty {
                            changed = true;
                        }
                        TypeField {
                            key: field.key,
                            ty: mapped,
                            is_optional: field.is_optional,
                            is_readonly: field.is_readonly,
                        }
                    })
                    .collect::<Vec<_>>();
                let mapped_call_signatures = call_signatures
                    .iter()
                    .map(|signature| {
                        let mapped =
                            self.substitute_this_type(*signature, this_ty_id, types, cache);
                        if mapped != *signature {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                let mapped_construct_signatures = construct_signatures
                    .iter()
                    .map(|signature| {
                        let mapped =
                            self.substitute_this_type(*signature, this_ty_id, types, cache);
                        if mapped != *signature {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                let mapped_index_signatures = index_signatures
                    .iter()
                    .map(|signature| {
                        let mapped_key =
                            self.substitute_this_type(signature.key_type, this_ty_id, types, cache);
                        let mapped_value = self.substitute_this_type(
                            signature.value_type,
                            this_ty_id,
                            types,
                            cache,
                        );
                        if mapped_key != signature.key_type || mapped_value != signature.value_type
                        {
                            changed = true;
                        }
                        let mut signature = signature.clone();
                        signature.key_type = mapped_key;
                        signature.value_type = mapped_value;
                        signature
                    })
                    .collect::<Vec<_>>();
                if changed {
                    types.insert_type_from_type(
                        Type::Object {
                            fields: mapped_fields,
                            call_signatures: mapped_call_signatures,
                            construct_signatures: mapped_construct_signatures,
                            index_signatures: mapped_index_signatures,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Function {
                asynchrony,
                cardinality,
                generic_parameters,
                this_parameter,
                parameters,
                return_type,
            } => {
                let mut changed = false;
                let mapped_static_parameters = generic_parameters
                    .iter()
                    .map(|parameter| {
                        let mapped =
                            self.substitute_this_type(*parameter, this_ty_id, types, cache);
                        if mapped != *parameter {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                let mapped_this = this_parameter.map(|this_parameter| {
                    let mapped =
                        self.substitute_this_type(this_parameter, this_ty_id, types, cache);
                    if mapped != this_parameter {
                        changed = true;
                    }
                    mapped
                });
                let mapped_parameters = parameters
                    .iter()
                    .map(|parameter| {
                        let mapped =
                            self.substitute_this_type(*parameter, this_ty_id, types, cache);
                        if mapped != *parameter {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                let mapped_return = return_type.map(|return_type| {
                    let mapped = self.substitute_this_type(return_type, this_ty_id, types, cache);
                    if mapped != return_type {
                        changed = true;
                    }
                    mapped
                });
                if changed {
                    types.insert_type_from_type(
                        Type::Function {
                            asynchrony,
                            cardinality,
                            generic_parameters: mapped_static_parameters,
                            this_parameter: mapped_this,
                            parameters: mapped_parameters,
                            return_type: mapped_return,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Union { elements } => {
                let mut changed = false;
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        let mapped = self.substitute_this_type(*element, this_ty_id, types, cache);
                        if mapped != *element {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                if changed {
                    types.insert_type_from_type(
                        Type::Union {
                            elements: mapped_elements,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Intersection { elements } => {
                let mut changed = false;
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        let mapped = self.substitute_this_type(*element, this_ty_id, types, cache);
                        if mapped != *element {
                            changed = true;
                        }
                        mapped
                    })
                    .collect::<Vec<_>>();
                if changed {
                    types.insert_type_from_type(
                        Type::Intersection {
                            elements: mapped_elements,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
        };

        cache.insert(ty_id, mapped);
        mapped
    }

    /// Substitute `this` types in a static argument.
    pub(crate) fn substitute_this_static_argument(
        &self,
        argument: &StaticArgument,
        this_ty_id: LocalTypeId,
        types: &mut TypeTable,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> StaticArgument {
        match argument {
            StaticArgument::Unevaluated { .. } => argument.clone(),
            StaticArgument::Evaluated { name, value } => {
                let mapped_value =
                    self.substitute_this_static_expression(value, this_ty_id, types, cache);
                StaticArgument::Evaluated {
                    name: *name,
                    value: mapped_value,
                }
            }
        }
    }

    /// Substitute `this` types in a static expression.
    pub(crate) fn substitute_this_static_expression(
        &self,
        expression: &StaticExpression,
        this_ty_id: LocalTypeId,
        types: &mut TypeTable,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> StaticExpression {
        match expression {
            StaticExpression::Unevaluated { .. } => expression.clone(),
            StaticExpression::ScalarLiteral { .. } => expression.clone(),
            StaticExpression::TypeLiteral { .. } => expression.clone(),
            StaticExpression::Type { ty } => StaticExpression::Type {
                ty: self.substitute_this_type(*ty, this_ty_id, types, cache),
            },
            StaticExpression::Declaration {
                declaration,
                generic_arguments,
            } => {
                let mapped_arguments = generic_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.substitute_this_static_argument(argument, this_ty_id, types, cache)
                        })
                        .collect::<Vec<_>>()
                });
                StaticExpression::Declaration {
                    declaration: *declaration,
                    generic_arguments: mapped_arguments,
                }
            }
            StaticExpression::ArrayExpression { elements } => {
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        self.substitute_this_static_expression(element, this_ty_id, types, cache)
                    })
                    .collect::<Vec<_>>();
                StaticExpression::ArrayExpression {
                    elements: mapped_elements,
                }
            }
            StaticExpression::TupleExpression { elements } => {
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        self.substitute_this_static_expression(element, this_ty_id, types, cache)
                    })
                    .collect::<Vec<_>>();
                StaticExpression::TupleExpression {
                    elements: mapped_elements,
                }
            }
            StaticExpression::ObjectExpression { properties } => {
                let mapped_properties = properties
                    .iter()
                    .map(|property| {
                        self.substitute_this_static_property(property, this_ty_id, types, cache)
                    })
                    .collect::<Vec<_>>();
                StaticExpression::ObjectExpression {
                    properties: mapped_properties,
                }
            }
        }
    }

    /// Substitute `this` types in a static property.
    pub(crate) fn substitute_this_static_property(
        &self,
        property: &StaticProperty,
        this_ty_id: LocalTypeId,
        types: &mut TypeTable,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> StaticProperty {
        match property {
            StaticProperty::Unevaluated { .. } => property.clone(),
            StaticProperty::Field { key, value, symbol } => {
                let mapped_value =
                    self.substitute_this_static_expression(value, this_ty_id, types, cache);
                StaticProperty::Field {
                    key: *key,
                    value: mapped_value,
                    symbol: *symbol,
                }
            }
            StaticProperty::Method {
                key,
                signature,
                body,
                symbol,
            } => {
                let mapped_body =
                    self.substitute_this_static_expression(body, this_ty_id, types, cache);
                StaticProperty::Method {
                    key: *key,
                    signature: signature.clone(),
                    body: mapped_body,
                    symbol: *symbol,
                }
            }
            StaticProperty::Spread { value, symbol } => {
                let mapped_value =
                    self.substitute_this_static_expression(value, this_ty_id, types, cache);
                StaticProperty::Spread {
                    value: mapped_value,
                    symbol: *symbol,
                }
            }
        }
    }

    /// Strip nullish types from a type id.
    pub(crate) fn strip_nullish_from_union(
        &self,
        ty_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> (Option<LocalTypeId>, bool) {
        let ty = types.get_type(ty_id);

        match ty {
            Type::Union { elements } => {
                let mut filtered = Vec::new();
                let mut has_nullish = false;

                for element_id in elements {
                    let element_ty = types.get_type(*element_id);
                    if self.is_nullish_type(element_ty) {
                        has_nullish = true;
                    } else {
                        filtered.push(*element_id);
                    }
                }

                if !has_nullish {
                    return (Some(ty_id), false);
                }

                let non_nullish_ty_id = match filtered.len() {
                    0 => None,
                    1 => Some(filtered[0]),
                    _ => {
                        Some(types.insert_type_from_type(Type::Union { elements: filtered }, ty_id))
                    }
                };

                (non_nullish_ty_id, true)
            }
            _ if self.is_nullish_type(ty) => (None, true),
            _ => (Some(ty_id), false),
        }
    }
}
