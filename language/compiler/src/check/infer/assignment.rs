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
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let left_node = left.into_global(module);
        let right_node = right.into_global_any(module);

        if operator == dir::AssignOperator::Assign {
            let pattern = self.module(module).view().get(left).clone();
            let value = if let dir::AssignPattern::Place { expression } = &pattern {
                let Some(place) = answer!(self.select_assign_place(
                    site.sibling(expression.into_global_any(module)),
                    *expression,
                    PlaceUse::Write
                )?) else {
                    self.record_decision(left_node.into_any(), Decision::Rejected)?;
                    let error =
                        self.push_type(module, dir::Type::Error, node.local_id.into_any())?;
                    self.commit_node_type(left_node.into_any(), error)?;
                    self.commit_node_type(node.into_any(), error)?;

                    return Ok(Answer::Ready(()));
                };
                let target = answer!(self.select_assign_pattern_place(left_node, place)?);
                let () = answer!(self.check_expression(
                    site.sibling(right_node),
                    target,
                    Relation::Assignable,
                    Origin::Node(right_node),
                    ValueUse::Store,
                )?);
                let value = answer!(self.node_type_at(site.sibling(right_node))?);
                self.commit_node_type(left_node.into_any(), value)?;

                value
            } else {
                let () = answer!(self.infer_node(site.sibling(right_node), PlaceUse::Read)?);

                answer!(self.node_type_at(site.sibling(right_node))?)
            };

            if !matches!(pattern, dir::AssignPattern::Place { .. }) {
                let selected = answer!(self.select_assign_pattern(
                    left_node,
                    site.flow,
                    value,
                    Origin::Node(right_node),
                )?);
                if !selected {
                    let error =
                        self.push_type(module, dir::Type::Error, node.local_id.into_any())?;
                    self.commit_node_type(left_node.into_any(), error)?;
                    self.commit_node_type(node.into_any(), error)?;

                    return Ok(Answer::Ready(()));
                }
                self.commit_node_type(left_node.into_any(), value)?;
            }
            self.commit_node_type(node.into_any(), value)?;

            return Ok(Answer::Ready(()));
        }

        let dir::AssignPattern::Place { expression: target } = self.module(module).view().get(left)
        else {
            self.report_invalid_assignment_target(module, left.into_any());
            self.record_decision(left_node.into_any(), Decision::Rejected)?;
            let error = self.push_type(module, dir::Type::Error, node.local_id.into_any())?;
            self.commit_node_type(left_node.into_any(), error)?;
            self.commit_node_type(node.into_any(), error)?;

            return Ok(Answer::Ready(()));
        };
        let target = *target;
        let Some(place) = answer!(self.select_assign_place(
            site.sibling(target.into_global_any(module)),
            target,
            PlaceUse::Update
        )?) else {
            self.record_decision(left_node.into_any(), Decision::Rejected)?;
            let error = self.push_type(module, dir::Type::Error, node.local_id.into_any())?;
            self.commit_node_type(left_node.into_any(), error)?;
            self.commit_node_type(node.into_any(), error)?;

            return Ok(Answer::Ready(()));
        };

        let target_type = answer!(self.place_type(place.clone())?);
        let place_resolution = place.clone().resolution(target_type);
        let resolution = dir::AssignPatternResolution::Place(place_resolution.clone());
        self.commit_node_type(place.source, target_type)?;
        let () = answer!(self.commit_assign_pattern(left_node, resolution)?);
        self.push_obligation(Obligation::WritablePlace(WritablePlaceObligation {
            place,
            ty: target_type,
        }));

        if let Some(operator) = operator.binary_operator() {
            let () = answer!(self.infer_node(site.sibling(right_node), PlaceUse::Read)?);
            let () = answer!(self.select_binary_operator(
                site,
                operator,
                target,
                right,
                Some(target_type)
            )?);
        } else {
            let () = answer!(self.check_expression(
                site.sibling(right_node),
                target_type,
                Relation::Assignable,
                Origin::Node(right_node),
                ValueUse::Store,
            )?);
            self.commit_node_type(node.into_any(), target_type)?;
        }

        Ok(Answer::Ready(()))
    }
}
