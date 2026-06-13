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

/// A borrowed string from a pool.
pub type StringRef<'a> = &'a str;

/// Serialized string pool representation.
#[derive(Serialize, Deserialize)]
struct StringPoolData {
    /// Stored strings sorted by stable id.
    strings: Vec<(StringId, String)>,
}

/// Append-only storage for interned string bytes.
#[derive(Clone, Default)]
struct StringStorage {
    /// Stable string allocations.
    strings: Vec<Box<str>>,
    /// Stable string ids in allocation order.
    ids: Vec<StringId>,
    /// Dense allocation index by stable string id.
    slot_by_id: FxHashMap<StringId, usize>,
}

impl Serialize for StringStorage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_serialized_data().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StringStorage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data = StringPoolData::deserialize(deserializer)?;
        Self::from_serialized_data(data).map_err(serde::de::Error::custom)
    }
}

impl Debug for StringStorage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("StringStorage")
            .field("length", &self.strings.len())
            .field("buffer_size", &self.string_bytes())
            .finish()
    }
}

impl StringStorage {
    /// Create a new empty storage.
    #[inline]
    fn new() -> Self {
        Self {
            strings: Vec::new(),
            ids: Vec::new(),
            slot_by_id: FxHashMap::default(),
        }
    }

    /// Create a new storage with pre-allocated capacity.
    #[inline]
    fn with_capacity(string_count: usize, _: usize) -> Self {
        Self {
            strings: Vec::with_capacity(string_count),
            ids: Vec::with_capacity(string_count),
            slot_by_id: FxHashMap::with_capacity_and_hasher(string_count, Default::default()),
        }
    }

    /// Reserve room for additional interned strings.
    #[inline]
    fn reserve(&mut self, string_count: usize) {
        self.strings.reserve(string_count);
        self.ids.reserve(string_count);
        self.slot_by_id.reserve(string_count);
    }

    /// Get the string associated with the given StringId.
    #[inline]
    fn get(&self, id: StringId) -> &str {
        let slot = self
            .slot_by_id
            .get(&id)
            .copied()
            .unwrap_or_else(|| panic!("string id {id} is not present in this string pool"));

        &self.strings[slot]
    }

    /// Return the string associated with the given StringId when present.
    #[inline]
    fn get_maybe(&self, id: StringId) -> Option<&str> {
        let slot = self.slot_by_id.get(&id).copied()?;

        Some(&self.strings[slot])
    }

    /// Get a stable raw string pointer.
    #[inline]
    fn get_ptr(&self, id: StringId) -> *const str {
        self.get(id) as *const str
    }

    /// Return a stable raw string pointer when present.
    #[inline]
    fn get_maybe_ptr(&self, id: StringId) -> Option<*const str> {
        self.get_maybe(id).map(|string| string as *const str)
    }

    /// Check if the pool contains the given StringId.
    #[inline]
    fn contains(&self, id: StringId) -> bool {
        self.slot_by_id.contains_key(&id)
    }

    /// Iterate stored strings in dense storage order.
    fn iter(&self) -> impl Iterator<Item = (StringId, &str)> + '_ {
        self.ids
            .iter()
            .copied()
            .zip(self.strings.iter().map(|string| string.as_ref()))
    }

    /// Get the number of unique strings stored in this pool.
    #[inline]
    fn len(&self) -> usize {
        self.strings.len()
    }

    /// Check if the pool contains no strings.
    #[inline]
    fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    /// Return the owned bytes for this storage.
    fn owned_bytes(&self) -> usize {
        let mut owned_bytes = size_of::<Self>();
        owned_bytes += self.strings.capacity() * size_of::<Box<str>>();
        owned_bytes += self.ids.capacity() * size_of::<StringId>();
        owned_bytes += self.slot_by_id.capacity() * size_of::<(StringId, usize)>();
        owned_bytes += self.string_bytes();

        owned_bytes
    }

    /// Return the number of bytes stored in string allocations.
    fn string_bytes(&self) -> usize {
        self.strings.iter().map(|string| string.len()).sum()
    }

    /// Create the canonical serialized representation.
    fn to_serialized_data(&self) -> StringPoolData {
        let mut strings = self
            .iter()
            .map(|(id, text)| (id, text.to_string()))
            .collect::<Vec<_>>();
        strings.sort_by_key(|(id, _)| *id);

        StringPoolData { strings }
    }

    /// Build one pool from its canonical serialized representation.
    fn from_serialized_data(data: StringPoolData) -> Result<Self, String> {
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
        let slot = self.strings.len();
        self.strings.push(Box::<str>::from(text));
        self.ids.push(id);
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
            let existing = &self.strings[slot];
            assert_eq!(
                existing.as_ref(),
                text,
                "string id collision for {id}: existing {existing:?}, new {text:?}",
            );

            return;
        }

        self.insert_verified(id, text);
    }
}

/// Thread-safe string interning with stable append-only references.
pub struct StringPool {
    inner: RwLock<StringStorage>,
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
        let state = StringStorage::deserialize(deserializer)?;

        Ok(Self {
            inner: RwLock::new(state),
        })
    }
}

impl StringPool {
    /// Create a new empty StringPool.
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(StringStorage::new()),
        }
    }

    /// Create a new StringPool with pre-allocated capacity.
    pub fn with_capacity(string_count: usize, total_bytes: usize) -> Self {
        Self {
            inner: RwLock::new(StringStorage::with_capacity(string_count, total_bytes)),
        }
    }

    /// Reserve room for additional interned strings.
    #[inline]
    pub fn reserve(&self, string_count: usize) {
        let mut state = self.inner.write();

        state.reserve(string_count);
    }

    /// Check if the pool contains the given StringId.
    #[inline]
    pub fn contains(&self, id: StringId) -> bool {
        let state = self.inner.read();

        state.contains(id)
    }

    /// Get the string associated with the given StringId.
    #[inline]
    pub fn get(&self, id: StringId) -> &str {
        let ptr = {
            let state = self.inner.read();

            state.get_ptr(id)
        };

        // SAFETY: interned strings are append-only boxed allocations owned by this pool
        unsafe { &*ptr }
    }

    /// Return the string associated with the given StringId when present.
    #[inline]
    pub fn get_maybe(&self, id: StringId) -> Option<&str> {
        let ptr = {
            let state = self.inner.read();

            state.get_maybe_ptr(id)
        }?;

        // SAFETY: interned strings are append-only boxed allocations owned by this pool
        Some(unsafe { &*ptr })
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
        let text = other.get(string_id);
        let mut state = self.inner.write();

        state.ensure_text(string_id, text);
    }

    /// Ensure this pool contains every string from another pool.
    pub fn ensure_all_from(&self, other: &StringPool) {
        if std::ptr::eq(self, other) {
            return;
        }

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

    /// Iterate stored strings by dense allocation order.
    pub fn iter(&self) -> Vec<(StringId, &str)> {
        let ptrs = {
            let state = self.inner.read();
            state
                .iter()
                .map(|(id, text)| (id, text as *const str))
                .collect::<Vec<_>>()
        };

        ptrs.into_iter()
            .map(|(id, ptr)| {
                // SAFETY: interned strings are append-only boxed allocations owned by this pool
                (id, unsafe { &*ptr })
            })
            .collect()
    }

    /// Return the owned bytes for this pool.
    pub fn owned_bytes(&self) -> usize {
        let state = self.inner.read();

        size_of::<Self>() + state.owned_bytes()
    }
}

/// Return the closest candidate within a maximum edit distance.
pub fn closest_string<C>(
    value: &str,
    candidates: impl IntoIterator<Item = C>,
    max_distance: usize,
) -> Option<C>
where
    C: AsRef<str>,
{
    let value = value.chars().collect::<Vec<_>>();
    let mut closest: Option<(usize, C)> = None;

    // keep only close non-identical candidates
    for candidate in candidates {
        let distance =
            edit_distance_chars_at_most(value.as_slice(), candidate.as_ref(), max_distance);
        let Some(distance) = distance else {
            continue;
        };
        if distance == 0 {
            continue;
        }

        if closest.as_ref().is_none_or(|(best, _)| distance < *best) {
            closest = Some((distance, candidate));
        }
    }

    closest.map(|(_, candidate)| candidate)
}

/// Return the bounded edit distance from pre-collected left characters.
fn edit_distance_chars_at_most(left: &[char], right: &str, max_distance: usize) -> Option<usize> {
    let right = right.chars().collect::<Vec<_>>();
    let length_difference = left.len().abs_diff(right.len());
    if length_difference > max_distance {
        return None;
    }

    // compute classic two-row levenshtein distance
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    let mut current = vec![0; right.len() + 1];
    for (row, left) in left.iter().enumerate() {
        current[0] = row + 1;
        let mut row_minimum = current[0];

        for (column, right) in right.iter().enumerate() {
            let substitution = previous[column] + usize::from(left != right);
            current[column + 1] = substitution
                .min(previous[column + 1] + 1)
                .min(current[column] + 1);
            row_minimum = row_minimum.min(current[column + 1]);
        }

        if row_minimum > max_distance {
            return None;
        }

        std::mem::swap(&mut previous, &mut current);
    }

    let distance = previous[right.len()];
    (distance <= max_distance).then_some(distance)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_closest_string_ignores_identical_candidates() {
        let candidates = ["value", "valu", "other"];

        assert_eq!(closest_string("value", candidates, 1), Some("valu"));
    }

    #[test]
    fn test_closest_string_respects_distance() {
        let candidates = ["kitten", "sitting", "distance"];

        assert_eq!(closest_string("kitten", candidates, 2), None);
        assert_eq!(closest_string("kitten", candidates, 3), Some("sitting"));
    }

    #[test]
    fn test_closest_string_returns_borrowed_candidate() {
        let candidates = ["alpha", "alhpa", "omega"];

        assert_eq!(closest_string("alpha", candidates, 2), Some("alhpa"));
        assert_eq!(closest_string("alpha", candidates, 1), None);
    }

    #[test]
    fn test_pool_intern_same_string_yields_same_id() {
        let pool = StringPool::new();
        let a = pool.intern("hello");
        let b = pool.intern("hello");

        assert_eq!(a, b);
    }

    #[test]
    fn test_pool_intern_distinct_strings() {
        let pool = StringPool::new();
        let a = pool.intern("alpha");
        let b = pool.intern("beta");

        assert_ne!(a, b);
        assert_eq!(pool.get(a), "alpha");
        assert_eq!(pool.get(b), "beta");
    }

    #[test]
    fn test_pool_intern_empty_string() {
        let pool = StringPool::new();
        let id1 = pool.intern("");
        let id2 = pool.intern("");

        assert_eq!(id1, id2);
        assert_eq!(pool.get(id1), "");
    }

    #[test]
    fn test_pool_get_maybe_missing_string() {
        let pool = StringPool::new();
        let id = StringId::for_text("missing");

        assert_eq!(pool.get_maybe(id), None);
    }

    #[test]
    fn test_pool_clone_copies_content() {
        let pool = StringPool::new();
        let id = pool.intern("hello");
        let cloned = pool.clone();

        assert_eq!(cloned.get(id), "hello");
    }

    #[test]
    fn test_pool_ensure_all_from() {
        let first = StringPool::new();
        first.intern("alpha");
        first.intern("beta");

        let second = StringPool::new();
        second.ensure_all_from(&first);

        assert_eq!(second.get(StringId::for_text("alpha")), "alpha");
        assert_eq!(second.get(StringId::for_text("beta")), "beta");
    }
}
