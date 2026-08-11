use destack_bytecode::{CodeOffset, Instruction, Opcode, ReferenceType, RegisterId, Space};
use destack_heap::{HeapEdge, HeapReference, SharedHeapReference};
use destack_mir as mir;
use destack_program::{FunctionId, Runtime, Word};

use crate::diagnostic::{Error, ExecutionResult, Result};
use crate::machine::Activation;

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Read one stable heap edge from a program storage space.
    pub(crate) fn read_edge(&self, register: RegisterId, space: mir::Space) -> Result<HeapEdge> {
        let bits = self.read(register.0).bits() as usize;

        match space {
            mir::Space::Local => Ok(HeapEdge::Local(HeapReference::from_bits(bits))),
            mir::Space::Shared => Ok(HeapEdge::Shared(SharedHeapReference::from_bits(bits))),
        }
    }

    /// Read one stable heap edge from a bytecode reference operand.
    pub(crate) fn read_reference_edge(
        &self,
        register: RegisterId,
        reference: ReferenceType,
    ) -> Result<HeapEdge> {
        let space = match reference.storage().heap_space() {
            Some(Space::LOCAL) => mir::Space::Local,
            Some(Space::SHARED) => mir::Space::Shared,
            _ => return Err(self.invalid_instruction()),
        };

        self.read_edge(register, space)
    }

    /// Execute one heap reference lifetime or collector operation.
    pub(crate) fn execute_reference(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let register = operands.register()?;
        let reference = operands.reference()?;
        let edge = self.read_reference_edge(register, reference)?;

        // execute one operation through engine-neutral program storage
        match instruction.opcode() {
            Opcode::FREE => self.activation.memory.free(edge).map_err(Error::heap)?,
            Opcode::PIN => {
                let edge = self.activation.memory.pin(edge).map_err(Error::heap)?;

                self.write(register.0, Word::from_bits(edge.bits() as u64));
            }
            Opcode::UNPIN => self.activation.memory.unpin(edge).map_err(Error::heap)?,
            Opcode::BARRIER => {
                let start = operands.register()?;
                let byte_len = operands.register()?;
                let start = self.read(start.0).as_u64() as usize;
                let byte_len = self.read(byte_len.0).as_u64() as usize;
                let trace_view = self.machine.program.trace_view();

                self.activation
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
        pc: CodeOffset,
        instruction: Instruction<'_>,
    ) -> ExecutionResult<(), R::Error> {
        let mut operands = self.operands(instruction);
        let value = operands.span()?;
        let function = FunctionId(operands.u32()?);
        let value = self.register_byte_range(value)?;
        let caller_state = self.machine.frame_state_at(self.frame(), pc)?;

        self.call_destructor(function, value.start, pc, caller_state, 0)
    }
}
