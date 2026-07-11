use destack_dir as dir;

use super::InferMode;
use crate::check::{
    Answer, BodyState, BoundMode, CheckFailure, CheckOutcome, Constraint, DecisionKind, FlowSite,
    Origin, Relation, ValueUse, answer,
};
use crate::{CompilerError, CompilerResult};

/// One syntactic use of a place expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum PlaceUse {
    /// Place value is read.
    Read,
    /// Place value is replaced.
    Write,
    /// Place value is read, transformed, and replaced.
    Update,
}

/// Type expected by one checked expression position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum ExpectedType {
    /// A concrete expected type.
    Type(dir::GlobalTypeId),
    /// The checked type of another source use.
    Node(FlowSite),
}

/// Type expected for one checked expression position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct Expectation {
    /// The type expected by this check.
    pub(in crate::check) expected: ExpectedType,
    /// The relation the expression value must satisfy.
    pub(in crate::check) relation: Relation,
    /// The source that produced this expectation.
    pub(in crate::check) origin: Origin,
    /// The expected value use.
    pub(in crate::check) use_: ValueUse,
}

impl Expectation {
    /// Create an assignable value expectation.
    pub(in crate::check) fn assignable(
        target: dir::GlobalTypeId,
        origin: Origin,
        use_: ValueUse,
    ) -> Self {
        Self {
            expected: ExpectedType::Type(target),
            relation: Relation::Assignable,
            origin,
            use_,
        }
    }
}

/// One checked expression result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct Checked {
    /// The expression type.
    pub(in crate::check) ty: dir::GlobalTypeId,
    /// Whether every judgment in the expression held.
    pub(in crate::check) holds: bool,
}

impl BodyState<'_, '_> {
    /// Check one source node and return its type at its flow site.
    pub(in crate::check) fn check_node(
        &mut self,
        site: FlowSite,
        use_: PlaceUse,
        expectation: Option<Expectation>,
    ) -> CompilerResult<Checked> {
        match self.attempt_node(site, use_, expectation)? {
            Answer::Ready(checked) => return Ok(checked),
            Answer::Pending(_) => {}
        }

        // the node's judgment has no further inference to wait for
        let origin = site.origin();
        self.report_cannot_infer_node(site.node)?;
        self.commit_error_node(site.node)?;
        let ty = self.intern_type(origin.module(), dir::Type::Error)?;

        Ok(Checked { ty, holds: false })
    }

    /// Reduce one type head, solving an open root from its bounds first.
    pub(in crate::check) fn reduce_type_head(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let ty = self.check.settled_root(ty)?;
        if let Some(variable) = self.check.root_variable(ty)? {
            match self.check.solve_variable(variable, BoundMode::Strong)? {
                Answer::Ready(_) => {}
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            }
        }

        self.check.reduce_type_head(origin, ty)
    }

    /// Check one source node's committed type against a target.
    pub(in crate::check) fn check_node_value(
        &mut self,
        site: FlowSite,
        relation: Relation,
        target: dir::GlobalTypeId,
        origin: Origin,
        use_: Option<ValueUse>,
    ) -> CompilerResult<Answer<(dir::GlobalTypeId, CheckOutcome)>> {
        let source = answer!(self.node_type_at(site)?);

        // park undecidable checks for fulfillment outside probes
        match self
            .check
            .constrain_type(origin, relation, source, target)?
        {
            Answer::Ready(holds) => {
                // literal freshness completes against the value expression
                let check = self.check.complete_constraint_check(
                    site.origin(),
                    relation,
                    source,
                    target,
                    holds,
                )?;

                Ok(Answer::Ready((source, check)))
            }
            Answer::Pending(blockers) if self.check.solver.is_probing() => {
                Ok(Answer::Pending(blockers))
            }
            Answer::Pending(_) => {
                let value_origin = self.check.intern_origin(site.origin());
                let origin = self.check.intern_origin(origin);
                self.check.push_constraint(Constraint::value(
                    relation,
                    source,
                    target,
                    value_origin,
                    origin,
                    use_,
                ));

                Ok(Answer::Ready((source, CheckOutcome::Holds)))
            }
        }
    }

    /// Attempt one node's judgment once.
    fn attempt_node(
        &mut self,
        site: FlowSite,
        use_: PlaceUse,
        expectation: Option<Expectation>,
    ) -> CompilerResult<Answer<Checked>> {
        // check against the expectation when one shapes the node
        let holds = match expectation {
            Some(expectation) => {
                let check = answer!(self.check_node_kind(site, expectation)?);
                let () = answer!(self.judge_committed_outcome(site, &expectation, check)?);

                matches!(check, CheckOutcome::Holds)
            }
            None => {
                let () = answer!(self.infer_node(site, use_)?);

                true
            }
        };
        let ty = answer!(self.node_type_at(site)?);

        Ok(Answer::Ready(Checked { ty, holds }))
    }

    /// Record or report one committed expectation outcome at its judgment.
    fn judge_committed_outcome(
        &mut self,
        site: FlowSite,
        expectation: &Expectation,
        check: CheckOutcome,
    ) -> CompilerResult<Answer<()>> {
        if self.check.solver.is_probing() {
            return Ok(Answer::Ready(()));
        }

        // accepted committed judgments record their coercion pair
        if matches!(check, CheckOutcome::Holds)
            && expectation.relation == Relation::Assignable
            && let Some(source) = self.check.node_type_maybe(site.node)
        {
            let target = answer!(self.expected_type(expectation.expected)?);
            let origin = Origin::Node(site.node, site.scope);
            self.check
                .push_solved_constraint(origin, expectation.use_, source, target)?;
        }

        // failed committed judgments report at the judgment
        if let CheckOutcome::Fails(failure) = check
            && failure != CheckFailure::Reported
        {
            let target = answer!(self.expected_type(expectation.expected)?);
            let source = match self.check.node_type_maybe(site.node) {
                Some(source) => source,
                None => self.check.commit_error_node(site.node)?,
            };

            // poisoned judgments already reported their cause
            if !self.check.ty(source)?.is_error() && !self.check.ty(target)?.is_error() {
                self.check.report_constraint_failure(
                    expectation.origin,
                    expectation.relation,
                    Some(expectation.use_),
                    source,
                    target,
                    failure,
                )?;
            }
        }

        Ok(Answer::Ready(()))
    }

    /// Resolve one expectation's expected type.
    pub(in crate::check) fn expected_type(
        &mut self,
        expected: ExpectedType,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        match expected {
            ExpectedType::Type(ty) => Ok(Answer::Ready(ty)),
            ExpectedType::Node(site) => {
                let ty = answer!(self.node_type(site.node)?);

                self.flow_type_at(site, ty)
            }
        }
    }

    /// Check one node by its kind against an expectation.
    pub(in crate::check) fn check_node_kind(
        &mut self,
        site: FlowSite,
        expectation: Expectation,
    ) -> CompilerResult<Answer<CheckOutcome>> {
        let node = site.node;
        let Expectation {
            expected,
            relation,
            origin,
            use_,
        } = expectation;
        let target = answer!(self.expected_type(expected)?);

        match node.local_id.ty {
            dir::NodeType::Expression => {
                self.check_expression(site, target, relation, origin, use_)
            }
            dir::NodeType::Block => self.check_block(
                site,
                node.into_typed().local_id,
                target,
                relation,
                origin,
                use_,
            ),
            dir::NodeType::Pattern => {
                answer!(self.check_pattern(node.into_typed(), site.flow, site.scope, target)?);

                Ok(Answer::Ready(CheckOutcome::Holds))
            }
            dir::NodeType::AssignPattern => {
                answer!(self.check_assign_pattern(
                    node.into_typed(),
                    site.flow,
                    site.scope,
                    target,
                    origin,
                )?);

                Ok(Answer::Ready(CheckOutcome::Holds))
            }
            dir::NodeType::TypeExpression => Ok(Answer::Ready(CheckOutcome::Holds)),
            other => self.reject_untyped_node("check", node, other),
        }
    }

    /// Check one node in place and return its decided kind.
    pub(in crate::check) fn decide_node(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<DecisionKind>> {
        if self.decision_kind(node).is_none() {
            let site = self.node_site(node)?;
            let () = answer!(self.infer_node(site, PlaceUse::Read)?);
        }

        match self.decision_kind(node) {
            Some(kind) => Ok(Answer::Ready(kind)),
            None => Err(CompilerError::Internal {
                message: format!("node {node:?} checked without a decision"),
            }),
        }
    }

    /// Check one node against an expected type inside an enclosing judgment.
    pub(in crate::check) fn check_node_expected(
        &mut self,
        site: FlowSite,
        target: dir::GlobalTypeId,
        relation: Relation,
        origin: Origin,
        use_: ValueUse,
    ) -> CompilerResult<Answer<CheckOutcome>> {
        let expectation = Expectation {
            expected: ExpectedType::Type(target),
            relation,
            origin,
            use_,
        };
        let check = answer!(self.check_node_kind(site, expectation)?);
        let () = answer!(self.judge_committed_outcome(site, &expectation, check)?);

        // enclosing judgments stay silent above a reported failure
        let check = match check {
            CheckOutcome::Fails(_) if !self.check.solver.is_probing() => {
                CheckOutcome::Fails(CheckFailure::Reported)
            }
            check => check,
        };

        Ok(Answer::Ready(check))
    }

    /// Return one node's type, checking the node in place when untyped.
    pub(in crate::check) fn node_type(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        if let Some(ty) = self.check.node_type_maybe(node) {
            return Ok(Answer::Ready(ty));
        }

        // type the node in place
        let site = self.check.node_site(node)?;
        let () = answer!(self.infer_node(site, PlaceUse::Read)?);

        Ok(Answer::Ready(self.check.require_node_type(node)?))
    }

    /// Return one node's type at its flow site, checking it in place.
    pub(in crate::check) fn node_type_at(
        &mut self,
        site: FlowSite,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let ty = answer!(self.node_type(site.node)?);

        self.check.flow_type_at(site, ty)
    }

    /// Infer one source node by its kind.
    pub(in crate::check) fn infer_node(
        &mut self,
        site: FlowSite,
        use_: PlaceUse,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node;

        // function values check their body in place, without a context
        if self.check.lambdas.contains_key(&node) {
            let _ = answer!(self.check_function_value(node, None)?);
        }
        if self.node_type_maybe(node).is_some() {
            return Ok(Answer::Ready(()));
        }

        match node.local_id.ty {
            dir::NodeType::Expression => self.infer_expression(site, use_, InferMode::Exact),
            dir::NodeType::Block => self.infer_block(site, node.into_typed().local_id),
            dir::NodeType::TypeExpression => Ok(Answer::Ready(())),
            other => self.reject_untyped_node("infer", node, other),
        }
    }

    /// Infer one source node and return its type at the same flow site.
    pub(in crate::check) fn infer_node_type(
        &mut self,
        site: FlowSite,
        use_: PlaceUse,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        answer!(self.infer_node(site, use_)?);

        self.node_type_at(site)
    }

    /// Reject inference on a node kind that never carries a checked type.
    fn reject_untyped_node<T>(
        &self,
        verb: &'static str,
        node: dir::GlobalNodeIdAny,
        kind: dir::NodeType,
    ) -> CompilerResult<T> {
        Err(CompilerError::Internal {
            message: format!("cannot {verb} {kind:?} node {node:?}"),
        })
    }
}
