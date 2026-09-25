use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self, Debug, Formatter};
use std::mem::size_of;
use tspp_serde::{Reflect, Schema, Type};

use crate::{StableHasher, stable_hash_text};

const LOCAL_INDEX_MIN_CAPACITY: usize = 16;
const LOCAL_INDEX_EMPTY_SLOT: u32 = u32::MAX;
const LOCAL_INDEX_LOAD_NUMERATOR: usize = 3;
const LOCAL_INDEX_LOAD_DENOMINATOR: usize = 4;

/// Stable content identity for one interned string.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize, Reflect)]
pub struct StringId(pub u64);

impl Debug for StringId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "#{:016x}", self.0)
    }
}

impl std::fmt::Display for StringId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "#{:016x}", self.0)
    }
}

impl StringId {
    /// Create the stable id for one string.
    #[inline]
    pub fn for_text(text: &str) -> Self {
        Self(stable_hash_text(text))
    }

    /// Return the raw stable hash bits.
    #[inline]
    pub fn raw(self) -> u64 {
        self.0
    }
}

/// One string in local contiguous storage.
#[derive(Debug, Copy, Clone)]
struct LocalStringEntry {
    /// The stable content identity.
    id: StringId,
    /// The byte offset in the string buffer.
    offset: u32,
    /// The string byte length.
    len: u32,
}

/// Content-addressed string storage owned by one operation.
#[derive(Clone, Default)]
pub struct LocalStringPool {
    /// The contiguous string bytes.
    buffer: String,
    /// The strings in insertion order.
    entries: Vec<LocalStringEntry>,
    /// The open-addressed entry slots by stable string ID.
    index: Vec<u32>,
}

impl Debug for LocalStringPool {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalStringPool")
            .field("length", &self.entries.len())
            .field("buffer_size", &self.buffer.len())
            .finish()
    }
}

impl LocalStringPool {
    /// Create an empty local string pool.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the string associated with one stable ID.
    #[inline]
    pub fn get(&self, id: StringId) -> &str {
        self.get_maybe(id)
            .unwrap_or_else(|| panic!("string id {id} is not present in this string pool"))
    }

    /// Return the string associated with one stable ID when present.
    #[inline]
    pub fn get_maybe(&self, id: StringId) -> Option<&str> {
        let slot = self.find_slot(id)? as usize;
        let entry = self.entries[slot];
        let start = entry.offset as usize;
        let end = start + entry.len as usize;

        Some(&self.buffer[start..end])
    }

    /// Intern one string and return its stable ID.
    #[inline]
    pub fn intern(&mut self, text: &str) -> StringId {
        let id = StringId::for_text(text);

        // return an existing string after checking the content identity
        if let Some(slot) = self.find_slot(id) {
            let entry = self.entries[slot as usize];
            let start = entry.offset as usize;
            let end = start + entry.len as usize;
            let existing = &self.buffer[start..end];
            assert_eq!(
                existing, text,
                "string id collision for {id}: existing {existing:?}, new {text:?}",
            );

            return id;
        }

        // grow before the next insertion would exceed the target load
        let next_len = self.entries.len() + 1;
        if self.index.is_empty()
            || next_len * LOCAL_INDEX_LOAD_DENOMINATOR
                > self.index.len() * LOCAL_INDEX_LOAD_NUMERATOR
        {
            self.grow_index();
        }

        // append the string and index its stable identity
        debug_assert!(self.buffer.len() <= u32::MAX as usize);
        debug_assert!(text.len() <= u32::MAX as usize);
        debug_assert!(self.entries.len() < u32::MAX as usize);
        let offset = self.buffer.len() as u32;
        let len = text.len() as u32;
        let slot = self.entries.len() as u32;
        self.buffer.push_str(text);
        self.entries.push(LocalStringEntry { id, offset, len });
        self.insert_slot(id, slot);

        id
    }

    /// Iterate strings in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (StringId, &str)> + '_ {
        self.entries.iter().map(|entry| {
            let start = entry.offset as usize;
            let end = start + entry.len as usize;

            (entry.id, &self.buffer[start..end])
        })
    }

    /// Return the number of unique strings.
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return whether no strings are stored.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return the bytes owned by this pool.
    pub fn owned_bytes(&self) -> usize {
        let mut bytes = size_of::<Self>();
        bytes += self.buffer.capacity();
        bytes += self.entries.capacity() * size_of::<LocalStringEntry>();
        bytes += self.index.capacity() * size_of::<u32>();

        bytes
    }

    /// Return the entry slot for one stable string ID.
    #[inline]
    fn find_slot(&self, id: StringId) -> Option<u32> {
        if self.index.is_empty() {
            return None;
        }

        let mask = self.index.len() - 1;
        let mut index = Self::index_start(id, mask);
        loop {
            let slot = self.index[index];
            if slot == LOCAL_INDEX_EMPTY_SLOT {
                return None;
            }
            if self.entries[slot as usize].id == id {
                return Some(slot);
            }

            index = (index + 1) & mask;
        }
    }

    /// Insert one known-unique stable string ID into the local index.
    fn insert_slot(&mut self, id: StringId, slot: u32) {
        let mask = self.index.len() - 1;
        let mut index = Self::index_start(id, mask);
        while self.index[index] != LOCAL_INDEX_EMPTY_SLOT {
            index = (index + 1) & mask;
        }

        self.index[index] = slot;
    }

    /// Grow the local index and reinsert existing entry slots.
    fn grow_index(&mut self) {
        let new_capacity = if self.index.is_empty() {
            LOCAL_INDEX_MIN_CAPACITY
        } else {
            self.index.len() * 2
        };
        self.index = vec![LOCAL_INDEX_EMPTY_SLOT; new_capacity];

        for slot in 0..self.entries.len() {
            let id = self.entries[slot].id;
            self.insert_slot(id, slot as u32);
        }
    }

    /// Return the first local index position for one stable string ID.
    #[inline]
    fn index_start(id: StringId, mask: usize) -> usize {
        id.raw() as usize & mask
    }
}

/// A borrowed string from a pool.
pub type StringRef<'a> = &'a str;

/// Serialized string pool representation.
#[derive(Serialize, Deserialize, Reflect)]
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

impl Reflect for StringPool {
    /// Register the serialized string pool shape.
    fn reflect(schema: &mut Schema) -> Type {
        StringPoolData::reflect(schema)
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
            if let Some(existing) = state.get_maybe(id) {
                assert_eq!(
                    existing, text,
                    "string id collision for {id}: existing {existing:?}, new {text:?}",
                );

                return id;
            }
        }

        let mut state = self.inner.write();

        if let Some(existing) = state.get_maybe(id) {
            assert_eq!(
                existing, text,
                "string id collision for {id}: existing {existing:?}, new {text:?}",
            );

            return id;
        }

        state.insert_verified(id, text);

        id
    }

    /// Add all strings from a local pool.
    pub fn extend(&self, strings: &LocalStringPool) {
        let mut state = self.inner.write();

        // merge unique strings while preserving content identities
        for (id, text) in strings.iter() {
            if let Some(existing) = state.get_maybe(id) {
                assert_eq!(
                    existing, text,
                    "string id collision for {id}: existing {existing:?}, new {text:?}",
                );
            } else {
                state.insert_verified(id, text);
            }
        }
    }

    /// Ensure this pool contains one string from another pool.
    #[inline]
    pub fn ensure_from(&self, other: &StringPool, string_id: StringId) {
        let text = other.get(string_id);
        let mut state = self.inner.write();

        state.ensure_text(string_id, text);
    }

    /// Ensure this pool contains one exact string.
    #[inline]
    pub fn ensure(&self, string_id: StringId, text: &str) {
        let mut state = self.inner.write();

        state.ensure_text(string_id, text);
    }

    /// Ensure this pool contains every exact string in one sequence.
    pub fn ensure_all<'a>(&self, strings: impl IntoIterator<Item = (StringId, &'a str)>) {
        let mut state = self.inner.write();
        for (string_id, text) in strings {
            state.ensure_text(string_id, text);
        }
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

/// How strongly one name match resembles the searched name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NameMatchTier {
    /// Same name up to letter case.
    Case,
    /// Same name up to case and `_`/`-` separators.
    Separators,
    /// Within the edit distance budget.
    Edit(usize),
    /// One name is a case-insensitive prefix of the other.
    Prefix,
}

/// One ranked candidate for a misspelled name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NameMatch<C> {
    /// The matched candidate.
    pub candidate: C,
    /// The strength of the match.
    pub tier: NameMatchTier,
    /// Whether no other candidate matched at the same tier.
    pub is_unique: bool,
}

/// Return the best candidate for a misspelled name.
///
/// Candidate order breaks ties: earlier candidates win, so callers pass
/// nearer scopes first.
pub fn find_best_match<C>(
    value: &str,
    candidates: impl IntoIterator<Item = C>,
    max_distance: usize,
) -> Option<NameMatch<C>>
where
    C: AsRef<str>,
{
    let chars = value.chars().collect::<Vec<_>>();
    let folded = fold_case(value);
    let stripped = strip_separators(&folded);
    let mut best: Option<NameMatch<C>> = None;

    for candidate in candidates {
        // skip the identical name
        let text = candidate.as_ref();
        if text == value {
            continue;
        }

        // rank the candidate at its strongest tier
        let candidate_folded = fold_case(text);
        let tier = if candidate_folded == folded {
            // same name up to letter case
            NameMatchTier::Case
        } else if strip_separators(&candidate_folded) == stripped {
            // same name up to case and separators
            NameMatchTier::Separators
        } else if let Some(distance) =
            edit_distance_chars_at_most(&chars, text, max_distance).filter(|distance| *distance > 0)
        {
            // close enough within the edit budget
            NameMatchTier::Edit(distance)
        } else if chars.len() >= 3
            && text.chars().count() >= 3
            && (candidate_folded.starts_with(&folded) || folded.starts_with(&candidate_folded))
        {
            // one name is a case-insensitive prefix of the other
            NameMatchTier::Prefix
        } else {
            // no meaningful resemblance
            continue;
        };

        match &mut best {
            // stronger tier replaces the current best
            Some(best) if tier < best.tier => {
                *best = NameMatch {
                    candidate,
                    tier,
                    is_unique: true,
                };
            }
            // another candidate ties the current best tier
            Some(best) if tier == best.tier => best.is_unique = false,
            // weaker tier keeps the current best
            Some(_) => {}
            // first matching candidate seeds the best
            None => {
                best = Some(NameMatch {
                    candidate,
                    tier,
                    is_unique: true,
                });
            }
        }
    }

    best
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
    find_best_match(value, candidates, max_distance).map(|best| best.candidate)
}

/// Lowercase one name for case-insensitive comparison.
fn fold_case(value: &str) -> String {
    value.to_lowercase()
}

/// Drop `_` and `-` separators from one case-folded name.
fn strip_separators(value: &str) -> String {
    value.chars().filter(|c| *c != '_' && *c != '-').collect()
}

/// Return the bounded edit distance from pre-collected left characters.
fn edit_distance_chars_at_most(left: &[char], right: &str, max_distance: usize) -> Option<usize> {
    // reject when the length gap alone exceeds the budget
    let right = right.chars().collect::<Vec<_>>();
    let length_difference = left.len().abs_diff(right.len());
    if length_difference > max_distance {
        return None;
    }

    // compute three-row optimal string alignment distance
    let mut before = vec![0; right.len() + 1];
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    let mut current = vec![0; right.len() + 1];
    for (row, left_char) in left.iter().enumerate() {
        current[0] = row + 1;
        let mut row_minimum = current[0];

        for (column, right_char) in right.iter().enumerate() {
            // take the cheapest of substitution, insertion, and deletion
            let substitution = previous[column] + usize::from(left_char != right_char);
            let mut best = substitution
                .min(previous[column + 1] + 1)
                .min(current[column] + 1);

            // count one adjacent transposition as a single edit
            if row > 0
                && column > 0
                && *left_char == right[column - 1]
                && left[row - 1] == *right_char
            {
                best = best.min(before[column - 1] + 1);
            }
            current[column + 1] = best;
            row_minimum = row_minimum.min(best);
        }

        // stop once the whole row exceeds the budget
        if row_minimum > max_distance {
            return None;
        }

        std::mem::swap(&mut before, &mut previous);
        std::mem::swap(&mut previous, &mut current);
    }

    let distance = previous[right.len()];
    (distance <= max_distance).then_some(distance)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_best_match_prefers_case_over_edits() {
        let candidates = ["jsom", "JSON", "Jason"];

        let best = find_best_match("json", candidates, 2).unwrap();
        assert_eq!(best.candidate, "JSON");
        assert_eq!(best.tier, NameMatchTier::Case);
        assert!(best.is_unique);
    }

    #[test]
    fn test_find_best_match_bridges_separator_styles() {
        let candidates = ["fooBaz", "foo_bar"];

        let best = find_best_match("fooBar", candidates, 1).unwrap();
        assert_eq!(best.candidate, "foo_bar");
        assert_eq!(best.tier, NameMatchTier::Separators);
    }

    #[test]
    fn test_find_best_match_marks_tier_ties() {
        let candidates = ["valye", "valus"];

        let best = find_best_match("value", candidates, 2).unwrap();
        assert_eq!(best.candidate, "valye");
        assert!(!best.is_unique);
    }

    #[test]
    fn test_find_best_match_falls_back_to_prefixes() {
        let candidates = ["configuration_reload_interval"];

        let best = find_best_match("configuration", candidates, 4).unwrap();
        assert_eq!(best.tier, NameMatchTier::Prefix);
    }

    #[test]
    fn test_find_best_match_skips_short_prefixes() {
        let candidates = ["about"];

        assert!(find_best_match("ab", candidates, 1).is_none());
    }

    #[test]
    fn test_find_best_match_counts_transpositions_once() {
        let candidates = ["value"];

        let best = find_best_match("valeu", candidates, 1).unwrap();
        assert_eq!(best.tier, NameMatchTier::Edit(1));
    }

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
        assert_eq!(closest_string("alpha", candidates, 1), Some("alhpa"));
        assert_eq!(closest_string("alpine", candidates, 1), None);
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
    fn test_local_pool_interns_unique_strings() {
        let mut pool = LocalStringPool::new();
        let alpha = pool.intern("alpha");
        let beta = pool.intern("beta");

        assert_eq!(pool.intern("alpha"), alpha);
        assert_eq!(pool.get(alpha), "alpha");
        assert_eq!(pool.get(beta), "beta");
        assert_eq!(
            pool.iter().collect::<Vec<_>>(),
            [(alpha, "alpha"), (beta, "beta")]
        );
        assert_eq!(pool.len(), 2);
    }

    /// Preserve every local string across index growth.
    #[test]
    fn test_local_pool_grows_index() {
        let mut pool = LocalStringPool::new();
        let strings = (0..128)
            .map(|index| format!("identifier_{index}"))
            .collect::<Vec<_>>();
        let ids = strings
            .iter()
            .map(|string| pool.intern(string))
            .collect::<Vec<_>>();

        for (id, string) in ids.into_iter().zip(strings) {
            assert_eq!(pool.intern(&string), id);
            assert_eq!(pool.get(id), string);
        }
    }

    #[test]
    fn test_pool_extends_from_local_strings() {
        let mut local = LocalStringPool::new();
        let alpha = local.intern("alpha");
        let beta = local.intern("beta");

        let pool = StringPool::new();
        pool.intern("alpha");
        pool.extend(&local);

        assert_eq!(pool.get(alpha), "alpha");
        assert_eq!(pool.get(beta), "beta");
        assert_eq!(pool.len(), 2);
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
