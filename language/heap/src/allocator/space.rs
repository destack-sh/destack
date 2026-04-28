use super::map::PageMap;
use crate::HeapResult;

/// One forkable virtual address space.
#[derive(Debug)]
pub(crate) struct AddressSpace {
    /// The page map used by direct address access.
    map: PageMap,
}

// mappings are synchronized by their owning space
unsafe impl Send for AddressSpace {}

// mappings are synchronized by their owning space
unsafe impl Sync for AddressSpace {}

impl AddressSpace {
    /// Reserve one virtual address space.
    pub(crate) fn reserve(byte_len: usize, page_bytes: usize) -> HeapResult<Self> {
        let mapping = Self {
            map: PageMap::reserve(byte_len, page_bytes)?,
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
        self.map.byte_len()
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

#[cfg(test)]
mod tests {
    use super::super::platform;
    use super::AddressSpace;

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
