use serde::{Deserialize, Serialize};

/// One sparse node table entry.
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
struct SparseNodeEntry<T> {
    /// The node id that owns the value.
    node_id: u32,
    /// The sparse value.
    value: T,
}

/// Sorted sparse table keyed by node id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SparseNodeMap<T> {
    /// The sparse entries sorted by node id.
    entries: Vec<SparseNodeEntry<T>>,
}

impl<T> Default for SparseNodeMap<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> SparseNodeMap<T> {
    /// Create an empty sparse node table.
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl<T> SparseNodeMap<T>
where
    T: Copy,
{
    /// Return one value by node id.
    #[inline]
    pub(crate) fn get(&self, node_id: u32) -> Option<T> {
        self.entries
            .binary_search_by_key(&node_id, |entry| entry.node_id)
            .ok()
            .map(|index| self.entries[index].value)
    }

    /// Insert or update one value.
    #[inline]
    pub(crate) fn insert(&mut self, node_id: u32, value: T) {
        if let Some(last) = self.entries.last_mut() {
            if last.node_id == node_id {
                last.value = value;
                return;
            }
            if last.node_id < node_id {
                self.entries.push(SparseNodeEntry { node_id, value });
                return;
            }
        }

        match self
            .entries
            .binary_search_by_key(&node_id, |entry| entry.node_id)
        {
            Ok(index) => self.entries[index].value = value,
            Err(index) => self
                .entries
                .insert(index, SparseNodeEntry { node_id, value }),
        }
    }

    /// Remove one value by node id.
    #[inline]
    pub(crate) fn remove(&mut self, node_id: u32) {
        if let Some(last) = self.entries.last() {
            if last.node_id == node_id {
                self.entries.pop();
                return;
            }
        }

        if let Ok(index) = self
            .entries
            .binary_search_by_key(&node_id, |entry| entry.node_id)
        {
            self.entries.remove(index);
        }
    }

    /// Retain values that satisfy one predicate.
    pub(crate) fn retain(&mut self, mut keep: impl FnMut(u32, T) -> bool) {
        self.entries
            .retain(|entry| keep(entry.node_id, entry.value));
    }
}
