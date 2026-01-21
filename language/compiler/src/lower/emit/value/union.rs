use destack_base::StringId;
use destack_dir::{AnchoredGlobalNodeId, CastSource, Expression, LocalNodeId, StaticKey, Type};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult, ScalarType};

use crate::lower::emit::FunctionContext;
use crate::lower::r#type::{
    DiscriminantKey, DiscriminantLiteral, DiscriminantValue, UnionDiscriminantField, UnionLayout,
    UnionPayloadKind,
};

/// Union discriminant comparison data for tag checks.
#[derive(Debug, Clone, Copy)]
pub(crate) struct UnionTagComparison {
    /// The tag value being compared.
    pub(crate) tag_value: mir::Value,
    /// The tag constant used for comparison.
    pub(crate) tag_const: mir::Value,
    /// The expected tag index.
    pub(crate) tag_index: usize,
    /// Whether the comparison expects equality.
    pub(crate) is_equal: bool,
}

impl FunctionContext<'_> {
    /// Build an inline union payload by storing the value into scratch memory.
    pub(crate) fn inline_union_payload_from_value(
        &mut self,
        payload_type: mir::LocalNodeId<mir::Type>,
        value: mir::Value,
        value_type: mir::LocalNodeId<mir::Type>,
        node: AnchoredGlobalNodeId,
    ) -> LowerResult<mir::Value> {
        // allocate payload storage on the stack
        let payload_ref_type = self.state.builder.type_reference(
            mir::ReferenceKind::Raw,
            payload_type,
            mir::Mutability::Mutable,
            mir::AddressSpace::Generic,
            false,
        );
        let payload_ptr = self
            .state
            .builder
            .stack_alloc(payload_type, payload_ref_type);

        // zero initialize the payload storage
        let payload_zero = self.inline_union_payload_zero_value(payload_type, node)?;
        self.state.builder.store(payload_ptr, payload_zero);

        // store the source value into the payload storage
        let value_ref_type = self.state.builder.type_reference(
            mir::ReferenceKind::Raw,
            value_type,
            mir::Mutability::Mutable,
            mir::AddressSpace::Generic,
            false,
        );
        let value_ptr = self.state.builder.bitcast(payload_ptr, value_ref_type);
        self.state.builder.store(value_ptr, value);

        // load the payload value
        Ok(self.state.builder.load(payload_ptr, payload_type))
    }

    /// Extract a value from an inline union payload.
    pub(crate) fn inline_union_payload_to_value(
        &mut self,
        payload_type: mir::LocalNodeId<mir::Type>,
        payload_value: mir::Value,
        target_type: mir::LocalNodeId<mir::Type>,
        _node: AnchoredGlobalNodeId,
    ) -> LowerResult<mir::Value> {
        // allocate payload storage on the stack
        let payload_ref_type = self.state.builder.type_reference(
            mir::ReferenceKind::Raw,
            payload_type,
            mir::Mutability::Mutable,
            mir::AddressSpace::Generic,
            false,
        );
        let payload_ptr = self
            .state
            .builder
            .stack_alloc(payload_type, payload_ref_type);

        // store the payload into the scratch memory
        self.state.builder.store(payload_ptr, payload_value);

        // load the target value from the payload storage
        let target_ref_type = self.state.builder.type_reference(
            mir::ReferenceKind::Raw,
            target_type,
            mir::Mutability::Mutable,
            mir::AddressSpace::Generic,
            false,
        );
        let target_ptr = self.state.builder.bitcast(payload_ptr, target_ref_type);
        Ok(self.state.builder.load(target_ptr, target_type))
    }

    /// Build a zero value for inline payload storage.
    fn inline_union_payload_zero_value(
        &mut self,
        payload_type: mir::LocalNodeId<mir::Type>,
        node: AnchoredGlobalNodeId,
    ) -> LowerResult<mir::Value> {
        let (element, length) = match self.state.builder.tree().get(payload_type) {
            mir::Type::Array {
                element, length, ..
            } => (*element, *length),
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node,
                    message: "inline union payload must be an array".to_string(),
                });
            }
        };

        // create a zero element value
        let zero_element = self.inline_union_payload_zero_element(element, node)?;
        let elements = vec![zero_element; length as usize];

        Ok(self.state.builder.array(payload_type, elements))
    }

    /// Build a zero element for inline payload arrays.
    fn inline_union_payload_zero_element(
        &mut self,
        element_type: mir::LocalNodeId<mir::Type>,
        node: AnchoredGlobalNodeId,
    ) -> LowerResult<mir::Value> {
        let element = self.state.builder.tree().get(element_type);
        match element {
            mir::Type::Int {
                width,
                is_signed: signed,
            } => Ok(self.state.builder.iconst(0, *width as u8, *signed)),
            mir::Type::Usize => {
                let width = self.env.type_lowerer.pointer_width_bits() as u8;
                let zero = self.state.builder.iconst(0, width, false);
                Ok(self.state.builder.bitcast(zero, element_type))
            }
            mir::Type::Isize => {
                let width = self.env.type_lowerer.pointer_width_bits() as u8;
                let zero = self.state.builder.iconst(0, width, true);
                Ok(self.state.builder.bitcast(zero, element_type))
            }
            mir::Type::Boolean => Ok(self.state.builder.bconst(false)),
            _ => Err(LowerError::UnsupportedConstruct {
                node,
                message: "inline union payload element must be scalar".to_string(),
            }),
        }
    }

    /// Resolve a union tag comparison for a discriminant check.
    pub(crate) fn union_tag_comparison(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        operator: dir::BinaryOperator,
        right: LocalNodeId<Expression>,
    ) -> LowerResult<Option<UnionTagComparison>> {
        // only handle equality comparisons
        if !matches!(
            operator,
            dir::BinaryOperator::Equal
                | dir::BinaryOperator::NotEqual
                | dir::BinaryOperator::EqualStrict
                | dir::BinaryOperator::NotEqualStrict
        ) {
            return Ok(None);
        }

        // unwrap implicit casts and parens before matching
        let unwrap_expression = |mut expr_id: LocalNodeId<Expression>| {
            loop {
                match self.env.dir_tree.get(expr_id) {
                    Expression::Parenthesized { expression } => expr_id = *expression,
                    Expression::Cast {
                        value,
                        source: CastSource::Implicit,
                        ..
                    } => expr_id = *value,
                    _ => return expr_id,
                }
            }
        };
        let left = unwrap_expression(left);
        let right = unwrap_expression(right);

        // match member access against a scalar literal
        let (member_id, literal_value) =
            match (self.env.dir_tree.get(left), self.env.dir_tree.get(right)) {
                (Expression::Member { .. }, Expression::ScalarLiteral { value }) => (left, value),
                (Expression::ScalarLiteral { value }, Expression::Member { .. }) => (right, value),
                _ => return Ok(None),
            };

        // extract the member access expression
        let Expression::Member {
            left: receiver_id,
            name,
            static_arguments,
        } = self.env.dir_tree.get(member_id)
        else {
            return Ok(None);
        };

        // reject static arguments on discriminant access
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

        // lower the receiver and resolve the tag constant
        let (union_value, _) = self.lower_value_expression(*receiver_id)?;
        let tag_value = self
            .state
            .builder
            .field_get(union_value, layout.tag_field_index);
        let tag_const = self.union_tag_constant(&layout, tag_index as usize)?;

        // determine comparison polarity
        let is_equal = matches!(
            operator,
            dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict
        );

        Ok(Some(UnionTagComparison {
            tag_value,
            tag_const,
            tag_index: tag_index as usize,
            is_equal,
        }))
    }

    /// Lower a union discriminant comparison when possible.
    pub(crate) fn lower_union_discriminant_comparison(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        operator: dir::BinaryOperator,
        right: LocalNodeId<Expression>,
    ) -> LowerResult<Option<mir::Value>> {
        // resolve the tag comparison data
        let Some(comparison) = self.union_tag_comparison(expression_id, left, operator, right)?
        else {
            return Ok(None);
        };

        // emit the comparison as a boolean value
        let op = if comparison.is_equal {
            mir::BinaryOperator::Equal
        } else {
            mir::BinaryOperator::NotEqual
        };
        let value = self
            .state
            .builder
            .binary_op(op, comparison.tag_value, comparison.tag_const);

        Ok(Some(value))
    }

    /// Lower a union tag comparison as a check terminator when possible.
    pub(crate) fn lower_union_tag_check(
        &mut self,
        condition_id: LocalNodeId<Expression>,
        then_block: mir::LocalNodeId<mir::Block>,
        else_block: mir::LocalNodeId<mir::Block>,
    ) -> LowerResult<bool> {
        // unwrap implicit casts and parens before matching
        let unwrap_expression = |mut expr_id: LocalNodeId<Expression>| {
            loop {
                match self.env.dir_tree.get(expr_id) {
                    Expression::Parenthesized { expression } => expr_id = *expression,
                    Expression::Cast {
                        value,
                        source: CastSource::Implicit,
                        ..
                    } => expr_id = *value,
                    _ => return expr_id,
                }
            }
        };
        let condition_id = unwrap_expression(condition_id);

        // match on binary expressions
        let Expression::Binary {
            left,
            operator,
            right,
        } = self.env.dir_tree.get(condition_id)
        else {
            return Ok(false);
        };

        // resolve the union tag comparison
        let Some(comparison) = self.union_tag_comparison(condition_id, *left, *operator, *right)?
        else {
            return Ok(false);
        };

        // build an equality condition for the union tag
        let condition_value = self.state.builder.binary_op(
            mir::BinaryOperator::Equal,
            comparison.tag_value,
            comparison.tag_const,
        );
        let constraint = mir::CheckConstraint::Union {
            value: comparison.tag_value,
            expected: comparison.tag_index as u64,
        };

        // swap branches for inequality comparisons
        let (success_block, failure_block) = if comparison.is_equal {
            (then_block, else_block)
        } else {
            (else_block, then_block)
        };
        self.state
            .builder
            .check(condition_value, constraint, success_block, failure_block);

        Ok(true)
    }

    /// Resolve a union discriminant field for a member access.
    pub(crate) fn union_discriminant_field_for_member(
        &self,
        receiver_id: LocalNodeId<Expression>,
        field_name: StringId,
    ) -> Option<(dir::LocalTypeId, UnionLayout, UnionDiscriminantField)> {
        // resolve the receiver type
        let receiver_type_id = self.type_for_expression(receiver_id)?;
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
    pub(crate) fn lower_union_discriminant_member(
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
        let result_type = self.lower_type_for_expression(expression_id)?;
        let result_type_id = self.type_for_expression_or_error(expression_id)?;
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

        // allocate a result variable for tag based selection
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
            self.lower_value_for_discriminant_literal(literal, node)?;

        // build the payload for the union
        let payload = match union_layout.payload_kind {
            UnionPayloadKind::Inline => self.inline_union_payload_from_value(
                union_layout.payload_type,
                literal_value,
                literal_type,
                node,
            )?,
            UnionPayloadKind::Boxed => {
                let boxed = self.box_value(literal_value, literal_type);
                self.state.builder.bitcast(boxed, union_layout.payload_type)
            }
        };

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
        let mir::Type::Int {
            width,
            is_signed: signed,
        } = self.state.builder.tree().get(layout.tag_type)
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
    fn lower_value_for_discriminant_literal(
        &mut self,
        literal: &DiscriminantLiteral,
        node: dir::AnchoredGlobalNodeId,
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
                    Some(ScalarType::SignedInt { width }) => {
                        let value = self.state.builder.iconst(*value as i64, width as u8, true);
                        let ty = if width == 64 {
                            self.env.type_lowerer.ty_i64
                        } else {
                            self.env.type_lowerer.ty_i32
                        };
                        Ok((value, ty))
                    }
                    Some(ScalarType::Float { width }) => {
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
}
