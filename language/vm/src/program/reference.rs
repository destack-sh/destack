use destack_heap::{
    HeapEdge, HeapReference, HeapResult, RootSlot, SharedHeapReference, visit_heap_edges_in_bytes,
    visit_heap_root_slots_in_bytes,
};
use destack_mir as mir;

use super::{Program, repr_type};
use crate::{Error, RootSink, Word};

impl Program {
    /// Visit one heap root stored in one word.
    pub(crate) fn visit_value_root(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        value: Word,
        roots: &mut impl RootSink,
    ) -> Result<(), Error> {
        let Some(edge) = self.scalar_heap_edge(ty, value)? else {
            return Ok(());
        };

        push_heap_edge(edge, roots);

        Ok(())
    }

    /// Return whether one scalar type carries a worker-local heap root.
    pub(crate) fn is_local_root_type(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Result<bool, Error> {
        Ok(matches!(
            self.scalar_heap_edge(ty, Word::VOID)?,
            Some(HeapEdge::Local(_))
        ))
    }

    /// Visit heap roots from one byte range.
    pub(crate) fn visit_byte_roots(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        bytes: &[u8],
        roots: &mut impl RootSink,
    ) -> Result<(), Error> {
        let layout = self.layout(ty).ok_or_else(|| Error::InvariantViolation {
            context: format!("missing layout for byte root scan: type={ty:?}"),
        })?;

        if bytes.len() != layout.byte_len {
            return Err(Error::InvariantViolation {
                context: format!(
                    "byte root length mismatch: type={ty:?}, bytes={}, layout_bytes={}",
                    bytes.len(),
                    layout.byte_len,
                ),
            });
        }

        visit_heap_edges_in_bytes(&layout.reference_map, bytes, &mut |edge| {
            push_heap_edge(edge, roots);

            Ok(())
        })
        .map_err(Error::from)?;

        Ok(())
    }

    /// Visit mutable local root slots from one byte range.
    pub(crate) fn visit_byte_root_slots(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        bytes: &mut [u8],
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Error> {
        let layout = self.layout(ty).ok_or_else(|| Error::InvariantViolation {
            context: format!("missing layout for byte heap roots: type={ty:?}"),
        })?;

        if bytes.len() != layout.byte_len {
            return Err(Error::InvariantViolation {
                context: format!(
                    "byte heap root length mismatch: type={ty:?}, bytes={}, layout_bytes={}",
                    bytes.len(),
                    layout.byte_len,
                ),
            });
        }

        visit_heap_root_slots_in_bytes(&layout.reference_map, bytes, visit).map_err(Error::from)
    }

    /// Return the heap edge carried by one scalar value.
    fn scalar_heap_edge(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        value: Word,
    ) -> Result<Option<HeapEdge>, Error> {
        let layout = self.layout(ty).ok_or_else(|| Error::InvariantViolation {
            context: format!("missing scalar layout for root scan: type={ty:?}"),
        })?;

        if !layout.is_word() {
            return Ok(None);
        }

        let repr_ty = repr_type(&self.tree, ty);
        let bits = value.bits() as usize;

        match self.tree.get(repr_ty) {
            mir::Type::Reference {
                kind: mir::ReferenceKind::Managed | mir::ReferenceKind::Owned,
                address_space: mir::AddressSpace::Local,
                ..
            }
            | mir::Type::TensorView {
                kind: mir::ReferenceKind::Managed | mir::ReferenceKind::Owned,
                address_space: mir::AddressSpace::Local,
                ..
            }
            | mir::Type::Callable { .. } => {
                Ok(Some(HeapEdge::Local(HeapReference::from_bits(bits))))
            }
            mir::Type::Reference {
                kind: mir::ReferenceKind::Managed | mir::ReferenceKind::Owned,
                address_space: mir::AddressSpace::Shared,
                ..
            }
            | mir::Type::TensorView {
                kind: mir::ReferenceKind::Managed | mir::ReferenceKind::Owned,
                address_space: mir::AddressSpace::Shared,
                ..
            } => Ok(Some(HeapEdge::Shared(SharedHeapReference::from_bits(bits)))),
            _ => Ok(None),
        }
    }
}

/// Push one non-null heap edge into the given root sink.
fn push_heap_edge(edge: HeapEdge, roots: &mut impl RootSink) {
    match edge {
        HeapEdge::Local(reference) => {
            if !reference.is_null() {
                roots.push_heap(reference);
            }
        }
        HeapEdge::Shared(reference) => {
            if !reference.is_null() {
                roots.push_shared_heap(reference);
            }
        }
    }
}
