use std::ptr::{NonNull, null_mut};
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};

use super::{PageId, PageRun};
use crate::{HeapError, HeapResult};

/// One fixed arena in the allocator.
#[derive(Debug)]
pub(super) struct Arena {
    /// The logical arena index.
    index: usize,
    /// The mapped arena base.
    base: NonNull<u8>,
    /// The next never-allocated page inside this arena.
    pub(super) next_unused_page: AtomicU32,
    /// The run refcounts keyed by run-start page index inside this arena.
    run_refcounts: Box<[AtomicU32]>,
}

impl Arena {
    /// Create one arena metadata record.
    pub(super) fn new(index: usize, base: NonNull<u8>, pages_per_arena: usize) -> Self {
        Self {
            index,
            base,
            next_unused_page: AtomicU32::new(0),
            run_refcounts: std::iter::repeat_with(|| AtomicU32::new(0))
                .take(pages_per_arena)
                .collect(),
        }
    }

    /// Return the logical arena index.
    pub(super) const fn index(&self) -> usize {
        self.index
    }

    /// Return the mapped arena base.
    pub(super) const fn base(&self) -> NonNull<u8> {
        self.base
    }

    /// Return one page pointer inside this arena.
    pub(super) fn page_ptr(&self, arena_page_index: usize, page_bytes: usize) -> Option<*mut u8> {
        let data = self.base.as_ptr();
        let page_offset = arena_page_index.checked_mul(page_bytes)?;

        Some(unsafe { data.add(page_offset) })
    }

    /// Allocate one fresh run from this arena.
    pub(super) fn allocate_run(
        &self,
        arena_index: usize,
        page_count: usize,
        pages_per_arena: usize,
    ) -> HeapResult<Option<PageRun>> {
        let page_count = u32::try_from(page_count)
            .map_err(|_| HeapError::InvalidPageId { index: page_count })?;

        let start_page = loop {
            let start_page = self.next_unused_page.load(Ordering::Acquire);
            let Some(end_page) = start_page.checked_add(page_count) else {
                return Err(HeapError::InvalidPageId {
                    index: start_page as usize,
                });
            };

            // stop once this arena is exhausted
            if end_page as usize > pages_per_arena {
                return Ok(None);
            }

            if self
                .next_unused_page
                .compare_exchange(start_page, end_page, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                break start_page as usize;
            }
        };

        let first_page = arena_index
            .checked_mul(pages_per_arena)
            .and_then(|first_page| first_page.checked_add(start_page))
            .ok_or(HeapError::InvalidPageId { index: arena_index })?;
        let first_page = PageId::new(first_page)?;

        Ok(Some(PageRun::new(first_page, page_count as usize)?))
    }

    /// Report whether this arena still has capacity for one fresh run.
    pub(super) fn has_capacity(
        &self,
        page_count: usize,
        pages_per_arena: usize,
    ) -> HeapResult<bool> {
        let start_page = self.next_unused_page.load(Ordering::Acquire) as usize;
        let Some(end_page) = start_page.checked_add(page_count) else {
            return Err(HeapError::InvalidPageId { index: start_page });
        };

        Ok(end_page <= pages_per_arena)
    }

    /// Return one run refcount by arena-local page index.
    pub(super) fn run_refcount(&self, arena_page_index: usize) -> Option<&AtomicU32> {
        self.run_refcounts.get(arena_page_index)
    }

    /// Raise the fresh-allocation watermark to the given page index.
    pub(super) fn raise_watermark(&self, next_unused_page: usize) {
        self.next_unused_page
            .fetch_max(next_unused_page as u32, Ordering::AcqRel);
    }
}

/// One sparse logical arena directory.
#[derive(Debug)]
pub(super) struct ArenaDirectory {
    /// The maximum arena index stored in this directory.
    entry_count: usize,
    /// The sparse directory chunks.
    chunks: Box<[AtomicPtr<ArenaDirectoryChunk>]>,
}

impl ArenaDirectory {
    /// Create one empty arena directory for the given arena count.
    pub(super) fn new(entry_count: usize) -> Self {
        let chunk_count = entry_count.div_ceil(ArenaDirectoryChunk::LEN);

        Self {
            entry_count,
            chunks: atomic_ptr_slice(chunk_count),
        }
    }

    /// Return one arena by logical arena index.
    pub(super) fn get(&self, arena_index: usize) -> Option<&Arena> {
        if arena_index >= self.entry_count {
            return None;
        }

        let chunk_index = arena_index / ArenaDirectoryChunk::LEN;
        let slot_index = arena_index % ArenaDirectoryChunk::LEN;
        let chunk = self.chunks.get(chunk_index)?.load(Ordering::Acquire);
        if chunk.is_null() {
            return None;
        }

        let arena = unsafe { &*chunk }.arenas[slot_index].load(Ordering::Acquire);
        if arena.is_null() {
            return None;
        }

        Some(unsafe { &*arena })
    }

    /// Insert one arena pointer at one logical arena index.
    pub(super) fn insert(&self, arena_index: usize, arena: *mut Arena) -> HeapResult<()> {
        if arena_index >= self.entry_count {
            return Err(HeapError::AllocatorArenaLimitExceeded {
                required_arenas: arena_index + 1,
                max_arenas: self.entry_count,
            });
        }

        let chunk_index = arena_index / ArenaDirectoryChunk::LEN;
        let slot_index = arena_index % ArenaDirectoryChunk::LEN;
        let chunk = self.ensure_chunk(chunk_index)?;

        if chunk.arenas[slot_index]
            .compare_exchange(null_mut(), arena, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(HeapError::InvariantViolation {
                context: "allocator arena directory entry installed twice",
            });
        }

        Ok(())
    }

    /// Ensure one sparse directory chunk exists.
    fn ensure_chunk(&self, chunk_index: usize) -> HeapResult<&ArenaDirectoryChunk> {
        let slot = self
            .chunks
            .get(chunk_index)
            .ok_or(HeapError::InvariantViolation {
                context: "allocator arena directory chunk missing",
            })?;
        let current = slot.load(Ordering::Acquire);
        if !current.is_null() {
            return Ok(unsafe { &*current });
        }

        let chunk = Box::into_raw(Box::new(ArenaDirectoryChunk::new()));
        match slot.compare_exchange(null_mut(), chunk, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => Ok(unsafe { &*chunk }),
            Err(current) => {
                unsafe {
                    drop(Box::from_raw(chunk));
                }

                Ok(unsafe { &*current })
            }
        }
    }
}

impl Drop for ArenaDirectory {
    fn drop(&mut self) {
        drop_atomic_ptr_chunks(&self.chunks);
    }
}

/// One sparse arena-directory chunk.
#[derive(Debug)]
struct ArenaDirectoryChunk {
    /// The arena pointers stored in this chunk.
    arenas: Box<[AtomicPtr<Arena>]>,
}

impl ArenaDirectoryChunk {
    /// The number of arena pointers stored in one chunk.
    const LEN: usize = 1 << 12;

    /// Create one empty arena-directory chunk.
    fn new() -> Self {
        Self {
            arenas: atomic_ptr_slice(Self::LEN),
        }
    }
}

/// One sparse address-map chunk.
#[derive(Debug)]
struct ArenaAddressChunk {
    /// The arena pointers stored in this chunk.
    arenas: Box<[AtomicPtr<Arena>]>,
}

impl ArenaAddressChunk {
    /// The number of arena pointers stored in one chunk.
    const LEN: usize = 1 << 12;

    /// Create one empty address-map chunk.
    fn new() -> Self {
        Self {
            arenas: atomic_ptr_slice(Self::LEN),
        }
    }
}

/// One sparse arena address map.
#[derive(Debug)]
pub(super) struct ArenaAddressMap {
    /// The maximum index stored in this map.
    entry_count: usize,
    /// The sparse address-map chunks.
    chunks: Box<[AtomicPtr<ArenaAddressChunk>]>,
}

impl ArenaAddressMap {
    /// Create one sparse arena address map for the given entry count.
    pub(super) fn new(entry_count: usize) -> Self {
        let chunk_count = entry_count.div_ceil(ArenaAddressChunk::LEN);

        Self {
            entry_count,
            chunks: atomic_ptr_slice(chunk_count),
        }
    }

    /// Return one arena by sparse address-map index.
    pub(super) fn get(&self, index: usize) -> Option<&Arena> {
        if index >= self.entry_count {
            return None;
        }

        let chunk_index = index / ArenaAddressChunk::LEN;
        let slot_index = index % ArenaAddressChunk::LEN;
        let chunk = self.chunks.get(chunk_index)?.load(Ordering::Acquire);
        if chunk.is_null() {
            return None;
        }

        let arena = unsafe { &*chunk }.arenas[slot_index].load(Ordering::Acquire);
        if arena.is_null() {
            return None;
        }

        Some(unsafe { &*arena })
    }

    /// Insert one arena pointer at one sparse address-map index.
    pub(super) fn insert(&self, index: usize, arena: *mut Arena) -> HeapResult<()> {
        if index >= self.entry_count {
            return Err(HeapError::AllocatorArenaMapIndexUnsupported { index });
        }

        let chunk_index = index / ArenaAddressChunk::LEN;
        let slot_index = index % ArenaAddressChunk::LEN;
        let chunk = self.ensure_chunk(chunk_index)?;
        if chunk.arenas[slot_index]
            .compare_exchange(null_mut(), arena, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(HeapError::InvariantViolation {
                context: "allocator arena address-map entry installed twice",
            });
        }

        Ok(())
    }

    /// Ensure one sparse address-map chunk exists.
    fn ensure_chunk(&self, chunk_index: usize) -> HeapResult<&ArenaAddressChunk> {
        let slot = self
            .chunks
            .get(chunk_index)
            .ok_or(HeapError::InvariantViolation {
                context: "allocator arena address-map chunk missing",
            })?;
        let current = slot.load(Ordering::Acquire);
        if !current.is_null() {
            return Ok(unsafe { &*current });
        }

        let chunk = Box::into_raw(Box::new(ArenaAddressChunk::new()));
        match slot.compare_exchange(null_mut(), chunk, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => Ok(unsafe { &*chunk }),
            Err(current) => {
                unsafe {
                    drop(Box::from_raw(chunk));
                }

                Ok(unsafe { &*current })
            }
        }
    }
}

impl Drop for ArenaAddressMap {
    fn drop(&mut self) {
        drop_atomic_ptr_chunks(&self.chunks);
    }
}

/// Return one boxed slice of null atomic pointers.
fn atomic_ptr_slice<T>(len: usize) -> Box<[AtomicPtr<T>]> {
    let mut pointers = Vec::with_capacity(len);

    for _ in 0..len {
        pointers.push(AtomicPtr::new(null_mut()));
    }

    pointers.into_boxed_slice()
}

/// Drop every initialized chunk in one atomic pointer slice.
fn drop_atomic_ptr_chunks<T>(chunks: &[AtomicPtr<T>]) {
    for chunk in chunks {
        let chunk = chunk.load(Ordering::Acquire);
        if chunk.is_null() {
            continue;
        }

        unsafe {
            drop(Box::from_raw(chunk));
        }
    }
}

/// Return the maximum allocator arena count addressable by page ids.
pub(super) fn max_arena_count(page_bytes: usize, arena_bytes: usize) -> usize {
    (u32::MAX as usize) / (arena_bytes / page_bytes)
}

/// Return the number of sparse arena-map entries addressable for this arena size.
pub(super) fn max_arena_frame_count(arena_bytes: usize) -> HeapResult<usize> {
    let address_bits = arena_address_bits();
    let arena_shift = arena_bytes.trailing_zeros() as usize;
    if arena_shift >= address_bits {
        return Err(HeapError::InvalidAllocatorArenaBytes { bytes: arena_bytes });
    }

    1usize
        .checked_shl((address_bits - arena_shift) as u32)
        .ok_or(HeapError::InvariantOverflow {
            context: "allocator arena-map entry count",
        })
}

/// Return one sparse arena-map index for one live arena address.
pub(super) fn arena_frame_index(address: usize, arena_bytes: usize) -> Option<usize> {
    if address == 0 {
        return None;
    }

    let address_bits = arena_address_bits();
    let address = if address_bits == usize::BITS as usize {
        address
    } else {
        address & ((1usize << address_bits) - 1)
    };

    Some(address / arena_bytes)
}

/// Return the supported user virtual address width for arena-map indexing.
pub(super) const fn arena_address_bits() -> usize {
    if cfg!(target_pointer_width = "64") {
        48
    } else {
        usize::BITS as usize
    }
}

/// Allocate one mapped allocator arena.
pub(super) fn allocate_arena_bytes(byte_len: usize) -> HeapResult<NonNull<u8>> {
    let mapped_byte_len = byte_len
        .checked_mul(2)
        .ok_or(HeapError::InvariantOverflow {
            context: "allocator arena alignment reservation",
        })?;
    let flags = libc::MAP_PRIVATE
        | if cfg!(any(target_os = "macos", target_os = "ios")) {
            libc::MAP_ANON
        } else {
            libc::MAP_ANONYMOUS
        };
    let data = unsafe {
        libc::mmap(
            null_mut(),
            mapped_byte_len,
            libc::PROT_READ | libc::PROT_WRITE,
            flags,
            -1,
            0,
        )
    };

    if data == libc::MAP_FAILED {
        return Err(HeapError::AllocatorArenaAllocationFailed {
            byte_len: mapped_byte_len,
        });
    }

    let mapped_base = data as usize;
    let arena_base = align_up(mapped_base, byte_len).ok_or(HeapError::InvariantOverflow {
        context: "allocator arena aligned base",
    })?;
    let mapped_end =
        mapped_base
            .checked_add(mapped_byte_len)
            .ok_or(HeapError::InvariantOverflow {
                context: "allocator arena reservation end",
            })?;
    let arena_end = arena_base
        .checked_add(byte_len)
        .ok_or(HeapError::InvariantOverflow {
            context: "allocator arena aligned end",
        })?;

    // trim the prefix before the aligned arena
    let prefix_byte_len = arena_base - mapped_base;
    if prefix_byte_len != 0 {
        unsafe {
            libc::munmap(data, prefix_byte_len);
        }
    }

    // trim the suffix after the aligned arena
    let suffix_byte_len = mapped_end - arena_end;
    if suffix_byte_len != 0 {
        unsafe {
            libc::munmap((arena_end as *mut u8).cast(), suffix_byte_len);
        }
    }

    NonNull::new(arena_base as *mut u8).ok_or(HeapError::AllocatorAddressUnsupported {
        address: arena_base,
    })
}

/// Free one mapped allocator arena.
pub(super) fn free_arena_bytes(data: *mut u8, byte_len: usize) {
    if data.is_null() {
        return;
    }

    unsafe {
        libc::munmap(data.cast(), byte_len);
    }
}

/// Return one address rounded up to the given alignment.
fn align_up(address: usize, alignment: usize) -> Option<usize> {
    let mask = alignment.checked_sub(1)?;
    let address = address.checked_add(mask)?;

    Some(address & !mask)
}

/// One allocator arena location resolved from one live address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ArenaLocation {
    /// The owning arena index.
    pub(crate) arena_index: usize,
    /// The byte offset inside the arena.
    pub(crate) arena_offset: usize,
}

#[cfg(test)]
mod tests {
    use super::{allocate_arena_bytes, free_arena_bytes};

    #[test]
    fn test_allocate_arena_bytes_aligns_to_arena_size() {
        let byte_len = 64 * 1024;
        let data = allocate_arena_bytes(byte_len).expect("arena bytes should map");

        assert_eq!(data.as_ptr() as usize % byte_len, 0);

        free_arena_bytes(data.as_ptr(), byte_len);
    }
}
