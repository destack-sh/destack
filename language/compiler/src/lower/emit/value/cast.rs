use destack_dir::{Expression, LocalNodeId};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult, ScalarType};

use crate::lower::emit::FunctionContext;

#[allow(clippy::too_many_arguments)]
impl FunctionContext<'_> {
    /// Lower a cast operator into a MIR cast operator.
    pub(crate) fn lower_cast_operator(
        &self,
        expression_id: LocalNodeId<Expression>,
        operator: dir::CastOperator,
        value_id: LocalNodeId<Expression>,
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
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
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
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
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
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "unsupported float to int cast".to_string(),
                    })?;
                }
            },
            dir::CastOperator::PointerToInt => mir::CastOperator::PointerToInt,
            dir::CastOperator::IntToPointer => mir::CastOperator::IntToPointer,
            dir::CastOperator::PointerCast => mir::CastOperator::Bitcast,
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: format!("unsupported cast operator '{operator:?}'"),
                })?;
            }
        };

        // return the resolved operator
        Ok(Some(cast_operator))
    }

    /// Lower an instance upcast, including interface upcasts.
    pub(crate) fn lower_instance_upcast(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // lower the source value and target type
        let (value, source_mir_type) = self.lower_value_expression(value_id)?;
        let target_mir_type = self.mir_type_for_expression(expression_id)?;

        // resolve the source and target dir types
        let source_type_id = self.dir_type_for_expression_or_error(value_id)?;
        let target_type_id = self.dir_type_for_expression_or_error(expression_id)?;
        let target_dir_type = self.env.types.get_type(target_type_id);

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

        // bitcast references to the target type
        let source_type = self.state.builder.tree().get(source_mir_type).clone();
        if let mir::Type::Reference { .. } = source_type {
            let value = self.state.builder.bitcast(value, target_mir_type);
            return Ok((value, target_mir_type));
        }

        // return the value as the target type
        Ok((value, target_mir_type))
    }

    /// Lower an instance downcast as an unchecked conversion.
    pub(crate) fn lower_instance_downcast(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // lower the source value and target type
        let (value, source_mir_type) = self.lower_value_expression(value_id)?;
        let target_mir_type = self.mir_type_for_expression(expression_id)?;

        // resolve the source dir type
        let source_type_id = self.dir_type_for_expression_or_error(value_id)?;
        let source_dir_type = self.env.types.get_type(source_type_id);

        // handle interface downcasts by extracting object pointers
        if let dir::Type::Reference { symbol, .. } = source_dir_type
            && symbol.ty() == dir::SymbolType::Interface
        {
            // resolve interface reference layout
            let layout = self
                .env
                .type_lowerer
                .interface_ref_layout(source_type_id)
                .ok_or_else(|| LowerError::MissingType {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                })?;

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

        // bitcast references to the target type
        let source_type = self.state.builder.tree().get(source_mir_type).clone();
        if let mir::Type::Reference { .. } = source_type {
            let value = self.state.builder.bitcast(value, target_mir_type);
            return Ok((value, target_mir_type));
        }

        // return the value as the target type
        Ok((value, target_mir_type))
    }

    /// Lower a union upcast into a tagged boxed union value.
    pub(crate) fn lower_union_upcast(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // lower the source value and target type
        let (value, source_mir_type) = self.lower_value_expression(value_id)?;
        let target_mir_type = self.mir_type_for_expression(expression_id)?;

        // resolve the source and target dir types
        let source_type_id = self.dir_type_for_expression_or_error(value_id)?;
        let target_type_id = self.dir_type_for_expression_or_error(expression_id)?;

        // resolve union layout metadata
        let layout = self
            .env
            .type_lowerer
            .union_layout(target_type_id)
            .ok_or_else(|| LowerError::MissingType {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
            })?;

        // skip when the source is already the target union type
        if dir::are_types_equal(source_type_id, target_type_id, self.env.types) {
            return Ok((value, target_mir_type));
        }

        // resolve the union tag index for the source type
        let tag_index = layout
            .element_types
            .iter()
            .position(|element| dir::are_types_equal(*element, source_type_id, self.env.types))
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "union upcast missing matching element".to_string(),
            })?;

        // build the tag constant
        let (tag_width, tag_signed) = match self.state.builder.tree().get(layout.tag_type) {
            mir::Type::Int { width, signed } => (*width as u8, *signed),
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "union tag must be an integer type".to_string(),
                });
            }
        };
        let tag_value = self
            .state
            .builder
            .iconst(tag_index as i64, tag_width, tag_signed);

        // box the payload and cast to the payload field type
        let boxed = self.box_value(value, source_mir_type);
        let payload = self.state.builder.bitcast(boxed, layout.payload_type);

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
    pub(crate) fn lower_union_downcast(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // lower the source value and target type
        let (value, _source_mir_type) = self.lower_value_expression(value_id)?;
        let target_mir_type = self.mir_type_for_expression(expression_id)?;

        // resolve the source dir type
        let source_type_id = self.dir_type_for_expression_or_error(value_id)?;

        // resolve union layout metadata
        let layout = self
            .env
            .type_lowerer
            .union_layout(source_type_id)
            .ok_or_else(|| LowerError::MissingType {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
            })?;

        // extract the payload pointer
        let payload = self
            .state
            .builder
            .field_get(value, layout.payload_field_index);

        // load the payload as the target type
        let reference_type = self.state.builder.type_reference(
            mir::ReferenceKind::Managed,
            target_mir_type,
            mir::Mutability::Immutable,
            mir::AddressSpace::Generic,
            false,
        );
        let casted = self.state.builder.bitcast(payload, reference_type);
        let value = self.state.builder.load(casted, target_mir_type);

        // return the payload value and type
        Ok((value, target_mir_type))
    }

    /// Build an interface reference from a concrete value.
    pub(crate) fn lower_interface_upcast(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        value: mir::Value,
        source_mir_type: mir::LocalNodeId<mir::Type>,
        source_type_id: dir::LocalTypeId,
        interface_symbol: dir::GlobalSymbolId,
        target_type_id: dir::LocalTypeId,
        target_mir_type: mir::LocalNodeId<mir::Type>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // resolve interface reference layout
        let layout = self
            .env
            .type_lowerer
            .interface_ref_layout(target_type_id)
            .ok_or_else(|| LowerError::MissingType {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
            })?;

        // resolve the concrete symbol for the source type
        let source_dir_type = self.env.types.get_type(source_type_id);
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
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "FUGU #Broken: interface to interface upcast requires RTTI"
                        .to_string(),
                });
            }

            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "interface upcast requires a concrete symbol".to_string(),
            });
        };

        // resolve the itab id for the concrete and interface pair
        let itab_id = self
            .env
            .interface_itab_ids
            .get(&(concrete_symbol, interface_symbol))
            .copied()
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "missing interface itab for concrete type".to_string(),
            })?;

        // convert the source value into an object pointer
        let object_ptr =
            self.object_pointer_for_instance(value, source_mir_type, layout.object_type);

        // encode the itab id as a pointer sized value
        let tag_width = self.env.type_lowerer.pointer_width_bits() as u8;
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
        match self.env.types.get_type(type_id) {
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
        expression_id: LocalNodeId<Expression>,
    ) -> Option<dir::GlobalSymbolId> {
        // peel parenthesized expressions
        let expression = self.env.dir_tree.get(expression_id);
        match expression {
            Expression::Parenthesized { expression } => {
                self.concrete_symbol_for_expression(*expression)
            }
            Expression::Cast { value, .. } => self.concrete_symbol_for_expression(*value),
            Expression::New { left, .. } => self.concrete_symbol_for_expression(*left),
            Expression::TaggedScalarExpression { ty, .. }
            | Expression::TaggedTupleExpression { ty, .. }
            | Expression::TaggedObjectExpression { ty, .. } => {
                let type_id = self.type_id_for_type_expression(*ty)?;
                self.concrete_symbol_for_type(type_id)
            }
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => match target_symbol.ty() {
                dir::SymbolType::Class | dir::SymbolType::Struct => Some(*target_symbol),
                _ => None,
            },
            _ => None,
        }
    }
}
