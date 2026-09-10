use destack_bytecode::{CodeOffset, Instruction, Opcode, ReferenceType, RegisterId, Space};
use destack_heap::{DropCardinality, HeapEdge, HeapReference, SharedHeapReference};
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
            mir::Space::Constant
            | mir::Space::Parameter(_)
            | mir::Space::Slot(_)
            | mir::Space::Join(_) => Err(self.invalid_instruction()),
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

    /// Execute one unique allocation return.
    pub(crate) fn execute_free(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let owner = operands.register()?;
        let bits = self.read(owner.0).bits() as usize;
        let Some(edge) = self.activation.memory.edge(bits).map_err(Error::heap)? else {
            return Ok(());
        };

        self.activation.memory.release(edge).map_err(Error::heap)
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
        let mut operands = self.operands(instruction);
        let (function, reference) = match instruction.opcode() {
            // call one statically linked frame destructor
            Opcode::DROP => {
                let value = operands.span()?;
                let function = FunctionId(operands.u32()?);
                let value = self.register_byte_range(value)?;
                let reference = self.fiber.stack.memory_offset(value.start);

                (function, Word::from_bits(reference as u64))
            }
            // select one erased allocation destructor from its heap metadata
            Opcode::DROP_INDIRECT => {
                let owner = operands.register()?;
                let reference = self.read(owner.0);
                let Some(edge) = self
                    .activation
                    .memory
                    .edge(reference.bits() as usize)
                    .map_err(Error::heap)?
                else {
                    return Ok(());
                };
                let Some(plan) = self
                    .activation
                    .memory
                    .drop_plan(edge)
                    .map_err(Error::heap)?
                else {
                    return Ok(());
                };
                if plan.cardinality != DropCardinality::One {
                    return Err(self.invalid_instruction().into());
                }
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

                (function, reference)
            }
            _ => unreachable!("drop dispatch selects one drop opcode"),
        };
        let caller_state = self.machine.frame_state_at(self.frame(), pc)?;

        self.call_destructor(function, reference, pc, caller_state, 0)
    }
}
