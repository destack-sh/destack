use crate::HeapReference;
use crate::local::space::HeapStorage;

/// One planned relocation for a heap reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Promotion {
    /// The reference whose physical storage moves.
    pub(crate) reference: HeapReference,
    /// The source physical location.
    pub(crate) source: HeapStorage,
    /// The target physical location.
    pub(crate) target: HeapStorage,
}
