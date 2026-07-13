use std::sync::Arc;

/// The page count stored in one page table chunk.
const PAGE_TABLE_CHUNK_LEN: usize = 1024;

/// One sparse page table keyed by world page index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PageTable<T> {
    /// Shared page chunks indexed directly by world chunk index.
    chunks: Vec<Option<Arc<PageChunk<T>>>>,
}

/// One allocated page table chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PageChunk<T> {
    /// The number of occupied entries in this chunk.
    occupied_count: usize,
    /// The page entries in this chunk.
    entries: Box<[Option<T>]>,
}

impl<T> Default for PageTable<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> PageTable<T> {
    /// Create one empty page table.
    pub(crate) const fn new() -> Self {
        Self { chunks: Vec::new() }
    }

    /// Return one page entry.
    pub(crate) fn get(&self, page_index: usize) -> Option<&T> {
        let chunk_index = page_index / PAGE_TABLE_CHUNK_LEN;
        let entry_index = page_index % PAGE_TABLE_CHUNK_LEN;
        let chunk = self.chunks.get(chunk_index)?.as_ref()?;

        chunk.entries[entry_index].as_ref()
    }

    /// Set one page entry.
    pub(crate) fn set(&mut self, page_index: usize, entry: T)
    where
        T: Clone,
    {
        let chunk_index = page_index / PAGE_TABLE_CHUNK_LEN;
        let entry_index = page_index % PAGE_TABLE_CHUNK_LEN;
        if self.chunks.len() <= chunk_index {
            self.chunks.resize_with(chunk_index + 1, || None);
        }

        // detach only the metadata chunk touched by this branch
        let chunk = self.chunks[chunk_index].get_or_insert_with(|| Arc::new(PageChunk::new()));
        let chunk = Arc::make_mut(chunk);
        if chunk.entries[entry_index].is_none() {
            chunk.occupied_count += 1;
        }
        chunk.entries[entry_index] = Some(entry);
    }

    /// Clear one page entry.
    pub(crate) fn clear(&mut self, page_index: usize)
    where
        T: Clone,
    {
        let chunk_index = page_index / PAGE_TABLE_CHUNK_LEN;
        let entry_index = page_index % PAGE_TABLE_CHUNK_LEN;
        let Some(Some(chunk)) = self.chunks.get_mut(chunk_index) else {
            return;
        };

        // preserve a shared chunk when the requested page is already empty
        if chunk.entries[entry_index].is_none() {
            return;
        }

        let chunk = Arc::make_mut(chunk);
        chunk.entries[entry_index] = None;
        chunk.occupied_count -= 1;
        if chunk.occupied_count == 0 {
            self.chunks[chunk_index] = None;
        }
    }

    /// Remove every page entry.
    pub(crate) fn clear_all(&mut self) {
        self.chunks.clear();
    }
}

impl<T> PageChunk<T> {
    /// Create one empty page chunk.
    fn new() -> Self {
        let entries = std::iter::repeat_with(|| None)
            .take(PAGE_TABLE_CHUNK_LEN)
            .collect();

        Self {
            occupied_count: 0,
            entries,
        }
    }
}
