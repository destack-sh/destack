use destack_dir as dir;

use super::InferMode;
use crate::check::{
    Answer, BodyState, CauseId, CheckFailure, CheckOutcome, Constraint, DecisionKind, Dependency,
    FlowSite, Origin, Relation, ValueCheck, ValueSource, ValueUse, answer,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum ExpectedType {
    /// A concrete expected type.
    Type(dir::GlobalTypeId),
    /// The checked type of another source use.
    Node(dir::GlobalNodeIdAny),
}

/// Type expected for one checked expression position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct Expectation {
    /// The type expected by this check.
    pub(in crate::check) target: dir::GlobalTypeId,
    /// The relation the expression value must satisfy.
    pub(in crate::check) relation: Relation,
    /// Why this expectation exists.
    pub(in crate::check) cause: CauseId,
    /// The expected value use.
    pub(in crate::check) use_: ValueUse,
}

impl Expectation {
    /// Create an assignable value expectation.
    pub(in crate::check) fn assignable(
        target: dir::GlobalTypeId,
        cause: CauseId,
        use_: ValueUse,
    ) -> Self {
        Self {
            target,
            relation: Relation::Assignable,
            cause,
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
    /// Reduce one type head, solving an open root from its bounds first.
    pub(in crate::check) fn reduce_type_head(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let ty = self.check.settled_root(ty)?;
        if let Some(variable) = self.check.root_variable(ty)? {
            let dependency = Dependency::Variable(variable);

            return Ok(Answer::pending([dependency]));
        }

        self.check.reduce_type_head(origin, ty)
    }

    /// Attempt one node's judgment once.
    pub(in crate::check) fn attempt_node(
        &mut self,
        site: FlowSite,
        use_: PlaceUse,
        expectation: Option<Expectation>,
    ) -> CompilerResult<Answer<Checked>> {
        // check against the expectation when one shapes the node
        let holds = match expectation {
            Some(expectation) => {
                let check = answer!(self.check_node(site, expectation)?);

                matches!(check.outcome, CheckOutcome::Holds)
            }
            None => {
                let () = answer!(self.infer_node(site, use_)?);

                true
            }
        };
        let ty = answer!(self.node_type_at(site)?);

        Ok(Answer::Ready(Checked { ty, holds }))
    }

    /// Report one failed contextual node judgment.
    fn report_node_failure(
        &mut self,
        site: FlowSite,
        expectation: &Expectation,
        check: CheckOutcome,
    ) -> CompilerResult<Answer<()>> {
        // failed committed judgments report at the judgment
        if let CheckOutcome::Fails(failure) = check
            && failure != CheckFailure::Reported
        {
            let target = expectation.target;
            let source = answer!(self.node_type_at(site)?);

            // poisoned judgments already reported their cause
            if !self.check.ty(source)?.is_error() && !self.check.ty(target)?.is_error() {
                self.check.report_constraint_failure(
                    expectation.cause,
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
        cause: CauseId,
        use_: ValueUse,
    ) -> CompilerResult<Answer<ValueCheck>> {
        let expectation = Expectation {
            target,
            relation,
            cause,
            use_,
        };
        self.check_node(site, expectation)
    }

    /// Constrain one committed node value against an expected type.
    pub(in crate::check) fn constrain_node_value(
        &mut self,
        site: FlowSite,
        target: dir::GlobalTypeId,
        relation: Relation,
        cause: CauseId,
        use_: ValueUse,
    ) -> CompilerResult<Answer<ValueCheck>> {
        let expectation = Expectation {
            target,
            relation,
            cause,
            use_,
        };
        let (_, check) = answer!(self.check_node_value(site, relation, target, cause)?);

        self.complete_node_check(site, expectation, check)
    }

    /// Check one node and record its completed contextual constraint.
    fn check_node(
        &mut self,
        site: FlowSite,
        expectation: Expectation,
    ) -> CompilerResult<Answer<ValueCheck>> {
        let check = match self.check_node_target(site, expectation)? {
            Answer::Ready(check) => check,
            Answer::Pending(blockers) => {
                // park until the node has a committed type
                let Some(source) = self.check.committed_node_type(site.node) else {
                    return Ok(Answer::Pending(blockers));
                };

                // schedule the remaining source to target relation
                let source = answer!(self.check.flow_type_at(site, source)?);
                let cause = self.check.solver.cause(expectation.cause);
                let cause = self.check.intern_cause(cause.with_origin(site.origin()));
                let constraint = Constraint::value(
                    expectation.relation,
                    ValueSource::Type(source),
                    expectation.target,
                    cause,
                    expectation.use_,
                );
                self.check.push_constraint(constraint);

                return Ok(Answer::Ready(ValueCheck {
                    outcome: CheckOutcome::Holds,
                    target: expectation.target,
                }));
            }
        };

        self.complete_node_check(site, expectation, check)
    }

    /// Check one node against an expectation.
    pub(in crate::check) fn check_node_target(
        &mut self,
        site: FlowSite,
        expectation: Expectation,
    ) -> CompilerResult<Answer<ValueCheck>> {
        let node = site.node;
        let target = expectation.target;
        let outcome = match node.local_id.ty {
            dir::NodeType::Expression => return self.check_expression(site, expectation),
            dir::NodeType::Block => {
                return self.check_block(
                    site,
                    node.into_typed().local_id,
                    target,
                    expectation.relation,
                    expectation.cause,
                    expectation.use_,
                );
            }
            dir::NodeType::Pattern => {
                answer!(self.check_pattern(node.into_typed(), site.flow, site.scope, target)?);

                CheckOutcome::Holds
            }
            dir::NodeType::AssignPattern => {
                let origin = self.check.cause_origin(expectation.cause);
                answer!(self.check_assign_pattern(
                    node.into_typed(),
                    site.flow,
                    site.scope,
                    target,
                    origin,
                )?);

                CheckOutcome::Holds
            }
            dir::NodeType::TypeExpression => CheckOutcome::Holds,
            other => return self.reject_untyped_node("check", node, other),
        };

        Ok(Answer::Ready(ValueCheck { outcome, target }))
    }

    /// Complete one contextual node judgment.
    fn complete_node_check(
        &mut self,
        site: FlowSite,
        expectation: Expectation,
        check: ValueCheck,
    ) -> CompilerResult<Answer<ValueCheck>> {
        let () = answer!(self.report_node_failure(site, &expectation, check.outcome)?);

        // record the completed contextual judgment
        let constraint = Constraint::value(
            expectation.relation,
            ValueSource::Node(site.node),
            expectation.target,
            expectation.cause,
            expectation.use_,
        );
        self.check
            .record_constraint(constraint, check.outcome.state(), Some(check.target))?;

        // enclosing judgments stay silent above a reported failure
        let outcome = match check.outcome {
            CheckOutcome::Fails(_) => CheckOutcome::Fails(CheckFailure::Reported),
            outcome => outcome,
        };

        Ok(Answer::Ready(ValueCheck {
            outcome,
            target: check.target,
        }))
    }

    /// Return one node's type, checking the node in place when untyped.
    pub(in crate::check) fn node_type(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        if let Some(ty) = self.check.committed_node_type(node) {
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
        if self.committed_node_type(node).is_some() {
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
