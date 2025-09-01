use std::marker::PhantomData;
use std::num::NonZeroU32;

use destack_language_token::Span;

use crate::NodeId;

impl<T> NodeId<T> {
    /// Convert this NodeId to a zero-based array index.
    #[inline]
    pub fn get(&self) -> usize {
        let val: u32 = self.idx.get() as u32;
        val as usize
    }
}

/// Arena for storing AST nodes with their associated spans.
///
/// Provides stable NodeId handles for nodes and efficient access to both
/// node data and source location information.
pub struct Arena<T> {
    nodes: Vec<T>,
    spans: Vec<Span>,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            spans: Vec::new(),
        }
    }
}

impl<T> Arena<T> {
    /// Allocate a new node in the arena with its span.
    ///
    /// Returns a stable NodeId that can be used to retrieve the node later.
    #[inline]
    pub fn alloc(&mut self, node: T, span: Span) -> NodeId<T> {
        let id = NodeId {
            idx: NonZeroU32::new(self.nodes.len() as u32).unwrap(),
            _ty: PhantomData {},
        };
        self.nodes.push(node);
        self.spans.push(span);
        id
    }

    /// Get an immutable reference to the node with the given NodeId.
    #[inline]
    pub fn get(&self, id: NodeId<T>) -> &T {
        &self.nodes[id.get()]
    }

    /// Get a mutable reference to the node with the given NodeId.
    #[inline]
    pub fn get_mut(&mut self, id: NodeId<T>) -> &mut T {
        &mut self.nodes[id.get()]
    }

    /// Get the span associated with the given NodeId.
    #[inline]
    pub fn span(&self, id: NodeId<T>) -> Span {
        self.spans[id.get()]
    }

    /// Reserve capacity for at least n additional nodes.
    #[inline]
    pub fn reserve(&mut self, n: usize) {
        self.nodes.reserve(n);
        self.spans.reserve(n);
    }
}
