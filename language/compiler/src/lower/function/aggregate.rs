use {destack_dir as dir, destack_mir as mir};

use crate::{
    CompilerError, CompilerResult, FieldLayout, FieldLayoutKind, LowerError, StructLayout,
};

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
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
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
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "unsupported tuple element kind".to_string(),
                    }
                    .into());
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
    /// function make(): [int32; 2] {
    ///     return [1, 2];
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: int32 = const 1
    /// v2: int32 = const 2
    /// v3: [int32; 2] = array [int32; 2] (v1, v2)
    /// ```
    pub(crate) fn lower_array_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        elements: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
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
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "unsupported array element kind".to_string(),
                    }
                    .into());
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
        ty_expr: dir::LocalNodeId<dir::TypeExpression>,
        properties: &[dir::LocalNodeId<dir::Property>],
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // get the struct type from explicit syntax
        let struct_type = self.lower_type_for_type_expression(ty_expr)?;

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
                            anchor: self.diagnostic_anchor(
                                expression_id
                                    .into_global_any(self.context.module_id)
                                    .into_anchored(Some(self.context.profile)),
                            ),
                            message: "duplicate struct field".to_string(),
                        }
                        .into());
                    }

                    field_values[field_index] = Some(value);
                }
                // reject spread properties
                dir::Property::Spread { .. } => {
                    return Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "spread properties not yet supported".to_string(),
                    }
                    .into());
                }
                // reject method properties
                dir::Property::Method { .. } => {
                    return Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "method properties in object literals not supported".to_string(),
                    }
                    .into());
                }
                dir::Property::Error { .. } => {
                    return Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "error properties in object literals not supported".to_string(),
                    }
                    .into());
                }
            }
        }

        // resolve class symbols for vtable header defaults
        let class_symbol = self
            .type_id_for_type_expression(ty_expr)
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
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: format!("struct field {i} not initialized"),
                })
                .map_err(CompilerError::from)
            })
            .collect::<CompilerResult<Vec<_>>>()?;

        // construct the struct
        let value = self.state.builder.struct_(struct_type, values);
        Ok((value, struct_type))
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
    /// v1: ref<User, managed, readonly> = new.zeroed User
    /// v2: void = call User.constructor(v1, v0)
    /// ```
    pub(crate) fn lower_new_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let construct_target = self.class_construct_target_for_new_expression(expression_id)?;

        // call explicit constructors when present
        if let Some(constructor_symbol) = construct_target.constructor {
            let function_id = self
                .function_for_symbol(constructor_symbol)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: "missing constructor function".to_string(),
                })
                .map_err(CompilerError::from)?;

            // resolve the call signature
            let signature = self.signature_type_for_function(expression_id, function_id)?;

            // lower constructor arguments
            let mut argument_values = Vec::with_capacity(arguments.len());
            for argument_id in arguments {
                let argument = self.context.dir_tree.get(*argument_id);
                // reject non positional constructor arguments
                if !matches!(argument, dir::Argument::Positional { .. }) {
                    return Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "unsupported non positional constructor argument".to_string(),
                    }
                    .into());
                }
                let argument_value = self.require_argument_value(expression_id, argument)?;
                let (value, _) = self.lower_value_expression(argument_value)?;
                argument_values.push(value);
            }

            // emit the constructor call
            let value = self
                .state
                .builder
                .call(function_id, signature, argument_values)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: "constructor returned no value".to_string(),
                })
                .map_err(CompilerError::from)?;

            // return the constructor value as-is
            let constructor_return_type = self
                .state
                .builder
                .tree()
                .get(function_id)
                .return_type
                .ty()
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: "constructor return type is not concrete".to_string(),
                })
                .map_err(CompilerError::from)?;
            return Ok((value, constructor_return_type));
        }

        // get the result type from the checked constructor target
        let result_type = if let Some(reference_type_id) =
            self.nominal_reference_type_id_for_symbol(construct_target.symbol)
        {
            let node = expression_id.into_global_any(self.context.module_id);
            self.lower_type_id_for_node(reference_type_id, node)?
        } else {
            self.lower_type_for_expression(expression_id)?
        };
        let (instance_type, reference_kind) = match self.state.builder.tree().get(result_type) {
            mir::Type::Reference { kind, pointee, .. } => (
                pointee
                    .ty()
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "constructor instance pointee is not concrete".to_string(),
                    })
                    .map_err(CompilerError::from)?,
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
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "constructor argument count does not match field count".to_string(),
            }
            .into());
        }

        // initialize field values array in layout order
        let mut field_values: Vec<Option<mir::Value>> = vec![None; layout.fields.len()];
        for (source_index, argument_id) in arguments.iter().enumerate() {
            let argument = self.context.dir_tree.get(*argument_id);
            // reject non positional constructor arguments
            let dir::Argument::Positional { value, .. } = argument else {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: "unsupported non positional constructor argument".to_string(),
                }
                .into());
            };

            // lower the argument value
            let (value, _) = self.lower_value_expression(*value)?;

            // map source index to layout index
            let layout_index = layout
                .field_index_by_source(source_index as u32)
                .map(|index| index as usize)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: "constructor field index out of bounds".to_string(),
                })
                .map_err(CompilerError::from)?;

            // check for duplicate field assignment
            if field_values[layout_index].is_some() {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: "duplicate constructor field".to_string(),
                }
                .into());
            }

            field_values[layout_index] = Some(value);
        }

        // resolve class symbols for vtable header defaults
        let class_symbol = constructor_target
            .filter(|symbol| self.context.symbol_kind_matches(*symbol, dir::SymbolKind::Class))
            .or_else(|| {
                self.type_for_expression(expression_id)
                    .and_then(|type_id| self.class_symbol_for_type(type_id))
            });

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
                value
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: format!("constructor field {index} not initialized"),
                    })
                    .map_err(CompilerError::from)
            })
            .collect::<CompilerResult<Vec<_>>>()?;

        // construct the struct
        let instance_value = self.state.builder.struct_(instance_type, values);
        // allocate when returning a typed reference
        let value = match reference_kind {
            Some(mir::ReferenceKind::Managed | mir::ReferenceKind::Unique) => {
                let pointer = self.state.builder.new_zeroed(instance_type, result_type);
                self.state.builder.store(pointer, instance_value);
                pointer
            }
            // reject unsupported reference kinds
            Some(_) => {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: "unsupported reference kind for constructor".to_string(),
                }
                .into());
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
    ) -> CompilerResult<mir::Value> {
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
    ) -> CompilerResult<mir::Value> {
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
    ) -> CompilerResult<mir::Value> {
        let class_symbol = class_symbol
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(node),
                message: "vtable field missing class symbol".to_string(),
            })
            .map_err(CompilerError::from)?;

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
    ) -> CompilerResult<usize> {
        match key {
            dir::Key::Name(name) => layout
                .field_index(name.string())
                .map(|i| i as usize)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: "struct field not found".to_string(),
                })
                .map_err(CompilerError::from),
            dir::Key::Private(_) => Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "private field key not supported in struct layout".to_string(),
            }
            .into()),
            dir::Key::Expression(_) => Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "computed property keys not supported".to_string(),
            }
            .into()),
        }
    }

    /// Return the checked class construct target for a new expression.
    fn class_construct_target_for_new_expression(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::ClassConstructCandidate> {
        let Some(resolution) = self.get_construct_resolution(expression_id) else {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "new expression missing DIR construct resolution".to_string(),
            }
            .into());
        };
        let dir::ConstructTarget::Class(candidate) = &resolution.target else {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "new expression construct resolution must be a class".to_string(),
            }
            .into());
        };

        Ok(candidate.clone())
    }

    /// Find a nominal reference type id for a symbol in checked type state.
    fn nominal_reference_type_id_for_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::LocalTypeId> {
        // scan type ids in deterministic insertion order
        let type_count = self.context.types.type_count();
        for index in 0..type_count {
            let type_id = dir::LocalTypeId::new(index);
            let dir_type = self.context.types.get_type(type_id);
            if let dir::Type::Reference(reference) = dir_type
                && reference.symbol == symbol
            {
                return Some(type_id);
            }
        }

        None
    }

}
