use crate::{Compiler, ResolveResult};
use dyst_dir::{Expression, LocalNodeIdAny, ModuleId, Node, NodeTree, NodeType};

#[allow(dead_code)]
impl<'a> Compiler<'a> {
    /// Whether a node is resolved.
    pub(super) fn is_resolved(&self, node_id: LocalNodeIdAny, tree: &NodeTree) -> bool {
        match node_id.ty {
            NodeType::Expression => tree
                .get::<dyst_dir::Expression>(node_id.into())
                .is_resolved(),
            NodeType::Block => tree.get::<dyst_dir::Block>(node_id.into()).is_resolved(),
            NodeType::Declaration => tree
                .get::<dyst_dir::Declaration>(node_id.into())
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

    /// Resolve a generic node.
    pub(super) fn resolve_node(
        &self,
        module_id: ModuleId,
        node: LocalNodeIdAny,
        tree: &mut NodeTree,
    ) -> ResolveResult<()> {
        match node.ty {
            NodeType::Expression => self.resolve_expression(module_id, node.into(), tree),
            NodeType::Type => self.resolve_type(module_id, node.into(), tree),
            NodeType::Argument => self.resolve_argument(module_id, node.into(), tree),
            NodeType::DependencyItem => self.resolve_dependency_item(module_id, node.into(), tree),
            NodeType::PatternField => self.resolve_pattern_field(module_id, node.into(), tree),
            NodeType::Annotation => self.resolve_annotation(module_id, node.into(), tree),
            _ => {
                // nothing to do
                Ok(())
            }
        }
    }

    /// Resolve an entire module.
    pub fn resolve_module(&self, module_id: ModuleId, tree: &mut NodeTree) -> ResolveResult<()> {
        for expression_id in tree
            .iter_node_ids_of_type_in_module::<Expression>(module_id)
            .into_iter()
        {
            self.resolve_expression(module_id, expression_id, tree)?;
        }

        Ok(())
    }
}
