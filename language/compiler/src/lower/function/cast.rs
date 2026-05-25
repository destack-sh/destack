use {destack_dir as dir, destack_mir as mir};

use crate::{CompilerError, CompilerResult, LowerError, ScalarType};

use crate::lower::FunctionLowerer;
use crate::lower::r#type::VariantPayload;

#[allow(clippy::too_many_arguments)]
impl FunctionLowerer<'_> {
    /// Lower a value expression into a declared target type.
    pub(crate) fn lower_value_for_target(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        value_id: dir::LocalNodeId<dir::Expression>,
        target_type_id: Option<dir::LocalTypeId>,
        target_mir_type: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let Some(target_type_id) = target_type_id else {
            return self.lower_value_expression(value_id);
        };
        let target_type_id = self
            .context
            .types
            .unwrap_form_payload_type_id(target_type_id);

        if self
            .context
            .type_lowerer
            .union_layout(target_type_id)
            .is_some()
        {
            return self.lower_union_upcast_to_type(
                expression_id,
                value_id,
                target_type_id,
                target_mir_type,
            );
        }

        let (value, value_type) = self.lower_value_expression(value_id)?;
        self.lower_assignment_conversion(
            expression_id,
            value_id,
            value,
            value_type,
            Some(target_type_id),
            target_mir_type,
        )
    }

    /// Lower an assignment conversion to a declared target type.
    pub(crate) fn lower_assignment_conversion(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        value_id: dir::LocalNodeId<dir::Expression>,
        value: mir::Value,
        source_mir_type: mir::LocalNodeId<mir::Type>,
        target_type_id: Option<dir::LocalTypeId>,
        target_mir_type: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        if source_mir_type == target_mir_type {
            return Ok((value, target_mir_type));
        }

        let Some(target_type_id) = target_type_id else {
            return Ok((value, source_mir_type));
        };
        let target_type_id = self
            .context
            .types
            .unwrap_form_payload_type_id(target_type_id);
        let target_type = self.context.types.get_type(target_type_id);

        let source_type_id = self.type_for_expression_or_error(value_id)?;
        let source_type_id = self
            .context
            .types
            .unwrap_form_payload_type_id(source_type_id);
        if let dir::Type::Named(reference) = target_type
            && self
                .context
                .symbol_is(reference.symbol, dir::SymbolForm::Interface)
        {
            if matches!(
                self.context.types.get_type(source_type_id),
                dir::Type::Named(source_reference)
                    if source_reference.symbol == reference.symbol
            ) {
                return Ok((value, source_mir_type));
            }

            return self.lower_any_upcast(
                expression_id,
                value_id,
                value,
                source_mir_type,
                source_type_id,
                reference.symbol,
                target_type_id,
                target_mir_type,
            );
        }

        if matches!(
            self.state.builder.tree().get(source_mir_type),
            mir::Type::Reference { .. }
        ) {
            if self.reference_types_assign_without_cast(source_mir_type, target_mir_type) {
                return Ok((value, source_mir_type));
            }

            let value = self.state.builder.bitcast(value, target_mir_type);
            return Ok((value, target_mir_type));
        }

        Ok((value, source_mir_type))
    }

    /// Return whether two reference types differ only by storage provenance.
    fn reference_types_assign_without_cast(
        &self,
        source_mir_type: mir::LocalNodeId<mir::Type>,
        target_mir_type: mir::LocalNodeId<mir::Type>,
    ) -> bool {
        let source_type = self.state.builder.tree().get(source_mir_type);
        let target_type = self.state.builder.tree().get(target_mir_type);

        matches!(
            (source_type, target_type),
            (
                mir::Type::Reference {
                    kind: source_kind,
                    access: source_access,
                    pointee: source_pointee,
                    nullability: source_nullability,
                    ..
                },
                mir::Type::Reference {
                    kind: target_kind,
                    access: target_access,
                    pointee: target_pointee,
                    nullability: target_nullability,
                    ..
                },
            ) if source_kind == target_kind
                && source_access == target_access
                && source_pointee == target_pointee
                && source_nullability == target_nullability
        )
    }

    /// Classify an explicit cast from checked expression types.
    pub(crate) fn classify_explicit_cast_operator(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        value_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::CastOperator> {
        let source_type_id = self.type_for_expression_or_error(value_id)?;
        let target_type_id = self.type_for_expression_or_error(expression_id)?;
        let source_type_id = self
            .context
            .types
            .unwrap_form_payload_type_id(source_type_id);
        let target_type_id = self
            .context
            .types
            .unwrap_form_payload_type_id(target_type_id);

        if source_type_id == target_type_id {
            return Ok(dir::CastOperator::Identity);
        }

        let source_is_union = matches!(
            self.context.types.get_type(source_type_id),
            dir::Type::Union(_)
        );
        let target_is_union = matches!(
            self.context.types.get_type(target_type_id),
            dir::Type::Union(_)
        );

        if target_is_union {
            return Ok(dir::CastOperator::UnionUpcast);
        }

        if source_is_union {
            return Ok(dir::CastOperator::UnionDowncast);
        }

        let source_scalar_type = self
            .scalar_type_for_expression(value_id)
            .ok_or_else(|| self.error(expression_id, "unsupported cast source type"))
            .map_err(CompilerError::from)?;
        let target_scalar_type = self
            .scalar_type_for_expression(expression_id)
            .ok_or_else(|| self.error(expression_id, "unsupported cast target type"))
            .map_err(CompilerError::from)?;

        match (source_scalar_type, target_scalar_type) {
            (
                ScalarType::SignedInt {
                    width: source_width,
                },
                ScalarType::SignedInt {
                    width: target_width,
                },
            )
            | (
                ScalarType::UnsignedInt {
                    width: source_width,
                },
                ScalarType::UnsignedInt {
                    width: target_width,
                },
            ) => {
                if target_width > source_width {
                    Ok(dir::CastOperator::IntWiden)
                } else if target_width < source_width {
                    Ok(dir::CastOperator::IntNarrow)
                } else {
                    Ok(dir::CastOperator::Identity)
                }
            }
            (
                ScalarType::SignedInt {
                    width: source_width,
                },
                ScalarType::UnsignedInt {
                    width: target_width,
                },
            )
            | (
                ScalarType::UnsignedInt {
                    width: source_width,
                },
                ScalarType::SignedInt {
                    width: target_width,
                },
            ) => {
                if source_width == target_width {
                    Ok(dir::CastOperator::IntSignChange)
                } else if target_width > source_width {
                    Ok(dir::CastOperator::IntWiden)
                } else {
                    Ok(dir::CastOperator::IntNarrow)
                }
            }
            (
                ScalarType::Float {
                    width: source_width,
                },
                ScalarType::Float {
                    width: target_width,
                },
            ) => {
                if target_width > source_width {
                    Ok(dir::CastOperator::FloatWiden)
                } else if target_width < source_width {
                    Ok(dir::CastOperator::FloatNarrow)
                } else {
                    Ok(dir::CastOperator::Identity)
                }
            }
            (
                ScalarType::SignedInt { .. } | ScalarType::UnsignedInt { .. },
                ScalarType::Float { .. },
            ) => Ok(dir::CastOperator::IntToFloat),
            (
                ScalarType::Float { .. },
                ScalarType::SignedInt { .. } | ScalarType::UnsignedInt { .. },
            ) => Ok(dir::CastOperator::FloatToInt),
            _ => Err(self
                .error(expression_id, "unsupported cast operator")
                .into()),
        }
    }

    /// Return whether one type may carry one local or shared heap address.
    fn is_heap_address_source_type(&self, ty: mir::LocalNodeId<mir::Type>) -> bool {
        matches!(
            self.state.builder.tree().get(ty),
            mir::Type::Reference {
                kind: mir::ReferenceKind::Managed,
                space: mir::Space::Local | mir::Space::Shared,
                ..
            } | mir::Type::Reference {
                kind: mir::ReferenceKind::Borrowed,
                space: mir::Space::Local | mir::Space::Shared,
                ..
            } | mir::Type::TensorView {
                kind: mir::ReferenceKind::Managed,
                space: mir::Space::Local | mir::Space::Shared,
                ..
            } | mir::Type::TensorView {
                kind: mir::ReferenceKind::Borrowed,
                space: mir::Space::Local | mir::Space::Shared,
                ..
            }
        )
    }

    /// Return whether one type is a raw pointer like result.
    fn is_raw_pointer_type(&self, ty: mir::LocalNodeId<mir::Type>) -> bool {
        matches!(
            self.state.builder.tree().get(ty),
            mir::Type::Reference {
                kind: mir::ReferenceKind::Raw,
                ..
            } | mir::Type::TensorView {
                kind: mir::ReferenceKind::Raw,
                ..
            }
        )
    }

    /// Reject one cast that would expose one heap address without explicit pinning.
    pub(crate) fn reject_implicit_heap_address_cast(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::CastOperator,
        source_type: mir::LocalNodeId<mir::Type>,
        target_type: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<()> {
        // local and shared heap references need one explicit stable address path
        if !self.is_heap_address_source_type(source_type) {
            return Ok(());
        }

        // reject direct heap to integer casts
        if matches!(operator, dir::CastOperator::PointerToInt) {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "pointer to int cast from heap storage requires explicit pinning"
                    .to_string(),
            }
            .into());
        }

        // reject direct heap to raw pointer casts
        if matches!(operator, dir::CastOperator::PointerCast)
            && self.is_raw_pointer_type(target_type)
        {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "raw pointer cast from heap storage requires explicit pinning".to_string(),
            }
            .into());
        }

        Ok(())
    }

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
    ) -> CompilerResult<Option<mir::CastOperator>> {
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
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "unsupported int widen cast".to_string(),
                    }
                    .into());
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
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "unsupported int to float cast".to_string(),
                    }
                    .into());
                }
            },
            dir::CastOperator::FloatToInt => match target_scalar_type {
                Some(ScalarType::SignedInt { .. }) => mir::CastOperator::FloatToSignedInt,
                Some(ScalarType::UnsignedInt { .. }) => mir::CastOperator::FloatToUnsignedInt,
                _ => {
                    return Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "unsupported float to int cast".to_string(),
                    }
                    .into());
                }
            },
            dir::CastOperator::PointerToInt => mir::CastOperator::PointerToInt,
            dir::CastOperator::IntToPointer => mir::CastOperator::IntToPointer,
            dir::CastOperator::PointerCast => mir::CastOperator::Bitcast,
            dir::CastOperator::EnumToInt | dir::CastOperator::IntToEnum => {
                let source_scalar_type = source_scalar_type
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "unsupported enum cast source type".to_string(),
                    })
                    .map_err(CompilerError::from)?;
                let target_scalar_type = target_scalar_type
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "unsupported enum cast target type".to_string(),
                    })
                    .map_err(CompilerError::from)?;
                return self.int_cast_operator_for_scalar(
                    expression_id,
                    source_scalar_type,
                    target_scalar_type,
                );
            }
            dir::CastOperator::EnumToString | dir::CastOperator::StringToEnum => {
                let string_type = self.context.type_lowerer.string_type().ok_or_else(|| {
                    LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "missing language item String layout (load core)".to_string(),
                    }
                })?;
                let source_type = self.lower_type_for_expression(value_id)?;
                let target_type = self.lower_type_for_expression(expression_id)?;
                if source_type != string_type || target_type != string_type {
                    return Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "enum string cast requires string types".to_string(),
                    }
                    .into());
                }

                return Ok(None);
            }
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: format!("unsupported cast operator '{operator:?}'"),
                }
                .into());
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
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // lower the source value and target type
        let (value, source_mir_type) = self.lower_value_expression(value_id)?;
        let target_mir_type = self.lower_type_for_expression(expression_id)?;

        // resolve the source and target dir types
        let source_type_id = self.type_for_expression_or_error(value_id)?;
        let target_type_id = self.type_for_expression_or_error(expression_id)?;
        let target_dir_type = self.context.types.get_type(target_type_id);

        // handle interface upcasts
        if let dir::Type::Named(reference) = target_dir_type
            && self
                .context
                .symbol_is(reference.symbol, dir::SymbolForm::Interface)
        {
            return self.lower_any_upcast(
                expression_id,
                value_id,
                value,
                source_mir_type,
                source_type_id,
                reference.symbol,
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
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // lower the source value and target type
        let (value, source_mir_type) = self.lower_value_expression(value_id)?;
        let target_mir_type = self.lower_type_for_expression(expression_id)?;

        // resolve the source dir type
        let source_type_id = self.type_for_expression_or_error(value_id)?;
        let source_dir_type = self.context.types.get_type(source_type_id);

        // handle Any downcasts by extracting value pointers
        if let dir::Type::Named(reference) = source_dir_type
            && self
                .context
                .symbol_is(reference.symbol, dir::SymbolForm::Interface)
        {
            // resolve Any value layout
            let layout = self
                .context
                .type_lowerer
                .any_value_layout(source_type_id)
                .ok_or_else(|| self.missing_type_error(expression_id))
                .map_err(CompilerError::from)?;

            // extract the erased value pointer
            let value_ptr = self
                .state
                .builder
                .field_get(value, layout.value_field_index);
            let value_ptr = self.state.builder.bitcast(value_ptr, layout.value_type);

            // cast the value pointer to the target type
            let target = match self.state.builder.tree().get(target_mir_type).clone() {
                mir::Type::Reference { .. } => {
                    self.state.builder.bitcast(value_ptr, target_mir_type)
                }
                _ => {
                    let reference_type = self.state.builder.type_reference(
                        mir::ReferenceKind::Managed,
                        target_mir_type,
                        mir::Access::Readonly,
                        mir::Space::Local,
                        mir::Nullability::None,
                    );
                    let casted = self.state.builder.bitcast(value_ptr, reference_type);
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
    /// v2: [usize; 1] = <payload>
    /// v3: Union = struct Union (v1, v2)
    /// ```
    pub(crate) fn lower_union_upcast(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        value_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let target_type_id = self.type_for_expression_or_error(expression_id)?;
        let target_mir_type = self.lower_type_for_expression(expression_id)?;

        self.lower_union_upcast_to_type(expression_id, value_id, target_type_id, target_mir_type)
    }

    /// Lower a union upcast into a known target type.
    pub(crate) fn lower_union_upcast_to_type(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        value_id: dir::LocalNodeId<dir::Expression>,
        target_type_id: dir::LocalTypeId,
        target_mir_type: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // resolve the source dir type
        let source_type_id = self.type_for_expression_or_error(value_id)?;

        // resolve union layout metadata
        let layout = self
            .context
            .type_lowerer
            .union_layout(target_type_id)
            .ok_or_else(|| self.missing_type_error(expression_id))
            .map_err(CompilerError::from)?;

        // skip when the source is already the target union type
        if source_type_id == target_type_id {
            let (value, _source_mir_type) = self.lower_value_expression(value_id)?;
            return Ok((value, target_mir_type));
        }

        // resolve the union tag index for the source type
        let tag_index = layout
            .source_types
            .iter()
            .position(|element| self.type_ids_equivalent(*element, source_type_id))
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "union upcast missing matching element".to_string(),
            })
            .map_err(CompilerError::from)?;

        // build the tag constant
        let (tag_width, tag_signed) = match self.state.builder.tree().get(layout.tag_type) {
            mir::Type::Int {
                width,
                is_signed: signed,
            } => (*width, *signed),
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: "union tag must be an integer type".to_string(),
                }
                .into());
            }
        };
        let tag_value = self
            .state
            .builder
            .iconst(tag_index as i128, tag_width, tag_signed);

        // resolve literals that do not carry payload data
        let is_nullish_literal = matches!(
            self.context.types.get_type(source_type_id),
            dir::Type::Null | dir::Type::Undefined
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
            match layout.payload {
                VariantPayload::Inline => self.inline_union_payload_from_value(
                    layout.payload_type,
                    value,
                    source_mir_type,
                    node,
                )?,
                VariantPayload::Boxed => {
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
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
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
            .ok_or_else(|| self.missing_type_error(expression_id))
            .map_err(CompilerError::from)?;

        // extract the payload value
        let payload_value = self
            .state
            .builder
            .field_get(value, layout.payload_field_index);

        // load the payload as the target type
        let node = expression_id
            .into_global_any(self.context.module_id)
            .into_anchored(Some(self.context.profile));
        let value = match layout.payload {
            VariantPayload::Inline => self.inline_union_payload_to_value(
                layout.payload_type,
                payload_value,
                target_mir_type,
                node,
            )?,
            VariantPayload::Boxed => {
                let reference_type = self.state.builder.type_reference(
                    mir::ReferenceKind::Managed,
                    target_mir_type,
                    mir::Access::Readonly,
                    mir::Space::Local,
                    mir::Nullability::None,
                );
                let casted = self.state.builder.bitcast(payload_value, reference_type);
                self.state.builder.load(casted, target_mir_type)
            }
        };

        // return the payload value and type
        Ok((value, target_mir_type))
    }

    /// Lower a nullable upcast into a union or reference that allows null.
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
    /// v1: ref<Node, managed, readonly, nullable> = cast.bit v0 -> ref<Node, managed, readonly, nullable>
    /// ```
    pub(crate) fn lower_nullable_upcast(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        value_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let target_type_id = self.type_for_expression_or_error(expression_id)?;
        if self
            .context
            .type_lowerer
            .union_layout(target_type_id)
            .is_some()
        {
            return self.lower_union_upcast(expression_id, value_id);
        }

        // reject undefined in null reference casts
        let source_type_id = self.type_for_expression_or_error(value_id)?;
        if matches!(
            self.context.types.get_type(source_type_id),
            dir::Type::Undefined
        ) {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "nullable upcast does not accept undefined".to_string(),
            }
            .into());
        }

        let target_mir_type = self.lower_type_for_expression(expression_id)?;
        let mir::Type::Reference { .. } = self.state.builder.tree().get(target_mir_type) else {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "nullable upcast requires a reference target".to_string(),
            }
            .into());
        };

        // handle null literals without lowering a payload value
        let is_null_literal =
            matches!(self.context.types.get_type(source_type_id), dir::Type::Null);
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

    /// Lower a nullable downcast into a union or reference that allows null.
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
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
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
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "nullable downcast requires a reference target".to_string(),
            }
            .into());
        };

        let value = self
            .state
            .builder
            .cast(mir::CastOperator::Bitcast, value, target_mir_type);

        Ok((value, target_mir_type))
    }

    /// Build an Any value from a concrete value.
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
    /// v1: Dynamic<Drawable> = struct Dynamic<Drawable> (v0, <table>)
    /// ```
    pub(crate) fn lower_any_upcast(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        value_id: dir::LocalNodeId<dir::Expression>,
        value: mir::Value,
        source_mir_type: mir::LocalNodeId<mir::Type>,
        source_type_id: dir::LocalTypeId,
        interface_symbol: dir::GlobalSymbolId,
        target_type_id: dir::LocalTypeId,
        target_mir_type: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // resolve Any value layout
        let layout = self
            .context
            .type_lowerer
            .any_value_layout(target_type_id)
            .ok_or_else(|| self.missing_type_error(expression_id))
            .map_err(CompilerError::from)?;

        // resolve the concrete symbol for the source type
        let source_dir_type = self.context.types.get_type(source_type_id);
        let concrete_symbol = self
            .concrete_symbol_for_type(source_type_id)
            .or_else(|| self.concrete_symbol_for_expression(value_id));

        // reject interface to interface casts without RTTI
        let Some(concrete_symbol) = concrete_symbol else {
            if matches!(
                source_dir_type,
                dir::Type::Named(reference)
                    if self
                        .context
                        .symbol_is(reference.symbol, dir::SymbolForm::Interface)
            ) {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: "TODO #Broken: interface to interface upcast requires RTTI"
                        .to_string(),
                }
                .into());
            }

            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "interface upcast requires a concrete symbol".to_string(),
            }
            .into());
        };

        // resolve the dispatch table global for the concrete and interface pair
        let table = self
            .context
            .interface_table_globals_by_pair
            .get(&(concrete_symbol, interface_symbol))
            .copied()
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "missing interface table for concrete type".to_string(),
            })
            .map_err(CompilerError::from)?;

        // convert the source value into an erased value pointer
        let value_ptr =
            self.erased_value_pointer_for_instance(value, source_mir_type, layout.value_type);

        // load the table pointer from static space
        let table_value = self
            .state
            .builder
            .global_addr(table.global_id, table.address_type);
        let table_value = if table.address_type == layout.table_type {
            table_value
        } else {
            self.state
                .builder
                .cast(mir::CastOperator::Bitcast, table_value, layout.table_type)
        };

        // assemble the Any value
        let mut fields = vec![value_ptr, table_value];
        if layout.value_field_index > layout.table_field_index {
            fields.swap(0, 1);
        }
        let any_value = self.state.builder.struct_(target_mir_type, fields);

        // return the Any value and type
        Ok((any_value, target_mir_type))
    }

    /// Select an integer cast operator for scalar types.
    fn int_cast_operator_for_scalar(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        source: ScalarType,
        target: ScalarType,
    ) -> CompilerResult<Option<mir::CastOperator>> {
        let source_signed = matches!(source, ScalarType::SignedInt { .. });
        let target_signed = matches!(target, ScalarType::SignedInt { .. });

        let source_width = match source {
            ScalarType::SignedInt { width } | ScalarType::UnsignedInt { width } => width,
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: "unsupported enum cast source type".to_string(),
                }
                .into());
            }
        };
        let target_width = match target {
            ScalarType::SignedInt { width } | ScalarType::UnsignedInt { width } => width,
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: "unsupported enum cast target type".to_string(),
                }
                .into());
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

    /// Convert a value into an erased managed pointer.
    fn erased_value_pointer_for_instance(
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

    /// Allocate heap storage for a value and return a reference to it.
    pub(super) fn box_value(
        &mut self,
        value: mir::Value,
        value_type: mir::LocalNodeId<mir::Type>,
    ) -> mir::Value {
        // build the managed reference type
        let ref_type = self.state.builder.type_reference(
            mir::ReferenceKind::Managed,
            value_type,
            mir::Access::Readonly,
            mir::Space::Local,
            mir::Nullability::None,
        );

        // allocate and store the value
        let pointer = self.state.builder.new_(value_type, ref_type);
        self.state.builder.store(pointer, value);

        // return the managed reference
        pointer
    }

    /// Resolve the concrete symbol for a nominal instance type.
    fn concrete_symbol_for_type(&self, type_id: dir::LocalTypeId) -> Option<dir::GlobalSymbolId> {
        // walk the type tree to find a nominal class or struct
        match self.context.types.get_type(type_id) {
            dir::Type::Named(reference)
                if matches!(
                    self.context.symbol_form(reference.symbol),
                    Some(dir::SymbolForm::Class | dir::SymbolForm::Struct)
                ) =>
            {
                Some(reference.symbol)
            }
            dir::Type::Form(value) => self.concrete_symbol_for_type(value.value),
            dir::Type::Intersection(intersection) => intersection
                .elements
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
            dir::Expression::New { .. } => self
                .constructor_target_symbol_for_expression(expression_id)
                .ok()
                .flatten(),
            dir::Expression::StructExpression { ty, .. } => {
                let type_id = self.type_id_for_type_expression(*ty)?;
                self.concrete_symbol_for_type(type_id)
            }
            dir::Expression::Identifier { .. } | dir::Expression::QualifiedReference { .. } => {
                let symbol = self.resolve_expression_symbol(expression_id).ok()?;
                match self.context.symbol_form(symbol) {
                    Some(dir::SymbolForm::Class | dir::SymbolForm::Struct) => Some(symbol),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}
