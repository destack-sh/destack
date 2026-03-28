use std::fmt::{Debug, Formatter};

use crate::{
    Arena, Attribute, Content, Doctype, Document, FileId, Fragment, LocalNodeId, Node, NodeType,
    Span,
};
use serde::{Deserialize, Serialize};

/// One side span kind.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeSpanKind {
    Name,
    Value,
}

/// One mutable HTML node tree.
#[derive(Clone, Serialize, Deserialize)]
pub struct NodeTree {
    /// The next global node id.
    pub(crate) next_global_id: u32,
    /// The local ids of all nodes.
    pub(crate) local_id_by_node_id: Vec<u32>,
    /// The types of all nodes.
    pub(crate) node_type_by_node_id: Vec<NodeType>,
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

impl Debug for NodeTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeTree")
            .field("next_global_id", &self.next_global_id)
            .field("node_count", &self.local_id_by_node_id.len())
            .finish()
    }
}

impl Default for NodeTree {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeTree {
    /// Create one empty node tree.
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Create one empty node tree with one initial capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            next_global_id: 0,
            local_id_by_node_id: Vec::with_capacity(capacity),
            node_type_by_node_id: Vec::with_capacity(capacity),
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
        Self: NodeTreeImpl<T>,
    {
        let global_id = self.next_global_id;
        self.next_global_id = global_id + 1;
        self.node_type_by_node_id.push(T::TYPE);
        let local_id = <Self as NodeTreeImpl<T>>::allocate(self, node);
        self.local_id_by_node_id.push(local_id);
        self.span_by_node_id.push(span);
        self.name_span_by_node_id.push(None);
        self.value_span_by_node_id.push(None);

        LocalNodeId::new(global_id)
    }

    /// Get one typed node by id.
    pub fn get<T>(&self, id: LocalNodeId<T>) -> &T
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let local_id = self.local_id_by_node_id[id.id as usize];
        <Self as NodeTreeImpl<T>>::get(self, local_id)
    }

    /// Get one mutable typed node by id.
    pub fn get_mut<T>(&mut self, id: LocalNodeId<T>) -> &mut T
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        let local_id = self.local_id_by_node_id[id.id as usize];
        <Self as NodeTreeImpl<T>>::get_mut(self, local_id)
    }

    /// Return the main span for one node.
    pub fn span<T>(&self, id: LocalNodeId<T>) -> Span
    where
        T: Node,
        Self: NodeTreeImpl<T>,
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
        Self: NodeTreeImpl<T>,
    {
        self.name_span_by_node_id[id.id as usize]
    }

    /// Return the value span for one node when one exists.
    pub fn value_span<T>(&self, id: LocalNodeId<T>) -> Option<Span>
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        self.value_span_by_node_id[id.id as usize]
    }

    /// Store one side span for one node.
    pub fn set_side_span<T>(&mut self, id: LocalNodeId<T>, kind: NodeSpanKind, span: Span)
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
        match kind {
            NodeSpanKind::Name => self.name_span_by_node_id[id.id as usize] = Some(span),
            NodeSpanKind::Value => self.value_span_by_node_id[id.id as usize] = Some(span),
        }
    }

    /// Return the type of one untyped node id.
    pub fn get_node_type(&self, id: u32) -> NodeType {
        self.node_type_by_node_id[id as usize]
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
pub trait NodeTreeImpl<T: Node> {
    /// Allocate one node in the correct arena.
    fn allocate(tree: &mut NodeTree, node: T) -> u32;
    /// Read one node from the correct arena.
    fn get(tree: &NodeTree, index: u32) -> &T;
    /// Mutably read one node from the correct arena.
    fn get_mut(tree: &mut NodeTree, index: u32) -> &mut T;
}

macro_rules! impl_node_tree_store {
    ($ty:ty, $field:ident) => {
        impl NodeTreeImpl<$ty> for NodeTree {
            fn allocate(tree: &mut NodeTree, node: $ty) -> u32 {
                tree.$field.allocate(node)
            }

            fn get(tree: &NodeTree, index: u32) -> &$ty {
                tree.$field.get(index)
            }

            fn get_mut(tree: &mut NodeTree, index: u32) -> &mut $ty {
                tree.$field.get_mut(index)
            }
        }
    };
}

impl_node_tree_store!(Document, documents);
impl_node_tree_store!(Doctype, doctypes);
impl_node_tree_store!(Fragment, fragments);
impl_node_tree_store!(Content, nodes);
impl_node_tree_store!(Attribute, attributes);

#[cfg(test)]
mod tests {
    use destack_source::FileId;

    use crate::{Attribute, Content, Document, Element, Name, Namespace, NodeSpanKind, NodeTree};

    /// Store the main span and side spans for inserted nodes.
    #[test]
    fn test_insert_tracks_main_and_side_spans() {
        let mut tree = NodeTree::new();
        let attribute = tree.insert(
            Attribute {
                name: Name {
                    prefix: None,
                    namespace: Namespace::Html,
                    local: "src".to_string(),
                },
                value: Some("./asset.png".to_string()),
            },
            destack_source::Span::new(FileId::new(1), 5, 21),
        );
        tree.set_side_span(
            attribute,
            NodeSpanKind::Name,
            destack_source::Span::new(FileId::new(1), 5, 8),
        );
        tree.set_side_span(
            attribute,
            NodeSpanKind::Value,
            destack_source::Span::new(FileId::new(1), 10, 21),
        );

        assert_eq!(
            tree.span(attribute),
            destack_source::Span::new(FileId::new(1), 5, 21)
        );
        assert_eq!(
            tree.name_span(attribute),
            Some(destack_source::Span::new(FileId::new(1), 5, 8))
        );
        assert_eq!(
            tree.value_span(attribute),
            Some(destack_source::Span::new(FileId::new(1), 10, 21))
        );
    }

    /// Keep explicit template content fragments on element nodes.
    #[test]
    fn test_element_keeps_template_content_fragment() {
        let mut tree = NodeTree::new();
        let text = tree.insert(
            Content::Text(crate::Text {
                value: "fragment".to_string(),
            }),
            destack_source::Span::new(FileId::new(1), 20, 28),
        );
        let fragment = tree.insert(
            crate::Fragment {
                children: vec![text],
            },
            destack_source::Span::new(FileId::new(1), 10, 39),
        );
        let element = tree.insert(
            Content::Element(Element {
                name: Name {
                    prefix: None,
                    namespace: Namespace::Html,
                    local: "template".to_string(),
                },
                attributes: Vec::new(),
                children: Vec::new(),
                content: Some(fragment),
            }),
            destack_source::Span::new(FileId::new(1), 0, 50),
        );
        let document = tree.insert(
            Document {
                doctype: None,
                children: vec![element],
            },
            destack_source::Span::new(FileId::new(1), 0, 50),
        );

        let Content::Element(element_node) = tree.get(element) else {
            panic!("expected element node");
        };

        assert_eq!(tree.get(document).children, vec![element]);
        assert_eq!(element_node.content, Some(fragment));
    }
}
