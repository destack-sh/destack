use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    BodyState, Cause, CauseKind, CheckAttempt, CheckOutcome, ConditionBranch, ControlTargetForm,
    Expectation, ExpectedType, FlowBranch, FlowSite, ForInSourceObligation, InferMode, Obligation,
    Origin, PatternArm, PatternCoverage, PatternCoverageObligation, PlaceUse, Relation,
    RelationCheck, ValueCheck, ValueUse,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Select the non-nullish operand inspected by one chain segment.
    pub(in crate::sema) fn select_chain_operand(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        is_optional: bool,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(split) = self.split_nullish_type(origin, ty)? else {
            return Ok(ty);
        };
        if !is_optional {
            self.report_possibly_nullish(origin, split.rejected.label().to_string())?;
        }

        Ok(split.value)
    }

    /// Infer one optional chain from its accesses.
    pub(in crate::sema) fn infer_chain_expression(
        &mut self,
        site: FlowSite,
        inner: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let node = site.node;
        let module = node.module_id;
        let inner_site = self.visit_site(inner.into_global_any(module))?;
        let ty = self.infer_node_type(inner_site, PlaceUse::Read)?;

        // add undefined where the chain can short circuit
        let result = match self.chain_short_circuits(site.origin(), module, inner)? {
            true => {
                let undefined = self.check.intern_type(dir::Type::Undefined)?;
                self.check.normalized_union_type([ty, undefined])?
            }
            false => ty,
        };
        self.check.commit_node_type(node, result)?;
        self.check.commit_chain_access(node)?;

        Ok(())
    }

    /// Return whether one optional chain drops a nullish receiver.
    fn chain_short_circuits(
        &mut self,
        origin: Origin,
        module: ModuleId,
        inner: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<bool> {
        // step inward through the chain's accesses
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
                _ => return Ok(false),
            };

            // an optional access over a nullish receiver drops the chain
            if is_optional {
                let receiver = self.require_node_type(left.into_global_any(module))?;
                if self.split_nullish_type(origin, receiver)?.is_some() {
                    return Ok(true);
                }
            }

            current = left;
        }
    }

    /// Infer one if expression with isolated branch flow.
    pub(in crate::sema) fn infer_if_expression(
        &mut self,
        site: FlowSite,
        condition: &dir::Condition,
        then_expression: dir::LocalNodeId<dir::Expression>,
        else_expression: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<()> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let before = self.check.fork_flow();

        // check the true branch under the narrowed condition
        self.check
            .narrow_condition(condition, ConditionBranch::True)?;
        let then_site = self.visit_site(then_expression.into_global_any(module))?;
        let then_type = self.infer_node_type(then_site, PlaceUse::Read)?;
        let mut branches = SmallVec::<[FlowBranch; 2]>::new();
        if self.check.expression_can_complete_normally(then_expression) {
            branches.push(self.check.collect_flow_branch(before));
        }

        // check the false branch, joining a missing else as void
        let result = if let Some(else_expression) = else_expression {
            self.check.restore_flow(before);
            self.check
                .narrow_condition(condition, ConditionBranch::False)?;
            let else_site = self.visit_site(else_expression.into_global_any(module))?;
            let else_type = self.infer_node_type(else_site, PlaceUse::Read)?;
            if self.check.expression_can_complete_normally(else_expression) {
                branches.push(self.check.collect_flow_branch(before));
            }

            self.normalized_union_type([then_type, else_type])?
        } else {
            self.check.restore_flow(before);
            self.check
                .narrow_condition(condition, ConditionBranch::False)?;
            branches.push(self.check.collect_flow_branch(before));
            let void = self.intern_type(dir::Type::Void)?;

            self.normalized_union_type([then_type, void])?
        };

        // merge normally completed branches
        self.check.merge_flow_branches_from(before, &branches);
        self.commit_node_type(node.into_any(), result)?;

        // a conditional whose branches are fresh literals stays fresh
        let then_fresh = self
            .check
            .fresh_nodes
            .contains_key(&then_expression.into_global_any(module));
        let else_fresh = else_expression.is_some_and(|expression| {
            self.check
                .fresh_nodes
                .contains_key(&expression.into_global_any(module))
        });
        if then_fresh && else_fresh {
            self.check.fresh_nodes.insert(site.node, None);
        }

        Ok(())
    }

    /// Check one if expression under an expected type.
    pub(in crate::sema) fn check_if_expression(
        &mut self,
        site: FlowSite,
        condition: &dir::Condition,
        then_expression: dir::LocalNodeId<dir::Expression>,
        else_expression: Option<dir::LocalNodeId<dir::Expression>>,
        expectation: Expectation,
    ) -> CompilerResult<CheckAttempt> {
        let module = site.node.module_id;
        let target = expectation.target;
        let relation = expectation.relation;
        let before = self.check.fork_flow();

        // check the then branch against the incoming expectation
        self.check
            .narrow_condition(condition, ConditionBranch::True)?;
        let then_site = self.visit_site(then_expression.into_global_any(module))?;
        let then_check = self.check_node(then_site, expectation)?;
        let then_type = then_check.source;
        let mut check = then_check.outcome;
        let mut branches = SmallVec::<[FlowBranch; 2]>::new();
        if self.check.expression_can_complete_normally(then_expression) {
            branches.push(self.check.collect_flow_branch(before));
        }

        // check an else branch, or make the missing branch explicit as void
        let result = if let Some(else_expression) = else_expression {
            self.check.restore_flow(before);
            self.check
                .narrow_condition(condition, ConditionBranch::False)?;
            let else_site = self.visit_site(else_expression.into_global_any(module))?;
            let else_check = self.check_node(else_site, expectation)?;
            let else_type = else_check.source;
            check = check.and(else_check.outcome);
            if self.check.expression_can_complete_normally(else_expression) {
                branches.push(self.check.collect_flow_branch(before));
            }

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
            self.check.restore_flow(before);
            self.check
                .narrow_condition(condition, ConditionBranch::False)?;
            branches.push(self.check.collect_flow_branch(before));
            let void = self.intern_type(dir::Type::Void)?;

            self.normalized_union_type([then_type, void])?
        };

        // merge normally completed branches
        self.check.merge_flow_branches_from(before, &branches);
        self.commit_node_type(site.node, result)?;

        // a conditional whose branches are fresh literals stays fresh
        let module = site.node.module_id;
        let then_fresh = self
            .check
            .fresh_nodes
            .contains_key(&then_expression.into_global_any(module));
        let else_fresh = else_expression.is_some_and(|expression| {
            self.check
                .fresh_nodes
                .contains_key(&expression.into_global_any(module))
        });
        if then_fresh && else_fresh {
            self.check.fresh_nodes.insert(site.node, None);
        }

        Ok(CheckAttempt::Checked(ValueCheck {
            source: result,
            stored: result,
            outcome: check,
            target,
        }))
    }

    /// Infer one try expression with branch flow for catch.
    pub(in crate::sema) fn infer_try_expression(
        &mut self,
        site: FlowSite,
        body: dir::LocalNodeId<dir::Expression>,
        catch: Option<dir::LocalNodeId<dir::Catch>>,
        finally: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<()> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let before = self.check.fork_flow();

        // check the body branch from the incoming flow, collecting residuals
        if catch.is_some() {
            self.check.enter_try_target(node);
        }
        let body_site = self.visit_site(body.into_global_any(module))?;
        let body_type = self.infer_node_type(body_site, PlaceUse::Read)?;
        let catch_failure = match catch {
            Some(_) => Some(self.check.leave_try_target()?),
            None => None,
        };
        let body_flow = self.check.collect_flow_branch(before);
        let body_can_complete = self.check.expression_can_complete_normally(body);

        // check the catch branch from the collected failure
        let catch_branch = if let Some(catch) = catch {
            self.check.restore_flow(before);
            let catch_type = self.check_catch(module, catch, site.node.local_id, catch_failure)?;
            let catch_body = self.module(module).view().get(catch).body;
            let catch_can_complete = self.check.expression_can_complete_normally(catch_body);
            let catch_flow = self.check.collect_flow_branch(before);

            Some((catch_type, catch_can_complete, catch_flow))
        } else {
            None
        };

        // merge only branches that continue normally
        let has_normal_flow = match &catch_branch {
            // join the completing sides of body and catch
            Some((_, catch_can_complete, catch_flow)) => {
                match (body_can_complete, *catch_can_complete) {
                    (true, true) => {
                        self.check
                            .merge_flow_branches(before, &body_flow, catch_flow);

                        true
                    }
                    (true, false) => {
                        self.check.restore_flow_branch(before, &body_flow);

                        true
                    }
                    (false, true) => {
                        self.check.restore_flow_branch(before, catch_flow);

                        true
                    }
                    (false, false) => {
                        self.check.restore_flow(before);

                        false
                    }
                }
            }
            // an uncaught body continues on its own
            None if body_can_complete => {
                self.check.restore_flow_branch(before, &body_flow);

                true
            }
            // no branch continues
            None => {
                self.check.restore_flow(before);

                false
            }
        };

        // check finally even when no normal path remains
        if let Some(finally) = finally {
            let finally_site = self.visit_site(finally.into_global_any(module))?;
            self.attempt_node(finally_site, PlaceUse::Read, None)?;
            if !has_normal_flow || !self.check.expression_can_complete_normally(finally) {
                self.check.restore_flow(before);
            }
        } else if !has_normal_flow {
            self.check.restore_flow(before);
        }

        // join the body and catch results
        let result = match catch_branch {
            Some((catch_type, ..)) => self.normalized_union_type([body_type, catch_type])?,
            None => body_type,
        };
        self.commit_node_type(node.into_any(), result)?;

        Ok(())
    }

    /// Check one catch clause against its collected failure.
    fn check_catch(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::Catch>,
        anchor: dir::LocalNodeIdAny,
        failure: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // walk decorators and skip absent handlers
        if !self.walk_body_decorators(module, id.into_any())? {
            return self.check.intern_type(dir::Type::Never);
        }
        let catch = self.module(module).view().get(id).clone();
        let (pattern, ty, body) = (catch.pattern, catch.ty, catch.body);

        // catch (error: T): the failure must match the annotation
        let expected = match ty {
            Some(ty) => {
                self.walk_body_type_expression(module, ty)?;

                Some(self.require_node_type(ty.into_global_any(module))?)
            }
            None => None,
        };
        if let (Some(failure), Some(expected)) = (failure, expected) {
            let origin = Origin::Node(id.into_global_any(module), self.check.flow.template_scope());
            let cause = self
                .check
                .intern_cause(Cause::root(origin, CauseKind::Expression));
            self.check.push_relation(RelationCheck::new(
                origin,
                Relation::Assignable,
                failure,
                expected,
                cause,
            ))?;
        }

        // catch (error): flow the caught value into the pattern
        if let Some(pattern) = pattern {
            if let Some(value) = expected.or(failure) {
                let pattern_site = self.visit_site(pattern.into_global_any(module))?;
                self.report_type_shadowing_binding(module, anchor, pattern, pattern_site.origin())?;
                let cause = self.check.intern_cause(Cause::root(
                    pattern_site.origin(),
                    CauseKind::Pattern {
                        pattern: pattern.into_global_any(module),
                    },
                ));
                self.check_node_expected(
                    pattern_site,
                    value,
                    Relation::Assignable,
                    cause,
                    ValueUse::Store,
                    InferMode::Regular,
                )?;
                let scope = self.check.flow.template_scope();
                self.check.push_obligation(
                    Obligation::PatternCoverage(PatternCoverageObligation {
                        source: pattern.into_global_any(module),
                        value: ExpectedType::Type(value),
                        coverage: PatternCoverage::Catch {
                            pattern: pattern.into_global(module),
                        },
                    }),
                    scope,
                )?;
            }
            self.check.assign_bindings(pattern.into_any());
        }

        // catch (...) { ... }: the handler value joins the try result
        let body_site = self.visit_site(body.into_global_any(module))?;

        self.infer_node_type(body_site, PlaceUse::Read)
    }

    /// Infer one match expression from its arm values.
    pub(in crate::sema) fn infer_match_expression(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        arms: &[dir::LocalNodeId<dir::MatchArm>],
    ) -> CompilerResult<()> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let first_visit = self.check.committed_node_type(node.into_any()).is_none();
        let arms = self.present_match_arms(module, arms)?;

        // type the selected value ahead of the arm patterns
        let value_site = self.check.visit_site(value.into_global_any(module))?;
        let scrutinee = self.infer_node_type(value_site, PlaceUse::Read)?;
        let (values, _, coverage) = self.check_match_arms(module, value, &arms, scrutinee, None)?;

        // join every arm body into the match value
        let result = if values.is_empty() {
            self.intern_type(dir::Type::Never)?
        } else {
            self.normalized_union_type(values)?
        };
        self.commit_node_type(node.into_any(), result)?;

        // queue exhaustiveness checking at the first visit
        if first_visit {
            self.queue_match_coverage(module, node.local_id, value, coverage)?;
        }

        Ok(())
    }

    /// Infer one switch statement with ordered selection and fallthrough.
    pub(in crate::sema) fn infer_switch_statement(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        cases: &[dir::LocalNodeId<dir::SwitchCase>],
    ) -> CompilerResult<()> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let cases = self.present_switch_cases(module, cases)?;

        // type the selected value ahead of the case selectors
        let value_site = self.check.visit_site(value.into_global_any(module))?;
        let scrutinee = self.infer_node_type(value_site, PlaceUse::Read)?;
        let before = self.check.fork_flow();
        self.check
            .enter_control_target(node, None, ControlTargetForm::Switch);

        // evaluate selectors in source order and retain each equality branch
        let mut selectors = Vec::new();
        let mut direct = vec![None; cases.len()];
        let mut default_index = None;
        for (index, case) in cases.iter().enumerate() {
            let selector = self.module(module).view().get(*case).selector;
            match selector {
                dir::SwitchSelector::Case(selected) => {
                    let selected_site = self.visit_site(selected.into_global_any(module))?;
                    let selected_type = self.infer_node_type(selected_site, PlaceUse::Read)?;
                    selectors.push((
                        case.into_global_any(module),
                        selected.into_global_any(module),
                        selected_type,
                    ));

                    // narrow the equality into the selected branch
                    let after_selector = self.check.fork_flow();
                    let case_node = case.into_global_any(module);
                    self.check
                        .narrow_by_equality(case_node, value, selected, true);
                    direct[index] = Some(self.check.collect_flow_branch(before));

                    // continue selection under the failed equality
                    self.check.restore_flow(after_selector);
                    self.check
                        .narrow_by_equality(case_node, value, selected, false);
                }
                dir::SwitchSelector::Default => default_index = Some(index),
            }
        }
        self.select_switch_equality(value.into_global_any(module), scrutinee, &selectors)?;

        // route the unmatched flow into the default case
        let unmatched = self.check.collect_flow_branch(before);
        let unmatched = match default_index {
            Some(index) => {
                direct[index] = Some(unmatched);

                None
            }
            None => Some(unmatched),
        };

        // execute bodies in source order and join adjacent fallthrough
        let mut fallthrough: Option<FlowBranch> = None;
        for (index, case) in cases.iter().enumerate() {
            let Some(selected) = direct[index].as_ref() else {
                return Err(CompilerError::Internal {
                    message: format!("switch case {case:?} has no selection entry"),
                });
            };
            if let Some(previous) = &fallthrough {
                self.check.merge_flow_branches(before, selected, previous);
            } else {
                self.check.restore_flow_branch(before, selected);
            }

            let body = self.module(module).view().get(*case).body;
            let body_site = self.visit_site(body.into_global_any(module))?;
            self.infer_node_type(body_site, PlaceUse::Read)?;
            fallthrough = self
                .check
                .block_can_complete_normally(body)
                .then(|| self.check.collect_flow_branch(before));
        }

        // join explicit breaks, final fallthrough, and an unmatched value
        let mut exits = self.check.leave_control_target();
        exits.extend(fallthrough);
        exits.extend(unmatched);
        if exits.is_empty() {
            self.check
                .module_mut(module)
                .unreachable_ends
                .insert(node.local_id.into_any());
        }
        self.check.merge_flow_branches_from(before, &exits);

        let result = self.end_type(module, node.local_id.into_any())?;
        self.commit_node_type(node.into_any(), result)?;

        Ok(())
    }

    /// Report one bare pattern binding whose name shadows a visible type.
    ///
    /// Examples:
    /// ```ds
    /// match (value) { Cancelled => 0 }
    /// if (let Cancelled = value) {}
    /// try { value? } catch (Cancelled) {}
    /// ```
    pub(in crate::sema) fn report_type_shadowing_binding(
        &mut self,
        module: ModuleId,
        anchor: dir::LocalNodeIdAny,
        pattern: dir::LocalNodeId<dir::Pattern>,
        origin: Origin,
    ) -> CompilerResult<()> {
        // unwrap marker and representation patterns to the bare binding name
        let view = self.module(module).view();
        let mut inner = pattern;
        while let dir::Pattern::Must(wrapped)
        | dir::Pattern::Default {
            pattern: wrapped, ..
        }
        | dir::Pattern::BorrowOf { right: wrapped, .. }
        | dir::Pattern::MoveOf { right: wrapped, .. }
        | dir::Pattern::DereferenceOf { right: wrapped } = view.get(inner)
        {
            inner = *wrapped;
        }
        let dir::Pattern::Binding {
            name,
            pattern: None,
        } = *view.get(inner)
        else {
            return Ok(());
        };

        // look the name up outside the pattern's own binding scope
        let lookup = self.check.binding_table(module).lookup_symbol_at(
            &view,
            anchor,
            dir::StaticKey::Name(name),
        );
        let symbols: SmallVec<[dir::LocalSymbolId; 2]> = match lookup {
            dir::SymbolLookup::Missing => return Ok(()),
            dir::SymbolLookup::Found(symbol) => SmallVec::from_slice(&[symbol]),
            dir::SymbolLookup::Ambiguous(symbols) => symbols.into_iter().collect(),
        };

        // resolve local declarations and imports to their declared kinds
        for symbol in symbols {
            let kind = self.check.binding_table(module).get_symbol(symbol).kind;
            if kind.is_type_definition() {
                let name = self.check.strings().get(name).to_string();

                return self.check.report_pattern_shadows_type(origin, name);
            } else if kind != dir::SymbolKind::Import {
                continue;
            }
            let Some(resolution) = self
                .module(module)
                .resolved
                .imports
                .symbol_resolution(symbol)
                .cloned()
            else {
                continue;
            };
            for target in resolution.targets() {
                let dir::ReferenceTarget::Symbol(target) = target else {
                    continue;
                };
                if !self.check.is_own_module(target.module_id) {
                    self.check.import_external_module(target.module_id)?;
                }
                if self.symbol_kind(target)?.is_type_definition() {
                    let name = self.check.strings().get(name).to_string();

                    return self.check.report_pattern_shadows_type(origin, name);
                }
            }
        }

        Ok(())
    }

    /// Check present match arms with isolated branch flow.
    fn check_match_arms(
        &mut self,
        module: ModuleId,
        value: dir::LocalNodeId<dir::Expression>,
        arms: &[dir::LocalNodeId<dir::MatchArm>],
        scrutinee: dir::GlobalTypeId,
        expectation: Option<Expectation>,
    ) -> CompilerResult<(
        SmallVec<[dir::GlobalTypeId; 4]>,
        CheckOutcome,
        Vec<PatternArm>,
    )> {
        let value_path = self.check.lexical_access_path(value);
        let before = self.check.fork_flow();
        let mut coverage = Vec::new();
        let mut excluded: Vec<dir::GlobalNodeId<dir::Pattern>> = Vec::new();
        let mut merged: Option<FlowBranch> = None;
        let mut values = SmallVec::new();
        let mut outcome = CheckOutcome::Holds;

        for arm in arms {
            // apply exclusions from previous arms
            self.check.restore_flow(before);
            if let Some(path) = &value_path {
                for pattern in &excluded {
                    self.check.exclude_match_pattern(path.clone(), *pattern);
                }
            }

            // check the pattern against the selected value
            let arm_node = self.module(module).view().get(*arm).clone();
            let pattern = arm_node.pattern();
            let guard = arm_node.guard();
            let pattern_site = self.check.visit_site(pattern.into_global_any(module))?;
            self.report_type_shadowing_binding(
                module,
                value.into_any(),
                pattern,
                pattern_site.origin(),
            )?;
            self.check_pattern(
                pattern.into_global(module),
                pattern_site.flow,
                pattern_site.scope,
                scrutinee,
            )?;
            if let Some(path) = &value_path {
                self.check.narrow_pattern(path.clone(), pattern, true)?;
            }
            self.check.assign_bindings(pattern.into_any());

            // apply the optional arm guard
            if let Some(guard) = guard {
                self.check_condition_operands(module, guard)?;
                self.check.narrow_condition(guard, ConditionBranch::True)?;
            }

            // check the arm body under the narrowed flow
            let body = match &arm_node {
                dir::MatchArm::Expression { body, .. } => body.into_global_any(module),
                dir::MatchArm::Block { body, .. } => body.into_global_any(module),
            };
            let body_site = self.visit_site(body)?;
            match expectation {
                Some(expectation) => {
                    let body_check = self.check_node(body_site, expectation)?;
                    outcome = body_check.outcome.and(outcome);
                    values.push(body_check.source);
                }
                None => values.push(self.infer_node_type(body_site, PlaceUse::Read)?),
            }

            // record coverage and unguarded exclusions
            coverage.push(PatternArm {
                pattern: pattern.into_global(module),
                is_guarded: guard.is_some(),
            });
            if guard.is_none() {
                excluded.push(pattern.into_global(module));
            }

            // merge completing arms into the post-match flow
            if self.check.match_arm_can_complete_normally(&arm_node) {
                let branch = self.check.collect_flow_branch(before);
                merged = match merged.take() {
                    Some(previous) => {
                        self.check.merge_flow_branches(before, &previous, &branch);

                        Some(self.check.collect_flow_branch(before))
                    }
                    None => Some(branch),
                };
            }
        }

        // restore the merged output or the pre-match input
        if let Some(merged) = merged {
            self.check.restore_flow_branch(before, &merged);
        } else {
            self.check.restore_flow(before);
        }

        Ok((values, outcome, coverage))
    }

    /// Queue coverage checking for one match expression.
    fn queue_match_coverage(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
        arms: Vec<PatternArm>,
    ) -> CompilerResult<()> {
        let scope = self.check.flow.template_scope();
        self.check.push_obligation(
            Obligation::PatternCoverage(PatternCoverageObligation {
                source: id.into_global_any(module),
                value: ExpectedType::Node(value.into_global_any(module)),
                coverage: PatternCoverage::Match { arms },
            }),
            scope,
        )?;

        Ok(())
    }

    /// Check one match expression under an expected result type.
    pub(in crate::sema) fn check_match_expression(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        arms: &[dir::LocalNodeId<dir::MatchArm>],
        expectation: Expectation,
    ) -> CompilerResult<CheckAttempt> {
        let module = site.node.module_id;
        let target = expectation.target;
        let relation = expectation.relation;
        let first_visit = self.check.committed_node_type(site.node).is_none();
        let arms = self.present_match_arms(module, arms)?;

        // type the selected value ahead of the arm patterns
        let value_site = self.check.visit_site(value.into_global_any(module))?;
        let scrutinee = self.infer_node_type(value_site, PlaceUse::Read)?;

        // check every arm body against the incoming expectation
        let (values, check, coverage) =
            self.check_match_arms(module, value, &arms, scrutinee, Some(expectation))?;

        // queue exhaustiveness checking at the first visit
        if first_visit {
            let node = site.node.into_typed::<dir::Expression>();
            self.queue_match_coverage(module, node.local_id, value, coverage)?;
        }

        // return never for an empty match
        if values.is_empty() {
            let never = self.intern_type(dir::Type::Never)?;
            self.commit_node_type(site.node, never)?;
            let check = ValueCheck {
                source: never,
                stored: never,
                outcome: CheckOutcome::Holds,
                target,
            };

            return Ok(CheckAttempt::Checked(check));
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

        Ok(CheckAttempt::Checked(ValueCheck {
            source: result,
            stored: result,
            outcome: check,
            target,
        }))
    }

    /// Return the statically present arms of one match expression.
    fn present_match_arms(
        &mut self,
        module: ModuleId,
        arms: &[dir::LocalNodeId<dir::MatchArm>],
    ) -> CompilerResult<Vec<dir::LocalNodeId<dir::MatchArm>>> {
        let mut present = Vec::new();
        for arm in arms {
            if self.walk_body_decorators(module, arm.into_any())? {
                present.push(*arm);
            }
        }

        Ok(present)
    }

    /// Return the statically present cases of one switch statement.
    fn present_switch_cases(
        &mut self,
        module: ModuleId,
        cases: &[dir::LocalNodeId<dir::SwitchCase>],
    ) -> CompilerResult<Vec<dir::LocalNodeId<dir::SwitchCase>>> {
        let mut present = Vec::new();
        for case in cases {
            if self.walk_body_decorators(module, case.into_any())? {
                present.push(*case);
            }
        }

        Ok(present)
    }

    /// Infer one for-in or for-of expression.
    pub(in crate::sema) fn infer_for_each_expression(
        &mut self,
        site: FlowSite,
        label: Option<dir::StringId>,
        operator: dir::ForEachOperator,
        binding: dir::ForEachBinding,
        iterator: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<()> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let iterator_site = self.visit_site(iterator.into_global_any(module))?;
        let iterator_type = self.infer_node(iterator_site, PlaceUse::Read, InferMode::Regular)?;
        let iterator_type = self.flow_type_at(iterator_site, iterator_type)?;
        let target = self.for_each_value_type(site.origin(), site.node, operator, iterator_type)?;

        // check the binding against the value produced by the iteration source
        let pattern = match binding {
            dir::ForEachBinding::Pattern { pattern, .. }
            | dir::ForEachBinding::Using { pattern, .. } => pattern,
        };
        let pattern_site = self.visit_site(pattern.into_global_any(module))?;
        let cause = self.intern_cause(Cause::root(site.origin(), CauseKind::Expression));
        self.check_node_expected(
            pattern_site,
            target,
            Relation::Assignable,
            cause,
            ValueUse::Store,
            InferMode::Regular,
        )?;

        // check the loop body with the iteration bindings assigned
        let label = self.check.control_label(site.node.into_typed(), label)?;
        self.check.enter_control_target(
            site.node.into_typed(),
            label,
            ControlTargetForm::Iteration,
        );
        let before_body = self.check.fork_flow();
        self.check.assign_bindings(pattern.into_any());
        let body_site = self.visit_site(body.into_global_any(module))?;
        self.attempt_node(body_site, PlaceUse::Read, None)?;
        self.check.restore_flow(before_body);

        // merge break branches with the normal exit
        let normal_flow = self.check.collect_flow_branch(before_body);
        let mut branches = self.check.leave_control_target();
        branches.push(normal_flow);
        self.check.merge_flow_branches_from(before_body, &branches);

        // for-in and for-of evaluate to void
        let void = self.intern_type(dir::Type::Void)?;
        self.commit_node_type(site.node, void)?;

        Ok(())
    }

    /// Return the value type bound by one for-in or for-of source.
    fn for_each_value_type(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        operator: dir::ForEachOperator,
        iterator_type: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
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
    ) -> CompilerResult<dir::GlobalTypeId> {
        let scope = self.origin_scope(origin)?;
        self.push_obligation(
            Obligation::ForInSource(ForInSourceObligation {
                source,
                ty: iterator_type,
            }),
            scope,
        )?;
        let string = self.intern_type(dir::Type::Primitive(dir::PrimitiveType::String))?;

        Ok(string)
    }

    /// Return the yielded value type of one for-of source.
    fn for_of_value_type(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        iterator_type: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // select the source's iterator member through the Iterable protocol
        let anchored = self.origin_at(origin, source)?;
        let key = dir::StaticKey::Name(self.strings().intern("iterator"));
        let source_site = self.visit_site(source)?;
        let source_value = self.expression_value(source_site, iterator_type)?;
        let selected = self.select_language_protocol_call(
            anchored,
            source_value,
            iterator_type,
            dir::MemberSpace::Instance,
            key,
            dir::LanguageItem::Iterable,
            &[],
            &[],
            &[],
        )?;

        // sources without an implementation cannot be iterated
        let Some((protocol, _call)) = selected else {
            self.report_for_of_source_not_iterable(source);
            let error = self.intern_type(dir::Type::Error)?;

            return Ok(error);
        };
        let Some(value) = protocol.arguments.first().copied() else {
            return Err(CompilerError::Internal {
                message: "Iterable protocol implementation has no value argument".to_owned(),
            });
        };

        Ok(value)
    }
}
