use destack_core::{
    EntryRange, EntryStore, SectionBuilder, SectionEntry, SectionImage, SectionSlice,
};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::ObjectFormat;

/// Relocatable native objects packed into one Program image.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Archive {
    /// The native object format shared by every member.
    format: ObjectFormat,
    /// Member byte ranges in module link order.
    objects: SectionSlice<EntryRange<u8>>,
    /// Contiguous native object bytes.
    bytes: SectionSlice<u8>,
}

impl Archive {
    /// Return whether every object range fits the packed byte section.
    pub(super) fn ranges_fit(self, sections: SectionImage<'_>) -> bool {
        let byte_len = sections.entries(self.bytes).len();

        sections
            .entries(self.objects)
            .iter()
            .all(|object| object.fits(byte_len))
    }

    /// Return the native object format.
    pub const fn format(self) -> ObjectFormat {
        self.format
    }

    /// Return the packed object ranges.
    pub fn objects(self, sections: SectionImage<'_>) -> &[EntryRange<u8>] {
        sections.entries(self.objects)
    }

    /// Return one packed native object.
    pub fn object(self, sections: SectionImage<'_>, index: usize) -> Option<&[u8]> {
        let range = self.objects(sections).get(index)?;

        Some(range.slice(sections.entries(self.bytes)))
    }
}

/// Relocatable native archive before Program section packing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveBuilder {
    /// The native object format shared by every member.
    format: ObjectFormat,
    /// Native object bytes in module link order.
    objects: Vec<Vec<u8>>,
}

impl ArchiveBuilder {
    /// Create one empty native archive.
    pub fn new(format: ObjectFormat) -> Self {
        Self {
            format,
            objects: Vec::new(),
        }
    }

    /// Set native object bytes in module link order.
    pub fn objects(mut self, objects: impl IntoIterator<Item = Vec<u8>>) -> Self {
        self.objects = objects.into_iter().collect();

        self
    }

    /// Pack this archive into Program sections.
    pub(super) fn build(self, sections: &mut SectionBuilder) -> Archive {
        let mut bytes = EntryStore::new();

        // concatenate members while retaining exact boundaries
        let objects = self
            .objects
            .into_iter()
            .map(|object| bytes.append(object))
            .collect::<Vec<_>>();

        Archive {
            format: self.format,
            objects: sections.insert(objects),
            bytes: sections.insert(bytes.into_entries()),
        }
    }
}
