use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use cranelift_module::Module;
use destack_bytecode as bytecode;
use destack_mir as mir;
use destack_native as native;

use crate::{EmitError, TensorCommand, TensorEmitter};

use super::FunctionEmitter;

/// Compact word frame used by one native tensor command.
struct TensorFrame {
    /// Physical registers keyed by MIR value.
    registers: Vec<Option<bytecode::RegisterSpan>>,
    /// Values copied into the frame before execution.
    inputs: Vec<mir::Value>,
    /// Total frame width in machine words.
    register_count: u16,
}

impl FunctionEmitter<'_> {
    /// Emit one tensor command through the shared runtime executor.
    pub(super) fn emit_tensor(
        &mut self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let frame = self.tensor_frame(instruction)?;
        let command = TensorEmitter::new(
            self.module,
            self.optimized,
            self.object,
            self.function,
            &frame.registers,
        )
        .emit(instruction_id, instruction)?;
        let descriptor = self.tensor_descriptor(command, builder)?;
        let byte_len = u32::from(frame.register_count) * size_of::<u64>() as u32;
        let words = self.allocate_bytes(byte_len, align_of::<u64>() as u32, builder);

        // marshal only values read by this command
        for input in frame.inputs {
            let registers = frame.registers[input.id() as usize]
                .ok_or_else(|| self.invalid("native tensor input has no register range"))?;
            let destination = Self::tensor_address(words, registers.start, builder);
            let ty = self.value_type(input)?;
            let value_type = self.types.value(ty)?;
            let value = self.value(input, builder)?;
            self.store_words(destination, value, ty, value_type, builder)?;
        }

        // execute one compact descriptor over its call-local word frame
        let register_count = builder
            .ins()
            .iconst(self.types.pointer(), i64::from(frame.register_count));
        self.emit_runtime(
            native::abi::Operation::TensorExecute,
            &[descriptor, words, register_count],
            builder,
        )?;

        // project the command result back into ordinary native SSA
        if let Some(destination) = instruction.destination() {
            let registers = frame.registers[destination.id() as usize]
                .ok_or_else(|| self.invalid("native tensor result has no register range"))?;
            let address = Self::tensor_address(words, registers.start, builder);
            let ty = self.value_type(destination)?;
            let value_type = self.types.value(ty)?;
            let value = self.load(address, value_type, builder)?;
            self.set(destination, value, builder)?;
        }

        Ok(())
    }

    /// Assign dense command-local registers to one tensor instruction.
    fn tensor_frame(&self, instruction: &mir::Instruction) -> Result<TensorFrame, EmitError> {
        let body = self
            .function
            .body()
            .ok_or_else(|| self.invalid("native tensor function has no body"))?;
        let mut registers = vec![None; body.value_capacity()];
        let mut inputs = Vec::new();
        let mut register_count = 0u16;

        // assign each read value once in encoded operand order
        for value in instruction.reads(&self.optimized.tree) {
            if registers[value.id() as usize].is_some() {
                continue;
            }
            let ty = self.value_type(value)?;
            let word_count = self.types.value(ty)?.word_count();
            let word_count = u16::try_from(word_count)
                .map_err(|_| self.invalid("native tensor value exceeds u16 words"))?;
            let span =
                bytecode::RegisterSpan::new(bytecode::RegisterId(register_count), word_count);
            register_count = register_count
                .checked_add(word_count)
                .ok_or_else(|| self.invalid("native tensor frame exceeds u16 words"))?;
            registers[value.id() as usize] = Some(span);
            inputs.push(value);
        }

        // reserve the result after every input range
        if let Some(destination) = instruction.destination() {
            let ty = self.value_type(destination)?;
            let word_count = self.types.value(ty)?.word_count();
            let word_count = u16::try_from(word_count)
                .map_err(|_| self.invalid("native tensor result exceeds u16 words"))?;
            let span =
                bytecode::RegisterSpan::new(bytecode::RegisterId(register_count), word_count);
            register_count = register_count
                .checked_add(word_count)
                .ok_or_else(|| self.invalid("native tensor frame exceeds u16 words"))?;
            registers[destination.id() as usize] = Some(span);
        }

        Ok(TensorFrame {
            registers,
            inputs,
            register_count,
        })
    }

    /// Return one command-local register address.
    fn tensor_address(
        words: cir::Value,
        register: bytecode::RegisterId,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> cir::Value {
        let offset = i64::from(register.0) * size_of::<u64>() as i64;
        if offset == 0 {
            words
        } else {
            builder.ins().iadd_imm_u(words, offset)
        }
    }

    /// Pack one tensor command into an addressable immutable image block.
    fn tensor_descriptor(
        &mut self,
        command: TensorCommand,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let destinations = command.destination.into_iter().collect::<Vec<_>>();
        let mut function = bytecode::FunctionBuilder::new();
        function
            .emit(command.instruction, &destinations)
            .map_err(|error| self.invalid(&error.to_string()))?;
        let body = function
            .build()
            .map_err(|error| self.invalid(&error.to_string()))?;
        let mut relocations = Vec::with_capacity(body.relocations.len());

        // project bytecode identities into native object relocations
        for relocation in body.relocations {
            let start = relocation.byte_offset as usize;
            let end = start + size_of::<u32>();
            let bytes = body
                .code
                .get(start..end)
                .ok_or_else(|| self.invalid("native tensor relocation is out of range"))?;
            let index = u32::from_le_bytes(
                bytes
                    .try_into()
                    .map_err(|_| self.invalid("native tensor relocation has invalid width"))?,
            );
            let index = if relocation.tag == bytecode::RelocationTag::LAYOUT {
                let ty = self
                    .object
                    .type_id(index)
                    .ok_or_else(|| self.invalid("native tensor layout type is absent"))?;

                native::Index::Layout {
                    ty: ty.get() as u32,
                }
            } else if relocation.tag == bytecode::RelocationTag::ALLOCATION {
                native::Index::Allocation { site: index }
            } else {
                return Err(self.invalid("native tensor descriptor has unsupported relocation"));
            };
            let target = self.symbols.insert(native::Symbol::Index(index));
            relocations.push(native::Relocation::new(
                relocation.byte_offset,
                target,
                0,
                native::RelocationKind::Index32,
            ));
        }

        // retain the descriptor in the same position-independent image as generated code
        let block = native::BlockId(self.image_blocks.len() as u32);
        let descriptor =
            native::BlockBuilder::new(body.code, native::Alignment::TWO).relocations(relocations);
        self.image_blocks.push(Some(descriptor));
        let data = self
            .symbols
            .declare_block(block, self.output)
            .map_err(|error| self.invalid(&error.to_string()))?;
        let data = self.output.declare_data_in_func(data, builder.func);

        Ok(builder.ins().symbol_value(self.types.pointer(), data))
    }
}
