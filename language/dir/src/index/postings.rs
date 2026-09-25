use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// Compact postings from one key to module ordinals.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Postings<K> {
    /// The sorted posting keys.
    pub keys: Vec<K>,
    /// Offsets into the module ordinal list.
    pub offsets: Vec<u32>,
    /// Module ordinals by key.
    pub modules: Vec<u32>,
}

impl<K> Default for Postings<K> {
    /// Return empty postings.
    fn default() -> Self {
        Self {
            keys: Vec::new(),
            offsets: vec![0],
            modules: Vec::new(),
        }
    }
}

impl<K: Ord> Postings<K> {
    /// Build postings from unsorted key and module ordinal pairs.
    pub fn from_pairs(pairs: impl IntoIterator<Item = (K, u32)>) -> Self {
        let mut pairs = pairs.into_iter().collect::<Vec<_>>();
        pairs.sort();
        pairs.dedup();

        let mut keys = Vec::new();
        let mut offsets = Vec::new();
        let mut modules = Vec::new();

        for (key, module) in pairs {
            if keys.last().is_none_or(|last| last != &key) {
                offsets.push(modules.len() as u32);
                keys.push(key);
            }

            modules.push(module);
        }

        offsets.push(modules.len() as u32);

        Self {
            keys,
            offsets,
            modules,
        }
    }

    /// Return module ordinals for one exact key.
    pub fn get(&self, key: &K) -> &[u32] {
        let Ok(index) = self.keys.binary_search(key) else {
            return &[];
        };

        self.range(index)
    }

    /// Return module ordinals for one key index.
    pub fn range(&self, index: usize) -> &[u32] {
        let start = self.offsets[index] as usize;
        let end = self.offsets[index + 1] as usize;

        &self.modules[start..end]
    }
}

impl<K: Clone + Ord> Postings<K> {
    /// Replace one module's posting keys.
    pub fn replace(&mut self, module: u32, keys: impl IntoIterator<Item = K>) {
        let mut pairs = Vec::new();

        // retain postings from every other module
        for (index, key) in self.keys.iter().enumerate() {
            pairs.extend(
                self.range(index)
                    .iter()
                    .filter(|candidate| **candidate != module)
                    .map(|candidate| (key.clone(), *candidate)),
            );
        }

        // append the replacement keys
        pairs.extend(keys.into_iter().map(|key| (key, module)));
        *self = Self::from_pairs(pairs);
    }
}
