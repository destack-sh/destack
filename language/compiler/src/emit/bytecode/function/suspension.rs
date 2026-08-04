use destack_bytecode as bytecode;
use destack_mir as mir;

use crate::EmitError;

use super::FunctionEmitter;

impl<'a> FunctionEmitter<'a> {
    /// Emit one ready continuation construction.
    pub(super) fn emit_continuation_new(
        &mut self,
        destination: mir::Value,
        function: mir::FunctionId,
        arguments: mir::ValueSlice,
    ) -> Result<(), EmitError> {
        let function = self.types.function_id(function)?;
        let arguments = self.optimized.tree.get_values(arguments);
        let arguments = self.emit_arguments(arguments)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::CONTINUATION_NEW);
        instruction.relocation(bytecode::RelocationTag::FUNCTION, function.0);
        instruction.span(arguments);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit destruction of one continuation.
    pub(super) fn emit_continuation_destroy(
        &mut self,
        continuation: mir::Value,
    ) -> Result<(), EmitError> {
        let mut instruction =
            bytecode::InstructionBuilder::new(bytecode::Opcode::CONTINUATION_DESTROY);
        instruction.register(self.word(continuation)?);

        self.encode(instruction, &[])
    }

    /// Emit one asynchronous waiter settlement.
    pub(super) fn emit_waiter_queue(
        &mut self,
        destination: mir::Value,
        waiter: mir::Value,
        value: mir::Value,
    ) -> Result<(), EmitError> {
        let ty = self.value_type(value)?;
        let ty = self.types.type_id(ty)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::WAITER_QUEUE);
        instruction.register(self.word(waiter)?);
        instruction.relocation(bytecode::RelocationTag::TYPE, ty.0);
        instruction.span(self.register(value)?);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit one asynchronous waiter cancellation.
    pub(super) fn emit_waiter_cancel(
        &mut self,
        destination: mir::Value,
        waiter: mir::Value,
    ) -> Result<(), EmitError> {
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::WAITER_CANCEL);
        instruction.register(self.word(waiter)?);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit one already completed task.
    pub(super) fn emit_task_resolve(
        &mut self,
        destination: mir::Value,
        value: mir::Value,
    ) -> Result<(), EmitError> {
        let ty = self.value_type(value)?;
        let ty = self.types.type_id(ty)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::TASK_RESOLVE);
        instruction.relocation(bytecode::RelocationTag::TYPE, ty.0);
        instruction.span(self.register(value)?);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit one eager task start.
    pub(super) fn emit_task_start(
        &mut self,
        destination: mir::Value,
        continuation: mir::Value,
    ) -> Result<(), EmitError> {
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::TASK_START);
        instruction.register(self.word(continuation)?);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit parking one waiter on a task.
    pub(super) fn emit_task_park(
        &mut self,
        task: mir::Value,
        waiter: mir::Value,
    ) -> Result<(), EmitError> {
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::TASK_PARK);
        instruction.register(self.word(task)?);
        instruction.register(self.word(waiter)?);

        self.encode(instruction, &[])
    }

    /// Emit one scalar task operation.
    pub(super) fn emit_task(
        &mut self,
        opcode: bytecode::Opcode,
        task: mir::Value,
    ) -> Result<(), EmitError> {
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(self.word(task)?);

        self.encode(instruction, &[])
    }

    /// Emit one asynchronous suspension.
    pub(super) fn emit_await(
        &mut self,
        terminator: &mir::Terminator,
        park: mir::FunctionId,
        value: mir::Value,
        resume: &mir::BlockTarget,
        cancel: &mir::BlockTarget,
        unwind: Option<&mir::BlockTarget>,
    ) -> Result<(), EmitError> {
        let destinations = self.successor_destinations(terminator, resume)?;
        let park = self.types.function_id(park)?;
        let resume = self.edge_label(terminator, resume)?;
        let cancel = self.edge_label(terminator, cancel)?;
        let unwind = self.unwind_label(terminator, unwind)?;

        // encode the selected park implementation and consumed awaitable
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::AWAIT);
        instruction.relocation(bytecode::RelocationTag::FUNCTION, park.0);
        instruction.span(self.register(value)?);
        instruction.branch(resume);
        instruction.branch(cancel);
        instruction.branch(unwind);

        self.encode(instruction, &destinations)
    }

    /// Emit one generator suspension.
    pub(super) fn emit_yield(
        &mut self,
        terminator: &mir::Terminator,
        value: mir::Value,
        resume: &mir::BlockTarget,
        complete: &mir::BlockTarget,
        unwind: Option<&mir::BlockTarget>,
    ) -> Result<(), EmitError> {
        let mut destinations = self.successor_destinations(terminator, resume)?;
        destinations.extend(self.successor_destinations(terminator, complete)?);
        let resume = self.edge_label(terminator, resume)?;
        let complete = self.edge_label(terminator, complete)?;
        let unwind = self.unwind_label(terminator, unwind)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::YIELD);
        instruction.span(self.register(value)?);
        instruction.branch(resume);
        instruction.branch(complete);
        instruction.branch(unwind);

        self.encode(instruction, &destinations)
    }

    /// Emit one continuation control transfer.
    pub(super) fn emit_continuation_transfer(
        &mut self,
        terminator: &mir::Terminator,
        opcode: bytecode::Opcode,
        continuation: mir::Value,
        value: mir::Value,
        yielded: &mir::BlockTarget,
        returned: &mir::BlockTarget,
        unwind: Option<&mir::BlockTarget>,
    ) -> Result<(), EmitError> {
        let mut destinations = self.successor_destinations(terminator, yielded)?;
        destinations.extend(self.successor_destinations(terminator, returned)?);
        let yielded = self.edge_label(terminator, yielded)?;
        let returned = self.edge_label(terminator, returned)?;
        let unwind = self.unwind_label(terminator, unwind)?;

        // encode both completion paths and their direct destinations
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(self.word(continuation)?);
        instruction.span(self.register(value)?);
        instruction.branch(yielded);
        instruction.branch(returned);
        instruction.branch(unwind);

        self.encode(instruction, &destinations)
    }
}
