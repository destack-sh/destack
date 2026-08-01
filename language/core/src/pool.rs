use std::hash::{BuildHasher, Hash};

use destack_serde::Reflect;
use rustc_hash::{FxBuildHasher, FxHashMap};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::Arena;

/// Identifier for one pooled value slot.
pub trait PoolId: Copy {
    /// Wrap a raw slot id.
    fn from_raw(raw: u32) -> Self;

    /// Return the raw slot id.
    fn raw(self) -> u32;
}

/// Deduplicating value arena continuing one id space across segments.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[serde(bound(
    serialize = "T: Serialize",
    deserialize = "T: serde::de::DeserializeOwned"
))]
pub struct ValuePool<I: PoolId, T> {
    /// The first value id owned by this pool.
    first_id: u32,
    /// The interned values.
    values: Arena<T>,
    /// The intern index from value hash to owned slots.
    #[serde(skip)]
    index: FxHashMap<u64, SmallVec<[I; 1]>>,
    /// The value hash per owned slot, parallel to `values`.
    #[serde(skip)]
    hashes: Vec<u64>,
}

impl<I: PoolId, T: Copy + Eq + Hash> ValuePool<I, T> {
    /// Create an empty pool starting at one first id.
    pub fn new(first_id: u32) -> Self {
        Self {
            first_id,
            values: Arena::new(),
            index: FxHashMap::default(),
            hashes: Vec::new(),
        }
    }

    /// Intern one value.
    pub fn intern(&mut self, value: T) -> I {
        // probe the index for an existing structural hit
        let hash = FxBuildHasher.hash_one(value);
        if let Some(slots) = self.index.get(&hash) {
            for slot in slots {
                if self.values.get(slot.raw() - self.first_id) == &value {
                    return *slot;
                }
            }
        }

        // allocate and index the new slot
        let id = I::from_raw(self.count());
        self.values.allocate(value);
        self.hashes.push(hash);
        self.index.entry(hash).or_default().push(id);

        id
    }

    /// Find one already-interned value without allocating.
    pub fn find(&self, value: &T) -> Option<I> {
        let hash = FxBuildHasher.hash_one(*value);
        let slots = self.index.get(&hash)?;
        slots
            .iter()
            .find(|slot| self.values.get(slot.raw() - self.first_id) == value)
            .copied()
    }

    /// Return the number of values owned up to and including this pool.
    pub fn count(&self) -> u32 {
        self.first_id + self.values.len() as u32
    }

    /// Return one value, when owned by this pool.
    pub fn get(&self, id: I) -> Option<&T> {
        id.raw()
            .checked_sub(self.first_id)
            .and_then(|slot| self.values.get_maybe(slot))
    }

    /// Drop every value interned after one mark.
    pub fn truncate_to(&mut self, count: u32) {
        // unindex the dropped slots
        let keep = count.saturating_sub(self.first_id) as usize;
        for slot in keep..self.values.len() {
            let hash = self.hashes[slot];
            if let Some(slots) = self.index.get_mut(&hash) {
                let id = self.first_id + slot as u32;
                slots.retain(|entry| entry.raw() != id);
            }
        }
        self.values.truncate(keep);
        self.hashes.truncate(keep);
    }
}
