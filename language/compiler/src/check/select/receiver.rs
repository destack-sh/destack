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
        let mut peeled_place = None;
        loop {
            // try the current step speculatively
            let related = self.confirm_candidate(ProbeReason::Receiver, |state| {
                let adjusted = state.receiver_adjustment(
                    origin,
                    module,
                    receiver,
                    this_parameter,
                    readonly_path,
                    peeled_place,
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
                // remember the outermost storage place across the peel
                if let dir::Form::Placed { place } = form.form {
                    peeled_place.get_or_insert(place);
                }
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
            // an owned reference-family value never steps to its managed
            // form: that conversion is an allocation the caller must spell
            if form.form == dir::Form::Owned
                && answer!(self.check.defaults_to_managed(origin, form.value)?)
            {
                return Ok(Answer::Ready(None));
            }

            return Ok(Answer::Ready(Some(dir::Projection::Dereference {
                read: dir::DereferenceOperation::Direct,
                ty: form.value,
            })));
        }

        // project one newtype to its backing
        if let Some(instance) = self.decompose_newtype(origin, head)? {
            return Ok(Answer::Ready(Some(instance.into_projection())));
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
        peeled_place: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<Option<ReceiverAdjustment>>> {
        // read borrow requirements from the canonical target form
        let this_chain = self.check.form_chain(origin, this_parameter)?;
        if let Some(form) = this_chain.ownership_form()
            && let dir::Form::Borrowed(borrow) = form.form
        {
            let receiver_chain = self.check.form_chain(origin, receiver)?;
            if receiver_chain
                .ownership_form()
                .is_some_and(|form| matches!(form.form, dir::Form::Borrowed(_)))
            {
                return Ok(Answer::Ready(Some(ReceiverAdjustment::direct(
                    receiver,
                    this_parameter,
                ))));
            }

            let borrow = self.check.type_borrow(this_parameter.module_id, borrow)?;
            // places behind or beneath readonly forms never re-borrow writable;
            // the sealed head keeps the view reduction would dissolve
            let head = self.settled_root(receiver)?;
            let readonly_receiver = readonly_path
                || matches!(
                    self.ty(head)?,
                    dir::Type::Form(form) if form.form == dir::Form::Readonly
                );
            if readonly_receiver && !answer!(self.access_is_readonly(origin, borrow.access)?) {
                return Ok(Answer::Ready(None));
            }

            // managed receivers grant exclusivity only in local space
            let is_managed = match receiver_chain.ownership_form() {
                Some(form) => matches!(form.form, dir::Form::Managed),
                // formless receivers answer by their family default
                None => answer!(self.check.defaults_to_managed(origin, receiver)?),
            };
            let access = self.check.access_literal(origin, borrow.access)?;
            if is_managed
                && !self
                    .check
                    .managed_acquisition_granted(access, receiver_chain.place().or(peeled_place))?
            {
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
                receiver,
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
