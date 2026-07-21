use crate::{
    CounterId, Error, Label, Opcode, ReferenceKind, RegisterId, RegisterRange, Result, SamplerId,
    Scalar, Space, Symbol, ValueType, VectorType,
};

/// Encoded operands for one instruction under construction.
#[derive(Debug)]
pub struct InstructionBuilder {
    /// The exact opcode.
    pub(crate) opcode: Opcode,
    /// Encoded operands in schema order.
    pub(crate) bytes: Vec<u8>,
    /// Symbol operands awaiting object linking.
    pub(crate) symbols: Vec<SymbolUse>,
    /// Branch operands awaiting local label resolution.
    pub(crate) branches: Vec<(usize, Label)>,
    /// Individual register words read by the instruction.
    pub(crate) registers: Vec<RegisterId>,
    /// Logical register ranges read by the instruction.
    pub(crate) ranges: Vec<RegisterRange>,
    /// Greatest profile counter index plus one.
    pub(crate) counter_count: u32,
    /// Greatest profile sampler index plus one.
    pub(crate) sampler_count: u32,
}

/// One symbolic operand awaiting object linking.
#[derive(Debug)]
pub(crate) struct SymbolUse {
    /// The operand byte inside the encoded operand stream.
    pub(crate) byte_offset: usize,
    /// The object-local symbol target.
    pub(crate) symbol: Symbol,
}

impl InstructionBuilder {
    /// Create one empty instruction builder.
    pub fn new(opcode: Opcode) -> Self {
        Self {
            opcode,
            bytes: Vec::new(),
            symbols: Vec::new(),
            branches: Vec::new(),
            registers: Vec::new(),
            ranges: Vec::new(),
            counter_count: 0,
            sampler_count: 0,
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

    /// Append one contiguous register range.
    pub fn range(&mut self, range: RegisterRange) {
        self.ranges.push(range);
        self.u16(range.start.0);
        self.u16(range.word_count);
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
    pub fn counter(&mut self, counter: CounterId) -> Result<()> {
        let count = counter
            .0
            .checked_add(1)
            .ok_or(Error::CounterOutOfRange(counter.0))?;

        self.counter_count = self.counter_count.max(count);
        self.u32(counter.0);

        Ok(())
    }

    /// Append one function-local profile sampler.
    pub fn sampler(&mut self, sampler: SamplerId) -> Result<()> {
        let count = sampler
            .0
            .checked_add(1)
            .ok_or(Error::SamplerOutOfRange(sampler.0))?;

        self.sampler_count = self.sampler_count.max(count);
        self.u32(sampler.0);

        Ok(())
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

    /// Append one object-local symbol operand.
    pub fn symbol(&mut self, symbol: Symbol) {
        let byte_offset = self.bytes.len();
        self.u32(symbol.index);
        self.symbols.push(SymbolUse {
            byte_offset,
            symbol,
        });
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

    /// Append one reference ownership and space operand.
    pub fn reference(&mut self, kind: ReferenceKind, space: Space) {
        self.bytes.push(kind.0);
        self.bytes.push(space.0);
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
