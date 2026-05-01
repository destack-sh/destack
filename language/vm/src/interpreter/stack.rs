use destack_heap::AddressSpace;

use crate::diagnostic::{Error, RuntimeError, RuntimeResult};

/// Align one stack byte count.
fn align_stack_bytes(offset: usize, alignment: usize) -> usize {
    // already aligned
    if alignment <= 1 {
        return offset;
    }

    // round up to the next aligned address
    let remainder = offset % alignment;
    if remainder == 0 {
        offset
    } else {
        offset + alignment - remainder
    }
}

/// Page-backed byte stack for one interpreter.
#[derive(Debug)]
pub(crate) struct Stack {
    /// The reserved stack address space.
    space: AddressSpace,
    /// The live byte length.
    len: usize,
    /// The reserved byte length.
    byte_len: usize,
}

impl Stack {
    /// Reserve one empty stack.
    pub(crate) fn reserve(byte_len: usize) -> RuntimeResult<Self> {
        // reserve page-backed virtual memory
        let page_bytes = destack_heap::DEFAULT_PAGE_BYTES;
        let space = AddressSpace::reserve(byte_len, page_bytes).map_err(Error::from)?;

        Ok(Self {
            space,
            len: 0,
            byte_len,
        })
    }

    /// Reserve a replacement stack when the requested capacity changed.
    pub(crate) fn reset(&mut self, byte_len: usize) -> RuntimeResult<()> {
        // replace the mapping when the limit changed
        if self.byte_len != byte_len {
            *self = Self::reserve(byte_len)?;

            return Ok(());
        }

        // keep the reservation and forget live bytes
        self.len = 0;

        Ok(())
    }

    /// Fork this stack with page-granular isolation.
    pub(crate) fn fork(&self) -> RuntimeResult<Self> {
        // fork the page map, not the bytes
        let stack = Self {
            space: self.space.fork().map_err(Error::from)?,
            len: self.len,
            byte_len: self.byte_len,
        };

        Ok(stack)
    }

    /// Return the live byte length.
    #[inline]
    pub(crate) const fn len(&self) -> usize {
        self.len
    }

    /// Truncate this stack to one live byte length.
    #[inline]
    pub(crate) fn truncate(&mut self, len: usize) {
        debug_assert!(len <= self.len);
        self.len = len;
    }

    /// Allocate one aligned byte range.
    pub(crate) fn allocate(&mut self, byte_len: usize, alignment: usize) -> RuntimeResult<usize> {
        // reserve the next aligned byte range
        let base = align_stack_bytes(self.len, alignment);
        let end = base + byte_len;
        if end > self.byte_len {
            return Err(RuntimeError::new(Error::StackOverflow));
        }

        // zero newly exposed stack bytes
        self.space.zero(base, end - self.len).map_err(Error::from)?;
        self.len = end;

        Ok(base)
    }

    /// Return the native address for one live byte range.
    #[inline]
    pub(crate) fn address(&self, offset: usize, byte_len: usize) -> RuntimeResult<*mut u8> {
        Ok(self.space.address(offset, byte_len).map_err(Error::from)?)
    }

    /// Return whether one native address range belongs to this stack.
    #[inline]
    pub(crate) fn contains_address(&self, address: usize, byte_len: usize) -> bool {
        // compare against the live stack range
        let start = self.space.base_address();
        let end = address + byte_len;
        let stack_end = start + self.len;

        start <= address && end <= stack_end
    }

    /// Write bytes into one live byte range.
    #[inline]
    pub(crate) fn write(&self, offset: usize, bytes: &[u8]) -> RuntimeResult<()> {
        Ok(self.space.write(offset, bytes).map_err(Error::from)?)
    }
}
