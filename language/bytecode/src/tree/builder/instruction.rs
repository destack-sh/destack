use crate::{
    CounterId, Error, Label, Opcode, Placement, ReferenceKind, RegisterId, RegisterSpan,
    Relocation, RelocationTag, Result, SamplerId, Scalar, Storage, TensorOperand, ValueType,
    VectorType,
};

/// Encoded operands for one instruction under construction.
#[derive(Debug)]
pub struct InstructionBuilder {
    /// The exact opcode.
    pub(crate) opcode: Opcode,
    /// Encoded operands in schema order.
    pub(crate) bytes: Vec<u8>,
    /// Operands awaiting object linking.
    pub(crate) relocations: Vec<Relocation>,
    /// Branch operands awaiting local label resolution.
    pub(crate) branches: Vec<(usize, Label)>,
    /// Individual register words read by the instruction.
    pub(crate) registers: Vec<RegisterId>,
    /// Contiguous register spans read by the instruction.
    pub(crate) spans: Vec<RegisterSpan>,
}

impl InstructionBuilder {
    /// Create one empty instruction builder.
    pub fn new(opcode: Opcode) -> Self {
        Self {
            opcode,
            bytes: Vec::new(),
            relocations: Vec::new(),
            branches: Vec::new(),
            registers: Vec::new(),
            spans: Vec::new(),
        }
    }

    /// Append one register operand.
    pub fn register(&mut self, register: RegisterId) {
        self.registers.push(register);
        self.u16(register.0);
    }

    /// Append one counted register list.
    pub fn registers(&mut self, registers: &[RegisterId]) -> Result<()> {
        self.encode_count(registers.len())?;
        self.registers.extend_from_slice(registers);
        for register in registers {
            self.u16(register.0);
        }

        Ok(())
    }

    /// Append one counted list of physical span starts.
    pub fn span_starts(&mut self, spans: &[RegisterSpan]) -> Result<()> {
        self.encode_count(spans.len())?;
        for span in spans {
            self.spans.push(*span);
            self.u16(span.start.0);
        }

        Ok(())
    }

    /// Append one counted physical aggregate placement list.
    pub fn placements(&mut self, placements: &[Placement]) -> Result<()> {
        self.encode_count(placements.len())?;
        for placement in placements {
            self.span(placement.registers);
            self.u32(placement.byte_offset);
            self.u32(placement.byte_len);
        }

        Ok(())
    }

    /// Append one contiguous register span.
    pub fn span(&mut self, span: RegisterSpan) {
        self.spans.push(span);
        self.u16(span.start.0);
        self.u16(span.word_count);
    }

    /// Append one unsigned 16-bit operand.
    pub fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    /// Append one counted unsigned 16-bit list.
    pub fn u16s(&mut self, values: &[u16]) -> Result<()> {
        self.encode_count(values.len())?;
        for value in values {
            self.u16(*value);
        }

        Ok(())
    }

    /// Append one unsigned 32-bit operand.
    pub fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    /// Append one function-local profile counter.
    pub fn counter(&mut self, counter: CounterId) {
        self.relocation(RelocationTag::COUNTER, counter.0);
    }

    /// Append one function-local profile sampler.
    pub fn sampler(&mut self, sampler: SamplerId) {
        self.relocation(RelocationTag::SAMPLER, sampler.0);
    }

    /// Append one signed 32-bit operand.
    pub fn i32(&mut self, value: i32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    /// Append one unsigned 64-bit operand.
    pub fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    /// Append one counted unsigned 64-bit list.
    pub fn u64s(&mut self, values: &[u64]) -> Result<()> {
        self.encode_count(values.len())?;
        for value in values {
            self.u64(*value);
        }

        Ok(())
    }

    /// Append one unsigned 128-bit operand.
    pub fn u128(&mut self, value: u128) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    /// Append one relocatable object-local operand.
    pub fn relocation(&mut self, tag: RelocationTag, index: u32) {
        let byte_offset = self.bytes.len() as u32;
        self.u32(index);
        self.relocations.push(Relocation::new(byte_offset, tag));
    }

    /// Append one unresolved dynamic dispatch table operand.
    pub fn dynamic_table(&mut self, table: u32) {
        self.relocation(RelocationTag::DYNAMIC, table);
    }

    /// Append one unresolved branch operand.
    pub fn branch(&mut self, label: Label) {
        let byte_offset = self.bytes.len();
        self.i32(0);
        self.branches.push((byte_offset, label));
    }

    /// Append one scalar representation operand.
    pub fn scalar(&mut self, scalar: Scalar) {
        self.u16(scalar.code() as u16);
    }

    /// Append one vector type operand.
    pub fn vector_type(&mut self, vector: VectorType) {
        self.bytes.push(vector.scalar.code());
        self.bytes.push(0);
        self.u16(vector.lane_count);
    }

    /// Append one reference ownership and storage operand.
    pub fn reference(&mut self, kind: ReferenceKind, storage: Storage) {
        self.bytes.push(kind.code());
        self.bytes.push(storage.code());
    }

    /// Append one tensor operand.
    pub fn tensor(&mut self, tensor: TensorOperand) {
        self.span(tensor.registers);
        self.relocation(RelocationTag::LAYOUT, tensor.layout.0);
    }

    /// Append one counted tensor operand list.
    pub fn tensors(&mut self, tensors: &[TensorOperand]) -> Result<()> {
        self.encode_count(tensors.len())?;
        for tensor in tensors {
            self.tensor(*tensor);
        }

        Ok(())
    }

    /// Append one complete value type operand.
    pub fn value_type(&mut self, ty: ValueType) {
        self.bytes.extend_from_slice(&ty.bytes());
    }

    /// Append one encoded variable operand count.
    fn encode_count(&mut self, count: usize) -> Result<()> {
        let count = u16::try_from(count).map_err(|_| Error::TooManyOperands(count))?;
        self.u16(count);

        Ok(())
    }
}
