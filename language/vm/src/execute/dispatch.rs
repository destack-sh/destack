use tspp_bytecode::{AtomicOperation, MemoryOperation, Opcode, VectorOperation};
use tspp_program::{Event, EventKind, MemoryAccess, Outcome, Poll, Runtime, StopReason, Word};

use crate::diagnostic::{Error, ExecutionResult, Trap};
use crate::machine::Activation;

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Dispatch instructions until the entry frame returns.
    #[inline(never)]
    pub(crate) fn dispatch<
        const STOP: bool,
        const WATCH: bool,
        const OBSERVE: bool,
        const PROFILE: bool,
        const BOUNDED: bool,
    >(
        &mut self,
    ) -> ExecutionResult<Outcome<Vec<Word>>, R::Error>
    where
        R::Error: From<Error>,
    {
        let mut position = self.cursor.position();
        let is_observing_point = OBSERVE && self.events.contains(EventKind::Point);
        let is_observing_memory = OBSERVE && self.events.contains(EventKind::Memory);
        let is_observing_edge = OBSERVE && self.events.contains(EventKind::Edge);

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

            // publish the point immediately before executing its instruction
            if is_observing_point {
                let frame = self.frame();
                let point = self.point(frame, operation_pc)?;
                self.observe(Event::Point { point })?;
            }
            position.advance(instruction.byte_len());

            // expose the completed operation position to observed execution
            if WATCH || OBSERVE {
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
                    Opcode::VARIANT_TAG_LOAD | Opcode::VARIANT_TAG_LOAD_POINTER => {
                        let needs_range = is_observing_memory
                            || (WATCH
                                && self
                                    .watch_points
                                    .is_some_and(|points| points.requires_memory_range()));
                        let address = if needs_range {
                            Some(self.variant_tag_address(instruction)?)
                        } else {
                            None
                        };
                        self.execute_aggregate(instruction)?;

                        if is_observing_memory {
                            self.observe_memory(
                                self.frame(),
                                operation_pc,
                                0,
                                MemoryAccess::Read,
                                address,
                            )?;
                        }
                        if WATCH
                            && let Some(outcome) = self.watch_after(
                                self.frame(),
                                operation_pc,
                                0,
                                MemoryAccess::Read,
                                address,
                            )?
                        {
                            return Ok(outcome);
                        }
                    }

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

                    // reference and pointer arithmetic
                    Opcode::ADDRESS_ADD_IMMEDIATE
                    | Opcode::ADDRESS_ADD
                    | Opcode::ADDRESS_ADD_SCALED
                    | Opcode::ADDRESS_DIFF
                    | Opcode::ADDRESS_POINTER
                    | Opcode::ADDRESS_REFERENCE => self.execute_address_arithmetic(instruction)?,
                    Opcode::CAST_POINTER_TO_INT | Opcode::CAST_INT_TO_POINTER => {
                        let Some((operation, source, target)) = opcode.cast_operation() else {
                            unreachable!("pointer cast opcodes carry one exact conversion");
                        };
                        self.execute_cast(instruction, operation, source, target)?;
                    }

                    // references
                    Opcode::LOAD
                    | Opcode::LOAD_POINTER
                    | Opcode::STORE
                    | Opcode::STORE_POINTER
                    | Opcode::LOAD_VOLATILE
                    | Opcode::LOAD_VOLATILE_POINTER
                    | Opcode::STORE_VOLATILE
                    | Opcode::STORE_VOLATILE_POINTER => {
                        let is_load = matches!(
                            opcode,
                            Opcode::LOAD
                                | Opcode::LOAD_POINTER
                                | Opcode::LOAD_VOLATILE
                                | Opcode::LOAD_VOLATILE_POINTER
                        );
                        let access = if is_load {
                            MemoryAccess::Read
                        } else {
                            MemoryAccess::Write
                        };
                        let needs_range = is_observing_memory
                            || (WATCH
                                && self
                                    .watch_points
                                    .is_some_and(|points| points.requires_memory_range()));
                        let address = if needs_range {
                            Some(self.value_address(instruction, is_load)?)
                        } else {
                            None
                        };
                        self.execute_value_memory(instruction)?;

                        if is_observing_memory {
                            self.observe_memory(self.frame(), operation_pc, 0, access, address)?;
                        }
                        if WATCH
                            && let Some(outcome) =
                                self.watch_after(self.frame(), operation_pc, 0, access, address)?
                        {
                            return Ok(outcome);
                        }
                    }
                    Opcode::RELEASE => {
                        self.cursor.set_position(position);
                        self.execute_release(operation_pc, instruction)?;
                        position = self.cursor.position();
                    }
                    Opcode::FREE => self.execute_free(instruction)?,
                    Opcode::BARRIER => self.execute_reference(instruction)?,
                    Opcode::DROP => {
                        self.cursor.set_position(position);
                        self.execute_drop(operation_pc, instruction)?;
                        position = self.cursor.position();
                    }

                    // atomics
                    Opcode::ATOMIC_FENCE => self.execute_atomic_fence(instruction)?,

                    // control flow
                    Opcode::JUMP | Opcode::BRANCH => {
                        let displacement = self.execute_control(instruction)?;
                        position.branch(displacement);
                        if is_observing_edge {
                            self.observe_edge(self.frame(), operation_pc, position.pc())?;
                        }
                    }
                    Opcode::SWITCH => {
                        let displacement = self.execute_switch(instruction)?;
                        position.branch(displacement);
                        if is_observing_edge {
                            self.observe_edge(self.frame(), operation_pc, position.pc())?;
                        }
                    }
                    Opcode::CHECK_NULLISH | Opcode::CHECK_EXACT_TYPE | Opcode::CHECK_SUBTYPE => {
                        if let Some(displacement) = self.execute_runtime_check(instruction)? {
                            position.branch(displacement);
                        }
                        if is_observing_edge {
                            self.observe_edge(self.frame(), operation_pc, position.pc())?;
                        }
                    }

                    // function values
                    Opcode::FUNCTION_ADDRESS | Opcode::FUNCTION_BIND => {
                        self.execute_function(instruction)?
                    }

                    // execution contexts
                    Opcode::CONTEXT_CURRENT
                    | Opcode::CONTEXT_REPLACE
                    | Opcode::CONTEXT_BIND
                    | Opcode::CONTEXT_GET => {
                        self.execute_context::<OBSERVE, PROFILE>(instruction)?
                    }

                    // dynamic values and calls
                    Opcode::DYNAMIC_BIND | Opcode::DYNAMIC_READ | Opcode::DYNAMIC_TYPE => {
                        let memory = self.execute_dynamic(instruction)?;
                        if let Some((access, address)) = memory {
                            if is_observing_memory {
                                self.observe_memory(
                                    self.frame(),
                                    operation_pc,
                                    0,
                                    access,
                                    Some(address),
                                )?;
                            }
                            if WATCH
                                && let Some(outcome) = self.watch_after(
                                    self.frame(),
                                    operation_pc,
                                    0,
                                    access,
                                    Some(address),
                                )?
                            {
                                return Ok(outcome);
                            }
                        }
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
                            self.cursor.set_position(position);
                            self.save_position();

                            match self.poll()? {
                                Poll::Continue | Poll::Deoptimize => {}
                                Poll::Pause => {
                                    let frame = self.frame();
                                    let point = self.point(frame, operation_pc)?;
                                    self.fiber.context = *self.activation.context;

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
                    let Some((operation, address, scalar, is_volatile)) = opcode.memory_operation()
                    else {
                        unreachable!("linked memory opcodes carry one exact operation");
                    };
                    let access = match operation {
                        MemoryOperation::Load => MemoryAccess::Read,
                        MemoryOperation::Store => MemoryAccess::Write,
                    };
                    let needs_range = is_observing_memory
                        || (WATCH
                            && self
                                .watch_points
                                .is_some_and(|points| points.requires_memory_range()));
                    let memory_range = if needs_range {
                        Some(self.memory_address(instruction, operation, address, scalar)?)
                    } else {
                        None
                    };
                    self.execute_memory(instruction, operation, address, scalar, is_volatile)?;

                    if is_observing_memory {
                        self.observe_memory(self.frame(), operation_pc, 0, access, memory_range)?;
                    }
                    if WATCH
                        && let Some(outcome) =
                            self.watch_after(self.frame(), operation_pc, 0, access, memory_range)?
                    {
                        return Ok(outcome);
                    }
                }
                0x0b | 0x0c => {
                    let Some((operation, address, scalar)) = opcode.atomic_operation() else {
                        unreachable!("linked atomic opcodes carry one exact operation");
                    };
                    let access = match operation {
                        AtomicOperation::Load => MemoryAccess::Read,
                        AtomicOperation::Store => MemoryAccess::Write,
                        _ => MemoryAccess::ReadWrite,
                    };
                    let needs_range = is_observing_memory
                        || (WATCH
                            && self
                                .watch_points
                                .is_some_and(|points| points.requires_memory_range()));
                    let memory_range = if needs_range {
                        Some(self.atomic_address(instruction, operation, address, scalar)?)
                    } else {
                        None
                    };
                    self.execute_atomic(instruction, operation, address, scalar)?;

                    if is_observing_memory {
                        self.observe_memory(self.frame(), operation_pc, 0, access, memory_range)?;
                    }
                    if WATCH
                        && let Some(outcome) =
                            self.watch_after(self.frame(), operation_pc, 0, access, memory_range)?
                    {
                        return Ok(outcome);
                    }
                }
                0x0d => match opcode.code() & 0x00ff {
                    0x00..=0x1f => {
                        let Some(operation) = opcode.new_operation() else {
                            unreachable!("linked new opcodes carry one exact operation");
                        };
                        self.cursor.set_position(position);
                        self.execute_new::<OBSERVE, PROFILE>(instruction, operation)?;
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
                        if is_observing_edge {
                            self.observe_edge(self.frame(), operation_pc, position.pc())?;
                        }
                    }
                    0xa0..=0xff => {
                        let Some((comparison, scalar)) = opcode.comparison() else {
                            unreachable!("linked branch opcodes carry one exact comparison");
                        };
                        let displacement =
                            self.execute_comparison(instruction, comparison, scalar)?;
                        position.branch(displacement);
                        if is_observing_edge {
                            self.observe_edge(self.frame(), operation_pc, position.pc())?;
                        }
                    }
                    _ => unreachable!("masked operation codes fit one byte"),
                },
                0x0e => {
                    let Some(operation) = opcode.vector_operation() else {
                        unreachable!("linked vector opcodes carry one exact operation");
                    };
                    let access = match operation {
                        VectorOperation::Load => Some(MemoryAccess::Read),
                        VectorOperation::Store => Some(MemoryAccess::Write),
                        _ => None,
                    };
                    let needs_range = access.is_some()
                        && (is_observing_memory
                            || (WATCH
                                && self
                                    .watch_points
                                    .is_some_and(|points| points.requires_memory_range())));
                    let address = if needs_range {
                        Some(self.vector_address(instruction, operation)?)
                    } else {
                        None
                    };
                    self.execute_vector(instruction, operation)?;

                    if is_observing_memory && let Some(access) = access {
                        self.observe_memory(self.frame(), operation_pc, 0, access, address)?;
                    }
                    if WATCH
                        && let Some(access) = access
                        && let Some(outcome) =
                            self.watch_after(self.frame(), operation_pc, 0, access, address)?
                    {
                        return Ok(outcome);
                    }
                }
                0x0f => {
                    let accesses = if WATCH || is_observing_memory {
                        self.byte_accesses(instruction)?
                    } else {
                        [None, None]
                    };

                    // execute one exact byte range operation
                    if let Some((operation, target, source, is_immediate)) =
                        opcode.transfer_operation()
                    {
                        self.execute_transfer(
                            instruction,
                            operation,
                            target,
                            source,
                            is_immediate,
                        )?;
                    } else if let Some((target, is_immediate)) = opcode.fill_operation() {
                        self.execute_fill(instruction, target, is_immediate)?;
                    } else if let Some((left, right, is_immediate)) = opcode.compare_operation() {
                        self.execute_compare(instruction, left, right, is_immediate)?;
                    } else if let Some((operation, address)) = opcode.prefetch_operation() {
                        self.execute_prefetch(instruction, operation, address)?;
                    } else {
                        return Err(Error::unsupported_opcode(opcode.code()).into());
                    }

                    if WATCH || is_observing_memory {
                        let accesses = accesses
                            .into_iter()
                            .enumerate()
                            .filter_map(|(index, access)| access.map(|access| (index, access)));
                        for (site_index, (access, address)) in accesses {
                            if is_observing_memory {
                                self.observe_memory(
                                    self.frame(),
                                    operation_pc,
                                    site_index,
                                    access,
                                    Some(address),
                                )?;
                            }
                            if !WATCH {
                                continue;
                            }

                            let outcome = self.watch_after(
                                self.frame(),
                                operation_pc,
                                site_index,
                                access,
                                Some(address),
                            )?;
                            if let Some(outcome) = outcome {
                                return Ok(outcome);
                            }
                        }
                    }
                }
                _ => return Err(Error::unsupported_opcode(opcode.code()).into()),
            }
        }
    }
}
