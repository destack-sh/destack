use serde::{Deserialize, Serialize};
use tspp_core::{EntryRange, SectionBuilder, SectionEntry, SectionImage, SectionSlice};
use tspp_serde::Reflect;

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
    /// Return whether every object unwind range and relocation is valid.
    pub(super) fn ranges_fit(self, sections: SectionImage<'_>, symbols: usize) -> bool {
        let entries = self.sections(sections);
        let bytes = self.bytes(sections);
        if !sections_fit(entries, bytes.len()) {
            return false;
        }

        // require every relocation source and target to fit
        self.relocations(sections)
            .iter()
            .all(|relocation| relocation.fits(entries, symbols))
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
    /// Return whether every linked unwind range fits its load image.
    pub(super) fn ranges_fit(self, sections: SectionImage<'_>, image_len: usize) -> bool {
        let entries = self.sections(sections);

        sections_fit(entries, image_len)
    }

    /// Return target unwind sections.
    pub fn sections<'a>(self, sections: SectionImage<'a>) -> &'a [UnwindSection] {
        sections.entries(self.sections)
    }
}

impl UnwindSection {
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

    /// Return whether this relocation fits its source and target.
    fn fits(self, sections: &[UnwindSection], symbols: usize) -> bool {
        if !relocation_source_fits(self.section, self.offset, self.kind, sections) {
            return false;
        }

        match self.target {
            UnwindTarget::Symbol(symbol) => symbol.index() < symbols,
            UnwindTarget::Section { section, offset } => sections
                .get(section as usize)
                .is_some_and(|section| offset <= section.byte_len()),
            UnwindTarget::Import(_) => true,
        }
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
        let (entries, bytes, alignment) = build_sections(self.sections);

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

/// Return whether target unwind sections fit their owning native image.
fn sections_fit(sections: &[UnwindSection], bytes: usize) -> bool {
    sections
        .iter()
        .all(|section| section.alignment.is_valid() && section.bytes.fits(bytes))
}

/// Return whether one relocation source fits its unwind section.
fn relocation_source_fits(
    section: u32,
    offset: u32,
    kind: RelocationKind,
    sections: &[UnwindSection],
) -> bool {
    sections.get(section as usize).is_some_and(|section| {
        offset
            .checked_add(kind.byte_len())
            .is_some_and(|end| end <= section.byte_len())
    })
}

/// Flatten independently aligned target unwind sections.
fn build_sections(sections: Vec<UnwindSectionBuilder>) -> (Vec<UnwindSection>, Vec<u8>, usize) {
    let alignment = sections
        .iter()
        .map(|section| section.alignment.bytes())
        .fold(1, u32::max) as usize;
    let mut bytes = Vec::new();
    let sections = sections
        .into_iter()
        .map(|section| section.build(&mut bytes))
        .collect();

    (sections, bytes, alignment)
}
