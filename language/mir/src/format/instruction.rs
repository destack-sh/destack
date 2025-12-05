//! Instruction formatting.

use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    BinaryOperator, CastKind, FormatMirNode, FunctionReference, Instruction, LocalNodeId,
    MirFormatter, UnaryOperator,
};

impl<'a> FormatMirNode<'a, Instruction> for Instruction {
    fn format_node(
        &self,
        _id: LocalNodeId<Instruction>,
        f: &mut MirFormatter<'a, '_>,
    ) -> FormatResult<()> {
        match self {
            Instruction::Constant { destination, value } => {
                write!(f, [destination, token(" = iconst "), value])
            }

            Instruction::Binary {
                destination,
                operator,
                left,
                right,
            } => {
                write!(
                    f,
                    [
                        destination,
                        token(" = "),
                        text(binary_op_name(*operator)),
                        token(" "),
                        left,
                        token(", "),
                        right
                    ]
                )
            }

            Instruction::Unary {
                destination,
                operator,
                argument,
            } => {
                write!(
                    f,
                    [
                        destination,
                        token(" = "),
                        text(unary_op_name(*operator)),
                        token(" "),
                        argument
                    ]
                )
            }

            Instruction::Cast {
                destination,
                kind,
                argument,
                to_type,
            } => {
                write!(
                    f,
                    [
                        destination,
                        token(" = "),
                        text(cast_kind_name(*kind)),
                        token(" "),
                        argument,
                        token(" -> "),
                        to_type
                    ]
                )
            }

            Instruction::LocalGet { destination, local } => {
                write!(
                    f,
                    [
                        destination,
                        token(" = load_local "),
                        text(&format!("local{}", local.id))
                    ]
                )
            }

            Instruction::LocalSet { local, value } => {
                write!(
                    f,
                    [
                        token("store_local "),
                        text(&format!("local{}", local.id)),
                        token(", "),
                        value
                    ]
                )
            }

            Instruction::Load {
                destination,
                pointer,
            } => {
                write!(f, [destination, token(" = load "), pointer])
            }

            Instruction::Store { pointer, value } => {
                write!(f, [token("store "), pointer, token(", "), value])
            }

            Instruction::ExtractField {
                destination,
                aggregate,
                index,
            } => {
                write!(
                    f,
                    [
                        destination,
                        token(" = extract_field "),
                        aggregate,
                        token(", "),
                        text(&index.to_string())
                    ]
                )
            }

            Instruction::InsertField {
                destination,
                aggregate,
                index,
                value,
            } => {
                write!(
                    f,
                    [
                        destination,
                        token(" = insert_field "),
                        aggregate,
                        token(", "),
                        text(&index.to_string()),
                        token(", "),
                        value
                    ]
                )
            }

            Instruction::ExtractElement {
                destination,
                array,
                index,
            } => {
                write!(
                    f,
                    [
                        destination,
                        token(" = extract_element "),
                        array,
                        token(", "),
                        index
                    ]
                )
            }

            Instruction::InsertElement {
                destination,
                array,
                index,
                value,
            } => {
                write!(
                    f,
                    [
                        destination,
                        token(" = insert_element "),
                        array,
                        token(", "),
                        index,
                        token(", "),
                        value
                    ]
                )
            }

            Instruction::Call {
                destination,
                function,
                arguments,
            } => {
                if let Some(dst) = destination {
                    write!(f, [dst, token(" = ")])?;
                }
                write!(f, [token("call ")])?;
                match function {
                    FunctionReference::Local(func_id) => {
                        write!(f, [text(&format!("@func{}", func_id.id))])?;
                    }
                    FunctionReference::Global(sym_id) => {
                        write!(f, [text(&format!("@global({sym_id:?})"))])?;
                    }
                }
                write!(f, [token("(")])?;
                for (i, arg) in arguments.iter().enumerate() {
                    if i > 0 {
                        write!(f, [token(", ")])?;
                    }
                    write!(f, [arg])?;
                }
                write!(f, [token(")")])
            }

            Instruction::CallIndirect {
                destination,
                callee,
                arguments,
            } => {
                if let Some(dst) = destination {
                    write!(f, [dst, token(" = ")])?;
                }
                write!(f, [token("call_indirect "), callee, token("(")])?;
                for (i, arg) in arguments.iter().enumerate() {
                    if i > 0 {
                        write!(f, [token(", ")])?;
                    }
                    write!(f, [arg])?;
                }
                write!(f, [token(")")])
            }
        }
    }
}

fn binary_op_name(op: BinaryOperator) -> &'static str {
    match op {
        BinaryOperator::Add => "iadd",
        BinaryOperator::Subtract => "isub",
        BinaryOperator::Multiply => "imul",
        BinaryOperator::SignedDivide => "sdiv",
        BinaryOperator::UnsignedDivide => "udiv",
        BinaryOperator::SignedRemainder => "srem",
        BinaryOperator::UnsignedRemainder => "urem",
        BinaryOperator::FloatAdd => "fadd",
        BinaryOperator::FloatSubtract => "fsub",
        BinaryOperator::FloatMultiply => "fmul",
        BinaryOperator::FloatDivide => "fdiv",
        BinaryOperator::And => "band",
        BinaryOperator::Or => "bor",
        BinaryOperator::Xor => "bxor",
        BinaryOperator::ShiftLeft => "ishl",
        BinaryOperator::ArithmeticShiftRight => "sshr",
        BinaryOperator::LogicalShiftRight => "ushr",
        BinaryOperator::Equal => "icmp eq",
        BinaryOperator::NotEqual => "icmp ne",
        BinaryOperator::SignedLessThan => "icmp slt",
        BinaryOperator::SignedLessEqual => "icmp sle",
        BinaryOperator::SignedGreaterThan => "icmp sgt",
        BinaryOperator::SignedGreaterEqual => "icmp sge",
        BinaryOperator::UnsignedLessThan => "icmp ult",
        BinaryOperator::UnsignedLessEqual => "icmp ule",
        BinaryOperator::UnsignedGreaterThan => "icmp ugt",
        BinaryOperator::UnsignedGreaterEqual => "icmp uge",
        BinaryOperator::FloatEqual => "fcmp eq",
        BinaryOperator::FloatNotEqual => "fcmp ne",
        BinaryOperator::FloatLessThan => "fcmp lt",
        BinaryOperator::FloatLessEqual => "fcmp le",
        BinaryOperator::FloatGreaterThan => "fcmp gt",
        BinaryOperator::FloatGreaterEqual => "fcmp ge",
    }
}

fn unary_op_name(op: UnaryOperator) -> &'static str {
    match op {
        UnaryOperator::Negate => "ineg",
        UnaryOperator::FloatNegate => "fneg",
        UnaryOperator::Not => "bnot",
    }
}

fn cast_kind_name(kind: CastKind) -> &'static str {
    match kind {
        CastKind::Bitcast => "bitcast",
        CastKind::Truncate => "trunc",
        CastKind::ZeroExtend => "uextend",
        CastKind::SignExtend => "sextend",
        CastKind::FloatToSignedInt => "fcvt_to_sint",
        CastKind::FloatToUnsignedInt => "fcvt_to_uint",
        CastKind::SignedIntToFloat => "scvt_to_float",
        CastKind::UnsignedIntToFloat => "ucvt_to_float",
        CastKind::FloatTruncate => "fnarrow",
        CastKind::FloatExtend => "fwiden",
        CastKind::PointerToInt => "ptr_to_int",
        CastKind::IntToPointer => "int_to_ptr",
    }
}
