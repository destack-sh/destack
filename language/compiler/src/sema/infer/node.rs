use destack_dir as dir;

use super::InferMode;
use crate::sema::{
    BodyState, CauseId, Check, CheckOutcome, FailedCheck, FlowSite, NodeCheck, Relation,
    ValueCheck, ValueConversion, ValueUse,
};
use crate::{CompilerError, CompilerResult};

/// One syntactic use of a place expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum PlaceUse {
    /// Place value is read.
    Read,
    /// Place value is replaced.
    Write,
    /// Place value is read, transformed, and replaced.
    Update,
}

/// Type expected by one checked expression position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum ExpectedType {
    /// A concrete expected type.
    Type(dir::GlobalTypeId),
    /// The checked type of another source use.
    Node(dir::GlobalNodeIdAny),
}

/// Type expected for one checked expression position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) struct Expectation {
    /// The type expected by this check.
    pub(in crate::sema) target: dir::GlobalTypeId,
    /// The relation the expression value must satisfy.
    pub(in crate::sema) relation: Relation,
    /// Why this expectation exists.
    pub(in crate::sema) cause: CauseId,
    /// The expected value use.
    pub(in crate::sema) use_: ValueUse,
    /// Literal inference applied when the target cannot contextualize the expression.
    pub(in crate::sema) mode: InferMode,
}

impl Expectation {
    /// Create an assignable value expectation.
    pub(in crate::sema) fn assignable(
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
    pub(in crate::sema) fn attempt_node(
        &mut self,
        site: FlowSite,
        use_: PlaceUse,
        expectation: Option<Expectation>,
    ) -> CompilerResult<Option<ValueCheck>> {
        // check against the expectation when one shapes the node
        let check = match expectation {
            Some(expectation) => {
                let check = self.check_node(site, expectation)?;

                Some(check)
            }
            None => {
                let _ = self.infer_node(site, use_, InferMode::Exact)?;

                None
            }
        };
        let ty = self.require_node_type(site.node)?;
        self.commit_expression_place(site, ty)?;

        Ok(check)
    }

    /// Check one node in place and return its decided resolution.
    pub(in crate::sema) fn decide_node(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::Decision> {
        if self.decision(node).is_none() {
            let site = self.visit_site(node)?;
            let _ = self.infer_node(site, PlaceUse::Read, InferMode::Exact)?;
        }

        match self.decision(node) {
            Some(resolution) => Ok(resolution.clone()),
            None => Err(CompilerError::Internal {
                message: format!("node {node:?} checked without a decision"),
            }),
        }
    }

    /// Check one node against an expected type inside an enclosing check.
    pub(in crate::sema) fn check_node_expected(
        &mut self,
        site: FlowSite,
        target: dir::GlobalTypeId,
        relation: Relation,
        cause: CauseId,
        use_: ValueUse,
        mode: InferMode,
    ) -> CompilerResult<ValueCheck> {
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
    pub(in crate::sema) fn check_value(
        &mut self,
        site: FlowSite,
        source: dir::GlobalTypeId,
        expectation: Expectation,
    ) -> CompilerResult<ValueCheck> {
        let value = self.expression_value(site, source)?;
        let conversion = self.convert_value(
            site,
            expectation.cause,
            expectation.relation,
            value,
            expectation.target,
            expectation.use_,
            expectation.mode,
        )?;

        let check = self.commit_value_conversion(site, source, expectation, conversion)?;

        Ok(check)
    }

    /// Commit one completed value conversion.
    pub(in crate::sema) fn commit_value_conversion(
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
            self.check.record_failure(FailedCheck {
                cause: expectation.cause,
                relation: expectation.relation,
                use_: Some(expectation.use_),
                source,
                target: conversion.target,
                failure,
                is_provisional: false,
            })?;
        }

        Ok(ValueCheck {
            source,
            outcome: conversion.outcome,
            target: conversion.target,
        })
    }

    /// Check one node against its contextual target.
    pub(in crate::sema) fn check_node(
        &mut self,
        site: FlowSite,
        mut expectation: Expectation,
    ) -> CompilerResult<ValueCheck> {
        let check = self.check_node_target(site, expectation)?;
        expectation.target = check.target;

        // preserve target-directed failures without attempting conversion
        if let CheckOutcome::Fails(failure) = check.outcome {
            let source = check.source;
            self.check.record_failure(FailedCheck {
                cause: expectation.cause,
                relation: expectation.relation,
                use_: Some(expectation.use_),
                source,
                target: check.target,
                failure,
                is_provisional: false,
            })?;

            return Ok(check);
        }

        let source = check.source;

        self.check_value(site, source, expectation)
    }

    /// Check one node against an expectation.
    pub(in crate::sema) fn check_node_target(
        &mut self,
        site: FlowSite,
        mut expectation: Expectation,
    ) -> CompilerResult<ValueCheck> {
        // remove inference barriers once the complete contextual type closes
        expectation.target = self.erase_inference_barriers_if_closed(expectation.target)?;

        // barrier targets check once their variables close, after the body
        if let Some(no_infer) = self.no_infer_target(expectation.target)? {
            let open = self.open_type_variables([no_infer])?;
            if !open.is_empty() {
                self.check
                    .register_check_stalled(Check::Node(NodeCheck { site, expectation }), &open);

                return Ok(ValueCheck {
                    source: expectation.target,
                    outcome: CheckOutcome::Holds,
                    target: expectation.target,
                });
            }
            let no_infer = self.shallow_resolve(no_infer)?;
            expectation.target = no_infer;
        }

        let node = site.node;
        let target = expectation.target;
        let mut check = match node.local_id.ty {
            dir::NodeType::Expression => self.check_expression(site, expectation)?,
            dir::NodeType::Block => {
                self.check_block(site, node.into_typed().local_id, expectation)?
            }
            dir::NodeType::Pattern => {
                self.check_pattern(node.into_typed(), site.flow, site.scope, target)?;

                ValueCheck {
                    source: target,
                    outcome: CheckOutcome::Holds,
                    target,
                }
            }
            dir::NodeType::AssignPattern => {
                self.check_assign_pattern(
                    node.into_typed(),
                    site.flow,
                    site.scope,
                    target,
                    site.origin(),
                )?;

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
        check.source = self.flow_type_at(site, check.source)?;

        Ok(check)
    }

    /// Infer one source node by its kind.
    pub(in crate::sema) fn infer_node(
        &mut self,
        site: FlowSite,
        use_: PlaceUse,
        mode: InferMode,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = site.node;
        if let Some(ty) = self.node_types.get(&node) {
            // committed holes re-infer until their node decides
            let is_hole = matches!(self.check.ty(ty)?, dir::Type::Variable(_))
                && self.check.decision(node).is_none();
            if !is_hole {
                return Ok(ty);
            }
        }

        // check function value bodies in place, without a context
        if self.check.lambdas.contains_key(&node) {
            let check = self.check_function_value(site, None, mode)?;

            return Ok(check.source);
        }

        // infer every other node by its syntax family
        match node.local_id.ty {
            dir::NodeType::Expression => {
                self.infer_expression(site, use_, mode)?;
            }
            dir::NodeType::Block => {
                self.infer_block(site, node.into_typed().local_id)?;
            }
            dir::NodeType::TypeExpression => {}
            other => return self.reject_untyped_node("infer", node, other),
        }

        let Some(ty) = self.node_types.get(&node) else {
            return Err(CompilerError::Internal {
                message: format!(
                    "node inference returned without publishing a type: {}",
                    self.node_label(node),
                ),
            });
        };

        Ok(ty)
    }

    /// Infer one source node and return its type at the same flow site.
    pub(in crate::sema) fn infer_node_type(
        &mut self,
        site: FlowSite,
        use_: PlaceUse,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let ty = self.infer_node(site, use_, InferMode::Exact)?;
        let ty = self.flow_type_at(site, ty)?;
        self.commit_expression_place(site, ty)?;

        Ok(ty)
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
