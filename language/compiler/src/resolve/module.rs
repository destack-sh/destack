use crate::{Compiler, ResolveResult};
use dyst_dir::{
    Expression, LocalNodeIdAny, Module, ModuleId, Node, NodeTree, NodeType, SymbolTable,
};

#[allow(dead_code)]
impl Compiler {
    /// Whether a node is resolved.
    pub(super) fn is_resolved(&self, node_id: LocalNodeIdAny, tree: &NodeTree) -> bool {
        match node_id.ty {
            NodeType::Expression => tree
                .get::<dyst_dir::Expression>(node_id.try_into().unwrap())
                .is_resolved(),
            NodeType::Block => tree
                .get::<dyst_dir::Block>(node_id.try_into().unwrap())
                .is_resolved(),
            NodeType::Declaration => tree
                .get::<dyst_dir::Declaration>(node_id.try_into().unwrap())
                .is_resolved(),
            NodeType::Property => tree
                .get::<dyst_dir::Property>(node_id.try_into().unwrap())
                .is_resolved(),
            NodeType::EnumField => tree
                .get::<dyst_dir::EnumField>(node_id.try_into().unwrap())
                .is_resolved(),
            NodeType::WhereClause => tree
                .get::<dyst_dir::WhereClause>(node_id.try_into().unwrap())
                .is_resolved(),
            NodeType::WithClause => tree
                .get::<dyst_dir::WithClause>(node_id.try_into().unwrap())
                .is_resolved(),
            NodeType::DependencyItem => tree
                .get::<dyst_dir::DependencyItem>(node_id.try_into().unwrap())
                .is_resolved(),
            NodeType::Parameter => tree
                .get::<dyst_dir::Parameter>(node_id.try_into().unwrap())
                .is_resolved(),
            NodeType::Argument => tree
                .get::<dyst_dir::Argument>(node_id.try_into().unwrap())
                .is_resolved(),
            NodeType::MatchCase => tree
                .get::<dyst_dir::MatchCase>(node_id.try_into().unwrap())
                .is_resolved(),
            NodeType::Pattern => tree
                .get::<dyst_dir::Pattern>(node_id.try_into().unwrap())
                .is_resolved(),
            NodeType::PatternField => tree
                .get::<dyst_dir::PatternField>(node_id.try_into().unwrap())
                .is_resolved(),
            NodeType::Annotation => tree
                .get::<dyst_dir::Annotation>(node_id.try_into().unwrap())
                .is_resolved(),
        }
    }

    /// Resolve a generic node.
    pub(super) fn resolve_node(
        &self,
        module: &Module,
        node: LocalNodeIdAny,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
    ) -> ResolveResult<()> {
        match node.ty {
            NodeType::Expression => {
                self.resolve_expression(module, node.try_into().unwrap(), tree, symbols)
            }
            NodeType::Argument => {
                self.resolve_argument(module, node.try_into().unwrap(), tree, symbols)
            }
            NodeType::DependencyItem => {
                self.resolve_dependency_item(module, node.try_into().unwrap(), tree, symbols)
            }
            NodeType::PatternField => {
                self.resolve_pattern_field(module, node.try_into().unwrap(), tree, symbols)
            }
            NodeType::Annotation => {
                self.resolve_annotation(module, node.try_into().unwrap(), tree, symbols)
            }
            _ => {
                // nothing to do
                Ok(())
            }
        }
    }

    /// Resolve an entire module.
    pub fn resolve_module(&self, module_id: ModuleId) -> ResolveResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let mut tree = module.tree.write();
        let symbols = module.symbols.read();

        // resolve roots
        for expression_id in tree.iter_node_ids_of_type::<Expression>() {
            self.try_resolve(|compiler| {
                compiler.resolve_expression(&module, expression_id, &mut tree, &symbols)
            });
        }

        Ok(())
    }
}
