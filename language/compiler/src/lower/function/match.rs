use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::equality::LoweredOperand;
use crate::{CompilerError, CompilerResult, LowerError};

/// One lowered match arm awaiting its body.
struct LoweredMatchArm {
    /// The arm body expression.
    body: dir::LocalNodeId<dir::Expression>,
    /// The block lowering this arm.
    block: mir::LocalNodeId<mir::Block>,
}

/// One lowered switch case awaiting its body.
struct LoweredSwitchCase {
    /// The source switch case.
    case: dir::LocalNodeId<dir::SwitchCase>,
    /// The block lowering this case.
    block: mir::LocalNodeId<mir::Block>,
}

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one match expression to a variant switch joining its arm values.
    pub(in crate::lower) fn lower_match(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
        arms: &[dir::LocalNodeId<dir::MatchArm>],
    ) -> CompilerResult<mir::Value> {
        // evaluate the matched value once before dispatch
        let matched = self.lower_expression(value)?;

        // resolve each arm's case through its pattern
        let mut lowered_arms = Vec::with_capacity(arms.len());
        let mut targets = Vec::new();
        let mut default = None;
        for arm in arms {
            let (pattern, is_guarded, body) = match self.source().tree().get(*arm) {
                dir::MatchArm::Expression {
                    pattern,
                    guard,
                    body,
                } => (*pattern, guard.is_some(), *body),
                dir::MatchArm::Block { .. } => {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a block match arm".to_string(),
                    }
                    .into());
                }
            };

            // reject guarded arms
            if is_guarded {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a guarded match arm".to_string(),
                }
                .into());
            }
            let selected = self.match_arm_case(pattern)?;

            // route each arm to its own block
            let block = self.builder.block();
            match selected {
                Some(index) => targets.push((index, block)),
                None if default.is_none() => default = Some(block),
                None => {
                    return Err(CompilerError::Internal {
                        message: "two irrefutable match arms".to_string(),
                    });
                }
            }
            lowered_arms.push(LoweredMatchArm { body, block });
        }

        // dispatch on the logical case and join the arm values through one slot
        let slot = self.value_slot(expression)?;
        self.builder.variant_switch(matched, default, targets);
        let exit = self.builder.block();
        for arm in &lowered_arms {
            self.builder.switch_to_block(arm.block);
            let value = self.lower_expression(arm.body)?;
            self.builder.local_set(slot, value);
            self.builder.jump(exit);
        }

        // read the joined value out of the exit block
        self.builder.switch_to_block(exit);

        Ok(self.builder.local_get(slot))
    }

    /// Lower one switch statement with source-order fallthrough.
    pub(in crate::lower) fn lower_switch(
        &mut self,
        value: dir::LocalNodeId<dir::Expression>,
        cases: &[dir::LocalNodeId<dir::SwitchCase>],
    ) -> CompilerResult<bool> {
        // require one shared scrutinee operand across every case
        let mut matched_operand = None;
        for case in cases {
            if !matches!(
                self.source().tree().get(*case).selector,
                dir::SwitchSelector::Case(_)
            ) {
                continue;
            }
            let resolution = self.operator_decision(*case)?;
            let dir::OperationResolution::One(dir::OperatorApplication::Binary {
                operator: dir::BinaryOperator::EqualStrict,
                target: dir::OperatorTarget::Builtin(operands),
                ..
            }) = resolution
            else {
                return Err(CompilerError::Internal {
                    message: "a switch case with a non-builtin equality resolution".to_string(),
                });
            };
            match &matched_operand {
                Some(selected) if selected != &operands[0] => {
                    return Err(CompilerError::Internal {
                        message: "switch cases selected different scrutinee operands".to_string(),
                    });
                }
                Some(_) => {}
                None => matched_operand = Some(operands[0].clone()),
            }
        }
        let matched = match &matched_operand {
            Some(operand) => Some(self.lower_operand(value, operand)?),
            None => {
                self.lower_expression(value)?;

                None
            }
        };
        let exit = self.builder.block();
        let mut lowered_cases = Vec::with_capacity(cases.len());
        let mut default = None;

        // allocate every case body before building selection edges
        for case in cases {
            let block = self.builder.block();
            let selector = self.source().tree().get(*case).selector;
            match selector {
                dir::SwitchSelector::Case(_) => {}
                dir::SwitchSelector::Default => {
                    if default.replace(block).is_some() {
                        return Err(CompilerError::Internal {
                            message: "two default switch cases".to_string(),
                        });
                    }
                }
            }
            lowered_cases.push(LoweredSwitchCase { case: *case, block });
        }

        // dispatch constant integer selectors through one switch terminator
        let constant = match matched {
            Some(LoweredOperand::Scalar { value: operand, .. }) => self
                .constant_switch_cases(value, &lowered_cases)?
                .map(|cases| (operand, cases)),
            _ => None,
        };
        if let Some((operand, cases)) = constant {
            self.builder.switch(operand, default.unwrap_or(exit), cases);
        } else {
            // evaluate selectors lazily in source order
            for case in &lowered_cases {
                let selector = self.source().tree().get(case.case).selector;
                let dir::SwitchSelector::Case(selector) = selector else {
                    continue;
                };
                let resolution = self.operator_decision(case.case)?;
                let dir::OperationResolution::One(dir::OperatorApplication::Binary {
                    operator: dir::BinaryOperator::EqualStrict,
                    target: dir::OperatorTarget::Builtin(operands),
                    ..
                }) = resolution
                else {
                    return Err(CompilerError::Internal {
                        message: "a switch case with a non-builtin equality resolution".to_string(),
                    });
                };
                let selected = self.lower_operand(selector, &operands[1])?;
                let equal = self.lower_carrier_equality(
                    matched.ok_or_else(|| CompilerError::Internal {
                        message: "a switch case without a scrutinee operand".to_string(),
                    })?,
                    selected,
                )?;
                let next = self.builder.block();
                self.builder.branch(equal, case.block, next);
                self.builder.switch_to_block(next);
            }
            self.builder.jump(default.unwrap_or(exit));
        }
        self.enter_control(None, exit, None);

        // preserve source-order fallthrough between adjacent case bodies
        for (index, case) in lowered_cases.iter().enumerate() {
            self.builder.switch_to_block(case.block);
            let terminated = self.lower_switch_body(case.case)?;
            if !terminated {
                let next = lowered_cases.get(index + 1).map_or(exit, |case| case.block);
                self.builder.jump(next);
            }
        }
        self.leave_control();
        self.builder.switch_to_block(exit);

        Ok(false)
    }

    /// Collect the constant dispatch of one integer switch.
    fn constant_switch_cases(
        &mut self,
        value: dir::LocalNodeId<dir::Expression>,
        cases: &[LoweredSwitchCase],
    ) -> CompilerResult<Option<Vec<(i128, mir::LocalNodeId<mir::Block>)>>> {
        // require a runtime carrier dispatching by integer identity
        let carrier = self.operand_carrier(value)?;
        let dir::Type::Primitive(dir::PrimitiveType::Integer(_)) = self.lowerer.ty(carrier)? else {
            return Ok(None);
        };

        // read the constant selector of every non-default case
        let mut selected = Vec::with_capacity(cases.len());
        for case in cases {
            let selector = self.source().tree().get(case.case).selector;
            let dir::SwitchSelector::Case(selector) = selector else {
                continue;
            };
            if !self.operator_decision(case.case)?.is_builtin() {
                return Ok(None);
            }
            let dir::Type::Literal(dir::ScalarLiteral::Integer(constant)) =
                self.node_type(selector)?
            else {
                return Ok(None);
            };

            // keep the first matching case
            let constant = i128::from(constant);
            if selected.iter().any(|(existing, _)| *existing == constant) {
                continue;
            }
            selected.push((constant, case.block));
        }

        Ok(Some(selected))
    }

    /// Lower one switch case body in statement position.
    fn lower_switch_body(
        &mut self,
        case: dir::LocalNodeId<dir::SwitchCase>,
    ) -> CompilerResult<bool> {
        let body = self.source().tree().get(case).body;

        self.lower_block(body)
    }

    /// Return the case index selected by one match arm's pattern.
    fn match_arm_case(
        &mut self,
        pattern: dir::LocalNodeId<dir::Pattern>,
    ) -> CompilerResult<Option<u32>> {
        match self.pattern_decision(pattern)? {
            // take the default arm for wildcards and bare bindings
            dir::PatternDecision::Ignore => Ok(None),
            dir::PatternDecision::Bind(dir::PatternBindingResolution { pattern: None, .. }) => {
                Ok(None)
            }

            // select the declared carrier position of enum variants
            dir::PatternDecision::Variant(resolution) => {
                let index = self
                    .lowerer
                    .variant_position(resolution.case.owner, resolution.case.variant)?;

                Ok(Some(index))
            }

            other => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("a '{other:?}' match pattern"),
            }
            .into()),
        }
    }
}
