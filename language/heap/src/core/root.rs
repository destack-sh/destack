use crate::{HeapError, HeapReference, HeapResult};

/// One mutable local root slot.
#[derive(Debug)]
pub enum RootSlot<'a> {
    /// One direct heap reference cell.
    Reference(&'a mut HeapReference),
    /// One pointer-width byte cell.
    Bytes(&'a mut [u8]),
}

impl RootSlot<'_> {
    /// Read the current heap reference from this root slot.
    pub fn load(&self) -> HeapResult<HeapReference> {
        match self {
            Self::Reference(reference) => Ok(**reference),
            Self::Bytes(bytes) => HeapReference::read_from_bytes(bytes),
        }
    }

    /// Write one heap reference into this root slot.
    pub fn store(&mut self, reference: HeapReference) -> HeapResult<()> {
        match self {
            Self::Reference(slot) => {
                **slot = reference;

                Ok(())
            }
            Self::Bytes(bytes) => reference.write_to_bytes(bytes),
        }
    }
}

/// Source of local roots for one collection safepoint.
pub trait RootSource {
    /// The root-source error type.
    type Error: From<HeapError>;

    /// Visit every mutable local root slot.
    fn visit_roots(
        &mut self,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Self::Error>;
}

impl<F, E> RootSource for F
where
    F: FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
    E: From<HeapError>,
{
    type Error = E;

    fn visit_roots(
        &mut self,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Self::Error> {
        self(visit)
    }
}

impl RootSource for [HeapReference] {
    type Error = HeapError;

    fn visit_roots(
        &mut self,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Self::Error> {
        for reference in self {
            visit(RootSlot::Reference(reference))?;
        }

        Ok(())
    }
}

impl RootSource for Vec<HeapReference> {
    type Error = HeapError;

    fn visit_roots(
        &mut self,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Self::Error> {
        self.as_mut_slice().visit_roots(visit)
    }
}

impl<const N: usize> RootSource for [HeapReference; N] {
    type Error = HeapError;

    fn visit_roots(
        &mut self,
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Self::Error> {
        self.as_mut_slice().visit_roots(visit)
    }
}

impl RootSource for () {
    type Error = HeapError;

    fn visit_roots(
        &mut self,
        _visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}
