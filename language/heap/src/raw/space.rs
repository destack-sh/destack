use std::sync::Arc;

use super::{LargeEntry, LargeEntryId, RawPointerEntry, SmallSpan};
use crate::alloc::{Arena, SizeClassTable};
use crate::heap::{AllocationTotals, CowTable, HeapError, HeapOptions, RawSpaceUsage};

/// The first non-null raw entry id.
const FIRST_ALLOCATED_RAW_ID: u64 = 1;

/// The first non-null raw large-entry id.
const FIRST_ALLOCATED_LARGE_ENTRY_ID: u64 = 1;

/// Convert a stable raw pointer id into its packed representation.
pub(crate) fn checked_pointer_id(pointer_id: u64) -> crate::HeapResult<u32> {
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

    /// The exact live raw totals.
    pub(crate) totals: AllocationTotals,
}

impl RawSpace {
    /// Create one raw space with the default options.
    pub fn new() -> Result<Self, HeapError> {
        let options = HeapOptions::default();
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

        Ok(Self {
            arena,
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
            totals: AllocationTotals::default(),
        })
    }

    /// Return the shared page arena.
    pub fn arena(&self) -> &Arc<Arena> {
        &self.arena
    }

    /// Return the number of live raw entries.
    pub fn allocation_count(&self) -> usize {
        self.totals.allocation_count()
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
        )
    }

    /// Return the exact borrowed raw image bytes.
    pub fn borrowed_bytes(&self) -> crate::HeapResult<u64> {
        self.arena.borrowed_bytes_for_page_views(
            self.small
                .spans
                .iter()
                .map(|span| &span.pages)
                .chain(self.large.entries.iter().map(|entry| &entry.pages)),
        )
    }

    /// Return the exact live usage for this raw space.
    pub fn usage(&self) -> crate::HeapResult<RawSpaceUsage> {
        Ok(RawSpaceUsage {
            allocation_count: self.totals.allocation_count(),
            allocated_bytes: self.totals.allocated_bytes(),
            active_bytes: self.active_bytes(),
            mapped_bytes: self.mapped_bytes(),
            borrowed_bytes: self.borrowed_bytes()?,
        })
    }

    /// Return the dense table index for one raw pointer id.
    pub(crate) fn pointer_index(pointer_id: u32) -> crate::HeapResult<usize> {
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
    ) -> crate::HeapResult<()> {
        self.pointers
            .set_or_push(Self::pointer_index(pointer_id)?, record)
    }

    /// Retire one stable raw pointer slot.
    pub(crate) fn retire_pointer(&mut self, pointer_id: u32) -> crate::HeapResult<()> {
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
