use crate::CompilerResult;
use crate::check::{BodyState, CandidateOutcome, Cause, CauseKind, Origin, Relation, Value};
use destack_dir as dir;

/// The implicit adjustments selected for one receiver.
pub(in crate::check) type ReceiverSteps = Vec<dir::ReceiverAdjustment>;

impl BodyState<'_, '_> {
    /// Match one implicit method receiver against a `this` parameter.
    pub(in crate::check) fn constrain_receiver_argument(
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
                    let acquired = state.constrain_borrow(
                        origin,
                        cause,
                        Relation::Assignable,
                        receiver,
                        &conversion,
                    )?;

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
                match state.constrain_type(
                    origin,
                    cause,
                    Relation::Assignable,
                    receiver.ty,
                    this_parameter,
                )? {
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
