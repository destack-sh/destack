use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

/// One lowered operand classified by its runtime equality carrier.
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
    /// One materialized variant value and its logical carrier.
    Variant {
        /// The lowered value.
        value: mir::Value,
        /// The type defining the cases.
        carrier: dir::GlobalTypeId,
    },
    /// The unmaterialized null value.
    Null,
    /// The unmaterialized undefined value.
    Undefined,
    /// One payload-free value.
    Singleton,
}

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one binary operation over the operand carrier.
    pub(in crate::lower) fn lower_binary(
        &mut self,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
        operands: &[dir::BuiltinOperand; 2],
    ) -> CompilerResult<mir::Value> {
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
        let left = self.lower_operand(left, &operands[0])?;
        let right = self.lower_operand(right, &operands[1])?;
        let equal = self.lower_carrier_equality(left, right)?;

        match operator {
            dir::BinaryOperator::EqualStrict => Ok(equal),
            dir::BinaryOperator::NotEqualStrict => Ok(self.builder.bnot(equal)),
            _ => Err(CompilerError::Internal {
                message: "strict equality lowering received a different operator".to_string(),
            }),
        }
    }

    /// Return the type carrying one operand after reading through a view.
    pub(in crate::lower) fn operand_carrier(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let ty = self.node_type_id(expression)?;
        let Some(layer) = self.lowerer.peel_indirection(ty, &self.type_substitution)? else {
            return Ok(ty);
        };
        if self
            .lowerer
            .peel_indirection(layer.stored, &self.type_substitution)?
            .is_some()
        {
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
        let mut carrier = operand.ty;
        match self.node_type(expression)? {
            dir::Type::Null => return Ok(LoweredOperand::Null),
            dir::Type::Undefined => return Ok(LoweredOperand::Undefined),
            _ => {}
        }

        // evaluate the operand at its own carrier
        let mut value = self.lower_expression(expression)?;

        // read inline values through their indirect carriers
        if let Some(layer) = self
            .lowerer
            .peel_indirection(carrier, &self.type_substitution)?
            && !self
                .lowerer
                .has_indirect_representation(layer.stored, &self.type_substitution)?
        {
            carrier = layer.stored;
            let pointee = self.lower_type(carrier)?;
            value = self.builder.load(value, pointee);
        }

        self.classify_equality_operand(carrier, operand.scalar_families.as_ref(), value)
    }

    /// Classify one evaluated value by its runtime equality carrier.
    fn classify_equality_operand(
        &mut self,
        mut carrier: dir::GlobalTypeId,
        scalar_families: Option<&dir::ScalarFamilySet>,
        mut value: mir::Value,
    ) -> CompilerResult<LoweredOperand> {
        carrier = self.lowerer.reduced_type(carrier)?;

        // compare transparent newtypes through their backing representation
        loop {
            let dir::Type::Application(instance) = self.lowerer.ty(carrier)? else {
                break;
            };
            let Some(dir::Definition::Newtype(definition)) =
                self.lowerer.definition(instance.symbol)?
            else {
                break;
            };
            if definition.is_tagged() {
                break;
            }
            if self.type_is_singleton(definition.backing)? {
                return Ok(LoweredOperand::Singleton);
            }

            value = self.builder.field_get(value, 0);
            carrier = self.lowerer.reduced_type(definition.backing)?;
        }

        // classify the operand by the carrier it lowered to
        let Some(ty) = self.builder.value_type(value) else {
            return Err(CompilerError::Internal {
                message: "the lowered equality operand has no type".to_string(),
            });
        };
        let ty = self.builder.tree().get(ty);

        // preserve aggregate carriers even when every case shares scalar behavior
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

            return Ok(LoweredOperand::Variant { value, carrier });
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
                        message: format!("an enum equality carrier {carrier:?} lowered to {ty:?}"),
                    });
                }
            }
        }

        // compare nested variant payloads through their scalar carrier
        if let Some(domain) = Self::mir_scalar_domain(ty) {
            return Ok(LoweredOperand::Scalar { value, domain });
        }

        match ty {
            // reference-family carriers compare by identity
            mir::Type::Dynamic { .. }
            | mir::Type::Reference { .. }
            | mir::Type::Pointer { .. }
            | mir::Type::Slice { .. } => Ok(LoweredOperand::Address(value)),
            other => Err(CompilerError::Internal {
                message: format!("an equality type lowered to the unsupported carrier {other:?}"),
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
            ty = self.lowerer.reduced_type(ty)?;

            match self.lowerer.ty(ty)? {
                dir::Type::Literal(_) | dir::Type::Null | dir::Type::Undefined => return Ok(true),
                dir::Type::Application(instance) => {
                    let Some(dir::Definition::Newtype(definition)) =
                        self.lowerer.definition(instance.symbol)?
                    else {
                        return Ok(false);
                    };
                    if definition.is_tagged() {
                        return Ok(false);
                    }
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
        let operator = self.binary_value_operator(operator, left_value)?;

        Ok(self.builder.binary_op(operator, left_value, right_value))
    }

    /// Lower equality over one common carrier.
    pub(in crate::lower) fn lower_carrier_equality(
        &mut self,
        left: LoweredOperand,
        right: LoweredOperand,
    ) -> CompilerResult<mir::Value> {
        match (left, right) {
            (
                LoweredOperand::Variant {
                    value: left,
                    carrier: left_carrier,
                },
                LoweredOperand::Variant {
                    value: right,
                    carrier: right_carrier,
                },
            ) => self.lower_variant_equality(left_carrier, left, right_carrier, right),
            (LoweredOperand::Variant { .. }, _) | (_, LoweredOperand::Variant { .. }) => {
                Err(CompilerError::Internal {
                    message: "equality operands use different runtime carriers".to_string(),
                })
            }
            (left, right) => self.lower_leaf_equality(left, right),
        }
    }

    /// Lower equality between two values of one indexed variant carrier.
    fn lower_variant_equality(
        &mut self,
        left_carrier: dir::GlobalTypeId,
        left: mir::Value,
        right_carrier: dir::GlobalTypeId,
        right: mir::Value,
    ) -> CompilerResult<mir::Value> {
        let left_members = self.variant_members(left_carrier)?;
        let right_members = self.variant_members(right_carrier)?;
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
            .binary_op(mir::BinaryOperator::Equal, left_tag, right_tag);
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

                    self.lower_carrier_equality(left, right)?
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

    /// Return the logical members of one indexed variant carrier.
    fn variant_members(
        &self,
        carrier: dir::GlobalTypeId,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let carrier = self.lowerer.reduced_type(carrier)?;
        match self.lowerer.ty(carrier)? {
            dir::Type::Union(union) => Ok(self
                .lowerer
                .types(carrier.module_id)?
                .type_ids(union.elements)
                .to_vec()),
            dir::Type::Application(instance) => {
                let Some(dir::Definition::Newtype(definition)) =
                    self.lowerer.definition(instance.symbol)?
                else {
                    return Err(CompilerError::Internal {
                        message: "the variant carrier is not a tagged newtype".to_string(),
                    });
                };
                if !definition.is_tagged() {
                    return Err(CompilerError::Internal {
                        message: "the variant carrier is not a tagged newtype".to_string(),
                    });
                }

                Ok(definition
                    .tagged_variants()
                    .map(|variant| variant.backing)
                    .collect())
            }
            other => Err(CompilerError::Internal {
                message: format!("the variant carrier has type {other:?}"),
            }),
        }
    }

    /// Lower equality between two non-variant operands.
    fn lower_leaf_equality(
        &mut self,
        left: LoweredOperand,
        right: LoweredOperand,
    ) -> CompilerResult<mir::Value> {
        // treat payload-free values as equal
        if matches!(left, LoweredOperand::Singleton) && matches!(right, LoweredOperand::Singleton) {
            return Ok(self.builder.bconst(true));
        }

        // only two scalar operands compare directly, anything else by address
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
        let operator = self.binary_value_operator(dir::BinaryOperator::EqualStrict, left_value)?;

        Ok(self.builder.binary_op(operator, left_value, right_value))
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

        // fold comparisons with no runtime carrier
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

        // materialize nullish niches at the compared address carrier
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
                    message: "the equality comparison has incompatible carriers".to_string(),
                });
            }
        };
        let operator = match is_equal {
            true => mir::BinaryOperator::Equal,
            false => mir::BinaryOperator::NotEqual,
        };

        Ok(self.builder.binary_op(operator, left, right))
    }

    /// Materialize one nullish operand at an address value's carrier.
    fn lower_nullish(
        &mut self,
        operand: LoweredOperand,
        address: mir::Value,
    ) -> CompilerResult<mir::Value> {
        let Some(carrier) = self.builder.value_type(address) else {
            return Err(CompilerError::Internal {
                message: "an untyped address in an equality comparison".to_string(),
            });
        };
        let constant = match operand {
            LoweredOperand::Null => mir::Constant::Null,
            LoweredOperand::Undefined => mir::Constant::Undefined,
            _ => {
                return Err(CompilerError::Internal {
                    message: "the lowered equality operand is not nullish".to_string(),
                });
            }
        };

        Ok(self.builder.constant(constant, carrier))
    }
}
