use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::operator::LoweredOperand;
use crate::lower::function::place::{Place, PlaceProjection, PlaceRoot};
use crate::{CompilerError, CompilerResult};

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
#[derive(Clone, Copy)]
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
    /// Lower one match expression into a destination, each arm writing it.
    pub(in crate::lower) fn lower_match_into(
        &mut self,
        value: dir::LocalNodeId<dir::Expression>,
        arms: &[dir::LocalNodeId<dir::MatchArm>],
        destination: &Place,
    ) -> CompilerResult<bool> {
        self.lower_match_arms(value, arms, Some(destination))
    }

    /// Lower one match statement, running each arm body as statements.
    pub(in crate::lower) fn lower_match_statement(
        &mut self,
        value: dir::LocalNodeId<dir::Expression>,
        arms: &[dir::LocalNodeId<dir::MatchArm>],
    ) -> CompilerResult<bool> {
        Ok(!self.lower_match_arms(value, arms, None)?)
    }

    /// Address one scrutinee: the place it names, a reference scrutinee the place behind it.
    pub(in crate::lower) fn scrutinee_place(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Place> {
        let is_indirect = self.indirect_storage(expression)?;
        if !is_indirect && self.is_place_expression(expression) && self.names_storage(expression)? {
            return self.receiver_place(expression);
        }
        let value = self.lower_value(expression)?;
        if is_indirect {
            let declared = self.storage_type(expression)?;

            return self.reference_target(value, declared);
        }

        Ok(Place::local(self.home(value)))
    }

    /// Return the place one reference value addresses, read through its stored references.
    pub(in crate::lower) fn reference_target(
        &mut self,
        value: mir::Value,
        declared: dir::GlobalTypeId,
    ) -> CompilerResult<Place> {
        let Some((reference, _)) = self.innermost_reference(value, Some(declared))? else {
            return Ok(Place::local(self.home(value)));
        };
        let received = self.value_representation(reference)?;
        let Some(access) = self.rooted_access(received) else {
            return Ok(Place::local(self.home(reference)));
        };

        Ok(Place {
            root: PlaceRoot::Reference {
                value: reference,
                access,
            },
            path: Vec::new(),
        })
    }

    /// Project one place onto the case holding a member, a place outside a variant kept whole.
    pub(in crate::lower) fn downcast_place(
        &mut self,
        place: Place,
        union: dir::GlobalTypeId,
        member: dir::GlobalTypeId,
    ) -> CompilerResult<Place> {
        let mut projected = self.through_newtypes(place.clone())?;
        if self.place_variant(&projected)?.is_none() {
            return Ok(place);
        }
        let payload = self.lower_type(member)?;
        let members = self.lower.union_members(union)?;
        let case = self.case(&members, member)?;
        projected
            .path
            .push(PlaceProjection::Downcast { case, ty: payload });

        Ok(projected)
    }

    /// Project one place through the newtype layers wrapping its variant.
    fn through_newtypes(&mut self, mut place: Place) -> CompilerResult<Place> {
        loop {
            let ty = self.place_type(&place)?;
            let ty = self.resolved_type(ty);
            let mir::Type::Newtype { inner, .. } = *self.builder.tree().type_definition(ty) else {
                return Ok(place);
            };
            place.path.push(PlaceProjection::Field {
                field: 0,
                ty: inner,
            });
        }
    }

    /// Return the variant one place holds, when it holds one.
    pub(in crate::lower) fn place_variant(
        &mut self,
        place: &Place,
    ) -> CompilerResult<Option<mir::TypeId>> {
        let ty = self.place_type(place)?;
        let ty = self.resolved_type(ty);
        let ty = self.builder.tree_mut().storage_type(ty);

        Ok(matches!(
            self.builder.tree().type_definition(ty),
            mir::Type::Variant { .. }
        )
        .then_some(ty))
    }

    /// Switch on the case one variant place holds.
    pub(in crate::lower) fn switch_place(
        &mut self,
        place: &Place,
        default: Option<mir::LocalNodeId<mir::Block>>,
        targets: Vec<(u32, mir::LocalNodeId<mir::Block>)>,
    ) -> CompilerResult<()> {
        let Some(variant) = self.place_variant(place)? else {
            return Err(CompilerError::Internal {
                message: "a case switch over a place without a variant".to_string(),
            });
        };
        let place = place.lower(self)?;
        let tag = self.builder.variant_tag_load(place, variant);
        let default = match default {
            Some(default) => default,
            None => {
                let unreached = self.builder.block();
                let resumed = self.builder.current_block();
                self.builder.switch_to_block(unreached);
                self.builder.unreachable();
                self.builder.switch_to_block(resumed);

                unreached
            }
        };
        let targets = targets
            .into_iter()
            .map(|(case, block)| (case as i128, block))
            .collect();
        self.builder.switch(tag, default, targets);

        Ok(())
    }

    /// Branch on whether one variant place holds one case.
    pub(in crate::lower) fn branch_place_case(
        &mut self,
        place: &Place,
        case: u32,
        pass: mir::LocalNodeId<mir::Block>,
        fail: mir::LocalNodeId<mir::Block>,
    ) -> CompilerResult<()> {
        self.switch_place(place, Some(fail), vec![(case, pass)])
    }

    /// Lower one match to a variant switch over source-order candidate chains.
    fn lower_match_arms(
        &mut self,
        value: dir::LocalNodeId<dir::Expression>,
        arms: &[dir::LocalNodeId<dir::MatchArm>],
        destination: Option<&Place>,
    ) -> CompilerResult<bool> {
        // address the scrutinee once, dispatching a newtype on its wrapped payload
        let matched = self.scrutinee_place(value)?;
        let dispatch = self.through_newtypes(matched.clone())?;

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
                // take a block arm's body
                dir::MatchArm::Block {
                    pattern,
                    guard,
                    body,
                } => (*pattern, guard.clone(), ArmBody::Block(*body)),
            };

            // resolve the case and the block the candidate takes
            let decision = self.pattern_decision(pattern)?;
            let case = self.match_arm_case(&decision)?;
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
            self.switch_place(&dispatch, Some(default), targets)?;
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

            // continue a failed test at the next candidate accepting the dispatched case
            let fail = match candidate.case {
                Some(_) => candidates[position + 1..]
                    .iter()
                    .find(|next| next.case.is_none() || next.case == candidate.case)
                    .map_or(exhaust, |next| next.block),
                None => self.match_continuation(&dispatch, &candidates, position, exhaust)?,
            };

            // test the fields beneath a dispatched case
            if candidate.case.is_some() {
                self.lower_pattern_field_tests(&candidate.decision, &matched, fail)?;
            }
            // otherwise test every refutable leg of the pattern
            else {
                self.lower_pattern_tests(candidate.pattern, &matched, fail)?;
            }

            // bind the accepted pattern before its guard and body
            self.lower_pattern_bindings(candidate.pattern, &matched)?;

            // test the guard over its bindings
            if let Some(guard) = &candidate.guard {
                let guard = guard.clone();
                let condition = self.lower_condition(&guard)?;
                let accepted = self.builder.block();
                self.builder.branch(condition, accepted, fail);
                self.builder.switch_to_block(accepted);
            }

            // run the accepted arm's body into the destination, a terminated arm ending there
            let falls_through = match (candidate.body, destination) {
                (ArmBody::Expression(body), Some(destination)) => {
                    self.lower_into(body, destination)?
                }
                (ArmBody::Expression(body), None) => !self.lower_statement(body)?,
                (ArmBody::Block(body), Some(destination)) => {
                    self.lower_block_into(body, destination)?
                }
                (ArmBody::Block(body), None) => !self.lower_block(body)?,
            };
            if !falls_through {
                continue;
            }

            // leave the arm at the exit block
            self.builder.jump(exit);
        }

        // continue at the exhaust block
        self.builder.switch_to_block(exhaust);

        // trap a value match, its arms cover the scrutinee
        if destination.is_some() {
            self.builder.unreachable();
        }
        // fall a statement match through to the exit
        else {
            self.builder.jump(exit);
        }

        // continue lowering at the exit block when any arm reaches it
        let is_exited = self.builder.is_entered(exit);
        self.builder.switch_to_block(exit);

        Ok(is_exited)
    }

    /// Build the continuation one failed caseless candidate re-dispatches through.
    fn match_continuation(
        &mut self,
        dispatch: &Place,
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
        self.switch_place(dispatch, Some(default), targets)?;
        self.builder.switch_to_block(resumed);

        Ok(continuation)
    }

    /// Test every refutable leg of one pattern, rejecting to the fail block.
    fn lower_pattern_tests(
        &mut self,
        pattern: dir::LocalNodeId<dir::Pattern>,
        place: &Place,
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

                    self.lower_pattern_tests(nested, place, fail)
                }
                None => Ok(()),
            },

            // accept must and default legs, the binding walk handles absence
            dir::PatternDecision::Must(_) | dir::PatternDecision::Default(_) => Ok(()),

            // test the selected predicate over the input
            dir::PatternDecision::Test(resolution) => {
                let predicate = resolution.predicate.clone();

                self.lower_predicate_test(&predicate, place, fail)
            }

            // test the selected variant's predicate over the input
            dir::PatternDecision::Variant(resolution) => {
                let predicate = resolution.predicate.clone();

                self.lower_predicate_test(&predicate, place, fail)
            }

            // project the input once, then test the nested pattern
            dir::PatternDecision::Project(resolution) => {
                let projection = resolution.projection.clone();
                let projected = self.lower_pattern_projection(&projection, place)?;
                match resolution.pattern {
                    Some(nested) => {
                        let nested = self.pattern_node(nested)?;

                        self.lower_pattern_tests(nested, &projected, fail)
                    }
                    None => Ok(()),
                }
            }

            // test each destructured field's nested pattern
            dir::PatternDecision::Destructure(_) => {
                self.lower_pattern_field_tests(&decision, place, fail)
            }

            // accept the first or-branch whose tests pass
            dir::PatternDecision::Or(resolution) => {
                let branches = resolution.patterns.clone();
                let accepted = self.builder.block();
                for (position, branch) in branches.iter().enumerate() {
                    let branch = self.pattern_node(*branch)?;

                    // reject a binding or-branch, its bindings would join inconsistently
                    if self.pattern_binds(branch) {
                        return Err(self.unsupported("a binding or-pattern branch"));
                    }

                    // continue a failed branch at the next alternative
                    let next = if position + 1 == branches.len() {
                        fail
                    } else {
                        self.builder.block()
                    };

                    // test the branch, accepting it when it passes
                    self.lower_pattern_tests(branch, place, next)?;
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
        place: &Place,
        fail: mir::LocalNodeId<mir::Block>,
    ) -> CompilerResult<()> {
        // read the fields a destructure declares
        let dir::PatternDecision::Destructure(resolution) = decision else {
            return Ok(());
        };

        // collect each nested field by destructure form, projecting the destructured arm
        let (adjustments, fields) = match &**resolution {
            dir::PatternDestructureResolution::Nominal(nominal) => {
                (nominal.adjustments.as_slice(), &nominal.fields)
            }
            dir::PatternDestructureResolution::Object(object) => {
                (object.adjustments.as_slice(), &object.fields)
            }
            dir::PatternDestructureResolution::Tuple(tuple) => (&[][..], &tuple.fields),
            dir::PatternDestructureResolution::Sequence(_) => return Ok(()),
        };
        let place = self.project_place_adjustments(place.clone(), adjustments)?;

        // project each nested field once for its tests
        for field in fields {
            let Some(nested) = field.pattern else {
                continue;
            };
            let nested = self.pattern_node(nested)?;
            let projected = self.lower_pattern_projection(&field.projection, &place)?;
            self.lower_pattern_tests(nested, &projected, fail)?;
        }

        Ok(())
    }

    /// Branch one selected predicate over its input, rejecting to the fail block.
    fn lower_predicate_test(
        &mut self,
        predicate: &dir::Predicate,
        place: &Place,
        fail: mir::LocalNodeId<mir::Block>,
    ) -> CompilerResult<()> {
        match &predicate.test {
            // test one condition over its operand
            dir::PredicateTest::Unary(test) => {
                // project the tested operand out of the input
                let input = match &test.input {
                    dir::PredicateOperand::Direct(_) => place.clone(),
                    dir::PredicateOperand::Projected(projection) => {
                        let projection = dir::OperationResolution::One((**projection).clone());

                        self.lower_pattern_projection(&projection, place)?
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
                        self.lower_literal_test(*literal, &input, fail)
                    }

                    // bound the input inside the scalar interval
                    dir::PredicateCondition::Range(range) => {
                        let input = self.read_place(&input)?;
                        let representation = self.value_representation(input)?;

                        // test the committed lower bound
                        if let Some(start) = range.start {
                            let start = self.lower_constant(start, representation)?;
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

                    // select the union case a type test names on a variant input
                    dir::PredicateCondition::Primitive(_)
                    | dir::PredicateCondition::Type(_)
                    | dir::PredicateCondition::Subtype(_)
                        if let Some(case) =
                            self.union_case_test(&test.input, &test.condition)? =>
                    {
                        let pass = self.builder.block();
                        self.branch_place_case(&input, case, pass, fail)?;
                        self.builder.switch_to_block(pass);
                        Ok(())
                    }

                    // reject a runtime type test until the dynamic representation exists
                    dir::PredicateCondition::Primitive(_)
                    | dir::PredicateCondition::Type(_)
                    | dir::PredicateCondition::Subtype(_) => {
                        Err(self.unsupported("a runtime type test pattern"))
                    }
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
                    self.lower_predicate_test(alternative, place, next)?;
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
            dir::PredicateTest::Membership(_) => Err(self.unsupported("a membership test pattern")),
        }
    }

    /// Lower one `is` expression to the boolean its recorded predicate decides.
    pub(in crate::lower) fn lower_guard(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        let node = expression.into_global_any(self.source);
        let predicate = match self
            .lower
            .state(self.source)?
            .decisions
            .guard_decision(node)
            .cloned()
        {
            Some(dir::GuardDecision::Is(guard)) => guard.predicate,
            Some(dir::GuardDecision::InstanceOf(guard)) => guard.predicate,
            _ => {
                return Err(CompilerError::Internal {
                    message: "a guard expression without its recorded predicate".to_string(),
                });
            }
        };

        // branch on the predicate, joining the verdict in a slot
        let input = self.scrutinee_place(value)?;
        let boolean = self.lower_type(self.node_type_id(expression)?)?;
        let verdict = self.builder.local(boolean, mir::Mutability::Immutable);
        let fail = self.builder.block();
        let exit = self.builder.block();
        self.lower_predicate_test(&predicate, &input, fail)?;
        let accepted = self
            .builder
            .constant(mir::Constant::Boolean { value: true }, boolean);
        self.builder.local_set(verdict, accepted);
        self.builder.jump(exit);
        self.builder.switch_to_block(fail);
        let rejected = self
            .builder
            .constant(mir::Constant::Boolean { value: false }, boolean);
        self.builder.local_set(verdict, rejected);
        self.builder.jump(exit);
        self.builder.switch_to_block(exit);

        Ok(self.builder.local_get(verdict))
    }

    /// Branch one literal comparison over its input, rejecting to the fail block.
    fn lower_literal_test(
        &mut self,
        literal: dir::Literal,
        input: &Place,
        fail: mir::LocalNodeId<mir::Block>,
    ) -> CompilerResult<()> {
        let representation = self.place_type(input)?;

        // accept a literal against a value its single-valued type already fixes
        if self.is_singleton_representation(representation) {
            return Ok(());
        }

        // switch a variant input onto the case holding a singleton literal
        if let Some(variant) = self.place_variant(input)? {
            let singleton = match literal {
                dir::Literal::Undefined => self.builder.tree_mut().intern_type(mir::Type::Void),
                literal => self.lower.singleton_type(self.builder.tree_mut(), &literal),
            };
            let case = self.variant_case(variant, singleton)?;
            let accepted = self.builder.block();
            self.branch_place_case(input, case, accepted, fail)?;
            self.builder.switch_to_block(accepted);

            return Ok(());
        }

        // compare every other input against the literal constant
        let input = self.read_place(input)?;
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
            Some(operand) => Some(self.lower_builtin_operand(value, operand)?),
            None => {
                self.lower_value(value)?;

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
                let selected = self.lower_builtin_operand(selector, &operands[1])?;
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
        self.enter_control(None, exit, None, None);

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
    fn match_arm_case(&mut self, decision: &dir::PatternDecision) -> CompilerResult<Option<u32>> {
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

            // select the union case the destructure projects the scrutinee onto
            dir::PatternDecision::Destructure(resolution) => {
                let adjustments = match &**resolution {
                    dir::PatternDestructureResolution::Nominal(nominal) => &nominal.adjustments,
                    dir::PatternDestructureResolution::Object(object) => &object.adjustments,
                    // run every other destructure form as a tested candidate
                    _ => return Ok(None),
                };
                let case = adjustments.iter().find_map(|adjustment| match adjustment {
                    dir::ReceiverAdjustment::UnionPayload { union, arm, .. } => {
                        Some((*union, *arm))
                    }
                    _ => None,
                });
                match case {
                    Some((union, arm)) => {
                        let members = self.lower.union_members(union)?;

                        Ok(Some(self.case(&members, arm)?))
                    }
                    None => Ok(None),
                }
            }

            // run every other pattern as a tested chain candidate
            dir::PatternDecision::Test(_)
            | dir::PatternDecision::Or(_)
            | dir::PatternDecision::Project(_)
            | dir::PatternDecision::Must(_)
            | dir::PatternDecision::Default(_)
            | dir::PatternDecision::Bind(_) => Ok(None),
        }
    }

    /// Return whether one pattern introduces a binding anywhere beneath it.
    fn pattern_binds(&self, pattern: dir::LocalNodeId<dir::Pattern>) -> bool {
        let tree = self.source().tree();
        match tree.get(pattern) {
            dir::Pattern::Binding { .. } => true,
            dir::Pattern::Wildcard
            | dir::Pattern::Expression { .. }
            | dir::Pattern::Range { .. } => false,
            dir::Pattern::Must(inner)
            | dir::Pattern::Default { pattern: inner, .. }
            | dir::Pattern::BorrowOf { right: inner, .. }
            | dir::Pattern::MoveOf { right: inner, .. }
            | dir::Pattern::DereferenceOf { right: inner } => self.pattern_binds(*inner),
            dir::Pattern::Union { patterns } => {
                patterns.iter().any(|pattern| self.pattern_binds(*pattern))
            }
            dir::Pattern::Tuple { fields }
            | dir::Pattern::NominalTuple { fields, .. }
            | dir::Pattern::Sequence { fields }
            | dir::Pattern::Object { fields }
            | dir::Pattern::NominalObject { fields, .. } => fields.iter().any(|field| {
                match tree.get(*field) {
                    // bind a shorthand field at its own name
                    dir::PatternField::Named { pattern: None, .. } => true,
                    dir::PatternField::Named {
                        pattern: Some(inner),
                        ..
                    }
                    | dir::PatternField::Computed { pattern: inner, .. }
                    | dir::PatternField::Positional { pattern: inner }
                    | dir::PatternField::Rest {
                        pattern: Some(inner),
                    } => self.pattern_binds(*inner),
                    dir::PatternField::Rest { pattern: None } | dir::PatternField::Elision => false,
                }
            }),
        }
    }
}
