use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one protocol operator through its selected method candidate.
    pub(in crate::lower) fn lower_operator_method(
        &mut self,
        receiver: dir::LocalNodeId<dir::Expression>,
        resolution: &dir::Call,
        function: &dir::FunctionTarget,
    ) -> CompilerResult<mir::Value> {
        let Some(value) =
            self.lower_function_target_call(receiver, resolution, function, None, false)?
        else {
            return Err(CompilerError::Internal {
                message: "a void operator method".to_string(),
            });
        };

        Ok(value)
    }

    /// Lower one builtin unary operation.
    pub(in crate::lower) fn lower_unary(
        &mut self,
        operator: dir::UnaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
        operand: &dir::BuiltinOperand,
    ) -> CompilerResult<mir::Value> {
        // require a scalar operand
        let operand = self.lower_operand(right, operand)?;
        let LoweredOperand::Scalar { value, .. } = operand else {
            return Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: format!("the '{}' operator on this representation", operator.text()),
            }
            .into());
        };

        match operator {
            // pass the operand through
            dir::UnaryOperator::Plus => Ok(value),
            // negate the operand
            dir::UnaryOperator::Negate => Ok(self.builder.unary(mir::UnaryOperator::Negate, value)),
            // invert the operand
            dir::UnaryOperator::Not | dir::UnaryOperator::ElementwiseNot => {
                Ok(self.builder.unary(mir::UnaryOperator::Not, value))
            }
            // reject every other unary operator
            other => Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: format!("the '{}' operator", other.text()),
            }
            .into()),
        }
    }

    /// Join one present reference at the coalesce result representation.
    fn coalesce_present_value(
        &mut self,
        value: mir::Value,
        result: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::Value> {
        // enter the present case of an optional variant result
        if let mir::Type::Variant { cases, .. } = self.builder.tree().get(result).clone() {
            // require the absent case beside exactly one present case
            let Some(mir::NullishCase::Case(absent)) = self.builder.tree().undefined_case(result)
            else {
                return Err(CompilerError::Internal {
                    message: "a coalesce variant result without an absent case".to_string(),
                });
            };
            let [_, _] = cases.as_slice() else {
                return Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
                    construct: "a coalesce joining a union of several present cases".to_string(),
                }
                .into());
            };

            // adapt the value into the remaining present case
            let present = 1 - absent;
            let payload = cases[present as usize].ty;
            let payload = self.adapt_to_representation(value, payload)?;

            return Ok(self.builder.variant_new(result, present, Some(payload)));
        }

        self.adapt_to_representation(value, result)
    }

    /// Lower one nullish coalescing operation over its variant or niched representation.
    pub(in crate::lower) fn lower_coalesce(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        // pre-classify both representations without lowering either operand
        let source = self.node_type_id(left)?;
        let representation = self.lower_type(source)?;
        let result = self.lower_type(self.node_type_id(expression)?)?;

        // narrow variant representations through their undefined case
        if matches!(
            self.builder.tree().get(representation),
            mir::Type::Variant { .. }
        ) {
            let value = self.lower_expression(left)?;
            if self.builder.tree().undefined_case(representation).is_none() {
                return self.adapt_to_representation(value, result);
            }

            return self.lower_absent_fallback(value, result, |lower| {
                lower.lower_coalesce_fallback(right, result)
            });
        }

        // keep the value of a never-absent operand
        let Some(nullability) = self.builder.tree().get(representation).nullability() else {
            let value = self.lower_expression(left)?;

            return self.adapt_to_representation(value, result);
        };

        // keep the value of a never-nullish reference
        let value = self.lower_expression(left)?;
        if nullability == mir::Nullability::None {
            return self.adapt_to_representation(value, result);
        }

        // test the nullish niches the representation declares
        let mut is_nullish = None;
        if nullability.admits(mir::Nullish::Undefined) {
            let undefined = self
                .builder
                .constant(mir::Constant::Undefined, representation);
            is_nullish = Some(
                self.builder
                    .binary(mir::BinaryOperator::Equal, value, undefined),
            );
        }
        if nullability.admits(mir::Nullish::Null) {
            let null = self.builder.constant(mir::Constant::Null, representation);
            let test = self.builder.binary(mir::BinaryOperator::Equal, value, null);
            is_nullish = Some(match is_nullish {
                Some(nullish) => self.builder.binary(mir::BinaryOperator::Or, nullish, test),
                None => test,
            });
        }
        let Some(is_nullish) = is_nullish else {
            return Err(CompilerError::Internal {
                message: "a nullable representation without a nullish test".to_string(),
            });
        };

        // short-circuit the right operand behind the nullish test
        let join_value = self.builder.local(result, mir::Mutability::Mutable);
        let keep_block = self.builder.block();
        let right_block = self.builder.block();
        let join = self.builder.block();
        self.builder.branch(is_nullish, right_block, keep_block);

        // keep the present reference at the joined result representation
        self.builder.switch_to_block(keep_block);
        let kept = self.coalesce_present_value(value, result)?;
        self.builder.local_set(join_value, kept);
        self.builder.jump(join);

        // evaluate the fallback only when the reference is nullish
        self.builder.switch_to_block(right_block);
        if let Some(fallback) = self.lower_coalesce_fallback(right, result)? {
            self.builder.local_set(join_value, fallback);
            self.builder.jump(join);
        }

        // continue in the joined block
        self.builder.switch_to_block(join);

        Ok(self.builder.local_get(join_value))
    }

    /// Lower one coalesce fallback, terminating instead of joining for never arms.
    fn lower_coalesce_fallback(
        &mut self,
        right: dir::LocalNodeId<dir::Expression>,
        result: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<Option<mir::Value>> {
        // end the block of a never-typed fallback without a value
        if matches!(self.node_type(right)?, dir::Type::Never) {
            self.lower_expression(right)?;

            return Ok(None);
        }

        // evaluate the fallback at the result representation
        let fallback = self.lower_expression(right)?;

        Ok(Some(self.adapt_to_representation(fallback, result)?))
    }

    /// Lower one short-circuiting logical operation over boolean operands.
    pub(in crate::lower) fn lower_logical(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        // reject non-boolean logical joins
        if !matches!(
            self.node_type(expression)?,
            dir::Type::Primitive(dir::PrimitiveType::Boolean)
        ) {
            return Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: "a union-valued logical operation".to_string(),
            }
            .into());
        }

        // branch on the left value to short-circuit the right operand
        let join_value = self.value_slot(expression)?;
        let value = self.lower_expression(left)?;
        self.builder.local_set(join_value, value);
        let right_block = self.builder.block();
        let join = self.builder.block();
        match operator {
            // evaluate the right operand when the left holds
            dir::BinaryOperator::And => self.builder.branch(value, right_block, join),
            // evaluate the right operand when the left fails
            _ => self.builder.branch(value, join, right_block),
        }

        // overwrite the join value when the right operand runs
        self.builder.switch_to_block(right_block);
        let value = self.lower_expression(right)?;
        self.builder.local_set(join_value, value);
        self.builder.jump(join);

        // continue in the joined block
        self.builder.switch_to_block(join);

        Ok(self.builder.local_get(join_value))
    }

    /// Lower one statement-position update operator through its place.
    pub(in crate::lower) fn lower_update(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        target: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // require a builtin unary update resolution
        let resolution = self.operator_decision(expression)?;
        let dir::OperationResolution::One(dir::OperatorApplication::Unary {
            operator,
            target: dir::OperatorTarget::Builtin(_),
            ..
        }) = resolution
        else {
            return Err(CompilerError::Internal {
                message: "an update with a non-builtin unary resolution".to_string(),
            });
        };

        // resolve the updated place
        let resolution = self.assignment_decision(target)?;
        let place = self.place(&resolution)?;

        // rewrite the place by one over its representation
        let current = self.read_place(&place)?;
        let one_type = self
            .builder
            .value_type(current)
            .ok_or_else(|| CompilerError::Internal {
                message: "a lowered update operand without a type".to_string(),
            })?;
        let one_type = self.builder.tree().get(one_type).clone();
        let one = self.lower_constant(dir::Literal::Integer(1), one_type)?;

        // select the operation the update names and store the result
        let operator = match operator {
            dir::UnaryOperator::PostIncrement | dir::UnaryOperator::PreIncrement => {
                dir::BinaryOperator::Add
            }
            _ => dir::BinaryOperator::Subtract,
        };
        let operator = self.binary_operator(operator)?;
        let value = self.builder.binary(operator, current, one);
        self.write_place(&place, value)?;

        Ok(())
    }

    /// Lower one resolved DIR binary operator into its type-neutral MIR operation.
    pub(super) fn binary_operator(
        &self,
        operator: dir::BinaryOperator,
    ) -> CompilerResult<mir::BinaryOperator> {
        Ok(match operator {
            // arithmetic
            dir::BinaryOperator::Add => mir::BinaryOperator::Add,
            dir::BinaryOperator::Subtract => mir::BinaryOperator::Subtract,
            dir::BinaryOperator::Multiply => mir::BinaryOperator::Multiply,
            dir::BinaryOperator::Divide => mir::BinaryOperator::Divide,
            dir::BinaryOperator::Remainder => mir::BinaryOperator::Remainder,

            // bitwise
            dir::BinaryOperator::ElementwiseAnd => mir::BinaryOperator::And,
            dir::BinaryOperator::ElementwiseOr => mir::BinaryOperator::Or,
            dir::BinaryOperator::ElementwiseXor => mir::BinaryOperator::Xor,
            dir::BinaryOperator::ShiftLeft => mir::BinaryOperator::ShiftLeft,
            dir::BinaryOperator::ShiftRight => mir::BinaryOperator::ShiftRight,
            dir::BinaryOperator::UnsignedShiftRight => mir::BinaryOperator::UnsignedShiftRight,

            // equality
            dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict => {
                mir::BinaryOperator::Equal
            }
            dir::BinaryOperator::NotEqual | dir::BinaryOperator::NotEqualStrict => {
                mir::BinaryOperator::NotEqual
            }

            // ordering
            dir::BinaryOperator::LessThan => mir::BinaryOperator::LessThan,
            dir::BinaryOperator::LessThanOrEqual => mir::BinaryOperator::LessEqual,
            dir::BinaryOperator::GreaterThan => mir::BinaryOperator::GreaterThan,
            dir::BinaryOperator::GreaterThanOrEqual => mir::BinaryOperator::GreaterEqual,

            // reject every other operator
            other => {
                return Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
                    construct: format!("the '{}' operator", other.text()),
                }
                .into());
            }
        })
    }
}

/// One lowered operand classified by its runtime equality representation.
#[derive(Clone, Copy)]
pub(in crate::lower) enum LoweredOperand {
    /// One scalar value.
    Scalar {
        /// The lowered value.
        value: mir::Value,
        /// The scalar domain.
        domain: dir::ScalarDomain,
    },
    /// One address-bearing value.
    Address(mir::Value),
    /// One materialized variant value and its logical representation.
    Variant {
        /// The lowered value.
        value: mir::Value,
        /// The type defining the cases.
        representation: dir::GlobalTypeId,
    },
    /// The unmaterialized null value.
    Null,
    /// The unmaterialized undefined value.
    Undefined,
    /// One payload-free value.
    Singleton,
}

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one binary operation over the operand representation.
    pub(in crate::lower) fn lower_binary(
        &mut self,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
        operands: &[dir::BuiltinOperand; 2],
    ) -> CompilerResult<mir::Value> {
        // compare structural discriminants through their physical union tag
        if operator.is_equality()
            && let Some(result) = self.lower_discriminant_equality(left, operator, right)?
        {
            return Ok(result);
        }

        let left = self.lower_operand(left, &operands[0])?;
        let right = self.lower_operand(right, &operands[1])?;

        self.lower_binary_operands(left, operator, right)
    }

    /// Lower one builtin strict equality operation.
    pub(in crate::lower) fn lower_strict_equality(
        &mut self,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
        operands: &[dir::BuiltinOperand; 2],
    ) -> CompilerResult<mir::Value> {
        // compare structural discriminants through their physical union tag
        if let Some(result) = self.lower_discriminant_equality(left, operator, right)? {
            return Ok(result);
        }

        let left = self.lower_operand(left, &operands[0])?;
        let right = self.lower_operand(right, &operands[1])?;
        let equal = self.lower_representation_equality(left, right)?;

        match operator {
            dir::BinaryOperator::EqualStrict => Ok(equal),
            dir::BinaryOperator::NotEqualStrict => {
                Ok(self.builder.unary(mir::UnaryOperator::Not, equal))
            }
            // reject every other operator
            _ => Err(CompilerError::Internal {
                message: "a different operator in a strict equality comparison".to_string(),
            }),
        }
    }

    /// Lower equality between one structural discriminant and one singleton literal.
    fn lower_discriminant_equality(
        &mut self,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<mir::Value>> {
        // classify the exact scalar value on each side
        let left_literal = self.node_type(left)?.singleton_literal();
        let right_literal = self.node_type(right)?.singleton_literal();

        // evaluate a leading discriminant before the trailing literal
        let equal = if let Some(literal) = right_literal
            && let Some(access) = self.discriminant_access(left)
        {
            let equal = self.lower_discriminant_comparison(left, literal, &access)?;
            self.lower_expression(right)?;

            Some(equal)
        }
        // evaluate a leading literal before the trailing discriminant access
        else if let Some(literal) = left_literal
            && let Some(access) = self.discriminant_access(right)
        {
            self.lower_expression(left)?;
            let equal = self.lower_discriminant_comparison(right, literal, &access)?;

            Some(equal)
        }
        // leave all other operand pairs to their selected equality representation
        else {
            None
        };
        let Some(equal) = equal else {
            return Ok(None);
        };

        // apply the source equality operator to the physical tag comparison
        let result = match operator {
            dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict => equal,
            dir::BinaryOperator::NotEqual | dir::BinaryOperator::NotEqualStrict => {
                self.builder.unary(mir::UnaryOperator::Not, equal)
            }
            // reject every other operator
            _ => {
                return Err(CompilerError::Internal {
                    message: "a different operator in a discriminant equality comparison"
                        .to_string(),
                });
            }
        };

        Ok(Some(result))
    }

    /// Return a discriminant member access selected for one expression.
    fn discriminant_access(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::MemberAccess> {
        let node = expression.into_global_any(self.source);
        let access = self
            .source()
            .decisions
            .decision(node)
            .and_then(dir::Decision::member_access)?;

        // select only structural discriminant projections
        if matches!(
            access.target,
            dir::MemberTarget::Projection {
                projection: dir::Projection::Discriminant { .. },
                ..
            }
        ) {
            Some(access.clone())
        } else {
            None
        }
    }

    /// Lower one structural discriminant comparison against an exact literal.
    fn lower_discriminant_comparison(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        literal: dir::Literal,
        access: &dir::MemberAccess,
    ) -> CompilerResult<mir::Value> {
        // read the discriminant projection and the arm the literal names
        let dir::MemberTarget::Projection {
            receiver,
            projection: dir::Projection::Discriminant { union, cases, .. },
            ..
        } = &access.target
        else {
            return Err(CompilerError::Internal {
                message: "a selected discriminant access with a different target".to_string(),
            });
        };
        let receiver = receiver.clone();
        let union = *union;
        let arm = cases
            .iter()
            .find(|case| case.value == literal)
            .map(|case| case.arm);
        let (left, index) = match *self.source().tree().get(expression) {
            dir::Expression::Member { left, .. } => (left, None),
            dir::Expression::Index { left, index, .. } => (left, index),
            _ => {
                return Err(CompilerError::Internal {
                    message: "a discriminant decision on a non-access expression".to_string(),
                });
            }
        };

        // evaluate the discriminant receiver and computed key exactly once
        let receiver = self.lower_adjusted_receiver(left, &receiver, false)?;
        if let Some(index) = index {
            self.lower_expression(index)?;
        }
        let tag = self.lower_discriminant_tag(receiver, union)?;

        // reject literals outside the projected source domain
        let Some(arm) = arm else {
            return Ok(self.builder.bconst(false));
        };
        let members = self.union_members(union)?;
        let Some(index) = members.iter().position(|member| *member == arm) else {
            return Err(CompilerError::Internal {
                message: "a discriminant comparison over an absent union arm".to_string(),
            });
        };

        // compare the tag with a constant of its exact integer representation
        let tag_type = self.value_representation(tag)?;
        let mir::Type::Int { width, is_signed } = *self.builder.tree().get(tag_type) else {
            return Err(CompilerError::Internal {
                message: "a non-integer lowered discriminant tag".to_string(),
            });
        };
        let expected = self.builder.iconst(index as i128, width, is_signed);
        let equal = self
            .builder
            .binary(mir::BinaryOperator::Equal, tag, expected);

        Ok(equal)
    }

    /// Return the type carrying one operand after reading through a view.
    pub(in crate::lower) fn operand_representation(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read through a single indirect layer
        let ty = self.node_type_id(expression)?;
        let Some(layer) = self.lower.peel_indirection(ty)? else {
            return Ok(ty);
        };
        if self.lower.peel_indirection(layer.stored)?.is_some() {
            return Ok(ty);
        }

        Ok(layer.stored)
    }

    /// Lower one builtin operand.
    pub(in crate::lower) fn lower_operand(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        operand: &dir::BuiltinOperand,
    ) -> CompilerResult<LoweredOperand> {
        // require the operand to name this expression
        let source = expression.into_global(self.source);
        if source != operand.source {
            return Err(CompilerError::Internal {
                message: format!(
                    "a builtin operand {:?} on the expression {:?}",
                    operand.source, source
                ),
            });
        }

        // leave standalone nullish values unmaterialized until they meet a reference
        let mut representation = self.lower.instance_type(self.instance, operand.ty)?;
        match self.node_type(expression)? {
            dir::Type::Null => return Ok(LoweredOperand::Null),
            dir::Type::Undefined => return Ok(LoweredOperand::Undefined),
            _ => {}
        }

        // evaluate the operand at its own representation
        let mut value = self.lower_expression(expression)?;

        // read inline values through their indirect representations
        if let Some(layer) = self.lower.peel_indirection(representation)?
            && !self.lower.has_indirect_representation(layer.stored)?
        {
            representation = layer.stored;
            let pointee = self.lower_type(representation)?;
            value = self.builder.load(value, pointee);
        }

        self.classify_equality_operand(representation, operand.scalar_families.as_ref(), value)
    }

    /// Classify one evaluated value by its runtime equality representation.
    fn classify_equality_operand(
        &mut self,
        mut representation: dir::GlobalTypeId,
        scalar_families: Option<&dir::ScalarFamilySet>,
        mut value: mir::Value,
    ) -> CompilerResult<LoweredOperand> {
        // compare transparent newtypes through their backing representation
        loop {
            let dir::Type::Application(instance) = self.lower.ty(representation)? else {
                break;
            };
            let Some(dir::Definition::Newtype(definition)) =
                self.lower.definition(instance.symbol)?
            else {
                break;
            };
            if self.type_is_singleton(definition.backing)? {
                return Ok(LoweredOperand::Singleton);
            }

            value = self.builder.field_get(value, 0);
            representation = definition.backing;
        }

        // classify the operand by the representation it lowered to
        let ty = self.value_representation(value)?;
        let ty = self.builder.tree().get(ty);

        // preserve aggregate representations even when every case shares scalar behavior
        if matches!(ty, mir::Type::Variant { .. }) {
            let family = scalar_families
                .filter(|families| families.len() == 1)
                .and_then(|families| families.iter().next());
            if matches!(family, Some(dir::ScalarFamily::Enum(_))) {
                let value = self.builder.variant_tag(value);

                return Ok(LoweredOperand::Scalar {
                    value,
                    domain: dir::ScalarDomain::Integer,
                });
            }

            return Ok(LoweredOperand::Variant {
                value,
                representation,
            });
        }

        // apply the selected leaf scalar behavior
        if let Some(families) = scalar_families
            && families.len() == 1
        {
            let family =
                families
                    .iter()
                    .next()
                    .copied()
                    .ok_or_else(|| CompilerError::Internal {
                        message: "an empty scalar family set".to_string(),
                    })?;
            match family {
                dir::ScalarFamily::Domain(domain) => {
                    return Ok(LoweredOperand::Scalar { value, domain });
                }
                dir::ScalarFamily::Enum(_) => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "an enum equality representation {representation:?} outside a variant"
                        ),
                    });
                }
            }
        }

        // compare nested variant payloads through their scalar representation
        if let Some(domain) = Self::mir_scalar_domain(ty) {
            return Ok(LoweredOperand::Scalar { value, domain });
        }

        match ty {
            // compare reference-family representations by identity
            mir::Type::Dynamic { .. }
            | mir::Type::Reference { .. }
            | mir::Type::Pointer { .. }
            | mir::Type::Slice { .. } => Ok(LoweredOperand::Address(value)),
            // reject every other representation
            _ => Err(CompilerError::Internal {
                message: "an equality type outside a supported representation".to_string(),
            }),
        }
    }

    /// Return the scalar domain one lowered type represents directly.
    fn mir_scalar_domain(ty: &mir::Type) -> Option<dir::ScalarDomain> {
        match ty {
            mir::Type::Boolean => Some(dir::ScalarDomain::Boolean),
            mir::Type::Int { .. } | mir::Type::Isize | mir::Type::Usize => {
                Some(dir::ScalarDomain::Integer)
            }
            mir::Type::Float(_) => Some(dir::ScalarDomain::Float),
            _ => None,
        }
    }

    /// Return whether one type has one value and no runtime payload.
    fn type_is_singleton(&self, mut ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        loop {
            match self.lower.ty(ty)? {
                dir::Type::Literal(_) | dir::Type::Null | dir::Type::Undefined => return Ok(true),
                dir::Type::Application(instance) => {
                    let Some(dir::Definition::Newtype(definition)) =
                        self.lower.definition(instance.symbol)?
                    else {
                        return Ok(false);
                    };
                    ty = definition.backing;
                }
                _ => return Ok(false),
            }
        }
    }

    /// Lower one binary operation over already evaluated operands.
    fn lower_binary_operands(
        &mut self,
        left: LoweredOperand,
        operator: dir::BinaryOperator,
        right: LoweredOperand,
    ) -> CompilerResult<mir::Value> {
        // require two scalar operands, else compare by address
        let (
            LoweredOperand::Scalar {
                value: left_value,
                domain: left_domain,
            },
            LoweredOperand::Scalar {
                value: right_value,
                domain: right_domain,
            },
        ) = (left, right)
        else {
            return self.lower_address_equality(left, operator, right);
        };
        if left_domain != right_domain {
            return Err(CompilerError::Internal {
                message: format!(
                    "binary operands in different scalar domains, {left_domain:?} and \
                     {right_domain:?}"
                ),
            });
        }
        let operator = self.binary_operator(operator)?;

        Ok(self.builder.binary(operator, left_value, right_value))
    }

    /// Lower equality over one common representation.
    pub(in crate::lower) fn lower_representation_equality(
        &mut self,
        left: LoweredOperand,
        right: LoweredOperand,
    ) -> CompilerResult<mir::Value> {
        match (left, right) {
            // compare two variants case by case
            (
                LoweredOperand::Variant {
                    value: left,
                    representation: left_representation,
                },
                LoweredOperand::Variant {
                    value: right,
                    representation: right_representation,
                },
            ) => {
                self.lower_variant_equality(left_representation, left, right_representation, right)
            }
            // reject a variant compared against a non-variant
            (LoweredOperand::Variant { .. }, _) | (_, LoweredOperand::Variant { .. }) => {
                Err(CompilerError::Internal {
                    message: "equality operands in different runtime representations".to_string(),
                })
            }
            // compare two leaf operands
            (left, right) => self.lower_leaf_equality(left, right),
        }
    }

    /// Lower equality between two values of one indexed variant representation.
    fn lower_variant_equality(
        &mut self,
        left_representation: dir::GlobalTypeId,
        left: mir::Value,
        right_representation: dir::GlobalTypeId,
        right: mir::Value,
    ) -> CompilerResult<mir::Value> {
        // require both variants to declare the same case count
        let left_members = self.union_members(left_representation)?;
        let right_members = self.union_members(right_representation)?;
        if left_members.len() != right_members.len() {
            return Err(CompilerError::Internal {
                message: "equality variants with different case counts".to_string(),
            });
        }

        // compare the shared discriminant once
        let left_tag = self.builder.variant_tag(left);
        let right_tag = self.builder.variant_tag(right);
        let same_case = self
            .builder
            .binary(mir::BinaryOperator::Equal, left_tag, right_tag);

        // answer with the discriminant alone for payload-free variants
        let mut is_payload_free = true;
        for (left, right) in left_members.iter().zip(&right_members) {
            is_payload_free &= self.type_is_singleton(*left)?;
            is_payload_free &= self.type_is_singleton(*right)?;
        }
        if is_payload_free {
            return Ok(same_case);
        }

        // reject different cases before projecting either payload
        let boolean = self.builder.tree_mut().intern_type(mir::Type::Boolean);
        let result = self.builder.local(boolean, mir::Mutability::Immutable);
        let compare = self.builder.block();
        let unequal = self.builder.block();
        let exit = self.builder.block();
        self.builder.branch(same_case, compare, unequal);

        // compare the payload selected by the shared case
        self.builder.switch_to_block(compare);
        let mut targets = Vec::with_capacity(left_members.len());
        for index in 0..left_members.len() {
            targets.push((index as u32, self.builder.block()));
        }
        self.builder
            .variant_switch(left, Some(unequal), targets.clone());
        for (index, block) in targets {
            self.builder.switch_to_block(block);
            let left_member = left_members[index as usize];
            let right_member = right_members[index as usize];
            let equal =
                if self.type_is_singleton(left_member)? && self.type_is_singleton(right_member)? {
                    self.builder.bconst(true)
                } else {
                    let left = self.builder.variant_payload(left, index);
                    let left = self.classify_equality_operand(left_member, None, left)?;
                    let right = self.builder.variant_payload(right, index);
                    let right = self.classify_equality_operand(right_member, None, right)?;

                    self.lower_representation_equality(left, right)?
                };
            self.builder.local_set(result, equal);
            self.builder.jump(exit);
        }

        // converge every unequal case on false
        self.builder.switch_to_block(unequal);
        let value = self.builder.bconst(false);
        self.builder.local_set(result, value);
        self.builder.jump(exit);
        self.builder.switch_to_block(exit);

        Ok(self.builder.local_get(result))
    }

    /// Lower equality between two leaf operands.
    fn lower_leaf_equality(
        &mut self,
        left: LoweredOperand,
        right: LoweredOperand,
    ) -> CompilerResult<mir::Value> {
        // treat payload-free values as equal
        if matches!(left, LoweredOperand::Singleton) && matches!(right, LoweredOperand::Singleton) {
            return Ok(self.builder.bconst(true));
        }

        // compare two scalar operands directly and anything else by address
        let (
            LoweredOperand::Scalar {
                value: left_value,
                domain: left_domain,
            },
            LoweredOperand::Scalar {
                value: right_value,
                domain: right_domain,
            },
        ) = (left, right)
        else {
            return self.lower_address_equality(left, dir::BinaryOperator::EqualStrict, right);
        };
        if left_domain != right_domain {
            return Err(CompilerError::Internal {
                message: format!(
                    "equality operands in different scalar domains, {left_domain:?} and \
                     {right_domain:?}"
                ),
            });
        }

        // reject string and bigint value equality
        if matches!(
            left_domain,
            dir::ScalarDomain::String | dir::ScalarDomain::Bigint
        ) {
            return Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: format!("{left_domain:?} value equality"),
            }
            .into());
        }
        let operator = self.binary_operator(dir::BinaryOperator::EqualStrict)?;

        Ok(self.builder.binary(operator, left_value, right_value))
    }

    /// Lower equality involving addresses or unmaterialized nullish values.
    fn lower_address_equality(
        &mut self,
        left: LoweredOperand,
        operator: dir::BinaryOperator,
        right: LoweredOperand,
    ) -> CompilerResult<mir::Value> {
        // require an equality operator
        let is_equal = match operator {
            dir::BinaryOperator::EqualStrict | dir::BinaryOperator::Equal => true,
            dir::BinaryOperator::NotEqualStrict | dir::BinaryOperator::NotEqual => false,
            other => {
                return Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
                    construct: format!("the '{}' operator on pointers or references", other.text()),
                }
                .into());
            }
        };

        // fold comparisons with no runtime representation
        match (left, right) {
            (LoweredOperand::Null, LoweredOperand::Null)
            | (LoweredOperand::Undefined, LoweredOperand::Undefined) => {
                return Ok(self.builder.bconst(is_equal));
            }
            (LoweredOperand::Null, LoweredOperand::Undefined)
            | (LoweredOperand::Undefined, LoweredOperand::Null) => {
                return Ok(self.builder.bconst(!is_equal));
            }
            _ => {}
        }

        // compare a nullish literal with the variant case tag carrying it
        let variant_nullish = match (&left, &right) {
            (
                LoweredOperand::Variant {
                    value,
                    representation,
                },
                LoweredOperand::Null,
            )
            | (
                LoweredOperand::Null,
                LoweredOperand::Variant {
                    value,
                    representation,
                },
            ) => Some((*value, *representation, dir::Type::Null)),
            (
                LoweredOperand::Variant {
                    value,
                    representation,
                },
                LoweredOperand::Undefined,
            )
            | (
                LoweredOperand::Undefined,
                LoweredOperand::Variant {
                    value,
                    representation,
                },
            ) => Some((*value, *representation, dir::Type::Undefined)),
            _ => None,
        };
        if let Some((value, representation, nullish)) = variant_nullish {
            return self.lower_variant_nullish_equality(value, representation, nullish, is_equal);
        }

        // materialize nullish niches at the compared address representation
        let (left, right) = match (left, right) {
            (LoweredOperand::Address(left), LoweredOperand::Address(right)) => (left, right),
            (LoweredOperand::Address(address), nullish) => {
                let nullish = self.lower_nullish(nullish, address)?;

                (address, nullish)
            }
            (nullish, LoweredOperand::Address(address)) => {
                let nullish = self.lower_nullish(nullish, address)?;

                (nullish, address)
            }
            _ => {
                return Err(CompilerError::Internal {
                    message: "an equality comparison over incompatible representations".to_string(),
                });
            }
        };
        let operator = match is_equal {
            true => mir::BinaryOperator::Equal,
            false => mir::BinaryOperator::NotEqual,
        };

        Ok(self.builder.binary(operator, left, right))
    }

    /// Lower equality between one variant value and the nullish case it may hold.
    fn lower_variant_nullish_equality(
        &mut self,
        value: mir::Value,
        representation: dir::GlobalTypeId,
        nullish: dir::Type,
        is_equal: bool,
    ) -> CompilerResult<mir::Value> {
        // select the union case declaring the compared nullish type
        let members = self.union_members(representation)?;
        let mut selected = None;
        for (index, member) in members.iter().enumerate() {
            if self.lower.ty(*member)? == nullish {
                selected = Some(index);

                break;
            }
        }
        let Some(index) = selected else {
            return Err(CompilerError::Internal {
                message: "a nullish comparison outside the variant's declared cases".to_string(),
            });
        };

        // compare the discriminant with the selected case index
        let tag = self.builder.variant_tag(value);
        let tag_type = self.value_representation(tag)?;
        let mir::Type::Int { width, is_signed } = *self.builder.tree().get(tag_type) else {
            return Err(CompilerError::Internal {
                message: "a non-integer lowered discriminant tag".to_string(),
            });
        };
        let expected = self.builder.iconst(index as i128, width, is_signed);
        let equal = self
            .builder
            .binary(mir::BinaryOperator::Equal, tag, expected);

        match is_equal {
            true => Ok(equal),
            false => Ok(self.builder.unary(mir::UnaryOperator::Not, equal)),
        }
    }

    /// Materialize one nullish operand at an address value's representation.
    fn lower_nullish(
        &mut self,
        operand: LoweredOperand,
        address: mir::Value,
    ) -> CompilerResult<mir::Value> {
        // materialize the nullish constant at the address representation
        let representation = self.value_representation(address)?;
        let constant = match operand {
            LoweredOperand::Null => mir::Constant::Null,
            LoweredOperand::Undefined => mir::Constant::Undefined,
            _ => {
                return Err(CompilerError::Internal {
                    message: "a non-nullish lowered equality operand".to_string(),
                });
            }
        };

        Ok(self.builder.constant(constant, representation))
    }
}
