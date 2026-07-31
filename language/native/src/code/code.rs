use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use destack_core::{
    EntryStore, Optional, SectionBuilder, SectionEntry, SectionImage, SectionSlice, StringId,
};
use destack_mir::TargetLayout;

use crate::abi;

use super::{
    CodeMap, CodeMapBuilder, Definition, Image, ImageBuilder, ImportTable, ImportTableBuilder,
    Module, ModuleBuilder,
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
    /// Native frame maps for collection, inspection, and deoptimization.
    pub map: CodeMap,
    /// Program identity mappings in archive object order.
    modules: SectionSlice<Module>,
    /// Program function identities referenced by native modules.
    functions: SectionSlice<abi::Function>,
    /// Optional Program type ids referenced by linked modules.
    types: SectionSlice<Optional<u32>>,
    /// Optional Program layout ids referenced by linked modules.
    layouts: SectionSlice<Optional<u32>>,
    /// Program global ids referenced by linked modules.
    globals: SectionSlice<u32>,
    /// Program dynamic-table ids referenced by linked modules.
    dynamics: SectionSlice<u32>,
    /// Native definitions keyed by Program function id.
    definitions: SectionSlice<Optional<Definition>>,
}

/// Build-time native code payload.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeBuilder {
    /// Target triple or equivalent target identity.
    target: StringId,
    /// Target ABI layout expected by this code.
    target_layout: TargetLayout,
    /// Native image payload.
    image: ImageBuilder,
    /// Required native imports.
    imports: ImportTableBuilder,
    /// Native code map.
    map: CodeMapBuilder,
    /// Program identity mappings in archive object order.
    modules: Vec<ModuleBuilder>,
    /// Native definitions keyed by Program function id.
    definitions: Vec<Option<Definition>>,
}

impl CodeBuilder {
    /// Create one native code builder.
    pub fn new(target: StringId, target_layout: TargetLayout, image: ImageBuilder) -> Self {
        Self {
            target,
            target_layout,
            image,
            imports: ImportTableBuilder::default(),
            map: CodeMapBuilder::default(),
            modules: Vec::new(),
            definitions: Vec::new(),
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

    /// Set Program identity mappings in archive object order.
    pub fn modules(mut self, modules: impl IntoIterator<Item = ModuleBuilder>) -> Self {
        self.modules = modules.into_iter().collect();

        self
    }

    /// Set native definitions in dense Program function order.
    pub fn definitions(
        mut self,
        definitions: impl IntoIterator<Item = Option<Definition>>,
    ) -> Self {
        self.definitions = definitions.into_iter().collect();

        self
    }

    /// Build this native code payload into program sections.
    pub fn build(self, sections: &mut SectionBuilder) -> Code {
        let image = self.image.build(sections);
        let imports = self.imports.build(sections);
        let map = self.map.build(sections);
        let mut functions = EntryStore::new();
        let mut types = EntryStore::new();
        let mut layouts = EntryStore::new();
        let mut globals = EntryStore::new();
        let mut dynamics = EntryStore::new();
        let modules = self
            .modules
            .into_iter()
            .map(|module| {
                module.build(
                    &mut functions,
                    &mut types,
                    &mut layouts,
                    &mut globals,
                    &mut dynamics,
                )
            })
            .collect::<Vec<_>>();
        let definitions = self
            .definitions
            .into_iter()
            .map(Optional::from)
            .collect::<Vec<_>>();

        Code {
            abi_version: abi::VERSION,
            target: self.target,
            target_layout: self.target_layout,
            image,
            imports,
            map,
            modules: sections.insert(modules),
            functions: sections.insert(functions.into_entries()),
            types: sections.insert(types.into_entries()),
            layouts: sections.insert(layouts.into_entries()),
            globals: sections.insert(globals.into_entries()),
            dynamics: sections.insert(dynamics.into_entries()),
            definitions: sections.insert(definitions),
        }
    }
}

impl Code {
    /// Return whether every relative range fits its sibling column.
    pub fn ranges_fit(&self, sections: SectionImage<'_>) -> bool {
        let functions = self.functions(sections);
        let types = self.types(sections);
        let layouts = self.layouts(sections);
        let globals = self.globals(sections);
        let dynamics = self.dynamics(sections);

        // check each module's linked identity ranges
        let modules_fit = self.modules(sections).iter().all(|module| {
            module.ranges_fit(
                functions.len(),
                types.len(),
                layouts.len(),
                globals.len(),
                dynamics.len(),
            )
        });
        if !modules_fit {
            return false;
        }

        // check canonical native frame maps
        if !self.map.ranges_fit(sections) {
            return false;
        }

        // check archived object byte ranges when present
        let Image::Archive(archive) = self.image else {
            return true;
        };

        archive.ranges_fit(sections)
    }

    /// Return Program identity mappings in archive object order.
    pub fn modules<'a>(&self, sections: SectionImage<'a>) -> &'a [Module] {
        sections.entries(self.modules)
    }

    /// Return one Program identity mapping by archive object index.
    pub fn module(&self, sections: SectionImage<'_>, index: u32) -> Option<Module> {
        sections.entries(self.modules).get(index as usize).copied()
    }

    /// Return Program function identities referenced by native modules.
    pub fn functions<'a>(&self, sections: SectionImage<'a>) -> &'a [abi::Function] {
        sections.entries(self.functions)
    }

    /// Return optional Program type ids referenced by linked modules.
    pub fn types<'a>(&self, sections: SectionImage<'a>) -> &'a [Optional<u32>] {
        sections.entries(self.types)
    }

    /// Return optional Program layout ids referenced by linked modules.
    pub fn layouts<'a>(&self, sections: SectionImage<'a>) -> &'a [Optional<u32>] {
        sections.entries(self.layouts)
    }

    /// Return Program global ids referenced by linked modules.
    pub fn globals<'a>(&self, sections: SectionImage<'a>) -> &'a [u32] {
        sections.entries(self.globals)
    }

    /// Return Program dynamic-table ids referenced by linked modules.
    pub fn dynamics<'a>(&self, sections: SectionImage<'a>) -> &'a [u32] {
        sections.entries(self.dynamics)
    }

    /// Return one native function definition.
    pub fn definition(&self, sections: SectionImage<'_>, function: usize) -> Option<Definition> {
        sections
            .entries(self.definitions)
            .get(function)
            .and_then(|definition| definition.get())
    }

    /// Return native definitions in dense Program function order.
    pub fn definitions<'a>(&self, sections: SectionImage<'a>) -> &'a [Optional<Definition>] {
        sections.entries(self.definitions)
    }
}
