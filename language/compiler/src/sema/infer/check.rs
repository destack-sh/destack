use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::sema::{
    CandidateOutcome, CheckAttempt, CheckFailure, CheckOutcome, CheckState, Expectation, FlowSite,
    InferMode, PlaceUse, ValueCheck, Verdict,
};

impl CheckState<'_> {
    /// Check one expression node against an expected type.
    pub(in crate::sema) fn check_expression(
        &mut self,
        site: FlowSite,
        expectation: Expectation,
    ) -> CompilerResult<ValueCheck> {
        // convert a node checked once by its committed type
        if let Some(source) = self.own_node_type(site.node) {
            return Ok(ValueCheck {
                source,
                outcome: CheckOutcome::Holds,
                target: expectation.target,
            });
        }

        // use the contextual type before inference
        let checked = self.try_check_expression(site, expectation)?;
        if let CheckAttempt::Checked(check) = checked {
            return Ok(check);
        }

        // infer expressions without a target-directed rule
        let source =
            self.infer_node_in(site, PlaceUse::Read, expectation.mode, Some(expectation))?;

        Ok(ValueCheck {
            source,
            outcome: CheckOutcome::Holds,
            target: expectation.target,
        })
    }

    /// Try checking one expression using its target.
    fn try_check_expression(
        &mut self,
        site: FlowSite,
        expectation: Expectation,
    ) -> CompilerResult<CheckAttempt> {
        // read the checked expression and its target
        let target = expectation.target;
        let origin = site.origin();
        let node = site.node.into_typed::<dir::Expression>();
        let expression = self
            .module(node.module_id)
            .view()
            .get(node.local_id)
            .clone();

        // type a function value once as its callable
        if let dir::Expression::Declaration(declaration) = &expression
            && self.is_function_value_declaration(site.node, *declaration)?
        {
            let callable = self.function_value_type(site.node, Some(expectation))?;

            // pass a callable its context through inference barriers
            let target = self.erase_inference_barriers(expectation.target)?;
            let check = self.check_value(
                site,
                callable,
                Expectation {
                    target,
                    ..expectation
                },
            )?;

            return Ok(CheckAttempt::Checked(check));
        }

        // check by the expression kind
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
            dir::Expression::Identifier { name } => {
                self.check_reference_expression(site, name, expectation)
            }
            dir::Expression::Literal(value) => {
                let source = self.literal_type(value)?;
                self.commit_node_type(site.node, source)?;
                let check = ValueCheck {
                    source,
                    outcome: CheckOutcome::Holds,
                    target,
                };

                Ok(CheckAttempt::Checked(check))
            }
            dir::Expression::TemplateExpression { value } => {
                let keeps_template = expectation.mode == InferMode::Const
                    || self.contextualizes_template(expectation.target)?;
                let source =
                    self.template_expression_type(site, value, keeps_template, Some(expectation))?;
                self.commit_node_type(site.node, source)?;
                let check = ValueCheck {
                    source,
                    outcome: CheckOutcome::Holds,
                    target,
                };

                Ok(CheckAttempt::Checked(check))
            }
            // convert a block's tail inside the block
            dir::Expression::Block(block) => {
                let check = self.check_block(site, block, expectation)?;

                Ok(CheckAttempt::Checked(check))
            }
            dir::Expression::Const { body } => {
                self.check_transparent_expression(site, body, expectation)
            }
            dir::Expression::Chain { expression } => {
                self.check_chain_expression(site, expression, expectation)
            }
            // check a composite through the values it builds or branches into
            _ if self.is_composite_node(site.node) => self.check_composite(site, expectation),
            dir::Expression::StructExpression { ty, properties } => {
                // select the aggregate type using the expected value
                let contextual = self.construction_value(origin, target)?;
                let construct_target =
                    self.construct_type(site.origin(), site.node.module_id, ty, contextual)?;

                // reject operands that cannot name an aggregate type
                if let Some(error) =
                    self.require_aggregate_construct_target(site.node, origin, construct_target)?
                {
                    return Ok(CheckAttempt::Checked(ValueCheck {
                        source: error,
                        outcome: CheckOutcome::Holds,
                        target,
                    }));
                }

                // check fields in the expected representation
                let representation = match contextual {
                    None => construct_target,
                    Some(_) => self.replace_form_value(origin, target, construct_target)?,
                };
                let check = self.select_property_merge(
                    site,
                    &properties.into_iter().collect::<SmallVec<[_; 4]>>(),
                    Some(representation),
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
            dir::Expression::New {
                left,
                generic_arguments,
                arguments,
            } => {
                let check = self.select_construct(
                    site,
                    left,
                    &generic_arguments,
                    &arguments.into_iter().collect::<SmallVec<[_; 4]>>(),
                    Some(expectation),
                )?;

                Ok(CheckAttempt::Checked(ValueCheck { target, ..check }))
            }

            // leave every other expression to inference
            _ => Ok(CheckAttempt::NotApplicable),
        }
    }

    /// Check one composite through the members its context declares.
    fn check_composite(
        &mut self,
        site: FlowSite,
        expectation: Expectation,
    ) -> CompilerResult<CheckAttempt> {
        // infer the composite under its context, counting the failures its values report
        let checks = self.fulfill.checks.count();
        let failures = self.fulfill.failures.len();
        let source =
            self.infer_node_in(site, PlaceUse::Read, expectation.mode, Some(expectation))?;
        let is_refused = self.fulfill.failures.len() > failures
            || self
                .fulfill
                .checks
                .relation_failures_from(checks)
                .next()
                .is_some();

        // report a refused value, and join a branching node's converted values
        if is_refused || self.branching_value_nodes(site.node).is_some() {
            let outcome = match is_refused {
                true => CheckOutcome::Fails(CheckFailure::Reported),
                false => CheckOutcome::Holds,
            };

            return Ok(CheckAttempt::Checked(ValueCheck {
                source,
                outcome,
                target: expectation.target,
            }));
        }

        Ok(CheckAttempt::Checked(self.check_value(
            site,
            source,
            expectation,
        )?))
    }

    /// Check one plural reference by selecting the first overload its target accepts.
    fn check_reference_expression(
        &mut self,
        site: FlowSite,
        name: dir::StringId,
        expectation: Expectation,
    ) -> CompilerResult<CheckAttempt> {
        // read the reference node
        let node = site.node.into_typed::<dir::Expression>();

        // revisited references keep their committed value
        if self.committed_node_type(site.node).is_some() {
            return Ok(CheckAttempt::NotApplicable);
        }

        // reuse a committed resolution, else resolve the reference
        let resolution = match self
            .resolutions(node.module_id)
            .name_resolution(node.into_any())
            .cloned()
        {
            Some(resolution) => resolution,
            None => match self.decide_name_reference(node, name)? {
                Some(resolution) => resolution,
                None => return Ok(CheckAttempt::NotApplicable),
            },
        };

        // leave singular references to their target-free inference
        let symbols = resolution.symbols();
        if symbols.len() < 2 {
            return Ok(CheckAttempt::NotApplicable);
        }

        // accept the first declared overload the expected value admits
        let mut selected = None;
        for symbol in symbols.iter().copied() {
            let source = self.symbol_type(symbol)?;
            let verdict = self.decide_candidate(|state| {
                let value = state.expression_value(site, source)?;
                let conversion = state.convert_value(
                    site,
                    expectation.cause,
                    expectation.relation,
                    value,
                    expectation.target,
                    expectation.use_,
                    expectation.mode,
                )?;

                Ok(match conversion.outcome {
                    CheckOutcome::Holds | CheckOutcome::Pending => CandidateOutcome::Accepted(()),
                    CheckOutcome::Fails(_) => CandidateOutcome::Rejected(()),
                })
            })?;
            if matches!(verdict, Verdict::Holds) {
                selected = Some(symbol);

                break;
            }
        }

        // leave references no overload admits to their plural rejection
        let Some(symbol) = selected else {
            return Ok(CheckAttempt::NotApplicable);
        };

        // commit the selected declaration and convert its value
        self.infer_name_expression(site, &dir::NameResolution::new(symbol), Some(expectation))?;
        let source = self.require_node_type(site.node)?;
        let check = self.check_value(site, source, expectation)?;

        Ok(CheckAttempt::Checked(check))
    }

    /// Check one expression whose value is exactly its child value.
    pub(in crate::sema) fn check_transparent_expression(
        &mut self,
        site: FlowSite,
        child: dir::LocalNodeId<dir::Expression>,
        expectation: Expectation,
    ) -> CompilerResult<CheckAttempt> {
        // check the child and take its value as this node's value
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
