use destack_base::StringId;
use destack_dir::{Expression, GlobalSymbolId, LocalNodeId, Resolution, StaticKey, Type};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use crate::lower::emit::FunctionContext;
use crate::lower::r#type::{
    DiscriminantKey, DiscriminantLiteral, DiscriminantValue, UnionDiscriminantField, UnionLayout,
};

impl FunctionContext<'_> {
    /// Lower a union discriminant comparison when possible.
    pub(crate) fn lower_union_discriminant_comparison(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        operator: dir::BinaryOperator,
        right: LocalNodeId<Expression>,
    ) -> LowerResult<Option<mir::Value>> {
        // only handle equality comparisons
        if !matches!(
            operator,
            dir::BinaryOperator::Equal | dir::BinaryOperator::NotEqual
        ) {
            return Ok(None);
        }

        // match member access against a scalar literal
        let (member_id, literal_value) =
            match (self.env.dir_tree.get(left), self.env.dir_tree.get(right)) {
                (Expression::Member { .. }, Expression::ScalarLiteral { value }) => (left, value),
                (Expression::ScalarLiteral { value }, Expression::Member { .. }) => (right, value),
                _ => return Ok(None),
            };

        let Expression::Member {
            left: receiver_id,
            name,
            static_arguments,
        } = self.env.dir_tree.get(member_id)
        else {
            return Ok(None);
        };

        if static_arguments.is_some() {
            return Ok(None);
        }

        // resolve discriminant metadata for the member access
        let Some((_receiver_type_id, layout, field)) =
            self.union_discriminant_field_for_member(*receiver_id, *name)
        else {
            return Ok(None);
        };

        // canonicalize the literal key
        let node = expression_id
            .into_global_any(self.env.module_id)
            .into_anchored(Some(self.env.profile));
        let key = DiscriminantKey::from_scalar_literal(literal_value, node)?.ok_or_else(|| {
            LowerError::UnsupportedConstruct {
                node,
                message: "unsupported discriminant literal in comparison".to_string(),
            }
        })?;

        let tag_index = field.tag_by_value.get(&key).copied().ok_or_else(|| {
            LowerError::UnsupportedConstruct {
                node,
                message: "discriminant literal does not match union".to_string(),
            }
        })?;

        // lower the receiver and compare tags
        let (union_value, _) = self.lower_value_expression(*receiver_id)?;
        let tag_value = self
            .state
            .builder
            .field_get(union_value, layout.tag_field_index);
        let tag_const = self.union_tag_constant(&layout, tag_index as usize)?;
        let op = match operator {
            dir::BinaryOperator::Equal => mir::BinaryOperator::Equal,
            dir::BinaryOperator::NotEqual => mir::BinaryOperator::NotEqual,
            _ => unreachable!(),
        };
        let value = self.state.builder.binary_op(op, tag_value, tag_const);

        Ok(Some(value))
    }

    /// Lower a member access expression to a field_get.
    pub(crate) fn lower_member_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        field_name: StringId,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // handle getter access using resolution
        if let Some(target_symbol) = self.resolved_member_symbol(expression_id)
            && matches!(
                self.member_mode_for_symbol(target_symbol),
                Some(destack_dir::FunctionMode::Getter)
            )
        {
            return self.lower_getter_call(expression_id, left_id, target_symbol);
        }

        // handle union discriminant field access
        if let Some((union_type_id, layout, field)) =
            self.union_discriminant_field_for_member(left_id, field_name)
        {
            return self.lower_union_discriminant_member(
                expression_id,
                left_id,
                union_type_id,
                layout,
                field,
            );
        }

        // lower the aggregate value
        let (mut aggregate_value, aggregate_type) = self.lower_value_expression(left_id)?;

        // resolve field index through the type lowerer
        let field_index = self
            .env
            .type_lowerer
            .field_index_for_type(
                aggregate_type,
                field_name,
                self.env.strings,
                self.state.builder.tree(),
            )
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "field not found in aggregate type".to_string(),
            })?;

        // ensure constructor fields are initialized before read
        if matches!(self.env.dir_tree.get(left_id), Expression::This) {
            let node = expression_id
                .into_global_any(self.env.module_id)
                .into_anchored(Some(self.env.profile));
            self.ensure_constructor_field_initialized(node, field_index as u32, field_name)?;
        }

        // load through references before field access
        let mut aggregate_type = aggregate_type;
        loop {
            let aggregate_mir_type = self.state.builder.tree().get(aggregate_type).clone();
            match aggregate_mir_type {
                mir::Type::Reference { pointee, .. } => {
                    aggregate_value = self.state.builder.load(aggregate_value, pointee);
                    aggregate_type = pointee;
                }
                _ => break,
            }
        }

        // get the result type
        let result_type = self.mir_type_for_expression(expression_id)?;

        // emit field_get
        let value = self
            .state
            .builder
            .field_get(aggregate_value, field_index as u32);
        Ok((value, result_type))
    }

    /// Resolve a union discriminant field for a member access.
    fn union_discriminant_field_for_member(
        &self,
        receiver_id: LocalNodeId<Expression>,
        field_name: StringId,
    ) -> Option<(dir::LocalTypeId, UnionLayout, UnionDiscriminantField)> {
        // resolve the receiver type
        let receiver_type_id = self.dir_type_for_expression(receiver_id)?;
        let receiver_type = self.env.types.get_type(receiver_type_id);
        if !matches!(receiver_type, Type::Union { .. }) {
            return None;
        }

        // require discriminant metadata for this union
        let layout = self
            .env
            .type_lowerer
            .union_layout(receiver_type_id)?
            .clone();
        let discriminant = layout.discriminant.as_ref()?;

        // match the member name against static keys
        let key = StaticKey::Name(field_name);
        let field = discriminant
            .fields
            .iter()
            .find(|field| field.key.matches(&key))?
            .clone();

        Some((receiver_type_id, layout, field))
    }

    /// Lower a union discriminant field access into tag selection.
    fn lower_union_discriminant_member(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        receiver_id: LocalNodeId<Expression>,
        _receiver_type_id: dir::LocalTypeId,
        layout: UnionLayout,
        field: UnionDiscriminantField,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // lower the receiver value
        let (union_value, _) = self.lower_value_expression(receiver_id)?;

        // extract the union tag
        let tag_value = self
            .state
            .builder
            .field_get(union_value, layout.tag_field_index);

        // resolve the result type and union layout
        let result_type = self.mir_type_for_expression(expression_id)?;
        let result_type_id =
            self.dir_type_for_expression(expression_id)
                .ok_or_else(|| LowerError::MissingType {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                })?;
        let result_layout = self
            .env
            .type_lowerer
            .union_layout(result_type_id)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "missing union layout for discriminant field".to_string(),
            })?;

        // allocate a result variable for tag-based selection
        let result_variable = self.state.builder.create_variable(result_type);
        let merge_block = self.state.builder.create_block();

        for (index, literal) in field.values.iter().enumerate() {
            let is_last = index + 1 == field.values.len();
            let next_block = if is_last {
                None
            } else {
                Some(self.state.builder.create_block())
            };
            let match_block = if is_last {
                None
            } else {
                Some(self.state.builder.create_block())
            };

            // compare the tag against the expected value
            if let (Some(match_block), Some(next_block)) = (match_block, next_block) {
                let tag_const = self.union_tag_constant(&layout, index)?;
                let is_match =
                    self.state
                        .builder
                        .binary_op(mir::BinaryOperator::Equal, tag_value, tag_const);
                self.state.builder.branch(is_match, match_block, next_block);

                // match block
                self.state.builder.switch_to_block(match_block);
                let value = self.build_union_value_from_discriminant_literal(
                    result_type_id,
                    result_type,
                    result_layout,
                    literal,
                )?;
                self.state.builder.define_variable(result_variable, value);
                self.state.builder.jump(merge_block);

                // continue in next block
                self.state.builder.switch_to_block(next_block);
                continue;
            }

            // final fallback uses the last value
            let value = self.build_union_value_from_discriminant_literal(
                result_type_id,
                result_type,
                result_layout,
                literal,
            )?;
            self.state.builder.define_variable(result_variable, value);
            self.state.builder.jump(merge_block);
        }

        // finish the selection
        self.state.builder.switch_to_block(merge_block);
        let result_value = self.state.builder.use_variable(result_variable);
        Ok((result_value, result_type))
    }

    /// Build a union value from a discriminant literal.
    fn build_union_value_from_discriminant_literal(
        &mut self,
        union_type_id: dir::LocalTypeId,
        union_mir_type: mir::LocalNodeId<mir::Type>,
        union_layout: &UnionLayout,
        literal: &DiscriminantLiteral,
    ) -> LowerResult<mir::Value> {
        // resolve the tag index for the literal type
        let tag_index = union_layout
            .element_types
            .iter()
            .position(|element| dir::are_types_equal(*element, literal.type_id, self.env.types))
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: self
                    .env
                    .types
                    .get_type_source(union_type_id)
                    .into_global(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "missing union element for discriminant literal".to_string(),
            })?;

        // build the literal payload
        let node = self
            .env
            .types
            .get_type_source(union_type_id)
            .into_global(self.env.module_id)
            .into_anchored(Some(self.env.profile));
        let (literal_value, literal_type) =
            self.mir_value_for_discriminant_literal(literal, node)?;

        // box the payload and cast to the union payload type
        let boxed = self.box_value(literal_value, literal_type);
        let payload = self.state.builder.bitcast(boxed, union_layout.payload_type);

        // assemble the union value
        let tag_value = self.union_tag_constant(union_layout, tag_index)?;
        let mut fields = vec![tag_value, payload];
        if union_layout.tag_field_index > union_layout.payload_field_index {
            fields.swap(0, 1);
        }
        Ok(self.state.builder.struct_(union_mir_type, fields))
    }

    /// Create a tag constant for a union layout.
    fn union_tag_constant(
        &mut self,
        layout: &UnionLayout,
        tag_index: usize,
    ) -> LowerResult<mir::Value> {
        let mir::Type::Int { width, signed } = self.state.builder.tree().get(layout.tag_type)
        else {
            return Err(LowerError::UnsupportedConstruct {
                node: self
                    .env
                    .types
                    .get_type_source(layout.element_types[0])
                    .into_global(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "union tag must be an integer type".to_string(),
            });
        };

        Ok(self
            .state
            .builder
            .iconst(tag_index as i64, *width as u8, *signed))
    }

    /// Lower a discriminant literal into a MIR value.
    fn mir_value_for_discriminant_literal(
        &mut self,
        literal: &DiscriminantLiteral,
        node: destack_dir::AnchoredGlobalNodeId,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        match &literal.value {
            DiscriminantValue::Boolean(value) => {
                let value = self.state.builder.bconst(*value);
                Ok((value, self.env.type_lowerer.ty_bool))
            }
            DiscriminantValue::Number { value } => {
                let dir_type = self.env.types.get_type(literal.type_id);
                let scalar = self.env.type_lowerer.scalar_type_for_dir_type(dir_type);
                match scalar {
                    Some(crate::lower::ScalarType::SignedInt { width }) => {
                        let value = self.state.builder.iconst(*value as i64, width as u8, true);
                        let ty = if width == 64 {
                            self.env.type_lowerer.ty_i64
                        } else {
                            self.env.type_lowerer.ty_i32
                        };
                        Ok((value, ty))
                    }
                    Some(crate::lower::ScalarType::Float { width }) => {
                        let value = self.state.builder.fconst(*value, width as u8);
                        let ty = if width == 32 {
                            self.env.type_lowerer.ty_f32
                        } else {
                            self.env.type_lowerer.ty_f64
                        };
                        Ok((value, ty))
                    }
                    _ => Err(LowerError::UnsupportedConstruct {
                        node,
                        message: "unsupported discriminant numeric literal".to_string(),
                    }),
                }
            }
            DiscriminantValue::String(value) => {
                let literal_value = self.env.strings.get(*value);
                let value = self.state.builder.sconst(literal_value.to_string());
                let ty = self.env.type_lowerer.string_type().ok_or_else(|| {
                    LowerError::UnsupportedConstruct {
                        node,
                        message: "missing builtin String layout (load lib/native)".to_string(),
                    }
                })?;
                Ok((value, ty))
            }
            DiscriminantValue::Null
            | DiscriminantValue::Undefined
            | DiscriminantValue::Bigint(_)
            | DiscriminantValue::UniqueSymbol => Err(LowerError::UnsupportedConstruct {
                node,
                message: "unsupported discriminant literal value".to_string(),
            }),
        }
    }

    /// Resolve a static member symbol for a member access expression.
    pub(crate) fn resolved_member_symbol(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<GlobalSymbolId> {
        // read the resolution from analyze
        let resolution = self.get_resolution(expression_id)?;

        match resolution {
            Resolution::Static { candidate, .. } => Some(candidate.target_symbol),
            _ => None,
        }
    }

    /// Resolve the function mode for a member symbol when available.
    pub(crate) fn member_mode_for_symbol(
        &self,
        symbol: GlobalSymbolId,
    ) -> Option<destack_dir::FunctionMode> {
        // load the module for this symbol
        let module = self.env.program.modules.get(symbol.module_id);
        let module = module.read();
        let dir = module.dir(self.env.profile);
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();

        // resolve the primary declaration node
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        let primary = symbol_entry.primary_declaration?;
        if primary.module_id != symbol.module_id {
            return None;
        }

        // handle member declarations
        if let Ok(member_id) = primary.local_id.try_into_typed::<destack_dir::Member>() {
            let member = tree.get(member_id);
            if let destack_dir::Member::Method { signature, .. } = member {
                return signature.mode;
            }
        }

        // handle property declarations
        if let Ok(property_id) = primary.local_id.try_into_typed::<destack_dir::Property>() {
            let property = tree.get(property_id);
            if let destack_dir::Property::Method { signature, .. } = property {
                return signature.mode;
            }
        }

        None
    }

    /// Lower a getter call for a resolved member access.
    pub(crate) fn lower_getter_call(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        receiver_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // resolve the receiver value
        let receiver_value = if self.receiver_is_namespace_reference(receiver_id) {
            None
        } else {
            let (value, _) = self.lower_value_expression(receiver_id)?;
            Some(value)
        };

        // get the result type
        let result_type = self.mir_type_for_expression(expression_id)?;

        // build argument list
        let mut arguments = Vec::new();
        if let Some(receiver) = receiver_value {
            arguments.push(receiver);
        }

        // use interface dispatch metadata when available
        if let (Some(receiver_type_id), Some(receiver_value)) =
            (self.dir_type_for_expression(receiver_id), receiver_value)
            && let Some((function_id, metadata)) = self.interface_dispatch_target(
                expression_id,
                receiver_type_id,
                receiver_value,
                target_symbol,
                result_type,
            )?
        {
            let value = self
                .state
                .builder
                .call_with_metadata(function_id, arguments, metadata)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "getter call returned no value".to_string(),
                })?;
            return Ok((value, result_type));
        }

        // resolve the target function
        let function_id = *self
            .env
            .functions_by_symbol
            .get(&target_symbol)
            .ok_or_else(|| LowerError::MissingFunction {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                symbol: target_symbol,
            })?;

        // emit the call
        let value = self
            .state
            .builder
            .call(function_id, arguments)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "getter call returned no value".to_string(),
            })?;

        Ok((value, result_type))
    }

    /// Lower a setter call for a resolved member assignment.
    pub(crate) fn lower_setter_call(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        receiver_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
        value: mir::Value,
    ) -> LowerResult<()> {
        // resolve the receiver value
        let receiver_value = if self.receiver_is_namespace_reference(receiver_id) {
            None
        } else {
            let (value, _) = self.lower_value_expression(receiver_id)?;
            Some(value)
        };

        // build argument list
        let mut arguments = Vec::new();
        if let Some(receiver) = receiver_value {
            arguments.push(receiver);
        }
        arguments.push(value);

        // use interface dispatch metadata when available
        if let (Some(receiver_type_id), Some(receiver_value)) =
            (self.dir_type_for_expression(receiver_id), receiver_value)
            && let Some((function_id, metadata)) = self.interface_dispatch_target(
                expression_id,
                receiver_type_id,
                receiver_value,
                target_symbol,
                self.env.type_lowerer.ty_void,
            )?
        {
            self.state
                .builder
                .call_void_with_metadata(function_id, arguments, metadata);
            return Ok(());
        }

        // resolve the target function
        let function_id = *self
            .env
            .functions_by_symbol
            .get(&target_symbol)
            .ok_or_else(|| LowerError::MissingFunction {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                symbol: target_symbol,
            })?;

        // emit the call
        self.state.builder.call_void(function_id, arguments);

        Ok(())
    }

    /// Lower an index expression to an element_get.
    pub(crate) fn lower_index_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        index_id: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // reject non array indices for native lowering
        let Some(left_type_id) = self.dir_type_for_expression(left_id) else {
            return Err(LowerError::MissingType {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
            });
        };
        match self.env.types.get_type(left_type_id) {
            Type::Array { .. } | Type::ArraySized { .. } => {}
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "index signatures are not supported for native lowering".to_string(),
                });
            }
        }

        // lower the array value and index
        let (array_value, _array_type) = self.lower_value_expression(left_id)?;
        let (index_value, _index_type) = self.lower_value_expression(index_id)?;

        // get the result type (element type)
        let result_type = self.mir_type_for_expression(expression_id)?;

        // emit element_get
        let value = self.state.builder.element_get(array_value, index_value);
        Ok((value, result_type))
    }
}
