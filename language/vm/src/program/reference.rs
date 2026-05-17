use destack_heap::{
    HeapEdge, HeapReference, HeapResult, RootSlot, SharedHeapReference,
    visit_heap_root_slots_in_bytes,
};
use destack_mir as mir;

use super::{Program, repr_type};
use crate::{Error, Word};

impl Program {
    /// Return whether one scalar type carries a worker heap root.
    pub(crate) fn is_local_root_type(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Result<bool, Error> {
        Ok(matches!(
            self.scalar_heap_edge(ty, Word::VOID)?,
            Some(HeapEdge::Local(_))
        ))
    }

    /// Return whether one scalar type carries a shared heap root.
    pub(crate) fn is_shared_root_type(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Result<bool, Error> {
        Ok(matches!(
            self.scalar_heap_edge(ty, Word::VOID)?,
            Some(HeapEdge::Shared(_))
        ))
    }

    /// Visit mutable heap root slots from one byte range.
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
                kind:
                    mir::ReferenceKind::Managed
                    | mir::ReferenceKind::Unique
                    | mir::ReferenceKind::Borrowed,
                space: mir::Space::Local,
                ..
            }
            | mir::Type::Callable { .. } => {
                Ok(Some(HeapEdge::Local(HeapReference::from_bits(bits))))
            }
            mir::Type::Reference {
                kind:
                    mir::ReferenceKind::Managed
                    | mir::ReferenceKind::Unique
                    | mir::ReferenceKind::Borrowed,
                space: mir::Space::Shared,
                ..
            } => Ok(Some(HeapEdge::Shared(SharedHeapReference::from_bits(bits)))),
            _ => Ok(None),
        }
    }
}
