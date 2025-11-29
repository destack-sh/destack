use crate::{Compiler, ResolveError, ResolveResult, TaskResultCollector};
use destack_dir::{
    Annotation, Argument, Block, Declaration, DependencyItem, EnumField, Expression,
    LocalNodeIdAny, MatchCase, ModuleId, Node, NodeTree, NodeType, Parameter, Pattern,
    PatternField, Property, WhereClause, WithClause,
};

#[allow(dead_code)]
impl Compiler {
    /// Whether a node is resolved.
    pub(super) fn is_resolved(&self, node_id: LocalNodeIdAny, tree: &NodeTree) -> bool {
        match node_id.ty {
            NodeType::Expression => tree
                .get::<Expression>(node_id.try_into().unwrap())
                .is_resolved(),
            NodeType::Block => tree.get::<Block>(node_id.try_into().unwrap()).is_resolved(),
            NodeType::Declaration => tree
                .get::<Declaration>(node_id.try_into().unwrap())
                .is_resolved(),
            NodeType::Property => tree
                .get::<Property>(node_id.try_into().unwrap())
                .is_resolved(),
            NodeType::EnumField => tree
                .get::<EnumField>(node_id.try_into().unwrap())
                .is_resolved(),
            NodeType::WhereClause => tree
                .get::<WhereClause>(node_id.try_into().unwrap())
                .is_resolved(),
            NodeType::WithClause => tree
                .get::<WithClause>(node_id.try_into().unwrap())
                .is_resolved(),
            NodeType::DependencyItem => tree
                .get::<DependencyItem>(node_id.try_into().unwrap())
                .is_resolved(),
            NodeType::Parameter => tree
                .get::<Parameter>(node_id.try_into().unwrap())
                .is_resolved(),
            NodeType::Argument => tree
                .get::<Argument>(node_id.try_into().unwrap())
                .is_resolved(),
            NodeType::MatchCase => tree
                .get::<MatchCase>(node_id.try_into().unwrap())
                .is_resolved(),
            NodeType::Pattern => tree
                .get::<Pattern>(node_id.try_into().unwrap())
                .is_resolved(),
            NodeType::PatternField => tree
                .get::<PatternField>(node_id.try_into().unwrap())
                .is_resolved(),
            NodeType::Annotation => tree
                .get::<Annotation>(node_id.try_into().unwrap())
                .is_resolved(),
        }
    }

    /// Resolve an entire module.
    pub fn resolve_module(&self, module_id: ModuleId) -> ResolveResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let mut tree = module.tree.write();
        let mut symbols = module.symbols.write();
        let mut collector = TaskResultCollector::new();

        // resolve expressions
        for expression_id in tree.iter_node_ids_of_type::<Expression>() {
            self.collect(
                &mut collector,
                self.resolve_expression(&module, expression_id, &mut tree, &mut symbols),
            );
        }

        // resolve dependency items
        for item_id in tree.iter_node_ids_of_type::<DependencyItem>() {
            self.collect(
                &mut collector,
                self.resolve_dependency_item(&module, item_id, &mut tree, &mut symbols),
            );
        }

        // resolve patterns
        for pattern_id in tree.iter_node_ids_of_type::<Pattern>() {
            self.collect(
                &mut collector,
                self.resolve_pattern(&module, pattern_id, &mut tree, &mut symbols),
            );
        }
        for pattern_field_id in tree.iter_node_ids_of_type::<PatternField>() {
            self.collect(
                &mut collector,
                self.resolve_pattern_field(&module, pattern_field_id, &mut tree, &mut symbols),
            );
        }

        // resolve arguments
        for argument_id in tree.iter_node_ids_of_type::<Argument>() {
            self.collect(
                &mut collector,
                self.resolve_argument(&module, argument_id, &mut tree, &mut symbols),
            );
        }

        // return combined any yield
        if let Some(dependency) = collector.try_into_yield_any() {
            return Err(ResolveError::Yield { dependency });
        }

        Ok(())
    }
}
