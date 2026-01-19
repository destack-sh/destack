//! Block formatting.

use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    Block, CheckConstraint, FormatMirNode, Function, LocalNodeId, MirFormatContext, MirFormatter,
    Terminator, Value,
};

impl<'a> FormatMirNode<'a, Block> for Block {
    fn format_node(
        &self,
        id: LocalNodeId<Block>,
        f: &mut MirFormatter<'a, '_>,
    ) -> FormatResult<()> {
        // block label
        let block_index = f.context().block_index(id);
        write!(f, [text(&format!("block{block_index}"))])?;

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
                [token("jump"), space(), text(&format!("block{block_index}"))]
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
                    text(&format!("block{then_index}"))
                ]
            )?;
            if !then_arguments.is_empty() {
                format_value_list(then_arguments, f)?;
            }
            write!(
                f,
                [token(","), space(), text(&format!("block{else_index}"))]
            )?;
            if !else_arguments.is_empty() {
                format_value_list(else_arguments, f)?;
            }
            Ok(())
        }

        Terminator::Check {
            condition,
            constraint,
            success,
            failure,
        } => {
            let success_index = f.context().block_index(success.target);
            let failure_index = f.context().block_index(failure.target);
            write!(f, [token("check"), space(), condition, token(","), space()])?;
            format_check_constraint(constraint, f)?;
            write!(
                f,
                [token(","), space(), text(&format!("block{success_index}"))]
            )?;
            if !success.arguments.is_empty() {
                format_value_list(&success.arguments, f)?;
            }
            write!(
                f,
                [token(","), space(), text(&format!("block{failure_index}"))]
            )?;
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
                    text(&format!("block{default_index}"))
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
                        text(&format!("block{case_index}"))
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
                    text(&format!("block{resume_index}"))
                ]
            )?;
            if !resume_arguments.is_empty() {
                format_value_list(resume_arguments, f)?;
            }
            Ok(())
        }

        Terminator::TailCall {
            function,
            arguments,
        } => {
            let tree = f.context().tree;
            let strings = f.context().strings;
            let func = tree.get(*function);
            let name = strings.get(func.name);
            write!(f, [token("tailcall"), space(), token("@"), text(name)])?;
            format_value_list(arguments, f)
        }

        Terminator::TailCallIndirect {
            callee,
            arguments,
            signature,
        } => {
            write!(f, [token("tailcall.indirect"), space(), callee])?;
            format_value_list(arguments, f)?;
            write!(f, [space(), token("->"), space(), signature])
        }

        Terminator::TailCallVirtual {
            receiver,
            arguments,
            declaring_type,
            slot_id,
            declared_target,
            signature,
        } => {
            write!(
                f,
                [
                    token("tailcall.virtual"),
                    space(),
                    receiver,
                    token(","),
                    space(),
                    declaring_type,
                    token(","),
                    space(),
                    text(&slot_id.to_string())
                ]
            )?;
            if let Some(target) = declared_target {
                write!(f, [token(","), space()])?;
                format_function_reference(*target, f)?;
            }
            format_value_list(arguments, f)?;
            write!(f, [space(), token("->"), space(), signature])
        }

        Terminator::TailCallInterface {
            receiver,
            arguments,
            declaring_type,
            slot_id,
            declared_target,
            signature,
        } => {
            write!(
                f,
                [
                    token("tailcall.interface"),
                    space(),
                    receiver,
                    token(","),
                    space(),
                    declaring_type,
                    token(","),
                    space(),
                    text(&slot_id.to_string())
                ]
            )?;
            if let Some(target) = declared_target {
                write!(f, [token(","), space()])?;
                format_function_reference(*target, f)?;
            }
            format_value_list(arguments, f)?;
            write!(f, [space(), token("->"), space(), signature])
        }
    }
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
            let prefix = if *is_signed {
                "bounds.signed"
            } else {
                "bounds.unsigned"
            };
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
            write!(f, [token("div_zero"), space(), divisor])
        }
        CheckConstraint::ShiftRange {
            value,
            bit_width,
            is_signed,
        } => {
            let prefix = if *is_signed {
                "shift.signed"
            } else {
                "shift.unsigned"
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
                "narrow.signed"
            } else {
                "narrow.unsigned"
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
            let prefix = if *is_signed {
                "overflow.signed"
            } else {
                "overflow.unsigned"
            };
            let op_name = operator.to_str();
            let name = format!("{prefix}.{op_name}");
            write!(f, [text(&name), space(), left, token(","), space(), right])
        }
        CheckConstraint::Type { value, expected } => write!(
            f,
            [
                token("type"),
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
                token("union"),
                space(),
                value,
                token(","),
                space(),
                text(&expected.to_string())
            ]
        ),
        CheckConstraint::Vtable { receiver, expected } => write!(
            f,
            [
                token("vtable"),
                space(),
                receiver,
                token(","),
                space(),
                expected
            ]
        ),
        CheckConstraint::Itab { receiver, expected } => write!(
            f,
            [
                token("itab"),
                space(),
                receiver,
                token(","),
                space(),
                text(&expected.index().to_string())
            ]
        ),
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

/// Format a function reference.
fn format_function_reference<'a>(
    function_id: LocalNodeId<Function>,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    let function = f.context().tree.get(function_id);
    let name = f.context().strings.get(function.name);
    write!(f, [token("@"), text(name)])
}
