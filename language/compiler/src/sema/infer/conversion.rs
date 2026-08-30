use destack_dir as dir;
use dir::TypeFold;
use smallvec::SmallVec;

use crate::sema::{
    Answer, BorrowConversion, CandidateOutcome, CauseId, Check, CheckFailure, CheckOutcome,
    CheckState, ConversionCheck, Expectation, FlowSite, Goal, InferMode, Origin, Relation, Value,
    ValueConversion, ValueUse, Verdict,
};
use crate::{CompilerError, CompilerResult};

/// The representation edge one value takes into its slot.
enum ValueEdge {
    /// The slot is open, the value flows in by inclusion until it closes.
    Open,
    /// The value erases behind an existential or a dynamic constraint.
    Erase,
    /// The value enters one arm of the slot's union.
    Inject(SmallVec<[dir::GlobalTypeId; 8]>),
    /// A constant materializes as the slot's runtime scalar.
    Materialize(dir::GlobalTypeId),
    /// The value stores as it is, its forms relating.
    Store,
}

impl CheckState<'_> {
    /// Record one implicit coercion selected for an authored value.
    pub(in crate::sema) fn commit_coercion(
        &mut self,
        node: dir::GlobalNodeIdAny,
        coercion: dir::Coercion,
    ) -> CompilerResult<()> {
        // leave decisions without a record
        if self.infer.is_deciding() {
            return Ok(());
        }

        // read the coercion an earlier pass recorded at the node
        let previous = self
            .module(node.module_id)
            .coercions
            .coercion(node)
            .cloned();

        // accept repeated identical selections and reject conflicting conversions
        if let Some(mut previous) = previous {
            let mut selected = coercion.clone();
            selected.map_types(&mut |ty| self.fully_resolve(ty))?;
            previous.map_types(&mut |ty| self.fully_resolve(ty))?;
            if previous == selected {
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

impl CheckState<'_> {
    /// Return whether one union target offers a borrowed arm.
    fn has_borrowed_arm(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let target = self.normalize(origin, target)?;
        let dir::Type::Union(union) = self.ty(target)? else {
            return Ok(false);
        };
        let arms: SmallVec<[_; 4]> = self.type_ids(target.module_id, union.elements)?.into();
        for arm in arms {
            let borrowed = self
                .form_chain(origin, arm)?
                .ownership_form()
                .is_some_and(|form| matches!(form.form, dir::Form::Borrowed(_)));
            if borrowed {
                return Ok(true);
            }
        }

        Ok(false)
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

        // bind open operands through a store candidate widened for the slot
        let inferred = if variables.is_empty() {
            None
        } else {
            let mut source =
                self.store_candidate(site.origin(), source, target, mode.keeps_literals())?;
            let verdict = self.constrain_conversion(site, cause, relation, source, target, use_)?;

            // queue a conversion still open after constraining for once its variables solve
            source.ty = self.shallow_resolve(source.ty)?;
            target = self.shallow_resolve(target)?;
            variables = self.type_variables(source.ty)?;
            variables.extend(self.type_variables(target)?);
            if verdict == Verdict::Ambiguous || (verdict.holds() && !variables.is_empty()) {
                let expectation = Expectation {
                    cause,
                    relation,
                    target,
                    use_,
                    mode,
                };
                self.queue_pending_conversion(site, written, expectation)?;
                let outcome = match verdict {
                    Verdict::Ambiguous => CheckOutcome::Pending,
                    _ => CheckOutcome::Holds,
                };

                return Ok(ValueConversion {
                    source: source.ty,
                    outcome,
                    target,
                    coercion: None,
                });
            }

            Some(verdict.holds())
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
                || relation != Relation::Storable
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

        // skip runtime coercion for closed logical checks and explicit casts
        if inferred.is_none()
            && (relation != Relation::Storable
                || !use_.requires_runtime_coercion()
                || use_ == ValueUse::Cast)
        {
            let verdict = self.constrain_conversion(site, cause, relation, source, target, use_)?;
            let outcome =
                self.complete_constraint_check(origin, relation, source.ty, target, verdict)?;

            return Ok(ValueConversion {
                source: source.ty,
                outcome,
                target,
                coercion: None,
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
            coercion,
        })
    }

    /// Queue one open conversion for once its variables close.
    fn queue_pending_conversion(
        &mut self,
        site: FlowSite,
        written: Value,
        expectation: Expectation,
    ) -> CompilerResult<()> {
        self.queue_check(Check::Conversion(ConversionCheck {
            site,
            source: written,
            expectation,
        }))?;

        Ok(())
    }

    /// Relate one fresh object value into the object shape its context constructs.
    ///
    /// A union context decides by the sole arm the value fits.
    fn relate_fresh_shape(
        &mut self,
        origin: Origin,
        cause: CauseId,
        value: dir::GlobalTypeId,
        slot: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Verdict>> {
        let arms = match self.union_leaves(origin, slot)? {
            Some(arms) => arms,
            None => SmallVec::from_slice(&[slot]),
        };
        let mut shapes = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for arm in arms {
            if let Some(shape) = self.construction_value(origin, arm)?
                && matches!(self.ty(shape)?, dir::Type::Object(_))
            {
                shapes.push(shape);
            }
        }
        match shapes.as_slice() {
            [] => Ok(None),
            [shape] => Ok(Some(self.relate_shape(
                origin,
                cause,
                Relation::Storable,
                true,
                value,
                *shape,
            )?)),
            _ => {
                // keep the sole arm the value fits
                let mut viable = SmallVec::<[dir::GlobalTypeId; 4]>::new();
                for shape in shapes {
                    let verdict = self.decide_candidate(|state| {
                        match state
                            .relate_shape(origin, cause, Relation::Storable, true, value, shape)?
                            .holds()
                        {
                            true => Ok(CandidateOutcome::Accepted(())),
                            false => Ok(CandidateOutcome::Rejected(())),
                        }
                    })?;
                    if verdict != Verdict::Fails {
                        viable.push(shape);
                    }
                }
                match viable.as_slice() {
                    [shape] => Ok(Some(self.relate_shape(
                        origin,
                        cause,
                        Relation::Storable,
                        true,
                        value,
                        *shape,
                    )?)),
                    [] => Ok(Some(Verdict::Fails)),
                    _ => Ok(Some(Verdict::Holds)),
                }
            }
        }
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

        // relate an explicit cast through the cast table, else spell the implicit conversion
        if use_ == ValueUse::Cast {
            let cast = self.relate_castable(origin, cause, source.ty, target)?;
            if cast != Verdict::Fails {
                return Ok(cast);
            }

            return self.constrain_conversion(
                site,
                cause,
                Relation::Storable,
                source,
                target,
                ValueUse::Store,
            );
        }

        // transfer an owned temporary into its family default form outside owned storage
        if use_.requires_runtime_coercion()
            && source.place.is_none()
            && let Some(adopted) = self.family_default_of_owned(source.ty)?
            && !self
                .form_chain(origin, target)?
                .ownership_form()
                .is_some_and(|form| form.form == dir::Form::Owned)
        {
            source.ty = adopted;
        }

        // flow a value into an open slot whole, converting once the slot closes
        if let (ValueEdge::Open, slot) = self.value_edge(origin, source.ty, target)? {
            return self.constrain_type(origin, cause, Relation::Subtype, source.ty, slot);
        }

        // match an open union against a union slot arm by arm
        let resolved = self.shallow_resolve(source.ty)?;
        if let dir::Type::Union(union) = self.ty(resolved)?
            && !self.type_variables(resolved)?.is_empty()
            && let slot = self.structurally_normalize(origin, target)?
            && let Some(slot_arms) = self.union_arms(origin, slot)?
        {
            let arms: SmallVec<[_; 8]> = self.type_ids(resolved.module_id, union.elements)?.into();
            let shared: SmallVec<[_; 8]> = arms
                .iter()
                .copied()
                .filter(|arm| slot_arms.contains(arm))
                .collect();
            let left: SmallVec<[_; 8]> = arms
                .iter()
                .copied()
                .filter(|arm| !shared.contains(arm))
                .collect();
            let right: SmallVec<[_; 8]> = slot_arms
                .iter()
                .copied()
                .filter(|arm| !shared.contains(arm))
                .collect();
            if let ([arm], [slot_arm]) = (left.as_slice(), right.as_slice()) {
                let arm = Value { ty: *arm, ..source };
                return self.constrain_conversion(site, cause, relation, arm, *slot_arm, use_);
            }
        }

        // convert each case of a union or deferred conditional value into the closed slot
        let chain = self.form_chain(origin, target)?;
        let slot = chain.base();
        let borrows_whole = self.borrows_whole(origin, source.ty, target)?;
        if !borrows_whole
            && self.root_variable(slot)?.is_none()
            && let Some(arms) = self.conversion_source_cases(origin, source.ty, target)?
        {
            let mut verdict = Verdict::Holds;
            for arm in arms {
                let arm = Value { ty: arm, ..source };
                verdict = verdict
                    .and(self.constrain_conversion(site, cause, relation, arm, target, use_)?);
                if verdict == Verdict::Fails {
                    break;
                }
            }

            return Ok(verdict);
        }

        // store a fresh temporary beneath the forms its destination declares
        if source.is_fresh && source.place.is_none() && relation == Relation::Storable {
            let slot = self.strip_forms(target)?;
            let value = self.shallow_resolve(source.ty)?;
            if matches!(self.ty(value)?, dir::Type::Object(_))
                && let Some(verdict) = self.relate_fresh_shape(origin, cause, value, slot)?
            {
                return Ok(verdict);
            }

            return self.constrain_edge(site, cause, source, slot, use_);
        }

        // explicit and implicit borrowing use the same value place
        if relation == Relation::Storable
            && use_.requires_runtime_coercion()
            && let Some(conversion) = self.borrow_conversion(origin, source.ty, target)?
        {
            return self.constrain_borrow(origin, cause, Relation::Storable, source, &conversion);
        }

        // compare types alone at every check only relation
        if relation != Relation::Storable || !use_.requires_runtime_coercion() {
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
            let observed = Value {
                ty: observed,
                place: None,
                ..source
            };

            return self.constrain_edge(site, cause, observed, target, use_);
        }

        // read a directly owned Copy payload out of a borrow at an expectation site
        let source_borrowed = source_chain
            .ownership_form()
            .is_some_and(|form| matches!(form.form, dir::Form::Borrowed(_)));
        let target_borrowed = target_chain
            .ownership_form()
            .is_some_and(|form| matches!(form.form, dir::Form::Borrowed(_)))
            || self.has_borrowed_arm(origin, target)?;
        if source_borrowed && !target_borrowed {
            let payload = source_chain.base();
            if !self.type_is_aliased(origin, payload)?
                && self
                    .decide_auto_interface(origin, payload, dir::AutoInterface::Copy)?
                    .holds()
            {
                let payload = Value {
                    ty: payload,
                    place: None,
                    ..source
                };

                return self.constrain_edge(site, cause, payload, target, use_);
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

        self.constrain_edge(site, cause, source, target, use_)
    }

    /// Classify the edge one value takes into its slot, reading through readonly views.
    fn value_edge(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<(ValueEdge, dir::GlobalTypeId)> {
        // read the source and peel readonly views off the slot
        let source = self.shallow_resolve(source)?;
        let mut slot = self.structurally_normalize(origin, target)?;
        while let dir::Type::Form(form) = self.ty(slot)?
            && form.form == dir::Form::Readonly
        {
            slot = self.structurally_normalize(origin, form.value)?;
        }

        // classify the edge by the source and slot heads
        let edge = match (self.ty(source)?, self.ty(slot)?) {
            // flow into an open slot whole
            (_, dir::Type::Variable(_)) => ValueEdge::Open,
            // erase behind a dynamic slot
            (_, dir::Type::Unknown | dir::Type::Dynamic(_)) => ValueEdge::Erase,
            (_, dir::Type::Object(_)) if self.is_erased_value(slot)? => ValueEdge::Erase,
            // materialize a literal untagged in a union slot of its own scalar domain
            (dir::Type::Literal(literal), dir::Type::Union(union))
                if self.stores_untagged(literal, slot, union)? =>
            {
                ValueEdge::Materialize(self.widen_type(source)?)
            }
            // store a parameter as the union its domain already is
            (dir::Type::Parameter(parameter), dir::Type::Union(_))
                if let Some(domain) = self.parameter_domain(origin, parameter)?
                    && let domain = self.structurally_normalize(origin, domain)?
                    && self
                        .decide_relation(origin, Relation::Equal, domain, slot)?
                        .holds() =>
            {
                ValueEdge::Store
            }
            // inject into the arms a union slot declares
            (_, dir::Type::Union(union)) => ValueEdge::Inject(SmallVec::from_slice(
                self.type_ids(slot.module_id, union.elements)?,
            )),
            // materialize a constant into the primitive that holds it
            (dir::Type::Key(key), dir::Type::Primitive(primitive))
                if key.widens_to_primitive(primitive) =>
            {
                ValueEdge::Materialize(slot)
            }
            // materialize a union of constants into the primitive that holds them all
            (dir::Type::Union(union), dir::Type::Primitive(primitive))
                if self.union_materializes_into(source, union, primitive)? =>
            {
                ValueEdge::Materialize(slot)
            }
            // materialize a constant that widens into a runtime slot
            (dir::Type::Literal(_) | dir::Type::Range(_), _) => {
                let base = self.strip_forms(slot)?;
                let head = self.ty(base)?;
                let is_runtime = matches!(head, dir::Type::Primitive(_) | dir::Type::Range(_));
                let widens = match self.ty(source)? {
                    dir::Type::Literal(literal) => literal.widens_to(&head),
                    dir::Type::Range(range) => range.widens_to(&head),
                    _ => false,
                };
                match is_runtime && widens {
                    true => ValueEdge::Materialize(base),
                    false => ValueEdge::Store,
                }
            }
            // store every remaining value as it is
            _ => ValueEdge::Store,
        };

        Ok((edge, slot))
    }

    /// Return whether every arm of one union is a constant the primitive holds.
    fn union_materializes_into(
        &mut self,
        source: dir::GlobalTypeId,
        union: dir::UnionType,
        primitive: dir::PrimitiveType,
    ) -> CompilerResult<bool> {
        // require every arm to widen into the primitive
        let arms = SmallVec::<[_; 8]>::from_slice(self.type_ids(source.module_id, union.elements)?);
        for arm in arms {
            let arm = self.shallow_resolve(arm)?;
            let holds = match self.ty(arm)? {
                dir::Type::Literal(literal) => literal.widens_to_primitive(primitive),
                dir::Type::Key(key) => key.widens_to_primitive(primitive),
                _ => false,
            };
            if !holds {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Return whether one literal stores untagged in a union slot of its own scalar domain.
    pub(in crate::sema) fn stores_untagged(
        &mut self,
        literal: dir::Literal,
        id: dir::GlobalTypeId,
        union: dir::UnionType,
    ) -> CompilerResult<bool> {
        let domain = literal.scalar_domain();

        Ok(domain.is_some() && self.scalar_literal_union_domain(id, union)? == domain)
    }

    /// Return the one scalar domain every arm of a literal union belongs to.
    pub(in crate::sema) fn scalar_literal_union_domain(
        &mut self,
        id: dir::GlobalTypeId,
        union: dir::UnionType,
    ) -> CompilerResult<Option<dir::ScalarDomain>> {
        let arms = SmallVec::<[_; 8]>::from_slice(self.type_ids(id.module_id, union.elements)?);

        // require every arm to share one scalar domain
        let mut domain = None;
        for arm in arms {
            let arm = self.shallow_resolve(arm)?;
            let dir::Type::Literal(literal) = self.ty(arm)? else {
                return Ok(None);
            };
            let arm_domain = literal.scalar_domain();
            if arm_domain.is_none() || (domain.is_some() && domain != arm_domain) {
                return Ok(None);
            }
            domain = arm_domain;
        }

        Ok(domain)
    }

    /// Constrain one value into its slot along its edge.
    pub(in crate::sema) fn constrain_edge(
        &mut self,
        site: FlowSite,
        cause: CauseId,
        source: Value,
        target: dir::GlobalTypeId,
        use_: ValueUse,
    ) -> CompilerResult<Verdict> {
        // read the value and the edge it takes into the slot
        let origin = site.origin();
        let value = self.shallow_resolve(source.ty)?;
        let (edge, slot) = self.value_edge(origin, value, target)?;

        // constrain the value along its edge
        match edge {
            // flow a value into an open slot by inclusion
            ValueEdge::Open => self.constrain_type(origin, cause, Relation::Subtype, value, slot),
            // erase a value behind a dynamic or dictionary slot
            ValueEdge::Erase => match self.ty(slot)? {
                dir::Type::Dynamic(dynamic) => {
                    self.relate_dynamic_assignable(origin, cause, value, dynamic.constraint)
                }
                _ => {
                    if self.erased_index_signatures_admit(origin, cause, value, slot)?
                        == Verdict::Fails
                    {
                        return Ok(Verdict::Fails);
                    }

                    self.erasable_source(origin, value)
                }
            },
            // convert a value into the one union arm it stores in
            ValueEdge::Inject(arms) => {
                let viable = self.viable_union_arms(&arms, |state, arm| {
                    state.constrain_conversion(site, cause, Relation::Storable, source, arm, use_)
                })?;
                match viable {
                    Ok(arm) => self.constrain_conversion(
                        site,
                        cause,
                        Relation::Storable,
                        source,
                        arm,
                        use_,
                    ),
                    Err(verdict) => Ok(verdict),
                }
            }
            // require a materialized constant the slot's type includes
            ValueEdge::Materialize(_) => {
                self.constrain_type(origin, cause, Relation::Subtype, value, slot)
            }
            // store a value as it is
            ValueEdge::Store => {
                self.constrain_type(origin, cause, Relation::Storable, value, target)
            }
        }
    }

    /// Relate one value's members to the index signatures of the dictionary it erases into.
    fn erased_index_signatures_admit(
        &mut self,
        origin: Origin,
        cause: CauseId,
        value: dir::GlobalTypeId,
        slot: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // read the object shape the value erases into
        let slot = self.structurally_normalize(origin, slot)?;
        let dir::Type::Object(shape) = self.ty(slot)? else {
            return Ok(Verdict::Holds);
        };

        // relate the value to every index signature the dictionary declares
        let indexes = SmallVec::<[dir::TypeIndexSignature; 2]>::from_slice(
            self.object_index_signatures(slot.module_id, shape.index_signatures)?,
        );
        let mut verdict = Verdict::Holds;
        for index in indexes {
            verdict = verdict.and(self.relate_index_signature(
                origin,
                cause,
                Relation::Subtype,
                value,
                &index,
            )?);
            if verdict == Verdict::Fails {
                break;
            }
        }

        Ok(verdict)
    }

    /// Return whether a borrow target lends one value whole.
    fn borrows_whole(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // require a borrowed target
        let chain = self.form_chain(origin, target)?;
        let Some(borrow) = chain.ownership_form() else {
            return Ok(false);
        };
        if !matches!(borrow.form, dir::Form::Borrowed(_)) {
            return Ok(false);
        }

        // relate the source to the payload the borrow lends
        let payload = self.readable_value(borrow.value)?;
        Ok(self
            .decide_relation(origin, Relation::Storable, source, payload)?
            .holds())
    }

    /// Return the sole union arm one value enters, else the verdict the arms leave.
    fn viable_union_arms(
        &mut self,
        arms: &[dir::GlobalTypeId],
        mut attempt: impl FnMut(&mut Self, dir::GlobalTypeId) -> CompilerResult<Verdict>,
    ) -> CompilerResult<Result<dir::GlobalTypeId, Verdict>> {
        // try every arm and keep the ones that hold or stay open
        let mut viable = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut holding = 0;
        for arm in arms.iter().copied() {
            let verdict = self.decide_candidate(|state| {
                Ok(match attempt(state, arm)?.holds() {
                    true => CandidateOutcome::Accepted(()),
                    false => CandidateOutcome::Rejected(()),
                })
            })?;
            match verdict {
                Verdict::Holds => {
                    holding += 1;
                    viable.push(arm);
                }
                Verdict::Ambiguous => viable.push(arm),
                Verdict::Fails => {}
            }
        }

        Ok(match (viable.as_slice(), holding) {
            ([arm], _) => Ok(*arm),
            ([], _) => Err(Verdict::Fails),
            (_, 0) => Err(Verdict::Ambiguous),
            _ => Err(Verdict::Holds),
        })
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

        // require the placement a written target region names from the source storage
        let mut verdict = Verdict::Holds;
        let borrow = self.type_borrow(conversion.module, target_borrow)?;
        let target_place = match self.ty(self.shallow_resolve(borrow.region)?)? {
            dir::Type::Region(region) => Some(region.space),
            _ => None,
        };
        if let Some(target_place) = target_place {
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
        let provenance = self.intern_region(place.lifetime, place.placement)?;
        let lifetime =
            self.constrain_type(origin, cause, Relation::Storable, provenance, borrow.region)?;
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

        // lend the borrow itself on a handle acquisition and its payload on a reborrow
        let source_value = match (
            conversion.acquires_handle,
            conversion.source.ownership_form(),
        ) {
            (true, Some(form)) => self.intern_type(dir::Type::Form(form))?,
            (_, Some(form)) => form.value,
            (_, None) => conversion.source.base(),
        };

        // lend the converted payload by the access the borrow requires
        let is_readonly = self.is_readonly_access(borrow.access)?;
        let payload = match is_readonly {
            true => {
                let lent = Value {
                    ty: source_value,
                    place: None,
                    ..source
                };
                let site = self.visit_site(source.node.unwrap_or(self.origin_source(origin)?))?;

                self.constrain_conversion(
                    site,
                    cause,
                    Relation::Storable,
                    lent,
                    conversion.borrow.value,
                    ValueUse::Store,
                )?
            }
            false => self.constrain_form_value(
                origin,
                cause,
                relation,
                conversion.module,
                conversion.borrow.form,
                source_value,
                conversion.borrow.value,
            )?,
        };
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

        // map every concrete source case through the target conversion
        let borrows_whole = self.borrows_whole(origin, source.ty, target)?;
        if !borrows_whole
            && let Some(sources) = self.conversion_source_cases(origin, source.ty, target)?
        {
            return self.convert_source_cases(site, origin, cause, source, sources, target, use_);
        }

        // take the value's edge into the slot
        let (edge, slot) = self.value_edge(origin, source.ty, target)?;
        match edge {
            // reject an open slot at a closed conversion
            ValueEdge::Open => {
                return Err(CompilerError::Internal {
                    message: "closed conversion reached an open slot".to_string(),
                });
            }
            // widen a constant first, then convert the widened runtime value
            ValueEdge::Materialize(base) => {
                let widen = dir::CoercionAdjustment::Materialize { target: base };
                if matches!(self.ty(slot)?, dir::Type::Union(_)) {
                    if !self
                        .decide_relation(origin, Relation::Subtype, source.ty, slot)?
                        .holds()
                    {
                        return Ok(Err(CheckFailure::Relation));
                    }

                    return Ok(Ok(Some(Box::new(dir::Coercion::new(
                        source.ty,
                        vec![widen],
                        dir::CastOrigin::Implicit,
                    )))));
                }
                let widened = Value { ty: base, ..source };
                let rest = self.convert_closed_value(site, origin, cause, widened, target, use_)?;

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
            // inject a singular value into its selected target union case
            ValueEdge::Inject(_) => {
                let head = match self.union_arms(origin, target)? {
                    Some(_) => target,
                    None => self.structurally_normalize(origin, target)?,
                };
                let Some(targets) = self.union_arms(origin, head)? else {
                    return self.convert_existing_value(site, origin, cause, source, target, use_);
                };
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
                    dir::Coercion::union(source.ty, head, vec![case], dir::CastOrigin::Implicit);

                return Ok(Ok(Some(Box::new(coercion))));
            }
            // fall through for erased and stored values
            ValueEdge::Erase | ValueEdge::Store => {}
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

        // convert the existing value through its memory forms and payload cases
        self.convert_existing_value(site, origin, cause, source, target, use_)
    }

    /// Map every concrete source case through the target conversion.
    fn convert_source_cases(
        &mut self,
        site: FlowSite,
        origin: Origin,
        cause: CauseId,
        source: Value,
        sources: SmallVec<[dir::GlobalTypeId; 4]>,
        target: dir::GlobalTypeId,
        use_: ValueUse,
    ) -> CompilerResult<Result<Option<Box<dir::Coercion>>, CheckFailure>> {
        // skip adjustment for a case set already representing the target
        if self.operation_head(source.ty)?.is_some() {
            let joined = self.normalized_union_type(sources.clone())?;
            if self
                .decide_relation(origin, Relation::Equal, joined, target)?
                .holds()
            {
                return Ok(Ok(None));
            }
        }

        // convert every source case into its target arm
        let targets = self.union_arms(origin, target)?;
        let mut cases = Vec::with_capacity(sources.len());
        for source_case in sources {
            let source_case = Value {
                ty: source_case,
                ..source
            };
            let (target_case, conversion) = match &targets {
                Some(targets) => {
                    match self.convert_union_case(
                        site,
                        origin,
                        cause,
                        source_case,
                        targets,
                        use_,
                    )? {
                        Ok(selection) => selection,
                        Err(failure) => return Ok(Err(failure)),
                    }
                }
                None => {
                    let conversion =
                        self.convert_closed_value(site, origin, cause, source_case, target, use_)?;
                    match conversion {
                        Ok(conversion) => (target, conversion),
                        Err(failure) => return Ok(Err(failure)),
                    }
                }
            };
            let adjustments = conversion
                .map(|coercion| coercion.adjustments)
                .unwrap_or_default();

            // record the arm each case enters
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

        // build one union coercion over the recorded cases
        let coercion = dir::Coercion::union(source.ty, target, cases, dir::CastOrigin::Implicit);

        Ok(Ok(Some(Box::new(coercion))))
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
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SmallVec<[dir::GlobalTypeId; 4]>>> {
        // take the arms of a union directly, aliased members expanded
        let expanded = self.structurally_normalize(origin, source)?;
        if let Some(cases) = self.union_arms(origin, expanded)? {
            return Ok(Some(cases));
        }

        // deferred conditionals convert case by case over both branches
        if let Some(dir::TypeOperation::Conditional(conditional)) = self.operation_head(source)? {
            let mut cases = SmallVec::new();
            for branch in [conditional.then_type, conditional.else_type] {
                match self.union_arms(origin, branch)? {
                    Some(arms) => cases.extend(arms),
                    None => cases.push(branch),
                }
            }

            return Ok(Some(cases));
        }

        // convert rigid parameters case by case over their resolved domain
        let source_value = self.strip_form(origin, source)?;
        let dir::Type::Parameter(parameter) = self.ty(source_value)? else {
            return Ok(None);
        };
        if self
            .decide_relation(origin, Relation::Subtype, source, target)?
            .holds()
        {
            return Ok(None);
        }
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
        if let Some(key) = self.goal_key(origin, subject, &operands)? {
            // reuse the recorded answer, which selects by target position
            match self.answers.get(&key) {
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
                    self.answers
                        .insert(key, Answer::Arm(Ok((arm as u16, true))));
                }

                return Ok(Ok((target, None)));
            }
        }

        // identify the sole represented case the source can enter
        let viable = self.viable_union_arms(targets, |state, target| {
            let conversion =
                state.convert_closed_value(site, origin, cause, source, target, use_)?;

            Ok(Verdict::decided(conversion.is_ok()))
        })?;
        let selected = match viable {
            Ok(target) => targets
                .iter()
                .position(|arm| *arm == target)
                .map(|arm| (arm, target)),
            Err(Verdict::Fails) => None,
            Err(_) => return Ok(Err(CheckFailure::AmbiguousUnionCoercion)),
        };

        // answer the arm as a failure when every case rejects the source
        let Some((arm, target)) = selected else {
            if let Some(key) = arm_key {
                self.answers
                    .insert(key, Answer::Arm(Err(CheckFailure::Relation)));
            }

            return Ok(Err(CheckFailure::Relation));
        };

        // answer the arm as a converted case
        if let Some(key) = arm_key {
            self.answers
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
        let conversion = self.convert_closed_value(site, origin, cause, source, target, use_)?;

        // require the conversion to reproduce the decided one
        let Ok(conversion) = conversion else {
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
        let relation = Relation::Storable;
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
            .is_some_and(|form| matches!(form.form, dir::Form::Borrowed(_)))
            || self.has_borrowed_arm(origin, target)?;

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

            // box erased values on entry and exit
            if self.is_erased_value(source_base)? || self.is_erased_value(target_base)? {
                if self.erased_index_signatures_admit(origin, cause, source_base, target_base)?
                    == Verdict::Fails
                {
                    return Ok(Err(CheckFailure::Relation));
                }

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
