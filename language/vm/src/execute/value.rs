use destack_bytecode::{ConstantId, Instruction, Opcode, Operands};
use destack_program::{TypeId, Word};

use crate::diagnostic::Result;
use crate::machine::Activation;

impl Activation<'_, '_> {
    /// Execute one exact scalar constant.
    pub(crate) fn execute_constant(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = instruction.operands();
        let target = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let bits = operands.u64().map_err(|_| self.invalid_instruction())?;
        let scalar = instruction
            .opcode()
            .constant_scalar()
            .ok_or_else(|| self.invalid_instruction())?;

        self.write(target.0, Word::from_bits(scalar.encode(bits)));

        Ok(())
    }

    /// Execute one base value operation.
    pub(crate) fn execute_value(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = instruction.operands();

        match instruction.opcode() {
            Opcode::MOVE => {
                let target = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let source = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;

                self.write(target.0, self.read(source.0));
            }
            Opcode::MOVE_RANGE => {
                let target = operands.range().map_err(|_| self.invalid_instruction())?;
                let source = operands.range().map_err(|_| self.invalid_instruction())?;

                self.move_range(source, target);
            }
            Opcode::SELECT => {
                let target = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let condition = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let yes = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let no = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let source = if self.read(condition.0).as_boolean() {
                    yes
                } else {
                    no
                };

                self.write(target.0, self.read(source.0));
            }
            Opcode::SELECT_RANGE => {
                let target = operands.range().map_err(|_| self.invalid_instruction())?;
                let condition = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let yes = operands.range().map_err(|_| self.invalid_instruction())?;
                let no = operands.range().map_err(|_| self.invalid_instruction())?;
                let source = if self.read(condition.0).as_boolean() {
                    yes
                } else {
                    no
                };

                self.move_range(source, target);
            }
            Opcode::EQUAL => {
                let target = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let left = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let right = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let is_equal = self.read(left.0) == self.read(right.0);

                self.write(target.0, Word::boolean(is_equal));
            }
            Opcode::CONSTANT_TYPE => {
                let target = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let ty = TypeId(operands.u32().map_err(|_| self.invalid_instruction())?);

                self.write(target.0, Word::uint32(ty.0));
            }
            Opcode::CONSTANT_BYTES => self.execute_constant_bytes(operands)?,
            Opcode::CONSTANT_INT128 | Opcode::CONSTANT_UINT128 => {
                let target = operands.range().map_err(|_| self.invalid_instruction())?;
                let bits = operands.u128().map_err(|_| self.invalid_instruction())?;
                if target.word_count != 2 {
                    return Err(self.invalid_instruction());
                }

                self.write(target.start.0, Word::from_bits(bits as u64));
                self.write(target.start.0 + 1, Word::from_bits((bits >> 64) as u64));
            }
            Opcode::CONSTANT_NULL | Opcode::CONSTANT_UNDEFINED => {
                let target = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let _ = operands
                    .value_type()
                    .map_err(|_| self.invalid_instruction())?;
                let bits = if instruction.opcode() == Opcode::CONSTANT_NULL {
                    0
                } else {
                    1
                };

                self.write(target.0, Word::from_bits(bits));
            }
            Opcode::CONSTANT_UNINIT | Opcode::CONSTANT_ZEROED => {
                let target = operands.range().map_err(|_| self.invalid_instruction())?;

                // uninitialized storage preserves prior bits, zeroed storage clears every word
                if instruction.opcode() == Opcode::CONSTANT_ZEROED {
                    for word in 0..target.word_count {
                        self.write(target.start.0 + word, Word::ZERO);
                    }
                }
            }
            _ => unreachable!("value dispatch selects one value opcode"),
        }

        Ok(())
    }

    /// Materialize one immutable byte sequence as an address and length.
    fn execute_constant_bytes(&mut self, mut operands: Operands<'_>) -> Result<()> {
        let target = operands.range().map_err(|_| self.invalid_instruction())?;
        let constant = ConstantId(operands.u32().map_err(|_| self.invalid_instruction())?);
        if target.word_count != 2 {
            return Err(self.invalid_instruction());
        }

        let program = &self.machine.program;
        let code = program.bytecode();
        let sections = program.sections();
        let constant = code
            .constants(sections)
            .get(constant.index())
            .ok_or_else(|| self.invalid_instruction())?;
        let bytes = constant.bytes(code.constant_bytes(sections));
        let address = bytes.as_ptr() as u64;
        let byte_len = bytes.len() as u64;

        self.write(target.start.0, Word::from_bits(address));
        self.write(target.start.0 + 1, Word::uint64(byte_len));

        Ok(())
    }
}
