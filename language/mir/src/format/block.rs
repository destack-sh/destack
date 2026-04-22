use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    Block, BlockTarget, CheckConstraint, FormatMirNode, LocalNodeId, MirFormatContext,
    MirFormatter, Terminator, TrapKind, TypeReference, ValueReference, write_comments_after,
    write_inline_comment_after, write_node_leading_comments,
};

impl<'a> FormatMirNode<'a, Block> for Block {
    fn format_node(
        &self,
        id: LocalNodeId<Block>,
        f: &mut MirFormatter<'a, '_>,
    ) -> FormatResult<()> {
        // block label
        let block_name = f.context().block_name(id);
        write!(f, [text(&block_name)])?;

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
        let terminator_id = self.terminator;

        write!(
            f,
            [block_indent(&format_with(
                |f: &mut Formatter<'_, MirFormatContext<'a>>| {
                    let tree = f.context().tree;
                    let block_span = tree.get_span(id);
                    let terminator_span = tree.get_span(terminator_id);

                    // instructions
                    for (index, inst_id) in instructions.iter().enumerate() {
                        write_node_leading_comments(tree, *inst_id, f)?;
                        let inst = tree.get(*inst_id);
                        inst.format_node(*inst_id, f)?;
                        let instruction_span = tree.get_span(*inst_id);
                        let next_boundary = instructions
                            .get(index + 1)
                            .and_then(|next_id| tree.get_span(*next_id))
                            .map(|span| span.start)
                            .or_else(|| terminator_span.map(|span| span.start));

                        if let (Some(instruction_span), Some(next_boundary)) =
                            (instruction_span, next_boundary)
                        {
                            write_inline_comment_after(
                                tree,
                                instruction_span.end,
                                next_boundary,
                                f,
                            )?;
                        }

                        write!(f, [hard_line_break()])?;
                    }

                    // terminator
                    let terminator = tree.get(terminator_id);
                    write_node_leading_comments(tree, terminator_id, f)?;
                    terminator.format_node(terminator_id, f)?;

                    if let (Some(terminator_span), Some(block_span)) = (terminator_span, block_span)
                    {
                        write_comments_after(tree, terminator_span.end, block_span.end, f)?;
                    }

                    Ok(())
                }
            ))]
        )
    }
}

impl<'a> FormatMirNode<'a, Terminator> for Terminator {
    fn format_node(
        &self,
        _id: LocalNodeId<Terminator>,
        f: &mut MirFormatter<'a, '_>,
    ) -> FormatResult<()> {
        format_terminator(self, f)
    }
}

fn format_terminator<'a>(term: &Terminator, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    match term {
        Terminator::Error => Err(FormatError::SyntaxError {
            message: "cannot format recovered MIR terminator",
        }),

        Terminator::Return { value } => {
            write!(f, [token("return")])?;
            if let Some(v) = value {
                write!(f, [space(), v])?;
            }
            Ok(())
        }

        Terminator::Jump { target } => {
            write!(f, [token("jump"), space()])?;
            format_block_target(target, f)
        }

        Terminator::Branch {
            condition,
            then_target,
            else_target,
        } => {
            write!(
                f,
                [token("branch"), space(), condition, token(","), space()]
            )?;
            format_block_target(then_target, f)?;
            write!(f, [token(","), space()])?;
            format_block_target(else_target, f)?;
            Ok(())
        }

        Terminator::Check {
            constraint,
            success,
            failure,
        } => {
            write!(f, [token("check"), space()])?;
            format_check_constraint(constraint, f)?;
            write!(f, [space(), token("->"), space()])?;
            format_block_target(success, f)?;
            write!(f, [token(","), space()])?;
            format_block_target(failure, f)?;
            Ok(())
        }

        Terminator::Switch {
            value,
            default,
            cases,
        } => {
            write!(f, [token("switch"), space(), value, token(","), space()])?;
            format_block_target(default, f)?;
            for case in cases {
                write!(
                    f,
                    [
                        token(","),
                        space(),
                        case.value,
                        space(),
                        token("=>"),
                        space()
                    ]
                )?;
                format_block_target(&case.target, f)?;
            }
            Ok(())
        }

        Terminator::Unreachable => {
            write!(f, [token("unreachable")])
        }

        Terminator::Yield { value, resume } => {
            write!(f, [token("yield"), space(), value, token(","), space()])?;
            format_block_target(resume, f)?;
            Ok(())
        }

        Terminator::Invoke {
            function,
            call,
            normal_target,
            unwind_target,
        } => {
            write!(f, [token("invoke"), space(), function])?;
            format_value_list(&call.arguments, f)?;
            format_call_signature_suffix(call.signature, f)?;
            format_call_continuations(normal_target, unwind_target, f)
        }

        Terminator::InvokeIndirect {
            callee,
            call,
            normal_target,
            unwind_target,
            ..
        } => {
            write!(f, [token("invoke.indirect"), space(), callee])?;
            format_value_list(&call.arguments, f)?;
            format_call_signature_suffix(call.signature, f)?;
            format_call_continuations(normal_target, unwind_target, f)
        }

        Terminator::InvokeVirtual {
            receiver,
            call,
            declaring_type,
            slot_id,
            normal_target,
            unwind_target,
            ..
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
            format_value_list(&call.arguments, f)?;
            format_call_signature_suffix(call.signature, f)?;
            format_call_continuations(normal_target, unwind_target, f)
        }

        Terminator::InvokeInterface {
            receiver,
            call,
            declaring_type,
            slot_id,
            normal_target,
            unwind_target,
            ..
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
            format_value_list(&call.arguments, f)?;
            format_call_signature_suffix(call.signature, f)?;
            format_call_continuations(normal_target, unwind_target, f)
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

        Terminator::TailCall { function, call } => {
            write!(f, [token("tailCall"), space(), function])?;
            format_value_list(&call.arguments, f)?;
            format_call_signature_suffix(call.signature, f)
        }

        Terminator::TailCallIndirect { callee, call, .. } => {
            write!(f, [token("tailCall.indirect"), space(), callee])?;
            format_value_list(&call.arguments, f)?;
            format_call_signature_suffix(call.signature, f)
        }

        Terminator::TailCallVirtual {
            receiver,
            call,
            declaring_type,
            slot_id,
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
            format_value_list(&call.arguments, f)?;
            format_call_signature_suffix(call.signature, f)
        }

        Terminator::TailCallInterface {
            receiver,
            call,
            declaring_type,
            slot_id,
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
            format_value_list(&call.arguments, f)?;
            format_call_signature_suffix(call.signature, f)
        }
    }
}

fn format_call_continuations<'a>(
    normal_target: &BlockTarget,
    unwind_target: &BlockTarget,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [space(), token("->"), space()])?;
    format_block_target(normal_target, f)?;
    write!(f, [token(","), space(), token("catch"), space()])?;
    format_block_target(unwind_target, f)?;

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
fn format_value_list<'a>(
    values: &[ValueReference],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
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
    signature: TypeReference,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token(":"), space()])?;

    match signature {
        TypeReference::Type(signature) => match f.context().tree.get(signature) {
            crate::Type::FunctionSignature { parameters, result } => {
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
        },
        TypeReference::Missing | TypeReference::Error => write!(f, [signature]),
    }
}

fn format_block_target<'a>(target: &BlockTarget, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    write!(f, [target.block])?;

    if !target.arguments.is_empty() {
        format_value_list(&target.arguments, f)?;
    }

    Ok(())
}
