use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::JsonTrivia;

use super::JsonFormatter;

/// Format a list of trivia items.
pub fn format_trivia_list(trivia: &[JsonTrivia], f: &mut JsonFormatter<'_>) -> FormatResult<()> {
    for item in trivia {
        format_trivia(item, f)?;
    }

    Ok(())
}

/// Format a single trivia item.
pub fn format_trivia(trivia: &JsonTrivia, f: &mut JsonFormatter<'_>) -> FormatResult<()> {
    match trivia {
        // line comment: // content
        JsonTrivia::LineComment { content, .. } => {
            write!(f, [token("//"), text(content), hard_line_break()])?;
        }

        // block comment: /* content */
        JsonTrivia::BlockComment { content, .. } => {
            write!(f, [token("/*"), text(content), token("*/")])?;
        }

        // whitespace is handled by the formatter layout
        JsonTrivia::Whitespace { .. } => {}

        // newlines are handled by the formatter layout
        JsonTrivia::Newline { .. } => {}
    }

    Ok(())
}
