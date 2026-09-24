use destack_dir as dir;
use smallvec::SmallVec;

use super::InferMode;
use crate::CompilerResult;
use crate::sema::{
    Cause, CauseKind, CheckState, ConditionBranch, Expectation, FlowSite, Obligation, PlaceUse,
    RangeElementObligation, Relation, RelationCheck, StoreTarget, Value, ValueUse, Verdict,
};

impl CheckState<'_> {
    /// Infer one binary expression, narrowing short-circuited right operands.
    ///
    /// Example:
    /// ```ds
    /// value !== undefined && value > 0
    /// ```
    pub(in crate::sema) fn infer_binary_expression(
        &mut self,
        site: FlowSite,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
        context: Option<Expectation>,
    ) -> CompilerResult<()> {
        // infer the left operand
        let module = site.node.module_id;
        let origin = site.origin();
        let left_source = left.into_global_any(module);
        let right_source = right.into_global_any(module);
        let left_site = self.visit_site(left_source)?;
        let left_type = self.operand_type(origin, left_site)?;

        // short-circuit operators narrow their right operand
        let right_type = match ConditionBranch::from_short_circuit(operator) {
            Some(branch) => {
                let before = self.fork_flow();
                self.narrow_expression(left, branch)?;
                let right_site = self.visit_site(right_source)?;
                let right_type = self.operand_type(origin, right_site)?;
                self.restore_flow(before);

                right_type
            }
            None => {
                let right_site = self.visit_site(right_source)?;

                self.operand_type(origin, right_site)?
            }
        };

        self.select_binary_operation(
            site,
            operator,
            left_type,
            right_type,
            left_source,
            right_source,
            None,
            context,
        )
    }

    /// Infer one `satisfies` expression from its value while checking the target.
    pub(in crate::sema) fn infer_satisfies_expression(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        // visit the checked value
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let value_site = self.visit_site(value.into_global_any(module))?;

        // check the value against the target without taking its type
        let target = self.require_node_type(target_type.into_global_any(module))?;
        let cause = self.intern_cause(Cause::root(value_site.origin(), CauseKind::Expression));
        let check = self.check_node(
            value_site,
            Expectation {
                target,
                relation: Relation::Subtype,
                cause,
                use_: ValueUse::Satisfies,
                mode: InferMode::Regular,
                store: StoreTarget::Exact,
            },
        )?;
        let value_type = check.source;
        self.commit_node_type(node.into_any(), value_type)?;

        Ok(())
    }

    /// Infer one cast or const assertion expression.
    pub(in crate::sema) fn infer_as_expression(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        // visit the cast operand
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let value_site = self.visit_site(value.into_global_any(module))?;

        // `as const` keeps the operand's literal precision
        let is_const_assertion = matches!(
            self.module(module).view().get(target_type),
            dir::TypeExpression::Const
        );
        if is_const_assertion {
            let ty = self.infer_node(value_site, PlaceUse::Read, InferMode::Const)?;
            let ty = self.flow_type_at(value_site, ty)?;
            self.commit_node_type(node.into_any(), ty)?;

            return Ok(());
        }

        // check the operand through the cast target
        let target = self.require_node_type(target_type.into_global_any(module))?;
        let cause = self.intern_cause(Cause::root(value_site.origin(), CauseKind::Expression));
        let expectation = Expectation {
            target,
            relation: Relation::Storable,
            cause,
            use_: ValueUse::Cast,
            mode: InferMode::Regular,
            store: StoreTarget::Exact,
        };
        let check = self.check_node(value_site, expectation)?;
        let value_type = check.source;

        // resolve both sides of the cast
        let origin = value_site.origin();
        let value_root = self.shallow_resolve(value_type)?;
        let target_root = self.shallow_resolve(target)?;

        // deny an unwrap the newtype's backing visibility rejects
        if value_root != target_root
            && let Some(instance) = self.decompose_newtype(origin, value_root)?
            && self
                .decide(|state| {
                    state.relate_castable(origin, cause, instance.backing, target_root)
                })?
                .0
                == Verdict::Holds
        {
            self.check_backing_access(origin, instance.symbol)?;
        }

        // warn when the cast target equals the operand's resolved type
        if value_root == target_root && self.type_variables(value_root)?.is_empty() {
            self.report_redundant_cast(node.into_any(), value.into_global_any(module), target);
        }

        // take the cast target as the expression type
        self.commit_node_type(node.into_any(), target)?;

        Ok(())
    }

    /// Infer one range expression from its written bounds.
    pub(in crate::sema) fn infer_range_expression(
        &mut self,
        site: FlowSite,
        start: Option<dir::LocalNodeId<dir::Expression>>,
        end: Option<dir::LocalNodeId<dir::Expression>>,
        end_kind: dir::RangeEnd,
    ) -> CompilerResult<()> {
        // read the range expression's node
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;

        // infer every written bound
        let mut bounds = SmallVec::<[Value; 2]>::new();
        for bound in [start, end].into_iter().flatten() {
            let bound_site = self.visit_site(bound.into_global_any(module))?;
            let ty = self.infer_node_type(bound_site, PlaceUse::Read)?;
            bounds.push(Value {
                ty,
                node: Some(bound_site.node),
                place: None,
                is_fresh: self.is_fresh_node(bound_site.node)?,
            });
        }

        // constrain every written bound into one element hole
        let element = match bounds.as_slice() {
            [] => None,
            bounds => {
                let origin = site.origin();
                let variable = self.open_variable(origin);
                let element = self.variable_type(variable)?;
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
                for bound in bounds {
                    let bound = self.store_candidate(origin, *bound, element, false)?.ty;
                    self.push_relation(RelationCheck::new(
                        origin,
                        Relation::Subtype,
                        bound,
                        element,
                        cause,
                    ))?;
                }

                // oblige both endpoints to share one element type
                let scope = self.origin_scope(origin)?;
                self.push_obligation(
                    Obligation::RangeElement(RangeElementObligation {
                        source: node.into_any(),
                        element,
                    }),
                    scope,
                )?;

                Some(element)
            }
        };

        // select the range family the written bounds describe
        let item = match (start, end, end_kind) {
            (Some(_), Some(_), dir::RangeEnd::Open) => dir::LanguageItem::Range,
            (Some(_), Some(_), dir::RangeEnd::Inclusive) => dir::LanguageItem::RangeInclusive,
            (Some(_), None, _) => dir::LanguageItem::RangeFrom,
            (None, Some(_), dir::RangeEnd::Open) => dir::LanguageItem::RangeTo,
            (None, Some(_), dir::RangeEnd::Inclusive) => dir::LanguageItem::RangeToInclusive,
            (None, None, _) => dir::LanguageItem::RangeFull,
        };
        let arguments = element.into_iter().collect::<Vec<_>>();
        let range = self.language_type(item, &arguments)?;
        self.commit_node_type(node.into_any(), range)?;

        Ok(())
    }

    /// Infer one try projection expression from its operand value.
    pub(in crate::sema) fn infer_try_projection_expression(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        propagates: bool,
    ) -> CompilerResult<()> {
        // project the operand through its try output
        let node = site.node.into_typed::<dir::Expression>();
        let value_site = self.visit_site(value.into_global_any(node.module_id))?;
        let value = self.infer_node_type(value_site, PlaceUse::Read)?;
        let output =
            self.reduce_operation_type(site.origin(), dir::TypeOperation::TryOutput { value })?;
        if propagates {
            self.propagate_try_residual(node.into_any(), value, site, Some(value_site))?;
        }
        // trap must on the residual its branch splits off
        else {
            let origin = site.origin();
            let residual = self.intern_operation(dir::TypeOperation::TryResidual { value })?;
            let branch = self.select_try_branch(origin, value_site, value)?;
            self.commit_decision(
                node.into_any(),
                dir::Decision::Residual(Box::new(dir::ResidualDecision {
                    target: dir::ResidualTarget::Trap,
                    residual,
                    branch,
                    from_residual: None,
                })),
            )?;
        }

        self.commit_node_type(node.into_any(), output)?;

        Ok(())
    }

    /// Propagate one try residual to its enclosing handler or return.
    pub(in crate::sema) fn propagate_try_residual(
        &mut self,
        node: dir::GlobalNodeIdAny,
        value: dir::GlobalTypeId,
        site: FlowSite,
        operand: Option<FlowSite>,
    ) -> CompilerResult<()> {
        // build the residual the operand propagates
        let origin = site.origin();
        let residual = self.intern_operation(dir::TypeOperation::TryResidual { value })?;

        // select the branch a try implementor splits through, a nullish operand splitting itself
        let branch = match operand {
            Some(operand) => self.select_try_branch(origin, operand, value)?,
            None => None,
        };

        // collect the failure directly at a local try target, the value its catch binds
        let failure = self.intern_operation(dir::TypeOperation::TryFailure { value })?;
        if let Some(target) = self.collect_try_failure(failure) {
            self.commit_decision(
                node,
                dir::Decision::Residual(Box::new(dir::ResidualDecision {
                    target: dir::ResidualTarget::Try(target),
                    residual,
                    branch,
                    from_residual: None,
                })),
            )?;

            return Ok(());
        }

        // propagation out of the function satisfies the return's FromResidual
        if let Some(return_target) = self.current_return_target() {
            let symbol = self.language_symbol(dir::LanguageItem::FromResidual)?;
            let arguments = self.intern_type_ids(&[residual])?;
            let target = self.intern_type(dir::Type::Application(dir::GenericApplication {
                symbol,
                arguments,
            }))?;
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            self.push_relation(RelationCheck::new(
                origin,
                Relation::Subtype,
                return_target,
                target,
                cause,
            ))?;

            // rebuild the return from the residual when the target implements the protocol
            let from_residual = self.select_from_residual(origin, node, return_target, residual)?;

            self.commit_decision(
                node,
                dir::Decision::Residual(Box::new(dir::ResidualDecision {
                    target: dir::ResidualTarget::Callable,
                    residual,
                    branch,
                    from_residual,
                })),
            )?;
        }
        // try propagation needs an enclosing function
        else {
            self.report_try_outside_function(node);
        }

        Ok(())
    }

    /// Infer one awaited expression through the awaited type operation.
    pub(in crate::sema) fn infer_await_expression(
        &mut self,
        site: FlowSite,
        awaited: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let origin = site.origin();

        // require the enclosing body's asynchrony
        if self
            .flow
            .current_function()
            .is_none_or(|function| function.asynchrony != dir::Asynchrony::Async)
        {
            self.report_await_outside_async_context(module, node.local_id.into_any());
        }

        // reduce the awaited value through its output
        let awaited_site = self.visit_site(awaited.into_global_any(module))?;
        let value = self.infer_node_type(awaited_site, PlaceUse::Read)?;
        let result = self.reduce_operation_type(
            origin,
            dir::TypeOperation::Awaited(dir::UnaryType { target: value }),
        )?;

        // select and record the park call this await runs
        let flags = self.type_flags(result)?;
        if !flags.has_error() {
            // await a newtype through its backing, casting the operand down to it
            let mut parked = value;
            loop {
                let object = self.strip_form(origin, parked)?;
                let Some(instance) = self.decompose_newtype(origin, object)? else {
                    break;
                };
                self.check_backing_access(origin, instance.symbol)?;
                parked = instance.backing;
            }
            let operand = awaited.into_global_any(module);
            let argument = match parked == value {
                true => dir::ArgumentSource::Provided(operand),
                false => {
                    let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
                    let expectation = Expectation {
                        target: parked,
                        relation: Relation::Storable,
                        cause,
                        use_: ValueUse::Cast,
                        mode: InferMode::Regular,
                        store: StoreTarget::Exact,
                    };
                    self.check_node(awaited_site, expectation)?;

                    dir::ArgumentSource::Static(parked)
                }
            };

            // select the park call at the parked value, the authored operand as its argument
            let receiver = self.expression_value(awaited_site, parked)?;
            let key = dir::StaticKey::Name(self.strings().intern("park"));
            let selected = self.select_language_protocol_call(
                origin,
                receiver,
                parked,
                dir::MemberSpace::Static,
                key,
                dir::LanguageItem::Awaitable,
                &[result],
                &[result],
                &[argument],
            )?;
            match selected {
                Some((_, call)) => {
                    let mut resolution = call.resolution;
                    for call in resolution.arms_mut() {
                        for binding in &mut call.arguments {
                            binding.source = dir::ArgumentSource::Provided(operand);
                        }
                    }
                    self.commit_decision(node.into_any(), dir::Decision::Call(resolution))?;
                }
                None => self.report_source_not_awaitable(operand),
            }
        }

        self.commit_node_type(node.into_any(), result)?;

        Ok(())
    }
}
