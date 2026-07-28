use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, BodyState, CandidateOutcome, CauseId, CheckFailure, CheckOutcome, Dependency,
    InferMode, Origin, Relation, ValueConversion, ValueUse, VarianceContext, answer,
};

impl BodyState<'_, '_> {
    /// Convert one checked value to its expected type.
    pub(in crate::check) fn convert_value(
        &mut self,
        node: dir::GlobalNodeIdAny,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        use_: ValueUse,
        mode: InferMode,
    ) -> CompilerResult<Answer<ValueConversion>> {
        // non-runtime checks require only their logical type relation
        if relation != Relation::Assignable || !use_.requires_runtime_coercion() {
            let holds = answer!(self.constrain_conversion(cause, relation, source, target, use_,)?);
            let outcome = self.complete_constraint_check(cause, relation, source, target, holds)?;

            return Ok(Answer::Ready(match outcome {
                CheckOutcome::Holds => Ok(None),
                CheckOutcome::Fails(failure) => Err(failure),
            }));
        }

        // bind open operands from the source candidate selected by its expression mode
        let source = self.settled_root(source)?;
        let target = self.settled_root(target)?;
        let mut variables = self.type_variables(source)?;
        variables.extend(self.type_variables(target)?);
        if !variables.is_empty() {
            let candidate = self.inference_candidate_type(node, source, mode)?;
            let holds =
                answer!(self.constrain_conversion(cause, relation, candidate, target, use_,)?);
            let outcome = self.complete_constraint_check(cause, relation, source, target, holds)?;
            if let CheckOutcome::Fails(failure) = outcome {
                return Ok(Answer::Ready(Err(failure)));
            }

            let source = self.settled_root(source)?;
            let target = self.settled_root(target)?;
            let mut variables = self.type_variables(source)?;
            variables.extend(self.type_variables(target)?);
            if !variables.is_empty() {
                let blockers = variables.into_iter().map(Dependency::Variable);

                return Ok(Answer::pending(blockers));
            }
        }

        let conversion = answer!(self.convert_closed_value(
            self.cause_origin(cause),
            cause,
            source,
            target,
            use_,
        )?);

        Ok(Answer::Ready(conversion))
    }

    /// Constrain the logical types carried through one value conversion.
    pub(in crate::check) fn constrain_conversion(
        &mut self,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        use_: ValueUse,
    ) -> CompilerResult<Answer<bool>> {
        if relation != Relation::Assignable || !use_.requires_runtime_coercion() {
            return self.constrain_type(cause, relation, source, target);
        }

        // classify source and target memory forms
        let origin = self.cause_origin(cause);
        let source_chain = self.form_chain(origin, source)?;
        let target_chain = self.form_chain(origin, target)?;

        // builtin observation sees through readonly value views without copying storage
        if use_ == ValueUse::Operand
            && source_chain.is_readonly()
            && source_chain.ownership_form().is_none()
        {
            return self.constrain_type(cause, relation, source_chain.base(), target);
        }

        // consuming positions transfer owned values into managed storage
        let is_owned = source_chain
            .ownership_form()
            .is_some_and(|form| form.form == dir::Form::Owned);
        let target_ownership = answer!(self.form_ownership(origin, &target_chain)?);
        if is_owned && target_ownership == Some(dir::Ownership::Managed) {
            let source_value = source_chain
                .ownership_form()
                .map_or(source_chain.base(), |form| form.value);
            let target_value = target_chain
                .ownership_form()
                .map_or(target_chain.base(), |form| form.value);

            return self.relate_context_payload(
                cause,
                VarianceContext::Aliased,
                relation.payload_edge(),
                source_value,
                target_value,
            );
        }

        self.constrain_type(cause, relation, source, target)
    }

    /// Convert one value whose inference variables have settled.
    fn convert_closed_value(
        &mut self,
        origin: Origin,
        cause: CauseId,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        use_: ValueUse,
    ) -> CompilerResult<Answer<ValueConversion>> {
        let source = answer!(self.reduce_named_head(origin, source)?);
        let target = answer!(self.reduce_named_head(origin, target)?);
        let target = match use_.requires_storage() {
            true => {
                let target = answer!(self.reduce_type(origin, target)?);

                self.storage_type(origin, target)?
            }
            false => target,
        };

        // identical and unreachable values need no adjustment
        if answer!(self.decide_relation(origin, Relation::Equal, source, target)?) {
            return Ok(Answer::Ready(Ok(None)));
        }
        let source_value = answer!(self.strip_form(origin, source)?);
        if matches!(self.ty(source_value)?, dir::Type::Never) {
            return Ok(Answer::Ready(Ok(None)));
        }

        // map every concrete source case through the target conversion
        if let Some(sources) = answer!(self.conversion_source_cases(origin, source)?) {
            let targets = answer!(self.union_arms(origin, target)?);
            let mut cases = Vec::with_capacity(sources.len());
            for source_case in sources {
                let (target_case, conversion) = match &targets {
                    Some(targets) => {
                        let Some((target, conversion)) = answer!(self.convert_union_case(
                            origin,
                            cause,
                            source_case,
                            targets,
                            use_,
                        )?) else {
                            return Ok(Answer::Ready(Err(CheckFailure::Relation)));
                        };

                        (target, conversion)
                    }
                    None => {
                        let conversion = answer!(self.convert_closed_value(
                            origin,
                            cause,
                            source_case,
                            target,
                            use_,
                        )?);
                        let conversion = match conversion {
                            Ok(conversion) => conversion,
                            Err(failure) => return Ok(Answer::Ready(Err(failure))),
                        };

                        (target, conversion)
                    }
                };
                let adjustments = conversion
                    .map(|coercion| coercion.adjustments)
                    .unwrap_or_default();
                cases.push(dir::CoercionCase {
                    source: source_case,
                    target: target_case,
                    adjustments,
                });
            }
            let coercion = dir::Coercion::union(source, target, cases, dir::CastOrigin::Implicit);

            return Ok(Answer::Ready(Ok(Some(Box::new(coercion)))));
        }

        // inject one singular source into its selected target union case
        if let Some(targets) = answer!(self.union_arms(origin, target)?) {
            let Some((member, conversion)) =
                answer!(self.convert_union_case(origin, cause, source, &targets, use_,)?)
            else {
                return Ok(Answer::Ready(Err(CheckFailure::Relation)));
            };
            let adjustments = conversion
                .map(|coercion| coercion.adjustments)
                .unwrap_or_default();
            let case = dir::CoercionCase {
                source,
                target: member,
                adjustments,
            };
            let coercion =
                dir::Coercion::union(source, target, vec![case], dir::CastOrigin::Implicit);

            return Ok(Answer::Ready(Ok(Some(Box::new(coercion)))));
        }

        self.convert_existing_value(origin, cause, source, target, use_)
    }

    /// Return the concrete cases represented by one source type.
    fn conversion_source_cases(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<SmallVec<[dir::GlobalTypeId; 4]>>>> {
        if let Some(cases) = answer!(self.union_arms(origin, source)?) {
            return Ok(Answer::Ready(Some(cases)));
        }
        let source_value = answer!(self.strip_form(origin, source)?);
        let dir::Type::Parameter(parameter) = self.ty(source_value)? else {
            return Ok(Answer::Ready(None));
        };

        // a union bound enumerates the representations admitted by the parameter
        for bound in self.parameter_bounds(origin, parameter)? {
            let bound = answer!(self.reduce_type(origin, bound)?);
            if let Some(cases) = answer!(self.union_arms(origin, bound)?) {
                return Ok(Answer::Ready(Some(cases)));
            }
        }

        Ok(Answer::Ready(None))
    }

    /// Select and convert one source into a declared target union case.
    fn convert_union_case(
        &mut self,
        origin: Origin,
        cause: CauseId,
        source: dir::GlobalTypeId,
        targets: &[dir::GlobalTypeId],
        use_: ValueUse,
    ) -> CompilerResult<Answer<Option<(dir::GlobalTypeId, Option<Box<dir::Coercion>>)>>> {
        // exact cases preserve their declared identity
        for target in targets.iter().copied() {
            if answer!(self.decide_relation(origin, Relation::Equal, source, target)?) {
                return Ok(Answer::Ready(Some((target, None))));
            }
        }

        // confirm the first declared case whose conversion holds
        for target in targets.iter().copied() {
            let conversion = answer!(self.confirm_candidate(|state| {
                let conversion =
                    answer!(state.convert_closed_value(origin, cause, source, target, use_,)?);
                let outcome = match conversion {
                    Ok(coercion) => CandidateOutcome::Accepted(coercion),
                    Err(_) => CandidateOutcome::Rejected(()),
                };

                Ok(Answer::Ready(outcome))
            })?);
            if let Some(conversion) = conversion {
                return Ok(Answer::Ready(Some((target, conversion))));
            }
        }

        Ok(Answer::Ready(None))
    }

    /// Convert one existing value after checking its logical relation.
    fn convert_existing_value(
        &mut self,
        origin: Origin,
        cause: CauseId,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        use_: ValueUse,
    ) -> CompilerResult<Answer<ValueConversion>> {
        let relation = Relation::Assignable;
        let holds = answer!(self.constrain_conversion(cause, relation, source, target, use_,)?);
        let outcome = self.complete_constraint_check(cause, relation, source, target, holds)?;
        if let CheckOutcome::Fails(failure) = outcome {
            return Ok(Answer::Ready(Err(failure)));
        }

        // borrow acquisition creates a new reference to source storage
        let adjustment = if self.borrow_conversion(origin, source, target)?.is_some() {
            Some(dir::CoercionAdjustment::Borrow { target })
        } else {
            let source_chain = self.form_chain(origin, source)?;
            let target_chain = self.form_chain(origin, target)?;
            let source_base = source_chain.base();
            let target_base = target_chain.base();
            let source_head = self.ty(source_base)?;
            let target_head = self.ty(target_base)?;

            // base types select their explicit value operation
            if let Some(adjustment) =
                dir::CoercionAdjustment::classify(&source_head, &target_head, target)
            {
                Some(adjustment)
            }
            // borrowed copyable values are read from their referenced storage
            else if source_chain
                .ownership_form()
                .is_some_and(|form| matches!(form.form, dir::Form::Borrowed(_)))
                && !target_chain
                    .ownership_form()
                    .is_some_and(|form| matches!(form.form, dir::Form::Borrowed(_)))
            {
                Some(dir::CoercionAdjustment::Read { target })
            }
            // explicit ownership changes select a different runtime carrier
            else if (source_chain.ownership_form().is_some()
                || target_chain.ownership_form().is_some())
                && answer!(self.form_ownership(origin, &source_chain)?)
                    != answer!(self.form_ownership(origin, &target_chain)?)
            {
                Some(dir::CoercionAdjustment::Carrier { target })
            }
            // every remaining accepted distinction preserves representation
            else {
                None
            }
        };
        let Some(adjustment) = adjustment else {
            return Ok(Answer::Ready(Ok(None)));
        };

        let coercion = dir::Coercion::new(source, vec![adjustment], dir::CastOrigin::Implicit);

        Ok(Answer::Ready(Ok(Some(Box::new(coercion)))))
    }
}
