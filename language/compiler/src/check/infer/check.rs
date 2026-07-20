use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Answer, BodyState, CauseId, CheckAttempt, CheckFailure, CheckOutcome, ConstructResult,
    Dependency, FlowSite, InferMode, PlaceUse, Relation, ValueUse, answer,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Check one source value type against a target type.
    pub(in crate::check) fn check_expression_relation(
        &mut self,
        cause: CauseId,
        relation: Relation,
        target: dir::GlobalTypeId,
        use_: Option<ValueUse>,
    ) -> CompilerResult<Answer<bool>> {
        let origin = self.cause_origin(cause);
        let Some(expression) = origin.expression() else {
            return Err(CompilerError::Internal {
                message: "expression relation has no expression source".to_string(),
            });
        };
        let site = self.node_site(expression.into())?;

        // function values deduce from the expected callable, then check;
        //  a failing body rejects the value in this context
        let mut target = target;
        if self.check.lambdas.contains_key(&site.node)
            && matches!(relation, Relation::Assignable | Relation::Satisfies)
        {
            // fresh function values materialize at owned and placed targets
            target = answer!(self.fresh_value_target(origin, target)?);
            let holds = answer!(self.check_function_value(site.node, Some(target))?);
            if !holds {
                return Ok(Answer::Ready(false));
            }
        }

        // check with the target when the relation can shape the expression
        let can_check = matches!(
            relation,
            Relation::Assignable | Relation::Writable | Relation::Satisfies
        ) && use_ != Some(ValueUse::Condition);
        if can_check
            && let Some(use_) = use_
            && self.node_type_maybe(site.node).is_none()
        {
            let check = answer!(self.check_expression(site, target, relation, cause, use_)?);
            if matches!(check, CheckOutcome::Fails(_)) {
                return Ok(Answer::Ready(false));
            }
        }
        // otherwise infer before relating
        else if self.node_type_maybe(site.node).is_none() {
            let () = answer!(self.infer_node(site, PlaceUse::Read)?);
        }

        // relate the checked source immediately for candidate matching
        let source = answer!(self.node_type_at(site)?);

        self.constrain_type(cause, relation, source, target)
    }

    /// Check one expression node against an expected type.
    pub(in crate::check) fn check_expression(
        &mut self,
        site: FlowSite,
        target: dir::GlobalTypeId,
        relation: Relation,
        cause: CauseId,
        use_: ValueUse,
    ) -> CompilerResult<Answer<CheckOutcome>> {
        let origin = self.cause_origin(cause);
        // function values deduce from the expected callable, then check;
        //  a failing body rejects the value in this context
        let mut target = target;
        if self.check.lambdas.contains_key(&site.node) {
            // fresh function values materialize at owned and placed targets
            target = answer!(self.fresh_value_target(origin, target)?);
            let holds = answer!(self.check_function_value(site.node, Some(target))?);
            if !holds {
                return Ok(Answer::Ready(CheckOutcome::Fails(CheckFailure::Relation)));
            }
        }
        if self.node_type_maybe(site.node).is_some() {
            let (_, check) = answer!(self.check_node_value(site, relation, target, cause)?);

            return Ok(Answer::Ready(check));
        }

        // try target-sensitive checking before inferred checking, without
        //  forcing open targets: bounds flow and settlement joins them
        let checked = match self.check.reduce_type_head(origin, target)? {
            Answer::Ready(target_head) => answer!(self.try_check_expression(
                site,
                target,
                target_head,
                relation,
                cause,
                use_
            )?),
            Answer::Pending(blockers) => {
                // infer through pure inference variables, park on unavailable declarations
                if blockers
                    .iter()
                    .all(|blocker| matches!(blocker, Dependency::Variable(_)))
                {
                    CheckAttempt::NotApplicable
                } else {
                    return Ok(Answer::Pending(blockers));
                }
            }
        };

        // infer ordinary expressions, then relate them to the target
        match checked {
            CheckAttempt::Checked(check) => Ok(Answer::Ready(check)),
            CheckAttempt::NotApplicable => {
                answer!(self.infer_node(site, PlaceUse::Read)?);
                let (_, check) = answer!(self.check_node_value(site, relation, target, cause)?);

                Ok(Answer::Ready(check))
            }
        }
    }

    /// Try checking one expression using its target.
    fn try_check_expression(
        &mut self,
        site: FlowSite,
        target: dir::GlobalTypeId,
        target_head: dir::GlobalTypeId,
        relation: Relation,
        cause: CauseId,
        use_: ValueUse,
    ) -> CompilerResult<Answer<CheckAttempt>> {
        let origin = self.cause_origin(cause);
        let node = site.node.into_typed::<dir::Expression>();
        let expression = self
            .module(node.module_id)
            .view()
            .get(node.local_id)
            .clone();
        let target_value = answer!(self.check.value_beneath_forms(origin, target_head)?);

        match expression {
            dir::Expression::ScalarLiteral(_) | dir::Expression::TemplateExpression { .. } => {
                answer!(self.infer_expression(site, PlaceUse::Read, InferMode::Exact)?);
                let source = answer!(self.node_type_at(site)?);
                let source = answer!(self.materialize_fresh_value(origin, source, Some(target))?);
                let check = answer!(self.check_value_relation(cause, relation, source, target)?);

                Ok(Answer::Ready(CheckAttempt::Checked(check)))
            }
            dir::Expression::Block(block) => {
                let check = answer!(self.check_block(site, block, target, relation, cause, use_)?);

                Ok(Answer::Ready(CheckAttempt::Checked(check)))
            }
            dir::Expression::Comptime { body } => {
                self.check_transparent_expression(site, body, target, relation, cause, use_)
            }
            dir::Expression::Satisfies { expression, .. } => {
                self.check_transparent_expression(site, expression, target, relation, cause, use_)
            }
            dir::Expression::If {
                condition,
                then_expression,
                else_expression,
                ..
            } => {
                answer!(self.check_condition_operands(site.node.module_id, &condition)?);

                self.check_if_expression(
                    site,
                    then_expression,
                    else_expression,
                    target,
                    relation,
                    cause,
                    use_,
                )
            }
            dir::Expression::Match { value, arms } => self.check_match_expression(
                site,
                value,
                &arms.into_iter().collect::<SmallVec<[_; 4]>>(),
                target,
                relation,
                cause,
                use_,
            ),
            dir::Expression::Switch { value, cases } => {
                answer!(self.infer_switch_statement(
                    site,
                    value,
                    &cases.into_iter().collect::<SmallVec<[_; 4]>>(),
                )?);
                let (_, check) = answer!(self.check_node_value(site, relation, target, cause)?);

                Ok(Answer::Ready(CheckAttempt::Checked(check)))
            }
            dir::Expression::ArrayExpression { elements } => self.check_array_expression(
                site,
                &elements.into_iter().collect::<SmallVec<[_; 4]>>(),
                target,
                target_value,
                relation,
                cause,
                use_,
            ),
            dir::Expression::FixedArrayExpression { value, length } => self
                .check_fixed_array_expression(
                    site,
                    value,
                    length,
                    target,
                    target_value,
                    relation,
                    cause,
                    use_,
                ),
            dir::Expression::TupleExpression { elements } => self.check_tuple_expression(
                site,
                &elements.into_iter().collect::<SmallVec<[_; 4]>>(),
                target,
                target_value,
                relation,
                use_,
            ),
            dir::Expression::ObjectExpression { properties } => self.check_object_expression(
                site,
                &properties.into_iter().collect::<SmallVec<[_; 4]>>(),
                target,
                target_value,
                relation,
                cause,
                use_,
            ),
            dir::Expression::StructExpression { ty, properties } => {
                let construct_target =
                    answer!(self.select_construct_target(site, ty, Some(target_value))?);
                let construct_target = answer!(self.materialize_fresh_value(
                    origin,
                    construct_target,
                    Some(target),
                )?);
                let field_check = answer!(self.select_property_merge(
                    site,
                    &properties.into_iter().collect::<SmallVec<[_; 4]>>(),
                    Some(construct_target),
                )?);
                let (_, check) = answer!(self.check_node_value(site, relation, target, cause)?);

                Ok(Answer::Ready(CheckAttempt::Checked(field_check.and(check))))
            }
            dir::Expression::Call {
                left,
                generic_arguments,
                arguments,
                ..
            } => {
                // fresh call results materialize at owned expected forms
                let target = answer!(self.owned_value_target(origin, target)?);
                let selected = answer!(self.select_call(
                    site,
                    left,
                    &generic_arguments.into_iter().collect::<SmallVec<[_; 2]>>(),
                    &arguments.into_iter().collect::<SmallVec<[_; 4]>>(),
                    Some(target),
                )?);

                Ok(Answer::Ready(CheckAttempt::Checked(selected)))
            }
            dir::Expression::New { ty, arguments } => {
                answer!(self.select_construct(
                    site,
                    ty,
                    &arguments.into_iter().collect::<SmallVec<[_; 4]>>(),
                    ConstructResult::Direct,
                    Some(target),
                )?);
                let (_, check) = answer!(self.check_node_value(site, relation, target, cause)?);

                Ok(Answer::Ready(CheckAttempt::Checked(check)))
            }
            dir::Expression::NewMaybe { ty, arguments } => {
                answer!(self.select_construct(
                    site,
                    ty,
                    &arguments.into_iter().collect::<SmallVec<[_; 4]>>(),
                    ConstructResult::Fallible,
                    Some(target),
                )?);
                let (_, check) = answer!(self.check_node_value(site, relation, target, cause)?);

                Ok(Answer::Ready(CheckAttempt::Checked(check)))
            }

            _ => Ok(Answer::Ready(CheckAttempt::NotApplicable)),
        }
    }

    /// Check one expression whose value is exactly its child value.
    pub(in crate::check) fn check_transparent_expression(
        &mut self,
        site: FlowSite,
        child: dir::LocalNodeId<dir::Expression>,
        target: dir::GlobalTypeId,
        relation: Relation,
        cause: CauseId,
        use_: ValueUse,
    ) -> CompilerResult<Answer<CheckAttempt>> {
        let module = site.node.module_id;
        let child_site = self.node_site(child.into_global_any(module))?;
        let check = answer!(self.check_node_expected(child_site, target, relation, cause, use_)?);
        let child_type = answer!(self.node_type_at(child_site)?);
        self.commit_node_type(site.node, child_type)?;

        Ok(Answer::Ready(CheckAttempt::Checked(check)))
    }
}
