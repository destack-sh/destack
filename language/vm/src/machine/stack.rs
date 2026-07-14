use std::sync::Arc;

use destack_memory::{MemoryError, MemoryMap, MemoryRange};

use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use destack_program::StackImage;
use destack_program::vm::Cell;

/// World-mapped byte stack for one machine.
#[derive(Debug)]
pub(crate) struct Stack {
    /// The world memory map.
    memory: Arc<MemoryMap>,
    /// The stack byte range inside world memory.
    range: MemoryRange,
    /// The live byte length.
    len: usize,
    /// The hard byte limit.
    limit_bytes: usize,
}

impl Stack {
    /// Create one empty stack with a hard byte limit.
    pub(crate) fn new(memory: Arc<MemoryMap>, limit_bytes: usize) -> RuntimeResult<Self> {
        // allocate one stable world range for this stack
        let range = memory
            .allocate(limit_bytes, Cell::BYTE_LEN)
            .map_err(Error::from)?;

        Ok(Self {
            memory,
            range,
            len: 0,
            limit_bytes,
        })
    }

    /// Reset this stack.
    pub(crate) fn reset(&mut self) {
        self.len = 0;
    }

    /// Fork this stack over one forked memory map.
    pub(crate) fn fork(&self, memory: Arc<MemoryMap>) -> Self {
        Self {
            memory,
            range: self.range,
            len: self.len,
            limit_bytes: self.limit_bytes,
        }
    }

    /// Capture the live stack bytes.
    pub(crate) fn image(&self) -> RuntimeResult<StackImage> {
        let bytes = self
            .memory
            .read_bytes(self.range.offset, self.len)
            .map_err(Error::from)
            .map_err(RuntimeError::new)?;

        Ok(StackImage {
            memory_offset: self.range.offset as u64,
            bytes,
        })
    }

    /// Restore one stack from an immutable image.
    pub(crate) fn from_image(
        memory: Arc<MemoryMap>,
        image: &StackImage,
        limit_bytes: usize,
    ) -> RuntimeResult<Self> {
        if image.len() > limit_bytes {
            return Err(RuntimeError::new(Error::stack_overflow()));
        }

        let offset = usize::try_from(image.memory_offset).map_err(|_| {
            Error::from(MemoryError::OffsetOverflow {
                offset: image.memory_offset,
            })
        })?;
        let range = MemoryRange {
            offset,
            byte_len: limit_bytes,
        };
        memory.claim(range).map_err(Error::from)?;

        let mut stack = Self {
            memory,
            range,
            len: 0,
            limit_bytes,
        };
        if !image.is_empty() {
            let base = stack.allocate_uninit(image.len(), Cell::BYTE_LEN)?;
            stack.copy_bytes(base, &image.bytes)?;
        }

        Ok(stack)
    }

    /// Restore captured bytes into this stack range.
    pub(crate) fn restore(&mut self, image: &StackImage, limit_bytes: usize) -> RuntimeResult<()> {
        let memory_offset = usize::try_from(image.memory_offset).map_err(|_| {
            Error::from(MemoryError::OffsetOverflow {
                offset: image.memory_offset,
            })
        })?;
        if memory_offset != self.range.offset
            || limit_bytes != self.limit_bytes
            || image.len() > limit_bytes
        {
            return Err(RuntimeError::new(Error::invalid_continuation()));
        }

        // replace only the captured live prefix
        self.copy_bytes(0, &image.bytes)?;
        self.len = image.len();

        Ok(())
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

        self.memory
            .zero(self.range.offset + old_len, self.len - old_len)
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

    /// Restore bytes at one exact stack offset.
    pub(crate) fn restore_bytes(&mut self, offset: usize, bytes: &[u8]) -> RuntimeResult<()> {
        let end = offset + bytes.len();
        if offset < self.len || end > self.limit_bytes {
            return Err(RuntimeError::new(Error::invalid_continuation()));
        }

        self.memory
            .zero(self.range.offset + self.len, offset - self.len)
            .map_err(Error::from)?;
        self.len = end;
        self.copy_bytes(offset, bytes)
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
            .memory
            .address(self.range.offset + offset, byte_len)
            .map_err(Error::from)
            .map_err(RuntimeError::new)?;

        Ok(address as usize)
    }

    /// Return one stack byte offset inside world memory.
    #[inline]
    pub(crate) const fn memory_offset(&self, stack_offset: usize) -> usize {
        self.range.offset + stack_offset
    }

    /// Return the native base address of world memory.
    #[inline]
    pub(crate) fn memory_base_address(&self) -> usize {
        self.memory.base_address()
    }

    /// Copy bytes into one live byte range.
    #[inline]
    pub(crate) fn copy_bytes(&self, offset: usize, bytes: &[u8]) -> RuntimeResult<()> {
        self.memory
            .write_bytes(self.range.offset + offset, bytes)
            .map_err(Error::from)
            .map_err(RuntimeError::new)
    }
}

impl Drop for Stack {
    fn drop(&mut self) {
        // abort because an owned range becoming invalid means memory state is corrupt
        if self.memory.release(self.range).is_err() {
            std::process::abort();
        }
    }
}
