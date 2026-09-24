use std::collections::BTreeMap;

use destack_core::FxIndexSet as IndexSet;
use destack_source::{FileId, NodeSpanType, Span};
use smallvec::SmallVec;

use crate::{
    Decorator, DirectChildCollector, Documentation, Expression, LocalNodeId, LocalNodeIdAny, Node,
    NodeType, Origin, Patch, Path, Tree, TreeStore,
};

/// The most patches one view layers over its tree.
const MAX_PATCHES: usize = 2;

/// A borrowed DIR tree with the patches layered over it.
#[derive(Debug, Clone, Copy)]
pub struct View<'a> {
    /// The base tree.
    tree: &'a Tree,
    /// The patches over the tree, lowest first, unused entries empty.
    patches: [Option<&'a Patch>; MAX_PATCHES],
}

/// One visible node and the tree node containing its value.
#[derive(Debug, Clone, Copy)]
struct VisibleNode<'a> {
    /// The node id retained across replacements.
    id: LocalNodeIdAny,
    /// The tree containing the visible node value.
    tree: &'a Tree,
    /// The node id of the visible value.
    value_id: LocalNodeIdAny,
}

impl<'a> View<'a> {
    /// Create an unpatched view over one tree.
    pub fn new(tree: &'a Tree) -> Self {
        Self {
            tree,
            patches: [None; MAX_PATCHES],
        }
    }

    /// Layer one more patch over this view.
    pub fn patched(mut self, patch: &'a Patch) -> Self {
        let Some(entry) = self.patches.iter_mut().find(|entry| entry.is_none()) else {
            panic!("a DIR view holds at most {MAX_PATCHES} patches");
        };
        *entry = Some(patch);

        self
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
        self.patches.iter().flatten().copied()
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

    /// Get the visible concrete source extent by local node id.
    pub fn get_source_extent<T>(&self, node_id: LocalNodeId<T>) -> Option<Span>
    where
        T: Node,
    {
        self.get_source_extent_by_id(node_id.id)
    }

    /// Return the concrete source extent owned by one visible node when known.
    pub fn get_source_extent_by_id(&self, node_id: u32) -> Option<Span> {
        let node_id = self.node_id_any(node_id);
        let (tree, node_id) = self.visible_node(node_id)?;

        tree.get_source_extent_by_id(node_id.id)
    }

    /// Return a visible node span extended across attached decorators.
    pub fn get_decorated_span(&self, root: LocalNodeIdAny) -> Option<Span> {
        let mut span = self.get_span_by_id(root.id)?;

        // include decorators attached anywhere inside the selected subtree
        for decorator in self.iter_node_ids_of_type::<Decorator>() {
            let mut node = decorator.into_any();
            while let Some(parent) = self.get_parent_any(node) {
                if parent != root {
                    node = parent;
                    continue;
                }

                let decorator_span = self.get_span(decorator);
                if decorator_span.file != span.file {
                    return None;
                }
                span.start = span.start.min(decorator_span.start);
                span.end = span.end.max(decorator_span.end);
                break;
            }
        }

        Some(span)
    }

    /// Get one visible side source span by typed id.
    pub fn get_side_span<T: Node>(
        &self,
        node_id: LocalNodeId<T>,
        span_type: NodeSpanType,
    ) -> Option<Span> {
        let node_id = node_id.into_any();
        let (tree, node_id) = self.visible_node(node_id)?;

        tree.get_side_span_by_id(node_id.id, span_type)
    }

    /// Get one visible side source span by local node id.
    pub fn get_side_span_by_id(&self, node_id: u32, span_type: NodeSpanType) -> Option<Span> {
        let node_id = self.node_id_any(node_id);
        let (tree, node_id) = self.visible_node(node_id)?;

        tree.get_side_span_by_id(node_id.id, span_type)
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

    /// Return one visible node's direct children in structural order.
    pub fn direct_children(
        &self,
        node_id: LocalNodeIdAny,
    ) -> Option<SmallVec<[LocalNodeIdAny; 8]>> {
        // collect direct children from visible storage
        let (tree, visible_id) = self.visible_node(node_id)?;
        let mut collector = DirectChildCollector::default();

        // retain visible children while preserving their structural source ids
        let children = collector
            .collect(tree, visible_id)
            .iter()
            .filter_map(|child| {
                let child = self.node_id_any(*child);

                self.is_visible(child).then_some(child)
            })
            .collect();

        Some(children)
    }

    /// Return whether one node sits inside another's subtree.
    pub fn is_inside(&self, node: LocalNodeIdAny, ancestor: LocalNodeIdAny) -> bool {
        // climb parents until the ancestor or the root
        let mut current = node;
        loop {
            if current == ancestor {
                return true;
            }
            match self.get_parent_any(current) {
                Some(parent) => current = parent,
                None => return false,
            }
        }
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

    /// Return the nearest visible ancestor of one node type.
    pub fn ancestor<T: Node>(&self, node: LocalNodeIdAny) -> Option<LocalNodeId<T>> {
        let mut parent = self.get_parent_any(node);
        while let Some(current) = parent {
            if let Ok(current) = current.try_into_typed() {
                return Some(current);
            }

            parent = self.get_parent_any(current);
        }

        None
    }

    /// Return the nearest visible node of one type containing both inputs.
    pub fn common_ancestor<T: Node>(
        &self,
        left: LocalNodeIdAny,
        right: LocalNodeIdAny,
    ) -> Option<LocalNodeId<T>> {
        let mut left_ancestors = IndexSet::default();
        let mut current = Some(left);

        // collect the left lineage including the node itself
        while let Some(node) = current {
            left_ancestors.insert(node);
            current = self.get_parent_any(node);
        }

        // select the first shared typed node in the right lineage
        let mut current = Some(right);
        while let Some(node) = current {
            if left_ancestors.contains(&node)
                && let Ok(ancestor) = node.try_into_typed()
            {
                return Some(ancestor);
            }

            current = self.get_parent_any(node);
        }

        None
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

    /// Return whether one visible node is a strict descendant of another.
    pub fn is_descendant(&self, node: LocalNodeIdAny, ancestor: LocalNodeIdAny) -> bool {
        let mut parent = self.get_parent_any(node);
        while let Some(current) = parent {
            if current == ancestor {
                return true;
            }

            parent = self.get_parent_any(current);
        }

        false
    }

    /// Return the origin of one visible node.
    pub fn origin(&self, node: LocalNodeIdAny) -> Option<&'a Origin> {
        let (tree, node) = self.visible_node(node)?;

        tree.origin(node.id)
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

    /// Return the static path a visible reference expression spells.
    pub fn reference_path(&self, id: LocalNodeId<Expression>) -> Option<Path> {
        let mut segments = SmallVec::new();
        let mut current = id;

        loop {
            match self.get(current) {
                // stop at the named root
                Expression::Identifier { name } => {
                    segments.push(*name);
                    segments.reverse();

                    return Some(Path { segments });
                }

                // prepend static member segments
                Expression::Member {
                    left,
                    name: Some(name),
                    ..
                } => {
                    segments.push(*name);
                    current = *left;
                }

                // dynamic members do not spell a static path
                _ => return None,
            }
        }
    }

    /// Resolve one parsed source node to its visible node.
    pub fn get_node_id_by_source_id(&self, source_id: u32) -> Option<LocalNodeIdAny> {
        for patch in self.patches().rev() {
            let Some(node_id) = patch.tree.get_alias(source_id) else {
                continue;
            };
            if let Some((_, node_id)) = self.visible_node(node_id) {
                return Some(node_id);
            }
        }

        let node_id = self.tree.resolve_alias(source_id);

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

    /// Return whether one visible erased node carries any decorator.
    pub fn has_decorators_any(&self, node_id: LocalNodeIdAny) -> bool {
        let Some((tree, node_id)) = self.visible_node(node_id) else {
            return false;
        };

        tree.has_decorators(node_id.id)
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

    /// Iterate over visible node ids.
    pub fn iter_node_ids(&self) -> impl Iterator<Item = LocalNodeIdAny> + '_ {
        self.iter_visible_nodes().map(|node| node.id)
    }

    /// Iterate over visible nodes.
    fn iter_visible_nodes(&self) -> impl Iterator<Item = VisibleNode<'a>> + '_ {
        let replacement_targets = self
            .patches()
            .flat_map(|patch| patch.replacement_targets())
            .collect::<IndexSet<_>>();

        // preserve base ids across visible replacements
        let base = self.tree.iter_node_ids().filter_map(|id| {
            let (tree, value_id) = self.visible_node(id)?;

            Some(VisibleNode { id, tree, value_id })
        });

        // yield introduced patch nodes that do not replace a base id
        let patches = self
            .patches()
            .flat_map(|patch| patch.tree.iter_node_ids())
            .filter_map(move |id| {
                if replacement_targets.contains(&id) {
                    return None;
                }
                let (tree, value_id) = self.visible_node(id)?;

                Some(VisibleNode { id, tree, value_id })
            });

        base.chain(patches)
    }

    /// Iterate visible node ids whose source spans belong to one file.
    pub fn iter_node_ids_in_file(&self, file: FileId) -> Vec<LocalNodeIdAny> {
        self.iter_visible_nodes()
            .filter(|node| {
                node.tree
                    .get_span_by_id(node.value_id.id)
                    .is_some_and(|span| span.file == file)
            })
            .map(|node| node.id)
            .collect()
    }

    /// Iterate over visible node ids of a given type.
    pub fn iter_node_ids_of_type<T>(&self) -> impl Iterator<Item = LocalNodeId<T>> + '_
    where
        T: Node,
        Tree: TreeStore<T>,
    {
        self.iter_node_ids()
            .filter(|node_id| node_id.ty == T::TYPE)
            .map(|node_id| LocalNodeId::<T>::new(node_id.id))
    }

    /// Iterate over visible nodes of a given type together with their ids.
    pub fn iter_nodes<T>(&self) -> impl Iterator<Item = (LocalNodeId<T>, &'a T)> + '_
    where
        T: Node + 'a,
        Tree: TreeStore<T>,
    {
        self.iter_visible_nodes()
            .filter(|node| node.id.ty == T::TYPE)
            .map(|node| {
                let id = LocalNodeId::<T>::new(node.id.id);
                let value = node.tree.get(LocalNodeId::<T>::new(node.value_id.id));

                (id, value)
            })
    }

    /// Resolve one node id to its visible storage tree and node id.
    fn visible_node(&self, node_id: LocalNodeIdAny) -> Option<(&'a Tree, LocalNodeIdAny)> {
        // resolve the final visible identity
        let node_id = self.resolve_replacement(node_id)?;

        // reject detached base nodes
        if self.tree.has_node_id(node_id.id) && self.tree.is_detached(node_id.id) {
            return None;
        }

        // select the newest patch that owns the node
        for patch in self.patches().rev() {
            // skip unrelated patches
            if !patch.has_node(node_id) {
                continue;
            }

            // reject detached or unindexed patch nodes
            if patch.tree.is_detached(node_id.id) || !patch.tree.parents().contains(node_id.id) {
                return None;
            }

            return Some((&patch.tree, node_id));
        }

        // fall back to indexed base storage
        let is_indexed =
            self.tree.has_node_id(node_id.id) && self.tree.parents().contains(node_id.id);

        is_indexed.then_some((self.tree, node_id))
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
