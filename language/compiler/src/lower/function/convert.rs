use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::operand::{Constant, Operand};
use crate::{CompilerError, CompilerResult};

impl FunctionLowerer<'_, '_, '_> {
    /// Convert an evaluated value through its selected coercion.
    pub(in crate::lower) fn convert_value(
        &mut self,
        value: mir::Value,
        coercion: Option<&dir::Coercion>,
    ) -> CompilerResult<mir::Value> {
        match coercion {
            Some(coercion) => {
                let operand = self.convert(Operand::Value(value), coercion, None)?;

                self.as_value(operand, coercion.target())
            }
            None => Ok(value),
        }
    }

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
            // own a fresh literal through the clone call the selection recorded
            dir::CoercionAdjustment::Clone { call, .. } => {
                let value = self.as_value(operand, source)?;
                let owned = self
                    .lower_target_call(Operand::Value(value), call)?
                    .ok_or_else(|| self.internal("a clone coercion without its value"))?;

                Ok(Operand::Value(owned))
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
                    Operand::Place(place) => self.borrow_place(&place, target)?,
                    operand => {
                        let value = self.as_value(operand, source)?;

                        self.borrow_value(value, target)?
                    }
                };

                Ok(Operand::Value(value))
            }
            // load the payload behind a borrow
            dir::CoercionAdjustment::Read { target } => {
                let value = self.as_value(operand, source)?;
                let target = self.lower_type(*target)?;
                let place = mir::Place::value(value).with_projection(mir::Projection::Deref);

                Ok(Operand::Value(self.load_place(place, target)))
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
            // wrap or unwrap a newtype layer, reinterpreting a pointer in place
            dir::CoercionAdjustment::Newtype { target } => {
                let value = self.as_value(operand, source)?;
                let target = self.lower_type(*target)?;
                let represented = mir::Substitution::resolve(target, self.builder.tree_mut());
                let value = match self.builder.tree().get(represented) {
                    mir::Type::Pointer { .. } | mir::Type::Reference { .. } => {
                        self.builder.cast(mir::CastOperator::Bitcast, value, target)
                    }
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
                let value =
                    self.lower_instantiated_value(expression, symbol, *target, arguments)?;

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
        target: mir::TypeId,
    ) -> CompilerResult<mir::Value> {
        let received = self.value_representation(value)?;
        let resolved_received = self.resolved_type(received);
        let resolved_target = self.resolved_type(target);
        let tree = self.builder.tree();
        if resolved_received == resolved_target {
            return Ok(value);
        }
        let is_received_reference = tree.get(resolved_received).is_reference_representation();
        let is_target_reference = tree.get(resolved_target).is_reference_representation();
        let is_open_target = matches!(
            tree.get(resolved_target),
            mir::Type::Parameter {
                referent: false,
                ..
            }
        );
        match (is_received_reference, is_target_reference) {
            (true, true) => self.reborrow_or_reinterpret(value, target),
            // complete a held value into a managed allocation, or into an open form at instantiation
            (false, true) => Ok(self.builder.new_complete(value, target)),
            (false, false) if is_open_target => Ok(self.builder.new_complete(value, target)),
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
        let stored = self.builder.tree_mut().storage_type(representation);
        if !matches!(
            self.builder.tree().type_definition(stored),
            mir::Type::Variant { .. }
        ) {
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
        let members = self.lower.union_members(target)?;
        let case = self.case(&members, source)?;
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
        if !matches!(
            self.builder.tree().type_definition(received),
            mir::Type::Variant { .. }
        ) {
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
        let source_members = self.lower.union_members(source)?;
        let mut targets = Vec::with_capacity(members.len());
        for member in members {
            targets.push((self.case(&source_members, *member)?, self.builder.block()));
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
        let Some(narrowing) = self.representation_narrowing(expression)? else {
            return Ok(value);
        };
        let target = self.node_type_id(expression)?;

        self.narrow(value, narrowing.union, &narrowing.arms, target)
    }

    /// Return the narrowing one read converts through.
    pub(in crate::lower) fn representation_narrowing(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::Narrowing>> {
        let node = expression.into_global_any(self.source);
        let Some(narrowing) = self.source().decisions.narrowing(node).cloned() else {
            return Ok(None);
        };
        let target = self.node_type_id(expression)?;
        let union = self.lower_type(narrowing.union)?;
        let narrowed = self.lower_type(target)?;

        Ok((union != narrowed).then_some(narrowing))
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
        union: dir::GlobalTypeId,
        live: &[dir::GlobalTypeId],
        target: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Value> {
        if union == target {
            return Ok(value);
        }

        // lower the recorded live members
        let members = self.lower.union_members(union)?;
        let narrowed = match live {
            [member] => self.lower_type(*member)?,
            _ => self.lower_type(target)?,
        };
        let representation = self.value_representation(value)?;

        // address the one live payload behind a referenced variant
        let resolved = self.resolved_type(representation);
        if let mir::Type::Reference {
            kind,
            access,
            pointee,
            ..
        } = *self.builder.tree().get(resolved)
            && let pointee = self.resolved_type(pointee)
            && matches!(self.builder.tree().get(pointee), mir::Type::Variant { .. })
        {
            let [member] = live else {
                return Err(self.unsupported("narrowing a referenced union to several members"));
            };
            let case = self.case(&members, *member)?;
            let lifetime = self.reborrow_lifetime(value);
            let result = self.insert_reference(kind, lifetime, access, narrowed);

            let place = mir::Place::value(value)
                .with_projection(mir::Projection::Deref)
                .with_projection(mir::Projection::Variant { case });

            return Ok(self.builder.address(place, result));
        }

        // keep a value outside a variant as it is
        let variant = self.builder.tree_mut().storage_type(representation);
        if !matches!(
            self.builder.tree().type_definition(variant),
            mir::Type::Variant { .. }
        ) {
            return Ok(value);
        }

        // read the single live payload as the value
        if let [member] = live {
            let case = self.case(&members, *member)?;

            return Ok(match self.is_singleton_representation(narrowed) {
                true => self.builder.constant(mir::Constant::Zeroed, narrowed),
                false => self.builder.variant_payload(value, case),
            });
        }

        // switch the live cases into the narrowed variant, trapping on the rest
        let destination = self.builder.local(narrowed, mir::Mutability::Mutable);
        let exit = self.builder.block();
        let trap = self.builder.block();
        let mut targets = Vec::with_capacity(live.len());
        for member in live {
            targets.push((self.case(&members, *member)?, self.builder.block()));
        }
        self.builder
            .variant_switch(value, Some(trap), targets.clone());
        self.builder.switch_to_block(trap);
        self.builder.panic(None);
        let target_members = self.lower.union_members(target)?;
        for (member, (case, block)) in live.iter().zip(targets) {
            self.builder.switch_to_block(block);

            // emit the member at the target union's variant or scalar representation
            let projected = match self.builder.tree().get(narrowed) {
                mir::Type::Variant { .. } => {
                    let projected = match self.case_has_payload(*member)? {
                        true => Some(self.builder.variant_payload(value, case)),
                        false => None,
                    };
                    let narrowed_case = self.case(&target_members, *member)?;

                    self.builder.variant_new(narrowed, narrowed_case, projected)
                }
                _ => {
                    let literal = self.singleton_literal(*member)?;

                    self.lower_constant(literal, narrowed)?
                }
            };
            self.builder.local_set(destination, projected);
            self.builder.jump(exit);
        }
        self.builder.switch_to_block(exit);

        Ok(self.builder.local_get(destination))
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
