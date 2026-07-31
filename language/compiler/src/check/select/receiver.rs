use crate::CompilerResult;
use crate::check::{
    Answer, BodyState, CandidateOutcome, Cause, CauseKind, Origin, Relation, Value, answer,
};
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
    ) -> CompilerResult<Answer<Option<ReceiverSteps>>> {
        let mut steps = ReceiverSteps::new();
        let mut receiver = receiver;
        loop {
            // try the current step speculatively
            let related = self.confirm_candidate(|state| {
                let cause = state.intern_cause(Cause::root(origin, CauseKind::Expression));
                let head = answer!(state.reduce_type_head(origin, receiver.ty)?);
                let projects_form = matches!(
                    state.ty(head)?,
                    dir::Type::Form(form) if form.form.ownership().is_none()
                );

                // project placement and readonly views before acquiring a borrow
                if !projects_form
                    && let Some(conversion) =
                        answer!(state.borrow_conversion(origin, receiver.ty, this_parameter)?)
                {
                    let acquired = state.constrain_borrow(
                        origin,
                        cause,
                        Relation::Assignable,
                        receiver,
                        &conversion,
                    )?;
                    return match acquired {
                        Answer::Ready(true) => {
                            let borrowed = state.intern_type(dir::Type::Form(dir::FormType {
                                form: conversion.borrow.form,
                                value: receiver.ty,
                            }))?;
                            let adjustment = dir::ReceiverAdjustment::Borrow { ty: borrowed };

                            Ok(Answer::Ready(CandidateOutcome::Accepted(Some(adjustment))))
                        }
                        Answer::Ready(false) => Ok(Answer::Ready(CandidateOutcome::Rejected(()))),
                        Answer::Pending(pending) => Ok(Answer::Pending(pending)),
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
                    Answer::Ready(true) => Ok(Answer::Ready(CandidateOutcome::Accepted(None))),
                    Answer::Ready(false) => Ok(Answer::Ready(CandidateOutcome::Rejected(()))),
                    Answer::Pending(pending) => Ok(Answer::Pending(pending)),
                }
            })?;

            // accept the step, reject it, or poll again
            match related {
                Answer::Ready(Some(adjustment)) => {
                    if let Some(adjustment) = adjustment {
                        steps.push(adjustment);
                    }

                    return Ok(Answer::Ready(Some(steps)));
                }
                Answer::Ready(None) => {}
                Answer::Pending(pending) => return Ok(Answer::Pending(pending)),
            }

            // dereference one step further, or run out of ladder
            let Some(step) = answer!(self.receiver_step(origin, receiver.ty)?) else {
                return Ok(Answer::Ready(None));
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
    ) -> CompilerResult<Answer<Option<dir::ReceiverAdjustment>>> {
        let head = answer!(self.reduce_type_head(origin, receiver)?);

        // dereference one memory form
        if let dir::Type::Form(form) = self.ty(head)? {
            // stop at an owned value whose family defaults to managed,
            //  stepping to the managed form would allocate
            if form.form == dir::Form::Owned
                && answer!(self.check.defaults_to_managed(origin, form.value)?)
            {
                return Ok(Answer::Ready(None));
            }

            return Ok(Answer::Ready(Some(dir::ReceiverAdjustment::Dereference(
                dir::Dereference {
                    receiver: head,
                    target: dir::DereferenceTarget::Direct,
                    ty: form.value,
                },
            ))));
        }

        // project one newtype to its backing
        if let Some(instance) = self.newtype_payload(origin, head)? {
            let backing = instance.backing;

            return Ok(Answer::Ready(Some(
                instance.into_receiver_adjustment(backing),
            )));
        }

        Ok(Answer::Ready(None))
    }
}
