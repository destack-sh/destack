use tspp_dir as dir;

use crate::lower::FunctionLowerer;
use crate::{CompilerError, CompilerResult};

impl FunctionLowerer<'_, '_, '_> {
    /// Return the type behind one expression node.
    pub(in crate::lower) fn node_type(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::Type> {
        self.lower.ty(self.node_type_id(expression)?)
    }

    /// Return the type of one node in this function's module.
    pub(in crate::lower) fn node_type_id(
        &self,
        node: impl Into<dir::LocalNodeIdAny>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read the node's committed type through the enclosing instance
        let node = node.into().into_global(self.source);
        let ty =
            self.source()
                .types
                .get_node_type_id(node)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("a missing type for node {}", node.local_id.id),
                })?;

        Ok(ty)
    }

    /// Return the call resolution of one applying expression.
    pub(in crate::lower) fn call_decision(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::CallDecision> {
        let node = expression.into_global_any(self.source);

        self.source()
            .decisions
            .call_decision(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!("a missing call resolution for node {}", node.local_id.id),
            })
    }

    /// Return the construct resolution of one call expression.
    pub(in crate::lower) fn construct_decision(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::ConstructDecision> {
        self.source()
            .decisions
            .construct_decision(expression.into_global_any(self.source))
            .cloned()
    }

    /// Return the tree resolution of one tree expression.
    pub(in crate::lower) fn tree_decision(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::TreeDecision> {
        let node = expression.into_global_any(self.source);

        self.source()
            .decisions
            .tree_decision(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!("a missing tree resolution for node {}", node.local_id.id),
            })
    }

    /// Return the assignment resolution of one target expression.
    pub(in crate::lower) fn assignment_decision(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::AssignmentDecision> {
        let node = expression.into_global_any(self.source);

        self.source()
            .decisions
            .assignment_decision(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "a missing assignment resolution for node {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the resolution of one assignment pattern.
    pub(in crate::lower) fn assign_resolution(
        &self,
        pattern: dir::LocalNodeId<dir::AssignPattern>,
    ) -> CompilerResult<dir::AssignPatternDecision> {
        let node = pattern.into_global_any(self.source);

        self.source()
            .decisions
            .assign_pattern_decision(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "a missing resolution for assignment pattern {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the member resolution of one member expression.
    pub(in crate::lower) fn member_decision(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::MemberDecision> {
        let node = expression.into_global_any(self.source);

        self.source()
            .decisions
            .member_decision(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "a missing member resolution for node {} of {}",
                    node.local_id.id, self.source,
                ),
            })
    }

    /// Return the subscript resolution of one index expression.
    pub(in crate::lower) fn subscript_decision(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::SubscriptDecision> {
        let node = expression.into_global_any(self.source);

        self.source()
            .decisions
            .subscript_decision(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "a missing subscript resolution for node {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the resolution of one pattern node.
    pub(in crate::lower) fn pattern_decision(
        &self,
        pattern: dir::LocalNodeId<dir::Pattern>,
    ) -> CompilerResult<dir::PatternDecision> {
        let node = pattern.into_global_any(self.source);

        self.source()
            .decisions
            .pattern_decision(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!("a missing pattern resolution for node {}", node.local_id.id),
            })
    }

    /// Return the operator resolution of one applying node.
    pub(in crate::lower) fn operator_decision<T: dir::Node>(
        &self,
        node: dir::LocalNodeId<T>,
    ) -> CompilerResult<dir::OperatorDecision> {
        let node = node.into_global_any(self.source);

        self.source()
            .decisions
            .operator_decision(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "a missing operator resolution for node {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the representation one expression lowers at.
    pub(in crate::lower) fn representation_type_id(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // take the materialized target a widening coercion names
        if let Some(coercion) = self.coercion(expression)
            && let Some(dir::CoercionAdjustment::Materialize { target }) =
                coercion.adjustments.first()
        {
            return Ok(*target);
        }

        self.node_type_id(expression)
    }

    /// Return the coercion of one authored value.
    pub(in crate::lower) fn coercion(
        &self,
        node: impl Into<dir::LocalNodeIdAny>,
    ) -> Option<dir::Coercion> {
        self.source()
            .coercions
            .coercion(node.into().into_global(self.source))
            .cloned()
    }
}
