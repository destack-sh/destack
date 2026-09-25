use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::isa::unwind::UnwindInfo;
use cranelift_codegen::{CompiledCode, FinalizedMachExceptionHandler};
use destack_native as native;
use destack_source::ModuleId;
use gimli::constants::{DW_EH_PE_absptr, DW_EH_PE_pcrel, DW_EH_PE_sdata4, DW_EH_PE_udata4};
use gimli::write::{
    Address, EhFrame, EndianVec, FrameTable, RelocateWriter, Relocation, RelocationTarget,
};
use gimli::{RunTimeEndian, constants};

use crate::EmitError;

const FRAME_SECTION: u32 = 0;
const EXCEPTION_SECTION: u32 = 1;

/// Target unwind tables emitted for one native object.
pub(in crate::emit::native) struct UnwindEmitter {
    /// Module receiving diagnostics.
    module: ModuleId,
    /// Target byte order.
    endian: RunTimeEndian,
    /// Shared call-frame records.
    frames: FrameTable,
    /// Common frame record used by every function.
    cie: gimli::write::CieId,
    /// Relocation targets indexed by Gimli symbols.
    targets: Vec<native::UnwindTarget>,
    /// Language-specific call-site tables.
    exceptions: Vec<u8>,
    /// Target pointer alignment.
    pointer_alignment: native::Alignment,
}

/// Relocatable `.eh_frame` bytes under construction.
struct FrameWriter {
    /// Target-endian bytes.
    bytes: EndianVec<RunTimeEndian>,
    /// Relocations requested while serializing frame records.
    relocations: Vec<Relocation>,
}

impl UnwindEmitter {
    /// Create one System V unwind emitter.
    pub(in crate::emit::native) fn new(
        module: ModuleId,
        isa: &dyn TargetIsa,
    ) -> Result<Self, EmitError> {
        let endian = match isa.triple().endianness() {
            Ok(target_lexicon::Endianness::Little) => RunTimeEndian::Little,
            Ok(target_lexicon::Endianness::Big) => RunTimeEndian::Big,
            Err(_) => return Err(Self::invalid(module, "native target byte order is unknown")),
        };
        let pointer_alignment = native::Alignment::new(isa.pointer_type().bytes())
            .ok_or_else(|| Self::invalid(module, "native pointer alignment is invalid"))?;
        let targets = vec![native::UnwindTarget::Import(
            native::Import::UnwindPersonality,
        )];
        let mut cie = isa
            .create_systemv_cie()
            .ok_or_else(|| Self::invalid(module, "native target has no System V unwind format"))?;

        // use the Rust personality while generated frames cross Rust runtime operations
        cie.personality = Some((
            constants::DwEhPe(DW_EH_PE_absptr.0),
            Address::Symbol {
                symbol: 0,
                addend: 0,
            },
        ));
        cie.lsda_encoding = Some(constants::DwEhPe(DW_EH_PE_pcrel.0 | DW_EH_PE_sdata4.0));
        cie.fde_address_encoding = constants::DwEhPe(DW_EH_PE_pcrel.0 | DW_EH_PE_sdata4.0);
        let mut frames = FrameTable::default();
        let cie = frames.add_cie(cie);

        Ok(Self {
            module,
            endian,
            frames,
            cie,
            targets,
            exceptions: Vec::new(),
            pointer_alignment,
        })
    }

    /// Add unwind records for one compiled native block.
    pub(in crate::emit::native) fn add(
        &mut self,
        symbol: native::SymbolId,
        compiled: &CompiledCode,
        isa: &dyn TargetIsa,
    ) -> Result<(), EmitError> {
        let unwind = compiled
            .create_unwind_info(isa)
            .map_err(|error| Self::invalid(self.module, error.to_string()))?
            .ok_or_else(|| {
                Self::invalid(self.module, "compiled native frame has no unwind data")
            })?;
        let UnwindInfo::SystemV(unwind) = unwind else {
            return Err(Self::invalid(
                self.module,
                "compiled native frame does not use System V unwind data",
            ));
        };
        // encode one cleanup-only call-site table for this function
        let exception_offset = u32::try_from(self.exceptions.len())
            .map_err(|_| Self::invalid(self.module, "native exception table exceeds u32"))?;
        let exception = Self::exception_table(self.module, compiled, self.endian)?;
        self.exceptions.extend_from_slice(&exception);

        // point one FDE at its code block and call-site table
        let function = self.target(native::UnwindTarget::Symbol(symbol));
        let exception = self.target(native::UnwindTarget::Section {
            section: EXCEPTION_SECTION,
            offset: exception_offset,
        });
        let mut frame = unwind.to_fde(Address::Symbol {
            symbol: function,
            addend: 0,
        });
        frame.lsda = Some(Address::Symbol {
            symbol: exception,
            addend: 0,
        });
        self.frames.add_fde(self.cie, frame);

        Ok(())
    }

    /// Build one relocatable native unwind table.
    pub(in crate::emit::native) fn build(self) -> Result<native::ObjectUnwindBuilder, EmitError> {
        let mut frame = EhFrame(FrameWriter {
            bytes: EndianVec::new(self.endian),
            relocations: Vec::new(),
        });
        self.frames
            .write_eh_frame(&mut frame)
            .map_err(|error| Self::invalid(self.module, error.to_string()))?;
        let EhFrame(FrameWriter { bytes, relocations }) = frame;
        let mut bytes = bytes.into_vec();
        Self::write_u32(0, self.endian, &mut bytes);
        let relocations = relocations
            .into_iter()
            .map(|relocation| self.relocation(relocation))
            .collect::<Result<Vec<_>, _>>()?;
        let sections = [
            native::UnwindSectionBuilder::new(
                native::UnwindSectionKind::DwarfFrame,
                bytes,
                self.pointer_alignment,
            ),
            native::UnwindSectionBuilder::new(
                native::UnwindSectionKind::DwarfException,
                self.exceptions,
                self.pointer_alignment,
            ),
        ];

        Ok(
            native::ObjectUnwindBuilder::new(native::UnwindFormat::Dwarf)
                .sections(sections)
                .relocations(relocations),
        )
    }

    /// Encode one cleanup-only Itanium call-site table.
    fn exception_table(
        module: ModuleId,
        compiled: &CompiledCode,
        endian: RunTimeEndian,
    ) -> Result<Vec<u8>, EmitError> {
        let code_byte_len = u32::try_from(compiled.buffer.data().len())
            .map_err(|_| Self::invalid(module, "native code exceeds the call-site table range"))?;
        let mut entries = Vec::new();
        let mut start = 0;

        // cover each landing call by its last byte and continue unwinding everywhere else
        for call_site in compiled.buffer.call_sites() {
            let landing = match call_site.exception_handlers {
                [] => continue,
                [FinalizedMachExceptionHandler::Default(offset)] => offset.to_owned(),
                _ => {
                    return Err(Self::invalid(
                        module,
                        "native call site uses unsupported exception handlers",
                    ));
                }
            };
            let call_end = call_site
                .ret_addr
                .checked_sub(1)
                .filter(|call_end| *call_end >= start)
                .ok_or_else(|| Self::invalid(module, "native call sites are not ordered"))?;
            if call_end > start {
                entries.push((start, call_end - start, 0));
            }
            entries.push((call_end, 1, landing));
            start = call_site.ret_addr;
        }
        if code_byte_len > start {
            entries.push((start, code_byte_len - start, 0));
        }

        // write the call-site header and one fixed width row per entry
        let byte_len = entries
            .len()
            .checked_mul(13)
            .ok_or_else(|| Self::invalid(module, "native call-site table exceeds usize"))?;
        let mut bytes = vec![0xff, 0xff, DW_EH_PE_udata4.0];
        Self::write_uleb(byte_len as u64, &mut bytes);
        for (start, length, landing) in entries {
            Self::write_u32(start, endian, &mut bytes);
            Self::write_u32(length, endian, &mut bytes);
            Self::write_u32(landing, endian, &mut bytes);
            bytes.push(0);
        }

        Ok(bytes)
    }

    /// Insert one Gimli relocation target.
    fn target(&mut self, target: native::UnwindTarget) -> usize {
        let index = self.targets.len();
        self.targets.push(target);

        index
    }

    /// Convert one Gimli relocation into native object terms.
    fn relocation(&self, relocation: Relocation) -> Result<native::UnwindRelocation, EmitError> {
        let target = match relocation.target {
            RelocationTarget::Symbol(index) => {
                self.targets.get(index).copied().ok_or_else(|| {
                    Self::invalid(self.module, "native unwind relocation target is absent")
                })?
            }
            RelocationTarget::Section(_) => {
                return Err(Self::invalid(
                    self.module,
                    "native unwind uses an unsupported section relocation",
                ));
            }
        };
        let kind = match (relocation.eh_pe, relocation.size) {
            (Some(encoding), 4) if encoding.application() == DW_EH_PE_pcrel => {
                native::RelocationKind::Relative32
            }
            (Some(encoding), 8) if encoding.application() == DW_EH_PE_absptr => {
                native::RelocationKind::Absolute64
            }
            (None, 8) => native::RelocationKind::Absolute64,
            _ => {
                return Err(Self::invalid(
                    self.module,
                    "native unwind uses an unsupported relocation encoding",
                ));
            }
        };
        let offset = u32::try_from(relocation.offset)
            .map_err(|_| Self::invalid(self.module, "native unwind relocation exceeds u32"))?;

        Ok(native::UnwindRelocation::new(
            FRAME_SECTION,
            offset,
            target,
            relocation.addend,
            kind,
        ))
    }

    /// Write one target-endian 32-bit integer.
    fn write_u32(value: u32, endian: RunTimeEndian, bytes: &mut Vec<u8>) {
        match endian {
            RunTimeEndian::Little => bytes.extend_from_slice(&value.to_le_bytes()),
            RunTimeEndian::Big => bytes.extend_from_slice(&value.to_be_bytes()),
        }
    }

    /// Write one unsigned LEB128 integer.
    fn write_uleb(mut value: u64, bytes: &mut Vec<u8>) {
        loop {
            let byte = value as u8 & 0x7f;
            value >>= 7;
            if value == 0 {
                bytes.push(byte);

                break;
            }
            bytes.push(byte | 0x80);
        }
    }

    /// Build one native emission diagnostic.
    fn invalid(module: ModuleId, message: impl Into<String>) -> EmitError {
        EmitError::Internal {
            anchor: module.into(),
            module,
            message: message.into(),
        }
    }
}

impl RelocateWriter for FrameWriter {
    type Writer = EndianVec<RunTimeEndian>;

    /// Return immutable target-endian bytes.
    fn writer(&self) -> &Self::Writer {
        &self.bytes
    }

    /// Return mutable target-endian bytes.
    fn writer_mut(&mut self) -> &mut Self::Writer {
        &mut self.bytes
    }

    /// Retain one requested frame relocation.
    fn relocate(&mut self, relocation: Relocation) {
        self.relocations.push(relocation);
    }
}
