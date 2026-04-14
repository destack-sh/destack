use super::SharedSpace;
use super::region::SharedRegion;
use crate::SharedPointer;

impl SharedSpace {
    /// Return the bytes for one shared region pointer.
    pub fn bytes_to_vec(&self, pointer: SharedPointer) -> Option<Vec<u8>> {
        // resolve the live region first
        let region = self.region(pointer)?;
        let byte_offset = pointer.byte_offset();
        let bytes = self.arena.bytes_to_vec(&region.pages, region.len);

        // then apply the requested logical byte offset
        Some(bytes.into_iter().skip(byte_offset).collect())
    }

    /// Replace the bytes for one shared region pointer.
    pub fn replace_bytes(&mut self, pointer: SharedPointer, bytes: &[u8]) -> bool {
        // resolve the live region and shared arena first
        let arena = self.arena.clone();
        let Some(region) = self.region_mut(pointer) else {
            return false;
        };

        // rebuild the shared page map around the new payload
        let pages = region.pages.clone();
        arena.release_pages(&pages);
        region.pages = arena.allocate_bytes(bytes);

        // charge the new live byte count
        let previous_len = region.len as u64;
        region.len = bytes.len();
        self.allocated_bytes = self
            .allocated_bytes
            .saturating_sub(previous_len)
            .saturating_add(bytes.len() as u64);

        true
    }

    /// Return one allocated shared region by pointer.
    fn region(&self, pointer: SharedPointer) -> Option<&SharedRegion> {
        // resolve the dense region slot first
        let index = pointer.id().checked_sub(1)? as usize;
        let region = self.regions.get(index)?;

        // skip free region entries
        if region.is_allocated {
            Some(region)
        } else {
            None
        }
    }

    /// Return one allocated shared region mutably by pointer.
    fn region_mut(&mut self, pointer: SharedPointer) -> Option<&mut SharedRegion> {
        // resolve the dense region slot first
        let index = pointer.id().checked_sub(1)? as usize;
        let region = self.regions.get_mut(index)?;

        // skip free region entries
        if region.is_allocated {
            Some(region)
        } else {
            None
        }
    }
}
