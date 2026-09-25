use serde::{Deserialize, Serialize};
use tspp_core::{Optional, SectionBuilder, SectionEntry, SectionImage, SectionSlice, StringId};
use tspp_serde::Reflect;

use crate::abi;

use super::{
    Alignment, CodeMap, CodeMapBuilder, Entry, Function, ImportRelocation, Unwind, UnwindBuilder,
};

/// Immutable native code linked into one Program.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Code {
    /// TS++ native ABI version required by this code.
    pub abi_version: u32,
    /// Required executable image base alignment.
    pub alignment: Alignment,
    /// Exact target triple.
    pub target: StringId,
    /// Sorted target CPU features.
    features: SectionSlice<StringId>,
    /// Linked native code and immutable data.
    bytes: SectionSlice<u8>,
    /// Native functions keyed by Program function id.
    functions: SectionSlice<Optional<Function>>,
    /// Native resume entries keyed by Program frame state id.
    resumes: SectionSlice<Optional<Entry>>,
    /// Platform imports patched when the executable image is loaded.
    imports: SectionSlice<ImportRelocation>,
    /// Fully linked target-native unwind tables.
    unwind: Optional<Unwind>,
    /// Native frame maps for collection, inspection, and deoptimization.
    map: CodeMap,
}

/// Native code under construction.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeBuilder {
    /// Exact target triple.
    target: StringId,
    /// Required executable image base alignment.
    alignment: Alignment,
    /// Sorted target CPU features.
    features: Vec<StringId>,
    /// Linked native code and immutable data.
    bytes: Vec<u8>,
    /// Native functions keyed by Program function id.
    functions: Vec<Option<Function>>,
    /// Native resume entries keyed by Program frame state id.
    resumes: Vec<Option<Entry>>,
    /// Platform imports patched when the executable image is loaded.
    imports: Vec<ImportRelocation>,
    /// Fully linked target-native unwind tables.
    unwind: Option<UnwindBuilder>,
    /// Native code map.
    map: CodeMapBuilder,
}

impl Code {
    /// Return whether every relative range fits its sibling column.
    pub fn ranges_fit(&self, sections: SectionImage<'_>) -> bool {
        let bytes = self.bytes(sections);
        let functions = self.functions(sections);
        let resumes = self.resumes(sections);
        let imports = self.imports(sections);
        let alignment = self.alignment.bytes() as usize;

        // require the linked image to satisfy its executable base alignment
        if !self.alignment.is_valid() || !(bytes.as_ptr() as usize).is_multiple_of(alignment) {
            return false;
        }

        // check every physical function and coroutine entry
        let functions_fit = functions
            .iter()
            .filter_map(|function| function.get())
            .all(|function| function.ranges_fit(bytes.len()));
        let resumes_fit = resumes
            .iter()
            .filter_map(|resume| resume.get())
            .all(|resume| resume.ranges_fit(bytes.len()));
        let unwind_fits = self
            .unwind
            .get()
            .is_none_or(|unwind| unwind.ranges_fit(sections, bytes.len()));
        let imports_fit = imports.iter().all(|import| import.is_within(bytes.len()));
        if !functions_fit || !resumes_fit || !imports_fit || !unwind_fits {
            return false;
        }

        // check the sorted physical frame map
        let frames = self.map.frames(sections);
        let frames_fit = frames
            .iter()
            .all(|frame| (frame.return_offset as usize) <= bytes.len());
        let frames_sorted = frames
            .windows(2)
            .all(|frames| frames[0].return_offset < frames[1].return_offset);
        let traps = self.map.traps(sections);
        let traps_fit = traps.iter().all(|trap| trap.fits(bytes.len()));
        let traps_sorted = traps
            .windows(2)
            .all(|traps| traps[0].offset < traps[1].offset);

        frames_fit && frames_sorted && traps_fit && traps_sorted && self.map.ranges_fit(sections)
    }

    /// Return sorted target CPU features.
    pub fn features<'a>(&self, sections: SectionImage<'a>) -> &'a [StringId] {
        sections.entries(self.features)
    }

    /// Return linked native code and immutable data.
    pub fn bytes<'a>(&self, sections: SectionImage<'a>) -> &'a [u8] {
        sections.entries(self.bytes)
    }

    /// Return one native function.
    pub fn function(&self, sections: SectionImage<'_>, function: usize) -> Option<Function> {
        sections
            .entries(self.functions)
            .get(function)
            .and_then(|function| function.get())
    }

    /// Return native functions in dense Program function order.
    pub fn functions<'a>(&self, sections: SectionImage<'a>) -> &'a [Optional<Function>] {
        sections.entries(self.functions)
    }

    /// Return one native coroutine resume entry.
    pub fn resume(&self, sections: SectionImage<'_>, state: usize) -> Option<Entry> {
        sections
            .entries(self.resumes)
            .get(state)
            .and_then(|resume| resume.get())
    }

    /// Return native resume entries in dense Program frame state order.
    pub fn resumes<'a>(&self, sections: SectionImage<'a>) -> &'a [Optional<Entry>] {
        sections.entries(self.resumes)
    }

    /// Return platform import relocations.
    pub fn imports<'a>(&self, sections: SectionImage<'a>) -> &'a [ImportRelocation] {
        sections.entries(self.imports)
    }

    /// Return fully linked target-native unwind tables when present.
    pub fn unwind(&self) -> Option<Unwind> {
        self.unwind.get()
    }

    /// Return native frame maps for this linked code.
    pub const fn map(&self) -> CodeMap {
        self.map
    }
}

impl CodeBuilder {
    /// Create one native code builder.
    pub fn new(target: StringId) -> Self {
        Self {
            target,
            alignment: Alignment::ONE,
            features: Vec::new(),
            bytes: Vec::new(),
            functions: Vec::new(),
            resumes: Vec::new(),
            imports: Vec::new(),
            unwind: None,
            map: CodeMapBuilder::new(),
        }
    }

    /// Set target CPU features.
    pub fn features(mut self, features: impl IntoIterator<Item = StringId>) -> Self {
        self.features = features.into_iter().collect();
        self.features.sort_unstable();
        self.features.dedup();

        self
    }

    /// Set linked native code and immutable data.
    pub fn bytes(mut self, bytes: impl Into<Vec<u8>>, alignment: Alignment) -> Self {
        self.bytes = bytes.into();
        self.alignment = alignment;

        self
    }

    /// Set native functions in dense Program function order.
    pub fn functions(mut self, functions: impl IntoIterator<Item = Option<Function>>) -> Self {
        self.functions = functions.into_iter().collect();

        self
    }

    /// Set native resume entries in dense Program frame state order.
    pub fn resumes(mut self, resumes: impl IntoIterator<Item = Option<Entry>>) -> Self {
        self.resumes = resumes.into_iter().collect();

        self
    }

    /// Set platform import relocations.
    pub fn imports(mut self, imports: impl IntoIterator<Item = ImportRelocation>) -> Self {
        self.imports = imports.into_iter().collect();

        self
    }

    /// Set fully linked target-native unwind tables.
    pub fn unwind(mut self, unwind: UnwindBuilder) -> Self {
        self.unwind = Some(unwind);

        self
    }

    /// Set the native code map.
    pub fn map(mut self, map: CodeMapBuilder) -> Self {
        self.map = map;

        self
    }

    /// Build this native code into Program sections.
    pub fn build(self, sections: &mut SectionBuilder) -> Code {
        let functions = self
            .functions
            .into_iter()
            .map(Optional::from)
            .collect::<Vec<_>>();
        let resumes = self
            .resumes
            .into_iter()
            .map(Optional::from)
            .collect::<Vec<_>>();
        let mut bytes = self.bytes;
        let (unwind, alignment) = match self.unwind {
            Some(unwind) => {
                let (unwind, unwind_alignment) = unwind.build(&mut bytes, sections);

                (Some(unwind), self.alignment.max(unwind_alignment))
            }
            None => (None, self.alignment),
        };

        Code {
            abi_version: abi::VERSION,
            alignment,
            target: self.target,
            features: sections.insert(self.features),
            bytes: sections.insert_bytes(bytes, alignment.bytes() as usize),
            functions: sections.insert(functions),
            resumes: sections.insert(resumes),
            imports: sections.insert(self.imports),
            unwind: Optional::from(unwind),
            map: self.map.build(sections),
        }
    }
}
