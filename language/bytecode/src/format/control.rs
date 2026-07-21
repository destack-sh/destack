use std::collections::BTreeSet;

use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    BytecodeFormatContext, CodeOffset, CodeRange, Comparison, Label, Opcode, ReferenceKind, Scalar,
    ScalarCheck, Space, Trap, ValueType,
};

use super::instruction::InstructionFormatter;

impl BytecodeFormatContext<'_> {
    /// Collect canonical labels for every branch target in one function.
    pub(super) fn collect_labels(&mut self, code: CodeRange) -> FormatResult<()> {
        let mut targets = BTreeSet::new();
        let mut instructions = BTreeSet::new();
        let mut instruction_offset = 0usize;

        // collect every direct and switch branch target
        for instruction in code.instructions(self.object.code()) {
            instructions.insert(instruction_offset);
            let instruction = instruction.map_err(|_| FormatError::SyntaxError {
                message: "function contains an invalid instruction",
            })?;
            let layout = instruction
                .opcode()
                .layout()
                .ok_or(FormatError::SyntaxError {
                    message: "function contains an unknown opcode",
                })?;

            // collect every displacement carried by this instruction
            let branch_offsets =
                instruction
                    .branch_offsets(layout)
                    .map_err(|_| FormatError::SyntaxError {
                        message: "instruction contains malformed branch operands",
                    })?;
            for branch_offset in branch_offsets {
                let displacement =
                    instruction
                        .read_u32(branch_offset)
                        .map_err(|_| FormatError::SyntaxError {
                            message: "instruction contains a truncated branch",
                        })? as i32;
                targets.insert(Self::branch_target(
                    instruction_offset,
                    instruction.byte_len(),
                    displacement,
                )?);
            }
            instruction_offset += instruction.byte_len();
        }

        // require every target to name an instruction in this function
        if !targets.is_subset(&instructions) {
            return Err(FormatError::SyntaxError {
                message: "branch target is not an instruction in its function",
            });
        }

        // assign stable labels in code order
        self.labels.clear();
        for (index, target) in targets.into_iter().enumerate() {
            self.labels
                .insert(CodeOffset(target as u32), Label(index as u32));
        }

        Ok(())
    }

    /// Return the canonical label selected by one relative branch.
    pub(super) fn branch_label(
        &self,
        instruction: CodeOffset,
        byte_len: usize,
        displacement: i32,
    ) -> FormatResult<Label> {
        let target = Self::branch_target(instruction.index(), byte_len, displacement)?;

        self.labels
            .get(&CodeOffset(target as u32))
            .copied()
            .ok_or(FormatError::SyntaxError {
                message: "branch targets an instruction without a label",
            })
    }

    /// Resolve one end-relative branch displacement.
    fn branch_target(start: usize, byte_len: usize, displacement: i32) -> FormatResult<usize> {
        let end = start as i64 + byte_len as i64;
        let target = end + displacement as i64;

        usize::try_from(target).map_err(|_| FormatError::SyntaxError {
            message: "branch target falls outside its function",
        })
    }
}

impl InstructionFormatter<'_, '_, '_> {
    /// Format one typed runtime check.
    pub(super) fn format_check(&mut self, check: ScalarCheck, scalar: Scalar) -> FormatResult<()> {
        self.write_token("check.")?;
        self.write_text(check.name())?;
        self.write_token(".")?;
        self.write_text(scalar.name())?;
        self.write_token(" ")?;
        let input = self.register_id()?;
        self.write_register(input)?;

        // format the operation-specific check bounds
        match check {
            ScalarCheck::Shift => {
                let width = self.u16()?.to_string();
                write!(self.formatter, [token(","), space()])?;
                self.write_text(&width)?;
            }
            ScalarCheck::Narrow => {
                let target =
                    Scalar::from_code(self.u16()? as u8).ok_or(FormatError::SyntaxError {
                        message: "narrow check has an invalid target scalar",
                    })?;
                write!(self.formatter, [space(), token("->"), space()])?;
                self.write_text(target.name())?;
            }
            ScalarCheck::Bounds
            | ScalarCheck::AddOverflow
            | ScalarCheck::SubtractOverflow
            | ScalarCheck::MultiplyOverflow => {
                let right = self.register_id()?;
                write!(self.formatter, [token(","), space()])?;
                self.write_register(right)?;
            }
            ScalarCheck::Range => {
                let start = self.register_id()?;
                let length = self.register_id()?;
                write!(self.formatter, [token(","), space()])?;
                self.write_register(start)?;
                write!(self.formatter, [token(","), space()])?;
                self.write_register(length)?;
            }
            ScalarCheck::Nonzero => {}
        }

        // write the common failure destination
        let failure = self.branch()?;
        write!(self.formatter, [space(), token("else"), space()])?;
        self.write_label(failure)
    }

    /// Format one fused scalar branch.
    pub(super) fn format_branch(
        &mut self,
        comparison: Comparison,
        scalar: Scalar,
    ) -> FormatResult<()> {
        // decode the compared values and both destinations
        let left = self.register_id()?;
        let right = self.register_id()?;
        let success = self.branch()?;
        let failure = self.branch()?;

        // write the typed comparison
        self.write_token("branch.")?;
        self.write_text(comparison.name())?;
        self.write_token(".")?;
        self.write_text(scalar.name())?;
        self.write_token(" ")?;
        self.write_register(left)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(right)?;
        write!(self.formatter, [space(), token("=>"), space()])?;
        self.write_label(success)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_label(failure)
    }

    /// Format one inline switch table.
    pub(super) fn format_switch(&mut self) -> FormatResult<()> {
        // write the switched value and table opening
        let value = self.register_id()?;
        let count = self.u16()? as usize;
        write!(self.formatter, [token("switch"), space()])?;
        self.write_register(value)?;
        write!(self.formatter, [space(), token("{")])?;

        // write cases in encoded order
        for index in 0..count {
            let case = self.u64()?.to_string();
            let target = self.branch()?;
            write!(self.formatter, [space()])?;
            self.write_text(&case)?;
            write!(self.formatter, [space(), token("=>"), space()])?;
            self.write_label(target)?;
            if index + 1 < count {
                self.write_token(",")?;
            }
        }

        // write the mandatory fallback
        let fallback = self.branch()?;
        if count > 0 {
            self.write_token(",")?;
        }
        write!(
            self.formatter,
            [space(), token("default"), space(), token("=>"), space()]
        )?;
        self.write_label(fallback)?;
        write!(self.formatter, [space(), token("}")])
    }

    /// Format one fixed control operation.
    pub(super) fn format_control(&mut self, opcode: Opcode) -> FormatResult<()> {
        match opcode {
            // control flow
            Opcode::JUMP => self.format_jump(),
            Opcode::BRANCH => self.format_boolean_branch(),
            Opcode::SWITCH => self.format_switch(),
            Opcode::YIELD => self.format_yield(),
            Opcode::RETURN => self.format_return(),
            Opcode::TRAP => self.format_trap(),
            Opcode::UNREACHABLE => {
                let name = self.fixed_name(opcode)?;

                self.write_text(name)
            }

            // panic and unwind
            Opcode::CATCH => {
                self.result(ValueType::reference(ReferenceKind::MANAGED, Space::LOCAL))?;
                write!(
                    self.formatter,
                    [space(), token("="), space(), token("catch")]
                )
            }
            Opcode::PANIC | Opcode::PANIC_VALUE => self.format_panic(opcode),
            Opcode::UNWIND_RESUME => {
                let name = self.fixed_name(opcode)?;

                self.write_text(name)
            }

            // collector protocol
            Opcode::SAFEPOINT => {
                let name = self.fixed_name(opcode)?;

                self.write_text(name)
            }

            // debug control
            Opcode::BREAKPOINT => {
                let name = self.fixed_name(opcode)?;

                self.write_text(name)
            }
            _ => Err(FormatError::SyntaxError {
                message: "invalid control opcode",
            }),
        }
    }

    /// Format one unconditional jump.
    fn format_jump(&mut self) -> FormatResult<()> {
        let target = self.branch()?;
        write!(self.formatter, [token("jump"), space()])?;
        self.write_label(target)
    }

    /// Format one boolean branch.
    fn format_boolean_branch(&mut self) -> FormatResult<()> {
        // decode the condition and both destinations
        let condition = self.register_id()?;
        let success = self.branch()?;
        let failure = self.branch()?;

        // write the condition and both destinations
        write!(self.formatter, [token("branch"), space()])?;
        self.write_register(condition)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_label(success)?;
        write!(self.formatter, [token(","), space()])?;
        self.write_label(failure)
    }

    /// Format one coroutine yield.
    fn format_yield(&mut self) -> FormatResult<()> {
        // write the resume result assignment
        let function = *self.formatter.context().active_function()?;
        let types = self.formatter.context().object.value_types();
        self.results(function.resume_types(types))?;
        write!(
            self.formatter,
            [space(), token("="), space(), token("yield")]
        )?;

        // write yielded logical values
        let values = self.register_value_ids()?;
        for (index, value) in values.into_iter().enumerate() {
            if index == 0 {
                write!(self.formatter, [space()])?;
            } else {
                write!(self.formatter, [token(","), space()])?;
            }
            self.write_register(value)?;
        }

        // write resume and unwind destinations
        let resume = self.branch()?;
        let unwind = self.branch()?;
        write!(self.formatter, [space(), token("=>"), space()])?;
        self.write_label(resume)?;
        write!(self.formatter, [space(), token("|"), space()])?;
        self.write_label(unwind)
    }

    /// Format one function return.
    fn format_return(&mut self) -> FormatResult<()> {
        // write every logical return value
        let values = self.register_value_ids()?;
        self.write_token("return")?;
        for (index, value) in values.into_iter().enumerate() {
            if index == 0 {
                write!(self.formatter, [space()])?;
            } else {
                write!(self.formatter, [token(","), space()])?;
            }
            self.write_register(value)?;
        }

        Ok(())
    }

    /// Format one explicit execution trap.
    fn format_trap(&mut self) -> FormatResult<()> {
        let trap = Trap::from_code(self.u16()? as u8).ok_or(FormatError::SyntaxError {
            message: "trap has an invalid reason",
        })?;
        write!(self.formatter, [token("trap"), space()])?;
        self.write_text(trap.name())
    }

    /// Format one panic with or without a value.
    fn format_panic(&mut self, opcode: Opcode) -> FormatResult<()> {
        self.write_token("panic")?;
        if opcode == Opcode::PANIC_VALUE {
            let value = self.register_id()?;
            write!(self.formatter, [space()])?;
            self.write_register(value)?;
        }

        Ok(())
    }

    /// Format one null or runtime type check.
    pub(super) fn format_runtime_check(&mut self, opcode: Opcode) -> FormatResult<()> {
        // write the checked runtime value
        let value = self.register_id()?;
        let name = self.fixed_name(opcode)?;
        self.write_text(name)?;
        self.write_token(" ")?;
        self.write_register(value)?;

        // write the expected runtime type when required
        if matches!(opcode, Opcode::CHECK_EXACT_TYPE | Opcode::CHECK_SUBTYPE) {
            let ty = self.symbol()?;
            write!(self.formatter, [token(":"), space()])?;
            self.write_text(&ty)?;
        }

        // write the common failure destination
        let failure = self.branch()?;
        write!(self.formatter, [space(), token("else"), space()])?;
        self.write_label(failure)
    }
}
