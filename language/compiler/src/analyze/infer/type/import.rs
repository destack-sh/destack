use super::*;

impl Compiler {
    /// Import a type from a remote module into the current module's TypeTable.
    /// For structural types (arrays, objects, ..): recursively copy the type structure.
    /// For nominal types (Type::Reference): keep them as references to the original symbol.
    pub(crate) fn import_remote_type_for_node(
        &self,
        node_id: LocalNodeIdAny,
        remote_ty: &Type,
        remote_types: &TypeTable,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        match remote_ty {
            // leaf types: copy directly
            Type::TypeLiteral { value } => types.insert_imported_type_from_any(
                Type::TypeLiteral {
                    value: value.clone(),
                },
                node_id,
            ),
            Type::InferVar { .. } => types.insert_imported_type_from_any(Type::Error, node_id),
            Type::Error => types.insert_imported_type_from_any(Type::Error, node_id),
            Type::This => types.insert_imported_type_from_any(Type::This, node_id),
            Type::Conditional {
                distributive_symbol,
                left,
                right,
                then_type,
                else_type,
            } => {
                let local_left = self.import_remote_type_for_node(
                    node_id,
                    remote_types.get_type(*left),
                    remote_types,
                    types,
                );
                let local_right = self.import_remote_type_for_node(
                    node_id,
                    remote_types.get_type(*right),
                    remote_types,
                    types,
                );
                let local_then = self.import_remote_type_for_node(
                    node_id,
                    remote_types.get_type(*then_type),
                    remote_types,
                    types,
                );
                let local_else = self.import_remote_type_for_node(
                    node_id,
                    remote_types.get_type(*else_type),
                    remote_types,
                    types,
                );
                types.insert_imported_type_from_any(
                    Type::Conditional {
                        distributive_symbol: *distributive_symbol,
                        left: local_left,
                        right: local_right,
                        then_type: local_then,
                        else_type: local_else,
                    },
                    node_id,
                )
            }
            Type::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                let local_constraint = self.import_remote_type_for_node(
                    node_id,
                    remote_types.get_type(parameter.constraint),
                    remote_types,
                    types,
                );
                let local_key_remap = parameter.key_remap.map(|key_remap| {
                    self.import_remote_type_for_node(
                        node_id,
                        remote_types.get_type(key_remap),
                        remote_types,
                        types,
                    )
                });
                let local_value = self.import_remote_type_for_node(
                    node_id,
                    remote_types.get_type(*value),
                    remote_types,
                    types,
                );
                let parameter = destack_dir::MappedTypeParameter {
                    name: parameter.name,
                    symbol: parameter.symbol,
                    constraint: local_constraint,
                    key_remap: local_key_remap,
                };
                types.insert_imported_type_from_any(
                    Type::Mapped {
                        parameter,
                        modifiers: *modifiers,
                        value: local_value,
                    },
                    node_id,
                )
            }
            Type::Index { left, index } => {
                let local_left = self.import_remote_type_for_node(
                    node_id,
                    remote_types.get_type(*left),
                    remote_types,
                    types,
                );
                let local_index = self.import_remote_type_for_node(
                    node_id,
                    remote_types.get_type(*index),
                    remote_types,
                    types,
                );
                types.insert_imported_type_from_any(
                    Type::Index {
                        left: local_left,
                        index: local_index,
                    },
                    node_id,
                )
            }
            Type::TemplateLiteral { strings, spans } => {
                let local_spans = spans
                    .iter()
                    .map(|span| {
                        self.import_remote_type_for_node(
                            node_id,
                            remote_types.get_type(*span),
                            remote_types,
                            types,
                        )
                    })
                    .collect();
                types.insert_imported_type_from_any(
                    Type::TemplateLiteral {
                        strings: strings.clone(),
                        spans: local_spans,
                    },
                    node_id,
                )
            }
            Type::Import {
                target,
                qualifier,
                static_arguments,
            } => {
                // map embedded type ids inside static arguments
                let local_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.import_remote_static_argument_for_node(
                                node_id,
                                argument,
                                remote_types,
                                types,
                            )
                        })
                        .collect::<Vec<_>>()
                });
                types.insert_imported_type_from_any(
                    Type::Import {
                        target: *target,
                        qualifier: qualifier.clone(),
                        static_arguments: local_arguments,
                    },
                    node_id,
                )
            }
            Type::Infer { name, constraint } => {
                let local_constraint = constraint.map(|constraint| {
                    self.import_remote_type_for_node(
                        node_id,
                        remote_types.get_type(constraint),
                        remote_types,
                        types,
                    )
                });
                types.insert_imported_type_from_any(
                    Type::Infer {
                        name: *name,
                        constraint: local_constraint,
                    },
                    node_id,
                )
            }
            Type::Predicate {
                asserts,
                subject,
                target,
            } => {
                let local_target = target.map(|target| {
                    self.import_remote_type_for_node(
                        node_id,
                        remote_types.get_type(target),
                        remote_types,
                        types,
                    )
                });
                types.insert_imported_type_from_any(
                    Type::Predicate {
                        asserts: *asserts,
                        subject: *subject,
                        target: local_target,
                    },
                    node_id,
                )
            }

            // array types
            Type::Array {
                element: None,
                is_readonly,
            } => types.insert_imported_type_from_any(
                Type::Array {
                    element: None,
                    is_readonly: *is_readonly,
                },
                node_id,
            ),
            Type::Array {
                element: Some(element_id),
                is_readonly,
            } => {
                let element_ty = remote_types.get_type(*element_id);
                let local_elem =
                    self.import_remote_type_for_node(node_id, element_ty, remote_types, types);
                types.insert_imported_type_from_any(
                    Type::Array {
                        element: Some(local_elem),
                        is_readonly: *is_readonly,
                    },
                    node_id,
                )
            }

            // tuple types
            Type::Tuple {
                elements,
                is_readonly,
            } => {
                let local_elements: Vec<_> = elements
                    .iter()
                    .map(|element| {
                        let ty = remote_types.get_type(element.ty);
                        let local_ty =
                            self.import_remote_type_for_node(node_id, ty, remote_types, types);
                        let mut element = element.clone();
                        element.ty = local_ty;
                        element
                    })
                    .collect();
                types.insert_imported_type_from_any(
                    Type::Tuple {
                        elements: local_elements,
                        is_readonly: *is_readonly,
                    },
                    node_id,
                )
            }

            // object types
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                let local_fields: Vec<_> = fields
                    .iter()
                    .map(|field| {
                        let ty = remote_types.get_type(field.ty);
                        let local_ty =
                            self.import_remote_type_for_node(node_id, ty, remote_types, types);
                        TypeField {
                            key: field.key,
                            ty: local_ty,
                            is_optional: field.is_optional,
                            is_readonly: field.is_readonly,
                        }
                    })
                    .collect();
                let local_call_signatures: Vec<_> = call_signatures
                    .iter()
                    .map(|signature| {
                        let ty = remote_types.get_type(*signature);
                        self.import_remote_type_for_node(node_id, ty, remote_types, types)
                    })
                    .collect();
                let local_construct_signatures: Vec<_> = construct_signatures
                    .iter()
                    .map(|signature| {
                        let ty = remote_types.get_type(*signature);
                        self.import_remote_type_for_node(node_id, ty, remote_types, types)
                    })
                    .collect();
                let local_index_signatures: Vec<_> = index_signatures
                    .iter()
                    .map(|signature| {
                        let key_type = remote_types.get_type(signature.key_type);
                        let value_type = remote_types.get_type(signature.value_type);
                        TypeIndexSignature {
                            name: signature.name,
                            key_type: self.import_remote_type_for_node(
                                node_id,
                                key_type,
                                remote_types,
                                types,
                            ),
                            value_type: self.import_remote_type_for_node(
                                node_id,
                                value_type,
                                remote_types,
                                types,
                            ),
                            is_optional: signature.is_optional,
                            is_readonly: signature.is_readonly,
                        }
                    })
                    .collect();
                types.insert_imported_type_from_any(
                    Type::Object {
                        fields: local_fields,
                        call_signatures: local_call_signatures,
                        construct_signatures: local_construct_signatures,
                        index_signatures: local_index_signatures,
                    },
                    node_id,
                )
            }

            // function types
            Type::Function {
                asynchrony,
                cardinality,
                static_parameters,
                this_parameter,
                dynamic_parameters,
                return_type,
            } => {
                let local_static_params: Vec<_> = static_parameters
                    .iter()
                    .map(|id| {
                        let ty = remote_types.get_type(*id);
                        self.import_remote_type_for_node(node_id, ty, remote_types, types)
                    })
                    .collect();
                let local_this = this_parameter.map(|this_parameter| {
                    let ty = remote_types.get_type(this_parameter);
                    self.import_remote_type_for_node(node_id, ty, remote_types, types)
                });
                let local_dynamic_params: Vec<_> = dynamic_parameters
                    .iter()
                    .map(|id| {
                        let ty = remote_types.get_type(*id);
                        self.import_remote_type_for_node(node_id, ty, remote_types, types)
                    })
                    .collect();
                let local_return = return_type.map(|id| {
                    let ty = remote_types.get_type(id);
                    self.import_remote_type_for_node(node_id, ty, remote_types, types)
                });
                types.insert_imported_type_from_any(
                    Type::Function {
                        asynchrony: *asynchrony,
                        cardinality: *cardinality,
                        static_parameters: local_static_params,
                        this_parameter: local_this,
                        dynamic_parameters: local_dynamic_params,
                        return_type: local_return,
                    },
                    node_id,
                )
            }

            // union and intersection types
            Type::Union { elements } => {
                let local_elements: Vec<_> = elements
                    .iter()
                    .map(|id| {
                        let ty = remote_types.get_type(*id);
                        self.import_remote_type_for_node(node_id, ty, remote_types, types)
                    })
                    .collect();
                types.insert_imported_type_from_any(
                    Type::Union {
                        elements: local_elements,
                    },
                    node_id,
                )
            }
            Type::Intersection { elements } => {
                let local_elements: Vec<_> = elements
                    .iter()
                    .map(|id| {
                        let ty = remote_types.get_type(*id);
                        self.import_remote_type_for_node(node_id, ty, remote_types, types)
                    })
                    .collect();
                types.insert_imported_type_from_any(
                    Type::Intersection {
                        elements: local_elements,
                    },
                    node_id,
                )
            }

            // type modifiers: recurse into inner type
            Type::Value { value } => {
                let inner_ty = remote_types.get_type(*value);
                let local_inner =
                    self.import_remote_type_for_node(node_id, inner_ty, remote_types, types);
                types.insert_imported_type_from_any(Type::Value { value: local_inner }, node_id)
            }
            Type::ValueOf {
                mutability,
                variance,
                right,
            } => {
                let inner_ty = remote_types.get_type(*right);
                let local_inner =
                    self.import_remote_type_for_node(node_id, inner_ty, remote_types, types);
                types.insert_imported_type_from_any(
                    Type::ValueOf {
                        mutability: *mutability,
                        variance: *variance,
                        right: local_inner,
                    },
                    node_id,
                )
            }
            Type::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                let inner_ty = remote_types.get_type(*right);
                let local_inner =
                    self.import_remote_type_for_node(node_id, inner_ty, remote_types, types);
                types.insert_imported_type_from_any(
                    Type::ReferenceOf {
                        mutability: *mutability,
                        variance: *variance,
                        right: local_inner,
                    },
                    node_id,
                )
            }
            Type::PointerOf { mutability, right } => {
                let inner_ty = remote_types.get_type(*right);
                let local_inner =
                    self.import_remote_type_for_node(node_id, inner_ty, remote_types, types);
                types.insert_imported_type_from_any(
                    Type::PointerOf {
                        mutability: *mutability,
                        right: local_inner,
                    },
                    node_id,
                )
            }
            Type::Readonly { target_type: right } => {
                let inner_ty = remote_types.get_type(*right);
                let local_inner =
                    self.import_remote_type_for_node(node_id, inner_ty, remote_types, types);
                types.insert_imported_type_from_any(
                    Type::Readonly {
                        target_type: local_inner,
                    },
                    node_id,
                )
            }
            Type::KeyOf { target_type: right } => {
                let inner_ty = remote_types.get_type(*right);
                let local_inner =
                    self.import_remote_type_for_node(node_id, inner_ty, remote_types, types);
                types.insert_imported_type_from_any(
                    Type::KeyOf {
                        target_type: local_inner,
                    },
                    node_id,
                )
            }
            Type::Must { target_type: right } => {
                let inner_ty = remote_types.get_type(*right);
                let local_inner =
                    self.import_remote_type_for_node(node_id, inner_ty, remote_types, types);
                types.insert_imported_type_from_any(
                    Type::Must {
                        target_type: local_inner,
                    },
                    node_id,
                )
            }
            Type::AsComptime { target_type: right } => {
                let inner_ty = remote_types.get_type(*right);
                let local_inner =
                    self.import_remote_type_for_node(node_id, inner_ty, remote_types, types);
                types.insert_imported_type_from_any(
                    Type::AsComptime {
                        target_type: local_inner,
                    },
                    node_id,
                )
            }
            Type::Not { target_type: right } => {
                let inner_ty = remote_types.get_type(*right);
                let local_inner =
                    self.import_remote_type_for_node(node_id, inner_ty, remote_types, types);
                types.insert_imported_type_from_any(
                    Type::Not {
                        target_type: local_inner,
                    },
                    node_id,
                )
            }
            Type::In { left, right } => {
                let left_ty = remote_types.get_type(*left);
                let right_ty = remote_types.get_type(*right);
                let local_left =
                    self.import_remote_type_for_node(node_id, left_ty, remote_types, types);
                let local_right =
                    self.import_remote_type_for_node(node_id, right_ty, remote_types, types);
                types.insert_imported_type_from_any(
                    Type::In {
                        left: local_left,
                        right: local_right,
                    },
                    node_id,
                )
            }
            Type::Extends { left, right } => {
                let left_ty = remote_types.get_type(*left);
                let right_ty = remote_types.get_type(*right);
                let local_left =
                    self.import_remote_type_for_node(node_id, left_ty, remote_types, types);
                let local_right =
                    self.import_remote_type_for_node(node_id, right_ty, remote_types, types);
                types.insert_imported_type_from_any(
                    Type::Extends {
                        left: local_left,
                        right: local_right,
                    },
                    node_id,
                )
            }
            Type::Implements { left, right } => {
                let left_ty = remote_types.get_type(*left);
                let right_ty = remote_types.get_type(*right);
                let local_left =
                    self.import_remote_type_for_node(node_id, left_ty, remote_types, types);
                let local_right =
                    self.import_remote_type_for_node(node_id, right_ty, remote_types, types);
                types.insert_imported_type_from_any(
                    Type::Implements {
                        left: local_left,
                        right: local_right,
                    },
                    node_id,
                )
            }

            // nominal/reference types: keep as Type::Reference to the original symbol
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                // map embedded type ids inside static arguments
                let local_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.import_remote_static_argument_for_node(
                                node_id,
                                argument,
                                remote_types,
                                types,
                            )
                        })
                        .collect::<Vec<_>>()
                });
                types.insert_imported_type_from_any(
                    Type::Reference {
                        symbol: *symbol,
                        static_arguments: local_arguments,
                    },
                    node_id,
                )
            }

            // unevaluated remote types are invalid at this boundary: fail closed
            Type::Unevaluated(_) => types.insert_imported_type_from_any(Type::Error, node_id),
            // import fixed arrays by recursively importing the count type
            Type::ArraySized {
                element,
                count,
                is_readonly,
            } => {
                let local_element = self.import_remote_type_for_node(
                    node_id,
                    remote_types.get_type(*element),
                    remote_types,
                    types,
                );
                let local_count = self.import_remote_type_for_node(
                    node_id,
                    remote_types.get_type(*count),
                    remote_types,
                    types,
                );

                types.insert_imported_type_from_any(
                    Type::ArraySized {
                        element: local_element,
                        count: local_count,
                        is_readonly: *is_readonly,
                    },
                    node_id,
                )
            }
        }
    }

    /// Import a static argument from a remote module into the local type table.
    fn import_remote_static_argument_for_node(
        &self,
        node_id: LocalNodeIdAny,
        argument: &StaticArgument,
        remote_types: &TypeTable,
        types: &mut TypeTable,
    ) -> StaticArgument {
        match argument {
            StaticArgument::Unevaluated { .. } => argument.clone(),
            StaticArgument::Evaluated { name, value } => {
                let mapped_value = self.import_remote_static_expression_for_node(
                    node_id,
                    value,
                    remote_types,
                    types,
                );
                StaticArgument::Evaluated {
                    name: *name,
                    value: mapped_value,
                }
            }
        }
    }

    /// Import a static expression from a remote module into the local type table.
    pub(crate) fn import_remote_static_expression_for_node(
        &self,
        node_id: LocalNodeIdAny,
        expression: &StaticExpression,
        remote_types: &TypeTable,
        types: &mut TypeTable,
    ) -> StaticExpression {
        match expression {
            StaticExpression::Unevaluated { .. } => expression.clone(),
            StaticExpression::ScalarLiteral { .. } => expression.clone(),
            StaticExpression::TypeLiteral { .. } => expression.clone(),
            StaticExpression::Type { ty } => {
                let remote_ty = remote_types.get_type(*ty);
                let local_ty =
                    self.import_remote_type_for_node(node_id, remote_ty, remote_types, types);
                StaticExpression::Type { ty: local_ty }
            }
            StaticExpression::Declaration {
                declaration,
                static_arguments,
            } => {
                // remap static arguments for declarations
                let local_arguments = static_arguments.as_ref().map(|arguments| {
                    arguments
                        .iter()
                        .map(|argument| {
                            self.import_remote_static_argument_for_node(
                                node_id,
                                argument,
                                remote_types,
                                types,
                            )
                        })
                        .collect::<Vec<_>>()
                });
                StaticExpression::Declaration {
                    declaration: *declaration,
                    static_arguments: local_arguments,
                }
            }
            StaticExpression::ArrayExpression { elements } => {
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        self.import_remote_static_expression_for_node(
                            node_id,
                            element,
                            remote_types,
                            types,
                        )
                    })
                    .collect();
                StaticExpression::ArrayExpression {
                    elements: mapped_elements,
                }
            }
            StaticExpression::TupleExpression { elements } => {
                let mapped_elements = elements
                    .iter()
                    .map(|element| {
                        self.import_remote_static_expression_for_node(
                            node_id,
                            element,
                            remote_types,
                            types,
                        )
                    })
                    .collect();
                StaticExpression::TupleExpression {
                    elements: mapped_elements,
                }
            }
            StaticExpression::ObjectExpression { properties } => {
                let mapped_properties = properties
                    .iter()
                    .map(|property| {
                        self.import_remote_static_property_for_node(
                            node_id,
                            property,
                            remote_types,
                            types,
                        )
                    })
                    .collect();
                StaticExpression::ObjectExpression {
                    properties: mapped_properties,
                }
            }
        }
    }

    /// Import a static property from a remote module into the local type table.
    fn import_remote_static_property_for_node(
        &self,
        node_id: LocalNodeIdAny,
        property: &StaticProperty,
        remote_types: &TypeTable,
        types: &mut TypeTable,
    ) -> StaticProperty {
        match property {
            StaticProperty::Unevaluated { .. } => property.clone(),
            StaticProperty::Field { key, value, symbol } => {
                let mapped_value = self.import_remote_static_expression_for_node(
                    node_id,
                    value,
                    remote_types,
                    types,
                );
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
                let mapped_body = self.import_remote_static_expression_for_node(
                    node_id,
                    body,
                    remote_types,
                    types,
                );
                StaticProperty::Method {
                    key: *key,
                    signature: signature.clone(),
                    body: mapped_body,
                    symbol: *symbol,
                }
            }
            StaticProperty::Spread { value, symbol } => {
                let mapped_value = self.import_remote_static_expression_for_node(
                    node_id,
                    value,
                    remote_types,
                    types,
                );
                StaticProperty::Spread {
                    value: mapped_value,
                    symbol: *symbol,
                }
            }
        }
    }
}
