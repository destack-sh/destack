use destack_core::FxIndexSet;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{
    BodyState, BorrowConversion, CandidateOutcome, CandidateVerdict, CauseId, Check, CheckFailure,
    CheckOutcome, CheckState, ConversionCheck, Expectation, FlowSite, InferMode, Origin, Relation,
    Value, ValueConversion, ValueUse, Verdict,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Record one implicit coercion selected for an authored value.
    pub(in crate::sema) fn commit_coercion(
        &mut self,
        node: dir::GlobalNodeIdAny,
        coercion: dir::Coercion,
    ) -> CompilerResult<()> {
        let previous = self.module(node.module_id).coercions.coercion(node);

        // accept repeated identical selections and reject conflicting conversions
        if let Some(previous) = previous {
            if previous == &coercion {
                return Ok(());
            }

            return Err(CompilerError::Internal {
                message: format!(
                    "check node {} selected conflicting coercions: previous = {previous:?}, new = {coercion:?}",
                    self.node_label(node),
                ),
            });
        }

        self.module_mut(node.module_id)
            .coercions
            .bind_coercion(node, coercion);

        Ok(())
    }
}

impl BodyState<'_, '_> {
    /// Convert one checked value to its expected type.
    pub(in crate::sema) fn convert_value(
        &mut self,
        site: FlowSite,
        cause: CauseId,
        relation: Relation,
        mut source: Value,
        target: dir::GlobalTypeId,
        use_: ValueUse,
        mode: InferMode,
    ) -> CompilerResult<ValueConversion> {
        // resolve both sides and collect the variables they still hold open
        source.ty = self.shallow_resolve(source.ty)?;
        let mut target = self.shallow_resolve(target)?;
        let mut variables = self.type_variables(source.ty)?;
        variables.extend(self.type_variables(target)?);

        // bind open operands from the candidate selected by the value's inference mode
        let inferred = if variables.is_empty() {
            None
        } else {
            let candidate = self.inference_candidate_type(source.ty, mode)?;
            let candidate = Value {
                ty: candidate,
                ..source
            };
            let verdict =
                self.constrain_conversion(site, cause, relation, candidate, target, use_)?;

            // re-check an undecided conversion in the queue once its blockers solve,
            //  keeping the candidate's own binding committed
            if verdict == Verdict::Ambiguous {
                source.ty = self.shallow_resolve(source.ty)?;
                target = self.shallow_resolve(target)?;
                self.check
                    .register_check(Check::Conversion(ConversionCheck {
                        site,
                        source,
                        expectation: Expectation {
                            cause,
                            relation,
                            target,
                            use_,
                            mode,
                        },
                    }));

                return Ok(ValueConversion {
                    outcome: CheckOutcome::Pending,
                    target,
                    coercion: None,
                });
            }
            let holds = verdict.holds();

            // finish open conversions once the enclosing inference closes
            if holds {
                source.ty = self.shallow_resolve(source.ty)?;
                target = self.shallow_resolve(target)?;
                variables = self.type_variables(source.ty)?;
                variables.extend(self.type_variables(target)?);
                if !variables.is_empty() {
                    self.check
                        .register_check(Check::Conversion(ConversionCheck {
                            site,
                            source,
                            expectation: Expectation {
                                cause,
                                relation,
                                target,
                                use_,
                                mode,
                            },
                        }));

                    return Ok(ValueConversion {
                        outcome: CheckOutcome::Holds,
                        target,
                        coercion: None,
                    });
                }

                // settle the conversion's own variables at the statement close
                if !variables.is_empty() {
                    self.resolve_variables(&variables)?;
                    source.ty = self.shallow_resolve(source.ty)?;
                    target = self.shallow_resolve(target)?;
                    variables = self.type_variables(source.ty)?;
                    variables.extend(self.type_variables(target)?);
                }

                // unsolvable conversions report and poison the source
                if !variables.is_empty() {
                    let mut reported = FxIndexSet::default();
                    for variable in variables {
                        let origin = self.infer.variable(variable)?.origin;
                        let origin = self.infer.origin(origin);
                        self.report_cannot_infer_type(origin, Some(variable), &mut reported)?;
                    }
                    let error = self.intern_type(dir::Type::Error)?;
                    source.ty = error;
                    target = error;
                }
            }

            Some(holds)
        };

        // erase inference-only operations before recording the checked conversion
        let origin = site.origin();
        let module = origin.module();
        target = self.erase_inference_barriers(module, target)?;

        // complete a relation that participated in inference
        if let Some(holds) = inferred {
            let outcome = self.complete_constraint_check(
                origin,
                relation,
                source.ty,
                target,
                Verdict::decided(holds),
            )?;
            if outcome != CheckOutcome::Holds
                || relation != Relation::Assignable
                || !use_.requires_runtime_coercion()
            {
                return Ok(ValueConversion {
                    outcome,
                    target,
                    coercion: None,
                });
            }
        }

        // skip runtime coercion for closed logical checks
        if inferred.is_none()
            && (relation != Relation::Assignable || !use_.requires_runtime_coercion())
        {
            let verdict = self.constrain_conversion(site, cause, relation, source, target, use_)?;
            let outcome =
                self.complete_constraint_check(origin, relation, source.ty, target, verdict)?;

            return Ok(ValueConversion {
                outcome,
                target,
                coercion: None,
            });
        }

        // materialize the closed runtime conversion
        let conversion = self.convert_closed_value(site, origin, cause, source, target, use_)?;

        let (outcome, coercion) = match conversion {
            Ok(coercion) => (CheckOutcome::Holds, coercion),
            Err(failure) => (CheckOutcome::Fails(failure), None),
        };

        Ok(ValueConversion {
            outcome,
            target,
            coercion,
        })
    }

    /// Constrain the logical types carried through one value conversion.
    pub(in crate::sema) fn constrain_conversion(
        &mut self,
        site: FlowSite,
        cause: CauseId,
        relation: Relation,
        source: Value,
        target: dir::GlobalTypeId,
        use_: ValueUse,
    ) -> CompilerResult<Verdict> {
        let origin = site.origin();

        // explicit and implicit borrowing use the same value place
        if matches!(relation, Relation::Assignable | Relation::Castable)
            && use_.requires_runtime_coercion()
            && let Some(conversion) = self.borrow_conversion(origin, source.ty, target)?
        {
            return self.constrain_borrow(origin, cause, Relation::Assignable, source, &conversion);
        }

        // compare types for every other check-only relation, without a value place
        if relation != Relation::Assignable || !use_.requires_runtime_coercion() {
            return self.constrain_type(origin, cause, relation, source.ty, target);
        }

        // classify source and target memory forms
        let source_chain = self.form_chain(origin, source.ty)?;
        let target_chain = self.form_chain(origin, target)?;

        // builtin observation sees through readonly value views without copying storage
        if use_ == ValueUse::Operand
            && source_chain.is_readonly()
            && source_chain.ownership_form().is_none()
        {
            return self.constrain_type(origin, cause, relation, source_chain.base(), target);
        }

        // expectation sites read directly owned Copy payloads out of borrows
        let source_borrowed = source_chain
            .ownership_form()
            .is_some_and(|form| matches!(form.form, dir::Form::Borrowed(_)));
        let target_borrowed = target_chain
            .ownership_form()
            .is_some_and(|form| matches!(form.form, dir::Form::Borrowed(_)));
        if source_borrowed && !target_borrowed {
            let payload = source_chain.base();
            if !self.type_is_aliased(origin, payload)?
                && self
                    .satisfies_auto_interface(origin, payload, dir::AutoInterface::Copy)?
                    .holds()
            {
                return self.constrain_type(origin, cause, relation, payload, target);
            }
        }

        // consuming positions transfer owned values into managed storage
        let is_owned = source_chain
            .ownership_form()
            .is_some_and(|form| form.form == dir::Form::Owned);
        let target_ownership = self.form_ownership(origin, &target_chain)?;
        if is_owned && target_ownership == Some(dir::Ownership::Managed) {
            let source_value = source_chain
                .ownership_form()
                .map_or(source_chain.base(), |form| form.value);
            let target_value = target_chain
                .ownership_form()
                .map_or(target_chain.base(), |form| form.value);

            return self.constrain_type(
                site.origin(),
                cause,
                Relation::Equal,
                source_value,
                target_value,
            );
        }

        self.constrain_type(origin, cause, relation, source.ty, target)
    }

    /// Constrain one borrow from a checked value place.
    pub(in crate::sema) fn constrain_borrow(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: Value,
        conversion: &BorrowConversion,
    ) -> CompilerResult<Verdict> {
        let place = self.value_place(origin, source)?;
        let dir::Form::Borrowed(target_borrow) = conversion.borrow.form else {
            return Err(CompilerError::Internal {
                message: "borrow conversion has no borrow constructor".into(),
            });
        };

        // require the selected target placement from the source storage
        let mut verdict = Verdict::Holds;
        if let Some(target_place) = conversion.target.place() {
            let placement = self.constrain_type(
                origin,
                cause,
                Relation::Equal,
                place.placement,
                target_place,
            )?;
            if placement == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
            verdict = verdict.and(placement);
        }

        // bind an elided borrow lifetime to the source place provenance
        let borrow = self.type_borrow(conversion.module, target_borrow)?;
        let lifetime = self.constrain_type(
            origin,
            cause,
            Relation::Assignable,
            place.lifetime,
            borrow.lifetime,
        )?;
        if lifetime == Verdict::Fails {
            return Ok(Verdict::Fails);
        }
        verdict = verdict.and(lifetime);

        // constrain the borrow through the source value place
        let access = self.constrain_access_assignable(origin, place.access, borrow.access)?;
        if access == Verdict::Fails {
            return Ok(Verdict::Fails);
        }
        verdict = verdict.and(access);

        // lend the borrow itself on a handle acquisition, its payload on a reborrow
        let source_value = match (
            conversion.acquires_handle,
            conversion.source.ownership_form(),
        ) {
            (true, Some(form)) => self.intern_type(dir::Type::Form(form))?,
            (_, Some(form)) => form.value,
            (_, None) => conversion.source.base(),
        };

        let payload = self.constrain_form_value(
            origin,
            cause,
            relation,
            conversion.module,
            conversion.borrow.form,
            source_value,
            conversion.borrow.value,
        )?;
        let verdict = verdict.and(payload);

        // record borrows that require mutable access to directly stored binding values
        if verdict == Verdict::Holds
            && !self.type_is_aliased(origin, source.ty)?
            && let Some(node) = source.node
        {
            let readonly = self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Access(
                dir::Access::Readonly,
            )))?;
            match self.relate_access_assignable(origin, readonly, borrow.access)? {
                Verdict::Holds => {}
                Verdict::Fails => self.record_access_use(node, dir::BindingUse::MUTABLE),
                Verdict::Ambiguous => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "settled borrow at {} has undecided access requirements",
                            self.node_label(node)
                        ),
                    });
                }
            }
        }

        Ok(verdict)
    }

    /// Convert one value whose inference variables have settled.
    fn convert_closed_value(
        &mut self,
        site: FlowSite,
        origin: Origin,
        cause: CauseId,
        source: Value,
        target: dir::GlobalTypeId,
        use_: ValueUse,
    ) -> CompilerResult<Result<Option<Box<dir::Coercion>>, CheckFailure>> {
        // shed redundant forms from both sides before converting
        let source_type = self.shallow_resolve(source.ty)?;
        let source_type = self.reduce_redundant_forms(origin, source_type)?;
        let source = Value {
            ty: source_type,
            ..source
        };
        let target = self.shallow_resolve(target)?;
        let target = self.reduce_redundant_forms(origin, target)?;

        // widen a scalar singleton first, then convert the widened runtime value
        let source_value = self.strip_form(origin, source.ty)?;
        if let dir::Type::Literal(literal) = self.ty(source_value)? {
            let base = self.strip_form(origin, target)?;
            let base_head = self.ty(base)?;
            let is_runtime = matches!(base_head, dir::Type::Primitive(_) | dir::Type::Range(_));
            if is_runtime && literal.widens_to(&base_head) {
                let widened = Value { ty: base, ..source };
                let rest = self.convert_closed_value(site, origin, cause, widened, target, use_)?;
                let widen = dir::CoercionAdjustment::Widen { target: base };

                return Ok(match rest {
                    Ok(Some(coercion)) => {
                        let mut adjustments = vec![widen];
                        adjustments.extend(coercion.adjustments);

                        Ok(Some(Box::new(dir::Coercion::new(
                            source.ty,
                            adjustments,
                            dir::CastOrigin::Implicit,
                        ))))
                    }
                    Ok(None) => Ok(Some(Box::new(dir::Coercion::new(
                        source.ty,
                        vec![widen],
                        dir::CastOrigin::Implicit,
                    )))),
                    Err(failure) => Err(failure),
                });
            }
        }

        // instantiate a generic callable reference first, then convert the instantiated value
        let required = self.strip_form(origin, target)?;
        if let Some(instantiation) = self.instantiate_signature(origin, source_value, required)?
            && let Some(arguments) = instantiation.arguments
            && !arguments.is_empty()
        {
            let selected = Value {
                ty: instantiation.signature,
                ..source
            };
            let rest = self.convert_closed_value(site, origin, cause, selected, target, use_)?;
            let instantiate = dir::CoercionAdjustment::Instantiate {
                target: instantiation.signature,
                arguments,
            };

            return Ok(match rest {
                Ok(Some(coercion)) => {
                    let mut adjustments = vec![instantiate];
                    adjustments.extend(coercion.adjustments);

                    Ok(Some(Box::new(dir::Coercion::new(
                        source.ty,
                        adjustments,
                        dir::CastOrigin::Implicit,
                    ))))
                }
                Ok(None) => Ok(Some(Box::new(dir::Coercion::new(
                    source.ty,
                    vec![instantiate],
                    dir::CastOrigin::Implicit,
                )))),
                Err(failure) => Err(failure),
            });
        }

        // stored positions convert into the settled written target
        let target = match use_.requires_storage() {
            true => self.deeply_resolve(origin, target)?,
            false => target,
        };

        // skip adjustment for identical and unreachable values
        if self
            .evaluate_relation(origin, Relation::Equal, source.ty, target)?
            .holds()
        {
            return Ok(Ok(None));
        }
        let source_value = self.strip_form(origin, source.ty)?;
        if matches!(self.ty(source_value)?, dir::Type::Never) {
            return Ok(Ok(None));
        }

        // apply memory carrier conversions before inspecting their payload cases
        let source_chain = self.form_chain(origin, source.ty)?;
        let target_chain = self.form_chain(origin, target)?;
        let borrows = self.borrow_conversion(origin, source.ty, target)?.is_some();
        let reads = source_chain
            .ownership_form()
            .is_some_and(|form| matches!(form.form, dir::Form::Borrowed(_)))
            && !target_chain
                .ownership_form()
                .is_some_and(|form| matches!(form.form, dir::Form::Borrowed(_)));
        if borrows || reads {
            return self.convert_existing_value(site, origin, cause, source, target, use_);
        }

        // map every concrete source case through the target conversion
        if let Some(sources) = self.conversion_source_cases(origin, source.ty)? {
            let targets = self.union_arms(origin, target)?;
            let mut cases = Vec::with_capacity(sources.len());
            for source_case in sources {
                let source_case = Value {
                    ty: source_case,
                    ..source
                };
                let (target_case, conversion) = match &targets {
                    Some(targets) => {
                        let (target, conversion) = match self.convert_union_case(
                            site,
                            origin,
                            cause,
                            source_case,
                            targets,
                            use_,
                        )? {
                            Ok(selection) => selection,
                            Err(failure) => return Ok(Err(failure)),
                        };

                        (target, conversion)
                    }
                    None => {
                        let conversion = self.convert_closed_value(
                            site,
                            origin,
                            cause,
                            source_case,
                            target,
                            use_,
                        )?;
                        let conversion = match conversion {
                            Ok(conversion) => conversion,
                            Err(failure) => return Ok(Err(failure)),
                        };

                        (target, conversion)
                    }
                };
                let adjustments = conversion
                    .map(|coercion| coercion.adjustments)
                    .unwrap_or_default();
                cases.push(dir::CoercionCase {
                    source: source_case.ty,
                    target: target_case,
                    adjustments,
                });
            }
            let coercion =
                dir::Coercion::union(source.ty, target, cases, dir::CastOrigin::Implicit);

            return Ok(Ok(Some(Box::new(coercion))));
        }

        // inject one singular source through a stuck aliased union head
        if matches!(self.ty(target)?, dir::Type::Application(_))
            && self.union_arms(origin, target)?.is_none()
        {
            let head = self.structurally_normalize(origin, target)?;
            if head != target && self.union_arms(origin, head)?.is_some() {
                return self.convert_closed_value(site, origin, cause, source, head, use_);
            }
        }

        // inject one singular source into its selected target union case
        if let Some(targets) = self.union_arms(origin, target)? {
            let (member, conversion) =
                match self.convert_union_case(site, origin, cause, source, &targets, use_)? {
                    Ok(selection) => selection,
                    Err(failure) => return Ok(Err(failure)),
                };
            let adjustments = conversion
                .map(|coercion| coercion.adjustments)
                .unwrap_or_default();
            let case = dir::CoercionCase {
                source: source.ty,
                target: member,
                adjustments,
            };
            let coercion =
                dir::Coercion::union(source.ty, target, vec![case], dir::CastOrigin::Implicit);

            return Ok(Ok(Some(Box::new(coercion))));
        }

        self.convert_existing_value(site, origin, cause, source, target, use_)
    }

    /// Return the concrete cases represented by one source type.
    fn conversion_source_cases(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SmallVec<[dir::GlobalTypeId; 4]>>> {
        if let Some(cases) = self.union_arms(origin, source)? {
            return Ok(Some(cases));
        }

        // rigid parameters convert case by case over their settled domain
        let source_value = self.strip_form(origin, source)?;
        let dir::Type::Parameter(parameter) = self.ty(source_value)? else {
            return Ok(None);
        };
        let Some(domain) = self.parameter_domain(origin, parameter)? else {
            return Ok(None);
        };
        let cases = match self.union_arms(origin, domain)? {
            Some(cases) => cases,
            None => SmallVec::from_slice(&[domain]),
        };

        Ok(Some(cases))
    }

    /// Select and convert one source into a declared target union case.
    fn convert_union_case(
        &mut self,
        site: FlowSite,
        origin: Origin,
        cause: CauseId,
        source: Value,
        targets: &[dir::GlobalTypeId],
        use_: ValueUse,
    ) -> CompilerResult<Result<(dir::GlobalTypeId, Option<Box<dir::Coercion>>), CheckFailure>> {
        // exact cases preserve their declared identity
        for target in targets.iter().copied() {
            if self
                .evaluate_relation(origin, Relation::Equal, source.ty, target)?
                .holds()
            {
                return Ok(Ok((target, None)));
            }
        }

        // identify every represented case the source can enter
        let mut selected = None;
        for target in targets.iter().copied() {
            let verdict = self.probe_candidate(|state| {
                let conversion =
                    state.convert_closed_value(site, origin, cause, source, target, use_)?;
                let outcome = match conversion {
                    Ok(coercion) => CandidateOutcome::Accepted(coercion),
                    Err(_) => CandidateOutcome::Rejected(()),
                };

                Ok(outcome)
            })?;
            if matches!(
                verdict,
                CandidateVerdict::Viable | CandidateVerdict::Indeterminate
            ) {
                if selected.is_some() {
                    return Ok(Err(CheckFailure::AmbiguousUnionCoercion));
                }
                selected = Some(target);
            }
        }
        let Some(target) = selected else {
            return Ok(Err(CheckFailure::Relation));
        };

        // commit the sole viable conversion
        let conversion = self.confirm_candidate(|state| {
            let conversion =
                state.convert_closed_value(site, origin, cause, source, target, use_)?;
            let outcome = match conversion {
                Ok(coercion) => CandidateOutcome::Accepted(coercion),
                Err(_) => CandidateOutcome::Rejected(()),
            };

            Ok(outcome)
        })?;
        let Some(conversion) = conversion else {
            return Err(CompilerError::Internal {
                message: "selected union conversion failed during confirmation".into(),
            });
        };

        Ok(Ok((target, conversion)))
    }

    /// Convert one existing value after checking its logical relation.
    fn convert_existing_value(
        &mut self,
        site: FlowSite,
        origin: Origin,
        cause: CauseId,
        source: Value,
        target: dir::GlobalTypeId,
        use_: ValueUse,
    ) -> CompilerResult<Result<Option<Box<dir::Coercion>>, CheckFailure>> {
        let relation = Relation::Assignable;
        let verdict = self.constrain_conversion(site, cause, relation, source, target, use_)?;
        let outcome =
            self.complete_constraint_check(origin, relation, source.ty, target, verdict)?;
        if let CheckOutcome::Fails(failure) = outcome {
            return Ok(Err(failure));
        }

        // record the settled result of a target reaching a deferred operation
        let resolved_target = self.shallow_resolve(target)?;
        let target_reaches_computation = match self.ty(resolved_target)? {
            dir::Type::Operation(_) => true,
            dir::Type::Application(instance) => self.alias_computes(instance.symbol)?,
            _ => false,
        };
        let recorded_target = if target_reaches_computation {
            self.normalize(origin, target)?
        } else {
            target
        };

        // record no adjustment for a dynamic read at exactly its declared constraint
        if let dir::Type::Dynamic(dynamic) = self.ty(self.shallow_resolve(source.ty)?)?
            && self
                .evaluate_relation(origin, Relation::Equal, dynamic.constraint, recorded_target)?
                .holds()
        {
            return Ok(Ok(None));
        }

        // borrowing creates a reference to the complete source storage
        if self.borrow_conversion(origin, source.ty, target)?.is_some() {
            let adjustment = dir::CoercionAdjustment::Borrow {
                target: recorded_target,
            };
            let coercion =
                dir::Coercion::new(source.ty, vec![adjustment], dir::CastOrigin::Implicit);

            return Ok(Ok(Some(Box::new(coercion))));
        }

        // classify the memory carrier on each side
        let source_chain = self.form_chain(origin, source.ty)?;
        let target_chain = self.form_chain(origin, target)?;
        let source_borrowed = source_chain
            .ownership_form()
            .is_some_and(|form| matches!(form.form, dir::Form::Borrowed(_)));
        let target_borrowed = target_chain
            .ownership_form()
            .is_some_and(|form| matches!(form.form, dir::Form::Borrowed(_)));

        // read borrowed values before converting their copied payload
        if source_borrowed && !target_borrowed {
            let payload = source_chain.base();
            let payload_value = Value {
                ty: payload,
                node: None,
                place: None,
            };
            let conversion =
                self.convert_closed_value(site, origin, cause, payload_value, target, use_)?;
            let conversion = match conversion {
                Ok(conversion) => conversion,
                Err(failure) => return Ok(Err(failure)),
            };
            let mut adjustments = Vec::with_capacity(
                conversion
                    .as_ref()
                    .map_or(1, |conversion| conversion.adjustments.len() + 1),
            );
            adjustments.push(dir::CoercionAdjustment::Read { target: payload });
            if let Some(conversion) = conversion {
                adjustments.extend(conversion.adjustments);
            }
            let coercion = dir::Coercion::new(source.ty, adjustments, dir::CastOrigin::Implicit);

            return Ok(Ok(Some(Box::new(coercion))));
        }

        let adjustment = {
            let source_base = source_chain.base();
            let target_base = target_chain.base();
            let source_head = self.ty(source_base)?;
            let target_head = self.ty(target_base)?;

            // erased carriers box their values on entry and exit
            if self.is_erased_value(source_base)? || self.is_erased_value(target_base)? {
                Some(dir::CoercionAdjustment::Erase {
                    target: recorded_target,
                })
            }
            // base types select their explicit value operation
            else if let Some(adjustment) =
                dir::CoercionAdjustment::classify(&source_head, &target_head, recorded_target)
            {
                Some(adjustment)
            }
            // explicit ownership changes select a different runtime carrier
            else if (source_chain.ownership_form().is_some()
                || target_chain.ownership_form().is_some())
                && self.form_ownership(origin, &source_chain)?
                    != self.form_ownership(origin, &target_chain)?
            {
                Some(dir::CoercionAdjustment::Carrier {
                    target: recorded_target,
                })
            }
            // accept every remaining distinction, which preserves representation
            else {
                None
            }
        };
        let Some(adjustment) = adjustment else {
            return Ok(Ok(None));
        };

        let coercion = dir::Coercion::new(source.ty, vec![adjustment], dir::CastOrigin::Implicit);

        Ok(Ok(Some(Box::new(coercion))))
    }
}
