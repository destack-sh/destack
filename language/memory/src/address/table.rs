use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, AtomicU8, Ordering};

#[cfg(not(target_arch = "wasm32"))]
use crate::platform;
use crate::platform::PageFrame;

/// The number of page entries stored in one sparse table chunk.
const PAGE_CHUNK_LEN: usize = 1024;

/// The atomic state tag for one mapped page.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PageTag {
    /// Virtual address space is reserved, but no frame is mapped.
    Reserved = 0,
    /// One writable owner has current bytes in the backing frame.
    Owned = 1,
    /// One fork-shared page has current bytes in the backing frame.
    Shared = 2,
    /// One fork-shared page may have private bytes outside the backing frame.
    Modified = 3,
}

impl PageTag {
    /// Return the atomic byte representation.
    const fn byte(self) -> u8 {
        self as u8
    }

    /// Return the page tag represented by one atomic byte.
    fn from_byte(byte: u8) -> Self {
        match byte {
            0 => Self::Reserved,
            1 => Self::Owned,
            2 => Self::Shared,
            3 => Self::Modified,
            _ => unreachable!("invalid page tag"),
        }
    }
}

/// The mapping state for one materialized page.
#[derive(Debug, Clone, Copy)]
pub(super) enum PageState {
    /// The page is reserved but not mapped.
    Reserved,
    /// The page is writable by this map and the backing frame is current.
    Owned(PageFrame),
    /// The page is fork-shared and the backing frame is current.
    Shared(PageFrame),
    /// The page is writable by this map and may differ from the backing frame.
    Modified(PageFrame),
}

impl PageState {
    /// Return the backing frame for mapped page states.
    pub(super) fn frame(self) -> Option<PageFrame> {
        match self {
            Self::Reserved => None,
            Self::Owned(frame) | Self::Shared(frame) | Self::Modified(frame) => Some(frame),
        }
    }
}

/// Atomic page metadata for one address map.
#[derive(Debug)]
pub(super) struct PageTable {
    /// The base address of the registered virtual space.
    #[cfg(not(target_arch = "wasm32"))]
    base_address: usize,
    /// The reserved byte length.
    #[cfg(not(target_arch = "wasm32"))]
    byte_len: usize,
    /// The native page-frame width.
    #[cfg(not(target_arch = "wasm32"))]
    frame_bytes: usize,
    /// The number of pages covered by the table.
    page_count: usize,
    /// The sparse page entry chunks indexed by page chunk.
    chunks: Box<[AtomicPtr<PageChunk>]>,
}

impl PageTable {
    /// Create one page table for a reserved virtual range.
    pub(super) fn new(base_address: usize, byte_len: usize, frame_bytes: usize) -> Self {
        #[cfg(target_arch = "wasm32")]
        let _ = base_address;

        let page_count = byte_len / frame_bytes;
        let chunk_count = page_count.div_ceil(PAGE_CHUNK_LEN);
        let chunks = (0..chunk_count)
            .map(|_| AtomicPtr::new(null_mut()))
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self {
            #[cfg(not(target_arch = "wasm32"))]
            base_address,
            #[cfg(not(target_arch = "wasm32"))]
            byte_len,
            #[cfg(not(target_arch = "wasm32"))]
            frame_bytes,
            page_count,
            chunks,
        }
    }

    /// Return one page entry by index when its chunk exists.
    fn entry(&self, page_index: usize) -> Option<&PageEntry> {
        let (chunk_index, entry_index) = self.chunk_location(page_index);
        let chunk = self.chunk(chunk_index)?;

        Some(&chunk.entries[entry_index])
    }

    /// Return one page entry by index, creating its chunk when needed.
    fn ensure_entry(&self, page_index: usize) -> &PageEntry {
        let (chunk_index, entry_index) = self.chunk_location(page_index);
        let chunk = self.ensure_chunk(chunk_index);

        &chunk.entries[entry_index]
    }

    /// Return one page state.
    pub(super) fn state(&self, page_index: usize) -> PageState {
        let Some(entry) = self.entry(page_index) else {
            return PageState::Reserved;
        };

        entry.state()
    }

    /// Store one page state.
    pub(super) fn set_state(&self, page_index: usize, state: PageState) {
        let entry = match state {
            PageState::Reserved => self.entry(page_index),
            _ => Some(self.ensure_entry(page_index)),
        };

        if let Some(entry) = entry {
            entry.set_state(state);
        }
    }

    /// Return true when one page is mapped.
    pub(super) fn is_mapped(&self, page_index: usize) -> bool {
        self.entry(page_index)
            .is_some_and(|entry| entry.is_mapped())
    }

    /// Return every mapped page index.
    pub(super) fn mapped_pages(&self) -> impl Iterator<Item = usize> + '_ {
        self.mapped_states().map(|(page_index, _)| page_index)
    }

    /// Return every mapped page state.
    pub(super) fn mapped_states(&self) -> impl Iterator<Item = (usize, PageState)> + '_ {
        let page_count = self.page_count;

        self.chunks
            .iter()
            .enumerate()
            .filter_map(|(chunk_index, slot)| {
                let chunk = Self::slot_chunk(slot)?;

                Some((chunk_index, chunk))
            })
            .flat_map(move |(chunk_index, chunk)| {
                let page_start = chunk_index * PAGE_CHUNK_LEN;

                chunk
                    .entries
                    .iter()
                    .enumerate()
                    .filter_map(move |(entry_index, entry)| {
                        let page_index = page_start + entry_index;
                        let is_inside_table = page_index < page_count;
                        if !is_inside_table {
                            return None;
                        }

                        match entry.state() {
                            PageState::Reserved => None,
                            state => Some((page_index, state)),
                        }
                    })
            })
    }

    /// Return one chunk and entry index for a page index.
    fn chunk_location(&self, page_index: usize) -> (usize, usize) {
        debug_assert!(page_index < self.page_count);

        (page_index / PAGE_CHUNK_LEN, page_index % PAGE_CHUNK_LEN)
    }

    /// Return one page chunk when it has been allocated.
    fn chunk(&self, chunk_index: usize) -> Option<&PageChunk> {
        self.chunks.get(chunk_index).and_then(Self::slot_chunk)
    }

    /// Return one page chunk from one atomic slot.
    fn slot_chunk(slot: &AtomicPtr<PageChunk>) -> Option<&PageChunk> {
        let pointer = slot.load(Ordering::Acquire);
        if pointer.is_null() {
            None
        } else {
            // SAFETY: chunks are heap allocated once and reclaimed only when the table drops
            Some(unsafe { &*pointer })
        }
    }

    /// Return one page chunk, allocating it if needed.
    fn ensure_chunk(&self, chunk_index: usize) -> &PageChunk {
        let slot = &self.chunks[chunk_index];
        if let Some(chunk) = Self::slot_chunk(slot) {
            return chunk;
        }

        // publish one fresh chunk, or use the chunk another thread published first
        let chunk = Box::into_raw(Box::new(PageChunk::new()));
        let pointer =
            match slot.compare_exchange(null_mut(), chunk, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_) => chunk,
                Err(existing) => {
                    // SAFETY: this thread still owns the unpublished chunk
                    unsafe {
                        drop(Box::from_raw(chunk));
                    }
                    existing
                }
            };

        // SAFETY: the winning chunk remains owned by the table until drop
        unsafe { &*pointer }
    }

    /// Return the number of allocated chunks.
    #[cfg(test)]
    fn allocated_chunk_count(&self) -> usize {
        self.chunks
            .iter()
            .filter(|slot| !slot.load(Ordering::Acquire).is_null())
            .count()
    }

    /// Mark the watched shared page modified and writable.
    #[cfg(not(target_arch = "wasm32"))]
    fn handle_write_watch(&self, address: usize) -> bool {
        if address < self.base_address || address >= self.base_address + self.byte_len {
            return false;
        }

        let page_index = (address - self.base_address) / self.frame_bytes;
        let Some(entry) = self.entry(page_index) else {
            return false;
        };
        if !entry.is_shared() {
            return false;
        }

        let base = self.base_address as *mut u8;

        if platform::make_shared_pages_writable(
            base,
            page_index,
            self.frame_bytes,
            self.frame_bytes,
        )
        .is_err()
        {
            return false;
        }

        entry.mark_modified();

        true
    }
}

impl Drop for PageTable {
    fn drop(&mut self) {
        for slot in &self.chunks {
            let pointer = slot.load(Ordering::Relaxed);
            if pointer.is_null() {
                continue;
            }

            // SAFETY: each non-null slot stores one Box allocated by ensure_chunk
            unsafe {
                drop(Box::from_raw(pointer));
            }
        }
    }
}

/// One sparse chunk of page entries.
#[derive(Debug)]
struct PageChunk {
    /// The page entries covered by this chunk.
    entries: Box<[PageEntry]>,
}

impl PageChunk {
    /// Create one reserved page chunk.
    fn new() -> Self {
        let entries = (0..PAGE_CHUNK_LEN)
            .map(|_| PageEntry::empty())
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self { entries }
    }
}

/// Mark one watched page modified.
///
/// # Safety
///
/// The context must be a live page table registered by its owning page map.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) unsafe fn watch_page_write(context: *const (), address: usize) -> bool {
    // SAFETY: the platform watch table registers only live page-table pointers
    let pages = unsafe { &*(context.cast::<PageTable>()) };

    pages.handle_write_watch(address)
}

/// One page entry in a forkable address map.
#[derive(Debug)]
pub(super) struct PageEntry {
    /// The current page state.
    state: AtomicU8,
    /// The backing frame for mapped page states.
    frame: UnsafeCell<MaybeUninit<PageFrame>>,
}

// SAFETY: frame writes are serialized by the owning page map
unsafe impl Send for PageEntry {}

// SAFETY: frame writes are serialized by the owning page map
unsafe impl Sync for PageEntry {}

impl PageEntry {
    /// Create one empty page entry.
    fn empty() -> Self {
        Self {
            state: AtomicU8::new(PageTag::Reserved.byte()),
            frame: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    /// Return the current page state.
    pub(super) fn state(&self) -> PageState {
        let tag = PageTag::from_byte(self.state.load(Ordering::Acquire));

        match tag {
            PageTag::Reserved => PageState::Reserved,
            PageTag::Owned => PageState::Owned(self.frame()),
            PageTag::Shared => PageState::Shared(self.frame()),
            PageTag::Modified => PageState::Modified(self.frame()),
        }
    }

    /// Return true when a page frame is mapped.
    fn is_mapped(&self) -> bool {
        self.tag() != PageTag::Reserved
    }

    /// Store one page state.
    pub(super) fn set_state(&self, state: PageState) {
        match state {
            PageState::Reserved => {
                self.set_tag(PageTag::Reserved);
            }
            PageState::Owned(frame) => {
                self.set_frame(frame, PageTag::Owned);
            }
            PageState::Shared(frame) => {
                self.set_frame(frame, PageTag::Shared);
            }
            PageState::Modified(frame) => {
                self.set_frame(frame, PageTag::Modified);
            }
        }
    }

    /// Return true when this page is fork-shared.
    #[cfg(not(target_arch = "wasm32"))]
    fn is_shared(&self) -> bool {
        self.tag() == PageTag::Shared
    }

    /// Mark one fork-shared page modified.
    #[cfg(not(target_arch = "wasm32"))]
    fn mark_modified(&self) {
        self.set_tag(PageTag::Modified);
    }

    /// Store one mapped frame and state.
    fn set_frame(&self, frame: PageFrame, tag: PageTag) {
        // SAFETY: callers serialize frame writes with the page-map lock
        unsafe {
            *self.frame.get() = MaybeUninit::new(frame);
        }

        self.set_tag(tag);
    }

    /// Return the current page tag.
    fn tag(&self) -> PageTag {
        PageTag::from_byte(self.state.load(Ordering::Acquire))
    }

    /// Store one page tag.
    fn set_tag(&self, tag: PageTag) {
        self.state.store(tag.byte(), Ordering::Release);
    }

    /// Return the mapped page frame.
    fn frame(&self) -> PageFrame {
        // SAFETY: non-reserved tags are published only after the frame is initialized
        unsafe { (*self.frame.get()).assume_init() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sparse page tables allocate no entry chunks on reservation.
    #[test]
    fn test_reserve_starts_without_chunks() {
        let table = PageTable::new(0, PAGE_CHUNK_LEN * 4 * 4096, 4096);

        assert_eq!(table.allocated_chunk_count(), 0);
        assert!(!table.is_mapped(0));
        assert!(table.mapped_pages().next().is_none());
    }

    /// Setting page states allocates only the touched chunks.
    #[test]
    fn test_set_state_allocates_touched_chunks() {
        let table = PageTable::new(0, PAGE_CHUNK_LEN * 4 * 4096, 4096);
        let first = PageFrame { offset: 0 };
        let second = PageFrame { offset: 4096 };
        let page_index = PAGE_CHUNK_LEN + 3;

        table.set_state(0, PageState::Owned(first));
        table.set_state(page_index, PageState::Shared(second));

        let pages = table.mapped_pages().collect::<Vec<_>>();
        let states = table.mapped_states().collect::<Vec<_>>();

        assert_eq!(table.allocated_chunk_count(), 2);
        assert_eq!(pages, vec![0, page_index]);
        assert!(matches!(states[0], (0, PageState::Owned(frame)) if frame == first));
        assert!(
            matches!(states[1], (index, PageState::Shared(frame)) if index == page_index && frame == second)
        );
    }
}
