use smallvec::SmallVec;
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::sema::{
    Cause, CauseKind, CheckAttempt, CheckOutcome, CheckState, ConditionBranch, ControlTargetForm,
    Expectation, ExpectedType, FlowBranch, FlowSite, InferMode, Obligation, Origin, PatternArm,
    PatternCoverage, PatternCoverageObligation, PlaceUse, Relation, RelationCheck, StoreTarget,
    Value, ValueCheck, ValueUse,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Infer one optional chain from its accesses.
    pub(in crate::sema) fn infer_chain_expression(
        &mut self,
        site: FlowSite,
        inner: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // infer the value beneath the chain
        let node = site.node;
        let module = node.module_id;
        let inner_site = self.visit_site(inner.into_global_any(module))?;
        let ty = self.infer_node_type(inner_site, PlaceUse::Read)?;

        // add undefined where the chain can short circuit
        let result = match self.chain_short_circuits(site.origin(), module, inner)? {
            true => {
                let undefined = self.intern_type(dir::Type::Undefined)?;
                let result = self.normalized_union_type([ty, undefined])?;
                self.convert_chain_completion(inner_site, ty, result)?;

                result
            }
            false => ty,
        };
        self.commit_node_type(node, result)?;
        self.commit_chain_access(node)?;

        Ok(())
    }

    /// Record the completed chain value's conversion into the joined result.
    fn convert_chain_completion(
        &mut self,
        inner_site: FlowSite,
        ty: dir::GlobalTypeId,
        result: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        if result == ty {
            return Ok(());
        }

        let value = self.expression_value(inner_site, ty)?;
        let cause = self.intern_cause(Cause::root(inner_site.origin(), CauseKind::Expression));
        self.convert_value(
            inner_site,
            cause,
            Relation::Storable,
            value,
            result,
            ValueUse::Output,
            InferMode::Regular,
        )?;

        Ok(())
    }

    /// Check one optional chain, contextualizing its accesses behind the nullish members.
    pub(in crate::sema) fn check_chain_expression(
        &mut self,
        site: FlowSite,
        inner: dir::LocalNodeId<dir::Expression>,
        expectation: Expectation,
    ) -> CompilerResult<CheckAttempt> {
        // visit the value beneath the chain
        let node = site.node;
        let module = node.module_id;
        let inner_site = self.visit_site(inner.into_global_any(module))?;

        // check the chain's value once under the context
        let check = self.check_node(inner_site, expectation)?;

        // add undefined where the chain can short circuit
        let result = match self.chain_short_circuits(site.origin(), module, inner)? {
            true => {
                let undefined = self.intern_type(dir::Type::Undefined)?;
                let result = self.normalized_union_type([check.source, undefined])?;
                self.convert_chain_completion(inner_site, check.source, result)?;

                result
            }
            false => check.source,
        };
        self.commit_node_type(node, result)?;
        self.commit_chain_access(node)?;

        // leave the joined result to convert at its use
        let source = self.flow_type_at(site, result)?;
        let value = self.expression_value(site, source)?;
        let converted = self.convert_value(
            site,
            expectation.cause,
            expectation.relation,
            value,
            expectation.target,
            expectation.use_,
            expectation.mode,
        )?;

        Ok(CheckAttempt::Checked(ValueCheck {
            source: result,
            outcome: converted.outcome,
            target: expectation.target,
        }))
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
        context: Option<Expectation>,
    ) -> CompilerResult<()> {
        // fork the flow before the branches
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let before = self.fork_flow();

        // check the true branch under the narrowed condition
        self.narrow_condition(condition, ConditionBranch::True)?;
        let then_site = self.visit_site(then_expression.into_global_any(module))?;
        let then_type = self.infer_branch(then_site, context)?;
        let mut branch_values = SmallVec::<[dir::GlobalNodeIdAny; 2]>::new();
        branch_values.push(then_site.node);
        let mut branches = SmallVec::<[FlowBranch; 2]>::new();
        if self.expression_can_complete_normally(then_expression) {
            branches.push(self.collect_flow_branch(before));
        }

        // check the false branch, joining a missing else as void
        let result = if let Some(else_expression) = else_expression {
            self.restore_flow(before);
            self.narrow_condition(condition, ConditionBranch::False)?;
            let else_site = self.visit_site(else_expression.into_global_any(module))?;
            let else_type = self.infer_branch(else_site, context)?;
            branch_values.push(else_site.node);
            if self.expression_can_complete_normally(else_expression) {
                branches.push(self.collect_flow_branch(before));
            }

            self.normalized_union_type([then_type, else_type])?
        } else {
            self.restore_flow(before);
            self.narrow_condition(condition, ConditionBranch::False)?;
            branches.push(self.collect_flow_branch(before));
            let void = self.intern_type(dir::Type::Void)?;

            self.normalized_union_type([then_type, void])?
        };

        // merge normally completed branches
        self.merge_flow_branches_from(before, &branches);
        self.widen_branch_values(branch_values, result, context)?;
        self.commit_node_type(node.into_any(), result)?;

        Ok(())
    }

    /// Convert each branch value into the type its branches join to.
    fn widen_branch_values(
        &mut self,
        branches: impl IntoIterator<Item = dir::GlobalNodeIdAny>,
        result: dir::GlobalTypeId,
        context: Option<Expectation>,
    ) -> CompilerResult<()> {
        // keep the branches converted against an inherited context
        if context.is_some() {
            return Ok(());
        }

        for branch in branches {
            self.widen_branch_value(branch, result)?;
        }

        Ok(())
    }

    /// Convert one branch value node into the type its branches join to.
    fn widen_branch_value(
        &mut self,
        node: dir::GlobalNodeIdAny,
        result: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        // convert each value a branching node produces
        if let Some(values) = self.branching_value_nodes(node) {
            for value in values {
                self.widen_branch_value(value, result)?;
            }

            return Ok(());
        }

        // leave every value already committed at a runtime type
        let Some(source) = self.own_node_type(node) else {
            return Ok(());
        };
        if source == result || !self.is_literal_shape(source)? {
            return Ok(());
        }

        // convert the literal this node commits to
        let site = self.visit_site(node)?;
        let cause = self.intern_cause(Cause::root(site.origin(), CauseKind::Expression));
        self.check_node(
            site,
            Expectation {
                target: result,
                relation: Relation::Storable,
                cause,
                use_: ValueUse::Store,
                mode: InferMode::Regular,
                store: StoreTarget::Exact,
            },
        )?;

        Ok(())
    }

    /// Infer one branch value, converting it into the context the branching node inherits.
    fn infer_branch(
        &mut self,
        site: FlowSite,
        context: Option<Expectation>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // check under the inherited context, else infer alone
        match context {
            Some(expectation) => {
                let check = self.check_node(site, expectation)?;

                Ok(match check.outcome {
                    CheckOutcome::Holds if !matches!(self.ty(check.source)?, dir::Type::Never) => {
                        check.target
                    }
                    _ => check.source,
                })
            }
            None => self.infer_node_type(site, PlaceUse::Read),
        }
    }

    /// Infer one try expression with branch flow for catch.
    pub(in crate::sema) fn infer_try_expression(
        &mut self,
        site: FlowSite,
        body: dir::LocalNodeId<dir::Expression>,
        catch: Option<dir::LocalNodeId<dir::Catch>>,
        finally: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<()> {
        // fork the flow before the body
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let before = self.fork_flow();

        // check the body branch from the incoming flow, collecting residuals
        if catch.is_some() {
            self.enter_try_target(node);
        }
        let body_site = self.visit_site(body.into_global_any(module))?;
        let body_type = self.infer_node_type(body_site, PlaceUse::Read)?;
        let catch_failure = match catch {
            Some(_) => Some(self.leave_try_target()?),
            None => None,
        };
        let body_flow = self.collect_flow_branch(before);
        let body_can_complete = self.expression_can_complete_normally(body);

        // check the catch branch from the collected failure
        let catch_branch = if let Some(catch) = catch {
            self.restore_flow(before);
            let catch_type = self.check_catch(module, catch, site.node.local_id, catch_failure)?;
            let catch_body = self.module(module).view().get(catch).body;
            let catch_can_complete = self.expression_can_complete_normally(catch_body);
            let catch_flow = self.collect_flow_branch(before);

            Some((catch_type, catch_can_complete, catch_flow))
        } else {
            None
        };

        // merge the branches that continue normally
        let has_normal_flow = match &catch_branch {
            // join the completing sides of body and catch
            Some((_, catch_can_complete, catch_flow)) => {
                match (body_can_complete, *catch_can_complete) {
                    (true, true) => {
                        self.merge_flow_branches(before, &body_flow, catch_flow);

                        true
                    }
                    (true, false) => {
                        self.restore_flow_branch(before, &body_flow);

                        true
                    }
                    (false, true) => {
                        self.restore_flow_branch(before, catch_flow);

                        true
                    }
                    (false, false) => {
                        self.restore_flow(before);

                        false
                    }
                }
            }
            // continue an uncaught body alone
            None if body_can_complete => {
                self.restore_flow_branch(before, &body_flow);

                true
            }
            // no branch continues
            None => {
                self.restore_flow(before);

                false
            }
        };

        // check finally even when no normal path remains
        if let Some(finally) = finally {
            let finally_site = self.visit_site(finally.into_global_any(module))?;
            self.attempt_node(finally_site, PlaceUse::Read, None)?;
            if !has_normal_flow || !self.expression_can_complete_normally(finally) {
                self.restore_flow(before);
            }
        } else if !has_normal_flow {
            self.restore_flow(before);
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
            return self.intern_type(dir::Type::Never);
        }

        // read the parts of the catch clause
        let catch = self.module(module).view().get(id).clone();
        let (pattern, ty, body) = (catch.pattern, catch.ty, catch.body);

        // catch (error: T): the failure must match the annotation
        let expected = match ty {
            Some(ty) => {
                self.walk_body_binding_type(module, ty, false)?;

                Some(self.require_node_type(ty.into_global_any(module))?)
            }
            None => None,
        };
        if let (Some(failure), Some(expected)) = (failure, expected) {
            let origin = Origin::Node(id.into_global_any(module), self.flow.template_scope());
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            self.push_relation(RelationCheck::new(
                origin,
                Relation::Storable,
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
                let cause = self.intern_cause(Cause::root(
                    pattern_site.origin(),
                    CauseKind::Pattern {
                        pattern: pattern.into_global_any(module),
                    },
                ));
                self.check_node(
                    pattern_site,
                    Expectation::assignable(value, cause, ValueUse::Store),
                )?;
                let scope = self.flow.template_scope();
                self.push_obligation(
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
            self.assign_bindings(pattern.into_any());
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
        context: Option<Expectation>,
    ) -> CompilerResult<()> {
        // read the arms the decorators leave present
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let first_visit = self.committed_node_type(node.into_any()).is_none();
        let arms = self.present_match_arms(module, arms)?;

        // type the selected value ahead of the arm patterns
        let value_site = self.visit_site(value.into_global_any(module))?;
        let scrutinee = self.infer_node_type(value_site, PlaceUse::Read)?;
        let (values, coverage) = self.infer_match_arms(module, value, &arms, scrutinee, context)?;

        // join every arm body into the match value
        let result = if values.is_empty() {
            self.intern_type(dir::Type::Never)?
        } else {
            let types = values
                .iter()
                .map(|(_, ty)| *ty)
                .collect::<SmallVec<[_; 4]>>();

            self.normalized_union_type(types)?
        };
        self.widen_branch_values(values.iter().map(|(site, _)| site.node), result, context)?;
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
        // read the cases the decorators leave present
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let cases = self.present_switch_cases(module, cases)?;

        // type the selected value ahead of the case selectors
        let value_site = self.visit_site(value.into_global_any(module))?;
        let scrutinee = self.infer_node_type(value_site, PlaceUse::Read)?;
        let before = self.fork_flow();
        self.enter_control_target(node, None, ControlTargetForm::Switch);

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
                    let after_selector = self.fork_flow();
                    let case_node = case.into_global_any(module);
                    self.narrow_by_equality(case_node, value, selected, true);
                    direct[index] = Some(self.collect_flow_branch(before));

                    // continue selection under the failed equality
                    self.restore_flow(after_selector);
                    self.narrow_by_equality(case_node, value, selected, false);
                }
                dir::SwitchSelector::Default => default_index = Some(index),
            }
        }
        self.select_switch_equality(value.into_global_any(module), scrutinee, &selectors)?;

        // route the unmatched flow into the default case
        let unmatched = self.collect_flow_branch(before);
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
                self.merge_flow_branches(before, selected, previous);
            } else {
                self.restore_flow_branch(before, selected);
            }

            let body = self.module(module).view().get(*case).body;
            let body_site = self.visit_site(body.into_global_any(module))?;
            self.infer_node_type(body_site, PlaceUse::Read)?;
            fallthrough = self
                .block_can_complete_normally(body)
                .then(|| self.collect_flow_branch(before));
        }

        // join explicit breaks, final fallthrough, and an unmatched value
        let mut exits = self.leave_control_target();
        exits.extend(fallthrough);
        exits.extend(unmatched);
        if exits.is_empty() {
            self.module_mut(module)
                .unreachable_ends
                .insert(node.local_id.into_any());
        }
        self.merge_flow_branches_from(before, &exits);

        // type the switch at the end its exits flow to
        let result = self.end_type(module, node.local_id.into_any())?;
        self.commit_node_type(node.into_any(), result)?;

        Ok(())
    }

    /// Report one bare pattern binding whose name shadows a visible type.
    ///
    /// Examples:
    /// ```tspp
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

        // look the name up outside the binding scope of the pattern
        let lookup =
            self.module
                .binding_table()
                .lookup_symbol_at(&view, anchor, dir::StaticKey::Name(name));
        let symbols: SmallVec<[dir::LocalSymbolId; 2]> = match lookup {
            dir::SymbolLookup::Missing => return Ok(()),
            dir::SymbolLookup::Found(symbol) => SmallVec::from_slice(&[symbol]),
            dir::SymbolLookup::Ambiguous(symbols) => symbols.into_iter().collect(),
        };

        // resolve local declarations and imports to their declared kinds
        for symbol in symbols {
            let kind = self.binding_table(module)?.get_symbol(symbol).kind;
            if kind.is_type_definition() {
                let name = self.strings().get(name).to_string();

                return self.report_pattern_shadows_type(origin, name);
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
                if self.symbol_kind(target)?.is_type_definition() {
                    let name = self.strings().get(name).to_string();

                    return self.report_pattern_shadows_type(origin, name);
                }
            }
        }

        Ok(())
    }

    /// Check present match arms with isolated branch flow.
    fn infer_match_arms(
        &mut self,
        module: ModuleId,
        value: dir::LocalNodeId<dir::Expression>,
        arms: &[dir::LocalNodeId<dir::MatchArm>],
        scrutinee: dir::GlobalTypeId,
        context: Option<Expectation>,
    ) -> CompilerResult<(
        SmallVec<[(FlowSite, dir::GlobalTypeId); 4]>,
        Vec<PatternArm>,
    )> {
        // fork the flow and collect the arm results
        let value_path = self.lexical_access_path(value);
        let before = self.fork_flow();
        let mut coverage = Vec::new();
        let mut excluded: Vec<dir::GlobalNodeId<dir::Pattern>> = Vec::new();
        let mut merged: Option<FlowBranch> = None;
        let mut values = SmallVec::new();

        // check each arm under its narrowing
        for arm in arms {
            // apply exclusions from previous arms
            self.restore_flow(before);
            if let Some(path) = &value_path {
                for pattern in &excluded {
                    self.exclude_match_pattern(path.clone(), *pattern);
                }
            }

            // check the pattern against the selected value
            let arm_node = self.module(module).view().get(*arm).clone();
            let pattern = arm_node.pattern();
            let guard = arm_node.guard();
            let pattern_site = self.visit_site(pattern.into_global_any(module))?;
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
                self.narrow_pattern(path.clone(), pattern, true)?;
            }
            self.assign_bindings(pattern.into_any());

            // apply the optional arm guard
            if let Some(guard) = guard {
                self.check_condition_operands(module, guard)?;
                self.narrow_condition(guard, ConditionBranch::True)?;
            }

            // infer the arm body under the narrowed flow
            let body = match &arm_node {
                dir::MatchArm::Expression { body, .. } => body.into_global_any(module),
                dir::MatchArm::Block { body, .. } => body.into_global_any(module),
            };
            let body_site = self.visit_site(body)?;
            let body_type = self.infer_branch(body_site, context)?;
            values.push((body_site, body_type));

            // record coverage and unguarded exclusions
            coverage.push(PatternArm {
                pattern: pattern.into_global(module),
                is_guarded: guard.is_some(),
            });
            if guard.is_none() {
                excluded.push(pattern.into_global(module));
            }

            // merge completing arms into the post-match flow
            if self.match_arm_can_complete_normally(&arm_node) {
                let branch = self.collect_flow_branch(before);
                merged = match merged.take() {
                    Some(previous) => {
                        self.merge_flow_branches(before, &previous, &branch);

                        Some(self.collect_flow_branch(before))
                    }
                    None => Some(branch),
                };
            }
        }

        // restore the merged output or the pre-match input
        if let Some(merged) = merged {
            self.restore_flow_branch(before, &merged);
        } else {
            self.restore_flow(before);
        }

        Ok((values, coverage))
    }

    /// Queue coverage checking for one match expression.
    fn queue_match_coverage(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
        arms: Vec<PatternArm>,
    ) -> CompilerResult<()> {
        // queue the coverage obligation over the arms
        let scope = self.flow.template_scope();
        self.push_obligation(
            Obligation::PatternCoverage(PatternCoverageObligation {
                source: id.into_global_any(module),
                value: ExpectedType::Node(value.into_global_any(module)),
                coverage: PatternCoverage::Match { arms },
            }),
            scope,
        )?;

        Ok(())
    }

    /// Return the statically present arms of one match expression.
    fn present_match_arms(
        &mut self,
        module: ModuleId,
        arms: &[dir::LocalNodeId<dir::MatchArm>],
    ) -> CompilerResult<Vec<dir::LocalNodeId<dir::MatchArm>>> {
        // keep the arms their decorators leave present
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
        // keep the cases their decorators leave present
        let mut present = Vec::new();
        for case in cases {
            if self.walk_body_decorators(module, case.into_any())? {
                present.push(*case);
            }
        }

        Ok(present)
    }

    /// Infer one for-of expression.
    pub(in crate::sema) fn infer_for_each_expression(
        &mut self,
        site: FlowSite,
        label: Option<dir::StringId>,
        asynchrony: dir::Asynchrony,
        binding: dir::ForEachBinding,
        iterator: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<()> {
        // infer the iteration source and the value it yields
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let iterator_site = self.visit_site(iterator.into_global_any(module))?;
        let iterator_type = self.infer_node(iterator_site, PlaceUse::Read, InferMode::Regular)?;
        let iterator_type = self.flow_type_at(iterator_site, iterator_type)?;
        let target = self.select_for_of_iteration(site, asynchrony, iterator, iterator_type)?;

        // check the binding against the value produced by the iteration source
        let pattern = match binding {
            dir::ForEachBinding::Pattern { pattern, .. }
            | dir::ForEachBinding::Using { pattern, .. } => pattern,
        };
        let pattern_site = self.visit_site(pattern.into_global_any(module))?;
        let cause = self.intern_cause(Cause::root(site.origin(), CauseKind::Expression));
        self.check_node(
            pattern_site,
            Expectation::assignable(target, cause, ValueUse::Store),
        )?;

        // select the disposal a using binding runs after every pass, beside the iteration calls
        if let dir::ForEachBinding::Using { asynchrony, .. } = binding
            && let Some(disposal) =
                self.select_disposal(pattern.into_global_any(module), pattern, asynchrony)?
        {
            let Some(dir::Decision::Iteration(iteration)) = self.decision(site.node).cloned()
            else {
                return Err(CompilerError::Internal {
                    message: "a for-of using binding without its iteration decision".to_owned(),
                });
            };
            let iteration = dir::IterationDecision {
                disposal: Some(disposal),
                ..*iteration
            };
            self.commit_decision(site.node, dir::Decision::Iteration(Box::new(iteration)))?;
        }

        // check the loop body with the iteration bindings assigned
        let label = self.control_label(site.node.into_typed(), label)?;
        self.enter_control_target(site.node.into_typed(), label, ControlTargetForm::Iteration);
        let before_body = self.fork_flow();
        self.assign_bindings(pattern.into_any());
        let body_site = self.visit_site(body.into_global_any(module))?;
        self.attempt_node(body_site, PlaceUse::Read, None)?;
        self.restore_flow(before_body);
        let continues = self.take_current_continue_branches();
        self.commit_single_pass_loop(site.node, body, &continues)?;

        // merge break branches with the normal exit
        let normal_flow = self.collect_flow_branch(before_body);
        let mut branches = self.leave_control_target();
        branches.push(normal_flow);
        self.merge_flow_branches_from(before_body, &branches);

        // for-of evaluates to void
        let void = self.intern_type(dir::Type::Void)?;
        self.commit_node_type(site.node, void)?;

        Ok(())
    }

    /// Select and record the iteration of one for-of expression.
    fn select_for_of_iteration(
        &mut self,
        site: FlowSite,
        asynchrony: dir::Asynchrony,
        iterator: dir::LocalNodeId<dir::Expression>,
        iterator_type: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read the source at its authored place
        let source = iterator.into_global_any(site.node.module_id);
        let origin = self.origin_at(site.origin(), source)?;
        let source_site = self.visit_site(source)?;
        let value = self.expression_value(source_site, iterator_type)?;

        // record the selected iteration or report the refused source
        match self.select_iteration(origin, value, asynchrony)? {
            Some((element, iteration)) => {
                self.commit_decision(site.node, dir::Decision::Iteration(Box::new(iteration)))?;

                Ok(element)
            }
            None => {
                self.report_source_not_iterable(source);

                self.intern_type(dir::Type::Error)
            }
        }
    }

    /// Select the operations and element type of an iteration.
    pub(in crate::sema) fn select_iteration(
        &mut self,
        origin: Origin,
        source_value: Value,
        asynchrony: dir::Asynchrony,
    ) -> CompilerResult<Option<(dir::GlobalTypeId, dir::IterationDecision)>> {
        // open the async protocol first, falling back to the sync one
        let protocols: &[(dir::LanguageItem, &str, dir::LanguageItem)] = match asynchrony {
            dir::Asynchrony::Sync => &[(
                dir::LanguageItem::Iterable,
                "iterator",
                dir::LanguageItem::Iterator,
            )],
            dir::Asynchrony::Async => &[
                (
                    dir::LanguageItem::AsyncIterable,
                    "asyncIterator",
                    dir::LanguageItem::AsyncIterator,
                ),
                (
                    dir::LanguageItem::Iterable,
                    "iterator",
                    dir::LanguageItem::Iterator,
                ),
            ],
        };
        let mut opened = None;
        for (iterable, iterator_key, iterator_item) in protocols.iter().copied() {
            let key = dir::StaticKey::Name(self.strings().intern(iterator_key));
            let selected = self.select_language_protocol_call(
                origin,
                source_value,
                source_value.ty,
                dir::MemberSpace::Instance,
                key,
                iterable,
                &[],
                &[],
                &[],
            )?;
            if let Some((protocol, call)) = selected {
                opened = Some((protocol, call, iterable, iterator_item));
                break;
            }
        }

        // report a source without an iteration protocol implementation
        let Some((protocol, opened, iterable, iterator_item)) = opened else {
            return Ok(None);
        };
        let Some(element) = protocol.arguments.first().copied() else {
            return Err(CompilerError::Internal {
                message: "Iterable protocol implementation has no value argument".to_owned(),
            });
        };

        // select the iterator's next member over the opened iterator
        let next_key = dir::StaticKey::Name(self.strings().intern("next"));
        let iterator_value = Value {
            ty: opened.return_type,
            node: None,
            place: None,
            is_fresh: false,
        };
        let selected = self.select_language_protocol_call(
            origin,
            iterator_value,
            opened.return_type,
            dir::MemberSpace::Instance,
            next_key,
            iterator_item,
            &[],
            &[],
            &[],
        )?;
        let Some((_, next)) = selected else {
            return Ok(None);
        };

        // park each result of an async iterator, or each element a sync iterable yields
        let (park, value) = match (asynchrony, iterable) {
            (dir::Asynchrony::Sync, _) => (None, element),
            (dir::Asynchrony::Async, dir::LanguageItem::AsyncIterable) => {
                let park = self.select_await_park(origin, next.return_type)?;

                (park.map(|call| (call, dir::AwaitTarget::Result)), element)
            }
            (dir::Asynchrony::Async, _) => match self.select_await_park(origin, element)? {
                Some(call) => {
                    let awaited = call.return_type;

                    (Some((call, dir::AwaitTarget::Element)), awaited)
                }
                None => (None, element),
            },
        };

        // record the protocol calls the loop lowers through
        let (
            dir::OperationResolution::One(iterator_call),
            dir::OperationResolution::One(next_call),
        ) = (opened.resolution, next.resolution)
        else {
            return Err(CompilerError::Internal {
                message: "an iteration protocol selected on a union receiver".to_owned(),
            });
        };
        let park = match park {
            Some((park, target)) => match park.resolution {
                dir::OperationResolution::One(call) => Some(dir::IterationAwait { call, target }),
                _ => {
                    return Err(CompilerError::Internal {
                        message: "an iteration park selected on a union receiver".to_owned(),
                    });
                }
            },
            None => None,
        };
        let iteration = dir::IterationDecision {
            iterator: iterator_call,
            next: next_call,
            awaits: park,
            disposal: None,
        };

        Ok(Some((value, iteration)))
    }

    /// Record the disposal protocol calls one using binding runs at scope exit.
    pub(in crate::sema) fn record_disposal(
        &mut self,
        anchor: dir::GlobalNodeIdAny,
        pattern: dir::LocalNodeId<dir::Pattern>,
        asynchrony: dir::Asynchrony,
    ) -> CompilerResult<()> {
        let Some(disposal) = self.select_disposal(anchor, pattern, asynchrony)? else {
            return Ok(());
        };

        self.commit_decision(anchor, dir::Decision::Disposal(Box::new(disposal)))
    }
}
