use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    AccessSet, CandidateOutcome, Cause, CauseKind, CheckState, MemberCandidate, MemberLookup,
    MemberRole, Origin, Protocol, ReceiverForm, Relation, Value, ValueUse, Verdict,
};
use crate::{CheckError, CompilerError, CompilerResult};

/// The most dereference steps one receiver lookup walks.
const DEREFERENCE_LIMIT: usize = 8;

/// One receiver produced by dereferencing the use-site receiver.
#[derive(Debug, Clone)]
pub(in crate::sema) struct ReceiverStep {
    /// The receiver type at this step.
    pub(in crate::sema) ty: dir::GlobalTypeId,
    /// The adjustments reaching this step from the use-site receiver.
    pub(in crate::sema) adjustments: Vec<dir::ReceiverAdjustment>,
}

/// The receiver forms one dereference step offers a member.
#[derive(Debug, Clone)]
pub(in crate::sema) struct ReceiverOffer {
    /// The form the step names, when it names one.
    explicit: Option<ReceiverForm>,
    /// The forms the step's value takes by default.
    defaults: SmallVec<[ReceiverForm; 2]>,
    /// Whether the step is an access view over its value.
    is_view: bool,
}

/// How one member takes a dereference step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum Acceptance {
    /// The member takes the step itself at every default form.
    ByValue,
    /// The member takes a borrow of the step at every default form.
    ByBorrow,
    /// The member takes the step at no form.
    Refused,
}

impl CheckState<'_> {
    /// Match one member key over the receiver's dereference steps.
    pub(in crate::sema) fn match_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: Value,
        subject: dir::MemberSubject,
        key: dir::StaticKey,
        access: dir::Access,
        protocol: Option<&Protocol>,
    ) -> CompilerResult<MemberLookup> {
        // look statics up on their declaration directly
        if subject.space == dir::MemberSpace::Static {
            return match protocol {
                Some(protocol) => self.lookup_protocol_member(
                    origin,
                    module,
                    subject.target,
                    subject.space,
                    key,
                    protocol,
                ),
                None => self.lookup_member(origin, module, subject, key),
            };
        }

        // step down the receiver until one step exposes the key
        let mut step = subject.target;
        for depth in 0..=DEREFERENCE_LIMIT {
            // look the key up at this step, by value and through its borrows
            let mut lookup =
                self.lookup_member_at_step(origin, module, subject, key, step, protocol)?;
            if !lookup.is_empty() {
                // walk to the found depth again, committed, under the access the members require
                let required = self.required_access(&lookup, access)?;
                let dereference = protocol.is_none().then_some(required);
                let steps = self.dereference_steps(origin, receiver, dereference, depth)?;
                let Some(found) = steps.get(depth) else {
                    self.report_borrow_access_not_granted(
                        origin,
                        required,
                        Some(dir::Access::Readonly),
                        subject.target,
                    )?;

                    return Ok(MemberLookup::default());
                };
                for adjustment in found.adjustments.clone().into_iter().rev() {
                    lookup.prepend_adjustment(&adjustment);
                }

                return Ok(lookup);
            }

            // explore one step further, keeping the exploration out of the committed state
            let stepped = Value {
                ty: step,
                ..receiver
            };
            let dereference = protocol.is_none().then_some(dir::Access::Readonly);
            let adjustment = self.decide_deduction(|state| {
                Ok(Some(state.dereference_step(
                    origin,
                    stepped,
                    dereference,
                )?))
            })?;
            let Some(Some(adjustment)) = adjustment else {
                break;
            };
            if depth == DEREFERENCE_LIMIT {
                self.report_dereference_depth_exceeded(origin, receiver.ty)?;
                break;
            }
            step = adjustment.ty();
        }

        Ok(MemberLookup::default())
    }

    /// Return the member one dereference step exposes under a key, by value or through a borrow.
    fn lookup_member_at_step(
        &mut self,
        origin: Origin,
        module: ModuleId,
        subject: dir::MemberSubject,
        key: dir::StaticKey,
        step: dir::GlobalTypeId,
        protocol: Option<&Protocol>,
    ) -> CompilerResult<MemberLookup> {
        // consider the step and its stored value in lookup order
        let mut targets = SmallVec::<[dir::GlobalTypeId; 2]>::from_slice(&[step]);
        let value = self.ownership_payload(origin, step)?;
        if value != step {
            targets.push(value);
        }

        // merge the members the targets expose, each member once
        let mut candidates = Vec::<MemberCandidate>::new();
        for target in targets {
            let stepped = dir::MemberSubject {
                receiver: target,
                target,
                key_source: target,
                ..subject
            };
            let lookup = match protocol {
                Some(protocol) => self.lookup_protocol_member(
                    origin,
                    module,
                    target,
                    subject.space,
                    key,
                    protocol,
                )?,
                None => self.lookup_member(origin, module, stepped, key)?,
            };

            // select a protocol implementation at the first matching receiver form
            if protocol.is_some() && !lookup.is_empty() {
                return Ok(lookup);
            }

            for candidate in lookup.candidates {
                if !candidates.iter().any(|known| known.reads_same(&candidate)) {
                    candidates.push(candidate);
                }
            }
        }

        // keep the candidates of the first tier this step takes
        self.applicable_receiver_tier(origin, step, candidates)
            .map(MemberLookup::from)
    }

    /// Keep the candidates of the first tier one step takes.
    fn applicable_receiver_tier(
        &mut self,
        origin: Origin,
        step: dir::GlobalTypeId,
        candidates: Vec<MemberCandidate>,
    ) -> CompilerResult<Vec<MemberCandidate>> {
        // read the forms the step offers
        let step = self.receiver_offer(origin, step)?;

        // sort each candidate into the tier its receiver form takes
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
            let Some(declared) = candidate.declaration() else {
                fields.push(candidate);
                continue;
            };
            methods.push(candidate.clone());
            // read the form an implicit this takes from its declaring extension
            let explicit_owner = match self.definition(declared.owner)?.as_deref() {
                Some(dir::Definition::Extension(extension)) => {
                    let target = self.normalize(origin, extension.target.r#type())?;

                    self.receiver_form(target)?
                }
                _ => None,
            };
            let ty = self.symbol_type(declared.symbol)?;

            // sort the method by how it takes the step
            match self.accept(origin, ty, explicit_owner, &step)? {
                Acceptance::ByValue => by_value.push(candidate),
                Acceptance::ByBorrow => by_borrow.push(candidate),
                Acceptance::Refused => {}
            }
        }

        // keep every method when no tier takes the step, so selection reports the receiver
        let mut tier = match (by_value.is_empty(), by_borrow.is_empty()) {
            (false, _) => by_value,
            (true, false) => by_borrow,
            (true, true) => methods,
        };
        tier.extend(fields);

        Ok(tier)
    }

    /// Return the receiver form one callable declares, an implicit this taking its owner's form.
    pub(in crate::sema) fn declared_this_form(
        &mut self,
        origin: Origin,
        callable: dir::GlobalTypeId,
        owner_form: ReceiverForm,
    ) -> CompilerResult<ReceiverForm> {
        let this = self
            .signature_head(callable)?
            .and_then(|head| head.this_parameter);
        match this {
            Some(this) if !matches!(self.ty(this)?, dir::Type::This) => {
                let this = self.normalize(origin, this)?;

                Ok(self.receiver_form(this)?.unwrap_or(owner_form))
            }
            _ => Ok(owner_form),
        }
    }

    /// Return the receiver forms one dereference step offers.
    pub(in crate::sema) fn receiver_offer(
        &mut self,
        origin: Origin,
        step: dir::GlobalTypeId,
    ) -> CompilerResult<ReceiverOffer> {
        // read the form the step names
        let step = self.normalize(origin, step)?;
        let is_view = matches!(
            self.ty(step)?,
            dir::Type::Form(form) if form.form.ownership().is_none()
        );
        let explicit = self.receiver_form(step)?;

        // read the default forms of the value beneath the step
        let value = self.shallow_strip_forms(step)?;
        let defaults = match self.ownership(value)? {
            Some(ownership) => SmallVec::from_slice(&[ReceiverForm {
                ownership,
                access: AccessSet::ALL,
            }]),
            None => SmallVec::from_slice(&[ReceiverForm::MANAGED, ReceiverForm::OWNED]),
        };

        Ok(ReceiverOffer {
            explicit,
            defaults,
            is_view,
        })
    }

    /// Return how one callable takes a step at every default form.
    pub(in crate::sema) fn accept(
        &mut self,
        origin: Origin,
        callable: dir::GlobalTypeId,
        owner: Option<ReceiverForm>,
        step: &ReceiverOffer,
    ) -> CompilerResult<Acceptance> {
        // compare the declared receiver form with the step at each default form
        let mut is_by_value = true;
        let mut is_by_borrow = true;
        for default in &step.defaults {
            let step_form = step.explicit.unwrap_or(*default);
            let owner_form = owner.unwrap_or(*default);
            let this_form = self.declared_this_form(origin, callable, owner_form)?;

            // take the step itself, else borrow it outside an access view
            is_by_value &= this_form.is_overlapping(step_form);
            is_by_borrow &= this_form.is_overlapping(step_form)
                || (this_form.ownership == dir::Ownership::Borrowed
                    && step_form.ownership != dir::Ownership::Borrowed
                    && !step.is_view);
        }

        Ok(match (is_by_value, is_by_borrow) {
            (true, _) => Acceptance::ByValue,
            (false, true) => Acceptance::ByBorrow,
            (false, false) => Acceptance::Refused,
        })
    }

    /// Walk the receiver's builtin dereference steps only, beneath memory forms and newtypes.
    pub(in crate::sema) fn builtin_steps(
        &mut self,
        origin: Origin,
        receiver: Value,
    ) -> CompilerResult<Vec<ReceiverStep>> {
        // check one further step to distinguish exhaustion from a complete walk
        let mut steps = self.dereference_steps(origin, receiver, None, DEREFERENCE_LIMIT + 1)?;
        if steps.len() > DEREFERENCE_LIMIT + 1 {
            self.report_dereference_depth_exceeded(origin, receiver.ty)?;
            steps.truncate(DEREFERENCE_LIMIT + 1);
        }

        Ok(steps)
    }

    /// Report a receiver that requires more dereferences than lookup allows.
    fn report_dereference_depth_exceeded(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let source = self.format_type(receiver);
        self.report(
            module,
            CheckError::DereferenceDepthExceeded {
                anchor,
                module,
                source,
                limit: DEREFERENCE_LIMIT,
            },
        );

        Ok(())
    }

    /// Walk the receiver's dereference steps.
    fn dereference_steps(
        &mut self,
        origin: Origin,
        receiver: Value,
        protocol: Option<dir::Access>,
        limit: usize,
    ) -> CompilerResult<Vec<ReceiverStep>> {
        // start at the use-site receiver
        let mut steps = vec![ReceiverStep {
            ty: receiver.ty,
            adjustments: Vec::new(),
        }];
        let mut adjustments = Vec::new();
        let mut ty = receiver.ty;

        // step down until the receiver stops dereferencing
        for _ in 0..limit {
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
        // read the receiver head
        let receiver = self.shallow_resolve(value.ty)?;

        // read through one memory form, stopping where it caps the requested access
        if let dir::Type::Form(form) = self.ty(receiver)? {
            // stop at a borrow or owned form of an object
            if matches!(form.form, dir::Form::Owned | dir::Form::Borrowed(_))
                && self.default_ownership(origin, form.value)? == Some(dir::Ownership::Managed)
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
                        let held = self.type_borrow(receiver.module_id, borrow)?.access;
                        let held = self.shallow_resolve(held)?;
                        match self.ty(held)? {
                            dir::Type::Literal(dir::Literal::String(held)) => {
                                dir::Access::from_text(self.strings().get(held))
                            }
                            _ => None,
                        }
                    }
                    dir::Form::Owned | dir::Form::Raw => None,
                };

                // read a Copy value immutably through a fresh copy
                let copies = access == dir::Access::Immutable
                    && self.decide_copy(origin, form.value, &mut SmallVec::new())?
                        == Verdict::Holds;
                if granted.is_some_and(|granted| !granted.grants(access)) && !copies {
                    return Ok(None);
                }
            }

            let ty = form.value;

            return Ok(Some(dir::ReceiverAdjustment::Dereference(
                dir::Dereference {
                    receiver,
                    protocol: None,
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
        if let Some(instance) = self.decompose_newtype(origin, receiver)? {
            let backing = instance.backing;

            return Ok(Some(instance.into_receiver_adjustment(backing)));
        }

        // step through a stuck named head that carries a memory form
        let head = self.structurally_normalize(origin, receiver)?;
        if head != receiver && matches!(self.ty(head)?, dir::Type::Form(_)) {
            return self.dereference_step(origin, Value { ty: head, ..value }, protocol);
        }

        Ok(None)
    }

    /// Return the frame-lived borrow of one receiver type under the given access.
    pub(in crate::sema) fn frame_borrow_of(
        &mut self,
        receiver: dir::GlobalTypeId,
        access: dir::Access,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // read the value beneath an owned form
        let mut receiver = self.shallow_resolve(receiver)?;
        if let dir::Type::Form(form) = self.ty(receiver)?
            && form.form == dir::Form::Owned
        {
            receiver = self.shallow_resolve(form.value)?;
        }

        // refuse a formed or open receiver
        if matches!(
            self.ty(receiver)?,
            dir::Type::Form(_) | dir::Type::Variable(_)
        ) {
            return Ok(None);
        }

        // intern a frame-lived borrow of the receiver
        let lifetime = self.lifetime_literal(dir::Lifetime::Frame)?;
        let access = self.access_literal(access)?;
        let place = self.local_space()?;
        let region = self.intern_region(lifetime, place)?;
        let form = self.intern_borrow(region, access)?;

        Ok(Some(self.intern_type(dir::Type::Form(dir::FormType {
            form,
            value: receiver,
        }))?))
    }

    /// Return the mode one callable value's receiver term names.
    pub(in crate::sema) fn receiver_mode(
        &self,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<dir::ReceiverMode> {
        let receiver = self.shallow_resolve(receiver)?;
        match self.ty(receiver)? {
            dir::Type::Literal(dir::Literal::String(text)) => {
                dir::ReceiverMode::from_text(self.strings().get(text)).ok_or_else(|| {
                    CompilerError::Internal {
                        message: format!("receiver term {receiver:?} names an unknown mode"),
                    }
                })
            }
            dir::Type::Variable(_) => Ok(dir::ReceiverMode::Borrowed {
                access: dir::Access::Readonly,
            }),
            other => Err(CompilerError::Internal {
                message: format!("receiver term {receiver:?} is a {other:?}"),
            }),
        }
    }

    /// Return the receiver mode literal one callable value writes.
    pub(in crate::sema) fn receiver_literal(
        &mut self,
        mode: dir::ReceiverMode,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let value = self.strings().intern(mode.text());

        self.intern_type(dir::Type::Literal(dir::Literal::String(value)))
    }

    /// Return the receiver parameter one call takes a callable value through.
    pub(in crate::sema) fn call_receiver_parameter(
        &mut self,
        origin: Origin,
        callee: dir::GlobalTypeId,
        mode: dir::ReceiverMode,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match mode {
            // take an already owned callee as written and own any other
            dir::ReceiverMode::Owned => {
                let owned = self.normalize(origin, callee)?;
                if matches!(self.ty(owned)?, dir::Type::Form(form) if form.form == dir::Form::Owned)
                {
                    return Ok(callee);
                }

                self.intern_type(dir::Type::Form(dir::FormType {
                    form: dir::Form::Owned,
                    value: callee,
                }))
            }

            // borrow the callee for the call at a region the receiver solves
            dir::ReceiverMode::Borrowed { access } => {
                let region = self.open_memory_type(origin, dir::MemoryParameter::Region)?;
                let access = self.access_literal(access)?;
                let form = self.intern_borrow(region, access)?;

                self.intern_type(dir::Type::Form(dir::FormType {
                    form,
                    value: callee,
                }))
            }
        }
    }

    /// Relate one callable value's receiver term to the receiver mode its slot takes.
    pub(in crate::sema) fn constrain_receiver_mode(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // decide two closed modes by what the slot grants
        let source_resolved = self.shallow_resolve(source)?;
        let target_resolved = self.shallow_resolve(target)?;
        let is_closed = self.root_variable(source_resolved)?.is_none()
            && self.root_variable(target_resolved)?.is_none();
        if is_closed {
            let source_mode = self.receiver_mode(source_resolved)?;
            let target_mode = self.receiver_mode(target_resolved)?;

            return Ok(Verdict::decided(target_mode.grants(source_mode)));
        }

        // admit every open term into an owned slot
        if self.receiver_mode(target_resolved)? == dir::ReceiverMode::Owned {
            return Ok(Verdict::Holds);
        }

        // bound the value's requirement by the mode its slot grants
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Receiver));
        self.constrain_type(origin, cause, Relation::Storable, source, target)
    }

    /// Return the strongest access the found members require of their receiver.
    fn required_access(
        &mut self,
        lookup: &MemberLookup,
        use_access: dir::Access,
    ) -> CompilerResult<dir::Access> {
        // take the strongest access any declared method requires
        let mut required = use_access;
        for candidate in &lookup.candidates {
            let Some(declared) = candidate.declaration() else {
                continue;
            };
            if candidate.role != MemberRole::Method {
                continue;
            }
            let ty = self.symbol_type(declared.symbol)?;
            let Some(head) = self.signature_head(ty)? else {
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
            let access = self.type_borrow(this.module_id, borrow)?.access;
            let access = self.shallow_resolve(access)?;
            if let dir::Type::Literal(dir::Literal::String(access)) = self.ty(access)?
                && let Some(access) = dir::Access::from_text(self.strings().get(access))
            {
                required = required.join(access);
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
    ) -> CompilerResult<Option<Vec<dir::ReceiverAdjustment>>> {
        // take the first step that satisfies the parameter
        let steps = self.builtin_steps(origin, receiver)?;
        for step in steps {
            let verdict = self.decide_candidate(|state| {
                let adjustments =
                    state.constrain_receiver_step(origin, receiver, &step, this_parameter)?;

                Ok(match adjustments {
                    Some(adjustments) => CandidateOutcome::Accepted(adjustments),
                    None => CandidateOutcome::Rejected(()),
                })
            })?;
            if verdict == Verdict::Fails {
                continue;
            }
            if let Some(adjustments) =
                self.constrain_receiver_step(origin, receiver, &step, this_parameter)?
            {
                return Ok(Some(adjustments));
            }
        }

        Ok(None)
    }

    /// Constrain one receiver step against the this parameter, returning its adjustments.
    fn constrain_receiver_step(
        &mut self,
        origin: Origin,
        receiver: Value,
        step: &ReceiverStep,
        this_parameter: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Vec<dir::ReceiverAdjustment>>> {
        // read the step as a value at this site
        let stepped = Value {
            ty: step.ty,
            ..receiver
        };
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));

        // take the step as is when it names the parameter's ownership
        let step_type = self.normalize(origin, step.ty)?;
        let step_ownership = self
            .form_chain(origin, step_type)?
            .ownership_form()
            .map(|form| form.form.ownership());
        let parameter_type = self.normalize(origin, this_parameter)?;
        let parameter_ownership = self
            .form_chain(origin, parameter_type)?
            .ownership_form()
            .map(|form| form.form.ownership());

        // align sides by ownership, by default ownership, or as a borrowed object with a handle
        let step_object = self.form_chain(origin, step_type)?.base();
        let is_borrowed_object = matches!(step_ownership, Some(Some(dir::Ownership::Borrowed)))
            && self.default_ownership(origin, step_object)? == Some(dir::Ownership::Managed);
        let takes_handle = matches!(
            parameter_ownership,
            None | Some(Some(dir::Ownership::Managed))
        );
        let is_aligned = step_ownership == parameter_ownership
            || self.default_ownership(origin, step_type)?
                == self.default_ownership(origin, parameter_type)?
            || (is_borrowed_object && takes_handle);

        // take the step as an argument of the parameter once the sides align
        if is_aligned {
            let site = self.visit_site(receiver.node.unwrap_or(self.origin_source(origin)?))?;
            let is_direct = self
                .constrain_edge(site, cause, stepped, this_parameter, ValueUse::Argument)?
                .holds();
            if is_direct {
                let mut adjustments = step.adjustments.clone();

                // take the handle of the object behind a borrowed receiver
                if is_borrowed_object && takes_handle && step_ownership != parameter_ownership {
                    adjustments.push(dir::ReceiverAdjustment::Dereference(dir::Dereference {
                        receiver: step.ty,
                        protocol: None,
                        ty: step_object,
                    }));
                }

                return Ok(Some(adjustments));
            }
        }

        // otherwise acquire the borrow the parameter writes, beneath any view form
        let is_view = matches!(
            self.ty(step.ty)?,
            dir::Type::Form(form) if form.form.ownership().is_none()
        );
        let parameter_access = match self.form_chain(origin, parameter_type)?.forms().first() {
            Some(form) => match form.form {
                dir::Form::Borrowed(borrow) => {
                    self.access_of(self.type_borrow(parameter_type.module_id, borrow)?.access)?
                }
                _ => None,
            },
            None => None,
        };

        // lend a view's storage readonly
        if is_view && parameter_access != Some(dir::Access::Readonly) {
            return Ok(None);
        }
        let Some(conversion) = self.borrow_conversion(origin, step.ty, this_parameter)? else {
            return Ok(None);
        };
        let acquired =
            self.constrain_borrow(origin, cause, Relation::Storable, stepped, &conversion)?;
        if !acquired.holds() {
            return Ok(None);
        }
        // lend the object the receiver names
        let value = self.ownership_payload(origin, step.ty)?;
        let borrowed = self.intern_type(dir::Type::Form(dir::FormType {
            form: conversion.borrow.form,
            value,
        }))?;
        let mut adjustments = step.adjustments.clone();

        // reborrow through the reference the step read
        if let Some(dir::ReceiverAdjustment::Dereference(dereference)) = adjustments.last()
            && dereference.protocol.is_none()
            && !self.is_form_dereference(dereference)?
        {
            adjustments.pop();
        }
        adjustments.push(dir::ReceiverAdjustment::Borrow { ty: borrowed });

        Ok(Some(adjustments))
    }

    /// Apply the physical receiver projections required by one flow-narrowed lookup.
    pub(in crate::sema) fn adjust_narrowed_lookup(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        narrowed: dir::GlobalTypeId,
        lookup: &mut MemberLookup,
    ) -> CompilerResult<()> {
        // project union lookups through each physical arm from the source
        if lookup
            .candidates
            .iter()
            .any(|candidate| candidate.arm.is_some())
        {
            let (payload, peeled) = self.project_newtype_receiver(origin, narrowed)?;
            if self.union_arms(origin, payload)?.is_some() {
                for candidate in lookup.candidates.iter_mut() {
                    let Some(arm) = candidate.arm else {
                        continue;
                    };
                    let adjustments =
                        self.project_narrowed_receiver(origin, source, arm.element)?;
                    candidate.strip_adjustments(peeled.len());
                    for adjustment in adjustments.into_iter().rev() {
                        candidate.prepend_adjustment(adjustment);
                    }
                }

                return Ok(());
            }
        }

        // unchanged receivers keep their direct form
        if source == narrowed {
            return Ok(());
        }

        // apply the common projection selected for one precise arm or compiler field
        let adjustments = self.project_narrowed_receiver(origin, source, narrowed)?;
        for adjustment in adjustments.into_iter().rev() {
            lookup.prepend_adjustment(&adjustment);
        }

        Ok(())
    }

    /// Select the runtime projections from one receiver to its flow-narrowed type.
    pub(in crate::sema) fn project_narrowed_receiver(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        narrowed: dir::GlobalTypeId,
    ) -> CompilerResult<Vec<dir::ReceiverAdjustment>> {
        // keep an unchanged receiver direct
        if source == narrowed {
            return Ok(Vec::new());
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

        // project a precise physical union arm, the narrowed value under the payload's forms
        let union = self.form_chain(origin, payload)?.base();
        let arm = self.narrowed_arm_value(origin, payload, narrowed)?;
        if arms.contains(&narrowed) {
            // keep the payload for an arm spanning several flat leaves
            if self.union_arms(origin, narrowed)?.is_some() {
                return Ok(steps);
            }

            steps.push(dir::ReceiverAdjustment::UnionPayload {
                union,
                arm: self.canonical_union_leaf(origin, union, narrowed, "a narrowed receiver")?,
                ty: self.replace_form_value(origin, payload, arm)?,
            });

            return Ok(steps);
        }

        // keep the physical payload for a narrowing that stays inside its arms
        if let Some(narrowed_arms) = self.union_arms(origin, narrowed)? {
            if narrowed_arms.iter().all(|arm| arms.contains(arm)) {
                return Ok(steps);
            }

            // fail loudly on a narrowing the representation cannot represent
            return Err(CompilerError::Internal {
                message: "a narrowed receiver outside its physical union representation"
                    .to_string(),
            });
        }

        // project a flattened arm onto its canonical leaf
        steps.push(dir::ReceiverAdjustment::UnionPayload {
            union,
            arm: self.canonical_union_leaf(origin, union, narrowed, "a flattened narrowing")?,
            ty: self.replace_form_value(origin, payload, arm)?,
        });

        Ok(steps)
    }

    /// Return one narrowed arm beneath its union payload's forms, keeping the forms the arm names.
    fn narrowed_arm_value(
        &mut self,
        origin: Origin,
        payload: dir::GlobalTypeId,
        narrowed: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut carried = self.normalize(origin, payload)?;
        let mut arm = self.normalize(origin, narrowed)?;
        while let dir::Type::Form(form) = self.ty(carried)?
            && let dir::Type::Form(named) = self.ty(arm)?
        {
            carried = self.normalize(origin, form.value)?;
            arm = self.normalize(origin, named.value)?;
        }

        Ok(arm)
    }

    /// Project one representation onto the union arm it keeps beside itself.
    pub(in crate::sema) fn project_carried_arm(
        &mut self,
        origin: Origin,
        representation: dir::GlobalTypeId,
        arm: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Vec<dir::ReceiverAdjustment>>> {
        // require a physical union beneath the enclosing newtypes
        let (payload, mut steps) = self.project_newtype_receiver(origin, representation)?;
        let Some(arms) = self.union_arms(origin, payload)? else {
            return Ok(None);
        };
        if !arms.contains(&arm) {
            return Ok(None);
        }

        // project onto the named arm beneath the payload's forms
        let union = self.form_chain(origin, payload)?.base();
        let value = self.narrowed_arm_value(origin, payload, arm)?;
        steps.push(dir::ReceiverAdjustment::UnionPayload {
            union,
            arm: self.canonical_union_leaf(origin, union, arm, "a carried arm")?,
            ty: self.replace_form_value(origin, payload, value)?,
        });

        Ok(Some(steps))
    }

    /// Project through every enclosing newtype.
    pub(in crate::sema) fn project_newtype_receiver(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<(dir::GlobalTypeId, Vec<dir::ReceiverAdjustment>)> {
        // start at the written receiver
        let mut receiver = source;
        let mut adjustments = Vec::new();

        // unwrap each newtype while keeping its enclosing memory forms
        loop {
            let base = self.form_chain(origin, receiver)?.base();
            let Some(instance) = self.decompose_newtype(origin, base)? else {
                break;
            };
            let backing = self.normalize(origin, instance.backing)?;
            let projected = self.replace_form_value(origin, receiver, backing)?;
            adjustments.push(instance.into_receiver_adjustment(projected));
            receiver = projected;
        }

        Ok((receiver, adjustments))
    }
}
