use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use super::r#type::{format_function_signature, format_generic_arguments};
use super::value::{format_function_id, format_type_id};
use crate::{Call, Callee, Type, TypeId, Value, Writer};

/// Format one call using dispatch-specific opcodes.
pub(super) fn format_call<'a>(
    call: &Call,
    opcodes: [&'static str; 5],
    formatter: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    // format the callable target
    match &call.callee {
        Callee::Direct {
            function,
            arguments,
        } => {
            write!(formatter, [token(opcodes[0]), space()])?;
            format_function_id(*function, formatter)?;
            format_generic_arguments(arguments, formatter)?;
        }
        Callee::Indirect { value } => {
            write!(formatter, [token(opcodes[1]), space(), value])?;
        }
        Callee::Virtual {
            receiver,
            class,
            slot,
        } => {
            write!(
                formatter,
                [
                    token(opcodes[2]),
                    space(),
                    receiver,
                    token(","),
                    space(),
                    class,
                    token(","),
                    space(),
                    copied_text(&slot.0.to_string())
                ]
            )?;
        }
        Callee::Dynamic {
            receiver,
            constraint,
            slot,
        } => {
            write!(
                formatter,
                [
                    token(opcodes[3]),
                    space(),
                    receiver,
                    token(","),
                    space(),
                    constraint,
                    token(","),
                    space(),
                    copied_text(&slot.0.to_string())
                ]
            )?;
        }
        Callee::Witness {
            receiver,
            interface,
            requirement,
            arguments,
        } => {
            let name = formatter.context().tree.get(*requirement).name;
            let member = formatter.context().strings.get(name).to_string();
            write!(
                formatter,
                [
                    token(opcodes[4]),
                    space(),
                    receiver,
                    token(","),
                    space(),
                    interface,
                    token(","),
                    space(),
                    copied_text(&member)
                ]
            )?;
            format_generic_arguments(arguments, formatter)?;
        }
    }

    // format arguments and the explicit signature
    let arguments = formatter.context().tree.get_values(call.arguments);
    format_value_list(arguments, formatter)?;
    format_call_signature_suffix(&call.signature, formatter)?;

    Ok(())
}

/// Format a parenthesized value list.
pub(super) fn format_value_list<'a>(
    values: &[Value],
    formatter: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    write!(formatter, [token("(")])?;
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            write!(formatter, [token(","), space()])?;
        }
        write!(formatter, [value])?;
    }

    write!(formatter, [token(")")])
}

/// Format one required open-call signature.
fn format_call_signature_suffix<'a>(
    signature: &TypeId,
    formatter: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    write!(formatter, [token(":"), space()])?;

    if let Type::FunctionSignature {
        lifetimes,
        parameters,
        result,
    } = formatter.context().tree.get(*signature)
    {
        format_function_signature(lifetimes, parameters, *result, formatter)
    } else {
        format_type_id(*signature, formatter)
    }
}
