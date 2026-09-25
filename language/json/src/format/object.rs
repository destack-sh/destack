use tspp_fir::format::FormatResult;
use tspp_fir::prelude::*;
use tspp_fir::{format_args, write};

use crate::JsonProperty;

use super::{JsonFormatter, format_string, format_trivia_list, format_value};

/// Format a JSON object.
pub fn format_object(
    properties: &[JsonProperty],
    f: &mut JsonFormatter<'_, '_>,
) -> FormatResult<()> {
    // empty object
    if properties.is_empty() {
        return write!(f, [token("{"), token("}")]);
    }

    // format object with soft line breaks
    write!(
        f,
        [group(&format_args!(
            token("{"),
            soft_block_indent(&format_with(|f| {
                format_object_properties(properties, f)
            })),
            token("}")
        ))]
    )
}

/// Format object properties with separators.
fn format_object_properties(
    properties: &[JsonProperty],
    f: &mut JsonFormatter<'_, '_>,
) -> FormatResult<()> {
    let trailing_comma = f.context().trailing_comma();
    let last_index = properties.len().saturating_sub(1);

    for (index, property) in properties.iter().enumerate() {
        let is_last = index == last_index;

        // format leading trivia (comments before property)
        format_trivia_list(&property.trivia_before, f)?;

        // format key
        format_string(&property.key.value, f)?;

        // colon with space
        write!(f, [token(":"), space()])?;

        // format value
        format_value(&property.value, f)?;

        // format trailing trivia (comments after value)
        format_trivia_list(&property.trivia_after, f)?;

        // add comma and separator
        if is_last {
            // trailing comma only when expanded
            if trailing_comma {
                write!(f, [if_group_breaks(&token(","))])?;
            }
        }
        // not last property
        else {
            write!(f, [token(","), soft_line_break_or_space()])?;
        }
    }

    Ok(())
}
