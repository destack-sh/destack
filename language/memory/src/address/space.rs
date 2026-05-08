use std::ops::RangeBounds;
use std::ptr::copy_nonoverlapping;

use super::page::PageMap;
use crate::MemoryResult;

/// One forkable virtual address space.
///
/// Forking and dropping require external synchronization with raw writes into exposed addresses.
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
    pub fn reserve(byte_len: usize, page_bytes: usize) -> MemoryResult<Self> {
        let mapping = Self {
            map: PageMap::reserve(byte_len, page_bytes)?,
        };

        Ok(mapping)
    }

    /// Fork this address space and isolate pages on first write.
    ///
    /// Call this only while no raw writes can race with remapping.
    pub fn fork_lazy(&self) -> MemoryResult<Self> {
        let mapping = Self {
            map: self.map.fork_lazy()?,
        };

        Ok(mapping)
    }

    /// Fork this address space and eagerly isolate mapped pages in one byte range.
    ///
    /// Reserved pages inside the range stay unmaterialized.
    pub fn fork_eager<R>(&self, range: R) -> MemoryResult<Self>
    where
        R: RangeBounds<usize>,
    {
        let mapping = Self {
            map: self.map.fork_eager(range)?,
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
    pub fn zero(&self, offset: usize, byte_len: usize) -> MemoryResult<()> {
        self.map.zero(offset, byte_len)
    }

    /// Read bytes into a caller-provided buffer.
    pub fn read_bytes_into(&self, offset: usize, target: &mut [u8]) -> MemoryResult<()> {
        self.map.read_bytes_into(offset, target)
    }

    /// Return one owned byte vector from this address space.
    pub fn read_bytes(&self, offset: usize, byte_len: usize) -> MemoryResult<Vec<u8>> {
        self.map.read_bytes(offset, byte_len)
    }

    /// Return one checked address inside this address space.
    ///
    /// Writes through the returned pointer may fault once after a lazy fork to make the page private.
    pub fn address(&self, offset: usize, byte_len: usize) -> MemoryResult<*mut u8> {
        self.map.address(offset, byte_len)
    }

    /// Materialize one byte range inside this address space.
    pub fn materialize(&self, offset: usize, byte_len: usize) -> MemoryResult<()> {
        self.map.materialize(offset, byte_len)
    }

    /// Make one byte range writable for an upcoming bulk write.
    pub fn make_writable(&self, offset: usize, byte_len: usize) -> MemoryResult<()> {
        self.map.make_writable(offset, byte_len)
    }

    /// Write caller-provided bytes into this address space.
    pub fn write_bytes(&self, offset: usize, bytes: &[u8]) -> MemoryResult<()> {
        self.map.write_bytes(offset, bytes)
    }

    /// Copy bytes into a range that the caller knows is already mapped.
    ///
    /// # Safety
    ///
    /// The byte range must be live and fully materialized in this address space.
    #[inline(always)]
    pub unsafe fn write_mapped_bytes(&self, offset: usize, bytes: &[u8]) {
        let target = (self.base_address() + offset) as *mut u8;

        // caller owns the mapped range invariant
        unsafe {
            copy_nonoverlapping(bytes.as_ptr(), target, bytes.len());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ptr::copy_nonoverlapping;

    use super::AddressSpace;
    use crate::platform;

    /// Raw pointer writes after fork stay isolated from the parent mapping.
    #[test]
    fn test_fork_preserves_raw_pointer_write_isolation() {
        let frame_bytes = platform::system_frame_bytes().expect("frame size should resolve");
        let parent =
            AddressSpace::reserve(frame_bytes, frame_bytes).expect("address space should reserve");

        // initialize the parent page before forking
        parent
            .write_bytes(0, &[1, 2, 3, 4])
            .expect("parent write should succeed");

        let child = parent
            .fork_lazy()
            .expect("address space fork should succeed");
        let child_address = child.address(0, 4).expect("child address should resolve");

        // write through the raw address instead of the mapping API
        unsafe {
            copy_nonoverlapping([9, 8, 7, 6].as_ptr(), child_address, 4);
        }

        let parent_bytes = parent.read_bytes(0, 4).expect("parent bytes should read");
        let child_bytes = child.read_bytes(0, 4).expect("child bytes should read");

        assert_eq!(parent_bytes, [1, 2, 3, 4]);
        assert_eq!(child_bytes, [9, 8, 7, 6]);
    }

    /// Forking a modified child preserves the child's visible bytes.
    #[test]
    fn test_fork_captures_modified_child_page() {
        let frame_bytes = platform::system_frame_bytes().expect("frame size should resolve");
        let parent =
            AddressSpace::reserve(frame_bytes, frame_bytes).expect("address space should reserve");

        // initialize the parent page before forking
        parent
            .write_bytes(0, &[1, 2, 3, 4])
            .expect("parent write should succeed");

        let child = parent
            .fork_lazy()
            .expect("address space fork should succeed");
        let child_address = child.address(0, 4).expect("child address should resolve");

        // modify the child mapping
        unsafe {
            copy_nonoverlapping([9, 8, 7, 6].as_ptr(), child_address, 4);
        }

        let grandchild = child.fork_lazy().expect("child fork should succeed");
        let grandchild_bytes = grandchild
            .read_bytes(0, 4)
            .expect("grandchild bytes should read");

        assert_eq!(grandchild_bytes, [9, 8, 7, 6]);
    }

    /// Modified re-forks keep later writes isolated.
    #[test]
    fn test_fork_isolates_modified_child_page() {
        let frame_bytes = platform::system_frame_bytes().expect("frame size should resolve");
        let parent =
            AddressSpace::reserve(frame_bytes, frame_bytes).expect("address space should reserve");

        // initialize the parent page before forking
        parent
            .write_bytes(0, &[1, 2, 3, 4])
            .expect("parent write should succeed");

        let child = parent
            .fork_lazy()
            .expect("address space fork should succeed");
        let child_address = child.address(0, 4).expect("child address should resolve");

        // modify the child before re-forking it
        unsafe {
            copy_nonoverlapping([9, 8, 7, 6].as_ptr(), child_address, 4);
        }

        let grandchild = child.fork_lazy().expect("child fork should succeed");
        let grandchild_address = grandchild
            .address(0, 4)
            .expect("grandchild address should resolve");

        // mutate both sides after the modified fork
        unsafe {
            copy_nonoverlapping([2, 2, 2, 2].as_ptr(), child_address, 4);
            copy_nonoverlapping([3, 3, 3, 3].as_ptr(), grandchild_address, 4);
        }

        let child_bytes = child.read_bytes(0, 4).expect("child bytes should read");
        let grandchild_bytes = grandchild
            .read_bytes(0, 4)
            .expect("grandchild bytes should read");

        assert_eq!(child_bytes, [2, 2, 2, 2]);
        assert_eq!(grandchild_bytes, [3, 3, 3, 3]);
    }

    /// Forking a shared child keeps later child and grandchild writes isolated.
    #[test]
    fn test_fork_reuses_shared_child_page() {
        let frame_bytes = platform::system_frame_bytes().expect("frame size should resolve");
        let parent =
            AddressSpace::reserve(frame_bytes, frame_bytes).expect("address space should reserve");

        // initialize the parent page before forking
        parent
            .write_bytes(0, &[1, 2, 3, 4])
            .expect("parent write should succeed");

        let child = parent
            .fork_lazy()
            .expect("address space fork should succeed");
        let grandchild = child.fork_lazy().expect("child fork should succeed");
        let child_address = child.address(0, 4).expect("child address should resolve");
        let grandchild_address = grandchild
            .address(0, 4)
            .expect("grandchild address should resolve");

        // modify both shared mappings independently
        unsafe {
            copy_nonoverlapping([9, 8, 7, 6].as_ptr(), child_address, 4);
            copy_nonoverlapping([4, 3, 2, 1].as_ptr(), grandchild_address, 4);
        }

        let parent_bytes = parent.read_bytes(0, 4).expect("parent bytes should read");
        let child_bytes = child.read_bytes(0, 4).expect("child bytes should read");
        let grandchild_bytes = grandchild
            .read_bytes(0, 4)
            .expect("grandchild bytes should read");

        assert_eq!(parent_bytes, [1, 2, 3, 4]);
        assert_eq!(child_bytes, [9, 8, 7, 6]);
        assert_eq!(grandchild_bytes, [4, 3, 2, 1]);
    }

    /// Raw pointer writes materialize reserved pages before exposing addresses.
    #[test]
    fn test_raw_pointer_write_materializes_reserved_page() {
        let frame_bytes = platform::system_frame_bytes().expect("frame size should resolve");
        let address_space =
            AddressSpace::reserve(frame_bytes, frame_bytes).expect("address space should reserve");
        let address = address_space.address(0, 4).expect("address should resolve");

        // write through the raw address instead of the mapping API
        unsafe {
            copy_nonoverlapping([5, 6, 7, 8].as_ptr(), address, 4);
        }

        let bytes = address_space.read_bytes(0, 4).expect("bytes should read");

        assert_eq!(bytes, [5, 6, 7, 8]);
    }
}
