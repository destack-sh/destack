use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    Answer, BodyState, Constraint, Expectation, ExpectedType, FlowSite, Origin, PlaceUse, Relation,
    ValueUse,
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
            // let pattern = value else { ... }
            dir::Expression::LetElse { else_branch, .. } => {
                // declarators check through their recorded bodies
                let else_site = self.check.node_site(else_branch.into_global_any(module))?;
                self.check_node(else_site, PlaceUse::Read, None)?;
                let void = self.check.intern_type(module, dir::Type::Void)?;
                self.check.commit_node_type(node, void)?;

                Ok(Answer::Ready(()))
            }
            // let x = value
            dir::Expression::Let { .. } => {
                // declarators check through their recorded bodies
                let void = self.check.intern_type(module, dir::Type::Void)?;
                self.check.commit_node_type(node, void)?;

                Ok(Answer::Ready(()))
            }
            // return value
            dir::Expression::Return { value } => {
                // relate the returned value to the body's return target
                if let Some(value) = value {
                    let value_site = self.check.node_site(value.into_global_any(module))?;
                    let expectation = self.ret.map(|ret| {
                        Expectation::assignable(ret, value_site.origin(), ValueUse::Output)
                    });
                    self.check_node(value_site, PlaceUse::Read, expectation)?;
                }
                // a bare return completes the body with void
                else if let Some(ret) = self.ret {
                    let void = self.check.intern_type(module, dir::Type::Void)?;
                    let origin = self.check.intern_origin(site.origin());
                    self.check.push_constraint(Constraint::r#type(
                        Relation::Assignable,
                        void,
                        ret,
                        origin,
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
                        Expectation::assignable(target, value_site.origin(), ValueUse::Output)
                    });
                    self.check_node(value_site, PlaceUse::Read, expectation)?;
                }
                // a bare yield produces void
                else if let Some(targets) = generator {
                    let void = self.check.intern_type(module, dir::Type::Void)?;
                    let origin = self.check.intern_origin(site.origin());
                    self.check.push_constraint(Constraint::r#type(
                        Relation::Assignable,
                        void,
                        targets.yielded,
                        origin,
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
                self.check_condition(module, *condition)?;
                let body_site = self.check.node_site(body.into_global_any(module))?;
                self.check_node(body_site, PlaceUse::Read, None)?;

                // while loops complete through their false condition with void
                let void = self.check.intern_type(module, dir::Type::Void)?;
                self.check.commit_node_type(node, void)?;

                Ok(Answer::Ready(()))
            }
            // loop { ... }
            dir::Expression::Loop { body } => {
                let body_site = self.check.node_site(body.into_global_any(module))?;
                self.check_node(body_site, PlaceUse::Read, None)?;

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
                    self.check_node(init_site, PlaceUse::Read, None)?;
                }
                if let Some(condition) = condition {
                    self.check_condition(module, *condition)?;
                }
                if let Some(increment) = increment {
                    let increment_site = self.check.node_site(increment.into_global_any(module))?;
                    self.check_node(increment_site, PlaceUse::Read, None)?;
                }
                let body_site = self.check.node_site(body.into_global_any(module))?;
                self.check_node(body_site, PlaceUse::Read, None)?;
                let void = self.check.intern_type(module, dir::Type::Void)?;
                self.check.commit_node_type(node, void)?;

                Ok(Answer::Ready(()))
            }
            // throw value
            dir::Expression::Throw { value } => {
                let value_site = self.check.node_site(value.into_global_any(module))?;
                self.check_node(value_site, PlaceUse::Read, None)?;
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
                            Expectation::assignable(target, value_site.origin(), ValueUse::Output)
                        });
                    self.check_node(value_site, PlaceUse::Read, expectation)?;
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

    /// Check every expression operand of one condition.
    pub(in crate::check) fn check_condition_operands(
        &mut self,
        module: ModuleId,
        condition: &dir::Condition,
    ) -> CompilerResult<Answer<()>> {
        for operand in &condition.operands {
            if let dir::ConditionOperand::Expression { condition } = operand {
                self.check_condition(module, *condition)?;
            }
        }

        Ok(Answer::Ready(()))
    }

    /// Check one loop or branch condition against boolean.
    fn check_condition(
        &mut self,
        module: ModuleId,
        condition: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let site = self.check.node_site(condition.into_global_any(module))?;
        let boolean = self
            .check
            .intern_type(module, dir::Type::Primitive(dir::PrimitiveType::Boolean))?;
        let expectation = Expectation {
            expected: ExpectedType::Type(boolean),
            relation: Relation::Assignable,
            origin: Origin::Node(condition.into_global_any(module), site.scope),
            use_: ValueUse::Condition,
        };
        self.check_node(site, PlaceUse::Read, Some(expectation))?;

        Ok(())
    }
}
