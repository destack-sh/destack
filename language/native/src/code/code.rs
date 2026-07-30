use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use destack_core::{Optional, SectionBuilder, SectionEntry, SectionImage, SectionSlice, StringId};
use destack_mir::TargetLayout;
use destack_source::ContentId;

use crate::abi;

use super::{CodeMap, CodeMapBuilder, Entry, Image, ImportTable, ImportTableBuilder};

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
    entries: SectionSlice<Optional<Entry>>,
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
    entries: Vec<Option<Entry>>,
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
            entries: Vec::new(),
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
    pub fn entries(mut self, entries: impl IntoIterator<Item = Option<Entry>>) -> Self {
        self.entries = entries.into_iter().collect();

        self
    }

    /// Build this native code payload into program sections.
    pub fn build(self, sections: &mut SectionBuilder) -> Code {
        let imports = self.imports.build(sections);
        let map = self.map.build(sections);
        let entries = self
            .entries
            .into_iter()
            .map(Optional::from)
            .collect::<Vec<_>>();

        Code {
            abi_version: abi::VERSION,
            target: self.target,
            target_layout: self.target_layout,
            image: self.image,
            imports,
            map,
            entries: sections.insert(entries),
        }
    }
}

impl Code {
    /// Return one native function entry.
    pub fn entry(&self, sections: SectionImage<'_>, function: usize) -> Option<Entry> {
        sections
            .entries(self.entries)
            .get(function)
            .and_then(|entry| entry.get())
    }

    /// Return native function entries in dense Program function order.
    pub fn entries<'a>(&self, sections: SectionImage<'a>) -> &'a [Optional<Entry>] {
        sections.entries(self.entries)
    }

    /// Return all content ids referenced by this native code.
    pub fn content_ids(&self) -> Vec<ContentId> {
        self.image.content_ids()
    }
}
