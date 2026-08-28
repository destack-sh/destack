use destack_core::{
    SectionBuilder, SectionEntry, SectionImage, SectionImageError, SectionSlice, StringId,
};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Section-backed program string table.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct StringTable {
    /// String entries sorted by stable string id.
    entries: SectionSlice<StringEntry>,
    /// Concatenated UTF-8 string bytes.
    bytes: SectionSlice<u8>,
}

impl StringTable {
    /// Pack prepared string entries and bytes into program sections.
    pub(crate) fn pack(
        sections: &mut SectionBuilder,
        entries: Vec<StringEntry>,
        bytes: Vec<u8>,
    ) -> Self {
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
        let bytes = &bytes[start..end];

        // SAFETY: builders originate from Rust strings and Program loading checks every entry.
        Some(unsafe { std::str::from_utf8_unchecked(bytes) })
    }

    /// Return whether this table is empty.
    pub fn is_empty(&self, sections: SectionImage<'_>) -> bool {
        sections.entries(self.entries).is_empty()
    }

    /// Return whether this table contains one string id.
    pub(super) fn contains(&self, sections: SectionImage<'_>, id: StringId) -> bool {
        self.entry(sections, id).is_some()
    }

    /// Validate every string entry and its byte range.
    pub(super) fn validate(&self, sections: SectionImage<'_>) -> Result<(), SectionImageError> {
        let bytes = sections.entries(self.bytes);
        let entries = sections.entries(self.entries);

        // validate strict identity order for binary search
        if !entries
            .windows(2)
            .all(|entries| entries[0].id < entries[1].id)
        {
            return Err(SectionImageError::InvalidOrder);
        }

        // validate each UTF-8 byte range
        for entry in entries {
            let Some(end) = entry.offset.checked_add(entry.byte_len) else {
                return Err(SectionImageError::InvalidRange);
            };
            let Some(text) = bytes.get(entry.offset as usize..end as usize) else {
                return Err(SectionImageError::InvalidRange);
            };
            if std::str::from_utf8(text).is_err() {
                return Err(SectionImageError::InvalidString);
            }
        }

        Ok(())
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct StringEntry {
    /// Stable string identity.
    pub id: StringId,
    /// Byte offset in the string byte section.
    pub offset: u32,
    /// UTF-8 byte length.
    pub byte_len: u32,
}
