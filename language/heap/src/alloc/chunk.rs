use std::borrow::Cow;
use std::mem::size_of;

use super::{PageArena, PageId, PageImage};

/// One immutable chunk image.
pub(crate) type ChunkImage = PageImage;

/// One copy on write chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Chunk {
    /// One local mutable page owned by the live heap.
    Local(PageId),
    /// One immutable page image shared through captured images.
    Shared(ChunkImage),
}

impl Chunk {
    /// Create one local chunk from the given bytes.
    pub(crate) fn new(bytes: &[u8], arena: &mut PageArena) -> Self {
        Self::Local(arena.allocate(bytes))
    }

    /// Create one zeroed local chunk.
    pub(crate) fn zeroed(arena: &mut PageArena) -> Self {
        Self::Local(arena.allocate_zeroed())
    }

    /// Restore one chunk from one immutable image.
    pub(crate) fn from_image(bytes: ChunkImage) -> Self {
        Self::Shared(bytes)
    }

    /// Return the chunk bytes as one slice.
    pub(crate) fn as_slice<'a>(&'a self, arena: &'a PageArena) -> Option<&'a [u8]> {
        match self {
            Self::Local(page_id) => arena.page(*page_id),
            Self::Shared(bytes) => Some(bytes.as_ref()),
        }
    }

    /// Return one byte at the given index.
    pub(crate) fn get(&self, arena: &PageArena, index: usize) -> Option<u8> {
        self.as_slice(arena)?.get(index).copied()
    }

    /// Return one immutable chunk image.
    pub(crate) fn image(&mut self, arena: &mut PageArena) -> ChunkImage {
        if let Self::Local(page_id) = self {
            let page_id = *page_id;
            let image = arena
                .image(page_id)
                .expect("local chunk page must stay addressable");
            let freed = arena.free(page_id);
            debug_assert!(freed, "imaged local chunk page must stay freeable");
            *self = Self::Shared(image.clone());
        }

        match self {
            Self::Local(_) => unreachable!(),
            Self::Shared(bytes) => bytes.clone(),
        }
    }

    /// Return mutable chunk bytes, detaching one shared image on first write.
    pub(crate) fn bytes_mut<'a>(&'a mut self, arena: &'a mut PageArena) -> Option<&'a mut [u8]> {
        if let Self::Shared(bytes) = self {
            let page_id = arena.allocate(bytes.as_ref());
            *self = Self::Local(page_id);
        }

        match self {
            Self::Local(page_id) => arena.page_mut(*page_id),
            Self::Shared(_) => unreachable!(),
        }
    }

    /// Write one byte at the given index.
    pub(crate) fn set(&mut self, arena: &mut PageArena, index: usize, byte: u8) -> bool {
        let Some(bytes) = self.bytes_mut(arena) else {
            return false;
        };
        let Some(slot) = bytes.get_mut(index) else {
            return false;
        };

        *slot = byte;
        true
    }

    /// Write one byte slice by range.
    pub(crate) fn set_bytes(&mut self, arena: &mut PageArena, start: usize, bytes: &[u8]) -> bool {
        let Some(end) = start.checked_add(bytes.len()) else {
            return false;
        };
        let Some(chunk_bytes) = self.bytes_mut(arena) else {
            return false;
        };
        let Some(window) = chunk_bytes.get_mut(start..end) else {
            return false;
        };

        window.copy_from_slice(bytes);
        true
    }

    /// Return the borrowed image bytes referenced by this chunk.
    pub(crate) fn borrowed_bytes(&self) -> usize {
        match self {
            Self::Local(_) => 0,
            Self::Shared(bytes) => bytes.len(),
        }
    }

    /// Free one local page owned by this chunk, if any.
    pub(crate) fn free(&mut self, arena: &mut PageArena) {
        if let Self::Local(page_id) = self {
            let freed = arena.free(*page_id);
            debug_assert!(freed, "local chunk page must stay freeable");
        }
    }
}

/// One chunk backed payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChunkPayload {
    /// The logical payload byte length.
    len: usize,
    /// The fixed page width for this payload.
    page_bytes: usize,
    /// The chunk backed storage.
    chunks: Vec<Chunk>,
}

impl ChunkPayload {
    /// Create one payload from the given bytes.
    pub(crate) fn new(bytes: &[u8], page_bytes: usize, arena: &mut PageArena) -> Self {
        let page_bytes = page_bytes.max(1);

        Self {
            len: bytes.len(),
            page_bytes,
            chunks: bytes
                .chunks(page_bytes)
                .map(|chunk| Chunk::new(chunk, arena))
                .collect(),
        }
    }

    /// Create one zeroed payload with the given byte length.
    pub(crate) fn new_zeroed(byte_len: usize, page_bytes: usize, arena: &mut PageArena) -> Self {
        let page_bytes = page_bytes.max(1);
        let chunk_count = byte_len.div_ceil(page_bytes);
        let mut chunks = Vec::with_capacity(chunk_count);

        for _ in 0..chunk_count {
            chunks.push(Chunk::zeroed(arena));
        }

        Self {
            len: byte_len,
            page_bytes,
            chunks,
        }
    }

    /// Restore one payload from one immutable image.
    pub(crate) fn from_image(
        len: usize,
        page_bytes: usize,
        chunks: impl IntoIterator<Item = ChunkImage>,
    ) -> Self {
        Self {
            len,
            page_bytes,
            chunks: chunks.into_iter().map(Chunk::from_image).collect(),
        }
    }

    /// Return the logical byte length.
    pub(crate) fn len(&self) -> usize {
        self.len
    }

    /// Return the fixed page width.
    pub(crate) fn page_bytes(&self) -> usize {
        self.page_bytes
    }

    /// Return one borrowed or materialized byte view over this payload.
    pub(crate) fn bytes<'a>(&'a self, arena: &'a PageArena) -> Cow<'a, [u8]> {
        if self.len == 0 {
            return Cow::Borrowed(&[]);
        }

        if self.chunks.len() == 1 {
            let bytes = self.chunks[0]
                .as_slice(arena)
                .expect("single chunk payload must stay addressable");
            return Cow::Borrowed(&bytes[..self.len]);
        }

        let mut bytes = Vec::with_capacity(self.len);

        for chunk in &self.chunks {
            let chunk_bytes = chunk
                .as_slice(arena)
                .expect("payload chunk must stay addressable");
            bytes.extend_from_slice(chunk_bytes);
        }

        bytes.truncate(self.len);
        Cow::Owned(bytes)
    }

    /// Return one byte by index.
    pub(crate) fn get(&self, arena: &PageArena, index: usize) -> Option<u8> {
        if index >= self.len {
            return None;
        }

        let chunk_index = index / self.page_bytes;
        let chunk_offset = index % self.page_bytes;
        self.chunks.get(chunk_index)?.get(arena, chunk_offset)
    }

    /// Copy one byte window out of this payload.
    pub(crate) fn read_window(&self, arena: &PageArena, start: usize, dest: &mut [u8]) -> bool {
        let end = match start.checked_add(dest.len()) {
            Some(end) => end,
            None => return false,
        };

        if end > self.len {
            return false;
        }

        if dest.is_empty() {
            return true;
        }

        let mut copied = 0usize;
        let mut offset = start;

        while copied < dest.len() {
            let chunk_index = offset / self.page_bytes;
            let chunk_offset = offset % self.page_bytes;
            let Some(chunk) = self.chunks.get(chunk_index) else {
                return false;
            };
            let Some(chunk_bytes) = chunk.as_slice(arena) else {
                return false;
            };

            let remaining = dest.len() - copied;
            let available = chunk_bytes.len().saturating_sub(chunk_offset);
            let count = remaining.min(available);
            if count == 0 {
                return false;
            }

            dest[copied..copied + count]
                .copy_from_slice(&chunk_bytes[chunk_offset..chunk_offset + count]);
            copied += count;
            offset += count;
        }

        true
    }

    /// Write one byte by index.
    pub(crate) fn set(&mut self, arena: &mut PageArena, index: usize, byte: u8) -> bool {
        if index >= self.len {
            return false;
        }

        let chunk_index = index / self.page_bytes;
        let chunk_offset = index % self.page_bytes;
        let Some(chunk) = self.chunks.get_mut(chunk_index) else {
            return false;
        };

        chunk.set(arena, chunk_offset, byte)
    }

    /// Write one byte slice by range.
    pub(crate) fn write_window(
        &mut self,
        arena: &mut PageArena,
        start: usize,
        bytes: &[u8],
    ) -> bool {
        let Some(end) = start.checked_add(bytes.len()) else {
            return false;
        };

        if end > self.len {
            return false;
        }

        let mut written = 0usize;
        let mut offset = start;

        while written < bytes.len() {
            let chunk_index = offset / self.page_bytes;
            let chunk_offset = offset % self.page_bytes;
            let Some(chunk) = self.chunks.get_mut(chunk_index) else {
                return false;
            };

            let remaining = bytes.len() - written;
            let available = self.page_bytes.saturating_sub(chunk_offset);
            let count = remaining.min(available);
            if count == 0 {
                return false;
            }

            if !chunk.set_bytes(arena, chunk_offset, &bytes[written..written + count]) {
                return false;
            }

            written += count;
            offset += count;
        }

        true
    }

    /// Replace the entire payload.
    pub(crate) fn replace(&mut self, arena: &mut PageArena, bytes: &[u8], page_bytes: usize) {
        self.clear(arena);
        *self = Self::new(bytes, page_bytes, arena);
    }

    /// Replace the entire payload with zeroed bytes.
    pub(crate) fn replace_zeroed(
        &mut self,
        arena: &mut PageArena,
        byte_len: usize,
        page_bytes: usize,
    ) {
        self.clear(arena);
        *self = Self::new_zeroed(byte_len, page_bytes, arena);
    }

    /// Clear this payload while preserving its owner.
    pub(crate) fn clear(&mut self, arena: &mut PageArena) {
        for chunk in &mut self.chunks {
            chunk.free(arena);
        }

        self.len = 0;
        self.page_bytes = 0;
        self.chunks.clear();
    }

    /// Capture one immutable payload image.
    pub(crate) fn image(&mut self, arena: &mut PageArena) -> Vec<ChunkImage> {
        self.chunks
            .iter_mut()
            .map(|chunk| chunk.image(arena))
            .collect()
    }

    /// Return the active local bytes owned by this payload.
    pub(crate) fn active_bytes(&self) -> usize {
        self.chunks.capacity() * size_of::<Chunk>()
    }

    /// Return the borrowed image bytes referenced by this payload.
    pub(crate) fn borrowed_bytes(&self) -> usize {
        self.chunks.iter().map(Chunk::borrowed_bytes).sum()
    }

    /// Return the active local bytes for one logical payload length.
    pub(crate) fn active_bytes_for_len(len: usize, page_bytes: usize) -> usize {
        let page_bytes = page_bytes.max(1);
        let chunk_count = if len == 0 {
            0
        } else {
            len.div_ceil(page_bytes)
        };

        chunk_count * size_of::<Chunk>()
    }

    /// Return the page count and active-byte reservation for writing the given window.
    pub(crate) fn write_page_reservation(&self, start: usize, len: usize) -> (usize, i64) {
        if len == 0 {
            return (0, 0);
        }

        let Some(end) = start.checked_add(len) else {
            return (0, 0);
        };
        if end > self.len {
            return (0, 0);
        }

        let first_chunk = start / self.page_bytes;
        let last_chunk = (end - 1) / self.page_bytes;
        let detached_chunks = (first_chunk..=last_chunk)
            .filter(|&index| matches!(self.chunks.get(index), Some(Chunk::Shared(_))))
            .count();

        (detached_chunks, 0)
    }

    /// Return the page counts and active-byte reservation for replacing this payload.
    pub(crate) fn replace_page_reservation(&self, new_len: usize) -> (usize, usize, i64) {
        let freed_local_pages = self
            .chunks
            .iter()
            .filter(|chunk| matches!(chunk, Chunk::Local(_)))
            .count();
        let new_page_count = if new_len == 0 {
            0
        } else {
            new_len.div_ceil(self.page_bytes.max(1))
        };
        let payload_reservation = Self::active_bytes_for_len(new_len, self.page_bytes) as i64
            - self.active_bytes() as i64;

        (freed_local_pages, new_page_count, payload_reservation)
    }
}
