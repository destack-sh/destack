use crate::HeapReference;
use crate::local::space::HeapPlace;

/// One planned relocation for a heap reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Promotion {
    /// The reference whose physical place moves.
    pub(crate) reference: HeapReference,
    /// The source physical location.
    pub(crate) source: HeapPlace,
    /// The target physical location.
    pub(crate) target: HeapPlace,
}
