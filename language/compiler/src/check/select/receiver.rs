use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, BodyState, CandidateOutcome, Cause, CauseKind, Origin, ProbeReason, Relation, answer,
};

/// The projection steps picked for one receiver.
pub(in crate::check) type ReceiverSteps = SmallVec<[dir::Projection; 2]>;

/// One adjusted receiver relation to try.
struct ReceiverAdjustment {
    /// The source type after implicit adjustment.
    source: dir::GlobalTypeId,
    /// The target type the adjusted source must satisfy.
    target: dir::GlobalTypeId,
    /// The projection produced by the adjustment.
    projection: Option<dir::Projection>,
}

impl ReceiverAdjustment {
    /// Create an adjustment without a projection step.
    fn direct(source: dir::GlobalTypeId, target: dir::GlobalTypeId) -> Self {
        Self {
            source,
            target,
            projection: None,
        }
    }

    /// Create an adjustment with one projection step.
    fn projected(
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        projection: dir::Projection,
    ) -> Self {
        Self {
            source,
            target,
            projection: Some(projection),
        }
    }
}

impl BodyState<'_, '_> {
    /// Match one implicit method receiver against a `this` parameter.
    pub(in crate::check) fn constrain_receiver_argument(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        this_parameter: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<ReceiverSteps>>> {
        let mut steps = ReceiverSteps::new();
        let mut receiver = receiver;
        let mut readonly_path = false;
        loop {
            // try the current step speculatively
            let related = self.confirm_candidate(ProbeReason::Receiver, |state| {
                let adjusted = state.receiver_adjustment(
                    origin,
                    module,
                    receiver,
                    this_parameter,
                    readonly_path,
                )?;
                let Some(adjusted) = answer!(adjusted) else {
                    return Ok(Answer::Ready(CandidateOutcome::Rejected(())));
                };

                let cause = state.intern_cause(Cause::root(origin, CauseKind::Expression));

                match state.constrain_type(
                    cause,
                    Relation::Assignable,
                    adjusted.source,
                    adjusted.target,
                )? {
                    Answer::Ready(true) => Ok(Answer::Ready(CandidateOutcome::Accepted(
                        adjusted.projection,
                    ))),
                    Answer::Ready(false) => Ok(Answer::Ready(CandidateOutcome::Rejected(()))),
                    Answer::Pending(pending) => Ok(Answer::Pending(pending)),
                }
            })?;

            // accept the step, reject it, or poll again
            match related {
                Answer::Ready(Some(borrow)) => {
                    if let Some(borrow) = borrow {
                        steps.push(borrow);
                    }

                    return Ok(Answer::Ready(Some(steps)));
                }
                Answer::Ready(None) => {}
                Answer::Pending(pending) => return Ok(Answer::Pending(pending)),
            }

            // dereference one step further, or run out of ladder
            let head = answer!(self.reduce_type_head(origin, receiver)?);
            if let dir::Type::Form(form) = self.ty(head)? {
                // places behind readonly forms never re-borrow writable
                readonly_path |= match form.form {
                    dir::Form::Readonly => true,
                    dir::Form::Borrowed(borrow) => {
                        let access = self.check.type_borrow(head.module_id, borrow)?.access;

                        answer!(self.access_is_readonly(origin, access)?)
                    }
                    _ => false,
                };
            }
            let Some(step) = answer!(self.receiver_step(origin, receiver)?) else {
                return Ok(Answer::Ready(None));
            };

            // record the next projected receiver
            receiver = step.ty();
            steps.push(step);
        }
    }

    /// Return the next dereference step beneath one receiver type.
    fn receiver_step(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::Projection>>> {
        let head = answer!(self.reduce_type_head(origin, receiver)?);

        // dereference one memory form
        if let dir::Type::Form(form) = self.ty(head)? {
            return Ok(Answer::Ready(Some(dir::Projection::Dereference {
                read: dir::DereferenceOperation::Direct,
                ty: form.value,
            })));
        }

        // project one newtype to its backing
        if let Some(projection) = answer!(self.newtype_backing_projection(origin, head)?) {
            return Ok(Answer::Ready(Some(projection)));
        }

        Ok(Answer::Ready(None))
    }

    /// Return one receiver adjustment for a method call.
    fn receiver_adjustment(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        this_parameter: dir::GlobalTypeId,
        readonly_path: bool,
    ) -> CompilerResult<Answer<Option<ReceiverAdjustment>>> {
        // borrowed receivers relate to the declared this directly
        let receiver_head = answer!(self.reduce_type_head(origin, receiver)?);
        if matches!(
            self.ty(receiver_head)?,
            dir::Type::Form(form) if matches!(form.form, dir::Form::Borrowed(_))
        ) {
            return Ok(Answer::Ready(Some(ReceiverAdjustment::direct(
                receiver,
                this_parameter,
            ))));
        }

        // read borrow requirements from a direct borrowed `this`
        let this_parameter = answer!(self.reduce_type_head(origin, this_parameter)?);
        if let dir::Type::Form(form) = self.ty(this_parameter)?
            && let dir::Form::Borrowed(borrow) = form.form
        {
            let borrow = self.check.type_borrow(this_parameter.module_id, borrow)?;
            // places behind readonly forms never re-borrow writable
            if readonly_path && !answer!(self.access_is_readonly(origin, borrow.access)?) {
                return Ok(Answer::Ready(None));
            }
            let form = self
                .check
                .intern_borrow(module, borrow.lifetime, borrow.access)?;
            let borrowed = self.intern_type(
                module,
                dir::Type::Form(dir::FormType {
                    form,
                    value: receiver,
                }),
            )?;
            let projection = dir::Projection::Borrow {
                access: None,
                ty: borrowed,
            };

            return Ok(Answer::Ready(Some(ReceiverAdjustment::projected(
                borrowed,
                this_parameter,
                projection,
            ))));
        }

        Ok(Answer::Ready(Some(ReceiverAdjustment::direct(
            receiver,
            this_parameter,
        ))))
    }
}
