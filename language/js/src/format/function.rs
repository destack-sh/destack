use crate::format::argument::list_like;
use crate::{FunctionSignature, JsFormatter};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

/// Format one function signature parameter list.
pub(crate) fn format_function_signature_parameters<'ast>(
    signature: &FunctionSignature,
    f: &mut JsFormatter<'ast, '_>,
) -> FormatResult<()> {
    let mut parameters = Vec::with_capacity(signature.parameters.len() + 1);

    if f.context().include_types()
        && let Some(this_parameter) = signature.this_parameter
    {
        parameters.push(this_parameter);
    }

    parameters.extend(signature.parameters.iter().copied());

    write!(f, [list_like("(", ")", ",", &parameters)])
}
