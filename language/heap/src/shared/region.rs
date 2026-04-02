use super::image::SharedRegionImage;
use crate::alloc::{ChunkPayload, PageArena};

/// One logical shared-memory region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SharedRegion {
    /// Whether this region id is currently allocated.
    is_allocated: bool,
    /// The shared payload storage.
    payload: ChunkPayload,
}

impl SharedRegion {
    /// Create one allocated shared-memory region from the given bytes.
    pub(crate) fn new(bytes: &[u8], page_bytes: usize, page_arena: &mut PageArena) -> Self {
        Self {
            is_allocated: true,
            payload: ChunkPayload::new(bytes, page_bytes, page_arena),
        }
    }

    /// Restore one shared-memory region from one immutable image.
    pub(crate) fn from_image(image: &SharedRegionImage) -> Self {
        Self {
            is_allocated: image.is_allocated,
            payload: ChunkPayload::from_image(
                image.len,
                image.page_bytes,
                image.chunks.iter().cloned(),
            ),
        }
    }

    /// Return whether this region id is currently allocated.
    pub(crate) fn is_allocated(&self) -> bool {
        self.is_allocated
    }

    /// Return the region length in bytes.
    pub(crate) fn len(&self) -> usize {
        self.payload.len()
    }

    /// Return one owned copy of this region's bytes.
    pub(crate) fn bytes_to_vec(&self, page_arena: &PageArena) -> Vec<u8> {
        self.payload.bytes(page_arena).into_owned()
    }

    /// Return one byte by index.
    pub(crate) fn get(&self, page_arena: &PageArena, index: usize) -> Option<u8> {
        self.payload.get(page_arena, index)
    }

    /// Reinitialize this region as one allocated shared-memory region.
    pub(crate) fn allocate(&mut self, page_arena: &mut PageArena, bytes: &[u8], page_bytes: usize) {
        self.is_allocated = true;
        self.payload.replace(page_arena, bytes, page_bytes);
    }

    /// Mark this region as freed while keeping its stable slot alive.
    pub(crate) fn free(&mut self, page_arena: &mut PageArena) {
        self.is_allocated = false;
        self.payload.clear(page_arena);
    }

    /// Write one byte at the given index.
    pub(crate) fn set(&mut self, page_arena: &mut PageArena, index: usize, byte: u8) -> bool {
        self.payload.set(page_arena, index, byte)
    }

    /// Return the detached-page count and active-byte reservation for one write.
    pub(crate) fn write_active_reservation(&self, index: usize, len: usize) -> (usize, i64) {
        self.payload.write_page_reservation(index, len)
    }

    /// Replace the entire byte payload.
    pub(crate) fn replace(&mut self, page_arena: &mut PageArena, bytes: &[u8], page_bytes: usize) {
        self.payload.replace(page_arena, bytes, page_bytes);
    }

    /// Return the active-byte reservation for replacing this payload.
    pub(crate) fn replace_active_reservation(&self, new_len: usize) -> (usize, usize, i64) {
        self.payload.replace_page_reservation(new_len)
    }

    /// Return one immutable image for this region.
    pub(crate) fn image(
        &mut self,
        page_arena: &mut PageArena,
        base: Option<&SharedRegionImage>,
    ) -> SharedRegionImage {
        let _ = base;
        let chunks = self.payload.image(page_arena);

        SharedRegionImage {
            is_allocated: self.is_allocated,
            len: self.payload.len(),
            page_bytes: self.payload.page_bytes(),
            chunks,
        }
    }

    /// Return the active local bytes owned by this region.
    pub(crate) fn active_bytes(&self) -> usize {
        self.payload.active_bytes()
    }

    /// Return the borrowed image bytes referenced by this region.
    pub(crate) fn borrowed_bytes(&self) -> usize {
        self.payload.borrowed_bytes()
    }

    /// Return the active local bytes for one region payload length.
    pub(crate) fn active_bytes_for_len(len: usize, page_bytes: usize) -> usize {
        ChunkPayload::active_bytes_for_len(len, page_bytes)
    }
}
