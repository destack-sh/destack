use super::*;
use destack_fir::write;

/// Format static type arguments without multiline trailing commas.
pub(super) fn format_static_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    static_arguments: &Vec<LocalNodeId<Argument>>,
) -> FormatResult<()> {
    if should_hug_static_argument_list(f.context(), static_arguments) {
        write!(f, [token("<")])?;
        for (index, argument_id) in static_arguments.iter().enumerate() {
            if index > 0 {
                write!(f, [token(","), space()])?;
            }
            write!(f, [*argument_id])?;
        }
        write!(f, [token(">")])?;
        return Ok(());
    }

    let should_expand = should_expand_static_argument_list(f.context(), static_arguments);
    let mut list = list_like("<", ">", ",", static_arguments);
    list.disallow_trailing_separator();
    if should_expand {
        write!(f, [list.as_collection().should_expand(true)])
    } else {
        write!(f, [list])
    }
}
