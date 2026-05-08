use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicU8, Ordering};

#[cfg(not(target_arch = "wasm32"))]
use crate::platform;
use crate::platform::PageFrame;

/// The atomic state tag for one mapped page.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PageTag {
    /// No page frame is currently mapped.
    Absent = 0,
    /// One writable owner has current bytes in the backing frame.
    Exclusive = 1,
    /// One fork-shared page has current bytes in the backing frame.
    Clean = 2,
    /// One fork-shared page may have private bytes outside the backing frame.
    Dirty = 3,
}

impl PageTag {
    /// Return the atomic byte representation.
    const fn byte(self) -> u8 {
        self as u8
    }

    /// Return the page tag represented by one atomic byte.
    fn from_byte(byte: u8) -> Self {
        match byte {
            0 => Self::Absent,
            1 => Self::Exclusive,
            2 => Self::Clean,
            3 => Self::Dirty,
            _ => unreachable!("invalid page tag"),
        }
    }
}

/// The mapping state for one materialized page.
#[derive(Debug, Clone, Copy)]
pub(super) enum PageState {
    /// The page is not mapped.
    Absent,
    /// The page is shared writable and the backing frame is current.
    Exclusive(PageFrame),
    /// The page is private read-only and the backing frame is current.
    Clean(PageFrame),
    /// The page is private writable and may differ from the backing frame.
    Dirty(PageFrame),
}

impl PageState {
    /// Return the backing frame for mapped page states.
    pub(super) fn frame(self) -> Option<PageFrame> {
        match self {
            Self::Absent => None,
            Self::Exclusive(frame) | Self::Clean(frame) | Self::Dirty(frame) => Some(frame),
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
    /// The page entries indexed by page number.
    entries: Box<[PageEntry]>,
}

impl PageTable {
    /// Create one page table for a reserved virtual range.
    pub(super) fn new(base_address: usize, byte_len: usize, frame_bytes: usize) -> Self {
        #[cfg(target_arch = "wasm32")]
        let _ = base_address;

        let page_count = byte_len / frame_bytes;
        let entries = (0..page_count)
            .map(|_| PageEntry::empty())
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self {
            #[cfg(not(target_arch = "wasm32"))]
            base_address,
            #[cfg(not(target_arch = "wasm32"))]
            byte_len,
            #[cfg(not(target_arch = "wasm32"))]
            frame_bytes,
            entries,
        }
    }

    /// Return one page entry by index.
    pub(super) fn entry(&self, page_index: usize) -> &PageEntry {
        &self.entries[page_index]
    }

    /// Return true when one page is mapped.
    pub(super) fn is_mapped(&self, page_index: usize) -> bool {
        self.entry(page_index).is_mapped()
    }

    /// Return every mapped page index.
    pub(super) fn mapped_pages(&self) -> impl Iterator<Item = usize> + '_ {
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(page_index, entry)| entry.is_mapped().then_some(page_index))
    }

    /// Return every mapped page state.
    pub(super) fn mapped_states(&self) -> impl Iterator<Item = (usize, PageState)> + '_ {
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(page_index, entry)| match entry.state() {
                PageState::Absent => None,
                state => Some((page_index, state)),
            })
    }

    /// Mark the watched clean page dirty and writable.
    #[cfg(not(target_arch = "wasm32"))]
    fn handle_write_watch(&self, address: usize) -> bool {
        if address < self.base_address || address >= self.base_address + self.byte_len {
            return false;
        }

        let page_index = (address - self.base_address) / self.frame_bytes;
        let entry = self.entry(page_index);
        if !entry.is_clean() {
            return false;
        }

        let base = self.base_address as *mut u8;

        if platform::make_clean_pages_writable(base, page_index, self.frame_bytes, self.frame_bytes)
            .is_err()
        {
            return false;
        }

        entry.mark_dirty();

        true
    }
}

/// Mark one watched page dirty.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) unsafe fn watch_page_write(context: *const (), address: usize) -> bool {
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

// frame writes are serialized by the owning page map
unsafe impl Send for PageEntry {}

// frame writes are serialized by the owning page map
unsafe impl Sync for PageEntry {}

impl PageEntry {
    /// Create one empty page entry.
    fn empty() -> Self {
        Self {
            state: AtomicU8::new(PageTag::Absent.byte()),
            frame: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    /// Return the current page state.
    pub(super) fn state(&self) -> PageState {
        let tag = PageTag::from_byte(self.state.load(Ordering::Acquire));

        match tag {
            PageTag::Absent => PageState::Absent,
            PageTag::Exclusive => PageState::Exclusive(self.frame()),
            PageTag::Clean => PageState::Clean(self.frame()),
            PageTag::Dirty => PageState::Dirty(self.frame()),
        }
    }

    /// Return true when a page frame is mapped.
    fn is_mapped(&self) -> bool {
        self.tag() != PageTag::Absent
    }

    /// Store one page state.
    pub(super) fn set_state(&self, state: PageState) {
        match state {
            PageState::Absent => {
                self.set_tag(PageTag::Absent);
            }
            PageState::Exclusive(frame) => {
                self.set_frame(frame, PageTag::Exclusive);
            }
            PageState::Clean(frame) => {
                self.set_frame(frame, PageTag::Clean);
            }
            PageState::Dirty(frame) => {
                self.set_frame(frame, PageTag::Dirty);
            }
        }
    }

    /// Return true when this page is clean.
    #[cfg(not(target_arch = "wasm32"))]
    fn is_clean(&self) -> bool {
        self.tag() == PageTag::Clean
    }

    /// Mark one clean page dirty.
    #[cfg(not(target_arch = "wasm32"))]
    fn mark_dirty(&self) {
        self.set_tag(PageTag::Dirty);
    }

    /// Store one mapped frame and state.
    fn set_frame(&self, frame: PageFrame, tag: PageTag) {
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
        unsafe { (*self.frame.get()).assume_init() }
    }
}
