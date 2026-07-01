use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Decision, FlowSite, Obligation, Origin, PlaceUse, Relation, ValueUse,
    WritablePlaceObligation, answer,
};

impl CheckState<'_> {
    /// Infer one assignment expression and check the written value.
    pub(in crate::check) fn infer_assignment_expression(
        &mut self,
        site: FlowSite,
        left: dir::LocalNodeId<dir::AssignPattern>,
        operator: dir::AssignOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        if operator == dir::AssignOperator::Assign {
            self.infer_plain_assignment_expression(site, left, right)
        } else {
            self.infer_update_assignment_expression(site, left, operator, right)
        }
    }

    /// Infer one plain assignment expression.
    fn infer_plain_assignment_expression(
        &mut self,
        site: FlowSite,
        left: dir::LocalNodeId<dir::AssignPattern>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let left_node = left.into_global(module);
        let right_node = right.into_global_any(module);

        // check place assignments against the selected write target
        let pattern = self.module(module).view().get(left).clone();
        let value = if let dir::AssignPattern::Place { expression } = &pattern {
            let expression_site = self.node_site(expression.into_global_any(module))?;
            let Some(place) =
                answer!(self.select_assign_place(expression_site, *expression, PlaceUse::Write)?)
            else {
                return self.reject_assignment_expression(node, left_node);
            };
            let target = answer!(self.select_assign_pattern_place(left_node, place)?);
            let right_site = self.node_site(right_node)?;
            let () = answer!(self.check_expression(
                right_site,
                target,
                Relation::Assignable,
                Origin::Node(right_node),
                ValueUse::Store,
            )?);
            let value = answer!(self.node_type_at(right_site)?);
            self.commit_node_type(left_node.into_any(), value)?;

            value
        } else {
            let right_site = self.node_site(right_node)?;
            answer!(self.infer_node_type(right_site, PlaceUse::Read)?)
        };

        // destructuring assignment selects against the inferred right value
        if !matches!(pattern, dir::AssignPattern::Place { .. }) {
            let selected = answer!(self.select_assign_pattern(
                left_node,
                site.flow,
                value,
                Origin::Node(right_node),
            )?);
            if !selected {
                return self.reject_assignment_expression(node, left_node);
            }
            self.commit_node_type(left_node.into_any(), value)?;
        }
        self.commit_node_type(node.into_any(), value)?;

        Ok(Answer::Ready(()))
    }

    /// Infer one update assignment expression.
    fn infer_update_assignment_expression(
        &mut self,
        site: FlowSite,
        left: dir::LocalNodeId<dir::AssignPattern>,
        operator: dir::AssignOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let left_node = left.into_global(module);
        let right_node = right.into_global_any(module);

        // update assignments always require a place target
        let dir::AssignPattern::Place { expression: target } = self.module(module).view().get(left)
        else {
            self.report_invalid_assignment_target(module, left.into_any());
            return self.reject_assignment_expression(node, left_node);
        };
        let target = *target;
        let target_site = self.node_site(target.into_global_any(module))?;
        let Some(place) =
            answer!(self.select_assign_place(target_site, target, PlaceUse::Update)?)
        else {
            return self.reject_assignment_expression(node, left_node);
        };

        // publish the selected place and require it to be writable
        let target_type = answer!(self.place_type(place.clone())?);
        let place_resolution = place.clone().resolution(target_type);
        let resolution = dir::AssignPatternResolution::Place(place_resolution.clone());
        self.commit_node_type(place.source, target_type)?;
        let () = answer!(self.commit_assign_pattern(left_node, resolution)?);
        self.push_obligation(Obligation::WritablePlace(WritablePlaceObligation {
            place,
            ty: target_type,
        }));

        // compound operators select through the binary operator protocol
        if let Some(operator) = operator.binary_operator() {
            let () = answer!(self.select_binary_operator(
                site,
                operator,
                target,
                right,
                Some(target_type)
            )?);

            return Ok(Answer::Ready(()));
        }

        // non-compound update assignments store directly into the target
        let right_site = self.node_site(right_node)?;
        let () = answer!(self.check_expression(
            right_site,
            target_type,
            Relation::Assignable,
            Origin::Node(right_node),
            ValueUse::Store,
        )?);
        self.commit_node_type(node.into_any(), target_type)?;

        Ok(Answer::Ready(()))
    }

    /// Reject one assignment expression and publish error types for its nodes.
    fn reject_assignment_expression(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        left: dir::GlobalNodeId<dir::AssignPattern>,
    ) -> CompilerResult<Answer<()>> {
        self.commit_decision(left.into_any(), Decision::Rejected)?;
        let error = self.commit_error_node(node.into_any())?;
        self.commit_node_type(left.into_any(), error)?;

        Ok(Answer::Ready(()))
    }
}
