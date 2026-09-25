use crate::format::argument::list_like;
use crate::{Formatter, FunctionSignature, LocalNodeId, Parameter};
use tspp_fir::format::FormatResult;
use tspp_fir::write;

/// Format one function signature parameter list.
pub(crate) fn format_function_signature_parameters<'ast>(
    signature: &FunctionSignature,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    format_function_parameters(&signature.parameters, f)
}

/// Format one JavaScript parameter list.
pub(crate) fn format_function_parameters<'ast>(
    parameters: &[LocalNodeId<Parameter>],
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    write!(f, [list_like("(", ")", ",", parameters)])
}
