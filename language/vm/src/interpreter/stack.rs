use destack_heap::DEFAULT_PAGE_BYTES;
use destack_memory::AddressSpace;

use crate::diagnostic::{Error, RuntimeError, RuntimeResult};

/// Page-backed byte stack for one interpreter.
#[derive(Debug)]
pub(crate) struct Stack {
    /// The stack address space.
    space: AddressSpace,
    /// The live byte length.
    len: usize,
    /// The hard byte limit.
    limit_bytes: usize,
}

impl Stack {
    /// Create one empty stack with a hard byte limit.
    pub(crate) fn new(limit_bytes: usize) -> RuntimeResult<Self> {
        // create the backing address space once
        let space = AddressSpace::reserve(limit_bytes, DEFAULT_PAGE_BYTES).map_err(Error::from)?;

        Ok(Self {
            space,
            len: 0,
            limit_bytes,
        })
    }

    /// Reset this stack for one byte limit.
    pub(crate) fn reset(&mut self, limit_bytes: usize) -> RuntimeResult<()> {
        // replace the mapping when the limit changed
        if self.limit_bytes != limit_bytes {
            *self = Self::new(limit_bytes)?;

            Ok(())
        }
        // keep the reservation and forget live bytes
        else {
            self.len = 0;

            Ok(())
        }
    }

    /// Fork this stack with page-granular isolation.
    pub(crate) fn fork(&self) -> RuntimeResult<Self> {
        // fork the page map, not the bytes
        let stack = Self {
            space: self.space.fork_lazy().map_err(Error::from)?,
            len: self.len,
            limit_bytes: self.limit_bytes,
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
        // grow to the next aligned byte range
        let old_len = self.len;
        let base = Self::align_len(self.len, alignment);
        let end = base + byte_len;
        if end > self.limit_bytes {
            return Err(RuntimeError::new(Error::StackOverflow));
        }

        // zero newly exposed stack bytes
        self.space
            .zero(old_len, end - old_len)
            .map_err(Error::from)?;
        self.len = end;

        Ok(base)
    }

    /// Align one stack byte count.
    fn align_len(offset: usize, alignment: usize) -> usize {
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

    /// Return the native address for one live byte range.
    #[inline]
    pub(crate) fn address(&self, offset: usize, byte_len: usize) -> RuntimeResult<*mut u8> {
        self.space
            .address(offset, byte_len)
            .map_err(Error::from)
            .map_err(RuntimeError::new)
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

    /// Return the stack offset for one live native address range.
    #[inline]
    pub(crate) fn offset_for_address(&self, address: usize, byte_len: usize) -> Option<usize> {
        if !self.contains_address(address, byte_len) {
            return None;
        }

        Some(address - self.space.base_address())
    }

    /// Copy bytes into one live byte range.
    #[inline]
    pub(crate) fn copy_bytes(&self, offset: usize, bytes: &[u8]) -> RuntimeResult<()> {
        self.space
            .write_bytes(offset, bytes)
            .map_err(Error::from)
            .map_err(RuntimeError::new)
    }
}
