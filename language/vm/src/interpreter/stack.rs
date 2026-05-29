use destack_heap::DEFAULT_ALLOCATOR_PAGE_SIZE_BYTES;
use destack_memory::AddressSpace;
use serde::{Deserialize, Serialize};

use crate::Word;
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
        let space = AddressSpace::reserve(limit_bytes, DEFAULT_ALLOCATOR_PAGE_SIZE_BYTES)
            .map_err(Error::from)?;

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

    /// Capture the live stack bytes.
    pub(crate) fn image(&self) -> RuntimeResult<StackImage> {
        let bytes = self
            .space
            .read_bytes(0, self.len)
            .map_err(Error::from)
            .map_err(RuntimeError::new)?;

        Ok(StackImage { bytes })
    }

    /// Restore one stack from an immutable image.
    pub(crate) fn from_image(image: &StackImage, limit_bytes: usize) -> RuntimeResult<Self> {
        if image.len() > limit_bytes {
            return Err(RuntimeError::new(Error::stack_overflow()));
        }

        let mut stack = Self::new(limit_bytes)?;
        if !image.is_empty() {
            let base = stack.allocate_uninit(image.len(), Word::BYTE_LEN)?;
            stack.copy_bytes(base, &image.bytes)?;
        }

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

    /// Allocate one zeroed aligned byte range.
    pub(crate) fn allocate_zeroed(
        &mut self,
        byte_len: usize,
        alignment: usize,
    ) -> RuntimeResult<usize> {
        let old_len = self.len;
        let base = self.reserve(byte_len, alignment)?;

        self.space
            .zero(old_len, self.len - old_len)
            .map_err(Error::from)?;

        Ok(base)
    }

    /// Allocate one uninitialized aligned byte range.
    pub(crate) fn allocate_uninit(
        &mut self,
        byte_len: usize,
        alignment: usize,
    ) -> RuntimeResult<usize> {
        self.reserve(byte_len, alignment)
    }

    /// Reserve one aligned live byte range.
    fn reserve(&mut self, byte_len: usize, alignment: usize) -> RuntimeResult<usize> {
        // grow to the next aligned byte range
        let base = Self::align_len(self.len, alignment);
        let end = base + byte_len;
        if end > self.limit_bytes {
            return Err(RuntimeError::new(Error::stack_overflow()));
        }

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
    pub(crate) fn address(&self, offset: usize, byte_len: usize) -> RuntimeResult<usize> {
        let address = self
            .space
            .address(offset, byte_len)
            .map_err(Error::from)
            .map_err(RuntimeError::new)?;

        Ok(address as usize)
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

/// Immutable stack byte image.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackImage {
    /// The captured live stack bytes.
    pub bytes: Vec<u8>,
}

impl StackImage {
    /// Return the live byte length.
    pub const fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Return whether this stack image has no live bytes.
    pub const fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Borrow one frame byte range.
    pub(crate) fn frame_bytes(&self, offset: usize, byte_len: usize) -> Option<&[u8]> {
        let end = offset.checked_add(byte_len)?;

        self.bytes.get(offset..end)
    }

    /// Borrow one frame byte range mutably.
    pub(crate) fn frame_bytes_mut(&mut self, offset: usize, byte_len: usize) -> Option<&mut [u8]> {
        let end = offset.checked_add(byte_len)?;

        self.bytes.get_mut(offset..end)
    }
}
