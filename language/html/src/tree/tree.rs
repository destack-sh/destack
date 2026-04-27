use std::fmt::{Debug, Formatter};

use crate::{
    Arena, Attribute, Content, Doctype, Document, FileId, Fragment, LocalNodeId, Node, NodeType,
    Span, StringId, StringPool,
};
use destack_core::StringRef;
use serde::{Deserialize, Serialize};

/// Dense metadata for one HTML node id.
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub(crate) struct NodeIndexEntry {
    /// The packed local id and node type.
    packed: u32,
}

impl NodeIndexEntry {
    const NODE_TYPE_SHIFT: u32 = 24;
    const LOCAL_ID_MASK: u32 = (1 << Self::NODE_TYPE_SHIFT) - 1;

    /// Pack one local id and node type into a dense entry.
    #[inline]
    pub(crate) fn new(local_id: u32, node_type: NodeType) -> Self {
        debug_assert!(
            local_id < Self::LOCAL_ID_MASK,
            "HTML node local id exceeds packed index capacity: {local_id}"
        );

        Self {
            packed: local_id | ((node_type as u32) << Self::NODE_TYPE_SHIFT),
        }
    }

    /// Return the local arena id for this entry.
    #[inline]
    pub(crate) fn local_id(self) -> u32 {
        self.packed & Self::LOCAL_ID_MASK
    }

    /// Return the concrete node type for this entry.
    #[inline]
    pub(crate) fn node_type(self) -> NodeType {
        match (self.packed >> Self::NODE_TYPE_SHIFT) as u8 {
            0 => NodeType::Document,
            1 => NodeType::Doctype,
            2 => NodeType::Fragment,
            3 => NodeType::Content,
            4 => NodeType::Attribute,
            _ => unreachable!("invalid HTML node type tag in packed node index"),
        }
    }
}

/// One side span kind.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeSpanKind {
    Name,
    Value,
}

/// One mutable HTML tree.
#[derive(Clone, Serialize, Deserialize)]
pub struct Tree {
    /// The interned HTML name strings.
    pub strings: StringPool,
    /// The next global node id.
    pub(crate) next_global_id: u32,
    /// Dense local id and node type metadata by node id.
    pub(crate) node_index_by_node_id: Vec<NodeIndexEntry>,
    /// The main source spans for all nodes.
    pub(crate) span_by_node_id: Vec<Span>,
    /// The name spans for all nodes when one exists.
    pub(crate) name_span_by_node_id: Vec<Option<Span>>,
    /// The value spans for all nodes when one exists.
    pub(crate) value_span_by_node_id: Vec<Option<Span>>,

    // node arenas
    pub(crate) documents: Arena<Document>,
    pub(crate) doctypes: Arena<Doctype>,
    pub(crate) fragments: Arena<Fragment>,
    pub(crate) nodes: Arena<Content>,
    pub(crate) attributes: Arena<Attribute>,
}

impl Debug for Tree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tree")
            .field("next_global_id", &self.next_global_id)
            .field("node_count", &self.node_index_by_node_id.len())
            .finish()
    }
}

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}

impl Tree {
    /// Create one empty tree.
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Create one empty tree with one initial capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            strings: StringPool::new(),
            next_global_id: 0,
            node_index_by_node_id: Vec::with_capacity(capacity),
            span_by_node_id: Vec::with_capacity(capacity),
            name_span_by_node_id: Vec::with_capacity(capacity),
            value_span_by_node_id: Vec::with_capacity(capacity),
            documents: Arena::new(),
            doctypes: Arena::new(),
            fragments: Arena::new(),
            nodes: Arena::new(),
            attributes: Arena::new(),
        }
    }

    /// Insert one node with one main span.
    pub fn insert<T>(&mut self, node: T, span: Span) -> LocalNodeId<T>
    where
        T: Node,
        Self: TreeImpl<T>,
    {
        let global_id = self.next_global_id;
        self.next_global_id = global_id + 1;
        let local_id = <Self as TreeImpl<T>>::allocate(self, node);
        self.node_index_by_node_id
            .push(NodeIndexEntry::new(local_id, T::TYPE));
        self.span_by_node_id.push(span);
        self.name_span_by_node_id.push(None);
        self.value_span_by_node_id.push(None);

        LocalNodeId::new(global_id)
    }

    /// Get one typed node by id.
    pub fn get<T>(&self, id: LocalNodeId<T>) -> &T
    where
        T: Node,
        Self: TreeImpl<T>,
    {
        let local_id = self.local_id_for_node_id(id.id);
        <Self as TreeImpl<T>>::get(self, local_id)
    }

    /// Get one mutable typed node by id.
    pub fn get_mut<T>(&mut self, id: LocalNodeId<T>) -> &mut T
    where
        T: Node,
        Self: TreeImpl<T>,
    {
        let local_id = self.local_id_for_node_id(id.id);
        <Self as TreeImpl<T>>::get_mut(self, local_id)
    }

    /// Return the main span for one node.
    pub fn span<T>(&self, id: LocalNodeId<T>) -> Span
    where
        T: Node,
        Self: TreeImpl<T>,
    {
        self.span_by_node_id[id.id as usize]
    }

    /// Return the main span for one untyped node.
    pub fn span_by_id(&self, id: u32) -> Span {
        self.span_by_node_id[id as usize]
    }

    /// Return the name span for one node when one exists.
    pub fn name_span<T>(&self, id: LocalNodeId<T>) -> Option<Span>
    where
        T: Node,
        Self: TreeImpl<T>,
    {
        self.name_span_by_node_id[id.id as usize]
    }

    /// Return the value span for one node when one exists.
    pub fn value_span<T>(&self, id: LocalNodeId<T>) -> Option<Span>
    where
        T: Node,
        Self: TreeImpl<T>,
    {
        self.value_span_by_node_id[id.id as usize]
    }

    /// Store one side span for one node.
    pub fn set_side_span<T>(&mut self, id: LocalNodeId<T>, kind: NodeSpanKind, span: Span)
    where
        T: Node,
        Self: TreeImpl<T>,
    {
        match kind {
            NodeSpanKind::Name => self.name_span_by_node_id[id.id as usize] = Some(span),
            NodeSpanKind::Value => self.value_span_by_node_id[id.id as usize] = Some(span),
        }
    }

    /// Store one main span for one node.
    pub fn set_span<T>(&mut self, id: LocalNodeId<T>, span: Span)
    where
        T: Node,
        Self: TreeImpl<T>,
    {
        self.span_by_node_id[id.id as usize] = span;
    }

    /// Return the type of one untyped node id.
    pub fn get_node_type(&self, id: u32) -> NodeType {
        self.node_index_by_node_id[id as usize].node_type()
    }

    /// Return the number of nodes stored in this tree.
    #[inline]
    pub fn node_count(&self) -> usize {
        self.node_index_by_node_id.len()
    }

    /// Return the local arena id for one untyped node id.
    #[inline]
    pub(crate) fn local_id_for_node_id(&self, id: u32) -> u32 {
        self.node_index_by_node_id[id as usize].local_id()
    }

    /// Intern one string in this tree.
    pub fn intern(&self, value: &str) -> StringId {
        self.strings.intern(value)
    }

    /// Read one interned string from this tree.
    pub fn string(&self, id: StringId) -> StringRef<'_> {
        self.strings.get(id)
    }

    /// Rebind every stored span to one file id.
    pub fn rebind_file(&mut self, file_id: FileId) {
        for span in &mut self.span_by_node_id {
            *span = span.with_file(file_id);
        }

        for span in &mut self.name_span_by_node_id {
            *span = span.map(|span| span.with_file(file_id));
        }

        for span in &mut self.value_span_by_node_id {
            *span = span.map(|span| span.with_file(file_id));
        }
    }
}

/// Map one node type to its arena.
pub trait TreeImpl<T: Node> {
    /// Allocate one node in the correct arena.
    fn allocate(tree: &mut Tree, node: T) -> u32;
    /// Read one node from the correct arena.
    fn get(tree: &Tree, index: u32) -> &T;
    /// Mutably read one node from the correct arena.
    fn get_mut(tree: &mut Tree, index: u32) -> &mut T;
}

macro_rules! impl_tree_store {
    ($ty:ty, $field:ident) => {
        impl TreeImpl<$ty> for Tree {
            fn allocate(tree: &mut Tree, node: $ty) -> u32 {
                tree.$field.allocate(node)
            }

            fn get(tree: &Tree, index: u32) -> &$ty {
                tree.$field.get(index)
            }

            fn get_mut(tree: &mut Tree, index: u32) -> &mut $ty {
                tree.$field.get_mut(index)
            }
        }
    };
}

impl_tree_store!(Document, documents);
impl_tree_store!(Doctype, doctypes);
impl_tree_store!(Fragment, fragments);
impl_tree_store!(Content, nodes);
impl_tree_store!(Attribute, attributes);
