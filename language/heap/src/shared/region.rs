use super::super::TreeVector;
use super::chunk::SharedChunk;
use super::image::SharedRegionImage;

/// One logical shared-memory region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SharedRegion {
    /// Whether this region id is currently allocated.
    is_allocated: bool,
    /// The logical byte length of this region.
    len: usize,
    /// The chunk width for this region.
    chunk_bytes: usize,
    /// The chunk-backed storage for this region.
    chunks: Vec<SharedChunk>,
}

impl SharedRegion {
    /// Create one allocated shared-memory region from the given bytes.
    pub(crate) fn new(bytes: &[u8], chunk_bytes: usize) -> Self {
        Self {
            is_allocated: true,
            len: bytes.len(),
            chunk_bytes,
            chunks: Self::chunk_storage(bytes, chunk_bytes),
        }
    }

    /// Restore one shared-memory region from one immutable image.
    pub(crate) fn from_image(image: &SharedRegionImage) -> Self {
        Self {
            is_allocated: image.is_allocated,
            len: image.len,
            chunk_bytes: image.chunk_bytes,
            chunks: image
                .chunks
                .iter()
                .cloned()
                .map(SharedChunk::from_image)
                .collect(),
        }
    }

    /// Return whether this region id is currently allocated.
    pub(crate) fn is_allocated(&self) -> bool {
        self.is_allocated
    }

    /// Return the region length in bytes.
    pub(crate) fn len(&self) -> usize {
        self.len
    }

    /// Return one owned copy of this region's bytes.
    pub(crate) fn bytes_to_vec(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.len);

        for chunk in &self.chunks {
            bytes.extend_from_slice(chunk.as_slice());
        }

        bytes.truncate(self.len);
        bytes
    }

    /// Return one byte by index.
    pub(crate) fn get(&self, index: usize) -> Option<u8> {
        if index >= self.len {
            return None;
        }

        let chunk_index = index / self.chunk_bytes;
        let chunk_offset = index % self.chunk_bytes;
        self.chunks.get(chunk_index)?.get(chunk_offset)
    }

    /// Reinitialize this region as one allocated shared-memory region.
    pub(crate) fn allocate(&mut self, bytes: &[u8], chunk_bytes: usize) {
        self.is_allocated = true;
        self.len = bytes.len();
        self.chunk_bytes = chunk_bytes;
        self.chunks = Self::chunk_storage(bytes, chunk_bytes);
    }

    /// Mark this region as freed while keeping its stable slot alive.
    pub(crate) fn free(&mut self) {
        self.is_allocated = false;
        self.len = 0;
        self.chunk_bytes = 0;
        self.chunks.clear();
    }

    /// Write one byte at the given index.
    pub(crate) fn set(&mut self, index: usize, byte: u8) -> bool {
        if index >= self.len {
            return false;
        }

        let chunk_index = index / self.chunk_bytes;
        let chunk_offset = index % self.chunk_bytes;
        let Some(chunk) = self.chunks.get_mut(chunk_index) else {
            return false;
        };

        chunk.set(chunk_offset, byte)
    }

    /// Replace the entire byte payload.
    pub(crate) fn replace(&mut self, bytes: &[u8], chunk_bytes: usize) {
        self.len = bytes.len();
        self.chunk_bytes = chunk_bytes;
        self.chunks = Self::chunk_storage(bytes, chunk_bytes);
    }

    /// Return one immutable image for this region.
    pub(crate) fn image(&mut self, base: Option<&SharedRegionImage>) -> SharedRegionImage {
        // capture chunk leaves
        let chunk_images = self
            .chunks
            .iter_mut()
            .map(SharedChunk::image)
            .collect::<Vec<_>>();

        // reuse unchanged image leaves
        let chunks = TreeVector::from_shared(&chunk_images, base.map(|image| &image.chunks));

        SharedRegionImage {
            is_allocated: self.is_allocated,
            len: self.len,
            chunk_bytes: self.chunk_bytes,
            chunks,
        }
    }

    /// Return the retained heap bytes owned by this region.
    pub(crate) fn retained_bytes(&self) -> usize {
        let mut retained_bytes = self.chunks.capacity() * std::mem::size_of::<SharedChunk>();

        for chunk in &self.chunks {
            retained_bytes += chunk.retained_bytes();
        }

        retained_bytes
    }

    /// Return the retained heap bytes for one region payload length.
    pub(crate) fn retained_bytes_for_len(len: usize, chunk_bytes: usize) -> usize {
        let chunk_count = if len == 0 {
            0
        } else {
            len.div_ceil(chunk_bytes)
        };
        chunk_count * std::mem::size_of::<SharedChunk>() + len
    }

    // split one byte slice into chunk-backed storage
    fn chunk_storage(bytes: &[u8], chunk_bytes: usize) -> Vec<SharedChunk> {
        bytes.chunks(chunk_bytes).map(SharedChunk::new).collect()
    }
}
