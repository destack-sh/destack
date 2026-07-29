use destack_dir as dir;

use crate::lower::FunctionLowerer;
use crate::{CompilerError, CompilerResult};

impl FunctionLowerer<'_, '_, '_> {
    /// Return the checked type behind one expression node.
    pub(in crate::lower) fn node_type(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::Type> {
        self.lowerer.ty(self.node_type_id(expression)?)
    }

    /// Return the checked type id behind one expression node.
    pub(in crate::lower) fn node_type_id(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = expression.into_global_any(self.source);

        self.source()
            .types
            .get_reduced_node_type_id(node)
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing a type for node {}",
                    node.local_id.id
                ),
            })
    }
}

impl FunctionLowerer<'_, '_, '_> {
    /// Return the checked call resolution of one applying expression.
    pub(in crate::lower) fn call_resolution(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::CallResolution> {
        let node = expression.into_global_any(self.source);

        self.source()
            .resolutions
            .call_resolution(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing a call resolution for node {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the checked construct resolution on one call expression.
    pub(in crate::lower) fn construct_resolution(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::ConstructResolution> {
        self.source()
            .resolutions
            .construct_resolution(expression.into_global_any(self.source))
            .cloned()
    }

    /// Return the checked assignment resolution of one target expression.
    pub(in crate::lower) fn assignment_resolution(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::AssignmentResolution> {
        let node = expression.into_global_any(self.source);

        self.source()
            .resolutions
            .assignment_resolution(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing an assignment resolution for node {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the checked resolution of one assignment pattern.
    pub(in crate::lower) fn assign_resolution(
        &self,
        pattern: dir::LocalNodeId<dir::AssignPattern>,
    ) -> CompilerResult<dir::AssignPatternResolution> {
        let node = pattern.into_global_any(self.source);

        self.source()
            .resolutions
            .assign_pattern_resolution(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing a resolution for assignment pattern {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the checked member resolution of one member expression.
    pub(in crate::lower) fn member_resolution(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::MemberResolution> {
        let node = expression.into_global_any(self.source);

        self.source()
            .resolutions
            .member_resolution(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing a member resolution for node {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the checked subscript resolution of one index expression.
    pub(in crate::lower) fn subscript_resolution(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::SubscriptResolution> {
        let node = expression.into_global_any(self.source);

        self.source()
            .resolutions
            .subscript_resolution(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing a subscript resolution for node {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the checked resolution of one pattern node.
    pub(in crate::lower) fn pattern_resolution(
        &self,
        pattern: dir::LocalNodeId<dir::Pattern>,
    ) -> CompilerResult<dir::PatternResolution> {
        let node = pattern.into_global_any(self.source);

        self.source()
            .resolutions
            .pattern_resolution(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing a pattern resolution for node {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the checked operator resolution of one applying node.
    pub(in crate::lower) fn operator_resolution<T: dir::Node>(
        &self,
        node: dir::LocalNodeId<T>,
    ) -> CompilerResult<dir::OperatorResolution> {
        let node = node.into_global_any(self.source);

        self.source()
            .resolutions
            .operator_resolution(node)
            .cloned()
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked DIR is missing an operator resolution for node {}",
                    node.local_id.id
                ),
            })
    }

    /// Return the checked coercion for one expression.
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
