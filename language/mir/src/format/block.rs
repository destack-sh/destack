use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use super::call::format_call;
use super::value::format_block_id;

use crate::{
    BinaryOperator, Block, BlockTarget, CheckConstraint, FormatNode, LocalNodeId, Terminator,
    Value, ValueSlice, Writer, write_comments_after, write_inline_comment_after,
    write_node_leading_comments,
};

impl FormatNode for Block {
    fn format_node<'a>(&self, id: LocalNodeId<Block>, f: &mut Writer<'a, '_>) -> FormatResult<()> {
        // block label
        let block_name = f.context().block_name(id)?;
        write!(f, [copied_text(&block_name)])?;

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
            [block_indent(&format_with(|f: &mut Writer<'a, '_>| {
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
                        write_inline_comment_after(tree, instruction_span.end, next_boundary, f)?;
                    }

                    write!(f, [hard_line_break()])?;
                }

                // terminator
                let terminator = tree.get(terminator_id);
                write_node_leading_comments(tree, terminator_id, f)?;
                terminator.format_node(terminator_id, f)?;

                if let (Some(terminator_span), Some(block_span)) = (terminator_span, block_span) {
                    write_comments_after(tree, terminator_span.end, block_span.end, f)?;
                }

                Ok(())
            }))]
        )
    }
}

impl FormatNode for Terminator {
    fn format_node<'a>(
        &self,
        _id: LocalNodeId<Terminator>,
        f: &mut Writer<'a, '_>,
    ) -> FormatResult<()> {
        format_terminator(self, f)
    }
}

fn format_terminator<'a>(term: &Terminator, f: &mut Writer<'a, '_>) -> FormatResult<()> {
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
            write!(f, [token("branch"), space(), condition])?;
            write!(f, [space(), token("=>"), space()])?;
            format_block_target(then_target, f)?;
            write!(f, [space(), token("|"), space()])?;
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
            write!(f, [space(), token("=>"), space()])?;
            format_block_target(success, f)?;
            write!(f, [space(), token("|"), space()])?;
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
            let cases = f.context().tree.get_switch_cases(*cases);
            for case in cases {
                write!(
                    f,
                    [
                        token(","),
                        space(),
                        copied_text(&case.value.to_string()),
                        space(),
                        token("=>"),
                        space()
                    ]
                )?;
                format_block_target(&case.target, f)?;
            }
            Ok(())
        }

        Terminator::VariantSwitch {
            value,
            default,
            cases,
        } => {
            write!(f, [token("variant.switch"), space(), value])?;
            let cases = f.context().tree.get_switch_cases(*cases);
            for case in cases {
                write!(
                    f,
                    [
                        token(","),
                        space(),
                        copied_text(&case.value.to_string()),
                        space(),
                        token("=>"),
                        space()
                    ]
                )?;
                format_block_target(&case.target, f)?;
            }
            if let Some(default) = default {
                write!(f, [token(","), space(), token("else"), space()])?;
                format_block_target(default, f)?;
            }
            Ok(())
        }

        Terminator::Unreachable => {
            write!(f, [token("unreachable")])
        }

        Terminator::Invoke {
            call,
            target,
            unwind,
        } => {
            format_call(
                call,
                [
                    "invoke",
                    "invoke.indirect",
                    "invoke.virtual",
                    "invoke.dynamic",
                    "invoke.witness",
                ],
                f,
            )?;
            format_invoke_continuation(target, unwind, f)
        }

        Terminator::NewZeroedTry {
            storage_type,
            space: heap,
            success,
            failure,
        } => {
            write!(
                f,
                [
                    token("new.zeroed.try"),
                    space(),
                    storage_type,
                    token(","),
                    space(),
                    token(heap.label())
                ]
            )?;
            format_allocation_continuation(success, failure, f)
        }

        Terminator::NewUninitTry {
            storage_type,
            space: heap,
            success,
            failure,
        } => {
            write!(
                f,
                [
                    token("new.uninit.try"),
                    space(),
                    storage_type,
                    token(","),
                    space(),
                    token(heap.label())
                ]
            )?;
            format_allocation_continuation(success, failure, f)
        }

        Terminator::NewSliceZeroedTry {
            element,
            length,
            space: heap,
            success,
            failure,
        } => {
            write!(
                f,
                [
                    token("new.slice.zeroed.try"),
                    space(),
                    element,
                    token(","),
                    space(),
                    length,
                    token(","),
                    space(),
                    token(heap.label())
                ]
            )?;
            format_allocation_continuation(success, failure, f)
        }

        Terminator::NewSliceUninitTry {
            element,
            length,
            space: heap,
            success,
            failure,
        } => {
            write!(
                f,
                [
                    token("new.slice.uninit.try"),
                    space(),
                    element,
                    token(","),
                    space(),
                    length,
                    token(","),
                    space(),
                    token(heap.label())
                ]
            )?;
            format_allocation_continuation(success, failure, f)
        }

        Terminator::Panic { payload } => {
            write!(f, [token("panic")])?;
            if let Some(payload) = payload {
                write!(f, [space(), payload])?;
            }
            Ok(())
        }

        Terminator::UnwindResume => write!(f, [token("unwind.resume")]),

        Terminator::Abort { payload } => {
            write!(f, [token("abort")])?;
            if let Some(payload) = payload {
                write!(f, [space(), payload])?;
            }

            Ok(())
        }

        Terminator::TailCall { call } => format_call(
            call,
            [
                "tail.call",
                "tail.call.indirect",
                "tail.call.virtual",
                "tail.call.dynamic",
                "tail.call.witness",
            ],
            f,
        ),
    }
}

/// Format normal and unwind continuations for one invoke.
fn format_invoke_continuation<'a>(
    target: &BlockTarget,
    unwind: &BlockTarget,
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    write!(f, [space(), token("=>"), space()])?;
    format_block_target(target, f)?;
    write!(f, [space(), token("|"), space()])?;
    format_block_target(unwind, f)
}

/// Format one fallible allocation continuation.
fn format_allocation_continuation<'a>(
    success: &BlockTarget,
    failure: &BlockTarget,
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    write!(f, [space(), token("=>"), space()])?;
    format_block_target(success, f)?;
    write!(f, [space(), token("|"), space()])?;
    format_block_target(failure, f)?;

    Ok(())
}

/// Format a check constraint.
fn format_check_constraint<'a>(
    constraint: &CheckConstraint,
    f: &mut Writer<'a, '_>,
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
            write!(f, [token("div.zero"), space(), divisor])
        }
        CheckConstraint::ShiftRange {
            value,
            bit_width,
            is_signed,
        } => {
            let prefix = if *is_signed {
                "shift.range.s"
            } else {
                "shift.range.u"
            };
            write!(
                f,
                [
                    token(prefix),
                    space(),
                    value,
                    token(","),
                    space(),
                    copied_text(&bit_width.to_string())
                ]
            )
        }
        CheckConstraint::Narrow {
            value,
            to_width,
            is_signed,
        } => {
            let prefix = if *is_signed {
                "narrow.range.s"
            } else {
                "narrow.range.u"
            };
            write!(
                f,
                [
                    token(prefix),
                    space(),
                    value,
                    token(","),
                    space(),
                    copied_text(&to_width.to_string())
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
            let name = format!("{}.overflow.{suffix}", overflow_check_family(*operator)?);
            write!(
                f,
                [
                    copied_text(&name),
                    space(),
                    left,
                    token(","),
                    space(),
                    right
                ]
            )
        }
        CheckConstraint::IsType { value, expected } => write!(
            f,
            [
                token("is.type"),
                space(),
                value,
                token(","),
                space(),
                expected
            ]
        ),
        CheckConstraint::IsSubtype { value, expected } => write!(
            f,
            [
                token("is.subtype"),
                space(),
                value,
                token(","),
                space(),
                expected
            ]
        ),
    }
}

/// Return the canonical operator family used in overflow checks.
fn overflow_check_family(operator: BinaryOperator) -> FormatResult<&'static str> {
    let family = match operator {
        BinaryOperator::Add => "add",
        BinaryOperator::Subtract => "sub",
        BinaryOperator::Multiply => "mul",
        BinaryOperator::Divide => "div",
        BinaryOperator::Remainder => "rem",
        _ => {
            return Err(FormatError::SyntaxError {
                message: "unsupported overflow check operator",
            });
        }
    };

    Ok(family)
}

/// Format a parenthesized, comma-separated list of values.
fn format_value_list<'a>(values: &[Value], f: &mut Writer<'a, '_>) -> FormatResult<()> {
    write!(f, [token("(")])?;
    for (i, val) in values.iter().enumerate() {
        if i > 0 {
            write!(f, [token(","), space()])?;
        }
        write!(f, [val])?;
    }
    write!(f, [token(")")])
}

fn format_value_slice<'a>(values: ValueSlice, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    let values = f.context().tree.get_values(values);

    format_value_list(values, f)
}

fn format_block_target<'a>(target: &BlockTarget, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    format_block_id(target.block, f)?;
    if !target.arguments.is_empty() {
        format_value_slice(target.arguments, f)?;
    }

    Ok(())
}
