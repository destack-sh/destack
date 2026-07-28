use destack_dir as dir;

use super::InferMode;
use crate::check::{
    Answer, BodyState, CauseId, CheckOutcome, DecisionKind, FlowSite, Relation, Task, ValueCheck,
    ValueConversion, ValueUse, answer,
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
    /// Literal inference applied when the target cannot contextualize the expression.
    pub(in crate::check) mode: InferMode,
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
            mode: InferMode::Exact,
        }
    }
}

impl BodyState<'_, '_> {
    /// Attempt one node's check once.
    pub(in crate::check) fn attempt_node(
        &mut self,
        site: FlowSite,
        use_: PlaceUse,
        expectation: Option<Expectation>,
    ) -> CompilerResult<Answer<Option<ValueCheck>>> {
        // check against the expectation when one shapes the node
        let check = match expectation {
            Some(expectation) => {
                let check = answer!(self.check_node(site, expectation)?);

                Some(check)
            }
            None => {
                let _ = answer!(self.infer_node(site, use_, InferMode::Exact)?);

                None
            }
        };
        let ty = self.require_node_type(site.node)?;
        answer!(self.commit_expression_place(site, ty)?);

        Ok(Answer::Ready(check))
    }

    /// Check one node in place and return its decided kind.
    pub(in crate::check) fn decide_node(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<DecisionKind>> {
        if self.decision_kind(node).is_none() {
            let site = self.node_site(node)?;
            let _ = answer!(self.infer_node(site, PlaceUse::Read, InferMode::Exact)?);
        }

        match self.decision_kind(node) {
            Some(kind) => Ok(Answer::Ready(kind)),
            None => Err(CompilerError::Internal {
                message: format!("node {node:?} checked without a decision"),
            }),
        }
    }

    /// Check one node against an expected type inside an enclosing check.
    pub(in crate::check) fn check_node_expected(
        &mut self,
        site: FlowSite,
        target: dir::GlobalTypeId,
        relation: Relation,
        cause: CauseId,
        use_: ValueUse,
        mode: InferMode,
    ) -> CompilerResult<Answer<ValueCheck>> {
        let expectation = Expectation {
            target,
            relation,
            cause,
            use_,
            mode,
        };
        self.check_node(site, expectation)
    }

    /// Check one committed node value against an expected type.
    pub(in crate::check) fn check_value(
        &mut self,
        site: FlowSite,
        source: dir::GlobalTypeId,
        expectation: Expectation,
    ) -> CompilerResult<Answer<ValueCheck>> {
        let value = answer!(self.expression_value(site, source)?);
        let conversion = self.convert_value(
            site,
            expectation.cause,
            expectation.relation,
            value,
            expectation.target,
            expectation.use_,
            expectation.mode,
        )?;

        // queue a pending conversion and report the source as checked
        let conversion = match conversion {
            Answer::Ready(conversion) => conversion,
            Answer::Pending(_) => {
                self.check.queue_task(Task::Convert {
                    site,
                    source: value,
                    expectation,
                });

                return Ok(Answer::Ready(ValueCheck {
                    source,
                    outcome: CheckOutcome::Holds,
                    target: expectation.target,
                }));
            }
        };

        let check = self.commit_value_conversion(site, source, expectation, conversion)?;

        Ok(Answer::Ready(check))
    }

    /// Commit one completed value conversion.
    pub(in crate::check) fn commit_value_conversion(
        &mut self,
        site: FlowSite,
        source: dir::GlobalTypeId,
        expectation: Expectation,
        conversion: ValueConversion,
    ) -> CompilerResult<ValueCheck> {
        // commit the selected runtime conversion for this authored value
        if let Some(coercion) = conversion.coercion {
            self.check.commit_coercion(site.node, *coercion)?;
        }

        // retain a failed confirmed value check at its authored cause
        if let CheckOutcome::Fails(failure) = conversion.outcome {
            self.check.record_failure(
                expectation.cause,
                expectation.relation,
                Some(expectation.use_),
                source,
                conversion.target,
                failure,
            );
        }

        Ok(ValueCheck {
            source,
            outcome: conversion.outcome,
            target: conversion.target,
        })
    }

    /// Check one node against its contextual target.
    pub(in crate::check) fn check_node(
        &mut self,
        site: FlowSite,
        mut expectation: Expectation,
    ) -> CompilerResult<Answer<ValueCheck>> {
        let check = answer!(self.check_node_target(site, expectation)?);
        expectation.target = check.target;

        // preserve target-directed failures without attempting conversion
        if let CheckOutcome::Fails(failure) = check.outcome {
            let source = check.source;
            self.check.record_failure(
                expectation.cause,
                expectation.relation,
                Some(expectation.use_),
                source,
                check.target,
                failure,
            );

            return Ok(Answer::Ready(check));
        }

        let source = check.source;

        self.check_value(site, source, expectation)
    }

    /// Check one node against an expectation.
    pub(in crate::check) fn check_node_target(
        &mut self,
        site: FlowSite,
        mut expectation: Expectation,
    ) -> CompilerResult<Answer<ValueCheck>> {
        // remove inference barriers once the complete contextual type closes
        if self.type_variables(expectation.target)?.is_empty() {
            expectation.target =
                self.erase_inference_barriers(expectation.target.module_id, expectation.target)?;
        }

        // wait for a barrier target to close before contextualizing
        if let Some(no_infer) = self.no_infer_target(expectation.target)? {
            let blockers = self.variable_dependencies([no_infer])?;
            if !blockers.is_empty() {
                return Ok(Answer::pending(blockers));
            }
            expectation.target = no_infer;
        }

        let node = site.node;
        let target = expectation.target;
        let mut check = match node.local_id.ty {
            dir::NodeType::Expression => answer!(self.check_expression(site, expectation)?),
            dir::NodeType::Block => {
                answer!(self.check_block(site, node.into_typed().local_id, expectation)?)
            }
            dir::NodeType::Pattern => {
                answer!(self.check_pattern(node.into_typed(), site.flow, site.scope, target)?);

                ValueCheck {
                    source: target,
                    outcome: CheckOutcome::Holds,
                    target,
                }
            }
            dir::NodeType::AssignPattern => {
                answer!(self.check_assign_pattern(
                    node.into_typed(),
                    site.flow,
                    site.scope,
                    target,
                    site.origin(),
                )?);

                ValueCheck {
                    source: target,
                    outcome: CheckOutcome::Holds,
                    target,
                }
            }
            dir::NodeType::TypeExpression => ValueCheck {
                source: target,
                outcome: CheckOutcome::Holds,
                target,
            },
            other => return self.reject_untyped_node("check", node, other),
        };
        check.source = answer!(self.flow_type_at(site, check.source)?);

        Ok(Answer::Ready(check))
    }

    /// Infer one source node by its kind.
    pub(in crate::check) fn infer_node(
        &mut self,
        site: FlowSite,
        use_: PlaceUse,
        mode: InferMode,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let node = site.node;
        if let Some(ty) = self.node_types.get(&node).copied() {
            return Ok(Answer::Ready(ty));
        }

        // check function value bodies in place, without a context
        if self.check.lambdas.contains_key(&node) {
            let check = answer!(self.check_function_value(site, None)?);

            return Ok(Answer::Ready(check.source));
        }

        // infer every other node by its syntax family
        match node.local_id.ty {
            dir::NodeType::Expression => {
                answer!(self.infer_expression(site, use_, mode)?);
            }
            dir::NodeType::Block => {
                answer!(self.infer_block(site, node.into_typed().local_id)?);
            }
            dir::NodeType::TypeExpression => {}
            other => return self.reject_untyped_node("infer", node, other),
        }

        let Some(ty) = self.node_types.get(&node).copied() else {
            return Err(CompilerError::Internal {
                message: format!(
                    "node inference returned without publishing a type: {}",
                    self.node_label(node),
                ),
            });
        };

        Ok(Answer::Ready(ty))
    }

    /// Infer one source node and return its type at the same flow site.
    pub(in crate::check) fn infer_node_type(
        &mut self,
        site: FlowSite,
        use_: PlaceUse,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let ty = answer!(self.infer_node(site, use_, InferMode::Exact)?);
        let ty = answer!(self.flow_type_at(site, ty)?);
        answer!(self.commit_expression_place(site, ty)?);

        Ok(Answer::Ready(ty))
    }

    /// Reject inference on a node kind that never has a checked type.
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
