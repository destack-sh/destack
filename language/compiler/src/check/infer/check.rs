use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    BodyState, CandidateOutcome, CandidateVerdict, CheckAttempt, CheckOutcome, Expectation,
    FlowSite, InferMode, PlaceUse, Relation, ValueCheck,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Check one expression node against an expected type.
    pub(in crate::check) fn check_expression(
        &mut self,
        site: FlowSite,
        expectation: Expectation,
    ) -> CompilerResult<ValueCheck> {
        // record the written expectation for tools and lowering
        self.check
            .expected_types
            .insert(site.node, expectation.target);

        // use the contextual type before inference
        let checked = self.try_check_expression(site, expectation)?;
        if let CheckAttempt::Checked(check) = checked {
            return Ok(check);
        }

        // select one contextual union member when exactly one applies
        if expectation.relation.distributes_over_union_target()
            && let Some(check) = self.check_union_target(site, expectation)?
        {
            return Ok(check);
        }

        // infer expressions without a target-directed rule
        let source = self.infer_node(site, PlaceUse::Read, expectation.mode)?;

        Ok(ValueCheck {
            source,
            outcome: CheckOutcome::Holds,
            target: expectation.target,
        })
    }

    /// Check one expression against one uniquely applicable union member.
    fn check_union_target(
        &mut self,
        site: FlowSite,
        expectation: Expectation,
    ) -> CompilerResult<Option<ValueCheck>> {
        let origin = site.origin();
        let Some(targets) = self.check.union_leaves(origin, expectation.target)? else {
            return Ok(None);
        };

        // track the best candidate and the best undecided fallback
        let mut viable = None;
        let mut is_viable_ambiguous = false;
        let mut indeterminate = None;
        let mut is_indeterminate_ambiguous = false;

        // classify every member without retaining speculative state
        for target in targets {
            let mode = self.contextual_literal_mode(site.origin(), target, expectation.mode)?;
            let candidate = Expectation {
                target,
                mode,
                ..expectation
            };
            let verdict = self.probe_candidate(|state| {
                let checked = state.try_check_expression(site, candidate)?;
                let outcome = match checked {
                    CheckAttempt::Checked(check) if check.outcome == CheckOutcome::Holds => {
                        let source = state.flow_type_at(site, check.source)?;
                        let value = state.expression_value(site, source)?;
                        let conversion = state.convert_value(
                            site,
                            candidate.cause,
                            candidate.relation,
                            value,
                            candidate.target,
                            candidate.use_,
                            candidate.mode,
                        )?;

                        match conversion.outcome {
                            CheckOutcome::Holds => CandidateOutcome::Accepted(()),
                            CheckOutcome::Fails(_) => CandidateOutcome::Rejected(()),
                        }
                    }
                    CheckAttempt::Checked(_) | CheckAttempt::NotApplicable => {
                        CandidateOutcome::Rejected(())
                    }
                };

                Ok(outcome)
            })?;

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
            return Ok(None);
        };

        // confirm the selected member in the owning state
        let mode = self.contextual_literal_mode(site.origin(), target, expectation.mode)?;
        let candidate = Expectation {
            target,
            mode,
            ..expectation
        };
        let checked = self.try_check_expression(site, candidate)?;
        let CheckAttempt::Checked(check) = checked else {
            return Err(CompilerError::Internal {
                message: "confirmed contextual union member became inapplicable".to_string(),
            });
        };
        let check = ValueCheck {
            source: check.source,
            outcome: check.outcome,
            target: expectation.target,
        };

        Ok(Some(check))
    }

    /// Try checking one expression using its target.
    fn try_check_expression(
        &mut self,
        site: FlowSite,
        expectation: Expectation,
    ) -> CompilerResult<CheckAttempt> {
        let target = expectation.target;
        let origin = site.origin();
        let node = site.node.into_typed::<dir::Expression>();
        let expression = self
            .module(node.module_id)
            .view()
            .get(node.local_id)
            .clone();

        // deduce function value parameters from a callable target
        if let dir::Expression::Declaration(declaration) = &expression
            && self.register_function_value(site.node, *declaration)?
        {
            let check = self.check_function_value(site, Some(expectation), InferMode::Exact)?;

            return Ok(CheckAttempt::Checked(check));
        }
        if self.check.lambdas.contains_key(&site.node) {
            let check = self.check_function_value(site, Some(expectation), InferMode::Exact)?;

            return Ok(CheckAttempt::Checked(check));
        }

        match expression {
            dir::Expression::TreeExpression { .. } => {
                let source = self.check_tree_expression(site, Some(&expectation))?;
                let check = ValueCheck {
                    source,
                    outcome: CheckOutcome::Holds,
                    target,
                };

                Ok(CheckAttempt::Checked(check))
            }
            dir::Expression::ScalarLiteral(value) => {
                let source = self.scalar_literal_type(node, value)?;
                self.commit_node_type(site.node, source)?;
                let check = ValueCheck {
                    source,
                    outcome: CheckOutcome::Holds,
                    target,
                };

                Ok(CheckAttempt::Checked(check))
            }
            dir::Expression::TemplateExpression { value } => {
                let source = self.template_expression_type(site, value)?;
                self.commit_node_type(site.node, source)?;
                let check = ValueCheck {
                    source,
                    outcome: CheckOutcome::Holds,
                    target,
                };

                Ok(CheckAttempt::Checked(check))
            }
            dir::Expression::Block(block) => {
                let check = self.check_block(site, block, expectation)?;

                Ok(CheckAttempt::Checked(check))
            }
            dir::Expression::Comptime { body } => {
                self.check_transparent_expression(site, body, expectation)
            }
            dir::Expression::Satisfies { expression, .. } => {
                self.check_transparent_expression(site, expression, expectation)
            }
            dir::Expression::If {
                condition,
                then_expression,
                else_expression,
                ..
            } => {
                self.check_condition_operands(site.node.module_id, &condition)?;

                self.check_if_expression(
                    site,
                    &condition,
                    then_expression,
                    else_expression,
                    expectation,
                )
            }
            dir::Expression::Match { value, arms } => self.check_match_expression(
                site,
                value,
                &arms.into_iter().collect::<SmallVec<[_; 4]>>(),
                expectation,
            ),
            expression @ (dir::Expression::ArrayExpression { .. }
            | dir::Expression::FixedArrayExpression { .. }
            | dir::Expression::TupleExpression { .. }
            | dir::Expression::ObjectExpression { .. }) => {
                // resolve the target head before matching structural literals
                let Some(target_value) = self.construction_value(origin, target)? else {
                    return Ok(CheckAttempt::NotApplicable);
                };

                match expression {
                    dir::Expression::ArrayExpression { elements } => self.check_array_expression(
                        site,
                        &elements.into_iter().collect::<SmallVec<[_; 4]>>(),
                        target,
                        target_value,
                        expectation,
                    ),
                    dir::Expression::FixedArrayExpression { value, length } => self
                        .check_fixed_array_expression(
                            site,
                            value,
                            length,
                            target,
                            target_value,
                            expectation,
                        ),
                    dir::Expression::TupleExpression { elements } => self.check_tuple_expression(
                        site,
                        &elements.into_iter().collect::<SmallVec<[_; 4]>>(),
                        target,
                        target_value,
                        expectation,
                    ),
                    dir::Expression::ObjectExpression { properties } => self
                        .check_object_expression(
                            site,
                            &properties.into_iter().collect::<SmallVec<[_; 4]>>(),
                            target_value,
                            expectation,
                        ),
                    _ => Ok(CheckAttempt::NotApplicable),
                }
            }
            dir::Expression::StructExpression { ty, properties } => {
                let contextual = self.construction_value(origin, target)?;
                let construct_target = self.select_construct_target(site, ty, contextual)?;
                let carrier = match (expectation.relation, contextual) {
                    (Relation::Satisfies, _) | (_, None) => construct_target,
                    (_, Some(_)) => self.replace_form_value(origin, target, construct_target)?,
                };
                let check = self.select_property_merge(
                    site,
                    &properties.into_iter().collect::<SmallVec<[_; 4]>>(),
                    Some(carrier),
                )?;

                Ok(CheckAttempt::Checked(ValueCheck {
                    source: check.source,
                    outcome: check.outcome,
                    target,
                }))
            }
            dir::Expression::Call {
                left,
                generic_arguments,
                arguments,
                is_optional,
                ..
            } => {
                let check = self.select_call(
                    site,
                    left,
                    &generic_arguments.into_iter().collect::<SmallVec<[_; 2]>>(),
                    &arguments.into_iter().collect::<SmallVec<[_; 4]>>(),
                    is_optional,
                    Some(expectation),
                )?;
                Ok(CheckAttempt::Checked(ValueCheck {
                    source: check.source,
                    outcome: check.outcome,
                    target,
                }))
            }
            dir::Expression::New { ty, arguments } => {
                let source = self.select_construct(
                    site,
                    ty,
                    &arguments.into_iter().collect::<SmallVec<[_; 4]>>(),
                    Some(expectation),
                )?;
                let check = ValueCheck {
                    source,
                    outcome: CheckOutcome::Holds,
                    target,
                };

                Ok(CheckAttempt::Checked(check))
            }

            _ => Ok(CheckAttempt::NotApplicable),
        }
    }

    /// Check one expression whose value is exactly its child value.
    pub(in crate::check) fn check_transparent_expression(
        &mut self,
        site: FlowSite,
        child: dir::LocalNodeId<dir::Expression>,
        expectation: Expectation,
    ) -> CompilerResult<CheckAttempt> {
        let module = site.node.module_id;
        let child_site = self.visit_site(child.into_global_any(module))?;
        let check = self.check_node(child_site, expectation)?;
        self.commit_node_type(site.node, check.source)?;
        let check = ValueCheck {
            source: check.source,
            outcome: check.outcome,
            target: check.target,
        };

        Ok(CheckAttempt::Checked(check))
    }
}
