use std::sync::Arc;

use destack_mir::LayoutId;
use serde::{Deserialize, Serialize};

use super::class::{MANAGED_PAGE_BITMAP_WORDS, MANAGED_PAGE_CAPACITY, PageBitmap};
use super::image::PageImage;
use super::live::Page;
use crate::ManagedAllocation;

#[derive(Debug, Clone, PartialEq, Eq)]
enum LayoutIdStorage {
    /// Owned mutable layout metadata local to one live heap.
    Owned(Box<[Option<LayoutId>]>),
    /// Immutable layout metadata shared through forked images.
    Image(Arc<[Option<LayoutId>]>),
}

/// One immutable managed page image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedPageImage {
    /// The immutable managed payload page image.
    pub page: Arc<PageImage<ManagedAllocation, MANAGED_PAGE_CAPACITY, MANAGED_PAGE_BITMAP_WORDS>>,
    /// The durable layout ids for occupied page slots.
    pub layout_ids: Arc<[Option<LayoutId>]>,
}

impl ManagedPageImage {
    /// Report whether this page image shares durable backing with another page image.
    pub fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.page, &other.page) && Arc::ptr_eq(&self.layout_ids, &other.layout_ids)
    }
}

/// One live managed page with page-local side metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManagedPage {
    /// The managed payload page storage.
    storage: Page<ManagedAllocation, MANAGED_PAGE_CAPACITY, MANAGED_PAGE_BITMAP_WORDS>,
    /// The live mark bitmap keyed by page slot.
    marked: PageBitmap<MANAGED_PAGE_CAPACITY, MANAGED_PAGE_BITMAP_WORDS>,
    /// Durable layout ids keyed by page slot.
    layout_ids: LayoutIdStorage,
    /// Cached immutable image for this page when the durable leaf is unchanged.
    image: Option<ManagedPageImage>,
    /// Live pin counts keyed by page slot.
    pin_counts: Box<[u32]>,
    /// The number of active pins in this page.
    active_pins: usize,
}

impl ManagedPage {
    /// Create one empty managed page.
    pub(crate) fn new() -> Self {
        Self {
            storage: Page::new(),
            marked: PageBitmap::new(),
            layout_ids: LayoutIdStorage::Owned(Self::layout_id_defaults()),
            image: None,
            pin_counts: Self::pin_defaults(),
            active_pins: 0,
        }
    }

    /// Restore one managed page from one immutable image.
    pub(crate) fn from_image(image: ManagedPageImage) -> Self {
        Self {
            storage: Page::from_image(image.page.clone()),
            marked: PageBitmap::new(),
            layout_ids: LayoutIdStorage::Image(image.layout_ids.clone()),
            image: Some(image),
            pin_counts: Self::pin_defaults(),
            active_pins: 0,
        }
    }

    /// Capture one immutable managed page image and share it with future forks.
    pub(crate) fn image(&mut self) -> ManagedPageImage {
        if let Some(image) = &self.image {
            return image.clone();
        }

        let page = self.storage.image();
        let layout_ids = self.freeze_layouts();
        let image = ManagedPageImage { page, layout_ids };
        self.image = Some(image.clone());
        image
    }

    /// Report whether one page offset is occupied.
    #[inline]
    pub(crate) fn is_occupied(&self, offset: usize) -> bool {
        self.storage.is_occupied(offset)
    }

    /// Report whether one page offset is marked.
    #[inline]
    pub(crate) fn is_marked(&self, offset: usize) -> bool {
        self.marked.contains(offset)
    }

    /// Mark one page offset.
    #[inline]
    pub(crate) fn mark(&mut self, offset: usize) {
        self.marked.set(offset);
    }

    /// Clear all page marks.
    #[inline]
    pub(crate) fn clear_marks(&mut self) {
        self.marked.clear_all();
    }

    /// Return one entry by page offset.
    #[inline]
    pub(crate) fn get(&self, offset: usize) -> Option<&ManagedAllocation> {
        self.storage.get(offset)
    }

    /// Return one mutable entry by page offset.
    #[inline]
    pub(crate) fn get_mut(&mut self, offset: usize) -> Option<&mut ManagedAllocation> {
        self.image = None;
        self.storage.get_mut(offset)
    }

    /// Return one entry by page offset without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the page offset is valid and occupied.
    #[inline]
    pub(crate) unsafe fn get_unchecked(&self, offset: usize) -> &ManagedAllocation {
        unsafe { self.storage.get_unchecked(offset) }
    }

    /// Return one mutable entry by page offset without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the page offset is valid and occupied.
    #[inline]
    pub(crate) unsafe fn get_unchecked_mut(&mut self, offset: usize) -> &mut ManagedAllocation {
        self.image = None;
        unsafe { self.storage.get_unchecked_mut(offset) }
    }

    /// Store one entry at the given page offset.
    #[inline]
    pub(crate) fn set(&mut self, offset: usize, entry: ManagedAllocation) {
        self.image = None;
        self.storage.set(offset, entry);
    }

    /// Remove one entry from the given page offset.
    #[inline]
    pub(crate) fn take(&mut self, offset: usize) -> Option<ManagedAllocation> {
        self.image = None;
        let value = self.storage.take(offset);

        // clear side metadata when the slot becomes free
        if value.is_some() {
            self.marked.clear(offset);
            self.clear_layout_id(offset);
            self.clear_pin_state(offset);
        }

        value
    }

    /// Return the layout id for one occupied page slot.
    #[inline]
    pub(crate) fn layout_id(&self, offset: usize) -> Option<LayoutId> {
        self.layout_ids().get(offset).copied().flatten()
    }

    /// Set the layout id for one page slot.
    #[inline]
    pub(crate) fn set_layout_id(&mut self, offset: usize, layout_id: LayoutId) {
        let layout_ids = self.layout_ids_mut();
        layout_ids[offset] = Some(layout_id);
    }

    /// Report whether one page slot is currently pinned.
    #[inline]
    pub(crate) fn is_pinned(&self, offset: usize) -> bool {
        self.pin_counts.get(offset).copied().unwrap_or_default() > 0
    }

    /// Return the active pin count for this page.
    #[inline]
    pub(crate) fn active_pins(&self) -> usize {
        self.active_pins
    }

    /// Increment the pin count for one occupied page slot.
    #[inline]
    pub(crate) fn pin(&mut self, offset: usize) -> bool {
        if !self.is_occupied(offset) {
            return false;
        }

        let pin_count = &mut self.pin_counts[offset];
        *pin_count = pin_count.saturating_add(1);
        self.active_pins = self.active_pins.saturating_add(1);
        true
    }

    /// Decrement the pin count for one occupied page slot.
    #[inline]
    pub(crate) fn unpin(&mut self, offset: usize) -> bool {
        if !self.is_occupied(offset) {
            return false;
        }

        let pin_count = &mut self.pin_counts[offset];
        if *pin_count == 0 {
            return false;
        }

        *pin_count -= 1;
        self.active_pins -= 1;
        true
    }

    // freeze layout metadata into shared immutable backing
    fn freeze_layouts(&mut self) -> Arc<[Option<LayoutId>]> {
        if let LayoutIdStorage::Owned(layouts) = &mut self.layout_ids {
            let layouts = std::mem::take(layouts);
            self.layout_ids = LayoutIdStorage::Image(Arc::from(layouts));
        }

        match &self.layout_ids {
            LayoutIdStorage::Owned(_) => unreachable!(),
            LayoutIdStorage::Image(layouts) => layouts.clone(),
        }
    }

    // return the durable layout metadata
    fn layout_ids(&self) -> &[Option<LayoutId>] {
        match &self.layout_ids {
            LayoutIdStorage::Owned(layouts) => layouts,
            LayoutIdStorage::Image(layouts) => layouts,
        }
    }

    // return mutable durable layout metadata, detaching shared backing on first write
    fn layout_ids_mut(&mut self) -> &mut [Option<LayoutId>] {
        self.image = None;
        if let LayoutIdStorage::Image(layouts) = &self.layout_ids {
            self.layout_ids = LayoutIdStorage::Owned(layouts.as_ref().to_vec().into());
        }

        match &mut self.layout_ids {
            LayoutIdStorage::Owned(layouts) => layouts,
            LayoutIdStorage::Image(_) => unreachable!(),
        }
    }

    /// Return the retained heap bytes owned by this managed page outside its inline storage.
    pub(crate) fn retained_bytes(&self) -> usize {
        let mut retained_bytes = self.storage.retained_bytes();
        retained_bytes += std::mem::size_of_val(self.pin_counts.as_ref());

        retained_bytes += match &self.layout_ids {
            LayoutIdStorage::Owned(layouts) => std::mem::size_of_val(layouts.as_ref()),
            LayoutIdStorage::Image(layouts) => std::mem::size_of_val(layouts.as_ref()),
        };

        retained_bytes
    }

    // clear one layout-id slot
    fn clear_layout_id(&mut self, offset: usize) {
        let layout_ids = self.layout_ids_mut();
        layout_ids[offset] = None;
    }

    // clear one pin-count slot
    fn clear_pin_state(&mut self, offset: usize) {
        let pin_count = &mut self.pin_counts[offset];
        self.active_pins -= *pin_count as usize;
        *pin_count = 0;
    }

    // build default layout metadata for one page
    fn layout_id_defaults() -> Box<[Option<LayoutId>]> {
        vec![None; MANAGED_PAGE_CAPACITY].into_boxed_slice()
    }

    // build default pin-count metadata for one page
    fn pin_defaults() -> Box<[u32]> {
        vec![0; MANAGED_PAGE_CAPACITY].into_boxed_slice()
    }
}
