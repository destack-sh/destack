use destack_bytecode as bytecode;
use destack_mir as mir;

use crate::{EmitError, ObjectEmitter};

use super::FunctionEmitter;

/// The byte width of one register word.
const WORD_BYTES: u32 = size_of::<u64>() as u32;

/// One selected place, addressed by a register.
#[derive(Debug, Clone, Copy)]
pub(super) struct PlaceAddress {
    /// The register holding the selected address.
    pub(super) address: bytecode::RegisterId,
    /// The memory the address points into.
    pub(super) kind: bytecode::Address,
    /// The register holding the second descriptor word, like a slice length, when selected.
    pub(super) metadata: Option<bytecode::RegisterId>,
}

impl<'a> FunctionEmitter<'a> {
    /// Select the storage of one place, into the given register when one is requested.
    pub(super) fn emit_place(
        &mut self,
        place: &mir::Place,
        into: Option<bytecode::RegisterId>,
    ) -> Result<PlaceAddress, EmitError> {
        let steps = self
            .optimized
            .layouts
            .address_steps(place, self.function_id, &self.optimized.tree)
            .map_err(|error| self.internal(&error.to_string()))?;
        let mut target = into;

        // read a root descriptor from its registers when the place follows it
        let (mut selected, steps) = match (place.origin, steps.split_first()) {
            (
                mir::PlaceOrigin::Local(local),
                Some((mir::AddressStep::Follow(descriptor), rest)),
            ) => (
                self.register_descriptor(self.local(local)?, *descriptor)?,
                rest,
            ),
            (
                mir::PlaceOrigin::Value(value),
                Some((mir::AddressStep::Follow(descriptor), rest)),
            ) => (
                self.register_descriptor(self.register(value)?, *descriptor)?,
                rest,
            ),
            // address a register range in the frame
            (mir::PlaceOrigin::Local(local), _) => {
                let registers = self.local(local)?;

                (
                    self.emit_frame_address(registers, &mut target)?,
                    steps.as_slice(),
                )
            }
            (mir::PlaceOrigin::Value(value), _) => {
                let registers = self.register(value)?;

                (
                    self.emit_frame_address(registers, &mut target)?,
                    steps.as_slice(),
                )
            }
            // address a global
            (mir::PlaceOrigin::Global(global), _) => {
                let address = self.place_target(&mut target)?;
                let mut instruction =
                    bytecode::InstructionBuilder::new(bytecode::Opcode::GLOBAL_ADDRESS);
                instruction.global(self.types.global_id(global)?.0);
                self.encode(instruction, &[bytecode::RegisterSpan::new(address, 1)])?;

                (PlaceAddress::reference(address), steps.as_slice())
            }
        };

        // apply each address computation in the one target register
        for (index, step) in steps.iter().enumerate() {
            selected = match *step {
                mir::AddressStep::Follow(descriptor) => {
                    // read the metadata word before following the address when the place ends here
                    let is_last = index + 1 == steps.len();
                    let metadata = match descriptor.metadata {
                        Some(offset) if is_last => {
                            let metadata = self.word_scratch()?;
                            self.emit_offset(selected.address, u64::from(offset), metadata)?;
                            self.emit_address_word(metadata, selected.kind, metadata)?;

                            Some(metadata)
                        }
                        _ => None,
                    };

                    // load the address word
                    let address = self.place_target(&mut target)?;
                    let source = match descriptor.address {
                        0 => selected.address,
                        offset => {
                            self.emit_offset(selected.address, u64::from(offset), address)?;

                            address
                        }
                    };
                    self.emit_address_word(source, selected.kind, address)?;

                    PlaceAddress {
                        address,
                        kind: Self::address_kind(descriptor.kind),
                        metadata,
                    }
                }
                // advance by a constant offset
                mir::AddressStep::Offset(offset) => {
                    let address = match offset {
                        0 => selected.address,
                        offset => {
                            let address = self.place_target(&mut target)?;
                            self.emit_offset(selected.address, offset, address)?;

                            address
                        }
                    };

                    PlaceAddress {
                        address,
                        kind: selected.kind,
                        metadata: None,
                    }
                }
                // advance by a scaled runtime index
                mir::AddressStep::Index {
                    index,
                    stride,
                    length,
                } => {
                    let address = self.place_target(&mut target)?;
                    let mut instruction =
                        bytecode::InstructionBuilder::new(bytecode::Opcode::ADDRESS_ADD_SCALED);
                    instruction.register(selected.address);
                    instruction.register(self.word(index)?);
                    instruction.u32(stride);
                    self.encode(instruction, &[bytecode::RegisterSpan::new(address, 1)])?;

                    PlaceAddress {
                        address,
                        kind: selected.kind,
                        metadata: length.map(|length| self.word(length)).transpose()?,
                    }
                }
            };
        }

        // copy an address that no computation produced
        if let Some(into) = into
            && selected.address != into
        {
            let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::MOVE);
            instruction.register(selected.address);
            self.encode(instruction, &[bytecode::RegisterSpan::new(into, 1)])?;
            selected.address = into;
        }

        Ok(selected)
    }

    /// Return the register one place computes its address in, a scratch word allocated once.
    fn place_target(
        &mut self,
        target: &mut Option<bytecode::RegisterId>,
    ) -> Result<bytecode::RegisterId, EmitError> {
        match *target {
            Some(target) => Ok(target),
            None => {
                let scratch = self.word_scratch()?;
                *target = Some(scratch);

                Ok(scratch)
            }
        }
    }

    /// Address one frame register range in the place target.
    fn emit_frame_address(
        &mut self,
        registers: bytecode::RegisterSpan,
        target: &mut Option<bytecode::RegisterId>,
    ) -> Result<PlaceAddress, EmitError> {
        let address = self.place_target(target)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::FRAME_ADDRESS);
        instruction.span(registers);
        self.encode(instruction, &[bytecode::RegisterSpan::new(address, 1)])?;

        Ok(PlaceAddress::reference(address))
    }

    /// Select the address and metadata words of one descriptor held in registers.
    fn register_descriptor(
        &self,
        registers: bytecode::RegisterSpan,
        descriptor: mir::Descriptor,
    ) -> Result<PlaceAddress, EmitError> {
        Ok(PlaceAddress {
            address: self.descriptor_word(registers, descriptor.address)?,
            kind: Self::address_kind(descriptor.kind),
            metadata: descriptor
                .metadata
                .map(|offset| self.descriptor_word(registers, offset))
                .transpose()?,
        })
    }

    /// Return the register holding one descriptor word.
    fn descriptor_word(
        &self,
        registers: bytecode::RegisterSpan,
        byte_offset: u32,
    ) -> Result<bytecode::RegisterId, EmitError> {
        let word = byte_offset / WORD_BYTES;
        if word >= u32::from(registers.word_count) {
            return Err(self.internal("descriptor word outside its registers"));
        }

        Ok(bytecode::RegisterId(registers.start.0 + word as u16))
    }

    /// Return the descriptor words of one reference-like type.
    fn descriptor(&self, ty: mir::TypeId) -> Result<mir::Descriptor, EmitError> {
        mir::Descriptor::new(ty, &self.optimized.tree, &self.optimized.layouts)
            .map_err(|error| self.internal(&error.to_string()))
    }

    /// Return the bytecode addressing mode of one address kind.
    const fn address_kind(kind: mir::AddressKind) -> bytecode::Address {
        match kind {
            mir::AddressKind::Reference => bytecode::Address::Reference,
            mir::AddressKind::Pointer => bytecode::Address::Pointer,
        }
    }

    /// Allocate one scratch word register.
    fn word_scratch(&mut self) -> Result<bytecode::RegisterId, EmitError> {
        Ok(self
            .scratch(bytecode::ValueType::scalar(bytecode::Scalar::Uint64))?
            .start)
    }

    /// Load an address word through the selected addressing mode.
    fn emit_address_word(
        &mut self,
        address: bytecode::RegisterId,
        kind: bytecode::Address,
        result: bytecode::RegisterId,
    ) -> Result<(), EmitError> {
        let scalar = bytecode::Scalar::Uint64;
        let opcode = bytecode::Opcode::memory(bytecode::MemoryOperation::Load, kind, scalar, false);
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(address);

        self.encode(instruction, &[bytecode::RegisterSpan::new(result, 1)])
    }

    /// Add a fixed byte offset to an address.
    fn emit_offset(
        &mut self,
        address: bytecode::RegisterId,
        offset: u64,
        result: bytecode::RegisterId,
    ) -> Result<(), EmitError> {
        // select the immediate encoding when the offset fits its signed operand
        let instruction = match i32::try_from(offset) {
            Ok(offset) => {
                let mut instruction =
                    bytecode::InstructionBuilder::new(bytecode::Opcode::ADDRESS_ADD_IMMEDIATE);
                instruction.register(address);
                instruction.i32(offset);

                instruction
            }
            Err(_) => {
                let word = bytecode::ValueType::scalar(bytecode::Scalar::Uint64);
                let constant = self.scratch(word)?;
                let instruction = self.scalar_constant(word, offset)?;
                self.encode(instruction, &[constant])?;
                let mut instruction =
                    bytecode::InstructionBuilder::new(bytecode::Opcode::ADDRESS_ADD);
                instruction.register(address);
                instruction.register(constant.start);

                instruction
            }
        };

        self.encode(instruction, &[bytecode::RegisterSpan::new(result, 1)])
    }

    /// Materialize one reference-like descriptor from its selected place.
    pub(super) fn emit_address(
        &mut self,
        destination: mir::Value,
        place: &mir::Place,
    ) -> Result<(), EmitError> {
        let descriptor = self.descriptor(self.value_type(destination)?)?;
        let registers = self.register(destination)?;
        let address = self.descriptor_word(registers, descriptor.address)?;
        let selected = self.emit_place(place, Some(address))?;

        // write the selected metadata word beside the address
        match (selected.metadata, descriptor.metadata) {
            (None, None) => Ok(()),
            (Some(metadata), Some(offset)) => {
                let target = self.descriptor_word(registers, offset)?;
                if metadata == target {
                    return Ok(());
                }
                let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::MOVE);
                instruction.register(metadata);

                self.encode(instruction, &[bytecode::RegisterSpan::new(target, 1)])
            }
            _ => Err(self.internal("address disagrees with its result descriptor")),
        }
    }

    /// Release one unique allocation.
    pub(super) fn emit_release(&mut self, value: mir::Value) -> Result<(), EmitError> {
        let owner = self.address_register(value)?;

        // select the release that destroys the allocation's values
        let ty = self.value_type(value)?;
        let is_destroying = ObjectEmitter::release_destroys(self.module, self.optimized, ty)?;
        let opcode = if is_destroying {
            bytecode::Opcode::RELEASE
        } else {
            bytecode::Opcode::FREE
        };
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(owner);

        self.encode(instruction, &[])
    }

    /// Record one managed reference write for the collector.
    pub(super) fn emit_barrier(
        &mut self,
        object: mir::Value,
        offset: mir::Value,
        byte_len: mir::Value,
    ) -> Result<(), EmitError> {
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::BARRIER);
        instruction.register(self.address_register(object)?);
        instruction.register(self.word(offset)?);
        instruction.register(self.word(byte_len)?);

        self.encode(instruction, &[])
    }

    /// Load one value from a place, volatile when requested.
    pub(super) fn emit_load(
        &mut self,
        destination: mir::Value,
        place: &mir::Place,
        is_volatile: bool,
    ) -> Result<(), EmitError> {
        let ty = self.register_type(destination)?;

        // transfer a whole nonvolatile local directly between register ranges
        if !is_volatile
            && place.path.is_root()
            && let mir::PlaceOrigin::Local(local) = place.origin
        {
            let source = self.local(local)?;
            let destination = self.register(destination)?;

            return self.emit_move(source, destination, ty);
        }

        // read a scalar or a byte range through the selected address
        let selected = self.emit_place(place, None)?;
        let instruction = if let Some(scalar) = ty.scalar_type() {
            let opcode = bytecode::Opcode::memory(
                bytecode::MemoryOperation::Load,
                selected.kind,
                scalar,
                is_volatile,
            );
            let mut instruction = bytecode::InstructionBuilder::new(opcode);
            instruction.register(selected.address);

            instruction
        } else {
            let opcode = bytecode::Opcode::memory_range(
                bytecode::MemoryOperation::Load,
                selected.kind,
                is_volatile,
            );
            let mut instruction = bytecode::InstructionBuilder::new(opcode);
            instruction.register(selected.address);
            instruction.u32(self.types.byte_len(self.value_type(destination)?)?);

            instruction
        };
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Store one value into a place, volatile when requested.
    pub(super) fn emit_store(
        &mut self,
        place: &mir::Place,
        value: mir::Value,
        is_volatile: bool,
    ) -> Result<(), EmitError> {
        let ty = self.register_type(value)?;

        // transfer a whole nonvolatile local directly between register ranges
        if !is_volatile
            && place.path.is_root()
            && let mir::PlaceOrigin::Local(local) = place.origin
        {
            let source = self.register(value)?;
            let destination = self.local(local)?;

            return self.emit_move(source, destination, ty);
        }

        // write a scalar or a byte range through the selected address
        let selected = self.emit_place(place, None)?;
        let instruction = if let Some(scalar) = ty.scalar_type() {
            let opcode = bytecode::Opcode::memory(
                bytecode::MemoryOperation::Store,
                selected.kind,
                scalar,
                is_volatile,
            );
            let mut instruction = bytecode::InstructionBuilder::new(opcode);
            instruction.register(selected.address);
            instruction.register(self.word(value)?);

            instruction
        } else {
            let opcode = bytecode::Opcode::memory_range(
                bytecode::MemoryOperation::Store,
                selected.kind,
                is_volatile,
            );
            let mut instruction = bytecode::InstructionBuilder::new(opcode);
            instruction.register(selected.address);
            instruction.span(self.register(value)?);
            instruction.u32(self.types.byte_len(self.value_type(value)?)?);

            instruction
        };

        self.encode(instruction, &[])
    }

    /// Return the addressing mode of one reference or pointer value.
    fn address(&self, value: mir::Value) -> Result<bytecode::Address, EmitError> {
        let descriptor = self.descriptor(self.value_type(value)?)?;

        Ok(Self::address_kind(descriptor.kind))
    }

    /// Emit one machine memory intrinsic.
    pub(super) fn emit_memory_intrinsic(
        &mut self,
        destination: Option<mir::Value>,
        intrinsic: mir::Intrinsic,
        arguments: mir::ValueSlice,
    ) -> Result<(), EmitError> {
        // read the intrinsic arguments
        let arguments = self.optimized.tree.get_values(arguments).to_vec();
        match intrinsic {
            mir::Intrinsic::Memcpy | mir::Intrinsic::Memmove => {
                let [target, source, length] = arguments.as_slice() else {
                    return Err(self.internal("memory transfer requires three arguments"));
                };
                let operation = if intrinsic == mir::Intrinsic::Memcpy {
                    bytecode::Transfer::Copy
                } else {
                    bytecode::Transfer::Move
                };
                let opcode = bytecode::Opcode::transfer(
                    operation,
                    self.address(*target)?,
                    self.address(*source)?,
                    false,
                );
                let mut instruction = bytecode::InstructionBuilder::new(opcode);
                instruction.register(self.word(*target)?);
                instruction.register(self.word(*source)?);
                instruction.register(self.word(*length)?);

                self.encode(instruction, &[])
            }
            mir::Intrinsic::Memset => {
                let [target, byte, length] = arguments.as_slice() else {
                    return Err(self.internal("memory fill requires three arguments"));
                };
                let opcode = bytecode::Opcode::fill(self.address(*target)?, false);
                let mut instruction = bytecode::InstructionBuilder::new(opcode);
                instruction.register(self.word(*target)?);
                instruction.register(self.word(*byte)?);
                instruction.register(self.word(*length)?);

                self.encode(instruction, &[])
            }
            mir::Intrinsic::Memcmp => {
                let [left, right, length] = arguments.as_slice() else {
                    return Err(self.internal("memory comparison requires three arguments"));
                };
                let destination = destination
                    .ok_or_else(|| self.internal("memory comparison result is missing"))?;
                let opcode =
                    bytecode::Opcode::compare(self.address(*left)?, self.address(*right)?, false);
                let mut instruction = bytecode::InstructionBuilder::new(opcode);
                instruction.register(self.word(*left)?);
                instruction.register(self.word(*right)?);
                instruction.register(self.word(*length)?);
                let destination = self.register(destination)?;

                self.encode(instruction, &[destination])
            }
            mir::Intrinsic::PrefetchRead | mir::Intrinsic::PrefetchWrite => {
                let [pointer] = arguments.as_slice() else {
                    return Err(self.internal("prefetch requires one argument"));
                };
                let operation = if intrinsic == mir::Intrinsic::PrefetchRead {
                    bytecode::Prefetch::Read
                } else {
                    bytecode::Prefetch::Write
                };
                let opcode = bytecode::Opcode::prefetch(operation, self.address(*pointer)?);
                let mut instruction = bytecode::InstructionBuilder::new(opcode);
                instruction.register(self.word(*pointer)?);

                self.encode(instruction, &[])
            }
            mir::Intrinsic::VolatileLoad => {
                let [pointer] = arguments.as_slice() else {
                    return Err(self.internal("volatile load requires one argument"));
                };
                let destination =
                    destination.ok_or_else(|| self.internal("volatile load result is missing"))?;
                let place = mir::Place::value(*pointer).with_projection(mir::Projection::Deref);

                self.emit_load(destination, &place, true)
            }
            mir::Intrinsic::VolatileStore => {
                let [pointer, value] = arguments.as_slice() else {
                    return Err(self.internal("volatile store requires two arguments"));
                };

                let place = mir::Place::value(*pointer).with_projection(mir::Projection::Deref);

                self.emit_store(&place, *value, true)
            }
            mir::Intrinsic::PointerByteOffsetFrom => {
                let [pointer, origin] = arguments.as_slice() else {
                    return Err(self.internal("pointer difference requires two arguments"));
                };
                let destination = destination
                    .ok_or_else(|| self.internal("pointer difference result is missing"))?;
                let pointer_address = self.address(*pointer)?;
                let origin_address = self.address(*origin)?;
                if pointer_address != origin_address {
                    return Err(self.internal("pointer difference requires one address space"));
                }
                let mut instruction =
                    bytecode::InstructionBuilder::new(bytecode::Opcode::ADDRESS_DIFF);
                instruction.register(self.word(*pointer)?);
                instruction.register(self.word(*origin)?);
                let destination = self.register(destination)?;

                self.encode(instruction, &[destination])
            }
            _ => Err(self.internal("intrinsic is not a memory operation")),
        }
    }

    /// Return the register holding the address word of one reference-like value.
    fn address_register(&self, value: mir::Value) -> Result<bytecode::RegisterId, EmitError> {
        let descriptor = self.descriptor(self.value_type(value)?)?;

        self.descriptor_word(self.register(value)?, descriptor.address)
    }
}

impl PlaceAddress {
    /// Create one world reference address outside any descriptor.
    const fn reference(address: bytecode::RegisterId) -> Self {
        Self {
            address,
            kind: bytecode::Address::Reference,
            metadata: None,
        }
    }
}
