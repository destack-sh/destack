use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    Answer, BodyState, CheckOutcome, Decision, FlowSite, Origin, PlaceUse, Relation, ValueUse,
    answer,
};

impl BodyState<'_, '_> {
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
            let target = place.ty;
            let right_site = self.node_site(right_node)?;
            let check = answer!(self.check_expression(
                right_site,
                target,
                Relation::Assignable,
                Origin::Node(right_node, site.scope),
                ValueUse::Store,
            )?);

            // a failed store reports but the assignment still resolves
            if let CheckOutcome::Fails(failure) = check {
                let source = answer!(self.node_type_at(right_site)?);
                self.report_constraint_failure(
                    Origin::Node(right_node, site.scope),
                    Relation::Assignable,
                    Some(ValueUse::Store),
                    source,
                    target,
                    failure,
                )?;
            }
            let value = answer!(self.node_type_at(right_site)?);
            if !self.check.solver.is_probing() {
                let origin = Origin::Node(right_node, site.scope);
                self.check
                    .push_solved_constraint(origin, ValueUse::Store, value, target)?;
            }
            let _ = answer!(self.commit_assign_pattern_place(site.origin(), left_node, place)?);
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
                site.scope,
                value,
                Origin::Node(right_node, site.scope),
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

        let target_type = place.ty;

        // compound operators select through the binary operator protocol
        if let Some(operator) = operator.binary_operator() {
            let right_site = self.node_site(right_node)?;
            let right_type = answer!(self.infer_node_type(right_site, PlaceUse::Read)?);
            let left_type = answer!(self.reduce_type_head(site.origin(), target_type)?);
            let right_type =
                answer!(self.reduce_type_head(Origin::Node(right_node, site.scope), right_type)?);
            let () = answer!(self.select_binary_operation(
                site,
                operator,
                left_type,
                right_type,
                right_node,
                Some(target_type)
            )?);
            let _ = answer!(self.commit_assign_pattern_place(site.origin(), left_node, place)?);

            return Ok(Answer::Ready(()));
        }

        // non-compound update assignments store directly into the target
        let right_site = self.node_site(right_node)?;
        let check = answer!(self.check_expression(
            right_site,
            target_type,
            Relation::Assignable,
            Origin::Node(right_node, site.scope),
            ValueUse::Store,
        )?);
        if let CheckOutcome::Fails(failure) = check {
            let source = answer!(self.node_type_at(right_site)?);
            self.report_constraint_failure(
                Origin::Node(right_node, site.scope),
                Relation::Assignable,
                Some(ValueUse::Store),
                source,
                target_type,
                failure,
            )?;

            return self.reject_assignment_expression(node, left_node);
        }
        let _ = answer!(self.commit_assign_pattern_place(site.origin(), left_node, place)?);
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
