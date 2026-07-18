use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

/// One lowered match arm awaiting its body.
struct MatchArm {
    /// The arm body expression.
    body: dir::LocalNodeId<dir::Expression>,
    /// The block lowering this arm.
    block: mir::LocalNodeId<mir::Block>,
}

impl FunctionLowerer<'_, '_> {
    /// Lower one match expression to a variant switch joining its arm values.
    pub(in crate::lower) fn lower_match(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
        cases: &[dir::LocalNodeId<dir::MatchCase>],
    ) -> CompilerResult<mir::Value> {
        // the matched carrier names the case set the arms select from
        let carrier = self.lowerer.coerced_type_id(value)?;
        let carrier = self.lowerer.lower_type_id(self.builder.tree_mut(), carrier)?;
        let matched = self.lower_expression(value)?;

        // resolve each arm's case through its sealed pattern
        let mut arms = Vec::with_capacity(cases.len());
        let mut targets = Vec::new();
        let mut default = None;
        for case in cases {
            let (pattern, guard, body) = match *self.lowerer.source().tree().get(*case) {
                dir::MatchCase::Expression {
                    selector: dir::MatchSelector::Pattern { pattern, guard },
                    body,
                } => (Some(pattern), guard, body),
                dir::MatchCase::Expression {
                    selector: dir::MatchSelector::Default,
                    body,
                } => (None, None, body),
                dir::MatchCase::Block { .. } => {
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
            let selected = match pattern {
                Some(pattern) => self.match_arm_case(pattern, carrier)?,
                None => None,
            };

            // route each arm to its own block
            let block = self.builder.block();
            match selected {
                Some(index) => targets.push((index, block)),
                None if default.is_none() => default = Some(block),
                None => {
                    return Err(CompilerError::Internal {
                        message: "checked DIR admitted two default match arms".to_string(),
                    });
                }
            }
            arms.push(MatchArm { body, block });
        }

        // dispatch on the logical case and join the arm values through one slot
        let slot = self.value_slot(expression)?;
        self.builder.variant_switch(matched, default, targets);
        let exit = self.builder.block();
        for arm in &arms {
            self.builder.switch_to_block(arm.block);
            let value = self.lower_expression(arm.body)?;
            self.builder.local_set(slot, value);
            self.builder.jump(exit);
        }

        self.builder.switch_to_block(exit);

        Ok(self.builder.local_get(slot))
    }

    /// Return the case index selected by one match arm's sealed pattern.
    fn match_arm_case(
        &mut self,
        pattern: dir::LocalNodeId<dir::Pattern>,
        carrier: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<Option<u32>> {
        match self.lowerer.pattern_resolution(pattern)? {
            // wildcards and bare bindings take the default arm
            dir::PatternResolution::Ignore => Ok(None),
            dir::PatternResolution::Bind(dir::PatternBindingResolution {
                pattern: None, ..
            }) => Ok(None),

            // discriminant tests select their case by tested literal
            dir::PatternResolution::Test(resolution) => {
                let dir::PredicateTest::Unary(dir::PredicateUnaryTest {
                    input:
                        dir::PredicateOperand {
                            projection: Some(dir::Projection::VariantTag { .. }),
                            ..
                        },
                    condition: dir::PredicateCondition::Literal(literal),
                }) = resolution.predicate.test
                else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a match arm beyond a variant case test".to_string(),
                    }
                    .into());
                };

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
