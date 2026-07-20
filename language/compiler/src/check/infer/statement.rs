use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    Answer, BodyState, Cause, CauseKind, Constraint, Expectation, ExpectedType, FlowSite, Origin,
    PlaceUse, Relation, ValueUse, answer,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Infer one statement-position expression.
    pub(in crate::check) fn infer_statement(
        &mut self,
        site: FlowSite,
        statement: &dir::Expression,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node;
        let module = node.module_id;

        match statement {
            // debugger
            dir::Expression::Debugger => {
                let void = self.check.intern_type(module, dir::Type::Void)?;
                self.check.commit_node_type(node, void)?;

                Ok(Answer::Ready(()))
            }
            // let pattern = value else { ... }
            dir::Expression::LetElse {
                declarator,
                else_branch,
                ..
            } => {
                answer!(self.check_declarator(module, *declarator)?);
                let else_site = self.check.node_site(else_branch.into_global_any(module))?;
                answer!(self.attempt_node(else_site, PlaceUse::Read, None)?);
                let void = self.check.intern_type(module, dir::Type::Void)?;
                self.check.commit_node_type(node, void)?;

                Ok(Answer::Ready(()))
            }
            // let x = value
            dir::Expression::Let { declarators, .. }
            | dir::Expression::Using { declarators, .. } => {
                for declarator in declarators {
                    answer!(self.check_declarator(module, *declarator)?);
                }
                let void = self.check.intern_type(module, dir::Type::Void)?;
                self.check.commit_node_type(node, void)?;

                Ok(Answer::Ready(()))
            }
            // return value
            dir::Expression::Return { value } => {
                // relate the returned value to the body's return target
                if let Some(value) = value {
                    let value_site = self.check.node_site(value.into_global_any(module))?;
                    let expectation = self.return_type.map(|return_type| {
                        Expectation::assignable(
                            return_type,
                            self.check.intern_cause(Cause::root(
                                value_site.origin(),
                                CauseKind::Return { annotation: None },
                            )),
                            ValueUse::Output,
                        )
                    });
                    answer!(self.attempt_node(value_site, PlaceUse::Read, expectation)?);
                }
                // a bare return completes the body with void
                else if let Some(return_type) = self.return_type {
                    let void = self.check.intern_type(module, dir::Type::Void)?;
                    let cause = self.check.intern_cause(Cause::root(
                        site.origin(),
                        CauseKind::Return { annotation: None },
                    ));
                    self.check.push_constraint(Constraint::r#type(
                        Relation::Assignable,
                        void,
                        return_type,
                        cause,
                    ));
                }
                let never = self.check.intern_type(module, dir::Type::Never)?;
                self.check.commit_node_type(node, never)?;

                Ok(Answer::Ready(()))
            }
            // yield value / yield* value
            dir::Expression::Yield { cardinality, value } => {
                // relate the yielded value to the generator targets
                let generator = self.generator;
                if let Some(value) = value {
                    let value_site = self.check.node_site(value.into_global_any(module))?;
                    let target = match cardinality {
                        // delegates check against their recorded protocol
                        dir::YieldCardinality::Generator => {
                            self.check.control_results.get(&value_site.node).copied()
                        }
                        // scalar values flow to the yield target
                        dir::YieldCardinality::Scalar => generator.map(|targets| targets.yielded),
                    };
                    let expectation = target.map(|target| {
                        Expectation::assignable(
                            target,
                            self.check.intern_cause(Cause::root(
                                value_site.origin(),
                                CauseKind::Return { annotation: None },
                            )),
                            ValueUse::Output,
                        )
                    });
                    answer!(self.attempt_node(value_site, PlaceUse::Read, expectation)?);
                }
                // a bare yield produces void
                else if let Some(targets) = generator {
                    let void = self.check.intern_type(module, dir::Type::Void)?;
                    let cause = self
                        .check
                        .intern_cause(Cause::root(site.origin(), CauseKind::Expression));
                    self.check.push_constraint(Constraint::r#type(
                        Relation::Assignable,
                        void,
                        targets.yielded,
                        cause,
                    ));
                }

                // scalar yields resume, delegates evaluate to the inner return
                let output = self.check.control_results.get(&node).copied();
                let ty = match (output, generator) {
                    (Some(output), _) => output,
                    (None, Some(targets)) => targets.resumed,
                    // yields outside generators were reported by the walk
                    (None, None) => self.check.intern_type(module, dir::Type::Error)?,
                };
                self.check.commit_node_type(node, ty)?;

                Ok(Answer::Ready(()))
            }
            // while (condition) { ... }
            dir::Expression::While {
                condition, body, ..
            } => {
                answer!(self.check_condition(module, *condition)?);
                let body_site = self.check.node_site(body.into_global_any(module))?;
                answer!(self.attempt_node(body_site, PlaceUse::Read, None)?);

                // while loops complete through their false condition with void
                let void = self.check.intern_type(module, dir::Type::Void)?;
                self.check.commit_node_type(node, void)?;

                Ok(Answer::Ready(()))
            }
            // loop { ... }
            dir::Expression::Loop { body } => {
                let body_site = self.check.node_site(body.into_global_any(module))?;
                answer!(self.attempt_node(body_site, PlaceUse::Read, None)?);

                // the walk joined break values into the loop output
                let Some(result) = self.check.control_results.get(&node).copied() else {
                    return Err(CompilerError::Internal {
                        message: format!("loop {node:?} has no walked control result"),
                    });
                };
                self.check.commit_node_type(node, result)?;

                Ok(Answer::Ready(()))
            }
            // for (init; condition; increment) { ... }
            dir::Expression::For {
                initialization,
                condition,
                increment,
                body,
            } => {
                if let Some(initialization) = initialization {
                    let init_site = self
                        .check
                        .node_site(initialization.into_global_any(module))?;
                    answer!(self.attempt_node(init_site, PlaceUse::Read, None)?);
                }
                if let Some(condition) = condition {
                    answer!(self.check_condition(module, *condition)?);
                }
                if let Some(increment) = increment {
                    let increment_site = self.check.node_site(increment.into_global_any(module))?;
                    answer!(self.attempt_node(increment_site, PlaceUse::Read, None)?);
                }
                let body_site = self.check.node_site(body.into_global_any(module))?;
                answer!(self.attempt_node(body_site, PlaceUse::Read, None)?);
                let void = self.check.intern_type(module, dir::Type::Void)?;
                self.check.commit_node_type(node, void)?;

                Ok(Answer::Ready(()))
            }
            // throw value
            dir::Expression::Throw { value } => {
                let value_site = self.check.node_site(value.into_global_any(module))?;
                answer!(self.attempt_node(value_site, PlaceUse::Read, None)?);
                let never = self.check.intern_type(module, dir::Type::Never)?;
                self.check.commit_node_type(node, never)?;

                Ok(Answer::Ready(()))
            }
            // break value / continue
            dir::Expression::Break { value, .. } => {
                if let Some(value) = value {
                    let value_site = self.check.node_site(value.into_global_any(module))?;

                    // targetless break values were already rejected by the walk
                    let expectation = self
                        .check
                        .control_results
                        .get(&value_site.node)
                        .copied()
                        .map(|target| {
                            Expectation::assignable(
                                target,
                                self.check.intern_cause(Cause::root(
                                    value_site.origin(),
                                    CauseKind::Return { annotation: None },
                                )),
                                ValueUse::Output,
                            )
                        });
                    answer!(self.attempt_node(value_site, PlaceUse::Read, expectation)?);
                }
                let never = self.check.intern_type(module, dir::Type::Never)?;
                self.check.commit_node_type(node, never)?;

                Ok(Answer::Ready(()))
            }
            dir::Expression::Continue { .. } => {
                let never = self.check.intern_type(module, dir::Type::Never)?;
                self.check.commit_node_type(node, never)?;

                Ok(Answer::Ready(()))
            }
            other => Err(CompilerError::Internal {
                message: format!("cannot infer statement {node:?}: {other:?}"),
            }),
        }
    }

    /// Check one declarator at its source evaluation position.
    fn check_declarator(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::Declarator>,
    ) -> CompilerResult<Answer<()>> {
        let declarator = self.module(module).view().get(id).clone();
        let pattern = declarator.pattern;

        // resolve the written type before checking the optional value
        let annotation = declarator.ty.map(|ty| ty.into_global_any(module));
        let written = annotation
            .map(|annotation| self.require_node_type(annotation))
            .transpose()?;
        let target = match (declarator.value, written) {
            (Some(value), Some(written)) => {
                let site = self.node_site(value.into_global_any(module))?;
                let cause = self.intern_cause(Cause::root(
                    site.origin(),
                    CauseKind::Initializer { annotation },
                ));
                let expectation = Expectation {
                    expected: ExpectedType::Type(written),
                    relation: Relation::Assignable,
                    cause,
                    use_: ValueUse::Store,
                };
                answer!(self.attempt_node(site, PlaceUse::Read, Some(expectation))?);

                Some(written)
            }
            (Some(value), None) => {
                let site = self.node_site(value.into_global_any(module))?;

                Some(answer!(self.infer_node_type(site, PlaceUse::Read)?))
            }
            (None, Some(written)) => Some(written),
            (None, None) => None,
        };

        // check the pattern against the destructured value
        if let Some(target) = target {
            let site = self.node_site(pattern.into_global_any(module))?;
            answer!(self.check_pattern(
                pattern.into_global(module),
                site.flow,
                site.scope,
                target,
            )?);
        }

        Ok(Answer::Ready(()))
    }

    /// Check every expression operand of one condition.
    pub(in crate::check) fn check_condition_operands(
        &mut self,
        module: ModuleId,
        condition: &dir::Condition,
    ) -> CompilerResult<Answer<()>> {
        for operand in &condition.operands {
            match operand {
                dir::ConditionOperand::Expression { condition } => {
                    answer!(self.check_condition(module, *condition)?);
                }
                dir::ConditionOperand::Binding { declarator, .. } => {
                    answer!(self.check_declarator(module, *declarator)?);
                }
            }
        }

        Ok(Answer::Ready(()))
    }

    /// Check one loop or branch condition against boolean.
    fn check_condition(
        &mut self,
        module: ModuleId,
        condition: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let site = self.check.node_site(condition.into_global_any(module))?;
        let boolean = self
            .check
            .intern_type(module, dir::Type::Primitive(dir::PrimitiveType::Boolean))?;
        let expectation = Expectation {
            expected: ExpectedType::Type(boolean),
            relation: Relation::Assignable,
            cause: self.check.intern_cause(Cause::root(
                Origin::Node(condition.into_global_any(module), site.scope),
                CauseKind::Expression,
            )),
            use_: ValueUse::Condition,
        };
        answer!(self.attempt_node(site, PlaceUse::Read, Some(expectation))?);

        Ok(Answer::Ready(()))
    }
}
