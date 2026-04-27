use std::collections::BTreeMap;
use std::sync::Arc;

use parking_lot::Mutex;

use super::platform::{self, PageFrame, PageStoreHandle, VirtualSpace};
use crate::{HeapError, HeapResult};

/// The shared page store.
#[derive(Debug)]
struct PageStore {
    /// The platform page-store handle.
    handle: PageStoreHandle,
}

impl PageStore {
    /// Create one empty page store.
    fn new(byte_len: usize) -> HeapResult<Self> {
        Ok(Self {
            handle: platform::create_page_store(byte_len)?,
        })
    }

    /// Allocate one zeroed page frame.
    fn allocate_frame(&self, frame_bytes: usize) -> HeapResult<PageFrame> {
        platform::allocate_frame(&self.handle, frame_bytes)
    }

    /// Copy one visible page into a new page frame.
    fn copy_frame(&self, source: *mut u8, frame_bytes: usize) -> HeapResult<PageFrame> {
        platform::copy_page(&self.handle, source, frame_bytes)
    }
}

/// One forkable virtual address space.
#[derive(Debug)]
pub(crate) struct AddressSpace {
    /// The page map used by direct address access.
    map: PageMap,
}

/// The mapped pages for one forkable heap space.
#[derive(Debug)]
pub(super) struct PageMap {
    /// The reserved virtual byte space.
    space: VirtualSpace,
    /// The reserved byte length.
    byte_len: usize,
    /// The fixed platform page-frame width.
    frame_bytes: usize,
    /// The shared page-frame store.
    page_store: Arc<PageStore>,
    /// The materialized page frames keyed by page index.
    pages: Mutex<BTreeMap<usize, PageFrame>>,
}

// mappings are synchronized by their owning space
unsafe impl Send for AddressSpace {}

// mappings are synchronized by their owning space
unsafe impl Sync for AddressSpace {}

impl AddressSpace {
    /// Reserve one virtual address space.
    pub(crate) fn reserve(byte_len: usize, page_bytes: usize) -> HeapResult<Self> {
        // use the larger of heap pages and platform pages
        let frame_bytes = page_bytes.max(platform::system_page_bytes()?);
        let page_count = byte_len.div_ceil(frame_bytes);

        // round reservation size to whole frames
        let byte_len = page_count
            .checked_mul(frame_bytes)
            .ok_or(HeapError::InvariantOverflow {
                context: "space mapping byte length",
            })?;

        // reserve virtual space and its shared page store
        let page_store = Arc::new(PageStore::new(byte_len)?);
        let space = platform::reserve_virtual_space(byte_len)?;

        let mapping = Self {
            map: PageMap {
                space,
                byte_len,
                frame_bytes,
                page_store,
                pages: Mutex::new(BTreeMap::new()),
            },
        };

        Ok(mapping)
    }

    /// Fork this address space with page-granular isolation.
    pub(crate) fn fork(&mut self) -> HeapResult<Self> {
        let mapping = Self {
            map: self.map.fork()?,
        };

        Ok(mapping)
    }

    /// Return the reserved virtual byte length.
    pub(crate) const fn byte_len(&self) -> usize {
        self.map.byte_len
    }

    /// Zero one byte range inside this address space.
    pub(crate) fn zero(&self, offset: usize, byte_len: usize) -> HeapResult<()> {
        self.map.zero(offset, byte_len)
    }

    /// Fill one caller-provided buffer from this address space.
    pub(crate) fn read(&self, offset: usize, target: &mut [u8]) -> HeapResult<()> {
        self.map.read(offset, target)
    }

    /// Return one owned byte vector from this address space.
    pub(crate) fn bytes(&self, offset: usize, byte_len: usize) -> HeapResult<Vec<u8>> {
        self.map.bytes(offset, byte_len)
    }

    /// Return one checked address inside this address space.
    pub(crate) fn address(&self, offset: usize, byte_len: usize) -> HeapResult<*mut u8> {
        self.map.address(offset, byte_len)
    }

    /// Write caller-provided bytes directly into this address space.
    pub(crate) fn write(&self, offset: usize, bytes: &[u8]) -> HeapResult<()> {
        self.map.write(offset, bytes)
    }
}

impl PageMap {
    /// Fork this page map with page-granular isolation.
    fn fork(&self) -> HeapResult<Self> {
        // reserve the child address range first
        let space = platform::reserve_virtual_space(self.byte_len)?;
        let fork = Self {
            space,
            byte_len: self.byte_len,
            frame_bytes: self.frame_bytes,
            page_store: self.page_store.clone(),
            pages: Mutex::new(BTreeMap::new()),
        };

        // native mappings use kernel copy-on-write after fork
        if platform::SUPPORTS_SHARED_PAGE_FRAMES {
            self.fork_shared_frames(&fork)?;

            return Ok(fork);
        }

        // wasm has linear memory, so each fork receives freshly copied pages
        self.fork_copied_frames(&fork)?;

        Ok(fork)
    }

    /// Fork this page map by sharing page frames.
    fn fork_shared_frames(&self, fork: &Self) -> HeapResult<()> {
        let mut pages = self.pages.lock();
        let mut fork_pages = fork.pages.lock();

        for (page_index, page) in &mut *pages {
            // copy the parent-visible bytes into a shareable frame
            let source = unsafe { self.space.base().add(*page_index * self.frame_bytes) };
            let frame = self.page_store.copy_frame(source, self.frame_bytes)?;

            // replace the parent page first so parent metadata stays coherent
            platform::map_page(
                self.space.base(),
                *page_index,
                self.frame_bytes,
                &self.page_store.handle,
                frame,
            )?;
            *page = frame;

            // map the child to the same frame with kernel copy-on-write
            platform::map_page(
                fork.space.base(),
                *page_index,
                self.frame_bytes,
                &self.page_store.handle,
                frame,
            )?;
            fork_pages.insert(*page_index, frame);
        }

        Ok(())
    }

    /// Fork this page map by copying materialized pages.
    fn fork_copied_frames(&self, fork: &Self) -> HeapResult<()> {
        let pages = self.pages.lock();
        let mut fork_pages = fork.pages.lock();

        for page_index in pages.keys() {
            // wasm has no separate virtual memory mappings
            let source = unsafe { self.space.base().add(*page_index * self.frame_bytes) };
            let frame = self.page_store.copy_frame(source, self.frame_bytes)?;

            // copy the frame into the child linear memory
            platform::map_page(
                fork.space.base(),
                *page_index,
                self.frame_bytes,
                &self.page_store.handle,
                frame,
            )?;

            fork_pages.insert(*page_index, frame);
        }

        Ok(())
    }

    /// Zero one byte range inside this address space.
    pub(crate) fn zero(&self, offset: usize, byte_len: usize) -> HeapResult<()> {
        let target = self.address(offset, byte_len)?;

        // nothing to clear
        if byte_len == 0 {
            return Ok(());
        }

        // clear the validated mapped range
        unsafe {
            std::ptr::write_bytes(target, 0, byte_len);
        }

        Ok(())
    }

    /// Fill one caller-provided buffer from this address space.
    pub(crate) fn read(&self, offset: usize, target: &mut [u8]) -> HeapResult<()> {
        let (first_frame, end_frame) = self.frame_range(offset, target.len())?;

        // empty reads only validate the range
        if target.is_empty() {
            return Ok(());
        }

        let mut written = 0;
        let pages = self.pages.lock();

        // copy page by page so sparse pages read back as zero
        for page_index in first_frame..end_frame {
            let page_start = page_index * self.frame_bytes;
            let page_end = page_start + self.frame_bytes;
            let copy_start = offset.max(page_start);
            let copy_end = (offset + target.len()).min(page_end);
            let copy_len = copy_end - copy_start;
            let target_start = copy_start - offset;

            // materialized pages read from their mapping
            if pages.contains_key(&page_index) {
                let source = unsafe { self.space.base().add(copy_start) };
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        source,
                        target[target_start..].as_mut_ptr(),
                        copy_len,
                    );
                }

                written += copy_len;
                continue;
            }

            // sparse pages are logically zero
            target[target_start..target_start + copy_len].fill(0);
            written += copy_len;
        }

        debug_assert_eq!(written, target.len());

        Ok(())
    }

    /// Return one owned byte vector from this address space.
    pub(crate) fn bytes(&self, offset: usize, byte_len: usize) -> HeapResult<Vec<u8>> {
        let mut bytes = vec![0; byte_len];

        // fill the owned buffer from the mapped range
        self.read(offset, &mut bytes)?;

        Ok(bytes)
    }

    /// Return one checked address inside this address space.
    pub(crate) fn address(&self, offset: usize, byte_len: usize) -> HeapResult<*mut u8> {
        let (first_frame, end_frame) = self.frame_range(offset, byte_len)?;

        // materialize sparse pages before exposing a raw address
        for page_index in first_frame..end_frame {
            self.materialize_page(page_index)?;
        }

        Ok(unsafe { self.space.base().add(offset) })
    }

    /// Write caller-provided bytes directly into this address space.
    pub(crate) fn write(&self, offset: usize, bytes: &[u8]) -> HeapResult<()> {
        let target = self.address(offset, bytes.len())?;

        // copy into the validated mapped range
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), target, bytes.len());
        }

        Ok(())
    }

    /// Materialize one private writable page frame.
    fn materialize_page(&self, page_index: usize) -> HeapResult<()> {
        let mut pages = self.pages.lock();

        // already materialized
        if pages.contains_key(&page_index) {
            return Ok(());
        }

        // allocate one private frame and map it into this address space
        let frame = self.page_store.allocate_frame(self.frame_bytes)?;
        platform::map_page(
            self.space.base(),
            page_index,
            self.frame_bytes,
            &self.page_store.handle,
            frame,
        )?;

        pages.insert(page_index, frame);

        Ok(())
    }

    /// Return the half-open page-frame range touched by one byte range.
    fn frame_range(&self, offset: usize, byte_len: usize) -> HeapResult<(usize, usize)> {
        // reject ranges that start outside the reservation
        if offset > self.byte_len {
            return Err(HeapError::InvalidByteRange {
                start: offset,
                len: byte_len,
                capacity: self.byte_len,
            });
        }

        // reject ranges that extend past the reservation
        let remaining = self.byte_len - offset;
        if byte_len > remaining {
            return Err(HeapError::InvalidByteRange {
                start: offset,
                len: byte_len,
                capacity: self.byte_len,
            });
        }

        let end = offset + byte_len;
        let first_frame = offset / self.frame_bytes;
        let end_frame = end.div_ceil(self.frame_bytes);

        Ok((first_frame, end_frame))
    }
}

impl Drop for PageMap {
    fn drop(&mut self) {
        let pages = self.pages.lock();
        self.space.unmap(self.frame_bytes, pages.keys().copied());
    }
}

#[cfg(test)]
mod tests {
    use super::{AddressSpace, platform};

    /// Raw pointer writes after fork stay isolated from the parent mapping.
    #[test]
    fn test_fork_preserves_raw_pointer_write_isolation() {
        let frame_bytes = platform::system_page_bytes().expect("page size should resolve");
        let mut parent =
            AddressSpace::reserve(frame_bytes, frame_bytes).expect("address space should reserve");

        // initialize the parent page before forking
        parent
            .write(0, &[1, 2, 3, 4])
            .expect("parent write should succeed");

        let child = parent.fork().expect("address space fork should succeed");
        let child_address = child.address(0, 4).expect("child address should resolve");

        // write through the raw address instead of the mapping API
        unsafe {
            std::ptr::copy_nonoverlapping([9, 8, 7, 6].as_ptr(), child_address, 4);
        }

        let parent_bytes = parent.bytes(0, 4).expect("parent bytes should read");
        let child_bytes = child.bytes(0, 4).expect("child bytes should read");

        assert_eq!(parent_bytes, [1, 2, 3, 4]);
        assert_eq!(child_bytes, [9, 8, 7, 6]);
    }

    /// Raw pointer writes materialize sparse pages before exposing addresses.
    #[test]
    fn test_raw_pointer_write_materializes_sparse_page() {
        let frame_bytes = platform::system_page_bytes().expect("page size should resolve");
        let address_space =
            AddressSpace::reserve(frame_bytes, frame_bytes).expect("address space should reserve");
        let address = address_space.address(0, 4).expect("address should resolve");

        // write through the raw address instead of the mapping API
        unsafe {
            std::ptr::copy_nonoverlapping([5, 6, 7, 8].as_ptr(), address, 4);
        }

        let bytes = address_space.bytes(0, 4).expect("bytes should read");

        assert_eq!(bytes, [5, 6, 7, 8]);
    }
}
