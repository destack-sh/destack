use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
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
        // the matched carrier names the case set the arms select from
        let carrier = self.coerced_type_id(value)?;
        let carrier = self.lower_type(carrier)?;
        let matched = self.lower_expression(value)?;

        // resolve each arm's case through its sealed pattern
        let mut lowered_arms = Vec::with_capacity(arms.len());
        let mut targets = Vec::new();
        let mut default = None;
        for arm in arms {
            let (pattern, guard, body) = match *self.source().tree().get(*arm) {
                dir::MatchArm::Expression {
                    pattern,
                    guard,
                    body,
                } => (pattern, guard, body),
                dir::MatchArm::Block { .. } => {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a block match arm".to_string(),
                    }
                    .into());
                }
            };

            // guards re-test at runtime beyond the case dispatch
            if guard.is_some() {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a guarded match arm".to_string(),
                }
                .into());
            }
            let selected = self.match_arm_case(pattern, carrier)?;

            // route each arm to its own block
            let block = self.builder.block();
            match selected {
                Some(index) => targets.push((index, block)),
                None if default.is_none() => default = Some(block),
                None => {
                    return Err(CompilerError::Internal {
                        message: "checked DIR admitted two irrefutable match arms".to_string(),
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

        self.builder.switch_to_block(exit);

        Ok(self.builder.local_get(slot))
    }

    /// Lower one switch statement with source-order fallthrough.
    pub(in crate::lower) fn lower_switch(
        &mut self,
        value: dir::LocalNodeId<dir::Expression>,
        cases: &[dir::LocalNodeId<dir::SwitchCase>],
    ) -> CompilerResult<bool> {
        let matched = self.lower_operand(value)?;
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
                            message: "checked DIR admitted two default switch cases".to_string(),
                        });
                    }
                }
            }
            lowered_cases.push(LoweredSwitchCase { case: *case, block });
        }

        // evaluate selectors lazily in source order
        for case in &lowered_cases {
            let selector = self.source().tree().get(case.case).selector;
            let dir::SwitchSelector::Case(selector) = selector else {
                continue;
            };
            let dir::OperatorResolution::Builtin = self.operator_resolution(case.case)? else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a protocol switch equality".to_string(),
                }
                .into());
            };
            let selected = self.lower_operand(selector)?;
            let equal = self.lower_carrier_equality(matched, selected)?;
            let next = self.builder.block();
            self.builder.branch(equal, case.block, next);
            self.builder.switch_to_block(next);
        }
        self.builder.jump(default.unwrap_or(exit));
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

    /// Lower one switch case body in statement position.
    fn lower_switch_body(
        &mut self,
        case: dir::LocalNodeId<dir::SwitchCase>,
    ) -> CompilerResult<bool> {
        let body = self.source().tree().get(case).body;

        self.lower_block(body)
    }

    /// Return the case index selected by one match arm's sealed pattern.
    fn match_arm_case(
        &mut self,
        pattern: dir::LocalNodeId<dir::Pattern>,
        carrier: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<Option<u32>> {
        match self.pattern_resolution(pattern)? {
            // wildcards and bare bindings take the default arm
            dir::PatternResolution::Ignore => Ok(None),
            dir::PatternResolution::Bind(dir::PatternBindingResolution {
                pattern: None, ..
            }) => Ok(None),

            // discriminant tests select their case by tested literal
            dir::PatternResolution::Test(resolution) => {
                let dir::PredicateTest::Unary(test) = resolution.predicate.test else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a match arm beyond a variant case test".to_string(),
                    }
                    .into());
                };
                let (
                    dir::PredicateOperand::Projected(projection),
                    dir::PredicateCondition::Literal(literal),
                ) = (test.input, test.condition)
                else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a match arm beyond a variant case test".to_string(),
                    }
                    .into());
                };
                if !matches!(projection.as_ref(), dir::Projection::VariantTag { .. }) {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a match arm beyond a variant case test".to_string(),
                    }
                    .into());
                }

                Ok(Some(self.variant_case_index(carrier, literal)?))
            }

            // payload destructures wait on the tagged payload model
            dir::PatternResolution::Destructure(_) => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a tagged payload pattern".to_string(),
            }
            .into()),

            other => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("a '{other:?}' match pattern"),
            }
            .into()),
        }
    }

    /// Return the case index carrying one tested discriminant.
    fn variant_case_index(
        &mut self,
        carrier: mir::LocalNodeId<mir::Type>,
        literal: dir::ScalarLiteral,
    ) -> CompilerResult<u32> {
        let mir::Type::Variant { cases, .. } = self.builder.tree().get(carrier) else {
            return Err(CompilerError::Internal {
                message: "checked DIR matched variant cases outside a variant carrier".to_string(),
            });
        };

        // find the case sealing this discriminant value
        let index = cases.iter().position(|case| {
            match (&case.discriminant, &literal) {
                // integer discriminants compare by value
                (mir::Constant::Int { value, .. }, dir::ScalarLiteral::Integer(tested)) => {
                    *value == *tested as i128
                }
                (mir::Constant::UInt { value, .. }, dir::ScalarLiteral::Integer(tested)) => {
                    *tested >= 0 && *value == *tested as u128
                }
                (mir::Constant::Boolean { value }, dir::ScalarLiteral::Boolean(tested)) => {
                    value == tested
                }
                _ => false,
            }
        });

        index
            .map(|index| index as u32)
            .ok_or_else(|| CompilerError::Internal {
                message: "checked DIR tested a discriminant missing from its carrier".to_string(),
            })
    }
}
