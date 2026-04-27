use crate::{
    LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalScopeMark, Node, NodeType, Patch, Tree,
    TreeImpl,
};

/// A borrowed DIR tree with an optional structural patch.
#[derive(Debug, Clone, Copy)]
pub struct View<'a> {
    /// The base tree.
    tree: &'a Tree,
    /// The optional structural patch.
    patch: Option<&'a Patch>,
}

impl<'a> View<'a> {
    /// Create an unpatched view over one tree.
    pub fn new(tree: &'a Tree) -> Self {
        Self { tree, patch: None }
    }

    /// Create a patched view over one tree.
    pub fn patched(tree: &'a Tree, patch: &'a Patch) -> Self {
        assert!(
            patch.applies_to(tree),
            "DIR patch does not apply to base tree"
        );

        Self {
            tree,
            patch: Some(patch),
        }
    }

    /// Return the base tree.
    #[inline]
    pub fn tree(&self) -> &'a Tree {
        self.tree
    }

    /// Return the optional patch.
    #[inline]
    pub fn patch(&self) -> Option<&'a Patch> {
        self.patch
    }

    /// Return whether one node is visible in this view.
    pub fn is_active(&self, node_id: LocalNodeIdAny) -> bool {
        self.visible_node(node_id).is_some()
    }

    /// Return the visible node type for one node id.
    pub fn get_node_type(&self, node_id: u32) -> NodeType {
        let node_id = self.node_id_any(node_id);
        let (_, node_id) = self
            .visible_node(node_id)
            .unwrap_or_else(|| panic!("DIR node {node_id:?} is not visible"));

        node_id.ty
    }

    /// Get a visible node by typed id.
    pub fn get<T>(&self, node_id: LocalNodeId<T>) -> &'a T
    where
        T: Node,
        Tree: TreeImpl<T>,
    {
        let node_id = LocalNodeIdAny::new(node_id.id, T::TYPE);
        let (tree, node_id) = self
            .visible_node(node_id)
            .unwrap_or_else(|| panic!("DIR node {node_id:?} is not visible"));
        assert_eq!(
            node_id.ty,
            T::TYPE,
            "DIR node {node_id:?} has unexpected visible type"
        );

        tree.get(LocalNodeId::new(node_id.id))
    }

    /// Get the visible parent for one node id.
    pub fn get_parent(&self, node_id: LocalNodeIdAny) -> Option<LocalNodeIdAny> {
        let (tree, node_id) = self.visible_node(node_id)?;
        if let Some(patch) = self.patch
            && let Some(parent) = patch.parent_for(node_id)
        {
            return parent.and_then(|parent| self.visible_node(parent).map(|(_, parent)| parent));
        }

        let parent = tree.get_parent(node_id.id)?;

        self.visible_node(parent).map(|(_, parent)| parent)
    }

    /// Get the visible scope for one node id.
    pub fn get_scope(&self, node_id: LocalNodeIdAny) -> (LocalScopeId, LocalScopeMark) {
        let (tree, node_id) = self
            .visible_node(node_id)
            .unwrap_or_else(|| panic!("DIR node {node_id:?} is not visible"));

        tree.get_scope_any(node_id)
    }

    /// Get the source id for one visible node.
    pub fn get_source(&self, node_id: LocalNodeIdAny) -> u32 {
        let (tree, node_id) = self
            .visible_node(node_id)
            .unwrap_or_else(|| panic!("DIR node {node_id:?} is not visible"));

        tree.get_source(node_id.id)
    }

    /// Resolve one node id to its visible storage tree and node id.
    fn visible_node(&self, node_id: LocalNodeIdAny) -> Option<(&'a Tree, LocalNodeIdAny)> {
        if let Some(patch) = self.patch
            && patch.is_dead(node_id)
        {
            return None;
        }

        if let Some(patch) = self.patch
            && let Some(replacement) = patch.replacement_for(node_id)
        {
            if patch.is_dead(replacement) {
                return None;
            }

            return Some((&patch.tree, replacement));
        }

        if let Some(patch) = self.patch
            && patch.has_node(node_id)
        {
            return Some((&patch.tree, node_id));
        }

        self.tree
            .has_node_id(node_id.id)
            .then_some((self.tree, node_id))
    }

    /// Build one erased node id from the visible storage containing it.
    fn node_id_any(&self, node_id: u32) -> LocalNodeIdAny {
        if let Some(patch) = self.patch
            && patch.tree.has_node_id(node_id)
        {
            return LocalNodeIdAny::new(node_id, patch.tree.get_node_type(node_id));
        }

        LocalNodeIdAny::new(node_id, self.tree.get_node_type(node_id))
    }
}
