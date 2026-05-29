use std::sync::atomic::{AtomicU32, Ordering};

use parking_lot::Mutex;

use super::chunk::{Chunk, ChunkIndex, max_chunk_count};
use super::{
    DEFAULT_ALLOCATOR_CHUNK_SIZE_BYTES, DEFAULT_ALLOCATOR_PAGE_SIZE_BYTES, PageId, PageSpan,
    PageSpanSet,
};
use crate::{
    HeapError, HeapRepresentationError, HeapResult, validate_allocator_chunk_size_bytes,
    validate_page_size_bytes,
};

/// One branchable allocator of fixed-size pages.
#[derive(Debug)]
pub struct Allocator {
    /// The fixed allocator page size.
    page_size_bytes: u32,
    /// The fixed chunk size for every allocator chunk.
    chunk_size_bytes: u32,
    /// The number of pages stored in each allocator chunk.
    pages_per_chunk: u32,
    /// The maximum addressable chunk count.
    max_chunk_count: usize,
    /// The chunks keyed by logical index and address frame.
    chunk_index: ChunkIndex,
    /// The chunk frontier, current chunk, and free-span index.
    state: Mutex<AllocatorState>,
}

// SAFETY: allocator metadata is synchronized internally
unsafe impl Send for Allocator {}

// SAFETY: payload access is external, allocator metadata is synchronized internally
unsafe impl Sync for Allocator {}

impl Allocator {
    /// Create one allocator with the default page and chunk sizes.
    pub fn try_default() -> HeapResult<Self> {
        Self::try_new(
            DEFAULT_ALLOCATOR_PAGE_SIZE_BYTES,
            DEFAULT_ALLOCATOR_CHUNK_SIZE_BYTES,
        )
    }

    /// Create one empty allocator with the given page and chunk sizes.
    pub fn try_new(page_size_bytes: usize, chunk_size_bytes: usize) -> HeapResult<Self> {
        let page_size_bytes = validate_page_size_bytes(page_size_bytes)?;
        let chunk_size_bytes =
            validate_allocator_chunk_size_bytes(page_size_bytes, chunk_size_bytes)?;
        let pages_per_chunk = chunk_size_bytes / page_size_bytes;
        let max_chunk_count = max_chunk_count(page_size_bytes, chunk_size_bytes);

        Ok(Self {
            page_size_bytes: page_size_bytes as u32,
            chunk_size_bytes: chunk_size_bytes as u32,
            pages_per_chunk: pages_per_chunk as u32,
            max_chunk_count,
            chunk_index: ChunkIndex::new(max_chunk_count),
            state: Mutex::new(AllocatorState {
                chunk_count: 0,
                current_chunk_index: None,
                free_spans: PageSpanSet::new(),
            }),
        })
    }

    /// Return the fixed allocator page size.
    pub const fn page_size_bytes(&self) -> usize {
        self.page_size_bytes as usize
    }

    /// Return the fixed chunk size.
    pub const fn chunk_size_bytes(&self) -> usize {
        self.chunk_size_bytes as usize
    }

    /// Return the maximum addressable chunk count.
    fn max_chunk_count(&self) -> usize {
        self.max_chunk_count
    }

    /// Allocate pages for one byte length.
    pub fn allocate_pages(&self, byte_len: usize) -> HeapResult<PageSpan> {
        let page_count = self.page_count(byte_len);

        self.allocate_span(page_count)
    }

    /// Release one page span after its metadata record drops it.
    pub fn release_page_span(&self, page_span: &PageSpan) -> HeapResult<()> {
        self.decrement_span_ref_count(*page_span)
    }

    /// Release every page span after its metadata records drop them.
    pub fn release_page_spans(&self, page_spans: &[PageSpan]) -> HeapResult<()> {
        // release in reverse so suffix spans recycle before prefix spans
        for page_span in page_spans.iter().rev() {
            self.release_page_span(page_span)?;
        }

        Ok(())
    }

    /// Return the exact retained bytes for one set of logical page spans.
    pub fn retained_bytes_for_page_spans<'a>(
        &self,
        page_spans: impl IntoIterator<Item = &'a PageSpan>,
    ) -> u64 {
        let page_count = page_spans
            .into_iter()
            .map(|page_span| page_span.len())
            .sum::<usize>();

        page_count as u64 * self.page_size_bytes() as u64
    }

    /// Share one page span with another metadata record.
    pub(crate) fn share_page_span(&self, page_span: PageSpan) -> HeapResult<PageSpan> {
        self.increment_span_ref_count(page_span)?;

        Ok(page_span)
    }

    /// Return the number of pages required for one byte length.
    pub fn page_count(&self, byte_len: usize) -> usize {
        // empty byte ranges never retain pages
        if byte_len == 0 {
            return 0;
        }

        byte_len.div_ceil(self.page_size_bytes())
    }

    /// Allocate one logical page span.
    pub(super) fn allocate_span(&self, page_count: usize) -> HeapResult<PageSpan> {
        // empty spans do not touch allocator state
        if page_count == 0 {
            return Ok(PageSpan::empty());
        }

        // reuse an existing span when possible
        if let Some(span) = self.allocate_free_span(page_count) {
            self.initialize_span_ref_count(span)?;

            return Ok(span);
        }

        // carve from the current chunk when it fits
        let span = if page_count <= self.pages_per_chunk()
            && let Some(span) = self.allocate_span_from_current_chunk(page_count)?
        {
            span
        }
        // allocate a contiguous multi-chunk span for large requests
        else {
            self.allocate_multi_chunk_span(page_count)?
        };

        // publish the first metadata reference after the span is reserved
        self.initialize_span_ref_count(span)?;

        Ok(span)
    }

    /// Increment one allocated span reference count.
    fn increment_span_ref_count(&self, span: PageSpan) -> HeapResult<()> {
        // empty spans have no reference count
        if span.is_empty() {
            return Ok(());
        }

        let ref_count = self.span_ref_count(span)?;

        loop {
            // read the current reference count
            let current_count = ref_count.load(Ordering::Acquire);

            // reject sharing after recycle
            if current_count == 0 {
                return Err(HeapError::Internal {
                    context: "allocator shared free span",
                });
            }

            // reject representational overflow
            if current_count == u32::MAX {
                return Err(HeapError::representation(
                    HeapRepresentationError::LimitExceeded {
                        context: "allocator span reference count",
                    },
                ));
            }

            let next_count = current_count + 1;

            // publish one more reference
            if ref_count
                .compare_exchange(
                    current_count,
                    next_count,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                return Ok(());
            }
        }
    }

    /// Decrement one allocated span reference count.
    pub(super) fn decrement_span_ref_count(&self, span: PageSpan) -> HeapResult<()> {
        // empty spans have no reference count
        if span.is_empty() {
            return Ok(());
        }

        let ref_count = self.span_ref_count(span)?;

        loop {
            // read the current reference count
            let current_count = ref_count.load(Ordering::Acquire);

            // reject double release
            if current_count == 0 {
                return Err(HeapError::Internal {
                    context: "allocator released free span",
                });
            }

            let next_count = current_count - 1;

            // publish one fewer reference
            if ref_count
                .compare_exchange(
                    current_count,
                    next_count,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_err()
            {
                continue;
            }

            // recycle only after the final reference drops
            if next_count == 0 {
                self.recycle_span(span)?;
            }

            return Ok(());
        }
    }

    /// Report whether one physical span is uniquely owned.
    pub(super) fn is_span_unique(&self, span: PageSpan) -> HeapResult<bool> {
        // empty spans are never shared
        if span.is_empty() {
            return Ok(true);
        }

        let ref_count = self.span_ref_count(span)?;

        Ok(ref_count.load(Ordering::Acquire) == 1)
    }

    /// Allocate one free span large enough for the requested size.
    fn allocate_free_span(&self, page_count: usize) -> Option<PageSpan> {
        let mut state = self.state.lock();

        state.free_spans.allocate(page_count)
    }

    /// Allocate one span from the current chunk.
    fn allocate_span_from_current_chunk(&self, page_count: usize) -> HeapResult<Option<PageSpan>> {
        loop {
            // claim from the current chunk
            let current_chunk_index = self.state.lock().current_chunk_index;
            if let Some(chunk_index) = current_chunk_index
                && let Some(span) = self.allocate_span_from_chunk(chunk_index, page_count)?
            {
                return Ok(Some(span));
            }

            // grow into the next chunk
            let chunk_index = self.current_chunk_for(page_count)?;
            if let Some(span) = self.allocate_span_from_chunk(chunk_index, page_count)? {
                return Ok(Some(span));
            }
        }
    }

    /// Allocate one span that crosses newly grown chunks.
    fn allocate_multi_chunk_span(&self, page_count: usize) -> HeapResult<PageSpan> {
        let required_chunk_count = page_count.div_ceil(self.pages_per_chunk());
        let first_chunk_index = self.grow_chunk_range(required_chunk_count)?;
        let last_chunk_len = page_count % self.pages_per_chunk();

        // mark each newly grown chunk span as consumed
        for chunk_offset in 0..required_chunk_count {
            let chunk_index = first_chunk_index + chunk_offset;
            let used_pages = if chunk_offset + 1 == required_chunk_count && last_chunk_len != 0 {
                last_chunk_len
            } else {
                self.pages_per_chunk()
            };

            let Some(chunk) = self.chunk(chunk_index) else {
                return Err(HeapError::representation(
                    HeapRepresentationError::AllocatorChunkLimitExceeded {
                        required_chunks: chunk_index + 1,
                        max_chunks: self.max_chunk_count(),
                    },
                ));
            };

            chunk.raise_watermark(used_pages);
        }

        let current_chunk_index =
            (last_chunk_len != 0).then_some(first_chunk_index + required_chunk_count - 1);
        self.state.lock().current_chunk_index = current_chunk_index;

        let first_page_index = first_chunk_index * self.pages_per_chunk();
        let first_page = PageId::new(first_page_index)?;

        PageSpan::new(first_page, page_count)
    }

    /// Grow the allocator by one contiguous chunk range.
    fn grow_chunk_range(&self, chunk_count: usize) -> HeapResult<usize> {
        let mut state = self.state.lock();
        let first_chunk_index = state.chunk_count;
        let end_chunk_index = first_chunk_index + chunk_count;

        let max_chunk_count = self.max_chunk_count();
        if end_chunk_index > max_chunk_count {
            return Err(HeapError::representation(
                HeapRepresentationError::AllocatorChunkLimitExceeded {
                    required_chunks: end_chunk_index,
                    max_chunks: max_chunk_count,
                },
            ));
        }

        // publish chunks before moving the frontier
        self.allocate_chunk_range(first_chunk_index, end_chunk_index)?;
        state.chunk_count = end_chunk_index;

        Ok(first_chunk_index)
    }

    /// Return one usable current chunk, growing one when necessary.
    fn current_chunk_for(&self, page_count: usize) -> HeapResult<usize> {
        let state = self.state.lock();
        let current_chunk_index = state.current_chunk_index;
        let chunk_count = state.chunk_count;
        drop(state);

        if let Some(chunk_index) = current_chunk_index
            && chunk_index < chunk_count
            && self.chunk_has_capacity(chunk_index, page_count)
        {
            return Ok(chunk_index);
        }

        let chunk_index = self.grow_chunk_range(1)?;
        self.state.lock().current_chunk_index = Some(chunk_index);

        Ok(chunk_index)
    }

    /// Allocate one span from one specific chunk.
    fn allocate_span_from_chunk(
        &self,
        chunk_index: usize,
        page_count: usize,
    ) -> HeapResult<Option<PageSpan>> {
        let Some(chunk) = self.chunk(chunk_index) else {
            return Ok(None);
        };

        chunk.allocate_span(chunk_index, page_count, self.pages_per_chunk())
    }

    /// Report whether one chunk still has capacity for one span.
    fn chunk_has_capacity(&self, chunk_index: usize, page_count: usize) -> bool {
        let Some(chunk) = self.chunk(chunk_index) else {
            return false;
        };

        chunk.has_capacity(page_count, self.pages_per_chunk())
    }

    /// Return one logical span reference count.
    fn span_ref_count(&self, span: PageSpan) -> HeapResult<&AtomicU32> {
        let (chunk_index, chunk_page_index) = self.chunk_position(span.first_page);
        let Some(chunk) = self.chunk(chunk_index) else {
            return Err(HeapError::internal("missing page"));
        };

        Ok(chunk.span_ref_count(chunk_page_index))
    }

    /// Initialize one span reference count before exposing it.
    fn initialize_span_ref_count(&self, span: PageSpan) -> HeapResult<()> {
        let ref_count = self.span_ref_count(span)?;

        ref_count.store(1, Ordering::Release);

        Ok(())
    }

    /// Return the chunk and page index for one page id.
    pub(super) fn chunk_position(&self, page_id: PageId) -> (usize, usize) {
        let pages_per_chunk = self.pages_per_chunk();
        let page_index = page_id.index();
        let chunk_index = page_index / pages_per_chunk;
        let chunk_page_index = page_index % pages_per_chunk;

        (chunk_index, chunk_page_index)
    }

    /// Return the number of pages stored in each chunk.
    pub(crate) const fn pages_per_chunk(&self) -> usize {
        self.pages_per_chunk as usize
    }

    /// Recycle one dead span into the allocator free-span index.
    fn recycle_span(&self, span: PageSpan) -> HeapResult<()> {
        let mut state = self.state.lock();

        state.free_spans.free(span);

        Ok(())
    }

    /// Recycle one cache-owned span into the allocator free-span index.
    pub(super) fn recycle_cached_span(&self, span: PageSpan) -> HeapResult<()> {
        // empty spans do not touch allocator state
        if span.is_empty() {
            return Ok(());
        }

        let ref_count = self.span_ref_count(span)?;
        let current_count = ref_count.swap(0, Ordering::AcqRel);

        // cached spans must be uniquely owned
        if current_count != 1 {
            return Err(HeapError::Internal {
                context: "allocator cached span reference count",
            });
        }

        self.recycle_span(span)?;

        Ok(())
    }

    /// Return one existing chunk by logical chunk index.
    fn chunk(&self, chunk_index: usize) -> Option<&Chunk> {
        self.chunk_index.chunk(chunk_index)
    }

    /// Allocate and publish one chunk range.
    fn allocate_chunk_range(
        &self,
        first_chunk_index: usize,
        end_chunk_index: usize,
    ) -> HeapResult<()> {
        for chunk_index in first_chunk_index..end_chunk_index {
            if self.chunk(chunk_index).is_some() {
                continue;
            }

            self.allocate_chunk(chunk_index)?;
        }

        Ok(())
    }

    /// Allocate and publish one chunk.
    fn allocate_chunk(&self, chunk_index: usize) -> HeapResult<()> {
        if chunk_index >= self.max_chunk_count() {
            return Err(HeapError::representation(
                HeapRepresentationError::AllocatorChunkLimitExceeded {
                    required_chunks: chunk_index + 1,
                    max_chunks: self.max_chunk_count(),
                },
            ));
        }

        // skip existing chunk
        if self.chunk(chunk_index).is_some() {
            return Ok(());
        }

        // allocate chunk metadata
        let chunk = Chunk::new(self.pages_per_chunk());
        self.chunk_index.insert(chunk_index, chunk)?;

        Ok(())
    }
}

/// The chunk frontier, current chunk, and free spans.
#[derive(Debug)]
struct AllocatorState {
    /// The number of chunks available to the page allocator.
    chunk_count: usize,
    /// The current chunk used for monotonic single-chunk block.
    current_chunk_index: Option<usize>,
    /// The free physical spans.
    free_spans: PageSpanSet,
}
