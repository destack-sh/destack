use destack_core::{SectionEntry, SectionImage, SectionPacker, SectionSlice, StringId, StringPool};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Section-backed program string table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct StringTable {
    /// String entries sorted by stable string id.
    entries: SectionSlice<StringEntry>,
    /// Concatenated UTF-8 string bytes.
    bytes: SectionSlice<u8>,
}

impl StringTable {
    /// Build program strings from one compiler string pool.
    pub fn from_pool(sections: &mut SectionPacker, pool: &StringPool) -> Self {
        let mut entries = Vec::with_capacity(pool.len());
        let mut bytes = Vec::new();
        let mut strings = pool
            .iter()
            .into_iter()
            .map(|(id, text)| (id, text.to_owned()))
            .collect::<Vec<_>>();
        strings.sort_by_key(|(id, _)| *id);

        for (id, text) in strings {
            let offset = bytes.len() as u32;
            let byte_len = text.len() as u32;
            bytes.extend_from_slice(text.as_bytes());
            entries.push(StringEntry {
                id,
                offset,
                byte_len,
            });
        }

        Self {
            entries: sections.insert(entries),
            bytes: sections.insert(bytes),
        }
    }

    /// Return one string by stable id when present.
    pub fn string<'a>(&self, sections: SectionImage<'a>, id: StringId) -> Option<&'a str> {
        let entry = self.entry(sections, id)?;
        let bytes = sections.entries(self.bytes);
        let start = entry.offset as usize;
        let end = start + entry.byte_len as usize;
        let bytes = bytes.get(start..end)?;

        std::str::from_utf8(bytes).ok()
    }

    /// Return whether this table is empty.
    pub fn is_empty(&self, sections: SectionImage<'_>) -> bool {
        sections.entries(self.entries).is_empty()
    }

    /// Return one string entry by stable id.
    fn entry<'a>(&self, sections: SectionImage<'a>, id: StringId) -> Option<&'a StringEntry> {
        let entries = sections.entries(self.entries);
        let index = entries.binary_search_by_key(&id, |entry| entry.id).ok()?;

        entries.get(index)
    }
}

/// One program string table entry.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct StringEntry {
    /// Stable string identity.
    pub id: StringId,
    /// Byte offset in the string byte section.
    pub offset: u32,
    /// UTF-8 byte length.
    pub byte_len: u32,
}

// SAFETY: StringEntry is a plain entry containing stable ids and byte offsets.
unsafe impl SectionEntry for StringEntry {}
