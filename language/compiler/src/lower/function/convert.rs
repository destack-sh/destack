use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::operand::{Constant, Operand};
use crate::{CompilerError, CompilerResult};

impl FunctionLowerer<'_, '_, '_> {
    /// Convert one operand along its recorded coercion path.
    pub(in crate::lower) fn convert(
        &mut self,
        operand: Operand,
        coercion: &dir::Coercion,
        expression: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<Operand> {
        let mut operand = operand;
        let mut source = coercion.source;
        for adjustment in &coercion.adjustments {
            operand = self.adjust(operand, source, adjustment, expression)?;
            source = adjustment.target();
        }

        Ok(operand)
    }

    /// Apply one recorded adjustment to an operand at its source type.
    fn adjust(
        &mut self,
        operand: Operand,
        source: dir::GlobalTypeId,
        adjustment: &dir::CoercionAdjustment,
        expression: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<Operand> {
        match adjustment {
            // widen a constant into its runtime scalar
            dir::CoercionAdjustment::Materialize { target } => {
                let value = self.as_value(operand, *target)?;

                Ok(Operand::Value(value))
            }
            // bind a callable declaration at the representation it converts to
            dir::CoercionAdjustment::Representation { target }
            | dir::CoercionAdjustment::Manage { target }
                if matches!(operand, Operand::Constant(Constant::Callable { .. })) =>
            {
                let value = self.as_value(operand, *target)?;

                Ok(Operand::Value(value))
            }
            // take a reference to the operand's place
            dir::CoercionAdjustment::Borrow { target } => {
                let target = self.lower_type(*target)?;
                let value = match operand {
                    Operand::Value(value)
                        if self.lower.indirection(source, &self.scope)?.is_some() =>
                    {
                        self.builder.cast(mir::CastOperator::Bitcast, value, target)
                    }
                    operand => {
                        let place = self.as_place(operand, source)?;

                        self.borrow_place(&place, target, mir::AddressKind::Borrow)?
                    }
                };

                Ok(Operand::Value(value))
            }
            // load the payload behind a borrow
            dir::CoercionAdjustment::Read { target } => {
                let value = self.as_value(operand, source)?;
                let target = self.lower_type(*target)?;

                Ok(Operand::Value(self.builder.load(value, target)))
            }
            // cast between scalar representations
            dir::CoercionAdjustment::Scalar { target } => {
                let value = self.as_value(operand, source)?;
                let from = self.value_representation(value)?;
                let target = self.lower_type(*target)?;
                if from == target {
                    return Ok(Operand::Value(value));
                }
                let operator = self.cast_operator(
                    self.builder.tree().get(from),
                    self.builder.tree().get(target),
                )?;

                Ok(Operand::Value(self.builder.cast(operator, value, target)))
            }
            // wrap a backing value in its newtype, or unwrap it to the backing
            dir::CoercionAdjustment::Newtype { target } => {
                let value = self.as_value(operand, source)?;
                let target = self.lower_type(*target)?;
                let represented = self.builder.tree().represented(target);
                let value = match self.builder.tree().get(represented) {
                    mir::Type::Newtype { .. } => self.builder.aggregate(target, vec![value]),
                    _ => self.builder.field_get(value, 0),
                };

                Ok(Operand::Value(value))
            }
            // enter the union case the record names
            dir::CoercionAdjustment::Union { target, cases } => {
                let value = self.inject(operand, source, *target, cases)?;

                Ok(Operand::Value(value))
            }
            // box the value behind its erased constraint
            dir::CoercionAdjustment::Erase { target } => {
                let value = self.as_value(operand, source)?;
                let value = self.lower_erasure(value, source, *target)?;

                Ok(Operand::Value(value))
            }
            // reinterpret one ownership as another, owned storage handed to managed storage
            dir::CoercionAdjustment::Representation { target }
            | dir::CoercionAdjustment::Manage { target } => {
                let value = self.as_value(operand, source)?;
                let target = self.lower_type(*target)?;
                let value = self.adopt(value, target)?;

                Ok(Operand::Value(value))
            }
            // select the instance a generic reference instantiates
            dir::CoercionAdjustment::Instantiate { target, arguments } => {
                let Some(expression) = expression else {
                    return Err(CompilerError::Internal {
                        message: "an instantiate coercion away from its reference".to_string(),
                    });
                };
                let node = expression.into_global_any(self.source);
                let symbol = self.lower.resolved_symbol(node)?;
                let value = self.lower_instantiated_value(symbol, *target, arguments)?;

                Ok(Operand::Value(value))
            }
            other => Err(CompilerError::Internal {
                message: format!("an unlowered {} coercion", other.as_str()),
            }),
        }
    }

    /// Bring one value to another ownership, a held value moving into a managed allocation.
    pub(in crate::lower) fn adopt(
        &mut self,
        value: mir::Value,
        target: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::Value> {
        let received = self.value_representation(value)?;
        let tree = self.builder.tree();
        if tree.same_representation(mir::TypeId::from(received), mir::TypeId::from(target)) {
            return Ok(value);
        }
        let is_received_reference = tree
            .get(self.resolved_type(received))
            .is_reference_representation();
        let is_target_reference = tree
            .get(self.resolved_type(target))
            .is_reference_representation();
        match (is_received_reference, is_target_reference) {
            (true, true) => Ok(self.builder.cast(mir::CastOperator::Bitcast, value, target)),
            (false, true) => Ok(self.builder.new_complete(value, target)),
            _ => Err(CompilerError::Internal {
                message: format!(
                    "a representation coercion from {:?} to {:?} outside reference storage in '{}'",
                    tree.get(received),
                    tree.get(target),
                    self.lower
                        .strings
                        .get(tree.get(self.builder.function_id()).name)
                ),
            }),
        }
    }

    /// Inject one operand into the union case its record names, a union source case by case.
    pub(in crate::lower) fn inject(
        &mut self,
        operand: Operand,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        cases: &[dir::CoercionCase],
    ) -> CompilerResult<mir::Value> {
        let representation = self.lower_type(target)?;

        // convert a union source case by case into the target
        if let Some(members) = self.lower.union_members_maybe(source)? {
            let value = self.as_value(operand, source)?;

            return self.convert_cases(value, source, &members, target, cases);
        }

        // a singular source converts through its one case
        let (operand, source) = match cases {
            [] => (operand, source),
            [case] if case.adjustments.is_empty() => (operand, case.target),
            [case] => {
                let coercion =
                    dir::Coercion::new(source, case.adjustments.clone(), dir::CastOrigin::Implicit);

                (self.convert(operand, &coercion, None)?, case.target)
            }
            _ => {
                return Err(CompilerError::Internal {
                    message: "several cases on a singular union injection".to_string(),
                });
            }
        };

        // an untagged union holds the value itself, a constant materializing at it
        let stored = self
            .builder
            .tree()
            .storage_type(mir::TypeId::from(representation));
        if !matches!(self.builder.tree().get(stored), mir::Type::Variant { .. }) {
            return match operand {
                Operand::Constant(Constant::Literal(literal)) => {
                    self.lower_constant(literal, representation)
                }
                operand => {
                    let value = self.as_value(operand, source)?;

                    self.adopt(value, representation)
                }
            };
        }
        let value = self.as_value(operand, source)?;
        let case = self.case(target, source)?;
        let payload = self.case_has_payload(source)?.then_some(value);

        Ok(self.builder.variant_new(representation, case, payload))
    }

    /// Convert one union value into a target case by case, each member through its recorded case.
    fn convert_cases(
        &mut self,
        value: mir::Value,
        source: dir::GlobalTypeId,
        members: &[dir::GlobalTypeId],
        target: dir::GlobalTypeId,
        cases: &[dir::CoercionCase],
    ) -> CompilerResult<mir::Value> {
        if cases.len() != members.len() {
            return Err(CompilerError::Internal {
                message: "a union conversion with an incomplete case map".to_string(),
            });
        }
        let representation = self.lower_type(target)?;
        let received = self.value_representation(value)?;

        // an untagged source converts through its one shared case
        if !matches!(self.builder.tree().get(received), mir::Type::Variant { .. }) {
            let Some(first) = cases.first() else {
                return Err(CompilerError::Internal {
                    message: "a union conversion without cases".to_string(),
                });
            };
            let converted = self.convert_case(Operand::Value(value), members[0], first)?;

            return self.inject(converted, first.target, target, &[]);
        }

        // dispatch on the source case and rebuild each member under its target case
        let destination = self.builder.local(representation, mir::Mutability::Mutable);
        let exit = self.builder.block();
        let mut targets = Vec::with_capacity(members.len());
        for member in members {
            targets.push((self.case(source, *member)?, self.builder.block()));
        }
        self.builder.variant_switch(value, None, targets.clone());
        for ((member, case), (position, block)) in members.iter().zip(cases).zip(targets) {
            self.builder.switch_to_block(block);
            let payload = match self.case_has_payload(*member)? {
                true => Operand::Value(self.builder.variant_payload(value, position)),
                false => Operand::Constant(Constant::Literal(self.singleton_literal(*member)?)),
            };
            let converted = self.convert_case(payload, *member, case)?;
            let converted = self.inject(converted, case.target, target, &[])?;
            self.builder.local_set(destination, converted);
            self.builder.jump(exit);
        }
        self.builder.switch_to_block(exit);

        Ok(self.builder.local_get(destination))
    }

    /// Project one read onto the cases its recorded narrowing keeps live.
    pub(in crate::lower) fn lower_narrowing(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        value: mir::Value,
    ) -> CompilerResult<mir::Value> {
        let node = expression.into_global_any(self.source);
        let Some(narrowing) = self
            .lower
            .state(self.source)?
            .decisions
            .narrowing(node)
            .cloned()
        else {
            return Ok(value);
        };
        let target = self.node_type_id(expression)?;

        self.narrow(value, &narrowing.arms, target)
    }

    /// Convert one operand through the adjustments one coercion case records.
    fn convert_case(
        &mut self,
        operand: Operand,
        source: dir::GlobalTypeId,
        case: &dir::CoercionCase,
    ) -> CompilerResult<Operand> {
        if case.adjustments.is_empty() {
            return Ok(operand);
        }
        let coercion =
            dir::Coercion::new(source, case.adjustments.clone(), dir::CastOrigin::Implicit);

        self.convert(operand, &coercion, None)
    }

    /// Narrow one union value onto the members a narrowing keeps live, trapping on the others.
    pub(in crate::lower) fn narrow(
        &mut self,
        value: mir::Value,
        live: &[dir::GlobalTypeId],
        target: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Value> {
        let leaves = self.union_leaves(live)?;
        let mut payloads = Vec::with_capacity(leaves.len());
        for member in &leaves {
            payloads.push(self.lower_type(*member)?);
        }
        let narrowed = match leaves.as_slice() {
            [_] => payloads[0],
            _ => self.lower_type(target)?,
        };

        self.narrow_value(value, &payloads, narrowed)
    }

    /// Return the leaf members some union members stand for, a nested union by its own members.
    pub(in crate::lower) fn union_leaves(
        &mut self,
        members: &[dir::GlobalTypeId],
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let mut leaves = Vec::with_capacity(members.len());
        for member in members {
            match self.lower.union_members_maybe(*member)? {
                Some(nested) => leaves.extend(self.union_leaves(&nested)?),
                None => leaves.push(*member),
            }
        }

        Ok(leaves)
    }

    /// Narrow one variant value onto the cases holding some payloads, trapping on the others.
    pub(in crate::lower) fn narrow_value(
        &mut self,
        value: mir::Value,
        payloads: &[mir::LocalNodeId<mir::Type>],
        narrowed: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::Value> {
        let representation = self.value_representation(value)?;
        if representation == narrowed {
            return Ok(value);
        }

        // address the one live payload behind a referenced variant
        if let mir::Type::Reference {
            kind,
            storage,
            access,
            pointee,
            ..
        } = *self.builder.tree().get(self.resolved_type(representation))
            && matches!(
                self.builder.tree().get(self.resolved_type(pointee)),
                mir::Type::Variant { .. }
            )
        {
            let [payload] = payloads else {
                return Err(self.unsupported("narrowing a referenced union to several members"));
            };
            let case = self.payload_case(pointee, *payload)?;
            let lifetime = self.reborrow_lifetime(value);
            let result = self.insert_reference(kind, lifetime, access, storage, *payload);

            return Ok(self.builder.variant_payload_addr(
                value,
                case,
                result,
                mir::AddressKind::Borrow,
            ));
        }

        // keep a value outside a variant as it is
        let variant = self
            .builder
            .tree()
            .storage_type(mir::TypeId::from(representation));
        if !matches!(self.builder.tree().get(variant), mir::Type::Variant { .. }) {
            return Ok(value);
        }

        // read the single live payload as the value
        if let [payload] = payloads {
            let case = self.payload_case(variant, *payload)?;

            return Ok(match self.is_singleton_representation(*payload) {
                true => self.builder.constant(mir::Constant::Zeroed, *payload),
                false => self.builder.variant_payload(value, case),
            });
        }

        // switch the live cases into the narrowed variant, trapping on the rest
        let destination = self.builder.local(narrowed, mir::Mutability::Mutable);
        let exit = self.builder.block();
        let trap = self.builder.block();
        let mut targets = Vec::with_capacity(payloads.len());
        for payload in payloads {
            targets.push((self.payload_case(variant, *payload)?, self.builder.block()));
        }
        self.builder
            .variant_switch(value, Some(trap), targets.clone());
        self.builder.switch_to_block(trap);
        self.builder.panic(None);
        for (payload, (case, block)) in payloads.iter().zip(targets) {
            self.builder.switch_to_block(block);
            let projected = match self.is_singleton_representation(*payload) {
                true => None,
                false => Some(self.builder.variant_payload(value, case)),
            };
            let narrowed_case = self.payload_case(narrowed, *payload)?;
            let projected = self.builder.variant_new(narrowed, narrowed_case, projected);
            self.builder.local_set(destination, projected);
            self.builder.jump(exit);
        }
        self.builder.switch_to_block(exit);

        Ok(self.builder.local_get(destination))
    }

    /// Return the case of one variant holding a payload, lifetimes aside.
    pub(in crate::lower) fn payload_case(
        &self,
        variant: mir::LocalNodeId<mir::Type>,
        payload: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<u32> {
        let tree = self.builder.tree();
        let variant = tree.storage_type(mir::TypeId::from(variant));
        let mir::Type::Variant { cases, .. } = tree.get(variant) else {
            return Err(CompilerError::Internal {
                message: "a payload case outside a variant".to_string(),
            });
        };
        let payload = mir::TypeId::from(payload);
        cases
            .iter()
            .position(|case| tree.same_representation(case.payload(tree), payload))
            .map(|case| case as u32)
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "a payload {:?} outside the variant's cases {:?}",
                    tree.get(payload),
                    cases
                        .iter()
                        .map(|case| tree.get(case.payload(tree)))
                        .collect::<Vec<_>>()
                ),
            })
    }

    /// Return whether one union member stores bytes.
    pub(in crate::lower) fn case_has_payload(
        &mut self,
        member: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        Ok(!matches!(
            self.lower.ty(member)?,
            dir::Type::Literal(_) | dir::Type::Null | dir::Type::Undefined
        ))
    }

    /// Return the literal one singleton member holds.
    fn singleton_literal(&mut self, member: dir::GlobalTypeId) -> CompilerResult<dir::Literal> {
        match self.lower.ty(member)? {
            dir::Type::Literal(literal) => Ok(literal),
            dir::Type::Null => Ok(dir::Literal::Null),
            dir::Type::Undefined => Ok(dir::Literal::Undefined),
            _ => Err(CompilerError::Internal {
                message: "a payload read as a singleton".to_string(),
            }),
        }
    }
}
