use std::sync::Arc;

use super::{LargeEntry, LargeEntryId, RawPointerEntry, SmallSpan};
use crate::arena::{Arena, PageRunCache, PageView, SizeClassTable};
use crate::{AllocationUsage, CowTable, HeapError, HeapOptions, HeapResult, RawSpaceUsage};

/// The first non-null raw entry id.
const FIRST_ALLOCATED_RAW_ID: u64 = 1;

/// The first non-null raw large-entry id.
const FIRST_ALLOCATED_LARGE_ENTRY_ID: u64 = 1;

/// Convert a stable raw pointer id into its packed representation.
pub(crate) fn checked_pointer_id(pointer_id: u64) -> HeapResult<u32> {
    if pointer_id == 0 || pointer_id > u32::MAX as u64 {
        return Err(HeapError::InvalidRawPointerId { id: pointer_id });
    }

    Ok(pointer_id as u32)
}

/// One raw small space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SmallSpace {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured span width.
    pub(crate) span_bytes: usize,
    /// The live raw spans.
    pub(crate) spans: CowTable<SmallSpan>,
    /// The reusable non-full spans per size class.
    pub(crate) available_spans: Vec<Vec<usize>>,
}

/// One raw large space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LargeSpace {
    /// The configured page width for entries in large space.
    pub(crate) page_bytes: usize,
    /// The live raw entries.
    pub(crate) entries: CowTable<LargeEntry>,
    /// The free raw entry ids available for reuse.
    pub(crate) free_large_entry_ids: Vec<u64>,
    /// The next raw entry id to allocate.
    pub(crate) next_unused_large_entry_id: u64,
}

/// One live raw entry space rooted in one arena.
#[derive(Debug)]
pub struct RawSpace {
    /// The shared page arena for every raw payload.
    pub(super) arena: Arc<Arena>,
    /// The local front-end cache of reusable page runs.
    pub(crate) page_run_cache: PageRunCache,

    /// The entry count per metadata table chunk.
    pub(crate) table_chunk_len: usize,

    /// The raw small space.
    pub(crate) small: SmallSpace,
    /// The raw large space.
    pub(crate) large: LargeSpace,
    /// Dense raw pointer metadata keyed by entry id minus one.
    pub(crate) pointers: CowTable<RawPointerEntry>,

    /// The free raw entry ids available for reuse.
    pub(crate) free_pointer_ids: Vec<u64>,
    /// The next raw entry id to allocate.
    pub(crate) next_unused_pointer_id: u64,

    /// The exact live raw usage.
    pub(crate) usage: AllocationUsage,
}

impl RawSpace {
    /// Create one raw space with the default options.
    pub fn new() -> Result<Self, HeapError> {
        let options = HeapOptions::local();
        let arena = Arc::new(Arena::try_new(
            options.page_bytes,
            options.arena_segment_bytes,
        )?);

        Self::with_options(arena, &options)
    }

    /// Create one raw space with explicit options.
    pub fn with_options(arena: Arc<Arena>, options: &HeapOptions) -> Result<Self, HeapError> {
        options.validate()?;
        options.validate_arena(&arena)?;
        let page_run_cache = PageRunCache::new(arena.pages_per_segment());

        Ok(Self {
            arena,
            page_run_cache,
            table_chunk_len: options.table_chunk_len,
            small: SmallSpace {
                size_classes: options.size_classes.clone(),
                span_bytes: options.raw_small_bytes,
                spans: CowTable::with_chunk_len(options.table_chunk_len)?,
                available_spans: vec![Vec::new(); options.size_classes.classes.len()],
            },
            large: LargeSpace {
                page_bytes: options.page_bytes,
                entries: CowTable::with_chunk_len(options.table_chunk_len)?,
                free_large_entry_ids: Vec::new(),
                next_unused_large_entry_id: FIRST_ALLOCATED_LARGE_ENTRY_ID,
            },
            pointers: CowTable::with_chunk_len(options.table_chunk_len)?,
            free_pointer_ids: Vec::new(),
            next_unused_pointer_id: FIRST_ALLOCATED_RAW_ID,
            usage: AllocationUsage::default(),
        })
    }

    /// Return the shared page arena.
    pub fn arena(&self) -> &Arc<Arena> {
        &self.arena
    }

    /// Return the number of live raw entries.
    pub fn allocation_count(&self) -> usize {
        self.usage.allocation_count()
    }

    /// Return the exact retained raw bytes.
    pub fn active_bytes(&self) -> u64 {
        self.mapped_bytes()
    }

    /// Return the exact mapped raw page bytes.
    pub fn mapped_bytes(&self) -> u64 {
        self.arena.mapped_bytes_for_page_views(
            self.small
                .spans
                .iter()
                .map(|span| &span.pages)
                .chain(self.large.entries.iter().map(|entry| &entry.pages)),
        ) + self.page_run_cache.cached_bytes(self.arena.page_bytes())
    }

    /// Return the exact borrowed raw image bytes.
    pub fn borrowed_bytes(&self) -> HeapResult<u64> {
        self.arena.borrowed_bytes_for_page_views(
            self.small
                .spans
                .iter()
                .map(|span| &span.pages)
                .chain(self.large.entries.iter().map(|entry| &entry.pages)),
        )
    }

    /// Return the exact live usage for this raw space.
    pub fn usage(&self) -> HeapResult<RawSpaceUsage> {
        Ok(RawSpaceUsage {
            allocation_count: self.usage.allocation_count(),
            allocated_bytes: self.usage.allocated_bytes(),
            active_bytes: self.active_bytes(),
            mapped_bytes: self.mapped_bytes(),
            borrowed_bytes: self.borrowed_bytes()?,
        })
    }

    /// Allocate one zeroed page view through the local page-run cache.
    pub(crate) fn allocate_page_view_zeroed(&mut self, byte_len: usize) -> HeapResult<PageView> {
        self.page_run_cache.allocate_zeroed(&self.arena, byte_len)
    }

    /// Allocate one initialized page view through the local page-run cache.
    pub(crate) fn allocate_page_view_bytes(&mut self, bytes: &[u8]) -> HeapResult<PageView> {
        self.page_run_cache.allocate_bytes(&self.arena, bytes)
    }

    /// Release one page view through the local page-run cache.
    pub(crate) fn release_page_view(&mut self, page_view: PageView) -> HeapResult<()> {
        self.page_run_cache
            .release_page_view(&self.arena, page_view)
    }

    /// Flush the local page-run cache back into the arena page-run pool.
    pub(crate) fn flush_page_run_cache(&mut self) {
        self.page_run_cache.flush(&self.arena);
    }

    /// Flush transient cache state before one exact branch boundary.
    pub(crate) fn flush_branch_boundary(&mut self) {
        self.flush_page_run_cache();
    }

    /// Return the dense table index for one raw pointer id.
    pub(crate) fn pointer_index(pointer_id: u32) -> HeapResult<usize> {
        let Some(index) = pointer_id.checked_sub(1) else {
            return Err(HeapError::InvalidRawPointerId {
                id: pointer_id.into(),
            });
        };

        Ok(index as usize)
    }

    /// Store one dense raw pointer record by stable pointer id.
    pub(crate) fn set_pointer_entry(
        &mut self,
        pointer_id: u32,
        record: RawPointerEntry,
    ) -> HeapResult<()> {
        self.pointers
            .set_or_push(Self::pointer_index(pointer_id)?, record)
    }

    /// Retire one stable raw pointer slot.
    pub(crate) fn retire_pointer(&mut self, pointer_id: u32) -> HeapResult<()> {
        self.pointers
            .set(Self::pointer_index(pointer_id)?, RawPointerEntry::vacant())?;
        self.free_pointer_ids.push(pointer_id.into());

        Ok(())
    }

    /// Return one live raw large entry by id.
    pub(super) fn large_entry(&self, entry_id: LargeEntryId) -> Option<&LargeEntry> {
        let index = entry_id.index().ok()?;
        let entry = self.large.entries.get(index)?;

        if entry.is_live { Some(entry) } else { None }
    }

    /// Return one live raw large entry mutably by id.
    pub(super) fn large_entry_mut(&mut self, entry_id: LargeEntryId) -> Option<&mut LargeEntry> {
        let index = entry_id.index().ok()?;
        let entry = self.large.entries.get_mut(index)?;

        if entry.is_live { Some(entry) } else { None }
    }

    /// Return one live raw span by index.
    pub(super) fn span(&self, span_index: usize) -> Option<&SmallSpan> {
        self.small.spans.get(span_index)
    }

    /// Return one live raw span mutably by index.
    pub(super) fn span_mut(&mut self, span_index: usize) -> Option<&mut SmallSpan> {
        self.small.spans.get_mut(span_index)
    }
}

impl Drop for RawSpace {
    fn drop(&mut self) {
        self.flush_page_run_cache();
    }
}
