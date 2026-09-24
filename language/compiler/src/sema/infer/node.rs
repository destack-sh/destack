use destack_dir as dir;

use super::InferMode;
use crate::sema::{
    CauseId, Check, CheckOutcome, CheckState, FailedCheck, FlowSite, NodeForm, PatternCheck,
    Relation, ValueCheck, ValueConversion, ValueUse,
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
    /// Where the checked value is stored.
    pub(in crate::sema) store: StoreTarget,
}

/// Where one checked value is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum StoreTarget {
    /// At the type it checks against.
    Exact,
    /// In an optional member, beside undefined.
    Optional,
}

impl Expectation {
    /// Return the target a value converts toward, absent under a cast.
    pub(in crate::sema) fn contextual_target(self) -> Option<dir::GlobalTypeId> {
        (self.use_ != ValueUse::Cast).then_some(self.target)
    }

    /// Create an assignable value expectation.
    pub(in crate::sema) fn assignable(
        target: dir::GlobalTypeId,
        cause: CauseId,
        use_: ValueUse,
    ) -> Self {
        Self {
            target,
            relation: Relation::Storable,
            cause,
            use_,
            mode: InferMode::Regular,
            store: StoreTarget::Exact,
        }
    }
}

impl CheckState<'_> {
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
                self.infer_node(site, use_, InferMode::Regular)?;

                None
            }
        };
        let ty = self.require_node_type(site.node)?;
        self.commit_expression_place(site, ty)?;

        Ok(check)
    }

    /// Check one committed node value against an expected type.
    pub(in crate::sema) fn check_value(
        &mut self,
        site: FlowSite,
        source: dir::GlobalTypeId,
        expectation: Expectation,
    ) -> CompilerResult<ValueCheck> {
        // convert the value at its expectation
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

        // commit the conversion the check selected
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
        // store a held value in an optional member beside undefined
        let mut coercion = conversion.coercion.map(|coercion| *coercion);
        if conversion.outcome == CheckOutcome::Holds && expectation.store == StoreTarget::Optional {
            let origin = site.origin();
            coercion = self.store_coercion(origin, source, coercion, conversion.target)?;
        }

        // commit the selected runtime conversion for this authored value, a cast owning it
        if let Some(mut coercion) = coercion {
            if expectation.use_ == ValueUse::Cast {
                coercion.origin = dir::CastOrigin::Explicit;
            }
            self.commit_coercion(site.node, coercion)?;
        }

        // retain a failed value check over the value the conversion related
        if let CheckOutcome::Fails(failure) = conversion.outcome {
            self.push_failure(FailedCheck {
                cause: expectation.cause,
                relation: expectation.relation,
                use_: Some(expectation.use_),
                source: conversion.source,
                target: conversion.target,
                failure,
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
        // check the node against its target
        let check = self.check_node_target(site, expectation)?;
        expectation.target = check.target;

        // preserve target-directed failures without attempting conversion
        if let CheckOutcome::Fails(failure) = check.outcome {
            let source = check.source;
            self.push_failure(FailedCheck {
                cause: expectation.cause,
                relation: expectation.relation,
                use_: Some(expectation.use_),
                source,
                target: check.target,
                failure,
            })?;

            return Ok(check);
        }

        // leave blocks and contextually typed nodes to convert inside their check
        if site.node.local_id.ty == dir::NodeType::Block || self.is_contextually_typed(site.node) {
            return Ok(check);
        }

        // resolve a cast operand structurally before the cast table judges it
        let source = check.source;
        if expectation.use_ == ValueUse::Cast {
            self.resolve_structurally(site, source)?;
        }

        self.check_value(site, source, expectation)
    }

    /// Check one destructuring pattern against its closed input.
    pub(in crate::sema) fn check_destructuring_pattern(
        &mut self,
        site: FlowSite,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let target = self.resolve_structurally(site, target)?;

        self.check_pattern(site.node.into_typed(), site.flow, site.scope, target)
    }

    /// Check one node against an expectation.
    pub(in crate::sema) fn check_node_target(
        &mut self,
        site: FlowSite,
        mut expectation: Expectation,
    ) -> CompilerResult<ValueCheck> {
        // remove inference barriers once the complete contextual type closes
        expectation.target = self.erase_inference_barriers_if_closed(expectation.target)?;

        // check by the node kind
        let node = site.node;
        let target = expectation.target;
        let mut check = match node.local_id.ty {
            dir::NodeType::Expression => self.check_expression(site, expectation)?,
            // convert a block's tail inside the block
            dir::NodeType::Block => {
                return self.check_block(site, node.into_typed().local_id, expectation);
            }
            dir::NodeType::Pattern => {
                // take an input apart once it closes
                let pattern = self
                    .module(node.module_id)
                    .view()
                    .get(node.into_typed::<dir::Pattern>().local_id)
                    .clone();
                match Self::is_destructuring_pattern(&pattern) {
                    true => match self.root_variable(target)? {
                        Some(root) if self.infer.variable(root)?.state.is_open() => {
                            self.queue_check_stalled(
                                Check::Pattern(PatternCheck { site, target }),
                                &[root],
                            )?;
                        }
                        _ => self.check_destructuring_pattern(site, target)?,
                    },
                    false => {
                        self.check_pattern(node.into_typed(), site.flow, site.scope, target)?
                    }
                }

                ValueCheck {
                    source: target,
                    outcome: CheckOutcome::Holds,
                    target,
                }
            }
            dir::NodeType::AssignPattern => {
                // resolve the input a destructuring target takes apart
                let pattern = self
                    .module(node.module_id)
                    .view()
                    .get(node.into_typed::<dir::AssignPattern>().local_id)
                    .clone();
                let target = match Self::is_destructuring_assign_pattern(&pattern) {
                    true => self.resolve_structurally(site, target)?,
                    false => target,
                };
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
            other => {
                return Err(CompilerError::Internal {
                    message: format!("cannot check {other:?} node {node:?}"),
                });
            }
        };

        // narrow the checked value at this site
        check.source = self.flow_type_at(site, check.source)?;

        Ok(check)
    }

    /// Infer one source node by its kind, returning its type through any solved variables.
    pub(in crate::sema) fn infer_node(
        &mut self,
        site: FlowSite,
        use_: PlaceUse,
        mode: InferMode,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.infer_node_in(site, use_, mode, None)
    }

    /// Infer one source node under the contextual type its site offers.
    pub(in crate::sema) fn infer_node_in(
        &mut self,
        site: FlowSite,
        use_: PlaceUse,
        mode: InferMode,
        context: Option<Expectation>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read a node typed once
        let node = site.node;
        if let Some(ty) = self.own_node_type(node) {
            return Ok(ty);
        }

        // type a function value as its callable
        if matches!(self.node_form(node), NodeForm::FunctionValue) {
            return self.function_value_type(node, context);
        }

        // infer every other node by its node family
        match node.local_id.ty {
            dir::NodeType::Expression => {
                self.infer_expression(site, use_, mode, context)?;
            }
            dir::NodeType::Block => {
                self.infer_block(site, node.into_typed().local_id)?;
            }
            dir::NodeType::TypeExpression => {}
            other => {
                return Err(CompilerError::Internal {
                    message: format!("cannot infer {other:?} node {node:?}"),
                });
            }
        }

        let Some(ty) = self.own_node_type(node) else {
            let kind = match node.local_id.ty {
                dir::NodeType::Expression => format!(
                    "{:?}",
                    self.module(node.module_id)
                        .view()
                        .get(node.into_typed::<dir::Expression>().local_id)
                ),
                _ => String::new(),
            };
            return Err(CompilerError::Internal {
                message: format!(
                    "node inference returned without publishing a type: {} {kind}",
                    self.node_label(node),
                ),
            });
        };

        self.shallow_resolve(ty)
    }

    /// Infer one source node and return its type at the same flow site.
    pub(in crate::sema) fn infer_node_type(
        &mut self,
        site: FlowSite,
        use_: PlaceUse,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // infer the node and narrow it at this site
        let ty = self.infer_node(site, use_, InferMode::Regular)?;
        let ty = self.flow_type_at(site, ty)?;
        self.commit_expression_place(site, ty)?;

        Ok(ty)
    }
}
