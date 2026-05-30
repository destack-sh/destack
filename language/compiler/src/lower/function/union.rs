use destack_core::StringId;
use {destack_dir as dir, destack_mir as mir};

use crate::{CompilerError, CompilerResult, LowerError, ScalarType};

use crate::lower::FunctionLowerer;
use crate::lower::r#type::{
    DiscriminantKey, DiscriminantLiteral, DiscriminantValue, UnionDiscriminantField, UnionLayout,
    VariantStorage,
};

/// Literal values used for union literal comparisons.
#[derive(Debug, Clone)]
enum UnionLiteralValue {
    /// Scalar literal value.
    Scalar(dir::ScalarLiteral),
    /// Null literal value.
    Null,
    /// Undefined literal value.
    Undefined,
}

/// Discriminant literal values used in comparisons.
#[derive(Debug, Clone, Copy)]
enum DiscriminantLiteralValue<'a> {
    /// Scalar literal comparison.
    Scalar(&'a dir::ScalarLiteral),
    /// Type literal comparison.
    Type(&'a dir::TypeLiteral),
}

/// Union discriminant comparison data for tag checks.
#[derive(Debug, Clone)]
pub(crate) struct UnionTagComparison {
    /// The tag value being compared.
    pub(crate) tag_value: mir::Value,
    /// The tag constant used for comparison.
    pub(crate) tag_const: mir::Value,
    /// The expected tag constant.
    pub(crate) expected: mir::Constant,
    /// Whether the comparison expects equality.
    pub(crate) is_equal: bool,
}

impl FunctionLowerer<'_> {
    /// Build zero storage for a union variant without data.
    pub(crate) fn union_storage_zero_value(
        &mut self,
        layout: &UnionLayout,
        node: dir::AnchoredGlobalNodeId,
    ) -> CompilerResult<mir::Value> {
        // build the zero value based on the storage strategy
        match layout.storage {
            VariantStorage::Inline => {
                self.inline_union_storage_zero_value(layout.storage_type, node)
            }
            VariantStorage::Boxed => self.zero_value_for_type(layout.storage_type, node),
        }
    }

    /// Build an inline union storage by storing the value into scratch memory.
    pub(crate) fn inline_union_storage_from_value(
        &mut self,
        storage_type: mir::LocalNodeId<mir::Type>,
        value: mir::Value,
        value_type: mir::LocalNodeId<mir::Type>,
        node: dir::AnchoredGlobalNodeId,
    ) -> CompilerResult<mir::Value> {
        // allocate storage on the stack
        let storage_ref_type = self.state.builder.type_reference(
            mir::ReferenceKind::Raw,
            storage_type,
            mir::Access::Mutable,
            mir::Space::Frame,
            mir::Nullability::None,
        );
        let storage_pointer = self
            .state
            .builder
            .frame_alloc_zeroed(storage_type, storage_ref_type);

        // zero initialize the storage
        let storage_zero = self.inline_union_storage_zero_value(storage_type, node)?;
        self.state.builder.store(storage_pointer, storage_zero);

        // store the source value into the storage
        let value_ref_type = self.state.builder.type_reference(
            mir::ReferenceKind::Raw,
            value_type,
            mir::Access::Mutable,
            mir::Space::Frame,
            mir::Nullability::None,
        );
        let value_ptr = self.state.builder.bitcast(storage_pointer, value_ref_type);
        self.state.builder.store(value_ptr, value);

        // load the storage value
        Ok(self.state.builder.load(storage_pointer, storage_type))
    }

    /// Extract a value from an inline union storage.
    pub(crate) fn inline_union_storage_to_value(
        &mut self,
        storage_type: mir::LocalNodeId<mir::Type>,
        storage_value: mir::Value,
        target_type: mir::LocalNodeId<mir::Type>,
        _node: dir::AnchoredGlobalNodeId,
    ) -> CompilerResult<mir::Value> {
        // use stack scratch storage for storage reinterpretation
        let storage_ref_type = self.state.builder.type_reference(
            mir::ReferenceKind::Raw,
            storage_type,
            mir::Access::Mutable,
            mir::Space::Frame,
            mir::Nullability::None,
        );
        let storage_pointer = self
            .state
            .builder
            .frame_alloc_zeroed(storage_type, storage_ref_type);

        // store the storage into the scratch memory
        self.state.builder.store(storage_pointer, storage_value);

        // load the target value from the storage
        let target_ref_type = self.state.builder.type_reference(
            mir::ReferenceKind::Raw,
            target_type,
            mir::Access::Mutable,
            mir::Space::Frame,
            mir::Nullability::None,
        );
        let target_ptr = self.state.builder.bitcast(storage_pointer, target_ref_type);
        Ok(self.state.builder.load(target_ptr, target_type))
    }

    /// Build a union value from a concrete variant value.
    pub(crate) fn union_value_from_variant(
        &mut self,
        layout: &UnionLayout,
        union_type: mir::LocalNodeId<mir::Type>,
        variant_type_id: dir::LocalTypeId,
        variant_value: mir::Value,
        variant_mir_type: mir::LocalNodeId<mir::Type>,
        node: dir::AnchoredGlobalNodeId,
    ) -> CompilerResult<mir::Value> {
        // resolve the tag index for the variant
        let tag_index = layout
            .source_types
            .iter()
            .position(|element| *element == variant_type_id)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(node),
                message: "union variant is not a member of the union type".to_string(),
            })
            .map_err(CompilerError::from)?;

        // build the tag constant
        let tag_value = self.union_tag_constant(layout, tag_index)?;

        // build the union storage
        let storage = match layout.storage {
            VariantStorage::Inline => self.inline_union_storage_from_value(
                layout.storage_type,
                variant_value,
                variant_mir_type,
                node,
            )?,
            VariantStorage::Boxed => {
                let boxed = self.box_value(variant_value, variant_mir_type);
                self.state.builder.bitcast(boxed, layout.storage_type)
            }
        };

        // assemble the union value
        let mut fields = vec![tag_value, storage];
        if layout.tag_field_index > layout.storage_field_index {
            fields.swap(0, 1);
        }
        Ok(self.state.builder.struct_(union_type, fields))
    }

    /// Build a zero value for inline storage.
    fn inline_union_storage_zero_value(
        &mut self,
        storage_type: mir::LocalNodeId<mir::Type>,
        node: dir::AnchoredGlobalNodeId,
    ) -> CompilerResult<mir::Value> {
        let (element, length) = match self.state.builder.tree().get(storage_type) {
            mir::Type::Array {
                element, length, ..
            } => (
                element
                    .ty()
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(node),
                        message: "inline union element type is not concrete".to_string(),
                    })
                    .map_err(CompilerError::from)?,
                *length,
            ),
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(node),
                    message: "inline union storage must be an array".to_string(),
                }
                .into());
            }
        };

        // create a zero element value
        let zero_element = self.inline_union_storage_zero_element(element, node)?;
        let elements = vec![zero_element; length as usize];

        Ok(self.state.builder.array(storage_type, elements))
    }

    /// Build a zero element for inline storage arrays.
    fn inline_union_storage_zero_element(
        &mut self,
        element_type: mir::LocalNodeId<mir::Type>,
        node: dir::AnchoredGlobalNodeId,
    ) -> CompilerResult<mir::Value> {
        let element = self.state.builder.tree().get(element_type);
        match element {
            mir::Type::Int {
                width,
                is_signed: signed,
            } => Ok(self.state.builder.iconst(0, *width, *signed)),
            mir::Type::Usize => {
                let width = self.context.type_lowerer.pointer_width_bits();
                let zero = self.state.builder.iconst(0, width, false);
                Ok(self.state.builder.bitcast(zero, element_type))
            }
            mir::Type::Isize => {
                let width = self.context.type_lowerer.pointer_width_bits();
                let zero = self.state.builder.iconst(0, width, true);
                Ok(self.state.builder.bitcast(zero, element_type))
            }
            mir::Type::Boolean => Ok(self.state.builder.bconst(false)),
            _ => Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(node),
                message: "inline union storage element must be scalar".to_string(),
            }
            .into()),
        }
    }

    /// Resolve a union tag comparison for a discriminant check.
    pub(crate) fn union_tag_comparison(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<UnionTagComparison>> {
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
        let left = self.unwrap_expression(left);
        let right = self.unwrap_expression(right);

        // match member access against a literal
        let literal_value = if let Some(literal) = self.discriminant_literal_for_expression(left)
            && self.is_member_expression(right)
        {
            (right, literal)
        } else if self.is_member_expression(left)
            && let Some(literal) = self.discriminant_literal_for_expression(right)
        {
            (left, literal)
        } else {
            return Ok(None);
        };

        let (member_id, literal_value) = literal_value;

        // extract the member access expression
        let (dir::Expression::Member {
            left: receiver_id,
            name,
        }
        | dir::Expression::PrivateMember {
            left: receiver_id,
            name,
        }) = self.context.dir_tree.get(member_id)
        else {
            return Ok(None);
        };

        // resolve discriminant metadata for the member access
        let Some(name) = *name else {
            return Ok(None);
        };
        let Some((_receiver_type_id, layout, field)) =
            self.union_discriminant_field_for_member(*receiver_id, name)
        else {
            return Ok(None);
        };

        // canonicalize the literal key
        let node = expression_id
            .into_global_any(self.context.module_id)
            .into_anchored(Some(self.context.profile));
        let key = match literal_value {
            DiscriminantLiteralValue::Scalar(literal) => {
                DiscriminantKey::from_scalar_literal(literal, self.diagnostic_anchor(node))?
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(node),
                        message: "unsupported discriminant literal in comparison".to_string(),
                    })?
            }
            DiscriminantLiteralValue::Type(literal) => match literal {
                dir::TypeLiteral::Null => DiscriminantKey::Null,
                dir::TypeLiteral::Undefined => DiscriminantKey::Undefined,
                _ => {
                    return Ok(None);
                }
            },
        };

        let tag_index = field.tag_by_value.get(&key).copied().ok_or_else(|| {
            LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(node),
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
            expected: self.union_tag_constant_value(&layout, tag_index as usize)?,
            is_equal,
        }))
    }

    /// Lower a union discriminant comparison when possible.
    ///
    /// ```ds
    /// function isReady(value: { kind: true; } | { kind: false; }): boolean {
    ///     return value.kind == true;
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: uint8 = field.get v0, 0
    /// v2: uint8 = const 0uint8
    /// v3: bool = int.eq v1, v2
    /// ```
    pub(crate) fn lower_union_discriminant_comparison(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<mir::Value>> {
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

    /// Lower a union literal comparison when possible.
    ///
    /// ```ds
    /// function isOne(value: 1 | 2): boolean {
    ///     return value == 1;
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: uint8 = field.get v0, 0
    /// v2: uint8 = const 0uint8
    /// v3: bool = int.eq v1, v2
    /// ```
    pub(crate) fn lower_union_literal_comparison(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<mir::Value>> {
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
        let left = self.unwrap_expression(left);
        let right = self.unwrap_expression(right);

        // resolve union operand
        let left_type_id = self.type_for_expression_or_error(left)?;
        let right_type_id = self.type_for_expression_or_error(right)?;
        let (union_expr, literal_expr_id, union_type_id) = if matches!(
            self.context.types.get_type(left_type_id),
            dir::Type::Union(_)
        ) {
            (left, right, left_type_id)
        } else if matches!(
            self.context.types.get_type(right_type_id),
            dir::Type::Union(_)
        ) {
            (right, left, right_type_id)
        } else {
            return Ok(None);
        };

        // resolve literal expression
        let Some(literal_value) = self.union_literal_value(literal_expr_id) else {
            return Ok(None);
        };

        // ensure the union layout is available
        let union_mir_type = self.lower_type_for_expression(union_expr)?;
        let layout = match self.context.type_lowerer.union_layout(union_type_id) {
            Some(layout) => layout,
            None => {
                let union_mir_type = self.state.builder.tree().get(union_mir_type);
                if matches!(
                    union_mir_type,
                    mir::Type::Reference { .. } | mir::Type::TensorView { .. }
                ) {
                    return Ok(None);
                }
                return Err(self.missing_type_error(expression_id).into());
            }
        };

        // resolve the union tag index for the literal element
        let node = expression_id
            .into_global_any(self.context.module_id)
            .into_anchored(Some(self.context.profile));
        let tag_index = self.union_tag_index_for_literal(layout, &literal_value, node)?;

        // lower the union value
        let (union_value, _) = self.lower_value_expression(union_expr)?;
        let tag_value = self
            .state
            .builder
            .field_get(union_value, layout.tag_field_index);
        let tag_const = self.union_tag_constant(layout, tag_index)?;

        // emit the comparison
        let op = if matches!(
            operator,
            dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict
        ) {
            mir::BinaryOperator::Equal
        } else {
            mir::BinaryOperator::NotEqual
        };
        let value = self.state.builder.binary_op(op, tag_value, tag_const);

        Ok(Some(value))
    }

    /// Resolve a union literal value when possible.
    fn union_literal_value(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<UnionLiteralValue> {
        // resolve literal expressions used in union comparisons
        let expression = self.context.dir_tree.get(expression_id);
        match expression {
            dir::Expression::ScalarLiteral(dir::ScalarLiteral::Null) => {
                Some(UnionLiteralValue::Null)
            }
            dir::Expression::ScalarLiteral(value) => Some(UnionLiteralValue::Scalar(value.clone())),
            dir::Expression::Type { .. } => {
                let type_id = self.type_for_expression(expression_id)?;
                match self.context.types.get_type(type_id) {
                    dir::Type::Null => Some(UnionLiteralValue::Null),
                    dir::Type::Undefined => Some(UnionLiteralValue::Undefined),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Return whether one expression is a member access.
    fn is_member_expression(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        matches!(
            self.context.dir_tree.get(expression_id),
            dir::Expression::Member { .. } | dir::Expression::PrivateMember { .. }
        )
    }

    /// Resolve a literal expression usable in discriminant comparisons.
    fn discriminant_literal_for_expression(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<DiscriminantLiteralValue<'_>> {
        match self.context.dir_tree.get(expression_id) {
            dir::Expression::ScalarLiteral(value) => Some(DiscriminantLiteralValue::Scalar(value)),
            dir::Expression::Type { value } => match self.context.dir_tree.get(*value) {
                dir::TypeExpression::Literal { value } => {
                    Some(DiscriminantLiteralValue::Type(value))
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// Resolve the union tag index for a literal element.
    fn union_tag_index_for_literal(
        &self,
        layout: &UnionLayout,
        literal: &UnionLiteralValue,
        node: dir::AnchoredGlobalNodeId,
    ) -> CompilerResult<usize> {
        let tag_index = layout
            .source_types
            .iter()
            .position(
                |element| match (self.context.types.get_type(*element), literal) {
                    (dir::Type::Literal(value), UnionLiteralValue::Scalar(literal)) => {
                        value == literal
                    }
                    (dir::Type::Null, UnionLiteralValue::Null) => true,
                    (dir::Type::Undefined, UnionLiteralValue::Undefined) => true,
                    _ => false,
                },
            )
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(node),
                message: "union literal does not match any element".to_string(),
            })
            .map_err(CompilerError::from)?;

        Ok(tag_index)
    }

    /// Lower a union tag comparison as a check terminator when possible.
    ///
    /// ```ds
    /// function pick(value: { kind: true; } | { kind: false; }): int32 {
    ///     if (value.kind == true) {
    ///         return 1;
    ///     }
    ///     return 2;
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: uint8 = field.get v0, 0
    /// v2: uint8 = const 0uint8
    /// check v1 == v2, union(tag=0) ? bb1 : bb2
    /// ```
    pub(crate) fn lower_union_tag_check(
        &mut self,
        condition_id: dir::LocalNodeId<dir::Expression>,
        then_block: mir::LocalNodeId<mir::Block>,
        else_block: mir::LocalNodeId<mir::Block>,
    ) -> CompilerResult<bool> {
        // unwrap implicit casts and parens before matching
        let condition_id = self.unwrap_expression(condition_id);

        // match on binary expressions
        let dir::Expression::Binary {
            left,
            operator,
            right,
        } = self.context.dir_tree.get(condition_id)
        else {
            return Ok(false);
        };

        // resolve the union tag comparison
        let Some(comparison) = self.union_tag_comparison(condition_id, *left, *operator, *right)?
        else {
            return Ok(false);
        };

        let constraint = mir::CheckConstraint::Variant {
            value: comparison.tag_value.into(),
            expected: comparison.expected,
        };

        // swap branches for inequality comparisons
        let (success_block, failure_block) = if comparison.is_equal {
            (then_block, else_block)
        } else {
            (else_block, then_block)
        };
        self.state
            .builder
            .check(constraint, success_block, failure_block);

        Ok(true)
    }

    /// Resolve a union discriminant field for a member access.
    pub(crate) fn union_discriminant_field_for_member(
        &self,
        receiver_id: dir::LocalNodeId<dir::Expression>,
        field_name: StringId,
    ) -> Option<(dir::LocalTypeId, UnionLayout, UnionDiscriminantField)> {
        // resolve the receiver type
        let receiver_type_id = self.type_for_expression(receiver_id)?;
        let receiver_type = self.context.types.get_type(receiver_type_id);
        if !matches!(receiver_type, dir::Type::Union(_)) {
            return None;
        }

        // require discriminant metadata for this union
        let layout = self
            .context
            .type_lowerer
            .union_layout(receiver_type_id)?
            .clone();
        let discriminant = layout.discriminant.as_ref()?;

        // match the member name against static keys
        let key = dir::StaticKey::Name(field_name);
        let field = discriminant
            .fields
            .iter()
            .find(|field| field.key.matches(&key))?
            .clone();

        Some((receiver_type_id, layout, field))
    }

    /// Lower a union discriminant field access into tag selection.
    ///
    /// ```ds
    /// function read(value: { kind: true; } | { kind: false; }): true | false {
    ///     return value.kind;
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: u8 = field.get v0, 0
    /// v2: bool = <tag_to_literal>
    /// ```
    pub(crate) fn lower_union_discriminant_member(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        receiver_id: dir::LocalNodeId<dir::Expression>,
        _receiver_type_id: dir::LocalTypeId,
        layout: UnionLayout,
        field: UnionDiscriminantField,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
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
            .context
            .type_lowerer
            .union_layout(result_type_id)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "missing union layout for discriminant field".to_string(),
            })
            .map_err(CompilerError::from)?;

        // allocate a result variable for tag based selection
        let result_variable = self.state.builder.variable(result_type);
        let merge_block = self.state.builder.block();

        for (index, literal) in field.values.iter().enumerate() {
            let is_last = index + 1 == field.values.len();
            let next_block = if is_last {
                None
            } else {
                Some(self.state.builder.block())
            };
            let match_block = if is_last {
                None
            } else {
                Some(self.state.builder.block())
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
    ) -> CompilerResult<mir::Value> {
        // resolve the tag index for the literal type
        let tag_index = union_layout
            .source_types
            .iter()
            .position(|element| self.type_ids_equivalent(*element, literal.type_id))
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    self.context
                        .types
                        .get_type_source(union_type_id)
                        .into_global(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "missing union element for discriminant literal".to_string(),
            })
            .map_err(CompilerError::from)?;

        // build the literal storage
        let node = self
            .context
            .types
            .get_type_source(union_type_id)
            .into_global(self.context.module_id)
            .into_anchored(Some(self.context.profile));
        let (literal_value, literal_type) =
            self.lower_value_for_discriminant_literal(literal, node)?;

        // build the storage for the union
        let storage = match union_layout.storage {
            VariantStorage::Inline => self.inline_union_storage_from_value(
                union_layout.storage_type,
                literal_value,
                literal_type,
                node,
            )?,
            VariantStorage::Boxed => {
                let boxed = self.box_value(literal_value, literal_type);
                self.state.builder.bitcast(boxed, union_layout.storage_type)
            }
        };

        // assemble the union value
        let tag_value = self.union_tag_constant(union_layout, tag_index)?;
        let mut fields = vec![tag_value, storage];
        if union_layout.tag_field_index > union_layout.storage_field_index {
            fields.swap(0, 1);
        }
        Ok(self.state.builder.struct_(union_mir_type, fields))
    }

    /// Create a tag constant for a union layout.
    fn union_tag_constant(
        &mut self,
        layout: &UnionLayout,
        tag_index: usize,
    ) -> CompilerResult<mir::Value> {
        let mir::Type::Int {
            width,
            is_signed: signed,
        } = self.state.builder.tree().get(layout.tag_type)
        else {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    self.context
                        .types
                        .get_type_source(layout.source_types[0])
                        .into_global(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "union tag must be an integer type".to_string(),
            }
            .into());
        };

        Ok(self
            .state
            .builder
            .iconst(tag_index as i128, *width, *signed))
    }

    /// Build a MIR tag constant for one union tag index.
    fn union_tag_constant_value(
        &self,
        layout: &UnionLayout,
        tag_index: usize,
    ) -> CompilerResult<mir::Constant> {
        let mir::Type::Int { width, is_signed } = self.state.builder.tree().get(layout.tag_type)
        else {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    self.context
                        .types
                        .get_type_source(layout.source_types[0])
                        .into_global(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "union tag must be an integer type".to_string(),
            }
            .into());
        };

        if *is_signed {
            return Ok(mir::Constant::Int {
                value: tag_index as i128,
                width: *width,
                is_signed: true,
            });
        }

        Ok(mir::Constant::UInt {
            value: tag_index as u128,
            width: *width,
        })
    }

    /// Lower a discriminant literal into a MIR value.
    fn lower_value_for_discriminant_literal(
        &mut self,
        literal: &DiscriminantLiteral,
        node: dir::AnchoredGlobalNodeId,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        match &literal.value {
            DiscriminantValue::Boolean(value) => {
                let value = self.state.builder.bconst(*value);
                Ok((value, self.context.type_lowerer.ty_bool))
            }
            DiscriminantValue::Number { value } => {
                let dir_type = self.context.types.get_type(literal.type_id);
                let scalar = self.context.type_lowerer.scalar_type_for_dir_type(dir_type);
                match scalar {
                    Some(ScalarType::SignedInt { width }) => {
                        let value = self.state.builder.iconst(*value as i128, width, true);
                        let ty = if width == 64 {
                            self.context.type_lowerer.ty_i64
                        } else {
                            self.context.type_lowerer.ty_i32
                        };
                        Ok((value, ty))
                    }
                    Some(ScalarType::Float { format }) => {
                        let value = self.state.builder.fconst(*value, format);
                        let ty = self.context.type_lowerer.type_for_float_format(format);
                        Ok((value, ty))
                    }
                    _ => Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(node),
                        message: "unsupported discriminant numeric literal".to_string(),
                    }
                    .into()),
                }
            }
            DiscriminantValue::String(value) => {
                let (value, ty) = self.string_literal_value_for_id(*value, Some(node))?;
                Ok((value, ty))
            }
            DiscriminantValue::Null
            | DiscriminantValue::Undefined
            | DiscriminantValue::Bigint(_)
            | DiscriminantValue::UniqueSymbol => Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(node),
                message: "unsupported discriminant literal value".to_string(),
            }
            .into()),
        }
    }
}
