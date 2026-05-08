use indexmap::IndexSet;

use crate::{
    Decorator, LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalScopeMark, Node, NodeType, Patch,
    Tree, TreeImpl,
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

    /// Get the visible parent for one typed node id.
    pub fn get_parent<T: Node>(&self, node_id: LocalNodeId<T>) -> Option<LocalNodeIdAny> {
        self.get_parent_any(node_id.into_any())
    }

    /// Get the visible parent for one erased node id.
    pub fn get_parent_any(&self, node_id: LocalNodeIdAny) -> Option<LocalNodeIdAny> {
        let (tree, node_id) = self.visible_node(node_id)?;
        if let Some(patch) = self.patch
            && let Some(parent) = patch.parent_for(node_id)
        {
            return parent.and_then(|parent| self.visible_node(parent).map(|(_, parent)| parent));
        }

        let parent = tree.get_parent(node_id.id)?;

        self.visible_node(parent).map(|(_, parent)| parent)
    }

    /// Get the visible parent id for one typed node id.
    pub fn get_parent_id<T: Node>(&self, node_id: LocalNodeId<T>) -> Option<u32> {
        self.get_parent(node_id).map(|parent| parent.id)
    }

    /// Get the visible parent id for one erased node id.
    pub fn get_parent_id_any(&self, node_id: LocalNodeIdAny) -> Option<u32> {
        self.get_parent_any(node_id).map(|parent| parent.id)
    }

    /// Get the visible scope for one typed node id.
    pub fn get_scope<T: Node>(&self, node_id: LocalNodeId<T>) -> (LocalScopeId, LocalScopeMark) {
        self.get_scope_any(node_id.into_any())
    }

    /// Get the visible scope for one erased node id.
    pub fn get_scope_any(&self, node_id: LocalNodeIdAny) -> (LocalScopeId, LocalScopeMark) {
        let (tree, node_id) = self
            .visible_node(node_id)
            .unwrap_or_else(|| panic!("DIR node {node_id:?} is not visible"));

        tree.get_scope_any(node_id)
    }

    /// Get the source id for one visible typed node.
    pub fn get_source<T: Node>(&self, node_id: LocalNodeId<T>) -> u32 {
        self.get_source_any(node_id.into_any())
    }

    /// Get the source id for one visible erased node.
    pub fn get_source_any(&self, node_id: LocalNodeIdAny) -> u32 {
        let (tree, node_id) = self
            .visible_node(node_id)
            .unwrap_or_else(|| panic!("DIR node {node_id:?} is not visible"));

        tree.get_source(node_id.id)
    }

    /// Get the visible node id associated with one source node id.
    pub fn get_node_id_by_source_id(&self, source_id: u32) -> Option<LocalNodeIdAny> {
        let node_id = self.tree.get_node_id_by_source_id(source_id)?;

        self.visible_node(node_id).map(|(_, node_id)| node_id)
    }

    /// Get normalized documentation attached to one visible typed node.
    pub fn get_documentation<T: Node>(
        &self,
        node_id: LocalNodeId<T>,
    ) -> Option<&'a crate::Documentation> {
        self.get_documentation_any(node_id.into_any())
    }

    /// Get normalized documentation attached to one visible erased node.
    pub fn get_documentation_any(
        &self,
        node_id: LocalNodeIdAny,
    ) -> Option<&'a crate::Documentation> {
        let (tree, node_id) = self.visible_node(node_id)?;

        tree.get_documentation(node_id.id)
    }

    /// Get visible decorators attached to one typed node.
    pub fn get_decorators<T: Node>(&self, node_id: LocalNodeId<T>) -> Vec<LocalNodeId<Decorator>> {
        self.get_decorators_any(node_id.into_any())
    }

    /// Get visible decorators attached to one erased node.
    pub fn get_decorators_any(&self, node_id: LocalNodeIdAny) -> Vec<LocalNodeId<Decorator>> {
        let Some((tree, node_id)) = self.visible_node(node_id) else {
            return Vec::new();
        };

        tree.get_decorators(node_id.id)
            .into_iter()
            .filter(|decorator_id| self.is_active(decorator_id.into_any()))
            .collect()
    }

    /// Iterate over visible node ids.
    pub fn iter_node_ids(&self) -> Vec<LocalNodeIdAny> {
        let mut nodes = Vec::new();

        // base ids remain the stable ids for replaced roots
        for node_id in self.tree.iter_node_ids() {
            if self.visible_node(node_id).is_none() {
                continue;
            }

            let node_type = self.get_node_type(node_id.id);
            nodes.push(LocalNodeIdAny::new(node_id.id, node_type));
        }

        // patch ids cover introduced nodes not represented by a base id
        if let Some(patch) = self.patch {
            let replacement_targets = patch.replacement_targets().collect::<IndexSet<_>>();

            for node_id in patch.tree.iter_node_ids() {
                if replacement_targets.contains(&node_id) {
                    continue;
                }
                if self.visible_node(node_id).is_none() {
                    continue;
                }

                nodes.push(node_id);
            }
        }

        nodes
    }

    /// Iterate over visible node ids of a given type.
    pub fn iter_node_ids_of_type<T>(&self) -> Vec<LocalNodeId<T>>
    where
        T: Node,
        Tree: TreeImpl<T>,
    {
        self.iter_node_ids()
            .into_iter()
            .filter_map(|node_id| (node_id.ty == T::TYPE).then(|| LocalNodeId::new(node_id.id)))
            .collect()
    }

    /// Iterate over visible nodes of a given type together with their ids.
    pub fn iter_nodes_of_type<T>(&self) -> impl Iterator<Item = (LocalNodeId<T>, &'a T)> + '_
    where
        T: Node + 'a,
        Tree: TreeImpl<T>,
    {
        self.iter_node_ids_of_type::<T>()
            .into_iter()
            .map(|node_id| {
                let node = self.get(LocalNodeId::new(node_id.id));

                (node_id, node)
            })
    }

    /// Resolve one node id to its visible storage tree and node id.
    fn visible_node(&self, node_id: LocalNodeIdAny) -> Option<(&'a Tree, LocalNodeIdAny)> {
        if let Some(patch) = self.patch
            && patch.is_dead(node_id)
        {
            return None;
        }

        if self.tree.has_node_id(node_id.id) && self.tree.is_inactive(node_id.id) {
            return None;
        }

        if let Some(patch) = self.patch
            && let Some(replacement) = patch.replacement_for(node_id)
        {
            if patch.is_dead(replacement) {
                return None;
            }

            if patch.tree.has_node_id(replacement.id) && patch.tree.is_inactive(replacement.id) {
                return None;
            }

            return Some((&patch.tree, replacement));
        }

        if let Some(patch) = self.patch
            && patch.has_node(node_id)
        {
            if patch.tree.is_inactive(node_id.id) {
                return None;
            }

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
