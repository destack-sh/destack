use std::slice;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// The child count per vector branch.
const VECTOR_WIDTH: usize = 32;

/// One branch-shared indexed sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Vector<T> {
    /// The logical element count.
    len: usize,
    /// The root depth in directory levels.
    depth: usize,
    /// The shared root node for this sequence.
    root: Option<Arc<Node<T>>>,
}

/// One indexed sequence node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum Node<T> {
    /// One leaf chunk of sequence values.
    Leaf {
        /// The leaf values stored in this chunk.
        values: Box<[T]>,
    },
    /// One branch of child nodes.
    Branch {
        /// The child nodes in this branch.
        children: Box<[Arc<Node<T>>]>,
    },
}

impl<T> Node<T> {
    /// Return the leaf payload when this node is one leaf.
    fn leaf(&self) -> Option<&[T]> {
        // leaf nodes expose values directly
        match self {
            Self::Leaf { values, .. } => Some(values),
            Self::Branch { .. } => None,
        }
    }

    /// Return the branch payload when this node is one branch.
    fn branch(&self) -> Option<&[Arc<Node<T>>]> {
        // branch nodes expose child links
        match self {
            Self::Leaf { .. } => None,
            Self::Branch { children, .. } => Some(children),
        }
    }
}

/// One indexed sequence iterator.
#[derive(Debug)]
pub(crate) struct VectorIter<'a, T> {
    /// The traversal stack.
    stack: Vec<VectorIterFrame<'a, T>>,
}

/// One traversal frame for one indexed sequence iterator.
#[derive(Debug)]
enum VectorIterFrame<'a, T> {
    /// One leaf iterator.
    Leaf(slice::Iter<'a, T>),
    /// One branch iterator.
    Branch(slice::Iter<'a, Arc<Node<T>>>),
}

impl<T> Default for Vector<T> {
    fn default() -> Self {
        Self {
            len: 0,
            depth: 0,
            root: None,
        }
    }
}

impl<T> Vector<T> {
    /// Return the number of indexed elements.
    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.len
    }

    /// Return one indexed element by position.
    pub(crate) fn get(&self, index: usize) -> Option<&T> {
        // reject out of range positions
        if index >= self.len {
            return None;
        }

        self.get_in_node(self.root.as_deref()?, index, self.depth)
    }

    /// Iterate all indexed elements in order.
    pub(crate) fn iter(&self) -> VectorIter<'_, T> {
        let mut iter = VectorIter { stack: Vec::new() };

        // seed traversal from the shared root
        if let Some(root) = self.root.as_deref() {
            iter.push_node(root);
        }

        iter
    }

    /// Return the number of values covered by one node at the given depth.
    fn node_capacity(depth: usize) -> usize {
        let mut capacity = VECTOR_WIDTH;

        // grow the covered range one branch level at a time
        for _ in 0..depth {
            capacity = capacity.saturating_mul(VECTOR_WIDTH);
        }

        capacity
    }

    /// Return the smallest root depth that covers the given value count.
    fn root_depth(len: usize) -> usize {
        let mut depth: usize = 0;
        let mut capacity = VECTOR_WIDTH;

        // widen the root until it covers the logical length
        while len > capacity {
            depth = depth.saturating_add(1);
            capacity = capacity.saturating_mul(VECTOR_WIDTH);
        }

        depth
    }

    /// Return one value from the indexed node tree.
    fn get_in_node<'a>(&'a self, node: &'a Node<T>, index: usize, depth: usize) -> Option<&'a T> {
        // leaf nodes index directly into one local chunk
        if depth == 0 {
            return node.leaf()?.get(index);
        }

        // branch nodes descend into one child range
        let child_capacity = Self::node_capacity(depth - 1);
        let child_index = index / child_capacity;
        let child_offset = index % child_capacity;
        let child = node.branch()?.get(child_index)?.as_ref();

        self.get_in_node(child, child_offset, depth - 1)
    }
}

impl<T: Clone> Vector<T> {
    /// Build one shared vector from values with one custom equality predicate.
    pub(crate) fn from_values_by<F>(values: &[T], base: Option<&Self>, is_equal: F) -> Self
    where
        F: Fn(&T, &T) -> bool,
    {
        // empty vectors keep no root
        if values.is_empty() {
            return Self::default();
        }

        // build one root that reuses unchanged base nodes when possible
        let depth = Self::root_depth(values.len());
        let root = Self::build_value_node(
            values,
            base.and_then(|base| base.root.as_ref()),
            depth,
            &is_equal,
        );

        Self {
            len: values.len(),
            depth,
            root: Some(root),
        }
    }

    /// Build one value node from one slice and one optional base node.
    fn build_value_node<F>(
        values: &[T],
        base: Option<&Arc<Node<T>>>,
        depth: usize,
        is_equal: &F,
    ) -> Arc<Node<T>>
    where
        F: Fn(&T, &T) -> bool,
    {
        // leaves either reuse one identical base leaf or own one new chunk
        if depth == 0 {
            if let Some(base) = base
                && let Node::Leaf {
                    values: base_values,
                } = base.as_ref()
                && base_values
                    .iter()
                    .zip(values.iter())
                    .all(|(left, right)| is_equal(left, right))
            {
                return base.clone();
            }

            return Arc::new(Node::Leaf {
                values: values.to_vec().into_boxed_slice(),
            });
        }

        // branch nodes rebuild only the changed child ranges
        let child_capacity = Self::node_capacity(depth - 1);
        let mut children = Vec::new();

        for (child_index, slice) in values.chunks(child_capacity).enumerate() {
            let base_child = base.and_then(|base| match base.as_ref() {
                Node::Branch { children, .. } => children.get(child_index),
                Node::Leaf { .. } => None,
            });
            let child = Self::build_value_node(slice, base_child, depth - 1, is_equal);
            children.push(child);
        }

        // reuse the base branch when every child pointer stayed identical
        if let Some(base) = base
            && let Node::Branch {
                children: base_children,
            } = base.as_ref()
            && base_children.len() == children.len()
            && base_children
                .iter()
                .zip(children.iter())
                .all(|(left, right)| Arc::ptr_eq(left, right))
        {
            return base.clone();
        }

        Arc::new(Node::Branch {
            children: children.into_boxed_slice(),
        })
    }
}

impl<T: ?Sized> Vector<Arc<T>> {
    /// Build one shared vector from shared leaves, reusing unchanged nodes from one base vector.
    pub(crate) fn from_shared(leaves: &[Arc<T>], base: Option<&Self>) -> Self {
        // empty vectors keep no root
        if leaves.is_empty() {
            return Self::default();
        }

        // build one root that reuses unchanged shared leaves when possible
        let depth = Self::root_depth(leaves.len());
        let root = Self::build_shared_node(leaves, base.and_then(|base| base.root.as_ref()), depth);

        Self {
            len: leaves.len(),
            depth,
            root: Some(root),
        }
    }

    /// Build one shared node from one slice and one optional base node.
    fn build_shared_node(
        leaves: &[Arc<T>],
        base: Option<&Arc<Node<Arc<T>>>>,
        depth: usize,
    ) -> Arc<Node<Arc<T>>> {
        // leaves either reuse one identical base leaf or own one new chunk
        if depth == 0 {
            if let Some(base) = base
                && let Node::Leaf { values } = base.as_ref()
                && values
                    .iter()
                    .zip(leaves.iter())
                    .all(|(left, right)| Arc::ptr_eq(left, right))
            {
                return base.clone();
            }

            return Arc::new(Node::Leaf {
                values: leaves.to_vec().into_boxed_slice(),
            });
        }

        // branch nodes rebuild only the changed child ranges
        let child_capacity = Self::node_capacity(depth - 1);
        let mut children = Vec::new();

        for (child_index, slice) in leaves.chunks(child_capacity).enumerate() {
            let base_child = base.and_then(|base| match base.as_ref() {
                Node::Branch { children, .. } => children.get(child_index),
                Node::Leaf { .. } => None,
            });
            let child = Self::build_shared_node(slice, base_child, depth - 1);
            children.push(child);
        }

        // reuse the base branch when every child pointer stayed identical
        if let Some(base) = base
            && let Node::Branch {
                children: base_children,
            } = base.as_ref()
            && base_children.len() == children.len()
            && base_children
                .iter()
                .zip(children.iter())
                .all(|(left, right)| Arc::ptr_eq(left, right))
        {
            return base.clone();
        }

        Arc::new(Node::Branch {
            children: children.into_boxed_slice(),
        })
    }
}

impl<'a, T> VectorIter<'a, T> {
    /// Push one traversal frame for the given node.
    fn push_node(&mut self, node: &'a Node<T>) {
        // leaf frames yield values, branch frames walk child nodes
        match node {
            Node::Leaf { values, .. } => self.stack.push(VectorIterFrame::Leaf(values.iter())),
            Node::Branch { children, .. } => {
                self.stack.push(VectorIterFrame::Branch(children.iter()))
            }
        }
    }
}

impl<'a, T> Iterator for VectorIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let frame = self.stack.last_mut()?;
            match frame {
                VectorIterFrame::Leaf(values) => {
                    if let Some(value) = values.next() {
                        return Some(value);
                    }

                    self.stack.pop();
                }
                VectorIterFrame::Branch(children) => {
                    if let Some(child) = children.next() {
                        self.push_node(child);
                        continue;
                    }

                    self.stack.pop();
                }
            }
        }
    }
}
