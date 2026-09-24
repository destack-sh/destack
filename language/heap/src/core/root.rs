use crate::{HeapReference, HeapResult};

/// One mutable heap root slot.
#[derive(Debug)]
pub enum RootSlot<'a> {
    /// One direct worker heap reference cell.
    HeapReference(&'a mut HeapReference),
    /// One encoded worker heap reference cell.
    HeapBytes(&'a mut [u8]),
    /// One encoded runtime heap reference cell.
    SharedHeapBytes(&'a mut [u8]),
    /// One encoded borrow cell whose target storage is classified by address.
    BorrowBytes(&'a mut [u8]),
}

/// Visit direct worker heap references as mutable root slots.
pub fn visit_heap_references(
    roots: &mut [HeapReference],
    visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
) -> HeapResult<()> {
    for reference in roots {
        visit(RootSlot::HeapReference(reference))?;
    }

    Ok(())
}
