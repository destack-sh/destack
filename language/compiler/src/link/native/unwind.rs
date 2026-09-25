use std::collections::HashMap;

use tspp_native as native;
use tspp_source::ModuleId;

use crate::LinkResult;

use super::{Image, NativeLinker};

/// One platform unwind section before native code packing.
#[derive(Debug)]
pub(super) struct UnwindSection {
    /// Object containing this section.
    pub(super) module: ModuleId,
    /// Object-local section identity.
    pub(super) source: u32,
    /// Target section role.
    pub(super) kind: native::UnwindSectionKind,
    /// Required section alignment.
    pub(super) alignment: native::Alignment,
    /// Final byte offset inside the native load image.
    pub(super) offset: u32,
    /// Mutable target-native section bytes.
    pub(super) bytes: Vec<u8>,
}

impl<'a, 'b> NativeLinker<'a, 'b> {
    /// Link target unwind sections into the native load image.
    pub(super) fn link_unwind(
        &self,
        image: &Image,
        functions: &[Option<native::Function>],
    ) -> LinkResult<(Option<native::UnwindBuilder>, Vec<native::ImportRelocation>)> {
        let mut format = None;
        let mut sections = Vec::new();
        let mut offsets = HashMap::new();
        let mut next_offset = image.bytes.len();

        // assign final image positions to every unwind section
        for (module, object) in self.program.objects() {
            let source = self.source(object)?;
            let Some(unwind) = source.unwind() else {
                continue;
            };
            if format
                .replace(unwind.format)
                .is_some_and(|value| value != unwind.format)
            {
                return Err(self.program.invalid_input("native unwind formats differ"));
            }
            let source_sections = source.sections();
            let source_bytes = unwind.bytes(source_sections);
            for (index, section) in unwind.sections(source_sections).iter().copied().enumerate() {
                next_offset = next_offset.next_multiple_of(section.alignment.bytes() as usize);
                let offset = u32::try_from(next_offset).map_err(|_| {
                    self.program
                        .layout_overflow("native unwind offset exceeds u32")
                })?;
                let bytes = section.bytes(source_bytes).to_vec();
                next_offset += bytes.len();
                offsets.insert((*module, index as u32), offset);
                sections.push(UnwindSection {
                    module: *module,
                    source: index as u32,
                    kind: section.kind,
                    alignment: section.alignment,
                    offset,
                    bytes,
                });
            }
        }

        let Some(format) = format else {
            return Ok((None, Vec::new()));
        };
        let mut imports = Vec::new();

        // resolve code and section references while image offsets are known
        for section in &mut sections {
            let object = self.program.object(section.module);
            let source = self.source(object)?;
            let unwind = source
                .unwind()
                .ok_or_else(|| self.program.invalid_input("native unwind table is absent"))?;
            for relocation in unwind
                .relocations(source.sections())
                .iter()
                .filter(|relocation| relocation.section == section.source)
            {
                let target = match relocation.target {
                    native::UnwindTarget::Symbol(symbol) => {
                        self.symbol(image, section.module, object, source, symbol, functions)?
                    }
                    native::UnwindTarget::Section {
                        section: target_section,
                        offset,
                    } => offsets
                        .get(&(section.module, target_section))
                        .and_then(|base| base.checked_add(offset))
                        .ok_or_else(|| {
                            self.program.invalid_input("native unwind target is absent")
                        })?,
                    native::UnwindTarget::Import(import) => {
                        let offset =
                            section
                                .offset
                                .checked_add(relocation.offset)
                                .ok_or_else(|| {
                                    self.program
                                        .layout_overflow("native import offset exceeds u32")
                                })?;
                        if relocation.kind != native::RelocationKind::Absolute64
                            || relocation.addend != 0
                        {
                            return Err(self
                                .program
                                .invalid_input("native import is not one absolute pointer"));
                        }
                        imports.push(native::ImportRelocation::new(offset, import));

                        continue;
                    }
                };
                let source_offset =
                    section
                        .offset
                        .checked_add(relocation.offset)
                        .ok_or_else(|| {
                            self.program
                                .layout_overflow("native unwind source exceeds u32")
                        })?;
                self.patch(
                    &mut section.bytes,
                    relocation.offset,
                    source_offset,
                    target,
                    relocation.addend,
                    relocation.kind,
                )?;
            }
        }

        let sections = sections.into_iter().map(|section| {
            native::UnwindSectionBuilder::new(section.kind, section.bytes, section.alignment)
        });

        let unwind = native::UnwindBuilder::new(format).sections(sections);

        Ok((Some(unwind), imports))
    }
}
