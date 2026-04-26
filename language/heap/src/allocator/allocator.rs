use std::slice;
use std::sync::atomic::Ordering;

use parking_lot::Mutex;

use super::arena::{
    Arena, ArenaIndex, ArenaLocation, ArenaReservation, max_arena_count, max_arena_frame_count,
};
use super::{
    DEFAULT_ALLOCATOR_ARENA_BYTES, DEFAULT_PAGE_BYTES, PageId, PageRun, PageRunSet, PageView,
};
use crate::{HeapError, HeapOptions, HeapResult};

/// The arena frontier, current arena, free runs, and arena ownership.
#[derive(Debug)]
#[allow(clippy::vec_box)]
struct AllocatorState {
    /// The number of arenas available to the page allocator.
    arena_count: usize,
    /// The current arena used for monotonic single-arena allocation.
    current_arena_index: Option<usize>,
    /// The free physical runs.
    free_runs: PageRunSet,
    /// The owned arena records, boxed so address-map pointers stay stable.
    owned_arenas: Vec<Box<Arena>>,
    /// The owned virtual memory ranges backing arenas.
    arena_reservations: Vec<ArenaReservation>,
}

/// One branchable allocator of fixed-size pages.
#[derive(Debug)]
pub struct Allocator {
    /// The fixed page size for every page.
    page_bytes: u32,
    /// The fixed arena size for every allocator arena.
    arena_bytes: u32,
    /// The number of pages stored in each allocator arena.
    pages_per_arena: u32,
    /// The maximum addressable arena count.
    max_arena_count: u32,
    /// The arenas keyed by logical index and address frame.
    arena_index: ArenaIndex,
    /// The arena frontier, current arena, and free-run index.
    state: Mutex<AllocatorState>,
}

// allocator metadata is synchronized internally
unsafe impl Send for Allocator {}

// payload access is external, allocator metadata is synchronized internally
unsafe impl Sync for Allocator {}

impl Drop for Allocator {
    fn drop(&mut self) {
        let state = self.state.get_mut();

        state.owned_arenas.clear();
        state.arena_reservations.clear();
    }
}

impl Allocator {
    /// Create one allocator with the default page and arena sizes.
    pub fn try_default() -> HeapResult<Self> {
        Self::try_new(DEFAULT_PAGE_BYTES, DEFAULT_ALLOCATOR_ARENA_BYTES)
    }

    /// Create one empty allocator with the given page and arena sizes.
    pub fn try_new(page_bytes: usize, arena_bytes: usize) -> HeapResult<Self> {
        let page_bytes = HeapOptions::validate_page_bytes(page_bytes)?;
        let arena_bytes = HeapOptions::validate_allocator_arena_bytes(page_bytes, arena_bytes)?;
        let pages_per_arena = arena_bytes / page_bytes;
        let max_arena_count = max_arena_count(page_bytes, arena_bytes);
        let max_arena_frames = max_arena_frame_count(arena_bytes)?;

        Ok(Self {
            page_bytes: page_bytes as u32,
            arena_bytes: arena_bytes as u32,
            pages_per_arena: pages_per_arena as u32,
            max_arena_count: max_arena_count as u32,
            arena_index: ArenaIndex::new(max_arena_count, max_arena_frames),
            state: Mutex::new(AllocatorState {
                arena_count: 0,
                current_arena_index: None,
                free_runs: PageRunSet::new(pages_per_arena),
                owned_arenas: Vec::new(),
                arena_reservations: Vec::new(),
            }),
        })
    }

    /// Return the fixed page size.
    pub const fn page_bytes(&self) -> usize {
        self.page_bytes as usize
    }

    /// Return the fixed arena size.
    pub const fn arena_bytes(&self) -> usize {
        self.arena_bytes as usize
    }

    /// Return the maximum addressable arena count.
    fn max_arena_count(&self) -> usize {
        self.max_arena_count as usize
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
            self.increment_run_refcount(page_view.base_run())?;
        }

        // retain each patched replacement run
        for patch in page_view.patches() {
            self.increment_run_refcount(PageRun::single_page(patch.page_id))?;
        }

        Ok(())
    }

    /// Return one cloned page view retained for another live root.
    pub fn clone_page_view(&self, page_view: &PageView) -> HeapResult<PageView> {
        self.retain_page_view(page_view)?;

        Ok(page_view.clone())
    }

    /// Retain every logical page view for another live root.
    pub fn retain_page_views<I>(&self, page_views: I) -> HeapResult<Vec<PageView>>
    where
        I: IntoIterator<Item = PageView>,
    {
        let mut retained = Vec::new();
        for page_view in page_views {
            if let Err(error) = self.retain_page_view(&page_view) {
                self.release_page_views(&retained)?;
                return Err(error);
            }

            retained.push(page_view);
        }
        Ok(retained)
    }

    /// Release one logical page view after one root drops it.
    pub fn release_page_view(&self, page_view: &PageView) -> HeapResult<()> {
        // release the base run when it still contributes visible pages
        if page_view.has_base_pages() {
            self.decrement_run_refcount(page_view.base_run())?;
        }

        // release each patched replacement run
        for patch in page_view.patches() {
            self.decrement_run_refcount(PageRun::single_page(patch.page_id))?;
        }

        Ok(())
    }

    /// Release every retained logical page view after one failed rebuild.
    pub fn release_page_views(&self, page_views: &[PageView]) -> HeapResult<()> {
        for page_view in page_views.iter().rev() {
            self.release_page_view(page_view)?;
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
        if let Some(run) = self.allocate_free_run(page_count) {
            self.zero_run(run)?;
            self.initialize_run_refcount(run)?;

            return Ok(run);
        }

        // otherwise carve from the current arena when it fits
        let run = if page_count <= self.pages_per_arena()
            && let Some(run) = self.allocate_run_from_current_arena(page_count)?
        {
            run
        }
        // fall back to one run that crosses arenas
        else {
            self.allocate_multi_arena_run(page_count)?
        };

        self.initialize_run_refcount(run)?;

        Ok(run)
    }

    /// Increment one physical run refcount.
    pub(super) fn increment_run_refcount(&self, run: PageRun) -> HeapResult<()> {
        if run.is_empty() {
            return Ok(());
        }

        let refcount = self.run_refcount(run)?;

        loop {
            let current_refcount = refcount.load(Ordering::Acquire);
            let Some(next_refcount) = current_refcount.checked_add(1) else {
                return Err(HeapError::InvariantViolation {
                    context: "allocator run refcount overflow",
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

    /// Decrement one physical run refcount.
    pub(super) fn decrement_run_refcount(&self, run: PageRun) -> HeapResult<()> {
        if run.is_empty() {
            return Ok(());
        }

        let refcount = self.run_refcount(run)?;

        loop {
            let current_refcount = refcount.load(Ordering::Acquire);
            if current_refcount == 0 {
                return Err(HeapError::InvariantViolation {
                    context: "allocator released free run",
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

            // free the fully released run
            if next_refcount == 0 {
                self.free_run(run);
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

    /// Grow this allocator until it can address the given page count.
    pub(super) fn grow_to_page_count(&self, page_count: usize) -> HeapResult<()> {
        let required_arena_count = page_count.div_ceil(self.pages_per_arena());
        let max_arena_count = self.max_arena_count();
        if required_arena_count > max_arena_count {
            return Err(HeapError::AllocatorArenaLimitExceeded {
                required_arenas: required_arena_count,
                max_arenas: max_arena_count,
            });
        }

        let mut state = self.state.lock();
        let first_new_arena = state.arena_count;
        let end_arena_index = state.arena_count.max(required_arena_count);

        // publish arenas before moving the frontier
        self.allocate_arena_range(&mut state, first_new_arena, end_arena_index)?;
        state.arena_count = end_arena_index;

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

    /// Allocate one free run large enough for the requested size.
    fn allocate_free_run(&self, page_count: usize) -> Option<PageRun> {
        let mut state = self.state.lock();
        state.free_runs.allocate(page_count)
    }

    /// Allocate one run from the current arena.
    fn allocate_run_from_current_arena(&self, page_count: usize) -> HeapResult<Option<PageRun>> {
        loop {
            // claim from the current arena
            let current_arena_index = self.state.lock().current_arena_index;
            if let Some(arena_index) = current_arena_index
                && let Some(run) = self.allocate_run_from_arena(arena_index, page_count)?
            {
                return Ok(Some(run));
            }

            // grow into the next arena
            let arena_index = self.current_arena_for(page_count)?;
            if let Some(run) = self.allocate_run_from_arena(arena_index, page_count)? {
                return Ok(Some(run));
            }
        }
    }

    /// Allocate one run that crosses newly grown arenas.
    fn allocate_multi_arena_run(&self, page_count: usize) -> HeapResult<PageRun> {
        let required_arena_count = page_count.div_ceil(self.pages_per_arena());
        let first_arena_index = self.grow_arena_range(required_arena_count)?;
        let last_arena_len = page_count % self.pages_per_arena();

        // mark each newly grown arena run as consumed
        for arena_offset in 0..required_arena_count {
            let arena_index = first_arena_index.checked_add(arena_offset).ok_or(
                HeapError::InvariantOverflow {
                    context: "allocator arena range index",
                },
            )?;
            let used_pages = if arena_offset + 1 == required_arena_count && last_arena_len != 0 {
                last_arena_len
            } else {
                self.pages_per_arena()
            };

            let Some(arena) = self.arena(arena_index) else {
                return Err(HeapError::AllocatorArenaLimitExceeded {
                    required_arenas: arena_index + 1,
                    max_arenas: self.max_arena_count(),
                });
            };

            arena.raise_watermark(used_pages);
        }

        let current_arena_index =
            (last_arena_len != 0).then_some(first_arena_index + required_arena_count - 1);
        self.state.lock().current_arena_index = current_arena_index;

        let first_page_index = first_arena_index
            .checked_mul(self.pages_per_arena())
            .ok_or(HeapError::InvalidPageId {
                index: first_arena_index,
            })?;
        let first_page = PageId::new(first_page_index)?;

        PageRun::new(first_page, page_count)
    }

    /// Grow the allocator by one contiguous arena range.
    fn grow_arena_range(&self, arena_count: usize) -> HeapResult<usize> {
        let mut state = self.state.lock();
        let first_arena_index = state.arena_count;
        let end_arena_index =
            first_arena_index
                .checked_add(arena_count)
                .ok_or(HeapError::InvariantOverflow {
                    context: "allocator arena count",
                })?;

        let max_arena_count = self.max_arena_count();
        if end_arena_index > max_arena_count {
            return Err(HeapError::AllocatorArenaLimitExceeded {
                required_arenas: end_arena_index,
                max_arenas: max_arena_count,
            });
        }

        // publish arenas before moving the frontier
        self.allocate_arena_range(&mut state, first_arena_index, end_arena_index)?;
        state.arena_count = end_arena_index;

        Ok(first_arena_index)
    }

    /// Return one usable current arena, growing one when necessary.
    fn current_arena_for(&self, page_count: usize) -> HeapResult<usize> {
        let state = self.state.lock();
        let current_arena_index = state.current_arena_index;
        let arena_count = state.arena_count;
        drop(state);

        if let Some(arena_index) = current_arena_index
            && arena_index < arena_count
            && self.arena_has_capacity(arena_index, page_count)?
        {
            return Ok(arena_index);
        }

        let arena_index = self.grow_arena_range(1)?;
        self.state.lock().current_arena_index = Some(arena_index);

        Ok(arena_index)
    }

    /// Allocate one run from one specific arena.
    fn allocate_run_from_arena(
        &self,
        arena_index: usize,
        page_count: usize,
    ) -> HeapResult<Option<PageRun>> {
        let Some(arena) = self.arena(arena_index) else {
            return Ok(None);
        };

        arena.allocate_run(arena_index, page_count, self.pages_per_arena())
    }

    /// Report whether one arena still has capacity for one run.
    fn arena_has_capacity(&self, arena_index: usize, page_count: usize) -> HeapResult<bool> {
        let Some(arena) = self.arena(arena_index) else {
            return Ok(false);
        };

        arena.has_capacity(page_count, self.pages_per_arena())
    }

    /// Return one physical run refcount.
    fn run_refcount(&self, run: PageRun) -> HeapResult<&std::sync::atomic::AtomicU32> {
        let (arena_index, arena_page_index) = self.page_position(run.first_page);
        let Some(arena) = self.arena(arena_index) else {
            return Err(HeapError::MissingPage {
                page_id: run.first_page,
            });
        };

        Ok(arena.run_refcount(arena_page_index))
    }

    /// Initialize one run refcount before exposing it.
    fn initialize_run_refcount(&self, run: PageRun) -> HeapResult<()> {
        let refcount = self.run_refcount(run)?;
        refcount.store(1, Ordering::Release);
        Ok(())
    }

    /// Return one immutable physical page slice.
    pub(crate) fn page_slice(&self, page_id: PageId) -> HeapResult<&[u8]> {
        let (arena_index, arena_page_index) = self.page_position(page_id);
        let Some(arena) = self.arena(arena_index) else {
            return Err(HeapError::MissingPage { page_id });
        };
        let data = arena
            .page_ptr(arena_page_index, self.page_bytes())
            .ok_or(HeapError::MissingPage { page_id })?;

        Ok(unsafe { slice::from_raw_parts(data, self.page_bytes()) })
    }

    /// Return one mutable physical page slice pointer.
    pub(crate) fn page_slice_mut_ptr(&self, page_id: PageId) -> HeapResult<*mut [u8]> {
        let (arena_index, arena_page_index) = self.page_position(page_id);
        let Some(arena) = self.arena(arena_index) else {
            return Err(HeapError::MissingPage { page_id });
        };
        let data = arena
            .page_ptr(arena_page_index, self.page_bytes())
            .ok_or(HeapError::MissingPage { page_id })?;

        Ok(std::ptr::slice_from_raw_parts_mut(data, self.page_bytes()))
    }

    /// Return one physical page as owned bytes.
    pub(crate) fn read_page_bytes(&self, page_id: PageId) -> HeapResult<Box<[u8]>> {
        let page = self.page_slice(page_id)?;

        Ok(page.to_vec().into_boxed_slice())
    }

    /// Report whether one arena index is addressable.
    pub(super) fn has_arena(&self, arena_index: usize) -> bool {
        arena_index < self.max_arena_count()
    }

    /// Return the arena and page index for one page id.
    pub(super) fn page_position(&self, page_id: PageId) -> (usize, usize) {
        let pages_per_arena = self.pages_per_arena();
        let page_index = page_id.index();
        let arena_index = page_index / pages_per_arena;
        let arena_page_index = page_index % pages_per_arena;

        (arena_index, arena_page_index)
    }

    /// Return the owning arena and arena-local offset for one raw address.
    pub(crate) fn arena_location(&self, address: usize) -> Option<ArenaLocation> {
        if address == 0 {
            return None;
        }

        let arena = self
            .arena_index
            .arena_for_address(address, self.arena_bytes())?;
        let arena_base = arena.base() as usize;
        let arena_offset = address.checked_sub(arena_base)?;
        if arena_offset >= self.arena_bytes() {
            return None;
        }

        Some(ArenaLocation {
            arena_index: arena.index(),
            arena_offset,
        })
    }

    /// Return the number of pages stored in each arena.
    pub(crate) const fn pages_per_arena(&self) -> usize {
        self.pages_per_arena as usize
    }

    /// Free one dead run into the allocator free-run index.
    fn free_run(&self, run: PageRun) {
        let mut state = self.state.lock();
        state.free_runs.free(run);
    }

    /// Free one cache-owned run back into the allocator free-run index.
    pub(super) fn free_cached_run(&self, run: PageRun) -> HeapResult<()> {
        if run.is_empty() {
            return Ok(());
        }

        let refcount = self.run_refcount(run)?;
        let current_refcount = refcount.swap(0, Ordering::AcqRel);
        if current_refcount != 1 {
            return Err(HeapError::InvariantViolation {
                context: "allocator cached run refcount",
            });
        }

        self.free_run(run);

        Ok(())
    }

    /// Raise one arena allocation watermark to the given page index.
    pub(super) fn raise_arena_high_watermark(
        &self,
        arena_index: usize,
        high_watermark: usize,
    ) -> HeapResult<()> {
        let Some(arena) = self.arena(arena_index) else {
            let required_arenas =
                arena_index
                    .checked_add(1)
                    .ok_or(HeapError::InvariantOverflow {
                        context: "allocator arena count",
                    })?;
            return Err(HeapError::AllocatorArenaLimitExceeded {
                required_arenas,
                max_arenas: self.max_arena_count(),
            });
        };

        arena.raise_watermark(high_watermark);

        let mut state = self.state.lock();
        state.arena_count = state.arena_count.max(arena_index + 1);

        if high_watermark < self.pages_per_arena() && arena_index + 1 == state.arena_count {
            state.current_arena_index = Some(arena_index);
        }

        Ok(())
    }

    /// Return one existing arena by logical arena index.
    fn arena(&self, arena_index: usize) -> Option<&Arena> {
        self.arena_index.arena(arena_index)
    }

    /// Allocate and publish one arena range.
    fn allocate_arena_range(
        &self,
        state: &mut AllocatorState,
        first_arena_index: usize,
        end_arena_index: usize,
    ) -> HeapResult<()> {
        for arena_index in first_arena_index..end_arena_index {
            if self.arena(arena_index).is_some() {
                continue;
            }

            let required_arena_count = end_arena_index - arena_index;
            self.allocate_arena(state, arena_index, required_arena_count)?;
        }

        Ok(())
    }

    /// Allocate and publish one arena.
    fn allocate_arena(
        &self,
        state: &mut AllocatorState,
        arena_index: usize,
        required_arena_count: usize,
    ) -> HeapResult<()> {
        if arena_index >= self.max_arena_count() {
            return Err(HeapError::AllocatorArenaLimitExceeded {
                required_arenas: arena_index + 1,
                max_arenas: self.max_arena_count(),
            });
        }

        // already have arena
        if self.arena(arena_index).is_some() {
            return Ok(());
        }

        // commit arena bytes
        let base = self.commit_arena_base(state, required_arena_count)?;
        let base_address = base as usize;
        let mut arena = Box::new(Arena::new(arena_index, base, self.pages_per_arena()));
        let arena_ptr = arena.as_mut() as *mut Arena;

        self.arena_index
            .insert(arena_index, base_address, self.arena_bytes(), arena_ptr)?;
        state.owned_arenas.push(arena);

        Ok(())
    }

    /// Commit one arena inside the current reservation.
    fn commit_arena_base(
        &self,
        state: &mut AllocatorState,
        required_arena_count: usize,
    ) -> HeapResult<*mut u8> {
        if let Some(reservation) = state.arena_reservations.last_mut()
            && reservation.remaining_arena_count(self.arena_bytes()) >= required_arena_count
            && let Some(base) = reservation.allocate_arena(self.arena_bytes())?
        {
            return Ok(base);
        }

        let mut reservation = ArenaReservation::reserve(self.arena_bytes(), required_arena_count)?;
        let Some(base) = reservation.allocate_arena(self.arena_bytes())? else {
            return Err(HeapError::InvariantViolation {
                context: "allocator empty arena reservation",
            });
        };
        state.arena_reservations.push(reservation);

        Ok(base)
    }
}
