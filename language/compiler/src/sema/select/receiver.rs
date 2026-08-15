use crate::sema::{
    BodyState, CandidateOutcome, Cause, CauseKind, MemberLookup, Origin, Relation, Value,
};
use crate::{CompilerError, CompilerResult};
use destack_dir as dir;

/// The implicit adjustments selected for one receiver.
pub(in crate::sema) type ReceiverSteps = Vec<dir::ReceiverAdjustment>;

impl BodyState<'_, '_> {
    /// Apply the physical receiver projections required by one flow-narrowed lookup.
    pub(in crate::sema) fn adjust_narrowed_lookup(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        narrowed: dir::GlobalTypeId,
        lookup: &mut MemberLookup,
    ) -> CompilerResult<()> {
        if source == narrowed {
            return Ok(());
        }

        // project ordinary narrowed union lookups through each physical arm
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

        // project a precise physical union arm
        let Some(arms) = self.union_arms(origin, carrier)? else {
            return Ok(steps);
        };
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

        Err(CompilerError::Internal {
            message: "narrowed receiver is outside its physical union carrier".to_string(),
        })
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

    /// Match one implicit method receiver against a `this` parameter.
    pub(in crate::sema) fn constrain_receiver_argument(
        &mut self,
        origin: Origin,
        receiver: Value,
        this_parameter: dir::GlobalTypeId,
    ) -> CompilerResult<Option<ReceiverSteps>> {
        let mut steps = ReceiverSteps::new();
        let mut receiver = receiver;
        loop {
            // try the current step speculatively
            let related = self.confirm_candidate(|state| {
                let cause = state.intern_cause(Cause::root(origin, CauseKind::Expression));
                let head = state.normalize(origin, receiver.ty)?;
                let projects_form = matches!(
                    state.ty(head)?,
                    dir::Type::Form(form) if form.form.ownership().is_none()
                );

                // project placement and readonly views before acquiring a borrow
                if !projects_form
                    && let Some(conversion) =
                        state.borrow_conversion(origin, receiver.ty, this_parameter)?
                {
                    let acquired = state
                        .constrain_borrow(
                            origin,
                            cause,
                            Relation::Assignable,
                            receiver,
                            &conversion,
                        )?
                        .holds();

                    return match acquired {
                        true => {
                            let borrowed = state.intern_type(dir::Type::Form(dir::FormType {
                                form: conversion.borrow.form,
                                value: receiver.ty,
                            }))?;
                            let adjustment = dir::ReceiverAdjustment::Borrow { ty: borrowed };

                            Ok(CandidateOutcome::Accepted(Some(adjustment)))
                        }
                        false => Ok(CandidateOutcome::Rejected(())),
                    };
                }

                // otherwise relate the current receiver without acquiring storage
                match state
                    .constrain_type(
                        origin,
                        cause,
                        Relation::Assignable,
                        receiver.ty,
                        this_parameter,
                    )?
                    .holds()
                {
                    true => Ok(CandidateOutcome::Accepted(None)),
                    false => Ok(CandidateOutcome::Rejected(())),
                }
            })?;

            // accept the step, reject it, or poll again
            if let Some(adjustment) = related {
                if let Some(adjustment) = adjustment {
                    steps.push(adjustment);
                }

                return Ok(Some(steps));
            }

            // dereference one step further, or run out of ladder
            let Some(step) = self.receiver_step(origin, receiver.ty)? else {
                return Ok(None);
            };

            // record the next projected receiver
            receiver.ty = step.ty();
            steps.push(step);
        }
    }

    /// Return the next dereference step beneath one receiver type.
    fn receiver_step(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::ReceiverAdjustment>> {
        // dereference one memory form
        if let dir::Type::Form(form) = self.ty(receiver)? {
            // stop at an owned value whose family defaults to managed
            if form.form == dir::Form::Owned
                && self.check.defaults_to_managed(origin, form.value)?
            {
                return Ok(None);
            }

            return Ok(Some(dir::ReceiverAdjustment::Dereference(
                dir::Dereference {
                    receiver,
                    target: dir::DereferenceTarget::Direct,
                    ty: form.value,
                },
            )));
        }

        // project one newtype to its backing
        if let Some(instance) = self.newtype_payload(origin, receiver)? {
            let backing = instance.backing;

            return Ok(Some(instance.into_receiver_adjustment(backing)));
        }

        // step through a stuck named head that carries a memory form
        let head = self.check.structurally_normalize(origin, receiver)?;
        if head != receiver && matches!(self.ty(head)?, dir::Type::Form(_)) {
            return self.receiver_step(origin, head);
        }

        Ok(None)
    }
}
