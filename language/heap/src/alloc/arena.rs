use std::alloc::{Layout, alloc_zeroed, dealloc};
use std::collections::BTreeMap;
use std::ptr::null_mut;
use std::sync::{Mutex, MutexGuard};

use super::{PageId, PageRun, PageView, Segment};
use crate::{
    DEFAULT_ARENA_SEGMENT_BYTES, DEFAULT_PAGE_BYTES, HeapError, HeapOptions, HeapResult,
    MAX_ARENA_SEGMENTS,
};

/// One branchable arena of fixed-width pages.
#[derive(Debug)]
pub struct Arena {
    /// The fixed page width for every page.
    page_bytes: u32,
    /// The fixed segment width for every arena segment.
    segment_bytes: u32,
    /// The number of pages stored in each arena segment.
    pages_per_segment: u32,
    /// The shared arena publication and reuse state.
    state: Mutex<ArenaState>,
}

/// One mutable arena directory and reusable-run state.
#[derive(Debug)]
struct ArenaState {
    /// The number of reserved arena segments.
    segment_count: usize,
    /// The append-only arena segment directory.
    segments: Vec<*mut Segment>,
    /// The free physical runs keyed by page count.
    free_runs: BTreeMap<usize, Vec<PageRun>>,
    /// The free physical runs keyed by first page index.
    free_runs_by_start: BTreeMap<usize, PageRun>,
}

// arena state is only accessed through the arena mutex
unsafe impl Send for ArenaState {}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        let state = self.lock_state();
        let segment_count = state.segment_count;
        for segment_index in 0..segment_count {
            let Some(&segment_ptr) = state.segments.get(segment_index) else {
                continue;
            };
            if segment_ptr.is_null() {
                continue;
            }

            unsafe {
                drop(Box::from_raw(segment_ptr));
            }
        }
    }
}

impl Arena {
    /// Create one empty arena with default page and segment widths.
    pub fn new() -> Self {
        Self {
            page_bytes: DEFAULT_PAGE_BYTES as u32,
            segment_bytes: DEFAULT_ARENA_SEGMENT_BYTES as u32,
            pages_per_segment: (DEFAULT_ARENA_SEGMENT_BYTES / DEFAULT_PAGE_BYTES) as u32,
            state: Mutex::new(ArenaState {
                segment_count: 0,
                segments: Vec::new(),
                free_runs: BTreeMap::new(),
                free_runs_by_start: BTreeMap::new(),
            }),
        }
    }

    /// Create one empty arena with the given page and segment widths.
    pub fn try_new(page_bytes: usize, segment_bytes: usize) -> HeapResult<Self> {
        let page_bytes = HeapOptions::validate_page_bytes(page_bytes)?;
        let segment_bytes = HeapOptions::validate_arena_segment_bytes(page_bytes, segment_bytes)?;
        let pages_per_segment = (segment_bytes / page_bytes) as u32;

        Ok(Self {
            page_bytes: page_bytes as u32,
            segment_bytes: segment_bytes as u32,
            pages_per_segment,
            state: Mutex::new(ArenaState {
                segment_count: 0,
                segments: Vec::new(),
                free_runs: BTreeMap::new(),
                free_runs_by_start: BTreeMap::new(),
            }),
        })
    }

    /// Return the fixed page width.
    pub const fn page_bytes(&self) -> usize {
        self.page_bytes as usize
    }

    /// Return the fixed segment width.
    pub const fn segment_bytes(&self) -> usize {
        self.segment_bytes as usize
    }

    /// Allocate zeroed pages for one byte length.
    pub fn allocate_zeroed(&self, byte_len: usize) -> HeapResult<PageView> {
        let page_count = self.page_count(byte_len);

        Ok(PageView::from_run(self.allocate_run(page_count)?))
    }

    /// Allocate pages and copy one byte slice into them.
    pub fn allocate_bytes(&self, bytes: &[u8]) -> HeapResult<PageView> {
        let mut page_view = self.allocate_zeroed(bytes.len())?;

        // initialize the new logical page range
        self.set_bytes(&mut page_view, 0, bytes)?;

        Ok(page_view)
    }

    /// Retain one logical page view for another live root.
    pub fn retain_pages(&self, page_view: &PageView) -> HeapResult<()> {
        // retain the base run when it still contributes visible pages
        if page_view.has_base_pages() {
            self.retain_run(page_view.base_run())?;
        }

        // retain each patched replacement run
        for patch in page_view.patches() {
            self.retain_run(PageRun::single_page(patch.page_id))?;
        }

        Ok(())
    }

    /// Return one cloned page view retained for another live root.
    pub fn clone_pages(&self, page_view: &PageView) -> HeapResult<PageView> {
        self.retain_pages(page_view)?;

        Ok(*page_view)
    }

    /// Release one logical page view after one root drops it.
    pub fn release_pages(&self, page_view: &PageView) -> HeapResult<()> {
        // release the base run when it still contributes visible pages
        if page_view.has_base_pages() {
            self.release_run(page_view.base_run())?;
        }

        // release each patched replacement run
        for patch in page_view.patches() {
            self.release_run(PageRun::single_page(patch.page_id))?;
        }

        Ok(())
    }

    /// Return the number of pages required for one byte length.
    pub fn page_count(&self, byte_len: usize) -> usize {
        if byte_len == 0 {
            return 0;
        }

        byte_len.div_ceil(self.page_bytes())
    }

    /// Allocate one zeroed physical run.
    pub(super) fn allocate_run(&self, page_count: usize) -> HeapResult<PageRun> {
        if page_count == 0 {
            return Ok(PageRun::empty());
        }

        // reuse an existing run when possible
        if let Some(run) = self.take_free_run(page_count) {
            self.zero_run(run)?;
            self.initialize_run_refcount(run)?;

            return Ok(run);
        }

        // otherwise carve a new single-segment run when it fits
        if page_count <= self.pages_per_segment()
            && let Some(run) = self.take_fresh_single_segment_run(page_count)?
        {
            self.initialize_run_refcount(run)?;

            return Ok(run);
        }

        // fall back to one fresh multi-segment run
        let run = self.allocate_fresh_multi_segment_run(page_count)?;
        self.initialize_run_refcount(run)?;

        Ok(run)
    }

    /// Retain one physical run.
    pub(super) fn retain_run(&self, run: PageRun) -> HeapResult<()> {
        if run.is_empty() {
            return Ok(());
        }

        let mut state = self.lock_state();
        let refcount = self.run_refcount_mut(&mut state, run)?;

        *refcount = refcount.saturating_add(1);

        Ok(())
    }

    /// Release one physical run.
    pub(super) fn release_run(&self, run: PageRun) -> HeapResult<()> {
        if run.is_empty() {
            return Ok(());
        }

        // stop once another root still retains the run
        let mut state = self.lock_state();
        let refcount = self.run_refcount_mut(&mut state, run)?;
        if *refcount > 1 {
            *refcount = refcount.saturating_sub(1);
            return Ok(());
        }

        // return the fully released run to the global free-run pool
        *refcount = 0;
        self.insert_free_run(&mut state, run);

        Ok(())
    }

    /// Report whether one physical run is shared.
    pub(super) fn run_is_shared(&self, run: PageRun) -> HeapResult<bool> {
        if run.is_empty() {
            return Ok(false);
        }

        let mut state = self.lock_state();

        let refcount = self.run_refcount_mut(&mut state, run)?;

        Ok(*refcount > 1)
    }

    /// Ensure the arena can address the given number of pages.
    pub(super) fn ensure_page_capacity(&self, page_count: usize) -> HeapResult<()> {
        let required_segments = page_count.div_ceil(self.pages_per_segment());
        if required_segments > MAX_ARENA_SEGMENTS {
            return Err(HeapError::ArenaSegmentDirectoryExhausted {
                required_segments,
                max_segments: MAX_ARENA_SEGMENTS,
            });
        }

        let current_segments = {
            let mut state = self.lock_state();
            let current_segments = state.segment_count;

            // reserve the required directory range before publishing segments
            if required_segments > state.segment_count {
                state.segment_count = required_segments;
                state.segments.resize(required_segments, null_mut());
            }

            current_segments
        };
        if required_segments <= current_segments {
            return Ok(());
        }

        // publish any newly required segments
        for segment_index in current_segments..required_segments {
            if self.publish_segment(segment_index)?.is_none() {
                return Err(HeapError::ArenaSegmentDirectoryExhausted {
                    required_segments,
                    max_segments: MAX_ARENA_SEGMENTS,
                });
            }
        }

        Ok(())
    }

    /// Zero one full run before reuse.
    fn zero_run(&self, run: PageRun) -> HeapResult<()> {
        // reset each page in the reused run
        for page_id in run.page_ids() {
            let Some(page) = self.page_bytes_mut(page_id) else {
                return Err(HeapError::CorruptMissingPage { page_id });
            };

            page.fill(0);
        }

        Ok(())
    }

    /// Return one free run large enough for the requested size.
    fn take_free_run(&self, page_count: usize) -> Option<PageRun> {
        let mut state = self.lock_state();
        let Some((run_len, run)) = state
            .free_runs
            .range_mut(page_count..)
            .find_map(|(&run_len, runs)| runs.pop().map(|run| (run_len, run)))
        else {
            return None;
        };

        // drop the selected extent from both free-run indexes
        state.free_runs_by_start.remove(&run.first_page.index());

        // drop empty buckets after one successful pop
        if state.free_runs.get(&run_len).is_some_and(Vec::is_empty) {
            state.free_runs.remove(&run_len);
        }

        let (allocation, remainder) = run.split_prefix(page_count)?;

        // return any remainder to the free-run indexes
        if !remainder.is_empty() {
            self.insert_free_run(&mut state, remainder);
        }

        Some(allocation)
    }

    /// Return one fresh run that fits inside one existing or new segment.
    fn take_fresh_single_segment_run(&self, page_count: usize) -> HeapResult<Option<PageRun>> {
        let segment_count = {
            let state = self.lock_state();
            state.segment_count
        };

        // first try already published segments
        for segment_index in 0..segment_count {
            if self.publish_segment(segment_index)?.is_none() {
                return Err(HeapError::ArenaSegmentDirectoryExhausted {
                    required_segments: segment_index.saturating_add(1),
                    max_segments: MAX_ARENA_SEGMENTS,
                });
            }
            let Some(run) = self.try_allocate_segment_run(segment_index, page_count)? else {
                continue;
            };
            return Ok(Some(run));
        }

        // otherwise publish one new segment and allocate from it
        let segment_index = self.reserve_segment_range(1)?;
        if self.publish_segment(segment_index)?.is_none() {
            return Err(HeapError::ArenaSegmentDirectoryExhausted {
                required_segments: segment_index.saturating_add(1),
                max_segments: MAX_ARENA_SEGMENTS,
            });
        }

        self.try_allocate_segment_run(segment_index, page_count)
    }

    /// Allocate one fresh run that may span multiple new segments.
    fn allocate_fresh_multi_segment_run(&self, page_count: usize) -> HeapResult<PageRun> {
        let required_segments = page_count.div_ceil(self.pages_per_segment());
        let first_segment_index = self.reserve_segment_range(required_segments)?;
        let last_segment_len = page_count % self.pages_per_segment();

        // mark each newly reserved segment run as consumed
        for segment_offset in 0..required_segments {
            let segment_index = first_segment_index.saturating_add(segment_offset);
            if self.publish_segment(segment_index)?.is_none() {
                return Err(HeapError::ArenaSegmentDirectoryExhausted {
                    required_segments: segment_index.saturating_add(1),
                    max_segments: MAX_ARENA_SEGMENTS,
                });
            }
            let used_pages = if segment_offset + 1 == required_segments && last_segment_len != 0 {
                last_segment_len
            } else {
                self.pages_per_segment()
            };

            let mut state = self.lock_state();
            let Some(segment) = self.segment_with_state(&mut state, segment_index) else {
                return Err(HeapError::ArenaSegmentDirectoryExhausted {
                    required_segments: segment_index.saturating_add(1),
                    max_segments: MAX_ARENA_SEGMENTS,
                });
            };
            segment.next_unused_page = used_pages as u32;
        }

        let first_page = PageId::new(first_segment_index.saturating_mul(self.pages_per_segment()))?;

        PageRun::new(first_page, page_count)
    }

    /// Return one newly reserved segment range start.
    fn reserve_segment_range(&self, segment_count: usize) -> HeapResult<usize> {
        let mut state = self.lock_state();
        let first_segment_index = state.segment_count;
        let end_segment_index = first_segment_index.saturating_add(segment_count);

        if end_segment_index > MAX_ARENA_SEGMENTS {
            return Err(HeapError::ArenaSegmentDirectoryExhausted {
                required_segments: end_segment_index,
                max_segments: MAX_ARENA_SEGMENTS,
            });
        }

        state.segment_count = end_segment_index;
        state.segments.resize(end_segment_index, null_mut());

        Ok(first_segment_index)
    }

    /// Publish one segment and return it.
    fn publish_segment(&self, segment_index: usize) -> HeapResult<Option<&Segment>> {
        let mut state = self.lock_state();
        if segment_index >= state.segment_count {
            return Ok(None);
        }

        let Some(slot) = state.segments.get_mut(segment_index) else {
            return Ok(None);
        };
        if slot.is_null() {
            let segment = Box::new(Segment::zeroed(
                self.segment_bytes(),
                self.pages_per_segment(),
                self.page_bytes(),
            )?);

            *slot = Box::into_raw(segment);
        }

        Ok(Some(unsafe { &**slot }))
    }

    /// Allocate one fresh run from one specific segment.
    fn try_allocate_segment_run(
        &self,
        segment_index: usize,
        page_count: usize,
    ) -> HeapResult<Option<PageRun>> {
        let mut state = self.lock_state();
        let Some(segment) = self.segment_with_state(&mut state, segment_index) else {
            return Ok(None);
        };
        let start_page = segment.next_unused_page as usize;
        let end_page = start_page.saturating_add(page_count);

        // stop once this segment is exhausted
        if end_page > self.pages_per_segment() {
            return Ok(None);
        }

        // consume the fresh segment range
        segment.next_unused_page = end_page as u32;

        let first_page = segment_index
            .saturating_mul(self.pages_per_segment())
            .saturating_add(start_page);

        let first_page = PageId::new(first_page)?;
        let run = PageRun::new(first_page, page_count)?;

        Ok(Some(run))
    }

    /// Return one mutable run refcount for one run start.
    fn run_refcount_mut<'a>(
        &self,
        state: &'a mut ArenaState,
        run: PageRun,
    ) -> HeapResult<&'a mut u32> {
        let (segment_index, segment_page_index) = self.page_position(run.first_page);
        let Some(segment) = self.segment_with_state(state, segment_index) else {
            return Err(HeapError::CorruptMissingRunRefcount {
                first_page: run.first_page,
            });
        };

        let Some(refcount) = segment.run_refcounts.get_mut(segment_page_index) else {
            return Err(HeapError::CorruptMissingRunRefcount {
                first_page: run.first_page,
            });
        };

        Ok(refcount)
    }

    /// Initialize one fresh run refcount before publishing it.
    fn initialize_run_refcount(&self, run: PageRun) -> HeapResult<()> {
        let mut state = self.lock_state();
        let refcount = self.run_refcount_mut(&mut state, run)?;

        *refcount = 1;

        Ok(())
    }

    /// Return one immutable page slice for one page id.
    pub(crate) fn page_bytes_from_id(&self, page_id: PageId) -> Option<&[u8]> {
        let (segment_index, segment_page_index) = self.page_position(page_id);
        let segment = self.segment(segment_index)?;

        Some(segment.page(segment_page_index, self.page_bytes()))
    }

    /// Return one mutable page slice for one page id.
    pub(crate) fn page_bytes_mut(&self, page_id: PageId) -> Option<&mut [u8]> {
        let (segment_index, segment_page_index) = self.page_position(page_id);
        let segment = self.segment(segment_index)?;

        Some(segment.page_mut(segment_page_index, self.page_bytes()))
    }

    /// Return one published arena segment by segment index.
    pub(super) fn segment(&self, segment_index: usize) -> Option<&Segment> {
        let state = self.lock_state();
        let segment_ptr = *state.segments.get(segment_index)?;
        if segment_ptr.is_null() {
            return None;
        }

        Some(unsafe { &*segment_ptr })
    }

    /// Return the segment and page index for one page id.
    pub(super) fn page_position(&self, page_id: PageId) -> (usize, usize) {
        let pages_per_segment = self.pages_per_segment();
        let page_index = page_id.index();
        let segment_index = page_index / pages_per_segment;
        let segment_page_index = page_index % pages_per_segment;

        (segment_index, segment_page_index)
    }

    /// Return the number of pages stored in each segment.
    pub(super) const fn pages_per_segment(&self) -> usize {
        self.pages_per_segment as usize
    }

    /// Insert one free run and coalesce it with adjacent free runs.
    fn insert_free_run(&self, state: &mut ArenaState, run: PageRun) {
        let mut run = run;

        // merge the immediate predecessor when it touches this run
        if let Some(previous_run) = self.find_previous_free_run(state, run) {
            if previous_run.is_immediately_before(run) {
                self.remove_free_run(state, previous_run);
                run = PageRun::from_raw_parts(
                    previous_run.first_page,
                    previous_run.page_count.saturating_add(run.page_count),
                );
            }
        }

        // merge the immediate successor when it touches this run
        if let Some(next_run) = self.find_next_free_run(state, run) {
            if run.is_immediately_before(next_run) {
                self.remove_free_run(state, next_run);
                run = PageRun::from_raw_parts(
                    run.first_page,
                    run.page_count.saturating_add(next_run.page_count),
                );
            }
        }

        // publish the merged run into both indexes
        state.free_runs.entry(run.len()).or_default().push(run);
        state.free_runs_by_start.insert(run.first_page.index(), run);
    }

    /// Remove one free run from both free-run indexes.
    fn remove_free_run(&self, state: &mut ArenaState, run: PageRun) {
        // drop the exact run from the size bucket
        if let Some(runs) = state.free_runs.get_mut(&run.len()) {
            if let Some(run_index) = runs.iter().position(|candidate| *candidate == run) {
                runs.swap_remove(run_index);
            }

            if runs.is_empty() {
                state.free_runs.remove(&run.len());
            }
        }

        // drop the exact run from the start index
        state.free_runs_by_start.remove(&run.first_page.index());
    }

    /// Return the immediately preceding free run when one exists.
    fn find_previous_free_run(&self, state: &ArenaState, run: PageRun) -> Option<PageRun> {
        state
            .free_runs_by_start
            .range(..run.start_page_index())
            .next_back()
            .map(|(_, run)| *run)
    }

    /// Return the immediately following free run when one exists.
    fn find_next_free_run(&self, state: &ArenaState, run: PageRun) -> Option<PageRun> {
        state
            .free_runs_by_start
            .range(run.end_page_index()..)
            .next()
            .map(|(_, run)| *run)
    }

    /// Raise one segment fresh-allocation cursor to the given high watermark.
    pub(super) fn raise_segment_high_watermark(
        &self,
        segment_index: usize,
        high_watermark: usize,
    ) -> HeapResult<()> {
        let mut state = self.lock_state();
        let Some(segment) = self.segment_with_state(&mut state, segment_index) else {
            return Err(HeapError::ArenaSegmentDirectoryExhausted {
                required_segments: segment_index.saturating_add(1),
                max_segments: MAX_ARENA_SEGMENTS,
            });
        };

        segment.next_unused_page = segment.next_unused_page.max(high_watermark as u32);

        Ok(())
    }

    /// Return the locked mutable arena state.
    fn lock_state(&self) -> MutexGuard<'_, ArenaState> {
        match self.state.lock() {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// Return one published segment through one locked arena state.
    fn segment_with_state<'a>(
        &self,
        state: &'a mut ArenaState,
        segment_index: usize,
    ) -> Option<&'a mut Segment> {
        let segment_ptr = *state.segments.get(segment_index)?;
        if segment_ptr.is_null() {
            return None;
        }

        Some(unsafe { &mut *segment_ptr })
    }
}

/// Allocate one zeroed page-aligned segment for arena page storage.
pub(crate) fn allocate_page_segment_bytes(
    byte_len: usize,
    page_bytes: usize,
) -> HeapResult<*mut u8> {
    let layout = page_segment_layout(byte_len, page_bytes)?;
    let data = unsafe { alloc_zeroed(layout) };

    if data.is_null() {
        return Err(HeapError::ArenaSegmentAllocationFailed {
            byte_len,
            page_bytes,
        });
    }

    Ok(data)
}

/// Free one page-aligned segment previously allocated by the arena.
pub(crate) fn free_page_segment_bytes(data: *mut u8, byte_len: usize, page_bytes: usize) {
    if data.is_null() {
        return;
    }

    let Ok(layout) = page_segment_layout(byte_len, page_bytes) else {
        return;
    };

    unsafe {
        dealloc(data, layout);
    }
}

/// Return one page-aligned allocation layout for one segment.
fn page_segment_layout(byte_len: usize, page_bytes: usize) -> HeapResult<Layout> {
    let byte_len = byte_len.max(1);

    Layout::from_size_align(byte_len, page_bytes).map_err(|_| {
        HeapError::InvalidArenaSegmentLayout {
            byte_len,
            page_bytes,
        }
    })
}
