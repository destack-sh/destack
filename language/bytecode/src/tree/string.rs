use destack_core::{EntryRange, SectionEntry, StringId};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// One UTF-8 string stored in a bytecode object.
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct StringEntry {
    /// The stable content identity.
    pub id: StringId,
    /// The UTF-8 bytes in the object string byte section.
    pub bytes: EntryRange<u8>,
}

impl StringEntry {
    /// Borrow this string's UTF-8 bytes.
    pub fn bytes<'a>(&self, bytes: &'a [u8]) -> &'a [u8] {
        self.bytes.slice(bytes)
    }
}

const _: () = assert!(size_of::<StringEntry>() == 16);
