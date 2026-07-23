use destack_bytecode::{CodeOffset, Instruction, Opcode, ReferenceType, RegisterId, Space};
use destack_heap::{HeapEdge, HeapReference, SharedHeapReference};
use destack_mir as mir;
use destack_program::{TypeId, Word};

use crate::diagnostic::{Error, Result};
use crate::machine::Activation;

impl Activation<'_, '_> {
    /// Read one stable heap edge from a program storage space.
    pub(crate) fn read_edge(&self, register: RegisterId, space: mir::Space) -> Result<HeapEdge> {
        let bits = self.read(register.0).bits() as usize;

        match space {
            mir::Space::Local => Ok(HeapEdge::Local(HeapReference::from_bits(bits))),
            mir::Space::Shared => Ok(HeapEdge::Shared(SharedHeapReference::from_bits(bits))),
            _ => Err(self.invalid_instruction()),
        }
    }

    /// Read one stable heap edge from a bytecode reference operand.
    pub(crate) fn read_reference_edge(
        &self,
        register: RegisterId,
        reference: ReferenceType,
    ) -> Result<HeapEdge> {
        let space = match reference.space() {
            Space::LOCAL => mir::Space::Local,
            Space::SHARED => mir::Space::Shared,
            _ => return Err(self.invalid_instruction()),
        };

        self.read_edge(register, space)
    }

    /// Execute one heap reference lifetime or collector operation.
    pub(crate) fn execute_reference(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = instruction.operands();
        let register = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let reference = operands
            .reference()
            .map_err(|_| self.invalid_instruction())?;
        let edge = self.read_reference_edge(register, reference)?;

        // execute one operation through engine-neutral program storage
        match instruction.opcode() {
            Opcode::FREE => self.call.memory.free(edge).map_err(Error::heap)?,
            Opcode::PIN => {
                let edge = self.call.memory.pin(edge).map_err(Error::heap)?;

                self.write(register.0, Word::from_bits(edge.bits() as u64));
            }
            Opcode::UNPIN => self.call.memory.unpin(edge).map_err(Error::heap)?,
            Opcode::BARRIER => {
                let start = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let byte_len = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let start = self.read(start.0).as_u64() as usize;
                let byte_len = self.read(byte_len.0).as_u64() as usize;
                let trace_view = self.machine.program.trace_view();

                self.call
                    .memory
                    .barrier(edge, start, byte_len, trace_view)
                    .map_err(Error::heap)?;
            }
            _ => unreachable!("reference dispatch selects one reference operation"),
        }

        Ok(())
    }

    /// Execute one explicit value destruction.
    pub(crate) fn execute_drop(
        &mut self,
        instruction: Instruction<'_>,
        instruction_offset: CodeOffset,
    ) -> Result<()> {
        let mut operands = instruction.operands();
        let value = operands.range().map_err(|_| self.invalid_instruction())?;
        let ty = TypeId(operands.u32().map_err(|_| self.invalid_instruction())?);
        let Some(function) = self
            .machine
            .program
            .destructor(ty)
            .map_err(Error::program)?
        else {
            return Ok(());
        };
        let layout = self
            .machine
            .program
            .layout(ty)
            .ok_or_else(|| self.invalid_instruction())?;
        let value = self.register_byte_range(value)?;
        if layout.byte_len() > value.len() {
            return Err(self.invalid_instruction());
        }

        self.call_destructor(function, value.start, instruction_offset)
    }
}
