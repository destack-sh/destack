use destack_bytecode as bytecode;
use destack_mir as mir;

use crate::EmitError;

use super::FunctionEmitter;

impl<'a> FunctionEmitter<'a> {
    /// Project one fixed field from an addressable aggregate.
    pub(super) fn emit_field_address(
        &mut self,
        destination: mir::Value,
        aggregate: mir::Value,
        field: u32,
    ) -> Result<(), EmitError> {
        let mut aggregate_type = self
            .optimized
            .tree
            .storage_type(self.value_type(aggregate)?);
        if let mir::Type::Reference { pointee, .. } | mir::Type::Pointer { pointee, .. } =
            self.optimized.tree.get(aggregate_type)
        {
            aggregate_type = self.optimized.tree.storage_type(*pointee);
        }
        let byte_offset = self.types.field(aggregate_type, field)?.offset;

        // materialize the aggregate's stable address before applying its field offset
        self.emit_base_address(destination, aggregate)?;
        if byte_offset == 0 {
            return Ok(());
        }

        self.emit_address_add_immediate(destination, destination, byte_offset)
    }

    /// Project one runtime index from an addressable indexed value.
    pub(super) fn emit_element_address(
        &mut self,
        destination: mir::Value,
        base: mir::Value,
        index: mir::Value,
    ) -> Result<(), EmitError> {
        let base_type = self.value_type(base)?;
        let stride = self.types.element_stride(base_type)?;

        // materialize the indexed value's stable address before scaling its index
        self.emit_base_address(destination, base)?;
        let mut instruction =
            bytecode::InstructionBuilder::new(bytecode::Opcode::ADDRESS_ADD_SCALED);
        instruction.register(self.word(destination)?);
        instruction.register(self.word(index)?);
        instruction.u32(stride);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit the stable address carried by or assigned to one MIR value.
    fn emit_base_address(
        &mut self,
        destination: mir::Value,
        value: mir::Value,
    ) -> Result<(), EmitError> {
        let ty = self.optimized.tree.storage_type(self.value_type(value)?);
        let source = match self.optimized.tree.get(ty) {
            // direct references and pointers already contain address bits
            mir::Type::Reference { .. } | mir::Type::Pointer { .. } => self.word(value)?,

            // indexed fat pointers keep their backing reference in one representation word
            mir::Type::Slice { .. } => {
                self.representation_register(self.register(value)?, value)?
            }

            // inline aggregates occupy stable bytecode frame registers
            mir::Type::FixedArray { .. }
            | mir::Type::Tuple { .. }
            | mir::Type::Struct { .. }
            | mir::Type::Variant { .. } => {
                let mut instruction =
                    bytecode::InstructionBuilder::new(bytecode::Opcode::FRAME_ADDRESS);
                instruction.span(self.register(value)?);
                let destination = self.register(destination)?;

                return self.encode(instruction, &[destination]);
            }
            _ => return Err(self.internal("address base is not addressable")),
        };
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::MOVE);
        instruction.register(source);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Release one unique representation's backing allocation.
    pub(super) fn emit_free(&mut self, value: mir::Value) -> Result<(), EmitError> {
        let owner = self.representation_register(self.register(value)?, value)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::FREE);
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
        let reference = self.representation_reference(object)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::BARRIER);
        instruction.register(self.representation_register(self.register(object)?, object)?);
        instruction.reference(reference.kind(), reference.storage());
        instruction.register(self.word(offset)?);
        instruction.register(self.word(byte_len)?);

        self.encode(instruction, &[])
    }

    /// Emit one relocatable global reference.
    pub(super) fn emit_global_address(
        &mut self,
        destination: mir::Value,
        global: mir::GlobalId,
    ) -> Result<(), EmitError> {
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::GLOBAL_ADDRESS);
        let global = self.types.global_id(global)?;
        instruction.global(global.0);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Copy one MIR local into an SSA value range.
    pub(super) fn emit_local_get(
        &mut self,
        destination: mir::Value,
        local: mir::LocalId,
    ) -> Result<(), EmitError> {
        let ty = self.register_type(destination)?;
        let source = self.local(local)?;
        let destination = self.register(destination)?;

        self.emit_move(source, destination, ty)
    }

    /// Return one MIR local's stable frame reference.
    pub(super) fn emit_local_address(
        &mut self,
        destination: mir::Value,
        local: mir::LocalId,
    ) -> Result<(), EmitError> {
        let local = self.local(local)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::FRAME_ADDRESS);
        instruction.span(local);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Copy one MIR value into a local register range.
    pub(super) fn emit_local_set(
        &mut self,
        local: mir::LocalId,
        value: mir::Value,
    ) -> Result<(), EmitError> {
        let source = self.register(value)?;
        let destination = self.local(local)?;
        let ty = self.register_type(value)?;

        self.emit_move(source, destination, ty)
    }

    /// Emit one scalar or packed value load.
    pub(super) fn emit_load(
        &mut self,
        destination: mir::Value,
        reference: mir::Value,
        result_type: mir::TypeId,
    ) -> Result<(), EmitError> {
        self.emit_load_with_volatility(destination, reference, result_type, false)
    }

    /// Emit one volatile scalar or packed value load.
    pub(super) fn emit_volatile_load(
        &mut self,
        destination: mir::Value,
        reference: mir::Value,
        result_type: mir::TypeId,
    ) -> Result<(), EmitError> {
        self.emit_load_with_volatility(destination, reference, result_type, true)
    }

    /// Emit one scalar or packed value load with explicit volatility.
    fn emit_load_with_volatility(
        &mut self,
        destination: mir::Value,
        reference: mir::Value,
        result_type: mir::TypeId,
        is_volatile: bool,
    ) -> Result<(), EmitError> {
        let ty = self.register_type(destination)?;
        let address = self.address(reference)?;
        let reference = self.word(reference)?;
        let instruction = if let Some(scalar) = ty.scalar_type() {
            let opcode = bytecode::Opcode::memory(
                bytecode::MemoryOperation::Load,
                address,
                scalar,
                is_volatile,
            );
            let mut instruction = bytecode::InstructionBuilder::new(opcode);
            instruction.register(reference);

            instruction
        } else {
            let opcode = bytecode::Opcode::memory_range(
                bytecode::MemoryOperation::Load,
                address,
                is_volatile,
            );
            let mut instruction = bytecode::InstructionBuilder::new(opcode);
            instruction.register(reference);
            instruction.u32(self.types.byte_len(result_type)?);

            instruction
        };
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit one scalar or packed value store.
    pub(super) fn emit_store(
        &mut self,
        reference: mir::Value,
        value: mir::Value,
    ) -> Result<(), EmitError> {
        self.emit_store_with_volatility(reference, value, false)
    }

    /// Emit one volatile scalar or packed value store.
    pub(super) fn emit_volatile_store(
        &mut self,
        reference: mir::Value,
        value: mir::Value,
    ) -> Result<(), EmitError> {
        self.emit_store_with_volatility(reference, value, true)
    }

    /// Emit one scalar or packed value store with explicit volatility.
    fn emit_store_with_volatility(
        &mut self,
        reference: mir::Value,
        value: mir::Value,
        is_volatile: bool,
    ) -> Result<(), EmitError> {
        let ty = self.register_type(value)?;
        let address = self.address(reference)?;
        let reference = self.word(reference)?;
        let instruction = if let Some(scalar) = ty.scalar_type() {
            let opcode = bytecode::Opcode::memory(
                bytecode::MemoryOperation::Store,
                address,
                scalar,
                is_volatile,
            );
            let mut instruction = bytecode::InstructionBuilder::new(opcode);
            instruction.register(reference);
            instruction.register(self.word(value)?);

            instruction
        } else {
            let value_type = self
                .function
                .value_type(value)
                .ok_or_else(|| self.internal("missing stored value type"))?;
            let opcode = bytecode::Opcode::memory_range(
                bytecode::MemoryOperation::Store,
                address,
                is_volatile,
            );
            let mut instruction = bytecode::InstructionBuilder::new(opcode);
            instruction.register(reference);
            instruction.span(self.register(value)?);
            instruction.u32(self.types.byte_len(value_type)?);

            instruction
        };

        self.encode(instruction, &[])
    }

    /// Return the addressing mode selected by one MIR reference.
    pub(super) fn address(&self, reference: mir::Value) -> Result<bytecode::Address, EmitError> {
        let ty = self
            .optimized
            .tree
            .storage_type(self.value_type(reference)?);
        let address = match self.optimized.tree.get(ty) {
            mir::Type::Pointer { .. } => bytecode::Address::Pointer,
            mir::Type::Reference { .. } => bytecode::Address::Reference,
            _ => return Err(self.internal("memory access requires a reference or pointer")),
        };

        Ok(address)
    }

    /// Emit one machine memory intrinsic.
    pub(super) fn emit_memory_intrinsic(
        &mut self,
        destination: Option<mir::Value>,
        intrinsic: mir::Intrinsic,
        arguments: mir::ValueSlice,
    ) -> Result<(), EmitError> {
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
                let result_type = self.value_type(destination)?;

                self.emit_volatile_load(destination, *pointer, result_type)
            }
            mir::Intrinsic::VolatileStore => {
                let [pointer, value] = arguments.as_slice() else {
                    return Err(self.internal("volatile store requires two arguments"));
                };

                self.emit_volatile_store(*pointer, *value)
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

    /// Add one fixed byte offset to a reference or pointer.
    pub(super) fn emit_address_add_immediate(
        &mut self,
        destination: mir::Value,
        base: mir::Value,
        byte_offset: u32,
    ) -> Result<(), EmitError> {
        let byte_offset = i32::try_from(byte_offset)
            .map_err(|_| self.internal("reference field offset exceeds i32"))?;
        let mut instruction =
            bytecode::InstructionBuilder::new(bytecode::Opcode::ADDRESS_ADD_IMMEDIATE);
        instruction.register(self.word(base)?);
        instruction.i32(byte_offset);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Return the reference carried by one reference-like MIR value.
    pub(super) fn representation_reference(
        &self,
        value: mir::Value,
    ) -> Result<bytecode::ReferenceType, EmitError> {
        let ty = self.register_type(value)?;
        ty.reference_type()
            .or_else(|| ty.slice_reference())
            .or_else(|| ty.dynamic_reference())
            .or_else(|| ty.function_reference())
            .ok_or_else(|| self.internal("ownership operation requires a reference representation"))
    }

    /// Return the register carrying one reference-like value's backing reference.
    pub(super) fn representation_register(
        &self,
        value: bytecode::RegisterSpan,
        source: mir::Value,
    ) -> Result<bytecode::RegisterId, EmitError> {
        let ty = self.optimized.tree.storage_type(self.value_type(source)?);
        let offset = usize::from(matches!(
            self.optimized.tree.get(ty),
            mir::Type::Function { .. }
        ));
        if offset >= usize::from(value.word_count) {
            return Err(self.internal("reference representation has no backing reference word"));
        }

        Ok(bytecode::RegisterId(value.start.0 + offset as u16))
    }
}
