use tspp_fir::format::FormatResult;
use tspp_fir::prelude::*;
use tspp_fir::{format_args, write};

use crate::JsonElement;

use super::{JsonFormatter, format_trivia_list, format_value};

/// Format a JSON array.
pub fn format_array(elements: &[JsonElement], f: &mut JsonFormatter<'_, '_>) -> FormatResult<()> {
    // empty array
    if elements.is_empty() {
        return write!(f, [token("["), token("]")]);
    }

    // format array with soft line breaks
    write!(
        f,
        [group(&format_args!(
            token("["),
            soft_block_indent(&format_with(|f| { format_array_elements(elements, f) })),
            token("]")
        ))]
    )
}

/// Format array elements with separators.
fn format_array_elements(
    elements: &[JsonElement],
    f: &mut JsonFormatter<'_, '_>,
) -> FormatResult<()> {
    let trailing_comma = f.context().trailing_comma();
    let last_index = elements.len().saturating_sub(1);

    for (index, element) in elements.iter().enumerate() {
        let is_last = index == last_index;

        // format leading trivia (comments before element)
        format_trivia_list(&element.trivia_before, f)?;

        // format the value
        format_value(&element.value, f)?;

        // format trailing trivia (comments after value)
        format_trivia_list(&element.trivia_after, f)?;

        // add comma and separator
        if is_last {
            // trailing comma only when expanded
            if trailing_comma {
                write!(f, [if_group_breaks(&token(","))])?;
            }
        }
        // not last element
        else {
            write!(f, [token(","), soft_line_break_or_space()])?;
        }
    }

    Ok(())
}
