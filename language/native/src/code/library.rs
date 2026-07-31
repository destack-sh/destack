use destack_core::{SectionBuilder, SectionEntry, SectionImage, SectionSlice};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Loadable native library image.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Library {
    /// Encoded native library bytes.
    image: SectionSlice<u8>,
    /// Optional native unwind table bytes.
    unwind: SectionSlice<u8>,
}

impl Library {
    /// Return the encoded native library bytes.
    pub fn image<'a>(self, sections: SectionImage<'a>) -> &'a [u8] {
        sections.entries(self.image)
    }

    /// Return native unwind tables when present.
    pub fn unwind<'a>(self, sections: SectionImage<'a>) -> Option<&'a [u8]> {
        if self.unwind.is_empty() {
            None
        } else {
            Some(sections.entries(self.unwind))
        }
    }
}

/// Native library before Program section packing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryBuilder {
    /// Encoded native library bytes.
    image: Vec<u8>,
    /// Optional native unwind table bytes.
    unwind: Vec<u8>,
}

impl LibraryBuilder {
    /// Create one native library builder.
    pub fn new(image: impl Into<Vec<u8>>) -> Self {
        Self {
            image: image.into(),
            unwind: Vec::new(),
        }
    }

    /// Set native unwind table bytes.
    pub fn unwind(mut self, unwind: impl Into<Vec<u8>>) -> Self {
        self.unwind = unwind.into();

        self
    }

    /// Pack this native library into Program sections.
    pub(super) fn build(self, sections: &mut SectionBuilder) -> Library {
        Library {
            image: sections.insert(self.image),
            unwind: sections.insert(self.unwind),
        }
    }
}
