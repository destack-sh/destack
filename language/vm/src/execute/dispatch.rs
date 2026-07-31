use destack_bytecode::{AtomicOperation, MemoryOperation, Opcode, VectorOperation};
use destack_program::{MemoryAccess, Outcome, Poll, Runtime, StopReason, Word};

use crate::diagnostic::{Error, ExecutionResult, Trap};
use crate::machine::Activation;

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Dispatch instructions until the entry frame returns.
    #[inline(never)]
    pub(crate) fn dispatch<
        const STOP: bool,
        const WATCH: bool,
        const PROFILE: bool,
        const BOUNDED: bool,
    >(
        &mut self,
    ) -> ExecutionResult<Outcome<Vec<Word>>, R::Error>
    where
        R::Error: From<Error>,
    {
        let mut position = self.cursor.position();

        loop {
            // retain the exact location before instruction handlers advance
            let operation_pc = position.pc();
            self.pc = operation_pc;

            // enforce the configured instruction budget outside opcode handlers
            if BOUNDED {
                let Some(limit) = self.machine.limits.max_instructions else {
                    unreachable!("bounded dispatch requires an instruction limit");
                };
                if self.instruction_count >= limit {
                    return Err(Error::instruction_limit_exceeded().into());
                }
                self.instruction_count += 1;
            }

            // decode the active instruction and advance before transfers
            // SAFETY: the activation borrows the Program for the complete dispatch
            let instruction = unsafe { position.decode() };

            // stop before externally configured instruction points
            let stopped = if STOP {
                self.cursor.set_position(position);

                self.stop_before(self.frame(), operation_pc)?
            } else {
                None
            };
            if let Some(outcome) = stopped {
                return Ok(outcome);
            }
            position.advance(instruction.byte_len());

            // expose the completed operation position to observed execution
            if WATCH {
                self.cursor.set_position(position);
            }

            // dispatch directly through the opcode's reserved high-byte range
            let opcode = instruction.opcode();
            match opcode.code() >> 8 {
                0x00 => match opcode {
                    // aggregates
                    Opcode::AGGREGATE
                    | Opcode::EXTRACT
                    | Opcode::INSERT
                    | Opcode::VARIANT_NEW
                    | Opcode::VARIANT_TAG => self.execute_aggregate(instruction)?,

                    // values
                    Opcode::MOVE
                    | Opcode::MOVE_RANGE
                    | Opcode::SELECT
                    | Opcode::SELECT_RANGE
                    | Opcode::EQUAL
                    | Opcode::CONSTANT_TYPE
                    | Opcode::CONSTANT_INT128
                    | Opcode::CONSTANT_UINT128
                    | Opcode::CONSTANT_NULL
                    | Opcode::CONSTANT_UNDEFINED
                    | Opcode::CONSTANT_ZEROED => self.execute_value(instruction)?,

                    // address construction
                    Opcode::GLOBAL_ADDRESS => self.execute_global_address(instruction)?,
                    Opcode::FRAME_ADDRESS => self.execute_frame_address(instruction)?,

                    // pointer materialization and arithmetic
                    Opcode::POINTER_FRAME
                    | Opcode::POINTER_GLOBAL
                    | Opcode::POINTER_LOCAL
                    | Opcode::POINTER_SHARED => self.execute_pointer(instruction)?,
                    Opcode::POINTER_ADD_IMMEDIATE
                    | Opcode::POINTER_ADD
                    | Opcode::POINTER_ADD_SCALED
                    | Opcode::POINTER_BYTE_OFFSET_FROM => {
                        self.execute_pointer_arithmetic(instruction)?
                    }
                    Opcode::CAST_POINTER_TO_INT | Opcode::CAST_INT_TO_POINTER => {
                        let Some((operation, source, target)) = opcode.cast_operation() else {
                            unreachable!("pointer cast opcodes carry one exact conversion");
                        };
                        self.execute_cast(instruction, operation, source, target)?;
                    }

                    // references
                    Opcode::LOAD | Opcode::STORE => {
                        let access = if opcode == Opcode::LOAD {
                            MemoryAccess::Read
                        } else {
                            MemoryAccess::Write
                        };
                        let needs_range = WATCH
                            && self
                                .watch_points
                                .is_some_and(|points| points.requires_memory_range());
                        let address = if needs_range {
                            Some(self.value_address(instruction, access == MemoryAccess::Read)?)
                        } else {
                            None
                        };
                        self.execute_value_memory(instruction)?;

                        if WATCH
                            && let Some(outcome) =
                                self.watch_after(self.frame(), operation_pc, access, address)?
                        {
                            return Ok(outcome);
                        }
                    }
                    Opcode::FREE | Opcode::PIN | Opcode::UNPIN | Opcode::BARRIER => {
                        self.execute_reference(instruction)?
                    }
                    Opcode::DROP => {
                        self.cursor.set_position(position);
                        self.execute_drop(operation_pc, instruction)?;
                        position = self.cursor.position();
                    }

                    // byte ranges
                    Opcode::COPY_BYTES
                    | Opcode::MOVE_BYTES
                    | Opcode::FILL_BYTES
                    | Opcode::COMPARE_BYTES
                    | Opcode::PREFETCH_READ
                    | Opcode::PREFETCH_WRITE => {
                        let accesses = if WATCH {
                            self.byte_accesses(instruction)?
                        } else {
                            [None, None]
                        };
                        self.execute_byte_memory(instruction)?;

                        if WATCH {
                            for (access, address) in accesses.into_iter().flatten() {
                                let outcome = self.watch_after(
                                    self.frame(),
                                    operation_pc,
                                    access,
                                    Some(address),
                                )?;
                                if let Some(outcome) = outcome {
                                    return Ok(outcome);
                                }
                            }
                        }
                    }

                    // atomics
                    Opcode::ATOMIC_FENCE => self.execute_atomic_fence(instruction)?,

                    // control flow
                    Opcode::JUMP | Opcode::BRANCH => {
                        let displacement = self.execute_control(instruction)?;
                        position.branch(displacement);
                    }
                    Opcode::SWITCH => {
                        let displacement = self.execute_switch(instruction)?;
                        position.branch(displacement);
                    }
                    Opcode::AWAIT | Opcode::YIELD => {
                        self.cursor.set_position(position);
                        let outcome = self.execute_suspension(operation_pc, instruction)?;
                        if let Some(outcome) = outcome {
                            return Ok(outcome);
                        }
                        position = self.cursor.position();
                    }
                    Opcode::CHECK_NULLISH | Opcode::CHECK_EXACT_TYPE | Opcode::CHECK_SUBTYPE => {
                        if let Some(displacement) = self.execute_runtime_check(instruction)? {
                            position.branch(displacement);
                        }
                    }

                    // slices and function values
                    Opcode::SLICE_VIEW => self.execute_slice(instruction)?,
                    Opcode::FUNCTION_ADDRESS | Opcode::FUNCTION_BIND => {
                        self.execute_function(instruction)?
                    }

                    // continuations, waiters, and tasks
                    Opcode::CONTINUATION_NEW
                    | Opcode::CONTINUATION_DESTROY
                    | Opcode::CONTINUATION_RESUME
                    | Opcode::CONTINUATION_COMPLETE => {
                        self.cursor.set_position(position);
                        self.execute_continuation(operation_pc, instruction)?;
                        position = self.cursor.position();
                    }
                    Opcode::WAITER_QUEUE | Opcode::WAITER_CANCEL => {
                        self.execute_waiter(instruction)?
                    }
                    Opcode::TASK_RESOLVE
                    | Opcode::TASK_PARK
                    | Opcode::TASK_CANCEL
                    | Opcode::TASK_DETACH => self.execute_task(operation_pc, instruction)?,
                    Opcode::TASK_START => {
                        self.cursor.set_position(position);
                        self.execute_task(operation_pc, instruction)?;
                        position = self.cursor.position();
                    }

                    // dynamic values and calls
                    Opcode::DYNAMIC_BIND | Opcode::DYNAMIC_TYPE => {
                        self.execute_dynamic(instruction)?
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
                        self.cursor.set_position(position);
                        if let Some(outcome) = self.execute_call(operation_pc, instruction)? {
                            return Ok(outcome);
                        }
                        position = self.cursor.position();
                    }
                    Opcode::RETURN => {
                        self.cursor.set_position(position);
                        let outcome = self.execute_return(instruction)?;
                        if let Some(outcome) = outcome {
                            return Ok(outcome);
                        }
                        position = self.cursor.position();
                    }
                    Opcode::PANIC | Opcode::PANIC_VALUE => {
                        self.cursor.set_position(position);
                        self.execute_panic(instruction)?;
                        position = self.cursor.position();
                    }
                    Opcode::UNWIND_RESUME => {
                        self.cursor.set_position(position);
                        self.execute_unwind_resume()?;
                        position = self.cursor.position();
                    }

                    // profiling and runtime cooperation
                    Opcode::PROFILE_INCREMENT | Opcode::PROFILE_SAMPLE => {
                        if PROFILE {
                            self.execute_profile(instruction)?;
                        }
                    }
                    Opcode::POLL => {
                        if self.activation.runtime.is_poll_requested() {
                            let frame = self.frame();
                            let state = self.machine.frame_state_at(frame, operation_pc)?;
                            self.cursor.set_position(position);
                            self.save_position();

                            match self.poll(state)? {
                                Poll::Continue | Poll::Deoptimize => {}
                                Poll::Pause => {
                                    let point = self.point(frame, operation_pc)?;
                                    self.machine.capture(state)?;

                                    return Ok(Outcome::Stopped {
                                        reason: StopReason::Pause { point },
                                    });
                                }
                            }
                        }
                    }

                    // stops
                    Opcode::BREAKPOINT => {
                        self.cursor.set_position(position);

                        return self
                            .stop_after(operation_pc, position.pc())
                            .map_err(Into::into);
                    }

                    // traps
                    Opcode::UNREACHABLE => return Err(Error::trap(Trap::Unreachable).into()),
                    Opcode::TRAP => self.execute_trap(instruction)?,

                    // unsupported
                    opcode => return Err(Error::unsupported_opcode(opcode.code()).into()),
                },
                0x01 => {
                    self.execute_constant(instruction)?;
                }
                0x02 => {
                    self.execute_boolean(instruction)?;
                }
                0x03 | 0x04 => {
                    let Some((operation, scalar)) = opcode.integer_operation() else {
                        unreachable!("linked integer opcodes carry one exact operation");
                    };
                    self.execute_integer(instruction, operation, scalar)?;
                }
                0x05 => {
                    let Some((operation, is_signed)) = opcode.integer128_operation() else {
                        unreachable!("linked wide integer opcodes carry one exact operation");
                    };
                    self.execute_integer128(instruction, operation, is_signed)?;
                }
                0x06 => {
                    let Some((operation, scalar)) = opcode.float_operation() else {
                        unreachable!("linked float opcodes carry one exact operation");
                    };
                    self.execute_float(instruction, operation, scalar)?;
                }
                0x07..=0x09 => {
                    let Some((operation, source, target)) = opcode.cast_operation() else {
                        unreachable!("linked cast opcodes carry one exact conversion");
                    };
                    self.execute_cast(instruction, operation, source, target)?;
                }
                0x0a => {
                    let Some((operation, scalar)) = opcode.memory_operation() else {
                        unreachable!("linked memory opcodes carry one exact operation");
                    };
                    let access = match operation {
                        MemoryOperation::Load => MemoryAccess::Read,
                        MemoryOperation::Store => MemoryAccess::Write,
                    };
                    let needs_range = WATCH
                        && self
                            .watch_points
                            .is_some_and(|points| points.requires_memory_range());
                    let address = if needs_range {
                        Some(self.memory_address(instruction, operation, scalar)?)
                    } else {
                        None
                    };
                    self.execute_memory(instruction, operation, scalar)?;

                    if WATCH
                        && let Some(outcome) =
                            self.watch_after(self.frame(), operation_pc, access, address)?
                    {
                        return Ok(outcome);
                    }
                }
                0x0b => {
                    let Some((operation, scalar)) = opcode.atomic_operation() else {
                        unreachable!("linked atomic opcodes carry one exact operation");
                    };
                    let access = match operation {
                        AtomicOperation::Load => MemoryAccess::Read,
                        AtomicOperation::Store => MemoryAccess::Write,
                        _ => MemoryAccess::ReadWrite,
                    };
                    let needs_range = WATCH
                        && self
                            .watch_points
                            .is_some_and(|points| points.requires_memory_range());
                    let address = if needs_range {
                        Some(self.atomic_address(instruction, operation, scalar)?)
                    } else {
                        None
                    };
                    self.execute_atomic(instruction, operation, scalar)?;

                    if WATCH
                        && let Some(outcome) =
                            self.watch_after(self.frame(), operation_pc, access, address)?
                    {
                        return Ok(outcome);
                    }
                }
                0x0c => match opcode.code() & 0x00ff {
                    0x00..=0x1f => {
                        let Some(operation) = opcode.new_operation() else {
                            unreachable!("linked new opcodes carry one exact operation");
                        };
                        self.cursor.set_position(position);
                        self.execute_new::<PROFILE>(instruction, operation)?;
                        position = self.cursor.position();
                    }
                    0x20..=0x9f => {
                        let Some((check, scalar)) = opcode.scalar_check() else {
                            unreachable!("linked check opcodes carry one exact operation");
                        };
                        if let Some(displacement) =
                            self.execute_check(instruction, check, scalar)?
                        {
                            position.branch(displacement);
                        }
                    }
                    0xa0..=0xff => {
                        let Some((comparison, scalar)) = opcode.comparison() else {
                            unreachable!("linked branch opcodes carry one exact comparison");
                        };
                        let displacement =
                            self.execute_comparison(instruction, comparison, scalar)?;
                        position.branch(displacement);
                    }
                    _ => unreachable!("masked operation codes fit one byte"),
                },
                0x0d => {
                    let Some(operation) = opcode.vector_operation() else {
                        unreachable!("linked vector opcodes carry one exact operation");
                    };
                    let access = match operation {
                        VectorOperation::Load => Some(MemoryAccess::Read),
                        VectorOperation::Store => Some(MemoryAccess::Write),
                        _ => None,
                    };
                    let needs_range = WATCH
                        && access.is_some()
                        && self
                            .watch_points
                            .is_some_and(|points| points.requires_memory_range());
                    let address = if needs_range {
                        Some(self.vector_address(instruction, operation)?)
                    } else {
                        None
                    };
                    self.execute_vector(instruction, operation)?;

                    if WATCH
                        && let Some(access) = access
                        && let Some(outcome) =
                            self.watch_after(self.frame(), operation_pc, access, address)?
                    {
                        return Ok(outcome);
                    }
                }
                0x0e | 0x0f => {
                    let Some(operation) = opcode.tensor_operation() else {
                        unreachable!("linked tensor opcodes carry one exact operation");
                    };
                    let access = self.execute_tensor(instruction, operation)?;
                    if WATCH
                        && let Some((access, address)) = access
                        && let Some(outcome) =
                            self.watch_after(self.frame(), operation_pc, access, Some(address))?
                    {
                        return Ok(outcome);
                    }
                }
                _ => return Err(Error::unsupported_opcode(opcode.code()).into()),
            }
        }
    }
}
