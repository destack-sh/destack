use destack_dir as dir;

use crate::check::Relation;

/// One empty intrusive list link.
pub(in crate::check) const EMPTY: u32 = u32::MAX;

/// When one bound may choose an inference variable's solution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum BoundMode {
    /// Bound may solve the variable during regular solver work.
    Strong,
    /// Bound may solve the variable only after regular work drains.
    Weak,
}

impl BoundMode {
    /// Return whether this solve pass may use weak bounds.
    pub(in crate::check) fn allows_weak(self) -> bool {
        matches!(self, Self::Weak)
    }
}

/// One bound collected for an inference variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct TypeBound {
    /// The bound type.
    pub(in crate::check) ty: dir::GlobalTypeId,
    /// The relation between the variable and this bound.
    pub(in crate::check) relation: Relation,
    /// The source occurrence that produced the bound.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// When this bound may choose the variable's solution.
    pub(in crate::check) mode: BoundMode,
}

impl TypeBound {
    /// Return one type bound from a source occurrence.
    pub(in crate::check) fn new(
        ty: dir::GlobalTypeId,
        relation: Relation,
        source: dir::GlobalNodeIdAny,
        mode: BoundMode,
    ) -> Self {
        Self {
            ty,
            relation,
            source,
            mode,
        }
    }
}

/// One linked bound in the shared bounds column.
#[derive(Debug, Clone, Copy)]
pub(in crate::check) struct BoundEntry {
    /// The collected bound.
    pub(in crate::check) bound: TypeBound,
    /// The next bound of the same variable and side.
    pub(in crate::check) next: u32,
}

/// One intrusive bound list of one variable side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct BoundList {
    /// The first bound of this side.
    pub(in crate::check) head: u32,
    /// The last bound of this side, for insertion-order append.
    pub(in crate::check) tail: u32,
    /// The number of bounds on this side.
    pub(in crate::check) count: u32,
}

impl BoundList {
    /// Create an empty bound list.
    pub(in crate::check) const fn new() -> Self {
        Self {
            head: EMPTY,
            tail: EMPTY,
            count: 0,
        }
    }

    /// Return whether this side carries no bounds.
    pub(in crate::check) fn is_empty(self) -> bool {
        self.count == 0
    }
}

/// One variable bound side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum BoundSide {
    /// Bounds that must relate to the variable.
    Lower,
    /// Bounds the variable must relate to.
    Upper,
}

/// Iterator over one variable side's bounds in insertion order.
pub(in crate::check) struct BoundIter<'a> {
    /// The shared bounds.
    pub(in crate::check) bounds: &'a [BoundEntry],
    /// The next bound to yield.
    pub(in crate::check) current: u32,
}

impl Iterator for BoundIter<'_> {
    type Item = TypeBound;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == EMPTY {
            return None;
        }
        let entry = self.bounds[self.current as usize];
        self.current = entry.next;

        Some(entry.bound)
    }
}
