use std::any::TypeId;
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::{Mutex, OnceLock};

/// One generated static atom set.
pub(crate) trait StaticAtomSet: 'static {
    /// Return the generated atom set.
    fn get() -> &'static StaticAtomSetData;

    /// Return the static empty-string index.
    fn empty_string_index() -> u32;
}

/// One generated atom set payload.
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct StaticAtomSetData {
    /// The generated perfect-hash key.
    pub(crate) key: u64,
    /// The generated perfect-hash displacements.
    pub(crate) disps: &'static [(u32, u32)],
    /// The generated atom strings.
    pub(crate) atoms: &'static [&'static str],
    /// The generated perfect-hash hashes.
    pub(crate) hashes: &'static [u32],
}

/// One atom storage strategy.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum AtomStorageKind {
    /// One generated static atom.
    Static,
    /// One short inline atom.
    Inline,
    /// One dynamically interned atom.
    Interned,
}

/// One local replacement for the old generated string-cache atom.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Atom<Set> {
    /// The storage kind.
    kind: AtomStorageKind,
    /// The inline string length.
    inline_length: u8,
    /// The packed data or interned id.
    data: u64,
    /// The marker for the generated static set.
    marker: PhantomData<Set>,
}

/// One dynamic interner for atom values outside the generated set.
#[derive(Default)]
struct AtomInterner {
    /// The atom values in insertion order.
    values: Vec<Box<str>>,
    /// The reverse lookup map.
    indices: HashMap<Box<str>, u32>,
}

static ATOM_INTERNERS: OnceLock<Mutex<HashMap<TypeId, AtomInterner>>> = OnceLock::new();

impl<Set> Atom<Set> {
    /// Build one generated static atom.
    pub(crate) const fn pack_static(index: u32) -> Self {
        Self {
            kind: AtomStorageKind::Static,
            inline_length: 0,
            data: index as u64,
            marker: PhantomData,
        }
    }

    /// Build one generated inline atom.
    pub(crate) const fn pack_inline(data: u64, inline_length: u8) -> Self {
        Self {
            kind: AtomStorageKind::Inline,
            inline_length,
            data,
            marker: PhantomData,
        }
    }

    /// Build one interned atom.
    const fn pack_interned(index: u32) -> Self {
        Self {
            kind: AtomStorageKind::Interned,
            inline_length: 0,
            data: index as u64,
            marker: PhantomData,
        }
    }
}

impl<Set: StaticAtomSet> Atom<Set> {
    /// Return whether this atom is the empty string.
    pub(crate) fn is_empty(&self) -> bool {
        match self.kind {
            AtomStorageKind::Static => self.data as u32 == Set::empty_string_index(),
            AtomStorageKind::Inline => self.inline_length == 0,
            AtomStorageKind::Interned => self.with_str(|value| value.is_empty()),
        }
    }

    /// Return one owned string for this atom.
    pub(crate) fn to_owned_string(&self) -> String {
        self.with_str(str::to_owned)
    }

    /// Return whether this atom matches one string exactly.
    pub(crate) fn eq_str(&self, other: &str) -> bool {
        self.with_str(|value| value == other)
    }

    /// Return whether this atom matches one other atom case-insensitively.
    pub(crate) fn eq_ignore_ascii_case(&self, other: &Self) -> bool {
        let value = self.to_owned_string();
        let other = other.to_owned_string();

        value.eq_ignore_ascii_case(&other)
    }

    /// Run one closure against this atom string.
    fn with_str<T>(&self, callback: impl FnOnce(&str) -> T) -> T {
        match self.kind {
            // generated atoms
            AtomStorageKind::Static => {
                let index = self.data as usize;
                callback(Set::get().atoms[index])
            }

            // inline atoms
            AtomStorageKind::Inline => {
                let value = decode_inline_atom(self.data, self.inline_length);
                callback(&value)
            }

            // interned atoms
            AtomStorageKind::Interned => {
                let interner = get_atom_interner::<Set>();
                let interner = interner.lock().unwrap_or_else(|poison| poison.into_inner());
                let index = self.data as usize;
                let value = interner
                    .get(&TypeId::of::<Set>())
                    .and_then(|set| set.values.get(index))
                    .map(|value| value.as_ref())
                    .unwrap_or_default();

                callback(value)
            }
        }
    }
}

impl<Set: StaticAtomSet> From<&str> for Atom<Set> {
    /// Convert one string slice into one atom.
    fn from(value: &str) -> Self {
        // short names
        if let Some(atom) = encode_inline_atom(value) {
            return atom;
        }

        // generated set
        if let Some(index) = Set::get().atoms.iter().position(|atom| *atom == value) {
            return Self::pack_static(index as u32);
        }

        // dynamic interner
        let interner = get_atom_interner::<Set>();
        let mut interner = interner.lock().unwrap_or_else(|poison| poison.into_inner());
        let set = interner.entry(TypeId::of::<Set>()).or_default();

        if let Some(index) = set.indices.get(value) {
            return Self::pack_interned(*index);
        }

        let index = set.values.len() as u32;
        let owned_value: Box<str> = value.into();
        set.values.push(owned_value.clone());
        set.indices.insert(owned_value, index);

        Self::pack_interned(index)
    }
}

impl<Set: StaticAtomSet> From<String> for Atom<Set> {
    /// Convert one owned string into one atom.
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl<Set: StaticAtomSet> fmt::Display for Atom<Set> {
    /// Format this atom for display.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.with_str(|value| formatter.write_str(value))
    }
}

impl<Set: StaticAtomSet> fmt::Debug for Atom<Set> {
    /// Format this atom for debugging.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.with_str(|value| formatter.write_str(value))
    }
}

/// Return the shared interner map for one atom set.
fn get_atom_interner<Set: 'static>() -> &'static Mutex<HashMap<TypeId, AtomInterner>> {
    let _ = TypeId::of::<Set>();

    ATOM_INTERNERS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Decode one inline atom payload.
fn decode_inline_atom(data: u64, inline_length: u8) -> String {
    let packed = data >> 8;
    let mut bytes = Vec::with_capacity(inline_length as usize);

    // unpack bytes
    for index in 0..inline_length {
        let shift = u32::from(index) * 8;
        let byte = ((packed >> shift) & 0xFF) as u8;
        bytes.push(byte);
    }

    // SAFETY: inline atoms are always packed from valid UTF 8 input
    unsafe { String::from_utf8_unchecked(bytes) }
}

/// Encode one short atom inline when possible.
fn encode_inline_atom<Set>(value: &str) -> Option<Atom<Set>> {
    let bytes = value.as_bytes();

    // inline size limit
    if bytes.len() > 7 {
        return None;
    }

    let mut packed = 0u64;

    // packed bytes
    for (index, byte) in bytes.iter().copied().enumerate() {
        let shift = (index as u32) * 8;
        packed |= u64::from(byte) << shift;
    }

    let inline_length = bytes.len() as u8;
    let data = packed << 8;

    Some(Atom::pack_inline(data, inline_length))
}

// generated atom tables and macros
#[macro_use]
#[path = "atom.generated.rs"]
mod generated;

pub(crate) use generated::{LocalName, Namespace, Prefix, *};
pub(crate) use {local_name, namespace_prefix, namespace_url, ns};
