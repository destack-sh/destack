use std::collections::HashSet;

use destack_dir::{AnchoredGlobalNodeId, Expression, GlobalSymbolId, LocalNodeId};
use {destack_dir as dir, destack_mir as mir};

use crate::{FieldLayout, FieldLayoutKind, LowerError, LowerResult, StructLayout};

use crate::lower::emit::FunctionContext;

impl FunctionContext<'_> {
    /// Lower a tuple expression to an aggregate value.
    ///
    /// ```ds
    /// function make(): (int32, boolean) {
    ///     return (1, true);
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: i32 = iconst 1
    /// v2: bool = bconst true
    /// v3: (i32, bool) = tuple (i32, bool) (v1, v2)
    /// ```
    pub(crate) fn lower_tuple_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        elements: &[LocalNodeId<dir::Argument>],
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // get the tuple type
        let tuple_type = self.lower_type_for_expression(expression_id)?;

        // lower each element value
        let mut element_values = Vec::with_capacity(elements.len());
        for element_id in elements {
            let element = self.env.dir_tree.get(*element_id);
            match element {
                dir::Argument::Positional { value, .. }
                | dir::Argument::Labeled { value, .. } => {
                    let (value, _) = self.lower_value_expression(*value)?;
                    element_values.push(value);
                }
                _ => {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
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
    /// v1: i32 = iconst 1
    /// v2: i32 = iconst 2
    /// v3: [i32; 2] = array [i32; 2] (v1, v2)
    /// ```
    pub(crate) fn lower_array_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        elements: &[LocalNodeId<dir::Argument>],
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // get the array type
        let array_type = self.lower_type_for_expression(expression_id)?;

        // lower each element value
        let mut element_values = Vec::with_capacity(elements.len());
        for element_id in elements {
            let element = self.env.dir_tree.get(*element_id);
            match element {
                dir::Argument::Positional { value, .. } => {
                    let (value, _) = self.lower_value_expression(*value)?;
                    element_values.push(value);
                }
                _ => {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
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
    /// v1: i32 = iconst 1
    /// v2: i32 = iconst 2
    /// v3: @Vec2 = struct @Vec2 (v1, v2)
    /// ```
    pub(crate) fn lower_tagged_object_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        _ty_expr: LocalNodeId<Expression>,
        properties: &[LocalNodeId<dir::Property>],
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // get the struct type from type inference
        let struct_type = self.lower_type_for_expression(expression_id)?;

        // get the cached layout for this struct type
        let node = expression_id
            .into_global_any(self.env.module_id)
            .into_anchored(Some(self.env.profile));
        let layout = self
            .env
            .type_lowerer
            .layout_for_type_or_error(struct_type, node)?;

        // initialize field values array (one slot per field in layout order)
        let field_count = layout.fields.len();
        let mut field_values: Vec<Option<mir::Value>> = vec![None; field_count];

        // lower each property and place in correct field slot
        for property_id in properties {
            let property = self.env.dir_tree.get(*property_id);
            match property {
                dir::Property::Field { key, value, .. } => {
                    // get the property key
                    let key = key
                        .as_ref()
                        .ok_or_else(|| LowerError::UnsupportedConstruct {
                            node: expression_id
                                .into_global_any(self.env.module_id)
                                .into_anchored(Some(self.env.profile)),
                            message: "struct field missing key".to_string(),
                        })?;

                    // resolve key to field index using the cached layout
                    let field_index =
                        self.resolve_property_key_to_field_index(expression_id, key, layout)?;

                    // get the initializer value
                    let value_expr = value.ok_or_else(|| LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "struct field missing initializer".to_string(),
                    })?;

                    // lower the value
                    let (value, _) = self.lower_value_expression(value_expr)?;

                    // check for duplicate field
                    if field_values[field_index].is_some() {
                        return Err(LowerError::UnsupportedConstruct {
                            node: expression_id
                                .into_global_any(self.env.module_id)
                                .into_anchored(Some(self.env.profile)),
                            message: "duplicate struct field".to_string(),
                        });
                    }

                    field_values[field_index] = Some(value);
                }
                // reject spread properties
                dir::Property::Spread { .. } => {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "spread properties not yet supported".to_string(),
                    });
                }
                // reject method properties
                dir::Property::Method { .. } => {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "method properties in object literals not supported".to_string(),
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
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: format!("struct field {i} not initialized"),
                })
            })
            .collect::<LowerResult<Vec<_>>>()?;

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
    /// v1: ref<managed @User> = managed.alloc @User
    /// v2: void = call @User.constructor(v1, v0) -> fn(ref<managed @User>, i32) -> void
    /// ```
    pub(crate) fn lower_new_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        static_arguments: &Option<Vec<LocalNodeId<dir::Argument>>>,
        dynamic_arguments: &[LocalNodeId<dir::Argument>],
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // reject static arguments for now
        if static_arguments
            .as_ref()
            .is_some_and(|args| !args.is_empty())
        {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "static arguments are not supported".to_string(),
            });
        }

        // call explicit constructors when present
        if let Some(constructor_id) = self.explicit_constructor_member_for_expression(expression_id)
        {
            // load the constructor member
            let constructor = self.env.dir_tree.get(constructor_id);
            // require a constructor method member
            let dir::Member::Method { symbol, .. } = constructor else {
                return Err(LowerError::UnsupportedConstruct {
                    node: constructor_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "unsupported constructor member".to_string(),
                });
            };

            let constructor_symbol = symbol.into_global(self.env.module_id);
            let function_id = *self
                .env
                .functions_by_symbol
                .get(&constructor_symbol)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: constructor_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "missing constructor function".to_string(),
                })?;

            // resolve the call signature
            let signature = self.signature_type_for_function(expression_id, function_id)?;

            // lower constructor arguments
            let mut arguments = Vec::with_capacity(dynamic_arguments.len());
            for argument_id in dynamic_arguments {
                let argument = self.env.dir_tree.get(*argument_id);
                // reject non positional constructor arguments
                if !matches!(argument, dir::Argument::Positional { .. }) {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "unsupported non positional constructor argument".to_string(),
                    });
                }
                let (value, _) = self.lower_value_expression(argument.value())?;
                arguments.push(value);
            }

            // emit the constructor call
            let value = self
                .state
                .builder
                .call(function_id, signature, arguments)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "constructor returned no value".to_string(),
                })?;

            // return the constructor value as-is
            let constructor_return_type = self.state.builder.tree().get(function_id).return_type;
            return Ok((value, constructor_return_type));
        }

        // get the instance type from type inference
        let result_type = self.lower_type_for_expression(expression_id)?;
        let (instance_type, reference_kind) = match self.state.builder.tree().get(result_type) {
            mir::Type::Reference { kind, pointee, .. } => (*pointee, Some(*kind)),
            _ => (result_type, None),
        };

        // get the cached layout for this struct type
        let layout = self.env.type_lowerer.layout_for_type_or_error(
            instance_type,
            expression_id
                .into_global_any(self.env.module_id)
                .into_anchored(Some(self.env.profile)),
        )?;

        // match arguments to fields in source order
        let field_count = layout
            .fields
            .iter()
            .filter(|field| field.kind == FieldLayoutKind::Source)
            .count();

        // reject mismatched argument counts
        if dynamic_arguments.len() != field_count {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "constructor argument count does not match field count".to_string(),
            });
        }

        // initialize field values array in layout order
        let mut field_values: Vec<Option<mir::Value>> = vec![None; layout.fields.len()];
        for (source_index, argument_id) in dynamic_arguments.iter().enumerate() {
            let argument = self.env.dir_tree.get(*argument_id);
            // reject non positional constructor arguments
            let dir::Argument::Positional { value, .. } = argument else {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
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
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "constructor field index out of bounds".to_string(),
                })?;

            // check for duplicate field assignment
            if field_values[layout_index].is_some() {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
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
            .into_global_any(self.env.module_id)
            .into_anchored(Some(self.env.profile));
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
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: format!("constructor field {index} not initialized"),
                })
            })
            .collect::<LowerResult<Vec<_>>>()?;

        // construct the struct
        let instance_value = self.state.builder.struct_(instance_type, values);
        // allocate when returning a managed reference
        let value = match reference_kind {
            Some(mir::ReferenceKind::Managed) => {
                let pointer = self.state.builder.managed_alloc(instance_type, result_type);
                self.state.builder.store(pointer, instance_value);
                pointer
            }
            // reject unsupported reference kinds
            Some(_) => {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "unsupported reference kind for constructor".to_string(),
                });
            }
            // return by value when no reference wrapper is present
            None => instance_value,
        };
        Ok((value, result_type))
    }

    /// Build a zero value for a MIR type.
    pub(crate) fn zero_value_for_type(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        node: AnchoredGlobalNodeId,
    ) -> LowerResult<mir::Value> {
        // initialize recursion guard
        let mut visiting = HashSet::new();

        // compute the zero value
        self.zero_value_for_type_inner(ty, node, &mut visiting)
    }

    /// Build a zero value for a MIR type with a recursion guard.
    fn zero_value_for_type_inner(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        node: AnchoredGlobalNodeId,
        visiting: &mut HashSet<mir::LocalNodeId<mir::Type>>,
    ) -> LowerResult<mir::Value> {
        // guard against recursive constructor initialization
        if !visiting.insert(ty) {
            return Err(LowerError::UnsupportedConstruct {
                node,
                message: "recursive constructor initialization not supported".to_string(),
            });
        }

        // build the zero value for the requested type
        let mir_type = self.state.builder.tree().get(ty).clone();
        let value = match mir_type {
            mir::Type::Void => {
                return Err(LowerError::UnsupportedConstruct {
                    node,
                    message: "constructor cannot initialize void field".to_string(),
                });
            }
            mir::Type::Boolean => self.state.builder.bconst(false),
            mir::Type::Int { width, is_signed } => {
                let width = u8::try_from(width).map_err(|_| LowerError::UnsupportedConstruct {
                    node,
                    message: "unsupported integer width for constructor initialization".to_string(),
                })?;
                self.state.builder.iconst(0, width, is_signed)
            }
            mir::Type::Float { width } => {
                let width = u8::try_from(width).map_err(|_| LowerError::UnsupportedConstruct {
                    node,
                    message: "unsupported float width for constructor initialization".to_string(),
                })?;
                self.state.builder.fconst(0.0, width)
            }
            mir::Type::Isize | mir::Type::Usize | mir::Type::Type => {
                let pointer_bits = self.env.type_lowerer.pointer_width_bits();
                let width =
                    u8::try_from(pointer_bits).map_err(|_| LowerError::UnsupportedConstruct {
                        node,
                        message: "unsupported pointer width for constructor initialization"
                            .to_string(),
                    })?;
                let signed = matches!(mir_type, mir::Type::Isize);
                self.state.builder.iconst(0, width, signed)
            }
            mir::Type::Reference { .. } => {
                let pointer_bits = self.env.type_lowerer.pointer_bytes() * 8;
                let zero = self.state.builder.iconst(0, pointer_bits, false);
                self.state
                    .builder
                    .cast(mir::CastOperator::IntToPointer, zero, ty)
            }
            mir::Type::TensorReference { .. } => {
                let pointer_bits = self.env.type_lowerer.pointer_bytes() * 8;
                let zero = self.state.builder.iconst(0, pointer_bits, false);
                self.state
                    .builder
                    .cast(mir::CastOperator::IntToPointer, zero, ty)
            }
            mir::Type::Array {
                element, length, ..
            } => {
                let length =
                    usize::try_from(length).map_err(|_| LowerError::UnsupportedConstruct {
                        node,
                        message: "array too large for constructor initialization".to_string(),
                    })?;
                let mut elements = Vec::with_capacity(length);
                for _ in 0..length {
                    elements.push(self.zero_value_for_type_inner(element, node, visiting)?);
                }
                self.state.builder.array(ty, elements)
            }
            mir::Type::Tuple { elements, .. } => {
                let mut values = Vec::with_capacity(elements.len());
                for element in elements {
                    values.push(self.zero_value_for_type_inner(element, node, visiting)?);
                }
                self.state.builder.tuple(ty, values)
            }
            mir::Type::Struct { .. } => {
                let layout = self.env.type_lowerer.layout_for_type_or_error(ty, node)?;
                let mut values = Vec::with_capacity(layout.fields.len());
                for field in &layout.fields {
                    values.push(self.zero_value_for_type_inner(field.ty, node, visiting)?);
                }
                self.state.builder.struct_(ty, values)
            }
            mir::Type::FunctionPointer { .. } => {
                let pointer_bits = self.env.type_lowerer.pointer_bytes() * 8;
                let zero = self.state.builder.iconst(0, pointer_bits, false);
                self.state
                    .builder
                    .cast(mir::CastOperator::IntToPointer, zero, ty)
            }
            mir::Type::Vector { .. } | mir::Type::Tensor { .. } => {
                return Err(LowerError::UnsupportedConstruct {
                    node,
                    message: "constructor cannot initialize vector or tensor values".to_string(),
                });
            }
        };

        // clear recursion guard
        visiting.remove(&ty);
        Ok(value)
    }

    /// Build a default value for a struct layout with vtable headers populated.
    pub(crate) fn default_struct_value_for_layout(
        &mut self,
        struct_type: mir::LocalNodeId<mir::Type>,
        layout: &StructLayout,
        class_symbol: Option<GlobalSymbolId>,
        node: AnchoredGlobalNodeId,
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
        class_symbol: Option<GlobalSymbolId>,
        node: AnchoredGlobalNodeId,
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
        class_symbol: Option<GlobalSymbolId>,
        node: AnchoredGlobalNodeId,
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
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "struct field not found".to_string(),
                    })
            }
            dir::DynamicKey::Number(num_str) => {
                // parse numeric key as source index
                let index_str = self.env.strings.get(*num_str);
                let source_index: u32 =
                    index_str
                        .parse()
                        .map_err(|_| LowerError::UnsupportedConstruct {
                            node: expression_id
                                .into_global_any(self.env.module_id)
                                .into_anchored(Some(self.env.profile)),
                            message: "invalid numeric field key".to_string(),
                        })?;
                // map source index to layout index
                layout
                    .field_index_by_source(source_index)
                    .map(|i| i as usize)
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "field index out of bounds".to_string(),
                    })
            }
            dir::DynamicKey::Expression(_) | dir::DynamicKey::NamedExpression { .. } => {
                // reject computed property keys
                Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "computed property keys not supported".to_string(),
                })
            }
        }
    }

    /// Find an explicit constructor member for the expression type when present.
    fn explicit_constructor_member_for_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<LocalNodeId<dir::Member>> {
        // prefer the constructor symbol from the new expression target
        if let Some(target_symbol) = self.constructor_target_symbol_for_expression(expression_id)
            && target_symbol.module_id == self.env.module_id
            && let Some(constructor) = self.explicit_constructor_member_for_symbol(target_symbol)
        {
            return Some(constructor);
        }

        // resolve the nominal symbol for constructor lookup
        let type_id = self.type_for_expression(expression_id)?;
        let type_id = self.unwrap_value_type_id(type_id);
        // require a nominal reference type
        let symbol = match self.env.types.get_type(type_id) {
            dir::Type::Reference { symbol, .. } => *symbol,
            _ => return None,
        };

        // skip remote symbols
        if symbol.module_id != self.env.module_id {
            return None;
        }

        self.explicit_constructor_member_for_symbol(symbol)
    }

    /// Resolve the target symbol for a new expression when it is a simple reference.
    fn constructor_target_symbol_for_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<GlobalSymbolId> {
        let Expression::New { left, .. } = self.env.dir_tree.get(expression_id) else {
            return None;
        };

        match self.env.dir_tree.get(*left) {
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => Some(*target_symbol),
            _ => None,
        }
    }

    /// Find explicit constructor members for a nominal symbol.
    fn explicit_constructor_member_for_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<LocalNodeId<dir::Member>> {
        // scan declarations for explicit constructors
        let declaration_ids = self.declaration_ids_for_symbol(symbol);
        for declaration_id in declaration_ids {
            let declaration = self.env.dir_tree.get(declaration_id);
            // select struct or class members
            let members = match declaration {
                dir::Declaration::Struct { members, .. }
                | dir::Declaration::Class { members, .. } => members,
                _ => continue,
            };

            for member_id in members {
                let member = self.env.dir_tree.get(*member_id);
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
    ) -> Vec<LocalNodeId<dir::Declaration>> {
        // read the symbol entry for declaration lists
        let symbol_entry = self.env.symbols.get_symbol(symbol.local_id);
        let mut declaration_ids = Vec::new();

        // add the primary declaration first
        if let Some(primary) = symbol_entry.primary_declaration
            && primary.module_id == self.env.module_id
            && let Ok(local_id) = primary.local_id.try_into_typed::<dir::Declaration>()
        {
            declaration_ids.push(local_id);
        }

        // add secondary declarations in order
        if let Some(secondary) = symbol_entry.secondary_declarations.as_deref() {
            for declaration_id in secondary {
                if declaration_id.module_id != self.env.module_id {
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
