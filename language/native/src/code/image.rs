use destack_core::{SectionBuilder, SectionEntry};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::{Archive, ArchiveBuilder, Library, LibraryBuilder};

/// Durable native image payload.
#[repr(C, u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub enum Image {
    /// Native functions are already linked into the current process.
    Resident,
    /// Native functions live in a loadable native library.
    Library(Library),
    /// Native functions live in a packed relocatable archive.
    Archive(Archive),
}

/// Native image before Program section packing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageBuilder {
    /// Native functions are already linked into the current process.
    Resident,
    /// Native functions live in a loadable native library.
    Library(LibraryBuilder),
    /// Native functions live in a packed relocatable archive.
    Archive(ArchiveBuilder),
}

impl ImageBuilder {
    /// Pack this native image into Program sections.
    pub(super) fn build(self, sections: &mut SectionBuilder) -> Image {
        match self {
            Self::Resident => Image::Resident,
            Self::Library(library) => Image::Library(library.build(sections)),
            Self::Archive(archive) => Image::Archive(archive.build(sections)),
        }
    }
}
