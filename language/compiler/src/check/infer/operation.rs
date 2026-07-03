use destack_dir as dir;
use smallvec::SmallVec;

use super::InferMode;
use crate::CompilerResult;
use crate::check::{Answer, CheckState, Constraint, FlowSite, PlaceUse, Relation, answer};

impl CheckState<'_> {
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
        let value_type = answer!(self.infer_node_type(value_site, PlaceUse::Read)?);
        let target = answer!(self.committed_node_type(target_type.into_global_any(module))?);
        self.push_constraint(Constraint::check(
            Relation::Satisfies,
            value_type,
            target,
            site.origin(),
        ));
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

        let value_type = answer!(self.infer_node_type(value_site, PlaceUse::Read)?);
        let target = answer!(self.committed_node_type(target_type.into_global_any(module))?);
        self.push_constraint(Constraint::check(
            Relation::Castable,
            value_type,
            target,
            site.origin(),
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
        let element = match bounds.as_slice() {
            [] => None,
            [single] => Some(*single),
            _ => Some(self.normalized_union_type(module, bounds)?),
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
        self.commit_node_type(node.into_any(), output)?;

        Ok(Answer::Ready(()))
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
