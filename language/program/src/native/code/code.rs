use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use destack_core::{SectionBuilder, SectionEntry, StringId};
use destack_mir::TargetLayout;
use destack_source::ContentId;

use crate::native::NATIVE_ABI_VERSION;

use super::{
    CodeMap, CodeMapBuilder, EntryTable, EntryTableBuilder, Image, ImportTable, ImportTableBuilder,
};

/// Durable native code produced for one program.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Code {
    /// Destack native ABI version required by this code.
    pub abi_version: u32,
    /// Target triple or equivalent target identity.
    pub target: StringId,
    /// Target ABI layout expected by this code.
    pub target_layout: TargetLayout,
    /// The native image.
    pub image: Image,
    /// Native imports required by this code.
    pub imports: ImportTable,
    /// Native code map for safepoints and deoptimization.
    pub map: CodeMap,
    /// Native entries keyed by program ids.
    pub entries: EntryTable,
}

/// Build-time native code payload.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeBuilder {
    /// Target triple or equivalent target identity.
    target: StringId,
    /// Target ABI layout expected by this code.
    target_layout: TargetLayout,
    /// Native image payload.
    image: Image,
    /// Required native imports.
    imports: ImportTableBuilder,
    /// Native code map.
    map: CodeMapBuilder,
    /// Native entry table.
    entries: EntryTableBuilder,
}

impl CodeBuilder {
    /// Create one native code builder.
    pub fn new(target: StringId, target_layout: TargetLayout, image: Image) -> Self {
        Self {
            target,
            target_layout,
            image,
            imports: ImportTableBuilder::default(),
            map: CodeMapBuilder::default(),
            entries: EntryTableBuilder::default(),
        }
    }

    /// Set required native imports.
    pub fn imports(mut self, imports: ImportTableBuilder) -> Self {
        self.imports = imports;

        self
    }

    /// Set the native code map.
    pub fn map(mut self, map: CodeMapBuilder) -> Self {
        self.map = map;

        self
    }

    /// Set the native entry table.
    pub fn entries(mut self, entries: EntryTableBuilder) -> Self {
        self.entries = entries;

        self
    }

    /// Build this native code payload into program sections.
    pub(crate) fn build(self, sections: &mut SectionBuilder) -> Code {
        let imports = self.imports.build(sections);
        let map = self.map.build(sections);
        let entries = self.entries.build(sections);

        Code {
            abi_version: NATIVE_ABI_VERSION,
            target: self.target,
            target_layout: self.target_layout,
            image: self.image,
            imports,
            map,
            entries,
        }
    }
}

impl Code {
    /// Return all content ids referenced by this native code.
    pub fn content_ids(&self) -> Vec<ContentId> {
        self.image.content_ids()
    }
}
