use std::sync::Arc;

/// The number of entries stored in one shared table chunk.
const TABLE_CHUNK_LEN: usize = 256;

/// One dense copy on write table for stable heap metadata ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CowTable<T> {
    /// The number of live entries stored in this table.
    len: usize,
    /// The shared metadata chunks in stable order.
    chunks: Vec<Arc<Vec<T>>>,
}

impl<T> Default for CowTable<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> CowTable<T> {
    /// Create one empty copy on write table.
    pub(crate) const fn new() -> Self {
        Self {
            len: 0,
            chunks: Vec::new(),
        }
    }

    /// Build one copy on write table from one dense vector.
    pub(crate) fn from_vec(values: Vec<T>) -> Self
    where
        T: Clone,
    {
        // keep one empty table empty
        if values.is_empty() {
            return Self::new();
        }

        let len = values.len();
        let chunks = values
            .chunks(TABLE_CHUNK_LEN)
            .map(|chunk| Arc::new(chunk.to_vec()))
            .collect();

        Self { len, chunks }
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

        let chunk_index = index / TABLE_CHUNK_LEN;
        let entry_index = index % TABLE_CHUNK_LEN;

        self.chunks.get(chunk_index)?.get(entry_index)
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

        let chunk_index = index / TABLE_CHUNK_LEN;
        let entry_index = index % TABLE_CHUNK_LEN;
        let chunk = self.chunks.get_mut(chunk_index)?;
        let chunk = Arc::make_mut(chunk);

        chunk.get_mut(entry_index)
    }

    /// Append one new entry at the dense tail.
    pub(crate) fn push(&mut self, value: T)
    where
        T: Clone,
    {
        // allocate one fresh chunk when the tail is full
        if self.len % TABLE_CHUNK_LEN == 0 {
            self.chunks
                .push(Arc::new(Vec::with_capacity(TABLE_CHUNK_LEN)));
        }

        // append into the current tail chunk
        let Some(last_chunk) = self.chunks.last_mut() else {
            return;
        };
        let last_chunk = Arc::make_mut(last_chunk);

        last_chunk.push(value);
        self.len = self.len.saturating_add(1);
    }

    /// Set one existing dense entry by index.
    pub(crate) fn set(&mut self, index: usize, value: T)
    where
        T: Clone,
    {
        if let Some(entry) = self.get_mut(index) {
            *entry = value;
        }
    }

    /// Return one iterator over every stored entry.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        self.chunks.iter().flat_map(|chunk| chunk.iter())
    }

    /// Return one boxed slice copy of every stored entry.
    pub(crate) fn to_boxed_slice(&self) -> Box<[T]>
    where
        T: Clone,
    {
        self.iter().cloned().collect::<Vec<_>>().into_boxed_slice()
    }
}
