use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

/// One value in flight along a coercion path.
pub(in crate::lower) enum CoercionValue {
    /// One source expression still to evaluate.
    Expression(dir::LocalNodeId<dir::Expression>),
    /// One materialized value.
    Runtime(mir::Value),
    /// One compile-time scalar literal.
    Literal(dir::Literal),
    /// The unmaterialized null value.
    Null,
    /// The unmaterialized undefined value.
    Undefined,
}

impl FunctionLowerer<'_, '_, '_> {
    /// Classify one expression before applying its coercion path.
    pub(in crate::lower) fn coercion_source(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<CoercionValue> {
        // keep sources whose value lives in their type unmaterialized
        let value = match self.lower.ty(source)? {
            dir::Type::Literal(literal) => CoercionValue::Literal(literal),
            dir::Type::Null => CoercionValue::Null,
            dir::Type::Undefined => CoercionValue::Undefined,
            _ => return Ok(CoercionValue::Expression(expression)),
        };

        // lower the const expression's remaining runtime evaluation
        self.lower_const_expression(expression)?;

        Ok(value)
    }

    /// Apply one complete adjustment path.
    pub(in crate::lower) fn lower_adjustments(
        &mut self,
        mut value: CoercionValue,
        mut source: dir::GlobalTypeId,
        adjustments: &[dir::CoercionAdjustment],
    ) -> CompilerResult<CoercionValue> {
        // apply each adjustment and advance the source type along the path
        for adjustment in adjustments {
            value = self.lower_adjustment(value, source, adjustment)?;
            source = adjustment.target();
        }

        Ok(value)
    }

    /// Apply one adjustment.
    fn lower_adjustment(
        &mut self,
        value: CoercionValue,
        source: dir::GlobalTypeId,
        adjustment: &dir::CoercionAdjustment,
    ) -> CompilerResult<CoercionValue> {
        match adjustment {
            // widen a constant into its runtime scalar
            dir::CoercionAdjustment::Materialize { target } => {
                let value = self.materialize_coercion_value(value, *target)?;

                Ok(CoercionValue::Runtime(value))
            }
            // take a reference to the value's place
            dir::CoercionAdjustment::Borrow { target } => {
                let target = self.lower_type(*target)?;
                let value = match value {
                    CoercionValue::Expression(expression) => {
                        self.lower_borrowed_place(expression, target)?
                    }
                    CoercionValue::Runtime(value)
                        if self.lower.has_indirect_representation(source)? =>
                    {
                        self.builder.cast(mir::CastOperator::Bitcast, value, target)
                    }
                    _ => {
                        return Err(CompilerError::Internal {
                            message: "a coercion borrow of a value without a place".to_string(),
                        });
                    }
                };

                Ok(CoercionValue::Runtime(value))
            }
            // load the payload behind a borrow
            dir::CoercionAdjustment::Read { target } => {
                let value = self.materialize_coercion_value(value, source)?;
                let target = self.lower_type(*target)?;
                let value = self.builder.load(value, target);

                Ok(CoercionValue::Runtime(value))
            }
            // cast between scalar representations
            dir::CoercionAdjustment::Scalar { target } => {
                let value = self.materialize_coercion_value(value, source)?;
                let source = self.value_representation(value)?;
                let target = self.lower_type(*target)?;
                let value = if self.builder.tree().get(source) == self.builder.tree().get(target) {
                    value
                } else {
                    let operator = self.cast_operator(
                        self.builder.tree().get(source),
                        self.builder.tree().get(target),
                    )?;

                    self.builder.cast(operator, value, target)
                };

                Ok(CoercionValue::Runtime(value))
            }
            // enter the union case the selection names
            dir::CoercionAdjustment::Union { target, cases } => {
                let value = self.lower_union_adjustment(value, source, *target, cases)?;

                Ok(CoercionValue::Runtime(value))
            }
            // box the value behind its erased constraint
            dir::CoercionAdjustment::Erase { target } => {
                let value = self.materialize_coercion_value(value, source)?;
                let value = self.lower_erasure(value, source, *target)?;

                Ok(CoercionValue::Runtime(value))
            }
            // reinterpret owned storage as managed storage
            dir::CoercionAdjustment::Manage { target } => {
                let value = self.materialize_coercion_value(value, source)?;
                let target = self.lower_type(*target)?;
                let value = self.builder.cast(mir::CastOperator::Bitcast, value, target);

                Ok(CoercionValue::Runtime(value))
            }
            // select the instance a generic reference instantiates
            dir::CoercionAdjustment::Instantiate { target, arguments } => {
                let CoercionValue::Expression(expression) = value else {
                    return Err(CompilerError::Internal {
                        message: "an instantiate coercion of a lowered value".to_string(),
                    });
                };
                let node = expression.into_global_any(self.source);
                let symbol = self.lower.resolved_symbol(node)?;
                let value = self.lower_instantiated_value(symbol, *target, arguments)?;

                Ok(CoercionValue::Runtime(value))
            }
            // reject every other adjustment
            adjustment => Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: format!("an implicit {} coercion", adjustment.as_str()),
            }
            .into()),
        }
    }

    /// Adjust one value into its union target representation.
    fn lower_union_adjustment(
        &mut self,
        value: CoercionValue,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        cases: &[dir::CoercionCase],
    ) -> CompilerResult<mir::Value> {
        // lower the union's own representation
        let representation = self.lower_type(target)?;

        // preserve the source case correspondence for indexed variants
        if let mir::Type::Variant { .. } = self.builder.tree().get(representation) {
            return self.lower_variant_adjustment(value, source, target, representation, cases);
        }

        // dispatch union sources before leaving their indexed representation
        if matches!(self.lower.ty(source)?, dir::Type::Union(_)) {
            return self.lower_union_exit(value, source, target, representation, cases);
        }

        // materialize nullish sentinels directly at the union's representation
        if matches!(value, CoercionValue::Null | CoercionValue::Undefined) {
            return self.materialize_coercion_value(value, target);
        }

        // convert the singular source through its selected case first
        let (value, source) = match cases {
            [] => (value, source),
            [case] => {
                let value = self.lower_adjustments(value, source, &case.adjustments)?;

                (value, case.target)
            }
            _ => {
                return Err(CompilerError::Internal {
                    message: "several cases on a singular union injection".to_string(),
                });
            }
        };

        // inject the converted source at the union's shared reference representation
        let value = self.materialize_coercion_value(value, source)?;

        // reach the representation transparent newtypes wrap
        let mut stored = representation;
        while let mir::Type::Newtype { inner, .. } = self.builder.tree().get(stored) {
            stored = *inner;
        }
        if self
            .builder
            .tree()
            .get(stored)
            .is_reference_representation()
        {
            return self.adapt_to_representation(value, representation);
        }

        Ok(value)
    }

    /// Inject one complete source value into an indexed variant case.
    fn lower_variant_adjustment(
        &mut self,
        value: CoercionValue,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        representation: mir::LocalNodeId<mir::Type>,
        cases: &[dir::CoercionCase],
    ) -> CompilerResult<mir::Value> {
        // read the members the target union declares
        let target_members = self.union_members(target)?;

        // convert each runtime case of a union source
        if let dir::Type::Union(source_union) = self.lower.ty(source)? {
            let source_members = self
                .lower
                .types(source.module_id)?
                .type_ids(source_union.elements)
                .to_vec();
            let value = self.materialize_coercion_value(value, source)?;
            let value_type = self.value_representation(value)?;
            if matches!(
                self.builder.tree().get(value_type),
                mir::Type::Variant { .. }
            ) {
                return self.lower_variant_conversion(
                    source_members,
                    target_members,
                    value,
                    representation,
                    cases,
                );
            }

            // enter a single target case for shared source representations
            let Some(first) = cases.first() else {
                return Err(CompilerError::Internal {
                    message: "a union conversion without source cases".to_string(),
                });
            };
            if cases.iter().any(|case| case.target != first.target) {
                return Err(CompilerError::Internal {
                    message: "a union conversion requiring a missing source discriminant"
                        .to_string(),
                });
            }

            // build the shared target case around the converted payload
            let target_member = first.target;
            self.require_union_case(&target_members, first.index, target_member)?;
            let payload = self.union_payload(CoercionValue::Runtime(value), target_member)?;

            return Ok(self
                .builder
                .variant_new(representation, first.index, payload));
        }

        // enter the single declared case of a plain injection
        let [case] = cases else {
            return Err(CompilerError::Internal {
                message: "a union injection requiring exactly one source case".to_string(),
            });
        };
        let target_member = case.target;
        self.require_union_case(&target_members, case.index, target_member)?;

        // convert the source value before inserting its target case
        let value = self.lower_adjustments(value, source, &case.adjustments)?;
        let payload = self.union_payload(value, target_member)?;

        Ok(self
            .builder
            .variant_new(representation, case.index, payload))
    }

    /// Convert one indexed union value into another ordered case set.
    fn lower_variant_conversion(
        &mut self,
        source_members: Vec<dir::GlobalTypeId>,
        target_members: Vec<dir::GlobalTypeId>,
        value: mir::Value,
        representation: mir::LocalNodeId<mir::Type>,
        mappings: &[dir::CoercionCase],
    ) -> CompilerResult<mir::Value> {
        // require one mapping per source member
        if mappings.len() != source_members.len() {
            return Err(CompilerError::Internal {
                message: "a union conversion with an incomplete case map".to_string(),
            });
        }

        // dispatch once over the source discriminant
        let result = self
            .builder
            .local(representation, mir::Mutability::Immutable);
        let exit = self.builder.block();
        let mut cases = Vec::with_capacity(mappings.len());
        for index in 0..mappings.len() {
            cases.push((index as u32, self.builder.block()));
        }
        self.builder.variant_switch(value, None, cases.clone());

        // rebuild every source case under its target discriminant
        for (source_index, block) in cases {
            self.builder.switch_to_block(block);
            let source_member = source_members[source_index as usize];
            let mapping = &mappings[source_index as usize];
            let source_value = self.union_case_value(value, source_index, source_member)?;
            let source_value =
                self.lower_adjustments(source_value, source_member, &mapping.adjustments)?;
            let target_member = mapping.target;
            self.require_union_case(&target_members, mapping.index, target_member)?;
            let payload = self.union_payload(source_value, target_member)?;
            let converted = self
                .builder
                .variant_new(representation, mapping.index, payload);
            self.builder.local_set(result, converted);
            self.builder.jump(exit);
        }

        // resume after the dispatch with the rebuilt value
        self.builder.switch_to_block(exit);

        Ok(self.builder.local_get(result))
    }

    /// Convert one indexed union value into a non-union representation.
    fn lower_union_exit(
        &mut self,
        value: CoercionValue,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        representation: mir::LocalNodeId<mir::Type>,
        cases: &[dir::CoercionCase],
    ) -> CompilerResult<mir::Value> {
        // require a union source
        let dir::Type::Union(source_union) = self.lower.ty(source)? else {
            return Err(CompilerError::Internal {
                message: "a union exit with a non-union source".to_string(),
            });
        };

        // require one case per source member on the same target
        let source_members = self
            .lower
            .types(source.module_id)?
            .type_ids(source_union.elements)
            .to_vec();
        if cases.len() != source_members.len() {
            return Err(CompilerError::Internal {
                message: "a union exit with an incomplete case map".to_string(),
            });
        }
        if cases.iter().any(|case| case.target != target) {
            return Err(CompilerError::Internal {
                message: "a union exit selecting a different target type".to_string(),
            });
        }

        // materialize the source at its own representation before converting
        let value = self.materialize_coercion_value(value, source)?;
        let value_type = self.value_representation(value)?;

        // dispatch indexed representations and convert each payload independently
        if matches!(
            self.builder.tree().get(value_type),
            mir::Type::Variant { .. }
        ) {
            // dispatch once over the source discriminant
            let result = self
                .builder
                .local(representation, mir::Mutability::Immutable);
            let exit = self.builder.block();
            let mut blocks = Vec::with_capacity(cases.len());
            for index in 0..cases.len() {
                blocks.push((index as u32, self.builder.block()));
            }
            self.builder.variant_switch(value, None, blocks.clone());

            // convert each source case into the shared target representation
            for (index, block) in blocks {
                self.builder.switch_to_block(block);
                let member = source_members[index as usize];
                let member_value = self.union_case_value(value, index, member)?;
                let member_value = self.lower_adjustments(
                    member_value,
                    member,
                    &cases[index as usize].adjustments,
                )?;
                let member_value = self.materialize_coercion_value(member_value, target)?;
                self.builder.local_set(result, member_value);
                self.builder.jump(exit);
            }

            // resume after the dispatch with the converted value
            self.builder.switch_to_block(exit);

            return Ok(self.builder.local_get(result));
        }

        // require one shared conversion for shared representations
        let Some(first) = cases.first() else {
            return Err(CompilerError::Internal {
                message: "a union exit without source cases".to_string(),
            });
        };
        if cases
            .iter()
            .any(|case| case.adjustments != first.adjustments)
        {
            return Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: "a per-arm conversion over an undiscriminated union representation"
                    .to_string(),
            }
            .into());
        }

        // convert the source through the shared adjustments
        let value = self.lower_adjustments(
            CoercionValue::Runtime(value),
            source_members[0],
            &first.adjustments,
        )?;

        self.materialize_coercion_value(value, target)
    }

    /// Return the logical members of one union type behind forms and aliases.
    pub(in crate::lower) fn union_members(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        // resolve the union standing behind the name
        let stored = self.union_stored(ty)?;

        // require the resolved type to be a union
        let dir::Type::Union(union) = self.lower.ty(stored)? else {
            return Err(CompilerError::Internal {
                message: "a non-union type in a union member read".to_string(),
            });
        };

        Ok(self
            .lower
            .types(stored.module_id)?
            .type_ids(union.elements)
            .to_vec())
    }

    /// Resolve the stored type one union name denotes, unfolding transparent aliases.
    fn union_stored(&self, ty: dir::GlobalTypeId) -> CompilerResult<dir::GlobalTypeId> {
        // resolve the union behind owned forms and transparent aliases
        let mut stored = self.lower.peel_owned(ty)?;
        while let dir::Type::Application(instance) = self.lower.ty(stored)? {
            let defined = match self.lower.definition(instance.symbol)? {
                Some(dir::Definition::TypeAlias(alias)) => alias.value,
                _ => break,
            };

            stored = self.lower.peel_owned(defined)?;
        }

        Ok(stored)
    }

    /// Return one indexed source case as a coercion value.
    fn union_case_value(
        &mut self,
        value: mir::Value,
        index: u32,
        member: dir::GlobalTypeId,
    ) -> CompilerResult<CoercionValue> {
        // keep members whose value lives in their type unmaterialized
        Ok(match self.lower.ty(member)? {
            dir::Type::Literal(literal) => CoercionValue::Literal(literal),
            dir::Type::Null => CoercionValue::Null,
            dir::Type::Undefined => CoercionValue::Undefined,
            _ => CoercionValue::Runtime(self.builder.variant_payload(value, index)),
        })
    }

    /// Materialize one target union case payload when it has runtime storage.
    fn union_payload(
        &mut self,
        value: CoercionValue,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<mir::Value>> {
        // skip literal and singleton cases, which carry no runtime payload
        if matches!(
            self.lower.ty(target)?,
            dir::Type::Literal(_) | dir::Type::Null | dir::Type::Undefined
        ) {
            return Ok(None);
        }

        // materialize every other case into its storage
        let value = self.materialize_coercion_value(value, target)?;

        Ok(Some(value))
    }

    /// Materialize one coercion value at its target representation.
    pub(in crate::lower) fn materialize_coercion_value(
        &mut self,
        value: CoercionValue,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Value> {
        // materialize by the value's own form
        match value {
            CoercionValue::Expression(expression) => self.lower_expression_value(expression),
            CoercionValue::Runtime(value) => Ok(value),
            CoercionValue::Literal(literal) => {
                let target = self.lower_type(target)?;
                let target = self.builder.tree().get(target).clone();

                self.lower_constant(literal, target)
            }
            CoercionValue::Null => {
                let target = self.lower_type(target)?;

                Ok(self.builder.constant(mir::Constant::Null, target))
            }
            CoercionValue::Undefined => {
                let target = self.lower_type(target)?;

                Ok(self.builder.constant(mir::Constant::Undefined, target))
            }
        }
    }

    /// Require one selected union case to name its declared member at its index.
    fn require_union_case(
        &self,
        members: &[dir::GlobalTypeId],
        index: u32,
        member: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        if members.get(index as usize) != Some(&member) {
            return Err(CompilerError::Internal {
                message: "a union conversion selecting an absent target member".to_string(),
            });
        }

        Ok(())
    }
}
