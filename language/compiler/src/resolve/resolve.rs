use crate::{Compiler, CompilerTask, ResolveError, ResolveResult};

use dyst_dir::{ModuleId, MutableNodeTree, Node, NodeIdAny, NodeType};

/// Task to statically resolve something in-place.
#[derive(Debug, Clone)]
pub enum ResolveTask {
    /// Resolve a Node.
    ResolveNode { module: ModuleId, node: NodeIdAny },
}

impl From<ResolveTask> for CompilerTask {
    fn from(task: ResolveTask) -> Self {
        CompilerTask::Resolve(task)
    }
}

impl<'a> Compiler<'a> {
    /// Generate tasks for all unresolved nodes across all modules.
    pub fn enqueue_resolve_all(&mut self) {
        let tree = self.session.tree.read();
        for node_id in tree.iter_node_ids() {
            self.enqueue_resolve_node_maybe(&tree, node_id);
        }
    }

    /// Generate tasks for all unresolved nodes in a module.
    pub fn enqueue_resolve_module(&mut self, module_id: ModuleId) {
        let tree = self.session.tree.read();
        for node_id in tree.iter_node_ids() {
            if tree.get_module_id(node_id.id) == module_id {
                self.enqueue_resolve_node_maybe(&tree, node_id);
            }
        }
    }

    /// Whether a node is resolved.
    pub(super) fn is_resolved(&self, tree: &MutableNodeTree, node_id: NodeIdAny) -> bool {
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

    /// Enqueue a node for resolution if it is unresolved.
    pub fn enqueue_resolve_node_maybe(&mut self, tree: &MutableNodeTree, node_id: NodeIdAny) {
        if !self.is_resolved(tree, node_id) {
            let module_id = tree.get_module_id(node_id.id);
            self.enqueue_resolve_node(Some(module_id), node_id);
        }
    }

    /// Queue a node for resolution. Pass in the module id if known, otherwise we lock to read.
    pub fn enqueue_resolve_node(&mut self, module_id: Option<ModuleId>, node_id: NodeIdAny) {
        let module_id = match module_id {
            Some(module_id) => module_id,
            None => self.session.tree.read().get_module_id(node_id.id),
        };
        let task = ResolveTask::ResolveNode {
            module: module_id,
            node: node_id,
        };
        self.enqueue(task.into());
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

    /// Resolve something.
    pub fn process_resolve(&mut self, task: ResolveTask) -> ResolveResult<()> {
        match task {
            ResolveTask::ResolveNode { module, node } => self.resolve_node(module, node),
        }
    }
}
