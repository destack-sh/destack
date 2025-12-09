//! Instruction formatting.

use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{FormatMirNode, Function, Global, Instruction, LocalNodeId, MirFormatter, Value};

impl<'a> FormatMirNode<'a, Instruction> for Instruction {
    fn format_node(
        &self,
        _id: LocalNodeId<Instruction>,
        f: &mut MirFormatter<'a, '_>,
    ) -> FormatResult<()> {
        match self {
            Instruction::Const { destination, value } => {
                write!(
                    f,
                    [
                        destination,
                        space(),
                        token("="),
                        space(),
                        token("iconst"),
                        space(),
                        value
                    ]
                )
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
                        token("local.get"),
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
                        token("local.set"),
                        space(),
                        text(&format!("local{local_index}")),
                        token(","),
                        space(),
                        value
                    ]
                )
            }

            Instruction::GlobalAddr {
                destination,
                global,
            } => {
                write!(
                    f,
                    [
                        destination,
                        space(),
                        token("="),
                        space(),
                        token("global.addr"),
                        space()
                    ]
                )?;
                format_global_reference(*global, f)
            }

            Instruction::GlobalConst {
                destination,
                global,
            } => {
                write!(
                    f,
                    [
                        destination,
                        space(),
                        token("="),
                        space(),
                        token("global.const"),
                        space()
                    ]
                )?;
                format_global_reference(*global, f)
            }

            Instruction::Load {
                destination,
                pointer,
            } => {
                write!(
                    f,
                    [
                        destination,
                        space(),
                        token("="),
                        space(),
                        token("load"),
                        space(),
                        pointer
                    ]
                )
            }

            Instruction::Store { pointer, value } => {
                write!(
                    f,
                    [token("store"), space(), pointer, token(","), space(), value]
                )
            }

            Instruction::FieldGet {
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
                        token("field.get"),
                        space(),
                        aggregate,
                        token(","),
                        space(),
                        text(&index.to_string())
                    ]
                )
            }

            Instruction::FieldSet {
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
                        token("field.set"),
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

            Instruction::ElementGet {
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
                        token("element.get"),
                        space(),
                        array,
                        token(","),
                        space(),
                        index
                    ]
                )
            }

            Instruction::ElementSet {
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
                        token("element.set"),
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
                format_function_reference(*function, f)?;
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
                write!(f, [token("call.indirect"), space(), callee])?;
                format_value_list(arguments, f)
            }

            Instruction::ManagedAlloc {
                destination,
                layout,
            } => {
                write!(
                    f,
                    [
                        destination,
                        space(),
                        token("="),
                        space(),
                        token("managed.alloc"),
                        space(),
                        layout
                    ]
                )
            }

            Instruction::ManagedAllocArray {
                destination,
                element,
                length,
            } => {
                write!(
                    f,
                    [
                        destination,
                        space(),
                        token("="),
                        space(),
                        token("managed.alloc_array"),
                        space(),
                        element,
                        token(","),
                        space(),
                        length
                    ]
                )
            }

            Instruction::RawAlloc {
                destination,
                layout,
            } => {
                write!(
                    f,
                    [
                        destination,
                        space(),
                        token("="),
                        space(),
                        token("raw.alloc"),
                        space(),
                        layout
                    ]
                )
            }

            Instruction::RawFree { pointer } => {
                write!(f, [token("raw.free"), space(), pointer])
            }

            Instruction::StackAlloc {
                destination,
                layout,
            } => {
                write!(
                    f,
                    [
                        destination,
                        space(),
                        token("="),
                        space(),
                        token("stack.alloc"),
                        space(),
                        layout
                    ]
                )
            }
        }
    }
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

/// Format a global reference.
fn format_global_reference<'a>(
    global_id: LocalNodeId<Global>,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    let global = f.context().tree.get(global_id);
    let name = f.context().strings.get(global.name);
    write!(f, [token("@"), text(name)])
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
