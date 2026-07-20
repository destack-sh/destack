use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, BodyState, Origin, answer};

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
    pub(in crate::check) fn check_function_value(
        &mut self,
        node: dir::GlobalNodeIdAny,
        target: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<bool>> {
        let Some(body) = self.check.lambdas.get(&node).copied() else {
            return Ok(Answer::Ready(true));
        };

        // deduce open signature holes from the contextual callable
        if let Some(target) = target {
            let origin = self.check.node_site(node)?.origin();
            let value = self.check.require_node_type(node)?;
            let value_signature = answer!(self.callable_signature_type(origin, value)?);
            let target_signature = answer!(self.callable_signature_type(origin, target)?);
            if let (Some((value_type, value_function)), Some((target_type, target_function))) =
                (value_signature, target_signature)
            {
                // parameters deduce pairwise
                let value_parameters = self
                    .check
                    .signature_parameters(value_type.module_id, value_function.parameters)?
                    .to_vec();
                let target_parameters = self
                    .check
                    .signature_parameters(target_type.module_id, target_function.parameters)?
                    .to_vec();
                for (value, target) in value_parameters.iter().zip(&target_parameters) {
                    if value.is_rest || target.is_rest {
                        break;
                    }
                    let hole = self.check.settled_root(value.ty)?;
                    if let Some(variable) = self.check.root_variable(hole)? {
                        self.check.commit_solution(variable, target.ty)?;
                    }
                }

                // the signature's return deduces from the target's return
                if let (Some(ret), Some(target_ret)) =
                    (value_function.return_type, target_function.return_type)
                {
                    let hole = self.check.settled_root(ret)?;
                    if let Some(variable) = self.check.root_variable(hole)? {
                        self.check.commit_solution(variable, target_ret)?;
                    }
                }
            }
        }

        // check the body under the deduced signature
        if self.check.node_type_maybe(body.site.node).is_some() {
            return Ok(Answer::Ready(true));
        }

        body.check(self.check)
    }
}
