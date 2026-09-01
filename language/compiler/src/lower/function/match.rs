use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::operator::LoweredOperand;
use crate::{CompilerError, CompilerResult, LowerError};

/// One match arm tested as a candidate in source order.
struct Candidate {
    /// The arm pattern.
    pattern: dir::LocalNodeId<dir::Pattern>,
    /// The pattern decision selecting this arm.
    decision: dir::PatternDecision,
    /// The case index the arm selects in the dispatched variant.
    case: Option<u32>,
    /// The arm guard.
    guard: Option<dir::Condition>,
    /// The arm body.
    body: ArmBody,
    /// The block testing this candidate.
    block: mir::LocalNodeId<mir::Block>,
}

/// One match arm body.
enum ArmBody {
    /// An expression arm body.
    Expression(dir::LocalNodeId<dir::Expression>),
    /// A block arm body.
    Block(dir::LocalNodeId<dir::Block>),
}

/// One switch case routed to its block.
struct CaseBlock {
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
        let slot = self.value_slot(expression)?;
        self.lower_match_arms(value, arms, Some(slot))?;

        Ok(self.builder.local_get(slot))
    }

    /// Lower one match statement, running each arm body as statements.
    pub(in crate::lower) fn lower_match_statement(
        &mut self,
        value: dir::LocalNodeId<dir::Expression>,
        arms: &[dir::LocalNodeId<dir::MatchArm>],
    ) -> CompilerResult<bool> {
        self.lower_match_arms(value, arms, None)?;

        Ok(false)
    }

    /// Lower one match to a variant switch over source-order candidate chains.
    fn lower_match_arms(
        &mut self,
        value: dir::LocalNodeId<dir::Expression>,
        arms: &[dir::LocalNodeId<dir::MatchArm>],
        join: Option<mir::LocalNodeId<mir::Local>>,
    ) -> CompilerResult<()> {
        // evaluate the matched value once before dispatch
        let matched = self.lower_expression(value)?;
        let scrutinee = self.node_type_id(value)?;

        // dispatch newtype scrutinees on their wrapped payload
        let dispatch = match self.builder.value_type(matched) {
            Some(representation)
                if matches!(
                    self.builder.tree().get(representation),
                    mir::Type::Newtype { .. }
                ) =>
            {
                self.builder.field_get(matched, 0)
            }
            _ => matched,
        };

        // parse each arm into a source-order candidate
        let mut candidates = Vec::with_capacity(arms.len());
        for arm in arms {
            let (pattern, guard, body) = match self.source().tree().get(*arm) {
                // take an expression arm's body
                dir::MatchArm::Expression {
                    pattern,
                    guard,
                    body,
                } => (*pattern, guard.clone(), ArmBody::Expression(*body)),
                // reject a block arm where the match joins a value
                dir::MatchArm::Block { .. } if join.is_some() => {
                    return Err(LowerError::Unsupported {
                        anchor: self.lower.module.into(),
                        construct: "a block match arm".to_string(),
                    }
                    .into());
                }
                // take a block arm's body
                dir::MatchArm::Block {
                    pattern,
                    guard,
                    body,
                } => (*pattern, guard.clone(), ArmBody::Block(*body)),
            };

            // resolve the case and the block the candidate takes
            let decision = self.pattern_decision(pattern)?;
            let case = self.match_arm_case(scrutinee, &decision)?;
            let block = self.builder.block();
            candidates.push(Candidate {
                pattern,
                decision,
                case,
                guard,
                body,
                block,
            });
        }

        // route values every candidate rejects to the exhaust block
        let exhaust = self.builder.block();

        // dispatch once over the variant discriminant when any arm selects a case
        if candidates.iter().any(|candidate| candidate.case.is_some()) {
            // require a materialized variant representation for case dispatch
            let representation = self.builder.value_type(dispatch);
            let is_variant = representation.is_some_and(|representation| {
                matches!(
                    self.builder.tree().get(representation),
                    mir::Type::Variant { .. }
                )
            });
            if !is_variant {
                return Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
                    construct: "a match over an indirect scrutinee".to_string(),
                }
                .into());
            }

            // route each case to its first accepting candidate, preserving source order
            let mut targets: Vec<(u32, mir::LocalNodeId<mir::Block>)> = Vec::new();
            for candidate in &candidates {
                let Some(index) = candidate.case else {
                    continue;
                };
                if targets.iter().any(|(existing, _)| *existing == index) {
                    continue;
                }
                let accepting = candidates
                    .iter()
                    .find(|earlier| earlier.case.is_none() || earlier.case == Some(index))
                    .map_or(candidate.block, |earlier| earlier.block);
                targets.push((index, accepting));
            }

            // take the first caseless candidate as the default edge
            let default = candidates
                .iter()
                .find(|candidate| candidate.case.is_none())
                .map_or(exhaust, |candidate| candidate.block);
            self.builder
                .variant_switch(dispatch, Some(default), targets);
        }
        // otherwise run the candidate chain from the first arm
        else {
            let head = candidates
                .first()
                .map_or(exhaust, |candidate| candidate.block);
            self.builder.jump(head);
        }

        // test, bind, and run each candidate in its own block
        let exit = self.builder.block();
        for (position, candidate) in candidates.iter().enumerate() {
            self.builder.switch_to_block(candidate.block);

            // continue a failed test at the next candidate accepting the dispatched case,
            //  a caseless candidate re-dispatching over the remaining acceptors
            let fail = match candidate.case {
                Some(_) => candidates[position + 1..]
                    .iter()
                    .find(|next| next.case.is_none() || next.case == candidate.case)
                    .map_or(exhaust, |next| next.block),
                None => self.match_continuation(dispatch, &candidates, position, exhaust)?,
            };

            // read the case payload for a destructured variant arm
            let destructures = matches!(candidate.decision, dir::PatternDecision::Destructure(_));
            let input = match candidate.case {
                Some(index) if destructures => self.builder.variant_payload(dispatch, index),
                _ => matched,
            };

            // test the fields beneath a dispatched case
            if candidate.case.is_some() {
                self.lower_pattern_field_tests(&candidate.decision, input, fail)?;
            }
            // otherwise test every refutable leg of the pattern
            else {
                self.lower_pattern_tests(candidate.pattern, input, fail)?;
            }

            // bind the accepted pattern before its guard and body
            self.lower_pattern_bindings(candidate.pattern, input, dir::Mutability::Immutable)?;

            // test the guard over its bindings
            if let Some(guard) = &candidate.guard {
                let guard = guard.clone();
                let condition = self.lower_condition(&guard)?;
                let accepted = self.builder.block();
                self.builder.branch(condition, accepted, fail);
                self.builder.switch_to_block(accepted);
            }

            // run the accepted arm's body
            match candidate.body {
                // write an expression arm's value into the join slot
                ArmBody::Expression(body) => {
                    let value = self.lower_expression(body)?;
                    if let Some(slot) = join {
                        self.builder.local_set(slot, value);
                    }
                }
                // run a block arm's statements, a terminated block skips the exit jump
                ArmBody::Block(body) => {
                    if self.lower_block(body)? {
                        continue;
                    }
                }
            }

            // leave the arm at the exit block
            self.builder.jump(exit);
        }

        // continue at the exhaust block
        self.builder.switch_to_block(exhaust);

        // trap a value match, its arms cover the scrutinee
        if join.is_some() {
            self.builder.unreachable();
        }
        // fall a statement match through to the exit
        else {
            self.builder.jump(exit);
        }

        // continue lowering at the exit block
        self.builder.switch_to_block(exit);

        Ok(())
    }

    /// Build the continuation one failed caseless candidate re-dispatches through.
    fn match_continuation(
        &mut self,
        dispatch: mir::Value,
        candidates: &[Candidate],
        position: usize,
        exhaust: mir::LocalNodeId<mir::Block>,
    ) -> CompilerResult<mir::LocalNodeId<mir::Block>> {
        let rest = &candidates[position + 1..];

        // continue linearly while no later candidate selects a case
        if rest.iter().all(|next| next.case.is_none()) {
            return Ok(rest.first().map_or(exhaust, |next| next.block));
        }

        // route each remaining case to its next accepting candidate
        let mut targets: Vec<(u32, mir::LocalNodeId<mir::Block>)> = Vec::new();
        for next in rest {
            let Some(index) = next.case else {
                continue;
            };
            if targets.iter().any(|(existing, _)| *existing == index) {
                continue;
            }
            let accepting = rest
                .iter()
                .find(|earlier| earlier.case.is_none() || earlier.case == Some(index))
                .map_or(next.block, |earlier| earlier.block);
            targets.push((index, accepting));
        }
        let default = rest
            .iter()
            .find(|next| next.case.is_none())
            .map_or(exhaust, |next| next.block);

        // dispatch the remaining candidates in their own block
        let resumed = self.builder.current_block();
        let continuation = self.builder.block();
        self.builder.switch_to_block(continuation);
        self.builder
            .variant_switch(dispatch, Some(default), targets);
        self.builder.switch_to_block(resumed);

        Ok(continuation)
    }

    /// Test every refutable leg of one pattern, rejecting to the fail block.
    fn lower_pattern_tests(
        &mut self,
        pattern: dir::LocalNodeId<dir::Pattern>,
        value: mir::Value,
        fail: mir::LocalNodeId<mir::Block>,
    ) -> CompilerResult<()> {
        let decision = self.pattern_decision(pattern)?;
        match &decision {
            // accept a wildcard as it stands
            dir::PatternDecision::Ignore => Ok(()),

            // test the pattern nested under a binding
            dir::PatternDecision::Bind(binding) => match binding.pattern {
                Some(nested) => {
                    let nested = self.pattern_node(nested)?;

                    self.lower_pattern_tests(nested, value, fail)
                }
                None => Ok(()),
            },

            // accept must and default legs, the binding walk handles absence
            dir::PatternDecision::Must(_) | dir::PatternDecision::Default(_) => Ok(()),

            // test the selected predicate over the input
            dir::PatternDecision::Test(resolution) => {
                let predicate = resolution.predicate.clone();

                self.lower_predicate_test(&predicate, value, fail)
            }

            // test the selected variant's predicate over the input
            dir::PatternDecision::Variant(resolution) => {
                let predicate = resolution.predicate.clone();

                self.lower_predicate_test(&predicate, value, fail)
            }

            // project the input once, then test the nested pattern
            dir::PatternDecision::Project(resolution) => {
                let projection = resolution.projection.clone();
                let projected = self.lower_pattern_projection(&projection, value)?;
                match resolution.pattern {
                    Some(nested) => {
                        let nested = self.pattern_node(nested)?;

                        self.lower_pattern_tests(nested, projected, fail)
                    }
                    None => Ok(()),
                }
            }

            // test each destructured field's nested pattern
            dir::PatternDecision::Destructure(_) => {
                self.lower_pattern_field_tests(&decision, value, fail)
            }

            // accept the first or-branch whose tests pass
            dir::PatternDecision::Or(resolution) => {
                let branches = resolution.patterns.clone();
                let accepted = self.builder.block();
                for (position, branch) in branches.iter().enumerate() {
                    let branch = self.pattern_node(*branch)?;

                    // reject a binding or-branch, its bindings would join inconsistently
                    if !matches!(
                        self.pattern_decision(branch)?,
                        dir::PatternDecision::Ignore
                            | dir::PatternDecision::Test(_)
                            | dir::PatternDecision::Variant(_)
                    ) {
                        return Err(LowerError::Unsupported {
                            anchor: self.lower.module.into(),
                            construct: "a binding or-pattern branch".to_string(),
                        }
                        .into());
                    }

                    // continue a failed branch at the next alternative
                    let next = if position + 1 == branches.len() {
                        fail
                    } else {
                        self.builder.block()
                    };

                    // test the branch, accepting it when it passes
                    self.lower_pattern_tests(branch, value, next)?;
                    self.builder.jump(accepted);
                    if next != fail {
                        self.builder.switch_to_block(next);
                    }
                }

                // continue at the accepted block
                self.builder.switch_to_block(accepted);

                Ok(())
            }
        }
    }

    /// Test each destructured field's nested pattern, rejecting to the fail block.
    fn lower_pattern_field_tests(
        &mut self,
        decision: &dir::PatternDecision,
        value: mir::Value,
        fail: mir::LocalNodeId<mir::Block>,
    ) -> CompilerResult<()> {
        // read the fields a destructure declares
        let dir::PatternDecision::Destructure(resolution) = decision else {
            return Ok(());
        };

        // collect each nested field by destructure form
        let fields = match &**resolution {
            dir::PatternDestructureResolution::Nominal(nominal) => nominal.fields.clone(),
            dir::PatternDestructureResolution::Object(object) => object.fields.clone(),
            dir::PatternDestructureResolution::Tuple(tuple) => tuple.fields.clone(),
            dir::PatternDestructureResolution::Sequence(_) => return Ok(()),
        };

        // project each nested field once for its tests
        for field in &fields {
            let Some(nested) = field.pattern else {
                continue;
            };
            let nested = self.pattern_node(nested)?;
            let projected = self.lower_pattern_projection(&field.projection, value)?;
            self.lower_pattern_tests(nested, projected, fail)?;
        }

        Ok(())
    }

    /// Branch one selected predicate over its input, rejecting to the fail block.
    fn lower_predicate_test(
        &mut self,
        predicate: &dir::Predicate,
        value: mir::Value,
        fail: mir::LocalNodeId<mir::Block>,
    ) -> CompilerResult<()> {
        match &predicate.test {
            // test one condition over its operand
            dir::PredicateTest::Unary(test) => {
                // project the tested operand out of the input
                let input = match &test.input {
                    dir::PredicateOperand::Direct(_) => value,
                    dir::PredicateOperand::Projected(projection) => {
                        let projection = dir::OperationResolution::One((**projection).clone());

                        self.lower_pattern_projection(&projection, value)?
                    }
                };

                match &test.condition {
                    // accept every input
                    dir::PredicateCondition::Always => Ok(()),

                    // reject every input
                    dir::PredicateCondition::Never => {
                        self.builder.jump(fail);
                        let dead = self.builder.block();
                        self.builder.switch_to_block(dead);

                        Ok(())
                    }

                    // compare the input against the literal at its representation
                    dir::PredicateCondition::Literal(literal) => {
                        self.lower_literal_test(*literal, input, fail)
                    }

                    // bound the input inside the scalar interval
                    dir::PredicateCondition::Range(range) => {
                        let representation = self.value_representation(input)?;
                        let representation = self.builder.tree().get(representation).clone();

                        // test the committed lower bound
                        if let Some(start) = range.start {
                            let start = self.lower_constant(start, representation.clone())?;
                            let low = self.builder.binary(
                                mir::BinaryOperator::GreaterEqual,
                                input,
                                start,
                            );
                            let inside = self.builder.block();
                            self.builder.branch(low, inside, fail);
                            self.builder.switch_to_block(inside);
                        }

                        // test the committed upper bound
                        if let Some(end) = range.end {
                            let end = self.lower_constant(end, representation)?;
                            let operator = match range.end_bound {
                                dir::RangeEnd::Open => mir::BinaryOperator::LessThan,
                                dir::RangeEnd::Inclusive => mir::BinaryOperator::LessEqual,
                            };
                            let high = self.builder.binary(operator, input, end);
                            let accepted = self.builder.block();
                            self.builder.branch(high, accepted, fail);
                            self.builder.switch_to_block(accepted);
                        }

                        Ok(())
                    }

                    // reject a runtime type test until the dynamic representation exists
                    dir::PredicateCondition::Primitive(_)
                    | dir::PredicateCondition::Type(_)
                    | dir::PredicateCondition::Subtype(_) => Err(LowerError::Unsupported {
                        anchor: self.lower.module.into(),
                        construct: "a runtime type test pattern".to_string(),
                    }
                    .into()),
                }
            }

            // accept the first alternative whose test passes
            dir::PredicateTest::Any(alternatives) => {
                let accepted = self.builder.block();
                for (position, alternative) in alternatives.iter().enumerate() {
                    // continue a failed alternative at the next one
                    let next = if position + 1 == alternatives.len() {
                        fail
                    } else {
                        self.builder.block()
                    };

                    // test the alternative, accepting it when it passes
                    self.lower_predicate_test(alternative, value, next)?;
                    self.builder.jump(accepted);
                    if next != fail {
                        self.builder.switch_to_block(next);
                    }
                }

                // continue at the accepted block
                self.builder.switch_to_block(accepted);

                Ok(())
            }

            // reject a membership test until the dynamic representation exists
            dir::PredicateTest::Membership(_) => Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: "a membership test pattern".to_string(),
            }
            .into()),
        }
    }

    /// Branch one literal comparison over its input, rejecting to the fail block.
    fn lower_literal_test(
        &mut self,
        literal: dir::Literal,
        input: mir::Value,
        fail: mir::LocalNodeId<mir::Block>,
    ) -> CompilerResult<()> {
        let representation = self.value_representation(input)?;

        // split a variant input on its absent case for nullish literals
        if matches!(literal, dir::Literal::Undefined | dir::Literal::Null)
            && matches!(
                self.builder.tree().get(representation),
                mir::Type::Variant { .. }
            )
        {
            let Some(mir::NullishCase::Case(absent)) =
                self.builder.tree().undefined_case(representation)
            else {
                return Err(CompilerError::Internal {
                    message: "a nullish literal test without an absent case".to_string(),
                });
            };
            let accepted = self.builder.block();
            self.builder
                .variant_switch(input, Some(fail), vec![(absent, accepted)]);
            self.builder.switch_to_block(accepted);

            return Ok(());
        }

        // compare every other input against the literal constant
        let representation = self.builder.tree().get(representation).clone();
        let expected = self.lower_constant(literal, representation)?;
        let equal = self
            .builder
            .binary(mir::BinaryOperator::Equal, input, expected);
        let accepted = self.builder.block();
        self.builder.branch(equal, accepted, fail);
        self.builder.switch_to_block(accepted);

        Ok(())
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
                        message: "two different scrutinee operands across the switch cases"
                            .to_string(),
                    });
                }
                Some(_) => {}
                None => matched_operand = Some(operands[0].clone()),
            }
        }

        // lower the shared scrutinee operand once
        let matched = match &matched_operand {
            Some(operand) => Some(self.lower_operand(value, operand)?),
            None => {
                self.lower_expression(value)?;

                None
            }
        };

        // allocate every case body before building selection edges
        let exit = self.builder.block();
        let mut lowered_cases = Vec::with_capacity(cases.len());
        let mut default = None;
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
            lowered_cases.push(CaseBlock { case: *case, block });
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
                let equal = self.lower_representation_equality(
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

        // route break out of the switch
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

        // continue lowering after the switch
        self.leave_control();
        self.builder.switch_to_block(exit);

        Ok(false)
    }

    /// Collect the constant dispatch of one integer switch.
    fn constant_switch_cases(
        &mut self,
        value: dir::LocalNodeId<dir::Expression>,
        cases: &[CaseBlock],
    ) -> CompilerResult<Option<Vec<(i128, mir::LocalNodeId<mir::Block>)>>> {
        // require a runtime representation dispatching by integer identity
        let representation = self.operand_representation(value)?;
        let dir::Type::Primitive(dir::PrimitiveType::Integer(_)) = self.lower.ty(representation)?
        else {
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
            let dir::Type::Literal(dir::Literal::Integer(constant)) = self.node_type(selector)?
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
        scrutinee: dir::GlobalTypeId,
        decision: &dir::PatternDecision,
    ) -> CompilerResult<Option<u32>> {
        match decision {
            // take the default arm for wildcards and bare bindings
            dir::PatternDecision::Ignore => Ok(None),
            dir::PatternDecision::Bind(dir::PatternBindingResolution { pattern: None, .. }) => {
                Ok(None)
            }

            // select the declared representation position of enum variants
            dir::PatternDecision::Variant(resolution) => {
                let index = self
                    .lower
                    .variant_position(resolution.case.owner, resolution.case.variant)?;

                Ok(Some(index))
            }

            // select the union case the destructured nominal narrows
            dir::PatternDecision::Destructure(resolution) => match &**resolution {
                dir::PatternDestructureResolution::Nominal(nominal) => {
                    Ok(Some(self.union_member_case(scrutinee, nominal.key.symbol)?))
                }

                // run every other destructure form as a tested candidate
                _ => Ok(None),
            },

            // run every other pattern as a tested chain candidate
            dir::PatternDecision::Test(_)
            | dir::PatternDecision::Or(_)
            | dir::PatternDecision::Project(_)
            | dir::PatternDecision::Must(_)
            | dir::PatternDecision::Default(_)
            | dir::PatternDecision::Bind(_) => Ok(None),
        }
    }

    /// Return the union case position of the member one nominal narrows.
    fn union_member_case(
        &mut self,
        scrutinee: dir::GlobalTypeId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<u32> {
        // find the member declaring the narrowed nominal
        for (index, member) in self.union_members(scrutinee)?.into_iter().enumerate() {
            let member = self.lower.peel_owned(member)?;
            if let dir::Type::Application(instance) = self.lower.ty(member)?
                && instance.symbol == symbol
            {
                return Ok(index as u32);
            }
        }

        Err(CompilerError::Internal {
            message: "a destructured nominal outside the matched union".to_string(),
        })
    }
}
