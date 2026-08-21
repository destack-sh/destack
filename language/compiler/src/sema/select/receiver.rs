use destack_dir as dir;
use destack_source::ModuleId;

use crate::sema::{
    BodyState, CandidateOutcome, Cause, CauseKind, MemberCandidate, MemberLookup, MemberRole,
    Origin, ReceiverForm, Relation, Value,
};
use crate::{CompilerError, CompilerResult};

/// The most dereference steps one receiver probe walks, as rustc bounds autoderef.
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
    ///
    /// Each step offers the key by value, then through its borrows, and selection takes the
    /// first applicable candidate in that order.
    pub(in crate::sema) fn probe_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: Value,
        subject: dir::MemberSubject,
        key: dir::StaticKey,
        access: dir::Access,
    ) -> CompilerResult<MemberLookup> {
        // statics name their declaration, without a receiver to step
        if subject.space == dir::MemberSpace::Static {
            return self.lookup_member(origin, module, subject, key);
        }

        // step down the receiver until one step exposes the key
        let mut step = subject.target;
        for depth in 0..=DEREFERENCE_LIMIT {
            // look the key up at this step, by value and through its borrows
            let mut lookup = self.lookup_member_at_step(origin, module, subject, key, step)?;
            if !matches!(lookup, MemberLookup::Missing) {
                // walk to the found depth again, committed, under the access the members demand
                let demanded = self.demanded_access(&lookup, access)?;
                let steps = self.autoderef(origin, receiver, demanded)?;
                let Some(found) = steps.get(depth) else {
                    self.check.report_borrow_access_not_granted(
                        origin,
                        demanded,
                        Some(dir::Access::Readonly),
                        subject.target,
                    )?;

                    return Ok(MemberLookup::Undecided);
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
                Ok(CandidateOutcome::<(), _>::Rejected(
                    state.dereference_step(origin, stepped, Some(dir::Access::Readonly))?,
                ))
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
                key_type: target,
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
    ///
    /// Methods taking the step by value come before methods taking a borrow of it, while fields
    /// read through every form.
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
            if this_form.overlaps(step_form) {
                by_value.push(candidate);
            } else if this_form.ownership == dir::Ownership::Borrowed
                && step_form.ownership != dir::Ownership::Borrowed
                && !is_view
            {
                by_borrow.push(candidate);
            }
        }

        // a step no tier takes keeps every method for reporting
        let mut tier = match (by_value.is_empty(), by_borrow.is_empty()) {
            (false, _) => by_value,
            (true, false) => by_borrow,
            (true, true) => methods,
        };
        tier.extend(fields);

        Ok(tier)
    }

    /// Walk the receiver's dereference steps under the given access.
    ///
    /// The steps run from the receiver itself down through each memory form, newtype, and
    /// `Dereference` projection.
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

    /// Walk the receiver's dereference steps, through the `Dereference` protocol under the
    /// given access when one is given.
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
            if form.form == dir::Form::Owned
                && self.check.defaults_to_managed(origin, form.value)?
            {
                return Ok(None);
            }
            if let Some(access) = protocol
                && access != dir::Access::Readonly
            {
                let granted = match form.form {
                    dir::Form::Readonly => Some(dir::Access::Readonly),
                    dir::Form::Borrowed(borrow) => {
                        let held = self.check.type_borrow(receiver.module_id, borrow)?.access;
                        let held = self.shallow_resolve(held)?;
                        match self.ty(held)? {
                            dir::Type::Memory(dir::MemoryLiteral::Access(held)) => Some(held),
                            _ => None,
                        }
                    }
                    _ => None,
                };
                if granted.is_some_and(|granted| granted < access) {
                    return Ok(None);
                }
            }

            return Ok(Some(dir::ReceiverAdjustment::Dereference(
                dir::Dereference {
                    receiver,
                    target: dir::DereferenceTarget::Direct,
                    ty: form.value,
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

    /// Return the frame-lived borrow of one receiver type under the given access: a value
    /// borrows as is, an owned value lends its payload.
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
        let lifetime = self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Lifetime(
            dir::Lifetime::Frame,
        )))?;
        let access = self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Access(access)))?;
        let form = self.check.intern_borrow(lifetime, access)?;

        Ok(Some(self.intern_type(dir::Type::Form(dir::FormType {
            form,
            value: receiver,
        }))?))
    }

    /// Return the strongest access the found members demand of their receiver: a method's
    /// `this` access, or the use's own access for fields.
    fn demanded_access(
        &mut self,
        lookup: &MemberLookup,
        use_access: dir::Access,
    ) -> CompilerResult<dir::Access> {
        let MemberLookup::Found(candidates) = lookup else {
            return Ok(use_access);
        };
        let mut demanded = use_access;
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
            if let dir::Type::Memory(dir::MemoryLiteral::Access(access)) = self.ty(access)? {
                demanded = demanded.max(access);
            }
        }

        Ok(demanded)
    }

    /// Relate one implicit method receiver to its `this` parameter.
    pub(in crate::sema) fn constrain_receiver(
        &mut self,
        origin: Origin,
        receiver: Value,
        this_parameter: dir::GlobalTypeId,
    ) -> CompilerResult<Option<ReceiverSteps>> {
        let steps = self.builtin_steps(origin, receiver)?;
        for step in steps {
            let stepped = Value {
                ty: step.ty,
                ..receiver
            };
            let related = self.confirm_candidate(|state| {
                let cause = state.intern_cause(Cause::root(origin, CauseKind::Expression));

                // take the step as is when it carries the parameter's ownership form
                let step_type = state.normalize(origin, step.ty)?;
                let step_form = state
                    .form_chain(origin, step_type)?
                    .ownership_form()
                    .map(|form| form.form.ownership());
                let parameter_type = state.normalize(origin, this_parameter)?;
                let parameter_form = state
                    .form_chain(origin, parameter_type)?
                    .ownership_form()
                    .map(|form| form.form.ownership());
                if step_form == parameter_form
                    && state
                        .constrain_type(
                            origin,
                            cause,
                            Relation::Assignable,
                            step.ty,
                            this_parameter,
                        )?
                        .holds()
                {
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

        // project newtype carriers while retaining their enclosing memory forms
        let (carrier, mut steps) = self.project_newtype_receiver(origin, source)?;
        if carrier == narrowed {
            return Ok(steps);
        }

        // keep a carrier that names no physical union
        let Some(arms) = self.union_arms(origin, carrier)? else {
            return Ok(steps);
        };

        // project a precise physical union arm
        if arms.contains(&narrowed) {
            let union = self.form_chain(origin, carrier)?.base();
            let arm = self.form_chain(origin, narrowed)?.base();
            steps.push(dir::ReceiverAdjustment::UnionPayload {
                union,
                arm,
                ty: narrowed,
            });

            return Ok(steps);
        }

        // narrowed union subsets retain the physical carrier representation
        let Some(narrowed_arms) = self.union_arms(origin, narrowed)? else {
            return Ok(steps);
        };
        if narrowed_arms.iter().all(|arm| arms.contains(arm)) {
            return Ok(steps);
        }

        // fail loudly on a narrowing the carrier cannot represent
        Err(CompilerError::Internal {
            message: "narrowed receiver is outside its physical union carrier".to_string(),
        })
    }

    /// Project one carrier onto the union arm it keeps beside itself, `None` for a type that
    /// carries no such arm.
    pub(in crate::sema) fn project_carried_arm(
        &mut self,
        origin: Origin,
        carrier: dir::GlobalTypeId,
        arm: dir::GlobalTypeId,
    ) -> CompilerResult<Option<ReceiverSteps>> {
        let (payload, mut steps) = self.project_newtype_receiver(origin, carrier)?;
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

        // unwrap each nominal carrier while preserving its memory forms
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
