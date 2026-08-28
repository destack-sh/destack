use destack_source::ProvenanceId;

use crate::{
    CodeOffset, Error, Instruction, InstructionBuilder, Label, Mapping, Opcode, Operand,
    RegisterId, RegisterSpan, Relocation, Result,
};

/// One bytecode function body under construction.
#[derive(Debug, Default)]
pub struct FunctionBuilder {
    /// The encoded instruction bytes.
    code: Vec<u8>,
    /// The byte offset and provenance of each logical operation.
    operations: Vec<(CodeOffset, ProvenanceId)>,
    /// Explicit physical mappings in byte order.
    mappings: Vec<Mapping>,
    /// The label byte offsets.
    labels: Vec<CodeOffset>,
    /// The branch operands awaiting label resolution.
    branches: Vec<Branch>,
    /// The identity operands awaiting object linking.
    relocations: Vec<Relocation>,
    /// The greatest register index plus one.
    register_count: u16,
}

/// One complete function body built for a bytecode object.
#[derive(Debug)]
pub struct FunctionBody {
    /// The encoded instruction bytes.
    pub code: Vec<u8>,
    /// The byte offset and provenance of each logical operation.
    pub operations: Vec<(CodeOffset, ProvenanceId)>,
    /// Explicit physical mappings.
    pub mappings: Vec<Mapping>,
    /// The identity operands awaiting object linking.
    pub relocations: Vec<Relocation>,
    /// The number of 64-bit words in the register file.
    pub register_count: u16,
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

    /// Define one dense branch label at the current byte offset.
    pub fn define(&mut self, label: Label) -> Result<()> {
        if label.index() != self.labels.len() {
            return Err(Error::InvalidLabel(label.0));
        }

        self.labels.push(CodeOffset(self.code.len() as u32));

        Ok(())
    }

    /// Append one instruction with its destination register spans.
    pub fn emit(
        &mut self,
        instruction: InstructionBuilder,
        definitions: &[RegisterSpan],
    ) -> Result<CodeOffset> {
        let result_bytes = Self::encode_results(instruction.opcode, definitions)?;

        // include every physical register touched by the instruction
        for span in definitions.iter().chain(&instruction.spans) {
            self.include_span(*span)?;
        }
        for register in &instruction.registers {
            self.include(*register)?;
        }
        // assemble results before operation operands
        let mut operands = result_bytes;
        let result_byte_len = operands.len();
        operands.extend_from_slice(&instruction.bytes);
        let offset = Instruction::pack(instruction.opcode, &operands, &mut self.code)?;
        let instruction_end = self.code.len();
        let operand_base = instruction_end - operands.len();

        // retain linked identities at their exact code offsets
        for relocation in instruction.relocations {
            let byte_offset = (operand_base + result_byte_len) as u32;
            self.relocations.push(relocation.rebase(byte_offset));
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

    /// Begin one logical operation at the current byte offset.
    pub fn begin_operation(&mut self, provenance: ProvenanceId) -> u32 {
        let offset = CodeOffset(self.code.len() as u32);

        self.record_operation(offset, provenance)
    }

    /// Return the mapping assigned to subsequent instructions.
    pub fn mapping(&self) -> Result<(u32, ProvenanceId)> {
        let operation = self
            .operations
            .len()
            .checked_sub(1)
            .ok_or(Error::MissingOperation)?;
        let (offset, provenance) = self.operations[operation];

        // prefer a later explicit mapping
        if let Some(mapping) = self.mappings.last()
            && mapping.offset >= offset
        {
            return Ok((mapping.operation, mapping.provenance));
        }

        Ok((operation as u32, provenance))
    }

    /// Map subsequent instructions to one logical operation and provenance.
    pub fn set_mapping(&mut self, operation: u32, provenance: ProvenanceId) -> Result<()> {
        if operation as usize >= self.operations.len() {
            return Err(Error::InvalidOperation(operation));
        }
        let offset = CodeOffset(self.code.len() as u32);

        self.record_mapping(offset, operation, provenance);

        Ok(())
    }

    /// Record one logical operation at an explicit byte offset.
    pub(crate) fn record_operation(&mut self, offset: CodeOffset, provenance: ProvenanceId) -> u32 {
        let operation = self.operations.len() as u32;
        self.operations.push((offset, provenance));

        // let the new logical operation supersede a change at the same offset
        if self
            .mappings
            .last()
            .is_some_and(|mapping| mapping.offset == offset)
        {
            self.mappings.pop();
        }

        operation
    }

    /// Record one explicit physical mapping.
    fn record_mapping(&mut self, offset: CodeOffset, operation: u32, provenance: ProvenanceId) {
        let mapping = Mapping::new(offset, operation, provenance);

        match self
            .mappings
            .binary_search_by_key(&offset, |mapping| mapping.offset)
        {
            Ok(index) => self.mappings[index] = mapping,
            Err(index) => self.mappings.insert(index, mapping),
        }
    }

    /// Return the current instruction byte offset.
    pub const fn code_offset(&self) -> CodeOffset {
        CodeOffset(self.code.len() as u32)
    }

    /// Anchor the current logical operation at the current byte offset.
    pub fn anchor_operation(&mut self) -> Result<()> {
        let operation = self
            .operations
            .len()
            .checked_sub(1)
            .ok_or(Error::MissingOperation)?;
        let (offset, provenance) = self.operations[operation];
        let anchor = CodeOffset(self.code.len() as u32);

        // map instructions emitted before the anchor
        if offset < anchor {
            self.record_mapping(offset, operation as u32, provenance);
        }
        self.operations[operation].0 = anchor;

        Ok(())
    }

    /// Resolve all labels and finish this function body.
    pub fn build(mut self) -> Result<FunctionBody> {
        self.resolve_branches()?;

        // require complete mapping for nonempty code
        let is_mapped = self
            .operations
            .first()
            .is_some_and(|(offset, _)| *offset == CodeOffset(0))
            || self
                .mappings
                .first()
                .is_some_and(|mapping| mapping.offset == CodeOffset(0));
        if !self.code.is_empty() && !is_mapped {
            return Err(Error::MissingOperation);
        }

        Ok(FunctionBody {
            code: self.code,
            operations: self.operations,
            mappings: self.mappings,
            relocations: self.relocations,
            register_count: self.register_count,
        })
    }

    /// Return the current register file word count.
    pub const fn register_count(&self) -> u16 {
        self.register_count
    }

    /// Reserve one contiguous register span in the function register file.
    pub fn reserve(&mut self, span: RegisterSpan) -> Result<()> {
        self.include_span(span)
    }

    /// Encode destination operands from one opcode layout.
    fn encode_results(opcode: Opcode, definitions: &[RegisterSpan]) -> Result<Vec<u8>> {
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
            let is_final_operand = operand_index + 1 == result_operands.len();
            if *operand == Operand::ResultRange
                && is_final_operand
                && definition_index == definitions.len()
            {
                bytes.extend_from_slice(&0u16.to_le_bytes());
                bytes.extend_from_slice(&0u16.to_le_bytes());

                continue;
            }
            let Some(definition) = definitions.get(definition_index) else {
                return Err(Error::InvalidResults);
            };
            if *operand == Operand::Result && definition.word_count != 1 {
                return Err(Error::InvalidResults);
            }

            bytes.extend_from_slice(&definition.start.0.to_le_bytes());
            if *operand == Operand::ResultRange {
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
    fn packed_word_count(definitions: &[RegisterSpan]) -> Result<u16> {
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
    fn include(&mut self, register: RegisterId) -> Result<()> {
        let register_count = u32::from(register.0) + 1;
        let register_count = u16::try_from(register_count)
            .map_err(|_| Error::RegisterFileTooLarge(register_count))?;
        self.register_count = self.register_count.max(register_count);

        Ok(())
    }

    /// Include every register in one contiguous span.
    fn include_span(&mut self, span: RegisterSpan) -> Result<()> {
        if span.word_count == 0 {
            return Ok(());
        }

        let end = u32::from(span.start.0) + u32::from(span.word_count);
        let register_count = u16::try_from(end).map_err(|_| Error::RegisterFileTooLarge(end))?;
        self.register_count = self.register_count.max(register_count);

        Ok(())
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
