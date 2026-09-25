use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use cranelift_frontend::FunctionBuilder;
use destack_mir as mir;
use destack_program as program;

use crate::EmitError;

use super::memory::AliasRegion;

use super::super::r#type::ValueType;
use super::FunctionEmitter;

/// The largest machine-word count copied without a loop.
const INLINE_WORD_COUNT: u32 = 8;

/// One physical native value.
#[derive(Debug, Clone, Copy)]
pub(super) enum Value {
    /// One value held directly in Cranelift SSA.
    Direct(cir::Value),
    /// Two scalar fields held in Cranelift SSA.
    ScalarPair([cir::Value; 2]),
    /// Canonical value bytes addressed in memory.
    Address(cir::Value),
}

impl Value {
    /// Return the direct Cranelift value.
    pub(super) fn direct(self) -> Option<cir::Value> {
        match self {
            Self::Direct(value) => Some(value),
            Self::ScalarPair(_) | Self::Address(_) => None,
        }
    }

    /// Return this value as one scalar pair.
    pub(super) fn scalar_pair(self) -> Option<[cir::Value; 2]> {
        match self {
            Self::ScalarPair(values) => Some(values),
            Self::Direct(_) | Self::Address(_) => None,
        }
    }

    /// Return this value as one canonical address.
    pub(super) fn address(self) -> Option<cir::Value> {
        match self {
            Self::Direct(_) | Self::ScalarPair(_) => None,
            Self::Address(address) => Some(address),
        }
    }

    /// Create one value from flattened ABI parameters.
    pub(super) fn from_parameters(
        value_type: ValueType,
        parameters: &[cir::Value],
        index: &mut usize,
    ) -> Option<Self> {
        let value = match value_type {
            ValueType::Direct { .. } => Self::Direct(*parameters.get(*index)?),
            ValueType::ScalarPair { .. } => {
                Self::ScalarPair([*parameters.get(*index)?, *parameters.get(*index + 1)?])
            }
            ValueType::Indirect { .. } => Self::Address(*parameters.get(*index)?),
        };
        *index += value_type.abi_parameter_count();

        Some(value)
    }

    /// Append this value's flattened SSA fields.
    pub(super) fn append_values(self, values: &mut Vec<cir::Value>) {
        match self {
            Self::Direct(value) | Self::Address(value) => values.push(value),
            Self::ScalarPair(fields) => values.extend(fields),
        }
    }

    /// Append this value's flattened block arguments.
    pub(super) fn append_block_arguments(self, arguments: &mut Vec<cir::BlockArg>) {
        match self {
            Self::Direct(value) | Self::Address(value) => arguments.push(value.into()),
            Self::ScalarPair(fields) => arguments.extend(fields.map(cir::BlockArg::from)),
        }
    }

    /// Append one native value's block parameters.
    pub(super) fn block_parameters(
        value_type: ValueType,
        pointer: cir::Type,
        block: cir::Block,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Self {
        match value_type {
            ValueType::Direct { ty, .. } => Self::Direct(builder.append_block_param(block, ty)),
            ValueType::ScalarPair { fields, .. } => {
                Self::ScalarPair(fields.map(|field| builder.append_block_param(block, field.ty)))
            }
            ValueType::Indirect { .. } => Self::Address(builder.append_block_param(block, pointer)),
        }
    }
}

/// One addressable native local.
#[derive(Debug, Clone, Copy)]
pub(super) struct Local {
    /// Cranelift stack slot containing the local.
    pub(super) slot: cir::StackSlot,
    /// Stable identity preserved through native frame layout.
    pub(super) key: cir::StackSlotKey,
}

impl<'a> FunctionEmitter<'a> {
    /// Allocate canonical stack storage for MIR locals.
    pub(super) fn create_locals(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        // allocate storage for each local
        for local_id in self.function.locals() {
            let local = self.optimized.tree.get(*local_id);
            let value_type = self.types.value(local.ty)?;
            let key = self.next_key();
            let slot = builder.create_sized_stack_slot(cir::StackSlotData::new_with_key(
                cir::StackSlotKind::ExplicitSlot,
                value_type.byte_len(),
                value_type.alignment().trailing_zeros() as u8,
                key,
            ));
            self.locals.insert(*local_id, Local { slot, key });
        }

        Ok(())
    }

    /// Load one canonical value from memory.
    pub(super) fn load(
        &self,
        address: cir::Value,
        value_type: ValueType,
        builder: &mut FunctionBuilder<'_>,
    ) -> Result<Value, EmitError> {
        let flags = self.memory_flags(AliasRegion::World);
        match value_type {
            ValueType::Direct { ty, .. } => {
                let value = builder.ins().load(ty, flags, address, 0);

                Ok(Value::Direct(value))
            }
            ValueType::ScalarPair { fields, .. } => {
                let values = fields.map(|field| {
                    builder
                        .ins()
                        .load(field.ty, flags, address, field.offset as i32)
                });

                Ok(Value::ScalarPair(values))
            }
            ValueType::Indirect { .. } => {
                let destination = self.allocate(value_type, builder);
                self.copy(destination, address, value_type, builder);

                Ok(Value::Address(destination))
            }
        }
    }

    /// Store one physical value into canonical memory.
    pub(super) fn store(
        &self,
        destination: cir::Value,
        value: Value,
        value_type: ValueType,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let flags = self.memory_flags(AliasRegion::World);
        match value_type {
            ValueType::Direct { .. } => {
                let value = value
                    .direct()
                    .ok_or_else(|| self.invalid("native direct value is stored indirectly"))?;
                builder.ins().store(flags, value, destination, 0);
            }
            ValueType::ScalarPair { fields, .. } => {
                let values = value
                    .scalar_pair()
                    .ok_or_else(|| self.invalid("native scalar-pair value is stored indirectly"))?;
                for (field, value) in fields.into_iter().zip(values) {
                    builder
                        .ins()
                        .store(flags, value, destination, field.offset as i32);
                }
            }
            ValueType::Indirect { .. } => {
                let source = value
                    .address()
                    .ok_or_else(|| self.invalid("native indirect value is stored directly"))?;
                self.copy(destination, source, value_type, builder);
            }
        }

        Ok(())
    }

    /// Initialize one canonical byte range to zero.
    pub(super) fn zero(
        &self,
        address: cir::Value,
        byte_len: u32,
        builder: &mut FunctionBuilder<'_>,
    ) {
        self.zero_range(address, 0, byte_len, builder);
    }

    /// Initialize one canonical subrange to zero.
    pub(super) fn zero_range(
        &self,
        address: cir::Value,
        byte_offset: u32,
        byte_len: u32,
        builder: &mut FunctionBuilder<'_>,
    ) {
        let flags = self.memory_flags(AliasRegion::World);
        let word_count = byte_len / 8;

        // clear complete machine words
        if word_count != 0 {
            let zero = builder.ins().iconst(cir::types::I64, 0);
            for index in 0..word_count {
                let offset = byte_offset + index * 8;
                builder.ins().store(flags, zero, address, offset as i32);
            }
        }

        // clear the fixed tail
        let mut offset = byte_offset + word_count * 8;
        let end = byte_offset + byte_len;
        for (ty, width) in [
            (cir::types::I32, 4),
            (cir::types::I16, 2),
            (cir::types::I8, 1),
        ] {
            if end - offset >= width {
                let zero = builder.ins().iconst(ty, 0);
                builder.ins().store(flags, zero, address, offset as i32);
                offset += width;
            }
        }
    }

    /// Store one value in canonical Program word storage.
    pub(super) fn store_words(
        &self,
        destination: cir::Value,
        value: Value,
        ty: mir::TypeId,
        value_type: ValueType,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        // read the physical layout of the stored value
        let layout = self
            .optimized
            .layouts
            .type_layout(ty)
            .ok_or_else(|| self.invalid("native Word value has no physical layout"))?;
        if let (mir::Representation::Scalar(scalar), Some(value)) =
            (layout.representation, value.direct())
            && scalar.primitive.bit_width() <= u16::from(program::Word::BIT_LEN)
        {
            let value = self.canonical_word(value, ty, scalar, builder)?;
            let flags = self.memory_flags(AliasRegion::World);
            builder.ins().store(flags, value, destination, 0);

            return Ok(());
        }

        // clear the padding the canonical value representation leaves behind
        let zero = builder.ins().iconst(cir::types::I64, 0);
        let flags = self.memory_flags(AliasRegion::World);
        for index in 0..value_type.word_count() {
            builder
                .ins()
                .store(flags, zero, destination, (index * 8) as i32);
        }

        self.store(destination, value, value_type, builder)
    }

    /// Convert one native scalar into its exact Program word bits.
    fn canonical_word(
        &self,
        value: cir::Value,
        ty: mir::TypeId,
        scalar: mir::Scalar,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        // read the source scalar type
        let source = builder.func.dfg.value_type(value);
        let width = scalar.primitive.bit_width();
        let integer = cir::Type::int(width)
            .ok_or_else(|| self.invalid("native Word scalar has an invalid width"))?;
        let value = if source == integer {
            value
        } else {
            builder
                .ins()
                .bitcast(integer, cir::MemFlagsData::new(), value)
        };
        if integer == cir::types::I64 {
            return Ok(value);
        }

        let ty = self.optimized.tree.storage_type(ty);
        let definition = self.optimized.tree.type_definition(ty);
        let is_signed = definition
            .integer(self.types.layout.pointer_bits())
            .is_some_and(|(_, is_signed)| is_signed);
        let value = if is_signed {
            builder.ins().sextend(cir::types::I64, value)
        } else {
            builder.ins().uextend(cir::types::I64, value)
        };

        Ok(value)
    }

    /// Copy one canonical value between nonoverlapping addresses.
    fn copy(
        &self,
        destination: cir::Value,
        source: cir::Value,
        value_type: ValueType,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) {
        let byte_len = value_type.byte_len();
        let word_count = byte_len / 8;
        let alignment = value_type.alignment().min(align_of::<u64>() as u32) as u8;
        let mut flags = self.memory_flags(AliasRegion::World);
        if alignment >= 8 {
            flags.set_aligned();
        }

        // unroll ordinary fixed-size values
        if word_count <= INLINE_WORD_COUNT {
            for index in 0..word_count {
                let offset = (index * 8) as i32;
                let word = builder.ins().load(cir::types::I64, flags, source, offset);
                builder.ins().store(flags, word, destination, offset);
            }
        }
        // loop over unusually large values without importing a compiler runtime
        else {
            let copy = builder.create_block();
            let complete = builder.create_block();
            let pointer = self.types.pointer();
            let target = builder.append_block_param(copy, pointer);
            let source_cursor = builder.append_block_param(copy, pointer);
            let remaining = builder.append_block_param(copy, pointer);
            let remaining_words = builder.ins().iconst(pointer, i64::from(word_count));
            builder.ins().jump(
                copy,
                &[destination.into(), source.into(), remaining_words.into()],
            );
            builder.switch_to_block(copy);
            let word = builder.ins().load(cir::types::I64, flags, source_cursor, 0);
            builder.ins().store(flags, word, target, 0);
            let target = builder.ins().iadd_imm_u(target, 8);
            let source_cursor = builder.ins().iadd_imm_u(source_cursor, 8);
            let remaining = builder.ins().iadd_imm_s(remaining, -1);
            builder.ins().brif(
                remaining,
                copy,
                &[target.into(), source_cursor.into(), remaining.into()],
                complete,
                &[],
            );
            builder.seal_block(copy);
            builder.switch_to_block(complete);
            builder.seal_block(complete);
        }

        // copy the fixed tail after the complete words
        let mut offset = word_count * 8;
        for (ty, width) in [
            (cir::types::I32, 4),
            (cir::types::I16, 2),
            (cir::types::I8, 1),
        ] {
            if byte_len - offset >= width {
                let byte_offset = offset as i32;
                let value = builder.ins().load(ty, flags, source, byte_offset);
                builder.ins().store(flags, value, destination, byte_offset);
                offset += width;
            }
        }
    }

    /// Return the address word of one reference-like value: a world offset or a native pointer.
    pub(super) fn reference(&self, value: mir::Value) -> Result<cir::Value, EmitError> {
        let descriptor = self.descriptor(self.value_type(value)?)?;
        let selected = self.split_descriptor(self.value(value)?, descriptor)?;

        Ok(selected.address)
    }

    /// Allocate canonical stack storage.
    pub(super) fn allocate(
        &self,
        value_type: ValueType,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> cir::Value {
        self.allocate_bytes(value_type.byte_len(), value_type.alignment(), builder)
    }

    /// Allocate stack storage with one exact physical layout.
    pub(super) fn allocate_bytes(
        &self,
        byte_len: u32,
        alignment: u32,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> cir::Value {
        let slot = builder.create_sized_stack_slot(cir::StackSlotData::new(
            cir::StackSlotKind::ExplicitSlot,
            byte_len,
            alignment.trailing_zeros() as u8,
        ));

        builder.ins().stack_addr(self.types.pointer(), slot, 0)
    }

    /// Return one canonical address, materializing direct values when needed.
    pub(super) fn materialize(
        &self,
        value: Value,
        value_type: ValueType,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        if let Value::Address(address) = value {
            return Ok(address);
        }

        let address = self.allocate(value_type, builder);
        self.store(address, value, value_type, builder)?;

        Ok(address)
    }

    /// Retain one canonical value in a keyed native stack slot.
    pub(super) fn retain(
        &mut self,
        value: Value,
        value_type: ValueType,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(cir::StackSlot, cir::StackSlotKey), EmitError> {
        let key = self.next_key();
        let slot = builder.create_sized_stack_slot(cir::StackSlotData::new_with_key(
            cir::StackSlotKind::ExplicitSlot,
            value_type.byte_len(),
            value_type.alignment().trailing_zeros() as u8,
            key,
        ));
        let address = builder.ins().stack_addr(self.types.pointer(), slot, 0);
        self.store(address, value, value_type, builder)?;

        Ok((slot, key))
    }

    /// Allocate one function-qualified native stack-slot key.
    pub(super) fn next_key(&mut self) -> cir::StackSlotKey {
        let bits = u64::from(self.function_index) << 32 | u64::from(self.next_key);
        self.next_key += 1;

        cir::StackSlotKey::new(bits)
    }

    /// Return one value's MIR type.
    pub(super) fn value_type(&self, value: mir::Value) -> Result<mir::TypeId, EmitError> {
        self.function
            .value_type(value)
            .ok_or_else(|| self.invalid("native value has no MIR type"))
    }

    /// Return whether one MIR value has a signed integer representation.
    pub(super) fn is_signed_integer(&self, value: mir::Value) -> Result<bool, EmitError> {
        let ty = self.value_type(value)?;

        self.types.is_signed_integer(ty)
    }

    /// Return one emitted value.
    pub(super) fn value(&self, value: mir::Value) -> Result<Value, EmitError> {
        self.values
            .get(value.id() as usize)
            .copied()
            .flatten()
            .ok_or_else(|| self.invalid("native value has not been defined"))
    }

    /// Return one emitted scalar.
    pub(super) fn scalar(&self, value: mir::Value) -> Result<cir::Value, EmitError> {
        self.value(value)?
            .direct()
            .ok_or_else(|| self.invalid("native value is not scalar"))
    }

    /// Define one emitted MIR value.
    pub(super) fn set(&mut self, destination: mir::Value, value: Value) -> Result<(), EmitError> {
        let index = destination.id() as usize;
        if index >= self.values.len() {
            return Err(self.invalid("native destination has no physical value"));
        }
        self.values[index] = Some(value);

        Ok(())
    }

    /// Return the hidden activation.
    pub(super) fn activation(&self) -> Result<cir::Value, EmitError> {
        self.activation
            .ok_or_else(|| self.invalid("native activation is unavailable"))
    }
}
