use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult, ScalarType};

use crate::lower::FunctionLowerer;
use crate::lower::r#type::UnionPayloadKind;

#[allow(clippy::too_many_arguments)]
impl FunctionLowerer<'_> {
    /// Lower a cast operator into a MIR cast operator.
    ///
    /// ```ds
    /// function widen(value: int32): float64 {
    ///     return value as float64;
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: f64 = cast <op> v0 -> f64
    /// ```
    pub(crate) fn lower_cast_operator(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::CastOperator,
        value_id: dir::LocalNodeId<dir::Expression>,
    ) -> LowerResult<Option<mir::CastOperator>> {
        // return early for identity casts
        if operator == dir::CastOperator::Identity {
            return Ok(None);
        }

        // resolve scalar types for operator selection
        let source_scalar_type = self.scalar_type_for_expression(value_id);
        let target_scalar_type = self.scalar_type_for_expression(expression_id);

        // select the mir cast operator
        let cast_operator = match operator {
            dir::CastOperator::IntWiden => match source_scalar_type {
                Some(ScalarType::SignedInt { .. }) => mir::CastOperator::SignExtend,
                Some(ScalarType::UnsignedInt { .. }) => mir::CastOperator::ZeroExtend,
                _ => {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                        message: "unsupported int widen cast".to_string(),
                    })?;
                }
            },
            dir::CastOperator::IntNarrow => mir::CastOperator::Truncate,
            dir::CastOperator::IntSignChange => mir::CastOperator::Bitcast,
            dir::CastOperator::FloatWiden => mir::CastOperator::FloatExtend,
            dir::CastOperator::FloatNarrow => mir::CastOperator::FloatTruncate,
            dir::CastOperator::IntToFloat => match source_scalar_type {
                Some(ScalarType::SignedInt { .. }) => mir::CastOperator::SignedIntToFloat,
                Some(ScalarType::UnsignedInt { .. }) => mir::CastOperator::UnsignedIntToFloat,
                _ => {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                        message: "unsupported int to float cast".to_string(),
                    })?;
                }
            },
            dir::CastOperator::FloatToInt => match target_scalar_type {
                Some(ScalarType::SignedInt { .. }) => mir::CastOperator::FloatToSignedInt,
                Some(ScalarType::UnsignedInt { .. }) => mir::CastOperator::FloatToUnsignedInt,
                _ => {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                        message: "unsupported float to int cast".to_string(),
                    })?;
                }
            },
            dir::CastOperator::PointerToInt => mir::CastOperator::PointerToInt,
            dir::CastOperator::IntToPointer => mir::CastOperator::IntToPointer,
            dir::CastOperator::PointerCast => mir::CastOperator::Bitcast,
            dir::CastOperator::EnumToInt | dir::CastOperator::IntToEnum => {
                let source_scalar_type =
                    source_scalar_type.ok_or_else(|| LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                        message: "unsupported enum cast source type".to_string(),
                    })?;
                let target_scalar_type =
                    target_scalar_type.ok_or_else(|| LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                        message: "unsupported enum cast target type".to_string(),
                    })?;
                return self.int_cast_operator_for_scalar(
                    expression_id,
                    source_scalar_type,
                    target_scalar_type,
                );
            }
            dir::CastOperator::EnumToString | dir::CastOperator::StringToEnum => {
                let string_type = self.context.type_lowerer.string_type().ok_or_else(|| {
                    LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                        message: "missing well known String layout (load library/native)"
                            .to_string(),
                    }
                })?;
                let source_type = self.lower_type_for_expression(value_id)?;
                let target_type = self.lower_type_for_expression(expression_id)?;
                if source_type != string_type || target_type != string_type {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                        message: "enum string cast requires string types".to_string(),
                    });
                }

                return Ok(None);
            }
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: format!("unsupported cast operator '{operator:?}'"),
                })?;
            }
        };

        // return the resolved operator
        Ok(Some(cast_operator))
    }

    /// Lower an instance upcast, including interface upcasts.
    ///
    /// ```ds
    /// function widen(dog: Dog): Animal {
    ///     return dog;
    /// }
    /// ```
    /// ->
    /// ```ds
    /// function widen(dog: Dog): Animal {
    ///     return dog as Animal;
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: ref<Animal, managed, readonly> = cast.bit v0 -> ref<Animal, managed, readonly>
    /// ```
    pub(crate) fn lower_instance_upcast(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        value_id: dir::LocalNodeId<dir::Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // lower the source value and target type
        let (value, source_mir_type) = self.lower_value_expression(value_id)?;
        let target_mir_type = self.lower_type_for_expression(expression_id)?;

        // resolve the source and target dir types
        let source_type_id = self.type_for_expression_or_error(value_id)?;
        let target_type_id = self.type_for_expression_or_error(expression_id)?;
        let target_dir_type = self.context.types.get_type(target_type_id);

        // handle interface upcasts
        if let dir::Type::Reference { symbol, .. } = target_dir_type
            && symbol.ty() == dir::SymbolType::Interface
        {
            return self.lower_interface_upcast(
                expression_id,
                value_id,
                value,
                source_mir_type,
                source_type_id,
                *symbol,
                target_type_id,
                target_mir_type,
            );
        }

        // short circuit identical types
        if source_mir_type == target_mir_type {
            return Ok((value, target_mir_type));
        }

        // cast.bit references -> the target type
        let source_type = self.state.builder.tree().get(source_mir_type).clone();
        if let mir::Type::Reference { .. } = source_type {
            let value = self.state.builder.bitcast(value, target_mir_type);
            return Ok((value, target_mir_type));
        }

        // return the value as the target type
        Ok((value, target_mir_type))
    }

    /// Lower an instance downcast as an unchecked conversion.
    ///
    /// ```ds
    /// function narrow(animal: Animal): Dog {
    ///     return animal as Dog;
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: ref<Dog, managed, readonly> = cast.bit v0 -> ref<Dog, managed, readonly>
    /// ```
    pub(crate) fn lower_instance_downcast(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        value_id: dir::LocalNodeId<dir::Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // lower the source value and target type
        let (value, source_mir_type) = self.lower_value_expression(value_id)?;
        let target_mir_type = self.lower_type_for_expression(expression_id)?;

        // resolve the source dir type
        let source_type_id = self.type_for_expression_or_error(value_id)?;
        let source_dir_type = self.context.types.get_type(source_type_id);

        // handle interface downcasts by extracting object pointers
        if let dir::Type::Reference { symbol, .. } = source_dir_type
            && symbol.ty() == dir::SymbolType::Interface
        {
            // resolve interface reference layout
            let layout = self
                .context
                .type_lowerer
                .interface_ref_layout(source_type_id)
                .ok_or_else(|| self.missing_type_error(expression_id))?;

            // extract the object pointer from the interface value
            let object_ptr = self
                .state
                .builder
                .field_get(value, layout.object_field_index);
            let object_ptr = self.state.builder.bitcast(object_ptr, layout.object_type);

            // cast the object pointer to the target type
            let target = match self.state.builder.tree().get(target_mir_type).clone() {
                mir::Type::Reference { .. } => {
                    self.state.builder.bitcast(object_ptr, target_mir_type)
                }
                _ => {
                    let reference_type = self.state.builder.type_reference(
                        mir::ReferenceKind::Managed,
                        target_mir_type,
                        mir::Mutability::Immutable,
                        mir::AddressSpace::Generic,
                        false,
                    );
                    let casted = self.state.builder.bitcast(object_ptr, reference_type);
                    self.state.builder.load(casted, target_mir_type)
                }
            };

            return Ok((target, target_mir_type));
        }

        // short circuit identical types
        if source_mir_type == target_mir_type {
            return Ok((value, target_mir_type));
        }

        // cast.bit references -> the target type
        let source_type = self.state.builder.tree().get(source_mir_type).clone();
        if let mir::Type::Reference { .. } = source_type {
            let value = self.state.builder.bitcast(value, target_mir_type);
            return Ok((value, target_mir_type));
        }

        // return the value as the target type
        Ok((value, target_mir_type))
    }

    /// Lower a union upcast into a tagged union value.
    ///
    /// ```ds
    /// function make(value: int32): int32 | boolean {
    ///     return value;
    /// }
    /// ```
    /// ->
    /// ```ds
    /// function make(value: int32): int32 | boolean {
    ///     return value as int32 | boolean;
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: uint8 = const 0
    /// v2: usize[1] = <payload>
    /// v3: Union = struct Union (v1, v2)
    /// ```
    pub(crate) fn lower_union_upcast(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        value_id: dir::LocalNodeId<dir::Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // resolve the source and target dir types
        let source_type_id = self.type_for_expression_or_error(value_id)?;
        let target_type_id = self.type_for_expression_or_error(expression_id)?;

        // resolve the target mir type
        let target_mir_type = self.lower_type_for_expression(expression_id)?;

        // resolve union layout metadata
        let layout = self
            .context
            .type_lowerer
            .union_layout(target_type_id)
            .ok_or_else(|| self.missing_type_error(expression_id))?;

        // skip when the source is already the target union type
        if dir::are_types_equal(source_type_id, target_type_id, self.context.types) {
            let (value, _source_mir_type) = self.lower_value_expression(value_id)?;
            return Ok((value, target_mir_type));
        }

        // resolve the union tag index for the source type
        let tag_index = layout
            .element_types
            .iter()
            .position(|element| self.type_ids_equivalent(*element, source_type_id))
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
                message: "union upcast missing matching element".to_string(),
            })?;

        // build the tag constant
        let (tag_width, tag_signed) = match self.state.builder.tree().get(layout.tag_type) {
            mir::Type::Int {
                width,
                is_signed: signed,
            } => (*width as u8, *signed),
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: "union tag must be an integer type".to_string(),
                });
            }
        };
        let tag_value = self
            .state
            .builder
            .iconst(tag_index as i64, tag_width, tag_signed);

        // resolve literals that do not carry payload data
        let is_nullish_literal = matches!(
            self.context.types.get_type(source_type_id),
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Null | dir::TypeLiteral::Undefined,
            }
        );

        // build the union payload
        let node = expression_id
            .into_global_any(self.context.module_id)
            .into_anchored(Some(self.context.profile));
        let payload = if is_nullish_literal {
            // zero payload for null or undefined
            self.union_payload_zero_value(layout, node)?
        } else {
            // lower the source value into the payload
            let (value, source_mir_type) = self.lower_value_expression(value_id)?;
            match layout.payload_kind {
                UnionPayloadKind::Inline => self.inline_union_payload_from_value(
                    layout.payload_type,
                    value,
                    source_mir_type,
                    node,
                )?,
                UnionPayloadKind::Boxed => {
                    let boxed = self.box_value(value, source_mir_type);
                    self.state.builder.bitcast(boxed, layout.payload_type)
                }
            }
        };

        // assemble the union value
        let mut fields = vec![tag_value, payload];
        if layout.tag_field_index > layout.payload_field_index {
            fields.swap(0, 1);
        }
        let union_value = self.state.builder.struct_(target_mir_type, fields);

        // return the union value and type
        Ok((union_value, target_mir_type))
    }

    /// Lower a union downcast as an unchecked payload extraction.
    ///
    /// ```ds
    /// function take(value: int32 | boolean): int32 {
    ///     return value as int32;
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: [usize; 1] = <payload>
    /// v2: i32 = <payload_extract>
    /// ```
    pub(crate) fn lower_union_downcast(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        value_id: dir::LocalNodeId<dir::Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // lower the source value and target type
        let (value, _source_mir_type) = self.lower_value_expression(value_id)?;
        let target_mir_type = self.lower_type_for_expression(expression_id)?;

        // resolve the source dir type
        let source_type_id = self.type_for_expression_or_error(value_id)?;

        // resolve union layout metadata
        let layout = self
            .context
            .type_lowerer
            .union_layout(source_type_id)
            .ok_or_else(|| self.missing_type_error(expression_id))?;

        // extract the payload value
        let payload_value = self
            .state
            .builder
            .field_get(value, layout.payload_field_index);

        // load the payload as the target type
        let node = expression_id
            .into_global_any(self.context.module_id)
            .into_anchored(Some(self.context.profile));
        let value = match layout.payload_kind {
            UnionPayloadKind::Inline => self.inline_union_payload_to_value(
                layout.payload_type,
                payload_value,
                target_mir_type,
                node,
            )?,
            UnionPayloadKind::Boxed => {
                let reference_type = self.state.builder.type_reference(
                    mir::ReferenceKind::Managed,
                    target_mir_type,
                    mir::Mutability::Immutable,
                    mir::AddressSpace::Generic,
                    false,
                );
                let casted = self.state.builder.bitcast(payload_value, reference_type);
                self.state.builder.load(casted, target_mir_type)
            }
        };

        // return the payload value and type
        Ok((value, target_mir_type))
    }

    /// Lower a nullable upcast into a union or nullable reference.
    ///
    /// ```ds
    /// function take(node: Node): Node | null {
    ///     return node;
    /// }
    /// ```
    /// ->
    /// ```ds
    /// function take(node: Node): Node | null {
    ///     return node as Node | null;
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: ref?<Node, managed, readonly> = cast.bit v0 -> ref?<Node, managed, readonly>
    /// ```
    pub(crate) fn lower_nullable_upcast(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        value_id: dir::LocalNodeId<dir::Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let target_type_id = self.type_for_expression_or_error(expression_id)?;
        if self
            .context
            .type_lowerer
            .union_layout(target_type_id)
            .is_some()
        {
            return self.lower_union_upcast(expression_id, value_id);
        }

        // reject undefined in nullable reference casts
        let source_type_id = self.type_for_expression_or_error(value_id)?;
        if matches!(
            self.context.types.get_type(source_type_id),
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Undefined
            }
        ) {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
                message: "nullable upcast does not accept undefined".to_string(),
            });
        }

        let target_mir_type = self.lower_type_for_expression(expression_id)?;
        let mir::Type::Reference { .. } = self.state.builder.tree().get(target_mir_type) else {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
                message: "nullable upcast requires a reference target".to_string(),
            });
        };

        // handle null literals without lowering a payload value
        let is_null_literal = matches!(
            self.context.types.get_type(source_type_id),
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Null,
            }
        );
        let value = if is_null_literal {
            let node = expression_id
                .into_global_any(self.context.module_id)
                .into_anchored(Some(self.context.profile));
            self.zero_value_for_type(target_mir_type, node)?
        } else {
            let (value, _source_type) = self.lower_value_expression(value_id)?;
            self.state
                .builder
                .cast(mir::CastOperator::Bitcast, value, target_mir_type)
        };

        Ok((value, target_mir_type))
    }

    /// Lower a nullable downcast into a union or nullable reference.
    ///
    /// ```ds
    /// function take(value: Node | null): Node {
    ///     return value as Node;
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: ref<Node, managed, readonly> = cast.bit v0 -> ref<Node, managed, readonly>
    /// ```
    pub(crate) fn lower_nullable_downcast(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        value_id: dir::LocalNodeId<dir::Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let source_type_id = self.type_for_expression_or_error(value_id)?;
        if self
            .context
            .type_lowerer
            .union_layout(source_type_id)
            .is_some()
        {
            return self.lower_union_downcast(expression_id, value_id);
        }

        let (value, _source_type) = self.lower_value_expression(value_id)?;
        let target_mir_type = self.lower_type_for_expression(expression_id)?;
        let mir::Type::Reference { .. } = self.state.builder.tree().get(target_mir_type) else {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
                message: "nullable downcast requires a reference target".to_string(),
            });
        };

        let value = self
            .state
            .builder
            .cast(mir::CastOperator::Bitcast, value, target_mir_type);

        Ok((value, target_mir_type))
    }

    /// Build an interface reference from a concrete value.
    ///
    /// ```ds
    /// interface Drawable {}
    ///
    /// class Sprite implements Drawable {}
    ///
    /// function asDrawable(value: Sprite): Drawable {
    ///     return value;
    /// }
    /// ```
    /// ->
    /// ```ds
    /// function asDrawable(value: Sprite): Drawable {
    ///     return value as Drawable;
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: @DrawableRef = struct @DrawableRef (v0, <itab>)
    /// ```
    pub(crate) fn lower_interface_upcast(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        value_id: dir::LocalNodeId<dir::Expression>,
        value: mir::Value,
        source_mir_type: mir::LocalNodeId<mir::Type>,
        source_type_id: dir::LocalTypeId,
        interface_symbol: dir::GlobalSymbolId,
        target_type_id: dir::LocalTypeId,
        target_mir_type: mir::LocalNodeId<mir::Type>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // resolve interface reference layout
        let layout = self
            .context
            .type_lowerer
            .interface_ref_layout(target_type_id)
            .ok_or_else(|| self.missing_type_error(expression_id))?;

        // resolve the concrete symbol for the source type
        let source_dir_type = self.context.types.get_type(source_type_id);
        let concrete_symbol = self
            .concrete_symbol_for_type(source_type_id)
            .or_else(|| self.concrete_symbol_for_expression(value_id));

        // reject interface to interface casts without RTTI
        let Some(concrete_symbol) = concrete_symbol else {
            if matches!(
                source_dir_type,
                dir::Type::Reference { symbol, .. } if symbol.ty() == dir::SymbolType::Interface
            ) {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: "FUGU #Broken: interface to interface upcast requires RTTI"
                        .to_string(),
                });
            }

            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
                message: "interface upcast requires a concrete symbol".to_string(),
            });
        };

        // resolve the itab id for the concrete and interface pair
        let itab_id = self
            .context
            .interface_itab_ids
            .get(&(concrete_symbol, interface_symbol))
            .copied()
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
                message: "missing interface itab for concrete type".to_string(),
            })?;

        // convert the source value into an object pointer
        let object_ptr =
            self.object_pointer_for_instance(value, source_mir_type, layout.object_type);

        // encode the itab id as a pointer sized value
        let tag_width = self.context.type_lowerer.pointer_width_bits() as u8;
        let itab_value = self
            .state
            .builder
            .iconst(itab_id.index() as i64, tag_width, false);
        let itab_value = self.state.builder.bitcast(itab_value, layout.itab_type);

        // assemble the interface reference value
        let mut fields = vec![object_ptr, itab_value];
        if layout.object_field_index > layout.itab_field_index {
            fields.swap(0, 1);
        }
        let interface_value = self.state.builder.struct_(target_mir_type, fields);

        // return the interface value and type
        Ok((interface_value, target_mir_type))
    }

    /// Select an integer cast operator for scalar types.
    fn int_cast_operator_for_scalar(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        source: ScalarType,
        target: ScalarType,
    ) -> LowerResult<Option<mir::CastOperator>> {
        let source_signed = matches!(source, ScalarType::SignedInt { .. });
        let target_signed = matches!(target, ScalarType::SignedInt { .. });

        let source_width = match source {
            ScalarType::SignedInt { width } | ScalarType::UnsignedInt { width } => width,
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: "unsupported enum cast source type".to_string(),
                });
            }
        };
        let target_width = match target {
            ScalarType::SignedInt { width } | ScalarType::UnsignedInt { width } => width,
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: "unsupported enum cast target type".to_string(),
                });
            }
        };

        // select cast based on width and signedness
        let cast = if source_width == target_width {
            if source_signed == target_signed {
                None
            } else {
                Some(mir::CastOperator::Bitcast)
            }
        } else if source_width < target_width {
            Some(if source_signed {
                mir::CastOperator::SignExtend
            } else {
                mir::CastOperator::ZeroExtend
            })
        } else {
            Some(mir::CastOperator::Truncate)
        };

        Ok(cast)
    }

    /// Convert a value into a managed object pointer.
    fn object_pointer_for_instance(
        &mut self,
        value: mir::Value,
        source_type: mir::LocalNodeId<mir::Type>,
        target_ptr_type: mir::LocalNodeId<mir::Type>,
    ) -> mir::Value {
        // load the source type for conversion
        let source_type_id = source_type;
        let source_type = self.state.builder.tree().get(source_type_id).clone();

        // convert based on source representation
        match source_type {
            mir::Type::Reference {
                kind: mir::ReferenceKind::Managed,
                ..
            } => self.state.builder.bitcast(value, target_ptr_type),
            mir::Type::Reference { pointee, .. } => {
                let Some(pointee) = pointee.ty() else {
                    return self.state.builder.bitcast(value, target_ptr_type);
                };

                let loaded = self.state.builder.load(value, pointee);
                let boxed = self.box_value(loaded, pointee);
                self.state.builder.bitcast(boxed, target_ptr_type)
            }
            _ => {
                let boxed = self.box_value(value, source_type_id);
                self.state.builder.bitcast(boxed, target_ptr_type)
            }
        }
    }

    /// Allocate managed storage for a value and return a reference to it.
    pub(super) fn box_value(
        &mut self,
        value: mir::Value,
        value_type: mir::LocalNodeId<mir::Type>,
    ) -> mir::Value {
        // build the managed reference type
        let ref_type = self.state.builder.type_reference(
            mir::ReferenceKind::Managed,
            value_type,
            mir::Mutability::Immutable,
            mir::AddressSpace::Generic,
            false,
        );

        // allocate and store the value
        let pointer = self.state.builder.managed_alloc(value_type, ref_type);
        self.state.builder.store(pointer, value);

        // return the managed reference
        pointer
    }

    /// Resolve the concrete symbol for a nominal instance type.
    fn concrete_symbol_for_type(&self, type_id: dir::LocalTypeId) -> Option<dir::GlobalSymbolId> {
        // walk the type tree to find a nominal class or struct
        match self.context.types.get_type(type_id) {
            dir::Type::Reference { symbol, .. }
                if matches!(
                    symbol.ty(),
                    dir::SymbolType::Class | dir::SymbolType::Struct
                ) =>
            {
                Some(*symbol)
            }
            dir::Type::Value { value } => self.concrete_symbol_for_type(*value),
            dir::Type::Intersection { elements } => elements
                .iter()
                .find_map(|element| self.concrete_symbol_for_type(*element)),
            _ => None,
        }
    }

    /// Resolve the concrete symbol for an expression when possible.
    fn concrete_symbol_for_expression(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalSymbolId> {
        // peel parenthesized expressions
        let expression = self.context.dir_tree.get(expression_id);
        match expression {
            dir::Expression::Parenthesized { expression } => {
                self.concrete_symbol_for_expression(*expression)
            }
            dir::Expression::As { expression, .. }
            | dir::Expression::Satisfies { expression, .. } => {
                self.concrete_symbol_for_expression(*expression)
            }
            dir::Expression::New { left, .. } => self.concrete_symbol_for_expression(*left),
            dir::Expression::TaggedScalarExpression { ty, .. }
            | dir::Expression::TaggedTupleExpression { ty, .. }
            | dir::Expression::TaggedObjectExpression { ty, .. } => {
                let type_id = self.type_id_for_type_expression(*ty)?;
                self.concrete_symbol_for_type(type_id)
            }
            dir::Expression::LocalReference { target_symbol, .. }
            | dir::Expression::ModuleReference { target_symbol, .. }
            | dir::Expression::GlobalReference { target_symbol, .. } => match target_symbol.ty() {
                dir::SymbolType::Class | dir::SymbolType::Struct => Some(*target_symbol),
                _ => None,
            },
            _ => None,
        }
    }
}
