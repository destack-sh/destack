use std::collections::BTreeMap;

use destack_source::Span;
use indexmap::IndexSet;

use crate::{
    Comment, Decorator, Documentation, LocalNodeId, LocalNodeIdAny, Node, NodeType, Patch, Tree,
    TreeStore,
};

/// A borrowed DIR tree with ordered structural patches.
#[derive(Debug, Clone, Copy)]
pub struct View<'a> {
    /// The base tree.
    tree: &'a Tree,
    /// Ordered structural patch slice.
    patches: &'a [Patch],
}

impl<'a> View<'a> {
    /// Create an unpatched view over one tree.
    pub fn new(tree: &'a Tree) -> Self {
        Self { tree, patches: &[] }
    }

    /// Create a view over one tree and ordered patches.
    pub fn with_patches(tree: &'a Tree, patches: &'a [Patch]) -> Self {
        Self { tree, patches }
    }

    /// Return the base tree.
    #[inline]
    pub fn tree(&self) -> &'a Tree {
        self.tree
    }

    /// Return the next visible global node id.
    #[inline]
    pub fn next_global_id(&self) -> u32 {
        if let Some(patch) = self.patches().last() {
            patch.tree.next_global_id()
        } else {
            self.tree.next_global_id()
        }
    }

    /// Return the ordered patches.
    #[inline]
    pub fn patches(&self) -> impl DoubleEndedIterator<Item = &'a Patch> + '_ {
        self.patches.iter()
    }

    /// Return whether one node is visible in this view.
    pub fn is_visible(&self, node_id: LocalNodeIdAny) -> bool {
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
        Tree: TreeStore<T>,
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

    /// Get the visible source span by typed id.
    pub fn get_span<T>(&self, node_id: LocalNodeId<T>) -> Span
    where
        T: Node,
        Tree: TreeStore<T>,
    {
        let node_id = LocalNodeIdAny::new(node_id.id, T::TYPE);
        let (tree, node_id) = self
            .visible_node(node_id)
            .unwrap_or_else(|| panic!("DIR node {node_id:?} is not visible"));

        tree.get_span(LocalNodeId::<T>::new(node_id.id))
    }

    /// Get the visible source span by local node id.
    pub fn get_span_by_id(&self, node_id: u32) -> Option<Span> {
        let node_id = self.node_id_any(node_id);
        let (tree, node_id) = self.visible_node(node_id)?;

        tree.get_span_by_id(node_id.id)
    }

    /// Get the visible parent for one local node id.
    pub fn get_parent(&self, node_id: u32) -> Option<LocalNodeIdAny> {
        let node_id = self.node_id_any(node_id);

        self.get_parent_any(node_id)
    }

    /// Get the visible parent for one typed node id.
    pub fn get_parent_for<T: Node>(&self, node_id: LocalNodeId<T>) -> Option<LocalNodeIdAny> {
        self.get_parent_any(node_id.into_any())
    }

    /// Get the visible parent for one erased node id.
    pub fn get_parent_any(&self, node_id: LocalNodeIdAny) -> Option<LocalNodeIdAny> {
        let (tree, node_id) = self.visible_node(node_id)?;

        for patch in self.patches().rev() {
            if let Some(parent) = patch.parent_for(node_id) {
                return parent
                    .and_then(|parent| self.visible_node(parent).map(|(_, parent)| parent));
            }
        }

        let parent = tree.get_parent(node_id.id)?;

        self.visible_node(parent).map(|(_, parent)| parent)
    }

    /// Get the visible parent id for one local node id.
    pub fn get_parent_id(&self, node_id: u32) -> Option<u32> {
        let node_id = self.node_id_any(node_id);

        self.get_parent_any(node_id).map(|parent| parent.id)
    }

    /// Get the visible parent id for one erased node id.
    pub fn get_parent_id_any(&self, node_id: LocalNodeIdAny) -> Option<u32> {
        self.get_parent_any(node_id).map(|parent| parent.id)
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
        for patch in self.patches().rev() {
            let Some(node_id) = patch.tree.get_alias(source_id) else {
                continue;
            };
            if let Some((_, node_id)) = self.visible_node(node_id) {
                return Some(node_id);
            }
        }

        let node_id = self.tree.get_alias(source_id)?;

        self.visible_node(node_id).map(|(_, node_id)| node_id)
    }

    /// Get normalized documentation attached to one visible typed node.
    pub fn get_documentation<T: Node>(&self, node_id: LocalNodeId<T>) -> Option<&'a Documentation> {
        self.get_documentation_any(node_id.into_any())
    }

    /// Get normalized documentation attached to one visible erased node.
    pub fn get_documentation_any(&self, node_id: LocalNodeIdAny) -> Option<&'a Documentation> {
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
            .filter(|decorator_id| self.is_visible(decorator_id.into_any()))
            .collect()
    }

    /// Return visible decorators grouped by visible node id.
    pub fn get_all_decorators(&self) -> BTreeMap<u32, Vec<LocalNodeId<Decorator>>> {
        let mut decorators_by_node_id = BTreeMap::new();

        for node_id in self.iter_node_ids() {
            let decorators = self.get_decorators_any(node_id);
            if decorators.is_empty() {
                continue;
            }

            decorators_by_node_id.insert(node_id.id, decorators);
        }

        decorators_by_node_id
    }

    /// Return source comments from the base tree.
    pub fn comments(&self) -> &'a [Comment] {
        self.tree.comments()
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

        let replacement_targets = self
            .patches()
            .flat_map(|patch| patch.replacement_targets())
            .collect::<IndexSet<_>>();

        // patch ids cover introduced nodes not represented by a base id
        for patch in self.patches() {
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
        Tree: TreeStore<T>,
    {
        self.iter_node_ids()
            .into_iter()
            .filter(|node_id| node_id.ty == T::TYPE)
            .map(|node_id| LocalNodeId::new(node_id.id))
            .collect()
    }

    /// Iterate over visible nodes of a given type together with their ids.
    pub fn iter_nodes_of_type<T>(&self) -> Vec<(LocalNodeId<T>, &'a T)>
    where
        T: Node + 'a,
        Tree: TreeStore<T>,
    {
        self.iter_node_ids_of_type::<T>()
            .into_iter()
            .map(|node_id| {
                let node = self.get(LocalNodeId::new(node_id.id));

                (node_id, node)
            })
            .collect()
    }

    /// Iterate over visible node ids of a given type.
    pub fn iter_nodes<T>(&self) -> Vec<LocalNodeId<T>>
    where
        T: Node + 'a,
        Tree: TreeStore<T>,
    {
        self.iter_node_ids_of_type::<T>()
    }

    /// Resolve one node id to its visible storage tree and node id.
    fn visible_node(&self, node_id: LocalNodeIdAny) -> Option<(&'a Tree, LocalNodeIdAny)> {
        let node_id = self.resolve_replacement(node_id)?;

        if self.tree.has_node_id(node_id.id) && self.tree.is_detached(node_id.id) {
            return None;
        }

        for patch in self.patches().rev() {
            if !patch.has_node(node_id) {
                continue;
            }
            if patch.tree.is_detached(node_id.id) {
                return None;
            }

            return Some((&patch.tree, node_id));
        }

        self.tree
            .has_node_id(node_id.id)
            .then_some((self.tree, node_id))
    }

    /// Resolve replacement chains for one node id.
    fn resolve_replacement(&self, mut node_id: LocalNodeIdAny) -> Option<LocalNodeIdAny> {
        loop {
            let mut changed = false;
            for patch in self.patches().rev() {
                if patch.is_deleted(node_id) {
                    return None;
                }
                if let Some(replacement) = patch.replacement_for(node_id) {
                    node_id = replacement;
                    changed = true;
                    break;
                }
            }
            if !changed {
                return Some(node_id);
            }
        }
    }

    /// Build one erased node id from the visible storage containing it.
    fn node_id_any(&self, node_id: u32) -> LocalNodeIdAny {
        for patch in self.patches().rev() {
            if !patch.tree.has_node_id(node_id) {
                continue;
            }
            return LocalNodeIdAny::new(node_id, patch.tree.get_node_type(node_id));
        }

        LocalNodeIdAny::new(node_id, self.tree.get_node_type(node_id))
    }
}
