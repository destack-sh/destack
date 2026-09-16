use std::collections::HashMap;

use destack_mir as mir;
use destack_native as native;
use destack_program::Object;
use destack_source::ModuleId;

use crate::LinkResult;

use super::{Image, NativeLinker};

impl<'a, 'b> NativeLinker<'a, 'b> {
    /// Apply every object-local native relocation.
    pub(super) fn patch_blocks(
        &self,
        image: &mut Image,
        functions: &[Option<native::Function>],
    ) -> LinkResult<()> {
        for (module, object) in self.program.objects() {
            let source = self.source(object)?;
            for (block_index, block) in source.blocks().iter().copied().enumerate() {
                let block_id = native::BlockId(block_index as u32);
                let linked = self.block(image, *module, block_id)?;
                for relocation in block.relocations(source.relocations()) {
                    let source_offset =
                        linked
                            .offset
                            .checked_add(relocation.offset)
                            .ok_or_else(|| {
                                self.program
                                    .layout_overflow("native relocation offset exceeds u32")
                            })?;
                    let target = self
                        .relocation_target(image, *module, object, source, relocation, functions)?;
                    self.patch(
                        &mut image.bytes,
                        source_offset,
                        source_offset,
                        target,
                        relocation.addend,
                        relocation.kind,
                    )?;
                }
            }
        }

        Ok(())
    }

    /// Resolve one relocation target as an image offset or Program identity.
    fn relocation_target(
        &self,
        image: &Image,
        module: ModuleId,
        object: &Object,
        source: &native::Object,
        relocation: &native::Relocation,
        functions: &[Option<native::Function>],
    ) -> LinkResult<u32> {
        if relocation.kind != native::RelocationKind::Index32 {
            return self.symbol(image, module, object, source, relocation.target, functions);
        }

        // resolve semantic identities directly into immutable descriptors
        let symbol = source
            .symbols()
            .get(relocation.target.index())
            .copied()
            .ok_or_else(|| self.program.invalid_input("native symbol is absent"))?;
        let native::Symbol::Index(index) = symbol else {
            return Err(self
                .program
                .invalid_input("native identity relocation targets a nonidentity symbol"));
        };
        let value = self.index(module, object, index, &image.frames)?;

        u32::try_from(value).map_err(|_| {
            self.program
                .layout_overflow("native Program identity exceeds u32")
        })
    }

    /// Resolve one object-local symbol into a linked image offset.
    pub(super) fn symbol(
        &self,
        image: &Image,
        module: ModuleId,
        object: &Object,
        source: &native::Object,
        symbol: native::SymbolId,
        functions: &[Option<native::Function>],
    ) -> LinkResult<u32> {
        let symbol_value = source
            .symbols()
            .get(symbol.index())
            .copied()
            .ok_or_else(|| self.program.invalid_input("native symbol is absent"))?;

        match symbol_value {
            native::Symbol::Block { block, offset } => self
                .block(image, module, block)?
                .offset
                .checked_add(offset)
                .ok_or_else(|| self.program.layout_overflow("native symbol exceeds u32")),
            native::Symbol::Function { function } => {
                let function = object.functions().get(function as usize).ok_or_else(|| {
                    self.program
                        .invalid_input("native function symbol is absent")
                })?;
                let function = self.program.function_id(module, function.id);
                functions
                    .get(function.index())
                    .and_then(Option::as_ref)
                    .map(|function| function.body.bytes.offset)
                    .ok_or_else(|| self.program.invalid_input("native function is undefined"))
            }
            native::Symbol::Import(import) => image
                .imports
                .get(&import)
                .copied()
                .ok_or_else(|| self.program.invalid_input("native import is absent")),
            native::Symbol::Index(_) => image
                .indices
                .get(&(module, symbol))
                .copied()
                .ok_or_else(|| self.program.invalid_input("native index cell is absent")),
        }
    }

    /// Resolve one object-local identity into its final Program index.
    pub(super) fn index(
        &self,
        module: ModuleId,
        object: &Object,
        index: native::Index,
        frames: &HashMap<(ModuleId, u32), u32>,
    ) -> LinkResult<u64> {
        match index {
            native::Index::Type { ty } => {
                let ty = mir::TypeId(ty);
                object
                    .ty(ty)
                    .ok_or_else(|| self.program.invalid_input("native type is absent"))?;

                Ok(u64::from(self.program.type_id(module, ty).0))
            }
            native::Index::Layout { ty } => {
                let ty = mir::TypeId(ty);
                object
                    .layouts()
                    .type_layout(ty)
                    .ok_or_else(|| self.program.invalid_input("native layout is absent"))?;

                Ok(u64::from(self.program.layout_id(module, ty).raw()))
            }
            native::Index::Function { function } => {
                let function = mir::FunctionId::new(function);
                object
                    .function(function)
                    .ok_or_else(|| self.program.invalid_input("native function is absent"))?;

                Ok(u64::from(self.program.function_id(module, function).0))
            }
            native::Index::Global { global } => {
                let global = mir::GlobalId::new(global);
                object
                    .global(global)
                    .ok_or_else(|| self.program.invalid_input("native global is absent"))?;

                let global = self.program.global_id(module, global);
                let global = self
                    .statics
                    .global(global)
                    .ok_or_else(|| self.program.invalid_input("linked global is absent"))?;

                Ok(global.offset)
            }
            native::Index::Dynamic { table } => {
                let table = object
                    .dispatch()
                    .iter_dynamic_tables()
                    .nth(table as usize)
                    .ok_or_else(|| self.program.invalid_input("native dynamic table is absent"))?;
                let table = self
                    .program
                    .dynamic_table_id(module, table.concrete, table.constraint)
                    .ok_or_else(|| {
                        self.program
                            .invalid_input("native dynamic table was not linked")
                    })?;

                Ok(u64::from(table.0))
            }
            native::Index::Allocation { site } => {
                object.allocations().get(site as usize).ok_or_else(|| {
                    self.program
                        .invalid_input("native allocation site is absent")
                })?;

                Ok(u64::from(self.program.allocation_id(module, site).0))
            }
            native::Index::Frame { frame } => frames
                .get(&(module, frame))
                .copied()
                .map(u64::from)
                .ok_or_else(|| self.program.invalid_input("native frame map is absent")),
            native::Index::Counter { function, counter } => {
                let function = mir::FunctionId::new(function);

                Ok(u64::from(
                    self.program
                        .counter_id(module, function, mir::CounterId(counter))
                        .0,
                ))
            }
            native::Index::Sampler { function, sampler } => {
                let function = mir::FunctionId::new(function);

                Ok(u64::from(
                    self.program
                        .sampler_id(module, function, mir::SamplerId(sampler))
                        .0,
                ))
            }
        }
    }

    /// Apply one target relocation to a mutable byte image.
    pub(super) fn patch(
        &self,
        bytes: &mut [u8],
        offset: u32,
        source: u32,
        target: u32,
        addend: i64,
        kind: native::RelocationKind,
    ) -> LinkResult<()> {
        let start = offset as usize;
        let end = start + kind.byte_len() as usize;
        let destination = bytes.get_mut(start..end).ok_or_else(|| {
            self.program
                .invalid_input("native relocation is out of range")
        })?;
        let delta = i64::from(target) + addend - i64::from(source);

        match kind {
            native::RelocationKind::Index32 => {
                let value = i64::from(target) + addend;
                let value = u32::try_from(value).map_err(|_| {
                    self.program
                        .layout_overflow("native Program identity exceeds u32")
                })?;
                destination.copy_from_slice(&value.to_le_bytes());
            }
            native::RelocationKind::Absolute64 => {
                return Err(self
                    .program
                    .invalid_input("process-local relocation reached Program linking"));
            }
            native::RelocationKind::Relative32 => {
                let value = i32::try_from(delta).map_err(|_| {
                    self.program
                        .layout_overflow("native relative relocation exceeds i32")
                })?;
                self.write_u32(destination, value as u32);
            }
            native::RelocationKind::Aarch64Call26 => {
                if delta & 0b11 != 0 || !(-(1 << 27)..(1 << 27)).contains(&delta) {
                    return Err(self.program.layout_overflow("AArch64 call exceeds imm26"));
                }
                let mut instruction = self.read_u32(destination)?;
                instruction = (instruction & !0x03ff_ffff) | ((delta >> 2) as u32 & 0x03ff_ffff);
                self.write_u32(destination, instruction);
            }
            native::RelocationKind::Aarch64Page21 => {
                let target = i64::from(target) + addend;
                let pages = (target >> 12) - (i64::from(source) >> 12);
                if !(-(1 << 20)..(1 << 20)).contains(&pages) {
                    return Err(self
                        .program
                        .layout_overflow("AArch64 page offset exceeds imm21"));
                }
                let immediate = pages as u32 & 0x001f_ffff;
                let mut instruction = self.read_u32(destination)?;
                instruction &= !((0x3 << 29) | (0x7ffff << 5));
                instruction |= (immediate & 0x3) << 29;
                instruction |= ((immediate >> 2) & 0x7ffff) << 5;
                self.write_u32(destination, instruction);
            }
            native::RelocationKind::Aarch64Low12 => {
                let immediate = (i64::from(target) + addend) as u32 & 0xfff;
                let mut instruction = self.read_u32(destination)?;
                instruction = (instruction & !(0xfff << 10)) | (immediate << 10);
                self.write_u32(destination, instruction);
            }
            native::RelocationKind::RiscvCall => {
                if !(-(1 << 31)..(1 << 31)).contains(&delta) {
                    return Err(self.program.layout_overflow("RISC-V call exceeds 32 bits"));
                }
                let upper = (delta + 0x800) >> 12;
                let lower = (delta - (upper << 12)) as u32 & 0xfff;
                let upper = upper as u32 & 0x000f_ffff;
                let mut first = self.read_u32(&destination[..4])?;
                let mut second = self.read_u32(&destination[4..])?;
                first = (first & 0xfff) | (upper << 12);
                second = (second & 0x000f_ffff) | (lower << 20);
                self.write_u32(&mut destination[..4], first);
                self.write_u32(&mut destination[4..], second);
            }
        }

        Ok(())
    }

    /// Read one target-endian instruction word.
    fn read_u32(&self, bytes: &[u8]) -> LinkResult<u32> {
        let bytes: [u8; 4] = bytes.try_into().map_err(|_| {
            self.program
                .invalid_input("native relocation has invalid width")
        })?;

        Ok(if self.program.target_layout().endian.is_little() {
            u32::from_le_bytes(bytes)
        } else {
            u32::from_be_bytes(bytes)
        })
    }

    /// Write one target-endian instruction word.
    fn write_u32(&self, destination: &mut [u8], value: u32) {
        let bytes = if self.program.target_layout().endian.is_little() {
            value.to_le_bytes()
        } else {
            value.to_be_bytes()
        };
        destination.copy_from_slice(&bytes);
    }
}
