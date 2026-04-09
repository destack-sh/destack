use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    Block, CheckConstraint, FormatMirNode, LocalNodeId, MirFormatContext, MirFormatter, Terminator,
    TrapKind, Value,
};

impl<'a> FormatMirNode<'a, Block> for Block {
    fn format_node(
        &self,
        id: LocalNodeId<Block>,
        f: &mut MirFormatter<'a, '_>,
    ) -> FormatResult<()> {
        // block label
        let block_index = f.context().block_index(id);
        write!(f, [text(&format!("b{block_index}"))])?;

        // block parameters
        if !self.parameters.is_empty() {
            write!(f, [token("(")])?;
            for (i, param) in self.parameters.iter().enumerate() {
                if i > 0 {
                    write!(f, [token(","), space()])?;
                }
                write!(f, [&param.value, token(":"), space(), param.ty])?;
            }
            write!(f, [token(")")])?;
        }

        write!(f, [token(":"), hard_line_break()])?;

        // instructions (indented)
        let instructions = self.instructions.clone();
        let terminator = self.terminator.clone();

        write!(
            f,
            [block_indent(&format_with(
                |f: &mut Formatter<'_, MirFormatContext<'a>>| {
                    let tree = f.context().tree;

                    // instructions
                    for inst_id in &instructions {
                        let inst = tree.get(*inst_id);
                        inst.format_node(*inst_id, f)?;
                        write!(f, [hard_line_break()])?;
                    }

                    // terminator
                    format_terminator(&terminator, f)?;

                    Ok(())
                }
            ))]
        )
    }
}

fn format_terminator<'a>(term: &Terminator, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    match term {
        Terminator::Return { value } => {
            write!(f, [token("return")])?;
            if let Some(v) = value {
                write!(f, [space(), v])?;
            }
            Ok(())
        }

        Terminator::Jump { target, arguments } => {
            let block_index = f.context().block_index(*target);
            write!(
                f,
                [token("jump"), space(), text(&format!("b{block_index}"))]
            )?;
            if !arguments.is_empty() {
                format_value_list(arguments, f)?;
            }
            Ok(())
        }

        Terminator::Branch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
        } => {
            let then_index = f.context().block_index(*then_target);
            let else_index = f.context().block_index(*else_target);
            write!(
                f,
                [
                    token("branch"),
                    space(),
                    condition,
                    token(","),
                    space(),
                    text(&format!("b{then_index}"))
                ]
            )?;
            if !then_arguments.is_empty() {
                format_value_list(then_arguments, f)?;
            }
            write!(f, [token(","), space(), text(&format!("b{else_index}"))])?;
            if !else_arguments.is_empty() {
                format_value_list(else_arguments, f)?;
            }
            Ok(())
        }

        Terminator::Check {
            constraint,
            success,
            failure,
        } => {
            let success_index = f.context().block_index(success.target);
            let failure_index = f.context().block_index(failure.target);
            write!(f, [token("check"), space()])?;
            format_check_constraint(constraint, f)?;
            write!(
                f,
                [
                    space(),
                    token("->"),
                    space(),
                    text(&format!("b{success_index}"))
                ]
            )?;
            if !success.arguments.is_empty() {
                format_value_list(&success.arguments, f)?;
            }
            write!(f, [token(","), space(), text(&format!("b{failure_index}"))])?;
            if !failure.arguments.is_empty() {
                format_value_list(&failure.arguments, f)?;
            }
            Ok(())
        }

        Terminator::Switch {
            value,
            default,
            default_arguments,
            cases,
        } => {
            let default_index = f.context().block_index(*default);
            write!(
                f,
                [
                    token("switch"),
                    space(),
                    value,
                    token(","),
                    space(),
                    text(&format!("b{default_index}"))
                ]
            )?;
            if !default_arguments.is_empty() {
                format_value_list(default_arguments, f)?;
            }
            for case in cases {
                let case_index = f.context().block_index(case.target);
                write!(
                    f,
                    [
                        token(","),
                        space(),
                        text(&case.value.to_string()),
                        space(),
                        token("=>"),
                        space(),
                        text(&format!("b{case_index}"))
                    ]
                )?;
                if !case.arguments.is_empty() {
                    format_value_list(&case.arguments, f)?;
                }
            }
            Ok(())
        }

        Terminator::Unreachable => {
            write!(f, [token("unreachable")])
        }

        Terminator::Yield {
            value,
            resume,
            resume_arguments,
        } => {
            let resume_index = f.context().block_index(*resume);
            write!(
                f,
                [
                    token("yield"),
                    space(),
                    value,
                    token(","),
                    space(),
                    text(&format!("b{resume_index}"))
                ]
            )?;
            if !resume_arguments.is_empty() {
                format_value_list(resume_arguments, f)?;
            }
            Ok(())
        }

        Terminator::Invoke {
            function,
            arguments,
            signature,
            normal_target,
            normal_arguments,
            unwind_target,
            unwind_arguments,
        } => {
            let tree = f.context().tree;
            let strings = f.context().strings;
            let func = tree.get(*function);
            let name = strings.get(func.name);
            write!(f, [token("invoke"), space(), text(name)])?;
            format_value_list(arguments, f)?;
            format_call_signature_suffix(*signature, f)?;
            format_call_continuations(
                *normal_target,
                normal_arguments,
                *unwind_target,
                unwind_arguments,
                f,
            )
        }

        Terminator::InvokeIndirect {
            callee,
            arguments,
            signature,
            normal_target,
            normal_arguments,
            unwind_target,
            unwind_arguments,
            ..
        } => {
            write!(f, [token("invoke.indirect"), space(), callee])?;
            format_value_list(arguments, f)?;
            format_call_signature_suffix(*signature, f)?;
            format_call_continuations(
                *normal_target,
                normal_arguments,
                *unwind_target,
                unwind_arguments,
                f,
            )
        }

        Terminator::InvokeVirtual {
            receiver,
            arguments,
            declaring_type,
            slot_id,
            signature,
            normal_target,
            normal_arguments,
            unwind_target,
            unwind_arguments,
        } => {
            write!(
                f,
                [
                    token("invoke.virtual"),
                    space(),
                    receiver,
                    token(","),
                    space(),
                    declaring_type,
                    token(","),
                    space(),
                    text(&slot_id.0.to_string())
                ]
            )?;
            format_value_list(arguments, f)?;
            format_call_signature_suffix(*signature, f)?;
            format_call_continuations(
                *normal_target,
                normal_arguments,
                *unwind_target,
                unwind_arguments,
                f,
            )
        }

        Terminator::InvokeInterface {
            receiver,
            arguments,
            declaring_type,
            slot_id,
            signature,
            normal_target,
            normal_arguments,
            unwind_target,
            unwind_arguments,
        } => {
            write!(
                f,
                [
                    token("invoke.interface"),
                    space(),
                    receiver,
                    token(","),
                    space(),
                    declaring_type,
                    token(","),
                    space(),
                    text(&slot_id.0.to_string())
                ]
            )?;
            format_value_list(arguments, f)?;
            format_call_signature_suffix(*signature, f)?;
            format_call_continuations(
                *normal_target,
                normal_arguments,
                *unwind_target,
                unwind_arguments,
                f,
            )
        }

        Terminator::Throw { value } => {
            write!(f, [token("throw"), space(), value])
        }

        Terminator::Trap { kind, payload } => {
            let trap_opcode = match kind {
                TrapKind::Abort => "trap.abort",
                TrapKind::Panic => "trap.panic",
            };

            write!(f, [token(trap_opcode)])?;

            if let Some(payload) = payload {
                write!(f, [space(), payload])?;
            }

            Ok(())
        }

        Terminator::TailCall {
            function,
            arguments,
            signature,
        } => {
            let tree = f.context().tree;
            let strings = f.context().strings;
            let func = tree.get(*function);
            let name = strings.get(func.name);
            write!(f, [token("tailCall"), space(), text(name)])?;
            format_value_list(arguments, f)?;
            format_call_signature_suffix(*signature, f)
        }

        Terminator::TailCallIndirect {
            callee,
            arguments,
            signature,
            ..
        } => {
            write!(f, [token("tailCall.indirect"), space(), callee])?;
            format_value_list(arguments, f)?;
            format_call_signature_suffix(*signature, f)
        }

        Terminator::TailCallVirtual {
            receiver,
            arguments,
            declaring_type,
            slot_id,
            signature,
            ..
        } => {
            write!(
                f,
                [
                    token("tailCall.virtual"),
                    space(),
                    receiver,
                    token(","),
                    space(),
                    declaring_type,
                    token(","),
                    space(),
                    text(&slot_id.0.to_string())
                ]
            )?;
            format_value_list(arguments, f)?;
            format_call_signature_suffix(*signature, f)
        }

        Terminator::TailCallInterface {
            receiver,
            arguments,
            declaring_type,
            slot_id,
            signature,
            ..
        } => {
            write!(
                f,
                [
                    token("tailCall.interface"),
                    space(),
                    receiver,
                    token(","),
                    space(),
                    declaring_type,
                    token(","),
                    space(),
                    text(&slot_id.0.to_string())
                ]
            )?;
            format_value_list(arguments, f)?;
            format_call_signature_suffix(*signature, f)
        }
    }
}

fn format_call_continuations<'a>(
    normal_target: LocalNodeId<Block>,
    normal_arguments: &[Value],
    unwind_target: LocalNodeId<Block>,
    unwind_arguments: &[Value],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    let normal_index = f.context().block_index(normal_target);
    let unwind_index = f.context().block_index(unwind_target);

    write!(
        f,
        [
            space(),
            token("->"),
            space(),
            text(&format!("b{normal_index}"))
        ]
    )?;
    if !normal_arguments.is_empty() {
        format_value_list(normal_arguments, f)?;
    }

    write!(
        f,
        [
            token(","),
            space(),
            token("catch"),
            space(),
            text(&format!("b{unwind_index}"))
        ]
    )?;
    if !unwind_arguments.is_empty() {
        format_value_list(unwind_arguments, f)?;
    }

    Ok(())
}

/// Format a check constraint.
fn format_check_constraint<'a>(
    constraint: &CheckConstraint,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    // write the check kind header
    match constraint {
        CheckConstraint::Bounds {
            index,
            length,
            collection,
            is_signed,
        } => {
            let prefix = if *is_signed { "bounds.s" } else { "bounds.u" };
            write!(
                f,
                [
                    token(prefix),
                    space(),
                    index,
                    token(","),
                    space(),
                    length,
                    token(","),
                    space(),
                    collection
                ]
            )
        }
        CheckConstraint::Null { value } => {
            write!(f, [token("null"), space(), value])
        }
        CheckConstraint::DivZero { divisor } => {
            write!(f, [token("zeroDivisor"), space(), divisor])
        }
        CheckConstraint::ShiftRange {
            value,
            bit_width,
            is_signed,
        } => {
            let prefix = if *is_signed {
                "shiftRange.s"
            } else {
                "shiftRange.u"
            };
            write!(
                f,
                [
                    token(prefix),
                    space(),
                    value,
                    token(","),
                    space(),
                    text(&bit_width.to_string())
                ]
            )
        }
        CheckConstraint::Narrow {
            value,
            to_width,
            is_signed,
        } => {
            let prefix = if *is_signed {
                "narrowRange.s"
            } else {
                "narrowRange.u"
            };
            write!(
                f,
                [
                    token(prefix),
                    space(),
                    value,
                    token(","),
                    space(),
                    text(&to_width.to_string())
                ]
            )
        }
        CheckConstraint::Overflow {
            operator,
            left,
            right,
            is_signed,
        } => {
            let suffix = if *is_signed { "s" } else { "u" };
            let name = format!("{}.overflow.{suffix}", overflow_check_family(*operator));
            write!(f, [text(&name), space(), left, token(","), space(), right])
        }
        CheckConstraint::Type { value, expected } => write!(
            f,
            [
                token("dynamicType"),
                space(),
                value,
                token(","),
                space(),
                expected
            ]
        ),
        CheckConstraint::Union { value, expected } => write!(
            f,
            [
                token("unionTag"),
                space(),
                value,
                token(","),
                space(),
                text(&expected.to_string())
            ]
        ),
        CheckConstraint::ReceiverType { receiver, expected } => write!(
            f,
            [
                token("receiverType"),
                space(),
                receiver,
                token(","),
                space(),
                expected
            ]
        ),
        CheckConstraint::Implements { receiver, expected } => write!(
            f,
            [
                token("interfaceConformance"),
                space(),
                receiver,
                token(","),
                space(),
                expected
            ]
        ),
    }
}

/// Return the canonical operator family used in overflow checks.
fn overflow_check_family(operator: crate::BinaryOperator) -> &'static str {
    match operator {
        crate::BinaryOperator::Add => "int.add",
        crate::BinaryOperator::Subtract => "int.sub",
        crate::BinaryOperator::Multiply => "int.mul",
        crate::BinaryOperator::SignedDivide | crate::BinaryOperator::UnsignedDivide => "int.div",
        crate::BinaryOperator::SignedRemainder | crate::BinaryOperator::UnsignedRemainder => {
            "int.rem"
        }
        _ => panic!("unsupported overflow check operator: {operator:?}"),
    }
}

/// Format a parenthesized, comma-separated list of values.
fn format_value_list<'a>(values: &[Value], f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    write!(f, [token("(")])?;
    for (i, val) in values.iter().enumerate() {
        if i > 0 {
            write!(f, [token(","), space()])?;
        }
        write!(f, [val])?;
    }
    write!(f, [token(")")])
}

fn format_call_signature_suffix<'a>(
    signature: LocalNodeId<crate::Type>,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token(":"), space()])?;

    match f.context().tree.get(signature) {
        crate::Type::FunctionPointer { parameters, result } => {
            write!(f, [token("(")])?;
            for (index, parameter) in parameters.iter().enumerate() {
                if index > 0 {
                    write!(f, [token(","), space()])?;
                }
                write!(f, [*parameter])?;
            }
            write!(f, [token(")"), space(), token("->"), space(), *result])
        }
        _ => write!(f, [signature]),
    }
}
