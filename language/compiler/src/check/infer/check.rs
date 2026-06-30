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

        // use backward typing when the expression form accepts it
        let answer = match self.reduce_type_head(origin, target)? {
            Answer::Ready(target) => {
                if answer!(self.check_expression_form(site, target, relation, origin, use_)?) {
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

    /// Check one expression form under an expected type.
    fn check_expression_form(
        &mut self,
        site: FlowSite,
        target: dir::GlobalTypeId,
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
            dir::Expression::Parenthesized { expression } => {
                self.check_forward_expression(site, expression, target, relation, origin, use_)
            }
            dir::Expression::Comptime { body } => {
                self.check_forward_expression(site, body, target, relation, origin, use_)
            }
            dir::Expression::Block(block) => {
                self.check_block_expression(site, block, target, relation, origin, use_)
            }
            dir::Expression::Satisfies { expression, .. } => {
                self.check_forward_expression(site, expression, target, relation, origin, use_)
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
                relation,
                origin,
                use_,
            ),
            dir::Expression::TupleExpression { elements } => self.check_tuple_expression(
                site,
                &elements.into_iter().collect::<SmallVec<[_; 4]>>(),
                target,
                relation,
                use_,
            ),
            dir::Expression::ObjectExpression { properties } => self.check_object_expression(
                site,
                &properties.into_iter().collect::<SmallVec<[_; 4]>>(),
                target,
                relation,
                origin,
                use_,
            ),
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
}
