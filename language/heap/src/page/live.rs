use std::sync::Arc;

use super::class::PageBitmap;
use super::image::{PageImage, RetainedBytes};

/// One live page with copy on write storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Page<T, const CAPACITY: usize, const WORDS: usize> {
    /// One owned mutable page payload.
    Owned {
        /// The entries stored in this page.
        entries: Box<[T]>,
        /// The occupancy bitmap for this page.
        occupied: PageBitmap<CAPACITY, WORDS>,
    },
    /// One immutable page image shared with snapshots or forked heaps.
    Shared {
        /// The shared immutable page image.
        image: Arc<PageImage<T, CAPACITY, WORDS>>,
    },
}

impl<T: Default, const CAPACITY: usize, const WORDS: usize> Page<T, CAPACITY, WORDS> {
    /// Create one empty owned page.
    pub(crate) fn new() -> Self {
        let entries = std::iter::repeat_with(T::default)
            .take(CAPACITY)
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self::Owned {
            entries,
            occupied: PageBitmap::new(),
        }
    }
}

impl<T, const CAPACITY: usize, const WORDS: usize> Page<T, CAPACITY, WORDS> {
    /// Restore one page from one immutable image.
    pub(crate) fn from_image(image: Arc<PageImage<T, CAPACITY, WORDS>>) -> Self {
        Self::Shared { image }
    }

    /// Capture one immutable page image and share it with future forks.
    pub(crate) fn image(&mut self) -> Arc<PageImage<T, CAPACITY, WORDS>> {
        // shared pages already have one durable image
        match self {
            Page::Shared { image } => image.clone(),
            Page::Owned { .. } => {
                let Page::Owned { entries, occupied } = self else {
                    unreachable!();
                };

                // move the owned payload into one immutable image
                let entries = std::mem::take(entries);
                let occupied = std::mem::take(occupied);
                let image = Arc::new(PageImage { entries, occupied });
                *self = Page::Shared {
                    image: image.clone(),
                };
                image
            }
        }
    }
}

impl<T: RetainedBytes, const CAPACITY: usize, const WORDS: usize> Page<T, CAPACITY, WORDS> {
    /// Return the retained bytes owned by this live page outside its inline form.
    pub(crate) fn retained_bytes(&self) -> usize {
        // owned pages count their entry payload directly
        match self {
            Page::Owned { entries, .. } => {
                let mut retained_bytes = std::mem::size_of_val(entries.as_ref());

                // include retained payload for each live entry
                for entry in entries.iter() {
                    retained_bytes += entry.retained_bytes();
                }

                retained_bytes
            }

            // shared pages defer to the immutable image
            Page::Shared { image } => image.retained_bytes(),
        }
    }
}

impl<T: Clone, const CAPACITY: usize, const WORDS: usize> Page<T, CAPACITY, WORDS> {
    /// Return mutable page storage, detaching one shared image on first write.
    fn storage_mut(&mut self) -> (&mut Box<[T]>, &mut PageBitmap<CAPACITY, WORDS>) {
        // detach one shared image into owned storage before mutation
        if let Page::Shared { image } = self {
            *self = Page::Owned {
                entries: image.entries.clone(),
                occupied: image.occupied,
            };
        }

        // owned pages expose mutable entries and occupancy directly
        match self {
            Page::Owned { entries, occupied } => (entries, occupied),
            Page::Shared { .. } => unreachable!(),
        }
    }
}

impl<T, const CAPACITY: usize, const WORDS: usize> Page<T, CAPACITY, WORDS> {
    /// Report whether one page offset is occupied.
    #[inline]
    pub(crate) fn is_occupied(&self, offset: usize) -> bool {
        // check occupancy against the current storage form
        match self {
            Page::Owned { occupied, .. } => occupied.contains(offset),
            Page::Shared { image } => image.is_occupied(offset),
        }
    }

    /// Return one entry by page offset.
    #[inline]
    pub(crate) fn get(&self, offset: usize) -> Option<&T> {
        // owned pages check occupancy before returning one entry
        match self {
            Page::Owned { entries, occupied } => {
                if !occupied.contains(offset) {
                    return None;
                }

                entries.get(offset)
            }

            // shared pages delegate to the immutable image
            Page::Shared { image } => image.get(offset),
        }
    }

    /// Return one mutable entry by page offset.
    #[inline]
    pub(crate) fn get_mut(&mut self, offset: usize) -> Option<&mut T>
    where
        T: Clone + Default,
    {
        let (entries, occupied) = self.storage_mut();

        // reject free page slots
        if !occupied.contains(offset) {
            return None;
        }

        entries.get_mut(offset)
    }

    /// Return one entry by page offset without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the page offset is valid and occupied.
    #[inline]
    pub(crate) unsafe fn get_unchecked(&self, offset: usize) -> &T {
        debug_assert!(self.is_occupied(offset), "page entry is not occupied");

        // read from the current storage form without bounds checks
        match self {
            Page::Owned { entries, .. } => unsafe { entries.get_unchecked(offset) },
            Page::Shared { image } => unsafe { image.get_unchecked(offset) },
        }
    }

    /// Return one mutable entry by page offset without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the page offset is valid and occupied.
    #[inline]
    pub(crate) unsafe fn get_unchecked_mut(&mut self, offset: usize) -> &mut T
    where
        T: Clone + Default,
    {
        let (entries, occupied) = self.storage_mut();
        debug_assert!(occupied.contains(offset), "page entry is not occupied");

        // read from detached owned storage without bounds checks
        unsafe { entries.get_unchecked_mut(offset) }
    }

    /// Store one entry at the given page offset.
    #[inline]
    pub(crate) fn set(&mut self, offset: usize, entry: T)
    where
        T: Clone + Default,
    {
        let (entries, occupied) = self.storage_mut();
        entries[offset] = entry;

        // mark newly occupied slots
        if !occupied.contains(offset) {
            occupied.set(offset);
        }
    }

    /// Remove one entry from the given page offset.
    #[inline]
    pub(crate) fn take(&mut self, offset: usize) -> Option<T>
    where
        T: Clone + Default,
    {
        let (entries, occupied) = self.storage_mut();

        // ignore free slots
        if !occupied.contains(offset) {
            return None;
        }

        // clear the entry and its occupancy bit
        let value = std::mem::take(&mut entries[offset]);
        occupied.clear(offset);

        Some(value)
    }
}
