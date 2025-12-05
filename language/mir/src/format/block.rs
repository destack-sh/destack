//! Block formatting.

use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{Block, FormatMirNode, LocalNodeId, MirFormatContext, MirFormatter, Terminator, Value};

impl<'a> FormatMirNode<'a, Block> for Block {
    fn format_node(
        &self,
        id: LocalNodeId<Block>,
        f: &mut MirFormatter<'a, '_>,
    ) -> FormatResult<()> {
        // block label
        write!(f, [text(&format!("block{}", id.id))])?;

        // block parameters
        if !self.parameters.is_empty() {
            write!(f, [token("(")])?;
            for (i, param) in self.parameters.iter().enumerate() {
                if i > 0 {
                    write!(f, [token(", ")])?;
                }
                write!(f, [&param.value, token(": "), param.ty])?;
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
                write!(f, [token(" "), v])?;
            }
            Ok(())
        }

        Terminator::Jump { target, arguments } => {
            write!(f, [token("jump "), text(&format!("block{}", target.id))])?;
            if !arguments.is_empty() {
                format_args_list(arguments, f)?;
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
            write!(
                f,
                [
                    token("brif "),
                    condition,
                    token(", "),
                    text(&format!("block{}", then_target.id))
                ]
            )?;
            if !then_arguments.is_empty() {
                format_args_list(then_arguments, f)?;
            }
            write!(f, [token(", "), text(&format!("block{}", else_target.id))])?;
            if !else_arguments.is_empty() {
                format_args_list(else_arguments, f)?;
            }
            Ok(())
        }

        Terminator::Switch {
            value,
            default,
            default_arguments,
            cases,
        } => {
            write!(
                f,
                [
                    token("br_table "),
                    value,
                    token(", "),
                    text(&format!("block{}", default.id))
                ]
            )?;
            if !default_arguments.is_empty() {
                format_args_list(default_arguments, f)?;
            }
            for case in cases {
                write!(
                    f,
                    [
                        token(", "),
                        text(&case.value.to_string()),
                        token(" => "),
                        text(&format!("block{}", case.target.id))
                    ]
                )?;
                if !case.arguments.is_empty() {
                    format_args_list(&case.arguments, f)?;
                }
            }
            Ok(())
        }

        Terminator::Unreachable => {
            write!(f, [token("trap unreachable")])
        }
    }
}

fn format_args_list<'a>(args: &[Value], f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    write!(f, [token("(")])?;
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            write!(f, [token(", ")])?;
        }
        write!(f, [arg])?;
    }
    write!(f, [token(")")])
}
