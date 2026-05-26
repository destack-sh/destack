use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};

use parking_lot::Mutex;

use super::{PageId, PageRun};
use crate::{HeapError, HeapResult};

/// The number of chunk pointers stored in one index segment.
const CHUNK_INDEX_SEGMENT_LEN: usize = 1024;

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
        debug_assert!(page_count <= pages_per_chunk);
        debug_assert!(u32::try_from(page_count).is_ok());
        let page_count = page_count as u32;

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
    pub(super) fn has_capacity(&self, page_count: usize, pages_per_chunk: usize) -> bool {
        let start_page = self.next_unused_page.load(Ordering::Acquire) as usize;
        let end_page = start_page + page_count;

        end_page <= pages_per_chunk
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
#[allow(clippy::vec_box)]
#[derive(Debug)]
pub(super) struct ChunkIndex {
    /// The maximum addressable chunk count.
    max_chunk_count: usize,
    /// The sparse chunk-index segments keyed by logical segment index.
    segments: Box<[AtomicPtr<ChunkIndexSegment>]>,
    /// The owned segments kept alive for lock-free readers.
    owned_segments: Mutex<Vec<Box<ChunkIndexSegment>>>,
}

impl ChunkIndex {
    /// Create one empty chunk index.
    pub(super) fn new(chunk_count: usize) -> Self {
        let segment_count = chunk_count.div_ceil(CHUNK_INDEX_SEGMENT_LEN);

        Self {
            max_chunk_count: chunk_count,
            segments: atomic_ptr_slice(segment_count),
            owned_segments: Mutex::new(Vec::new()),
        }
    }

    /// Return one chunk by logical chunk index.
    pub(super) fn chunk(&self, chunk_index: usize) -> Option<&Chunk> {
        if chunk_index >= self.max_chunk_count {
            return None;
        }

        let segment_index = chunk_index / CHUNK_INDEX_SEGMENT_LEN;
        let chunk_slot_index = chunk_index % CHUNK_INDEX_SEGMENT_LEN;
        let segment = self.segments.get(segment_index)?.load(Ordering::Acquire);

        // missing segments are represented by null pointers
        if segment.is_null() {
            return None;
        }

        // SAFETY: segments are owned by `owned_segments` until this index is dropped
        let segment = unsafe { &*segment };
        let chunk = segment.chunks[chunk_slot_index].load(Ordering::Acquire);

        // missing chunks are represented by null pointers
        if chunk.is_null() {
            return None;
        }

        // SAFETY: chunks are owned by allocator state until the allocator is dropped
        Some(unsafe { &*chunk })
    }

    /// Insert one chunk by logical chunk index.
    pub(super) fn insert(&self, chunk_index: usize, chunk: *mut Chunk) -> HeapResult<()> {
        if chunk_index >= self.max_chunk_count {
            return Err(HeapError::AllocatorChunkLimitExceeded {
                required_chunks: chunk_index + 1,
                max_chunks: self.max_chunk_count,
            });
        }

        let segment_index = chunk_index / CHUNK_INDEX_SEGMENT_LEN;
        let chunk_slot_index = chunk_index % CHUNK_INDEX_SEGMENT_LEN;
        let segment = self.segment_for_insert(segment_index);
        let slot = &segment.chunks[chunk_slot_index];

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

    /// Return one segment, allocating it when it does not exist yet.
    fn segment_for_insert(&self, segment_index: usize) -> &ChunkIndexSegment {
        let segment = self.segments[segment_index].load(Ordering::Acquire);
        if !segment.is_null() {
            // SAFETY: segments are owned by `owned_segments` until this index is dropped
            return unsafe { &*segment };
        }

        let mut owned_segments = self.owned_segments.lock();
        let segment = self.segments[segment_index].load(Ordering::Acquire);
        if !segment.is_null() {
            // SAFETY: segments are owned by `owned_segments` until this index is dropped
            return unsafe { &*segment };
        }

        let mut segment = Box::new(ChunkIndexSegment::new());
        let segment_ptr = segment.as_mut() as *mut ChunkIndexSegment;

        self.segments[segment_index].store(segment_ptr, Ordering::Release);
        owned_segments.push(segment);

        // SAFETY: the segment was just moved into `owned_segments`
        unsafe { &*segment_ptr }
    }
}

/// One sparse chunk-index segment.
#[derive(Debug)]
struct ChunkIndexSegment {
    /// The chunks keyed by segment-local index.
    chunks: Box<[AtomicPtr<Chunk>]>,
}

impl ChunkIndexSegment {
    /// Create one empty chunk-index segment.
    fn new() -> Self {
        Self {
            chunks: atomic_ptr_slice(CHUNK_INDEX_SEGMENT_LEN),
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

/// Return the maximum allocator chunk count addressable by page ids.
pub(super) fn max_chunk_count(page_bytes: usize, chunk_bytes: usize) -> usize {
    let pages_per_chunk = (chunk_bytes / page_bytes) as u64;
    let addressable_pages = u64::from(u32::MAX) + 1;
    let chunk_count = addressable_pages / pages_per_chunk;

    chunk_count.min(usize::MAX as u64) as usize
}
