use destack_dir::{Expression, LocalNodeId};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult, StructLayout};

use crate::lower::emit::FunctionContext;

impl FunctionContext<'_> {
    /// Lower a tuple expression to an aggregate value.
    pub(crate) fn lower_tuple_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        elements: &[LocalNodeId<dir::Argument>],
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // get the tuple type
        let tuple_type =
            self.mir_type_for_expression(expression_id)
                .ok_or_else(|| LowerError::MissingType {
                    node: expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                })?;

        // lower each element value
        let mut element_values = Vec::with_capacity(elements.len());
        for element_id in elements {
            let element = self.dir_tree.get(*element_id);
            match element {
                dir::Argument::Positional { value } | dir::Argument::Labeled { value, .. } => {
                    let (value, _) = self.lower_value_expression(*value)?;
                    element_values.push(value);
                }
                _ => {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                        message: "unsupported tuple element kind".to_string(),
                    })?;
                }
            }
        }

        // construct the tuple
        let value = self.builder.tuple(tuple_type, element_values);
        Ok((value, tuple_type))
    }

    /// Lower an array expression to an array value.
    pub(crate) fn lower_array_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        elements: &[LocalNodeId<dir::Argument>],
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // get the array type
        let array_type =
            self.mir_type_for_expression(expression_id)
                .ok_or_else(|| LowerError::MissingType {
                    node: expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                })?;

        // lower each element value
        let mut element_values = Vec::with_capacity(elements.len());
        for element_id in elements {
            let element = self.dir_tree.get(*element_id);
            match element {
                dir::Argument::Positional { value } => {
                    let (value, _) = self.lower_value_expression(*value)?;
                    element_values.push(value);
                }
                _ => {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                        message: "unsupported array element kind".to_string(),
                    })?;
                }
            }
        }

        // construct the array
        let value = self.builder.array(array_type, element_values);
        Ok((value, array_type))
    }

    /// Lower a tagged object expression (struct literal) to a struct value.
    pub(crate) fn lower_tagged_object_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        _ty_expr: LocalNodeId<Expression>,
        properties: &[LocalNodeId<dir::Property>],
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // get the struct type from type inference
        let struct_type =
            self.mir_type_for_expression(expression_id)
                .ok_or_else(|| LowerError::MissingType {
                    node: expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                })?;

        // get the cached layout for this struct type
        let layout = self
            .type_lowerer
            .layout_for_type(struct_type)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: "missing struct layout".to_string(),
            })?;

        // initialize field values array (one slot per field in layout order)
        let field_count = layout.fields.len();
        let mut field_values: Vec<Option<mir::Value>> = vec![None; field_count];

        // lower each property and place in correct field slot
        for property_id in properties {
            let property = self.dir_tree.get(*property_id);
            match property {
                dir::Property::Field { key, value, .. } => {
                    // get the property key
                    let key = key
                        .as_ref()
                        .ok_or_else(|| LowerError::UnsupportedConstruct {
                            node: expression_id
                                .into_global_any(self.module_id)
                                .into_anchored(Some(self.profile)),
                            message: "struct field missing key".to_string(),
                        })?;

                    // resolve key to field index using the cached layout
                    let field_index =
                        self.resolve_property_key_to_field_index(expression_id, key, layout)?;

                    // get the initializer value
                    let value_expr = value.ok_or_else(|| LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                        message: "struct field missing initializer".to_string(),
                    })?;

                    // lower the value
                    let (value, _) = self.lower_value_expression(value_expr)?;

                    // check for duplicate field
                    if field_values[field_index].is_some() {
                        return Err(LowerError::UnsupportedConstruct {
                            node: expression_id
                                .into_global_any(self.module_id)
                                .into_anchored(Some(self.profile)),
                            message: "duplicate struct field".to_string(),
                        });
                    }

                    field_values[field_index] = Some(value);
                }
                dir::Property::Spread { .. } => {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                        message: "spread properties not yet supported".to_string(),
                    });
                }
                dir::Property::Method { .. } => {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                        message: "method properties in object literals not supported".to_string(),
                    });
                }
            }
        }

        // check all fields are initialized
        let values: Vec<mir::Value> = field_values
            .into_iter()
            .enumerate()
            .map(|(i, v)| {
                v.ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: format!("struct field {i} not initialized"),
                })
            })
            .collect::<LowerResult<Vec<_>>>()?;

        // construct the struct
        let value = self.builder.struct_(struct_type, values);
        Ok((value, struct_type))
    }

    /// Resolve a property key to a field index in the struct using the cached layout.
    fn resolve_property_key_to_field_index(
        &self,
        expression_id: LocalNodeId<Expression>,
        key: &dir::DynamicKey,
        layout: &StructLayout,
    ) -> LowerResult<usize> {
        match key {
            dir::DynamicKey::Name(name) => {
                // find field by name using layout's field_index method
                layout
                    .field_index(*name)
                    .map(|i| i as usize)
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                        message: "struct field not found".to_string(),
                    })
            }
            dir::DynamicKey::Number(num_str) => {
                // numeric key - parse as source index, then map to layout index
                let index_str = self.strings.get(*num_str);
                let source_index: u32 =
                    index_str
                        .parse()
                        .map_err(|_| LowerError::UnsupportedConstruct {
                            node: expression_id
                                .into_global_any(self.module_id)
                                .into_anchored(Some(self.profile)),
                            message: "invalid numeric field key".to_string(),
                        })?;
                layout
                    .field_index_by_source(source_index)
                    .map(|i| i as usize)
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                        message: "field index out of bounds".to_string(),
                    })
            }
            dir::DynamicKey::Expression(_) | dir::DynamicKey::NamedExpression { .. } => {
                Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "computed property keys not supported".to_string(),
                })
            }
        }
    }
}
