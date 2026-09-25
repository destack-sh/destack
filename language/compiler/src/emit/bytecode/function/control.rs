use tspp_bytecode as bytecode;
use tspp_mir as mir;

use crate::EmitError;

use super::{FunctionEmitter, Stub};

impl<'a> FunctionEmitter<'a> {
    /// Emit one runtime check with explicit success and failure edges.
    pub(super) fn emit_check(
        &mut self,
        terminator: &mir::Terminator,
        constraint: &mir::CheckConstraint,
        success: &mir::BlockTarget,
        failure: &mir::BlockTarget,
    ) -> Result<(), EmitError> {
        let failure = self.edge_label(terminator, mir::Successor::CheckFailure, failure)?;
        let mut instruction = self.check(constraint)?;
        instruction.branch(failure);
        self.encode(instruction, &[])?;

        self.emit_jump(terminator, mir::Successor::CheckSuccess, success)
    }

    /// Emit one compact integer switch.
    pub(super) fn emit_switch(
        &mut self,
        terminator: &mir::Terminator,
        value: mir::Value,
        default: &mir::BlockTarget,
        cases: mir::SwitchCaseSlice,
    ) -> Result<(), EmitError> {
        // read the discriminant register
        let value = self.word(value)?;
        let mut branches = Vec::with_capacity(cases.len());

        // resolve every logical case through its exact argument-transfer edge
        for case in self.optimized.tree.get_switch_cases(cases) {
            let successor = mir::Successor::SwitchCase { value: case.value };
            let label = self.edge_label(terminator, successor, &case.target)?;
            branches.push((case.value as u64, label));
        }

        // encode one compact table followed by its mandatory fallback
        let default = self.edge_label(terminator, mir::Successor::SwitchDefault, default)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::SWITCH);
        instruction.register(value);
        instruction
            .switch(&branches)
            .map_err(|error| self.bytecode_error(error))?;
        instruction.branch(default);

        self.encode(instruction, &[])
    }

    /// Emit one switch over a variant's logical discriminant.
    pub(super) fn emit_variant_switch(
        &mut self,
        terminator: &mir::Terminator,
        value: mir::Value,
        default: Option<&mir::BlockTarget>,
        cases: mir::SwitchCaseSlice,
    ) -> Result<(), EmitError> {
        // read the variant layout and encoding
        let variant_type = self.optimized.tree.storage_type(self.value_type(value)?);
        let discriminant = match self.optimized.tree.get(variant_type) {
            mir::Type::Variant { discriminant, .. } => *discriminant,
            _ => return Err(self.internal("variant switch requires a variant value")),
        };
        let tag_type = self.types.register_type(discriminant)?;
        let tag = self.scratch(tag_type)?;

        // decode the logical discriminant into one reusable scratch register
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::VARIANT_TAG);
        instruction.span(self.register(value)?);
        let layout = self.types.type_id(variant_type)?;
        instruction.relocation(bytecode::RelocationTag::LAYOUT, layout.0);
        self.encode(instruction, &[tag])?;

        // map case indices to their declared discriminant values
        let mut branches = Vec::with_capacity(cases.len());
        for case in self.optimized.tree.get_switch_cases(cases) {
            let case_index = u32::try_from(case.value)
                .map_err(|_| self.internal("variant case index is negative"))?;
            let discriminant = self.types.variant_discriminant(variant_type, case_index)?;
            let discriminant = u64::try_from(discriminant)
                .map_err(|_| self.internal("variant discriminant exceeds bytecode switch"))?;
            let successor = mir::Successor::SwitchCase { value: case.value };
            let target = self.edge_label(terminator, successor, &case.target)?;
            branches.push((discriminant, target));
        }

        // route invalid discriminants to the explicit default or an unreachable stub
        let fallback = if let Some(default) = default {
            self.edge_label(terminator, mir::Successor::SwitchDefault, default)?
        } else {
            let label = bytecode::Label(self.next_label);
            self.next_label += 1;
            self.stubs.push(Stub::Unreachable { label });

            label
        };
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::SWITCH);
        instruction.register(tag.start);
        instruction
            .switch(&branches)
            .map_err(|error| self.bytecode_error(error))?;
        instruction.branch(fallback);

        self.encode(instruction, &[])
    }

    /// Emit one unconditional control transfer.
    pub(super) fn emit_jump(
        &mut self,
        terminator: &mir::Terminator,
        successor: mir::Successor,
        target: &mir::BlockTarget,
    ) -> Result<(), EmitError> {
        let target = self.edge_label(terminator, successor, target)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::JUMP);
        instruction.branch(target);

        self.encode(instruction, &[])
    }

    /// Emit one conditional control transfer.
    pub(super) fn emit_branch(
        &mut self,
        terminator: &mir::Terminator,
        condition: mir::Value,
        then_target: &mir::BlockTarget,
        else_target: &mir::BlockTarget,
    ) -> Result<(), EmitError> {
        let then_target = self.edge_label(terminator, mir::Successor::BranchThen, then_target)?;
        let else_target = self.edge_label(terminator, mir::Successor::BranchElse, else_target)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::BRANCH);
        instruction.register(self.word(condition)?);
        instruction.branch(then_target);
        instruction.branch(else_target);

        self.encode(instruction, &[])
    }

    /// Build one bytecode runtime check before its failure edge.
    fn check(
        &self,
        constraint: &mir::CheckConstraint,
    ) -> Result<bytecode::InstructionBuilder, EmitError> {
        match constraint {
            mir::CheckConstraint::Bounds { index, length, .. } => {
                let scalar = self.scalar(*index)?;

                self.scalar_check(bytecode::ScalarCheck::Bounds, scalar, &[*index, *length])
            }
            mir::CheckConstraint::Null { value } => {
                let mut instruction =
                    bytecode::InstructionBuilder::new(bytecode::Opcode::CHECK_NULLISH);
                instruction.register(self.word(*value)?);

                Ok(instruction)
            }
            mir::CheckConstraint::DivZero { divisor } => {
                let scalar = self.scalar(*divisor)?;

                self.scalar_check(bytecode::ScalarCheck::Nonzero, scalar, &[*divisor])
            }
            mir::CheckConstraint::Overflow {
                operator,
                left,
                right,
                ..
            } => {
                let operation = match operator {
                    mir::BinaryOperator::Add => bytecode::ScalarCheck::AddOverflow,
                    mir::BinaryOperator::Subtract => bytecode::ScalarCheck::SubtractOverflow,
                    mir::BinaryOperator::Multiply => bytecode::ScalarCheck::MultiplyOverflow,
                    _ => return Err(self.internal("invalid overflow check operator")),
                };
                let scalar = self.scalar(*left)?;

                self.scalar_check(operation, scalar, &[*left, *right])
            }
            mir::CheckConstraint::IsType { value, expected }
            | mir::CheckConstraint::IsSubtype { value, expected } => {
                let opcode = if matches!(constraint, mir::CheckConstraint::IsType { .. }) {
                    bytecode::Opcode::CHECK_EXACT_TYPE
                } else {
                    bytecode::Opcode::CHECK_SUBTYPE
                };
                let mut instruction = bytecode::InstructionBuilder::new(opcode);
                instruction.register(self.word(*value)?);
                let expected = self.types.type_id(*expected)?;
                instruction.relocation(bytecode::RelocationTag::TYPE, expected.0);

                Ok(instruction)
            }
            mir::CheckConstraint::ShiftRange {
                value, bit_width, ..
            } => {
                let scalar = self.scalar(*value)?;
                let opcode = bytecode::Opcode::check(bytecode::ScalarCheck::Shift, scalar)
                    .ok_or_else(|| self.internal("invalid shift check"))?;
                let mut instruction = bytecode::InstructionBuilder::new(opcode);
                instruction.register(self.word(*value)?);
                instruction.u16(u16::from(*bit_width));

                Ok(instruction)
            }
            mir::CheckConstraint::Narrow {
                value,
                to_width,
                is_signed,
            } => {
                let scalar = self.scalar(*value)?;
                let target = self
                    .types
                    .integer_scalar(u16::from(*to_width), *is_signed)?;
                let opcode = bytecode::Opcode::check(bytecode::ScalarCheck::Narrow, scalar)
                    .ok_or_else(|| self.internal("invalid narrow check"))?;
                let mut instruction = bytecode::InstructionBuilder::new(opcode);
                instruction.register(self.word(*value)?);
                instruction.scalar(target);

                Ok(instruction)
            }
        }
    }

    /// Build one scalar check instruction.
    fn scalar_check(
        &self,
        operation: bytecode::ScalarCheck,
        scalar: bytecode::Scalar,
        values: &[mir::Value],
    ) -> Result<bytecode::InstructionBuilder, EmitError> {
        let opcode = bytecode::Opcode::check(operation, scalar)
            .ok_or_else(|| self.internal("invalid scalar check"))?;
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        for value in values {
            instruction.register(self.word(*value)?);
        }

        Ok(instruction)
    }

    /// Return one MIR value's scalar bytecode representation.
    fn scalar(&self, value: mir::Value) -> Result<bytecode::Scalar, EmitError> {
        self.register_type(value)?
            .scalar_type()
            .ok_or_else(|| self.internal("runtime check requires one scalar register"))
    }

    /// Return a direct block label or defer one argument transfer.
    pub(super) fn edge_label(
        &mut self,
        terminator: &mir::Terminator,
        successor: mir::Successor,
        target: &mir::BlockTarget,
    ) -> Result<bytecode::Label, EmitError> {
        // read the destination block parameters
        let parameters = terminator
            .target_parameters(&self.optimized.tree, successor, target)
            .ok_or_else(|| self.internal("invalid block argument count"))?;
        if target.arguments(&self.optimized.tree).is_empty() {
            return self.block_label(target.block);
        }

        let parameters = parameters.iter().map(|parameter| parameter.value).collect();
        let label = bytecode::Label(self.next_label);
        self.next_label += 1;
        self.stubs.push(Stub::Transfer {
            label,
            target: target.clone(),
            parameters,
        });

        Ok(label)
    }

    /// Return direct result destinations at one successor.
    pub(super) fn successor_destinations(
        &self,
        terminator: &mir::Terminator,
        successor: mir::Successor,
        target: &mir::BlockTarget,
    ) -> Result<Vec<bytecode::RegisterSpan>, EmitError> {
        // read the successor parameter registers
        let block = self.optimized.tree.get(target.block);
        terminator
            .target_parameters(&self.optimized.tree, successor, target)
            .ok_or_else(|| self.internal("invalid block argument count"))?;
        let count = terminator.target_result_count(&self.optimized.tree, successor);

        block.parameters[..count]
            .iter()
            .map(|parameter| self.register(parameter.value))
            .collect()
    }

    /// Emit one deferred block argument transfer.
    pub(super) fn emit_transfer(
        &mut self,
        target: &mir::BlockTarget,
        parameters: &[mir::Value],
    ) -> Result<(), EmitError> {
        // read the arguments transferred to the successor
        let arguments = target.arguments(&self.optimized.tree);
        if arguments.len() != parameters.len() {
            return Err(self.internal("block argument count"));
        }

        let mut moves = arguments
            .iter()
            .zip(parameters)
            .map(|(argument, parameter)| {
                Ok((
                    self.register(*argument)?,
                    self.register(*parameter)?,
                    self.register_type(*argument)?,
                ))
            })
            .collect::<Result<Vec<_>, EmitError>>()?;
        moves.retain(|(source, destination, _)| source != destination);

        // emit acyclic moves first and preserve one source to break each cycle
        while !moves.is_empty() {
            let ready = moves.iter().position(|(_, destination, _)| {
                !moves.iter().any(|(source, _, _)| source == destination)
            });
            if let Some(index) = ready {
                let (source, destination, ty) = moves.remove(index);
                self.emit_move(source, destination, ty)?;
            } else {
                let source = moves[0].0;
                let ty = moves[0].2;
                let scratch = self.scratch(ty)?;
                self.emit_move(source, scratch, ty)?;
                for (candidate, _, _) in &mut moves {
                    if *candidate == source {
                        *candidate = scratch;
                    }
                }
            }
        }

        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::JUMP);
        instruction.branch(self.block_label(target.block)?);

        self.encode(instruction, &[])
    }
}
