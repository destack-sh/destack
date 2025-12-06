use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, StackSlot, Value as CraneliftValue};
use cranelift_frontend::FunctionBuilder;
use destack_mir::{
    BinaryOperator, CastKind, Constant, Instruction, Local, LocalNodeId, NodeTree, UnaryOperator,
    Value,
};

use super::types::lower_type;
use crate::CraneliftError;

/// Lower a MIR instruction to Cranelift IR.
pub(crate) fn lower_instruction(
    tree: &NodeTree,
    instruction: &Instruction,
    builder: &mut FunctionBuilder<'_>,
    value_map: &mut HashMap<Value, CraneliftValue>,
    local_map: &HashMap<LocalNodeId<Local>, StackSlot>,
) -> Result<(), CraneliftError> {
    match instruction {
        Instruction::Constant { destination, value } => {
            let cl_value = lower_constant(value, builder)?;
            value_map.insert(*destination, cl_value);
        }

        Instruction::Binary {
            destination,
            operator,
            left,
            right,
        } => {
            let left_value = value_map[left];
            let right_value = value_map[right];
            let cl_value = lower_binary_op(*operator, left_value, right_value, builder)?;
            value_map.insert(*destination, cl_value);
        }

        Instruction::Unary {
            destination,
            operator,
            argument,
        } => {
            let argument_value = value_map[argument];
            let cl_value = lower_unary_op(*operator, argument_value, builder)?;
            value_map.insert(*destination, cl_value);
        }

        Instruction::Cast {
            destination,
            kind,
            argument,
            to_type,
        } => {
            let argument_value = value_map[argument];
            let target_type = lower_type(tree, *to_type)?;
            let cl_value = lower_cast(*kind, argument_value, target_type, builder)?;
            value_map.insert(*destination, cl_value);
        }

        Instruction::LocalGet { destination, local } => {
            let slot = local_map[local];
            let local_data = tree.get(*local);
            let ty = lower_type(tree, local_data.ty)?;
            let cl_value = builder.ins().stack_load(ty, slot, 0);
            value_map.insert(*destination, cl_value);
        }

        Instruction::LocalSet { local, value } => {
            let slot = local_map[local];
            let cl_value = value_map[value];
            builder.ins().stack_store(cl_value, slot, 0);
        }

        Instruction::Load {
            destination,
            pointer,
        } => {
            // nocheckin TODO: need to know the loaded type from context
            let pointer_value = value_map[pointer];
            let cl_value = builder.ins().load(
                cranelift_codegen::ir::types::I64,
                cranelift_codegen::ir::MemFlags::new(),
                pointer_value,
                0,
            );
            value_map.insert(*destination, cl_value);
        }

        Instruction::Store { pointer, value } => {
            let pointer_value = value_map[pointer];
            let store_value = value_map[value];
            builder.ins().store(
                cranelift_codegen::ir::MemFlags::new(),
                store_value,
                pointer_value,
                0,
            );
        }

        Instruction::ExtractField { .. } => {
            return Err(CraneliftError::UnsupportedInstruction {
                description: "ExtractField not yet implemented".into(),
            });
        }

        Instruction::InsertField { .. } => {
            return Err(CraneliftError::UnsupportedInstruction {
                description: "InsertField not yet implemented".into(),
            });
        }

        Instruction::ExtractElement { .. } => {
            return Err(CraneliftError::UnsupportedInstruction {
                description: "ExtractElement not yet implemented".into(),
            });
        }

        Instruction::InsertElement { .. } => {
            return Err(CraneliftError::UnsupportedInstruction {
                description: "InsertElement not yet implemented".into(),
            });
        }

        Instruction::Call { .. } => {
            return Err(CraneliftError::UnsupportedInstruction {
                description: "Call not yet implemented".into(),
            });
        }

        Instruction::CallIndirect { .. } => {
            return Err(CraneliftError::UnsupportedInstruction {
                description: "CallIndirect not yet implemented".into(),
            });
        }
    }

    Ok(())
}

/// Lower a constant to Cranelift IR.
fn lower_constant(
    constant: &Constant,
    builder: &mut FunctionBuilder<'_>,
) -> Result<CraneliftValue, CraneliftError> {
    match constant {
        Constant::Boolean { value } => {
            let int_value = if *value { 1i64 } else { 0i64 };
            Ok(builder
                .ins()
                .iconst(cranelift_codegen::ir::types::I8, int_value))
        }

        Constant::Int {
            value,
            width,
            is_signed: _,
        } => {
            let ty = match width {
                8 => cranelift_codegen::ir::types::I8,
                16 => cranelift_codegen::ir::types::I16,
                32 => cranelift_codegen::ir::types::I32,
                64 => cranelift_codegen::ir::types::I64,
                _ => {
                    return Err(CraneliftError::UnsupportedType {
                        description: format!("integer width {width}"),
                    });
                }
            };
            Ok(builder.ins().iconst(ty, *value))
        }

        Constant::UInt { value, width } => {
            let ty = match width {
                8 => cranelift_codegen::ir::types::I8,
                16 => cranelift_codegen::ir::types::I16,
                32 => cranelift_codegen::ir::types::I32,
                64 => cranelift_codegen::ir::types::I64,
                _ => {
                    return Err(CraneliftError::UnsupportedType {
                        description: format!("unsigned integer width {width}"),
                    });
                }
            };
            Ok(builder.ins().iconst(ty, *value as i64))
        }

        Constant::Float { bits, width } => {
            match width {
                32 => Ok(builder.ins().f32const(
                    cranelift_codegen::ir::immediates::Ieee32::with_bits(*bits as u32),
                )),
                64 => Ok(builder
                    .ins()
                    .f64const(cranelift_codegen::ir::immediates::Ieee64::with_bits(*bits))),
                _ => Err(CraneliftError::UnsupportedType {
                    description: format!("float width {width}"),
                }),
            }
        }
    }
}

/// Lower a binary operation to Cranelift IR.
fn lower_binary_op(
    operator: BinaryOperator,
    left: CraneliftValue,
    right: CraneliftValue,
    builder: &mut FunctionBuilder<'_>,
) -> Result<CraneliftValue, CraneliftError> {
    let ins = builder.ins();

    let result = match operator {
        // arithmetic
        BinaryOperator::Add => ins.iadd(left, right),
        BinaryOperator::Subtract => ins.isub(left, right),
        BinaryOperator::Multiply => ins.imul(left, right),
        BinaryOperator::SignedDivide => ins.sdiv(left, right),
        BinaryOperator::UnsignedDivide => ins.udiv(left, right),
        BinaryOperator::SignedRemainder => ins.srem(left, right),
        BinaryOperator::UnsignedRemainder => ins.urem(left, right),

        // floating point
        BinaryOperator::FloatAdd => ins.fadd(left, right),
        BinaryOperator::FloatSubtract => ins.fsub(left, right),
        BinaryOperator::FloatMultiply => ins.fmul(left, right),
        BinaryOperator::FloatDivide => ins.fdiv(left, right),

        // bitwise
        BinaryOperator::And => ins.band(left, right),
        BinaryOperator::Or => ins.bor(left, right),
        BinaryOperator::Xor => ins.bxor(left, right),
        BinaryOperator::ShiftLeft => ins.ishl(left, right),
        BinaryOperator::ArithmeticShiftRight => ins.sshr(left, right),
        BinaryOperator::LogicalShiftRight => ins.ushr(left, right),

        // integer comparison
        BinaryOperator::Equal => {
            ins.icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, left, right)
        }
        BinaryOperator::NotEqual => ins.icmp(
            cranelift_codegen::ir::condcodes::IntCC::NotEqual,
            left,
            right,
        ),
        BinaryOperator::SignedLessThan => ins.icmp(
            cranelift_codegen::ir::condcodes::IntCC::SignedLessThan,
            left,
            right,
        ),
        BinaryOperator::SignedLessEqual => ins.icmp(
            cranelift_codegen::ir::condcodes::IntCC::SignedLessThanOrEqual,
            left,
            right,
        ),
        BinaryOperator::SignedGreaterThan => ins.icmp(
            cranelift_codegen::ir::condcodes::IntCC::SignedGreaterThan,
            left,
            right,
        ),
        BinaryOperator::SignedGreaterEqual => ins.icmp(
            cranelift_codegen::ir::condcodes::IntCC::SignedGreaterThanOrEqual,
            left,
            right,
        ),
        BinaryOperator::UnsignedLessThan => ins.icmp(
            cranelift_codegen::ir::condcodes::IntCC::UnsignedLessThan,
            left,
            right,
        ),
        BinaryOperator::UnsignedLessEqual => ins.icmp(
            cranelift_codegen::ir::condcodes::IntCC::UnsignedLessThanOrEqual,
            left,
            right,
        ),
        BinaryOperator::UnsignedGreaterThan => ins.icmp(
            cranelift_codegen::ir::condcodes::IntCC::UnsignedGreaterThan,
            left,
            right,
        ),
        BinaryOperator::UnsignedGreaterEqual => ins.icmp(
            cranelift_codegen::ir::condcodes::IntCC::UnsignedGreaterThanOrEqual,
            left,
            right,
        ),

        // float comparison
        BinaryOperator::FloatEqual => ins.fcmp(
            cranelift_codegen::ir::condcodes::FloatCC::Equal,
            left,
            right,
        ),
        BinaryOperator::FloatNotEqual => ins.fcmp(
            cranelift_codegen::ir::condcodes::FloatCC::NotEqual,
            left,
            right,
        ),
        BinaryOperator::FloatLessThan => ins.fcmp(
            cranelift_codegen::ir::condcodes::FloatCC::LessThan,
            left,
            right,
        ),
        BinaryOperator::FloatLessEqual => ins.fcmp(
            cranelift_codegen::ir::condcodes::FloatCC::LessThanOrEqual,
            left,
            right,
        ),
        BinaryOperator::FloatGreaterThan => ins.fcmp(
            cranelift_codegen::ir::condcodes::FloatCC::GreaterThan,
            left,
            right,
        ),
        BinaryOperator::FloatGreaterEqual => ins.fcmp(
            cranelift_codegen::ir::condcodes::FloatCC::GreaterThanOrEqual,
            left,
            right,
        ),
    };

    Ok(result)
}

/// Lower a unary operation to Cranelift IR.
fn lower_unary_op(
    operator: UnaryOperator,
    argument: CraneliftValue,
    builder: &mut FunctionBuilder<'_>,
) -> Result<CraneliftValue, CraneliftError> {
    let ins = builder.ins();

    let result = match operator {
        UnaryOperator::Negate => ins.ineg(argument),
        UnaryOperator::FloatNegate => ins.fneg(argument),
        UnaryOperator::Not => ins.bnot(argument),
    };

    Ok(result)
}

/// Lower a cast operation to Cranelift IR.
fn lower_cast(
    kind: CastKind,
    argument: CraneliftValue,
    to_type: cranelift_codegen::ir::Type,
    builder: &mut FunctionBuilder<'_>,
) -> Result<CraneliftValue, CraneliftError> {
    let ins = builder.ins();

    let result = match kind {
        CastKind::Bitcast => ins.bitcast(to_type, cranelift_codegen::ir::MemFlags::new(), argument),
        CastKind::Truncate => ins.ireduce(to_type, argument),
        CastKind::ZeroExtend => ins.uextend(to_type, argument),
        CastKind::SignExtend => ins.sextend(to_type, argument),
        CastKind::FloatToSignedInt => ins.fcvt_to_sint(to_type, argument),
        CastKind::FloatToUnsignedInt => ins.fcvt_to_uint(to_type, argument),
        CastKind::SignedIntToFloat => ins.fcvt_from_sint(to_type, argument),
        CastKind::UnsignedIntToFloat => ins.fcvt_from_uint(to_type, argument),
        CastKind::FloatTruncate => ins.fdemote(to_type, argument),
        CastKind::FloatExtend => ins.fpromote(to_type, argument),
        CastKind::PointerToInt => {
            // pointer is already an integer in Cranelift
            argument
        }
        CastKind::IntToPointer => {
            // pointer is already an integer in Cranelift
            argument
        }
    };

    Ok(result)
}
