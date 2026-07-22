use destack_dir as dir;
use smallvec::SmallVec;

use super::InferMode;
use crate::CompilerResult;
use crate::check::{
    Answer, BodyState, Cause, CauseKind, Constraint, Expectation, FlowSite, PlaceUse, Relation,
    TryPropagationTarget, ValueUse, VariableRole, Widening, answer,
};

impl BodyState<'_, '_> {
    /// Infer one `satisfies` expression from its value while checking the target.
    pub(in crate::check) fn infer_satisfies_expression(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let value_site = self.node_site(value.into_global_any(module))?;

        // the value checks against the target without taking its type
        let target = answer!(self.node_type(target_type.into_global_any(module))?);
        let cause = self.intern_cause(Cause::root(site.origin(), CauseKind::Expression));
        answer!(self.check_node_expected(
            value_site,
            target,
            Relation::Satisfies,
            cause,
            ValueUse::Satisfies,
        )?);
        let value_type = answer!(self.node_type_at(value_site)?);
        self.commit_node_type(node.into_any(), value_type)?;

        Ok(Answer::Ready(()))
    }

    /// Infer one cast or const assertion expression.
    pub(in crate::check) fn infer_as_expression(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let value_site = self.node_site(value.into_global_any(module))?;

        let is_const_assertion = matches!(
            self.module(module).view().get(target_type),
            dir::TypeExpression::Const
        );
        if is_const_assertion {
            answer!(self.infer_expression(value_site, PlaceUse::Read, InferMode::Const)?);
            let ty = answer!(self.node_type_at(value_site)?);
            self.commit_node_type(node.into_any(), ty)?;

            return Ok(Answer::Ready(()));
        }

        let target = answer!(self.node_type(target_type.into_global_any(module))?);

        // cast targets with open holes deduce them from their operand
        if !self.check.type_variables(target)?.is_empty()
            && self.committed_node_type(value_site.node).is_none()
        {
            let cause = self.intern_cause(Cause::root(value_site.origin(), CauseKind::Expression));
            let expectation = Expectation {
                target,
                relation: Relation::Castable,
                cause,
                use_: ValueUse::Store,
            };
            answer!(self.check_node(value_site, expectation)?);
            self.commit_node_type(node.into_any(), target)?;

            return Ok(Answer::Ready(()));
        }

        let value_type = answer!(self.infer_node_type(value_site, PlaceUse::Read)?);

        // a cast onto the operand's own settled type has no effect
        let value_root = self.check.settled_root(value_type)?;
        let target_root = self.check.settled_root(target)?;
        if value_root == target_root && self.check.type_variables(value_root)?.is_empty() {
            self.check.report_redundant_cast(
                node.into_any(),
                value.into_global_any(module),
                target,
            );
        }

        let cause = self.intern_cause(Cause::root(value_site.origin(), CauseKind::Expression));
        self.push_constraint(Constraint::r#type(
            Relation::Castable,
            value_type,
            target,
            cause,
        ));
        self.commit_node_type(node.into_any(), target)?;

        Ok(Answer::Ready(()))
    }

    /// Infer one range expression from its written bounds.
    pub(in crate::check) fn infer_range_expression(
        &mut self,
        site: FlowSite,
        start: Option<dir::LocalNodeId<dir::Expression>>,
        end: Option<dir::LocalNodeId<dir::Expression>>,
        end_kind: dir::RangeEnd,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let mut bounds = SmallVec::<[dir::GlobalTypeId; 2]>::new();
        if let Some(start) = start {
            let start_site = self.node_site(start.into_global_any(module))?;
            bounds.push(answer!(self.infer_node_type(start_site, PlaceUse::Read)?));
        }
        if let Some(end) = end {
            let end_site = self.node_site(end.into_global_any(module))?;
            bounds.push(answer!(self.infer_node_type(end_site, PlaceUse::Read)?));
        }
        // bounds flow into one element hole so context can choose the element
        let element = match bounds.as_slice() {
            [] => None,
            bounds => {
                let origin = site.origin();
                let variable =
                    self.allocate_variable(origin, Widening::Always, VariableRole::Regular);
                let element = self.variable_type(variable)?;
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
                for bound in bounds {
                    self.push_constraint(Constraint::r#type(
                        Relation::Assignable,
                        *bound,
                        element,
                        cause,
                    ));
                }

                Some(element)
            }
        };
        let item = match (start, end, end_kind) {
            (Some(_), Some(_), dir::RangeEnd::Open) => dir::LanguageItem::Range,
            (Some(_), Some(_), dir::RangeEnd::Inclusive) => dir::LanguageItem::RangeInclusive,
            (Some(_), None, _) => dir::LanguageItem::RangeFrom,
            (None, Some(_), dir::RangeEnd::Open) => dir::LanguageItem::RangeTo,
            (None, Some(_), dir::RangeEnd::Inclusive) => dir::LanguageItem::RangeToInclusive,
            (None, None, _) => dir::LanguageItem::RangeFull,
        };
        let arguments = element.into_iter().collect::<Vec<_>>();
        let range = self.language_type(module, item, &arguments)?;
        self.commit_node_type(node.into_any(), range)?;

        Ok(Answer::Ready(()))
    }

    /// Infer one try projection expression from its carrier value.
    pub(in crate::check) fn infer_try_projection_expression(
        &mut self,
        site: FlowSite,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let value_site = self.node_site(value.into_global_any(node.module_id))?;
        let value = answer!(self.infer_node_type(value_site, PlaceUse::Read)?);
        let output = answer!(
            self.reduce_operation_type(site.origin(), dir::TypeOperation::TryOutput { value },)?
        );
        self.propagate_try_residual(node.into_any(), value, site)?;
        self.commit_node_type(node.into_any(), output)?;

        Ok(Answer::Ready(()))
    }

    /// Queue one try residual against its recorded propagation target.
    pub(in crate::check) fn propagate_try_residual(
        &mut self,
        node: dir::GlobalNodeIdAny,
        value: dir::GlobalTypeId,
        site: FlowSite,
    ) -> CompilerResult<()> {
        let Some(target) = self.check.try_propagations.get(&node).copied() else {
            return Ok(());
        };
        let module = node.module_id;
        let origin = site.origin();
        let residual = self.intern_operation(module, dir::TypeOperation::TryResidual { value })?;
        match target {
            TryPropagationTarget::Failure { ty } => {
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
                self.push_constraint(Constraint::r#type(
                    Relation::Assignable,
                    residual,
                    ty,
                    cause,
                ));
            }
            TryPropagationTarget::Return { ty: Some(ret) } => {
                let symbol = self
                    .check
                    .language_symbol(dir::LanguageItem::FromResidual)?;
                let arguments = self.check.intern_type_ids(module, &[residual])?;
                let target = self.intern_type(
                    module,
                    dir::Type::Instance(dir::GenericInstance { symbol, arguments }),
                )?;
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
                self.push_constraint(Constraint::r#type(Relation::Implements, ret, target, cause));
            }
            TryPropagationTarget::Return { ty: None } => {
                self.check.report_try_outside_function(node);
            }
        }

        Ok(())
    }

    /// Infer one awaited expression through the awaited type operation.
    pub(in crate::check) fn infer_await_expression(
        &mut self,
        site: FlowSite,
        awaited: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let origin = site.origin();
        let awaited_site = self.node_site(awaited.into_global_any(module))?;
        let value = answer!(self.infer_node_type(awaited_site, PlaceUse::Read)?);
        let result = answer!(self.reduce_operation_type(
            origin,
            dir::TypeOperation::Awaited(dir::UnaryType { target: value }),
        )?);
        self.commit_node_type(node.into_any(), result)?;

        Ok(Answer::Ready(()))
    }
}
