use crate::{Compiler, ResolveError, ResolveResult};
use dyst_dir::{ModuleId, Node, NodeIdAny, NodeTree, NodeType};

impl<'a> Compiler<'a> {
    /// Whether a node is resolved.
    pub(super) fn is_resolved(&self, tree: &NodeTree, node_id: NodeIdAny) -> bool {
        match node_id.ty {
            NodeType::Expression => tree
                .get::<dyst_dir::Expression>(node_id.into())
                .is_resolved(),
            NodeType::Block => tree.get::<dyst_dir::Block>(node_id.into()).is_resolved(),
            NodeType::Definition => tree
                .get::<dyst_dir::Definition>(node_id.into())
                .is_resolved(),
            NodeType::Type => tree.get::<dyst_dir::Type>(node_id.into()).is_resolved(),
            NodeType::TypeField => tree
                .get::<dyst_dir::TypeField>(node_id.into())
                .is_resolved(),
            NodeType::Property => tree.get::<dyst_dir::Property>(node_id.into()).is_resolved(),
            NodeType::EnumField => tree
                .get::<dyst_dir::EnumField>(node_id.into())
                .is_resolved(),
            NodeType::WhereClause => tree
                .get::<dyst_dir::WhereClause>(node_id.into())
                .is_resolved(),
            NodeType::WithClause => tree
                .get::<dyst_dir::WithClause>(node_id.into())
                .is_resolved(),
            NodeType::DependencyItem => tree
                .get::<dyst_dir::DependencyItem>(node_id.into())
                .is_resolved(),
            NodeType::Parameter => tree
                .get::<dyst_dir::Parameter>(node_id.into())
                .is_resolved(),
            NodeType::Argument => tree.get::<dyst_dir::Argument>(node_id.into()).is_resolved(),
            NodeType::MatchCase => tree
                .get::<dyst_dir::MatchCase>(node_id.into())
                .is_resolved(),
            NodeType::Pattern => tree.get::<dyst_dir::Pattern>(node_id.into()).is_resolved(),
            NodeType::PatternField => tree
                .get::<dyst_dir::PatternField>(node_id.into())
                .is_resolved(),
            NodeType::Annotation => tree
                .get::<dyst_dir::Annotation>(node_id.into())
                .is_resolved(),
        }
    }

    /// Resolve a module.
    pub fn resolve_module(&mut self, module_id: ModuleId) -> ResolveResult<()> {
        Ok(())
    }

    /// Resolve a node.
    pub(super) fn resolve_node(
        &mut self,
        module_id: ModuleId,
        node: NodeIdAny,
    ) -> ResolveResult<()> {
        match node.ty {
            NodeType::Expression => self.resolve_expression(module_id, node.into()),
            NodeType::Type => self.resolve_type(module_id, node.into()),
            NodeType::Argument => self.resolve_argument(module_id, node.into()),
            NodeType::DependencyItem => self.resolve_dependency_item(module_id, node.into()),
            NodeType::Annotation => self.resolve_annotation(module_id, node.into()),
            _ => Err(ResolveError::UnsupportedNode { node }),
        }
    }
}
