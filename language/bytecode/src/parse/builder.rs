use crate::{
    CounterId, Label, Opcode, ReferenceKind, RegisterId, RegisterRange, SamplerId, Scalar, Space,
    Symbol, ValueType, VectorType,
};

/// Encoded operands for one instruction under construction.
#[derive(Debug)]
pub(super) struct InstructionBuilder {
    /// The exact opcode.
    pub(super) opcode: Opcode,
    /// Encoded operands in schema order.
    pub(super) bytes: Vec<u8>,
    /// Symbol operands awaiting object linking.
    pub(super) symbols: Vec<SymbolUse>,
    /// Branch operands awaiting local label resolution.
    pub(super) branches: Vec<(usize, Label)>,
    /// Individual register words read by the instruction.
    pub(super) registers: Vec<RegisterId>,
    /// Logical register ranges read by the instruction.
    pub(super) ranges: Vec<RegisterRange>,
}

/// One symbolic operand awaiting object linking.
#[derive(Debug)]
pub(super) struct SymbolUse {
    /// The operand byte inside the encoded operand stream.
    pub(super) byte_offset: usize,
    /// The object-local symbol target.
    pub(super) symbol: Symbol,
}

impl InstructionBuilder {
    /// Create one empty instruction builder.
    pub(super) fn new(opcode: Opcode) -> Self {
        Self {
            opcode,
            bytes: Vec::new(),
            symbols: Vec::new(),
            branches: Vec::new(),
            registers: Vec::new(),
            ranges: Vec::new(),
        }
    }

    /// Append one register operand.
    pub(super) fn register(&mut self, register: RegisterId) {
        self.registers.push(register);
        self.u16(register.0);
    }

    /// Append one counted register list.
    pub(super) fn registers(&mut self, registers: &[RegisterId]) {
        self.registers.extend_from_slice(registers);
        self.u16(registers.len() as u16);
        for register in registers {
            self.u16(register.0);
        }
    }

    /// Append one contiguous register range.
    pub(super) fn range(&mut self, range: RegisterRange) {
        self.ranges.push(range);
        self.u16(range.start.0);
        self.u16(range.word_count);
    }

    /// Append one unsigned 16-bit operand.
    pub(super) fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    /// Append one unsigned 32-bit operand.
    pub(super) fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    /// Append one function-local profile counter.
    pub(super) fn counter(&mut self, counter: CounterId) {
        self.u32(counter.0);
    }

    /// Append one function-local profile sampler.
    pub(super) fn sampler(&mut self, sampler: SamplerId) {
        self.u32(sampler.0);
    }

    /// Append one signed 32-bit operand.
    pub(super) fn i32(&mut self, value: i32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    /// Append one unsigned 64-bit operand.
    pub(super) fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    /// Append one unsigned 128-bit operand.
    pub(super) fn u128(&mut self, value: u128) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    /// Append one object-local symbol operand.
    pub(super) fn symbol(&mut self, symbol: Symbol) {
        let byte_offset = self.bytes.len();
        self.u32(symbol.index);
        self.symbols.push(SymbolUse {
            byte_offset,
            symbol,
        });
    }

    /// Append one unresolved branch operand.
    pub(super) fn branch(&mut self, label: Label) {
        let byte_offset = self.bytes.len();
        self.i32(0);
        self.branches.push((byte_offset, label));
    }

    /// Append one scalar representation operand.
    pub(super) fn scalar(&mut self, scalar: Scalar) {
        self.u16(scalar.code() as u16);
    }

    /// Append one vector type operand.
    pub(super) fn vector_type(&mut self, vector: VectorType) {
        self.bytes.push(vector.scalar.code());
        self.bytes.push(0);
        self.u16(vector.lane_count);
    }

    /// Append one reference ownership and space operand.
    pub(super) fn reference(&mut self, kind: ReferenceKind, space: Space) {
        self.bytes.push(kind.0);
        self.bytes.push(space.0);
    }

    /// Append one complete value type operand.
    pub(super) fn value_type(&mut self, ty: ValueType) {
        self.bytes.extend_from_slice(&ty.bytes());
    }
}
