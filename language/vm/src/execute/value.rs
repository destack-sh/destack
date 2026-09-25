use tspp_bytecode::{Instruction, Opcode};
use tspp_program::{Runtime, TypeId, Word};

use crate::diagnostic::Result;
use crate::machine::Activation;

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Execute one exact scalar constant.
    #[inline(always)]
    pub(crate) fn execute_constant(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let bits = operands.u64()?;
        let scalar = instruction
            .opcode()
            .constant_scalar()
            .ok_or_else(|| self.invalid_instruction())?;

        self.write(target.0, Word::from_bits(scalar.encode(bits)));

        Ok(())
    }

    /// Execute one base value operation.
    #[inline(always)]
    pub(crate) fn execute_value(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);

        match instruction.opcode() {
            Opcode::MOVE => {
                let target = operands.register()?;
                let source = operands.register()?;

                self.write(target.0, self.read(source.0));
            }
            Opcode::MOVE_RANGE => {
                let target = operands.span()?;
                let source = operands.span()?;

                self.move_range(source, target);
            }
            Opcode::SELECT => {
                let target = operands.register()?;
                let condition = operands.register()?;
                let yes = operands.register()?;
                let no = operands.register()?;
                let source = if self.read(condition.0).as_boolean() {
                    yes
                } else {
                    no
                };

                self.write(target.0, self.read(source.0));
            }
            Opcode::SELECT_RANGE => {
                let target = operands.span()?;
                let condition = operands.register()?;
                let yes = operands.span()?;
                let no = operands.span()?;
                let source = if self.read(condition.0).as_boolean() {
                    yes
                } else {
                    no
                };

                self.move_range(source, target);
            }
            Opcode::EQUAL => {
                let target = operands.register()?;
                let left = operands.register()?;
                let right = operands.register()?;
                let is_equal = self.read(left.0) == self.read(right.0);

                self.write(target.0, Word::boolean(is_equal));
            }
            Opcode::CONSTANT_TYPE => {
                let target = operands.register()?;
                let ty = TypeId(operands.u32()?);

                self.write(target.0, Word::uint32(ty.0));
            }
            Opcode::CONSTANT_INT128 | Opcode::CONSTANT_UINT128 => {
                let target = operands.span()?;
                let bits = operands.u128()?;
                if target.word_count != 2 {
                    return Err(self.invalid_instruction());
                }

                self.write(target.start.0, Word::from_bits(bits as u64));
                self.write(target.start.0 + 1, Word::from_bits((bits >> 64) as u64));
            }
            Opcode::CONSTANT_NULL | Opcode::CONSTANT_UNDEFINED => {
                let target = operands.span()?;
                if target.word_count == 0 {
                    return Err(self.invalid_instruction());
                }
                let value = if instruction.opcode() == Opcode::CONSTANT_NULL {
                    Word::NULL
                } else {
                    Word::UNDEFINED
                };

                // zero every word before writing the nullish tag
                for word in 0..target.word_count {
                    self.write(target.start.0 + word, Word::ZERO);
                }
                self.write(target.start.0, value);
            }
            Opcode::CONSTANT_ZEROED => {
                let target = operands.span()?;

                // clear every physical result word
                for word in 0..target.word_count {
                    self.write(target.start.0 + word, Word::ZERO);
                }
            }
            _ => unreachable!("value dispatch selects one value opcode"),
        }

        Ok(())
    }
}
