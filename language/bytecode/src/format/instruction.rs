use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    BytecodeFormatter, CodeOffset, Instruction, Label, Opcode, Operands, RegisterId, RegisterSpan,
};

impl<'object> Instruction<'object> {
    /// Format this instruction at its function-local byte offset.
    pub(crate) fn format_at(
        self,
        offset: CodeOffset,
        formatter: &mut BytecodeFormatter<'object, '_>,
    ) -> FormatResult<()> {
        let body =
            format_with(|formatter| InstructionFormatter::new(self, offset, formatter).format());

        write!(formatter, [group(&indent(&body))])
    }
}

/// Formatter for one encoded instruction.
pub(super) struct InstructionFormatter<'code, 'state, 'buffer> {
    /// The encoded instruction.
    pub(super) instruction: Instruction<'code>,
    /// The instruction's function-local byte offset.
    pub(super) instruction_offset: CodeOffset,
    /// The unread encoded operands.
    pub(super) operands: Operands<'code>,
    /// The FIR formatter receiving this instruction.
    pub(super) formatter: &'state mut BytecodeFormatter<'code, 'buffer>,
}

impl<'code, 'state, 'buffer> InstructionFormatter<'code, 'state, 'buffer> {
    /// Create one instruction formatter.
    fn new(
        instruction: Instruction<'code>,
        instruction_offset: CodeOffset,
        formatter: &'state mut BytecodeFormatter<'code, 'buffer>,
    ) -> Self {
        Self {
            instruction,
            instruction_offset,
            operands: instruction.operands(),
            formatter,
        }
    }

    /// Format the complete instruction.
    fn format(mut self) -> FormatResult<()> {
        let opcode = self.instruction.opcode();
        self.format_opcode(opcode)?;

        // require every encoded operand to be represented
        if !self.operands.is_empty() {
            return Err(FormatError::SyntaxError {
                message: "formatter did not consume every instruction operand",
            });
        }

        Ok(())
    }

    /// Format one directly named or parameterized opcode.
    fn format_opcode(&mut self, opcode: Opcode) -> FormatResult<()> {
        // format every value conversion through one path
        if let Some((operation, source, target)) = opcode.cast_operation() {
            return self.format_cast(operation, source, target);
        }

        // format directly named opcodes
        if opcode.name().is_some() {
            return self.format_named_opcode(opcode);
        }

        // format parameterized opcode ranges in encoded order
        if let Some(scalar) = opcode.constant_scalar() {
            self.format_constant(scalar)
        } else if let Some(operation) = opcode.boolean_operation() {
            self.format_boolean(operation)
        } else if let Some((operation, scalar)) = opcode.integer_operation() {
            self.format_scalar(operation.name(), scalar)
        } else if let Some((operation, is_signed)) = opcode.integer128_operation() {
            self.format_integer128(operation, is_signed)
        } else if let Some((operation, scalar)) = opcode.float_operation() {
            self.format_scalar(operation.name(), scalar)
        } else if let Some((operation, scalar)) = opcode.memory_operation() {
            self.format_memory(operation, scalar)
        } else if let Some((operation, scalar)) = opcode.atomic_operation() {
            self.format_atomic(operation, scalar)
        } else if let Some(operation) = opcode.new_operation() {
            self.format_new(operation)
        } else if let Some((operation, scalar)) = opcode.scalar_check() {
            self.format_check(operation, scalar)
        } else if let Some((comparison, scalar)) = opcode.comparison() {
            self.format_branch(comparison, scalar)
        } else if let Some(operation) = opcode.vector_operation() {
            self.format_vector(operation)
        } else if opcode.tensor_operation().is_some() {
            self.format_tensor()
        } else {
            Err(FormatError::SyntaxError {
                message: "unsupported bytecode opcode",
            })
        }
    }

    /// Format one opcode without encoded type parameters.
    fn format_named_opcode(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            // values
            Opcode::MOVE
            | Opcode::MOVE_RANGE
            | Opcode::SELECT
            | Opcode::SELECT_RANGE
            | Opcode::EQUAL => self.format_value(opcode),

            // aggregates
            Opcode::AGGREGATE
            | Opcode::EXTRACT
            | Opcode::INSERT
            | Opcode::VARIANT_NEW
            | Opcode::VARIANT_TAG => self.format_aggregate(opcode),

            // constants
            Opcode::CONSTANT_TYPE
            | Opcode::CONSTANT_INT128
            | Opcode::CONSTANT_UINT128
            | Opcode::CONSTANT_NULL
            | Opcode::CONSTANT_UNDEFINED
            | Opcode::CONSTANT_ZEROED => self.format_named_constant(opcode),

            // addresses and pointers
            Opcode::FRAME_ADDRESS | Opcode::GLOBAL_ADDRESS => self.format_address(opcode),
            Opcode::POINTER_FRAME
            | Opcode::POINTER_GLOBAL
            | Opcode::POINTER_LOCAL
            | Opcode::POINTER_SHARED
            | Opcode::POINTER_ADD_IMMEDIATE
            | Opcode::POINTER_ADD
            | Opcode::POINTER_ADD_SCALED
            | Opcode::POINTER_BYTE_OFFSET_FROM => self.format_pointer(opcode),

            // byte ranges
            Opcode::COPY_BYTES
            | Opcode::MOVE_BYTES
            | Opcode::FILL_BYTES
            | Opcode::COMPARE_BYTES => self.format_bytes(opcode),

            // prefetch
            Opcode::PREFETCH_READ | Opcode::PREFETCH_WRITE => self.format_prefetch(opcode),

            // memory
            Opcode::LOAD => self.format_load(),
            Opcode::STORE => self.format_store(),

            // function values
            Opcode::FUNCTION_ADDRESS | Opcode::FUNCTION_BIND => self.format_function_value(opcode),

            // slices
            Opcode::SLICE_VIEW => self.format_slice(opcode),

            // dynamic values
            Opcode::DYNAMIC_BIND | Opcode::DYNAMIC_TYPE => self.format_dynamic(opcode),

            // allocation and destruction
            Opcode::FREE | Opcode::DROP => self.format_reference(opcode),

            // address stability
            Opcode::PIN | Opcode::UNPIN => self.format_reference(opcode),

            // collector protocol
            Opcode::BARRIER => self.format_reference(opcode),

            // calls
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
            | Opcode::TAIL_CALL_DYNAMIC => self.format_call(opcode),

            // continuations
            Opcode::CONTINUATION_NEW
            | Opcode::CONTINUATION_DESTROY
            | Opcode::CONTINUATION_RESUME
            | Opcode::CONTINUATION_COMPLETE => self.format_continuation(opcode),

            // waiters
            Opcode::WAITER_QUEUE | Opcode::WAITER_CANCEL => self.format_waiter(opcode),

            // tasks
            Opcode::TASK_RESOLVE
            | Opcode::TASK_START
            | Opcode::TASK_PARK
            | Opcode::TASK_CANCEL
            | Opcode::TASK_DETACH => self.format_task(opcode),

            // control flow
            Opcode::JUMP
            | Opcode::BRANCH
            | Opcode::SWITCH
            | Opcode::AWAIT
            | Opcode::YIELD
            | Opcode::RETURN
            | Opcode::TRAP
            | Opcode::UNREACHABLE
            | Opcode::BREAKPOINT
            | Opcode::POLL => self.format_control(opcode),

            // panic and unwind
            Opcode::PANIC | Opcode::PANIC_VALUE | Opcode::UNWIND_RESUME => {
                self.format_control(opcode)
            }

            // atomic memory
            Opcode::ATOMIC_FENCE => self.format_fence(),

            // runtime checks
            Opcode::CHECK_NULLISH | Opcode::CHECK_EXACT_TYPE | Opcode::CHECK_SUBTYPE => {
                self.format_runtime_check(opcode)
            }

            // profile instrumentation
            Opcode::PROFILE_INCREMENT | Opcode::PROFILE_SAMPLE => self.format_profile(opcode),

            _ => Err(FormatError::SyntaxError {
                message: "unsupported bytecode opcode",
            }),
        }
    }

    /// Write one fixed bytecode token.
    pub(super) fn write_token(&mut self, text: &'static str) -> FormatResult<()> {
        write!(self.formatter, [token(text)])
    }

    /// Write one generated bytecode name or literal.
    pub(super) fn write_text(&mut self, text: &str) -> FormatResult<()> {
        write!(self.formatter, [copied_text(text)])
    }

    /// Write one opcode followed by its operand separator.
    pub(super) fn write_opcode(&mut self, name: &str) -> FormatResult<()> {
        self.write_text(name)?;
        self.write_token(" ")
    }

    /// Write one physical register name.
    pub(super) fn write_register(&mut self, register: RegisterId) -> FormatResult<()> {
        let register = format!("r{}", register.0);

        self.write_text(&register)
    }

    /// Write one function-local branch label.
    pub(super) fn write_label(&mut self, label: Label) -> FormatResult<()> {
        self.write_text(&label.to_string())
    }

    /// Write one comma-separated register sequence.
    pub(super) fn write_registers(&mut self, registers: &[RegisterId]) -> FormatResult<()> {
        for (index, register) in registers.iter().copied().enumerate() {
            if index > 0 {
                write!(self.formatter, [token(","), space()])?;
            }
            self.write_register(register)?;
        }

        Ok(())
    }

    /// Write one breakable comma separator.
    pub(super) fn write_comma(&mut self) -> FormatResult<()> {
        write!(self.formatter, [token(","), soft_line_break_or_space()])
    }

    /// Write one unpunctuated line break opportunity.
    pub(super) fn write_break(&mut self) -> FormatResult<()> {
        write!(self.formatter, [soft_line_break_or_space()])
    }

    /// Return one required direct opcode name.
    pub(super) fn opcode_name(&self, opcode: Opcode) -> FormatResult<&'static str> {
        opcode.name().ok_or(FormatError::SyntaxError {
            message: "opcode has no direct text name",
        })
    }

    /// Write one result register.
    pub(super) fn result(&mut self) -> FormatResult<RegisterId> {
        let register = self.register_id()?;
        self.write_register(register)?;

        Ok(register)
    }

    /// Write one previously decoded result register.
    pub(super) fn write_result(&mut self, register: RegisterId) -> FormatResult<()> {
        self.write_register(register)
    }

    /// Write one result register span.
    pub(super) fn result_span(&mut self) -> FormatResult<RegisterSpan> {
        let (register, word_count) = self.register_span_id()?;
        let span = RegisterSpan::new(register, word_count);
        self.write_span(span)?;

        Ok(span)
    }

    /// Write one result register span with an inclusive end.
    pub(super) fn write_span(&mut self, span: RegisterSpan) -> FormatResult<()> {
        if span.word_count == 0 {
            return self.write_token("_");
        }

        self.write_register(span.start)?;
        if span.word_count > 1 {
            let end = span.end() - 1;
            if end >= u16::MAX as u32 {
                return Err(FormatError::SyntaxError {
                    message: "register span exceeds the bytecode register file",
                });
            }
            self.write_token(":")?;
            self.write_register(RegisterId(end as u16))?;
        }

        Ok(())
    }
}
