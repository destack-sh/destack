use destack_bytecode::{CodeOffset, Instruction, Opcode, ReferenceType, RegisterId, Space};
use destack_heap::{HeapEdge, HeapReference, Release, SharedHeapReference};
use destack_mir as mir;
use destack_program::{FunctionId, Runtime, Word};

use crate::diagnostic::{Error, ExecutionResult, Result};
use crate::machine::{Activation, Released, Return};

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Read one stable heap edge from a program storage space.
    pub(crate) fn read_edge(&self, register: RegisterId, space: mir::Space) -> Result<HeapEdge> {
        let bits = self.read(register.0).bits() as usize;

        match space {
            mir::Space::Local => Ok(HeapEdge::Local(HeapReference::from_bits(bits))),
            mir::Space::Shared => Ok(HeapEdge::Shared(SharedHeapReference::from_bits(bits))),
            mir::Space::Constant | mir::Space::Parameter(_) | mir::Space::Join(_) => {
                Err(self.invalid_instruction())
            }
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

    /// Execute one release of a unique allocation.
    pub(crate) fn execute_release(
        &mut self,
        pc: CodeOffset,
        instruction: Instruction<'_>,
    ) -> ExecutionResult<(), R::Error> {
        let mut operands = self.operands(instruction);
        let owner = operands.register()?;
        let bits = self.read(owner.0).bits() as usize;
        let Some(edge) = self.activation.memory.edge(bits).map_err(Error::heap)? else {
            return Ok(());
        };
        let Release::Destroy { plan, byte_len } =
            self.activation.memory.release(edge).map_err(Error::heap)?
        else {
            return Ok(());
        };

        // select the placement-specific destructor
        let entry = self
            .machine
            .program
            .drop_entry(plan.drop)
            .ok_or_else(|| self.invalid_instruction())?;
        let function = match edge {
            HeapEdge::Local(_) => entry.local.get(),
            HeapEdge::Shared(_) => entry.shared.get(),
        }
        .ok_or_else(|| self.invalid_instruction())?;
        let remaining = plan.value_count(byte_len).map_err(Error::heap)?;
        let remaining = u32::try_from(remaining).map_err(|_| self.invalid_instruction())?;
        let stride = u32::try_from(plan.stride()).map_err(|_| self.invalid_instruction())?;
        let caller_state = self.machine.frame_state_at(self.frame(), pc)?;

        self.destroy_released(Released {
            pc,
            caller_state,
            frame_count: 0,
            owner: bits,
            function,
            stride,
            remaining,
        })
    }

    /// Execute one free of a unique allocation holding no live values.
    pub(crate) fn execute_free(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let owner = operands.register()?;
        let bits = self.read(owner.0).bits() as usize;
        let Some(edge) = self.activation.memory.edge(bits).map_err(Error::heap)? else {
            return Ok(());
        };

        self.activation.memory.free(edge).map_err(Error::heap)
    }

    /// Destroy the next value of one released allocation, freeing it after the last.
    pub(crate) fn destroy_released(&mut self, released: Released) -> ExecutionResult<(), R::Error> {
        // free the storage once every value is destroyed
        let Some(remaining) = released.remaining.checked_sub(1) else {
            let edge = self
                .activation
                .memory
                .edge(released.owner)
                .map_err(Error::heap)?
                .ok_or_else(|| self.invalid_instruction())?;
            self.activation.memory.free(edge).map_err(Error::heap)?;

            return Ok(());
        };

        // destroy the values from the last to the first
        let offset = remaining as usize * released.stride as usize;
        let reference = Word::from_bits((released.owner + offset) as u64);
        let return_to = Return::Release(Released {
            remaining,
            ..released
        });

        self.call_destructor(released.function, reference, return_to)
    }

    /// Execute one heap reference lifetime or collector operation.
    pub(crate) fn execute_reference(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let register = operands.register()?;
        let reference = operands.reference()?;
        let edge = self.read_reference_edge(register, reference)?;

        // execute one operation through engine-neutral program storage
        match instruction.opcode() {
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
        // call one statically linked frame destructor
        let mut operands = self.operands(instruction);
        let value = operands.span()?;
        let function = FunctionId(operands.u32()?);
        let value = self.register_byte_range(value)?;
        let reference = Word::from_bits(self.fiber.stack.memory_offset(value.start) as u64);
        let caller_state = self.machine.frame_state_at(self.frame(), pc)?;
        let return_to = Return::Drop {
            pc,
            caller_state,
            frame_count: 0,
        };

        self.call_destructor(function, reference, return_to)
    }
}
