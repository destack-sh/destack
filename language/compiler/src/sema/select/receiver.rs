use destack_dir as dir;
use destack_source::ModuleId;

use crate::sema::{
    BodyState, CandidateOutcome, Cause, CauseKind, MemberCandidate, MemberLookup, MemberRole,
    Origin, ReceiverForm, Relation, Value,
};
use crate::{CompilerError, CompilerResult};

/// The most dereference steps one receiver probe walks, matching how rustc bounds autoderef.
const DEREFERENCE_LIMIT: usize = 8;

/// The implicit adjustments selected for one receiver.
pub(in crate::sema) type ReceiverSteps = Vec<dir::ReceiverAdjustment>;

/// One receiver reached by dereferencing the use-site receiver.
#[derive(Debug, Clone)]
pub(in crate::sema) struct ReceiverStep {
    /// The receiver type at this step.
    pub(in crate::sema) ty: dir::GlobalTypeId,
    /// The adjustments reaching this step from the use-site receiver.
    pub(in crate::sema) adjustments: ReceiverSteps,
}

impl BodyState<'_, '_> {
    /// Probe one member key over the receiver's dereference steps.
    pub(in crate::sema) fn probe_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: Value,
        subject: dir::MemberSubject,
        key: dir::StaticKey,
        access: dir::Access,
    ) -> CompilerResult<MemberLookup> {
        // look statics up on their declaration directly
        if subject.space == dir::MemberSpace::Static {
            return self.lookup_member(origin, module, subject, key);
        }

        // step down the receiver until one step exposes the key
        let mut step = subject.target;
        for depth in 0..=DEREFERENCE_LIMIT {
            // look the key up at this step, by value and through its borrows
            let mut lookup = self.lookup_member_at_step(origin, module, subject, key, step)?;
            if !matches!(lookup, MemberLookup::Missing) {
                // walk to the found depth again, committed, under the access the members require
                let required = self.required_access(&lookup, access)?;
                let steps = self.autoderef(origin, receiver, required)?;
                let Some(found) = steps.get(depth) else {
                    self.check.report_borrow_access_not_granted(
                        origin,
                        required,
                        Some(dir::Access::Readonly),
                        subject.target,
                    )?;

                    return Ok(MemberLookup::Ambiguous);
                };
                for adjustment in found.adjustments.clone().into_iter().rev() {
                    lookup.prepend_adjustment(adjustment);
                }

                return Ok(lookup);
            }

            // explore one step further, keeping the exploration out of the committed state
            let stepped = Value {
                ty: step,
                ..receiver
            };
            let adjustment = self.probe_deduction(|state| {
                Ok(Some(state.dereference_step(
                    origin,
                    stepped,
                    Some(dir::Access::Readonly),
                )?))
            })?;
            let Some(Some(adjustment)) = adjustment else {
                break;
            };
            step = adjustment.ty();
        }

        Ok(MemberLookup::Missing)
    }

    /// Return the member one dereference step exposes under a key, by value or through a borrow.
    fn lookup_member_at_step(
        &mut self,
        origin: Origin,
        module: ModuleId,
        subject: dir::MemberSubject,
        key: dir::StaticKey,
        step: dir::GlobalTypeId,
    ) -> CompilerResult<MemberLookup> {
        // offer the step by value, then through each borrow of it
        let mut targets = vec![step];
        for access in [
            dir::Access::Readonly,
            dir::Access::Mutable,
            dir::Access::Exclusive,
        ] {
            if let Some(borrowed) = self.frame_borrow_of(step, access)? {
                targets.push(borrowed);
            }
        }

        // merge the candidates the targets expose, stopping at the first other lookup
        let mut candidates = Vec::new();
        let mut lookup = MemberLookup::Missing;
        for target in targets {
            let stepped = dir::MemberSubject {
                receiver: target,
                target,
                key_source: target,
                ..subject
            };
            match self.lookup_member(origin, module, stepped, key)? {
                MemberLookup::Missing => {}
                MemberLookup::Found(found) => {
                    for candidate in found {
                        if !candidates
                            .iter()
                            .any(|known: &MemberCandidate| known.symbol == candidate.symbol)
                        {
                            candidates.push(candidate);
                        }
                    }
                }
                other if candidates.is_empty() => {
                    lookup = other;
                    break;
                }
                _ => break,
            }
        }

        // keep the candidates this step reaches first
        if !candidates.is_empty() {
            let candidates = self.applicable_receiver_tier(origin, step, candidates)?;
            if !candidates.is_empty() {
                lookup = MemberLookup::Found(candidates);
            }
        }

        Ok(lookup)
    }

    /// Keep the candidates one step reaches first.
    fn applicable_receiver_tier(
        &mut self,
        origin: Origin,
        step: dir::GlobalTypeId,
        candidates: Vec<MemberCandidate>,
    ) -> CompilerResult<Vec<MemberCandidate>> {
        // read the form the step itself offers
        let step_type = self.normalize(origin, step)?;
        let is_view = matches!(
            self.ty(step_type)?,
            dir::Type::Form(form) if form.form.ownership().is_none()
        );
        let step_form = self
            .receiver_form(step_type)?
            .unwrap_or(ReceiverForm::MANAGED);

        // sort each candidate into the tier its receiver form reaches
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let mut by_value = Vec::new();
        let mut by_borrow = Vec::new();
        for candidate in candidates {
            // fields read through every form
            if candidate.role != MemberRole::Method {
                fields.push(candidate);
                continue;
            }

            // read the receiver form this method declares
            methods.push(candidate.clone());
            let ty = self.check.symbol_type(candidate.symbol)?;
            let this = self
                .check
                .signature_head(ty)?
                .and_then(|head| head.this_parameter);
            let this_form = match this {
                Some(this) if !matches!(self.ty(this)?, dir::Type::This) => {
                    let this = self.normalize(origin, this)?;
                    self.receiver_form(this)?.unwrap_or(ReceiverForm::MANAGED)
                }
                // an implicit this takes its declaring extension's target form
                _ => match self.definition(candidate.owner)?.cloned() {
                    Some(dir::Definition::Extension(extension)) => {
                        let target = self.normalize(origin, extension.target.r#type())?;
                        self.receiver_form(target)?.unwrap_or(ReceiverForm::MANAGED)
                    }
                    _ => ReceiverForm::MANAGED,
                },
            };

            // take the method by value, or through a borrow of the step
            if this_form.is_overlapping(step_form) {
                by_value.push(candidate);
            } else if this_form.ownership == dir::Ownership::Borrowed
                && step_form.ownership != dir::Ownership::Borrowed
                && !is_view
            {
                by_borrow.push(candidate);
            }
        }

        // fall back to every method for reporting when no tier takes the step
        let mut tier = match (by_value.is_empty(), by_borrow.is_empty()) {
            (false, _) => by_value,
            (true, false) => by_borrow,
            (true, true) => methods,
        };
        tier.extend(fields);

        Ok(tier)
    }

    /// Walk the receiver's dereference steps under the given access.
    pub(in crate::sema) fn autoderef(
        &mut self,
        origin: Origin,
        receiver: Value,
        access: dir::Access,
    ) -> CompilerResult<Vec<ReceiverStep>> {
        self.dereference_steps(origin, receiver, Some(access))
    }

    /// Walk the receiver's builtin dereference steps only, beneath memory forms and newtypes.
    pub(in crate::sema) fn builtin_steps(
        &mut self,
        origin: Origin,
        receiver: Value,
    ) -> CompilerResult<Vec<ReceiverStep>> {
        self.dereference_steps(origin, receiver, None)
    }

    /// Walk the receiver's dereference steps.
    fn dereference_steps(
        &mut self,
        origin: Origin,
        receiver: Value,
        protocol: Option<dir::Access>,
    ) -> CompilerResult<Vec<ReceiverStep>> {
        let mut steps = vec![ReceiverStep {
            ty: receiver.ty,
            adjustments: ReceiverSteps::new(),
        }];
        let mut adjustments = ReceiverSteps::new();
        let mut ty = receiver.ty;
        for _ in 0..DEREFERENCE_LIMIT {
            let stepped = Value { ty, ..receiver };
            let Some(adjustment) = self.dereference_step(origin, stepped, protocol)? else {
                break;
            };
            ty = adjustment.ty();
            adjustments.push(adjustment);
            steps.push(ReceiverStep {
                ty,
                adjustments: adjustments.clone(),
            });
        }

        Ok(steps)
    }

    /// Return the next dereference beneath one receiver type.
    fn dereference_step(
        &mut self,
        origin: Origin,
        value: Value,
        protocol: Option<dir::Access>,
    ) -> CompilerResult<Option<dir::ReceiverAdjustment>> {
        let receiver = self.shallow_resolve(value.ty)?;

        // read through one memory form, stopping where it caps the requested access
        if let dir::Type::Form(form) = self.ty(receiver)? {
            // stop at an owned form over a value that defaults to managed
            if form.form == dir::Form::Owned
                && self.check.default_ownership(origin, form.value)?
                    == Some(dir::Ownership::Managed)
            {
                return Ok(None);
            }

            // stop where the form grants less than the requested access
            if let Some(access) = protocol
                && access != dir::Access::Readonly
            {
                let granted = match form.form {
                    dir::Form::Readonly => Some(dir::Access::Readonly),
                    dir::Form::Borrowed(borrow) => {
                        let held = self.check.type_borrow(receiver.module_id, borrow)?.access;
                        let held = self.shallow_resolve(held)?;
                        match self.ty(held)? {
                            dir::Type::Literal(dir::Literal::String(held)) => {
                                dir::Access::from_text(held)
                            }
                            _ => None,
                        }
                    }
                    dir::Form::Managed { .. } | dir::Form::Owned | dir::Form::Raw => None,
                };
                if granted.is_some_and(|granted| granted < access) {
                    return Ok(None);
                }
            }

            // keep the handle qualification outside the local space
            if let dir::Form::Managed { place } = form.form {
                let place = self.check.shallow_resolve(place)?;
                let is_erased = self.check.erased_constraint(form.value)?.is_some();
                if !is_erased
                    && matches!(
                        self.check.place_space(place)?,
                        Some(dir::Space::Shared | dir::Space::Constant)
                    )
                {
                    return Ok(None);
                }
            }

            // step to a borrow's payload in the region's referent spaces
            let ty = match form.form {
                dir::Form::Borrowed(borrow) => {
                    let borrow = self.check.type_borrow(receiver.module_id, borrow)?;
                    let region = self.check.shallow_resolve(borrow.region)?;
                    let spaces = match self.check.ty(region)? {
                        dir::Type::Region(pair) => pair.space,
                        _ => region,
                    };

                    self.place_relative_type(origin, spaces, form.value)?
                }
                _ => form.value,
            };

            return Ok(Some(dir::ReceiverAdjustment::Dereference(
                dir::Dereference {
                    receiver,
                    target: dir::DereferenceTarget::Direct,
                    ty,
                },
            )));
        }

        // project through the Dereference protocol
        if let Some(access) = protocol {
            let value = Value {
                ty: receiver,
                ..value
            };
            let selected = self.select_dereference(origin, value, access)?;
            if let Some(dir::OperationResolution::One(mut dereference)) = selected {
                dereference.ty = self.normalize(origin, dereference.ty)?;

                return Ok(Some(dir::ReceiverAdjustment::Dereference(dereference)));
            }

            // stop at a type that dereferences under readonly access alone
            if access != dir::Access::Readonly
                && self
                    .select_dereference(origin, value, dir::Access::Readonly)?
                    .is_some()
            {
                return Ok(None);
            }
        }

        // project one newtype to its backing
        if let Some(instance) = self.newtype_payload(origin, receiver)? {
            let backing = instance.backing;

            return Ok(Some(instance.into_receiver_adjustment(backing)));
        }

        // step through a stuck named head that carries a memory form
        let head = self.check.structurally_normalize(origin, receiver)?;
        if head != receiver && matches!(self.ty(head)?, dir::Type::Form(_)) {
            return self.dereference_step(origin, Value { ty: head, ..value }, protocol);
        }

        Ok(None)
    }

    /// Return the frame-lived borrow of one receiver type under the given access.
    fn frame_borrow_of(
        &mut self,
        receiver: dir::GlobalTypeId,
        access: dir::Access,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let mut receiver = self.shallow_resolve(receiver)?;
        if let dir::Type::Form(form) = self.ty(receiver)?
            && form.form == dir::Form::Owned
        {
            receiver = self.shallow_resolve(form.value)?;
        }
        if matches!(
            self.ty(receiver)?,
            dir::Type::Form(_) | dir::Type::Variable(_)
        ) {
            return Ok(None);
        }
        let lifetime = self.lifetime_literal(dir::Lifetime::Frame)?;
        let access = self.access_literal(access)?;
        let place = self.check.local_place()?;
        let region = self.check.intern_region(lifetime, place)?;
        let form = self.check.intern_borrow(region, access)?;

        Ok(Some(self.intern_type(dir::Type::Form(dir::FormType {
            form,
            value: receiver,
        }))?))
    }

    /// Return the strongest access the found members require of their receiver.
    fn required_access(
        &mut self,
        lookup: &MemberLookup,
        use_access: dir::Access,
    ) -> CompilerResult<dir::Access> {
        let MemberLookup::Found(candidates) = lookup else {
            return Ok(use_access);
        };
        let mut required = use_access;
        for candidate in candidates {
            if candidate.role != MemberRole::Method {
                continue;
            }
            let ty = self.check.symbol_type(candidate.symbol)?;
            let Some(head) = self.check.signature_head(ty)? else {
                continue;
            };
            let Some(this) = head.this_parameter else {
                continue;
            };
            let this = self.shallow_resolve(this)?;
            let dir::Type::Form(form) = self.ty(this)? else {
                continue;
            };
            let dir::Form::Borrowed(borrow) = form.form else {
                continue;
            };
            let access = self.check.type_borrow(this.module_id, borrow)?.access;
            let access = self.shallow_resolve(access)?;
            if let dir::Type::Literal(dir::Literal::String(access)) = self.ty(access)?
                && let Some(access) = dir::Access::from_text(access)
            {
                required = required.max(access);
            }
        }

        Ok(required)
    }

    /// Relate one implicit method receiver to its `this` parameter.
    pub(in crate::sema) fn constrain_receiver(
        &mut self,
        origin: Origin,
        receiver: Value,
        this_parameter: dir::GlobalTypeId,
    ) -> CompilerResult<Option<ReceiverSteps>> {
        // solve an open this-parameter place from the receiver
        let parameter_type = self.check.normalize(origin, this_parameter)?;
        if let Some(place) = self.check.form_chain(origin, parameter_type)?.place() {
            let place = self.check.shallow_resolve(place)?;
            if matches!(self.check.ty(place)?, dir::Type::Variable(_)) {
                // read the referent place of the receiver type
                let held = match self.check.form_chain(origin, receiver.ty)?.place() {
                    Some(held) => held,
                    None => self.value_place(origin, receiver)?.placement,
                };
                let cause = self
                    .check
                    .intern_cause(Cause::root(origin, CauseKind::Expression));
                self.check
                    .constrain_type(origin, cause, Relation::Equal, held, place)?;
            }
        }

        let steps = self.builtin_steps(origin, receiver)?;
        for step in steps {
            let stepped = Value {
                ty: step.ty,
                ..receiver
            };
            let related = self.confirm_candidate(|state| {
                let cause = state.intern_cause(Cause::root(origin, CauseKind::Expression));

                // take the step as is when it names the parameter's ownership
                let step_type = state.normalize(origin, step.ty)?;
                let step_ownership = state
                    .form_chain(origin, step_type)?
                    .ownership_form()
                    .map(|form| form.form.ownership());
                let parameter_type = state.normalize(origin, this_parameter)?;
                let parameter_ownership = state
                    .form_chain(origin, parameter_type)?
                    .ownership_form()
                    .map(|form| form.form.ownership());

                // align two sides that carry no ownership through their defaults
                let aligned = step_ownership == parameter_ownership
                    || state.default_ownership(origin, step_type)?
                        == state.default_ownership(origin, parameter_type)?;
                let direct = aligned
                    && state
                        .constrain_type(
                            origin,
                            cause,
                            Relation::Assignable,
                            step.ty,
                            this_parameter,
                        )?
                        .holds();
                if direct {
                    return Ok(CandidateOutcome::Accepted(step.adjustments.clone()));
                }

                // otherwise acquire the borrow the parameter writes, beneath any view form
                let projects_view = matches!(
                    state.ty(step.ty)?,
                    dir::Type::Form(form) if form.form.ownership().is_none()
                );
                if projects_view {
                    return Ok(CandidateOutcome::Rejected(()));
                }
                let Some(conversion) = state.borrow_conversion(origin, step.ty, this_parameter)?
                else {
                    return Ok(CandidateOutcome::Rejected(()));
                };
                let acquired = state
                    .constrain_borrow(origin, cause, Relation::Assignable, stepped, &conversion)?
                    .holds();
                if !acquired {
                    return Ok(CandidateOutcome::Rejected(()));
                }
                let borrowed = state.intern_type(dir::Type::Form(dir::FormType {
                    form: conversion.borrow.form,
                    value: step.ty,
                }))?;
                let mut adjustments = step.adjustments.clone();
                adjustments.push(dir::ReceiverAdjustment::Borrow { ty: borrowed });

                Ok(CandidateOutcome::Accepted(adjustments))
            })?;
            if let Some(adjustments) = related {
                return Ok(Some(adjustments));
            }
        }

        Ok(None)
    }

    /// Apply the physical receiver projections required by one flow-narrowed lookup.
    pub(in crate::sema) fn adjust_narrowed_lookup(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        narrowed: dir::GlobalTypeId,
        lookup: &mut MemberLookup,
    ) -> CompilerResult<()> {
        // project union lookups through each physical arm
        if let MemberLookup::Union(lookups) = lookup
            && let Some(arms) = self.union_arms(origin, narrowed)?
        {
            // require one member lookup for every narrowed arm
            if lookups.len() != arms.len() {
                return Err(CompilerError::Internal {
                    message: "narrowed union lookup does not cover every target arm".to_string(),
                });
            }

            // attach each physical arm projection to its corresponding lookup
            for (lookup, arm) in lookups.iter_mut().zip(arms) {
                let adjustments = self.project_narrowed_receiver(origin, source, arm)?;
                for adjustment in adjustments.into_iter().rev() {
                    lookup.lookup.prepend_adjustment(adjustment);
                }
            }

            return Ok(());
        }

        // unchanged receivers keep their direct form
        if source == narrowed {
            return Ok(());
        }

        // apply the common projection selected for one precise arm or compiler field
        let adjustments = self.project_narrowed_receiver(origin, source, narrowed)?;
        for adjustment in adjustments.into_iter().rev() {
            lookup.prepend_adjustment(adjustment);
        }

        Ok(())
    }

    /// Select the runtime projections from one receiver to its flow-narrowed type.
    fn project_narrowed_receiver(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        narrowed: dir::GlobalTypeId,
    ) -> CompilerResult<ReceiverSteps> {
        if source == narrowed {
            return Ok(ReceiverSteps::new());
        }

        // project through newtypes while keeping their enclosing memory forms
        let (payload, mut steps) = self.project_newtype_receiver(origin, source)?;
        if payload == narrowed {
            return Ok(steps);
        }

        // keep the payload when it names no physical union
        let Some(arms) = self.union_arms(origin, payload)? else {
            return Ok(steps);
        };

        // project a precise physical union arm
        if arms.contains(&narrowed) {
            let union = self.form_chain(origin, payload)?.base();
            let arm = self.form_chain(origin, narrowed)?.base();
            steps.push(dir::ReceiverAdjustment::UnionPayload {
                union,
                arm,
                ty: narrowed,
            });

            return Ok(steps);
        }

        // keep the physical payload for a narrowing that stays inside its arms
        let Some(narrowed_arms) = self.union_arms(origin, narrowed)? else {
            return Ok(steps);
        };
        if narrowed_arms.iter().all(|arm| arms.contains(arm)) {
            return Ok(steps);
        }

        // fail loudly on a narrowing the representation cannot represent
        Err(CompilerError::Internal {
            message: "narrowed receiver is outside its physical union representation".to_string(),
        })
    }

    /// Project one representation onto the union arm it keeps beside itself, or None for other types.
    pub(in crate::sema) fn project_carried_arm(
        &mut self,
        origin: Origin,
        representation: dir::GlobalTypeId,
        arm: dir::GlobalTypeId,
    ) -> CompilerResult<Option<ReceiverSteps>> {
        let (payload, mut steps) = self.project_newtype_receiver(origin, representation)?;
        let Some(arms) = self.union_arms(origin, payload)? else {
            return Ok(None);
        };
        if !arms.contains(&arm) {
            return Ok(None);
        }
        let union = self.form_chain(origin, payload)?.base();
        let payload = self.form_chain(origin, arm)?.base();
        steps.push(dir::ReceiverAdjustment::UnionPayload {
            union,
            arm: payload,
            ty: arm,
        });

        Ok(Some(steps))
    }

    /// Project through every enclosing newtype.
    pub(in crate::sema) fn project_newtype_receiver(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<(dir::GlobalTypeId, ReceiverSteps)> {
        let mut receiver = source;
        let mut steps = ReceiverSteps::new();

        // unwrap each newtype while keeping its enclosing memory forms
        loop {
            let base = self.form_chain(origin, receiver)?.base();
            let Some(instance) = self.decompose_newtype(origin, base)? else {
                break;
            };
            let backing = self.normalize(origin, instance.backing)?;
            let projected = self.replace_form_value(origin, receiver, backing)?;
            steps.push(instance.into_receiver_adjustment(projected));
            receiver = projected;
        }

        Ok((receiver, steps))
    }
}
