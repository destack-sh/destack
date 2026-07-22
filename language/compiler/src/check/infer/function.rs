use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, BodyState, Cause, CauseKind, Origin, Relation, answer};

impl BodyState<'_, '_> {
    /// Return the callable payload of one owned or placed target for a fresh function value.
    pub(in crate::check) fn fresh_value_target(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        self.peel_value_target(origin, target, |form| {
            matches!(form, dir::Form::Owned | dir::Form::Placed { .. })
        })
    }

    /// Return the value beneath owned expected forms, which a fresh call result takes directly.
    pub(in crate::check) fn owned_value_target(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        self.peel_value_target(origin, target, |form| matches!(form, dir::Form::Owned))
    }

    /// Return the value beneath the expected forms a fresh value takes directly.
    fn peel_value_target(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
        peels: impl Fn(&dir::Form) -> bool,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let mut target = target;
        loop {
            // peel settled form heads without waiting on open payloads
            let root = self.check.settled_root(target)?;
            let head = match self.ty(root)? {
                dir::Type::Form(_) => root,
                _ => match self.check.reduce_type_head(origin, root)? {
                    Answer::Ready(head) => head,
                    Answer::Pending(_) => return Ok(Answer::Ready(target)),
                },
            };
            match self.ty(head)? {
                dir::Type::Form(form) if peels(&form.form) => {
                    target = form.value;
                }
                _ => return Ok(Answer::Ready(target)),
            }
        }
    }

    /// Check one function value's body in its receiving context.
    ///
    /// Contextual flow is the ordinary relation: the structural decomposition
    /// assigns parameters, equates the contextual return slot to its contract,
    /// and evidence transmission routes body candidates into open inference.
    pub(in crate::check) fn check_function_value(
        &mut self,
        node: dir::GlobalNodeIdAny,
        target: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<bool>> {
        let Some(body) = self.check.lambdas.get(&node).copied() else {
            return Ok(Answer::Ready(true));
        };

        // constrain the signature against the contextual callable
        if let Some(target) = target {
            let origin = self.check.node_site(node)?.origin();
            let cause = self
                .check
                .intern_cause(Cause::root(origin, CauseKind::Expression));
            let value = self.check.require_node_type(node)?;
            answer!(
                self.check
                    .constrain_type(cause, Relation::Assignable, value, target)?
            );
        }

        body.check(self.check)
    }
}
