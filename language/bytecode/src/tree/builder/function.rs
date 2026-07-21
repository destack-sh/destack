use crate::{
    CodeOffset, Error, Instruction, InstructionBuilder, InstructionRelocation, Label, Opcode,
    Operand, RegisterId, RegisterRange, Result,
};

/// One bytecode function body under construction.
#[derive(Debug, Default)]
pub struct FunctionBuilder {
    /// The encoded instruction bytes.
    code: Vec<u8>,
    /// The byte offset of each logical operation in order.
    operation_offsets: Vec<CodeOffset>,
    /// The label byte offsets.
    labels: Vec<CodeOffset>,
    /// The branch operands awaiting label resolution.
    branches: Vec<Branch>,
    /// The symbol operands awaiting object linking.
    relocations: Vec<InstructionRelocation>,
    /// The greatest register index plus one.
    register_count: u16,
    /// The greatest profile counter index plus one.
    counter_count: u32,
    /// The greatest profile sampler index plus one.
    sampler_count: u32,
}

/// One complete function body built for a bytecode object.
#[derive(Debug)]
pub struct FunctionBody {
    /// The encoded instruction bytes.
    pub code: Vec<u8>,
    /// The byte offset of each logical operation in order.
    pub operation_offsets: Vec<CodeOffset>,
    /// The symbol operands awaiting object linking.
    pub relocations: Vec<InstructionRelocation>,
    /// The number of 64-bit words in the register file.
    pub register_count: u16,
    /// The number of function-local profile counters.
    pub counter_count: u32,
    /// The number of function-local profile samplers.
    pub sampler_count: u32,
}

/// One branch operand awaiting label resolution.
#[derive(Debug)]
struct Branch {
    /// The target label.
    label: Label,
    /// The branch operand byte inside the function code.
    byte_offset: usize,
    /// The end of the containing instruction.
    instruction_end: usize,
}

impl FunctionBuilder {
    /// Create one empty function builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Begin one logical operation at the current byte offset.
    pub fn begin_operation(&mut self) {
        self.operation_offsets
            .push(CodeOffset(self.code.len() as u32));
    }

    /// Define one dense branch label at the current byte offset.
    pub fn define(&mut self, label: Label) -> Result<()> {
        if label.index() != self.labels.len() {
            return Err(Error::InvalidLabel(label.0));
        }

        let offset = CodeOffset(self.code.len() as u32);
        self.labels.push(offset);

        Ok(())
    }

    /// Append one instruction with its destination register ranges.
    pub fn emit(
        &mut self,
        instruction: InstructionBuilder,
        definitions: &[RegisterRange],
    ) -> Result<CodeOffset> {
        let result_bytes = Self::encode_results(instruction.opcode, definitions)?;

        // include every physical register touched by the instruction
        for range in definitions.iter().chain(&instruction.ranges) {
            self.include_range(*range);
        }
        for register in &instruction.registers {
            self.include(*register);
        }
        self.counter_count = self.counter_count.max(instruction.counter_count);
        self.sampler_count = self.sampler_count.max(instruction.sampler_count);

        // assemble results before operation operands
        let mut operands = result_bytes;
        let result_byte_len = operands.len();
        operands.extend_from_slice(&instruction.bytes);
        let offset = Instruction::pack(instruction.opcode, &operands, &mut self.code)?;
        let instruction_end = self.code.len();
        let operand_base = instruction_end - operands.len();

        // retain symbolic operands for object linking
        for symbol in instruction.symbols {
            let byte_offset = (operand_base + result_byte_len + symbol.byte_offset) as u32;
            self.relocations
                .push(InstructionRelocation::new(byte_offset, symbol.symbol));
        }

        // retain branch labels for displacement resolution
        for (byte_offset, label) in instruction.branches {
            self.branches.push(Branch {
                label,
                byte_offset: operand_base + result_byte_len + byte_offset,
                instruction_end,
            });
        }

        Ok(offset)
    }

    /// Resolve all labels and finish this function body.
    pub fn build(mut self) -> Result<FunctionBody> {
        self.resolve_branches()?;

        Ok(FunctionBody {
            code: self.code,
            operation_offsets: self.operation_offsets,
            relocations: self.relocations,
            register_count: self.register_count,
            counter_count: self.counter_count,
            sampler_count: self.sampler_count,
        })
    }

    /// Return the current register file word count.
    pub const fn register_count(&self) -> u16 {
        self.register_count
    }

    /// Reserve one contiguous register range in the function register file.
    pub fn reserve(&mut self, range: RegisterRange) {
        self.include_range(range);
    }

    /// Encode destination operands from one opcode layout.
    fn encode_results(opcode: Opcode, definitions: &[RegisterRange]) -> Result<Vec<u8>> {
        let layout = opcode.layout().ok_or(Error::InvalidOpcode(opcode.code()))?;
        let result_count = layout
            .operands()
            .iter()
            .take_while(|operand| matches!(operand, Operand::Result | Operand::ResultRange))
            .count();
        let result_operands = &layout.operands()[..result_count];
        let mut bytes = Vec::new();
        let mut definition_index = 0;

        // encode each declared destination in layout order
        for (operand_index, operand) in result_operands.iter().enumerate() {
            let Some(definition) = definitions.get(definition_index) else {
                return Err(Error::InvalidResults);
            };
            if *operand == Operand::Result && definition.word_count != 1 {
                return Err(Error::InvalidResults);
            }

            bytes.extend_from_slice(&definition.start.0.to_le_bytes());
            if *operand == Operand::ResultRange {
                let is_final_operand = operand_index + 1 == result_operands.len();
                let word_count = if is_final_operand {
                    Self::packed_word_count(&definitions[definition_index..])?
                } else {
                    definition.word_count
                };
                bytes.extend_from_slice(&word_count.to_le_bytes());
                definition_index = if is_final_operand {
                    definitions.len()
                } else {
                    definition_index + 1
                };
            } else {
                definition_index += 1;
            }
        }

        if definition_index != definitions.len() {
            return Err(Error::InvalidResults);
        }

        Ok(bytes)
    }

    /// Return the physical width of contiguous logical results.
    fn packed_word_count(definitions: &[RegisterRange]) -> Result<u16> {
        let Some(first) = definitions.first() else {
            return Err(Error::InvalidResults);
        };
        let mut end = u32::from(first.start.0);

        // consume each logical result without gaps
        for definition in definitions {
            if u32::from(definition.start.0) != end {
                return Err(Error::NoncontiguousResults);
            }
            end += u32::from(definition.word_count);
        }

        u16::try_from(end - u32::from(first.start.0)).map_err(|_| Error::InvalidResults)
    }

    /// Include one register in this function's fixed register file.
    fn include(&mut self, register: RegisterId) {
        self.register_count = self.register_count.max(register.0 + 1);
    }

    /// Include every register in one contiguous range.
    fn include_range(&mut self, range: RegisterRange) {
        if range.word_count == 0 {
            return;
        }

        let last = RegisterId(range.start.0 + range.word_count - 1);
        self.include(last);
    }

    /// Resolve branch labels into signed end-relative displacements.
    fn resolve_branches(&mut self) -> Result<()> {
        for branch in &self.branches {
            let Some(target) = self.labels.get(branch.label.index()) else {
                return Err(Error::UnknownLabel(branch.label.0));
            };
            let displacement = target.index() as i64 - branch.instruction_end as i64;
            let displacement = i32::try_from(displacement)
                .map_err(|_| Error::FunctionTooLarge(self.code.len()))?;
            self.code[branch.byte_offset..branch.byte_offset + size_of::<i32>()]
                .copy_from_slice(&displacement.to_le_bytes());
        }

        Ok(())
    }
}
