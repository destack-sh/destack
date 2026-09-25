use tspp_dir as dir;

use crate::sema::{
    Cause, CauseKind, CheckOutcome, CheckState, Expectation, FlowSite, Origin, PlaceUse, ValueUse,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Infer one assignment expression and check the written value.
    pub(in crate::sema) fn infer_assignment_expression(
        &mut self,
        site: FlowSite,
        left: dir::LocalNodeId<dir::AssignPattern>,
        operator: dir::AssignOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // check a plain store against its target
        if operator == dir::AssignOperator::Assign {
            self.infer_plain_assignment_expression(site, left, right)
        }
        // check an update through its operator
        else {
            self.infer_update_assignment_expression(site, left, operator, right)
        }
    }

    /// Infer one plain assignment expression.
    fn infer_plain_assignment_expression(
        &mut self,
        site: FlowSite,
        left: dir::LocalNodeId<dir::AssignPattern>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // read the assignment's nodes
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let left_node = left.into_global(module);
        let right_node = right.into_global_any(module);

        // read the written pattern
        let pattern = self.module(module).view().get(left).clone();

        // check a place assignment against the selected write target
        let value = if let dir::AssignPattern::Place { expression } = &pattern {
            let expression_site = self.visit_site(expression.into_global_any(module))?;
            let Some(place) =
                self.select_assignment(expression_site, *expression, PlaceUse::Write)?
            else {
                return self.commit_rejected_assignment(node, left_node);
            };
            let target = place.write.ty();
            let store = place.store;
            let right_site = self.visit_site(right_node)?;
            let cause = self.intern_cause(Cause::root(
                Origin::Node(right_node, site.scope),
                CauseKind::Write {
                    place: expression.into_global_any(module),
                },
            ));
            let check = self.check_node(
                right_site,
                Expectation {
                    store,
                    ..Expectation::assignable(target, cause, ValueUse::Store)
                },
            )?;
            let value = check.source;
            self.commit_assign_pattern_place(site.origin(), left_node, place)?;
            self.commit_node_type(left_node.into_any(), value)?;

            // mark the written place assigned and drop stale narrowings
            if let Some(assigned) = self.assigned_place(*expression)? {
                self.assign_place(assigned);
            }
            self.clear_mutated_expression_narrowings(*expression);

            value
        } else {
            // infer the right value alone
            let right_site = self.visit_site(right_node)?;
            self.infer_node_type(right_site, PlaceUse::Read)?
        };

        // select destructuring patterns against the inferred right value
        if !matches!(pattern, dir::AssignPattern::Place { .. }) {
            // read the access of the right value
            let access = self
                .module(module)
                .decisions_tail
                .access_resolution(right_node)
                .cloned();

            // root the field projections of the pattern at the right value
            if let Some(access) = access {
                self.commit_access(left_node.into_any(), access.path().clone())?;
            }

            // select the pattern against the inferred value
            let selected = self.select_assign_pattern(
                left_node,
                site.flow,
                site.scope,
                value,
                Origin::Node(right_node, site.scope),
            )?;
            if !selected {
                return self.commit_rejected_assignment(node, left_node);
            }
            self.commit_node_type(left_node.into_any(), value)?;
        }

        // commit the value type of the assignment
        self.commit_node_type(node.into_any(), value)?;

        Ok(())
    }

    /// Infer one update assignment expression.
    fn infer_update_assignment_expression(
        &mut self,
        site: FlowSite,
        left: dir::LocalNodeId<dir::AssignPattern>,
        operator: dir::AssignOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // read the assignment's nodes
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let left_node = left.into_global(module);
        let right_node = right.into_global_any(module);

        // require a place target for update assignments
        let dir::AssignPattern::Place { expression: target } = self.module(module).view().get(left)
        else {
            self.report_invalid_assignment_target(module, left.into_any());

            return self.commit_rejected_assignment(node, left_node);
        };
        let target = *target;
        let target_site = self.visit_site(target.into_global_any(module))?;
        let Some(place) = self.select_assignment(target_site, target, PlaceUse::Update)? else {
            return self.commit_rejected_assignment(node, left_node);
        };

        // read the place's read and write types
        let read_type = place
            .read
            .as_ref()
            .map(dir::ReadResolution::ty)
            .ok_or_else(|| CompilerError::Internal {
                message: "update place has no readable type".to_string(),
            })?;
        let write_type = place.write.ty();

        // select compound operators through the binary operator protocol
        if let Ok(operator) = dir::BinaryOperator::try_from(operator) {
            let right_site = self.visit_site(right_node)?;
            let right_type = self.infer_node_type(right_site, PlaceUse::Read)?;
            let right_type = self.normalize(Origin::Node(right_node, site.scope), right_type)?;
            let () = self.select_binary_operation(
                site,
                operator,
                read_type,
                right_type,
                target.into_global_any(module),
                right_node,
                Some(write_type),
                None,
            )?;
            self.commit_assign_pattern_place(site.origin(), left_node, place)?;

            return Ok(());
        }

        // store non-compound update assignments directly into the target
        let right_site = self.visit_site(right_node)?;
        let cause = self.intern_cause(Cause::root(
            Origin::Node(right_node, site.scope),
            CauseKind::Write {
                place: target.into_global_any(module),
            },
        ));
        let check = self.check_node(
            right_site,
            Expectation::assignable(write_type, cause, ValueUse::Store),
        )?;
        if matches!(check.outcome, CheckOutcome::Fails(_)) {
            return self.commit_rejected_assignment(node, left_node);
        }
        self.commit_assign_pattern_place(site.origin(), left_node, place)?;
        self.commit_node_type(node.into_any(), write_type)?;

        Ok(())
    }

    /// Commit the rejected decision and error types for one assignment expression.
    fn commit_rejected_assignment(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        left: dir::GlobalNodeId<dir::AssignPattern>,
    ) -> CompilerResult<()> {
        // commit the rejection and its error types
        self.commit_decision(left.into_any(), dir::Decision::Rejected)?;
        let error = self.commit_error_node(node.into_any())?;
        self.commit_node_type(left.into_any(), error)?;

        Ok(())
    }
}
