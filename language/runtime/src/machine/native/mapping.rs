use std::fmt;
use std::ops::Range;
use std::sync::Arc;

/// Process-local memory mapping.
#[derive(Debug, Clone)]
pub struct Mapping {
    /// The base address.
    base: usize,
    /// The mapped byte size.
    byte_size: usize,
    /// Owner that unmaps memory on drop.
    owner: Arc<dyn fmt::Debug + Send + Sync>,
}

impl Mapping {
    /// Create one process-local memory mapping.
    pub fn new(
        base: usize,
        byte_size: usize,
        owner: impl fmt::Debug + Send + Sync + 'static,
    ) -> Self {
        Self {
            base,
            byte_size,
            owner: Arc::new(owner),
        }
    }

    /// Return the base address.
    pub const fn base(&self) -> usize {
        self.base
    }

    /// Return the mapped byte size.
    pub const fn byte_size(&self) -> usize {
        self.byte_size
    }

    /// Return the mapped addresses.
    pub const fn range(&self) -> Range<usize> {
        self.base..self.base + self.byte_size
    }

    /// Return one address inside this mapped region.
    pub fn address_at(&self, offset: u32) -> Option<usize> {
        let offset = offset as usize;
        if offset >= self.byte_size {
            return None;
        }

        self.base.checked_add(offset)
    }

    /// Borrow the owner that keeps this mapping alive.
    pub fn owner(&self) -> &(dyn fmt::Debug + Send + Sync + '_) {
        self.owner.as_ref()
    }
}
