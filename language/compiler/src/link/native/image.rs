use std::collections::{BTreeSet, HashMap};

use target_lexicon::{Architecture, Triple};
use tspp_native as native;
use tspp_source::ModuleId;

use crate::LinkResult;

use super::NativeLinker;

/// One native code image under construction.
#[derive(Debug, Default)]
pub(super) struct Image {
    /// Linked native bytes.
    pub(super) bytes: Vec<u8>,
    /// Required executable image base alignment.
    pub(super) alignment: native::Alignment,
    /// Linked ranges keyed by module and object-local block.
    pub(super) blocks: HashMap<(ModuleId, native::BlockId), native::CodeRange>,
    /// Linked index cells keyed by module and object-local symbol.
    pub(super) indices: HashMap<(ModuleId, native::SymbolId), u32>,
    /// Linked frame-map ids keyed by module and object-local frame map.
    pub(super) frames: HashMap<(ModuleId, u32), u32>,
    /// Image-local trampolines keyed by platform import.
    pub(super) imports: HashMap<native::Import, u32>,
    /// Process-local pointers patched when the image is loaded.
    pub(super) import_relocations: Vec<native::ImportRelocation>,
}

impl<'a, 'b> NativeLinker<'a, 'b> {
    /// Place every object-local code block into the linked image.
    pub(super) fn place_blocks(&self, image: &mut Image) -> LinkResult<()> {
        for (module, object) in self.program.objects() {
            let source = self.source(object)?;
            for (index, block) in source.blocks().iter().copied().enumerate() {
                let offset = self.align(image, block.alignment)?;
                let bytes = block.bytes(source.code());
                image.bytes.extend_from_slice(bytes);
                let block = native::BlockId(index as u32);
                let range = native::CodeRange::new(offset, bytes.len() as u32);
                image.blocks.insert((*module, block), range);
            }
        }

        Ok(())
    }

    /// Reserve and initialize every addressable Program index cell.
    pub(super) fn place_indices(&self, image: &mut Image) -> LinkResult<()> {
        for (module, object) in self.program.objects() {
            let source = self.source(object)?;
            for (index, symbol) in source.symbols().iter().copied().enumerate() {
                let native::Symbol::Index(index_value) = symbol else {
                    continue;
                };
                let byte_len = if matches!(index_value, native::Index::Global { .. }) {
                    self.program.target_layout().pointer_bytes() as usize
                } else {
                    size_of::<u32>()
                };
                let alignment = native::Alignment::new(byte_len as u32).ok_or_else(|| {
                    self.program
                        .invalid_input("native index alignment is invalid")
                })?;
                let offset = self.align(image, alignment)?;
                let value = self.index(*module, object, index_value, &image.frames)?;
                let value = if byte_len == size_of::<u32>() {
                    let value = u32::try_from(value).map_err(|_| {
                        self.program
                            .layout_overflow("native Program index exceeds u32")
                    })?;
                    if self.program.target_layout().endian.is_little() {
                        value.to_le_bytes().to_vec()
                    } else {
                        value.to_be_bytes().to_vec()
                    }
                } else if self.program.target_layout().endian.is_little() {
                    value.to_le_bytes()[..byte_len].to_vec()
                } else {
                    value.to_be_bytes()[size_of::<u64>() - byte_len..].to_vec()
                };
                image.bytes.extend_from_slice(&value);
                image
                    .indices
                    .insert((*module, native::SymbolId(index as u32)), offset);
            }
        }

        Ok(())
    }

    /// Place one image-local trampoline for every platform import.
    pub(super) fn place_imports(&self, image: &mut Image, target: &str) -> LinkResult<()> {
        let target = target.parse::<Triple>().map_err(|error| {
            self.program
                .invalid_input(format!("native target triple is invalid: {error}"))
        })?;
        let mut imports = BTreeSet::new();
        for (_, object) in self.program.objects() {
            let source = self.source(object)?;
            imports.extend(source.symbols().iter().filter_map(|symbol| match symbol {
                native::Symbol::Import(import) => Some(*import),
                _ => None,
            }));
        }

        // append one fixed target trampoline and retain its pointer cell
        for import in imports {
            let alignment = native::Alignment::new(8).ok_or_else(|| {
                self.program
                    .invalid_input("native import alignment is invalid")
            })?;
            let offset = self.align(image, alignment)?;
            let pointer_offset = self.append_trampoline(image, target.architecture)?;
            let pointer_offset = offset.checked_add(pointer_offset).ok_or_else(|| {
                self.program
                    .layout_overflow("native import pointer exceeds u32")
            })?;
            image.imports.insert(import, offset);
            image
                .import_relocations
                .push(native::ImportRelocation::new(pointer_offset, import));
        }

        Ok(())
    }

    /// Append one target jump through an adjacent absolute pointer.
    fn append_trampoline(&self, image: &mut Image, architecture: Architecture) -> LinkResult<u32> {
        let pointer_offset = match architecture {
            Architecture::X86_64 | Architecture::X86_64h => {
                image
                    .bytes
                    .extend_from_slice(&[0xff, 0x25, 0x00, 0x00, 0x00, 0x00]);
                image.bytes.extend_from_slice(&[0; 8]);

                6
            }
            Architecture::Aarch64(_) => {
                image
                    .bytes
                    .extend_from_slice(&0x5800_0050_u32.to_le_bytes());
                image
                    .bytes
                    .extend_from_slice(&0xd61f_0200_u32.to_le_bytes());
                image.bytes.extend_from_slice(&[0; 8]);

                8
            }
            Architecture::Riscv64(_) => {
                image
                    .bytes
                    .extend_from_slice(&0x0000_0297_u32.to_le_bytes());
                image
                    .bytes
                    .extend_from_slice(&0x0102_b283_u32.to_le_bytes());
                image
                    .bytes
                    .extend_from_slice(&0x0002_8067_u32.to_le_bytes());
                image
                    .bytes
                    .extend_from_slice(&0x0000_0013_u32.to_le_bytes());
                image.bytes.extend_from_slice(&[0; 8]);

                16
            }
            _ => {
                return Err(self
                    .program
                    .invalid_input(format!("native imports do not support {architecture}")));
            }
        };

        Ok(pointer_offset)
    }

    /// Align one image and return the new byte offset.
    pub(super) fn align(&self, image: &mut Image, alignment: native::Alignment) -> LinkResult<u32> {
        image.alignment = image.alignment.max(alignment);
        let offset = image
            .bytes
            .len()
            .next_multiple_of(alignment.bytes() as usize);
        image.bytes.resize(offset, 0);

        u32::try_from(offset).map_err(|_| {
            self.program
                .layout_overflow("native code image exceeds u32")
        })
    }

    /// Return one linked block range.
    pub(super) fn block(
        &self,
        image: &Image,
        module: ModuleId,
        block: native::BlockId,
    ) -> LinkResult<native::CodeRange> {
        image
            .blocks
            .get(&(module, block))
            .copied()
            .ok_or_else(|| self.program.invalid_input("native code block is absent"))
    }
}
