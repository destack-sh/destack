use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_frontend::Switch;
use tspp_mir as mir;
use tspp_native as native;

use crate::EmitError;

use super::FunctionEmitter;

impl<'a> FunctionEmitter<'a> {
    /// Emit one explicit runtime check branch.
    pub(super) fn emit_check(
        &mut self,
        constraint: &mir::CheckConstraint,
        success: &mir::BlockTarget,
        failure: &mir::BlockTarget,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let condition = self.check_condition(constraint, builder)?;
        let success_arguments = self.block_arguments(success)?;
        let failure_arguments = self.block_arguments(failure)?;
        builder.ins().brif(
            condition,
            self.blocks[&success.block],
            &success_arguments,
            self.blocks[&failure.block],
            &failure_arguments,
        );

        Ok(())
    }

    /// Emit one sparse or dense integer switch.
    pub(super) fn emit_switch(
        &self,
        value: mir::Value,
        default: &mir::BlockTarget,
        cases: mir::SwitchCaseSlice,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        // read the scalar discriminant
        let value = self.scalar(value)?;
        let mut transfers = Vec::new();
        let default = self.switch_block(default, &mut transfers, builder)?;
        let mut switch = Switch::new();

        // map every MIR case to one direct or argument-transfer block
        for case in self.optimized.tree.get_switch_cases(cases) {
            let block = self.switch_block(&case.target, &mut transfers, builder)?;
            switch.set_entry(case.value as u128, block);
        }
        switch.emit(builder, value, default);

        // transfer case-specific arguments outside the switch dispatch
        for (block, target, arguments) in transfers {
            builder.switch_to_block(block);
            builder.seal_block(block);
            builder.ins().jump(target, &arguments);
        }

        Ok(())
    }

    /// Return the boolean condition for one MIR runtime check.
    fn check_condition(
        &mut self,
        constraint: &mir::CheckConstraint,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        match constraint {
            mir::CheckConstraint::Bounds {
                index,
                length,
                is_signed,
                ..
            } => {
                let index = self.scalar(*index)?;
                let length = self.scalar(*length)?;
                let upper = if *is_signed {
                    builder.ins().icmp(IntCC::SignedLessThan, index, length)
                } else {
                    builder.ins().icmp(IntCC::UnsignedLessThan, index, length)
                };
                if !*is_signed {
                    return Ok(upper);
                }
                let lower = builder
                    .ins()
                    .icmp_imm_s(IntCC::SignedGreaterThanOrEqual, index, 0);

                Ok(builder.ins().band(lower, upper))
            }
            mir::CheckConstraint::Null { value } => {
                let value = self.reference(*value)?;

                Ok(builder.ins().icmp_imm_u(IntCC::NotEqual, value, 0))
            }
            mir::CheckConstraint::DivZero { divisor } => {
                let divisor = self.scalar(*divisor)?;

                Ok(builder.ins().icmp_imm_u(IntCC::NotEqual, divisor, 0))
            }
            mir::CheckConstraint::ShiftRange {
                value,
                bit_width,
                is_signed,
            } => {
                let value = self.scalar(*value)?;
                let upper = if *is_signed {
                    builder
                        .ins()
                        .icmp_imm_s(IntCC::SignedLessThan, value, i64::from(*bit_width))
                } else {
                    builder
                        .ins()
                        .icmp_imm_u(IntCC::UnsignedLessThan, value, i64::from(*bit_width))
                };
                if !*is_signed {
                    return Ok(upper);
                }
                let lower = builder
                    .ins()
                    .icmp_imm_s(IntCC::SignedGreaterThanOrEqual, value, 0);

                Ok(builder.ins().band(lower, upper))
            }
            mir::CheckConstraint::Narrow {
                value,
                to_width,
                is_signed,
            } => self.narrow_condition(*value, u16::from(*to_width), *is_signed, builder),
            mir::CheckConstraint::Overflow {
                operator,
                left,
                right,
                is_signed,
            } => self.overflow_condition(*operator, *left, *right, *is_signed, builder),
            mir::CheckConstraint::IsType { value, expected } => {
                let concrete = self.scalar(*value)?;
                let expected = self.type_id(*expected, builder)?;

                Ok(builder.ins().icmp(IntCC::Equal, concrete, expected))
            }
            mir::CheckConstraint::IsSubtype { value, expected } => {
                let concrete = self.scalar(*value)?;
                let expected = self.type_id(*expected, builder)?;
                let call = self.emit_runtime(
                    native::abi::Operation::IsSubtype,
                    &[concrete, expected],
                    builder,
                )?;
                let is_subtype = builder.inst_results(call)[0];

                Ok(builder.ins().icmp_imm_u(IntCC::NotEqual, is_subtype, 0))
            }
        }
    }

    /// Load one object-local type as its linked Program id.
    fn type_id(
        &mut self,
        ty: mir::TypeId,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let ty = u32::try_from(ty.index())
            .map_err(|_| self.invalid("native type identity exceeds u32"))?;

        self.index_u32(native::Index::Type { ty }, builder)
    }

    /// Return whether one integer fits one narrower target representation.
    fn narrow_condition(
        &self,
        value: mir::Value,
        target_width: u16,
        target_signed: bool,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        // resolve integer widths using the target pointer width
        let pointer_bits = self.types.layout.pointer_bits();
        let source = self.optimized.tree.storage_type(self.value_type(value)?);
        let (source_width, source_signed) = self
            .optimized
            .tree
            .type_definition(source)
            .integer(pointer_bits)
            .ok_or_else(|| self.invalid("native narrow check value is not an integer"))?;
        let source_type = cir::Type::int(source_width)
            .ok_or_else(|| self.invalid("native narrow check width is unsupported"))?;
        let value = self.scalar(value)?;
        let mut conditions = Vec::with_capacity(2);

        // reject negative signed values for an unsigned target
        if source_signed && !target_signed {
            conditions.push(
                builder
                    .ins()
                    .icmp_imm_s(IntCC::SignedGreaterThanOrEqual, value, 0),
            );
        }

        // enforce a narrower signed lower bound
        if source_signed && target_signed && target_width < source_width {
            let minimum = -(1i128 << (target_width - 1));
            let minimum = self.emit_integer_constant(source_type, minimum as u128, builder)?;
            conditions.push(
                builder
                    .ins()
                    .icmp(IntCC::SignedGreaterThanOrEqual, value, minimum),
            );
        }

        // enforce an upper bound when the target maximum is smaller
        let target_maximum = self.types.integer_maximum(target_width, target_signed);
        let source_maximum = self.types.integer_maximum(source_width, source_signed);
        if target_maximum < source_maximum {
            let maximum = self.emit_integer_constant(source_type, target_maximum, builder)?;
            let condition = if source_signed {
                builder
                    .ins()
                    .icmp(IntCC::SignedLessThanOrEqual, value, maximum)
            } else {
                builder
                    .ins()
                    .icmp(IntCC::UnsignedLessThanOrEqual, value, maximum)
            };
            conditions.push(condition);
        }

        // combine both active bounds or emit a constant true condition
        let condition = match conditions.as_slice() {
            [] => builder.ins().iconst(cir::types::I8, 1),
            [condition] => *condition,
            [lower, upper] => builder.ins().band(*lower, *upper),
            _ => return Err(self.invalid("native narrow check has invalid bounds")),
        };

        Ok(condition)
    }

    /// Return whether one arithmetic operation does not overflow.
    fn overflow_condition(
        &self,
        operator: mir::BinaryOperator,
        left: mir::Value,
        right: mir::Value,
        is_signed: bool,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let left = self.scalar(left)?;
        let right = self.scalar(right)?;
        let (_, overflow) = match (operator, is_signed) {
            (mir::BinaryOperator::Add, true) => builder.ins().sadd_overflow(left, right),
            (mir::BinaryOperator::Add, false) => builder.ins().uadd_overflow(left, right),
            (mir::BinaryOperator::Subtract, true) => builder.ins().ssub_overflow(left, right),
            (mir::BinaryOperator::Subtract, false) => builder.ins().usub_overflow(left, right),
            (mir::BinaryOperator::Multiply, true) => builder.ins().smul_overflow(left, right),
            (mir::BinaryOperator::Multiply, false) => builder.ins().umul_overflow(left, right),
            _ => return Err(self.invalid("native overflow check operator is unsupported")),
        };

        Ok(builder.ins().icmp_imm_u(IntCC::Equal, overflow, 0))
    }

    /// Return a switch destination, inserting an argument transfer when needed.
    fn switch_block(
        &self,
        target: &mir::BlockTarget,
        transfers: &mut Vec<(cir::Block, cir::Block, Vec<cir::BlockArg>)>,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Block, EmitError> {
        let arguments = self.block_arguments(target)?;
        let target_block = self.blocks[&target.block];
        if arguments.is_empty() {
            return Ok(target_block);
        }

        let block = builder.create_block();
        transfers.push((block, target_block, arguments));

        Ok(block)
    }
}
