use {destack_dir as dir, destack_mir as mir};

use crate::{FieldLayout, FieldLayoutKind, LowerError, LowerResult, StructLayout};

use crate::lower::FunctionLowerer;

impl FunctionLowerer<'_> {
    /// Lower a tuple expression to an aggregate value.
    ///
    /// ```ds
    /// function make(): (int32, boolean) {
    ///     return (1, true);
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: int32 = const 1
    /// v2: bool = bconst true
    /// v3: (int32, bool) = tuple (int32, bool) (v1, v2)
    /// ```
    pub(crate) fn lower_tuple_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        elements: &[dir::LocalNodeId<dir::Argument>],
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // get the tuple type
        let tuple_type = self.lower_type_for_expression(expression_id)?;

        // lower each element value
        let mut element_values = Vec::with_capacity(elements.len());
        for element_id in elements {
            let element = self.context.dir_tree.get(*element_id);
            match element {
                dir::Argument::Positional { value, .. } | dir::Argument::Labeled { value, .. } => {
                    let (value, _) = self.lower_value_expression(*value)?;
                    element_values.push(value);
                }
                _ => {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                        message: "unsupported tuple element kind".to_string(),
                    })?;
                }
            }
        }

        // construct the tuple
        let value = self.state.builder.tuple(tuple_type, element_values);
        Ok((value, tuple_type))
    }

    /// Lower an array expression to an array value.
    ///
    /// ```ds
    /// function make(): int32[2] {
    ///     return [1, 2];
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: int32 = const 1
    /// v2: int32 = const 2
    /// v3: int32[2] = array int32[2] (v1, v2)
    /// ```
    pub(crate) fn lower_array_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        elements: &[dir::LocalNodeId<dir::Argument>],
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // get the array type
        let array_type = self.lower_type_for_expression(expression_id)?;

        // lower each element value
        let mut element_values = Vec::with_capacity(elements.len());
        for element_id in elements {
            let element = self.context.dir_tree.get(*element_id);
            match element {
                dir::Argument::Positional { value, .. } => {
                    let (value, _) = self.lower_value_expression(*value)?;
                    element_values.push(value);
                }
                _ => {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                        message: "unsupported array element kind".to_string(),
                    })?;
                }
            }
        }

        // construct the array
        let value = self.state.builder.array(array_type, element_values);
        Ok((value, array_type))
    }

    /// Lower a tagged object expression (struct literal) to a struct value.
    ///
    /// ```ds
    /// struct Vec2 { x: int32; y: int32; }
    ///
    /// function make(): Vec2 {
    ///     return Vec2 { x: 1, y: 2 };
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: int32 = const 1
    /// v2: int32 = const 2
    /// v3: Vec2 = struct Vec2 (v1, v2)
    /// ```
    pub(crate) fn lower_tagged_object_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        _ty_expr: dir::LocalNodeId<dir::TypeExpression>,
        properties: &[dir::LocalNodeId<dir::Property>],
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // get the struct type from type inference
        let struct_type = self.lower_type_for_expression(expression_id)?;

        // get the cached layout for this struct type
        let node = expression_id
            .into_global_any(self.context.module_id)
            .into_anchored(Some(self.context.profile));
        let layout = self
            .context
            .type_lowerer
            .layout_for_type_or_error(struct_type, node)?;

        // initialize field values array (one slot per field in layout order)
        let field_count = layout.fields.len();
        let mut field_values: Vec<Option<mir::Value>> = vec![None; field_count];

        // lower each property and place in correct field slot
        for property_id in properties {
            let property = self.context.dir_tree.get(*property_id);
            match property {
                dir::Property::Field { key, value, .. } => {
                    // resolve key to field index using the cached layout
                    let field_index =
                        self.resolve_property_key_to_field_index(expression_id, key, layout)?;

                    // lower the value
                    let (value, _) = self.lower_value_expression(*value)?;

                    // check for duplicate field
                    if field_values[field_index].is_some() {
                        return Err(LowerError::UnsupportedConstruct {
                            node: expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                            message: "duplicate struct field".to_string(),
                        });
                    }

                    field_values[field_index] = Some(value);
                }
                // reject spread properties
                dir::Property::Spread { .. } => {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                        message: "spread properties not yet supported".to_string(),
                    });
                }
                // reject method properties
                dir::Property::Method { .. } => {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                        message: "method properties in object literals not supported".to_string(),
                    });
                }
                dir::Property::Error { .. } => {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                        message: "error properties in object literals not supported".to_string(),
                    });
                }
            }
        }

        // resolve class symbols for vtable header defaults
        let class_symbol = self
            .type_for_expression(expression_id)
            .and_then(|type_id| self.class_symbol_for_type(type_id));

        // fill synthetic fields with default values
        let mut vtable_value = None;
        for (index, field) in layout.fields.iter().enumerate() {
            // fill synthetic fields that are not initialized
            if field.kind != FieldLayoutKind::Source && field_values[index].is_none() {
                let value =
                    self.default_field_value(field, class_symbol, node, &mut vtable_value)?;
                field_values[index] = Some(value);
            }
        }

        // check all fields are initialized
        let values: Vec<mir::Value> = field_values
            .into_iter()
            .enumerate()
            .map(|(i, v)| {
                v.ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: format!("struct field {i} not initialized"),
                })
            })
            .collect::<LowerResult<Vec<_>>>()?;

        // construct the struct
        let value = self.state.builder.struct_(struct_type, values);
        Ok((value, struct_type))
    }

    /// Lower a tagged scalar expression (newtype constructor) to a newtype value.
    ///
    /// ```ds
    /// newtype UserId = int32;
    ///
    /// function make(value: int32): UserId {
    ///     return UserId(value);
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v0: int32 = ...
    /// v1: newtype<int32> = cast.bit v0 -> newtype<int32>
    /// ```
    pub(crate) fn lower_tagged_scalar_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        _ty_expr: dir::LocalNodeId<dir::TypeExpression>,
        value_id: dir::LocalNodeId<dir::Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // get the newtype result type from type inference
        let newtype_type = self.lower_type_for_expression(expression_id)?;
        let node = expression_id
            .into_global_any(self.context.module_id)
            .into_anchored(Some(self.context.profile));

        // require a nominal newtype wrapper
        let inner_type = match self.state.builder.tree().get(newtype_type) {
            mir::Type::Newtype { inner, .. } => {
                inner.ty().ok_or_else(|| LowerError::UnsupportedConstruct {
                    node,
                    message: "tagged scalar inner type is not concrete".to_string(),
                })?
            }
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node,
                    message: "tagged scalar expression requires a newtype".to_string(),
                });
            }
        };

        // reject tuple payloads for scalar constructors
        if matches!(
            self.state.builder.tree().get(inner_type),
            mir::Type::Tuple { .. }
        ) {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
                message: "tuple newtypes require a tagged tuple expression".to_string(),
            });
        }

        // lower the payload value
        let (value, value_type) = self.lower_value_expression(value_id)?;

        // require the payload type to match the newtype inner type
        if value_type != inner_type {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
                message: "newtype payload type does not match inner type".to_string(),
            });
        }

        // wrap the payload value
        let wrapped_value = self.state.builder.bitcast(value, newtype_type);
        Ok((wrapped_value, newtype_type))
    }

    /// Lower a tagged tuple expression (tuple newtype constructor) to a newtype value.
    ///
    /// ```ds
    /// newtype Pair = (int32, int32);
    ///
    /// function make(x: int32, y: int32): Pair {
    ///     return Pair(x, y);
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v0: int32 = ...
    /// v1: int32 = ...
    /// v2: (int32, int32) = tuple (int32, int32) (v0, v1)
    /// v3: newtype<(int32, int32)> = cast.bit v2 -> newtype<(int32, int32)>
    /// ```
    pub(crate) fn lower_tagged_tuple_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        _ty_expr: dir::LocalNodeId<dir::TypeExpression>,
        elements: &[dir::LocalNodeId<dir::Argument>],
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // get the newtype result type from type inference
        let newtype_type = self.lower_type_for_expression(expression_id)?;
        let node = expression_id
            .into_global_any(self.context.module_id)
            .into_anchored(Some(self.context.profile));

        // require a nominal newtype wrapper
        let inner_type = match self.state.builder.tree().get(newtype_type) {
            mir::Type::Newtype { inner, .. } => {
                inner.ty().ok_or_else(|| LowerError::UnsupportedConstruct {
                    node,
                    message: "tagged tuple inner type is not concrete".to_string(),
                })?
            }
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node,
                    message: "tagged tuple expression requires a newtype".to_string(),
                });
            }
        };

        // require a tuple payload for tuple constructors
        let tuple_elements = match self.state.builder.tree().get(inner_type) {
            mir::Type::Tuple { elements, .. } => elements.clone(),
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: "tagged tuple expression requires a tuple newtype".to_string(),
                });
            }
        };

        // lower each element value
        let mut element_values = Vec::with_capacity(elements.len());
        for element_id in elements {
            let element = self.context.dir_tree.get(*element_id);
            match element {
                dir::Argument::Positional { value, .. } | dir::Argument::Labeled { value, .. } => {
                    let (value, _) = self.lower_value_expression(*value)?;
                    element_values.push(value);
                }
                _ => {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                        message: "unsupported tuple newtype element kind".to_string(),
                    })?;
                }
            }
        }

        // require matching tuple arity
        if element_values.len() != tuple_elements.len() {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
                message: "tuple newtype arity does not match inner type".to_string(),
            });
        }

        // construct the tuple payload
        let tuple_value = self.state.builder.tuple(inner_type, element_values);

        // wrap the tuple payload
        let wrapped_value = self.state.builder.bitcast(tuple_value, newtype_type);
        Ok((wrapped_value, newtype_type))
    }

    /// Lower a constructor call expression to a struct value.
    ///
    /// ```ds
    /// class User {
    ///     id: int32;
    ///     constructor(id: int32) { this.id = id; }
    /// }
    ///
    /// function make(): User {
    ///     return new User(1);
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: ref<User, managed, readonly> = new User
    /// v2: void = call User.constructor(v1, v0)
    /// ```
    pub(crate) fn lower_new_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // reject generic arguments for now
        if !generic_arguments.is_empty() {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
                message: "generic arguments are not supported".to_string(),
            });
        }

        // call explicit constructors when present
        if let Some(constructor_id) = self.explicit_constructor_member_for_expression(expression_id)
        {
            // load the constructor member
            let constructor = self.context.dir_tree.get(constructor_id);
            // require a constructor method member
            let dir::Member::Method { symbol, .. } = constructor else {
                return Err(LowerError::UnsupportedConstruct {
                    node: constructor_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: "unsupported constructor member".to_string(),
                });
            };

            let constructor_symbol = symbol.into_global(self.context.module_id);
            let function_id = self
                .function_for_symbol(constructor_symbol)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: constructor_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: "missing constructor function".to_string(),
                })?;

            // resolve the call signature
            let signature = self.signature_type_for_function(expression_id, function_id)?;

            // lower constructor arguments
            let mut argument_values = Vec::with_capacity(arguments.len());
            for argument_id in arguments {
                let argument = self.context.dir_tree.get(*argument_id);
                // reject non positional constructor arguments
                if !matches!(argument, dir::Argument::Positional { .. }) {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                        message: "unsupported non positional constructor argument".to_string(),
                    });
                }
                let (value, _) = self.lower_value_expression(argument.value())?;
                argument_values.push(value);
            }

            // emit the constructor call
            let value = self
                .state
                .builder
                .call(function_id, signature, argument_values)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: "constructor returned no value".to_string(),
                })?;

            // return the constructor value as-is
            let constructor_return_type = self
                .state
                .builder
                .tree()
                .get(function_id)
                .return_type
                .ty()
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: "constructor return type is not concrete".to_string(),
                })?;
            return Ok((value, constructor_return_type));
        }

        // get the instance type from type inference
        let result_type = self.lower_type_for_expression(expression_id)?;
        let (instance_type, reference_kind) = match self.state.builder.tree().get(result_type) {
            mir::Type::Reference { kind, pointee, .. } => (
                pointee
                    .ty()
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                        message: "constructor instance pointee is not concrete".to_string(),
                    })?,
                Some(*kind),
            ),
            _ => (result_type, None),
        };

        // get the cached layout for this struct type
        let layout = self.context.type_lowerer.layout_for_type_or_error(
            instance_type,
            expression_id
                .into_global_any(self.context.module_id)
                .into_anchored(Some(self.context.profile)),
        )?;

        // match arguments to fields in source order
        let field_count = layout
            .fields
            .iter()
            .filter(|field| field.kind == FieldLayoutKind::Source)
            .count();

        // reject mismatched argument counts
        if arguments.len() != field_count {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
                message: "constructor argument count does not match field count".to_string(),
            });
        }

        // initialize field values array in layout order
        let mut field_values: Vec<Option<mir::Value>> = vec![None; layout.fields.len()];
        for (source_index, argument_id) in arguments.iter().enumerate() {
            let argument = self.context.dir_tree.get(*argument_id);
            // reject non positional constructor arguments
            let dir::Argument::Positional { value, .. } = argument else {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: "unsupported non positional constructor argument".to_string(),
                });
            };

            // lower the argument value
            let (value, _) = self.lower_value_expression(*value)?;

            // map source index to layout index
            let layout_index = layout
                .field_index_by_source(source_index as u32)
                .map(|index| index as usize)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: "constructor field index out of bounds".to_string(),
                })?;

            // check for duplicate field assignment
            if field_values[layout_index].is_some() {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: "duplicate constructor field".to_string(),
                });
            }

            field_values[layout_index] = Some(value);
        }

        // resolve class symbols for vtable header defaults
        let class_symbol = self
            .type_for_expression(expression_id)
            .and_then(|type_id| self.class_symbol_for_type(type_id));

        // fill synthetic fields with default values
        let node = expression_id
            .into_global_any(self.context.module_id)
            .into_anchored(Some(self.context.profile));
        let mut vtable_value = None;
        for (index, field) in layout.fields.iter().enumerate() {
            // fill synthetic fields that are not initialized
            if field.kind != FieldLayoutKind::Source && field_values[index].is_none() {
                let value =
                    self.default_field_value(field, class_symbol, node, &mut vtable_value)?;
                field_values[index] = Some(value);
            }
        }

        // check all fields are initialized
        let values: Vec<mir::Value> = field_values
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                value.ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: format!("constructor field {index} not initialized"),
                })
            })
            .collect::<LowerResult<Vec<_>>>()?;

        // construct the struct
        let instance_value = self.state.builder.struct_(instance_type, values);
        // allocate when returning a typed reference
        let value = match reference_kind {
            Some(mir::ReferenceKind::Managed | mir::ReferenceKind::Owned) => {
                let pointer = self.state.builder.new_(instance_type, result_type);
                self.state.builder.store(pointer, instance_value);
                pointer
            }
            // reject unsupported reference kinds
            Some(_) => {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: "unsupported reference kind for constructor".to_string(),
                });
            }
            // return by value when no reference wrapper is present
            None => instance_value,
        };
        Ok((value, result_type))
    }

    /// Build a default value for a struct layout with vtable headers populated.
    pub(crate) fn default_struct_value_for_layout(
        &mut self,
        struct_type: mir::LocalNodeId<mir::Type>,
        layout: &StructLayout,
        class_symbol: Option<dir::GlobalSymbolId>,
        node: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<mir::Value> {
        // initialize values in layout order
        let mut values = Vec::with_capacity(layout.fields.len());
        let mut vtable_value = None;
        for field in &layout.fields {
            let value = self.default_field_value(field, class_symbol, node, &mut vtable_value)?;
            values.push(value);
        }

        Ok(self.state.builder.struct_(struct_type, values))
    }

    /// Build the default value for a layout field.
    fn default_field_value(
        &mut self,
        field: &FieldLayout,
        class_symbol: Option<dir::GlobalSymbolId>,
        node: dir::AnchoredGlobalNodeId,
        vtable_value: &mut Option<mir::Value>,
    ) -> LowerResult<mir::Value> {
        // use the vtable pointer for header fields
        // NOTE #Architecture: it feels a bit wonky to have vtable_header_value in default_field_value?
        if field.kind == FieldLayoutKind::VtableHeader {
            self.vtable_header_value(field, class_symbol, node, vtable_value)
        }
        // fall back to type zero values
        else {
            self.zero_value_for_type(field.ty, node)
        }
    }

    /// Build the default value for a vtable header field.
    fn vtable_header_value(
        &mut self,
        field: &FieldLayout,
        class_symbol: Option<dir::GlobalSymbolId>,
        node: dir::AnchoredGlobalNodeId,
        vtable_value: &mut Option<mir::Value>,
    ) -> LowerResult<mir::Value> {
        let class_symbol = class_symbol.ok_or_else(|| LowerError::UnsupportedConstruct {
            node,
            message: "vtable field missing class symbol".to_string(),
        })?;

        // reuse the cached vtable pointer when available
        if let Some(value) = vtable_value {
            return Ok(*value);
        }

        // compute and cache the vtable pointer
        let value = self.vtable_pointer_for_class(class_symbol, node, field.ty)?;
        *vtable_value = Some(value);
        Ok(value)
    }

    /// Resolve a property key to a field index in the struct using the cached layout.
    fn resolve_property_key_to_field_index(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        key: &dir::Key,
        layout: &StructLayout,
    ) -> LowerResult<usize> {
        match key {
            dir::Key::Name(name) => layout
                .field_index(name.string())
                .map(|i| i as usize)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: "struct field not found".to_string(),
                }),
            dir::Key::Private(_) => Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
                message: "private field key not supported in struct layout".to_string(),
            }),
            dir::Key::Expression(_) => Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
                message: "computed property keys not supported".to_string(),
            }),
        }
    }

    /// Find an explicit constructor member for the expression type when present.
    fn explicit_constructor_member_for_expression(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalNodeId<dir::Member>> {
        // prefer the constructor symbol from the new expression target
        if let Some(target_symbol) = self.constructor_target_symbol_for_expression(expression_id)
            && target_symbol.module_id == self.context.module_id
            && let Some(constructor) = self.explicit_constructor_member_for_symbol(target_symbol)
        {
            return Some(constructor);
        }

        // resolve the nominal symbol for constructor lookup
        let type_id = self.type_for_expression(expression_id)?;
        let type_id = self.unwrap_value_type_id(type_id);
        // require a nominal reference type
        let symbol = match self.context.types.get_type(type_id) {
            dir::Type::Reference { symbol, .. } => *symbol,
            _ => return None,
        };

        // skip remote symbols
        if symbol.module_id != self.context.module_id {
            return None;
        }

        self.explicit_constructor_member_for_symbol(symbol)
    }

    /// Resolve the target symbol for a new expression when it is a simple reference.
    fn constructor_target_symbol_for_expression(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalSymbolId> {
        let dir::Expression::New { left, .. } = self.context.dir_tree.get(expression_id) else {
            return None;
        };

        match self.context.dir_tree.get(*left) {
            dir::Expression::LocalReference { target_symbol, .. }
            | dir::Expression::ModuleReference { target_symbol, .. }
            | dir::Expression::GlobalReference { target_symbol, .. } => Some(*target_symbol),
            _ => None,
        }
    }

    /// Find explicit constructor members for a nominal symbol.
    fn explicit_constructor_member_for_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::LocalNodeId<dir::Member>> {
        // scan declarations for explicit constructors
        let declaration_ids = self.declaration_ids_for_symbol(symbol);
        for declaration_id in declaration_ids {
            let declaration = self.context.dir_tree.get(declaration_id);
            // select struct or class members
            let members = match declaration {
                dir::Declaration::Struct(declaration) => &declaration.members,
                dir::Declaration::Class(declaration) => &declaration.members,
                _ => continue,
            };

            for member_id in members {
                let member = self.context.dir_tree.get(*member_id);
                let dir::Member::Method { key, signature, .. } = member else {
                    continue;
                };

                // constructors are nameless methods with constructor or new mode
                if key.is_none()
                    && matches!(
                        signature.mode,
                        Some(dir::FunctionMode::Constructor | dir::FunctionMode::New)
                    )
                {
                    return Some(*member_id);
                }
            }
        }

        None
    }

    /// Collect declaration ids that belong to a symbol in this module.
    fn declaration_ids_for_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Vec<dir::LocalNodeId<dir::Declaration>> {
        // read the symbol entry for declaration lists
        let symbol_entry = self.context.symbols.get_symbol(symbol.local_id);
        let mut declaration_ids = Vec::new();

        // add the primary declaration first
        if let Some(primary) = symbol_entry.primary_declaration
            && primary.module_id == self.context.module_id
            && let Ok(local_id) = primary.local_id.try_into_typed::<dir::Declaration>()
        {
            declaration_ids.push(local_id);
        }

        // add secondary declarations in order
        if let Some(secondary) = symbol_entry.secondary_declarations.as_deref() {
            for declaration_id in secondary {
                if declaration_id.module_id != self.context.module_id {
                    continue;
                }
                if let Ok(local_id) = declaration_id.local_id.try_into_typed::<dir::Declaration>() {
                    declaration_ids.push(local_id);
                }
            }
        }

        declaration_ids
    }
}
