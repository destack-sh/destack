use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    BytecodeFormatter, CodeOffset, Instruction, Label, Opcode, Operands, RegisterId, ValueType,
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
        } else if opcode.vector_operation().is_some() {
            self.format_vector()
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
            | Opcode::FIELD_GET
            | Opcode::FIELD_SET
            | Opcode::ELEMENT_GET
            | Opcode::ELEMENT_SET
            | Opcode::VARIANT_NEW
            | Opcode::VARIANT_TAG
            | Opcode::VARIANT_PAYLOAD => self.format_aggregate(opcode),

            // constants
            Opcode::CONSTANT_TYPE
            | Opcode::CONSTANT_BYTES
            | Opcode::CONSTANT_INT128
            | Opcode::CONSTANT_UINT128
            | Opcode::CONSTANT_NULL
            | Opcode::CONSTANT_UNDEFINED
            | Opcode::CONSTANT_UNINIT
            | Opcode::CONSTANT_ZEROED => self.format_named_constant(opcode),

            // addresses
            Opcode::GLOBAL_ADDRESS
            | Opcode::FRAME_ADDRESS
            | Opcode::POINTER_OFFSET
            | Opcode::POINTER_INDEX
            | Opcode::POINTER_DISTANCE
            | Opcode::REFERENCE_POINTER => self.format_pointer(opcode),

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
            Opcode::FRAME_LOAD | Opcode::FRAME_STORE => self.format_frame(opcode),

            // function values
            Opcode::FUNCTION_ADDRESS
            | Opcode::FUNCTION_BIND
            | Opcode::FUNCTION_ENVIRONMENT
            | Opcode::FUNCTION_ENVIRONMENT_CURRENT => self.format_function_value(opcode),

            // slices
            Opcode::SLICE_VIEW | Opcode::SLICE_LENGTH => self.format_slice(opcode),

            // dynamic values
            Opcode::DYNAMIC_BIND | Opcode::DYNAMIC_PAYLOAD | Opcode::DYNAMIC_TYPE => {
                self.format_dynamic(opcode)
            }

            // allocation and destruction
            Opcode::NEW_COMPLETE => self.format_new_complete(),
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

            // control flow
            Opcode::JUMP
            | Opcode::BRANCH
            | Opcode::SWITCH
            | Opcode::YIELD
            | Opcode::RETURN
            | Opcode::TRAP
            | Opcode::UNREACHABLE
            | Opcode::BREAKPOINT => self.format_control(opcode),

            // panic and unwind
            Opcode::PANIC | Opcode::PANIC_VALUE | Opcode::UNWIND_RESUME => {
                self.format_control(opcode)
            }

            // atomic memory
            Opcode::ATOMIC_FENCE => self.format_fence(),

            // runtime checks
            Opcode::CHECK_NULL | Opcode::CHECK_EXACT_TYPE | Opcode::CHECK_SUBTYPE => {
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

    /// Write one comma-separated break opportunity.
    pub(super) fn comma(&mut self) -> FormatResult<()> {
        write!(self.formatter, [token(","), soft_line_break_or_space()])
    }

    /// Write one unpunctuated break opportunity.
    pub(super) fn continuation(&mut self) -> FormatResult<()> {
        write!(self.formatter, [soft_line_break_or_space()])
    }

    /// Return one required direct opcode name.
    pub(super) fn opcode_name(&self, opcode: Opcode) -> FormatResult<&'static str> {
        opcode.name().ok_or(FormatError::SyntaxError {
            message: "opcode has no direct text name",
        })
    }

    /// Append one result register.
    pub(super) fn result(&mut self, ty: ValueType) -> FormatResult<()> {
        let register = self.register_id()?;
        self.write_result(register, ty)
    }

    /// Append one previously decoded result register.
    pub(super) fn write_result(&mut self, register: RegisterId, ty: ValueType) -> FormatResult<()> {
        if self.formatter.context().register_type(register)? != ty {
            return Err(FormatError::SyntaxError {
                message: "instruction result type differs from its register type",
            });
        }

        let name = self.formatter.context().value_type_text(ty)?.to_string();
        self.write_register(register)?;
        self.write_token(": ")?;
        self.write_text(&name)?;

        Ok(())
    }

    /// Append the first register of one result range.
    pub(super) fn result_range(&mut self, ty: ValueType) -> FormatResult<()> {
        let (register, word_count) = self.register_range_id()?;
        if word_count != ty.word_count() {
            return Err(FormatError::SyntaxError {
                message: "instruction result width does not match its type",
            });
        }
        self.write_result(register, ty)
    }

    /// Append one result range using its declared register type.
    pub(super) fn declared_result_range(&mut self) -> FormatResult<ValueType> {
        let (register, word_count) = self.register_range_id()?;
        let ty = self.formatter.context().register_type(register)?;
        if word_count != ty.word_count() {
            return Err(FormatError::SyntaxError {
                message: "instruction result width does not match its register type",
            });
        }
        self.write_result(register, ty)?;

        Ok(ty)
    }

    /// Append the logical values in one encoded result range.
    pub(super) fn results(&mut self, types: &[ValueType]) -> FormatResult<()> {
        let (start, word_count) = self.register_range_id()?;
        self.write_results(start, word_count, types)
    }

    /// Append logical values in one previously decoded result range.
    pub(super) fn write_results(
        &mut self,
        start: RegisterId,
        word_count: u16,
        types: &[ValueType],
    ) -> FormatResult<()> {
        let expected_word_count = types.iter().map(|ty| ty.word_count()).sum::<u16>();
        if word_count != expected_word_count {
            return Err(FormatError::SyntaxError {
                message: "instruction result range does not match its value types",
            });
        }

        let mut register = start;
        for (index, ty) in types.iter().enumerate() {
            if index > 0 {
                write!(self.formatter, [token(","), space()])?;
            }
            self.write_result(register, *ty)?;
            register.0 += ty.word_count();
        }

        Ok(())
    }
}
