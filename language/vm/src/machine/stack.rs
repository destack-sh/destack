use std::ops::Range;
use std::sync::Arc;
use std::{process, ptr};

use tspp_memory::{MemoryMap, MemoryRange};
use tspp_program as program;
use tspp_program::Word;

use crate::diagnostic::{Error, Result};

/// One contiguous VM stack inside world memory.
#[derive(Debug)]
pub(crate) struct Stack {
    /// The world memory map.
    memory: Arc<MemoryMap>,
    /// The reserved stack range.
    range: MemoryRange,
    /// The native address of the first stack byte.
    base: usize,
    /// The stack prefix backed by mapped memory frames.
    materialized_byte_len: usize,
    /// The live stack byte length.
    byte_len: usize,
}

impl Stack {
    /// Reserve one empty VM stack.
    pub(crate) fn new(memory: Arc<MemoryMap>, byte_len: usize) -> Result<Self> {
        let range = memory
            .allocate(byte_len, align_of::<Word>())
            .map_err(|_| Error::memory_exhausted())?;
        let base = memory.base_address() + range.offset;

        Ok(Self {
            memory,
            range,
            base,
            materialized_byte_len: 0,
            byte_len: 0,
        })
    }

    /// Rebuild one stack over a range retained in restored world memory.
    pub(crate) fn from_range(
        memory: Arc<MemoryMap>,
        range: MemoryRange,
        byte_len: usize,
    ) -> Result<Self> {
        let base = memory.base_address() + range.offset;
        let mut stack = Self {
            memory,
            range,
            base,
            materialized_byte_len: 0,
            byte_len: 0,
        };
        stack.grow(byte_len)?;

        Ok(stack)
    }

    /// Return the reserved stack range in world memory.
    pub(crate) const fn range(&self) -> MemoryRange {
        self.range
    }

    /// Fork this stack over the corresponding range in one forked memory map.
    pub(crate) fn fork(&self, memory: Arc<MemoryMap>) -> Self {
        let base = memory.base_address() + self.range.offset;

        Self {
            memory,
            range: self.range,
            base,
            materialized_byte_len: self.materialized_byte_len,
            byte_len: self.byte_len,
        }
    }

    /// Return the world memory map that owns this stack.
    pub(crate) fn memory(&self) -> Arc<MemoryMap> {
        self.memory.clone()
    }

    /// Remove every live stack byte.
    pub(crate) fn clear(&mut self) {
        self.byte_len = 0;
    }

    /// Return the live stack byte length.
    pub(crate) const fn byte_len(&self) -> usize {
        self.byte_len
    }

    /// Allocate one aligned live byte range.
    pub(crate) fn push_bytes(&mut self, byte_len: usize, alignment: usize) -> Result<usize> {
        let start = self.byte_len.next_multiple_of(alignment);
        let end = start + byte_len;
        self.grow(end)?;

        Ok(start)
    }

    /// Grow the live stack to one byte length.
    pub(crate) fn grow(&mut self, byte_len: usize) -> Result<()> {
        if byte_len <= self.byte_len {
            return Ok(());
        }

        if byte_len > self.range.byte_len {
            return Err(Error::stack_overflow());
        }

        // materialize each mapping frame at most once for this stack
        if byte_len > self.materialized_byte_len {
            let frame_byte_len = self.memory.frame_size_bytes();
            let end = self.range.offset + byte_len;
            let end = end.next_multiple_of(frame_byte_len);
            let materialized_byte_len = (end - self.range.offset).min(self.range.byte_len);

            // map only the newly reached frame prefix
            let materialize_start = self.range.offset + self.materialized_byte_len;
            let materialize_byte_len = materialized_byte_len - self.materialized_byte_len;
            self.memory
                .materialize(materialize_start, materialize_byte_len)
                .map_err(|_| Error::memory_exhausted())?;
            self.materialized_byte_len = materialized_byte_len;
        }

        self.byte_len = byte_len;

        Ok(())
    }

    /// Zero one live stack byte range.
    pub(crate) fn zero(&mut self, byte_offset: usize, byte_len: usize) -> Result<()> {
        self.memory
            .zero(self.range.offset + byte_offset, byte_len)
            .map_err(|_| Error::memory_exhausted())
    }

    /// Allocate register words and return the first word index.
    pub(crate) fn push_words(&mut self, word_count: usize) -> Result<usize> {
        let byte_len = word_count * Word::BYTE_LEN;
        let start = self.push_bytes(byte_len, align_of::<Word>())?;

        Ok(start / Word::BYTE_LEN)
    }

    /// Truncate this stack to one byte offset.
    pub(crate) fn truncate(&mut self, byte_len: usize) {
        self.byte_len = byte_len;
    }

    /// Return the native address of one live stack byte.
    #[inline(always)]
    pub(crate) const fn address(&self, byte_offset: usize) -> usize {
        self.base + byte_offset
    }

    /// Return one stack byte as an offset inside world memory.
    #[inline(always)]
    pub(crate) const fn memory_offset(&self, byte_offset: usize) -> usize {
        self.range.offset + byte_offset
    }

    /// Return the stack-relative offset of one native address.
    pub(crate) fn byte_offset(&self, address: usize) -> Option<usize> {
        let offset = address.checked_sub(self.base)?;

        (offset < self.byte_len).then_some(offset)
    }

    /// Copy one live stack byte range into a destination slice.
    pub(crate) fn read_bytes(&self, byte_offset: usize, bytes: &mut [u8]) -> Result<()> {
        self.live_range(byte_offset, bytes.len())?;

        // SAFETY: frame ranges address initialized live stack bytes
        unsafe {
            ptr::copy_nonoverlapping(
                self.address(byte_offset) as *const u8,
                bytes.as_mut_ptr(),
                bytes.len(),
            );
        }

        Ok(())
    }

    /// Copy one byte slice into a live stack range.
    pub(crate) fn write_bytes(&mut self, byte_offset: usize, bytes: &[u8]) -> Result<()> {
        self.live_range(byte_offset, bytes.len())?;

        // SAFETY: frame ranges address initialized live stack bytes
        unsafe {
            ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                self.address(byte_offset) as *mut u8,
                bytes.len(),
            );
        }

        Ok(())
    }

    /// Borrow one live stack byte range mutably.
    pub(crate) fn bytes_mut(&mut self, byte_offset: usize, byte_len: usize) -> Result<&mut [u8]> {
        self.live_range(byte_offset, byte_len)?;

        // SAFETY: callers provide one live range and hold exclusive machine access
        let bytes = unsafe {
            std::slice::from_raw_parts_mut(self.address(byte_offset) as *mut u8, byte_len)
        };

        Ok(bytes)
    }

    /// Move one possibly overlapping live byte range.
    #[inline(always)]
    pub(crate) fn move_bytes(&mut self, source: usize, target: usize, byte_len: usize) {
        // SAFETY: linked value and frame ranges address live stack bytes
        unsafe {
            ptr::copy(
                self.address(source) as *const u8,
                self.address(target) as *mut u8,
                byte_len,
            );
        }
    }

    /// Read one live register word.
    #[inline(always)]
    pub(crate) fn read(&self, index: usize) -> Word {
        // SAFETY: linked register indices address initialized live stack words
        unsafe { ptr::read((self.base as *const Word).add(index)) }
    }

    /// Write one live register word.
    #[inline(always)]
    pub(crate) fn write(&mut self, index: usize, value: Word) {
        // SAFETY: linked register indices address initialized live stack words
        unsafe {
            ptr::write((self.base as *mut Word).add(index), value);
        }
    }

    /// Copy one non-overlapping register range.
    #[inline(always)]
    pub(crate) fn copy_words(&mut self, source: usize, target: usize, word_count: usize) {
        // SAFETY: linked call windows are live, contiguous, and non-overlapping
        unsafe {
            ptr::copy_nonoverlapping(
                (self.base as *const Word).add(source),
                (self.base as *mut Word).add(target),
                word_count,
            );
        }
    }

    /// Move one possibly overlapping register range.
    #[inline(always)]
    pub(crate) fn move_words(&mut self, source: usize, target: usize, word_count: usize) {
        // SAFETY: linked register ranges address live stack words
        unsafe {
            ptr::copy(
                (self.base as *const Word).add(source),
                (self.base as *mut Word).add(target),
                word_count,
            );
        }
    }

    /// Copy one live register range into owned result storage.
    pub(crate) fn words(&self, start: usize, word_count: usize) -> Vec<Word> {
        let mut words = Vec::with_capacity(word_count);

        // copy the final result once across the host boundary
        for index in start..start + word_count {
            words.push(self.read(index));
        }

        words
    }

    /// Require one byte range to lie inside the live stack prefix.
    fn live_range(&self, byte_offset: usize, byte_len: usize) -> Result<()> {
        let end = byte_offset
            .checked_add(byte_len)
            .ok_or_else(Error::invalid_image)?;
        if end > self.byte_len {
            return Err(Error::invalid_image());
        }

        Ok(())
    }
}

impl Drop for Stack {
    /// Release this stack's reserved world-memory range.
    fn drop(&mut self) {
        if self.memory.release(self.range).is_err() {
            process::abort();
        }
    }
}

/// One fiber's native machine stack in world memory, above a frozen guard frame.
#[derive(Debug)]
pub struct NativeStack {
    /// The world memory map.
    memory: Arc<MemoryMap>,
    /// The reserved range, the guard frame lowest.
    range: MemoryRange,
}

impl NativeStack {
    /// Reserve one native stack of at least one usable byte length.
    pub(crate) fn new(memory: Arc<MemoryMap>, byte_len: usize) -> Result<Self> {
        let frame_byte_len = memory.frame_size_bytes();
        let byte_len = byte_len.next_multiple_of(frame_byte_len);
        let range = memory
            .allocate(frame_byte_len + byte_len, frame_byte_len)
            .map_err(|_| Error::memory_exhausted())?;
        let stack = Self { memory, range };

        // freeze the lowest frame so an overflowing write faults, then map the frames above it
        let guard = MemoryRange {
            offset: range.offset,
            byte_len: frame_byte_len,
        };
        stack.memory.freeze(guard).map_err(program::Error::from)?;
        stack
            .memory
            .materialize(range.offset + frame_byte_len, byte_len)
            .map_err(program::Error::from)?;

        Ok(stack)
    }

    /// Rebuild one native stack over a range retained in restored world memory.
    pub(crate) fn from_range(memory: Arc<MemoryMap>, range: MemoryRange) -> Self {
        Self { memory, range }
    }

    /// Return the reserved range in world memory, the guard frame included.
    pub(crate) const fn range(&self) -> MemoryRange {
        self.range
    }

    /// Fork this native stack over the corresponding range in one forked memory map.
    pub(crate) fn fork(&self, memory: Arc<MemoryMap>) -> Self {
        Self {
            memory,
            range: self.range,
        }
    }

    /// Return the native address of the lowest usable byte, just above the guard frame.
    pub fn bottom(&self) -> *mut u8 {
        let offset = self.range.offset + self.memory.frame_size_bytes();

        (self.memory.base_address() + offset) as *mut u8
    }

    /// Return the world offsets of the usable stack above the guard frame.
    pub fn world_range(&self) -> Range<usize> {
        let start = self.range.offset + self.memory.frame_size_bytes();

        start..start + self.byte_len()
    }

    /// Return the usable byte length above the guard frame.
    pub fn byte_len(&self) -> usize {
        self.range.byte_len - self.memory.frame_size_bytes()
    }
}

impl Drop for NativeStack {
    /// Release this native stack's reserved world-memory range.
    fn drop(&mut self) {
        if self.memory.release(self.range).is_err() {
            process::abort();
        }
    }
}
