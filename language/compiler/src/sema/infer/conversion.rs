use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{
    Answer, BodyState, BorrowConversion, CandidateOutcome, CauseId, Check, CheckFailure,
    CheckOutcome, CheckState, ConversionCheck, Expectation, FlowSite, Goal, InferMode, Origin,
    Relation, Value, ValueConversion, ValueUse, Verdict,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Record one implicit coercion selected for an authored value.
    pub(in crate::sema) fn commit_coercion(
        &mut self,
        node: dir::GlobalNodeIdAny,
        coercion: dir::Coercion,
    ) -> CompilerResult<()> {
        // read the coercion already selected at this node
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

        // bind the first selection
        self.module_mut(node.module_id)
            .coercions
            .bind_coercion(node, coercion);

        Ok(())
    }

    /// Return the family default form of one owned type, or `None` for every other type.
    pub(in crate::sema) fn family_default_of_owned(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let ty = self.shallow_resolve(ty)?;
        match self.ty(ty)? {
            dir::Type::Form(form) if form.form == dir::Form::Owned => Ok(Some(form.value)),
            _ => Ok(None),
        }
    }

    /// Return the value a readonly view observes, arm-wise through unions.
    fn readonly_view_payload(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // normalize before reading the head
        let value = self.normalize(origin, id)?;

        // observe union values arm-wise
        if let dir::Type::Union(union) = self.ty(value)? {
            let elements = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(
                self.type_ids(value.module_id, union.elements)?,
            );
            let mut observed = SmallVec::<[dir::GlobalTypeId; 4]>::new();
            let mut changed = false;
            for element in elements {
                let element_observed = self.readonly_view_payload(origin, element)?;
                changed |= element_observed.is_some();
                observed.push(element_observed.unwrap_or(element));
            }

            // rebuild the union once any arm observed through a view
            if changed {
                return self.normalized_union_type(observed).map(Some);
            }

            return Ok(None);
        }

        // read through a readonly view onto the value it views
        let chain = self.form_chain(origin, value)?;
        if chain.is_readonly() {
            let observed = match chain.ownership_form() {
                Some(form) => self.intern_type(dir::Type::Form(form))?,
                None => chain.base(),
            };

            return Ok(Some(observed));
        }

        Ok(None)
    }
}

impl BodyState<'_, '_> {
    /// Return whether one destination leaves its ownership to inference.
    ///
    /// An open variable or hole qualifies on its own; inside a union, an owned arm settles it.
    pub(in crate::sema) fn infers_ownership(
        &mut self,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let target = self.shallow_resolve(target)?;
        match self.ty(target)? {
            // an open variable or hole leaves ownership to inference
            dir::Type::Variable(_) | dir::Type::Hole(_) => Ok(true),
            // a union settles its ownership at the first owned arm
            dir::Type::Union(union) => {
                let members = self.type_ids(target.module_id, union.elements)?;
                let mut has_variable = false;
                for member in members {
                    let member = self.shallow_resolve(*member)?;
                    match self.ty(member)? {
                        dir::Type::Variable(_) | dir::Type::Hole(_) => has_variable = true,
                        dir::Type::Form(form) if form.form == dir::Form::Owned => {
                            return Ok(false);
                        }
                        _ => {}
                    }
                }

                Ok(has_variable)
            }
            // every other type carries its own ownership
            _ => Ok(false),
        }
    }

    /// Commit the access one borrowed value's reborrow requires of the place it lends from.
    fn commit_reborrow_access(
        &mut self,
        origin: Origin,
        source: Value,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        // require an authored source node to record the access against
        let Some(node) = source.node else {
            return Ok(());
        };

        // require a borrowed destination
        let target_chain = self.form_chain(origin, target)?;
        let Some(dir::FormType {
            form: dir::Form::Borrowed(borrow),
            ..
        }) = target_chain.ownership_form()
        else {
            return Ok(());
        };

        // record the access the destination borrow requires
        let access = self.type_borrow(target.module_id, borrow)?.access;
        if let Some(requested) = self.access_of(access)? {
            self.commit_required_access(node, requested, true);
        }

        Ok(())
    }

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
        // resolve both sides, keeping the written value and the variables they hold open
        let written = source;
        source.ty = self.shallow_resolve(source.ty)?;
        let mut target = self.shallow_resolve(target)?;
        let mut variables = self.type_variables(source.ty)?;
        variables.extend(self.type_variables(target)?);

        // bind open operands, widening a fresh literal to suit its destination
        let inferred = if variables.is_empty() {
            None
        } else {
            let mut source = self.literal_candidate(site.origin(), source, target, use_)?;
            let verdict = self.constrain_conversion(site, cause, relation, source, target, use_)?;

            // queue an undecided conversion for a re-check once its blockers solve
            if verdict == Verdict::Ambiguous {
                source.ty = self.shallow_resolve(source.ty)?;
                target = self.shallow_resolve(target)?;
                self.check.queue_check(Check::Conversion(ConversionCheck {
                    site,
                    source: written,
                    expectation: Expectation {
                        cause,
                        relation,
                        target,
                        use_,
                        mode,
                    },
                }))?;

                return Ok(ValueConversion {
                    source: source.ty,
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
                    self.check.queue_check(Check::Conversion(ConversionCheck {
                        site,
                        source: written,
                        expectation: Expectation {
                            cause,
                            relation,
                            target,
                            use_,
                            mode,
                        },
                    }))?;

                    return Ok(ValueConversion {
                        source: source.ty,
                        outcome: CheckOutcome::Holds,
                        target,
                        coercion: None,
                    });
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

            // stop at a failed outcome and at every check only relation
            if outcome != CheckOutcome::Holds
                || relation != Relation::Assignable
                || !use_.requires_runtime_coercion()
            {
                return Ok(ValueConversion {
                    source: source.ty,
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
                source: source.ty,
                outcome,
                target,
                coercion: self.widening_coercion(written.ty, source.ty, None),
            });
        }

        // materialize the closed runtime conversion
        let conversion = self.convert_closed_value(site, origin, cause, source, target, use_)?;

        // split the conversion into its outcome and coercion
        let (outcome, coercion) = match conversion {
            Ok(coercion) => (CheckOutcome::Holds, coercion),
            Err(failure) => (CheckOutcome::Fails(failure), None),
        };

        Ok(ValueConversion {
            source: source.ty,
            outcome,
            target,
            coercion: self.widening_coercion(written.ty, source.ty, coercion),
        })
    }

    /// Lead one conversion's coercion with the widening its fresh literal took.
    fn widening_coercion(
        &self,
        written: dir::GlobalTypeId,
        widened: dir::GlobalTypeId,
        coercion: Option<Box<dir::Coercion>>,
    ) -> Option<Box<dir::Coercion>> {
        if written == widened {
            return coercion;
        }

        // lead the adjustments with the widening
        let mut adjustments = vec![dir::CoercionAdjustment::Widen { target: widened }];
        if let Some(coercion) = coercion {
            adjustments.extend(coercion.adjustments);
        }

        Some(Box::new(dir::Coercion::new(
            written,
            adjustments,
            dir::CastOrigin::Implicit,
        )))
    }

    /// Constrain the logical types carried through one value conversion.
    pub(in crate::sema) fn constrain_conversion(
        &mut self,
        site: FlowSite,
        cause: CauseId,
        relation: Relation,
        mut source: Value,
        target: dir::GlobalTypeId,
        use_: ValueUse,
    ) -> CompilerResult<Verdict> {
        let origin = site.origin();

        // adopt the family default form for an owned temporary at an inferring destination
        if use_.requires_runtime_coercion()
            && source.place.is_none()
            && self.infers_ownership(target)?
            && let Some(adopted) = self.family_default_of_owned(source.ty)?
        {
            source.ty = adopted;
        }

        // explicit and implicit borrowing use the same value place
        if matches!(relation, Relation::Assignable | Relation::Castable)
            && use_.requires_runtime_coercion()
            && let Some(conversion) = self.borrow_conversion(origin, source.ty, target)?
        {
            return self.constrain_borrow(origin, cause, Relation::Assignable, source, &conversion);
        }

        // compare types alone at every check only relation
        if relation != Relation::Assignable || !use_.requires_runtime_coercion() {
            return self.constrain_type(origin, cause, relation, source.ty, target);
        }

        // classify source and target memory forms
        let source_chain = self.form_chain(origin, source.ty)?;
        let target_chain = self.form_chain(origin, target)?;

        // observe through a readonly value view, leaving its storage in place
        let may_observe = source_chain.is_readonly()
            || self.type_flags(source.ty)?.has_alias()
            || matches!(self.ty(source_chain.base())?, dir::Type::Union(_));
        if use_ == ValueUse::Operand
            && may_observe
            && let Some(observed) = self.readonly_view_payload(origin, source.ty)?
        {
            return self.constrain_type(origin, cause, relation, observed, target);
        }

        // read a directly owned Copy payload out of a borrow at an expectation site
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
                    .decide_auto_interface(origin, payload, dir::AutoInterface::Copy)?
                    .holds()
            {
                return self.constrain_type(origin, cause, relation, payload, target);
            }
        }

        // record the access a reborrow into a borrowed destination requires
        if source_borrowed && target_borrowed {
            self.commit_reborrow_access(origin, source, target)?;
        }

        // transfer an owned value into managed storage at a consuming position
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
        // require the borrow constructor the conversion target carries
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

        // bind an elided borrow region to the source place provenance
        let borrow = self.type_borrow(conversion.module, target_borrow)?;
        let provenance = self.intern_region(place.lifetime, place.placement)?;
        let lifetime = self.constrain_type(
            origin,
            cause,
            Relation::Assignable,
            provenance,
            borrow.region,
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

        // record the access this borrow requires from the lent place
        if let Some(node) = source.node
            && let Some(requested) = self.access_of(borrow.access)?
        {
            let is_aliased = self.type_is_aliased(origin, source.ty)?;
            self.commit_required_access(node, requested, is_aliased);
        }

        // lend the borrow itself on a handle acquisition, its payload on a reborrow
        let source_value = match (
            conversion.acquires_handle,
            conversion.source.ownership_form(),
        ) {
            (true, Some(form)) => self.intern_type(dir::Type::Form(form))?,
            (_, Some(form)) => form.value,
            (_, None) => conversion.source.base(),
        };

        // widen a literal source to the borrowed value before it lends
        let borrowed = self.normalize(origin, conversion.borrow.value)?;
        let borrowed_type = self.ty(borrowed)?;
        let source_value = match self.ty(source_value)? {
            dir::Type::Literal(literal) if literal.widens_to(&borrowed_type) => borrowed,
            _ => source_value,
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
            let readonly = self.access_literal(dir::Access::Readonly)?;
            match self.relate_access_assignable(origin, readonly, borrow.access)? {
                Verdict::Holds => {}
                Verdict::Fails => self.commit_access_use(node, dir::BindingUse::MUTATE),
                Verdict::Ambiguous => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "resolved borrow at {} has undecided access requirements",
                            self.node_label(node)
                        ),
                    });
                }
            }
        }

        Ok(verdict)
    }

    /// Convert one value whose inference variables have resolved.
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

        // resolve the written target at a stored position
        let target = match use_.requires_storage() {
            true => self.deeply_resolve(origin, target)?,
            false => target,
        };

        // skip adjustment for identical values, recording the reborrow access
        if self
            .decide_relation(origin, Relation::Equal, source.ty, target)?
            .holds()
        {
            self.commit_reborrow_access(origin, source, target)?;

            return Ok(Ok(None));
        }

        // skip adjustment for unreachable values
        let source_value = self.strip_form(origin, source.ty)?;
        if matches!(self.ty(source_value)?, dir::Type::Never) {
            return Ok(Ok(None));
        }

        // convert through the memory forms before reading the payload cases
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

                // record the arm each case enters, scalar targets holding a sole arm
                let index = match &targets {
                    Some(targets) => self.declared_arm_index(targets, target_case)?,
                    None => 0,
                };

                cases.push(dir::CoercionCase {
                    source: source_case.ty,
                    target: target_case,
                    index,
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
            let index = self.declared_arm_index(&targets, member)?;
            let case = dir::CoercionCase {
                source: source.ty,
                target: member,
                index,
                adjustments,
            };
            let coercion =
                dir::Coercion::union(source.ty, target, vec![case], dir::CastOrigin::Implicit);

            return Ok(Ok(Some(Box::new(coercion))));
        }

        self.convert_existing_value(site, origin, cause, source, target, use_)
    }

    /// Return one selected member's arm position in the declared target union.
    fn declared_arm_index(
        &self,
        targets: &[dir::GlobalTypeId],
        member: dir::GlobalTypeId,
    ) -> CompilerResult<u32> {
        let index = targets
            .iter()
            .position(|arm| *arm == member)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("union conversion selected the absent member {member:?}"),
            })?;

        Ok(index as u32)
    }

    /// Return the concrete cases represented by one source type.
    fn conversion_source_cases(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SmallVec<[dir::GlobalTypeId; 4]>>> {
        // take the declared arms of a union directly
        if let Some(cases) = self.union_arms(origin, source)? {
            return Ok(Some(cases));
        }

        // rigid parameters convert case by case over their resolved domain
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
        // reuse the arm a resolved source already selected for this union
        let mut arm_key = None;
        let mut operands = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        operands.push(source.ty);
        operands.extend_from_slice(targets);
        let subject = Goal::Arm {
            value_use: use_,
            is_placed: source.place.is_some(),
        };

        // look the selection up under its canonical goal
        if let Some((key, _)) = self
            .check
            .canonicalize_goal(origin, subject, &operands, false)?
        {
            // reuse the recorded answer, which selects by target position
            match self.check.answers.get(&key) {
                // reuse an exact case at its declared identity
                Some(Answer::Arm(Ok((arm, true)))) => {
                    return Ok(Ok((targets[*arm as usize], None)));
                }
                // reuse a converted case through its recorded arm
                Some(Answer::Arm(Ok((arm, false)))) => {
                    let target = targets[*arm as usize];

                    return self.confirm_union_case(site, origin, cause, source, target, use_);
                }
                // reuse the recorded failure
                Some(Answer::Arm(Err(failure))) => return Ok(Err(*failure)),
                // record the arm this selection settles on
                _ => arm_key = Some(key),
            }
        }

        // exact cases preserve their declared identity
        for (arm, target) in targets.iter().copied().enumerate() {
            if self
                .decide_relation(origin, Relation::Equal, source.ty, target)?
                .holds()
            {
                // answer the arm as an exact case
                if let Some(key) = arm_key {
                    self.check
                        .answers
                        .insert(key, Answer::Arm(Ok((arm as u16, true))));
                }

                return Ok(Ok((target, None)));
            }
        }

        // identify every represented case the source can enter
        let mut selected = None;
        for (arm, target) in targets.iter().copied().enumerate() {
            let verdict = self.probe_candidate(|state| {
                let conversion =
                    state.convert_closed_value(site, origin, cause, source, target, use_)?;
                let outcome = match conversion {
                    Ok(coercion) => CandidateOutcome::Accepted(coercion),
                    Err(_) => CandidateOutcome::Rejected(()),
                };

                Ok(outcome)
            })?;

            // keep the sole viable case, rejecting a second one as ambiguous
            if matches!(verdict, Verdict::Holds | Verdict::Ambiguous) {
                if selected.is_some() {
                    return Ok(Err(CheckFailure::AmbiguousUnionCoercion));
                }
                selected = Some((arm, target));
            }
        }

        // answer the arm as a failure when every case rejects the source
        let Some((arm, target)) = selected else {
            if let Some(key) = arm_key {
                self.check
                    .answers
                    .insert(key, Answer::Arm(Err(CheckFailure::Relation)));
            }

            return Ok(Err(CheckFailure::Relation));
        };

        // answer the arm as a converted case
        if let Some(key) = arm_key {
            self.check
                .answers
                .insert(key, Answer::Arm(Ok((arm as u16, false))));
        }

        self.confirm_union_case(site, origin, cause, source, target, use_)
    }

    /// Commit the sole viable conversion into one selected union case.
    fn confirm_union_case(
        &mut self,
        site: FlowSite,
        origin: Origin,
        cause: CauseId,
        source: Value,
        target: dir::GlobalTypeId,
        use_: ValueUse,
    ) -> CompilerResult<Result<(dir::GlobalTypeId, Option<Box<dir::Coercion>>), CheckFailure>> {
        let conversion = self.confirm_candidate(|state| {
            let conversion =
                state.convert_closed_value(site, origin, cause, source, target, use_)?;
            let outcome = match conversion {
                Ok(coercion) => CandidateOutcome::Accepted(coercion),
                Err(_) => CandidateOutcome::Rejected(()),
            };

            Ok(outcome)
        })?;

        // require the confirmation to reproduce the probed conversion
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
        // check the logical relation before adjusting the value
        let relation = Relation::Assignable;
        let verdict = self.constrain_conversion(site, cause, relation, source, target, use_)?;
        let outcome =
            self.complete_constraint_check(origin, relation, source.ty, target, verdict)?;
        if let CheckOutcome::Fails(failure) = outcome {
            return Ok(Err(failure));
        }

        // record the resolved result of a target reaching a deferred operation
        let resolved_target = self.shallow_resolve(target)?;
        let target_reaches_computation = match self.ty(resolved_target)? {
            dir::Type::Operation(_) => true,
            dir::Type::Application(instance) => self.is_computed_alias(instance.symbol)?,
            _ => false,
        };
        let recorded_target = if target_reaches_computation {
            self.normalize(origin, target)?
        } else {
            target
        };

        // skip adjustment for a dynamic read at exactly its declared constraint
        if let dir::Type::Dynamic(dynamic) = self.ty(self.shallow_resolve(source.ty)?)?
            && self
                .decide_relation(origin, Relation::Equal, dynamic.constraint, recorded_target)?
                .holds()
        {
            return Ok(Ok(None));
        }

        // borrow a reference to the complete source storage
        if self.borrow_conversion(origin, source.ty, target)?.is_some() {
            let adjustment = dir::CoercionAdjustment::Borrow {
                target: recorded_target,
            };
            let coercion =
                dir::Coercion::new(source.ty, vec![adjustment], dir::CastOrigin::Implicit);

            return Ok(Ok(Some(Box::new(coercion))));
        }

        // classify the memory form on each side
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
                is_fresh: false,
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

        // select the adjustment this conversion requires
        let adjustment = {
            let source_base = source_chain.base();
            let target_base = target_chain.base();
            let source_head = self.ty(source_base)?;
            let target_head = self.ty(target_base)?;

            // erased values box on entry and exit
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
            // owned values transfer into managed storage
            else if source_chain
                .ownership_form()
                .is_some_and(|form| form.form == dir::Form::Owned)
                && self.form_ownership(origin, &target_chain)? == Some(dir::Ownership::Managed)
            {
                Some(dir::CoercionAdjustment::Manage {
                    target: recorded_target,
                })
            }
            // an explicit ownership change selects a different runtime representation
            else if (source_chain.ownership_form().is_some()
                || target_chain.ownership_form().is_some())
                && self.form_ownership(origin, &source_chain)?
                    != self.form_ownership(origin, &target_chain)?
            {
                Some(dir::CoercionAdjustment::Representation {
                    target: recorded_target,
                })
            }
            // accept every remaining distinction, which preserves representation
            else {
                None
            }
        };
        // leave a representation preserving conversion unadjusted
        let Some(adjustment) = adjustment else {
            return Ok(Ok(None));
        };

        let coercion = dir::Coercion::new(source.ty, vec![adjustment], dir::CastOrigin::Implicit);

        Ok(Ok(Some(Box::new(coercion))))
    }
}
