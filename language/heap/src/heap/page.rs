use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::bitmap::Bitmap;

/// The number of cell slots in one heap page.
pub const HEAP_PAGE_CAPACITY: usize = 1024;

/// Heap page classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PageKind {
    /// Managed GC tracked heap cells.
    Managed,
    /// Raw heap cells.
    Raw,
}

/// Immutable page image shared by snapshots and forked heaps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageImage<T> {
    /// The cells stored in this page.
    pub cells: Box<[T]>,
    /// The occupancy bitmap for this page.
    pub occupied: Bitmap,
    /// The page classification.
    pub kind: PageKind,
}

impl<T> PageImage<T> {
    /// Return the page kind.
    #[inline]
    pub fn kind(&self) -> PageKind {
        self.kind
    }

    /// Report whether one slot offset is occupied.
    #[inline]
    pub fn is_occupied(&self, offset: usize) -> bool {
        self.occupied.contains(offset)
    }

    /// Return one cell by slot offset.
    #[inline]
    pub fn get(&self, offset: usize) -> Option<&T> {
        if !self.is_occupied(offset) {
            return None;
        }

        self.cells.get(offset)
    }

    /// Return one cell by slot offset without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the slot offset is valid and occupied.
    #[inline]
    pub unsafe fn get_unchecked(&self, offset: usize) -> &T {
        debug_assert!(self.is_occupied(offset), "page slot is not occupied");

        unsafe { self.cells.get_unchecked(offset) }
    }
}

/// One live heap page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page<T> {
    /// The cells stored in this page.
    cells: Box<[T]>,
    /// The occupancy bitmap for this page.
    occupied: Bitmap,
    /// The live mark bitmap for this page.
    marked: Bitmap,
    /// The page classification.
    kind: PageKind,
}

/// One private page slot reference inside one heap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PageReference<T> {
    /// One live page owned by this heap.
    Page(Box<Page<T>>),
    /// One immutable page image shared with snapshots or forked heaps.
    Image(Arc<PageImage<T>>),
}

impl<T: Default> Page<T> {
    /// Create one empty page.
    pub fn new(kind: PageKind) -> Self {
        let cells = std::iter::repeat_with(T::default)
            .take(HEAP_PAGE_CAPACITY)
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self {
            cells,
            occupied: Bitmap::new(),
            marked: Bitmap::new(),
            kind,
        }
    }
}

impl<T: Clone + Default> Page<T> {
    /// Restore one live page from one immutable image.
    pub fn from_image(image: &PageImage<T>) -> Self {
        Self {
            cells: image.cells.clone(),
            occupied: image.occupied,
            marked: Bitmap::new(),
            kind: image.kind,
        }
    }

    /// Convert one live page into one immutable image.
    pub fn into_image(self) -> PageImage<T> {
        PageImage {
            cells: self.cells,
            occupied: self.occupied,
            kind: self.kind,
        }
    }
}

impl<T: Clone + Default> PageReference<T> {
    /// Create one empty page reference.
    pub(super) fn new(kind: PageKind) -> Self {
        Self::Page(Box::new(Page::new(kind)))
    }

    /// Restore one page reference from one immutable image.
    pub(super) fn from_image(image: Arc<PageImage<T>>) -> Self {
        Self::Image(image)
    }

    /// Capture one immutable page image and share it with future forks.
    pub(super) fn freeze(&mut self) -> Arc<PageImage<T>> {
        // freeze one owned page into a shared page image
        if matches!(self, Self::Page(_)) {
            let replacement = match self {
                Self::Page(page) => Self::Page(Box::new(Page::new(page.kind()))),
                Self::Image(_) => unreachable!(),
            };

            let page = match std::mem::replace(self, replacement) {
                Self::Page(page) => page,
                Self::Image(_) => unreachable!(),
            };

            let image = Arc::new(page.into_image());
            *self = Self::Image(image.clone());

            return image;
        }

        match self {
            Self::Page(_) => unreachable!(),
            Self::Image(image) => image.clone(),
        }
    }

    // promote one shared image on first write
    fn page_mut(&mut self) -> &mut Page<T> {
        if let Self::Image(image) = self {
            *self = Self::Page(Box::new(Page::from_image(image)));
        }

        match self {
            Self::Page(page) => page,
            Self::Image(_) => unreachable!(),
        }
    }

    /// Return one mutable cell by slot offset.
    #[inline]
    pub(super) fn get_mut(&mut self, offset: usize) -> Option<&mut T> {
        let page = self.page_mut();
        page.get_mut(offset)
    }

    /// Return one mutable cell by slot offset without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the slot offset is valid and occupied.
    #[inline]
    pub(super) unsafe fn get_unchecked_mut(&mut self, offset: usize) -> &mut T {
        let page = self.page_mut();

        unsafe { page.get_unchecked_mut(offset) }
    }

    /// Store one cell at the given slot offset.
    #[inline]
    pub(super) fn set(&mut self, offset: usize, cell: T) {
        let page = self.page_mut();
        page.set(offset, cell);
    }

    /// Remove one cell from the given slot offset.
    #[inline]
    pub(super) fn take(&mut self, offset: usize) -> Option<T> {
        let page = self.page_mut();
        page.take(offset)
    }
}

impl<T: Clone + Default> PageReference<T> {
    /// Report whether one slot offset is occupied.
    #[inline]
    pub(super) fn is_occupied(&self, offset: usize) -> bool {
        match self {
            Self::Page(page) => page.is_occupied(offset),
            Self::Image(image) => image.is_occupied(offset),
        }
    }

    /// Report whether one slot offset is marked.
    #[inline]
    pub(super) fn is_marked(&self, offset: usize) -> bool {
        match self {
            Self::Page(page) => page.is_marked(offset),
            Self::Image(_) => false,
        }
    }

    /// Mark one slot offset.
    #[inline]
    pub(super) fn mark(&mut self, offset: usize) {
        let page = self.page_mut();
        page.mark(offset);
    }

    /// Clear all page marks.
    #[inline]
    pub(super) fn clear_marks(&mut self) {
        if let Self::Page(page) = self {
            page.clear_marks();
        }
    }

    /// Return one cell by slot offset.
    #[inline]
    pub(super) fn get(&self, offset: usize) -> Option<&T> {
        match self {
            Self::Page(page) => page.get(offset),
            Self::Image(image) => image.get(offset),
        }
    }

    /// Return one cell by slot offset without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the slot offset is valid and occupied.
    #[inline]
    pub(super) unsafe fn get_unchecked(&self, offset: usize) -> &T {
        debug_assert!(self.is_occupied(offset), "page slot is not occupied");

        match self {
            Self::Page(page) => unsafe { page.get_unchecked(offset) },
            Self::Image(image) => unsafe { image.get_unchecked(offset) },
        }
    }
}

impl<T> Page<T> {
    /// Return the page kind.
    #[inline]
    pub fn kind(&self) -> PageKind {
        self.kind
    }

    /// Report whether one slot offset is occupied.
    #[inline]
    pub fn is_occupied(&self, offset: usize) -> bool {
        self.occupied.contains(offset)
    }

    /// Report whether one slot offset is marked.
    #[inline]
    pub fn is_marked(&self, offset: usize) -> bool {
        self.marked.contains(offset)
    }

    /// Mark one slot offset.
    #[inline]
    pub fn mark(&mut self, offset: usize) {
        self.marked.set(offset);
    }

    /// Clear one slot mark.
    #[inline]
    pub fn clear_mark(&mut self, offset: usize) {
        self.marked.clear(offset);
    }

    /// Clear all page marks.
    #[inline]
    pub fn clear_marks(&mut self) {
        self.marked.clear_all();
    }

    /// Return the number of occupied slots in this page.
    #[inline]
    pub fn allocated_slots(&self) -> usize {
        self.occupied.count_ones()
    }

    /// Report whether this page is full.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.allocated_slots() == HEAP_PAGE_CAPACITY
    }

    /// Return one cell by slot offset.
    #[inline]
    pub fn get(&self, offset: usize) -> Option<&T> {
        if !self.is_occupied(offset) {
            return None;
        }
        self.cells.get(offset)
    }

    /// Return one mutable cell by slot offset.
    #[inline]
    pub fn get_mut(&mut self, offset: usize) -> Option<&mut T> {
        if !self.is_occupied(offset) {
            return None;
        }
        self.cells.get_mut(offset)
    }

    /// Return one cell by slot offset without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the slot offset is valid and occupied.
    #[inline]
    pub unsafe fn get_unchecked(&self, offset: usize) -> &T {
        debug_assert!(self.is_occupied(offset), "page slot is not occupied");
        unsafe { self.cells.get_unchecked(offset) }
    }

    /// Return one mutable cell by slot offset without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the slot offset is valid and occupied.
    #[inline]
    pub unsafe fn get_unchecked_mut(&mut self, offset: usize) -> &mut T {
        debug_assert!(self.is_occupied(offset), "page slot is not occupied");
        unsafe { self.cells.get_unchecked_mut(offset) }
    }

    /// Store one cell at the given slot offset.
    #[inline]
    pub fn set(&mut self, offset: usize, cell: T) {
        self.cells[offset] = cell;
        if !self.occupied.contains(offset) {
            self.occupied.set(offset);
        }
    }

    /// Remove one cell from the given slot offset.
    #[inline]
    pub fn take(&mut self, offset: usize) -> Option<T>
    where
        T: Default,
    {
        if !self.is_occupied(offset) {
            return None;
        }

        let value = std::mem::take(&mut self.cells[offset]);
        self.occupied.clear(offset);
        self.marked.clear(offset);

        Some(value)
    }
}
