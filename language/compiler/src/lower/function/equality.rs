use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

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
            _ => Err(CompilerError::Internal {
                message: "strict equality lowering received a different operator".to_string(),
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
            _ => {
                return Err(CompilerError::Internal {
                    message: "discriminant equality received a different operator".to_string(),
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
        let dir::MemberTarget::Projection {
            receiver,
            projection: dir::Projection::Discriminant { union, cases, .. },
            ..
        } = &access.target
        else {
            return Err(CompilerError::Internal {
                message: "a selected discriminant access has a different target".to_string(),
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
                    message: "a discriminant decision is attached to a non-access expression"
                        .to_string(),
                });
            }
        };

        // evaluate the discriminant receiver and computed key exactly once
        let receiver = self.lower_adjusted_receiver(left, &receiver)?;
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
                message: "a discriminant comparison selects an absent union arm".to_string(),
            });
        };

        // compare the tag with a constant of its exact integer representation
        let tag_type = self.value_representation(tag)?;
        let mir::Type::Int { width, is_signed } = *self.builder.tree().get(tag_type) else {
            return Err(CompilerError::Internal {
                message: "a lowered discriminant tag is not an integer".to_string(),
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
        let ty = self.node_type_id(expression)?;
        let Some(layer) = self.lowerer.peel_indirection(ty)? else {
            return Ok(ty);
        };
        if self.lowerer.peel_indirection(layer.stored)?.is_some() {
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
        let source = expression.into_global(self.source);
        if source != operand.source {
            return Err(CompilerError::Internal {
                message: format!(
                    "builtin operand {:?} is attached to expression {:?}",
                    operand.source, source
                ),
            });
        }

        // leave standalone nullish values unmaterialized until they meet a reference
        let mut representation = self.lowerer.instance_type(self.instance, operand.ty)?;
        match self.node_type(expression)? {
            dir::Type::Null => return Ok(LoweredOperand::Null),
            dir::Type::Undefined => return Ok(LoweredOperand::Undefined),
            _ => {}
        }

        // evaluate the operand at its own representation
        let mut value = self.lower_expression(expression)?;

        // read inline values through their indirect representations
        if let Some(layer) = self.lowerer.peel_indirection(representation)?
            && !self.lowerer.has_indirect_representation(layer.stored)?
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
            let dir::Type::Application(instance) = self.lowerer.ty(representation)? else {
                break;
            };
            let Some(dir::Definition::Newtype(definition)) =
                self.lowerer.definition(instance.symbol)?
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
                        message: "the scalar family set is empty".to_string(),
                    })?;
            match family {
                dir::ScalarFamily::Domain(domain) => {
                    return Ok(LoweredOperand::Scalar { value, domain });
                }
                dir::ScalarFamily::Enum(_) => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "an enum equality representation {representation:?} lowered to {ty:?}"
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
            // reference-family representations compare by identity
            mir::Type::Dynamic { .. }
            | mir::Type::Reference { .. }
            | mir::Type::Pointer { .. }
            | mir::Type::Slice { .. } => Ok(LoweredOperand::Address(value)),
            other => Err(CompilerError::Internal {
                message: format!(
                    "an equality type lowered to the unsupported representation {other:?}"
                ),
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
            match self.lowerer.ty(ty)? {
                dir::Type::Literal(_) | dir::Type::Null | dir::Type::Undefined => return Ok(true),
                dir::Type::Application(instance) => {
                    let Some(dir::Definition::Newtype(definition)) =
                        self.lowerer.definition(instance.symbol)?
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
    pub(in crate::lower) fn lower_binary_operands(
        &mut self,
        left: LoweredOperand,
        operator: dir::BinaryOperator,
        right: LoweredOperand,
    ) -> CompilerResult<mir::Value> {
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
                    "binary operands have different scalar domains: {left_domain:?} and \
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
            (LoweredOperand::Variant { .. }, _) | (_, LoweredOperand::Variant { .. }) => {
                Err(CompilerError::Internal {
                    message: "equality operands use different runtime representations".to_string(),
                })
            }
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
        let left_members = self.union_members(left_representation)?;
        let right_members = self.union_members(right_representation)?;
        if left_members.len() != right_members.len() {
            return Err(CompilerError::Internal {
                message: "equality variants have different case counts".to_string(),
            });
        }

        // compare the shared discriminant once
        let left_tag = self.builder.variant_tag(left);
        let right_tag = self.builder.variant_tag(right);
        let same_case = self
            .builder
            .binary(mir::BinaryOperator::Equal, left_tag, right_tag);
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
                    "equality operands have different scalar domains: {left_domain:?} and \
                     {right_domain:?}"
                ),
            });
        }
        if matches!(
            left_domain,
            dir::ScalarDomain::String | dir::ScalarDomain::Bigint
        ) {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
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
        let is_equal = match operator {
            dir::BinaryOperator::EqualStrict | dir::BinaryOperator::Equal => true,
            dir::BinaryOperator::NotEqualStrict | dir::BinaryOperator::NotEqual => false,
            other => {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
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
                    message: "the equality comparison has incompatible representations".to_string(),
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
            if self.lowerer.ty(*member)? == nullish {
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
                message: "a lowered discriminant tag is not an integer".to_string(),
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
        let representation = self.value_representation(address)?;
        let constant = match operand {
            LoweredOperand::Null => mir::Constant::Null,
            LoweredOperand::Undefined => mir::Constant::Undefined,
            _ => {
                return Err(CompilerError::Internal {
                    message: "the lowered equality operand is not nullish".to_string(),
                });
            }
        };

        Ok(self.builder.constant(constant, representation))
    }
}
