use tspp_dir as dir;
use tspp_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::assign::UnionMemberStore;
use crate::lower::function::call::ReceiverUse;
use crate::lower::function::place::Place;
use crate::{CompilerError, CompilerResult};

impl FunctionLowerer<'_, '_, '_> {
    /// Map one comparison operator's protocol result onto its boolean.
    pub(in crate::lower) fn lower_comparison_result(
        &mut self,
        operator: dir::BinaryOperator,
        value: mir::Value,
        return_type: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Value> {
        if !operator.is_comparison() || operator.is_equality() {
            return Ok(value);
        }

        // a total comparison answers an ordering directly
        let Some(members) = self.lower.union_members_maybe(return_type)? else {
            return self.lower_ordering_test(operator, value, return_type);
        };

        // a partial comparison answers an ordering in one case, false outside it
        let ordering = self
            .lower
            .language_item_symbol(dir::LanguageItem::Ordering)?;
        let mut selected = None;
        for member in &members {
            if let dir::Type::Application(application) = self.lower.ty(*member)?
                && application.symbol == ordering
            {
                selected = Some(*member);
            }
        }
        let Some(member) = selected else {
            return Err(CompilerError::Internal {
                message: "a partial comparison without an ordering case".to_string(),
            });
        };
        let position = self.case(&members, member)? as usize;
        let boolean = self.builder.tree().boolean_type();
        let result = self.builder.local(boolean, mir::Mutability::Immutable);
        let present = self.builder.block();
        let absent = self.builder.block();
        let exit = self.builder.block();
        let place = Place::local(self.home(value));
        self.branch_place_case(&place, position as u32, present, absent)?;

        // test the ordering the present case holds
        self.builder.switch_to_block(present);
        let ordering_value = self.builder.variant_payload(value, position as u32);
        let tested = self.lower_ordering_test(operator, ordering_value, member)?;
        self.builder.local_set(result, tested);
        self.builder.jump(exit);

        // answer false for an absent ordering
        self.builder.switch_to_block(absent);
        let unordered = self.builder.bconst(false);
        self.builder.local_set(result, unordered);
        self.builder.jump(exit);

        self.builder.switch_to_block(exit);

        Ok(self.builder.local_get(result))
    }

    /// Test one ordering value for the case a comparison operator names.
    fn lower_ordering_test(
        &mut self,
        operator: dir::BinaryOperator,
        value: mir::Value,
        ordering: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Value> {
        // name the case the operator tests and whether it holds or is excluded
        let (case, holds) = match operator {
            dir::BinaryOperator::LessThan => ("Less", true),
            dir::BinaryOperator::GreaterThan => ("Greater", true),
            dir::BinaryOperator::LessThanOrEqual => ("Greater", false),
            dir::BinaryOperator::GreaterThanOrEqual => ("Less", false),
            _ => {
                return Err(CompilerError::Internal {
                    message: "an ordering test outside a comparison operator".to_string(),
                });
            }
        };

        // find the case's discriminant through the enum declaring it
        let dir::Type::Application(application) = self.lower.ty(ordering)? else {
            return Err(CompilerError::Internal {
                message: "an ordering outside its enum application".to_string(),
            });
        };
        let Some(dir::Definition::Enum(definition)) =
            self.lower.definition(application.symbol)?.cloned()
        else {
            return Err(CompilerError::Internal {
                message: "an ordering without its enum definition".to_string(),
            });
        };
        let key = dir::LanguageItem::Ordering.member(case).key;
        let Some(index) = definition.variant_by_key(key).map(|variant| variant.symbol) else {
            return Err(CompilerError::Internal {
                message: format!("an ordering without its '{case}' case"),
            });
        };
        let index = self.lower.variant_position(application.symbol, index)?;
        let representation = self.lower_type(ordering)?;
        let mir::Type::Variant { cases, .. } =
            self.builder.tree().type_definition(representation).clone()
        else {
            return Err(CompilerError::Internal {
                message: "an ordering outside a variant representation".to_string(),
            });
        };
        let discriminant = cases[index as usize].discriminant.clone();

        // compare the tag with that discriminant
        let tag = self.builder.variant_tag(value);
        let tag_type = self.value_representation(tag)?;
        let expected = self.builder.constant(discriminant, tag_type);
        let operator = match holds {
            true => mir::BinaryOperator::Equal,
            false => mir::BinaryOperator::NotEqual,
        };

        Ok(self.builder.binary(operator, tag, expected))
    }

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
        // require a scalar operand, a parameter's instance deciding its format
        let operand = self.lower_builtin_operand(right, operand)?;
        let (LoweredOperand::Scalar { value, .. } | LoweredOperand::Polymorphic(value)) = operand
        else {
            return Err(self.unsupported(format!(
                "the '{}' operator on this representation",
                operator.text()
            )));
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
            other => Err(self.unsupported(format!("the '{}' operator", other.text()))),
        }
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
        let result_ty = self.node_type_id(expression)?;
        let result = self.lower_type(result_ty)?;

        // narrow variant representations through their undefined case
        if matches!(
            self.builder.tree().type_definition(representation),
            mir::Type::Variant { .. }
        ) {
            let value = self.lower_value(left)?;
            if self.absent_case(representation).is_none() {
                return self.adopt(value, result);
            }

            return self.lower_absent_fallback(value, source, result_ty, |lower| {
                lower.lower_coalesce_fallback(right, result_ty)
            });
        }

        // keep the value of an operand no variant stores absent
        let value = self.lower_value(left)?;

        self.adopt(value, result)
    }

    /// Lower one coalesce fallback, a never arm ending its block without a value.
    fn lower_coalesce_fallback(
        &mut self,
        right: dir::LocalNodeId<dir::Expression>,
        result_ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<mir::Value>> {
        // end the block of a never-typed fallback without a value
        if matches!(self.node_type(right)?, dir::Type::Never) {
            self.lower_value(right)?;

            return Ok(None);
        }

        // evaluate the fallback at the result type its record converts it to
        let operand = self.lower_operand(right)?;

        Ok(Some(self.as_value(operand, result_ty)?))
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
            return Err(self.unsupported("a union-valued logical operation"));
        }

        // branch on the left value to short-circuit the right operand
        let destination = self.join_place(expression)?;
        let value = self.lower_value(left)?;
        self.write_place(&destination, value)?;
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
        if self.lower_into(right, &destination)? {
            self.builder.jump(join);
        }

        // continue in the joined block
        self.builder.switch_to_block(join);

        self.read_place(&destination)
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

        // select the operation the update names
        let operator = match operator {
            dir::UnaryOperator::PostIncrement | dir::UnaryOperator::PreIncrement => {
                dir::BinaryOperator::Add
            }
            _ => dir::BinaryOperator::Subtract,
        };
        let operator = self.binary_operator(operator)?;

        // dispatch a union member update over its recorded arms
        let resolution = self.assignment_decision(target)?;
        if let dir::WriteResolution::Member(member) = &resolution.write
            && !matches!(member, dir::OperationResolution::One(_))
        {
            let arms = member.arms().to_vec();

            return self.lower_union_member_write(
                target,
                UnionMemberStore::Update(operator),
                &arms,
            );
        }

        // rewrite the place by one over its representation
        let place = self.place(&resolution)?;
        let current = self.read_place(&place)?;
        let one_type = self
            .builder
            .value_type(current)
            .ok_or_else(|| CompilerError::Internal {
                message: "a lowered update operand without a type".to_string(),
            })?;
        let one = self.one_value(one_type)?;
        let value = self.builder.binary(operator, current, one);
        self.write_place(&place, value)?;

        Ok(())
    }

    /// Build the constant one at a value representation.
    pub(in crate::lower) fn one_value(
        &mut self,
        representation: mir::TypeId,
    ) -> CompilerResult<mir::Value> {
        self.lower_constant(dir::Literal::Integer(1), representation)
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
                return Err(self.unsupported(format!("the '{}' operator", other.text())));
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
    /// One value of a parameter type, its instance deciding the scalar format or identity.
    Polymorphic(mir::Value),
    /// One materialized variant value and its logical representation.
    Variant {
        /// The lowered value.
        value: mir::Value,
        /// The type defining the cases.
        representation: dir::GlobalTypeId,
    },
    /// One value its type holds alone: null, undefined, or a literal.
    Singleton {
        /// The zero-sized type.
        ty: mir::TypeId,
        /// The literal the type holds, absent for an empty nominal.
        literal: Option<dir::Literal>,
    },
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

        // lower both operands and apply the builtin operator
        let left = self.lower_builtin_operand(left, &operands[0])?;
        let right = self.lower_builtin_operand(right, &operands[1])?;

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

        // compare the operands' representations
        let left = self.lower_builtin_operand(left, &operands[0])?;
        let right = self.lower_builtin_operand(right, &operands[1])?;
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
            self.lower_value(right)?;

            Some(equal)
        }
        // evaluate a leading literal before the trailing discriminant access
        else if let Some(literal) = left_literal
            && let Some(access) = self.discriminant_access(right)
        {
            self.lower_value(left)?;
            let equal = self.lower_discriminant_comparison(right, literal, &access)?;

            Some(equal)
        }
        // test a nullish literal at the place of a move-only operand
        else if let Some(nullish) = Self::nullish_operand(&self.node_type(right)?)
            && self.tests_at_place(left)?
        {
            Some(self.lower_nullish_place_test(left, nullish)?)
        } else if let Some(nullish) = Self::nullish_operand(&self.node_type(left)?)
            && self.tests_at_place(right)?
        {
            Some(self.lower_nullish_place_test(right, nullish)?)
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

    /// Return the nullish type one operand's type is, when it is one.
    fn nullish_operand(ty: &dir::Type) -> Option<dir::Type> {
        match ty {
            dir::Type::Null => Some(dir::Type::Null),
            dir::Type::Undefined => Some(dir::Type::Undefined),
            _ => None,
        }
    }

    /// Return whether one operand tests its case at its place, a read moving the variant it holds.
    fn tests_at_place(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<bool> {
        if !self.is_place_expression(expression) {
            return Ok(false);
        }
        let representation = self.operand_representation(expression)?;
        let variant = self.lower_type(representation)?;
        let is_variant = matches!(
            self.builder.tree().type_definition(variant),
            mir::Type::Variant { .. }
        );

        Ok(is_variant)
    }

    /// Lower one nullish test reading the case tag at the operand's place.
    fn lower_nullish_place_test(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        nullish: dir::Type,
    ) -> CompilerResult<mir::Value> {
        let representation = self.operand_representation(expression)?;
        let variant = self.lower_type(representation)?;
        let place = self.receiver_place(expression)?;
        let place = place.lower(self)?;
        let tag = self.builder.variant_tag_load(place, variant);
        let singleton = match nullish {
            dir::Type::Null => self
                .lower
                .singleton_type(self.builder.tree_mut(), &dir::Literal::Null),
            _ => self.builder.tree_mut().intern_type(mir::Type::Void),
        };

        self.lower_case_tag_test(tag, variant, singleton)
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
        let receiver = self.lower_adjusted_receiver(left, &receiver, false, ReceiverUse::Value)?;
        if let Some(index) = index {
            self.lower_value(index)?;
        }
        let tag = self.lower_discriminant_tag(receiver, union)?;

        // reject literals outside the projected source domain
        let Some(arm) = arm else {
            return Ok(self.builder.bconst(false));
        };
        let members = self.lower.union_members(union)?;
        let index = self.case(&members, arm)?;

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

    /// Return the type of one operand after reading through a view.
    pub(in crate::lower) fn operand_representation(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read through a single indirect layer
        let ty = self.node_type_id(expression)?;
        let Some(layer) = self.lower.indirection(ty, &self.scope)? else {
            return Ok(ty);
        };
        if self.lower.indirection(layer.stored, &self.scope)?.is_some() {
            return Ok(ty);
        }

        Ok(layer.stored)
    }

    /// Lower one builtin operand.
    pub(in crate::lower) fn lower_builtin_operand(
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

        // a nullish operand is the singleton of its type
        let mut representation = operand.ty;
        match self.node_type(expression)? {
            dir::Type::Null => {
                let ty = self
                    .lower
                    .singleton_type(self.builder.tree_mut(), &dir::Literal::Null);

                return Ok(LoweredOperand::Singleton {
                    ty,
                    literal: Some(dir::Literal::Null),
                });
            }
            dir::Type::Undefined => {
                let ty = self.builder.tree_mut().intern_type(mir::Type::Void);

                return Ok(LoweredOperand::Singleton {
                    ty,
                    literal: Some(dir::Literal::Undefined),
                });
            }
            _ => {}
        }

        // evaluate the operand at its own representation
        let lowered = self.lower_operand(expression)?;
        let mut value = self.as_value(lowered, representation)?;

        // read inline values through their indirect representations
        if let Some(layer) = self.lower.indirection(representation, &self.scope)?
            && self.lower.indirection(layer.stored, &self.scope)?.is_none()
        {
            representation = layer.stored;
            let pointee = self.lower_type(representation)?;
            let place = mir::Place::value(value).with_projection(mir::Projection::Deref);
            value = self.load_place(place, pointee);
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
                self.lower.definition(instance.symbol)?.cloned()
            else {
                break;
            };

            value = self.builder.field_get(value, 0);
            representation = definition.backing;
        }

        // classify the operand by its lowered representation
        let ty = self.value_representation(value)?;
        if self.is_singleton_representation(ty) {
            let literal = match self.lower.ty(representation)? {
                dir::Type::Literal(literal) => Some(literal),
                dir::Type::Null => Some(dir::Literal::Null),
                dir::Type::Undefined | dir::Type::Void => Some(dir::Literal::Undefined),
                _ => None,
            };

            return Ok(LoweredOperand::Singleton { ty, literal });
        }
        let ty = self.builder.tree().type_definition(ty);

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
            // leave a parameter's representation to its instance
            mir::Type::Parameter { .. } => Ok(LoweredOperand::Polymorphic(value)),
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

    /// Materialize the literal of a singleton meeting a scalar at that scalar.
    fn materialize_singleton_operand(
        &mut self,
        left: LoweredOperand,
        right: LoweredOperand,
    ) -> CompilerResult<(LoweredOperand, LoweredOperand)> {
        let (scalar, literal, is_left_scalar) = match (left, right) {
            (
                LoweredOperand::Scalar { value, domain },
                LoweredOperand::Singleton {
                    literal: Some(literal),
                    ..
                },
            ) => ((value, Some(domain)), literal, true),
            (
                LoweredOperand::Singleton {
                    literal: Some(literal),
                    ..
                },
                LoweredOperand::Scalar { value, domain },
            ) => ((value, Some(domain)), literal, false),
            // materialize the literal at a parameter operand's instance representation
            (
                LoweredOperand::Polymorphic(value),
                LoweredOperand::Singleton {
                    literal: Some(literal),
                    ..
                },
            ) => ((value, None), literal, true),
            (
                LoweredOperand::Singleton {
                    literal: Some(literal),
                    ..
                },
                LoweredOperand::Polymorphic(value),
            ) => ((value, None), literal, false),
            pair => return Ok(pair),
        };
        let (value, domain) = scalar;
        let representation = self.value_representation(value)?;
        let constant = self.lower_constant(literal, representation)?;
        let materialized = match domain {
            Some(domain) => LoweredOperand::Scalar {
                value: constant,
                domain,
            },
            None => LoweredOperand::Polymorphic(constant),
        };

        Ok(match is_left_scalar {
            true => (left, materialized),
            false => (materialized, right),
        })
    }

    /// Lower one binary operation over already evaluated operands.
    fn lower_binary_operands(
        &mut self,
        left: LoweredOperand,
        operator: dir::BinaryOperator,
        right: LoweredOperand,
    ) -> CompilerResult<mir::Value> {
        // apply the operator over two parameter operands as their instance decides
        let (left, right) = self.materialize_singleton_operand(left, right)?;
        if let (LoweredOperand::Polymorphic(left), LoweredOperand::Polymorphic(right)) =
            (&left, &right)
        {
            let operator = self.binary_operator(operator)?;

            return Ok(self.builder.binary(operator, *left, *right));
        }

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
            // compare a variant against one member value through the case that stores it
            (
                LoweredOperand::Variant {
                    value,
                    representation,
                },
                leaf,
            )
            | (
                leaf,
                LoweredOperand::Variant {
                    value,
                    representation,
                },
            ) => self.lower_variant_member_equality(value, representation, leaf),
            // compare two leaf operands
            (left, right) => self.lower_leaf_equality(left, right),
        }
    }

    /// Lower equality between one variant and a value of one of its cases.
    fn lower_variant_member_equality(
        &mut self,
        variant: mir::Value,
        representation: dir::GlobalTypeId,
        leaf: LoweredOperand,
    ) -> CompilerResult<mir::Value> {
        // compare a singleton through the tag of the case holding it
        let leaf_value = match leaf {
            LoweredOperand::Singleton { ty, .. } => {
                let tag = self.builder.variant_tag(variant);
                let held = self.value_representation(variant)?;

                return self.lower_case_tag_test(tag, held, ty);
            }
            LoweredOperand::Scalar { value, .. }
            | LoweredOperand::Address(value)
            | LoweredOperand::Polymorphic(value) => value,
            LoweredOperand::Variant { .. } => unreachable!("variants compare case by case"),
        };

        // find the case stored at the member's representation
        let leaf_type = self.value_representation(leaf_value)?;
        let leaf_type = mir::Substitution::resolve(leaf_type, self.builder.tree_mut());
        let mut selected = None;
        for (index, member) in self
            .lower
            .union_members(representation)?
            .into_iter()
            .enumerate()
        {
            let case = self.lower_type(member)?;
            if mir::Substitution::resolve(case, self.builder.tree_mut()) == leaf_type {
                selected = Some((index as u32, member));
                break;
            }
        }
        let Some((index, member)) = selected else {
            return Err(CompilerError::Internal {
                message: "equality operands in different runtime representations".to_string(),
            });
        };

        // compare the payload when the variant holds that case, else answer false
        let boolean = self.builder.tree_mut().intern_type(mir::Type::Boolean);
        let result = self.builder.local(boolean, mir::Mutability::Immutable);
        let compare = self.builder.block();
        let unequal = self.builder.block();
        let exit = self.builder.block();
        self.builder
            .variant_switch(variant, Some(unequal), vec![(index, compare)]);
        self.builder.switch_to_block(compare);
        let payload = self.builder.variant_payload(variant, index);
        let payload = self.classify_equality_operand(member, None, payload)?;
        let equal = self.lower_leaf_equality(payload, leaf)?;
        self.builder.local_set(result, equal);
        self.builder.jump(exit);
        self.builder.switch_to_block(unequal);
        let value = self.builder.bconst(false);
        self.builder.local_set(result, value);
        self.builder.jump(exit);
        self.builder.switch_to_block(exit);

        Ok(self.builder.local_get(result))
    }

    /// Lower equality between two values of one indexed variant representation.
    fn lower_variant_equality(
        &mut self,
        left_representation: dir::GlobalTypeId,
        left: mir::Value,
        right_representation: dir::GlobalTypeId,
        right: mir::Value,
    ) -> CompilerResult<mir::Value> {
        // compare the shared discriminant once
        let left_tag = self.builder.variant_tag(left);
        let right_tag = self.builder.variant_tag(right);
        let same_case = self
            .builder
            .binary(mir::BinaryOperator::Equal, left_tag, right_tag);

        // answer with the discriminant alone when neither variant has a payload
        let representation = self.value_representation(left)?;
        let representation = self.resolved_type(representation);
        let mir::Type::Variant { cases, .. } =
            self.builder.tree().type_definition(representation).clone()
        else {
            return Err(self.internal("equality operands outside a variant representation"));
        };
        let right_value = self.value_representation(right)?;
        let right_value = self.resolved_type(right_value);
        let mir::Type::Variant {
            cases: right_cases, ..
        } = self.builder.tree().type_definition(right_value).clone()
        else {
            return Err(self.internal("equality operands outside a variant representation"));
        };
        if cases
            .iter()
            .chain(&right_cases)
            .all(|case| self.is_singleton_representation(case.ty))
        {
            return Ok(same_case);
        }

        // pair the members of both variants by the case each stores at
        let (Some(left_members), Some(right_members)) = (
            self.lower.union_members_maybe(left_representation)?,
            self.lower.union_members_maybe(right_representation)?,
        ) else {
            return Err(self.unsupported("equality over an enum with payloads"));
        };
        if left_members.len() != right_members.len() {
            return Err(CompilerError::Internal {
                message: "equality variants with different case counts".to_string(),
            });
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
            let left_representation = self.lower_type(left_member)?;
            let right_representation = self.lower_type(right_member)?;
            let equal = if self.is_singleton_representation(left_representation)
                && self.is_singleton_representation(right_representation)
            {
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
        // fold two singletons by type identity
        if let (
            LoweredOperand::Singleton { ty: left, .. },
            LoweredOperand::Singleton { ty: right, .. },
        ) = (left, right)
        {
            return Ok(self.builder.bconst(left == right));
        }

        // a singleton meeting a scalar materializes its literal at the scalar
        let (left, right) = self.materialize_singleton_operand(left, right)?;

        // compare two parameter operands as their instance decides
        if let (LoweredOperand::Polymorphic(left), LoweredOperand::Polymorphic(right)) =
            (&left, &right)
        {
            return Ok(self
                .builder
                .binary(mir::BinaryOperator::Equal, *left, *right));
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
            return Err(self.unsupported(format!("{left_domain:?} value equality")));
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
                return Err(self.unsupported(format!(
                    "the '{}' operator on pointers or references",
                    other.text()
                )));
            }
        };

        // fold two singletons by their types
        if let (
            LoweredOperand::Singleton { ty: left, .. },
            LoweredOperand::Singleton { ty: right, .. },
        ) = (left, right)
        {
            return Ok(self.builder.bconst((left == right) == is_equal));
        }

        // compare a singleton with the variant case tag holding it
        let variant_singleton = match (&left, &right) {
            (LoweredOperand::Variant { value, .. }, LoweredOperand::Singleton { ty, .. })
            | (LoweredOperand::Singleton { ty, .. }, LoweredOperand::Variant { value, .. }) => {
                Some((*value, *ty))
            }
            _ => None,
        };
        if let Some((value, singleton)) = variant_singleton {
            let tag = self.builder.variant_tag(value);
            let held = self.value_representation(value)?;
            let equal = self.lower_case_tag_test(tag, held, singleton)?;

            return Ok(match is_equal {
                true => equal,
                false => self.builder.unary(mir::UnaryOperator::Not, equal),
            });
        }

        // compare an address with the null it may hold
        let (left, right) = match (left, right) {
            (LoweredOperand::Address(left), LoweredOperand::Address(right)) => (left, right),
            (LoweredOperand::Address(address), LoweredOperand::Singleton { ty, .. })
            | (LoweredOperand::Singleton { ty, .. }, LoweredOperand::Address(address)) => {
                let null = self.lower_null_address(ty, address)?;

                (address, null)
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

    /// Test one case tag against the case of a variant holding one payload type.
    fn lower_case_tag_test(
        &mut self,
        tag: mir::Value,
        variant: mir::TypeId,
        payload: mir::TypeId,
    ) -> CompilerResult<mir::Value> {
        // select the case holding the payload
        let variant = self.resolved_type(variant);
        let mir::Type::Variant { cases, .. } = self.builder.tree().type_definition(variant).clone()
        else {
            return Err(CompilerError::Internal {
                message: "a case test outside a variant".to_string(),
            });
        };

        let Some(case) = cases.iter().find(|case| case.ty == payload) else {
            return Err(self.internal("a case test outside the variant's cases"));
        };

        // compare the tag with the case's discriminant
        let tag_type = self.value_representation(tag)?;
        let discriminant = case.discriminant.clone();
        let expected = self.builder.constant(discriminant, tag_type);

        Ok(self
            .builder
            .binary(mir::BinaryOperator::Equal, tag, expected))
    }

    /// Materialize the null one address value compares with, the singleton being null.
    fn lower_null_address(
        &mut self,
        singleton: mir::TypeId,
        address: mir::Value,
    ) -> CompilerResult<mir::Value> {
        let null = self
            .lower
            .singleton_type(self.builder.tree_mut(), &dir::Literal::Null);
        if singleton != null {
            return Err(CompilerError::Internal {
                message: "an address compared with a singleton other than null".to_string(),
            });
        }
        let representation = self.value_representation(address)?;

        Ok(self.builder.constant(mir::Constant::Null, representation))
    }
}
