use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self, Debug, Formatter};
use std::mem::size_of;

use crate::{StableHasher, stable_hash_text_128};

/// Stable content identity for one interned string.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct StringId(pub u128);

impl Debug for StringId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "#{:032x}", self.0)
    }
}

impl std::fmt::Display for StringId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "#{:032x}", self.0)
    }
}

impl StringId {
    /// Create the stable id for one string.
    #[inline]
    pub fn for_text(text: &str) -> Self {
        Self(stable_hash_text_128(text))
    }

    /// Return the raw stable hash bits.
    #[inline]
    pub fn raw(self) -> u128 {
        self.0
    }
}

/// Dense storage entry for one interned string.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct StringEntry {
    /// Stable content id.
    id: StringId,
    /// Byte offset into the contiguous buffer.
    offset: u32,
    /// Byte length in the contiguous buffer.
    len: u32,
}

/// Serialized string pool representation.
#[derive(Serialize, Deserialize)]
struct LocalStringPoolData {
    /// Stored strings sorted by stable id.
    strings: Vec<(StringId, String)>,
}

/// Arena-based string pool for single-threaded use.
#[derive(Clone, Default)]
pub struct LocalStringPool {
    /// Contiguous buffer containing all interned string bytes.
    buffer: String,
    /// Dense entries for stored strings.
    entries: Vec<StringEntry>,
    /// Dense entry index by stable string id.
    slot_by_id: FxHashMap<StringId, usize>,
}

impl Serialize for LocalStringPool {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_serialized_data().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for LocalStringPool {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data = LocalStringPoolData::deserialize(deserializer)?;
        Self::from_serialized_data(data).map_err(serde::de::Error::custom)
    }
}

impl Debug for LocalStringPool {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalStringPool")
            .field("length", &self.entries.len())
            .field("buffer_size", &self.buffer.len())
            .finish()
    }
}

impl LocalStringPool {
    /// Create a new empty LocalStringPool.
    #[inline]
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            entries: Vec::new(),
            slot_by_id: FxHashMap::default(),
        }
    }

    /// Create a new LocalStringPool with pre-allocated capacity.
    #[inline]
    pub fn with_capacity(string_count: usize, total_bytes: usize) -> Self {
        Self {
            buffer: String::with_capacity(total_bytes),
            entries: Vec::with_capacity(string_count),
            slot_by_id: FxHashMap::with_capacity_and_hasher(string_count, Default::default()),
        }
    }

    /// Get the string associated with the given StringId.
    #[inline]
    pub fn get(&self, id: StringId) -> &str {
        let slot = self
            .slot_by_id
            .get(&id)
            .copied()
            .unwrap_or_else(|| panic!("string id {id} is not present in this string pool"));
        let entry = self.entries[slot];

        &self.buffer[entry.offset as usize..(entry.offset + entry.len) as usize]
    }

    /// Check if the pool contains the given StringId.
    #[inline]
    pub fn contains(&self, id: StringId) -> bool {
        self.slot_by_id.contains_key(&id)
    }

    /// Intern a string, storing only one owned copy of bytes.
    #[inline]
    pub fn intern<S: AsRef<str>>(&mut self, text: S) -> StringId {
        let text = text.as_ref();
        let id = StringId::for_text(text);

        if let Some(slot) = self.slot_by_id.get(&id).copied() {
            let entry = self.entries[slot];
            let existing = &self.buffer[entry.offset as usize..(entry.offset + entry.len) as usize];
            assert_eq!(
                existing, text,
                "string id collision for {id}: existing {existing:?}, new {text:?}",
            );

            return id;
        }

        self.insert_verified(id, text);

        id
    }

    /// Ensure this pool contains one string from another pool.
    #[inline]
    pub fn ensure_from(&mut self, other: &LocalStringPool, string_id: StringId) {
        let text = other.get(string_id);

        self.ensure_text(string_id, text);
    }

    /// Ensure this pool contains every string from another pool.
    pub fn ensure_all_from(&mut self, other: &LocalStringPool) {
        for (id, text) in other.iter() {
            self.ensure_text(id, text);
        }
    }

    /// Iterate stored strings in dense storage order.
    pub fn iter(&self) -> impl Iterator<Item = (StringId, &str)> + '_ {
        self.entries
            .iter()
            .map(|entry| (entry.id, self.get(entry.id)))
    }

    /// Get the number of unique strings stored in this pool.
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the pool contains no strings.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Convert to an immutable pool.
    #[inline]
    pub fn into_immutable(self) -> ImmutableStringPool {
        ImmutableStringPool { inner: self }
    }

    /// Return the owned bytes for this pool.
    pub fn owned_bytes(&self) -> usize {
        let mut owned_bytes = size_of::<Self>();
        owned_bytes += self.buffer.capacity() * size_of::<u8>();
        owned_bytes += self.entries.capacity() * size_of::<StringEntry>();
        owned_bytes += self.slot_by_id.capacity() * size_of::<(StringId, usize)>();

        owned_bytes
    }

    /// Create the canonical serialized representation.
    fn to_serialized_data(&self) -> LocalStringPoolData {
        let mut strings = self
            .entries
            .iter()
            .map(|entry| (entry.id, self.get(entry.id).to_string()))
            .collect::<Vec<_>>();
        strings.sort_by_key(|(id, _)| *id);

        LocalStringPoolData { strings }
    }

    /// Build one pool from its canonical serialized representation.
    fn from_serialized_data(data: LocalStringPoolData) -> Result<Self, String> {
        let mut pool = Self::with_capacity(
            data.strings.len(),
            data.strings.iter().map(|(_, string)| string.len()).sum(),
        );

        for (id, string) in data.strings {
            let actual_id = StringId::for_text(&string);
            if actual_id != id {
                return Err(format!(
                    "string pool entry id {id} does not match content id {actual_id}"
                ));
            }

            if pool.contains(id) {
                return Err(format!("duplicate string pool entry id {id}"));
            }

            pool.insert_verified(id, &string);
        }

        Ok(pool)
    }

    /// Insert one string after its stable id has already been checked.
    fn insert_verified(&mut self, id: StringId, text: &str) {
        let offset = u32::try_from(self.buffer.len())
            .expect("StringPool exhausted u32 address space for byte offsets");
        let len = u32::try_from(text.len())
            .expect("StringPool exhausted u32 address space for string lengths");

        self.buffer.push_str(text);

        let slot = self.entries.len();
        self.entries.push(StringEntry { id, offset, len });
        self.slot_by_id.insert(id, slot);
    }

    /// Ensure one checked string is present in the pool.
    fn ensure_text(&mut self, id: StringId, text: &str) {
        let actual_id = StringId::for_text(text);
        assert_eq!(
            actual_id, id,
            "string id {id} does not match copied text id {actual_id}",
        );

        if let Some(slot) = self.slot_by_id.get(&id).copied() {
            let entry = self.entries[slot];
            let existing = &self.buffer[entry.offset as usize..(entry.offset + entry.len) as usize];
            assert_eq!(
                existing, text,
                "string id collision for {id}: existing {existing:?}, new {text:?}",
            );

            return;
        }

        self.insert_verified(id, text);
    }
}

/// Thread-safe string interning with stable identifiers.
pub struct StringPool {
    inner: RwLock<LocalStringPool>,
}

impl Clone for StringPool {
    fn clone(&self) -> Self {
        let state = self.inner.read().clone();
        Self {
            inner: RwLock::new(state),
        }
    }
}

impl Debug for StringPool {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let state = self.inner.read();
        f.debug_struct("StringPool")
            .field("length", &state.len())
            .finish()
    }
}

impl Default for StringPool {
    fn default() -> Self {
        Self::new()
    }
}

impl Serialize for StringPool {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let state = self.inner.read();

        state.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StringPool {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let state = LocalStringPool::deserialize(deserializer)?;

        Ok(Self {
            inner: RwLock::new(state),
        })
    }
}

impl StringPool {
    /// Create a new empty StringPool.
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(LocalStringPool::new()),
        }
    }

    /// Create a StringPool from a LocalStringPool.
    pub fn from_local(local: LocalStringPool) -> Self {
        Self {
            inner: RwLock::new(local),
        }
    }

    /// Check if the pool contains the given StringId.
    #[inline]
    pub fn contains(&self, id: StringId) -> bool {
        let state = self.inner.read();

        state.contains(id)
    }

    /// Get the string associated with the given StringId.
    #[inline]
    pub fn get(&self, id: StringId) -> StringRef<'_> {
        let state = self.inner.read();

        StringRef { pool: state, id }
    }

    /// Intern a string, storing only one owned copy of bytes.
    pub fn intern(&self, text: &str) -> StringId {
        let id = StringId::for_text(text);

        {
            let state = self.inner.read();
            if state.contains(id) {
                let existing = state.get(id);
                assert_eq!(
                    existing, text,
                    "string id collision for {id}: existing {existing:?}, new {text:?}",
                );

                return id;
            }
        }

        let mut state = self.inner.write();
        if state.contains(id) {
            let existing = state.get(id);
            assert_eq!(
                existing, text,
                "string id collision for {id}: existing {existing:?}, new {text:?}",
            );

            return id;
        }

        state.insert_verified(id, text);

        id
    }

    /// Ensure this pool contains one string from another pool.
    #[inline]
    pub fn ensure_from(&self, other: &StringPool, string_id: StringId) {
        let text = other.get(string_id).to_string();
        let mut state = self.inner.write();

        state.ensure_text(string_id, text.as_str());
    }

    /// Ensure this pool contains every string from another pool.
    pub fn ensure_all_from(&self, other: &StringPool) {
        let strings = {
            let other = other.inner.read();
            other
                .iter()
                .map(|(id, text)| (id, text.to_string()))
                .collect::<Vec<_>>()
        };

        let mut state = self.inner.write();
        for (id, text) in strings {
            state.ensure_text(id, text.as_str());
        }
    }

    /// Copy all strings from an immutable pool into this pool.
    pub fn copy_from_immutable(&self, other: &ImmutableStringPool) {
        for (_, text) in other.iter() {
            self.intern(text);
        }
    }

    /// Replace the entire pool contents from another pool.
    pub fn replace_from(&self, other: &StringPool) {
        let next_state = other.inner.read().clone();
        let mut state = self.inner.write();

        *state = next_state;
    }

    /// Return one stable hash for the current pool contents.
    pub fn stable_hash(&self) -> u64 {
        let state = self.inner.read();
        let mut hasher = StableHasher::new();
        let mut entries = state.iter().collect::<Vec<_>>();
        entries.sort_by_key(|(id, _)| *id);

        for (id, text) in entries {
            hasher.update(&id.raw().to_le_bytes());
            hasher.update_len_prefixed(text.as_bytes());
        }

        hasher.finish_u64()
    }

    /// Get the number of unique strings stored in this pool.
    #[inline]
    pub fn len(&self) -> usize {
        let state = self.inner.read();

        state.len()
    }

    /// Check if the pool contains no strings.
    #[inline]
    pub fn is_empty(&self) -> bool {
        let state = self.inner.read();

        state.is_empty()
    }

    /// Convert the pool to an immutable pool.
    pub fn into_immutable(self) -> ImmutableStringPool {
        ImmutableStringPool {
            inner: self.inner.into_inner(),
        }
    }
}

/// A reference to a string in a StringPool.
#[derive(Debug)]
pub struct StringRef<'a> {
    pool: parking_lot::RwLockReadGuard<'a, LocalStringPool>,
    id: StringId,
}

impl std::ops::Deref for StringRef<'_> {
    type Target = str;

    fn deref(&self) -> &str {
        self.pool.get(self.id)
    }
}

impl AsRef<str> for StringRef<'_> {
    fn as_ref(&self) -> &str {
        self.pool.get(self.id)
    }
}

impl std::cmp::PartialEq<&str> for StringRef<'_> {
    fn eq(&self, other: &&str) -> bool {
        &**self == *other
    }
}

/// Frozen string pool with lock-free read-only access.
#[derive(Clone, Default)]
pub struct ImmutableStringPool {
    inner: LocalStringPool,
}

impl Serialize for ImmutableStringPool {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ImmutableStringPool {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let inner = LocalStringPool::deserialize(deserializer)?;

        Ok(Self { inner })
    }
}

impl Debug for ImmutableStringPool {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImmutableStringPool")
            .field("length", &self.inner.len())
            .finish()
    }
}

impl ImmutableStringPool {
    /// Create a new empty immutable string pool.
    pub fn empty() -> Self {
        Self {
            inner: LocalStringPool::new(),
        }
    }

    /// Get the string associated with the given StringId.
    #[inline]
    pub fn get(&self, id: StringId) -> &str {
        self.inner.get(id)
    }

    /// Iterate stored strings in dense storage order.
    pub fn iter(&self) -> impl Iterator<Item = (StringId, &str)> + '_ {
        self.inner.iter()
    }

    /// Get the number of unique strings stored in this pool.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if the pool contains no strings.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Return the owned bytes for this immutable pool.
    pub fn owned_bytes(&self) -> usize {
        size_of::<Self>() + self.inner.owned_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_pool_intern_same_string_yields_same_id() {
        let mut pool = LocalStringPool::new();
        let a = pool.intern("hello");
        let b = pool.intern("hello");

        assert_eq!(a, b);
        assert_eq!(a, StringId::for_text("hello"));
        assert_eq!(pool.len(), 1);
        assert_eq!(pool.get(a), "hello");
        assert_eq!(size_of::<StringId>(), 16);
    }

    #[test]
    fn test_local_pool_intern_distinct_strings() {
        let mut pool = LocalStringPool::new();
        let a = pool.intern("alpha");
        let b = pool.intern("beta");

        assert_ne!(a, b);
        assert_eq!(pool.len(), 2);
        assert_eq!(pool.get(a), "alpha");
        assert_eq!(pool.get(b), "beta");
    }

    #[test]
    fn test_local_pool_intern_empty_string() {
        let mut pool = LocalStringPool::new();
        let id1 = pool.intern("");
        let id2 = pool.intern("");

        assert_eq!(id1, id2);
        assert_eq!(pool.len(), 1);
        assert_eq!(pool.get(id1), "");
    }

    #[test]
    fn test_thread_safe_pool_intern_same_string_yields_same_id() {
        let pool = StringPool::new();
        let a = pool.intern("hello");
        let b = pool.intern("hello");

        assert_eq!(a, b);
        assert_eq!(pool.len(), 1);
        assert_eq!(pool.get(a).as_ref(), "hello");
    }

    #[test]
    fn test_thread_safe_pool_intern_distinct_strings() {
        let pool = StringPool::new();
        let a = pool.intern("alpha");
        let b = pool.intern("beta");

        assert_ne!(a, b);
        assert_eq!(pool.len(), 2);
        assert_eq!(pool.get(a).as_ref(), "alpha");
        assert_eq!(pool.get(b).as_ref(), "beta");
    }

    #[test]
    fn test_immutable_pool() {
        let mut local = LocalStringPool::new();
        let a = local.intern("foo");
        let b = local.intern("bar");

        let immutable = local.into_immutable();

        assert_eq!(immutable.get(a), "foo");
        assert_eq!(immutable.get(b), "bar");
        assert_eq!(immutable.len(), 2);
    }

    #[test]
    fn test_pool_serialized_data_is_sorted_by_string_id() {
        let mut pool = LocalStringPool::new();
        pool.intern("zeta");
        pool.intern("alpha");
        pool.intern("middle");

        let data = pool.to_serialized_data();
        let mut expected = data.strings.clone();
        expected.sort_by_key(|(id, _)| *id);

        assert_eq!(data.strings, expected);
    }

    #[test]
    fn test_pool_deserialize_rejects_duplicate_id() {
        let id = StringId::for_text("alpha");
        let data = LocalStringPoolData {
            strings: vec![(id, "alpha".to_string()), (id, "alpha".to_string())],
        };

        let result = LocalStringPool::from_serialized_data(data);

        assert_eq!(
            result.unwrap_err(),
            format!("duplicate string pool entry id {id}")
        );
    }

    #[test]
    fn test_pool_deserialize_rejects_mismatched_id() {
        let id = StringId::for_text("beta");
        let actual_id = StringId::for_text("alpha");
        let data = LocalStringPoolData {
            strings: vec![(id, "alpha".to_string())],
        };

        let result = LocalStringPool::from_serialized_data(data);

        assert_eq!(
            result.unwrap_err(),
            format!("string pool entry id {id} does not match content id {actual_id}")
        );
    }

    #[test]
    fn test_pool_deserialize_restores_strings() {
        let mut pool = LocalStringPool::new();
        let alpha = pool.intern("alpha");
        let beta = pool.intern("beta");
        let data = pool.to_serialized_data();

        let restored = LocalStringPool::from_serialized_data(data).unwrap();

        assert_eq!(restored.get(alpha), "alpha");
        assert_eq!(restored.get(beta), "beta");
        assert_eq!(restored.len(), 2);
    }

    #[test]
    fn test_pool_stable_hash_ignores_insertion_order() {
        let first = StringPool::new();
        first.intern("alpha");
        first.intern("beta");

        let second = StringPool::new();
        second.intern("beta");
        second.intern("alpha");

        assert_eq!(first.stable_hash(), second.stable_hash());
    }
}
