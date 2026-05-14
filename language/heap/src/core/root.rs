use crate::{HeapError, HeapReference, HeapResult, SharedHeapReference};

/// One copied heap root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Root {
    /// One worker heap reference root.
    HeapReference(HeapReference),
    /// One runtime heap reference root.
    SharedHeapReference(SharedHeapReference),
}

impl Root {
    /// Return whether this root is null.
    pub fn is_null(self) -> bool {
        match self {
            Self::HeapReference(reference) => reference.is_null(),
            Self::SharedHeapReference(reference) => reference.is_null(),
        }
    }
}

/// Sink for copied roots discovered during one scan.
pub trait RootSink {
    /// Record one heap root.
    fn push(&mut self, root: Root);

    /// Record one worker heap root.
    fn push_heap(&mut self, reference: HeapReference) {
        self.push(Root::HeapReference(reference));
    }

    /// Record one runtime heap root.
    fn push_shared_heap(&mut self, reference: SharedHeapReference) {
        self.push(Root::SharedHeapReference(reference));
    }
}

impl RootSink for Vec<HeapReference> {
    fn push(&mut self, root: Root) {
        if root.is_null() {
            return;
        }

        if let Root::HeapReference(reference) = root {
            Vec::push(self, reference);
        }
    }
}

impl RootSink for Vec<SharedHeapReference> {
    fn push(&mut self, root: Root) {
        if root.is_null() {
            return;
        }

        if let Root::SharedHeapReference(reference) = root {
            Vec::push(self, reference);
        }
    }
}

impl RootSink for () {
    fn push(&mut self, _root: Root) {}
}

/// One mutable heap root slot.
#[derive(Debug)]
pub enum RootSlot<'a> {
    /// One direct worker heap reference cell.
    HeapReference(&'a mut HeapReference),
    /// One direct runtime heap reference cell.
    SharedHeapReference(&'a mut SharedHeapReference),
    /// One encoded worker heap reference cell.
    HeapBytes(&'a mut [u8]),
    /// One encoded runtime heap reference cell.
    SharedHeapBytes(&'a mut [u8]),
}

impl RootSlot<'_> {
    /// Read the current heap root from this root slot.
    pub fn load(&self) -> HeapResult<Root> {
        match self {
            Self::HeapReference(reference) => Ok(Root::HeapReference(**reference)),
            Self::SharedHeapReference(reference) => Ok(Root::SharedHeapReference(**reference)),
            Self::HeapBytes(bytes) => {
                HeapReference::read_from_bytes(bytes).map(Root::HeapReference)
            }
            Self::SharedHeapBytes(bytes) => {
                SharedHeapReference::read_from_bytes(bytes).map(Root::SharedHeapReference)
            }
        }
    }

    /// Read this root slot as a worker heap reference.
    pub fn load_heap_reference(&self) -> HeapResult<Option<HeapReference>> {
        match self {
            Self::HeapReference(reference) => Ok(Some(**reference)),
            Self::HeapBytes(bytes) => HeapReference::read_from_bytes(bytes).map(Some),
            Self::SharedHeapReference(_) | Self::SharedHeapBytes(_) => Ok(None),
        }
    }

    /// Write one worker heap reference into this root slot.
    pub fn store_heap_reference(&mut self, reference: HeapReference) -> HeapResult<()> {
        match self {
            Self::HeapReference(slot) => {
                **slot = reference;

                Ok(())
            }
            Self::HeapBytes(bytes) => reference.write_to_bytes(bytes),
            Self::SharedHeapReference(_) | Self::SharedHeapBytes(_) => {
                Err(HeapError::InvariantViolation {
                    context: "shared root slot cannot store worker heap reference",
                })
            }
        }
    }
}

/// Mutable heap roots for one collection safepoint.
pub trait RootSet {
    /// The root visitor error type.
    type Error: From<HeapError>;

    /// Visit every mutable heap root slot.
    fn visit_root_slots(
        &mut self,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Self::Error>;
}

impl<F, E> RootSet for F
where
    F: FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
    E: From<HeapError>,
{
    type Error = E;

    fn visit_root_slots(
        &mut self,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Self::Error> {
        self(visit)
    }
}

impl RootSet for [HeapReference] {
    type Error = HeapError;

    fn visit_root_slots(
        &mut self,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Self::Error> {
        for reference in self {
            visit(RootSlot::HeapReference(reference))?;
        }

        Ok(())
    }
}

impl RootSet for Vec<HeapReference> {
    type Error = HeapError;

    fn visit_root_slots(
        &mut self,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Self::Error> {
        self.as_mut_slice().visit_root_slots(visit)
    }
}

impl<const N: usize> RootSet for [HeapReference; N] {
    type Error = HeapError;

    fn visit_root_slots(
        &mut self,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Self::Error> {
        self.as_mut_slice().visit_root_slots(visit)
    }
}

impl RootSet for () {
    type Error = HeapError;

    fn visit_root_slots(
        &mut self,
        _visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}
