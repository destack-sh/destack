use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, BodyState, Cause, CauseKind, CheckAttempt, CheckOutcome, Expectation, FlowSite,
    ForInSourceObligation, InferMode, Obligation, Origin, PlaceUse, Relation, StaticPresence,
    ValueCheck, ValueUse, answer,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Infer one optional chain from its accesses.
    pub(in crate::check) fn infer_chain_expression(
        &mut self,
        site: FlowSite,
        inner: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node;
        let module = node.module_id;
        let inner_site = self.node_site(inner.into_global_any(module))?;
        let ty = answer!(self.infer_node_type(inner_site, PlaceUse::Read)?);

        // add undefined where the chain can short circuit
        let result = match answer!(self.chain_short_circuits(site.origin(), module, inner)?) {
            true => {
                let undefined = self.check.intern_type(dir::Type::Undefined)?;
                self.check.normalized_union_type([ty, undefined])?
            }
            false => ty,
        };
        self.check.commit_node_type(node, result)?;
        self.check.commit_chain_access(node)?;

        Ok(Answer::Ready(()))
    }

    /// Return whether one optional chain drops a nullish receiver.
    fn chain_short_circuits(
        &mut self,
        origin: Origin,
        module: ModuleId,
        inner: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<bool>> {
        let mut current = inner;
        loop {
            let (left, is_optional) = match self.module(module).view().get(current) {
                dir::Expression::Member {
                    left, is_optional, ..
                }
                | dir::Expression::Index {
                    left, is_optional, ..
                }
                | dir::Expression::Call {
                    left, is_optional, ..
                } => (*left, *is_optional),
                dir::Expression::Instantiation { left, .. }
                | dir::Expression::Maybe { left, .. }
                | dir::Expression::Must { left, .. } => (*left, false),
                _ => return Ok(Answer::Ready(false)),
            };

            if is_optional {
                let receiver = self.require_node_type(left.into_global_any(module))?;
                if answer!(self.split_nullish_type(origin, receiver)?).is_some() {
                    return Ok(Answer::Ready(true));
                }
            }
            current = left;
        }
    }

    /// Infer one if expression from its branches.
    pub(in crate::check) fn infer_if_expression(
        &mut self,
        site: FlowSite,
        then_expression: dir::LocalNodeId<dir::Expression>,
        else_expression: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let then_site = self.node_site(then_expression.into_global_any(module))?;
        let then_type = answer!(self.infer_node_type(then_site, PlaceUse::Read)?);
        let result = if let Some(else_expression) = else_expression {
            let else_site = self.node_site(else_expression.into_global_any(module))?;
            let else_type = answer!(self.infer_node_type(else_site, PlaceUse::Read)?);
            self.normalized_union_type([then_type, else_type])?
        } else {
            let void = self.intern_type(dir::Type::Void)?;
            self.normalized_union_type([then_type, void])?
        };
        self.commit_node_type(node.into_any(), result)?;

        Ok(Answer::Ready(()))
    }

    /// Check one if expression under an expected result type.
    pub(in crate::check) fn check_if_expression(
        &mut self,
        site: FlowSite,
        then_expression: dir::LocalNodeId<dir::Expression>,
        else_expression: Option<dir::LocalNodeId<dir::Expression>>,
        expectation: Expectation,
    ) -> CompilerResult<Answer<CheckAttempt>> {
        let module = site.node.module_id;
        let target = expectation.target;
        let relation = expectation.relation;

        // check the then branch against the incoming expectation
        let then_site = self.node_site(then_expression.into_global_any(module))?;
        let then_check = answer!(self.check_node(then_site, expectation)?);
        let then_type = then_check.source;
        let mut check = then_check.outcome;

        // check an else branch, or make the missing branch explicit as void
        let result = if let Some(else_expression) = else_expression {
            let else_site = self.node_site(else_expression.into_global_any(module))?;
            let else_check = answer!(self.check_node(else_site, expectation)?);
            let else_type = else_check.source;
            check = check.and(else_check.outcome);

            // use the target when both branches hold and produce a value
            let joined = self.normalized_union_type([then_type, else_type])?;
            match (relation, check) {
                (Relation::Assignable, CheckOutcome::Holds)
                    if !matches!(self.check.ty(joined)?, dir::Type::Never) =>
                {
                    target
                }
                _ => joined,
            }
        } else {
            let void = self.intern_type(dir::Type::Void)?;

            self.normalized_union_type([then_type, void])?
        };
        self.commit_node_type(site.node, result)?;

        Ok(Answer::Ready(CheckAttempt::Checked(ValueCheck {
            source: result,
            outcome: check,
            target,
        })))
    }

    /// Infer one try expression from its body and catch branches.
    pub(in crate::check) fn infer_try_expression(
        &mut self,
        site: FlowSite,
        body: dir::LocalNodeId<dir::Expression>,
        catch: Option<dir::LocalNodeId<dir::Catch>>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let body_site = self.node_site(body.into_global_any(module))?;
        let body_type = answer!(self.infer_node_type(body_site, PlaceUse::Read)?);

        // read the catch result recorded when the handler was checked
        let catch_type = catch.and_then(|catch| {
            self.check
                .catch_results
                .get(&catch.into_global_any(module))
                .copied()
        });
        let result = match catch_type {
            Some(catch_type) => self.normalized_union_type([body_type, catch_type])?,
            None => body_type,
        };
        self.commit_node_type(node.into_any(), result)?;

        Ok(Answer::Ready(()))
    }

    /// Infer one match expression from its arm values.
    pub(in crate::check) fn infer_match_expression(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        arms: &[dir::LocalNodeId<dir::MatchArm>],
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let arms = self.present_match_arms(module, arms)?;
        answer!(self.infer_match_patterns(module, value, &arms)?);

        // join every arm body into the match value
        let mut values = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for arm in &arms {
            values.push(answer!(self.infer_match_arm_body(module, *arm)?));
        }
        let result = if values.is_empty() {
            self.intern_type(dir::Type::Never)?
        } else {
            self.normalized_union_type(values)?
        };
        self.commit_node_type(node.into_any(), result)?;

        Ok(Answer::Ready(()))
    }

    /// Infer one switch statement and its case bodies.
    pub(in crate::check) fn infer_switch_statement(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        cases: &[dir::LocalNodeId<dir::SwitchCase>],
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let cases = self.present_switch_cases(module, cases)?;
        answer!(self.infer_switch_selectors(module, value, &cases)?);

        // infer case bodies in source order without joining their values
        for case in &cases {
            let body = self.module(module).view().get(*case).body;
            let site = self.node_site(body.into_global_any(module))?;
            answer!(self.infer_node_type(site, PlaceUse::Read)?);
        }
        let result = self.end_type(module, node.local_id.into_any())?;
        self.commit_node_type(node.into_any(), result)?;

        Ok(Answer::Ready(()))
    }

    /// Infer match patterns against one selected value.
    fn infer_match_patterns(
        &mut self,
        module: ModuleId,
        value: dir::LocalNodeId<dir::Expression>,
        arms: &[dir::LocalNodeId<dir::MatchArm>],
    ) -> CompilerResult<Answer<()>> {
        let value_site = self.check.node_site(value.into_global_any(module))?;
        let scrutinee = answer!(self.infer_node_type(value_site, PlaceUse::Read)?);

        // check every pattern and guard against the selected value
        for arm in arms {
            let arm = self.module(module).view().get(*arm);
            let pattern = arm.pattern();
            let guard = arm.guard();
            answer!(self.infer_match_pattern(module, pattern, scrutinee)?);
            if let Some(guard) = guard {
                answer!(self.check_match_guard(module, guard)?);
            }
        }

        Ok(Answer::Ready(()))
    }

    /// Infer switch selectors against one selected value.
    fn infer_switch_selectors(
        &mut self,
        module: ModuleId,
        value: dir::LocalNodeId<dir::Expression>,
        cases: &[dir::LocalNodeId<dir::SwitchCase>],
    ) -> CompilerResult<Answer<()>> {
        let value_site = self.check.node_site(value.into_global_any(module))?;
        let scrutinee = answer!(self.infer_node_type(value_site, PlaceUse::Read)?);
        let mut selectors = Vec::new();

        // infer every case selector before selecting their shared equality
        for case in cases {
            let dir::SwitchSelector::Case(selected) =
                self.module(module).view().get(*case).selector
            else {
                continue;
            };
            let selected_site = self.node_site(selected.into_global_any(module))?;
            let selected_type = answer!(self.infer_node_type(selected_site, PlaceUse::Read)?);
            selectors.push((
                case.into_global_any(module),
                selected.into_global_any(module),
                selected_type,
            ));
        }

        answer!(self.select_switch_equality(
            value.into_global_any(module),
            scrutinee,
            &selectors,
        )?);

        Ok(Answer::Ready(()))
    }

    /// Infer one match pattern against its selected value.
    fn infer_match_pattern(
        &mut self,
        module: ModuleId,
        pattern: dir::LocalNodeId<dir::Pattern>,
        scrutinee: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        let pattern_site = self.check.node_site(pattern.into_global_any(module))?;

        self.check_pattern(
            pattern.into_global(module),
            pattern_site.flow,
            pattern_site.scope,
            scrutinee,
        )
    }

    /// Infer one match arm body.
    fn infer_match_arm_body(
        &mut self,
        module: ModuleId,
        arm: dir::LocalNodeId<dir::MatchArm>,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let body = match self.module(module).view().get(arm) {
            dir::MatchArm::Expression { body, .. } => body.into_global_any(module),
            dir::MatchArm::Block { body, .. } => body.into_global_any(module),
        };
        let body_site = self.node_site(body)?;

        self.infer_node_type(body_site, PlaceUse::Read)
    }

    /// Check one match expression under an expected result type.
    pub(in crate::check) fn check_match_expression(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        arms: &[dir::LocalNodeId<dir::MatchArm>],
        expectation: Expectation,
    ) -> CompilerResult<Answer<CheckAttempt>> {
        let module = site.node.module_id;
        let target = expectation.target;
        let relation = expectation.relation;
        let arms = self.present_match_arms(module, arms)?;

        // type the matched value and its patterns like the inferred form
        let value_site = self.check.node_site(value.into_global_any(module))?;
        let scrutinee = answer!(self.infer_node_type(value_site, PlaceUse::Read)?);
        for arm in &arms {
            let arm = self.module(module).view().get(*arm);
            let pattern = arm.pattern();
            let guard = arm.guard();
            let pattern_site = self.check.node_site(pattern.into_global_any(module))?;
            answer!(self.check_pattern(
                pattern.into_global(module),
                pattern_site.flow,
                pattern_site.scope,
                scrutinee,
            )?);
            if let Some(guard) = guard {
                answer!(self.check_match_guard(module, guard)?);
            }
        }

        // return never for an empty match
        if arms.is_empty() {
            let never = self.intern_type(dir::Type::Never)?;
            self.commit_node_type(site.node, never)?;
            let check = ValueCheck {
                source: never,
                outcome: CheckOutcome::Holds,
                target,
            };

            return Ok(Answer::Ready(CheckAttempt::Checked(check)));
        }

        // check every arm body against the incoming expectation
        let mut values = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut check = CheckOutcome::Holds;
        for arm in &arms {
            let body = match self.module(module).view().get(*arm) {
                dir::MatchArm::Expression { body, .. } => body.into_global_any(module),
                dir::MatchArm::Block { body, .. } => body.into_global_any(module),
            };
            let body_site = self.node_site(body)?;
            let body_check = answer!(self.check_node(body_site, expectation)?);
            check = body_check.outcome.and(check);
            values.push(body_check.source);
        }

        // use the target when every arm holds and produces a value
        let joined = self.normalized_union_type(values)?;
        let result = match (relation, check) {
            (Relation::Assignable, CheckOutcome::Holds)
                if !matches!(self.check.ty(joined)?, dir::Type::Never) =>
            {
                target
            }
            _ => joined,
        };
        self.commit_node_type(site.node, result)?;

        Ok(Answer::Ready(CheckAttempt::Checked(ValueCheck {
            source: result,
            outcome: check,
            target,
        })))
    }

    /// Return the statically present arms of one match expression.
    fn present_match_arms(
        &self,
        module: ModuleId,
        arms: &[dir::LocalNodeId<dir::MatchArm>],
    ) -> CompilerResult<Vec<dir::LocalNodeId<dir::MatchArm>>> {
        let mut present = Vec::new();
        for arm in arms {
            let node = arm.into_global_any(module);
            if self.check.static_gate(node)? == StaticPresence::Present {
                present.push(*arm);
            }
        }

        Ok(present)
    }

    /// Return the statically present cases of one switch statement.
    fn present_switch_cases(
        &self,
        module: ModuleId,
        cases: &[dir::LocalNodeId<dir::SwitchCase>],
    ) -> CompilerResult<Vec<dir::LocalNodeId<dir::SwitchCase>>> {
        let mut present = Vec::new();
        for case in cases {
            let node = case.into_global_any(module);
            if self.check.static_gate(node)? == StaticPresence::Present {
                present.push(*case);
            }
        }

        Ok(present)
    }

    /// Check one match guard against boolean.
    fn check_match_guard(
        &mut self,
        module: ModuleId,
        guard: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let site = self.check.node_site(guard.into_global_any(module))?;
        let boolean = self
            .check
            .intern_type(dir::Type::Primitive(dir::PrimitiveType::Boolean))?;
        let expectation = Expectation {
            target: boolean,
            relation: Relation::Assignable,
            cause: self.check.intern_cause(Cause::root(
                Origin::Node(guard.into_global_any(module), site.scope),
                CauseKind::Expression,
            )),
            use_: ValueUse::Condition,
            mode: InferMode::Exact,
        };
        answer!(self.attempt_node(site, PlaceUse::Read, Some(expectation))?);

        Ok(Answer::Ready(()))
    }

    /// Infer one for-in or for-of expression.
    pub(in crate::check) fn infer_for_each_expression(
        &mut self,
        site: FlowSite,
        operator: dir::ForEachOperator,
        binding: dir::ForEachBinding,
        iterator: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let iterator_site = self.node_site(iterator.into_global_any(module))?;
        let iterator_type =
            answer!(self.infer_node(iterator_site, PlaceUse::Read, InferMode::Mutable,)?);
        let iterator_type = answer!(self.flow_type_at(iterator_site, iterator_type)?);
        let target =
            answer!(self.for_each_value_type(site.origin(), site.node, operator, iterator_type)?);

        // check the binding against the value produced by the iteration source
        let pattern = match binding {
            dir::ForEachBinding::Pattern { pattern, .. }
            | dir::ForEachBinding::Using { pattern, .. } => pattern,
        };
        let pattern_site = self.node_site(pattern.into_global_any(module))?;
        let cause = self.intern_cause(Cause::root(site.origin(), CauseKind::Expression));
        answer!(self.check_node_expected(
            pattern_site,
            target,
            Relation::Assignable,
            cause,
            ValueUse::Store,
            InferMode::Exact,
        )?);

        // check the loop body under the bound pattern
        let body_site = self.node_site(body.into_global_any(module))?;
        answer!(self.attempt_node(body_site, PlaceUse::Read, None)?);

        // for-in and for-of evaluate to void
        let void = self.intern_type(dir::Type::Void)?;
        self.commit_node_type(site.node, void)?;

        Ok(Answer::Ready(()))
    }

    /// Return the value type bound by one for-in or for-of source.
    fn for_each_value_type(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        operator: dir::ForEachOperator,
        iterator_type: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        match operator {
            dir::ForEachOperator::In => self.for_in_value_type(origin, source, iterator_type),
            dir::ForEachOperator::Of => self.for_of_value_type(origin, source, iterator_type),
        }
    }

    /// Return the key type bound by one for-in source.
    fn for_in_value_type(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        iterator_type: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let scope = self.origin_scope(origin)?;
        self.push_obligation(
            Obligation::ForInSource(ForInSourceObligation {
                source,
                ty: iterator_type,
            }),
            scope,
        );
        let string = self.intern_type(dir::Type::Primitive(dir::PrimitiveType::String))?;

        Ok(Answer::Ready(string))
    }

    /// Return the yielded value type of one for-of source.
    fn for_of_value_type(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        iterator_type: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let anchored = self.origin_at(origin, source)?;
        let key = dir::StaticKey::Name(self.strings().intern("iterator"));
        let source_site = self.node_site(source)?;
        let source_value = answer!(self.expression_value(source_site, iterator_type)?);
        let selected = answer!(self.select_language_protocol_call(
            anchored,
            source_value,
            iterator_type,
            dir::MemberSpace::Instance,
            key,
            dir::LanguageItem::Iterable,
            &[],
            &[],
            &[],
        )?);
        let Some((protocol, _call)) = selected else {
            self.report_for_of_source_not_iterable(source);
            let error = self.intern_type(dir::Type::Error)?;

            return Ok(Answer::Ready(error));
        };
        let Some(value) = protocol.arguments.first().copied() else {
            return Err(CompilerError::Internal {
                message: "Iterable protocol implementation has no value argument".to_owned(),
            });
        };

        Ok(Answer::Ready(value))
    }
}
