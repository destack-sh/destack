use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, Relation, answer};

impl CheckState<'_> {
    /// Match one implicit method receiver against a `this` parameter.
    pub(in crate::check) fn constrain_receiver_argument(
        &mut self,
        origin: Origin,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        receiver: dir::GlobalTypeId,
        this_parameter: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let (source, target) =
            answer!(self.receiver_relation(origin, module, source, receiver, this_parameter)?);

        self.constrain(origin, Relation::Assignable, source, target)
    }

    /// Select the receiver relation for one method call autoref adjustment.
    fn receiver_relation(
        &mut self,
        origin: Origin,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        receiver: dir::GlobalTypeId,
        this_parameter: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<(dir::GlobalTypeId, dir::GlobalTypeId)>> {
        let receiver_head = answer!(self.reduce_type_head(origin, receiver)?);
        if matches!(
            self.ty(receiver_head)?,
            dir::Type::Form(form) if matches!(form.form, dir::Form::Borrowed { .. })
        ) {
            return Ok(Answer::Ready((receiver, this_parameter)));
        }

        // materialize autoref when the `this` parameter demands a borrow
        let Some(borrow) = answer!(self.implicit_borrow(origin, module, source, this_parameter)?)
        else {
            return Ok(Answer::Ready((receiver, this_parameter)));
        };
        let borrowed = self.intern_type(
            module,
            dir::Type::Form(dir::FormType {
                form: dir::Form::Borrowed {
                    lifetime: borrow.lifetime,
                    access: borrow.access,
                },
                value: receiver,
            }),
        )?;

        Ok(Answer::Ready((borrowed, borrow.target)))
    }
}
