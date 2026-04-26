use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};

use super::{ALLOCATOR_ADDRESS_BITS, ARENA_TABLE_CHUNK_LEN, PageId, PageRun};
use crate::{HeapError, HeapResult};

/// One fixed arena in the allocator.
#[derive(Debug)]
pub(super) struct Arena {
    /// The logical arena index.
    index: usize,
    /// The mapped arena base.
    base: *mut u8,
    /// The next never-allocated page inside this arena.
    pub(super) next_unused_page: AtomicU32,
    /// The run refcounts keyed by run-start page index inside this arena.
    run_refcounts: Box<[AtomicU32]>,
}

impl Arena {
    /// Create one arena metadata record.
    pub(super) fn new(index: usize, base: *mut u8, pages_per_arena: usize) -> Self {
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
    pub(super) const fn base(&self) -> *mut u8 {
        self.base
    }

    /// Return one page pointer inside this arena.
    pub(super) fn page_ptr(&self, arena_page_index: usize, page_bytes: usize) -> Option<*mut u8> {
        let data = self.base;
        let page_offset = arena_page_index.checked_mul(page_bytes)?;

        Some(unsafe { data.add(page_offset) })
    }

    /// Allocate one run from this arena.
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

    /// Report whether this arena still has capacity for one run.
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
    pub(super) fn run_refcount(&self, arena_page_index: usize) -> &AtomicU32 {
        &self.run_refcounts[arena_page_index]
    }

    /// Raise the allocation watermark to the given page index.
    pub(super) fn raise_watermark(&self, next_unused_page: usize) {
        self.next_unused_page
            .fetch_max(next_unused_page as u32, Ordering::AcqRel);
    }
}

/// One aligned virtual memory range that backs allocator arenas.
#[derive(Debug)]
pub(super) struct ArenaReservation {
    /// The aligned reservation base.
    base: *mut u8,
    /// The reserved byte length.
    byte_len: usize,
    /// The next uncommitted arena offset.
    next_offset: usize,
}

impl ArenaReservation {
    /// Reserve one aligned range for allocator arenas.
    pub(super) fn reserve(arena_bytes: usize, arena_count: usize) -> HeapResult<Self> {
        let reservation_bytes =
            arena_bytes
                .checked_mul(arena_count)
                .ok_or(HeapError::InvariantOverflow {
                    context: "allocator arena reservation length",
                })?;
        let base = reserve_aligned_arena_range(reservation_bytes, arena_bytes)?;

        Ok(Self {
            base,
            byte_len: reservation_bytes,
            next_offset: 0,
        })
    }

    /// Return the number of uncommitted arenas left in this reservation.
    pub(super) fn remaining_arena_count(&self, arena_bytes: usize) -> usize {
        (self.byte_len - self.next_offset) / arena_bytes
    }

    /// Commit and return the next arena in this reservation.
    pub(super) fn allocate_arena(&mut self, arena_bytes: usize) -> HeapResult<Option<*mut u8>> {
        let Some(next_offset) = self.next_offset.checked_add(arena_bytes) else {
            return Err(HeapError::InvariantOverflow {
                context: "allocator arena reservation offset",
            });
        };
        if next_offset > self.byte_len {
            return Ok(None);
        }

        let data = unsafe { self.base.add(self.next_offset) };
        commit_arena_bytes(data, arena_bytes)?;
        self.next_offset = next_offset;

        Ok(Some(data))
    }
}

impl Drop for ArenaReservation {
    fn drop(&mut self) {
        try_unmap_arena_range(self.base, self.byte_len);
    }
}

/// Arena lookup by logical index and address frame.
#[derive(Debug)]
pub(super) struct ArenaIndex {
    /// The arenas keyed by logical arena index.
    by_index: ArenaTable,
    /// The arenas keyed by mapped arena address frame.
    by_address: ArenaTable,
}

impl ArenaIndex {
    /// Create one empty arena index.
    pub(super) fn new(arena_count: usize, address_frame_count: usize) -> Self {
        Self {
            by_index: ArenaTable::new(arena_count),
            by_address: ArenaTable::new(address_frame_count),
        }
    }

    /// Return one arena by logical arena index.
    pub(super) fn arena(&self, arena_index: usize) -> Option<&Arena> {
        self.by_index.get(arena_index)
    }

    /// Return one arena by mapped address frame.
    pub(super) fn arena_for_address(&self, address: usize, arena_bytes: usize) -> Option<&Arena> {
        let frame_index = arena_frame_index(address, arena_bytes)?;

        self.by_address.get(frame_index)
    }

    /// Insert one arena into both indexes.
    pub(super) fn insert(
        &self,
        arena_index: usize,
        address: usize,
        arena_bytes: usize,
        arena: *mut Arena,
    ) -> HeapResult<()> {
        if arena_index >= self.by_index.entry_count {
            return Err(HeapError::AllocatorArenaLimitExceeded {
                required_arenas: arena_index + 1,
                max_arenas: self.by_index.entry_count,
            });
        }

        let address_index = arena_frame_index(address, arena_bytes)
            .ok_or(HeapError::AllocatorAddressUnsupported { address })?;

        self.by_address.insert(address_index, arena)?;
        self.by_index.insert(arena_index, arena)
    }
}

/// Chunked arena pointer table.
#[derive(Debug)]
struct ArenaTable {
    /// The maximum index stored in this table.
    entry_count: usize,
    /// The lazily allocated arena-pointer chunks.
    chunks: Box<[AtomicPtr<ArenaTableChunk>]>,
}

impl ArenaTable {
    /// Create one arena pointer table for the given entry count.
    fn new(entry_count: usize) -> Self {
        let chunk_count = entry_count.div_ceil(ARENA_TABLE_CHUNK_LEN);

        Self {
            entry_count,
            chunks: atomic_ptr_slice(chunk_count),
        }
    }

    /// Return one arena by index.
    fn get(&self, index: usize) -> Option<&Arena> {
        if index >= self.entry_count {
            return None;
        }

        let chunk_index = index / ARENA_TABLE_CHUNK_LEN;
        let slot_index = index % ARENA_TABLE_CHUNK_LEN;
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

    /// Insert one arena pointer at one index.
    fn insert(&self, index: usize, arena: *mut Arena) -> HeapResult<()> {
        if index >= self.entry_count {
            return Err(HeapError::InvariantViolation {
                context: "allocator arena-table index",
            });
        }

        let chunk_index = index / ARENA_TABLE_CHUNK_LEN;
        let slot_index = index % ARENA_TABLE_CHUNK_LEN;
        let chunk = self.ensure_chunk(chunk_index)?;
        if chunk.arenas[slot_index]
            .compare_exchange(null_mut(), arena, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(HeapError::InvariantViolation {
                context: "allocator arena-table entry installed twice",
            });
        }

        Ok(())
    }

    /// Ensure one arena-pointer chunk exists.
    fn ensure_chunk(&self, chunk_index: usize) -> HeapResult<&ArenaTableChunk> {
        let slot = self
            .chunks
            .get(chunk_index)
            .ok_or(HeapError::InvariantViolation {
                context: "allocator arena-table chunk",
            })?;
        let current = slot.load(Ordering::Acquire);
        if !current.is_null() {
            return Ok(unsafe { &*current });
        }

        let chunk = Box::into_raw(Box::new(ArenaTableChunk::new()));
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

impl Drop for ArenaTable {
    fn drop(&mut self) {
        drop_atomic_ptr_chunks(&self.chunks);
    }
}

/// One arena-pointer chunk.
#[derive(Debug)]
struct ArenaTableChunk {
    /// The arena pointers stored in this chunk.
    arenas: Box<[AtomicPtr<Arena>]>,
}

impl ArenaTableChunk {
    /// Create one empty arena-pointer chunk.
    fn new() -> Self {
        Self {
            arenas: atomic_ptr_slice(ARENA_TABLE_CHUNK_LEN),
        }
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

/// Return the number of arena address-frame entries addressable for this arena size.
pub(super) fn max_arena_frame_count(arena_bytes: usize) -> HeapResult<usize> {
    let arena_shift = arena_bytes.trailing_zeros() as usize;
    if arena_shift >= ALLOCATOR_ADDRESS_BITS {
        return Err(HeapError::InvalidAllocatorArenaBytes { bytes: arena_bytes });
    }

    1usize
        .checked_shl((ALLOCATOR_ADDRESS_BITS - arena_shift) as u32)
        .ok_or(HeapError::InvariantOverflow {
            context: "allocator arena address-frame count",
        })
}

/// Return one arena address-frame index for one live arena address.
pub(super) fn arena_frame_index(address: usize, arena_bytes: usize) -> Option<usize> {
    if address == 0 {
        return None;
    }

    let address = if ALLOCATOR_ADDRESS_BITS == usize::BITS as usize {
        address
    } else {
        address & ((1usize << ALLOCATOR_ADDRESS_BITS) - 1)
    };

    Some(address / arena_bytes)
}

/// Reserve one aligned virtual address range for allocator arenas.
fn reserve_aligned_arena_range(byte_len: usize, alignment_bytes: usize) -> HeapResult<*mut u8> {
    let system_page_bytes = system_page_bytes()?;
    let slack_bytes = alignment_bytes.checked_sub(system_page_bytes).ok_or(
        HeapError::InvalidAllocatorArenaBytes {
            bytes: alignment_bytes,
        },
    )?;
    let reservation_byte_len =
        byte_len
            .checked_add(slack_bytes)
            .ok_or(HeapError::InvariantOverflow {
                context: "allocator arena reservation length",
            })?;
    let reservation = reserve_arena_bytes(reservation_byte_len)?;
    let reservation_base = reservation as usize;
    let arena_base =
        align_up(reservation_base, alignment_bytes).ok_or(HeapError::InvariantOverflow {
            context: "allocator arena aligned base",
        })?;

    trim_arena_reservation(reservation, reservation_byte_len, arena_base, byte_len)?;
    let arena_base = arena_base as *mut u8;
    if arena_base.is_null() {
        return Err(HeapError::AllocatorAddressUnsupported { address: 0 });
    }

    Ok(arena_base)
}

/// Reserve one inaccessible virtual address range for arena alignment.
fn reserve_arena_bytes(byte_len: usize) -> HeapResult<*mut u8> {
    let data = unsafe { libc::mmap(null_mut(), byte_len, libc::PROT_NONE, mmap_flags(), -1, 0) };
    if data == libc::MAP_FAILED {
        return Err(HeapError::AllocatorArenaAllocationFailed { byte_len });
    }

    let data: *mut u8 = data.cast();
    if data.is_null() {
        return Err(HeapError::AllocatorAddressUnsupported { address: 0 });
    }

    Ok(data)
}

/// Trim reservation slack around the aligned arena.
fn trim_arena_reservation(
    reservation: *mut u8,
    reservation_byte_len: usize,
    arena_base: usize,
    arena_bytes: usize,
) -> HeapResult<()> {
    let reservation_base = reservation as usize;
    let reservation_end =
        reservation_base
            .checked_add(reservation_byte_len)
            .ok_or(HeapError::InvariantOverflow {
                context: "allocator arena reservation end",
            })?;
    let arena_end = arena_base
        .checked_add(arena_bytes)
        .ok_or(HeapError::InvariantOverflow {
            context: "allocator arena aligned end",
        })?;

    let prefix_bytes = arena_base - reservation_base;
    if let Err(error) = unmap_arena_range(reservation, prefix_bytes) {
        try_unmap_arena_range(reservation, reservation_byte_len);

        return Err(error);
    }

    let suffix_bytes = reservation_end - arena_end;
    if let Err(error) = unmap_arena_range((arena_end as *mut u8).cast(), suffix_bytes) {
        try_unmap_arena_range((arena_base as *mut u8).cast(), arena_bytes + suffix_bytes);

        return Err(error);
    }

    Ok(())
}

/// Commit one aligned arena reservation for allocator payloads.
fn commit_arena_bytes(data: *mut u8, byte_len: usize) -> HeapResult<()> {
    let result =
        unsafe { libc::mprotect(data.cast(), byte_len, libc::PROT_READ | libc::PROT_WRITE) };
    if result == 0 {
        return Ok(());
    }

    Err(HeapError::AllocatorArenaAllocationFailed { byte_len })
}

/// Unmap one arena range and report unexpected OS failure.
fn unmap_arena_range(data: *mut u8, byte_len: usize) -> HeapResult<()> {
    if byte_len == 0 {
        return Ok(());
    }

    let result = unsafe { libc::munmap(data.cast(), byte_len) };
    if result == 0 {
        return Ok(());
    }

    Err(HeapError::InvariantViolation {
        context: "allocator arena unmap",
    })
}

/// Try to unmap one arena range during cleanup.
fn try_unmap_arena_range(data: *mut u8, byte_len: usize) {
    let _ = unmap_arena_range(data, byte_len);
}

/// Return the operating-system page width for virtual memory calls.
fn system_page_bytes() -> HeapResult<usize> {
    let page_bytes = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if page_bytes <= 0 {
        return Err(HeapError::InvariantViolation {
            context: "system page size",
        });
    }

    Ok(page_bytes as usize)
}

/// Return anonymous private mmap flags.
fn mmap_flags() -> i32 {
    libc::MAP_PRIVATE
        | if cfg!(any(target_os = "macos", target_os = "ios")) {
            libc::MAP_ANON
        } else {
            libc::MAP_ANONYMOUS
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
    use super::ArenaReservation;

    #[test]
    fn test_arena_reservation_commits_aligned_arenas() {
        let arena_bytes = 64 * 1024;
        let mut reservation =
            ArenaReservation::reserve(arena_bytes, 2).expect("arena reservation should map");
        let first = reservation
            .allocate_arena(arena_bytes)
            .expect("first arena should commit")
            .expect("first arena should exist");
        let second = reservation
            .allocate_arena(arena_bytes)
            .expect("second arena should commit")
            .expect("second arena should exist");

        assert_eq!(first as usize % arena_bytes, 0);
        assert_eq!(second as usize - first as usize, arena_bytes);
        assert_eq!(reservation.remaining_arena_count(arena_bytes), 0);
    }
}
