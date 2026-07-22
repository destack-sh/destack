use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Answer, BodyState, CandidateOutcome, CandidateVerdict, CauseId, CheckAttempt, CheckFailure,
    CheckOutcome, Constraint, ConstructResult, Dependency, Expectation, FlowSite, PlaceUse,
    Relation, ValueCheck, ValueUse, answer,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Check one expression node against an expected type.
    pub(in crate::check) fn check_expression(
        &mut self,
        site: FlowSite,
        expectation: Expectation,
    ) -> CompilerResult<Answer<ValueCheck>> {
        let origin = self.cause_origin(expectation.cause);
        let is_resolved = match self.reduce_type_head(origin, expectation.target)? {
            Answer::Ready(_) => true,
            Answer::Pending(blockers)
                if blockers
                    .iter()
                    .all(|blocker| matches!(blocker, Dependency::Variable(_))) =>
            {
                false
            }
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        // use a resolved contextual type before inference
        if is_resolved {
            let checked = answer!(self.try_check_expression(site, expectation)?);
            if let CheckAttempt::Checked(check) = checked {
                return Ok(Answer::Ready(check));
            }

            // select one contextual union member when exactly one applies
            if expectation.relation.distributes_over_union_target()
                && let Some(check) = answer!(self.check_union_target(site, expectation)?)
            {
                return Ok(Answer::Ready(check));
            }
        }

        // infer expressions without a target-directed rule
        answer!(self.infer_node(site, PlaceUse::Read)?);

        Ok(Answer::Ready(ValueCheck {
            outcome: CheckOutcome::Holds,
            target: expectation.target,
        }))
    }

    /// Check one expression against one uniquely applicable union member.
    fn check_union_target(
        &mut self,
        site: FlowSite,
        expectation: Expectation,
    ) -> CompilerResult<Answer<Option<ValueCheck>>> {
        let origin = self.cause_origin(expectation.cause);
        let Some(targets) = answer!(self.check.union_arms(origin, expectation.target)?) else {
            return Ok(Answer::Ready(None));
        };
        let mut viable = None;
        let mut is_viable_ambiguous = false;
        let mut indeterminate = None;
        let mut is_indeterminate_ambiguous = false;

        // classify every member without retaining speculative state
        for target in targets {
            let candidate = Expectation {
                target,
                ..expectation
            };
            let verdict = answer!(self.probe_candidate(|state| {
                let checked = answer!(state.try_check_expression(site, candidate)?);
                let outcome = match checked {
                    CheckAttempt::Checked(check) if check.outcome == CheckOutcome::Holds => {
                        let constraint = Constraint::value(
                            candidate.relation,
                            site.node,
                            candidate.target,
                            candidate.cause,
                            candidate.use_,
                        );
                        state.check.push_constraint(constraint);

                        CandidateOutcome::Accepted(())
                    }
                    CheckAttempt::Checked(_) | CheckAttempt::NotApplicable => {
                        CandidateOutcome::Rejected(())
                    }
                };

                Ok(Answer::Ready(outcome))
            })?);

            // prefer a unique viable member over speculative members
            match verdict {
                CandidateVerdict::Viable if viable.replace(target).is_some() => {
                    is_viable_ambiguous = true;
                }
                CandidateVerdict::Viable => {}
                CandidateVerdict::Indeterminate if indeterminate.replace(target).is_some() => {
                    is_indeterminate_ambiguous = true;
                }
                CandidateVerdict::Indeterminate | CandidateVerdict::Rejected => {}
            }
        }

        // infer ambiguous contexts before selecting a member
        let selected = match (
            is_viable_ambiguous,
            viable,
            is_indeterminate_ambiguous,
            indeterminate,
        ) {
            (false, Some(selected), _, _) => Some(selected),
            (_, None, false, Some(selected)) => Some(selected),
            _ => None,
        };
        let Some(target) = selected else {
            return Ok(Answer::Ready(None));
        };

        // confirm the selected member in the owning state
        let candidate = Expectation {
            target,
            ..expectation
        };
        let checked = answer!(self.try_check_expression(site, candidate)?);
        let CheckAttempt::Checked(check) = checked else {
            return Err(CompilerError::Internal {
                message: "confirmed contextual union member became inapplicable".to_string(),
            });
        };
        let check = ValueCheck {
            outcome: check.outcome,
            target: expectation.target,
        };

        Ok(Answer::Ready(Some(check)))
    }

    /// Try checking one expression using its target.
    fn try_check_expression(
        &mut self,
        site: FlowSite,
        expectation: Expectation,
    ) -> CompilerResult<Answer<CheckAttempt>> {
        let Expectation {
            target,
            relation,
            cause,
            use_,
        } = expectation;
        let origin = self.cause_origin(cause);
        let node = site.node.into_typed::<dir::Expression>();
        let expression = self
            .module(node.module_id)
            .view()
            .get(node.local_id)
            .clone();

        // function values deduce their parameters from a callable target
        if self.check.lambdas.contains_key(&site.node) {
            let target = answer!(self.fresh_value_target(origin, target)?);
            let holds = answer!(self.check_function_value(site.node, Some(target))?);
            if !holds {
                return Ok(Answer::Ready(CheckAttempt::Checked(ValueCheck {
                    outcome: CheckOutcome::Fails(CheckFailure::Relation),
                    target,
                })));
            }
            let check = ValueCheck {
                outcome: CheckOutcome::Holds,
                target,
            };

            return Ok(Answer::Ready(CheckAttempt::Checked(check)));
        }

        match expression {
            dir::Expression::ScalarLiteral(value) => {
                let source = self.scalar_literal_type(node, value)?;
                let source = answer!(self.materialize_fresh_value(origin, source, Some(target))?);
                self.commit_node_type(site.node, source)?;
                let check = ValueCheck {
                    outcome: CheckOutcome::Holds,
                    target,
                };

                Ok(Answer::Ready(CheckAttempt::Checked(check)))
            }
            dir::Expression::TemplateExpression { value } => {
                let source = answer!(self.template_expression_type(site, value)?);
                let source = answer!(self.materialize_fresh_value(origin, source, Some(target))?);
                self.commit_node_type(site.node, source)?;
                let check = ValueCheck {
                    outcome: CheckOutcome::Holds,
                    target,
                };

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
            expression @ (dir::Expression::ArrayExpression { .. }
            | dir::Expression::FixedArrayExpression { .. }
            | dir::Expression::TupleExpression { .. }
            | dir::Expression::ObjectExpression { .. }) => {
                // open expectations provide no structural guidance
                let target_value = match self.check.reduce_type_head(origin, target)? {
                    Answer::Ready(target) => answer!(self.check.strip_form(origin, target)?),
                    Answer::Pending(blockers)
                        if blockers
                            .iter()
                            .all(|blocker| matches!(blocker, Dependency::Variable(_))) =>
                    {
                        return Ok(Answer::Ready(CheckAttempt::NotApplicable));
                    }
                    Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                };

                match expression {
                    dir::Expression::ArrayExpression { elements } => self.check_array_expression(
                        site,
                        &elements.into_iter().collect::<SmallVec<[_; 4]>>(),
                        target,
                        target_value,
                        relation,
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
                    dir::Expression::ObjectExpression { properties } => self
                        .check_object_expression(
                            site,
                            &properties.into_iter().collect::<SmallVec<[_; 4]>>(),
                            target,
                            target_value,
                            relation,
                            cause,
                            use_,
                        ),
                    _ => Ok(Answer::Ready(CheckAttempt::NotApplicable)),
                }
            }
            dir::Expression::StructExpression { ty, properties } => {
                let construct_target =
                    answer!(self.select_construct_target(site, ty, Some(target))?);
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

                Ok(Answer::Ready(CheckAttempt::Checked(ValueCheck {
                    outcome: field_check,
                    target,
                })))
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
                Ok(Answer::Ready(CheckAttempt::Checked(ValueCheck {
                    outcome: selected,
                    target,
                })))
            }
            dir::Expression::New { ty, arguments } => {
                answer!(self.select_construct(
                    site,
                    ty,
                    &arguments.into_iter().collect::<SmallVec<[_; 4]>>(),
                    ConstructResult::Direct,
                    Some(target),
                )?);
                let check = ValueCheck {
                    outcome: CheckOutcome::Holds,
                    target,
                };

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
                let check = ValueCheck {
                    outcome: CheckOutcome::Holds,
                    target,
                };

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
