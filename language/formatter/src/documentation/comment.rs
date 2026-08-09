use destack_dir::{Comment, Documentation};
use destack_fir::format::{FormatError, FormatResult};

use crate::{DestackFormatContext, DestackFormatter};

use super::render::FormattedDocumentation;

/// Format the valid documentation group beginning with one source comment.
pub(crate) fn format_documentation_comment<'ast>(
    formatter: &mut DestackFormatter<'ast, '_>,
    comment: Comment,
) -> FormatResult<Option<destack_source::Span>> {
    let Some(documentation) = documentation_for_comment(formatter.context(), comment)? else {
        return Ok(None);
    };

    // render from the parser owned documentation model
    let available_width = documentation_width(formatter.context(), comment);
    let formatted = FormattedDocumentation::new(
        documentation,
        available_width,
        formatter.context().tree,
        formatter.context().strings,
        &formatter.context().options,
    )?;
    let span = documentation.span;
    destack_fir::write!(formatter, [formatted])?;

    Ok(Some(span))
}

/// Return the documentation attached to one leading comment.
fn documentation_for_comment<'a>(
    context: &'a DestackFormatContext<'_>,
    comment: Comment,
) -> FormatResult<Option<&'a Documentation>> {
    if !comment.is_documentation() || comment.is_legal() || !comment.is_leading() {
        return Ok(None);
    }

    // read the exact parser attachment
    let nodes = context
        .source_index
        .documentation_nodes_at(comment.span.start);
    if nodes.len() > 1 {
        return Err(FormatError::SyntaxError {
            message: "documentation comment has multiple owners",
        });
    }
    let Some((_, node_id)) = nodes.first() else {
        return Err(FormatError::SyntaxError {
            message: "documentation comment has no owner",
        });
    };
    let documentation =
        context
            .tree
            .get_documentation(*node_id)
            .ok_or(FormatError::SyntaxError {
                message: "documented node has no documentation",
            })?;

    Ok(Some(documentation))
}

/// Return the documentation width available at one source indentation.
fn documentation_width(context: &DestackFormatContext<'_>, comment: Comment) -> usize {
    let source = context.source_text();
    let tab_width = usize::from(context.options.indent_width);
    let indentation = source
        .bytes_to(comment.span.start)
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .map(|byte| if byte == b'\t' { tab_width } else { 1 })
        .sum::<usize>();

    usize::from(context.options.line_width).saturating_sub(indentation + 4)
}
