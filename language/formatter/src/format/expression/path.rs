use crate::DestackFormatter;
use crate::format::operator::format_generic_argument_list;
use destack_ast::{Expression, GenericArgument, LocalNodeId, Path};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::write;

/// Format one path expression and its generic arguments.
pub(crate) fn format_path_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    path: &Path,
    generic_arguments: &[LocalNodeId<GenericArgument>],
) -> FormatResult<()> {
    let _ = node_id;

    write!(f, [path])?;

    if !generic_arguments.is_empty() {
        format_generic_argument_list(f, generic_arguments)?;
    }

    Ok(())
}
