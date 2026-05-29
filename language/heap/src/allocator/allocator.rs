use std::sync::atomic::{AtomicU32, Ordering};

use parking_lot::Mutex;

use super::chunk::{Chunk, ChunkIndex, max_chunk_count};
use super::{
    DEFAULT_ALLOCATOR_CHUNK_SIZE_BYTES, DEFAULT_PAGE_SIZE_BYTES, PageId, PageRun, PageRunSet,
};
use crate::{
    HeapError, HeapRepresentationError, HeapResult, validate_allocator_chunk_size_bytes,
    validate_page_size_bytes,
};

/// One branchable allocator of fixed-size pages.
#[derive(Debug)]
pub struct Allocator {
    /// The fixed page size for every page.
    page_size_bytes: u32,
    /// The fixed chunk size for every allocator chunk.
    chunk_size_bytes: u32,
    /// The number of pages stored in each allocator chunk.
    pages_per_chunk: u32,
    /// The maximum addressable chunk count.
    max_chunk_count: usize,
    /// The chunks keyed by logical index and address frame.
    chunk_index: ChunkIndex,
    /// The chunk frontier, current chunk, and free-run index.
    state: Mutex<AllocatorState>,
}

// SAFETY: allocator metadata is synchronized internally
unsafe impl Send for Allocator {}

// SAFETY: payload access is external, allocator metadata is synchronized internally
unsafe impl Sync for Allocator {}

impl Allocator {
    /// Create one allocator with the default page and chunk sizes.
    pub fn try_default() -> HeapResult<Self> {
        Self::try_new(DEFAULT_PAGE_SIZE_BYTES, DEFAULT_ALLOCATOR_CHUNK_SIZE_BYTES)
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
                free_runs: PageRunSet::new(),
            }),
        })
    }

    /// Return the fixed page size.
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
    pub fn allocate_pages(&self, byte_len: usize) -> HeapResult<PageRun> {
        let page_count = self.page_count(byte_len);

        self.allocate_run(page_count)
    }

    /// Release one page run after its metadata record drops it.
    pub fn release_page_run(&self, page_run: &PageRun) -> HeapResult<()> {
        self.decrement_run_ref_count(*page_run)
    }

    /// Release every page run after its metadata records drop them.
    pub fn release_page_runs(&self, page_runs: &[PageRun]) -> HeapResult<()> {
        // release in reverse so suffix runs recycle before prefix runs
        for page_run in page_runs.iter().rev() {
            self.release_page_run(page_run)?;
        }

        Ok(())
    }

    /// Return the exact retained bytes for one set of logical page runs.
    pub fn retained_bytes_for_page_runs<'a>(
        &self,
        page_runs: impl IntoIterator<Item = &'a PageRun>,
    ) -> u64 {
        let page_count = page_runs
            .into_iter()
            .map(|page_run| page_run.len())
            .sum::<usize>();

        page_count as u64 * self.page_size_bytes() as u64
    }

    /// Share one page run with another metadata record.
    pub(crate) fn share_page_run(&self, page_run: PageRun) -> HeapResult<PageRun> {
        self.increment_run_ref_count(page_run)?;

        Ok(page_run)
    }

    /// Return the number of pages required for one byte length.
    pub fn page_count(&self, byte_len: usize) -> usize {
        // empty byte ranges never retain pages
        if byte_len == 0 {
            return 0;
        }

        byte_len.div_ceil(self.page_size_bytes())
    }

    /// Allocate one logical page run.
    pub(super) fn allocate_run(&self, page_count: usize) -> HeapResult<PageRun> {
        // empty runs do not touch allocator state
        if page_count == 0 {
            return Ok(PageRun::empty());
        }

        // reuse an existing run when possible
        if let Some(run) = self.allocate_free_run(page_count) {
            self.initialize_run_ref_count(run)?;

            return Ok(run);
        }

        // carve from the current chunk when it fits
        let run = if page_count <= self.pages_per_chunk()
            && let Some(run) = self.allocate_run_from_current_chunk(page_count)?
        {
            run
        }
        // allocate a contiguous multi-chunk run for large requests
        else {
            self.allocate_multi_chunk_run(page_count)?
        };

        // publish the first metadata reference after the run is reserved
        self.initialize_run_ref_count(run)?;

        Ok(run)
    }

    /// Increment one allocated run reference count.
    fn increment_run_ref_count(&self, run: PageRun) -> HeapResult<()> {
        // empty runs have no reference count
        if run.is_empty() {
            return Ok(());
        }

        let ref_count = self.run_ref_count(run)?;

        loop {
            // read the current reference count
            let current_count = ref_count.load(Ordering::Acquire);

            // reject sharing after recycle
            if current_count == 0 {
                return Err(HeapError::Internal {
                    context: "allocator shared free run",
                });
            }

            // reject representational overflow
            if current_count == u32::MAX {
                return Err(HeapError::representation(
                    HeapRepresentationError::LimitExceeded {
                        context: "allocator run reference count",
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

    /// Decrement one allocated run reference count.
    pub(super) fn decrement_run_ref_count(&self, run: PageRun) -> HeapResult<()> {
        // empty runs have no reference count
        if run.is_empty() {
            return Ok(());
        }

        let ref_count = self.run_ref_count(run)?;

        loop {
            // read the current reference count
            let current_count = ref_count.load(Ordering::Acquire);

            // reject double release
            if current_count == 0 {
                return Err(HeapError::Internal {
                    context: "allocator released free run",
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
                self.recycle_run(run)?;
            }

            return Ok(());
        }
    }

    /// Report whether one physical run is uniquely owned.
    pub(super) fn is_run_unique(&self, run: PageRun) -> HeapResult<bool> {
        // empty runs are never shared
        if run.is_empty() {
            return Ok(true);
        }

        let ref_count = self.run_ref_count(run)?;

        Ok(ref_count.load(Ordering::Acquire) == 1)
    }

    /// Allocate one free run large enough for the requested size.
    fn allocate_free_run(&self, page_count: usize) -> Option<PageRun> {
        let mut state = self.state.lock();

        state.free_runs.allocate(page_count)
    }

    /// Allocate one run from the current chunk.
    fn allocate_run_from_current_chunk(&self, page_count: usize) -> HeapResult<Option<PageRun>> {
        loop {
            // claim from the current chunk
            let current_chunk_index = self.state.lock().current_chunk_index;
            if let Some(chunk_index) = current_chunk_index
                && let Some(run) = self.allocate_run_from_chunk(chunk_index, page_count)?
            {
                return Ok(Some(run));
            }

            // grow into the next chunk
            let chunk_index = self.current_chunk_for(page_count)?;
            if let Some(run) = self.allocate_run_from_chunk(chunk_index, page_count)? {
                return Ok(Some(run));
            }
        }
    }

    /// Allocate one run that crosses newly grown chunks.
    fn allocate_multi_chunk_run(&self, page_count: usize) -> HeapResult<PageRun> {
        let required_chunk_count = page_count.div_ceil(self.pages_per_chunk());
        let first_chunk_index = self.grow_chunk_range(required_chunk_count)?;
        let last_chunk_len = page_count % self.pages_per_chunk();

        // mark each newly grown chunk run as consumed
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

        PageRun::new(first_page, page_count)
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

    /// Allocate one run from one specific chunk.
    fn allocate_run_from_chunk(
        &self,
        chunk_index: usize,
        page_count: usize,
    ) -> HeapResult<Option<PageRun>> {
        let Some(chunk) = self.chunk(chunk_index) else {
            return Ok(None);
        };

        chunk.allocate_run(chunk_index, page_count, self.pages_per_chunk())
    }

    /// Report whether one chunk still has capacity for one run.
    fn chunk_has_capacity(&self, chunk_index: usize, page_count: usize) -> bool {
        let Some(chunk) = self.chunk(chunk_index) else {
            return false;
        };

        chunk.has_capacity(page_count, self.pages_per_chunk())
    }

    /// Return one logical run reference count.
    fn run_ref_count(&self, run: PageRun) -> HeapResult<&AtomicU32> {
        let (chunk_index, chunk_page_index) = self.chunk_position(run.first_page);
        let Some(chunk) = self.chunk(chunk_index) else {
            return Err(HeapError::internal("missing page"));
        };

        Ok(chunk.run_ref_count(chunk_page_index))
    }

    /// Initialize one run reference count before exposing it.
    fn initialize_run_ref_count(&self, run: PageRun) -> HeapResult<()> {
        let ref_count = self.run_ref_count(run)?;

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

    /// Recycle one dead run into the allocator free-run index.
    fn recycle_run(&self, run: PageRun) -> HeapResult<()> {
        let mut state = self.state.lock();

        state.free_runs.free(run);

        Ok(())
    }

    /// Recycle one cache-owned run into the allocator free-run index.
    pub(super) fn recycle_cached_run(&self, run: PageRun) -> HeapResult<()> {
        // empty runs do not touch allocator state
        if run.is_empty() {
            return Ok(());
        }

        let ref_count = self.run_ref_count(run)?;
        let current_count = ref_count.swap(0, Ordering::AcqRel);

        // cached runs must be uniquely owned
        if current_count != 1 {
            return Err(HeapError::Internal {
                context: "allocator cached run reference count",
            });
        }

        self.recycle_run(run)?;

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

/// The chunk frontier, current chunk, and free runs.
#[derive(Debug)]
struct AllocatorState {
    /// The number of chunks available to the page allocator.
    chunk_count: usize,
    /// The current chunk used for monotonic single-chunk allocation.
    current_chunk_index: Option<usize>,
    /// The free physical runs.
    free_runs: PageRunSet,
}
