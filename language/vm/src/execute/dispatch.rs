use destack_bytecode::{CodeOffset, Instruction, Opcode};
use destack_program::{Continuation, Outcome, Word};

use crate::diagnostic::{Error, Result, Trap};
use crate::machine::Activation;

impl Activation<'_, '_> {
    /// Dispatch instructions until the entry frame returns.
    pub(crate) fn dispatch<
        const STOP: bool,
        const WATCH: bool,
        const PROFILE: bool,
        const BOUNDED: bool,
    >(
        &mut self,
    ) -> Result<Outcome<Continuation, Vec<Word>>> {
        let program = self.machine.program.clone();
        let code_bytes = program.bytecode().bytes(program.sections());

        loop {
            // retain the exact location before instruction handlers advance
            let Some(frame) = self.machine.frames.last() else {
                unreachable!("bytecode dispatch requires an active frame");
            };
            let function = frame.function;
            let code = frame.code;
            let instruction_offset = frame.code_offset;
            self.instruction_offset = instruction_offset;

            // enforce the configured instruction budget outside opcode handlers
            if BOUNDED {
                let Some(limit) = self.machine.limits.max_instructions else {
                    unreachable!("bounded dispatch requires an instruction limit");
                };
                if self.instruction_count >= limit {
                    return Err(Error::instruction_limit_exceeded());
                }
                self.instruction_count += 1;
            }

            // decode the active instruction and advance before transfers
            let function_bytes = code.slice(code_bytes);
            let instruction_bytes = function_bytes
                .get(instruction_offset.index()..)
                .ok_or_else(|| Error::invalid_instruction(function, instruction_offset))?;
            let instruction = Instruction::read(instruction_bytes)
                .map_err(|_| Error::invalid_instruction(function, instruction_offset))?;

            // stop before externally configured instruction points
            let stopped = if STOP {
                self.stop_before(self.frame(), instruction_offset)?
            } else {
                None
            };
            if let Some(outcome) = stopped {
                return Ok(outcome);
            }
            self.frame_mut().advance(instruction.byte_len());

            // execute one decoded instruction
            let outcome =
                self.execute_instruction::<WATCH, PROFILE>(instruction_offset, instruction)?;
            if let Some(outcome) = outcome {
                return Ok(outcome);
            }
        }
    }

    /// Execute one instruction selected from an encoded opcode family.
    fn execute_instruction<const WATCH: bool, const PROFILE: bool>(
        &mut self,
        instruction_offset: CodeOffset,
        instruction: Instruction<'_>,
    ) -> Result<Option<Outcome<Continuation, Vec<Word>>>> {
        let opcode = instruction.opcode();

        // directly named opcodes
        if !opcode.is_parameterized() {
            self.execute_opcode::<WATCH, PROFILE>(instruction_offset, instruction)
        }
        // integer operations
        else if let Some((operation, scalar)) = opcode.integer_operation() {
            self.execute_integer(instruction, operation, scalar)?;

            Ok(None)
        }
        // 128-bit integer operations
        else if let Some((operation, is_signed)) = opcode.integer128_operation() {
            self.execute_integer128(instruction, operation, is_signed)?;

            Ok(None)
        }
        // floating point operations
        else if let Some((operation, scalar)) = opcode.float_operation() {
            self.execute_float(instruction, operation, scalar)?;

            Ok(None)
        }
        // scalar memory families
        else if let Some((operation, scalar)) = opcode.memory_operation() {
            self.execute_memory::<WATCH>(
                self.frame(),
                instruction_offset,
                instruction,
                operation,
                scalar,
            )
        }
        // scalar constants
        else if opcode.constant_scalar().is_some() {
            self.execute_constant(instruction)?;

            Ok(None)
        }
        // boolean operations
        else if opcode.boolean_operation().is_some() {
            self.execute_boolean(instruction)?;

            Ok(None)
        }
        // scalar casts
        else if let Some((operation, source, target)) = opcode.cast_operation() {
            self.execute_cast(instruction, operation, source, target)?;

            Ok(None)
        }
        // checked scalar transfers
        else if let Some((check, scalar)) = opcode.scalar_check() {
            self.execute_check(instruction, check, scalar)?;

            Ok(None)
        }
        // scalar comparisons
        else if let Some((comparison, scalar)) = opcode.comparison() {
            self.execute_comparison(instruction, comparison, scalar)?;

            Ok(None)
        }
        // atomic memory families
        else if let Some((operation, scalar)) = opcode.atomic_operation() {
            self.execute_atomic::<WATCH>(
                self.frame(),
                instruction_offset,
                instruction,
                operation,
                scalar,
            )
        }
        // heap allocation families
        else if let Some(operation) = opcode.new_operation() {
            self.execute_new::<PROFILE>(instruction, instruction_offset, operation)?;

            Ok(None)
        }
        // packed vector operations
        else if let Some(operation) = opcode.vector_operation() {
            self.execute_vector::<WATCH>(self.frame(), instruction_offset, instruction, operation)
        }
        // tensor operations
        else if let Some(operation) = opcode.tensor_operation() {
            self.execute_tensor::<WATCH>(self.frame(), instruction_offset, instruction, operation)
        }
        // unassigned opcode
        else {
            Err(Error::unsupported_opcode(opcode.code()))
        }
    }

    /// Execute one fixed opcode outside the encoded operation families.
    fn execute_opcode<const WATCH: bool, const PROFILE: bool>(
        &mut self,
        instruction_offset: CodeOffset,
        instruction: Instruction<'_>,
    ) -> Result<Option<Outcome<Continuation, Vec<Word>>>> {
        match instruction.opcode() {
            // aggregates
            Opcode::AGGREGATE
            | Opcode::FIELD_GET
            | Opcode::FIELD_SET
            | Opcode::ELEMENT_GET
            | Opcode::ELEMENT_SET
            | Opcode::VARIANT_NEW
            | Opcode::VARIANT_TAG
            | Opcode::VARIANT_PAYLOAD => {
                self.execute_aggregate(instruction)?;

                Ok(None)
            }
            // values
            Opcode::MOVE
            | Opcode::MOVE_RANGE
            | Opcode::SELECT
            | Opcode::SELECT_RANGE
            | Opcode::EQUAL
            | Opcode::CONSTANT_TYPE
            | Opcode::CONSTANT_BYTES
            | Opcode::CONSTANT_INT128
            | Opcode::CONSTANT_UINT128
            | Opcode::CONSTANT_NULL
            | Opcode::CONSTANT_UNDEFINED
            | Opcode::CONSTANT_UNINIT
            | Opcode::CONSTANT_ZEROED => {
                self.execute_value(instruction)?;

                Ok(None)
            }
            // initialization
            Opcode::NEW_COMPLETE => {
                self.execute_new_complete(instruction)?;

                Ok(None)
            }
            // addresses and pointers
            Opcode::GLOBAL_ADDRESS => {
                self.execute_global_address(instruction)?;

                Ok(None)
            }
            Opcode::FRAME_ADDRESS => {
                self.execute_frame_address(instruction)?;

                Ok(None)
            }
            Opcode::FRAME_LOAD | Opcode::FRAME_STORE => {
                self.execute_frame_memory(instruction)?;

                Ok(None)
            }
            Opcode::REFERENCE_POINTER => {
                self.execute_reference_pointer(instruction)?;

                Ok(None)
            }
            Opcode::POINTER_OFFSET | Opcode::POINTER_INDEX | Opcode::POINTER_DISTANCE => {
                self.execute_pointer(instruction)?;

                Ok(None)
            }
            Opcode::CAST_POINTER_TO_INT | Opcode::CAST_INT_TO_POINTER => {
                let Some((operation, source, target)) = instruction.opcode().cast_operation()
                else {
                    unreachable!("pointer cast opcodes carry one exact conversion");
                };
                self.execute_cast(instruction, operation, source, target)?;

                Ok(None)
            }
            // references
            Opcode::LOAD | Opcode::STORE => {
                self.execute_value_memory::<WATCH>(self.frame(), instruction_offset, instruction)
            }
            Opcode::FREE | Opcode::PIN | Opcode::UNPIN | Opcode::BARRIER => {
                self.execute_reference(instruction)?;

                Ok(None)
            }
            Opcode::DROP => {
                self.execute_drop(instruction, instruction_offset)?;

                Ok(None)
            }
            // byte ranges
            Opcode::COPY_BYTES
            | Opcode::MOVE_BYTES
            | Opcode::FILL_BYTES
            | Opcode::COMPARE_BYTES
            | Opcode::PREFETCH_READ
            | Opcode::PREFETCH_WRITE => {
                self.execute_byte_memory::<WATCH>(self.frame(), instruction_offset, instruction)
            }
            // atomics
            Opcode::ATOMIC_FENCE => {
                self.execute_atomic_fence(instruction)?;

                Ok(None)
            }
            // control flow
            Opcode::JUMP | Opcode::BRANCH => {
                self.execute_control(instruction)?;

                Ok(None)
            }
            Opcode::SWITCH => {
                self.execute_switch(instruction)?;

                Ok(None)
            }
            Opcode::CHECK_NULL | Opcode::CHECK_EXACT_TYPE | Opcode::CHECK_SUBTYPE => {
                self.execute_runtime_check(instruction)?;

                Ok(None)
            }
            // slices
            Opcode::SLICE_VIEW | Opcode::SLICE_LENGTH => {
                self.execute_slice(instruction)?;

                Ok(None)
            }
            // function values
            Opcode::FUNCTION_ADDRESS
            | Opcode::FUNCTION_BIND
            | Opcode::FUNCTION_ENVIRONMENT
            | Opcode::FUNCTION_ENVIRONMENT_CURRENT => {
                self.execute_function(instruction)?;

                Ok(None)
            }
            // dynamic values
            Opcode::DYNAMIC_BIND | Opcode::DYNAMIC_PAYLOAD | Opcode::DYNAMIC_TYPE => {
                self.execute_dynamic(instruction)?;

                Ok(None)
            }
            Opcode::CALL
            | Opcode::CALL_INDIRECT
            | Opcode::CALL_VIRTUAL
            | Opcode::CALL_DYNAMIC
            | Opcode::INVOKE
            | Opcode::INVOKE_INDIRECT
            | Opcode::INVOKE_VIRTUAL
            | Opcode::INVOKE_DYNAMIC
            | Opcode::TAIL_CALL
            | Opcode::TAIL_CALL_INDIRECT
            | Opcode::TAIL_CALL_VIRTUAL
            | Opcode::TAIL_CALL_DYNAMIC => {
                self.execute_call(instruction, instruction_offset)?;

                Ok(None)
            }
            Opcode::RETURN => {
                let outcome = self
                    .execute_return(instruction)?
                    .map(|value| Outcome::Completed { value });

                Ok(outcome)
            }
            Opcode::YIELD => self
                .execute_yield(instruction, instruction_offset)
                .map(Some),
            Opcode::PANIC | Opcode::PANIC_VALUE => {
                self.execute_panic(instruction)?;

                Ok(None)
            }
            Opcode::UNWIND_RESUME => {
                self.execute_unwind_resume()?;

                Ok(None)
            }
            // observation
            Opcode::PROFILE_INCREMENT | Opcode::PROFILE_SAMPLE => {
                if PROFILE {
                    self.execute_profile(instruction)?;
                }

                Ok(None)
            }
            Opcode::BREAKPOINT => self.stop_after(instruction_offset).map(Some),
            // traps
            Opcode::UNREACHABLE => Err(Error::trap(Trap::Unreachable)),
            Opcode::TRAP => {
                self.execute_trap(instruction)?;

                Ok(None)
            }
            // unsupported
            opcode => Err(Error::unsupported_opcode(opcode.code())),
        }
    }
}
