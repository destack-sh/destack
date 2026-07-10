use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use super::r#type::format_function_signature;
use super::value::{format_function_id, format_type_id};
use crate::{Call, Callee, MirFormatter, Type, TypeId, Value};

/// Format one call using dispatch-specific opcodes.
pub(super) fn format_call<'a>(
    call: &Call,
    opcodes: [&'static str; 4],
    formatter: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    // format the callable target
    let has_signature = match &call.callee {
        Callee::Direct { function } => {
            write!(formatter, [token(opcodes[0]), space()])?;
            format_function_id(*function, formatter)?;

            false
        }
        Callee::Indirect { value } => {
            write!(formatter, [token(opcodes[1]), space(), value])?;

            true
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

            true
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

            true
        }
    };

    // format arguments and the explicit open-call signature
    let arguments = formatter.context().tree.get_values(call.arguments);
    format_value_list(arguments, formatter)?;
    if has_signature {
        format_call_signature_suffix(&call.signature, formatter)?;
    }

    Ok(())
}

/// Format a parenthesized value list.
fn format_value_list<'a>(
    values: &[Value],
    formatter: &mut MirFormatter<'a, '_>,
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
    formatter: &mut MirFormatter<'a, '_>,
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
