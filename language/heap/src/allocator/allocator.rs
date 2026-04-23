use std::slice;
use std::sync::atomic::Ordering;

use parking_lot::Mutex;

use super::arena::{
    Arena, ArenaAddressMap, ArenaLocation, ArenaTable, allocate_arena_bytes, arena_frame_index,
    arena_frame_indices, free_arena_bytes, max_arena_count, max_arena_frame_count,
};
use super::{PageId, PageRun, PageRunSet, PageView};
use crate::{HeapError, HeapOptions, HeapResult};

/// The standard allocator page size.
pub(crate) const DEFAULT_PAGE_BYTES: usize = 4 * 1024;

/// The standard allocator arena size.
pub(crate) const DEFAULT_ALLOCATOR_ARENA_BYTES: usize = 1024 * 1024;

/// The admitted arena frontier, current fresh arena, free runs, and arena ownership.
#[derive(Debug)]
struct AllocatorState {
    /// The number of arenas already admitted into allocation state.
    admitted_arena_count: usize,
    /// The current fresh arena used for monotonic single-arena allocation.
    fresh_arena_index: Option<usize>,
    /// The free physical runs.
    free_runs: PageRunSet,
    /// The owned arena records.
    owned_arenas: Vec<Box<Arena>>,
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
    /// The dense arena table keyed by logical arena index.
    arenas: ArenaTable,
    /// The sparse arena map keyed by mapped arena address.
    arena_map: ArenaAddressMap,
    /// The admitted arena frontier, fresh arena, and free-run index.
    state: Mutex<AllocatorState>,
}

// allocator metadata is synchronized internally
unsafe impl Send for Allocator {}

// payload access is external, allocator metadata is synchronized internally
unsafe impl Sync for Allocator {}

impl Drop for Allocator {
    fn drop(&mut self) {
        let arena_bytes = self.arena_bytes();
        let state = self.state.get_mut();

        for arena in state.owned_arenas.drain(..) {
            free_arena_bytes(arena.base().as_ptr(), arena_bytes);
        }
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
            arenas: ArenaTable::new(max_arena_count),
            arena_map: ArenaAddressMap::new(max_arena_frames),
            state: Mutex::new(AllocatorState {
                admitted_arena_count: 0,
                fresh_arena_index: None,
                free_runs: PageRunSet::new(pages_per_arena),
                owned_arenas: Vec::new(),
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
        if let Some(run) = self.take_page_run(page_count) {
            self.zero_run(run)?;
            self.initialize_run_refcount(run)?;

            return Ok(run);
        }

        // otherwise carve a fresh run from the current arena when it fits
        let run = if page_count <= self.pages_per_arena()
            && let Some(run) = self.take_fresh_run(page_count)?
        {
            run
        }
        // fall back to one spanning fresh run
        else {
            self.allocate_spanning_run(page_count)?
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

    /// Decrement one physical run refcount.
    pub(super) fn decrement_run_refcount(&self, run: PageRun) -> HeapResult<()> {
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

    /// Ensure this allocator can address the given number of pages.
    pub(super) fn ensure_page_capacity(&self, page_count: usize) -> HeapResult<()> {
        let required_arena_count = page_count.div_ceil(self.pages_per_arena());
        let max_arena_count = self.max_arena_count();
        if required_arena_count > max_arena_count {
            return Err(HeapError::AllocatorArenaLimitExceeded {
                required_arenas: required_arena_count,
                max_arenas: max_arena_count,
            });
        }

        // admit the required arena range into allocation state
        let mut state = self.state.lock();
        let first_new_arena = state.admitted_arena_count;
        state.admitted_arena_count = state.admitted_arena_count.max(required_arena_count);
        let admitted_arena_count = state.admitted_arena_count;
        drop(state);

        // initialize newly admitted arena metadata
        self.initialize_arena_range(first_new_arena, admitted_arena_count)?;

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
    fn take_page_run(&self, page_count: usize) -> Option<PageRun> {
        let mut state = self.state.lock();

        state.free_runs.take(page_count)
    }

    /// Return one fresh run that fits inside one existing or new arena.
    fn take_fresh_run(&self, page_count: usize) -> HeapResult<Option<PageRun>> {
        loop {
            // fast path: claim from the current fresh arena directly
            let fresh_arena_index = self.state.lock().fresh_arena_index;
            if let Some(arena_index) = fresh_arena_index
                && let Some(run) = self.allocate_arena_run(arena_index, page_count)?
            {
                return Ok(Some(run));
            }

            // slow path: pick or admit the next fresh arena
            let arena_index = self.ensure_fresh_arena(page_count)?;
            if let Some(run) = self.allocate_arena_run(arena_index, page_count)? {
                return Ok(Some(run));
            }
        }
    }

    /// Allocate one fresh run that spans newly admitted arenas.
    fn allocate_spanning_run(&self, page_count: usize) -> HeapResult<PageRun> {
        let required_arena_count = page_count.div_ceil(self.pages_per_arena());
        let first_arena_index = self.admit_arena_range(required_arena_count)?;
        let last_arena_len = page_count % self.pages_per_arena();

        // mark each newly admitted arena run as consumed
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

        let fresh_arena_index =
            (last_arena_len != 0).then_some(first_arena_index + required_arena_count - 1);
        self.state.lock().fresh_arena_index = fresh_arena_index;

        let first_page_index = first_arena_index
            .checked_mul(self.pages_per_arena())
            .ok_or(HeapError::InvalidPageId {
                index: first_arena_index,
            })?;
        let first_page = PageId::new(first_page_index)?;

        PageRun::new(first_page, page_count)
    }

    /// Return one newly admitted arena range start.
    fn admit_arena_range(&self, arena_count: usize) -> HeapResult<usize> {
        let mut state = self.state.lock();
        let first_arena_index = state.admitted_arena_count;
        let end_arena_index =
            first_arena_index
                .checked_add(arena_count)
                .ok_or(HeapError::InvariantOverflow {
                    context: "allocator admitted arena count",
                })?;

        let max_arena_count = self.max_arena_count();
        if end_arena_index > max_arena_count {
            return Err(HeapError::AllocatorArenaLimitExceeded {
                required_arenas: end_arena_index,
                max_arenas: max_arena_count,
            });
        }

        state.admitted_arena_count = end_arena_index;
        drop(state);

        // initialize the admitted arena metadata
        self.initialize_arena_range(first_arena_index, end_arena_index)?;

        Ok(first_arena_index)
    }

    /// Return one usable fresh arena, admitting one when necessary.
    fn ensure_fresh_arena(&self, page_count: usize) -> HeapResult<usize> {
        let state = self.state.lock();
        let fresh_arena_index = state.fresh_arena_index;
        let admitted_arena_count = state.admitted_arena_count;
        drop(state);

        if let Some(arena_index) = fresh_arena_index
            && arena_index < admitted_arena_count
            && self.arena_has_capacity(arena_index, page_count)?
        {
            return Ok(arena_index);
        }

        let arena_index = self.admit_arena_range(1)?;
        self.state.lock().fresh_arena_index = Some(arena_index);

        Ok(arena_index)
    }

    /// Allocate one fresh run from one specific arena.
    fn allocate_arena_run(
        &self,
        arena_index: usize,
        page_count: usize,
    ) -> HeapResult<Option<PageRun>> {
        let Some(arena) = self.arena(arena_index) else {
            return Ok(None);
        };

        arena.allocate_run(arena_index, page_count, self.pages_per_arena())
    }

    /// Report whether one arena still has capacity for one fresh run.
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
            return Err(HeapError::MissingRunRefcount {
                first_page: run.first_page,
            });
        };

        let Some(refcount) = arena.run_refcount(arena_page_index) else {
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

        let arena_frame_index = arena_frame_index(address, self.arena_bytes())?;
        let arena = self.arena_map.get(arena_frame_index)?;
        let arena_base = arena.base().as_ptr() as usize;
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

        state.free_runs.insert(run);
    }

    /// Free one cache-owned run back into the allocator free-run index.
    pub(super) fn free_cached_run(&self, run: PageRun) {
        if run.is_empty() {
            return;
        }

        let refcount = self.run_refcount(run).unwrap_or_else(|error| {
            panic!("cached page run should resolve one live refcount: {error}")
        });

        let current_refcount = refcount.swap(0, Ordering::AcqRel);
        if current_refcount != 1 {
            panic!(
                "cached page run should stay uniquely owned: first_page={}, refcount={current_refcount}",
                run.first_page.index()
            );
        }

        self.free_run(run);
    }

    /// Raise one arena fresh-allocation watermark to the given page index.
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
        state.admitted_arena_count = state.admitted_arena_count.max(arena_index + 1);

        if high_watermark < self.pages_per_arena() && arena_index + 1 == state.admitted_arena_count
        {
            state.fresh_arena_index = Some(arena_index);
        }

        Ok(())
    }

    /// Return one existing arena through the fixed arena metadata table.
    fn arena(&self, arena_index: usize) -> Option<&Arena> {
        self.arenas.get(arena_index)
    }

    /// Initialize one admitted arena range.
    fn initialize_arena_range(
        &self,
        first_arena_index: usize,
        end_arena_index: usize,
    ) -> HeapResult<()> {
        for arena_index in first_arena_index..end_arena_index {
            if self.arena(arena_index).is_some() {
                continue;
            }

            self.allocate_arena(arena_index)?;
        }

        Ok(())
    }

    /// Allocate and register one newly admitted arena.
    fn allocate_arena(&self, arena_index: usize) -> HeapResult<()> {
        if arena_index >= self.max_arena_count() {
            return Err(HeapError::AllocatorArenaLimitExceeded {
                required_arenas: arena_index + 1,
                max_arenas: self.max_arena_count(),
            });
        }

        // allocate actual bytes
        let base = allocate_arena_bytes(self.arena_bytes())?;
        let base_address = base.as_ptr() as usize;
        let mut arena = Box::new(Arena::new(arena_index, base, self.pages_per_arena()));
        let arena_ptr = arena.as_mut() as *mut Arena;

        self.arenas.insert(arena_index, arena_ptr);
        for arena_frame_index in arena_frame_indices(base_address, self.arena_bytes())? {
            self.arena_map.insert(arena_frame_index, arena_ptr);
        }
        self.state.lock().owned_arenas.push(arena);

        Ok(())
    }
}
