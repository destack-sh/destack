use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};

use super::{PageId, PageRun};
use crate::{HeapError, HeapResult};

/// One fixed chunk in the allocator.
#[derive(Debug)]
pub(super) struct Chunk {
    /// The next never-allocated page inside this chunk.
    pub(super) next_unused_page: AtomicU32,
    /// The run reference counts keyed by run-start page index inside this chunk.
    run_ref_counts: Box<[AtomicU32]>,
}

impl Chunk {
    /// Create one chunk metadata record.
    pub(super) fn new(pages_per_chunk: usize) -> Self {
        Self {
            next_unused_page: AtomicU32::new(0),
            run_ref_counts: std::iter::repeat_with(|| AtomicU32::new(0))
                .take(pages_per_chunk)
                .collect(),
        }
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

    /// Return one run reference count by chunk-local page index.
    pub(super) fn run_ref_count(&self, chunk_page_index: usize) -> &AtomicU32 {
        &self.run_ref_counts[chunk_page_index]
    }

    /// Raise the allocation watermark to the given page index.
    pub(super) fn raise_watermark(&self, next_unused_page: usize) {
        self.next_unused_page
            .fetch_max(next_unused_page as u32, Ordering::AcqRel);
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
