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
    /// One reference value.
    Reference(mir::Value),
    /// One materialized variant value and its logical carrier.
    Variant {
        /// The lowered value.
        value: mir::Value,
        /// The checked type defining the cases.
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
    /// Lower one binary operation over the checked operand carrier.
    pub(in crate::lower) fn lower_binary(
        &mut self,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        let left = self.lower_operand(left)?;
        let right = self.lower_operand(right)?;

        self.lower_binary_operands(left, operator, right)
    }

    /// Lower one builtin strict equality operation.
    pub(in crate::lower) fn lower_strict_equality(
        &mut self,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        let left = self.lower_operand(left)?;
        let right = self.lower_operand(right)?;
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
        let ty = self.coerced_type_id(expression)?;
        let Some(layer) = self.lowerer.peel_reference(ty)? else {
            return Ok(ty);
        };
        if self.lowerer.type_is_reference(layer.stored)? {
            return Ok(ty);
        }

        Ok(layer.stored)
    }

    /// Lower one operand, reading borrowed values out of their views.
    pub(in crate::lower) fn lower_operand(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<LoweredOperand> {
        let carrier = self.operand_carrier(expression)?;
        let coerced = self.coerced_type_id(expression)?;

        // leave standalone nullish values unmaterialized until they meet a reference
        if carrier == coerced {
            match self.node_type(expression)? {
                dir::Type::Null => return Ok(LoweredOperand::Null),
                dir::Type::Undefined => return Ok(LoweredOperand::Undefined),
                _ => {}
            }
        }
        let mut value = self.lower_expression(expression)?;

        // read non-reference values through their borrowed views
        if carrier != coerced {
            let pointee = self.lower_type(carrier)?;
            value = self.builder.load(value, pointee);
        }

        self.classify_equality_operand(carrier, value)
    }

    /// Classify one evaluated value by its runtime equality carrier.
    fn classify_equality_operand(
        &mut self,
        mut carrier: dir::GlobalTypeId,
        mut value: mir::Value,
    ) -> CompilerResult<LoweredOperand> {
        carrier = self.lowerer.reduced_type(carrier)?;

        // transparent newtypes compare through their backing representation
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

        // value enums compare through their integer discriminants
        let ty = self.lowerer.ty(carrier)?;
        if self.is_enum_operand(&ty)? {
            let value = self.builder.variant_tag(value);

            return Ok(LoweredOperand::Scalar {
                value,
                domain: dir::ScalarDomain::Integer,
            });
        }

        // scalar semantics take precedence over class-backed representations
        if let Some(domain) = self.scalar_domain(carrier)? {
            return Ok(LoweredOperand::Scalar { value, domain });
        }

        let Some(ty) = self.builder.value_type(value) else {
            return Err(CompilerError::Internal {
                message: "lowered equality operand has no MIR type".to_string(),
            });
        };
        match self.builder.tree().get(ty) {
            mir::Type::Reference { .. } => Ok(LoweredOperand::Reference(value)),
            mir::Type::Variant { .. } => Ok(LoweredOperand::Variant { value, carrier }),
            other => Err(CompilerError::Internal {
                message: format!(
                    "checked equality type lowered to the unsupported MIR carrier {other:?}"
                ),
            }),
        }
    }

    /// Return one type's scalar domain when every union member agrees.
    fn scalar_domain(
        &self,
        carrier: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::ScalarDomain>> {
        let ty = self.lowerer.ty(carrier)?;
        let dir::Type::Union(union) = ty else {
            return Ok(ty.scalar_domain());
        };

        let mut domain = None;
        for element in self
            .lowerer
            .types(carrier.module_id)?
            .type_ids(union.elements)
        {
            let Some(element) = self.scalar_domain(*element)? else {
                return Ok(None);
            };
            match domain {
                None => domain = Some(element),
                Some(domain) if domain == element => {}
                Some(_) => return Ok(None),
            }
        }

        Ok(domain)
    }

    /// Return whether one checked type has one value and no runtime payload.
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

    /// Return whether one operand type is a value enum or its member.
    pub(super) fn is_enum_operand(&self, operand: &dir::Type) -> CompilerResult<bool> {
        let symbol = match operand {
            dir::Type::Application(instance) => instance.symbol,
            dir::Type::EnumMember(member) => member.member,
            _ => return Ok(false),
        };
        if matches!(operand, dir::Type::EnumMember(_)) {
            return Ok(true);
        }

        Ok(matches!(
            self.lowerer.definition(symbol)?,
            Some(dir::Definition::Enum(_))
        ))
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
            return self.lower_reference_equality(left, operator, right);
        };
        if left_domain != right_domain {
            return Err(CompilerError::Internal {
                message: format!(
                    "checked binary operands have different scalar domains: {left_domain:?} and \
                     {right_domain:?}"
                ),
            });
        }
        let operator = self.binary_value_operator(operator, left_value)?;

        Ok(self.builder.binary_op(operator, left_value, right_value))
    }

    /// Lower equality over one checked common carrier.
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
                    message: "checked equality operands use different runtime carriers".to_string(),
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
                message: "checked equality variants have different case counts".to_string(),
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
                    let left = self.classify_equality_operand(left_member, left)?;
                    let right = self.builder.variant_payload(right, index);
                    let right = self.classify_equality_operand(right_member, right)?;

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
                        message: "checked variant carrier is not a tagged newtype".to_string(),
                    });
                };
                if !definition.is_tagged() {
                    return Err(CompilerError::Internal {
                        message: "checked variant carrier is not a tagged newtype".to_string(),
                    });
                }

                Ok(definition
                    .tagged_variants()
                    .map(|variant| variant.backing)
                    .collect())
            }
            other => Err(CompilerError::Internal {
                message: format!("checked variant carrier has type {other:?}"),
            }),
        }
    }

    /// Lower equality between two non-variant operands.
    fn lower_leaf_equality(
        &mut self,
        left: LoweredOperand,
        right: LoweredOperand,
    ) -> CompilerResult<mir::Value> {
        // payload-free values of one checked carrier are equal
        if matches!(left, LoweredOperand::Singleton) && matches!(right, LoweredOperand::Singleton) {
            return Ok(self.builder.bconst(true));
        }

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
            return self.lower_reference_equality(left, dir::BinaryOperator::EqualStrict, right);
        };
        if left_domain != right_domain {
            return Err(CompilerError::Internal {
                message: format!(
                    "checked equality operands have different scalar domains: {left_domain:?} and \
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

    /// Lower equality involving references or unmaterialized nullish values.
    fn lower_reference_equality(
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
                    construct: format!("the '{}' operator on references", other.text()),
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

        // materialize nullish niches at the compared reference carrier
        let (left, right) = match (left, right) {
            (LoweredOperand::Reference(left), LoweredOperand::Reference(right)) => (left, right),
            (LoweredOperand::Reference(reference), nullish) => {
                let nullish = self.lower_nullish(nullish, reference)?;

                (reference, nullish)
            }
            (nullish, LoweredOperand::Reference(reference)) => {
                let nullish = self.lower_nullish(nullish, reference)?;

                (nullish, reference)
            }
            _ => {
                return Err(CompilerError::Internal {
                    message: "checked equality comparison has incompatible carriers".to_string(),
                });
            }
        };
        let operator = match is_equal {
            true => mir::BinaryOperator::Equal,
            false => mir::BinaryOperator::NotEqual,
        };

        Ok(self.builder.binary_op(operator, left, right))
    }

    /// Materialize one nullish operand at a reference value's carrier.
    fn lower_nullish(
        &mut self,
        operand: LoweredOperand,
        reference: mir::Value,
    ) -> CompilerResult<mir::Value> {
        let Some(carrier) = self.builder.value_type(reference) else {
            return Err(CompilerError::Internal {
                message: "lowered MIR compared an untyped reference".to_string(),
            });
        };
        let constant = match operand {
            LoweredOperand::Null => mir::Constant::Null,
            LoweredOperand::Undefined => mir::Constant::Undefined,
            _ => {
                return Err(CompilerError::Internal {
                    message: "lowered equality operand is not nullish".to_string(),
                });
            }
        };

        Ok(self.builder.constant(constant, carrier))
    }
}
