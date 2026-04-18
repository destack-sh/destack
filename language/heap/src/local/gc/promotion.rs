use crate::local::managed::ManagedLocation;
use crate::value::ManagedReference;

/// One planned relocation for a managed reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Promotion {
    /// The reference whose physical storage moves.
    pub(crate) reference: ManagedReference,
    /// The source physical location.
    pub(crate) source: ManagedLocation,
    /// The target physical location.
    pub(crate) target: ManagedLocation,
}
