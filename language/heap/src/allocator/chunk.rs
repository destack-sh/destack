use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};

use super::{PageId, PageRun, platform};
use crate::{HeapError, HeapResult};

/// One fixed chunk in the allocator.
#[derive(Debug)]
pub(super) struct Chunk {
    /// The mapped chunk base.
    base: *mut u8,
    /// The next never-allocated page inside this chunk.
    pub(super) next_unused_page: AtomicU32,
    /// The run owner counts keyed by run-start page index inside this chunk.
    run_owner_counts: Box<[AtomicU32]>,
}

impl Chunk {
    /// Create one chunk metadata record.
    pub(super) fn new(base: *mut u8, pages_per_chunk: usize) -> Self {
        Self {
            base,
            next_unused_page: AtomicU32::new(0),
            run_owner_counts: std::iter::repeat_with(|| AtomicU32::new(0))
                .take(pages_per_chunk)
                .collect(),
        }
    }

    /// Return one page pointer inside this chunk.
    pub(super) fn page_ptr(&self, chunk_page_index: usize, page_bytes: usize) -> Option<*mut u8> {
        let data = self.base;
        let page_offset = chunk_page_index * page_bytes;

        Some(unsafe { data.add(page_offset) })
    }

    /// Allocate one run from this chunk.
    pub(super) fn allocate_run(
        &self,
        chunk_index: usize,
        page_count: usize,
        pages_per_chunk: usize,
    ) -> HeapResult<Option<PageRun>> {
        let page_count = u32::try_from(page_count)
            .map_err(|_| HeapError::InvalidPageId { index: page_count })?;

        let start_page = loop {
            // read the current chunk tail
            let start_page = self.next_unused_page.load(Ordering::Acquire);
            let end_page = start_page + page_count;

            // stop once this chunk is exhausted
            if end_page as usize > pages_per_chunk {
                return Ok(None);
            }

            // claim the run by moving the tail
            if self
                .next_unused_page
                .compare_exchange(start_page, end_page, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                break start_page as usize;
            }
        };

        // convert chunk-local pages to global page ids
        let first_page = chunk_index * pages_per_chunk + start_page;
        let first_page = PageId::new(first_page)?;

        Ok(Some(PageRun::new(first_page, page_count as usize)?))
    }

    /// Report whether this chunk still has capacity for one run.
    pub(super) fn has_capacity(
        &self,
        page_count: usize,
        pages_per_chunk: usize,
    ) -> HeapResult<bool> {
        let start_page = self.next_unused_page.load(Ordering::Acquire) as usize;
        let end_page = start_page + page_count;

        Ok(end_page <= pages_per_chunk)
    }

    /// Return one run owner count by chunk-local page index.
    pub(super) fn run_owner_count(&self, chunk_page_index: usize) -> &AtomicU32 {
        &self.run_owner_counts[chunk_page_index]
    }

    /// Raise the allocation watermark to the given page index.
    pub(super) fn raise_watermark(&self, next_unused_page: usize) {
        self.next_unused_page
            .fetch_max(next_unused_page as u32, Ordering::AcqRel);
    }
}

/// One aligned virtual memory range that backs allocator chunks.
#[derive(Debug)]
pub(super) struct ChunkReservation {
    /// The aligned reservation base.
    base: *mut u8,
    /// The reserved byte length.
    reservation_bytes: usize,
    /// The chunk byte length.
    chunk_bytes: usize,
    /// The next uncommitted chunk offset.
    next_chunk_offset: usize,
}

impl ChunkReservation {
    /// Reserve one aligned range for allocator chunks.
    pub(super) fn reserve(chunk_bytes: usize, chunk_count: usize) -> HeapResult<Self> {
        let reservation_bytes = chunk_bytes * chunk_count;
        let base = reserve_aligned_chunk_range(reservation_bytes, chunk_bytes)?;

        Ok(Self {
            base,
            reservation_bytes,
            chunk_bytes,
            next_chunk_offset: 0,
        })
    }

    /// Return the number of uncommitted chunks left in this reservation.
    pub(super) fn remaining_chunk_count(&self, chunk_bytes: usize) -> usize {
        (self.reservation_bytes - self.next_chunk_offset) / chunk_bytes
    }

    /// Commit and return the next chunk in this reservation.
    pub(super) fn commit_chunk(&mut self, chunk_bytes: usize) -> HeapResult<Option<*mut u8>> {
        let next_chunk_offset = self.next_chunk_offset + chunk_bytes;

        // stop once this reservation is exhausted
        if next_chunk_offset > self.reservation_bytes {
            return Ok(None);
        }

        // commit the next chunk in place
        let data = unsafe { self.base.add(self.next_chunk_offset) };
        platform::commit_chunk_space(data, chunk_bytes)?;
        self.next_chunk_offset = next_chunk_offset;

        Ok(Some(data))
    }
}

impl Drop for ChunkReservation {
    fn drop(&mut self) {
        let mut offset = 0;

        // release committed chunks
        while offset < self.next_chunk_offset {
            let data = unsafe { self.base.add(offset) };
            try_unmap_chunk_range(data, self.chunk_bytes);
            offset += self.chunk_bytes;
        }

        // release the remaining reservation
        let byte_len = self.reservation_bytes - self.next_chunk_offset;
        let data = unsafe { self.base.add(self.next_chunk_offset) };
        try_unmap_chunk_range(data, byte_len);
    }
}

/// Chunk lookup by logical index.
#[derive(Debug)]
pub(super) struct ChunkIndex {
    /// The chunks keyed by logical chunk index.
    chunks: Box<[AtomicPtr<Chunk>]>,
}

impl ChunkIndex {
    /// Create one empty chunk index.
    pub(super) fn new(chunk_count: usize) -> Self {
        Self {
            chunks: atomic_ptr_slice(chunk_count),
        }
    }

    /// Return one chunk by logical chunk index.
    pub(super) fn chunk(&self, chunk_index: usize) -> Option<&Chunk> {
        let chunk = self.chunks.get(chunk_index)?.load(Ordering::Acquire);

        // missing chunks are represented by null pointers
        if chunk.is_null() {
            return None;
        }

        Some(unsafe { &*chunk })
    }

    /// Insert one chunk by logical chunk index.
    pub(super) fn insert(&self, chunk_index: usize, chunk: *mut Chunk) -> HeapResult<()> {
        let Some(slot) = self.chunks.get(chunk_index) else {
            return Err(HeapError::AllocatorChunkLimitExceeded {
                required_chunks: chunk_index + 1,
                max_chunks: self.chunks.len(),
            });
        };

        // publish exactly once
        if slot
            .compare_exchange(null_mut(), chunk, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(HeapError::InvariantViolation {
                context: "allocator chunk index entry installed twice",
            });
        }

        Ok(())
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

/// Return the maximum allocator chunk count addressable by page ids.
pub(super) fn max_chunk_count(page_bytes: usize, chunk_bytes: usize) -> usize {
    (u32::MAX as usize) / (chunk_bytes / page_bytes)
}

/// Reserve one aligned virtual address range for allocator chunks.
fn reserve_aligned_chunk_range(byte_len: usize, alignment_bytes: usize) -> HeapResult<*mut u8> {
    let system_page_bytes = platform::system_page_bytes()?;
    let slack_bytes = alignment_bytes.checked_sub(system_page_bytes).ok_or(
        HeapError::InvalidAllocatorChunkBytes {
            bytes: alignment_bytes,
        },
    )?;
    let reservation_byte_len =
        byte_len
            .checked_add(slack_bytes)
            .ok_or(HeapError::InvariantOverflow {
                context: "allocator chunk reservation length",
            })?;
    let reservation = platform::reserve_chunk_space(reservation_byte_len)?;
    let reservation_base = reservation as usize;
    let chunk_base =
        align_up(reservation_base, alignment_bytes).ok_or(HeapError::InvariantOverflow {
            context: "allocator chunk aligned base",
        })?;

    unmap_chunk_alignment_slack(reservation, reservation_byte_len, chunk_base, byte_len)?;
    let chunk_base = chunk_base as *mut u8;
    if chunk_base.is_null() {
        return Err(HeapError::AllocatorAddressUnsupported { address: 0 });
    }

    Ok(chunk_base)
}

/// Unmap reservation slack around the aligned chunk range.
fn unmap_chunk_alignment_slack(
    reservation: *mut u8,
    reservation_byte_len: usize,
    chunk_base: usize,
    byte_len: usize,
) -> HeapResult<()> {
    let reservation_base = reservation as usize;
    let reservation_end =
        reservation_base
            .checked_add(reservation_byte_len)
            .ok_or(HeapError::InvariantOverflow {
                context: "allocator chunk reservation end",
            })?;
    let chunk_end = chunk_base
        .checked_add(byte_len)
        .ok_or(HeapError::InvariantOverflow {
            context: "allocator chunk aligned end",
        })?;

    let prefix_bytes = chunk_base - reservation_base;
    if let Err(error) = platform::unmap_chunk_space(reservation, prefix_bytes) {
        try_unmap_chunk_range(reservation, reservation_byte_len);

        return Err(error);
    }

    let suffix_bytes = reservation_end - chunk_end;
    if let Err(error) = platform::unmap_chunk_space((chunk_end as *mut u8).cast(), suffix_bytes) {
        try_unmap_chunk_range((chunk_base as *mut u8).cast(), byte_len + suffix_bytes);

        return Err(error);
    }

    Ok(())
}

/// Try to unmap one chunk range during cleanup.
fn try_unmap_chunk_range(data: *mut u8, byte_len: usize) {
    let _ = platform::unmap_chunk_space(data, byte_len);
}

/// Return one address rounded up to the given alignment.
fn align_up(address: usize, alignment: usize) -> Option<usize> {
    let mask = alignment.checked_sub(1)?;
    let address = address.checked_add(mask)?;

    Some(address & !mask)
}

#[cfg(test)]
mod tests {
    use super::ChunkReservation;

    #[test]
    fn test_chunk_reservation_commits_aligned_chunks() {
        let chunk_bytes = 64 * 1024;
        let mut reservation =
            ChunkReservation::reserve(chunk_bytes, 2).expect("chunk reservation should map");
        let first = reservation
            .commit_chunk(chunk_bytes)
            .expect("first chunk should commit")
            .expect("first chunk should exist");
        let second = reservation
            .commit_chunk(chunk_bytes)
            .expect("second chunk should commit")
            .expect("second chunk should exist");

        assert_eq!(first as usize % chunk_bytes, 0);
        assert_eq!(second as usize - first as usize, chunk_bytes);
        assert_eq!(reservation.remaining_chunk_count(chunk_bytes), 0);
    }
}
