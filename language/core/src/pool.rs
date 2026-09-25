use std::hash::{BuildHasher, Hash, Hasher};

use rustc_hash::{FxBuildHasher, FxHashMap};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use tspp_serde::Reflect;

use crate::Arena;

/// Identifier for one pooled value slot.
pub trait PoolId: Copy {
    /// Wrap a raw slot id.
    fn from_raw(raw: u32) -> Self;

    /// Return the raw slot id.
    fn raw(self) -> u32;
}

/// Value entries of one pool kind continuing one id space across segments.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[serde(bound(
    serialize = "T: Serialize",
    deserialize = "T: serde::de::DeserializeOwned"
))]
pub struct ValuePool<T> {
    /// The first value id owned by this pool.
    first_id: u32,
    /// The interned values.
    values: Arena<T>,
}

impl<T: Hash> Hash for ValuePool<T> {
    /// Hash the owned values behind the pool's base id.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.first_id.hash(state);
        self.values.hash(state);
    }
}

impl<T> ValuePool<T> {
    /// Create an empty pool starting at one first id.
    pub fn new(first_id: u32) -> Self {
        Self {
            first_id,
            values: Arena::new(),
        }
    }

    /// Return the number of values owned up to and including this pool.
    pub fn count(&self) -> u32 {
        self.first_id + self.values.len() as u32
    }

    /// Return one value, when owned by this pool.
    pub fn get<I: PoolId>(&self, id: I) -> Option<&T> {
        id.raw()
            .checked_sub(self.first_id)
            .and_then(|slot| self.values.get_maybe(slot))
    }

    /// Iterate the owned values with their slot ids.
    pub fn iter<I: PoolId>(&self) -> impl Iterator<Item = (I, &T)> + '_ {
        self.values
            .as_slice()
            .iter()
            .enumerate()
            .map(|(slot, value)| (I::from_raw(self.first_id + slot as u32), value))
    }
}

/// Intern bookkeeping growing one value pool.
#[derive(Debug, Clone, Default)]
pub struct ValueInterner<I> {
    /// The intern index from value hash to owned slots.
    index: FxHashMap<u64, SmallVec<[I; 1]>>,
    /// The value hash per owned slot, parallel to the pool's values.
    hashes: Vec<u64>,
}

impl<I: PoolId> ValueInterner<I> {
    /// Create empty intern bookkeeping.
    pub fn new() -> Self {
        Self {
            index: FxHashMap::default(),
            hashes: Vec::new(),
        }
    }

    /// Index every value one pool already owns.
    pub fn over<T: Copy + Eq + Hash>(pool: &ValuePool<T>) -> Self {
        let mut interner = Self::new();
        for (id, value) in pool.iter::<I>() {
            let hash = FxBuildHasher.hash_one(*value);
            interner.hashes.push(hash);
            interner.index.entry(hash).or_default().push(id);
        }

        interner
    }

    /// Index every value in the committed pool under its structural hash.
    pub fn seed<T: Copy + Eq + Hash>(&mut self, committed: &ValuePool<T>) {
        for (id, value) in committed.iter::<I>() {
            let hash = FxBuildHasher.hash_one(*value);
            self.index.entry(hash).or_default().push(id);
        }
    }

    /// Intern one value into the pool.
    pub fn intern<T: Copy + Eq + Hash>(&mut self, pool: &mut ValuePool<T>, value: T) -> I {
        self.intern_with(pool, value, |_| None)
    }

    /// Intern one value into the pool, resolving committed ids through the given lookup.
    pub fn intern_with<T: Copy + Eq + Hash>(
        &mut self,
        pool: &mut ValuePool<T>,
        value: T,
        committed: impl Fn(I) -> Option<T>,
    ) -> I {
        // probe the index for an existing structural hit
        let hash = FxBuildHasher.hash_one(value);
        if let Some(slots) = self.index.get(&hash) {
            for slot in slots {
                let found = pool.get(*slot).copied().or_else(|| committed(*slot));
                if found == Some(value) {
                    return *slot;
                }
            }
        }

        // allocate and index the new slot
        let id = I::from_raw(pool.count());
        pool.values.allocate(value);
        self.hashes.push(hash);
        self.index.entry(hash).or_default().push(id);

        id
    }

    /// Find one already-interned value without allocating.
    pub fn find<T: Copy + Eq + Hash>(&self, pool: &ValuePool<T>, value: &T) -> Option<I> {
        let hash = FxBuildHasher.hash_one(*value);
        let slots = self.index.get(&hash)?;

        slots
            .iter()
            .find(|slot| pool.get(**slot) == Some(value))
            .copied()
    }

    /// Drop every value interned after one count.
    pub fn truncate<T>(&mut self, pool: &mut ValuePool<T>, count: u32) {
        // unindex the dropped slots
        let keep = count.saturating_sub(pool.first_id) as usize;
        for slot in keep..pool.values.len() {
            let hash = self.hashes[slot];
            if let Some(slots) = self.index.get_mut(&hash) {
                let id = pool.first_id + slot as u32;
                slots.retain(|entry| entry.raw() != id);
            }
        }
        pool.values.truncate(keep);
        self.hashes.truncate(keep);
    }
}
