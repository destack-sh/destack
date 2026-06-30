use destack_dir as dir;

use crate::check::{Answer, CheckState, FlowSite, Origin, PlaceUse, Relation, ValueUse};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Infer one source node.
    pub(in crate::check) fn infer_node(
        &mut self,
        site: FlowSite,
        use_: PlaceUse,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node;
        if self.committed_node_type_maybe(node).is_some() {
            return Ok(Answer::Ready(()));
        }

        match node.local_id.ty {
            dir::NodeType::Expression => self.infer_expression(site, use_),
            dir::NodeType::Block => self.infer_block(site, node.into_typed().local_id),
            dir::NodeType::TypeExpression => Ok(Answer::Ready(())),
            other => self.reject_untyped_node("infer", node, other),
        }
    }

    /// Check one source node against an expected type.
    pub(in crate::check) fn check_node(
        &mut self,
        site: FlowSite,
        target: dir::GlobalTypeId,
        relation: Relation,
        origin: Origin,
        use_: ValueUse,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node;
        match node.local_id.ty {
            dir::NodeType::Expression => {
                self.check_expression(site, target, relation, origin, use_)
            }
            dir::NodeType::Pattern => self.check_pattern(node.into_typed(), site.flow, target),
            dir::NodeType::AssignPattern => {
                self.check_assign_pattern(node.into_typed(), site.flow, target, origin)
            }
            dir::NodeType::TypeExpression => Ok(Answer::Ready(())),
            other => self.reject_untyped_node("check", node, other),
        }
    }

    /// Reject solver inference on a node kind that never carries a checked type.
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

impl FlowSite {
    /// Return a source use for one sibling node under the same flow point.
    pub(in crate::check) fn sibling(self, node: dir::GlobalNodeIdAny) -> Self {
        Self {
            node,
            flow: self.flow,
        }
    }
}
