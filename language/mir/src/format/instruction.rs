//! Instruction formatting.

use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    FormatMirNode, Function, Global, Instruction, LocalNodeId, MemoryOrdering, MirFormatter, Value,
};

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
                operator,
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
                        token(operator.to_str()),
                        space(),
                        argument,
                        space(),
                        token("->"),
                        space(),
                        to_type
                    ]
                )
            }

            Instruction::Select {
                destination,
                condition,
                then_value,
                else_value,
            } => {
                write!(
                    f,
                    [
                        destination,
                        space(),
                        token("="),
                        space(),
                        token("select"),
                        space(),
                        condition,
                        token(","),
                        space(),
                        then_value,
                        token(","),
                        space(),
                        else_value
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
                result_type,
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
                format_global_reference(*global, f)?;
                write!(f, [space(), token("->"), space(), result_type])
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
                result_type,
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
                        pointer,
                        space(),
                        token("->"),
                        space(),
                        result_type
                    ]
                )
            }

            Instruction::Store { pointer, value } => {
                write!(
                    f,
                    [token("store"), space(), pointer, token(","), space(), value]
                )
            }

            Instruction::RawDrop { value } => write!(f, [token("raw.drop"), space(), value]),

            Instruction::StackDrop { value } => write!(f, [token("stack.drop"), space(), value]),

            Instruction::Assume { condition } => {
                write!(f, [token("assume"), space(), condition])
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

            Instruction::FieldAddr {
                destination,
                aggregate,
                index,
                result_type,
            } => {
                write!(
                    f,
                    [
                        destination,
                        space(),
                        token("="),
                        space(),
                        token("field.addr"),
                        space(),
                        aggregate,
                        token(","),
                        space(),
                        text(&index.to_string()),
                        space(),
                        token("->"),
                        space(),
                        result_type
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

            Instruction::ElementAddr {
                destination,
                array,
                index,
                result_type,
            } => {
                write!(
                    f,
                    [
                        destination,
                        space(),
                        token("="),
                        space(),
                        token("element.addr"),
                        space(),
                        array,
                        token(","),
                        space(),
                        index,
                        space(),
                        token("->"),
                        space(),
                        result_type
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

            Instruction::Struct {
                destination,
                ty,
                fields,
            } => {
                write!(
                    f,
                    [
                        destination,
                        space(),
                        token("="),
                        space(),
                        token("struct"),
                        space(),
                        ty,
                        space()
                    ]
                )?;
                let args = f.context().tree.get_arguments(*fields);
                format_value_list(args, f)
            }

            Instruction::Tuple {
                destination,
                ty,
                elements,
            } => {
                write!(
                    f,
                    [
                        destination,
                        space(),
                        token("="),
                        space(),
                        token("tuple"),
                        space(),
                        ty,
                        space()
                    ]
                )?;
                let args = f.context().tree.get_arguments(*elements);
                format_value_list(args, f)
            }

            Instruction::Array {
                destination,
                ty,
                elements,
            } => {
                write!(
                    f,
                    [
                        destination,
                        space(),
                        token("="),
                        space(),
                        token("array"),
                        space(),
                        ty,
                        space()
                    ]
                )?;
                let args = f.context().tree.get_arguments(*elements);
                format_value_list(args, f)
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
                let args = f.context().tree.get_arguments(*arguments);
                format_value_list(args, f)
            }

            Instruction::CallIndirect {
                destination,
                callee,
                arguments,
                signature,
            } => {
                if let Some(dst) = destination {
                    write!(f, [dst, space(), token("="), space()])?;
                }
                write!(f, [token("call.indirect"), space(), callee])?;
                let args = f.context().tree.get_arguments(*arguments);
                format_value_list(args, f)?;
                write!(f, [space(), token("->"), space(), signature])
            }

            Instruction::ManagedAlloc {
                destination,
                layout,
                result_type,
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
                        layout,
                        space(),
                        token("->"),
                        space(),
                        result_type
                    ]
                )
            }

            Instruction::ManagedAllocArray {
                destination,
                element,
                length,
                result_type,
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
                        length,
                        space(),
                        token("->"),
                        space(),
                        result_type
                    ]
                )
            }

            Instruction::RawAlloc {
                destination,
                layout,
                result_type,
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
                        layout,
                        space(),
                        token("->"),
                        space(),
                        result_type
                    ]
                )
            }

            Instruction::RawFree { pointer } => {
                write!(f, [token("raw.free"), space(), pointer])
            }

            Instruction::StackAlloc {
                destination,
                layout,
                result_type,
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
                        layout,
                        space(),
                        token("->"),
                        space(),
                        result_type
                    ]
                )
            }

            Instruction::Intrinsic {
                destination,
                intrinsic,
                arguments,
                ordering,
            } => {
                if let Some(dst) = destination {
                    write!(f, [dst, space(), token("="), space()])?;
                }
                write!(f, [token("intrinsic."), token(intrinsic.to_str())])?;
                let args = f.context().tree.get_arguments(*arguments);
                format_intrinsic_args(args, *ordering, f)
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

/// Format intrinsic arguments with optional memory ordering.
fn format_intrinsic_args<'a>(
    values: &[Value],
    ordering: Option<MemoryOrdering>,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token("(")])?;
    for (i, val) in values.iter().enumerate() {
        if i > 0 {
            write!(f, [token(","), space()])?;
        }
        write!(f, [val])?;
    }
    if let Some(ord) = ordering {
        if !values.is_empty() {
            write!(f, [token(","), space()])?;
        }
        write!(f, [token(ord.to_str())])?;
    }
    write!(f, [token(")")])
}
