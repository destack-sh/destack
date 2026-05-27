use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

use crate::{MemoryError, MemoryResult};

/// The maximum number of concurrently watched address spaces.
const MAX_WRITE_WATCH_ENTRIES: usize = 16 * 1024;
/// The number of watch entries allocated together.
const WRITE_WATCH_ENTRY_PAGE_LEN: usize = 256;
/// The number of lazily allocated watch table pages.
const WRITE_WATCH_PAGE_COUNT: usize = MAX_WRITE_WATCH_ENTRIES / WRITE_WATCH_ENTRY_PAGE_LEN;

/// The process wide write watch registry.
static WRITE_WATCH_TABLE: OnceLock<WriteWatchTable> = OnceLock::new();

/// The process wide write watch table.
pub(crate) struct WriteWatchTable {
    /// The lazily allocated watch pages.
    pages: Box<[WatchPage]>,
}

impl WriteWatchTable {
    /// Register one write watched virtual range.
    pub(crate) fn register(
        base: *mut u8,
        byte_len: usize,
        context: *const (),
    ) -> MemoryResult<WriteWatchRegistration> {
        let base = base as usize;
        let table = Self::global();

        // reuse already allocated pages before growing the table
        for (page_index, page) in table.pages.iter().enumerate() {
            let Some(entries) = page.entries() else {
                continue;
            };

            if let Some(entry_index) = register_entry(entries, base, byte_len, context) {
                return Ok(WriteWatchRegistration {
                    page: page_index,
                    entry: entry_index,
                });
            }
        }

        // allocate exactly one new page when no existing entry is free
        for (page_index, page) in table.pages.iter().enumerate() {
            if page.entries().is_some() {
                continue;
            }

            let entries = page.allocate_entries();
            if let Some(entry_index) = register_entry(entries, base, byte_len, context) {
                return Ok(WriteWatchRegistration {
                    page: page_index,
                    entry: entry_index,
                });
            }
        }

        Err(MemoryError::TooManyWriteWatchRanges {
            capacity: MAX_WRITE_WATCH_ENTRIES,
        })
    }

    /// Return the registered page table context for one watched address.
    pub(crate) fn context(address: usize) -> Option<*const ()> {
        let table = WRITE_WATCH_TABLE.get()?;

        // scan watched ranges without signal unsafe locks
        for page in table.pages.iter() {
            let Some(entries) = page.entries() else {
                continue;
            };

            for entry in entries {
                let Some(context) = entry.context(address) else {
                    continue;
                };

                return Some(context);
            }
        }

        None
    }

    /// Unregister one write watched virtual range.
    pub(crate) fn unregister(registration: &WriteWatchRegistration) {
        let table = Self::global();
        let Some(entries) = table.pages[registration.page].entries() else {
            return;
        };
        let entry = &entries[registration.entry];

        entry
            .state
            .store(WatchState::Empty.byte(), Ordering::Release);
    }

    /// Return the process wide write watch table.
    fn global() -> &'static Self {
        WRITE_WATCH_TABLE.get_or_init(Self::new)
    }

    /// Create one empty write watch table.
    fn new() -> Self {
        let pages = (0..WRITE_WATCH_PAGE_COUNT)
            .map(|_| WatchPage::empty())
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self { pages }
    }
}

/// A write watch entry lifecycle state.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WatchState {
    /// The entry is available for registration.
    Empty = 0,
    /// The entry is being published.
    Claiming = 1,
    /// The entry is visible to the fault handler.
    Live = 2,
}

impl WatchState {
    /// Return the atomic byte representation.
    const fn byte(self) -> u8 {
        self as u8
    }
}

/// One registered write watched virtual range.
#[derive(Debug)]
pub(crate) struct WriteWatchRegistration {
    /// The occupied watch table page.
    page: usize,
    /// The occupied watch table entry.
    entry: usize,
}

/// One lazily allocated watch table page.
struct WatchPage {
    /// The page entries.
    entries: OnceLock<Box<[WriteWatchEntry]>>,
}

impl WatchPage {
    /// Create one empty watch table page.
    fn empty() -> Self {
        Self {
            entries: OnceLock::new(),
        }
    }

    /// Return the allocated entries when this page has been used.
    fn entries(&self) -> Option<&[WriteWatchEntry]> {
        self.entries.get().map(Box::as_ref)
    }

    /// Allocate the entries for this page if needed.
    fn allocate_entries(&self) -> &[WriteWatchEntry] {
        self.entries.get_or_init(|| {
            (0..WRITE_WATCH_ENTRY_PAGE_LEN)
                .map(|_| WriteWatchEntry::empty())
                .collect::<Vec<_>>()
                .into_boxed_slice()
        })
    }
}

/// One registered write watch entry.
struct WriteWatchEntry {
    /// The entry lifecycle state.
    state: AtomicU8,
    /// The inclusive start address.
    base: AtomicUsize,
    /// The watched byte length.
    byte_len: AtomicUsize,
    /// The registered page table context.
    context: AtomicUsize,
}

impl WriteWatchEntry {
    /// Create one empty watch table entry.
    fn empty() -> Self {
        Self {
            state: AtomicU8::new(WatchState::Empty.byte()),
            base: AtomicUsize::new(0),
            byte_len: AtomicUsize::new(0),
            context: AtomicUsize::new(0),
        }
    }

    /// Register one watched range in this entry.
    fn register(&self, base: usize, byte_len: usize, context: *const ()) -> bool {
        let result = self.state.compare_exchange(
            WatchState::Empty.byte(),
            WatchState::Claiming.byte(),
            Ordering::AcqRel,
            Ordering::Acquire,
        );

        if result.is_err() {
            return false;
        }

        // publish the range before making the entry visible
        self.base.store(base, Ordering::Relaxed);
        self.byte_len.store(byte_len, Ordering::Relaxed);
        self.context.store(context as usize, Ordering::Relaxed);
        self.state.store(WatchState::Live.byte(), Ordering::Release);

        true
    }

    /// Return the registered page table context for one address.
    fn context(&self, address: usize) -> Option<*const ()> {
        let state = self.state.load(Ordering::Acquire);

        if state != WatchState::Live.byte() {
            return None;
        }

        let base = self.base.load(Ordering::Relaxed);

        // ignore addresses before this range
        if address < base {
            return None;
        }

        let byte_len = self.byte_len.load(Ordering::Relaxed);
        let offset = address - base;

        // ignore addresses after this range
        if offset >= byte_len {
            return None;
        }

        let context = self.context.load(Ordering::Relaxed);

        Some(context as *const ())
    }
}

/// Register one watched range in the first free entry.
fn register_entry(
    entries: &[WriteWatchEntry],
    base: usize,
    byte_len: usize,
    context: *const (),
) -> Option<usize> {
    for (entry_index, entry) in entries.iter().enumerate() {
        if entry.register(base, byte_len, context) {
            return Some(entry_index);
        }
    }

    None
}
