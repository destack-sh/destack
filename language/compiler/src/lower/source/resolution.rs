use destack_dir as dir;

use crate::lower::FunctionLowerer;
use crate::{CompilerError, CompilerResult};

impl FunctionLowerer<'_, '_, '_> {
    /// Return the type behind one expression node.
    pub(in crate::lower) fn node_type(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::Type> {
        self.lowerer.ty(self.node_type_id(expression)?)
    }

    /// Return the type id behind one expression node.
    pub(in crate::lower) fn node_type_id(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = expression.into_global_any(self.source);

        self.source()
            .types
            .get_reduced_node_type_id(node)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("missing a type for node {}", node.local_id.id),
            })
    }

    /// Return the written expectation one expression was checked against.
    pub(in crate::lower) fn expected_type_id(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalTypeId> {
        let node = expression.into_global_any(self.source);

        self.source().types.get_expected_type_id(node)
    }
}

impl FunctionLowerer<'_, '_, '_> {
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
                message: format!("missing a call resolution for node {}", node.local_id.id),
            })
    }

    /// Return the construct resolution on one call expression.
    pub(in crate::lower) fn construct_decision(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::ConstructDecision> {
        self.source()
            .decisions
            .construct_decision(expression.into_global_any(self.source))
            .cloned()
    }

    /// Return the tree resolution on one tree expression.
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
                message: format!("missing a tree resolution for node {}", node.local_id.id),
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
                    "missing an assignment resolution for node {}",
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
                    "missing a resolution for assignment pattern {}",
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
                    "missing a member resolution for node {} of {}: {:?}",
                    node.local_id.id,
                    self.source,
                    self.source().tree().get(expression),
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
                    "missing a subscript resolution for node {}",
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
                message: format!("missing a pattern resolution for node {}", node.local_id.id),
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
                    "missing an operator resolution for node {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the coercion for one expression.
    pub(in crate::lower) fn coercion(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::Coercion> {
        self.source()
            .coercions
            .coercion(expression.into_global_any(self.source))
            .cloned()
    }
}
