use destack_core::{
    EntryRange, SectionBuilder, SectionEntry, SectionImage, SectionImageError, SectionSlice,
};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::{Alignment, Import, RelocationKind, SymbolId};

/// Target-native unwind tables inside one relocatable object.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct ObjectUnwind {
    /// Platform unwind encoding.
    pub format: UnwindFormat,
    /// Target unwind sections.
    sections: SectionSlice<UnwindSection>,
    /// Flattened target unwind bytes.
    bytes: SectionSlice<u8>,
    /// Object-local unwind relocations.
    relocations: SectionSlice<UnwindRelocation>,
}

/// Fully linked target-native unwind tables inside one Program.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Unwind {
    /// Platform unwind encoding.
    pub format: UnwindFormat,
    /// Target unwind sections inside the native load image.
    sections: SectionSlice<UnwindSection>,
}

/// One target unwind section.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct UnwindSection {
    /// Platform section role.
    pub kind: UnwindSectionKind,
    /// Section bytes inside the owning native image.
    bytes: EntryRange<u8>,
    /// Required section alignment.
    pub alignment: Alignment,
}

/// One object-local relocation inside a target unwind section.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct UnwindRelocation {
    /// Source unwind section index.
    pub section: u32,
    /// Byte offset inside the source section.
    pub offset: u32,
    /// Object-local relocation target.
    pub target: UnwindTarget,
    /// Value added to the target address.
    pub addend: i64,
    /// Machine relocation encoding.
    pub kind: RelocationKind,
}

/// Target of one object-local unwind relocation.
#[repr(C, u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub enum UnwindTarget {
    /// Object-local native symbol.
    Symbol(SymbolId),
    /// Byte offset inside one object-local unwind section.
    Section {
        /// Object-local section index.
        section: u32,
        /// Byte offset inside the section.
        offset: u32,
    },
    /// Platform function imported by the unwind encoding.
    Import(Import),
}

/// Platform unwind encoding.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub enum UnwindFormat {
    /// DWARF call-frame information used by System V targets.
    Dwarf = 0,
    /// Windows x64 unwind information.
    WindowsX64 = 1,
    /// Windows ARM64 unwind information.
    WindowsArm64 = 2,
}

/// Platform role of one unwind section.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub enum UnwindSectionKind {
    /// DWARF `.eh_frame` call-frame information.
    DwarfFrame = 0,
    /// DWARF language-specific exception tables.
    DwarfException = 1,
    /// Windows runtime function entries.
    WindowsFunction = 2,
    /// Windows unwind records and language handler data.
    WindowsData = 3,
}

/// Object-local target unwind tables under construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectUnwindBuilder {
    /// Platform unwind encoding.
    format: UnwindFormat,
    /// Target unwind sections.
    sections: Vec<UnwindSectionBuilder>,
    /// Object-local unwind relocations.
    relocations: Vec<UnwindRelocation>,
}

/// Fully linked target unwind tables under construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnwindBuilder {
    /// Platform unwind encoding.
    format: UnwindFormat,
    /// Target unwind sections.
    sections: Vec<UnwindSectionBuilder>,
}

/// One target unwind section under construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnwindSectionBuilder {
    /// Platform section role.
    kind: UnwindSectionKind,
    /// Target unwind bytes.
    bytes: Vec<u8>,
    /// Required section alignment.
    alignment: Alignment,
}

impl ObjectUnwind {
    /// Validate every object unwind section and relocation.
    pub(super) fn validate(
        self,
        sections: SectionImage<'_>,
        symbols: usize,
    ) -> Result<(), SectionImageError> {
        let entries = self.sections(sections);
        let bytes = self.bytes(sections);
        for section in entries {
            section.validate(bytes.len())?;
        }

        // validate every relocation source and target
        for relocation in self.relocations(sections) {
            relocation.validate(entries, symbols)?;
        }

        Ok(())
    }

    /// Return target unwind sections.
    pub fn sections<'a>(self, sections: SectionImage<'a>) -> &'a [UnwindSection] {
        sections.entries(self.sections)
    }

    /// Return flattened target unwind bytes.
    pub fn bytes<'a>(self, sections: SectionImage<'a>) -> &'a [u8] {
        sections.entries(self.bytes)
    }

    /// Return object-local unwind relocations.
    pub fn relocations<'a>(self, sections: SectionImage<'a>) -> &'a [UnwindRelocation] {
        sections.entries(self.relocations)
    }
}

impl Unwind {
    /// Validate every linked unwind section against its load image.
    pub(super) fn validate(
        self,
        sections: SectionImage<'_>,
        image_len: usize,
    ) -> Result<(), SectionImageError> {
        for section in self.sections(sections) {
            section.validate(image_len)?;
        }

        Ok(())
    }

    /// Return target unwind sections.
    pub fn sections<'a>(self, sections: SectionImage<'a>) -> &'a [UnwindSection] {
        sections.entries(self.sections)
    }
}

impl UnwindSection {
    /// Validate this section against its owning native image.
    fn validate(self, bytes: usize) -> Result<(), SectionImageError> {
        if !self.alignment.is_valid() {
            return Err(SectionImageError::InvalidEntry);
        }
        if !self.bytes.start.is_multiple_of(self.alignment.bytes()) {
            return Err(SectionImageError::InvalidRange);
        }

        self.bytes.validate(bytes)
    }

    /// Return this section's byte offset inside its native image.
    pub const fn byte_offset(self) -> u32 {
        self.bytes.start
    }

    /// Return this section's target bytes.
    pub fn bytes(self, bytes: &[u8]) -> &[u8] {
        self.bytes.slice(bytes)
    }

    /// Return this section's target byte length.
    pub const fn byte_len(self) -> u32 {
        self.bytes.len
    }
}

impl UnwindRelocation {
    /// Create one object-local unwind relocation.
    pub const fn new(
        section: u32,
        offset: u32,
        target: UnwindTarget,
        addend: i64,
        kind: RelocationKind,
    ) -> Self {
        Self {
            section,
            offset,
            target,
            addend,
            kind,
        }
    }

    /// Validate this relocation against its source section and target table.
    fn validate(self, sections: &[UnwindSection], symbols: usize) -> Result<(), SectionImageError> {
        let Some(source) = sections.get(self.section as usize) else {
            return Err(SectionImageError::InvalidReference);
        };
        let Some(end) = self.offset.checked_add(self.kind.byte_len()) else {
            return Err(SectionImageError::InvalidRange);
        };
        if end > source.byte_len() {
            return Err(SectionImageError::InvalidRange);
        }

        // validate the relocation target
        match self.target {
            UnwindTarget::Symbol(symbol) if symbol.index() >= symbols => {
                return Err(SectionImageError::InvalidReference);
            }
            UnwindTarget::Section { section, .. } if section as usize >= sections.len() => {
                return Err(SectionImageError::InvalidReference);
            }
            UnwindTarget::Section { section, offset }
                if offset > sections[section as usize].byte_len() =>
            {
                return Err(SectionImageError::InvalidRange);
            }
            UnwindTarget::Symbol(_) | UnwindTarget::Section { .. } | UnwindTarget::Import(_) => {}
        }

        Ok(())
    }
}

impl ObjectUnwindBuilder {
    /// Create one object-local unwind table builder.
    pub fn new(format: UnwindFormat) -> Self {
        Self {
            format,
            sections: Vec::new(),
            relocations: Vec::new(),
        }
    }

    /// Set target unwind sections.
    pub fn sections(mut self, sections: impl IntoIterator<Item = UnwindSectionBuilder>) -> Self {
        self.sections = sections.into_iter().collect();

        self
    }

    /// Set object-local unwind relocations.
    pub fn relocations(mut self, relocations: impl IntoIterator<Item = UnwindRelocation>) -> Self {
        self.relocations = relocations.into_iter().collect();

        self
    }

    /// Build object-local target unwind tables.
    pub(super) fn build(self, sections: &mut SectionBuilder) -> ObjectUnwind {
        let alignment = self
            .sections
            .iter()
            .map(|section| section.alignment.bytes())
            .fold(1, u32::max) as usize;
        let mut bytes = Vec::new();
        let entries = self
            .sections
            .into_iter()
            .map(|section| section.build(&mut bytes))
            .collect::<Vec<_>>();

        ObjectUnwind {
            format: self.format,
            sections: sections.insert(entries),
            bytes: sections.insert_bytes(bytes, alignment),
            relocations: sections.insert(self.relocations),
        }
    }
}

impl UnwindBuilder {
    /// Create one fully linked native unwind table builder.
    pub fn new(format: UnwindFormat) -> Self {
        Self {
            format,
            sections: Vec::new(),
        }
    }

    /// Set target unwind sections.
    pub fn sections(mut self, sections: impl IntoIterator<Item = UnwindSectionBuilder>) -> Self {
        self.sections = sections.into_iter().collect();

        self
    }

    /// Append linked unwind sections to one native load image.
    pub(super) fn build(
        self,
        image: &mut Vec<u8>,
        sections: &mut SectionBuilder,
    ) -> (Unwind, Alignment) {
        let alignment = self
            .sections
            .iter()
            .map(|section| section.alignment)
            .fold(Alignment::ONE, Alignment::max);
        let entries = self
            .sections
            .into_iter()
            .map(|section| section.build(image))
            .collect::<Vec<_>>();

        (
            Unwind {
                format: self.format,
                sections: sections.insert(entries),
            },
            alignment,
        )
    }
}

impl UnwindSectionBuilder {
    /// Create one target unwind section.
    pub fn new(kind: UnwindSectionKind, bytes: impl Into<Vec<u8>>, alignment: Alignment) -> Self {
        Self {
            kind,
            bytes: bytes.into(),
            alignment,
        }
    }

    /// Pack this section into flattened unwind storage.
    fn build(self, bytes: &mut Vec<u8>) -> UnwindSection {
        let offset = bytes
            .len()
            .next_multiple_of(self.alignment.bytes() as usize);
        bytes.resize(offset, 0);
        let byte_len = self.bytes.len();
        bytes.extend_from_slice(&self.bytes);

        UnwindSection {
            kind: self.kind,
            bytes: EntryRange::new(offset as u32, byte_len as u32),
            alignment: self.alignment,
        }
    }
}
