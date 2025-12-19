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
