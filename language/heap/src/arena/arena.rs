use std::alloc::{Layout, alloc_zeroed, dealloc};
use std::collections::BTreeMap;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

use parking_lot::Mutex;

use super::{PageId, PageRun, PageView, Segment};
use crate::{HeapError, HeapOptions, HeapResult};

/// The maximum number of arena segments in one heap arena.
const MAX_ARENA_SEGMENTS: usize = 1 << 16;
/// Sentinel for one missing fresh segment.
const MISSING_SEGMENT_INDEX: usize = usize::MAX;

/// One runtime-local branchable arena of fixed-width pages.
#[derive(Debug)]
pub struct Arena {
    /// The fixed page width for every page.
    page_bytes: u32,
    /// The fixed segment width for every arena segment.
    segment_bytes: u32,
    /// The number of pages stored in each arena segment.
    pages_per_segment: u32,
    /// The stable segment slots for this arena.
    segments: Box<[AtomicPtr<Segment>]>,
    /// The number of reserved segment slots.
    reserved_segment_count: AtomicUsize,
    /// The current fresh segment used for monotonic single-segment allocation.
    fresh_segment: AtomicUsize,
    /// The reusable free-run index.
    free_run_set: Mutex<FreeRunSet>,
}

// segment slots, fresh allocation, and run refcounts are synchronized internally
unsafe impl Send for Arena {}

// payload access is external, arena metadata is synchronized internally
unsafe impl Sync for Arena {}

/// One free-run index.
#[derive(Debug)]
struct FreeRunSet {
    /// The reusable free physical runs keyed by page count.
    by_len: BTreeMap<usize, Vec<PageRun>>,
    /// The reusable free physical runs keyed by first page index.
    by_start: BTreeMap<usize, PageRun>,
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        let reserved_segment_count = *self.reserved_segment_count.get_mut();

        for segment_index in 0..reserved_segment_count {
            let segment_ptr = self.segments[segment_index].load(Ordering::Relaxed);
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
        let options = HeapOptions::default();
        let page_bytes = options.page_bytes;
        let segment_bytes = options.arena_segment_bytes;

        Self {
            page_bytes: page_bytes as u32,
            segment_bytes: segment_bytes as u32,
            pages_per_segment: (segment_bytes / page_bytes) as u32,
            segments: std::iter::repeat_with(|| AtomicPtr::new(null_mut()))
                .take(MAX_ARENA_SEGMENTS)
                .collect(),
            reserved_segment_count: AtomicUsize::new(0),
            fresh_segment: AtomicUsize::new(MISSING_SEGMENT_INDEX),
            free_run_set: Mutex::new(FreeRunSet::new()),
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
            segments: std::iter::repeat_with(|| AtomicPtr::new(null_mut()))
                .take(MAX_ARENA_SEGMENTS)
                .collect(),
            reserved_segment_count: AtomicUsize::new(0),
            fresh_segment: AtomicUsize::new(MISSING_SEGMENT_INDEX),
            free_run_set: Mutex::new(FreeRunSet::new()),
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
    pub fn retain_page_view(&self, page_view: &PageView) -> HeapResult<()> {
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
    pub fn clone_page_view(&self, page_view: &PageView) -> HeapResult<PageView> {
        self.retain_page_view(page_view)?;

        Ok(*page_view)
    }

    /// Release one logical page view after one root drops it.
    pub fn release_page_view(&self, page_view: &PageView) -> HeapResult<()> {
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
        if let Some(run) = self.take_free_run(page_count)? {
            self.zero_run(run)?;
            self.initialize_run_refcount(run)?;

            return Ok(run);
        }

        // otherwise carve a new single-segment run when it fits
        let run = if page_count <= self.pages_per_segment()
            && let Some(run) = self.take_fresh_single_segment_run(page_count)?
        {
            run
        }
        // fall back to one fresh multi-segment run
        else {
            self.allocate_fresh_multi_segment_run(page_count)?
        };

        self.initialize_run_refcount(run)?;

        Ok(run)
    }

    /// Retain one physical run.
    pub(super) fn retain_run(&self, run: PageRun) -> HeapResult<()> {
        if run.is_empty() {
            return Ok(());
        }

        let refcount = self.run_refcount(run)?;

        loop {
            let current_refcount = refcount.load(Ordering::Acquire);
            let Some(next_refcount) = current_refcount.checked_add(1) else {
                return Err(HeapError::InvalidRunRefcount {
                    first_page: run.first_page,
                    refcount: current_refcount,
                });
            };

            if refcount
                .compare_exchange(
                    current_refcount,
                    next_refcount,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                return Ok(());
            }
        }
    }

    /// Release one physical run.
    pub(super) fn release_run(&self, run: PageRun) -> HeapResult<()> {
        if run.is_empty() {
            return Ok(());
        }

        let refcount = self.run_refcount(run)?;

        loop {
            let current_refcount = refcount.load(Ordering::Acquire);
            if current_refcount == 0 {
                return Err(HeapError::InvalidRunRefcount {
                    first_page: run.first_page,
                    refcount: current_refcount,
                });
            }

            let next_refcount = current_refcount - 1;
            if refcount
                .compare_exchange(
                    current_refcount,
                    next_refcount,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_err()
            {
                continue;
            }

            // return the fully released run to the global free-run pool
            if next_refcount == 0 {
                self.insert_free_run(run)?;
            }

            return Ok(());
        }
    }

    /// Report whether one physical run is uniquely owned.
    pub(super) fn run_is_unique(&self, run: PageRun) -> HeapResult<bool> {
        if run.is_empty() {
            return Ok(true);
        }

        let refcount = self.run_refcount(run)?;

        Ok(refcount.load(Ordering::Acquire) == 1)
    }

    /// Ensure the arena can address the given number of pages.
    pub(super) fn ensure_page_capacity(&self, page_count: usize) -> HeapResult<()> {
        let required_segments = page_count.div_ceil(self.pages_per_segment());
        if required_segments > MAX_ARENA_SEGMENTS {
            return Err(HeapError::ArenaSegmentLimitExceeded {
                required_segments,
                max_segments: MAX_ARENA_SEGMENTS,
            });
        }

        // reserve the required directory range before materializing segments
        self.reserve_segment_count(required_segments)?;

        // materialize any newly required segments
        for segment_index in 0..required_segments {
            self.ensure_segment(segment_index)?;
        }

        Ok(())
    }

    /// Zero one full run before reuse.
    pub(super) fn zero_run(&self, run: PageRun) -> HeapResult<()> {
        // reset each page in the reused run
        for page_id in run.page_ids() {
            let page_ptr = self.page_slice_mut_ptr(page_id)?;
            let page = unsafe { &mut *page_ptr };

            page.fill(0);
        }

        Ok(())
    }

    /// Return one free run large enough for the requested size.
    fn take_free_run(&self, page_count: usize) -> HeapResult<Option<PageRun>> {
        let mut free_run_set = self.free_run_set.lock();
        let Some((run_len, run)) = free_run_set
            .by_len
            .range_mut(page_count..)
            .find_map(|(&run_len, runs)| runs.pop().map(|run| (run_len, run)))
        else {
            return Ok(None);
        };

        // drop the selected extent from both free-run indexes
        free_run_set.by_start.remove(&run.first_page.index());

        // drop empty buckets after one successful pop
        if free_run_set.by_len.get(&run_len).is_some_and(Vec::is_empty) {
            free_run_set.by_len.remove(&run_len);
        }

        let (allocation, remainder) =
            run.split_prefix(page_count)
                .ok_or(HeapError::InvalidPageRun {
                    first_page: run.first_page,
                    page_count,
                })?;

        // return any remainder to the free-run indexes
        if !remainder.is_empty() {
            self.insert_free_run_locked(&mut free_run_set, remainder)?;
        }

        Ok(Some(allocation))
    }

    /// Return one fresh run that fits inside one existing or new segment.
    fn take_fresh_single_segment_run(&self, page_count: usize) -> HeapResult<Option<PageRun>> {
        loop {
            // fast path: claim from the current fresh segment directly
            let segment_index = self.fresh_segment.load(Ordering::Acquire);
            if segment_index != MISSING_SEGMENT_INDEX
                && let Some(run) = self.try_allocate_segment_run(segment_index, page_count)?
            {
                return Ok(Some(run));
            }

            // slow path: pick or reserve the next fresh segment
            let segment_index = self.ensure_fresh_segment(page_count)?;

            if let Some(run) = self.try_allocate_segment_run(segment_index, page_count)? {
                return Ok(Some(run));
            }
        }
    }

    /// Allocate one fresh run that may span multiple new segments.
    fn allocate_fresh_multi_segment_run(&self, page_count: usize) -> HeapResult<PageRun> {
        let required_segments = page_count.div_ceil(self.pages_per_segment());
        let first_segment_index = self.reserve_segment_range(required_segments)?;
        let last_segment_len = page_count % self.pages_per_segment();

        // mark each newly reserved segment run as consumed
        for segment_offset in 0..required_segments {
            let segment_index = first_segment_index.checked_add(segment_offset).ok_or(
                HeapError::InvariantOverflow {
                    context: "arena segment range index",
                },
            )?;
            let used_pages = if segment_offset + 1 == required_segments && last_segment_len != 0 {
                last_segment_len
            } else {
                self.pages_per_segment()
            };

            self.ensure_segment(segment_index)?;

            let Some(segment) = self.segment(segment_index) else {
                let required_segments =
                    segment_index
                        .checked_add(1)
                        .ok_or(HeapError::InvariantOverflow {
                            context: "arena segment count",
                        })?;
                return Err(HeapError::ArenaSegmentLimitExceeded {
                    required_segments,
                    max_segments: MAX_ARENA_SEGMENTS,
                });
            };

            segment
                .next_unused_page
                .store(used_pages as u32, Ordering::Release);
        }

        let fresh_segment = if last_segment_len != 0 {
            first_segment_index + required_segments - 1
        } else {
            MISSING_SEGMENT_INDEX
        };
        self.fresh_segment.store(fresh_segment, Ordering::Release);

        let first_page_index = first_segment_index
            .checked_mul(self.pages_per_segment())
            .ok_or(HeapError::InvalidPageId {
                index: first_segment_index,
            })?;
        let first_page = PageId::new(first_page_index)?;

        PageRun::new(first_page, page_count)
    }

    /// Return one newly reserved segment range start.
    fn reserve_segment_range(&self, segment_count: usize) -> HeapResult<usize> {
        loop {
            let first_segment_index = self.reserved_segment_count.load(Ordering::Acquire);
            let end_segment_index = first_segment_index.checked_add(segment_count).ok_or(
                HeapError::InvariantOverflow {
                    context: "arena reserved segment count",
                },
            )?;

            if end_segment_index > MAX_ARENA_SEGMENTS {
                return Err(HeapError::ArenaSegmentLimitExceeded {
                    required_segments: end_segment_index,
                    max_segments: MAX_ARENA_SEGMENTS,
                });
            }

            if self
                .reserved_segment_count
                .compare_exchange(
                    first_segment_index,
                    end_segment_index,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                return Ok(first_segment_index);
            }
        }
    }

    /// Reserve at least the given segment count.
    fn reserve_segment_count(&self, segment_count: usize) -> HeapResult<()> {
        loop {
            let current_count = self.reserved_segment_count.load(Ordering::Acquire);
            if current_count >= segment_count {
                return Ok(());
            }

            if segment_count > MAX_ARENA_SEGMENTS {
                return Err(HeapError::ArenaSegmentLimitExceeded {
                    required_segments: segment_count,
                    max_segments: MAX_ARENA_SEGMENTS,
                });
            }

            if self
                .reserved_segment_count
                .compare_exchange(
                    current_count,
                    segment_count,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                return Ok(());
            }
        }
    }

    /// Ensure one reserved segment exists.
    fn ensure_segment(&self, segment_index: usize) -> HeapResult<()> {
        let reserved_segment_count = self.reserved_segment_count.load(Ordering::Acquire);
        if segment_index >= reserved_segment_count {
            let required_segments =
                segment_index
                    .checked_add(1)
                    .ok_or(HeapError::InvariantOverflow {
                        context: "arena segment count",
                    })?;
            return Err(HeapError::ArenaSegmentLimitExceeded {
                required_segments,
                max_segments: MAX_ARENA_SEGMENTS,
            });
        }

        let slot = &self.segments[segment_index];
        if slot.load(Ordering::Acquire).is_null() {
            let segment = Box::new(Segment::zeroed(
                self.segment_bytes(),
                self.pages_per_segment(),
                self.page_bytes(),
            )?);
            let segment_ptr = Box::into_raw(segment);

            // publish one newly materialized segment once
            if let Err(existing_ptr) =
                slot.compare_exchange(null_mut(), segment_ptr, Ordering::AcqRel, Ordering::Acquire)
            {
                unsafe {
                    drop(Box::from_raw(segment_ptr));
                }

                debug_assert!(!existing_ptr.is_null());
            }
        }

        Ok(())
    }

    /// Return one usable fresh segment, reserving one when necessary.
    fn ensure_fresh_segment(&self, page_count: usize) -> HeapResult<usize> {
        let current_segment = self.fresh_segment.load(Ordering::Acquire);
        let reserved_segment_count = self.reserved_segment_count.load(Ordering::Acquire);
        if current_segment != MISSING_SEGMENT_INDEX
            && current_segment < reserved_segment_count
            && self.segment_has_capacity(current_segment, page_count)?
        {
            return Ok(current_segment);
        }

        let segment_index = self.reserve_segment_range(1)?;
        self.ensure_segment(segment_index)?;
        self.fresh_segment.store(segment_index, Ordering::Release);

        Ok(segment_index)
    }

    /// Allocate one fresh run from one specific segment.
    fn try_allocate_segment_run(
        &self,
        segment_index: usize,
        page_count: usize,
    ) -> HeapResult<Option<PageRun>> {
        let Some(segment) = self.segment(segment_index) else {
            return Ok(None);
        };

        let page_count = u32::try_from(page_count)
            .map_err(|_| HeapError::InvalidPageId { index: page_count })?;

        let start_page = loop {
            let start_page = segment.next_unused_page.load(Ordering::Acquire);
            let Some(end_page) = start_page.checked_add(page_count) else {
                return Err(HeapError::InvalidPageId {
                    index: start_page as usize,
                });
            };

            // stop once this segment is exhausted
            if end_page as usize > self.pages_per_segment() {
                return Ok(None);
            }

            if segment
                .next_unused_page
                .compare_exchange(start_page, end_page, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                break start_page as usize;
            }
        };

        let first_page = segment_index
            .checked_mul(self.pages_per_segment())
            .and_then(|first_page| first_page.checked_add(start_page))
            .ok_or(HeapError::InvalidPageId {
                index: segment_index,
            })?;

        let first_page = PageId::new(first_page)?;
        let run = PageRun::new(first_page, page_count as usize)?;

        Ok(Some(run))
    }

    /// Report whether one segment still has capacity for one fresh run.
    fn segment_has_capacity(&self, segment_index: usize, page_count: usize) -> HeapResult<bool> {
        let Some(segment) = self.segment(segment_index) else {
            return Ok(false);
        };
        let start_page = segment.next_unused_page.load(Ordering::Acquire) as usize;
        let Some(end_page) = start_page.checked_add(page_count) else {
            return Err(HeapError::InvalidPageId { index: start_page });
        };

        Ok(end_page <= self.pages_per_segment())
    }

    /// Return one physical run refcount.
    fn run_refcount(&self, run: PageRun) -> HeapResult<&std::sync::atomic::AtomicU32> {
        let (segment_index, segment_page_index) = self.page_position(run.first_page);
        let Some(segment) = self.segment(segment_index) else {
            return Err(HeapError::MissingRunRefcount {
                first_page: run.first_page,
            });
        };

        let Some(refcount) = segment.run_refcounts.get(segment_page_index) else {
            return Err(HeapError::MissingRunRefcount {
                first_page: run.first_page,
            });
        };

        Ok(refcount)
    }

    /// Initialize one fresh run refcount before exposing it.
    fn initialize_run_refcount(&self, run: PageRun) -> HeapResult<()> {
        let refcount = self.run_refcount(run)?;

        refcount.store(1, Ordering::Release);

        Ok(())
    }

    /// Return one immutable physical page slice.
    pub(crate) fn page_slice(&self, page_id: PageId) -> HeapResult<&[u8]> {
        let (segment_index, segment_page_index) = self.page_position(page_id);
        let Some(segment) = self.segment(segment_index) else {
            return Err(HeapError::MissingPage { page_id });
        };

        Ok(segment.page(segment_page_index, self.page_bytes()))
    }

    /// Return one mutable physical page slice pointer.
    pub(crate) fn page_slice_mut_ptr(&self, page_id: PageId) -> HeapResult<*mut [u8]> {
        let (segment_index, segment_page_index) = self.page_position(page_id);
        let Some(segment) = self.segment(segment_index) else {
            return Err(HeapError::MissingPage { page_id });
        };

        Ok(segment.page_mut_ptr(segment_page_index, self.page_bytes()))
    }

    /// Return one physical page as owned bytes.
    pub(crate) fn read_page_bytes(&self, page_id: PageId) -> HeapResult<Box<[u8]>> {
        let page = self.page_slice(page_id)?;

        Ok(page.to_vec().into_boxed_slice())
    }

    /// Report whether one reserved segment already exists.
    pub(super) fn has_segment(&self, segment_index: usize) -> bool {
        self.segments
            .get(segment_index)
            .is_some_and(|segment| !segment.load(Ordering::Acquire).is_null())
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
    pub(crate) const fn pages_per_segment(&self) -> usize {
        self.pages_per_segment as usize
    }

    /// Insert one free run and coalesce it with adjacent free runs.
    fn insert_free_run(&self, run: PageRun) -> HeapResult<()> {
        let mut free_run_set = self.free_run_set.lock();

        self.insert_free_run_locked(&mut free_run_set, run)
    }

    /// Release one cached run back into the arena free-run pool.
    pub(super) fn recycle_cached_run(&self, run: PageRun) {
        if run.is_empty() {
            return;
        }

        let refcount = match self.run_refcount(run) {
            Ok(refcount) => refcount,
            Err(error) => panic!("cached run lost arena refcount: {error}"),
        };

        let current_refcount = refcount.swap(0, Ordering::AcqRel);
        if current_refcount != 1 {
            panic!(
                "cached run had invalid refcount {} at page {}",
                current_refcount,
                run.first_page.index()
            );
        }

        if let Err(error) = self.insert_free_run(run) {
            panic!("cached run could not reenter free-run index: {error}");
        }
    }

    /// Insert one free run while holding the arena state lock.
    fn insert_free_run_locked(
        &self,
        free_run_set: &mut FreeRunSet,
        run: PageRun,
    ) -> HeapResult<()> {
        let mut run = run;

        // merge the immediate predecessor when it touches this run
        if let Some(previous_run) = self.find_previous_free_run(free_run_set, run)
            && previous_run.is_immediately_before(run)
        {
            self.remove_free_run(free_run_set, previous_run);
            run = PageRun::new(
                previous_run.first_page,
                previous_run
                    .len()
                    .checked_add(run.len())
                    .ok_or(HeapError::InvariantOverflow {
                        context: "arena free-run merge length",
                    })?,
            )?;
        }

        // merge the immediate successor when it touches this run
        if let Some(next_run) = self.find_next_free_run(free_run_set, run)
            && run.is_immediately_before(next_run)
        {
            self.remove_free_run(free_run_set, next_run);
            run = PageRun::new(
                run.first_page,
                run.len()
                    .checked_add(next_run.len())
                    .ok_or(HeapError::InvariantOverflow {
                        context: "arena free-run merge length",
                    })?,
            )?;
        }

        // publish the merged run into both indexes
        free_run_set.by_len.entry(run.len()).or_default().push(run);
        free_run_set.by_start.insert(run.first_page.index(), run);

        Ok(())
    }

    /// Remove one free run from both free-run indexes.
    fn remove_free_run(&self, free_run_set: &mut FreeRunSet, run: PageRun) {
        // drop the exact run from the size bucket
        if let Some(runs) = free_run_set.by_len.get_mut(&run.len()) {
            if let Some(run_index) = runs.iter().position(|candidate| *candidate == run) {
                runs.swap_remove(run_index);
            }

            if runs.is_empty() {
                free_run_set.by_len.remove(&run.len());
            }
        }

        // drop the exact run from the start index
        free_run_set.by_start.remove(&run.first_page.index());
    }

    /// Return the immediately preceding free run when one exists.
    fn find_previous_free_run(&self, free_run_set: &FreeRunSet, run: PageRun) -> Option<PageRun> {
        free_run_set
            .by_start
            .range(..run.start_page_index())
            .next_back()
            .map(|(_, run)| *run)
    }

    /// Return the immediately following free run when one exists.
    fn find_next_free_run(&self, free_run_set: &FreeRunSet, run: PageRun) -> Option<PageRun> {
        free_run_set
            .by_start
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
        let Some(segment) = self.segment(segment_index) else {
            let required_segments =
                segment_index
                    .checked_add(1)
                    .ok_or(HeapError::InvariantOverflow {
                        context: "arena segment count",
                    })?;
            return Err(HeapError::ArenaSegmentLimitExceeded {
                required_segments,
                max_segments: MAX_ARENA_SEGMENTS,
            });
        };

        segment
            .next_unused_page
            .fetch_max(high_watermark as u32, Ordering::AcqRel);

        Ok(())
    }

    /// Return one existing segment through the stable segment slots.
    fn segment(&self, segment_index: usize) -> Option<&Segment> {
        let segment_ptr = self.segments.get(segment_index)?.load(Ordering::Acquire);
        if segment_ptr.is_null() {
            return None;
        }

        Some(unsafe { &*segment_ptr })
    }
}

impl FreeRunSet {
    /// Create one empty free-run index.
    fn new() -> Self {
        Self {
            by_len: BTreeMap::new(),
            by_start: BTreeMap::new(),
        }
    }
}

/// Allocate one zeroed page-aligned segment for arena page storage.
pub(crate) fn allocate_page_segment_bytes(
    byte_len: usize,
    page_bytes: usize,
) -> HeapResult<*mut u8> {
    let layout = page_segment_layout(byte_len, page_bytes)?;

    // actually allocate
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

    // actually deallocate
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
