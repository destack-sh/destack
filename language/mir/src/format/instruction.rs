//! Instruction formatting.

use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{FormatMirNode, FunctionReference, Instruction, LocalNodeId, MirFormatter, Value};

impl<'a> FormatMirNode<'a, Instruction> for Instruction {
    fn format_node(
        &self,
        _id: LocalNodeId<Instruction>,
        f: &mut MirFormatter<'a, '_>,
    ) -> FormatResult<()> {
        match self {
            Instruction::Constant { destination, value } => {
                write!(f, [destination, space(), token("="), space(), token("iconst"), space(), value])
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
                        space(),
                        token("="),
                        space(),
                        token(operator.to_str()),
                        space(),
                        left,
                        token(","),
                        space(),
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
                        space(),
                        token("="),
                        space(),
                        token(operator.to_str()),
                        space(),
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
                        space(),
                        token("="),
                        space(),
                        token(kind.to_str()),
                        space(),
                        argument,
                        space(),
                        token("->"),
                        space(),
                        to_type
                    ]
                )
            }

            Instruction::LocalGet { destination, local } => {
                let local_index = f.context().local_index(*local);
                write!(
                    f,
                    [
                        destination,
                        space(),
                        token("="),
                        space(),
                        token("load_local"),
                        space(),
                        text(&format!("local{local_index}"))
                    ]
                )
            }

            Instruction::LocalSet { local, value } => {
                let local_index = f.context().local_index(*local);
                write!(
                    f,
                    [
                        token("store_local"),
                        space(),
                        text(&format!("local{local_index}")),
                        token(","),
                        space(),
                        value
                    ]
                )
            }

            Instruction::Load {
                destination,
                pointer,
            } => {
                write!(
                    f,
                    [destination, space(), token("="), space(), token("load"), space(), pointer]
                )
            }

            Instruction::Store { pointer, value } => {
                write!(f, [token("store"), space(), pointer, token(","), space(), value])
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
                        space(),
                        token("="),
                        space(),
                        token("extract_field"),
                        space(),
                        aggregate,
                        token(","),
                        space(),
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
                        space(),
                        token("="),
                        space(),
                        token("insert_field"),
                        space(),
                        aggregate,
                        token(","),
                        space(),
                        text(&index.to_string()),
                        token(","),
                        space(),
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
                        space(),
                        token("="),
                        space(),
                        token("extract_element"),
                        space(),
                        array,
                        token(","),
                        space(),
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
                        space(),
                        token("="),
                        space(),
                        token("insert_element"),
                        space(),
                        array,
                        token(","),
                        space(),
                        index,
                        token(","),
                        space(),
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
                    write!(f, [dst, space(), token("="), space()])?;
                }
                write!(f, [token("call"), space()])?;
                format_function_reference(function, f)?;
                format_value_list(arguments, f)
            }

            Instruction::CallIndirect {
                destination,
                callee,
                arguments,
            } => {
                if let Some(dst) = destination {
                    write!(f, [dst, space(), token("="), space()])?;
                }
                write!(f, [token("call_indirect"), space(), callee])?;
                format_value_list(arguments, f)
            }
        }
    }
}

/// Format a function reference.
fn format_function_reference<'a>(
    reference: &FunctionReference,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    match reference {
        FunctionReference::Local(func_id) => {
            write!(f, [text(&format!("@function{}", func_id.id))])
        }
        FunctionReference::Global(sym_id) => {
            write!(f, [text(&format!("@global({sym_id:?})"))])
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
