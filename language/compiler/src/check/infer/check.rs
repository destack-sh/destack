use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Dependency, ExpectedType, FlowSite, Origin, PlaceUse, Relation, Task,
    ValueUse, answer,
};

impl CheckState<'_> {
    /// Check one expression node against an expected type.
    pub(in crate::check) fn check_expression(
        &mut self,
        site: FlowSite,
        target: dir::GlobalTypeId,
        relation: Relation,
        origin: Origin,
        use_: ValueUse,
    ) -> CompilerResult<Answer<()>> {
        let task = Task::Check {
            site,
            expected: ExpectedType::Type(target),
            relation,
            origin,
            use_,
        };
        if self.solver.is_task_complete(&task) {
            return Ok(Answer::Ready(()));
        }

        if self.committed_node_type_maybe(site.node).is_some() {
            return self.constrain_node_value(site, relation, target, origin, use_);
        }

        // choose the structural checking path from the reduced head
        let answer = match self.reduce_type_head(origin, target)? {
            Answer::Ready(target_head) => {
                if answer!(self.check_expression_with_expectation(
                    site,
                    target,
                    target_head,
                    relation,
                    origin,
                    use_
                )?) {
                    Ok(Answer::Ready(()))
                } else {
                    let () = answer!(
                        self.infer_and_relate_expression(site, target, relation, origin, use_)?
                    );
                    Ok(Answer::Ready(()))
                }
            }
            Answer::Pending(blockers) => {
                // infer through pure inference variables, park on unavailable declarations
                if blockers
                    .iter()
                    .all(|blocker| matches!(blocker, Dependency::Variable(_)))
                {
                    let () = answer!(
                        self.infer_and_relate_expression(site, target, relation, origin, use_)?
                    );

                    Ok(Answer::Ready(()))
                } else {
                    Ok(Answer::Pending(blockers))
                }
            }
        };

        // mark both directions complete after a successful check
        if matches!(answer, Ok(Answer::Ready(()))) {
            self.solver.complete_task(&task);
            self.solver.complete_task(&Task::Infer {
                site,
                use_: PlaceUse::Read,
            });
        }

        answer
    }

    /// Try checking one expression by propagating its expected type.
    fn check_expression_with_expectation(
        &mut self,
        site: FlowSite,
        target: dir::GlobalTypeId,
        target_head: dir::GlobalTypeId,
        relation: Relation,
        origin: Origin,
        use_: ValueUse,
    ) -> CompilerResult<Answer<bool>> {
        let node = site.node.into_typed::<dir::Expression>();
        let expression = self
            .module(node.module_id)
            .view()
            .get(node.local_id)
            .clone();

        match expression {
            dir::Expression::Block(block) => {
                self.check_block_expression(site, block, target, relation, origin, use_)
            }
            dir::Expression::Parenthesized { expression } => {
                self.check_transparent_expression(site, expression, target, relation, origin, use_)
            }
            dir::Expression::Comptime { body } => {
                self.check_transparent_expression(site, body, target, relation, origin, use_)
            }
            dir::Expression::Satisfies { expression, .. } => {
                self.check_transparent_expression(site, expression, target, relation, origin, use_)
            }
            dir::Expression::SequenceExpression { expressions } => self.check_sequence_expression(
                site,
                &expressions.into_iter().collect::<SmallVec<[_; 4]>>(),
                target,
                relation,
                origin,
                use_,
            ),
            dir::Expression::If {
                then_expression,
                else_expression,
                ..
            } => self.check_if_expression(
                site,
                then_expression,
                else_expression,
                target,
                relation,
                origin,
                use_,
            ),
            dir::Expression::ArrayExpression { elements } => self.check_array_expression(
                site,
                &elements.into_iter().collect::<SmallVec<[_; 4]>>(),
                target,
                target_head,
                relation,
                origin,
                use_,
            ),
            dir::Expression::TupleExpression { elements } => self.check_tuple_expression(
                site,
                &elements.into_iter().collect::<SmallVec<[_; 4]>>(),
                target_head,
                relation,
                use_,
            ),
            dir::Expression::ObjectExpression { properties } => self.check_object_expression(
                site,
                &properties.into_iter().collect::<SmallVec<[_; 4]>>(),
                target,
                target_head,
                relation,
                origin,
                use_,
            ),
            dir::Expression::New { .. } => {
                let form = match self.ty(target_head)? {
                    dir::Type::Form(form) if form.form == dir::Form::Owned => form,
                    _ => return Ok(Answer::Ready(false)),
                };
                let () = answer!(self.check_expression(site, form.value, relation, origin, use_)?);

                Ok(Answer::Ready(true))
            }

            _ => Ok(Answer::Ready(false)),
        }
    }

    /// Infer one expression and relate the inferred type to a target.
    fn infer_and_relate_expression(
        &mut self,
        site: FlowSite,
        target: dir::GlobalTypeId,
        relation: Relation,
        origin: Origin,
        use_: ValueUse,
    ) -> CompilerResult<Answer<()>> {
        let () = answer!(self.infer_node(site, PlaceUse::Read)?);
        let source = answer!(self.node_type_at(site)?);
        let variables = self.type_variables(source)?;
        let () = answer!(self.infer_from_expected(origin, relation, source, target, variables)?);
        let () = answer!(self.constrain_node_value(site, relation, target, origin, use_)?);

        Ok(Answer::Ready(()))
    }

    /// Check one expression whose value is exactly its child value.
    pub(in crate::check) fn check_transparent_expression(
        &mut self,
        site: FlowSite,
        child: dir::LocalNodeId<dir::Expression>,
        target: dir::GlobalTypeId,
        relation: Relation,
        origin: Origin,
        use_: ValueUse,
    ) -> CompilerResult<Answer<bool>> {
        let module = site.node.module_id;
        let child_site = self.node_site(child.into_global_any(module))?;
        let () = answer!(self.check_node(child_site, target, relation, origin, use_)?);
        let child_type = answer!(self.node_type_at(child_site)?);
        self.commit_node_type(site.node, child_type)?;

        Ok(Answer::Ready(true))
    }
}
