use tspp_dir as dir;

use crate::sema::{CauseId, Origin, Relation};

/// One empty intrusive list link.
pub(in crate::sema) const EMPTY: u32 = u32::MAX;

/// One bound collected for an inference variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) struct Bound {
    /// The type evaluation site.
    pub(in crate::sema) origin: Origin,
    /// The bound type.
    pub(in crate::sema) ty: dir::GlobalTypeId,
    /// The relation between the variable and this bound.
    pub(in crate::sema) relation: Relation,
    /// Why this bound exists.
    pub(in crate::sema) cause: CauseId,
}

impl Bound {
    /// Return one type bound from its cause.
    pub(in crate::sema) fn new(
        origin: Origin,
        ty: dir::GlobalTypeId,
        relation: Relation,
        cause: CauseId,
    ) -> Self {
        Self {
            origin,
            ty,
            relation,
            cause,
        }
    }
}

/// One linked bound in the shared bounds column.
#[derive(Debug, Clone, Copy)]
pub(in crate::sema) struct BoundEntry {
    /// The collected bound.
    pub(in crate::sema) bound: Bound,
    /// The next bound of the same variable and side.
    pub(in crate::sema) next: u32,
}

/// One intrusive bound list of one variable side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) struct BoundList {
    /// The first bound of this side.
    pub(in crate::sema) head: u32,
    /// The last bound of this side, for insertion-order append.
    pub(in crate::sema) tail: u32,
    /// The number of bounds on this side.
    pub(in crate::sema) count: u32,
}

impl BoundList {
    /// Create an empty bound list.
    pub(in crate::sema) const fn new() -> Self {
        Self {
            head: EMPTY,
            tail: EMPTY,
            count: 0,
        }
    }

    /// Return whether this side has no bounds.
    pub(in crate::sema) const fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// One variable bound side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum BoundSide {
    /// Bounds that must relate to the variable.
    Lower,
    /// Bounds the variable must relate to.
    Upper,
}

impl BoundSide {
    /// Return the side one bound takes seen from its other end.
    pub(in crate::sema) fn opposite(self) -> Self {
        match self {
            Self::Lower => Self::Upper,
            Self::Upper => Self::Lower,
        }
    }
}

/// Iterator over one variable side's bounds in insertion order.
pub(in crate::sema) struct BoundIter<'a> {
    /// The shared bounds.
    pub(in crate::sema) bounds: &'a [BoundEntry],
    /// The next bound to yield.
    pub(in crate::sema) current: u32,
}

impl Iterator for BoundIter<'_> {
    type Item = Bound;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == EMPTY {
            return None;
        }
        let entry = self.bounds[self.current as usize];
        self.current = entry.next;

        Some(entry.bound)
    }
}
