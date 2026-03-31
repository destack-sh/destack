use std::mem::size_of;
use std::rc::Rc;
use std::slice;

#[cfg(not(unix))]
use std::alloc::{Layout, alloc_zeroed, dealloc};

/// The number of fixed-width pages reserved in one arena block.
const PAGES_PER_BLOCK: usize = 64;

/// One stable local page identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageId(u32);

impl PageId {
    /// Create one page identifier.
    pub const fn new(id: usize) -> Self {
        Self(id as u32)
    }

    /// Return the zero-based page index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// One immutable shared page image.
pub type PageImage = Rc<[u8]>;

/// One stable local page location inside one arena block.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PageSlot {
    /// The arena block containing this page.
    block_index: u32,
    /// The zero-based page index inside that block.
    page_index: u32,
}

/// One owned block of fixed-width local pages.
#[derive(Debug)]
struct PageBlock {
    /// The fixed byte width for every page in this block.
    page_bytes: usize,
    /// The zero-based free page indexes inside this block.
    free_page_indexes: Vec<u32>,
    /// The contiguous page bytes for this block.
    bytes: *mut u8,
    /// The total byte length of this block.
    byte_len: usize,
}

impl PageBlock {
    /// Create one empty page block with the given page width.
    fn with_page_bytes(page_bytes: usize) -> Self {
        let mut free_page_indexes = Vec::with_capacity(PAGES_PER_BLOCK);

        for page_index in (0..PAGES_PER_BLOCK).rev() {
            free_page_indexes.push(page_index as u32);
        }

        Self {
            page_bytes,
            free_page_indexes,
            bytes: allocate_page_block_bytes(page_bytes * PAGES_PER_BLOCK),
            byte_len: page_bytes * PAGES_PER_BLOCK,
        }
    }

    /// Report whether this block still has one free page.
    fn has_free_page(&self) -> bool {
        !self.free_page_indexes.is_empty()
    }

    /// Allocate one page inside this block initialized from the given bytes.
    fn allocate_page(&mut self, bytes: &[u8]) -> u32 {
        let page_index = self
            .free_page_indexes
            .pop()
            .expect("page block allocation requires one free page");
        let page = self
            .page_mut(page_index)
            .expect("allocated block page must stay addressable");

        page.fill(0);
        page[..bytes.len()].copy_from_slice(bytes);

        page_index
    }

    /// Free one page inside this block.
    fn free_page(&mut self, page_index: u32) {
        debug_assert!(
            !self.free_page_indexes.contains(&page_index),
            "page block frees must not duplicate page indexes",
        );
        self.free_page_indexes.push(page_index);
    }

    /// Return one block page as an immutable byte slice.
    fn page(&self, page_index: u32) -> Option<&[u8]> {
        let start = self.page_range_start(page_index)?;
        let end = start.checked_add(self.page_bytes)?;
        if end > self.byte_len {
            return None;
        }

        Some(&self.bytes_slice()[start..end])
    }

    /// Return one block page as a mutable byte slice.
    fn page_mut(&mut self, page_index: u32) -> Option<&mut [u8]> {
        let start = self.page_range_start(page_index)?;
        let end = start.checked_add(self.page_bytes)?;
        if end > self.byte_len {
            return None;
        }

        Some(&mut self.bytes_slice_mut()[start..end])
    }

    /// Return the retained bytes owned by this block.
    fn retained_bytes(&self) -> usize {
        self.free_page_indexes.capacity() * size_of::<u32>()
    }

    /// Return the byte-range start for one page index.
    fn page_range_start(&self, page_index: u32) -> Option<usize> {
        if page_index as usize >= PAGES_PER_BLOCK {
            return None;
        }

        (page_index as usize).checked_mul(self.page_bytes)
    }

    /// Return the whole block as one immutable byte slice.
    fn bytes_slice(&self) -> &[u8] {
        // block storage is allocated for exactly byte_len bytes
        unsafe { slice::from_raw_parts(self.bytes, self.byte_len) }
    }

    /// Return the whole block as one mutable byte slice.
    fn bytes_slice_mut(&mut self) -> &mut [u8] {
        // block storage is allocated for exactly byte_len bytes
        unsafe { slice::from_raw_parts_mut(self.bytes, self.byte_len) }
    }
}

impl Clone for PageBlock {
    fn clone(&self) -> Self {
        let mut clone = Self::with_page_bytes(self.page_bytes);
        clone.free_page_indexes = self.free_page_indexes.clone();
        clone.bytes_slice_mut().copy_from_slice(self.bytes_slice());
        clone
    }
}

impl PartialEq for PageBlock {
    fn eq(&self, other: &Self) -> bool {
        self.page_bytes == other.page_bytes
            && self.free_page_indexes == other.free_page_indexes
            && self.bytes_slice() == other.bytes_slice()
    }
}

impl Eq for PageBlock {}

impl Drop for PageBlock {
    fn drop(&mut self) {
        free_page_block_bytes(self.bytes, self.byte_len);
    }
}

/// One owned arena of fixed-width local pages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageArena {
    /// The fixed byte width for every page.
    page_bytes: usize,
    /// The owned page blocks.
    blocks: Vec<PageBlock>,
    /// The stable local pages keyed by page id.
    pages: Vec<Option<PageSlot>>,
    /// The reusable vacant page ids.
    free_ids: Vec<u32>,
}

/// Allocate one page-aligned block of zeroed bytes.
pub fn allocate_page_block_bytes(byte_len: usize) -> *mut u8 {
    if byte_len == 0 {
        return std::ptr::null_mut();
    }

    #[cfg(unix)]
    {
        // anonymous private mappings give us real page-backed arena blocks
        let bytes = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                byte_len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_ANON | libc::MAP_PRIVATE,
                -1,
                0,
            )
        };
        if bytes == libc::MAP_FAILED {
            panic!("page arena mmap failed for block of {byte_len} bytes");
        }

        bytes.cast()
    }

    #[cfg(not(unix))]
    {
        let layout = Layout::from_size_align(byte_len, 4096)
            .unwrap_or_else(|_| panic!("invalid page arena layout for block of {byte_len} bytes"));
        let bytes = unsafe { alloc_zeroed(layout) };
        if bytes.is_null() {
            panic!("page arena allocation failed for block of {byte_len} bytes");
        }

        bytes
    }
}

/// Free one page-aligned block of bytes.
pub fn free_page_block_bytes(bytes: *mut u8, byte_len: usize) {
    if bytes.is_null() || byte_len == 0 {
        return;
    }

    #[cfg(unix)]
    {
        let status = unsafe { libc::munmap(bytes.cast(), byte_len) };
        debug_assert_eq!(status, 0, "page arena munmap must succeed");
    }

    #[cfg(not(unix))]
    {
        let layout = Layout::from_size_align(byte_len, 4096)
            .unwrap_or_else(|_| panic!("invalid page arena layout for block of {byte_len} bytes"));
        unsafe { dealloc(bytes, layout) };
    }
}

impl PageArena {
    /// Create one empty page arena with the given page width.
    pub fn with_page_bytes(page_bytes: usize) -> Self {
        Self {
            page_bytes: page_bytes.max(1),
            blocks: Vec::new(),
            pages: Vec::new(),
            free_ids: Vec::new(),
        }
    }

    /// Return the fixed byte width for every page.
    pub fn page_bytes(&self) -> usize {
        self.page_bytes
    }

    /// Allocate one page initialized from the given bytes.
    pub fn allocate(&mut self, bytes: &[u8]) -> PageId {
        assert!(
            bytes.len() <= self.page_bytes,
            "page allocation exceeded arena page width"
        );

        let slot = self.allocate_slot(bytes);

        if let Some(id) = self.free_ids.pop() {
            let page_id = PageId(id);
            self.pages[page_id.index()] = Some(slot);
            return page_id;
        }

        let page_id = PageId::new(self.pages.len());
        self.pages.push(Some(slot));
        page_id
    }

    /// Allocate one zeroed page.
    pub fn allocate_zeroed(&mut self) -> PageId {
        self.allocate(&[])
    }

    /// Return one page as an immutable byte slice.
    pub fn page(&self, page_id: PageId) -> Option<&[u8]> {
        let slot = self.pages.get(page_id.index())?.as_ref()?;
        self.blocks
            .get(slot.block_index as usize)?
            .page(slot.page_index)
    }

    /// Return one page as a mutable byte slice.
    pub fn page_mut(&mut self, page_id: PageId) -> Option<&mut [u8]> {
        let slot = self.pages.get(page_id.index())?.as_ref()?.clone();
        self.blocks
            .get_mut(slot.block_index as usize)?
            .page_mut(slot.page_index)
    }

    /// Free one local page while keeping its stable page id reusable.
    pub fn free(&mut self, page_id: PageId) -> bool {
        let Some(page) = self.pages.get_mut(page_id.index()) else {
            return false;
        };

        if page.is_none() {
            return false;
        }

        let slot = page
            .take()
            .expect("local page arena frees require one live page slot");
        let block = self
            .blocks
            .get_mut(slot.block_index as usize)
            .expect("page slot block index must stay addressable");
        block.free_page(slot.page_index);
        self.free_ids.push(page_id.0);
        true
    }

    /// Return one immutable image for the given local page.
    pub fn image(&self, page_id: PageId) -> Option<PageImage> {
        Some(Rc::from(self.page(page_id)?.to_vec().into_boxed_slice()))
    }

    /// Allocate one local page slice initialized from the given bytes.
    pub fn allocate_pages(&mut self, bytes: &[u8], byte_len: usize) -> Vec<PageId> {
        let page_count = byte_len
            .div_ceil(self.page_bytes)
            .max((byte_len > 0) as usize);
        let mut pages = Vec::with_capacity(page_count);

        for page_index in 0..page_count {
            let start = page_index * self.page_bytes;
            let end = (start + self.page_bytes).min(bytes.len());
            let page_bytes = if start < bytes.len() {
                &bytes[start..end]
            } else {
                &[]
            };
            pages.push(self.allocate(page_bytes));
        }

        pages
    }

    /// Free one local page slice.
    pub fn free_pages(&mut self, pages: &[PageId]) {
        for &page_id in pages {
            let freed = self.free(page_id);
            debug_assert!(freed, "page slice pages must stay freeable");
        }
    }

    /// Borrow one contiguous window from a page slice when it fits inside one page.
    pub fn borrow_window<'a>(
        &'a self,
        pages: &'a [PageId],
        start: usize,
        len: usize,
    ) -> Option<&'a [u8]> {
        if len == 0 {
            return Some(&[]);
        }

        let end = start.checked_add(len)?;
        let start_page = start / self.page_bytes;
        let end_page = (end - 1) / self.page_bytes;
        if start_page != end_page {
            return None;
        }

        let page_id = *pages.get(start_page)?;
        let page = self.page(page_id)?;
        let page_offset = start % self.page_bytes;
        page.get(page_offset..page_offset + len)
    }

    /// Copy one byte window out of one page slice.
    pub fn read_window(&self, pages: &[PageId], start: usize, dest: &mut [u8]) -> bool {
        if dest.is_empty() {
            return true;
        }

        let mut copied = 0usize;
        let mut offset = start;

        while copied < dest.len() {
            let page_index = offset / self.page_bytes;
            let page_offset = offset % self.page_bytes;
            let Some(page_id) = pages.get(page_index).copied() else {
                return false;
            };
            let Some(page) = self.page(page_id) else {
                return false;
            };

            let remaining = dest.len() - copied;
            let available = page.len().saturating_sub(page_offset);
            let count = remaining.min(available);
            if count == 0 {
                return false;
            }

            dest[copied..copied + count].copy_from_slice(&page[page_offset..page_offset + count]);
            copied += count;
            offset += count;
        }

        true
    }

    /// Fill one byte window inside one page slice.
    pub fn fill_window(&mut self, pages: &[PageId], start: usize, len: usize, byte: u8) -> bool {
        if len == 0 {
            return true;
        }

        let mut written = 0usize;
        let mut offset = start;

        while written < len {
            let page_index = offset / self.page_bytes;
            let page_offset = offset % self.page_bytes;
            let Some(page_id) = pages.get(page_index).copied() else {
                return false;
            };
            let Some(page) = self.page_mut(page_id) else {
                return false;
            };

            let remaining = len - written;
            let available = page.len().saturating_sub(page_offset);
            let count = remaining.min(available);
            if count == 0 {
                return false;
            }

            page[page_offset..page_offset + count].fill(byte);
            written += count;
            offset += count;
        }

        true
    }

    /// Write one byte window into one page slice.
    pub fn write_window(&mut self, pages: &[PageId], start: usize, bytes: &[u8]) -> bool {
        if bytes.is_empty() {
            return true;
        }

        let mut written = 0usize;
        let mut offset = start;

        while written < bytes.len() {
            let page_index = offset / self.page_bytes;
            let page_offset = offset % self.page_bytes;
            let Some(page_id) = pages.get(page_index).copied() else {
                return false;
            };
            let Some(page) = self.page_mut(page_id) else {
                return false;
            };

            let remaining = bytes.len() - written;
            let available = page.len().saturating_sub(page_offset);
            let count = remaining.min(available);
            if count == 0 {
                return false;
            }

            page[page_offset..page_offset + count]
                .copy_from_slice(&bytes[written..written + count]);
            written += count;
            offset += count;
        }

        true
    }

    /// Return the retained bytes owned by arena metadata.
    pub fn retained_bytes(&self) -> usize {
        let mut retained_bytes = self.blocks.capacity() * size_of::<PageBlock>();
        retained_bytes += self.pages.capacity() * size_of::<Option<PageSlot>>();
        retained_bytes += self.free_ids.capacity() * size_of::<u32>();

        for block in &self.blocks {
            retained_bytes += block.retained_bytes();
        }

        retained_bytes
    }

    /// Allocate one local page slot initialized from the given bytes.
    fn allocate_slot(&mut self, bytes: &[u8]) -> PageSlot {
        if let Some((block_index, block)) = self
            .blocks
            .iter_mut()
            .enumerate()
            .find(|(_, block)| block.has_free_page())
        {
            let page_index = block.allocate_page(bytes);

            return PageSlot {
                block_index: block_index as u32,
                page_index,
            };
        }

        let mut block = PageBlock::with_page_bytes(self.page_bytes);
        let page_index = block.allocate_page(bytes);
        let block_index = self.blocks.len() as u32;
        self.blocks.push(block);

        PageSlot {
            block_index,
            page_index,
        }
    }
}
