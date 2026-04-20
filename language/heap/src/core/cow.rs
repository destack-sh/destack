use destack_core::CowBuffer;

use super::{HeapError, HeapResult};

/// The standard entry count per copy on write metadata chunk.
const DEFAULT_COW_TABLE_CHUNK_LEN: usize = 256;

/// One dense copy on write table for stable heap metadata ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CowTable<T> {
    /// The number of live entries stored in this table.
    len: usize,
    /// The entry count per shared metadata chunk.
    chunk_len: usize,
    /// The shared metadata chunks in stable order.
    chunks: Vec<CowBuffer<T>>,
}

impl<T> Default for CowTable<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> CowTable<T> {
    /// Create one empty copy on write table.
    pub(crate) fn new() -> Self {
        Self {
            len: 0,
            chunk_len: DEFAULT_COW_TABLE_CHUNK_LEN,
            chunks: Vec::new(),
        }
    }

    /// Create one empty copy on write table with one explicit chunk length.
    pub(crate) fn with_chunk_len(chunk_len: usize) -> HeapResult<Self> {
        if chunk_len == 0 {
            return Err(HeapError::InvalidTableChunkLen { len: chunk_len });
        }

        Ok(Self {
            len: 0,
            chunk_len,
            chunks: Vec::new(),
        })
    }

    /// Build one copy on write table from one dense vector and chunk length.
    pub(crate) fn from_vec_with_chunk_len(values: Vec<T>, chunk_len: usize) -> HeapResult<Self>
    where
        T: Clone,
    {
        if chunk_len == 0 {
            return Err(HeapError::InvalidTableChunkLen { len: chunk_len });
        }

        // keep one empty table empty
        if values.is_empty() {
            return Self::with_chunk_len(chunk_len);
        }

        let len = values.len();
        let chunks = values
            .chunks(chunk_len)
            .map(|chunk| CowBuffer::from_vec(chunk.to_vec()))
            .collect();

        Ok(Self {
            len,
            chunk_len,
            chunks,
        })
    }

    /// Return the number of stored entries.
    pub(crate) const fn len(&self) -> usize {
        self.len
    }

    /// Return one immutable entry by dense index.
    pub(crate) fn get(&self, index: usize) -> Option<&T> {
        // reject indices outside the dense table first
        if index >= self.len {
            return None;
        }

        let chunk_index = index / self.chunk_len;
        let entry_index = index % self.chunk_len;

        self.chunks.get(chunk_index)?.as_slice().get(entry_index)
    }

    /// Return one mutable entry by dense index.
    pub(crate) fn get_mut(&mut self, index: usize) -> Option<&mut T>
    where
        T: Clone,
    {
        // reject indices outside the dense table first
        if index >= self.len {
            return None;
        }

        let chunk_index = index / self.chunk_len;
        let entry_index = index % self.chunk_len;
        self.chunks
            .get_mut(chunk_index)?
            .make_mut()
            .get_mut(entry_index)
    }

    /// Append one new entry at the dense tail.
    pub(crate) fn push(&mut self, value: T) -> HeapResult<()>
    where
        T: Clone,
    {
        // allocate one fresh chunk when the tail is full
        if self.len.is_multiple_of(self.chunk_len) || self.chunks.is_empty() {
            self.chunks
                .push(CowBuffer::from_vec(Vec::with_capacity(self.chunk_len)));
        }

        // append into the current tail chunk
        let last_chunk_index = self
            .chunks
            .len()
            .checked_sub(1)
            .ok_or(HeapError::MissingTableEntry { index: self.len })?;
        let last_chunk = &mut self.chunks[last_chunk_index];
        last_chunk.make_mut().push(value);
        self.len = self
            .len
            .checked_add(1)
            .ok_or(HeapError::InvariantOverflow {
                context: "copy on write table length",
            })?;

        Ok(())
    }

    /// Set one existing dense entry by index.
    pub(crate) fn set(&mut self, index: usize, value: T) -> HeapResult<()>
    where
        T: Clone,
    {
        let Some(entry) = self.get_mut(index) else {
            return Err(HeapError::MissingTableEntry { index });
        };

        *entry = value;

        Ok(())
    }

    /// Set one dense entry or append it at the exact tail.
    pub(crate) fn set_or_push(&mut self, index: usize, value: T) -> HeapResult<()>
    where
        T: Clone,
    {
        if index == self.len() {
            self.push(value)?;

            return Ok(());
        }

        self.set(index, value)
    }

    /// Return one iterator over every stored entry.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        self.chunks.iter().flat_map(CowBuffer::as_slice)
    }

    /// Return one boxed slice copy of every stored entry.
    pub(crate) fn to_boxed_slice(&self) -> Box<[T]>
    where
        T: Clone,
    {
        self.iter().cloned().collect::<Vec<_>>().into_boxed_slice()
    }
}
