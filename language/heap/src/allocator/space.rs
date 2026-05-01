use super::map::PageMap;
use crate::HeapResult;

/// One forkable virtual address space.
#[derive(Debug)]
pub struct AddressSpace {
    /// The page map used by direct address access.
    map: PageMap,
}

// mappings are synchronized by their owning space
unsafe impl Send for AddressSpace {}

// mappings are synchronized by their owning space
unsafe impl Sync for AddressSpace {}

impl AddressSpace {
    /// Reserve one virtual address space.
    pub fn reserve(byte_len: usize, page_bytes: usize) -> HeapResult<Self> {
        let mapping = Self {
            map: PageMap::reserve(byte_len, page_bytes)?,
        };

        Ok(mapping)
    }

    /// Fork this address space with page-granular isolation.
    ///
    /// Call this only while no raw writes can race with remapping.
    pub fn fork(&self) -> HeapResult<Self> {
        let mapping = Self {
            map: self.map.fork()?,
        };

        Ok(mapping)
    }

    /// Return the reserved virtual byte length.
    pub const fn byte_len(&self) -> usize {
        self.map.byte_len()
    }

    /// Return the native page-frame width used by this address space.
    pub const fn frame_bytes(&self) -> usize {
        self.map.frame_bytes()
    }

    /// Return the base native address for this address space.
    pub fn base_address(&self) -> usize {
        self.map.base_address()
    }

    /// Zero one byte range inside this address space.
    pub fn zero(&self, offset: usize, byte_len: usize) -> HeapResult<()> {
        self.map.zero(offset, byte_len)
    }

    /// Fill one caller-provided buffer from this address space.
    pub fn read(&self, offset: usize, target: &mut [u8]) -> HeapResult<()> {
        self.map.read(offset, target)
    }

    /// Return one owned byte vector from this address space.
    pub fn bytes(&self, offset: usize, byte_len: usize) -> HeapResult<Vec<u8>> {
        self.map.bytes(offset, byte_len)
    }

    /// Return one checked address inside this address space.
    pub fn address(&self, offset: usize, byte_len: usize) -> HeapResult<*mut u8> {
        self.map.address(offset, byte_len)
    }

    /// Materialize one byte range inside this address space.
    pub fn materialize(&self, offset: usize, byte_len: usize) -> HeapResult<()> {
        self.map.materialize(offset, byte_len)
    }

    /// Write caller-provided bytes directly into this address space.
    pub fn write(&self, offset: usize, bytes: &[u8]) -> HeapResult<()> {
        self.map.write(offset, bytes)
    }

    /// Write bytes into a range that the caller knows is already mapped.
    #[inline(always)]
    pub(crate) unsafe fn write_mapped(&self, offset: usize, bytes: &[u8]) {
        let target = (self.base_address() + offset) as *mut u8;

        // caller owns the mapped range invariant
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), target, bytes.len());
        }
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
        let parent =
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

    /// Forking a dirty child preserves the child's visible bytes.
    #[test]
    fn test_fork_captures_dirty_child_page() {
        let frame_bytes = platform::system_page_bytes().expect("page size should resolve");
        let parent =
            AddressSpace::reserve(frame_bytes, frame_bytes).expect("address space should reserve");

        // initialize the parent page before forking
        parent
            .write(0, &[1, 2, 3, 4])
            .expect("parent write should succeed");

        let child = parent.fork().expect("address space fork should succeed");
        let child_address = child.address(0, 4).expect("child address should resolve");

        // dirty the private child mapping
        unsafe {
            std::ptr::copy_nonoverlapping([9, 8, 7, 6].as_ptr(), child_address, 4);
        }

        let grandchild = child.fork().expect("child fork should succeed");
        let grandchild_bytes = grandchild
            .bytes(0, 4)
            .expect("grandchild bytes should read");

        assert_eq!(grandchild_bytes, [9, 8, 7, 6]);
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
