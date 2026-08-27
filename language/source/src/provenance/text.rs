use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::mem;
use std::num::NonZeroU32;

use destack_core::{SectionEntry, SectionImageError, SectionLoader};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::ByteRange;

use super::{ProvenanceId, ProvenanceRemap};

/// An error produced while composing emitted text maps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextMapError {
    /// The combined name table exceeds its index representation.
    NameCountOverflow,
    /// An emitted byte offset exceeds its range representation.
    ByteOffsetOverflow,
    /// An extent references provenance outside the imported table.
    ProvenanceOutOfRange {
        /// The invalid provenance id.
        provenance: ProvenanceId,
    },
    /// An extent references a name outside its text map.
    NameOutOfRange {
        /// The invalid name id.
        name: TextNameId,
    },
}

impl Display for TextMapError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NameCountOverflow => {
                write!(formatter, "text map name count exceeds the supported range")
            }
            Self::ByteOffsetOverflow => {
                write!(
                    formatter,
                    "text map byte offset exceeds the supported range"
                )
            }
            Self::ProvenanceOutOfRange { provenance } => write!(
                formatter,
                "provenance {} is outside the imported table",
                provenance.index()
            ),
            Self::NameOutOfRange { name } => {
                write!(
                    formatter,
                    "text name {} is outside the appended name table",
                    name.index()
                )
            }
        }
    }
}

impl Error for TextMapError {}

/// One interned name in an emitted text map.
#[repr(transparent)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
pub struct TextNameId(NonZeroU32);

impl TextNameId {
    /// Create a text name id from its zero based table index.
    pub const fn new(index: u32) -> Self {
        let Some(raw) = index.checked_add(1) else {
            panic!("text name table exhausted its id space");
        };
        let Some(raw) = NonZeroU32::new(raw) else {
            unreachable!();
        };

        Self(raw)
    }

    /// Return the zero based table index.
    pub const fn index(self) -> u32 {
        self.0.get() - 1
    }
}

/// One emitted text range associated with provenance.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TextExtent {
    /// The emitted UTF-8 byte range.
    pub generated: ByteRange,
    /// The compilation provenance represented by the range.
    pub provenance: ProvenanceId,
    /// The authored identifier name represented by the range.
    pub name: Option<TextNameId>,
}

// SAFETY: repr(C) fixes the field layout, Option<TextNameId> uses the NonZeroU32 null value,
// and validation rejects the only invalid field representation.
unsafe impl SectionEntry for TextExtent {
    const NEEDS_VALIDATION: bool = true;

    fn validate(bytes: &[u8], loader: SectionLoader<'_>) -> Result<(), SectionImageError> {
        if bytes.len() != mem::size_of::<Self>() {
            return Err(SectionImageError::InvalidEntry);
        }
        let start = mem::offset_of!(Self, provenance);
        let end = start + mem::size_of::<ProvenanceId>();

        ProvenanceId::validate(&bytes[start..end], loader)
    }
}

/// Provenance extents for one emitted text file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TextMap {
    /// Authored names referenced by emitted extents.
    pub names: Vec<String>,
    /// Nested and discontiguous emitted text extents.
    pub extents: Vec<TextExtent>,
}

impl TextMap {
    /// Append one remapped text map at the given emitted byte offset.
    pub fn append(
        &mut self,
        mut map: TextMap,
        remap: &ProvenanceRemap,
        byte_offset: u32,
    ) -> Result<(), TextMapError> {
        let name_start =
            u32::try_from(self.names.len()).map_err(|_| TextMapError::NameCountOverflow)?;
        let added_names =
            u32::try_from(map.names.len()).map_err(|_| TextMapError::NameCountOverflow)?;
        name_start
            .checked_add(added_names)
            .ok_or(TextMapError::NameCountOverflow)?;

        // remap the owned extents before changing this map
        for extent in &mut map.extents {
            extent.generated.start = extent
                .generated
                .start
                .checked_add(byte_offset)
                .ok_or(TextMapError::ByteOffsetOverflow)?;
            extent.generated.end = extent
                .generated
                .end
                .checked_add(byte_offset)
                .ok_or(TextMapError::ByteOffsetOverflow)?;
            extent.provenance =
                remap
                    .get(extent.provenance)
                    .ok_or(TextMapError::ProvenanceOutOfRange {
                        provenance: extent.provenance,
                    })?;
            if let Some(name) = extent.name {
                if name.index() >= added_names {
                    return Err(TextMapError::NameOutOfRange { name });
                }
                extent.name = Some(TextNameId::new(name_start + name.index()));
            }
        }

        self.names.append(&mut map.names);
        self.extents.append(&mut map.extents);

        Ok(())
    }

    /// Shift every emitted range by the given byte count.
    pub fn shift(&mut self, bytes: u32) -> Result<(), TextMapError> {
        if self.extents.iter().any(|extent| {
            extent.generated.start.checked_add(bytes).is_none()
                || extent.generated.end.checked_add(bytes).is_none()
        }) {
            return Err(TextMapError::ByteOffsetOverflow);
        }

        for extent in &mut self.extents {
            extent.generated = ByteRange {
                start: extent.generated.start + bytes,
                end: extent.generated.end + bytes,
            };
        }

        Ok(())
    }

    /// Truncate emitted ranges to the given text byte length.
    pub fn truncate(&mut self, bytes: u32) {
        self.extents.retain_mut(|extent| {
            extent.generated.end = extent.generated.end.min(bytes);

            extent.generated.start < extent.generated.end
        });
    }
}

const _: () = assert!(std::mem::size_of::<TextExtent>() == 16);
