use destack_dir as dir;
use smallvec::SmallVec;

use super::InferMode;
use crate::CompilerResult;
use crate::sema::{
    BodyState, Cause, CauseKind, Expectation, FlowSite, Obligation, PlaceUse,
    RangeElementObligation, Relation, RelationCheck, ValueUse, VariableRole, Widening,
};

impl BodyState<'_, '_> {
    /// Infer one `satisfies` expression from its value while checking the target.
    pub(in crate::sema) fn infer_satisfies_expression(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let value_site = self.visit_site(value.into_global_any(module))?;

        // check the value against the target without taking its type
        let target = self.require_node_type(target_type.into_global_any(module))?;
        let cause = self.intern_cause(Cause::root(site.origin(), CauseKind::Expression));
        let check = self.check_node_expected(
            value_site,
            target,
            Relation::Satisfies,
            cause,
            ValueUse::Satisfies,
            InferMode::Mutable,
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

        let target = self.require_node_type(target_type.into_global_any(module))?;

        // check the operand through the cast target
        let cause = self.intern_cause(Cause::root(value_site.origin(), CauseKind::Expression));
        let expectation = Expectation {
            target,
            relation: Relation::Castable,
            cause,
            use_: ValueUse::Store,
            mode: InferMode::Widen,
        };
        let check = self.check_node(value_site, expectation)?;
        let value_type = check.source;

        // warn when the cast target equals the operand's settled type
        let value_root = self.check.shallow_resolve(value_type)?;
        let target_root = self.check.shallow_resolve(target)?;
        if value_root == target_root && self.check.type_variables(value_root)?.is_empty() {
            self.check.report_redundant_cast(
                node.into_any(),
                value.into_global_any(module),
                target,
            );
        }

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
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;

        // infer every written bound
        let mut bounds = SmallVec::<[dir::GlobalTypeId; 2]>::new();
        if let Some(start) = start {
            let start_site = self.visit_site(start.into_global_any(module))?;
            bounds.push(self.infer_node_type(start_site, PlaceUse::Read)?);
        }
        if let Some(end) = end {
            let end_site = self.visit_site(end.into_global_any(module))?;
            bounds.push(self.infer_node_type(end_site, PlaceUse::Read)?);
        }

        // constrain every written bound into one element hole
        let element = match bounds.as_slice() {
            [] => None,
            bounds => {
                let origin = site.origin();
                let variable =
                    self.allocate_variable(origin, Widening::Const, VariableRole::Regular);
                let element = self.variable_type(variable)?;
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
                for bound in bounds {
                    self.push_relation(RelationCheck::new(
                        origin,
                        Relation::Assignable,
                        *bound,
                        element,
                        cause,
                    ))?;
                }

                // oblige both endpoints to share one element type
                let scope = self.check.origin_scope(origin)?;
                self.check.push_obligation(
                    Obligation::RangeElement(RangeElementObligation {
                        source: node.into_any(),
                        element,
                    }),
                    scope,
                );

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
        let node = site.node.into_typed::<dir::Expression>();
        let value_site = self.visit_site(value.into_global_any(node.module_id))?;
        let value = self.infer_node_type(value_site, PlaceUse::Read)?;
        let output =
            self.reduce_operation_type(site.origin(), dir::TypeOperation::TryOutput { value })?;
        if propagates {
            self.propagate_try_residual(node.into_any(), value, site)?;
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
    ) -> CompilerResult<()> {
        let origin = site.origin();
        let residual = self.intern_operation(dir::TypeOperation::TryResidual { value })?;

        // collect the residual directly at a local try target
        if self.check.collect_try_residual(residual) {
            return Ok(());
        }

        // propagation out of the function satisfies the return's FromResidual
        if let Some(return_target) = self.check.current_return_target() {
            let symbol = self
                .check
                .language_symbol(dir::LanguageItem::FromResidual)?;
            let arguments = self.check.intern_type_ids(&[residual])?;
            let target = self.intern_type(dir::Type::Application(dir::GenericApplication {
                symbol,
                arguments,
            }))?;
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            self.push_relation(RelationCheck::new(
                origin,
                Relation::Satisfies,
                return_target,
                target,
                cause,
            ))?;
        }
        // try propagation needs an enclosing function
        else {
            self.check.report_try_outside_function(node);
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
            .check
            .flow
            .current_function()
            .is_none_or(|function| function.asynchrony != dir::Asynchrony::Async)
        {
            self.check
                .report_await_outside_async_context(module, node.local_id.into_any());
        }

        let awaited_site = self.visit_site(awaited.into_global_any(module))?;
        let value = self.infer_node_type(awaited_site, PlaceUse::Read)?;
        let result = self.reduce_operation_type(
            origin,
            dir::TypeOperation::Awaited(dir::UnaryType { target: value }),
        )?;
        self.commit_node_type(node.into_any(), result)?;

        Ok(())
    }
}
