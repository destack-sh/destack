use serde::{Deserialize, Serialize};
use tspp_heap::HeapReference;
use tspp_serde::Reflect;

/// One immutable dynamically scoped execution context.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Context(HeapReference);

/// Fixed header shared by every execution context node.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextNode {
    /// Previous execution context.
    pub parent: Context,
    /// Context variable overridden by this node.
    pub variable: HeapReference,
}

impl Default for Context {
    /// Create the empty execution context.
    fn default() -> Self {
        Self::empty()
    }
}

impl Context {
    /// Return the empty execution context.
    pub const fn empty() -> Self {
        Self(HeapReference::NULL)
    }

    /// Create one context from its local heap reference.
    pub const fn new(reference: HeapReference) -> Self {
        Self(reference)
    }

    /// Return this context's local heap reference.
    pub const fn reference(self) -> HeapReference {
        self.0
    }

    /// Borrow this context's mutable local heap reference slot.
    pub fn reference_mut(&mut self) -> &mut HeapReference {
        &mut self.0
    }
}

impl ContextNode {
    /// The byte offset of the parent context.
    pub const PARENT_OFFSET: usize = std::mem::offset_of!(Self, parent);
    /// The byte offset of the context variable reference.
    pub const VARIABLE_OFFSET: usize = std::mem::offset_of!(Self, variable);
    /// The fixed node header byte length.
    pub const BYTE_LEN: usize = size_of::<Self>();
}
