use serde::{Deserialize, Serialize};

use destack_mir::LayoutId;

use crate::HeapResult;
use crate::allocator::{Allocator, PageRunCache, PageView};

/// One heap young-entry identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct HeapYoungId {
    /// The young-space generation that owns this entry.
    generation: u32,
    /// The zero-based entry index inside that generation.
    index: u32,
}

impl HeapYoungId {
    /// Create one heap young-entry identifier.
    pub(crate) const fn new(generation: u32, index: u32) -> Self {
        Self { generation, index }
    }

    /// Return the owning young-space generation.
    pub(crate) const fn generation(self) -> u32 {
        self.generation
    }

    /// Return the zero-based young-entry index.
    pub(crate) const fn index(self) -> u32 {
        self.index
    }
}

/// One live young-entry metadata entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct YoungEntry {
    /// The first page touched by this entry.
    pub(crate) first_page: u32,
    /// The first byte offset inside the first page.
    pub(crate) first_offset: u32,
    /// The logical byte length for this entry.
    pub(crate) byte_len: usize,
    /// The managed layout stored in this entry.
    pub(crate) layout_id: LayoutId,
    /// Whether this young entry is still live.
    pub(crate) is_live: bool,
    /// Whether this young entry is marked in the active cycle.
    pub(crate) is_marked: bool,
}

/// One frozen young-space root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct YoungImage {
    /// The generation number for this young space.
    generation: u32,
    /// The configured byte capacity for the young space.
    capacity_bytes: usize,
    /// The fixed page width for young storage.
    page_bytes: usize,
    /// The bump-allocation cursor inside the logical young byte space.
    next_offset: usize,
    /// The allocator pages backing this young space.
    pages: PageView,
    /// The captured young-entry metadata entries.
    entries: Box<[YoungEntry]>,
}

/// One live heap young space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct YoungSpace {
    /// The generation number for this young space.
    pub(crate) generation: u32,
    /// The configured byte capacity for the young space.
    pub(crate) capacity_bytes: usize,
    /// The fixed page width for young storage.
    pub(crate) page_bytes: usize,
    /// The bump-allocation cursor inside the logical young byte space.
    pub(crate) next_offset: usize,
    /// The allocator pages backing this young space.
    pub(crate) pages: PageView,
    /// The live young-entry metadata entries.
    pub(crate) entries: Vec<YoungEntry>,
}

impl YoungSpace {
    /// Create one empty young space with its full page run.
    pub(crate) fn new(
        allocator: &Allocator,
        capacity_bytes: usize,
        page_bytes: usize,
        cache: &mut PageRunCache,
    ) -> HeapResult<Self> {
        Ok(Self {
            generation: 0,
            capacity_bytes,
            page_bytes,
            next_offset: 0,
            pages: cache.allocate_zeroed(allocator, capacity_bytes)?,
            entries: Vec::new(),
        })
    }
}

impl YoungImage {
    /// Create one frozen young-space root.
    pub(crate) fn new(
        generation: u32,
        capacity_bytes: usize,
        page_bytes: usize,
        next_offset: usize,
        pages: PageView,
        entries: Box<[YoungEntry]>,
    ) -> Self {
        Self {
            generation,
            capacity_bytes,
            page_bytes,
            next_offset,
            pages,
            entries,
        }
    }

    /// Return the generation number.
    pub(crate) const fn generation(&self) -> u32 {
        self.generation
    }

    /// Return the young-space byte capacity.
    pub(crate) const fn capacity_bytes(&self) -> usize {
        self.capacity_bytes
    }

    /// Return the young page width.
    pub(crate) const fn page_bytes(&self) -> usize {
        self.page_bytes
    }

    /// Return the next bump offset.
    pub(crate) const fn next_offset(&self) -> usize {
        self.next_offset
    }

    /// Return the allocator pages for this young root.
    pub(crate) fn pages(&self) -> &PageView {
        &self.pages
    }

    /// Return the entry table.
    pub(crate) fn entries(&self) -> &[YoungEntry] {
        &self.entries
    }
}
